use std::collections::BTreeMap;

use memchr::memmem;

use crate::error::{Result, PdfruneError};
use crate::parser::lexer::{parse_ascii_u16, parse_ascii_u32, Lexer};
use crate::parser::object::{PdfDict, PdfDictExt, PdfObject};

/// Location of an indirect PDF object in the cross-reference table.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum XrefEntry {
    /// Uncompressed object at a byte offset in the file.
    Uncompressed { offset: usize, generation: u16 },
    /// Compressed object stored inside an `/ObjStm` stream object at index `index`.
    Compressed { stream_obj_num: u32, index: usize },
}

/// Complete cross-reference table and merged trailer dictionary for a PDF document.
#[derive(Debug, Clone, Default)]
pub struct XrefTable {
    pub entries: BTreeMap<u32, XrefEntry>,
    pub trailer: PdfDict,
    pub pdf_version: String,
}

/// Parses the PDF version header, cross-reference tables/streams (following `/Prev`),
/// and trailer dictionary. Automatically falls back to a full-file scan if damaged.
pub fn parse_xref_and_trailer(data: &[u8]) -> Result<XrefTable> {
    let pdf_version = extract_pdf_header_version(data)?;

    if let Ok(table) = parse_standard_xref_chain(data, &pdf_version) {
        if !table.entries.is_empty() && table.trailer.contains_key(b"Root".as_slice()) {
            return Ok(table);
        }
    }

    // Fallback: repair by scanning the entire file for indirect objects and trailer
    repair_xref_by_scanning(data, &pdf_version)
}

fn extract_pdf_header_version(data: &[u8]) -> Result<String> {
    let search_window = &data[..data.len().min(1024)];
    let Some(pos) = memmem::find(search_window, b"%PDF-") else {
        return Err(PdfruneError::InvalidFormat(
            "Missing '%PDF-' header signature".into(),
        ));
    };
    let ver_start = pos + 5;
    let ver_end = (ver_start + 3).min(data.len());
    let ver = String::from_utf8_lossy(&data[ver_start..ver_end]).into_owned();
    Ok(ver)
}

fn find_startxref_offset(data: &[u8]) -> Result<usize> {
    let tail_start = data.len().saturating_sub(4096);
    let tail = &data[tail_start..];
    let Some(rel_pos) = memmem::rfind(tail, b"startxref") else {
        return Err(PdfruneError::XrefError("Missing 'startxref' marker".into()));
    };
    let mut lexer = Lexer::with_offset(data, tail_start + rel_pos + 9);
    let Some(offset_word) = lexer.read_word() else {
        return Err(PdfruneError::XrefError(
            "Missing byte offset after 'startxref'".into(),
        ));
    };
    let offset = std::str::from_utf8(offset_word)
        .ok()
        .and_then(|s| s.parse::<usize>().ok())
        .ok_or_else(|| PdfruneError::XrefError("Invalid integer after 'startxref'".into()))?;
    Ok(offset)
}

fn parse_standard_xref_chain(data: &[u8], pdf_version: &str) -> Result<XrefTable> {
    let mut current_offset = find_startxref_offset(data)?;
    let mut visited = std::collections::BTreeSet::new();
    let mut merged_entries = BTreeMap::new();
    let mut merged_trailer = PdfDict::new();

    while !visited.contains(&current_offset) && current_offset < data.len() {
        visited.insert(current_offset);

        let mut lexer = Lexer::with_offset(data, current_offset);
        lexer.skip_whitespace_and_comments();

        let (entries, trailer) = if lexer.remaining().starts_with(b"xref") {
            parse_ascii_xref_section(&mut lexer)?
        } else {
            parse_xref_stream_object(&mut lexer)?
        };

        // Newer revisions take precedence over older (/Prev) revisions
        for (obj_num, entry) in entries {
            merged_entries.entry(obj_num).or_insert(entry);
        }
        for (k, v) in &trailer {
            merged_trailer.entry(k.clone()).or_insert_with(|| v.clone());
        }

        // Check if hybrid XRefStm exists in this trailer
        if let Some(stm_offset) = trailer.get_int(b"XRefStm") {
            if stm_offset > 0 && (stm_offset as usize) < data.len() {
                let mut stm_lexer = Lexer::with_offset(data, stm_offset as usize);
                if let Ok((stm_entries, _)) = parse_xref_stream_object(&mut stm_lexer) {
                    for (obj_num, entry) in stm_entries {
                        merged_entries.entry(obj_num).or_insert(entry);
                    }
                }
            }
        }

        if let Some(prev) = trailer.get_int(b"Prev") {
            if prev >= 0 {
                current_offset = prev as usize;
                continue;
            }
        }
        break;
    }

    Ok(XrefTable {
        entries: merged_entries,
        trailer: merged_trailer,
        pdf_version: pdf_version.to_string(),
    })
}

fn parse_ascii_xref_section(lexer: &mut Lexer<'_>) -> Result<(BTreeMap<u32, XrefEntry>, PdfDict)> {
    let kw = lexer.read_word();
    if kw != Some(b"xref") {
        return Err(PdfruneError::XrefError("Expected 'xref' keyword".into()));
    }

    let mut entries = BTreeMap::new();

    loop {
        lexer.skip_whitespace_and_comments();
        if lexer.remaining().starts_with(b"trailer") {
            let _ = lexer.read_word(); // consume "trailer"
            break;
        }
        let Some(first_word) = lexer.read_word() else {
            break;
        };
        if first_word == b"trailer" {
            break;
        }
        let Some(count_word) = lexer.read_word() else {
            break;
        };
        let start_obj = parse_ascii_u32(first_word)
            .map_err(|_| PdfruneError::XrefError("Invalid subsection start".into()))?;
        let count = parse_ascii_u32(count_word)
            .map_err(|_| PdfruneError::XrefError("Invalid subsection count".into()))?;

        for i in 0..count {
            let Some(off_w) = lexer.read_word() else {
                break;
            };
            let Some(gen_w) = lexer.read_word() else {
                break;
            };
            let Some(flag_w) = lexer.read_word() else {
                break;
            };

            if flag_w == b"n" {
                if let (Ok(offset), Ok(generation)) = (
                    std::str::from_utf8(off_w).unwrap_or("0").parse::<usize>(),
                    parse_ascii_u16(gen_w),
                ) {
                    if offset > 0 {
                        entries
                            .entry(start_obj + i)
                            .or_insert(XrefEntry::Uncompressed { offset, generation });
                    }
                }
            }
        }
    }

    lexer.skip_whitespace_and_comments();
    let trailer = lexer.parse_dict()?;
    Ok((entries, trailer))
}

fn parse_xref_stream_object(lexer: &mut Lexer<'_>) -> Result<(BTreeMap<u32, XrefEntry>, PdfDict)> {
    let (_, obj) = lexer.parse_indirect_object()?;
    let Some(stream) = obj.as_stream() else {
        return Err(PdfruneError::XrefError(
            "Expected stream object at XREF offset".into(),
        ));
    };

    let dict = &stream.dict;
    let w_arr = dict
        .get_array(b"W")
        .ok_or_else(|| PdfruneError::XrefError("Missing /W array in XRef stream".into()))?;
    if w_arr.len() != 3 {
        return Err(PdfruneError::XrefError("Invalid /W array length".into()));
    }
    let w0 = w_arr[0].as_i64().unwrap_or(0).max(0) as usize;
    let w1 = w_arr[1].as_i64().unwrap_or(0).max(0) as usize;
    let w2 = w_arr[2].as_i64().unwrap_or(0).max(0) as usize;
    let entry_size = w0 + w1 + w2;
    if entry_size == 0 {
        return Err(PdfruneError::XrefError(
            "Zero entry size in XRef stream".into(),
        ));
    }

    let size = dict.get_int(b"Size").unwrap_or(0).max(0) as u32;
    let subsections: Vec<(u32, u32)> = if let Some(idx_arr) = dict.get_array(b"Index") {
        idx_arr
            .chunks_exact(2)
            .filter_map(|c| Some((c[0].as_i64()? as u32, c[1].as_i64()? as u32)))
            .collect()
    } else {
        vec![(0, size)]
    };

    let decoded = stream.decode_data()?;
    let mut entries = BTreeMap::new();
    let mut cursor = 0usize;

    for (start_obj, count) in subsections {
        for i in 0..count {
            if cursor + entry_size > decoded.len() {
                break;
            }
            let field_type = if w0 == 0 {
                1u64 // Default type is 1 per ISO 32000-1 Table 18
            } else {
                read_be_uint(&decoded[cursor..cursor + w0])
            };
            let field2 = read_be_uint(&decoded[cursor + w0..cursor + w0 + w1]);
            let field3 = read_be_uint(&decoded[cursor + w0 + w1..cursor + entry_size]);
            cursor += entry_size;

            let obj_num = start_obj + i;
            match field_type {
                1 => {
                    if field2 > 0 {
                        entries.insert(
                            obj_num,
                            XrefEntry::Uncompressed {
                                offset: field2 as usize,
                                generation: field3 as u16,
                            },
                        );
                    }
                }
                2 => {
                    entries.insert(
                        obj_num,
                        XrefEntry::Compressed {
                            stream_obj_num: field2 as u32,
                            index: field3 as usize,
                        },
                    );
                }
                _ => {}
            }
        }
    }

    Ok((entries, dict.clone()))
}

#[inline]
fn read_be_uint(bytes: &[u8]) -> u64 {
    let mut val = 0u64;
    for &b in bytes {
        val = (val << 8) | (b as u64);
    }
    val
}

/// Fallback scanner that reconstructs the XREF table and trailer from raw bytes
/// when a PDF has a broken or missing cross-reference table.
pub fn repair_xref_by_scanning(data: &[u8], pdf_version: &str) -> Result<XrefTable> {
    let mut entries = BTreeMap::new();
    let mut trailer = PdfDict::new();

    for pos in memmem::find_iter(data, b" obj") {
        // Scan backward to find the start of `obj_num gen_num obj`
        let lookback_start = pos.saturating_sub(32);
        let slice = &data[lookback_start..pos + 4];
        let mut i = slice.len().saturating_sub(4);
        // Skip whitespace before `obj`
        while i > 0 && Lexer::is_pdf_whitespace(slice[i - 1]) {
            i -= 1;
        }
        // Read gen_num digits
        let gen_end = i;
        while i > 0 && slice[i - 1].is_ascii_digit() {
            i -= 1;
        }
        let gen_start = i;
        if gen_start == gen_end {
            continue;
        }
        // Skip whitespace before gen_num
        while i > 0 && Lexer::is_pdf_whitespace(slice[i - 1]) {
            i -= 1;
        }
        let obj_end = i;
        while i > 0 && slice[i - 1].is_ascii_digit() {
            i -= 1;
        }
        let obj_start = i;
        if obj_start == obj_end {
            continue;
        }
        if obj_start > 0 && Lexer::is_regular_char(slice[obj_start - 1]) {
            continue;
        }

        if let (Ok(obj_num), Ok(generation)) = (
            parse_ascii_u32(&slice[obj_start..obj_end]),
            parse_ascii_u16(&slice[gen_start..gen_end]),
        ) {
            let offset = lookback_start + obj_start;
            entries.insert(obj_num, XrefEntry::Uncompressed { offset, generation });

            // Check if this object is the Catalog (/Type /Catalog)
            let mut lexer = Lexer::with_offset(data, offset);
            if let Ok((_, obj)) = lexer.parse_indirect_object() {
                if let Some(d) = obj.as_dict() {
                    if d.get_name(b"Type") == Some(b"Catalog") {
                        trailer.insert(
                            b"Root".to_vec(),
                            PdfObject::Reference((obj_num, generation)),
                        );
                    } else if d.get_name(b"Type") == Some(b"XRef") {
                        for (k, v) in d {
                            trailer.entry(k.clone()).or_insert_with(|| v.clone());
                        }
                    }
                }
            }
        }
    }

    // Also scan for explicit `trailer` keywords
    for pos in memmem::find_iter(data, b"trailer") {
        let mut lexer = Lexer::with_offset(data, pos + 7);
        lexer.skip_whitespace_and_comments();
        if lexer.remaining().starts_with(b"<<") {
            if let Ok(d) = lexer.parse_dict() {
                for (k, v) in d {
                    trailer.insert(k, v);
                }
            }
        }
    }

    if entries.is_empty() {
        return Err(PdfruneError::XrefError(
            "Unable to locate any valid PDF objects in document".into(),
        ));
    }

    Ok(XrefTable {
        entries,
        trailer,
        pdf_version: pdf_version.to_string(),
    })
}
