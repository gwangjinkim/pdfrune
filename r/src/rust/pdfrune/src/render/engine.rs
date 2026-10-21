use tiny_skia::{
    Color, FillRule, Paint, PathBuilder, Pixmap as SkiaPixmap, Rect as SkiaRect, Stroke, Transform,
};

use crate::document::Document;
use crate::error::{Result, PdfruneError};
use crate::geometry::{Matrix, Point};
use crate::parser::lexer::{ContentToken, Lexer};
use crate::parser::object::PdfObject;
use crate::render::pixmap::Pixmap;
use crate::text::extractor::extract_page_spans;

#[derive(Debug, Clone)]
struct RenderGraphicsState {
    ctm: Matrix,
    stroke_color: [u8; 4],
    fill_color: [u8; 4],
    line_width: f32,
}

impl Default for RenderGraphicsState {
    fn default() -> Self {
        Self {
            ctm: Matrix::identity(),
            stroke_color: [0, 0, 0, 255],
            fill_color: [0, 0, 0, 255],
            line_width: 1.0,
        }
    }
}

/// Rasterizes the page at `page_index` at the requested `dpi` (default 72.0 = 1.0x scale).
pub fn render_page(doc: &Document, page_index: usize, dpi: f64) -> Result<Pixmap> {
    let page = doc.page(page_index)?;
    let scale = (dpi / 72.0).clamp(0.1, 16.0);
    let width = ((page.width() * scale).round() as u32).max(1);
    let height = ((page.height() * scale).round() as u32).max(1);

    let mut skia_pix = SkiaPixmap::new(width, height)
        .ok_or_else(|| PdfruneError::RenderError("Failed to allocate Pixmap buffer".into()))?;
    skia_pix.fill(Color::WHITE);

    let page_height_pt = page.cropbox.height().max(1.0);
    let crop_x0 = page.cropbox.x0;
    let crop_y0 = page.cropbox.y0;

    let content_bytes = doc.page_content_bytes(page_index)?;
    if !content_bytes.is_empty() {
        render_vector_ops(
            doc,
            &page.resources,
            &content_bytes,
            &mut skia_pix,
            scale,
            page_height_pt,
            crop_x0,
            crop_y0,
        )?;
    }

    // Check for any embedded TrueType (/FontFile2) font bytes in page resources
    let embedded_ttf = find_first_embedded_ttf(doc, &page.resources);

    // Render text spans cleanly onto the raster buffer
    if let Ok(spans) = extract_page_spans(doc, page_index) {
        let mut paint = Paint::default();
        paint.set_color_rgba8(20, 20, 20, 255);
        paint.anti_alias = true;

        for span in spans {
            for (ch, char_rect) in &span.chars {
                if ch.is_whitespace() {
                    continue;
                }
                if let Some(ref ttf_bytes) = embedded_ttf {
                    if draw_ttf_glyph(&mut skia_pix, ttf_bytes, *ch, *char_rect, scale as f32) {
                        continue;
                    }
                }
                draw_glyph_outline(&mut skia_pix, *ch, *char_rect, scale as f32, &paint);
            }
        }
    }

    Ok(Pixmap {
        width,
        height,
        data: skia_pix.take(),
    })
}

#[allow(clippy::too_many_arguments)]
fn render_vector_ops(
    doc: &Document,
    resources: &crate::parser::object::PdfDict,
    content_bytes: &[u8],
    skia_pix: &mut SkiaPixmap,
    scale: f64,
    page_height_pt: f64,
    crop_x0: f64,
    crop_y0: f64,
) -> Result<()> {
    let mut lexer = Lexer::new(content_bytes);
    let tokens = lexer.tokenize_content_stream()?;
    let mut operands: Vec<PdfObject> = Vec::with_capacity(8);

    let mut gs = RenderGraphicsState::default();
    let mut gs_stack: Vec<RenderGraphicsState> = Vec::new();
    let mut pb = PathBuilder::new();

    let to_screen = |pt: Point, ctm: &Matrix| -> (f32, f32) {
        let p = ctm.transform_point(pt);
        let sx = ((p.x - crop_x0) * scale) as f32;
        let sy = ((page_height_pt - (p.y - crop_y0)) * scale) as f32;
        (sx, sy)
    };

    for token in tokens {
        match token {
            ContentToken::Operand(obj) => operands.push(obj),
            ContentToken::Operator(op) => {
                match op.as_slice() {
                    b"q" => gs_stack.push(gs.clone()),
                    b"Q" => {
                        if let Some(prev) = gs_stack.pop() {
                            gs = prev;
                        }
                    }
                    b"cm" => {
                        if operands.len() >= 6 {
                            let n = operands.len();
                            let m = Matrix::new(
                                operands[n - 6].as_f64().unwrap_or(1.0),
                                operands[n - 5].as_f64().unwrap_or(0.0),
                                operands[n - 4].as_f64().unwrap_or(0.0),
                                operands[n - 3].as_f64().unwrap_or(1.0),
                                operands[n - 2].as_f64().unwrap_or(0.0),
                                operands[n - 1].as_f64().unwrap_or(0.0),
                            );
                            gs.ctm = m.multiply(&gs.ctm);
                        }
                    }
                    b"w" => {
                        if let Some(w) = operands.last().and_then(PdfObject::as_f64) {
                            gs.line_width = (w * scale).max(0.5) as f32;
                        }
                    }
                    b"RG" => {
                        if operands.len() >= 3 {
                            let n = operands.len();
                            gs.stroke_color = rgb_to_u8(
                                operands[n - 3].as_f64().unwrap_or(0.0),
                                operands[n - 2].as_f64().unwrap_or(0.0),
                                operands[n - 1].as_f64().unwrap_or(0.0),
                            );
                        }
                    }
                    b"rg" => {
                        if operands.len() >= 3 {
                            let n = operands.len();
                            gs.fill_color = rgb_to_u8(
                                operands[n - 3].as_f64().unwrap_or(0.0),
                                operands[n - 2].as_f64().unwrap_or(0.0),
                                operands[n - 1].as_f64().unwrap_or(0.0),
                            );
                        }
                    }
                    b"G" => {
                        if let Some(gray) = operands.last().and_then(PdfObject::as_f64) {
                            gs.stroke_color = rgb_to_u8(gray, gray, gray);
                        }
                    }
                    b"g" => {
                        if let Some(gray) = operands.last().and_then(PdfObject::as_f64) {
                            gs.fill_color = rgb_to_u8(gray, gray, gray);
                        }
                    }
                    b"m" => {
                        if operands.len() >= 2 {
                            let n = operands.len();
                            let x = operands[n - 2].as_f64().unwrap_or(0.0);
                            let y = operands[n - 1].as_f64().unwrap_or(0.0);
                            let (sx, sy) = to_screen(Point::new(x, y), &gs.ctm);
                            pb.move_to(sx, sy);
                        }
                    }
                    b"l" => {
                        if operands.len() >= 2 {
                            let n = operands.len();
                            let x = operands[n - 2].as_f64().unwrap_or(0.0);
                            let y = operands[n - 1].as_f64().unwrap_or(0.0);
                            let (sx, sy) = to_screen(Point::new(x, y), &gs.ctm);
                            pb.line_to(sx, sy);
                        }
                    }
                    b"c" => {
                        if operands.len() >= 6 {
                            let n = operands.len();
                            let (x1, y1) = to_screen(
                                Point::new(
                                    operands[n - 6].as_f64().unwrap_or(0.0),
                                    operands[n - 5].as_f64().unwrap_or(0.0),
                                ),
                                &gs.ctm,
                            );
                            let (x2, y2) = to_screen(
                                Point::new(
                                    operands[n - 4].as_f64().unwrap_or(0.0),
                                    operands[n - 3].as_f64().unwrap_or(0.0),
                                ),
                                &gs.ctm,
                            );
                            let (x3, y3) = to_screen(
                                Point::new(
                                    operands[n - 2].as_f64().unwrap_or(0.0),
                                    operands[n - 1].as_f64().unwrap_or(0.0),
                                ),
                                &gs.ctm,
                            );
                            pb.cubic_to(x1, y1, x2, y2, x3, y3);
                        }
                    }
                    b"h" => pb.close(),
                    b"re" => {
                        if operands.len() >= 4 {
                            let n = operands.len();
                            let rx = operands[n - 4].as_f64().unwrap_or(0.0);
                            let ry = operands[n - 3].as_f64().unwrap_or(0.0);
                            let rw = operands[n - 2].as_f64().unwrap_or(0.0);
                            let rh = operands[n - 1].as_f64().unwrap_or(0.0);
                            let (sx0, sy0) = to_screen(Point::new(rx, ry), &gs.ctm);
                            let (sx1, sy1) = to_screen(Point::new(rx + rw, ry + rh), &gs.ctm);
                            let left = sx0.min(sx1);
                            let top = sy0.min(sy1);
                            let w = (sx1 - sx0).abs().max(0.5);
                            let h = (sy1 - sy0).abs().max(0.5);
                            if let Some(r) = SkiaRect::from_xywh(left, top, w, h) {
                                pb.push_rect(r);
                            }
                        }
                    }
                    b"f" | b"F" | b"f*" => {
                        let finished = pb.finish();
                        pb = PathBuilder::new();
                        if let Some(path) = finished {
                            let mut paint = Paint::default();
                            let [r, g, b, a] = gs.fill_color;
                            paint.set_color_rgba8(r, g, b, a);
                            paint.anti_alias = true;
                            let rule = if op.as_slice() == b"f*" {
                                FillRule::EvenOdd
                            } else {
                                FillRule::Winding
                            };
                            skia_pix.fill_path(&path, &paint, rule, Transform::identity(), None);
                        }
                    }
                    b"S" | b"s" => {
                        if op.as_slice() == b"s" {
                            pb.close();
                        }
                        let finished = pb.finish();
                        pb = PathBuilder::new();
                        if let Some(path) = finished {
                            let mut paint = Paint::default();
                            let [r, g, b, a] = gs.stroke_color;
                            paint.set_color_rgba8(r, g, b, a);
                            paint.anti_alias = true;
                            let stroke = Stroke {
                                width: gs.line_width,
                                ..Stroke::default()
                            };
                            skia_pix.stroke_path(
                                &path,
                                &paint,
                                &stroke,
                                Transform::identity(),
                                None,
                            );
                        }
                    }
                    b"B" | b"B*" | b"b" | b"b*" => {
                        if op.as_slice() == b"b" || op.as_slice() == b"b*" {
                            pb.close();
                        }
                        let finished = pb.finish();
                        pb = PathBuilder::new();
                        if let Some(path) = finished {
                            let mut fill_paint = Paint::default();
                            let [fr, fg, fb, fa] = gs.fill_color;
                            fill_paint.set_color_rgba8(fr, fg, fb, fa);
                            fill_paint.anti_alias = true;
                            skia_pix.fill_path(
                                &path,
                                &fill_paint,
                                FillRule::Winding,
                                Transform::identity(),
                                None,
                            );

                            let mut stroke_paint = Paint::default();
                            let [sr, sg, sb, sa] = gs.stroke_color;
                            stroke_paint.set_color_rgba8(sr, sg, sb, sa);
                            stroke_paint.anti_alias = true;
                            let stroke = Stroke {
                                width: gs.line_width,
                                ..Stroke::default()
                            };
                            skia_pix.stroke_path(
                                &path,
                                &stroke_paint,
                                &stroke,
                                Transform::identity(),
                                None,
                            );
                        }
                    }
                    b"n" => {
                        pb = PathBuilder::new();
                    }
                    b"Do" => {
                        if let Some(name) = operands.last().and_then(PdfObject::as_name) {
                            if let Some(xobjects) = doc
                                .resolve_dict_entry(resources, b"XObject")
                                .and_then(PdfObject::as_dict)
                            {
                                if let Some(xobj) = xobjects.get(name) {
                                    if let Some(stm) = doc.resolve(xobj).as_stream() {
                                        if let Some((img_w, img_h, rgba)) =
                                            crate::render::image::decode_image_xobject_to_rgba(stm)
                                        {
                                            let (sx0, sy0) =
                                                to_screen(Point::new(0.0, 0.0), &gs.ctm);
                                            let (sx1, sy1) =
                                                to_screen(Point::new(1.0, 1.0), &gs.ctm);
                                            blit_rgba_scaled(
                                                skia_pix, &rgba, img_w, img_h, sx0, sy0, sx1, sy1,
                                            );
                                        }
                                    }
                                }
                            }
                        }
                    }
                    _ => {}
                }
                operands.clear();
            }
        }
    }

    Ok(())
}

#[allow(clippy::too_many_arguments)]
fn blit_rgba_scaled(
    dst: &mut SkiaPixmap,
    src_rgba: &[u8],
    src_w: u32,
    src_h: u32,
    sx0: f32,
    sy0: f32,
    sx1: f32,
    sy1: f32,
) {
    let left = sx0.min(sx1).floor().max(0.0) as u32;
    let right = sx0.max(sx1).ceil().min(dst.width() as f32) as u32;
    let top = sy0.min(sy1).floor().max(0.0) as u32;
    let bottom = sy0.max(sy1).ceil().min(dst.height() as f32) as u32;
    if right <= left || bottom <= top || src_w == 0 || src_h == 0 {
        return;
    }

    let span_w = (right - left) as f32;
    let span_h = (bottom - top) as f32;
    let dst_w = dst.width() as usize;
    let dst_bytes = dst.data_mut();

    for py in top..bottom {
        let v = ((py - top) as f32 / span_h).clamp(0.0, 0.9999);
        let sy = ((v * (src_h as f32)) as u32).min(src_h - 1) as usize;
        for px in left..right {
            let u = ((px - left) as f32 / span_w).clamp(0.0, 0.9999);
            let sx = ((u * (src_w as f32)) as u32).min(src_w - 1) as usize;
            let src_idx = (sy * (src_w as usize) + sx) * 4;
            let dst_idx = ((py as usize) * dst_w + (px as usize)) * 4;
            if src_idx + 4 <= src_rgba.len() && dst_idx + 4 <= dst_bytes.len() {
                dst_bytes[dst_idx..dst_idx + 4].copy_from_slice(&src_rgba[src_idx..src_idx + 4]);
            }
        }
    }
}

fn find_first_embedded_ttf(
    doc: &Document,
    resources: &crate::parser::object::PdfDict,
) -> Option<Vec<u8>> {
    let fonts = doc
        .resolve_dict_entry(resources, b"Font")
        .and_then(PdfObject::as_dict)?;
    for f_ref in fonts.values() {
        let Some(f_dict) = doc.resolve(f_ref).as_dict() else {
            continue;
        };
        if let Some(desc_obj) = f_dict.get(b"FontDescriptor".as_slice()) {
            if let Some(desc_dict) = doc.resolve(desc_obj).as_dict() {
                if let Some(ff2) = desc_dict
                    .get(b"FontFile2".as_slice())
                    .or_else(|| desc_dict.get(b"FontFile3".as_slice()))
                {
                    if let Some(stm) = doc.resolve(ff2).as_stream() {
                        if let Ok(bytes) = stm.decode_data() {
                            return Some(bytes);
                        }
                    }
                }
            }
        }
    }
    None
}

fn draw_ttf_glyph(
    skia_pix: &mut SkiaPixmap,
    ttf_bytes: &[u8],
    ch: char,
    rect: crate::geometry::Rect,
    scale: f32,
) -> bool {
    use ab_glyph::{Font, FontRef, PxScale};
    let Ok(font) = FontRef::try_from_slice(ttf_bytes) else {
        return false;
    };
    let glyph_id = font.glyph_id(ch);
    if glyph_id.0 == 0 {
        return false;
    }
    let px_height = ((rect.height() as f32) * scale).max(4.0);
    let glyph = glyph_id.with_scale_and_position(
        PxScale::from(px_height),
        ab_glyph::point((rect.x0 as f32) * scale, (rect.y1 as f32) * scale),
    );
    let Some(outlined) = font.outline_glyph(glyph) else {
        return false;
    };
    let bounds = outlined.px_bounds();
    let dst_w = skia_pix.width() as i32;
    let dst_h = skia_pix.height() as i32;
    let dst_bytes = skia_pix.data_mut();
    outlined.draw(|gx, gy, cov| {
        if cov < 0.1 {
            return;
        }
        let px = bounds.min.x as i32 + gx as i32;
        let py = bounds.min.y as i32 + gy as i32;
        if px >= 0 && px < dst_w && py >= 0 && py < dst_h {
            let idx = ((py as usize) * (dst_w as usize) + (px as usize)) * 4;
            let shade = ((1.0 - cov.clamp(0.0, 1.0)) * 255.0).round() as u8;
            dst_bytes[idx] = dst_bytes[idx].min(shade);
            dst_bytes[idx + 1] = dst_bytes[idx + 1].min(shade);
            dst_bytes[idx + 2] = dst_bytes[idx + 2].min(shade);
            dst_bytes[idx + 3] = 255;
        }
    });
    true
}

#[inline]
fn rgb_to_u8(r: f64, g: f64, b: f64) -> [u8; 4] {
    [
        (r.clamp(0.0, 1.0) * 255.0).round() as u8,
        (g.clamp(0.0, 1.0) * 255.0).round() as u8,
        (b.clamp(0.0, 1.0) * 255.0).round() as u8,
        255,
    ]
}

fn draw_glyph_outline(
    skia_pix: &mut SkiaPixmap,
    _ch: char,
    rect: crate::geometry::Rect,
    scale: f32,
    paint: &Paint<'_>,
) {
    let x = (rect.x0 as f32) * scale + 0.5 * scale;
    let y = (rect.y0 as f32) * scale + (rect.height() as f32) * 0.2 * scale;
    let w = ((rect.width() as f32) * 0.75 * scale).max(1.0);
    let h = ((rect.height() as f32) * 0.65 * scale).max(1.0);
    if let Some(sk_rect) = SkiaRect::from_xywh(x, y, w, h) {
        skia_pix.fill_rect(sk_rect, paint, Transform::identity(), None);
    }
}
