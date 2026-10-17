use std::fmt::Write as _;

use rayon::prelude::*;

use crate::document::Document;
use crate::error::Result;
use crate::text::layout::{PageLayout, TextBlock};

/// Converts a single PDF page into structured Markdown.
pub fn page_to_markdown(doc: &Document, page_index: usize) -> Result<String> {
    let layout = doc.page_layout(page_index)?;
    Ok(layout_to_markdown(&layout))
}

/// Converts an entire PDF document into structured Markdown (parallelized across pages via `rayon`).
pub fn doc_to_markdown(doc: &Document) -> Result<String> {
    let page_mds: Result<Vec<String>> = (0..doc.page_count())
        .into_par_iter()
        .map(|idx| page_to_markdown(doc, idx))
        .collect();
    Ok(page_mds?.join("\n\n---\n\n"))
}

/// Converts a single PDF page into semantic HTML.
pub fn page_to_html(doc: &Document, page_index: usize) -> Result<String> {
    let layout = doc.page_layout(page_index)?;
    Ok(layout_to_html(&layout))
}

/// Converts an entire PDF document into semantic HTML (parallelized across pages via `rayon`).
pub fn doc_to_html(doc: &Document) -> Result<String> {
    let page_htmls: Result<Vec<String>> = (0..doc.page_count())
        .into_par_iter()
        .map(|idx| page_to_html(doc, idx))
        .collect();
    let body = page_htmls?.join("\n<hr/>\n");
    Ok(format!(
        "<!DOCTYPE html>\n<html>\n<body>\n{body}</body>\n</html>\n"
    ))
}

pub fn detect_body_font_size(blocks: &[TextBlock]) -> f64 {
    let mut sizes: Vec<f64> = blocks
        .iter()
        .flat_map(|b| b.lines.iter().map(|l| l.font_size))
        .collect();
    if sizes.is_empty() {
        return 12.0;
    }
    sizes.sort_by(|a, b| a.partial_cmp(b).unwrap_or(std::cmp::Ordering::Equal));
    // Lower median / 40th percentile represents body text even on heading-heavy pages
    let idx = (sizes.len() * 2) / 5;
    sizes[idx].max(1.0)
}

pub fn classify_heading_level(font_size: f64, body_size: f64) -> Option<u8> {
    let ratio = font_size / body_size.max(1.0);
    if ratio >= 1.75 {
        Some(1)
    } else if ratio >= 1.35 {
        Some(2)
    } else if ratio >= 1.15 {
        Some(3)
    } else {
        None
    }
}

pub fn layout_to_markdown(layout: &PageLayout) -> String {
    if layout.blocks.is_empty() {
        return String::new();
    }
    let body_size = detect_body_font_size(&layout.blocks);
    let mut sections: Vec<String> = Vec::with_capacity(layout.blocks.len());

    for block in &layout.blocks {
        let first_fs = block
            .lines
            .first()
            .map(|l| l.font_size)
            .unwrap_or(body_size);
        let text = block.text();
        let trimmed = text.trim();
        if trimmed.is_empty() {
            continue;
        }

        if let Some(level) = classify_heading_level(first_fs, body_size) {
            let prefix = "#".repeat(level as usize);
            let single_line = trimmed.replace('\n', " ");
            sections.push(format!("{prefix} {single_line}"));
        } else {
            let mut formatted_lines = Vec::new();
            for line in &block.lines {
                let lt = line.text();
                let ltrim = lt.trim();
                if let Some(rest) = ltrim
                    .strip_prefix("\u{2022} ")
                    .or_else(|| ltrim.strip_prefix("\u{2022}"))
                {
                    formatted_lines.push(format!("- {}", rest.trim()));
                } else {
                    formatted_lines.push(ltrim.to_string());
                }
            }
            sections.push(formatted_lines.join("\n"));
        }
    }

    sections.join("\n\n")
}

pub fn layout_to_html(layout: &PageLayout) -> String {
    if layout.blocks.is_empty() {
        return String::new();
    }
    let body_size = detect_body_font_size(&layout.blocks);
    let mut out = String::new();

    for block in &layout.blocks {
        let first_fs = block
            .lines
            .first()
            .map(|l| l.font_size)
            .unwrap_or(body_size);
        let text = block.text();
        let trimmed = text.trim();
        if trimmed.is_empty() {
            continue;
        }

        if let Some(level) = classify_heading_level(first_fs, body_size) {
            let single = escape_html(&trimmed.replace('\n', " "));
            let _ = writeln!(out, "<h{level}>{single}</h{level}>");
        } else {
            let all_bullets = block.lines.iter().all(|l| {
                let t = l.text();
                let tr = t.trim();
                tr.starts_with("- ") || tr.starts_with("* ") || tr.starts_with('\u{2022}')
            });
            if all_bullets && !block.lines.is_empty() {
                out.push_str("<ul>\n");
                for line in &block.lines {
                    let lt = line.text();
                    let item = lt.trim().trim_start_matches(['-', '*', '\u{2022}', ' ']);
                    let _ = writeln!(out, "  <li>{}</li>", escape_html(item));
                }
                out.push_str("</ul>\n");
            } else {
                let _ = writeln!(out, "<p>{}</p>", escape_html(trimmed));
            }
        }
    }

    out
}

fn escape_html(s: &str) -> String {
    let mut out = String::with_capacity(s.len());
    for c in s.chars() {
        match c {
            '&' => out.push_str("&amp;"),
            '<' => out.push_str("&lt;"),
            '>' => out.push_str("&gt;"),
            '"' => out.push_str("&quot;"),
            other => out.push(other),
        }
    }
    out
}
