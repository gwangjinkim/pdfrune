use std::fmt::Write as _;

use crate::document::Document;
use crate::error::Result;
use crate::geometry::Rect;
use crate::text::extractor::{extract_page_spans, PositionedSpan};

/// A structured table detected on a PDF page.
#[derive(Debug, Clone, PartialEq)]
pub struct DetectedTable {
    pub bbox: Rect,
    pub rows: Vec<Vec<String>>,
}

impl DetectedTable {
    pub fn row_count(&self) -> usize {
        self.rows.len()
    }

    pub fn col_count(&self) -> usize {
        self.rows.first().map(Vec::len).unwrap_or(0)
    }

    pub fn header(&self) -> Vec<String> {
        self.rows.first().cloned().unwrap_or_default()
    }

    pub fn extract(&self) -> Vec<Vec<String>> {
        self.rows.clone()
    }

    pub fn to_dicts(&self) -> Vec<Vec<(String, String)>> {
        let hdr = self.header();
        self.rows
            .iter()
            .skip(1)
            .map(|row| {
                hdr.iter()
                    .cloned()
                    .zip(row.iter().cloned())
                    .collect::<Vec<(String, String)>>()
            })
            .collect()
    }

    /// Formats the table as a GitHub-Flavored Markdown table.
    pub fn to_markdown(&self) -> String {
        if self.rows.is_empty() {
            return String::new();
        }
        let mut out = String::new();
        let cols = self.col_count();
        if let Some(hdr) = self.rows.first() {
            let _ = writeln!(out, "| {} |", hdr.join(" | "));
            let sep: Vec<&str> = (0..cols).map(|_| "---").collect();
            let _ = writeln!(out, "| {} |", sep.join(" | "));
        }
        for row in self.rows.iter().skip(1) {
            let _ = writeln!(out, "| {} |", row.join(" | "));
        }
        out.trim_end().to_string()
    }
}

#[derive(Debug, Clone)]
struct CandidateRow {
    y_center: f64,
    bbox: Rect,
    cells: Vec<(Rect, String)>,
}

/// Detects structured tables on `page_index` by clustering horizontally aligned multi-column rows.
pub fn find_page_tables(doc: &Document, page_index: usize) -> Result<Vec<DetectedTable>> {
    let mut spans = extract_page_spans(doc, page_index)?;
    spans.retain(|s| !s.text.trim().is_empty());
    if spans.is_empty() {
        return Ok(Vec::new());
    }

    // Sort top-to-bottom, then left-to-right
    spans.sort_by(|a, b| {
        a.bbox
            .y0
            .partial_cmp(&b.bbox.y0)
            .unwrap_or(std::cmp::Ordering::Equal)
            .then_with(|| {
                a.bbox
                    .x0
                    .partial_cmp(&b.bbox.x0)
                    .unwrap_or(std::cmp::Ordering::Equal)
            })
    });

    // Group spans into horizontal visual rows
    let mut raw_rows: Vec<Vec<PositionedSpan>> = Vec::new();
    for span in spans {
        let tol = (span.font_size * 0.5).max(4.0);
        if let Some(last_row) = raw_rows.last_mut() {
            let row_y = last_row[0].bbox.y0;
            if (span.bbox.y0 - row_y).abs() <= tol {
                last_row.push(span);
                continue;
            }
        }
        raw_rows.push(vec![span]);
    }

    // Within each row, sort by x0 and merge closely spaced words while keeping column gaps separate
    let mut candidate_rows: Vec<CandidateRow> = Vec::new();
    for mut r_spans in raw_rows {
        r_spans.sort_by(|a, b| {
            a.bbox
                .x0
                .partial_cmp(&b.bbox.x0)
                .unwrap_or(std::cmp::Ordering::Equal)
        });
        let mut cells: Vec<(Rect, String)> = Vec::new();
        let mut row_bbox = r_spans[0].bbox;

        for sp in r_spans {
            row_bbox = row_bbox.union(&sp.bbox);
            let gap_threshold = (sp.font_size * 1.5).max(14.0);
            if let Some((last_rect, last_text)) = cells.last_mut() {
                if sp.bbox.x0 - last_rect.x1 < gap_threshold {
                    if !last_text.ends_with(' ') && !sp.text.starts_with(' ') {
                        last_text.push(' ');
                    }
                    last_text.push_str(sp.text.trim());
                    *last_rect = last_rect.union(&sp.bbox);
                    continue;
                }
            }
            cells.push((sp.bbox, sp.text.trim().to_string()));
        }

        let y_center = (row_bbox.y0 + row_bbox.y1) * 0.5;
        candidate_rows.push(CandidateRow {
            y_center,
            bbox: row_bbox,
            cells,
        });
    }

    // Group consecutive multi-column rows (`cells.len() >= 2`) with matching column counts into tables
    let mut tables: Vec<DetectedTable> = Vec::new();
    let mut current_group: Vec<CandidateRow> = Vec::new();

    for row in candidate_rows {
        if row.cells.len() >= 2 {
            if let Some(prev) = current_group.last() {
                let v_gap = row.y_center - prev.y_center;
                if prev.cells.len() == row.cells.len() && v_gap <= 60.0 {
                    current_group.push(row);
                    continue;
                } else {
                    flush_table_group(&mut current_group, &mut tables);
                }
            }
            current_group.push(row);
        } else {
            flush_table_group(&mut current_group, &mut tables);
        }
    }
    flush_table_group(&mut current_group, &mut tables);

    Ok(tables)
}

fn flush_table_group(group: &mut Vec<CandidateRow>, out: &mut Vec<DetectedTable>) {
    if group.len() >= 2 {
        let mut bbox = group[0].bbox;
        let mut rows = Vec::with_capacity(group.len());
        for r in group.iter() {
            bbox = bbox.union(&r.bbox);
            rows.push(r.cells.iter().map(|(_, txt)| txt.clone()).collect());
        }
        out.push(DetectedTable { bbox, rows });
    }
    group.clear();
}
