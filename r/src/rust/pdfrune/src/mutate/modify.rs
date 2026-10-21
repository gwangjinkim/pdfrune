use std::collections::BTreeMap;
use std::fmt::Write as _;

use crate::document::Document;
use crate::error::{Result, PdfruneError};
use crate::geometry::{Point, Rect};
use crate::page::Page;
use crate::parser::object::{ObjectId, PdfDict, PdfDictExt, PdfObject, PdfStream};

impl Document {
    /// Creates and inserts a new blank page with `(width, height)` at `index` (or at the end if `None`).
    /// Returns the 0-based index of the newly inserted page.
    pub fn new_page(&mut self, index: Option<usize>, width: f64, height: f64) -> Result<usize> {
        let font_id = self.allocate_object_id();
        let mut font_dict = PdfDict::new();
        font_dict.insert(b"Type".to_vec(), PdfObject::Name(b"Font".to_vec()));
        font_dict.insert(b"Subtype".to_vec(), PdfObject::Name(b"Type1".to_vec()));
        font_dict.insert(b"BaseFont".to_vec(), PdfObject::Name(b"Helvetica".to_vec()));
        self.objects.insert(font_id.0, PdfObject::Dict(font_dict));

        let content_id = self.allocate_object_id();
        let content_stm = PdfStream::new(PdfDict::new(), Vec::new());
        self.objects
            .insert(content_id.0, PdfObject::Stream(content_stm));

        let mut font_map = PdfDict::new();
        font_map.insert(b"F1".to_vec(), PdfObject::Reference(font_id));
        let mut resources = PdfDict::new();
        resources.insert(b"Font".to_vec(), PdfObject::Dict(font_map));

        let mediabox = Rect::new(0.0, 0.0, width.max(1.0), height.max(1.0));
        let page_id = self.allocate_object_id();
        let mut page_dict = PdfDict::new();
        page_dict.insert(b"Type".to_vec(), PdfObject::Name(b"Page".to_vec()));
        page_dict.insert(
            b"MediaBox".to_vec(),
            PdfObject::Array(vec![
                PdfObject::Real(mediabox.x0),
                PdfObject::Real(mediabox.y0),
                PdfObject::Real(mediabox.x1),
                PdfObject::Real(mediabox.y1),
            ]),
        );
        page_dict.insert(b"Resources".to_vec(), PdfObject::Dict(resources.clone()));
        page_dict.insert(b"Contents".to_vec(), PdfObject::Reference(content_id));
        self.objects.insert(page_id.0, PdfObject::Dict(page_dict));

        let insert_idx = index.unwrap_or(self.pages.len()).min(self.pages.len());
        let page = Page {
            index: insert_idx,
            object_id: page_id,
            mediabox,
            cropbox: mediabox,
            bleedbox: mediabox,
            trimbox: mediabox,
            artbox: mediabox,
            rotation: 0,
            resources,
        };
        self.pages.insert(insert_idx, page);
        self.reindex_pages();
        Ok(insert_idx)
    }

    /// Appends raw PDF content stream operators to the page at `page_index`.
    pub fn append_page_content(&mut self, page_index: usize, ops: &[u8]) -> Result<()> {
        let page_obj_id = self.page(page_index)?.object_id;

        // Ensure default /F1 Helvetica font resource is registered on the page
        let existing_f1 = self
            .objects
            .get(&page_obj_id.0)
            .and_then(PdfObject::as_dict)
            .and_then(|d| d.get_dict(b"Resources"))
            .and_then(|r| r.get_dict(b"Font"))
            .is_some_and(|f| f.contains_key(b"F1".as_slice()));

        if !existing_f1 {
            let font_id = self.allocate_object_id();
            let mut font_dict = PdfDict::new();
            font_dict.insert(b"Type".to_vec(), PdfObject::Name(b"Font".to_vec()));
            font_dict.insert(b"Subtype".to_vec(), PdfObject::Name(b"Type1".to_vec()));
            font_dict.insert(b"BaseFont".to_vec(), PdfObject::Name(b"Helvetica".to_vec()));
            self.objects.insert(font_id.0, PdfObject::Dict(font_dict));

            if let Some(page_dict) = self
                .objects
                .get_mut(&page_obj_id.0)
                .and_then(PdfObject::as_dict_mut)
            {
                let res_entry = page_dict
                    .entry(b"Resources".to_vec())
                    .or_insert_with(|| PdfObject::Dict(PdfDict::new()));
                if let Some(res_dict) = res_entry.as_dict_mut() {
                    let font_entry = res_dict
                        .entry(b"Font".to_vec())
                        .or_insert_with(|| PdfObject::Dict(PdfDict::new()));
                    if let Some(f_map) = font_entry.as_dict_mut() {
                        f_map
                            .entry(b"F1".to_vec())
                            .or_insert(PdfObject::Reference(font_id));
                    }
                }
            }
            if let Some(p) = self.pages.get_mut(page_index) {
                let font_entry = p
                    .resources
                    .entry(b"Font".to_vec())
                    .or_insert_with(|| PdfObject::Dict(PdfDict::new()));
                if let Some(f_map) = font_entry.as_dict_mut() {
                    f_map
                        .entry(b"F1".to_vec())
                        .or_insert(PdfObject::Reference(font_id));
                }
            }
        }

        let content_ref = self
            .objects
            .get(&page_obj_id.0)
            .and_then(PdfObject::as_dict)
            .and_then(|d| d.get_ref(b"Contents"));

        if let Some(cid) = content_ref {
            if let Some(PdfObject::Stream(stm)) = self.objects.get_mut(&cid.0) {
                let mut decoded = stm.decode_data().unwrap_or_else(|_| stm.data.clone());
                if !decoded.is_empty() && !decoded.ends_with(b"\n") {
                    decoded.push(b'\n');
                }
                decoded.extend_from_slice(ops);
                stm.dict.remove(b"Filter".as_slice());
                stm.data = decoded;
                return Ok(());
            }
        }

        // Otherwise allocate a new stream object and set/append to /Contents
        let new_cid = self.allocate_object_id();
        self.objects.insert(
            new_cid.0,
            PdfObject::Stream(PdfStream::new(PdfDict::new(), ops.to_vec())),
        );
        if let Some(page_dict) = self
            .objects
            .get_mut(&page_obj_id.0)
            .and_then(PdfObject::as_dict_mut)
        {
            page_dict.insert(b"Contents".to_vec(), PdfObject::Reference(new_cid));
        }
        Ok(())
    }

    /// Inserts text at `point` (top-down page coordinates) on `page_index`.
    pub fn insert_text(
        &mut self,
        page_index: usize,
        point: Point,
        text: &str,
        font_size: f64,
        color: [f64; 3],
    ) -> Result<()> {
        let page = self.page(page_index)?;
        let page_height = page.cropbox.height();
        let pdf_x = page.cropbox.x0 + point.x;
        let pdf_y = page.cropbox.y0 + (page_height - point.y);

        let mut buf = String::new();
        for (idx, line) in text.lines().enumerate() {
            let escaped = escape_pdf_literal(line);
            let line_y = pdf_y - (idx as f64) * (font_size * 1.2);
            let _ = writeln!(
                buf,
                "BT /F1 {:.2} Tf {:.3} {:.3} {:.3} rg {:.2} {:.2} Td ({escaped}) Tj ET",
                font_size, color[0], color[1], color[2], pdf_x, line_y
            );
        }

        self.append_page_content(page_index, buf.as_bytes())
    }

    /// Draws a rectangle on `page_index` (in top-down page coordinates).
    pub fn draw_rect(
        &mut self,
        page_index: usize,
        rect: Rect,
        stroke_color: Option<[f64; 3]>,
        fill_color: Option<[f64; 3]>,
        line_width: f64,
    ) -> Result<()> {
        let page = self.page(page_index)?;
        let page_height = page.cropbox.height();
        let norm = rect.normalize();
        let pdf_x = page.cropbox.x0 + norm.x0;
        let pdf_y = page.cropbox.y0 + (page_height - norm.y1);
        let w = norm.width();
        let h = norm.height();

        let mut buf = String::from("q\n");
        let _ = writeln!(buf, "{:.2} w", line_width.max(0.1));
        if let Some([r, g, b]) = stroke_color {
            let _ = writeln!(buf, "{r:.3} {g:.3} {b:.3} RG");
        }
        if let Some([r, g, b]) = fill_color {
            let _ = writeln!(buf, "{r:.3} {g:.3} {b:.3} rg");
        }
        let op = match (stroke_color.is_some(), fill_color.is_some()) {
            (true, true) => "B",
            (false, true) => "f",
            _ => "S",
        };
        let _ = writeln!(buf, "{pdf_x:.2} {pdf_y:.2} {w:.2} {h:.2} re {op}\nQ");
        self.append_page_content(page_index, buf.as_bytes())
    }

    /// Deletes the page at `page_index` (0-based).
    pub fn delete_page(&mut self, page_index: usize) -> Result<()> {
        if page_index >= self.pages.len() {
            return Err(PdfruneError::PageOutOfBounds {
                index: page_index,
                count: self.pages.len(),
            });
        }
        self.pages.remove(page_index);
        self.reindex_pages();
        Ok(())
    }

    /// Reorders or filters the document's pages in-place to match `indices`.
    pub fn select_pages(&mut self, indices: &[usize]) -> Result<()> {
        let count = self.pages.len();
        let mut selected = Vec::with_capacity(indices.len());
        for &idx in indices {
            let page = self
                .pages
                .get(idx)
                .ok_or(PdfruneError::PageOutOfBounds { index: idx, count })?;
            selected.push(page.clone());
        }
        self.pages = selected;
        self.reindex_pages();
        Ok(())
    }

    /// Extracts a subset of pages into a new standalone `Document`.
    pub fn extract_pages(&self, indices: &[usize]) -> Result<Document> {
        let mut cloned = self.clone();
        cloned.select_pages(indices)?;
        Ok(cloned)
    }

    /// Inserts pages `[from_page..=to_page]` from `other` into `self` starting at `start_at`.
    pub fn insert_pdf(
        &mut self,
        other: &Document,
        from_page: Option<usize>,
        to_page: Option<usize>,
        start_at: Option<usize>,
    ) -> Result<()> {
        if other.pages.is_empty() {
            return Ok(());
        }
        let other_count = other.pages.len();
        let start_idx = from_page.unwrap_or(0).min(other_count - 1);
        let end_idx = to_page.unwrap_or(other_count - 1).min(other_count - 1);

        let id_offset = self.objects.keys().next_back().copied().unwrap_or(0) + 10;
        let mut id_map: BTreeMap<u32, u32> = BTreeMap::new();
        for &old_num in other.objects.keys() {
            id_map.insert(old_num, old_num + id_offset);
        }

        for (&old_num, obj) in &other.objects {
            let new_num = id_map[&old_num];
            let remapped = remap_object_references(obj, &id_map);
            self.objects.insert(new_num, remapped);
        }

        let range: Vec<usize> = if start_idx <= end_idx {
            (start_idx..=end_idx).collect()
        } else {
            (end_idx..=start_idx).rev().collect()
        };

        let mut insert_pos = start_at.unwrap_or(self.pages.len()).min(self.pages.len());
        for p_idx in range {
            let src_page = &other.pages[p_idx];
            let new_obj_num = id_map
                .get(&src_page.object_id.0)
                .copied()
                .unwrap_or(src_page.object_id.0 + id_offset);
            let mut new_page = src_page.clone();
            new_page.object_id = (new_obj_num, 0);
            new_page.resources = remap_dict_references(&src_page.resources, &id_map);
            self.pages.insert(insert_pos, new_page);
            insert_pos += 1;
        }

        self.reindex_pages();
        Ok(())
    }

    fn reindex_pages(&mut self) {
        for (idx, p) in self.pages.iter_mut().enumerate() {
            p.index = idx;
        }
    }
}

/// Merges multiple `Document` references into a single combined `Document`.
pub fn merge_documents(docs: &[&Document]) -> Result<Document> {
    let mut out = Document::new();
    for doc in docs {
        out.insert_pdf(doc, None, None, None)?;
    }
    Ok(out)
}

fn remap_object_references(obj: &PdfObject, id_map: &BTreeMap<u32, u32>) -> PdfObject {
    match obj {
        PdfObject::Reference((num, _gen)) => {
            let mapped = id_map.get(num).copied().unwrap_or(*num);
            PdfObject::Reference((mapped, 0))
        }
        PdfObject::Array(arr) => PdfObject::Array(
            arr.iter()
                .map(|item| remap_object_references(item, id_map))
                .collect(),
        ),
        PdfObject::Dict(dict) => PdfObject::Dict(remap_dict_references(dict, id_map)),
        PdfObject::Stream(stm) => PdfObject::Stream(PdfStream::new(
            remap_dict_references(&stm.dict, id_map),
            stm.data.clone(),
        )),
        other => other.clone(),
    }
}

fn remap_dict_references(dict: &PdfDict, id_map: &BTreeMap<u32, u32>) -> PdfDict {
    dict.iter()
        .map(|(k, v)| (k.clone(), remap_object_references(v, id_map)))
        .collect()
}

fn escape_pdf_literal(s: &str) -> String {
    let mut out = String::with_capacity(s.len());
    for ch in s.chars() {
        match ch {
            '(' => out.push_str("\\("),
            ')' => out.push_str("\\)"),
            '\\' => out.push_str("\\\\"),
            _ => out.push(ch),
        }
    }
    out
}

pub type _ObjectIdAlias = ObjectId;
