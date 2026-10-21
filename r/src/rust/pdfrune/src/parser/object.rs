use std::collections::BTreeMap;

use crate::error::{Result, PdfruneError};
use crate::parser::filter::decode_stream;

/// Unique identifier for an indirect PDF object: `(object_number, generation_number)`.
pub type ObjectId = (u32, u16);

/// PDF Dictionary mapping PDF Name keys (without leading `/`) to `PdfObject` values.
pub type PdfDict = BTreeMap<Vec<u8>, PdfObject>;

/// Represents a PDF Stream object (`<< ... >> stream ... endstream`).
#[derive(Debug, Clone, PartialEq)]
pub struct PdfStream {
    pub dict: PdfDict,
    pub data: Vec<u8>,
}

impl PdfStream {
    pub fn new(dict: PdfDict, data: Vec<u8>) -> Self {
        Self { dict, data }
    }

    /// Decodes the stream payload by applying any `/Filter` and `/DecodeParms` specified in `dict`.
    pub fn decode_data(&self) -> Result<Vec<u8>> {
        decode_stream(&self.data, &self.dict)
    }
}

/// Core ISO 32000 PDF Object DOM.
#[derive(Debug, Clone, PartialEq)]
pub enum PdfObject {
    Null,
    Bool(bool),
    Int(i64),
    Real(f64),
    String(Vec<u8>),
    HexString(Vec<u8>),
    Name(Vec<u8>),
    Array(Vec<PdfObject>),
    Dict(PdfDict),
    Stream(PdfStream),
    Reference(ObjectId),
}

impl PdfObject {
    pub fn as_bool(&self) -> Option<bool> {
        match self {
            PdfObject::Bool(b) => Some(*b),
            _ => None,
        }
    }

    pub fn as_i64(&self) -> Option<i64> {
        match self {
            PdfObject::Int(i) => Some(*i),
            PdfObject::Real(r) => Some(*r as i64),
            _ => None,
        }
    }

    pub fn as_f64(&self) -> Option<f64> {
        match self {
            PdfObject::Real(r) => Some(*r),
            PdfObject::Int(i) => Some(*i as f64),
            _ => None,
        }
    }

    pub fn as_bytes(&self) -> Option<&[u8]> {
        match self {
            PdfObject::String(b) | PdfObject::HexString(b) | PdfObject::Name(b) => Some(b),
            _ => None,
        }
    }

    pub fn as_name(&self) -> Option<&[u8]> {
        match self {
            PdfObject::Name(b) => Some(b),
            _ => None,
        }
    }

    pub fn as_name_str(&self) -> Option<&str> {
        self.as_name().and_then(|b| std::str::from_utf8(b).ok())
    }

    pub fn as_array(&self) -> Option<&[PdfObject]> {
        match self {
            PdfObject::Array(arr) => Some(arr),
            _ => None,
        }
    }

    pub fn as_dict(&self) -> Option<&PdfDict> {
        match self {
            PdfObject::Dict(d) => Some(d),
            PdfObject::Stream(s) => Some(&s.dict),
            _ => None,
        }
    }

    pub fn as_dict_mut(&mut self) -> Option<&mut PdfDict> {
        match self {
            PdfObject::Dict(d) => Some(d),
            PdfObject::Stream(s) => Some(&mut s.dict),
            _ => None,
        }
    }

    pub fn as_stream(&self) -> Option<&PdfStream> {
        match self {
            PdfObject::Stream(s) => Some(s),
            _ => None,
        }
    }

    pub fn as_reference(&self) -> Option<ObjectId> {
        match self {
            PdfObject::Reference(id) => Some(*id),
            _ => None,
        }
    }

    /// Decodes a PDF string (`String` or `HexString`) into a Rust `String`,
    /// supporting UTF-16BE BOM (`\xFE\xFF`), UTF-8 BOM (`\xEF\xBB\xBF`), and PDFDocEncoding/Latin-1.
    pub fn decode_pdf_string(&self) -> Option<String> {
        let bytes = match self {
            PdfObject::String(b) | PdfObject::HexString(b) => b.as_slice(),
            PdfObject::Name(b) => return Some(String::from_utf8_lossy(b).into_owned()),
            _ => return None,
        };
        Some(decode_pdf_text_bytes(bytes))
    }

    /// Extracts a 4-element `[x0, y0, x1, y1]` array as `[f64; 4]`.
    pub fn as_rect(&self) -> Option<[f64; 4]> {
        let arr = self.as_array()?;
        if arr.len() != 4 {
            return None;
        }
        Some([
            arr[0].as_f64()?,
            arr[1].as_f64()?,
            arr[2].as_f64()?,
            arr[3].as_f64()?,
        ])
    }
}

/// Decodes raw PDF string bytes according to ISO 32000 text string rules.
pub fn decode_pdf_text_bytes(bytes: &[u8]) -> String {
    if bytes.starts_with(&[0xFE, 0xFF]) {
        let utf16: Vec<u16> = bytes[2..]
            .chunks_exact(2)
            .map(|c| u16::from_be_bytes([c[0], c[1]]))
            .collect();
        String::from_utf16_lossy(&utf16)
    } else if bytes.starts_with(&[0xEF, 0xBB, 0xBF]) {
        String::from_utf8_lossy(&bytes[3..]).into_owned()
    } else if let Ok(utf8) = std::str::from_utf8(bytes) {
        utf8.to_string()
    } else {
        // Fallback to Latin-1 / PDFDocEncoding
        bytes.iter().map(|&b| b as char).collect()
    }
}

/// Extension trait for ergonomic lookups on `PdfDict`.
pub trait PdfDictExt {
    fn get_obj(&self, key: &[u8]) -> Option<&PdfObject>;
    fn get_int(&self, key: &[u8]) -> Option<i64>;
    fn get_f64(&self, key: &[u8]) -> Option<f64>;
    fn get_name(&self, key: &[u8]) -> Option<&[u8]>;
    fn get_name_str(&self, key: &[u8]) -> Option<&str>;
    fn get_array(&self, key: &[u8]) -> Option<&[PdfObject]>;
    fn get_dict(&self, key: &[u8]) -> Option<&PdfDict>;
    fn get_ref(&self, key: &[u8]) -> Option<ObjectId>;
    fn get_string_decoded(&self, key: &[u8]) -> Option<String>;
}

impl PdfDictExt for PdfDict {
    fn get_obj(&self, key: &[u8]) -> Option<&PdfObject> {
        self.get(key)
    }

    fn get_int(&self, key: &[u8]) -> Option<i64> {
        self.get(key).and_then(PdfObject::as_i64)
    }

    fn get_f64(&self, key: &[u8]) -> Option<f64> {
        self.get(key).and_then(PdfObject::as_f64)
    }

    fn get_name(&self, key: &[u8]) -> Option<&[u8]> {
        self.get(key).and_then(PdfObject::as_name)
    }

    fn get_name_str(&self, key: &[u8]) -> Option<&str> {
        self.get(key).and_then(PdfObject::as_name_str)
    }

    fn get_array(&self, key: &[u8]) -> Option<&[PdfObject]> {
        self.get(key).and_then(PdfObject::as_array)
    }

    fn get_dict(&self, key: &[u8]) -> Option<&PdfDict> {
        self.get(key).and_then(PdfObject::as_dict)
    }

    fn get_ref(&self, key: &[u8]) -> Option<ObjectId> {
        self.get(key).and_then(PdfObject::as_reference)
    }

    fn get_string_decoded(&self, key: &[u8]) -> Option<String> {
        self.get(key).and_then(PdfObject::decode_pdf_string)
    }
}

impl TryFrom<&PdfObject> for i64 {
    type Error = PdfruneError;

    fn try_from(value: &PdfObject) -> Result<Self> {
        value
            .as_i64()
            .ok_or_else(|| PdfruneError::InvalidArgument("Expected integer PDF object".into()))
    }
}
