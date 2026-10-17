use serde::Serialize;

use crate::document::Document;
use crate::error::Result;
use crate::geometry::Rect;
use crate::text::extractor::{extract_page_spans, PositionedSpan};

/// A single extracted word with bounding box and structural indices.
#[derive(Debug, Clone, Serialize)]
pub struct TextWord {
    pub rect: Rect,
    pub text: String,
    pub block_no: usize,
    pub line_no: usize,
    pub word_no: usize,
    pub font_size: f64,
    pub font_name: String,
}

/// A horizontal line of words.
#[derive(Debug, Clone, Serialize)]
pub struct TextLine {
    pub rect: Rect,
    pub words: Vec<TextWord>,
    pub font_size: f64,
    pub font_name: String,
    #[serde(skip)]
    pub chars: Vec<(char, Rect)>,
}

impl TextLine {
    pub fn text(&self) -> String {
        self.words
            .iter()
            .map(|w| w.text.as_str())
            .collect::<Vec<_>>()
            .join(" ")
    }
}

/// A paragraph or visual block of text lines.
#[derive(Debug, Clone, Serialize)]
pub struct TextBlock {
    pub rect: Rect,
    pub lines: Vec<TextLine>,
    pub block_no: usize,
    pub block_type: u8,
}

impl TextBlock {
    pub fn text(&self) -> String {
        self.lines
            .iter()
            .map(TextLine::text)
            .collect::<Vec<_>>()
            .join("\n")
    }
}

/// Complete structured layout of a PDF page (blocks -> lines -> words -> characters).
#[derive(Debug, Clone, Default, Serialize)]
pub struct PageLayout {
    pub width: f64,
    pub height: f64,
    pub blocks: Vec<TextBlock>,
}

impl PageLayout {
    /// Extracts and reconstructs the structured layout of `page_index`.
    pub fn from_document(doc: &Document, page_index: usize) -> Result<Self> {
        let page = doc.page(page_index)?;
        let width = page.width();
        let height = page.height();
        let spans = extract_page_spans(doc, page_index)?;
        Ok(Self::from_spans(width, height, spans))
    }

    pub fn from_spans(width: f64, height: f64, spans: Vec<PositionedSpan>) -> Self {
        // 1. Collect all characters with their font metadata
        #[derive(Clone)]
        struct CharItem {
            ch: char,
            rect: Rect,
            font_size: f64,
            font_name: String,
        }

        let mut char_items = Vec::new();
        for span in spans {
            for (ch, rect) in span.chars {
                char_items.push(CharItem {
                    ch,
                    rect,
                    font_size: span.font_size,
                    font_name: span.font_name.clone(),
                });
            }
        }

        if char_items.is_empty() {
            return Self {
                width,
                height,
                blocks: Vec::new(),
            };
        }

        // 2. Group characters into horizontal lines based on vertical center proximity
        let mut raw_lines: Vec<Vec<CharItem>> = Vec::new();
        for item in char_items {
            let item_cy = (item.rect.y0 + item.rect.y1) * 0.5;
            if let Some(last_line) = raw_lines.last_mut() {
                let first = &last_line[0];
                let line_cy = (first.rect.y0 + first.rect.y1) * 0.5;
                let tol = (first.font_size.max(item.font_size)) * 0.45;
                if (item_cy - line_cy).abs() <= tol {
                    last_line.push(item);
                    continue;
                }
            }
            raw_lines.push(vec![item]);
        }

        // Sort lines top-to-bottom, and sort characters within each line left-to-right
        raw_lines.sort_by(|a, b| {
            let ay = a[0].rect.y0;
            let by = b[0].rect.y0;
            ay.partial_cmp(&by).unwrap_or(std::cmp::Ordering::Equal)
        });

        let mut built_lines: Vec<TextLine> = Vec::with_capacity(raw_lines.len());

        for mut line_chars in raw_lines {
            line_chars.sort_by(|a, b| {
                a.rect
                    .x0
                    .partial_cmp(&b.rect.x0)
                    .unwrap_or(std::cmp::Ordering::Equal)
            });

            let mut words: Vec<TextWord> = Vec::new();
            let mut cur_word_text = String::new();
            let mut cur_word_rect: Option<Rect> = None;
            let mut cur_font_size = line_chars[0].font_size;
            let mut cur_font_name = line_chars[0].font_name.clone();
            let mut prev_x1: Option<f64> = None;
            let mut line_char_tuples: Vec<(char, Rect)> = Vec::new();

            for item in line_chars {
                let space_threshold = (item.font_size * 0.22).max(1.8);
                let has_gap = prev_x1.is_some_and(|px| item.rect.x0 - px > space_threshold);

                if item.ch.is_whitespace() || has_gap {
                    if !cur_word_text.is_empty() {
                        if let Some(r) = cur_word_rect.take() {
                            words.push(TextWord {
                                rect: r,
                                text: std::mem::take(&mut cur_word_text),
                                block_no: 0,
                                line_no: 0,
                                word_no: words.len(),
                                font_size: cur_font_size,
                                font_name: cur_font_name.clone(),
                            });
                        }
                        if has_gap && !item.ch.is_whitespace() {
                            let sp_x0 = prev_x1.unwrap_or(item.rect.x0);
                            line_char_tuples.push((
                                ' ',
                                Rect::new(sp_x0, item.rect.y0, item.rect.x0, item.rect.y1),
                            ));
                        }
                    }
                    if item.ch.is_whitespace() {
                        line_char_tuples.push((' ', item.rect));
                        prev_x1 = Some(item.rect.x1);
                        continue;
                    }
                }

                if cur_word_text.is_empty() {
                    cur_font_size = item.font_size;
                    cur_font_name = item.font_name.clone();
                }
                cur_word_text.push(item.ch);
                cur_word_rect = Some(match cur_word_rect {
                    Some(r) => r.union(&item.rect),
                    None => item.rect,
                });
                line_char_tuples.push((item.ch, item.rect));
                prev_x1 = Some(item.rect.x1);
            }

            if !cur_word_text.is_empty() {
                if let Some(r) = cur_word_rect.take() {
                    words.push(TextWord {
                        rect: r,
                        text: cur_word_text,
                        block_no: 0,
                        line_no: 0,
                        word_no: words.len(),
                        font_size: cur_font_size,
                        font_name: cur_font_name.clone(),
                    });
                }
            }

            if !words.is_empty() {
                let mut line_rect = words[0].rect;
                for w in &words[1..] {
                    line_rect = line_rect.union(&w.rect);
                }
                built_lines.push(TextLine {
                    rect: line_rect,
                    font_size: words[0].font_size,
                    font_name: words[0].font_name.clone(),
                    words,
                    chars: line_char_tuples,
                });
            }
        }

        // 3. Group lines into blocks (paragraphs/sections) based on vertical gap and font size changes
        let mut blocks: Vec<TextBlock> = Vec::new();
        for mut line in built_lines {
            let attach_to_prev = if let Some(prev_block) = blocks.last() {
                if let Some(prev_line) = prev_block.lines.last() {
                    let v_gap = line.rect.y0 - prev_line.rect.y1;
                    let max_fs = prev_line.font_size.max(line.font_size);
                    let same_size = (prev_line.font_size - line.font_size).abs() <= 2.0;
                    v_gap <= max_fs * 0.85 && same_size
                } else {
                    false
                }
            } else {
                false
            };

            if attach_to_prev {
                let block = blocks.last_mut().unwrap();
                let block_no = block.block_no;
                let line_no = block.lines.len();
                for (w_idx, w) in line.words.iter_mut().enumerate() {
                    w.block_no = block_no;
                    w.line_no = line_no;
                    w.word_no = w_idx;
                }
                block.rect = block.rect.union(&line.rect);
                block.lines.push(line);
            } else {
                let block_no = blocks.len();
                for (w_idx, w) in line.words.iter_mut().enumerate() {
                    w.block_no = block_no;
                    w.line_no = 0;
                    w.word_no = w_idx;
                }
                blocks.push(TextBlock {
                    rect: line.rect,
                    lines: vec![line],
                    block_no,
                    block_type: 0,
                });
            }
        }

        Self {
            width,
            height,
            blocks,
        }
    }

    /// Returns the page's plain text in reading order.
    pub fn plain_text(&self) -> String {
        self.blocks
            .iter()
            .map(TextBlock::text)
            .collect::<Vec<_>>()
            .join("\n")
    }

    /// Returns all words in reading order.
    pub fn words(&self) -> Vec<TextWord> {
        let mut out = Vec::new();
        for block in &self.blocks {
            for line in &block.lines {
                out.extend(line.words.iter().cloned());
            }
        }
        out
    }

    /// Searches for `needle` (case-insensitive) across the page and returns bounding boxes.
    pub fn search(&self, needle: &str) -> Vec<Rect> {
        let needle_trimmed = needle.trim();
        if needle_trimmed.is_empty() {
            return Vec::new();
        }
        let needle_lower = needle_trimmed.to_lowercase();
        let needle_chars: Vec<char> = needle_lower.chars().collect();
        let mut results = Vec::new();

        for block in &self.blocks {
            for line in &block.lines {
                let line_chars: Vec<(char, Rect)> = line
                    .chars
                    .iter()
                    .flat_map(|&(c, r)| c.to_lowercase().map(move |lc| (lc, r)))
                    .collect();

                if line_chars.len() < needle_chars.len() {
                    continue;
                }

                let mut i = 0;
                while i + needle_chars.len() <= line_chars.len() {
                    let matches = line_chars[i..i + needle_chars.len()]
                        .iter()
                        .zip(&needle_chars)
                        .all(|((lc, _), nc)| lc == nc);

                    if matches {
                        let mut hit_rect = line_chars[i].1;
                        for &(_, r) in &line_chars[i + 1..i + needle_chars.len()] {
                            hit_rect = hit_rect.union(&r);
                        }
                        results.push(hit_rect);
                        i += needle_chars.len();
                    } else {
                        i += 1;
                    }
                }
            }
        }

        results
    }
}

impl Serialize for Rect {
    fn serialize<S>(&self, serializer: S) -> std::result::Result<S::Ok, S::Error>
    where
        S: serde::Serializer,
    {
        [self.x0, self.y0, self.x1, self.y1].serialize(serializer)
    }
}
