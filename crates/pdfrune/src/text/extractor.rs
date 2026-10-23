use std::collections::BTreeMap;

use crate::document::Document;
use crate::error::Result;
use crate::geometry::{Matrix, Point, Rect};
use crate::parser::lexer::{ContentToken, Lexer};
use crate::parser::object::{PdfDict, PdfDictExt, PdfObject};
use crate::text::cmap::FontInfo;

/// A positioned text span extracted from a PDF content stream.
#[derive(Debug, Clone)]
pub struct PositionedSpan {
    /// Bounding box in top-down page coordinates `(x0, y0, x1, y1)`.
    pub bbox: Rect,
    /// Decoded UTF-8 text of the span.
    pub text: String,
    /// Font resource name or base font name.
    pub font_name: String,
    /// Effective font size in points.
    pub font_size: f64,
    /// Per-character bounding boxes in top-down page coordinates.
    pub chars: Vec<(char, Rect)>,
    /// PDF text rendering mode; 3 is invisible OCR, 7 is clip-only.
    pub render_mode: u8,
}

#[derive(Debug, Clone)]
struct GraphicsState {
    ctm: Matrix,
    font_name: String,
    font_size: f64,
    char_space: f64,
    word_space: f64,
    horiz_scale: f64,
    leading: f64,
    text_rise: f64,
    render_mode: u8,
}

impl Default for GraphicsState {
    fn default() -> Self {
        Self {
            ctm: Matrix::identity(),
            font_name: "F1".to_string(),
            font_size: 12.0,
            char_space: 0.0,
            word_space: 0.0,
            horiz_scale: 100.0,
            leading: 0.0,
            text_rise: 0.0,
            render_mode: 0,
        }
    }
}

/// Extracts all positioned text spans from a page's content stream(s).
pub fn extract_page_spans(doc: &Document, page_index: usize) -> Result<Vec<PositionedSpan>> {
    let page = doc.page(page_index)?;
    let page_height = page.cropbox.height().max(1.0);
    let crop_x0 = page.cropbox.x0;
    let crop_y0 = page.cropbox.y0;
    let content_bytes = doc.page_content_bytes(page_index)?;
    if content_bytes.is_empty() {
        return Ok(Vec::new());
    }

    let fonts = load_fonts_from_resources(doc, &page.resources);
    let mut spans = Vec::new();
    let mut gs = GraphicsState::default();
    let mut gs_stack: Vec<GraphicsState> = Vec::new();

    interpret_stream(
        doc,
        &content_bytes,
        &page.resources,
        &fonts,
        page_height,
        crop_x0,
        crop_y0,
        &mut gs,
        &mut gs_stack,
        &mut spans,
        0,
    )?;

    Ok(spans)
}

pub fn load_fonts_from_resources(
    doc: &Document,
    resources: &PdfDict,
) -> BTreeMap<String, FontInfo> {
    let mut fonts = BTreeMap::new();
    if let Some(font_dict) = doc
        .resolve_dict_entry(resources, b"Font")
        .and_then(PdfObject::as_dict)
    {
        for (name_bytes, font_ref) in font_dict {
            let name = String::from_utf8_lossy(name_bytes).into_owned();
            if let Some(f_dict) = doc.resolve(font_ref).as_dict() {
                fonts.insert(name.clone(), FontInfo::from_font_dict(doc, &name, f_dict));
            }
        }
    }
    fonts
}

#[allow(clippy::too_many_arguments)]
fn interpret_stream(
    doc: &Document,
    stream_bytes: &[u8],
    resources: &PdfDict,
    fonts: &BTreeMap<String, FontInfo>,
    page_height: f64,
    crop_x0: f64,
    crop_y0: f64,
    gs: &mut GraphicsState,
    gs_stack: &mut Vec<GraphicsState>,
    spans: &mut Vec<PositionedSpan>,
    depth: usize,
) -> Result<()> {
    if depth > 8 {
        return Ok(());
    }

    let mut lexer = Lexer::new(stream_bytes);
    let tokens = lexer.tokenize_content_stream()?;
    let mut operands: Vec<PdfObject> = Vec::with_capacity(8);

    let mut text_matrix = Matrix::identity();
    let mut text_line_matrix = Matrix::identity();

    for token in tokens {
        match token {
            ContentToken::Operand(obj) => operands.push(obj),
            ContentToken::Operator(op) => {
                match op.as_slice() {
                    b"q" => gs_stack.push(gs.clone()),
                    b"Q" => {
                        if let Some(prev) = gs_stack.pop() {
                            *gs = prev;
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
                    b"BT" => {
                        text_matrix = Matrix::identity();
                        text_line_matrix = Matrix::identity();
                    }
                    b"ET" => {}
                    b"Tf" => {
                        if operands.len() >= 2 {
                            let n = operands.len();
                            if let Some(name) = operands[n - 2].as_name_str() {
                                gs.font_name = name.to_string();
                            }
                            if let Some(size) = operands[n - 1].as_f64() {
                                gs.font_size = size;
                            }
                        }
                    }
                    b"Tc" => {
                        if let Some(v) = operands.last().and_then(PdfObject::as_f64) {
                            gs.char_space = v;
                        }
                    }
                    b"Tw" => {
                        if let Some(v) = operands.last().and_then(PdfObject::as_f64) {
                            gs.word_space = v;
                        }
                    }
                    b"Tz" => {
                        if let Some(v) = operands.last().and_then(PdfObject::as_f64) {
                            gs.horiz_scale = v;
                        }
                    }
                    b"TL" => {
                        if let Some(v) = operands.last().and_then(PdfObject::as_f64) {
                            gs.leading = v;
                        }
                    }
                    b"Tr" => {
                        if let Some(v) = operands.last().and_then(PdfObject::as_i64) {
                            if (0..=7).contains(&v) {
                                gs.render_mode = v as u8;
                            }
                        }
                    }
                    b"Ts" => {
                        if let Some(v) = operands.last().and_then(PdfObject::as_f64) {
                            gs.text_rise = v;
                        }
                    }
                    b"Td" => {
                        if operands.len() >= 2 {
                            let n = operands.len();
                            let tx = operands[n - 2].as_f64().unwrap_or(0.0);
                            let ty = operands[n - 1].as_f64().unwrap_or(0.0);
                            text_line_matrix =
                                Matrix::translate(tx, ty).multiply(&text_line_matrix);
                            text_matrix = text_line_matrix;
                        }
                    }
                    b"TD" => {
                        if operands.len() >= 2 {
                            let n = operands.len();
                            let tx = operands[n - 2].as_f64().unwrap_or(0.0);
                            let ty = operands[n - 1].as_f64().unwrap_or(0.0);
                            gs.leading = -ty;
                            text_line_matrix =
                                Matrix::translate(tx, ty).multiply(&text_line_matrix);
                            text_matrix = text_line_matrix;
                        }
                    }
                    b"Tm" => {
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
                            text_line_matrix = m;
                            text_matrix = m;
                        }
                    }
                    b"T*" => {
                        text_line_matrix =
                            Matrix::translate(0.0, -gs.leading).multiply(&text_line_matrix);
                        text_matrix = text_line_matrix;
                    }
                    b"Tj" => {
                        if let Some(last) = operands.last() {
                            if let Some(bytes) = last.as_bytes() {
                                emit_text_bytes(
                                    bytes,
                                    gs,
                                    fonts,
                                    &mut text_matrix,
                                    page_height,
                                    crop_x0,
                                    crop_y0,
                                    spans,
                                );
                            }
                        }
                    }
                    b"'" => {
                        text_line_matrix =
                            Matrix::translate(0.0, -gs.leading).multiply(&text_line_matrix);
                        text_matrix = text_line_matrix;
                        if let Some(last) = operands.last() {
                            if let Some(bytes) = last.as_bytes() {
                                emit_text_bytes(
                                    bytes,
                                    gs,
                                    fonts,
                                    &mut text_matrix,
                                    page_height,
                                    crop_x0,
                                    crop_y0,
                                    spans,
                                );
                            }
                        }
                    }
                    b"\"" => {
                        if operands.len() >= 3 {
                            let n = operands.len();
                            gs.word_space = operands[n - 3].as_f64().unwrap_or(gs.word_space);
                            gs.char_space = operands[n - 2].as_f64().unwrap_or(gs.char_space);
                            text_line_matrix =
                                Matrix::translate(0.0, -gs.leading).multiply(&text_line_matrix);
                            text_matrix = text_line_matrix;
                            if let Some(bytes) = operands[n - 1].as_bytes() {
                                emit_text_bytes(
                                    bytes,
                                    gs,
                                    fonts,
                                    &mut text_matrix,
                                    page_height,
                                    crop_x0,
                                    crop_y0,
                                    spans,
                                );
                            }
                        }
                    }
                    b"TJ" => {
                        if let Some(arr) = operands.last().and_then(PdfObject::as_array) {
                            emit_tj_array(
                                arr,
                                gs,
                                fonts,
                                &mut text_matrix,
                                page_height,
                                crop_x0,
                                crop_y0,
                                spans,
                            );
                        }
                    }
                    b"Do" => {
                        // Execute Form XObject if referenced
                        if let Some(xobj_name) = operands.last().and_then(PdfObject::as_name) {
                            if let Some(xobj_dict) = doc
                                .resolve_dict_entry(resources, b"XObject")
                                .and_then(PdfObject::as_dict)
                            {
                                if let Some(xobj) = doc.resolve_dict_entry(xobj_dict, xobj_name) {
                                    if let Some(stm) = xobj.as_stream() {
                                        if stm.dict.get_name(b"Subtype") == Some(b"Form") {
                                            if let Ok(decoded) = stm.decode_data() {
                                                let mut sub_res = resources.clone();
                                                if let Some(r) = doc
                                                    .resolve_dict_entry(&stm.dict, b"Resources")
                                                    .and_then(PdfObject::as_dict)
                                                {
                                                    for (k, v) in r {
                                                        sub_res.insert(k.clone(), v.clone());
                                                    }
                                                }
                                                let sub_fonts =
                                                    load_fonts_from_resources(doc, &sub_res);
                                                gs_stack.push(gs.clone());
                                                let _ = interpret_stream(
                                                    doc,
                                                    &decoded,
                                                    &sub_res,
                                                    &sub_fonts,
                                                    page_height,
                                                    crop_x0,
                                                    crop_y0,
                                                    gs,
                                                    gs_stack,
                                                    spans,
                                                    depth + 1,
                                                );
                                                if let Some(prev) = gs_stack.pop() {
                                                    *gs = prev;
                                                }
                                            }
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
fn emit_tj_array(
    arr: &[PdfObject],
    gs: &GraphicsState,
    fonts: &BTreeMap<String, FontInfo>,
    text_matrix: &mut Matrix,
    page_height: f64,
    crop_x0: f64,
    crop_y0: f64,
    spans: &mut Vec<PositionedSpan>,
) {
    let scale = gs.horiz_scale / 100.0;
    for item in arr {
        match item {
            PdfObject::String(bytes) | PdfObject::HexString(bytes) => {
                emit_text_bytes(
                    bytes,
                    gs,
                    fonts,
                    text_matrix,
                    page_height,
                    crop_x0,
                    crop_y0,
                    spans,
                );
            }
            PdfObject::Int(adj) => {
                let tx = -((*adj as f64) / 1000.0) * gs.font_size * scale;
                *text_matrix = Matrix::translate(tx, 0.0).multiply(text_matrix);
            }
            PdfObject::Real(adj) => {
                let tx = -(*adj / 1000.0) * gs.font_size * scale;
                *text_matrix = Matrix::translate(tx, 0.0).multiply(text_matrix);
            }
            _ => {}
        }
    }
}

#[allow(clippy::too_many_arguments)]
fn emit_text_bytes(
    bytes: &[u8],
    gs: &GraphicsState,
    fonts: &BTreeMap<String, FontInfo>,
    text_matrix: &mut Matrix,
    page_height: f64,
    crop_x0: f64,
    crop_y0: f64,
    spans: &mut Vec<PositionedSpan>,
) {
    let default_font = FontInfo::default();
    let font = fonts.get(&gs.font_name).unwrap_or(&default_font);
    let decoded_glyphs = font.decode_bytes(bytes);
    if decoded_glyphs.is_empty() {
        return;
    }

    let scale = gs.horiz_scale / 100.0;
    let combined_tm = text_matrix.multiply(&gs.ctm);
    let font_scale = (combined_tm.a.hypot(combined_tm.b)).max(1.0);
    let effective_font_size = (gs.font_size * font_scale).abs().max(1.0);

    let mut span_text = String::new();
    let mut chars = Vec::new();
    let mut span_rect: Option<Rect> = None;

    for (code, glyph_str, width_1000) in decoded_glyphs {
        let cur_tm = text_matrix.multiply(&gs.ctm);
        let origin_pdf = cur_tm.transform_point(Point::new(0.0, gs.text_rise));

        let w_text_space = (width_1000 / 1000.0) * gs.font_size * scale;
        let extra_space = if code == 32 { gs.word_space } else { 0.0 };
        let tx = w_text_space + (gs.char_space + extra_space) * scale;

        let end_tm = Matrix::translate(w_text_space.max(1.0), 0.0)
            .multiply(text_matrix)
            .multiply(&gs.ctm);
        let end_pdf = end_tm.transform_point(Point::new(0.0, gs.text_rise));

        // Convert bottom-up PDF user space to top-down page coordinates
        let x0 = (origin_pdf.x - crop_x0).min(end_pdf.x - crop_x0);
        let x1 = (origin_pdf.x - crop_x0)
            .max(end_pdf.x - crop_x0)
            .max(x0 + 1.0);
        let baseline_y = page_height - (origin_pdf.y - crop_y0);
        let y0 = (baseline_y - effective_font_size * 0.85).max(0.0);
        let y1 = baseline_y + effective_font_size * 0.15;

        let glyph_rect = Rect::new(x0, y0, x1, y1);
        let char_count = glyph_str.chars().count().max(1) as f64;
        let char_w = glyph_rect.width() / char_count;

        for (idx, ch) in glyph_str.chars().enumerate() {
            let cx0 = glyph_rect.x0 + (idx as f64) * char_w;
            let cx1 = cx0 + char_w;
            let crect = Rect::new(cx0, glyph_rect.y0, cx1, glyph_rect.y1);
            chars.push((ch, crect));
        }

        span_rect = Some(match span_rect {
            Some(r) => r.union(&glyph_rect),
            None => glyph_rect,
        });
        span_text.push_str(&glyph_str);

        *text_matrix = Matrix::translate(tx, 0.0).multiply(text_matrix);
    }

    if !span_text.is_empty() {
        if let Some(bbox) = span_rect {
            let font_label = if font.base_font.is_empty() {
                gs.font_name.clone()
            } else {
                font.base_font.clone()
            };
            spans.push(PositionedSpan {
                bbox,
                text: span_text,
                font_name: font_label,
                font_size: effective_font_size,
                chars,
                render_mode: gs.render_mode,
            });
        }
    }
}
