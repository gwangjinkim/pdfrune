use std::collections::BTreeMap;

use crate::error::{PdfruneError, Result};
use crate::parser::lexer::{parse_ascii_u32, Lexer};
use crate::parser::object::{PdfDict, PdfDictExt, PdfObject, PdfStream};

/// Unpacks all objects stored inside a PDF 1.5+ `/ObjStm` compressed object stream.
pub fn unpack_object_stream(stream: &PdfStream) -> Result<BTreeMap<u32, PdfObject>> {
    let n = stream
        .dict
        .get_int(b"N")
        .ok_or_else(|| PdfruneError::XrefError("Missing /N in /ObjStm".into()))?
        .max(0) as usize;
    let first = stream
        .dict
        .get_int(b"First")
        .ok_or_else(|| PdfruneError::XrefError("Missing /First in /ObjStm".into()))?
        .max(0) as usize;

    let decoded = stream.decode_data()?;
    let mut header_lexer = Lexer::new(&decoded[..first.min(decoded.len())]);
    let mut pairs = Vec::with_capacity(n);

    for _ in 0..n {
        let Some(obj_w) = header_lexer.read_word() else {
            break;
        };
        let Some(off_w) = header_lexer.read_word() else {
            break;
        };
        if let (Ok(obj_num), Ok(rel_off)) = (
            parse_ascii_u32(obj_w),
            std::str::from_utf8(off_w).unwrap_or("0").parse::<usize>(),
        ) {
            pairs.push((obj_num, first + rel_off));
        }
    }

    let mut unpacked = BTreeMap::new();
    for (obj_num, abs_offset) in pairs {
        if abs_offset < decoded.len() {
            let mut obj_lexer = Lexer::with_offset(&decoded, abs_offset);
            if let Ok(obj) = obj_lexer.parse_object() {
                unpacked.insert(obj_num, obj);
            }
        }
    }

    Ok(unpacked)
}

/// Extracts standardized document metadata from an `/Info` dictionary.
pub fn extract_metadata_map(
    info_dict: Option<&PdfDict>,
    pdf_version: &str,
) -> BTreeMap<String, String> {
    let mut meta = BTreeMap::new();
    meta.insert("format".to_string(), format!("PDF {pdf_version}"));

    let keys: &[(&str, &[u8])] = &[
        ("title", b"Title"),
        ("author", b"Author"),
        ("subject", b"Subject"),
        ("keywords", b"Keywords"),
        ("creator", b"Creator"),
        ("producer", b"Producer"),
        ("creationDate", b"CreationDate"),
        ("modDate", b"ModDate"),
        ("trapped", b"Trapped"),
    ];

    for &(out_key, pdf_key) in keys {
        let val = info_dict
            .and_then(|d| d.get_string_decoded(pdf_key))
            .unwrap_or_default();
        meta.insert(out_key.to_string(), val);
    }

    meta
}
