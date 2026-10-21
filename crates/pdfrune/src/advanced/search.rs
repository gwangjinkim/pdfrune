use aho_corasick::AhoCorasickBuilder;
use rayon::prelude::*;
use regex::RegexBuilder;

use crate::document::Document;
use crate::error::{PdfruneError, Result};
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
            .map_err(|e| PdfruneError::InvalidArgument(format!("Invalid regex pattern: {e}")))?;

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
            .map_err(|e| PdfruneError::InvalidArgument(e.to_string()))?;

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

/// Performance metrics returned by `run_benchmark`.
#[derive(Debug, Clone)]
pub struct BenchmarkMetrics {
    pub pages: usize,
    pub parse_ms: f64,
    pub extract_ms: f64,
    pub search_ms: f64,
    pub markdown_ms: f64,
    pub render_ms: f64,
    pub pages_per_sec_extract: f64,
    pub pages_per_sec_render: f64,
}

/// Runs an internal multi-core benchmark across `pages` synthesized pages.
pub fn run_benchmark(pages: usize) -> Result<BenchmarkMetrics> {
    use crate::geometry::Point;
    use std::time::Instant;

    let page_count = pages.max(1);
    let mut builder = Document::new();
    for i in 0..page_count {
        let pno = builder.new_page(None, 612.0, 792.0)?;
        builder.insert_text(
            pno,
            Point::new(72.0, 72.0),
            &format!("Section {i} Benchmark Header"),
            22.0,
            [0.1, 0.2, 0.5],
        )?;
        builder.insert_text(
            pno,
            Point::new(72.0, 120.0),
            "pdfrune delivers memory-safe, SIMD-accelerated PDF parsing and rendering in Rust.",
            12.0,
            [0.0, 0.0, 0.0],
        )?;
        builder.draw_rect(
            pno,
            Rect::new(72.0, 150.0, 400.0, 220.0),
            Some([0.2, 0.3, 0.7]),
            Some([0.9, 0.95, 1.0]),
            1.5,
        )?;
    }
    let raw_bytes = builder.to_bytes(true)?;

    let t0 = Instant::now();
    let doc = Document::from_bytes(&raw_bytes)?;
    let parse_ms = t0.elapsed().as_secs_f64() * 1000.0;

    let t1 = Instant::now();
    let _: Result<Vec<String>> = (0..doc.page_count())
        .into_par_iter()
        .map(|i| doc.extract_page_text(i))
        .collect();
    let extract_ms = t1.elapsed().as_secs_f64() * 1000.0;

    let t2 = Instant::now();
    let _ = doc.search_all("SIMD-accelerated", false)?;
    let search_ms = t2.elapsed().as_secs_f64() * 1000.0;

    let t3 = Instant::now();
    let _ = doc.to_markdown()?;
    let markdown_ms = t3.elapsed().as_secs_f64() * 1000.0;

    let t4 = Instant::now();
    let _ = doc.render_all_pages(72.0)?;
    let render_ms = t4.elapsed().as_secs_f64() * 1000.0;

    let pages_per_sec_extract = (page_count as f64) / (extract_ms / 1000.0).max(1e-6);
    let pages_per_sec_render = (page_count as f64) / (render_ms / 1000.0).max(1e-6);

    Ok(BenchmarkMetrics {
        pages: page_count,
        parse_ms,
        extract_ms,
        search_ms,
        markdown_ms,
        render_ms,
        pages_per_sec_extract,
        pages_per_sec_render,
    })
}
