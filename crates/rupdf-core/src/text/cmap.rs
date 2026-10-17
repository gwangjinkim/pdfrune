use std::collections::BTreeMap;

use crate::document::Document;
use crate::parser::lexer::{ContentToken, Lexer};
use crate::parser::object::{PdfDict, PdfDictExt, PdfObject};

/// Resolved font encoding and glyph width metrics for a PDF font resource.
#[derive(Debug, Clone, Default)]
pub struct FontInfo {
    pub name: String,
    pub base_font: String,
    pub is_two_byte: bool,
    pub to_unicode: BTreeMap<u32, String>,
    pub differences: BTreeMap<u8, String>,
    pub widths: BTreeMap<u32, f64>,
    pub default_width: f64,
}

impl FontInfo {
    /// Extracts `FontInfo` from a PDF `/Font` resource dictionary.
    pub fn from_font_dict(doc: &Document, name: &str, font_dict: &PdfDict) -> Self {
        let subtype = font_dict.get_name_str(b"Subtype").unwrap_or("Type1");
        let base_font = font_dict
            .get_name_str(b"BaseFont")
            .unwrap_or(name)
            .to_string();
        let mut is_two_byte =
            subtype == "Type0" || subtype == "CIDFontType0" || subtype == "CIDFontType2";

        let mut to_unicode = BTreeMap::new();
        if let Some(tu_obj) = doc.resolve_dict_entry(font_dict, b"ToUnicode") {
            if let Some(stm) = tu_obj.as_stream() {
                if let Ok(decoded) = stm.decode_data() {
                    let (map, two_byte_cmap) = parse_tounicode_cmap(&decoded);
                    to_unicode = map;
                    if two_byte_cmap {
                        is_two_byte = true;
                    }
                }
            }
        }

        let mut differences = BTreeMap::new();
        if let Some(enc_obj) = doc.resolve_dict_entry(font_dict, b"Encoding") {
            if let Some(enc_dict) = enc_obj.as_dict() {
                if let Some(diff_arr) = doc
                    .resolve_dict_entry(enc_dict, b"Differences")
                    .and_then(PdfObject::as_array)
                {
                    let mut cur_code: i64 = 0;
                    for item in diff_arr {
                        match item {
                            PdfObject::Int(c) => cur_code = *c,
                            PdfObject::Name(glyph_name) => {
                                if (0..=255).contains(&cur_code) {
                                    if let Some(s) = glyph_name_to_unicode(glyph_name) {
                                        differences.insert(cur_code as u8, s);
                                    }
                                }
                                cur_code += 1;
                            }
                            _ => {}
                        }
                    }
                }
            }
        }

        let mut widths = BTreeMap::new();
        let mut default_width = 550.0;

        // Standard /FirstChar + /Widths array
        if let (Some(first_char), Some(w_arr)) = (
            font_dict.get_int(b"FirstChar"),
            doc.resolve_dict_entry(font_dict, b"Widths")
                .and_then(PdfObject::as_array),
        ) {
            for (idx, w_obj) in w_arr.iter().enumerate() {
                if let Some(w) = w_obj.as_f64() {
                    widths.insert((first_char.max(0) as u32) + (idx as u32), w);
                }
            }
        }

        // CIDFont /DescendantFonts widths (/W and /DW)
        if let Some(desc_arr) = doc
            .resolve_dict_entry(font_dict, b"DescendantFonts")
            .and_then(PdfObject::as_array)
        {
            if let Some(first_desc) = desc_arr.first() {
                if let Some(cid_dict) = doc.resolve(first_desc).as_dict() {
                    if let Some(dw) = cid_dict.get_f64(b"DW") {
                        default_width = dw;
                    }
                    if let Some(w_arr) = doc
                        .resolve_dict_entry(cid_dict, b"W")
                        .and_then(PdfObject::as_array)
                    {
                        parse_cid_widths(doc, w_arr, &mut widths);
                    }
                }
            }
        }

        Self {
            name: name.to_string(),
            base_font,
            is_two_byte,
            to_unicode,
            differences,
            widths,
            default_width,
        }
    }

    /// Decodes a PDF string byte slice into `(char_code, unicode_text, width_thousandths)` items.
    pub fn decode_bytes(&self, bytes: &[u8]) -> Vec<(u32, String, f64)> {
        let mut out = Vec::with_capacity(bytes.len());
        if self.is_two_byte && bytes.len() >= 2 && bytes.len().is_multiple_of(2) {
            for chunk in bytes.chunks_exact(2) {
                let code = u16::from_be_bytes([chunk[0], chunk[1]]) as u32;
                let text = if let Some(mapped) = self.to_unicode.get(&code) {
                    mapped.clone()
                } else if let Some(ch) = char::from_u32(code) {
                    ch.to_string()
                } else {
                    String::new()
                };
                let w = self.glyph_width(code, &text);
                out.push((code, text, w));
            }
        } else {
            for &b in bytes {
                let code = b as u32;
                let text = if let Some(mapped) = self.to_unicode.get(&code) {
                    mapped.clone()
                } else if let Some(diff) = self.differences.get(&b) {
                    diff.clone()
                } else {
                    win_ansi_to_char(b).to_string()
                };
                let w = self.glyph_width(code, &text);
                out.push((code, text, w));
            }
        }
        out
    }

    /// Returns glyph width in 1/1000ths of text space.
    pub fn glyph_width(&self, code: u32, decoded: &str) -> f64 {
        if let Some(&w) = self.widths.get(&code) {
            if w > 0.0 {
                return w;
            }
        }
        if let Some(ch) = decoded.chars().next() {
            return estimate_latin_char_width(ch);
        }
        self.default_width
    }
}

fn parse_cid_widths(doc: &Document, w_arr: &[PdfObject], widths: &mut BTreeMap<u32, f64>) {
    let mut i = 0;
    while i + 1 < w_arr.len() {
        let Some(start_cid) = w_arr[i].as_i64() else {
            i += 1;
            continue;
        };
        let next = doc.resolve(&w_arr[i + 1]);
        if let Some(arr) = next.as_array() {
            for (idx, val) in arr.iter().enumerate() {
                if let Some(w) = val.as_f64() {
                    widths.insert((start_cid.max(0) as u32) + (idx as u32), w);
                }
            }
            i += 2;
        } else if i + 2 < w_arr.len() {
            if let (Some(end_cid), Some(w)) = (w_arr[i + 1].as_i64(), w_arr[i + 2].as_f64()) {
                for cid in start_cid.max(0)..=end_cid.max(0) {
                    widths.insert(cid as u32, w);
                }
            }
            i += 3;
        } else {
            break;
        }
    }
}

/// Parses a PDF `/ToUnicode` CMap stream (`beginbfchar` and `beginbfrange`).
pub fn parse_tounicode_cmap(data: &[u8]) -> (BTreeMap<u32, String>, bool) {
    let mut map = BTreeMap::new();
    let mut is_two_byte = false;
    let mut lexer = Lexer::new(data);
    let Ok(tokens) = lexer.tokenize_content_stream() else {
        return (map, false);
    };

    let mut i = 0;
    while i < tokens.len() {
        match &tokens[i] {
            ContentToken::Operator(op) if op == b"begincodespacerange" => {
                i += 1;
                while i < tokens.len() {
                    if let ContentToken::Operator(end_op) = &tokens[i] {
                        if end_op == b"endcodespacerange" {
                            break;
                        }
                    }
                    if let ContentToken::Operand(PdfObject::HexString(bytes)) = &tokens[i] {
                        if bytes.len() >= 2 {
                            is_two_byte = true;
                        }
                    }
                    i += 1;
                }
            }
            ContentToken::Operator(op) if op == b"beginbfchar" => {
                i += 1;
                while i + 1 < tokens.len() {
                    if let ContentToken::Operator(end_op) = &tokens[i] {
                        if end_op == b"endbfchar" {
                            break;
                        }
                    }
                    if let (ContentToken::Operand(src_obj), ContentToken::Operand(dst_obj)) =
                        (&tokens[i], &tokens[i + 1])
                    {
                        if let Some((code, len)) = obj_to_char_code(src_obj) {
                            if len >= 2 {
                                is_two_byte = true;
                            }
                            if let Some(uni) = obj_to_unicode_string(dst_obj) {
                                map.insert(code, uni);
                            }
                        }
                        i += 2;
                    } else {
                        i += 1;
                    }
                }
            }
            ContentToken::Operator(op) if op == b"beginbfrange" => {
                i += 1;
                while i + 2 < tokens.len() {
                    if let ContentToken::Operator(end_op) = &tokens[i] {
                        if end_op == b"endbfrange" {
                            break;
                        }
                    }
                    if let (
                        ContentToken::Operand(start_obj),
                        ContentToken::Operand(end_obj),
                        ContentToken::Operand(target_obj),
                    ) = (&tokens[i], &tokens[i + 1], &tokens[i + 2])
                    {
                        if let (Some((start_code, len)), Some((end_code, _))) =
                            (obj_to_char_code(start_obj), obj_to_char_code(end_obj))
                        {
                            if len >= 2 {
                                is_two_byte = true;
                            }
                            match target_obj {
                                PdfObject::Array(arr) => {
                                    for (offset, item) in arr.iter().enumerate() {
                                        let code = start_code + (offset as u32);
                                        if code > end_code {
                                            break;
                                        }
                                        if let Some(uni) = obj_to_unicode_string(item) {
                                            map.insert(code, uni);
                                        }
                                    }
                                }
                                _ => {
                                    if let Some(base_str) = obj_to_unicode_string(target_obj) {
                                        let mut chars: Vec<u32> =
                                            base_str.chars().map(|c| c as u32).collect();
                                        let count = end_code.saturating_sub(start_code).min(1024);
                                        for offset in 0..=count {
                                            let s: String = chars
                                                .iter()
                                                .filter_map(|&c| char::from_u32(c))
                                                .collect();
                                            map.insert(start_code + offset, s);
                                            if let Some(last) = chars.last_mut() {
                                                *last += 1;
                                            }
                                        }
                                    }
                                }
                            }
                        }
                        i += 3;
                    } else {
                        i += 1;
                    }
                }
            }
            _ => {}
        }
        i += 1;
    }

    (map, is_two_byte)
}

fn obj_to_char_code(obj: &PdfObject) -> Option<(u32, usize)> {
    let bytes = obj.as_bytes()?;
    let mut val = 0u32;
    for &b in bytes {
        val = (val << 8) | (b as u32);
    }
    Some((val, bytes.len()))
}

fn obj_to_unicode_string(obj: &PdfObject) -> Option<String> {
    let bytes = obj.as_bytes()?;
    if bytes.is_empty() {
        return None;
    }
    if bytes.len() == 1 {
        return Some((bytes[0] as char).to_string());
    }
    if bytes.len().is_multiple_of(2) {
        let u16s: Vec<u16> = bytes
            .chunks_exact(2)
            .map(|c| u16::from_be_bytes([c[0], c[1]]))
            .collect();
        return Some(String::from_utf16_lossy(&u16s));
    }
    Some(String::from_utf8_lossy(bytes).into_owned())
}

fn glyph_name_to_unicode(name: &[u8]) -> Option<String> {
    let s = std::str::from_utf8(name).ok()?;
    if s.len() == 1 {
        return Some(s.to_string());
    }
    if let Some(hex) = s.strip_prefix("uni") {
        if hex.len() == 4 {
            if let Ok(code) = u32::from_str_radix(hex, 16) {
                return char::from_u32(code).map(|c| c.to_string());
            }
        }
    }
    let mapped = match s {
        "space" | "nbspace" => " ",
        "exclam" => "!",
        "quotedbl" => "\"",
        "numbersign" => "#",
        "dollar" => "$",
        "percent" => "%",
        "ampersand" => "&",
        "quotesingle" | "quoteright" => "'",
        "quoteleft" => "`",
        "quotedblleft" => "\u{201C}",
        "quotedblright" => "\u{201D}",
        "parenleft" => "(",
        "parenright" => ")",
        "asterisk" => "*",
        "plus" => "+",
        "comma" => ",",
        "hyphen" | "minus" => "-",
        "endash" => "\u{2013}",
        "emdash" => "\u{2014}",
        "period" => ".",
        "slash" => "/",
        "zero" => "0",
        "one" => "1",
        "two" => "2",
        "three" => "3",
        "four" => "4",
        "five" => "5",
        "six" => "6",
        "seven" => "7",
        "eight" => "8",
        "nine" => "9",
        "colon" => ":",
        "semicolon" => ";",
        "less" => "<",
        "equal" => "=",
        "greater" => ">",
        "question" => "?",
        "at" => "@",
        "bracketleft" => "[",
        "backslash" => "\\",
        "bracketright" => "]",
        "asciicircum" => "^",
        "underscore" => "_",
        "braceleft" => "{",
        "bar" => "|",
        "braceright" => "}",
        "asciitilde" => "~",
        "bullet" => "\u{2022}",
        "fi" => "fi",
        "fl" => "fl",
        "ff" => "ff",
        "ffi" => "ffi",
        "ffl" => "ffl",
        "ellipsis" => "\u{2026}",
        "copyright" => "\u{00A9}",
        "registered" => "\u{00AE}",
        "trademark" => "\u{2122}",
        "degree" => "\u{00B0}",
        "plusminus" => "\u{00B1}",
        "Euro" => "\u{20AC}",
        _ => return None,
    };
    Some(mapped.to_string())
}

fn win_ansi_to_char(b: u8) -> char {
    match b {
        0x80 => '\u{20AC}',
        0x85 => '\u{2026}',
        0x91 => '\u{2018}',
        0x92 => '\u{2019}',
        0x93 => '\u{201C}',
        0x94 => '\u{201D}',
        0x95 => '\u{2022}',
        0x96 => '\u{2013}',
        0x97 => '\u{2014}',
        0x99 => '\u{2122}',
        other => other as char,
    }
}

fn estimate_latin_char_width(ch: char) -> f64 {
    match ch {
        ' ' => 278.0,
        'i' | 'l' | 'I' | '!' | '.' | ',' | ':' | ';' | '|' | '\'' => 278.0,
        'f' | 'j' | 'r' | 't' | '(' | ')' | '[' | ']' | '-' => 333.0,
        'm' | 'w' => 833.0,
        'M' | 'W' => 889.0,
        'A'..='Z' => 667.0,
        '0'..='9' => 556.0,
        _ => 530.0,
    }
}
