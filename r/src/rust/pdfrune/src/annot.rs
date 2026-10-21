use crate::document::Document;
use crate::error::Result;
use crate::geometry::{Point, Rect};
use crate::nav::append_annot_to_page;
use crate::parser::object::{PdfDict, PdfDictExt, PdfObject, PdfStream};
use crate::text::extractor::extract_page_spans;

/// Metadata for a PDF annotation on a page.
#[derive(Debug, Clone, PartialEq)]
pub struct AnnotInfo {
    pub xref: u32,
    pub subtype: String,
    pub rect: Rect,
    pub contents: String,
    pub color: [f64; 3],
    pub overlay_text: Option<String>,
}

/// Metadata for an interactive AcroForm `/Widget` field on a page.
#[derive(Debug, Clone, PartialEq)]
pub struct WidgetInfo {
    pub xref: u32,
    pub field_name: String,
    pub field_type: String,
    pub field_value: String,
    pub rect: Rect,
}

/// Lists all annotations on `page_index` (excluding `/Link` and `/Widget` which have dedicated methods).
pub fn get_page_annots(doc: &Document, page_index: usize) -> Result<Vec<AnnotInfo>> {
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

    let mut out = Vec::new();
    for item in annots_arr {
        let xref = item.as_reference().map(|r| r.0).unwrap_or(0);
        let Some(dict) = doc.resolve(item).as_dict() else {
            continue;
        };
        let Some(subtype) = dict.get_name_str(b"Subtype") else {
            continue;
        };
        if subtype == "Link" || subtype == "Widget" {
            continue;
        }
        let pdf_rect = dict
            .get(b"Rect".as_slice())
            .and_then(|o| doc.resolve(o).as_rect())
            .unwrap_or([0.0, 0.0, 0.0, 0.0]);
        let rect = Rect::new(
            pdf_rect[0],
            page_height - pdf_rect[3],
            pdf_rect[2],
            page_height - pdf_rect[1],
        )
        .normalize();

        let contents = dict.get_string_decoded(b"Contents").unwrap_or_default();
        let overlay_text = dict.get_string_decoded(b"OverlayText");
        let color = dict
            .get(b"C".as_slice())
            .and_then(|o| doc.resolve(o).as_array())
            .filter(|a| a.len() >= 3)
            .map(|a| {
                [
                    a[0].as_f64().unwrap_or(0.0),
                    a[1].as_f64().unwrap_or(0.0),
                    a[2].as_f64().unwrap_or(0.0),
                ]
            })
            .unwrap_or([1.0, 1.0, 0.0]);

        out.push(AnnotInfo {
            xref,
            subtype: subtype.to_string(),
            rect,
            contents,
            color,
            overlay_text,
        });
    }
    Ok(out)
}

/// Adds an annotation (`/Highlight`, `/Text`, `/Square`, `/Redact`, etc.) to `page_index`.
pub fn add_page_annot(
    doc: &mut Document,
    page_index: usize,
    subtype: &str,
    rect: Rect,
    contents: Option<&str>,
    color: [f64; 3],
    overlay_text: Option<&str>,
) -> Result<AnnotInfo> {
    let page = doc.page(page_index)?.clone();
    let page_height = page.height();
    let norm = rect.normalize();
    let pdf_rect = [
        norm.x0,
        page_height - norm.y1,
        norm.x1,
        page_height - norm.y0,
    ];

    let mut dict = PdfDict::new();
    dict.insert(b"Type".to_vec(), PdfObject::Name(b"Annot".to_vec()));
    dict.insert(
        b"Subtype".to_vec(),
        PdfObject::Name(subtype.as_bytes().to_vec()),
    );
    dict.insert(
        b"Rect".to_vec(),
        PdfObject::Array(vec![
            PdfObject::Real(pdf_rect[0]),
            PdfObject::Real(pdf_rect[1]),
            PdfObject::Real(pdf_rect[2]),
            PdfObject::Real(pdf_rect[3]),
        ]),
    );
    dict.insert(
        b"C".to_vec(),
        PdfObject::Array(vec![
            PdfObject::Real(color[0]),
            PdfObject::Real(color[1]),
            PdfObject::Real(color[2]),
        ]),
    );
    if let Some(c) = contents {
        dict.insert(
            b"Contents".to_vec(),
            PdfObject::String(c.as_bytes().to_vec()),
        );
    }
    if let Some(ot) = overlay_text {
        dict.insert(
            b"OverlayText".to_vec(),
            PdfObject::String(ot.as_bytes().to_vec()),
        );
    }

    let annot_id = doc.allocate_object_id();
    doc.objects.insert(annot_id.0, PdfObject::Dict(dict));
    append_annot_to_page(doc, page.object_id, annot_id);

    Ok(AnnotInfo {
        xref: annot_id.0,
        subtype: subtype.to_string(),
        rect: norm,
        contents: contents.unwrap_or_default().to_string(),
        color,
        overlay_text: overlay_text.map(str::to_string),
    })
}

/// Deletes an annotation by `xref` from `page_index`.
pub fn delete_page_annot(doc: &mut Document, page_index: usize, xref: u32) -> Result<()> {
    let page = doc.page(page_index)?.clone();
    let existing = doc
        .get_object(page.object_id)
        .and_then(PdfObject::as_dict)
        .and_then(|d| d.get(b"Annots".as_slice()))
        .and_then(|o| doc.resolve(o).as_array())
        .map(|a| a.to_vec())
        .unwrap_or_default();

    let filtered: Vec<PdfObject> = existing
        .into_iter()
        .filter(|item| item.as_reference().map(|r| r.0) != Some(xref))
        .collect();

    if let Some(page_dict) = doc
        .objects
        .get_mut(&page.object_id.0)
        .and_then(PdfObject::as_dict_mut)
    {
        page_dict.insert(b"Annots".to_vec(), PdfObject::Array(filtered));
    }
    doc.objects.remove(&xref);
    Ok(())
}

/// Applies all `/Subtype /Redact` annotations on `page_index`, permanently scrubbing
/// intersecting text spans from the page's content stream and drawing the redaction boxes.
pub fn apply_page_redactions(doc: &mut Document, page_index: usize) -> Result<usize> {
    let annots = get_page_annots(doc, page_index)?;
    let redactions: Vec<AnnotInfo> = annots
        .into_iter()
        .filter(|a| a.subtype == "Redact")
        .collect();
    if redactions.is_empty() {
        return Ok(0);
    }

    let spans = extract_page_spans(doc, page_index).unwrap_or_default();
    let page_obj_id = doc.page(page_index)?.object_id;

    // Reset page /Contents stream to empty before re-emitting surviving spans
    let content_ref = doc
        .get_object(page_obj_id)
        .and_then(PdfObject::as_dict)
        .and_then(|d| d.get_ref(b"Contents"));
    if let Some(cid) = content_ref {
        if let Some(PdfObject::Stream(stm)) = doc.objects.get_mut(&cid.0) {
            stm.dict.remove(b"Filter".as_slice());
            stm.data.clear();
        }
    } else {
        let new_cid = doc.allocate_object_id();
        doc.objects.insert(
            new_cid.0,
            PdfObject::Stream(PdfStream::new(PdfDict::new(), Vec::new())),
        );
        if let Some(page_dict) = doc
            .objects
            .get_mut(&page_obj_id.0)
            .and_then(PdfObject::as_dict_mut)
        {
            page_dict.insert(b"Contents".to_vec(), PdfObject::Reference(new_cid));
        }
    }

    let mut removed_count = 0usize;
    for span in spans {
        let is_redacted = redactions.iter().any(|r| r.rect.intersects(&span.bbox));
        if is_redacted {
            removed_count += 1;
        } else {
            doc.insert_text(
                page_index,
                Point::new(span.bbox.x0, span.bbox.y1),
                &span.text,
                span.font_size.max(8.0),
                [0.0, 0.0, 0.0],
            )?;
        }
    }

    // Draw redaction fill boxes and optional overlay replacement text, then remove Redact annots
    for r in &redactions {
        doc.draw_rect(page_index, r.rect, None, Some(r.color), 0.0)?;
        if let Some(ref text) = r.overlay_text {
            if !text.is_empty() {
                let text_color = if r.color[0] + r.color[1] + r.color[2] < 1.2 {
                    [1.0, 1.0, 1.0]
                } else {
                    [0.0, 0.0, 0.0]
                };
                let font_sz = (r.rect.height() * 0.5).clamp(8.0, 14.0);
                doc.insert_text(
                    page_index,
                    Point::new(r.rect.x0 + 2.0, r.rect.y1 - 4.0),
                    text,
                    font_sz,
                    text_color,
                )?;
            }
        }
        delete_page_annot(doc, page_index, r.xref)?;
    }

    Ok(removed_count.max(redactions.len()))
}

/// Lists all interactive AcroForm `/Subtype /Widget` fields on `page_index`.
pub fn get_page_widgets(doc: &Document, page_index: usize) -> Result<Vec<WidgetInfo>> {
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

    let mut out = Vec::new();
    for item in annots_arr {
        let xref = item.as_reference().map(|r| r.0).unwrap_or(0);
        let Some(dict) = doc.resolve(item).as_dict() else {
            continue;
        };
        if dict.get_name_str(b"Subtype") != Some("Widget") {
            continue;
        }
        let pdf_rect = dict
            .get(b"Rect".as_slice())
            .and_then(|o| doc.resolve(o).as_rect())
            .unwrap_or([0.0, 0.0, 0.0, 0.0]);
        let rect = Rect::new(
            pdf_rect[0],
            page_height - pdf_rect[3],
            pdf_rect[2],
            page_height - pdf_rect[1],
        )
        .normalize();

        let field_name = dict.get_string_decoded(b"T").unwrap_or_default();
        let field_type = match dict.get_name_str(b"FT") {
            Some("Btn") => "checkbox".to_string(),
            Some("Ch") => "choice".to_string(),
            Some("Sig") => "signature".to_string(),
            _ => "text".to_string(),
        };
        let field_value = dict
            .get_string_decoded(b"V")
            .or_else(|| dict.get_name_str(b"V").map(str::to_string))
            .unwrap_or_default();

        out.push(WidgetInfo {
            xref,
            field_name,
            field_type,
            field_value,
            rect,
        });
    }
    Ok(out)
}

/// Adds an interactive AcroForm `/Subtype /Widget` field to `page_index`.
pub fn add_page_widget(
    doc: &mut Document,
    page_index: usize,
    field_name: &str,
    field_type: &str,
    rect: Rect,
    field_value: &str,
) -> Result<WidgetInfo> {
    let page = doc.page(page_index)?.clone();
    let page_height = page.height();
    let norm = rect.normalize();
    let pdf_rect = [
        norm.x0,
        page_height - norm.y1,
        norm.x1,
        page_height - norm.y0,
    ];

    let ft_name: &[u8] = match field_type.to_ascii_lowercase().as_str() {
        "checkbox" | "button" | "btn" => b"Btn",
        "choice" | "combobox" | "listbox" | "ch" => b"Ch",
        "signature" | "sig" => b"Sig",
        _ => b"Tx",
    };

    let mut dict = PdfDict::new();
    dict.insert(b"Type".to_vec(), PdfObject::Name(b"Annot".to_vec()));
    dict.insert(b"Subtype".to_vec(), PdfObject::Name(b"Widget".to_vec()));
    dict.insert(b"FT".to_vec(), PdfObject::Name(ft_name.to_vec()));
    dict.insert(
        b"T".to_vec(),
        PdfObject::String(field_name.as_bytes().to_vec()),
    );
    dict.insert(
        b"V".to_vec(),
        PdfObject::String(field_value.as_bytes().to_vec()),
    );
    dict.insert(
        b"Rect".to_vec(),
        PdfObject::Array(vec![
            PdfObject::Real(pdf_rect[0]),
            PdfObject::Real(pdf_rect[1]),
            PdfObject::Real(pdf_rect[2]),
            PdfObject::Real(pdf_rect[3]),
        ]),
    );

    let widget_id = doc.allocate_object_id();
    doc.objects.insert(widget_id.0, PdfObject::Dict(dict));
    append_annot_to_page(doc, page.object_id, widget_id);

    // Also register in Catalog -> /AcroForm -> /Fields
    if let Some(root_id) = doc.trailer.get_ref(b"Root") {
        if let Some(cat) = doc
            .objects
            .get_mut(&root_id.0)
            .and_then(PdfObject::as_dict_mut)
        {
            let acro = cat
                .entry(b"AcroForm".to_vec())
                .or_insert_with(|| PdfObject::Dict(PdfDict::new()));
            if let Some(acro_dict) = acro.as_dict_mut() {
                let fields = acro_dict
                    .entry(b"Fields".to_vec())
                    .or_insert_with(|| PdfObject::Array(Vec::new()));
                if let PdfObject::Array(arr) = fields {
                    arr.push(PdfObject::Reference(widget_id));
                }
            }
        }
    }

    Ok(WidgetInfo {
        xref: widget_id.0,
        field_name: field_name.to_string(),
        field_type: field_type.to_string(),
        field_value: field_value.to_string(),
        rect: norm,
    })
}

/// Updates the value (`/V`) of a widget field matching `field_name` on `page_index`.
pub fn set_page_field_value(
    doc: &mut Document,
    page_index: usize,
    field_name: &str,
    new_value: &str,
) -> Result<bool> {
    let widgets = get_page_widgets(doc, page_index)?;
    let mut updated = false;
    for w in widgets {
        if w.field_name == field_name {
            if let Some(dict) = doc
                .objects
                .get_mut(&w.xref)
                .and_then(PdfObject::as_dict_mut)
            {
                dict.insert(
                    b"V".to_vec(),
                    PdfObject::String(new_value.as_bytes().to_vec()),
                );
                updated = true;
            }
        }
    }
    Ok(updated)
}
