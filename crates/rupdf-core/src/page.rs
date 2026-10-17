use crate::geometry::Rect;
use crate::parser::object::{ObjectId, PdfDict};

/// Resolved representation of a single PDF Page and its inherited attributes.
#[derive(Debug, Clone)]
pub struct Page {
    /// 0-based page index in the document.
    pub index: usize,
    /// Object ID of the `/Page` dictionary in the PDF object store.
    pub object_id: ObjectId,
    /// `/MediaBox` rectangle in default user space units (1/72 inch).
    pub mediabox: Rect,
    /// `/CropBox` rectangle (defaults to `mediabox` if not specified).
    pub cropbox: Rect,
    /// `/BleedBox` rectangle (defaults to `cropbox`).
    pub bleedbox: Rect,
    /// `/TrimBox` rectangle (defaults to `cropbox`).
    pub trimbox: Rect,
    /// `/ArtBox` rectangle (defaults to `cropbox`).
    pub artbox: Rect,
    /// Page rotation in degrees clockwise (normalized to `0`, `90`, `180`, or `270`).
    pub rotation: i32,
    /// Merged `/Resources` dictionary (combining inherited parent resources and page resources).
    pub resources: PdfDict,
}

impl Page {
    /// Effective visible bounding box (`cropbox`), taking rotation into account for width/height.
    pub fn rect(&self) -> Rect {
        let w = self.width();
        let h = self.height();
        Rect::new(0.0, 0.0, w, h)
    }

    /// Effective page width after applying `rotation`.
    pub fn width(&self) -> f64 {
        if self.rotation == 90 || self.rotation == 270 {
            self.cropbox.height()
        } else {
            self.cropbox.width()
        }
    }

    /// Effective page height after applying `rotation`.
    pub fn height(&self) -> f64 {
        if self.rotation == 90 || self.rotation == 270 {
            self.cropbox.width()
        } else {
            self.cropbox.height()
        }
    }
}

/// Normalizes any integer rotation angle to `0`, `90`, `180`, or `270`.
#[inline]
pub fn normalize_rotation(deg: i64) -> i32 {
    let r = deg.rem_euclid(360);
    match r {
        90 | 180 | 270 => r as i32,
        _ => 0,
    }
}
