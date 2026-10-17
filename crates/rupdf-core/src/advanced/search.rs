use aho_corasick::AhoCorasickBuilder;
use rayon::prelude::*;
use regex::RegexBuilder;

use crate::document::Document;
use crate::error::{Result, RupdfError};
use crate::geometry::Rect;
use crate::render::{render_page, Pixmap};
use crate::text::PageLayout;

pub type PageSearchHits = (usize, Vec<Rect>);

/// Searches all pages of `doc` in parallel using SIMD substring matching (`aho-corasick`)
/// or SIMD-accelerated regular expressions (`regex`).
pub fn search_document(
    doc: &Document,
    query: &str,
    use_regex: bool,
) -> Result<Vec<PageSearchHits>> {
    if query.trim().is_empty() {
        return Ok(Vec::new());
    }

    if use_regex {
        let re = RegexBuilder::new(query)
            .case_insensitive(true)
            .build()
            .map_err(|e| RupdfError::InvalidArgument(format!("Invalid regex pattern: {e}")))?;

        let results: Result<Vec<Option<PageSearchHits>>> = (0..doc.page_count())
            .into_par_iter()
            .map(|page_idx| {
                let layout = doc.page_layout(page_idx)?;
                let rects = search_layout_regex(&layout, &re);
                if rects.is_empty() {
                    Ok(None)
                } else {
                    Ok(Some((page_idx, rects)))
                }
            })
            .collect();

        Ok(results?.into_iter().flatten().collect())
    } else {
        let ac = AhoCorasickBuilder::new()
            .ascii_case_insensitive(true)
            .build([query])
            .map_err(|e| RupdfError::InvalidArgument(e.to_string()))?;

        let results: Result<Vec<Option<PageSearchHits>>> = (0..doc.page_count())
            .into_par_iter()
            .map(|page_idx| {
                let layout = doc.page_layout(page_idx)?;
                let rects = search_layout_aho_corasick(&layout, &ac, query);
                if rects.is_empty() {
                    Ok(None)
                } else {
                    Ok(Some((page_idx, rects)))
                }
            })
            .collect();

        Ok(results?.into_iter().flatten().collect())
    }
}

fn search_layout_aho_corasick(
    layout: &PageLayout,
    ac: &aho_corasick::AhoCorasick,
    fallback_needle: &str,
) -> Vec<Rect> {
    let mut hits = Vec::new();
    for block in &layout.blocks {
        for line in &block.lines {
            if line.chars.is_empty() {
                continue;
            }
            let mut text = String::with_capacity(line.chars.len());
            let mut byte_to_char_idx = Vec::with_capacity(line.chars.len());
            for (char_idx, &(ch, _)) in line.chars.iter().enumerate() {
                let len = ch.len_utf8();
                text.push(ch);
                for _ in 0..len {
                    byte_to_char_idx.push(char_idx);
                }
            }

            for mat in ac.find_iter(&text) {
                if mat.start() < byte_to_char_idx.len() && mat.end() > mat.start() {
                    let c_start = byte_to_char_idx[mat.start()];
                    let c_end = byte_to_char_idx[mat.end() - 1];
                    let mut rect = line.chars[c_start].1;
                    for &(_, r) in &line.chars[c_start + 1..=c_end] {
                        rect = rect.union(&r);
                    }
                    hits.push(rect);
                }
            }
        }
    }
    if hits.is_empty() {
        layout.search(fallback_needle)
    } else {
        hits
    }
}

fn search_layout_regex(layout: &PageLayout, re: &regex::Regex) -> Vec<Rect> {
    let mut hits = Vec::new();
    for block in &layout.blocks {
        for line in &block.lines {
            if line.chars.is_empty() {
                continue;
            }
            let mut text = String::with_capacity(line.chars.len());
            let mut byte_to_char_idx = Vec::with_capacity(line.chars.len());
            for (char_idx, &(ch, _)) in line.chars.iter().enumerate() {
                let len = ch.len_utf8();
                text.push(ch);
                for _ in 0..len {
                    byte_to_char_idx.push(char_idx);
                }
            }

            for mat in re.find_iter(&text) {
                if mat.start() < byte_to_char_idx.len() && mat.end() > mat.start() {
                    let c_start = byte_to_char_idx[mat.start()];
                    let c_end = byte_to_char_idx[mat.end() - 1];
                    let mut rect = line.chars[c_start].1;
                    for &(_, r) in &line.chars[c_start + 1..=c_end] {
                        rect = rect.union(&r);
                    }
                    hits.push(rect);
                }
            }
        }
    }
    hits
}

/// Extracts plain text across a batch of PDF byte buffers concurrently on the `rayon` thread pool.
pub fn batch_extract_bytes(pdf_buffers: &[Vec<u8>]) -> Result<Vec<String>> {
    pdf_buffers
        .par_iter()
        .map(|bytes| {
            let doc = Document::from_bytes(bytes)?;
            let page_texts: Result<Vec<String>> = (0..doc.page_count())
                .map(|i| doc.extract_page_text(i))
                .collect();
            Ok(page_texts?.join("\n"))
        })
        .collect()
}

/// Renders all pages of `doc` in parallel across CPU cores on the `rayon` thread pool.
pub fn batch_render_pages(doc: &Document, dpi: f64) -> Result<Vec<Pixmap>> {
    (0..doc.page_count())
        .into_par_iter()
        .map(|idx| render_page(doc, idx, dpi))
        .collect()
}
