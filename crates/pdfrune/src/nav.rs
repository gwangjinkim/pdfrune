use std::collections::HashSet;

use crate::document::Document;
use crate::error::{PdfruneError, Result};
use crate::geometry::Rect;
use crate::parser::object::{ObjectId, PdfDict, PdfDictExt, PdfObject};

/// A Table of Contents (bookmark / outline) entry.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct TocEntry {
    /// 1-based hierarchy level (`1` = top-level chapter, `2` = section, etc.).
    pub level: usize,
    /// Bookmark title text.
    pub title: String,
    /// 1-based target page number (`1..=page_count`).
    pub page: usize,
}

/// A hyperlink annotation on a page.
#[derive(Debug, Clone, PartialEq)]
pub struct LinkEntry {
    /// Indirect object number (`xref`) of the `/Link` annotation.
    pub xref: u32,
    /// Link kind: `"uri"` or `"goto"`.
    pub kind: String,
    /// Top-down bounding box of the clickable link area.
    pub from: Rect,
    /// Target URI when `kind == "uri"`.
    pub uri: Option<String>,
    /// 0-based target page index when `kind == "goto"`.
    pub page: Option<usize>,
}

/// Extracts the hierarchical Table of Contents (`/Outlines`) from `doc`.
pub fn get_toc(doc: &Document) -> Vec<TocEntry> {
    let Some(root_id) = doc.trailer.get_ref(b"Root") else {
        return Vec::new();
    };
    let Some(catalog) = doc.get_object(root_id).and_then(PdfObject::as_dict) else {
        return Vec::new();
    };
    let Some(outlines_obj) = catalog.get(b"Outlines".as_slice()) else {
        return Vec::new();
    };
    let Some(outlines_dict) = doc.resolve(outlines_obj).as_dict() else {
        return Vec::new();
    };
    let Some(first_ref) = outlines_dict.get_ref(b"First") else {
        return Vec::new();
    };

    let mut out = Vec::new();
    let mut visited = HashSet::new();
    collect_outline_items(doc, first_ref, 1, &mut visited, &mut out);
    out
}

fn collect_outline_items(
    doc: &Document,
    first_id: ObjectId,
    level: usize,
    visited: &mut HashSet<u32>,
    out: &mut Vec<TocEntry>,
) {
    let mut current_opt = Some(first_id);
    while let Some(curr_id) = current_opt {
        if !visited.insert(curr_id.0) {
            break;
        }
        let Some(dict) = doc.get_object(curr_id).and_then(PdfObject::as_dict) else {
            break;
        };
        let title = dict.get_string_decoded(b"Title").unwrap_or_default();
        let page_0 = resolve_dest_page_index(doc, dict).unwrap_or(0);
        out.push(TocEntry {
            level,
            title,
            page: page_0 + 1,
        });

        if let Some(child_first) = dict.get_ref(b"First") {
            collect_outline_items(doc, child_first, level + 1, visited, out);
        }

        current_opt = dict.get_ref(b"Next");
    }
}

fn resolve_dest_page_index(doc: &Document, dict: &PdfDict) -> Option<usize> {
    if let Some(dest_obj) = dict.get(b"Dest".as_slice()) {
        if let Some(p) = dest_object_to_page_index(doc, dest_obj) {
            return Some(p);
        }
    }
    if let Some(action_obj) = dict.get(b"A".as_slice()) {
        if let Some(action_dict) = doc.resolve(action_obj).as_dict() {
            if let Some(d_obj) = action_dict.get(b"D".as_slice()) {
                if let Some(p) = dest_object_to_page_index(doc, d_obj) {
                    return Some(p);
                }
            }
        }
    }
    None
}

fn dest_object_to_page_index(doc: &Document, dest_obj: &PdfObject) -> Option<usize> {
    let resolved = doc.resolve(dest_obj);
    let arr = resolved.as_array()?;
    let first = arr.first()?;
    if let Some(page_ref) = first.as_reference() {
        return doc.pages.iter().position(|p| p.object_id.0 == page_ref.0);
    }
    if let Some(page_num) = first.as_i64() {
        let idx = page_num.max(0) as usize;
        if idx < doc.pages.len() {
            return Some(idx);
        }
    }
    None
}

/// Replaces the document's `/Outlines` Table of Contents hierarchy with `entries`.
pub fn set_toc(doc: &mut Document, entries: &[TocEntry]) -> Result<usize> {
    // Ensure /Root Catalog exists
    let root_id = if let Some(id) = doc.trailer.get_ref(b"Root") {
        id
    } else {
        let id = doc.allocate_object_id();
        let mut cat = PdfDict::new();
        cat.insert(b"Type".to_vec(), PdfObject::Name(b"Catalog".to_vec()));
        doc.objects.insert(id.0, PdfObject::Dict(cat));
        doc.trailer
            .insert(b"Root".to_vec(), PdfObject::Reference(id));
        id
    };

    if entries.is_empty() {
        if let Some(cat) = doc
            .objects
            .get_mut(&root_id.0)
            .and_then(PdfObject::as_dict_mut)
        {
            cat.remove(b"Outlines".as_slice());
        }
        return Ok(0);
    }

    let outlines_root_id = doc.allocate_object_id();
    let n = entries.len();
    let mut item_ids = Vec::with_capacity(n);
    for _ in 0..n {
        item_ids.push(doc.allocate_object_id());
    }

    // Build parent-child relationships using a level stack.
    let mut children_of_root: Vec<usize> = Vec::new();
    let mut children_of_node: Vec<Vec<usize>> = vec![Vec::new(); n];
    let mut parent_of_node: Vec<Option<usize>> = vec![None; n];
    let mut stack: Vec<(usize, usize)> = Vec::new(); // (level, node_idx)

    for (idx, entry) in entries.iter().enumerate() {
        let lvl = entry.level.max(1);
        while let Some(&(top_lvl, _)) = stack.last() {
            if top_lvl >= lvl {
                stack.pop();
            } else {
                break;
            }
        }
        if let Some(&(_, parent_idx)) = stack.last() {
            parent_of_node[idx] = Some(parent_idx);
            children_of_node[parent_idx].push(idx);
        } else {
            parent_of_node[idx] = None;
            children_of_root.push(idx);
        }
        stack.push((lvl, idx));
    }

    // Create each outline item dictionary
    for (idx, entry) in entries.iter().enumerate() {
        let mut item_dict = PdfDict::new();
        item_dict.insert(
            b"Title".to_vec(),
            PdfObject::String(entry.title.as_bytes().to_vec()),
        );

        let parent_ref = match parent_of_node[idx] {
            Some(p_idx) => item_ids[p_idx],
            None => outlines_root_id,
        };
        item_dict.insert(b"Parent".to_vec(), PdfObject::Reference(parent_ref));

        let siblings = match parent_of_node[idx] {
            Some(p_idx) => &children_of_node[p_idx],
            None => &children_of_root,
        };
        if let Some(pos) = siblings.iter().position(|&x| x == idx) {
            if pos > 0 {
                item_dict.insert(
                    b"Prev".to_vec(),
                    PdfObject::Reference(item_ids[siblings[pos - 1]]),
                );
            }
            if pos + 1 < siblings.len() {
                item_dict.insert(
                    b"Next".to_vec(),
                    PdfObject::Reference(item_ids[siblings[pos + 1]]),
                );
            }
        }

        let kids = &children_of_node[idx];
        if !kids.is_empty() {
            item_dict.insert(
                b"First".to_vec(),
                PdfObject::Reference(item_ids[*kids.first().unwrap()]),
            );
            item_dict.insert(
                b"Last".to_vec(),
                PdfObject::Reference(item_ids[*kids.last().unwrap()]),
            );
            item_dict.insert(b"Count".to_vec(), PdfObject::Int(kids.len() as i64));
        }

        // Destination page reference (1-based `entry.page` -> 0-based index)
        if !doc.pages.is_empty() {
            let page_idx = entry
                .page
                .saturating_sub(1)
                .min(doc.pages.len().saturating_sub(1));
            let page_ref = doc.pages[page_idx].object_id;
            item_dict.insert(
                b"Dest".to_vec(),
                PdfObject::Array(vec![
                    PdfObject::Reference(page_ref),
                    PdfObject::Name(b"XYZ".to_vec()),
                    PdfObject::Null,
                    PdfObject::Null,
                    PdfObject::Null,
                ]),
            );
        }

        doc.objects
            .insert(item_ids[idx].0, PdfObject::Dict(item_dict));
    }

    let mut outlines_dict = PdfDict::new();
    outlines_dict.insert(b"Type".to_vec(), PdfObject::Name(b"Outlines".to_vec()));
    if !children_of_root.is_empty() {
        outlines_dict.insert(
            b"First".to_vec(),
            PdfObject::Reference(item_ids[*children_of_root.first().unwrap()]),
        );
        outlines_dict.insert(
            b"Last".to_vec(),
            PdfObject::Reference(item_ids[*children_of_root.last().unwrap()]),
        );
        outlines_dict.insert(b"Count".to_vec(), PdfObject::Int(n as i64));
    }
    doc.objects
        .insert(outlines_root_id.0, PdfObject::Dict(outlines_dict));

    if let Some(cat) = doc
        .objects
        .get_mut(&root_id.0)
        .and_then(PdfObject::as_dict_mut)
    {
        cat.insert(b"Outlines".to_vec(), PdfObject::Reference(outlines_root_id));
    }

    Ok(n)
}

/// Extracts all `/Subtype /Link` annotations from `page_index`.
pub fn get_page_links(doc: &Document, page_index: usize) -> Result<Vec<LinkEntry>> {
    let page = doc.page(page_index)?;
    let page_height = page.height();
    let Some(page_dict) = doc.get_object(page.object_id).and_then(PdfObject::as_dict) else {
        return Ok(Vec::new());
    };
    let Some(annots_obj) = page_dict.get(b"Annots".as_slice()) else {
        return Ok(Vec::new());
    };
    let Some(annots_arr) = doc.resolve(annots_obj).as_array() else {
        return Ok(Vec::new());
    };

    let mut links = Vec::new();
    for item in annots_arr {
        let xref = item.as_reference().map(|r| r.0).unwrap_or(0);
        let Some(annot_dict) = doc.resolve(item).as_dict() else {
            continue;
        };
        if annot_dict.get_name_str(b"Subtype") != Some("Link") {
            continue;
        }
        let pdf_rect = annot_dict
            .get(b"Rect".as_slice())
            .and_then(|o| doc.resolve(o).as_rect())
            .unwrap_or([0.0, 0.0, 0.0, 0.0]);
        let from = Rect::new(
            pdf_rect[0],
            page_height - pdf_rect[3],
            pdf_rect[2],
            page_height - pdf_rect[1],
        )
        .normalize();

        let mut kind = "unknown".to_string();
        let mut uri = None;
        let mut target_page = None;

        if let Some(a_obj) = annot_dict.get(b"A".as_slice()) {
            if let Some(a_dict) = doc.resolve(a_obj).as_dict() {
                match a_dict.get_name_str(b"S") {
                    Some("URI") => {
                        kind = "uri".to_string();
                        uri = a_dict.get_string_decoded(b"URI");
                    }
                    Some("GoTo") => {
                        kind = "goto".to_string();
                        if let Some(d_obj) = a_dict.get(b"D".as_slice()) {
                            target_page = dest_object_to_page_index(doc, d_obj);
                        }
                    }
                    _ => {}
                }
            }
        } else if let Some(dest_obj) = annot_dict.get(b"Dest".as_slice()) {
            kind = "goto".to_string();
            target_page = dest_object_to_page_index(doc, dest_obj);
        }

        links.push(LinkEntry {
            xref,
            kind,
            from,
            uri,
            page: target_page,
        });
    }
    Ok(links)
}

/// Inserts a hyperlink annotation (`/Subtype /Link`) onto `page_index`.
pub fn insert_page_link(
    doc: &mut Document,
    page_index: usize,
    from: Rect,
    uri: Option<&str>,
    target_page: Option<usize>,
) -> Result<u32> {
    let page = doc.page(page_index)?.clone();
    let page_height = page.height();
    let norm = from.normalize();
    let pdf_rect = [
        norm.x0,
        page_height - norm.y1,
        norm.x1,
        page_height - norm.y0,
    ];

    let mut action_dict = PdfDict::new();
    if let Some(u) = uri {
        action_dict.insert(b"S".to_vec(), PdfObject::Name(b"URI".to_vec()));
        action_dict.insert(b"URI".to_vec(), PdfObject::String(u.as_bytes().to_vec()));
    } else if let Some(tp) = target_page {
        if tp >= doc.pages.len() {
            return Err(PdfruneError::PageOutOfBounds {
                index: tp,
                count: doc.pages.len(),
            });
        }
        let target_ref = doc.pages[tp].object_id;
        action_dict.insert(b"S".to_vec(), PdfObject::Name(b"GoTo".to_vec()));
        action_dict.insert(
            b"D".to_vec(),
            PdfObject::Array(vec![
                PdfObject::Reference(target_ref),
                PdfObject::Name(b"XYZ".to_vec()),
                PdfObject::Null,
                PdfObject::Null,
                PdfObject::Null,
            ]),
        );
    } else {
        return Err(PdfruneError::InvalidArgument(
            "insert_link requires either 'uri' or 'page'".to_string(),
        ));
    }

    let mut annot_dict = PdfDict::new();
    annot_dict.insert(b"Type".to_vec(), PdfObject::Name(b"Annot".to_vec()));
    annot_dict.insert(b"Subtype".to_vec(), PdfObject::Name(b"Link".to_vec()));
    annot_dict.insert(
        b"Rect".to_vec(),
        PdfObject::Array(vec![
            PdfObject::Real(pdf_rect[0]),
            PdfObject::Real(pdf_rect[1]),
            PdfObject::Real(pdf_rect[2]),
            PdfObject::Real(pdf_rect[3]),
        ]),
    );
    annot_dict.insert(
        b"Border".to_vec(),
        PdfObject::Array(vec![
            PdfObject::Int(0),
            PdfObject::Int(0),
            PdfObject::Int(0),
        ]),
    );
    annot_dict.insert(b"A".to_vec(), PdfObject::Dict(action_dict));

    let annot_id = doc.allocate_object_id();
    doc.objects.insert(annot_id.0, PdfObject::Dict(annot_dict));
    append_annot_to_page(doc, page.object_id, annot_id);
    Ok(annot_id.0)
}

/// Deletes a hyperlink annotation from `page_index` by `xref` or bounding box `from`.
pub fn delete_page_link(
    doc: &mut Document,
    page_index: usize,
    xref: Option<u32>,
    from: Option<Rect>,
) -> Result<()> {
    let page = doc.page(page_index)?.clone();
    let page_height = page.height();
    let existing_annots = doc
        .get_object(page.object_id)
        .and_then(PdfObject::as_dict)
        .and_then(|d| d.get(b"Annots".as_slice()))
        .and_then(|o| doc.resolve(o).as_array())
        .map(|a| a.to_vec())
        .unwrap_or_default();

    let mut filtered = Vec::new();
    for item in existing_annots {
        if let Some(target_xref) = xref {
            if item.as_reference().map(|r| r.0) == Some(target_xref) {
                continue;
            }
        }
        if let Some(target_rect) = from {
            if let Some(annot_dict) = doc.resolve(&item).as_dict() {
                if let Some(r) = annot_dict
                    .get(b"Rect".as_slice())
                    .and_then(|o| doc.resolve(o).as_rect())
                {
                    let top_down =
                        Rect::new(r[0], page_height - r[3], r[2], page_height - r[1]).normalize();
                    if (top_down.x0 - target_rect.x0).abs() < 1.0
                        && (top_down.y0 - target_rect.y0).abs() < 1.0
                    {
                        continue;
                    }
                }
            }
        }
        filtered.push(item);
    }

    if let Some(page_dict) = doc
        .objects
        .get_mut(&page.object_id.0)
        .and_then(PdfObject::as_dict_mut)
    {
        page_dict.insert(b"Annots".to_vec(), PdfObject::Array(filtered));
    }
    Ok(())
}

pub(crate) fn append_annot_to_page(doc: &mut Document, page_obj_id: ObjectId, annot_ref: ObjectId) {
    let mut annots = doc
        .get_object(page_obj_id)
        .and_then(PdfObject::as_dict)
        .and_then(|d| d.get(b"Annots".as_slice()))
        .and_then(|o| doc.resolve(o).as_array())
        .map(|a| a.to_vec())
        .unwrap_or_default();
    annots.push(PdfObject::Reference(annot_ref));
    if let Some(page_dict) = doc
        .objects
        .get_mut(&page_obj_id.0)
        .and_then(PdfObject::as_dict_mut)
    {
        page_dict.insert(b"Annots".to_vec(), PdfObject::Array(annots));
    }
}
