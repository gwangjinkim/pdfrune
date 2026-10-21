#![allow(clippy::not_unsafe_ptr_arg_deref)]

use std::collections::BTreeMap;
use std::ffi::{CStr, CString};
use std::os::raw::c_char;
use std::ptr;

use pdfrune::{Document, Point, Rect};

// Helper to convert C string pointer to &str
unsafe fn c_to_str<'a>(p: *const c_char) -> &'a str {
    if p.is_null() {
        ""
    } else {
        CStr::from_ptr(p).to_str().unwrap_or("")
    }
}

// Helper to convert String to raw C string pointer
fn to_c_string(s: impl Into<String>) -> *mut c_char {
    let s = s.into();
    CString::new(s).unwrap_or_default().into_raw()
}

#[no_mangle]
pub extern "C" fn pdfrune_c_version() -> *mut c_char {
    to_c_string(pdfrune::version())
}

#[no_mangle]
pub extern "C" fn pdfrune_c_free_string(s: *mut c_char) {
    if !s.is_null() {
        unsafe {
            let _ = CString::from_raw(s);
        }
    }
}

#[no_mangle]
pub extern "C" fn pdfrune_c_free_bytes(b: *mut u8, len: usize) {
    if !b.is_null() && len > 0 {
        unsafe {
            let _ = Vec::from_raw_parts(b, len, len);
        }
    }
}

#[no_mangle]
pub extern "C" fn pdfrune_c_new() -> *mut Document {
    let doc = Document::new();
    Box::into_raw(Box::new(doc))
}

#[no_mangle]
pub extern "C" fn pdfrune_c_open(bytes: *const u8, len: usize) -> *mut Document {
    if bytes.is_null() || len == 0 {
        return ptr::null_mut();
    }
    let slice = unsafe { std::slice::from_raw_parts(bytes, len) };
    match Document::from_bytes(slice) {
        Ok(doc) => Box::into_raw(Box::new(doc)),
        Err(_) => ptr::null_mut(),
    }
}

#[no_mangle]
pub extern "C" fn pdfrune_c_open_bytes(bytes: *const u8, len: usize) -> *mut Document {
    pdfrune_c_open(bytes, len)
}

#[no_mangle]
pub extern "C" fn pdfrune_c_open_path(path: *const c_char) -> *mut Document {
    if path.is_null() {
        return ptr::null_mut();
    }
    let p = unsafe { c_to_str(path) };
    match Document::open(p) {
        Ok(doc) => Box::into_raw(Box::new(doc)),
        Err(_) => ptr::null_mut(),
    }
}

#[no_mangle]
pub extern "C" fn pdfrune_c_free(doc: *mut Document) {
    if !doc.is_null() {
        unsafe {
            drop(Box::from_raw(doc));
        }
    }
}

#[no_mangle]
pub extern "C" fn pdfrune_c_page_count(doc: *mut Document) -> i32 {
    if doc.is_null() {
        return -1;
    }
    let d = unsafe { &*doc };
    d.page_count() as i32
}

#[no_mangle]
pub extern "C" fn pdfrune_c_is_encrypted(doc: *mut Document) -> i32 {
    if doc.is_null() {
        return 0;
    }
    let d = unsafe { &*doc };
    if d.is_encrypted {
        1
    } else {
        0
    }
}

#[no_mangle]
pub extern "C" fn pdfrune_c_authenticate(doc: *mut Document, password: *const c_char) -> i32 {
    if doc.is_null() {
        return 0;
    }
    let d = unsafe { &mut *doc };
    let pw = unsafe { c_to_str(password) };
    match d.authenticate(pw) {
        Ok(true) => 1,
        _ => 0,
    }
}

#[no_mangle]
pub extern "C" fn pdfrune_c_get_metadata_json(doc: *mut Document) -> *mut c_char {
    if doc.is_null() {
        return ptr::null_mut();
    }
    let d = unsafe { &*doc };
    let meta = d.metadata();
    let json = serde_json::to_string(&meta).unwrap_or_else(|_| "{}".to_string());
    to_c_string(json)
}

#[no_mangle]
pub extern "C" fn pdfrune_c_metadata_json(doc: *mut Document) -> *mut c_char {
    pdfrune_c_get_metadata_json(doc)
}

#[no_mangle]
pub extern "C" fn pdfrune_c_set_metadata(
    doc: *mut Document,
    title: *const c_char,
    author: *const c_char,
    subject: *const c_char,
    keywords: *const c_char,
    creator: *const c_char,
    producer: *const c_char,
) -> i32 {
    if doc.is_null() {
        return 0;
    }
    let d = unsafe { &mut *doc };
    let mut current: BTreeMap<String, String> = d.metadata();
    let t = unsafe { c_to_str(title) };
    let a = unsafe { c_to_str(author) };
    let s = unsafe { c_to_str(subject) };
    let k = unsafe { c_to_str(keywords) };
    let c = unsafe { c_to_str(creator) };
    let p = unsafe { c_to_str(producer) };
    if !t.is_empty() {
        current.insert("title".to_string(), t.to_string());
    }
    if !a.is_empty() {
        current.insert("author".to_string(), a.to_string());
    }
    if !s.is_empty() {
        current.insert("subject".to_string(), s.to_string());
    }
    if !k.is_empty() {
        current.insert("keywords".to_string(), k.to_string());
    }
    if !c.is_empty() {
        current.insert("creator".to_string(), c.to_string());
    }
    if !p.is_empty() {
        current.insert("producer".to_string(), p.to_string());
    }
    d.set_metadata(&current);
    1
}

#[no_mangle]
pub extern "C" fn pdfrune_c_set_metadata_entry(
    doc: *mut Document,
    key: *const c_char,
    val: *const c_char,
) -> i32 {
    if doc.is_null() {
        return 0;
    }
    let d = unsafe { &mut *doc };
    let k = unsafe { c_to_str(key) };
    let v = unsafe { c_to_str(val) };
    let mut current: BTreeMap<String, String> = d.metadata();
    current.insert(k.to_string(), v.to_string());
    d.set_metadata(&current);
    1
}

#[no_mangle]
pub extern "C" fn pdfrune_c_page_info_json(doc: *mut Document, page: usize) -> *mut c_char {
    if doc.is_null() {
        return ptr::null_mut();
    }
    let d = unsafe { &*doc };
    match d.page(page) {
        Ok(p) => {
            let val = serde_json::json!({
                "page_index": p.index,
                "width": p.width(),
                "height": p.height(),
                "rotation": p.rotation,
                "mediabox": [p.mediabox.x0, p.mediabox.y0, p.mediabox.x1, p.mediabox.y1],
                "cropbox": [p.cropbox.x0, p.cropbox.y0, p.cropbox.x1, p.cropbox.y1],
            });
            to_c_string(val.to_string())
        }
        Err(_) => ptr::null_mut(),
    }
}

#[no_mangle]
pub extern "C" fn pdfrune_c_extract_text(doc: *mut Document, page: usize) -> *mut c_char {
    if doc.is_null() {
        return ptr::null_mut();
    }
    let d = unsafe { &*doc };
    match d.extract_page_text(page) {
        Ok(t) => to_c_string(t),
        Err(_) => to_c_string(""),
    }
}

#[no_mangle]
pub extern "C" fn pdfrune_c_extract_words_json(doc: *mut Document, page: usize) -> *mut c_char {
    if doc.is_null() {
        return ptr::null_mut();
    }
    let d = unsafe { &*doc };
    match d.page_layout(page) {
        Ok(layout) => {
            let words = layout.words();
            let list: Vec<serde_json::Value> = words
                .into_iter()
                .map(|w| {
                    serde_json::json!({
                        "x0": w.rect.x0,
                        "y0": w.rect.y0,
                        "x1": w.rect.x1,
                        "y1": w.rect.y1,
                        "word": w.text,
                        "block_no": w.block_no,
                        "line_no": w.line_no,
                        "word_no": w.word_no,
                    })
                })
                .collect();
            let json = serde_json::to_string(&list).unwrap_or_else(|_| "[]".to_string());
            to_c_string(json)
        }
        Err(_) => to_c_string("[]"),
    }
}

#[no_mangle]
pub extern "C" fn pdfrune_c_extract_blocks_json(doc: *mut Document, page: usize) -> *mut c_char {
    if doc.is_null() {
        return ptr::null_mut();
    }
    let d = unsafe { &*doc };
    match d.page_layout(page) {
        Ok(layout) => {
            let blocks = layout.blocks;
            let list: Vec<serde_json::Value> = blocks
                .into_iter()
                .map(|b| {
                    serde_json::json!({
                        "x0": b.rect.x0,
                        "y0": b.rect.y0,
                        "x1": b.rect.x1,
                        "y1": b.rect.y1,
                        "text": b.text(),
                        "block_no": b.block_no,
                        "block_type": b.block_type,
                    })
                })
                .collect();
            let json = serde_json::to_string(&list).unwrap_or_else(|_| "[]".to_string());
            to_c_string(json)
        }
        Err(_) => to_c_string("[]"),
    }
}

#[no_mangle]
pub extern "C" fn pdfrune_c_search_page_json(
    doc: *mut Document,
    page: usize,
    needle: *const c_char,
) -> *mut c_char {
    if doc.is_null() {
        return ptr::null_mut();
    }
    let d = unsafe { &*doc };
    let q = unsafe { c_to_str(needle) };
    match d.search_page(page, q) {
        Ok(rects) => {
            let list: Vec<serde_json::Value> = rects
                .into_iter()
                .map(|r| serde_json::json!({ "x0": r.x0, "y0": r.y0, "x1": r.x1, "y1": r.y1 }))
                .collect();
            let json = serde_json::to_string(&list).unwrap_or_else(|_| "[]".to_string());
            to_c_string(json)
        }
        Err(_) => to_c_string("[]"),
    }
}

#[no_mangle]
pub extern "C" fn pdfrune_c_search_all_json(
    doc: *mut Document,
    needle: *const c_char,
    regex: i32,
) -> *mut c_char {
    if doc.is_null() {
        return ptr::null_mut();
    }
    let d = unsafe { &*doc };
    let q = unsafe { c_to_str(needle) };
    match d.search_all(q, regex != 0) {
        Ok(results) => {
            let mut list = Vec::new();
            for (page_idx, rects) in results {
                for r in rects {
                    list.push(serde_json::json!({
                        "page": page_idx + 1, // 1-based for R
                        "x0": r.x0,
                        "y0": r.y0,
                        "x1": r.x1,
                        "y1": r.y1,
                    }));
                }
            }
            let json = serde_json::to_string(&list).unwrap_or_else(|_| "[]".to_string());
            to_c_string(json)
        }
        Err(_) => to_c_string("[]"),
    }
}

#[no_mangle]
pub extern "C" fn pdfrune_c_find_tables_json(doc: *mut Document, page: usize) -> *mut c_char {
    if doc.is_null() {
        return ptr::null_mut();
    }
    let d = unsafe { &*doc };
    match d.find_page_tables(page) {
        Ok(tables) => {
            let list: Vec<serde_json::Value> = tables
                .into_iter()
                .map(|t| {
                    serde_json::json!({
                        "row_count": t.row_count(),
                        "col_count": t.col_count(),
                        "bbox": [t.bbox.x0, t.bbox.y0, t.bbox.x1, t.bbox.y1],
                        "header": t.header(),
                        "rows": t.extract(),
                        "markdown": t.to_markdown(),
                    })
                })
                .collect();
            let json = serde_json::to_string(&list).unwrap_or_else(|_| "[]".to_string());
            to_c_string(json)
        }
        Err(_) => to_c_string("[]"),
    }
}

#[no_mangle]
pub extern "C" fn pdfrune_c_page_to_markdown(doc: *mut Document, page: usize) -> *mut c_char {
    if doc.is_null() {
        return ptr::null_mut();
    }
    let d = unsafe { &*doc };
    match d.page_to_markdown(page) {
        Ok(md) => to_c_string(md),
        Err(_) => to_c_string(""),
    }
}

#[no_mangle]
pub extern "C" fn pdfrune_c_doc_to_markdown(doc: *mut Document) -> *mut c_char {
    if doc.is_null() {
        return ptr::null_mut();
    }
    let d = unsafe { &*doc };
    match d.to_markdown() {
        Ok(md) => to_c_string(md),
        Err(_) => to_c_string(""),
    }
}

#[no_mangle]
pub extern "C" fn pdfrune_c_page_to_html(doc: *mut Document, page: usize) -> *mut c_char {
    if doc.is_null() {
        return ptr::null_mut();
    }
    let d = unsafe { &*doc };
    match d.page_to_html(page) {
        Ok(h) => to_c_string(h),
        Err(_) => to_c_string(""),
    }
}

#[no_mangle]
pub extern "C" fn pdfrune_c_doc_to_html(doc: *mut Document) -> *mut c_char {
    if doc.is_null() {
        return ptr::null_mut();
    }
    let d = unsafe { &*doc };
    match d.to_html() {
        Ok(h) => to_c_string(h),
        Err(_) => to_c_string(""),
    }
}

#[no_mangle]
pub extern "C" fn pdfrune_c_to_rag_chunks_json(
    doc: *mut Document,
    max_chars: usize,
) -> *mut c_char {
    if doc.is_null() {
        return ptr::null_mut();
    }
    let d = unsafe { &*doc };
    match d.to_rag_chunks(max_chars) {
        Ok(chunks) => {
            let list: Vec<serde_json::Value> = chunks
                .into_iter()
                .map(|c| {
                    serde_json::json!({
                        "chunk_id": c.chunk_id,
                        "page_number": c.page_number,
                        "x0": c.bbox.x0,
                        "y0": c.bbox.y0,
                        "x1": c.bbox.x1,
                        "y1": c.bbox.y1,
                        "text": c.text,
                        "char_count": c.char_count,
                        "section_heading": c.section_heading,
                    })
                })
                .collect();
            let json = serde_json::to_string(&list).unwrap_or_else(|_| "[]".to_string());
            to_c_string(json)
        }
        Err(_) => to_c_string("[]"),
    }
}

#[no_mangle]
pub extern "C" fn pdfrune_c_page_to_svg(doc: *mut Document, page: usize) -> *mut c_char {
    if doc.is_null() {
        return ptr::null_mut();
    }
    let d = unsafe { &*doc };
    match d.render_page_svg(page) {
        Ok(svg) => to_c_string(svg),
        Err(_) => to_c_string(""),
    }
}

#[no_mangle]
pub extern "C" fn pdfrune_c_render_page_rgba(
    doc: *mut Document,
    page: usize,
    dpi: f64,
    out_w: *mut i32,
    out_h: *mut i32,
    out_stride: *mut i32,
    out_len: *mut usize,
) -> *mut u8 {
    if doc.is_null() {
        return ptr::null_mut();
    }
    let d = unsafe { &*doc };
    match d.render_page(page, dpi) {
        Ok(pix) => {
            if !out_w.is_null() {
                unsafe { *out_w = pix.width as i32 };
            }
            if !out_h.is_null() {
                unsafe { *out_h = pix.height as i32 };
            }
            if !out_stride.is_null() {
                unsafe { *out_stride = pix.stride() as i32 };
            }
            if !out_len.is_null() {
                unsafe { *out_len = pix.data.len() };
            }
            let mut v = pix.data;
            v.shrink_to_fit();
            let p = v.as_mut_ptr();
            std::mem::forget(v);
            p
        }
        Err(_) => ptr::null_mut(),
    }
}

#[no_mangle]
pub extern "C" fn pdfrune_c_render_page_png(
    doc: *mut Document,
    page: usize,
    dpi: f64,
    out_len: *mut usize,
) -> *mut u8 {
    if doc.is_null() {
        return ptr::null_mut();
    }
    let d = unsafe { &*doc };
    match d.render_page(page, dpi).and_then(|pix| pix.to_png()) {
        Ok(mut v) => {
            if !out_len.is_null() {
                unsafe { *out_len = v.len() };
            }
            v.shrink_to_fit();
            let p = v.as_mut_ptr();
            std::mem::forget(v);
            p
        }
        Err(_) => ptr::null_mut(),
    }
}

#[no_mangle]
pub extern "C" fn pdfrune_c_pixmap_to_png(
    data: *const u8,
    data_len: usize,
    width: u32,
    height: u32,
    out_len: *mut usize,
) -> *mut u8 {
    if data.is_null() || data_len == 0 {
        return ptr::null_mut();
    }
    let slice = unsafe { std::slice::from_raw_parts(data, data_len) };
    let pix = pdfrune::Pixmap {
        width,
        height,
        data: slice.to_vec(),
    };
    match pix.to_png() {
        Ok(mut v) => {
            if !out_len.is_null() {
                unsafe { *out_len = v.len() };
            }
            v.shrink_to_fit();
            let p = v.as_mut_ptr();
            std::mem::forget(v);
            p
        }
        Err(_) => ptr::null_mut(),
    }
}

#[no_mangle]
pub extern "C" fn pdfrune_c_get_toc_json(doc: *mut Document) -> *mut c_char {
    if doc.is_null() {
        return ptr::null_mut();
    }
    let d = unsafe { &*doc };
    let toc = d.get_toc();
    let list: Vec<serde_json::Value> = toc
        .into_iter()
        .map(|e| {
            serde_json::json!({
                "level": e.level,
                "title": e.title,
                "page": e.page,
            })
        })
        .collect();
    let json = serde_json::to_string(&list).unwrap_or_else(|_| "[]".to_string());
    to_c_string(json)
}

#[derive(serde::Deserialize)]
struct RJsonTocEntry {
    level: usize,
    title: String,
    page: usize,
}

#[no_mangle]
pub extern "C" fn pdfrune_c_set_toc_json(doc: *mut Document, json_str: *const c_char) -> i32 {
    if doc.is_null() {
        return 0;
    }
    let d = unsafe { &mut *doc };
    let s = unsafe { c_to_str(json_str) };
    if let Ok(entries) = serde_json::from_str::<Vec<RJsonTocEntry>>(s) {
        let mapped: Vec<pdfrune::nav::TocEntry> = entries
            .into_iter()
            .map(|e| pdfrune::nav::TocEntry {
                level: e.level,
                title: e.title,
                page: e.page,
            })
            .collect();
        match d.set_toc(&mapped) {
            Ok(_) => 1,
            Err(_) => 0,
        }
    } else {
        0
    }
}

#[no_mangle]
pub extern "C" fn pdfrune_c_get_links_json(doc: *mut Document, page: usize) -> *mut c_char {
    if doc.is_null() {
        return ptr::null_mut();
    }
    let d = unsafe { &*doc };
    match d.get_page_links(page) {
        Ok(links) => {
            let list: Vec<serde_json::Value> = links
                .into_iter()
                .map(|l| {
                    serde_json::json!({
                        "kind": l.kind,
                        "uri": l.uri,
                        "page": l.page.map(|p| p + 1),
                        "x0": l.from.x0,
                        "y0": l.from.y0,
                        "x1": l.from.x1,
                        "y1": l.from.y1,
                        "xref": l.xref,
                    })
                })
                .collect();
            let json = serde_json::to_string(&list).unwrap_or_else(|_| "[]".to_string());
            to_c_string(json)
        }
        Err(_) => to_c_string("[]"),
    }
}

#[no_mangle]
pub extern "C" fn pdfrune_c_insert_link(
    doc: *mut Document,
    page: usize,
    x0: f64,
    y0: f64,
    x1: f64,
    y1: f64,
    uri: *const c_char,
    dest_page: i32,
) -> i32 {
    if doc.is_null() {
        return 0;
    }
    let d = unsafe { &mut *doc };
    let rect = Rect::new(x0, y0, x1, y1);
    let u_str = unsafe { c_to_str(uri) };
    let uri_opt = if !uri.is_null() && !u_str.is_empty() {
        Some(u_str)
    } else {
        None
    };
    let dest_opt = if dest_page > 0 {
        Some((dest_page - 1) as usize)
    } else {
        None
    };
    match d.insert_page_link(page, rect, uri_opt, dest_opt) {
        Ok(_) => 1,
        Err(_) => 0,
    }
}

#[no_mangle]
pub extern "C" fn pdfrune_c_delete_link(
    doc: *mut Document,
    page: usize,
    xref: u32,
    x0: f64,
    y0: f64,
    x1: f64,
    y1: f64,
) -> i32 {
    if doc.is_null() {
        return 0;
    }
    let d = unsafe { &mut *doc };
    let rect = if x0 != x1 && y0 != y1 {
        Some(Rect::new(x0, y0, x1, y1))
    } else {
        None
    };
    match d.delete_page_link(page, if xref > 0 { Some(xref) } else { None }, rect) {
        Ok(_) => 1,
        Err(_) => 0,
    }
}

#[no_mangle]
pub extern "C" fn pdfrune_c_get_images_json(doc: *mut Document, page: usize) -> *mut c_char {
    if doc.is_null() {
        return ptr::null_mut();
    }
    let d = unsafe { &*doc };
    match d.get_page_images(page) {
        Ok(images) => {
            let list: Vec<serde_json::Value> = images
                .into_iter()
                .map(|i| {
                    serde_json::json!({
                        "xref": i.xref,
                        "smask": i.smask,
                        "width": i.width,
                        "height": i.height,
                        "bpc": i.bpc,
                        "colorspace": i.colorspace,
                        "name": i.name,
                        "filter": i.filter,
                    })
                })
                .collect();
            let json = serde_json::to_string(&list).unwrap_or_else(|_| "[]".to_string());
            to_c_string(json)
        }
        Err(_) => to_c_string("[]"),
    }
}

#[no_mangle]
pub extern "C" fn pdfrune_c_extract_image_bytes(
    doc: *mut Document,
    xref: u32,
    out_w: *mut i32,
    out_h: *mut i32,
    out_ext: *mut *mut c_char,
    out_len: *mut usize,
) -> *mut u8 {
    if doc.is_null() {
        return ptr::null_mut();
    }
    let d = unsafe { &*doc };
    match d.extract_image(xref) {
        Ok(img) => {
            if !out_w.is_null() {
                unsafe { *out_w = img.width as i32 };
            }
            if !out_h.is_null() {
                unsafe { *out_h = img.height as i32 };
            }
            if !out_ext.is_null() {
                unsafe { *out_ext = to_c_string(img.ext) };
            }
            if !out_len.is_null() {
                unsafe { *out_len = img.image.len() };
            }
            let mut v = img.image;
            v.shrink_to_fit();
            let p = v.as_mut_ptr();
            std::mem::forget(v);
            p
        }
        Err(_) => ptr::null_mut(),
    }
}

#[no_mangle]
pub extern "C" fn pdfrune_c_insert_image_bytes(
    doc: *mut Document,
    page: usize,
    x0: f64,
    y0: f64,
    x1: f64,
    y1: f64,
    bytes: *const u8,
    len: usize,
) -> i32 {
    if doc.is_null() || bytes.is_null() || len == 0 {
        return -1;
    }
    let d = unsafe { &mut *doc };
    let slice = unsafe { std::slice::from_raw_parts(bytes, len) };
    let rect = Rect::new(x0, y0, x1, y1);
    match d.insert_image_bytes(page, rect, slice) {
        Ok(xref) => xref as i32,
        Err(_) => -1,
    }
}

#[no_mangle]
pub extern "C" fn pdfrune_c_get_annots_json(doc: *mut Document, page: usize) -> *mut c_char {
    if doc.is_null() {
        return ptr::null_mut();
    }
    let d = unsafe { &*doc };
    match d.get_page_annots(page) {
        Ok(annots) => {
            let list: Vec<serde_json::Value> = annots
                .into_iter()
                .map(|a| {
                    serde_json::json!({
                        "xref": a.xref,
                        "type": a.subtype,
                        "x0": a.rect.x0,
                        "y0": a.rect.y0,
                        "x1": a.rect.x1,
                        "y1": a.rect.y1,
                        "contents": a.contents,
                        "color": [a.color[0], a.color[1], a.color[2]],
                        "overlay_text": a.overlay_text,
                    })
                })
                .collect();
            let json = serde_json::to_string(&list).unwrap_or_else(|_| "[]".to_string());
            to_c_string(json)
        }
        Err(_) => to_c_string("[]"),
    }
}

#[no_mangle]
pub extern "C" fn pdfrune_c_add_annot(
    doc: *mut Document,
    page: usize,
    subtype: *const c_char,
    x0: f64,
    y0: f64,
    x1: f64,
    y1: f64,
    contents: *const c_char,
    r: f64,
    g: f64,
    b: f64,
    overlay: *const c_char,
) -> u32 {
    if doc.is_null() {
        return 0;
    }
    let d = unsafe { &mut *doc };
    let st = unsafe { c_to_str(subtype) };
    let cont = if contents.is_null() {
        None
    } else {
        Some(unsafe { c_to_str(contents) })
    };
    let ot = if overlay.is_null() {
        None
    } else {
        Some(unsafe { c_to_str(overlay) })
    };
    let rect = Rect::new(x0, y0, x1, y1);
    match d.add_page_annot(page, st, rect, cont, [r, g, b], ot) {
        Ok(info) => info.xref,
        Err(_) => 0,
    }
}

#[no_mangle]
pub extern "C" fn pdfrune_c_delete_annot(doc: *mut Document, page: usize, xref: u32) -> i32 {
    if doc.is_null() {
        return 0;
    }
    let d = unsafe { &mut *doc };
    match d.delete_page_annot(page, xref) {
        Ok(_) => 1,
        Err(_) => 0,
    }
}

#[no_mangle]
pub extern "C" fn pdfrune_c_apply_redactions(doc: *mut Document, page: usize) -> i32 {
    if doc.is_null() {
        return -1;
    }
    let d = unsafe { &mut *doc };
    match d.apply_page_redactions(page) {
        Ok(count) => count as i32,
        Err(_) => -1,
    }
}

#[no_mangle]
pub extern "C" fn pdfrune_c_get_widgets_json(doc: *mut Document, page: usize) -> *mut c_char {
    if doc.is_null() {
        return ptr::null_mut();
    }
    let d = unsafe { &*doc };
    match d.get_page_widgets(page) {
        Ok(widgets) => {
            let list: Vec<serde_json::Value> = widgets
                .into_iter()
                .map(|w| {
                    serde_json::json!({
                        "xref": w.xref,
                        "field_name": w.field_name,
                        "field_type": w.field_type,
                        "field_value": w.field_value,
                        "x0": w.rect.x0,
                        "y0": w.rect.y0,
                        "x1": w.rect.x1,
                        "y1": w.rect.y1,
                    })
                })
                .collect();
            let json = serde_json::to_string(&list).unwrap_or_else(|_| "[]".to_string());
            to_c_string(json)
        }
        Err(_) => to_c_string("[]"),
    }
}

#[no_mangle]
pub extern "C" fn pdfrune_c_add_widget(
    doc: *mut Document,
    page: usize,
    name: *const c_char,
    field_type: *const c_char,
    x0: f64,
    y0: f64,
    x1: f64,
    y1: f64,
    val: *const c_char,
) -> u32 {
    if doc.is_null() {
        return 0;
    }
    let d = unsafe { &mut *doc };
    let n = unsafe { c_to_str(name) };
    let ft = unsafe { c_to_str(field_type) };
    let v = unsafe { c_to_str(val) };
    let rect = Rect::new(x0, y0, x1, y1);
    match d.add_page_widget(page, n, ft, rect, v) {
        Ok(w) => w.xref,
        Err(_) => 0,
    }
}

#[no_mangle]
pub extern "C" fn pdfrune_c_set_field_value(
    doc: *mut Document,
    page: usize,
    name: *const c_char,
    val: *const c_char,
) -> i32 {
    if doc.is_null() {
        return 0;
    }
    let d = unsafe { &mut *doc };
    let n = unsafe { c_to_str(name) };
    let v = unsafe { c_to_str(val) };
    match d.set_page_field_value(page, n, v) {
        Ok(updated) => {
            if updated {
                1
            } else {
                0
            }
        }
        Err(_) => 0,
    }
}

#[no_mangle]
pub extern "C" fn pdfrune_c_new_page(doc: *mut Document, pno: i32, width: f64, height: f64) -> i32 {
    if doc.is_null() {
        return -1;
    }
    let d = unsafe { &mut *doc };
    let target = if pno < 0 { None } else { Some(pno as usize) };
    match d.new_page(target, width, height) {
        Ok(idx) => idx as i32,
        Err(_) => -1,
    }
}

#[no_mangle]
pub extern "C" fn pdfrune_c_delete_page(doc: *mut Document, pno: i32) -> i32 {
    if doc.is_null() || pno < 0 {
        return 0;
    }
    let d = unsafe { &mut *doc };
    match d.delete_page(pno as usize) {
        Ok(_) => 1,
        Err(_) => 0,
    }
}

#[no_mangle]
pub extern "C" fn pdfrune_c_insert_text(
    doc: *mut Document,
    page: usize,
    x: f64,
    y: f64,
    text: *const c_char,
    fontsize: f64,
    r: f64,
    g: f64,
    b: f64,
) -> i32 {
    if doc.is_null() {
        return 0;
    }
    let d = unsafe { &mut *doc };
    let t = unsafe { c_to_str(text) };
    let pt = Point::new(x, y);
    match d.insert_text(page, pt, t, fontsize, [r, g, b]) {
        Ok(_) => 1,
        Err(_) => 0,
    }
}

#[no_mangle]
pub extern "C" fn pdfrune_c_draw_rect(
    doc: *mut Document,
    page: usize,
    x0: f64,
    y0: f64,
    x1: f64,
    y1: f64,
    has_stroke: i32,
    sr: f64,
    sg: f64,
    sb: f64,
    has_fill: i32,
    fr: f64,
    fg: f64,
    fb: f64,
    width: f64,
) -> i32 {
    if doc.is_null() {
        return 0;
    }
    let d = unsafe { &mut *doc };
    let rect = Rect::new(x0, y0, x1, y1);
    let stroke = if has_stroke != 0 {
        Some([sr, sg, sb])
    } else {
        None
    };
    let fill = if has_fill != 0 {
        Some([fr, fg, fb])
    } else {
        None
    };
    match d.draw_rect(page, rect, stroke, fill, width) {
        Ok(_) => 1,
        Err(_) => 0,
    }
}

#[no_mangle]
pub extern "C" fn pdfrune_c_to_bytes(
    doc: *mut Document,
    deflate: i32,
    encrypt: i32,
    user_pw: *const c_char,
    owner_pw: *const c_char,
    out_len: *mut usize,
) -> *mut u8 {
    if doc.is_null() {
        return ptr::null_mut();
    }
    let d = unsafe { &*doc };
    let res = if encrypt != 0 {
        let u = unsafe { c_to_str(user_pw) };
        let o = unsafe { c_to_str(owner_pw) };
        d.to_bytes_encrypted(deflate != 0, Some(u), Some(o))
    } else {
        d.to_bytes(deflate != 0)
    };

    match res {
        Ok(mut v) => {
            if !out_len.is_null() {
                unsafe { *out_len = v.len() };
            }
            v.shrink_to_fit();
            let p = v.as_mut_ptr();
            std::mem::forget(v);
            p
        }
        Err(_) => ptr::null_mut(),
    }
}

#[no_mangle]
pub extern "C" fn pdfrune_c_merge_pdfs(
    docs_bytes: *const *const u8,
    docs_lens: *const usize,
    count: usize,
    out_len: *mut usize,
) -> *mut u8 {
    if docs_bytes.is_null() || docs_lens.is_null() || count == 0 {
        return ptr::null_mut();
    }
    let mut parsed_docs = Vec::with_capacity(count);
    for i in 0..count {
        let ptr = unsafe { *docs_bytes.add(i) };
        let len = unsafe { *docs_lens.add(i) };
        let slice = unsafe { std::slice::from_raw_parts(ptr, len) };
        if let Ok(d) = Document::from_bytes(slice) {
            parsed_docs.push(d);
        }
    }
    if parsed_docs.is_empty() {
        return ptr::null_mut();
    }

    let mut merged = Document::new();
    for d in &parsed_docs {
        let cnt = d.page_count();
        if cnt > 0 {
            let _ = merged.insert_pdf(d, Some(0), Some(cnt - 1), None);
        }
    }

    match merged.to_bytes(true) {
        Ok(mut v) => {
            if !out_len.is_null() {
                unsafe { *out_len = v.len() };
            }
            v.shrink_to_fit();
            let p = v.as_mut_ptr();
            std::mem::forget(v);
            p
        }
        Err(_) => ptr::null_mut(),
    }
}

#[no_mangle]
pub extern "C" fn pdfrune_c_save(
    doc: *mut Document,
    path: *const c_char,
    deflate: i32,
    encrypt: i32,
    user_pw: *const c_char,
    owner_pw: *const c_char,
) -> i32 {
    if doc.is_null() || path.is_null() {
        return 0;
    }
    let d = unsafe { &*doc };
    let path_str = unsafe { c_to_str(path) };
    let res = if encrypt != 0 {
        let u = unsafe { c_to_str(user_pw) };
        let o = unsafe { c_to_str(owner_pw) };
        d.to_bytes_encrypted(deflate != 0, Some(u), Some(o))
    } else {
        d.to_bytes(deflate != 0)
    };
    match res {
        Ok(bytes) => match std::fs::write(path_str, bytes) {
            Ok(_) => 1,
            Err(_) => 0,
        },
        Err(_) => 0,
    }
}
