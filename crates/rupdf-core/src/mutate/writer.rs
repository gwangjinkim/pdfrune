use std::collections::BTreeMap;
use std::io::Write;
use std::path::Path;

use flate2::write::ZlibEncoder;
use flate2::Compression;

use crate::document::Document;
use crate::error::Result;
use crate::parser::object::{PdfDict, PdfDictExt, PdfObject, PdfStream};

/// Serializes a `Document` into a valid ISO 32000 PDF byte vector.
pub fn serialize_document(doc: &Document, deflate: bool) -> Result<Vec<u8>> {
    serialize_document_encrypted(doc, deflate, None, None)
}

/// Serializes a `Document` with optional ISO 32000 Standard Security Handler (`user_pw` / `owner_pw`).
pub fn serialize_document_encrypted(
    doc: &Document,
    deflate: bool,
    user_pw: Option<&str>,
    owner_pw: Option<&str>,
) -> Result<Vec<u8>> {
    let mut objects = doc.objects.clone();
    let mut trailer = doc.trailer.clone();

    // Ensure /Root Catalog and /Pages node exist and match `doc.pages`
    let root_id = trailer.get_ref(b"Root").unwrap_or_else(|| {
        let next = objects.keys().next_back().copied().unwrap_or(0) + 1;
        (next, 0)
    });
    trailer.insert(b"Root".to_vec(), PdfObject::Reference(root_id));

    let pages_id = objects
        .get(&root_id.0)
        .and_then(PdfObject::as_dict)
        .and_then(|d| d.get_ref(b"Pages"))
        .unwrap_or_else(|| {
            let next = objects.keys().next_back().copied().unwrap_or(root_id.0) + 1;
            (next, 0)
        });

    // Update or create Catalog
    let mut catalog_dict = objects
        .get(&root_id.0)
        .and_then(PdfObject::as_dict)
        .cloned()
        .unwrap_or_default();
    catalog_dict.insert(b"Type".to_vec(), PdfObject::Name(b"Catalog".to_vec()));
    catalog_dict.insert(b"Pages".to_vec(), PdfObject::Reference(pages_id));
    objects.insert(root_id.0, PdfObject::Dict(catalog_dict));

    // Synchronize each page dictionary with its `Page` state and build `/Kids`
    let mut kids = Vec::with_capacity(doc.pages.len());
    for page in &doc.pages {
        kids.push(PdfObject::Reference(page.object_id));
        if let Some(page_dict) = objects
            .get_mut(&page.object_id.0)
            .and_then(PdfObject::as_dict_mut)
        {
            page_dict.insert(b"Type".to_vec(), PdfObject::Name(b"Page".to_vec()));
            page_dict.insert(b"Parent".to_vec(), PdfObject::Reference(pages_id));
            page_dict.insert(
                b"MediaBox".to_vec(),
                rect_to_pdf_array(page.mediabox.to_array()),
            );
            if page.cropbox != page.mediabox {
                page_dict.insert(
                    b"CropBox".to_vec(),
                    rect_to_pdf_array(page.cropbox.to_array()),
                );
            }
            if page.rotation != 0 {
                page_dict.insert(b"Rotate".to_vec(), PdfObject::Int(page.rotation as i64));
            } else {
                page_dict.remove(b"Rotate".as_slice());
            }
            if !page.resources.is_empty() && !page_dict.contains_key(b"Resources".as_slice()) {
                page_dict.insert(
                    b"Resources".to_vec(),
                    PdfObject::Dict(page.resources.clone()),
                );
            }
        }
    }

    let mut pages_dict = PdfDict::new();
    pages_dict.insert(b"Type".to_vec(), PdfObject::Name(b"Pages".to_vec()));
    pages_dict.insert(b"Kids".to_vec(), PdfObject::Array(kids));
    pages_dict.insert(b"Count".to_vec(), PdfObject::Int(doc.pages.len() as i64));
    objects.insert(pages_id.0, PdfObject::Dict(pages_dict));

    let effective_deflate = if user_pw.is_some() || owner_pw.is_some() {
        // When encrypting, do not apply post-encryption Flate in write_pdf_stream
        false
    } else {
        deflate
    };

    if user_pw.is_some() || owner_pw.is_some() {
        crate::crypto::encrypt_document_objects(
            &mut objects,
            &mut trailer,
            user_pw.unwrap_or(""),
            owner_pw.unwrap_or(""),
        );
    }

    // Write PDF byte stream
    let mut out = Vec::with_capacity(8192);
    let version = if doc.pdf_version.is_empty() {
        "1.7"
    } else {
        &doc.pdf_version
    };
    writeln!(out, "%PDF-{version}")?;
    out.extend_from_slice(b"%\xE2\xE3\xCF\xD3\n");

    let mut offsets: BTreeMap<u32, usize> = BTreeMap::new();
    for (&obj_num, obj) in &objects {
        offsets.insert(obj_num, out.len());
        writeln!(out, "{obj_num} 0 obj")?;
        write_pdf_object(&mut out, obj, effective_deflate)?;
        out.extend_from_slice(b"\nendobj\n");
    }

    let max_obj = offsets.keys().next_back().copied().unwrap_or(0);
    let size = max_obj + 1;
    let xref_offset = out.len();

    writeln!(out, "xref\n0 {size}")?;
    out.extend_from_slice(b"0000000000 65535 f \n");
    for obj_num in 1..size {
        if let Some(&off) = offsets.get(&obj_num) {
            writeln!(out, "{off:010} 00000 n ")?;
        } else {
            out.extend_from_slice(b"0000000000 65535 f \n");
        }
    }

    // Prepare clean trailer dictionary
    let mut clean_trailer = PdfDict::new();
    clean_trailer.insert(b"Size".to_vec(), PdfObject::Int(size as i64));
    clean_trailer.insert(b"Root".to_vec(), PdfObject::Reference(root_id));
    if let Some(info_ref) = trailer.get(b"Info".as_slice()) {
        clean_trailer.insert(b"Info".to_vec(), info_ref.clone());
    }
    if let Some(enc_ref) = trailer.get(b"Encrypt".as_slice()) {
        clean_trailer.insert(b"Encrypt".to_vec(), enc_ref.clone());
    }
    if let Some(id_arr) = trailer.get(b"ID".as_slice()) {
        clean_trailer.insert(b"ID".to_vec(), id_arr.clone());
    }

    out.extend_from_slice(b"trailer\n");
    write_pdf_dict(&mut out, &clean_trailer, false)?;
    write!(out, "\nstartxref\n{xref_offset}\n%%EOF\n")?;

    Ok(out)
}

/// Saves a `Document` to a filesystem path.
pub fn save_document<P: AsRef<Path>>(doc: &Document, path: P, deflate: bool) -> Result<()> {
    let bytes = serialize_document(doc, deflate)?;
    std::fs::write(path, bytes)?;
    Ok(())
}

fn rect_to_pdf_array(r: [f64; 4]) -> PdfObject {
    PdfObject::Array(vec![
        PdfObject::Real(r[0]),
        PdfObject::Real(r[1]),
        PdfObject::Real(r[2]),
        PdfObject::Real(r[3]),
    ])
}

pub fn write_pdf_object<W: Write>(out: &mut W, obj: &PdfObject, deflate: bool) -> Result<()> {
    match obj {
        PdfObject::Null => out.write_all(b"null")?,
        PdfObject::Bool(b) => out.write_all(if *b { b"true" } else { b"false" })?,
        PdfObject::Int(i) => write!(out, "{i}")?,
        PdfObject::Real(r) => {
            if (r.fract()).abs() < 1e-9 {
                write!(out, "{:.1}", *r)?
            } else {
                write!(out, "{r}")?
            }
        }
        PdfObject::String(bytes) => {
            out.write_all(b"(")?;
            for &b in bytes {
                match b {
                    b'(' => out.write_all(b"\\(")?,
                    b')' => out.write_all(b"\\)")?,
                    b'\\' => out.write_all(b"\\\\")?,
                    b'\n' => out.write_all(b"\\n")?,
                    b'\r' => out.write_all(b"\\r")?,
                    b'\t' => out.write_all(b"\\t")?,
                    other => out.write_all(&[other])?,
                }
            }
            out.write_all(b")")?;
        }
        PdfObject::HexString(bytes) => {
            out.write_all(b"<")?;
            for &b in bytes {
                write!(out, "{b:02X}")?;
            }
            out.write_all(b">")?;
        }
        PdfObject::Name(name) => {
            out.write_all(b"/")?;
            for &b in name {
                if !(33..=126).contains(&b)
                    || b == b'#'
                    || b == b'/'
                    || b == b'('
                    || b == b')'
                    || b == b'<'
                    || b == b'>'
                    || b == b'['
                    || b == b']'
                {
                    write!(out, "#{b:02X}")?;
                } else {
                    out.write_all(&[b])?;
                }
            }
        }
        PdfObject::Array(arr) => {
            out.write_all(b"[")?;
            for (idx, item) in arr.iter().enumerate() {
                if idx > 0 {
                    out.write_all(b" ")?;
                }
                write_pdf_object(out, item, deflate)?;
            }
            out.write_all(b"]")?;
        }
        PdfObject::Dict(dict) => {
            write_pdf_dict(out, dict, deflate)?;
        }
        PdfObject::Stream(stm) => {
            write_pdf_stream(out, stm, deflate)?;
        }
        PdfObject::Reference((obj_num, gen_num)) => {
            write!(out, "{obj_num} {gen_num} R")?;
        }
    }
    Ok(())
}

fn write_pdf_dict<W: Write>(out: &mut W, dict: &PdfDict, deflate: bool) -> Result<()> {
    out.write_all(b"<<")?;
    for (k, v) in dict {
        out.write_all(b" ")?;
        write_pdf_object(out, &PdfObject::Name(k.clone()), deflate)?;
        out.write_all(b" ")?;
        write_pdf_object(out, v, deflate)?;
    }
    out.write_all(b" >>")?;
    Ok(())
}

fn write_pdf_stream<W: Write>(out: &mut W, stm: &PdfStream, deflate: bool) -> Result<()> {
    let mut dict = stm.dict.clone();
    let payload = if deflate && !dict.contains_key(b"Filter".as_slice()) {
        let mut encoder = ZlibEncoder::new(Vec::new(), Compression::default());
        encoder.write_all(&stm.data)?;
        let compressed = encoder.finish()?;
        dict.insert(b"Filter".to_vec(), PdfObject::Name(b"FlateDecode".to_vec()));
        compressed
    } else {
        stm.data.clone()
    };

    dict.insert(b"Length".to_vec(), PdfObject::Int(payload.len() as i64));
    write_pdf_dict(out, &dict, deflate)?;
    out.write_all(b"\nstream\n")?;
    out.write_all(&payload)?;
    out.write_all(b"\nendstream")?;
    Ok(())
}
