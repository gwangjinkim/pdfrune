use std::fmt::Write as _;

use crate::document::Document;
use crate::error::Result;
use crate::geometry::{Matrix, Point};
use crate::parser::lexer::{ContentToken, Lexer};
use crate::parser::object::PdfObject;
use crate::text::extractor::extract_page_spans;

/// Renders the page at `page_index` into a scalable vector SVG document string.
pub fn render_page_svg(doc: &Document, page_index: usize) -> Result<String> {
    let page = doc.page(page_index)?;
    let w = page.width();
    let h = page.height();
    let page_height_pt = page.cropbox.height().max(1.0);
    let crop_x0 = page.cropbox.x0;
    let crop_y0 = page.cropbox.y0;

    let mut svg = String::with_capacity(2048);
    let _ = writeln!(
        svg,
        r#"<svg xmlns="http://www.w3.org/2000/svg" width="{}" height="{}" viewBox="0 0 {} {}">"#,
        format_num(w),
        format_num(h),
        format_num(w),
        format_num(h)
    );
    svg.push_str("  <rect width=\"100%\" height=\"100%\" fill=\"#ffffff\"/>\n");

    // Emit vector paths from content stream
    if let Ok(content_bytes) = doc.page_content_bytes(page_index) {
        if !content_bytes.is_empty() {
            emit_svg_paths(&content_bytes, page_height_pt, crop_x0, crop_y0, &mut svg);
        }
    }

    // Emit positioned text spans
    if let Ok(spans) = extract_page_spans(doc, page_index) {
        for span in spans {
            if matches!(span.render_mode, 3 | 7) {
                continue;
            }
            let escaped = escape_xml(&span.text);
            let x = span.bbox.x0;
            let y = span.bbox.y1 - span.font_size * 0.15;
            let _ = writeln!(
                svg,
                r#"  <text x="{:.2}" y="{:.2}" font-family="{}" font-size="{:.2}">{}</text>"#,
                x, y, span.font_name, span.font_size, escaped
            );
        }
    }

    svg.push_str("</svg>\n");
    Ok(svg)
}

fn emit_svg_paths(
    content_bytes: &[u8],
    page_height_pt: f64,
    crop_x0: f64,
    crop_y0: f64,
    svg: &mut String,
) {
    let mut lexer = Lexer::new(content_bytes);
    let Ok(tokens) = lexer.tokenize_content_stream() else {
        return;
    };

    let mut operands: Vec<PdfObject> = Vec::new();
    let mut ctm = Matrix::identity();
    let mut stroke_hex = String::from("#000000");
    let mut fill_hex = String::from("#000000");
    let mut line_width = 1.0f64;
    let mut path_d = String::new();

    let to_svg_pt = |pt: Point, m: &Matrix| -> (f64, f64) {
        let p = m.transform_point(pt);
        (p.x - crop_x0, page_height_pt - (p.y - crop_y0))
    };

    for token in tokens {
        match token {
            ContentToken::Operand(obj) => operands.push(obj),
            ContentToken::Operator(op) => {
                match op.as_slice() {
                    b"cm" if operands.len() >= 6 => {
                        let n = operands.len();
                        let m = Matrix::new(
                            operands[n - 6].as_f64().unwrap_or(1.0),
                            operands[n - 5].as_f64().unwrap_or(0.0),
                            operands[n - 4].as_f64().unwrap_or(0.0),
                            operands[n - 3].as_f64().unwrap_or(1.0),
                            operands[n - 2].as_f64().unwrap_or(0.0),
                            operands[n - 1].as_f64().unwrap_or(0.0),
                        );
                        ctm = m.multiply(&ctm);
                    }
                    b"w" => {
                        if let Some(w) = operands.last().and_then(PdfObject::as_f64) {
                            line_width = w.max(0.1);
                        }
                    }
                    b"RG" if operands.len() >= 3 => {
                        let n = operands.len();
                        stroke_hex = rgb_to_hex(
                            operands[n - 3].as_f64().unwrap_or(0.0),
                            operands[n - 2].as_f64().unwrap_or(0.0),
                            operands[n - 1].as_f64().unwrap_or(0.0),
                        );
                    }
                    b"rg" if operands.len() >= 3 => {
                        let n = operands.len();
                        fill_hex = rgb_to_hex(
                            operands[n - 3].as_f64().unwrap_or(0.0),
                            operands[n - 2].as_f64().unwrap_or(0.0),
                            operands[n - 1].as_f64().unwrap_or(0.0),
                        );
                    }
                    b"m" if operands.len() >= 2 => {
                        let n = operands.len();
                        let (sx, sy) = to_svg_pt(
                            Point::new(
                                operands[n - 2].as_f64().unwrap_or(0.0),
                                operands[n - 1].as_f64().unwrap_or(0.0),
                            ),
                            &ctm,
                        );
                        let _ = write!(path_d, "M {sx:.2} {sy:.2} ");
                    }
                    b"l" if operands.len() >= 2 => {
                        let n = operands.len();
                        let (sx, sy) = to_svg_pt(
                            Point::new(
                                operands[n - 2].as_f64().unwrap_or(0.0),
                                operands[n - 1].as_f64().unwrap_or(0.0),
                            ),
                            &ctm,
                        );
                        let _ = write!(path_d, "L {sx:.2} {sy:.2} ");
                    }
                    b"h" => path_d.push_str("Z "),
                    b"re" if operands.len() >= 4 => {
                        let n = operands.len();
                        let rx = operands[n - 4].as_f64().unwrap_or(0.0);
                        let ry = operands[n - 3].as_f64().unwrap_or(0.0);
                        let rw = operands[n - 2].as_f64().unwrap_or(0.0);
                        let rh = operands[n - 1].as_f64().unwrap_or(0.0);
                        let (x0, y0) = to_svg_pt(Point::new(rx, ry), &ctm);
                        let (x1, y1) = to_svg_pt(Point::new(rx + rw, ry + rh), &ctm);
                        let left = x0.min(x1);
                        let top = y0.min(y1);
                        let width = (x1 - x0).abs();
                        let height = (y1 - y0).abs();
                        let _ = write!(
                            path_d,
                            "M {left:.2} {top:.2} h {width:.2} v {height:.2} h -{width:.2} Z "
                        );
                    }
                    b"S" | b"s" => {
                        if !path_d.is_empty() {
                            let d = std::mem::take(&mut path_d);
                            let _ = writeln!(
                                svg,
                                r#"  <path d="{}" fill="none" stroke="{stroke_hex}" stroke-width="{line_width:.2}"/>"#,
                                d.trim()
                            );
                        }
                    }
                    b"f" | b"F" | b"f*" => {
                        if !path_d.is_empty() {
                            let d = std::mem::take(&mut path_d);
                            let _ = writeln!(
                                svg,
                                r#"  <path d="{}" fill="{fill_hex}" stroke="none"/>"#,
                                d.trim()
                            );
                        }
                    }
                    b"B" | b"B*" | b"b" | b"b*" => {
                        if !path_d.is_empty() {
                            let d = std::mem::take(&mut path_d);
                            let _ = writeln!(
                                svg,
                                r#"  <path d="{}" fill="{fill_hex}" stroke="{stroke_hex}" stroke-width="{line_width:.2}"/>"#,
                                d.trim()
                            );
                        }
                    }
                    b"n" => path_d.clear(),
                    _ => {}
                }
                operands.clear();
            }
        }
    }
}

fn format_num(v: f64) -> String {
    if v.fract().abs() < 1e-6 {
        format!("{}", v.round() as i64)
    } else {
        format!("{v:.2}")
    }
}

fn rgb_to_hex(r: f64, g: f64, b: f64) -> String {
    let ru = (r.clamp(0.0, 1.0) * 255.0).round() as u8;
    let gu = (g.clamp(0.0, 1.0) * 255.0).round() as u8;
    let bu = (b.clamp(0.0, 1.0) * 255.0).round() as u8;
    format!("#{ru:02x}{gu:02x}{bu:02x}")
}

fn escape_xml(s: &str) -> String {
    let mut out = String::with_capacity(s.len());
    for c in s.chars() {
        match c {
            '&' => out.push_str("&amp;"),
            '<' => out.push_str("&lt;"),
            '>' => out.push_str("&gt;"),
            '"' => out.push_str("&quot;"),
            '\'' => out.push_str("&apos;"),
            other => out.push(other),
        }
    }
    out
}
