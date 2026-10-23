use std::fmt::Write as _;
use std::io::Write;

use flate2::write::ZlibEncoder;
use flate2::Compression;

use crate::document::Document;
use crate::error::{PdfruneError, Result};
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

    if let Ok((w, h, rgba)) = decode_image_xobject(doc, stm) {
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

/// Decode an image without document-level references. Kept for API compatibility.
/// Rendering uses the checked, document-aware decoder so errors are not hidden.
pub fn decode_image_xobject_to_rgba(stm: &PdfStream) -> Option<(u32, u32, Vec<u8>)> {
    decode_image_inner(None, stm, 0).ok()
}

/// Decode raster samples and the optional document-referenced soft mask.
pub fn decode_image_xobject(doc: &Document, stm: &PdfStream) -> Result<(u32, u32, Vec<u8>)> {
    decode_image_inner(Some(doc), stm, 0)
}

fn image_error(message: impl Into<String>) -> PdfruneError {
    PdfruneError::RenderError(message.into())
}

fn checked_pixels(width: u32, height: u32) -> Result<usize> {
    let n = (width as usize)
        .checked_mul(height as usize)
        .filter(|n| *n > 0 && *n <= 64_000_000)
        .ok_or_else(|| {
            image_error("Image exceeds the 64-million-pixel decode limit or has invalid dimensions")
        })?;
    Ok(n)
}

fn image_filters(stm: &PdfStream) -> Result<Vec<&[u8]>> {
    match stm.dict.get(b"Filter".as_slice()) {
        None => Ok(vec![]),
        Some(PdfObject::Name(name)) => Ok(vec![name]),
        Some(PdfObject::Array(arr)) => arr
            .iter()
            .map(|item| {
                item.as_name()
                    .ok_or_else(|| image_error("Invalid image filter array"))
            })
            .collect(),
        _ => Err(image_error("Invalid image filter")),
    }
}

fn decode_image_inner(
    doc: Option<&Document>,
    stm: &PdfStream,
    depth: usize,
) -> Result<(u32, u32, Vec<u8>)> {
    if depth > 8 {
        return Err(image_error(
            "Image soft-mask reference cycle or excessive depth",
        ));
    }
    let width = u32::try_from(
        stm.dict
            .get_int(b"Width")
            .ok_or_else(|| image_error("Missing image width"))?,
    )
    .map_err(|_| image_error("Invalid image width"))?;
    let height = u32::try_from(
        stm.dict
            .get_int(b"Height")
            .ok_or_else(|| image_error("Missing image height"))?,
    )
    .map_err(|_| image_error("Invalid image height"))?;
    let count = checked_pixels(width, height)?;
    let filters = image_filters(stm)?;
    let codec = filters.last().copied().unwrap_or(b"");
    let encoded = matches!(codec, b"DCTDecode" | b"DCT" | b"JPXDecode" | b"JBIG2Decode");
    let data = if encoded {
        // Decode transport filters before handing the final codestream to its
        // image codec. Do not treat compressed JPEG/JPX bytes as pixel samples.
        let mut prefix = stm.clone();
        prefix.dict.remove(b"Filter".as_slice());
        prefix.dict.remove(b"DecodeParms".as_slice());
        if filters.len() > 1 {
            prefix.dict.insert(
                b"Filter".to_vec(),
                PdfObject::Array(
                    filters[..filters.len() - 1]
                        .iter()
                        .map(|f| PdfObject::Name(f.to_vec()))
                        .collect(),
                ),
            );
            if let Some(PdfObject::Array(parms)) = stm.dict.get(b"DecodeParms".as_slice()) {
                prefix.dict.insert(
                    b"DecodeParms".to_vec(),
                    PdfObject::Array(parms.iter().take(filters.len() - 1).cloned().collect()),
                );
            }
        }
        prefix.decode_data()?
    } else {
        stm.decode_data()?
    };
    let (samples, channels, alpha) = match codec {
        b"DCTDecode" | b"DCT" => {
            let mut decoder = jpeg_decoder::Decoder::new(std::io::Cursor::new(&data));
            decoder
                .read_info()
                .map_err(|e| image_error(format!("JPEG header: {e}")))?;
            let info = decoder
                .info()
                .ok_or_else(|| image_error("Missing JPEG info"))?;
            if (info.width as u32, info.height as u32) != (width, height) {
                return Err(image_error(
                    "JPEG dimensions disagree with image dictionary",
                ));
            }
            let n = match info.pixel_format {
                jpeg_decoder::PixelFormat::L8 => 1,
                jpeg_decoder::PixelFormat::RGB24 => 3,
                jpeg_decoder::PixelFormat::CMYK32 => 4,
                _ => return Err(image_error("Unsupported JPEG sample format")),
            };
            (
                decoder
                    .decode()
                    .map_err(|e| image_error(format!("JPEG decode: {e}")))?,
                n,
                false,
            )
        }
        b"JPXDecode" => {
            let image =
                hayro_jpeg2000::Image::new(&data, &hayro_jpeg2000::DecodeSettings::default())
                    .map_err(|e| image_error(format!("JPEG2000 header: {e}")))?;
            if (image.width(), image.height()) != (width, height) {
                return Err(image_error(
                    "JPEG2000 dimensions disagree with image dictionary",
                ));
            }
            let channels = image.color_space().num_channels() as usize;
            let has_alpha = image.has_alpha();
            let mut ctx = hayro_jpeg2000::DecoderContext::default();
            let decoded = image
                .decode(&mut ctx)
                .map_err(|e| image_error(format!("JPEG2000 decode: {e}")))?;
            (decoded.data_u8(), channels, has_alpha)
        }
        b"JBIG2Decode" => {
            let parms = match stm.dict.get(b"DecodeParms".as_slice()) {
                Some(PdfObject::Array(arr)) => arr.last().and_then(PdfObject::as_dict),
                Some(PdfObject::Dict(d)) => Some(d),
                _ => None,
            };
            let global_stream = parms.and_then(|p| p.get(b"JBIG2Globals".as_slice()));
            let globals = if let Some(obj) = global_stream {
                let stream = doc
                    .ok_or_else(|| image_error("JBIG2 globals require document context"))?
                    .resolve(obj)
                    .as_stream()
                    .ok_or_else(|| image_error("Invalid JBIG2 globals"))?;
                Some(stream.decode_data()?)
            } else {
                None
            };
            let image = hayro_jbig2::Image::new_embedded(&data, globals.as_deref())
                .map_err(|e| image_error(format!("JBIG2 header: {e}")))?;
            if (image.width(), image.height()) != (width, height) {
                return Err(image_error(
                    "JBIG2 dimensions disagree with image dictionary",
                ));
            }
            struct Gray(Vec<u8>);
            impl hayro_jbig2::Decoder for Gray {
                fn push_pixel(&mut self, black: bool) {
                    self.0.push(if black { 0 } else { 255 });
                }
                fn push_pixel_chunk(&mut self, black: bool, n: u32) {
                    self.0.extend(std::iter::repeat_n(
                        if black { 0 } else { 255 },
                        n as usize * 8,
                    ));
                }
                fn next_line(&mut self) {}
            }
            let mut gray = Gray(Vec::with_capacity(count));
            image
                .decode(&mut gray)
                .map_err(|e| image_error(format!("JBIG2 decode: {e}")))?;
            (gray.0, 1, false)
        }
        _ => {
            let cs = stm.dict.get_name_str(b"ColorSpace").unwrap_or("DeviceRGB");
            let channels = match cs {
                "DeviceGray" | "G" => 1,
                "DeviceRGB" | "RGB" => 3,
                "DeviceCMYK" | "CMYK" => 4,
                _ => return Err(image_error(format!("Unsupported image color space {cs}"))),
            };
            let bits = stm.dict.get_int(b"BitsPerComponent").unwrap_or(8);
            if !matches!(bits, 1 | 2 | 4 | 8 | 16) {
                return Err(image_error("Unsupported image bit depth"));
            }
            let bits = bits as usize;
            let row_bits = width as usize * channels * bits;
            let row_bytes = row_bits.div_ceil(8);
            if data.len() < row_bytes * height as usize {
                return Err(image_error("Truncated image sample data"));
            }
            let mut samples = Vec::with_capacity(count * channels);
            for row in data.chunks_exact(row_bytes).take(height as usize) {
                for index in 0..width as usize * channels {
                    let bit = index * bits;
                    let v = if bits == 16 {
                        u16::from_be_bytes([row[bit / 8], row[bit / 8 + 1]]) as u32
                    } else {
                        ((row[bit / 8] >> (8 - bits - bit % 8)) & ((1u16 << bits) - 1) as u8) as u32
                    };
                    samples
                        .push(((v * 255 + ((1u32 << bits) - 1) / 2) / ((1u32 << bits) - 1)) as u8);
                }
            }
            (samples, channels, false)
        }
    };
    if !matches!(channels, 1 | 3 | 4) {
        return Err(image_error("Unsupported decoded image channel count"));
    }
    let stride = channels + usize::from(alpha);
    if samples.len() != count * stride {
        return Err(image_error("Decoded image sample count mismatch"));
    }
    let decode = stm
        .dict
        .get(b"Decode".as_slice())
        .and_then(PdfObject::as_array);
    let mut rgba = Vec::with_capacity(count * 4);
    for sample in samples.chunks_exact(stride) {
        let mut color = [0u8; 4];
        color[..channels].copy_from_slice(&sample[..channels]);
        if codec != b"JPXDecode" {
            if let Some(decode) = decode {
                if decode.len() != channels * 2 {
                    return Err(image_error("Invalid image Decode array"));
                }
                for c in 0..channels {
                    let lo = decode[2 * c]
                        .as_f64()
                        .ok_or_else(|| image_error("Invalid Decode value"))?;
                    let hi = decode[2 * c + 1]
                        .as_f64()
                        .ok_or_else(|| image_error("Invalid Decode value"))?;
                    color[c] = ((lo + (color[c] as f64 / 255.0) * (hi - lo)) * 255.0)
                        .clamp(0.0, 255.0)
                        .round() as u8;
                }
            }
        }
        let rgb = match channels {
            1 => [color[0]; 3],
            3 => [color[0], color[1], color[2]],
            4 => [
                (((255 - color[0]) as u16 * (255 - color[3]) as u16 + 127) / 255) as u8,
                (((255 - color[1]) as u16 * (255 - color[3]) as u16 + 127) / 255) as u8,
                (((255 - color[2]) as u16 * (255 - color[3]) as u16 + 127) / 255) as u8,
            ],
            _ => unreachable!(),
        };
        rgba.extend_from_slice(&[
            rgb[0],
            rgb[1],
            rgb[2],
            if alpha { sample[channels] } else { 255 },
        ]);
    }
    if let Some(mask_obj) = stm
        .dict
        .get(b"SMask".as_slice())
        .filter(|m| !matches!(m,PdfObject::Name(n) if n==b"None"))
    {
        let doc = doc.ok_or_else(|| image_error("Soft mask requires document context"))?;
        let mask = doc
            .resolve(mask_obj)
            .as_stream()
            .ok_or_else(|| image_error("Invalid image soft mask"))?;
        let (mw, mh, mask_rgba) = decode_image_inner(Some(doc), mask, depth + 1)?;
        for y in 0..height as usize {
            for x in 0..width as usize {
                let mx = x * mw as usize / width as usize;
                let my = y * mh as usize / height as usize;
                let a = mask_rgba[(my * mw as usize + mx) * 4] as u16;
                let offset = (y * width as usize + x) * 4 + 3;
                rgba[offset] = ((rgba[offset] as u16 * a + 127) / 255) as u8;
            }
        }
    }
    Ok((width, height, rgba))
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
