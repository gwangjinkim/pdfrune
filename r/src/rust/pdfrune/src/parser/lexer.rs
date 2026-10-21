use memchr::memmem;

use crate::error::{Result, PdfruneError};
use crate::parser::object::{ObjectId, PdfDict, PdfDictExt, PdfObject, PdfStream};

/// Token produced when scanning a PDF content stream or object stream.
#[derive(Debug, Clone, PartialEq)]
pub enum ContentToken {
    Operand(PdfObject),
    Operator(Vec<u8>),
}

/// High-speed, zero-allocation-where-possible byte slice lexer for ISO 32000 PDF syntax.
#[derive(Clone)]
pub struct Lexer<'a> {
    data: &'a [u8],
    pos: usize,
}

impl<'a> Lexer<'a> {
    pub fn new(data: &'a [u8]) -> Self {
        Self { data, pos: 0 }
    }

    pub fn with_offset(data: &'a [u8], pos: usize) -> Self {
        Self {
            data,
            pos: pos.min(data.len()),
        }
    }

    #[inline]
    pub fn position(&self) -> usize {
        self.pos
    }

    #[inline]
    pub fn set_position(&mut self, pos: usize) {
        self.pos = pos.min(self.data.len());
    }

    #[inline]
    pub fn is_eof(&self) -> bool {
        self.pos >= self.data.len()
    }

    #[inline]
    pub fn remaining(&self) -> &'a [u8] {
        &self.data[self.pos..]
    }

    /// Returns true if byte is a PDF whitespace character (ISO 32000-1 Table 1).
    #[inline]
    pub fn is_pdf_whitespace(b: u8) -> bool {
        matches!(b, 0x00 | 0x09 | 0x0A | 0x0C | 0x0D | 0x20)
    }

    /// Returns true if byte is a PDF delimiter character (ISO 32000-1 Table 2).
    #[inline]
    pub fn is_pdf_delimiter(b: u8) -> bool {
        matches!(
            b,
            b'(' | b')' | b'<' | b'>' | b'[' | b']' | b'{' | b'}' | b'/' | b'%'
        )
    }

    #[inline]
    pub fn is_regular_char(b: u8) -> bool {
        !Self::is_pdf_whitespace(b) && !Self::is_pdf_delimiter(b)
    }

    /// Skips all PDF whitespace and `% ... \n` comments.
    pub fn skip_whitespace_and_comments(&mut self) {
        while self.pos < self.data.len() {
            let b = self.data[self.pos];
            if Self::is_pdf_whitespace(b) {
                self.pos += 1;
            } else if b == b'%' {
                self.pos += 1;
                while self.pos < self.data.len() {
                    let c = self.data[self.pos];
                    self.pos += 1;
                    if c == b'\n' || c == b'\r' {
                        break;
                    }
                }
            } else {
                break;
            }
        }
    }

    /// Skips a single EOL sequence (`\r\n`, `\n`, or `\r`) after `stream`.
    pub fn skip_stream_eol(&mut self) {
        if self.pos < self.data.len() && self.data[self.pos] == b' ' {
            while self.pos < self.data.len() && self.data[self.pos] == b' ' {
                self.pos += 1;
            }
        }
        if self.pos < self.data.len() && self.data[self.pos] == b'\r' {
            self.pos += 1;
            if self.pos < self.data.len() && self.data[self.pos] == b'\n' {
                self.pos += 1;
            }
        } else if self.pos < self.data.len() && self.data[self.pos] == b'\n' {
            self.pos += 1;
        }
    }

    #[inline]
    pub fn peek_byte(&self) -> Option<u8> {
        self.data.get(self.pos).copied()
    }

    /// Reads the next regular word/keyword token without advancing across delimiters.
    pub fn read_word(&mut self) -> Option<&'a [u8]> {
        self.skip_whitespace_and_comments();
        if self.pos >= self.data.len() {
            return None;
        }
        let start = self.pos;
        while self.pos < self.data.len() && Self::is_regular_char(self.data[self.pos]) {
            self.pos += 1;
        }
        if self.pos > start {
            Some(&self.data[start..self.pos])
        } else {
            None
        }
    }

    /// Parses an indirect object definition `obj_num gen_num obj ... endobj` at the current offset,
    /// or a direct object if no `obj` header is present.
    pub fn parse_indirect_object(&mut self) -> Result<(Option<ObjectId>, PdfObject)> {
        self.skip_whitespace_and_comments();
        let saved = self.pos;

        if let (Some(obj_word), Some(gen_word), Some(kw)) =
            (self.read_word(), self.read_word(), self.read_word())
        {
            if kw == b"obj" {
                if let (Ok(obj_num), Ok(gen_num)) =
                    (parse_ascii_u32(obj_word), parse_ascii_u16(gen_word))
                {
                    let obj = self.parse_object()?;
                    self.skip_whitespace_and_comments();
                    let before_end = self.pos;
                    if self.read_word() != Some(b"endobj") {
                        self.pos = before_end;
                    }
                    return Ok((Some((obj_num, gen_num)), obj));
                }
            }
        }

        self.pos = saved;
        let obj = self.parse_object()?;
        Ok((None, obj))
    }

    /// Parses the next `PdfObject` from the byte stream.
    pub fn parse_object(&mut self) -> Result<PdfObject> {
        self.skip_whitespace_and_comments();
        let Some(b) = self.peek_byte() else {
            return Err(PdfruneError::SyntaxError {
                offset: self.pos,
                message: "Unexpected EOF while parsing PDF object".into(),
            });
        };

        match b {
            b'/' => self.parse_name(),
            b'(' => self.parse_literal_string(),
            b'[' => self.parse_array(),
            b'<' => {
                if self.data.get(self.pos + 1) == Some(&b'<') {
                    let dict = self.parse_dict()?;
                    let saved = self.pos;
                    self.skip_whitespace_and_comments();
                    if self.remaining().starts_with(b"stream")
                        && !self
                            .data
                            .get(self.pos + 6)
                            .copied()
                            .is_some_and(Self::is_regular_char)
                    {
                        self.pos += 6; // consume "stream"
                        self.skip_stream_eol();
                        let data = self.read_stream_body(&dict)?;
                        return Ok(PdfObject::Stream(PdfStream::new(dict, data)));
                    }
                    self.pos = saved;
                    Ok(PdfObject::Dict(dict))
                } else {
                    self.parse_hex_string()
                }
            }
            _ => self.parse_number_keyword_or_ref(),
        }
    }

    /// Parses a PDF `/Name` token, decoding `#XX` hex escapes per ISO 32000-1 Section 7.3.5.
    fn parse_name(&mut self) -> Result<PdfObject> {
        self.pos += 1; // skip '/'
        let mut name = Vec::with_capacity(16);
        while self.pos < self.data.len() {
            let b = self.data[self.pos];
            if !Self::is_regular_char(b) {
                break;
            }
            if b == b'#' && self.pos + 2 < self.data.len() {
                if let (Some(h1), Some(h2)) = (
                    hex_val(self.data[self.pos + 1]),
                    hex_val(self.data[self.pos + 2]),
                ) {
                    name.push((h1 << 4) | h2);
                    self.pos += 3;
                    continue;
                }
            }
            name.push(b);
            self.pos += 1;
        }
        Ok(PdfObject::Name(name))
    }

    /// Parses a literal string `(...)` with balanced parentheses and backslash escapes.
    fn parse_literal_string(&mut self) -> Result<PdfObject> {
        self.pos += 1; // skip '('
        let mut out = Vec::new();
        let mut depth = 1i32;

        while self.pos < self.data.len() && depth > 0 {
            let b = self.data[self.pos];
            self.pos += 1;
            match b {
                b'(' => {
                    depth += 1;
                    out.push(b'(');
                }
                b')' => {
                    depth -= 1;
                    if depth > 0 {
                        out.push(b')');
                    }
                }
                b'\\' => {
                    if self.pos >= self.data.len() {
                        break;
                    }
                    let esc = self.data[self.pos];
                    self.pos += 1;
                    match esc {
                        b'n' => out.push(b'\n'),
                        b'r' => out.push(b'\r'),
                        b't' => out.push(b'\t'),
                        b'b' => out.push(0x08),
                        b'f' => out.push(0x0C),
                        b'(' => out.push(b'('),
                        b')' => out.push(b')'),
                        b'\\' => out.push(b'\\'),
                        b'\r' => {
                            if self.pos < self.data.len() && self.data[self.pos] == b'\n' {
                                self.pos += 1;
                            }
                        }
                        b'\n' => {}
                        b'0'..=b'7' => {
                            let mut oct = (esc - b'0') as u16;
                            for _ in 0..2 {
                                if self.pos < self.data.len()
                                    && (b'0'..=b'7').contains(&self.data[self.pos])
                                {
                                    oct = (oct << 3) | ((self.data[self.pos] - b'0') as u16);
                                    self.pos += 1;
                                } else {
                                    break;
                                }
                            }
                            out.push((oct & 0xFF) as u8);
                        }
                        other => out.push(other),
                    }
                }
                b'\r' => {
                    if self.pos < self.data.len() && self.data[self.pos] == b'\n' {
                        self.pos += 1;
                    }
                    out.push(b'\n');
                }
                other => out.push(other),
            }
        }

        Ok(PdfObject::String(out))
    }

    /// Parses a hexadecimal string `<...>`.
    fn parse_hex_string(&mut self) -> Result<PdfObject> {
        self.pos += 1; // skip '<'
        let mut out = Vec::new();
        let mut high: Option<u8> = None;

        while self.pos < self.data.len() {
            let b = self.data[self.pos];
            self.pos += 1;
            if b == b'>' {
                break;
            }
            if Self::is_pdf_whitespace(b) {
                continue;
            }
            if let Some(val) = hex_val(b) {
                if let Some(h) = high.take() {
                    out.push((h << 4) | val);
                } else {
                    high = Some(val);
                }
            }
        }
        if let Some(h) = high {
            out.push(h << 4);
        }
        Ok(PdfObject::HexString(out))
    }

    /// Parses a PDF array `[ ... ]`.
    fn parse_array(&mut self) -> Result<PdfObject> {
        self.pos += 1; // skip '['
        let mut items = Vec::new();
        loop {
            self.skip_whitespace_and_comments();
            if self.pos >= self.data.len() {
                break;
            }
            if self.data[self.pos] == b']' {
                self.pos += 1;
                break;
            }
            items.push(self.parse_object()?);
        }
        Ok(PdfObject::Array(items))
    }

    /// Parses a PDF dictionary `<< ... >>`.
    pub fn parse_dict(&mut self) -> Result<PdfDict> {
        self.pos += 2; // skip '<<'
        let mut dict = PdfDict::new();
        loop {
            self.skip_whitespace_and_comments();
            if self.pos >= self.data.len() {
                break;
            }
            if self.remaining().starts_with(b">>") {
                self.pos += 2;
                break;
            }
            if self.data[self.pos] != b'/' {
                // Recover gracefully if malformed token inside dict
                self.pos += 1;
                continue;
            }
            let key_obj = self.parse_name()?;
            let Some(key) = key_obj.as_name().map(|s| s.to_vec()) else {
                continue;
            };
            self.skip_whitespace_and_comments();
            if self.remaining().starts_with(b">>") {
                self.pos += 2;
                break;
            }
            let val = self.parse_object()?;
            dict.insert(key, val);
        }
        Ok(dict)
    }

    /// Reads the raw bytes of a stream body between `stream` and `endstream`.
    fn read_stream_body(&mut self, dict: &PdfDict) -> Result<Vec<u8>> {
        let start = self.pos;

        // If /Length is a direct integer and points straight to endstream, use it fast
        if let Some(len) = dict.get_int(b"Length") {
            if len >= 0 {
                let len_usize = len as usize;
                if start + len_usize <= self.data.len() {
                    let after = &self.data[start + len_usize..];
                    let trimmed = trim_leading_pdf_ws(after);
                    if trimmed.starts_with(b"endstream") {
                        let data = self.data[start..start + len_usize].to_vec();
                        self.pos = start + len_usize;
                        self.skip_whitespace_and_comments();
                        if self.remaining().starts_with(b"endstream") {
                            self.pos += 9;
                        }
                        return Ok(data);
                    }
                }
            }
        }

        // Fallback: SIMD search for `endstream`
        if let Some(rel_end) = memmem::find(&self.data[start..], b"endstream") {
            let mut end = start + rel_end;
            // Strip trailing EOL before endstream
            if end > start && self.data[end - 1] == b'\n' {
                end -= 1;
                if end > start && self.data[end - 1] == b'\r' {
                    end -= 1;
                }
            } else if end > start && self.data[end - 1] == b'\r' {
                end -= 1;
            }
            let data = self.data[start..end].to_vec();
            self.pos = start + rel_end + 9;
            return Ok(data);
        }

        Err(PdfruneError::SyntaxError {
            offset: start,
            message: "Missing 'endstream' marker".into(),
        })
    }

    /// Parses a number, boolean, null, or indirect reference `N G R`.
    fn parse_number_keyword_or_ref(&mut self) -> Result<PdfObject> {
        let start = self.pos;
        let Some(word) = self.read_word() else {
            return Err(PdfruneError::SyntaxError {
                offset: start,
                message: "Expected token".into(),
            });
        };

        match word {
            b"true" => return Ok(PdfObject::Bool(true)),
            b"false" => return Ok(PdfObject::Bool(false)),
            b"null" => return Ok(PdfObject::Null),
            _ => {}
        }

        // Check if integer and potentially an indirect reference `obj_num gen_num R`
        if let Ok(int_val) = parse_ascii_i64(word) {
            if int_val >= 0 {
                let saved = self.pos;
                if let (Some(gen_word), Some(r_word)) = (self.read_word(), self.read_word()) {
                    if r_word == b"R" {
                        if let Ok(gen_num) = parse_ascii_u16(gen_word) {
                            return Ok(PdfObject::Reference((int_val as u32, gen_num)));
                        }
                    }
                }
                self.pos = saved;
            }
            return Ok(PdfObject::Int(int_val));
        }

        if let Ok(real_val) = parse_ascii_f64(word) {
            return Ok(PdfObject::Real(real_val));
        }

        // If encountered in a loose stream context, treat unknown bare symbol as Name/String
        Ok(PdfObject::Name(word.to_vec()))
    }

    /// Tokenizes a PDF page content stream into operands and operators (`BT`, `Tf`, `Tj`, etc.).
    pub fn tokenize_content_stream(&mut self) -> Result<Vec<ContentToken>> {
        let mut tokens = Vec::new();
        loop {
            self.skip_whitespace_and_comments();
            let Some(b) = self.peek_byte() else {
                break;
            };

            match b {
                b'/' | b'(' | b'[' => {
                    tokens.push(ContentToken::Operand(self.parse_object()?));
                }
                b'<' => {
                    if self.data.get(self.pos + 1) == Some(&b'<') {
                        let dict = self.parse_dict()?;
                        tokens.push(ContentToken::Operand(PdfObject::Dict(dict)));
                    } else {
                        tokens.push(ContentToken::Operand(self.parse_hex_string()?));
                    }
                }
                b'+' | b'-' | b'.' | b'0'..=b'9' => {
                    let word = self.read_word().unwrap_or(b"0");
                    if let Ok(i) = parse_ascii_i64(word) {
                        tokens.push(ContentToken::Operand(PdfObject::Int(i)));
                    } else if let Ok(r) = parse_ascii_f64(word) {
                        tokens.push(ContentToken::Operand(PdfObject::Real(r)));
                    } else {
                        tokens.push(ContentToken::Operator(word.to_vec()));
                    }
                }
                b']' | b'>' | b')' => {
                    // Stray closing delimiter recovery
                    self.pos += 1;
                }
                _ => {
                    let Some(word) = self.read_word() else {
                        self.pos += 1;
                        continue;
                    };
                    match word {
                        b"true" => tokens.push(ContentToken::Operand(PdfObject::Bool(true))),
                        b"false" => tokens.push(ContentToken::Operand(PdfObject::Bool(false))),
                        b"null" => tokens.push(ContentToken::Operand(PdfObject::Null)),
                        b"BI" => {
                            // Inline image: skip until EI operator
                            self.skip_inline_image();
                        }
                        op => tokens.push(ContentToken::Operator(op.to_vec())),
                    }
                }
            }
        }
        Ok(tokens)
    }

    fn skip_inline_image(&mut self) {
        if let Some(rel) = memmem::find(self.remaining(), b"EI") {
            self.pos += rel + 2;
        } else {
            self.pos = self.data.len();
        }
    }
}

#[inline]
fn hex_val(b: u8) -> Option<u8> {
    match b {
        b'0'..=b'9' => Some(b - b'0'),
        b'a'..=b'f' => Some(b - b'a' + 10),
        b'A'..=b'F' => Some(b - b'A' + 10),
        _ => None,
    }
}

#[inline]
fn trim_leading_pdf_ws(mut s: &[u8]) -> &[u8] {
    while let Some(&first) = s.first() {
        if Lexer::is_pdf_whitespace(first) {
            s = &s[1..];
        } else {
            break;
        }
    }
    s
}

#[allow(clippy::result_unit_err)]
pub fn parse_ascii_i64(bytes: &[u8]) -> std::result::Result<i64, ()> {
    std::str::from_utf8(bytes)
        .map_err(|_| ())?
        .parse::<i64>()
        .map_err(|_| ())
}

#[allow(clippy::result_unit_err)]
pub fn parse_ascii_u32(bytes: &[u8]) -> std::result::Result<u32, ()> {
    std::str::from_utf8(bytes)
        .map_err(|_| ())?
        .parse::<u32>()
        .map_err(|_| ())
}

#[allow(clippy::result_unit_err)]
pub fn parse_ascii_u16(bytes: &[u8]) -> std::result::Result<u16, ()> {
    std::str::from_utf8(bytes)
        .map_err(|_| ())?
        .parse::<u16>()
        .map_err(|_| ())
}

#[allow(clippy::result_unit_err)]
pub fn parse_ascii_f64(bytes: &[u8]) -> std::result::Result<f64, ()> {
    std::str::from_utf8(bytes)
        .map_err(|_| ())?
        .parse::<f64>()
        .map_err(|_| ())
}
