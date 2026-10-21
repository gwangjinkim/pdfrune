use std::io::Read;

use flate2::read::{DeflateDecoder, ZlibDecoder};

use crate::error::{PdfruneError, Result};
use crate::parser::object::{PdfDict, PdfDictExt, PdfObject};

/// Decodes a PDF stream's raw byte payload according to `/Filter` and `/DecodeParms`.
pub fn decode_stream(raw: &[u8], dict: &PdfDict) -> Result<Vec<u8>> {
    let filters = extract_filters(dict);
    if filters.is_empty() {
        return Ok(raw.to_vec());
    }

    let parms_list = extract_decode_parms(dict, filters.len());
    let mut current = raw.to_vec();

    for (idx, filter) in filters.iter().enumerate() {
        let parms = parms_list.get(idx).and_then(|p| *p);
        current = apply_single_filter(&current, filter, parms)?;
    }

    Ok(current)
}

fn extract_filters(dict: &PdfDict) -> Vec<&[u8]> {
    match dict.get(b"Filter".as_slice()) {
        Some(PdfObject::Name(n)) => vec![n.as_slice()],
        Some(PdfObject::Array(arr)) => arr.iter().filter_map(PdfObject::as_name).collect(),
        _ => Vec::new(),
    }
}

fn extract_decode_parms(dict: &PdfDict, count: usize) -> Vec<Option<&PdfDict>> {
    match dict.get(b"DecodeParms".as_slice()) {
        Some(PdfObject::Dict(d)) => vec![Some(d); count],
        Some(PdfObject::Array(arr)) => arr.iter().map(PdfObject::as_dict).collect(),
        _ => vec![None; count],
    }
}

fn apply_single_filter(data: &[u8], filter: &[u8], parms: Option<&PdfDict>) -> Result<Vec<u8>> {
    match filter {
        b"FlateDecode" | b"Fl" => {
            let decompressed = decode_flate(data)?;
            apply_predictor(&decompressed, parms)
        }
        b"ASCIIHexDecode" | b"AHx" => decode_ascii_hex(data),
        b"ASCII85Decode" | b"A85" => decode_ascii85(data),
        b"RunLengthDecode" | b"RL" => decode_run_length(data),
        b"LZWDecode" | b"LZW" => {
            let early_change = parms.and_then(|p| p.get_int(b"EarlyChange")).unwrap_or(1) != 0;
            let decompressed = decode_lzw(data, early_change)?;
            apply_predictor(&decompressed, parms)
        }
        b"Crypt" => Ok(data.to_vec()),
        other => Err(PdfruneError::StreamError {
            filter: String::from_utf8_lossy(other).into_owned(),
            message: "Unsupported stream filter".into(),
        }),
    }
}

/// Decompresses `/FlateDecode` data using zlib, with automatic fallback to raw deflate
/// for streams with missing or non-standard zlib headers.
pub fn decode_flate(data: &[u8]) -> Result<Vec<u8>> {
    // Strip trailing whitespace if needed, try standard Zlib first
    let mut out = Vec::new();
    let mut zlib = ZlibDecoder::new(data);
    if zlib.read_to_end(&mut out).is_ok() && (!out.is_empty() || data.is_empty()) {
        return Ok(out);
    }

    // Fallback: raw Deflate without 2-byte zlib header
    out.clear();
    let mut deflate = DeflateDecoder::new(data);
    if deflate.read_to_end(&mut out).is_ok() {
        return Ok(out);
    }

    // Second fallback: skip 2-byte broken header and raw Deflate
    if data.len() > 2 {
        out.clear();
        let mut deflate_skip = DeflateDecoder::new(&data[2..]);
        if deflate_skip.read_to_end(&mut out).is_ok() && !out.is_empty() {
            return Ok(out);
        }
    }

    Err(PdfruneError::StreamError {
        filter: "FlateDecode".into(),
        message: "Failed to decompress zlib/deflate stream".into(),
    })
}

/// Applies TIFF (Predictor=2) or PNG (Predictor=10..=15) row filters specified in `/DecodeParms`.
fn apply_predictor(data: &[u8], parms: Option<&PdfDict>) -> Result<Vec<u8>> {
    let Some(parms) = parms else {
        return Ok(data.to_vec());
    };
    let predictor = parms.get_int(b"Predictor").unwrap_or(1);
    if predictor <= 1 {
        return Ok(data.to_vec());
    }

    let columns = parms.get_int(b"Columns").unwrap_or(1).max(1) as usize;
    let colors = parms.get_int(b"Colors").unwrap_or(1).max(1) as usize;
    let bits_per_component = parms.get_int(b"BitsPerComponent").unwrap_or(8).max(1) as usize;
    let bytes_per_pixel = (colors * bits_per_component).div_ceil(8);
    let row_bytes = (columns * colors * bits_per_component).div_ceil(8);

    if predictor == 2 {
        // TIFF Predictor 2 (horizontal differencing)
        let mut out = data.to_vec();
        for row in out.chunks_mut(row_bytes) {
            for i in bytes_per_pixel..row.len() {
                row[i] = row[i].wrapping_add(row[i - bytes_per_pixel]);
            }
        }
        return Ok(out);
    }

    if predictor >= 10 {
        // PNG Predictors (10 = None, 11 = Sub, 12 = Up, 13 = Average, 14 = Paeth, 15 = Optimum)
        let stride = row_bytes + 1;
        let mut out = Vec::with_capacity((data.len() / stride) * row_bytes);
        let mut prev_row = vec![0u8; row_bytes];

        for chunk in data.chunks(stride) {
            if chunk.is_empty() {
                break;
            }
            let filter_type = chunk[0];
            let raw_row = &chunk[1..];
            let mut cur_row = vec![0u8; row_bytes];
            let copy_len = raw_row.len().min(row_bytes);
            cur_row[..copy_len].copy_from_slice(&raw_row[..copy_len]);

            match filter_type {
                0 => {} // None
                1 => {
                    // Sub
                    for i in bytes_per_pixel..row_bytes {
                        cur_row[i] = cur_row[i].wrapping_add(cur_row[i - bytes_per_pixel]);
                    }
                }
                2 => {
                    // Up
                    for i in 0..row_bytes {
                        cur_row[i] = cur_row[i].wrapping_add(prev_row[i]);
                    }
                }
                3 => {
                    // Average
                    for i in 0..row_bytes {
                        let left = if i >= bytes_per_pixel {
                            cur_row[i - bytes_per_pixel] as u16
                        } else {
                            0
                        };
                        let up = prev_row[i] as u16;
                        cur_row[i] = cur_row[i].wrapping_add(((left + up) / 2) as u8);
                    }
                }
                4 => {
                    // Paeth
                    for i in 0..row_bytes {
                        let a = if i >= bytes_per_pixel {
                            cur_row[i - bytes_per_pixel]
                        } else {
                            0
                        };
                        let b = prev_row[i];
                        let c = if i >= bytes_per_pixel {
                            prev_row[i - bytes_per_pixel]
                        } else {
                            0
                        };
                        cur_row[i] = cur_row[i].wrapping_add(paeth_predictor(a, b, c));
                    }
                }
                _ => {}
            }

            out.extend_from_slice(&cur_row);
            prev_row = cur_row;
        }
        return Ok(out);
    }

    Ok(data.to_vec())
}

#[inline]
fn paeth_predictor(a: u8, b: u8, c: u8) -> u8 {
    let a_i = a as i32;
    let b_i = b as i32;
    let c_i = c as i32;
    let p = a_i + b_i - c_i;
    let pa = (p - a_i).abs();
    let pb = (p - b_i).abs();
    let pc = (p - c_i).abs();
    if pa <= pb && pa <= pc {
        a
    } else if pb <= pc {
        b
    } else {
        c
    }
}

/// Decodes `/ASCIIHexDecode` data.
pub fn decode_ascii_hex(data: &[u8]) -> Result<Vec<u8>> {
    let mut out = Vec::with_capacity(data.len() / 2);
    let mut high_nibble: Option<u8> = None;

    for &b in data {
        if b == b'>' {
            break;
        }
        if b.is_ascii_whitespace() {
            continue;
        }
        let val = match b {
            b'0'..=b'9' => b - b'0',
            b'a'..=b'f' => b - b'a' + 10,
            b'A'..=b'F' => b - b'A' + 10,
            _ => {
                return Err(PdfruneError::StreamError {
                    filter: "ASCIIHexDecode".into(),
                    message: format!("Invalid hex character: 0x{b:02X}"),
                })
            }
        };
        if let Some(h) = high_nibble.take() {
            out.push((h << 4) | val);
        } else {
            high_nibble = Some(val);
        }
    }

    if let Some(h) = high_nibble {
        out.push(h << 4);
    }

    Ok(out)
}

/// Decodes `/ASCII85Decode` data.
pub fn decode_ascii85(data: &[u8]) -> Result<Vec<u8>> {
    let mut input = data;
    if input.starts_with(b"<~") {
        input = &input[2..];
    }

    let mut out = Vec::with_capacity((input.len() * 4) / 5);
    let mut group = [0u8; 5];
    let mut count = 0usize;

    let mut i = 0;
    while i < input.len() {
        let b = input[i];
        i += 1;

        if b == b'~' {
            break;
        }
        if b.is_ascii_whitespace() {
            continue;
        }
        if b == b'z' {
            if count != 0 {
                return Err(PdfruneError::StreamError {
                    filter: "ASCII85Decode".into(),
                    message: "'z' inside 5-tuple in ASCII85 stream".into(),
                });
            }
            out.extend_from_slice(&[0, 0, 0, 0]);
            continue;
        }
        if !(b'!'..=b'u').contains(&b) {
            return Err(PdfruneError::StreamError {
                filter: "ASCII85Decode".into(),
                message: format!("Invalid ASCII85 byte: 0x{b:02X}"),
            });
        }

        group[count] = b - b'!';
        count += 1;
        if count == 5 {
            let mut val: u64 = 0;
            for &g in &group {
                val = val * 85 + (g as u64);
            }
            let bytes = (val as u32).to_be_bytes();
            out.extend_from_slice(&bytes);
            count = 0;
        }
    }

    if count >= 2 {
        for g in group.iter_mut().skip(count) {
            *g = 84;
        }
        let mut val: u64 = 0;
        for &g in &group {
            val = val * 85 + (g as u64);
        }
        let bytes = (val as u32).to_be_bytes();
        out.extend_from_slice(&bytes[..count - 1]);
    }

    Ok(out)
}

/// Decodes `/RunLengthDecode` data.
pub fn decode_run_length(data: &[u8]) -> Result<Vec<u8>> {
    let mut out = Vec::new();
    let mut i = 0;
    while i < data.len() {
        let length = data[i];
        i += 1;
        if length == 128 {
            break; // EOD
        } else if length < 128 {
            let count = (length as usize) + 1;
            let end = (i + count).min(data.len());
            out.extend_from_slice(&data[i..end]);
            i = end;
        } else {
            let count = 257 - (length as usize);
            if i < data.len() {
                let b = data[i];
                i += 1;
                out.resize(out.len() + count, b);
            }
        }
    }
    Ok(out)
}

/// Decodes `/LZWDecode` data (MSB-first variable bit-width LZW used in PDF).
pub fn decode_lzw(data: &[u8], early_change: bool) -> Result<Vec<u8>> {
    const CLEAR_CODE: u16 = 256;
    const EOD_CODE: u16 = 257;

    let mut out = Vec::new();
    let mut table: Vec<Vec<u8>> = (0..=255).map(|b| vec![b as u8]).collect();
    table.push(Vec::new()); // 256 CLEAR
    table.push(Vec::new()); // 257 EOD

    let mut code_size = 9usize;
    let mut bit_pos = 0usize;
    let mut prev_seq: Option<Vec<u8>> = None;

    let read_bits = |pos: &mut usize, width: usize| -> Option<u16> {
        if *pos + width > data.len() * 8 {
            return None;
        }
        let mut acc = 0u32;
        for _ in 0..width {
            let byte_idx = *pos / 8;
            let bit_idx = 7 - (*pos % 8);
            let bit = (data[byte_idx] >> bit_idx) & 1;
            acc = (acc << 1) | (bit as u32);
            *pos += 1;
        }
        Some(acc as u16)
    };

    while let Some(code) = read_bits(&mut bit_pos, code_size) {
        if code == EOD_CODE {
            break;
        }
        if code == CLEAR_CODE {
            table.truncate(258);
            code_size = 9;
            prev_seq = None;
            continue;
        }

        let entry = if (code as usize) < table.len() {
            table[code as usize].clone()
        } else if code as usize == table.len() {
            if let Some(ref prev) = prev_seq {
                let mut seq = prev.clone();
                seq.push(prev[0]);
                seq
            } else {
                break;
            }
        } else {
            break;
        };

        out.extend_from_slice(&entry);

        if let Some(mut prev) = prev_seq {
            prev.push(entry[0]);
            if table.len() < 4096 {
                table.push(prev);
                let threshold = if early_change {
                    (1usize << code_size) - 1
                } else {
                    1usize << code_size
                };
                if table.len() >= threshold && code_size < 12 {
                    code_size += 1;
                }
            }
        }
        prev_seq = Some(entry);
    }

    Ok(out)
}
