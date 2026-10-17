use serde::Serialize;

use crate::advanced::markdown::{classify_heading_level, detect_body_font_size};
use crate::document::Document;
use crate::error::Result;
use crate::geometry::Rect;

/// Semantic text chunk with page and bounding box coordinate metadata for AI / RAG pipelines.
#[derive(Debug, Clone, Serialize)]
pub struct RagChunk {
    pub chunk_id: usize,
    pub page_number: usize,
    pub bbox: Rect,
    pub text: String,
    pub char_count: usize,
    pub section_heading: String,
}

/// Produces semantically coherent text chunks with accurate bounding box coordinates across the document.
pub fn doc_to_rag_chunks(doc: &Document, max_chars: usize) -> Result<Vec<RagChunk>> {
    let limit = max_chars.max(32);
    let mut chunks = Vec::new();
    let mut current_heading = String::new();

    for page_idx in 0..doc.page_count() {
        let layout = doc.page_layout(page_idx)?;
        if layout.blocks.is_empty() {
            continue;
        }
        let body_size = detect_body_font_size(&layout.blocks);

        let mut acc_text = String::new();
        let mut acc_rect: Option<Rect> = None;

        for block in &layout.blocks {
            let first_fs = block
                .lines
                .first()
                .map(|l| l.font_size)
                .unwrap_or(body_size);
            let block_text = block.text();
            let trimmed = block_text.trim();
            if trimmed.is_empty() {
                continue;
            }

            if classify_heading_level(first_fs, body_size).is_some() {
                // Flush any accumulated chunk before starting a new section heading
                if !acc_text.is_empty() {
                    if let Some(bbox) = acc_rect.take() {
                        let text = std::mem::take(&mut acc_text);
                        let char_count = text.chars().count();
                        chunks.push(RagChunk {
                            chunk_id: chunks.len(),
                            page_number: page_idx,
                            bbox,
                            text,
                            char_count,
                            section_heading: current_heading.clone(),
                        });
                    }
                }
                current_heading = trimmed.replace('\n', " ");
            }

            let candidate_len = if acc_text.is_empty() {
                trimmed.len()
            } else {
                acc_text.len() + 1 + trimmed.len()
            };

            if !acc_text.is_empty() && candidate_len > limit {
                if let Some(bbox) = acc_rect.take() {
                    let text = std::mem::take(&mut acc_text);
                    let char_count = text.chars().count();
                    chunks.push(RagChunk {
                        chunk_id: chunks.len(),
                        page_number: page_idx,
                        bbox,
                        text,
                        char_count,
                        section_heading: current_heading.clone(),
                    });
                }
            }

            if !acc_text.is_empty() {
                acc_text.push('\n');
            }
            acc_text.push_str(trimmed);
            acc_rect = Some(match acc_rect {
                Some(r) => r.union(&block.rect),
                None => block.rect,
            });
        }

        if !acc_text.is_empty() {
            if let Some(bbox) = acc_rect.take() {
                let char_count = acc_text.chars().count();
                chunks.push(RagChunk {
                    chunk_id: chunks.len(),
                    page_number: page_idx,
                    bbox,
                    text: acc_text,
                    char_count,
                    section_heading: current_heading.clone(),
                });
            }
        }
    }

    Ok(chunks)
}
