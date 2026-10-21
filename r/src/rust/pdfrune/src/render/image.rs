use std::fmt::Write as _;
use std::io::Write;

use flate2::write::ZlibEncoder;
use flate2::Compression;

use crate::document::Document;
use crate::error::{Result, PdfruneError};
use crate::geometry::Rect;
use crate::parser::object::{PdfDict, PdfDictExt, PdfObject, PdfStream};
use crate::render::Pixmap;

/// Metadata for an `/Image` XObject referenced on a page.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct PageImageInfo {
    pub xref: u32,
    pub smask: u32,
    pub width: u32,
    pub height: u32,
    pub bpc: u8,
    pub colorspace: String,
    pub alt_colorspace: String,
    pub name: String,
    pub filter: String,
}

/// Extracted image payload and metadata returned by `extract_image(xref)`.
#[derive(Debug, Clone)]
pub struct ExtractedImage {
    pub ext: String,
    pub image: Vec<u8>,
    pub width: u32,
    pub height: u32,
    pub colorspace: u8,
    pub bpc: u8,
}

/// Lists all `/Subtype /Image` XObjects in the page's `/Resources /XObject` dictionary.
pub fn get_page_images(doc: &Document, page_index: usize) -> Result<Vec<PageImageInfo>> {
    let page = doc.page(page_index)?;
    let xobj_dict = doc
        .resolve_dict_entry(&page.resources, b"XObject")
        .and_then(PdfObject::as_dict)
        .or_else(|| {
            doc.get_object(page.object_id)
                .and_then(PdfObject::as_dict)
                .and_then(|d| doc.resolve_dict_entry(d, b"Resources"))
                .and_then(PdfObject::as_dict)
                .and_then(|r| doc.resolve_dict_entry(r, b"XObject"))
                .and_then(PdfObject::as_dict)
        });

    let Some(xobjects) = xobj_dict else {
        return Ok(Vec::new());
    };

    let mut out = Vec::new();
    for (name_bytes, obj_ref) in xobjects {
        let xref = obj_ref.as_reference().map(|r| r.0).unwrap_or(0);
        let Some(stm) = doc.resolve(obj_ref).as_stream() else {
            continue;
        };
        if stm.dict.get_name_str(b"Subtype") != Some("Image") {
            continue;
        }
        let width = stm
            .dict
            .get_int(b"Width")
            .or_else(|| stm.dict.get_int(b"W"))
            .unwrap_or(0)
            .max(0) as u32;
        let height = stm
            .dict
            .get_int(b"Height")
            .or_else(|| stm.dict.get_int(b"H"))
            .unwrap_or(0)
            .max(0) as u32;
        let bpc = stm
            .dict
            .get_int(b"BitsPerComponent")
            .or_else(|| stm.dict.get_int(b"BPC"))
            .unwrap_or(8)
            .max(1) as u8;
        let smask = stm.dict.get_ref(b"SMask").map(|r| r.0).unwrap_or(0);
        let colorspace = stm
            .dict
            .get_name_str(b"ColorSpace")
            .unwrap_or("DeviceRGB")
            .to_string();
        let filter = stm.dict.get_name_str(b"Filter").unwrap_or("").to_string();
        let name = String::from_utf8_lossy(name_bytes).into_owned();

        out.push(PageImageInfo {
            xref,
            smask,
            width,
            height,
            bpc,
            colorspace,
            alt_colorspace: String::new(),
            name,
            filter,
        });
    }

    Ok(out)
}

/// Extracts an `/Image` XObject by its indirect object number (`xref`).
pub fn extract_image(doc: &Document, xref: u32) -> Result<ExtractedImage> {
    let stm = doc
        .get_object((xref, 0))
        .and_then(PdfObject::as_stream)
        .ok_or_else(|| {
            PdfruneError::InvalidArgument(format!("Object {xref} is not an image stream"))
        })?;

    if stm.dict.get_name_str(b"Subtype") != Some("Image") {
        return Err(PdfruneError::InvalidArgument(format!(
            "Stream {xref} is not /Subtype /Image"
        )));
    }

    let width = stm.dict.get_int(b"Width").unwrap_or(0).max(1) as u32;
    let height = stm.dict.get_int(b"Height").unwrap_or(0).max(1) as u32;
    let bpc = stm.dict.get_int(b"BitsPerComponent").unwrap_or(8).max(1) as u8;
    let cs_name = stm.dict.get_name_str(b"ColorSpace").unwrap_or("DeviceRGB");
    let colorspace: u8 = match cs_name {
        "DeviceGray" | "G" => 1,
        "DeviceCMYK" | "CMYK" => 4,
        _ => 3,
    };

    let filter = stm.dict.get_name_str(b"Filter").unwrap_or("");
    if filter == "DCTDecode" {
        return Ok(ExtractedImage {
            ext: "jpg".to_string(),
            image: stm.data.clone(),
            width,
            height,
            colorspace,
            bpc,
        });
    }

    if let Some((w, h, rgba)) = decode_image_xobject_to_rgba(stm) {
        let mut pix = Pixmap::new_white(w, h);
        pix.data = rgba;
        let png_bytes = pix.to_png()?;
        return Ok(ExtractedImage {
            ext: "png".to_string(),
            image: png_bytes,
            width: w,
            height: h,
            colorspace,
            bpc,
        });
    }

    Err(PdfruneError::RenderError(format!(
        "Unsupported image stream format in object {xref}"
    )))
}

/// Decodes a PDF `/Image` XObject stream into RGBA8888 pixel bytes `(width, height, rgba_bytes)`.
pub fn decode_image_xobject_to_rgba(stm: &PdfStream) -> Option<(u32, u32, Vec<u8>)> {
    let width = stm.dict.get_int(b"Width")?.max(1) as u32;
    let height = stm.dict.get_int(b"Height")?.max(1) as u32;
    let cs_name = stm.dict.get_name_str(b"ColorSpace").unwrap_or("DeviceRGB");
    let raw = stm.decode_data().ok()?;
    let pixel_count = (width as usize) * (height as usize);
    let mut rgba = Vec::with_capacity(pixel_count * 4);

    match cs_name {
        "DeviceGray" | "G" => {
            if raw.len() < pixel_count {
                return None;
            }
            for &g in &raw[..pixel_count] {
                rgba.extend_from_slice(&[g, g, g, 255]);
            }
        }
        _ => {
            if raw.len() >= pixel_count * 3 {
                for chunk in raw[..pixel_count * 3].chunks_exact(3) {
                    rgba.extend_from_slice(&[chunk[0], chunk[1], chunk[2], 255]);
                }
            } else {
                return None;
            }
        }
    }

    Some((width, height, rgba))
}

/// Inserts an image from PNG bytes onto `page_index` at `rect` (top-down coordinates).
/// Returns the `xref` object number of the newly created `/Image` XObject.
pub fn insert_image_bytes(
    doc: &mut Document,
    page_index: usize,
    rect: Rect,
    image_bytes: &[u8],
) -> Result<u32> {
    let decoder = png::Decoder::new(std::io::Cursor::new(image_bytes));
    let mut reader = decoder
        .read_info()
        .map_err(|e| PdfruneError::InvalidArgument(format!("PNG decode error: {e}")))?;
    let mut buf = vec![0u8; reader.output_buffer_size()];
    let info = reader
        .next_frame(&mut buf)
        .map_err(|e| PdfruneError::InvalidArgument(format!("PNG frame error: {e}")))?;
    let frame_bytes = &buf[..info.buffer_size()];

    let width = info.width;
    let height = info.height;
    let pixel_count = (width as usize) * (height as usize);
    let mut rgb = Vec::with_capacity(pixel_count * 3);

    match info.color_type {
        png::ColorType::Rgba => {
            for px in frame_bytes.chunks_exact(4) {
                // Alpha composite over white background
                let a = (px[3] as u16) as f32 / 255.0;
                let r = ((px[0] as f32) * a + 255.0 * (1.0 - a)).round() as u8;
                let g = ((px[1] as f32) * a + 255.0 * (1.0 - a)).round() as u8;
                let b = ((px[2] as f32) * a + 255.0 * (1.0 - a)).round() as u8;
                rgb.extend_from_slice(&[r, g, b]);
            }
        }
        png::ColorType::Rgb => {
            rgb.extend_from_slice(&frame_bytes[..pixel_count * 3]);
        }
        png::ColorType::Grayscale => {
            for &g in &frame_bytes[..pixel_count] {
                rgb.extend_from_slice(&[g, g, g]);
            }
        }
        png::ColorType::GrayscaleAlpha => {
            for px in frame_bytes.chunks_exact(2) {
                let a = (px[1] as f32) / 255.0;
                let g = ((px[0] as f32) * a + 255.0 * (1.0 - a)).round() as u8;
                rgb.extend_from_slice(&[g, g, g]);
            }
        }
        png::ColorType::Indexed => {
            return Err(PdfruneError::InvalidArgument(
                "Indexed PNG palette not directly supported; convert to RGB/RGBA".to_string(),
            ));
        }
    }

    insert_rgb_image_xobject(doc, page_index, rect, width, height, &rgb)
}

/// Inserts a `Pixmap` onto `page_index` at `rect` (top-down coordinates).
pub fn insert_image_pixmap(
    doc: &mut Document,
    page_index: usize,
    rect: Rect,
    pixmap: &Pixmap,
) -> Result<u32> {
    let pixel_count = (pixmap.width as usize) * (pixmap.height as usize);
    let mut rgb = Vec::with_capacity(pixel_count * 3);
    for px in pixmap.data.chunks_exact(4) {
        let a = (px[3] as f32) / 255.0;
        let r = ((px[0] as f32) * a + 255.0 * (1.0 - a)).round() as u8;
        let g = ((px[1] as f32) * a + 255.0 * (1.0 - a)).round() as u8;
        let b = ((px[2] as f32) * a + 255.0 * (1.0 - a)).round() as u8;
        rgb.extend_from_slice(&[r, g, b]);
    }
    insert_rgb_image_xobject(doc, page_index, rect, pixmap.width, pixmap.height, &rgb)
}

fn insert_rgb_image_xobject(
    doc: &mut Document,
    page_index: usize,
    rect: Rect,
    width: u32,
    height: u32,
    rgb_data: &[u8],
) -> Result<u32> {
    let page = doc.page(page_index)?.clone();
    let page_height = page.height();
    let norm = rect.normalize();
    let pdf_x = norm.x0;
    let pdf_y = page_height - norm.y1;
    let pdf_w = norm.width().max(1.0);
    let pdf_h = norm.height().max(1.0);

    // Compress RGB data with FlateDecode
    let mut enc = ZlibEncoder::new(Vec::new(), Compression::default());
    enc.write_all(rgb_data)?;
    let compressed = enc.finish()?;

    let mut img_dict = PdfDict::new();
    img_dict.insert(b"Type".to_vec(), PdfObject::Name(b"XObject".to_vec()));
    img_dict.insert(b"Subtype".to_vec(), PdfObject::Name(b"Image".to_vec()));
    img_dict.insert(b"Width".to_vec(), PdfObject::Int(width as i64));
    img_dict.insert(b"Height".to_vec(), PdfObject::Int(height as i64));
    img_dict.insert(
        b"ColorSpace".to_vec(),
        PdfObject::Name(b"DeviceRGB".to_vec()),
    );
    img_dict.insert(b"BitsPerComponent".to_vec(), PdfObject::Int(8));
    img_dict.insert(b"Filter".to_vec(), PdfObject::Name(b"FlateDecode".to_vec()));
    img_dict.insert(b"Length".to_vec(), PdfObject::Int(compressed.len() as i64));

    let img_id = doc.allocate_object_id();
    doc.objects.insert(
        img_id.0,
        PdfObject::Stream(PdfStream::new(img_dict, compressed)),
    );

    let im_name = format!("Im{}", img_id.0);

    // Register `/Im<id>` in `doc.pages[page_index].resources` and `page_dict`
    if let Some(p_mut) = doc.pages.get_mut(page_index) {
        let xobj_entry = p_mut
            .resources
            .entry(b"XObject".to_vec())
            .or_insert_with(|| PdfObject::Dict(PdfDict::new()));
        if let PdfObject::Dict(xmap) = xobj_entry {
            xmap.insert(im_name.as_bytes().to_vec(), PdfObject::Reference(img_id));
        }
        let updated_resources = p_mut.resources.clone();
        if let Some(page_dict) = doc
            .objects
            .get_mut(&page.object_id.0)
            .and_then(PdfObject::as_dict_mut)
        {
            page_dict.insert(b"Resources".to_vec(), PdfObject::Dict(updated_resources));
        }
    }

    // Append `q <w> 0 0 <h> <x> <y> cm /Im<id> Do Q` to page content stream
    let mut ops = String::new();
    let _ = writeln!(
        ops,
        "\nq {:.3} 0 0 {:.3} {:.3} {:.3} cm /{} Do Q",
        pdf_w, pdf_h, pdf_x, pdf_y, im_name
    );
    doc.append_page_content(page_index, ops.as_bytes())?;

    Ok(img_id.0)
}
