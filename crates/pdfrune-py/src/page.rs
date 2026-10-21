use std::sync::{Arc, RwLock};

use pyo3::exceptions::PyRuntimeError;
use pyo3::prelude::*;

use crate::doc::{to_py_err, DocState};
use crate::rect::PyRect;

pub type WordTuple = (f64, f64, f64, f64, String, usize, usize, usize);
pub type BlockTuple = (f64, f64, f64, f64, String, usize, u8);

/// Python wrapper for a single PDF `Page`.
#[pyclass(name = "Page", module = "pdfrune")]
#[derive(Clone)]
pub struct PyPage {
    pub(crate) state: Arc<RwLock<DocState>>,
    pub(crate) page_index: usize,
}

impl PyPage {
    fn with_doc<F, R>(&self, f: F) -> PyResult<R>
    where
        F: FnOnce(&pdfrune::Document, &pdfrune::Page) -> PyResult<R>,
    {
        let guard = self
            .state
            .read()
            .map_err(|_| PyRuntimeError::new_err("Document lock poisoned"))?;
        let doc = guard.doc()?;
        let page = doc.page(self.page_index).map_err(to_py_err)?;
        f(doc, page)
    }

    fn with_doc_mut<F, R>(&self, f: F) -> PyResult<R>
    where
        F: FnOnce(&mut pdfrune::Document) -> PyResult<R>,
    {
        let mut guard = self
            .state
            .write()
            .map_err(|_| PyRuntimeError::new_err("Document lock poisoned"))?;
        let doc = guard.doc_mut()?;
        f(doc)
    }
}

#[pymethods]
impl PyPage {
    /// 0-based page index in the document.
    #[getter]
    pub fn number(&self) -> usize {
        self.page_index
    }

    /// Normalized visible bounding box (`Rect(0, 0, width, height)`).
    #[getter]
    pub fn rect(&self) -> PyResult<PyRect> {
        self.with_doc(|_, page| Ok(PyRect { inner: page.rect() }))
    }

    /// Page `/MediaBox`.
    #[getter]
    pub fn mediabox(&self) -> PyResult<PyRect> {
        self.with_doc(|_, page| {
            Ok(PyRect {
                inner: page.mediabox,
            })
        })
    }

    #[setter]
    pub fn set_mediabox(&self, rect: &PyRect) -> PyResult<()> {
        self.with_doc_mut(|doc| {
            doc.set_page_mediabox(self.page_index, rect.inner)
                .map_err(to_py_err)
        })
    }

    /// Page `/CropBox`.
    #[getter]
    pub fn cropbox(&self) -> PyResult<PyRect> {
        self.with_doc(|_, page| {
            Ok(PyRect {
                inner: page.cropbox,
            })
        })
    }

    #[setter]
    pub fn set_cropbox(&self, rect: &PyRect) -> PyResult<()> {
        self.with_doc_mut(|doc| {
            doc.set_page_cropbox(self.page_index, rect.inner)
                .map_err(to_py_err)
        })
    }

    /// Page `/BleedBox`.
    #[getter]
    pub fn bleedbox(&self) -> PyResult<PyRect> {
        self.with_doc(|_, page| {
            Ok(PyRect {
                inner: page.bleedbox,
            })
        })
    }

    /// Page `/TrimBox`.
    #[getter]
    pub fn trimbox(&self) -> PyResult<PyRect> {
        self.with_doc(|_, page| {
            Ok(PyRect {
                inner: page.trimbox,
            })
        })
    }

    /// Page `/ArtBox`.
    #[getter]
    pub fn artbox(&self) -> PyResult<PyRect> {
        self.with_doc(|_, page| Ok(PyRect { inner: page.artbox }))
    }

    /// Effective page width taking rotation into account.
    #[getter]
    pub fn width(&self) -> PyResult<f64> {
        self.with_doc(|_, page| Ok(page.width()))
    }

    /// Effective page height taking rotation into account.
    #[getter]
    pub fn height(&self) -> PyResult<f64> {
        self.with_doc(|_, page| Ok(page.height()))
    }

    /// Page rotation in degrees clockwise (`0`, `90`, `180`, `270`).
    #[getter]
    pub fn rotation(&self) -> PyResult<i32> {
        self.with_doc(|_, page| Ok(page.rotation))
    }

    #[setter(rotation)]
    pub fn set_rotation_prop(&self, deg: i64) -> PyResult<()> {
        self.set_rotation(deg)
    }

    /// Sets the page rotation in degrees clockwise (`0`, `90`, `180`, `270`).
    pub fn set_rotation(&self, deg: i64) -> PyResult<()> {
        self.with_doc_mut(|doc| {
            doc.set_page_rotation(self.page_index, deg)
                .map_err(to_py_err)
        })
    }

    /// Extracts plain text from the page in reading order (releases the GIL).
    pub fn extract_text(&self, py: Python<'_>) -> PyResult<String> {
        py.allow_threads(|| {
            self.with_doc(|doc, _| doc.extract_page_text(self.page_index).map_err(to_py_err))
        })
    }

    /// Extracts words as `(x0, y0, x1, y1, word, block_no, line_no, word_no)` tuples.
    pub fn extract_words(&self, py: Python<'_>) -> PyResult<Vec<WordTuple>> {
        let words = py.allow_threads(|| {
            self.with_doc(|doc, _| {
                let layout = doc.page_layout(self.page_index).map_err(to_py_err)?;
                Ok(layout.words())
            })
        })?;
        Ok(words
            .into_iter()
            .map(|w| {
                (
                    w.rect.x0, w.rect.y0, w.rect.x1, w.rect.y1, w.text, w.block_no, w.line_no,
                    w.word_no,
                )
            })
            .collect())
    }

    /// Extracts text blocks as `(x0, y0, x1, y1, text, block_no, block_type)` tuples.
    pub fn extract_blocks(&self, py: Python<'_>) -> PyResult<Vec<BlockTuple>> {
        let blocks = py.allow_threads(|| {
            self.with_doc(|doc, _| {
                let layout = doc.page_layout(self.page_index).map_err(to_py_err)?;
                Ok(layout.blocks)
            })
        })?;
        Ok(blocks
            .into_iter()
            .map(|b| {
                let text = b.text();
                (
                    b.rect.x0,
                    b.rect.y0,
                    b.rect.x1,
                    b.rect.y1,
                    text,
                    b.block_no,
                    b.block_type,
                )
            })
            .collect())
    }

    /// PyMuPDF-compatible `get_text(option="text" | "words" | "blocks" | "json")`.
    #[pyo3(signature = (option="text"))]
    pub fn get_text<'py>(&self, py: Python<'py>, option: &str) -> PyResult<Bound<'py, PyAny>> {
        match option {
            "text" | "" => Ok(self.extract_text(py)?.into_pyobject(py)?.into_any()),
            "words" => Ok(self.extract_words(py)?.into_pyobject(py)?.into_any()),
            "blocks" => Ok(self.extract_blocks(py)?.into_pyobject(py)?.into_any()),
            "markdown" | "md" => Ok(self.to_markdown(py)?.into_pyobject(py)?.into_any()),
            "html" => Ok(self.to_html(py)?.into_pyobject(py)?.into_any()),
            "json" => {
                let json_str = py.allow_threads(|| {
                    self.with_doc(|doc, _| {
                        let layout = doc.page_layout(self.page_index).map_err(to_py_err)?;
                        serde_json::to_string_pretty(&layout)
                            .map_err(|e| pyo3::exceptions::PyValueError::new_err(e.to_string()))
                    })
                })?;
                Ok(json_str.into_pyobject(py)?.into_any())
            }
            other => Err(pyo3::exceptions::PyValueError::new_err(format!(
                "Unsupported get_text option: {other}"
            ))),
        }
    }

    /// Exports structured page text as Markdown (headers, lists, paragraphs).
    pub fn to_markdown(&self, py: Python<'_>) -> PyResult<String> {
        py.allow_threads(|| {
            self.with_doc(|doc, _| doc.page_to_markdown(self.page_index).map_err(to_py_err))
        })
    }

    /// Exports structured page text as HTML (`<h1>`, `<h2>`, `<ul>`, `<p>`).
    pub fn to_html(&self, py: Python<'_>) -> PyResult<String> {
        py.allow_threads(|| {
            self.with_doc(|doc, _| doc.page_to_html(self.page_index).map_err(to_py_err))
        })
    }

    /// Searches for `needle` (case-insensitive) on the page and returns matching `Rect` list.
    pub fn search(&self, py: Python<'_>, needle: &str) -> PyResult<Vec<PyRect>> {
        let rects = py.allow_threads(|| {
            self.with_doc(|doc, _| doc.search_page(self.page_index, needle).map_err(to_py_err))
        })?;
        Ok(rects.into_iter().map(|inner| PyRect { inner }).collect())
    }

    /// PyMuPDF alias for `search(needle)`.
    pub fn search_for(&self, py: Python<'_>, needle: &str) -> PyResult<Vec<PyRect>> {
        self.search(py, needle)
    }

    /// Inserts text at `point` (top-down coordinates) onto the page.
    #[pyo3(signature = (point, text, fontsize=12.0, color=(0.0, 0.0, 0.0)))]
    pub fn insert_text(
        &self,
        point: &crate::rect::PyPoint,
        text: &str,
        fontsize: f64,
        color: (f64, f64, f64),
    ) -> PyResult<()> {
        self.with_doc_mut(|doc| {
            doc.insert_text(
                self.page_index,
                point.inner,
                text,
                fontsize,
                [color.0, color.1, color.2],
            )
            .map_err(to_py_err)
        })
    }

    /// Draws a rectangle onto the page.
    #[pyo3(signature = (rect, color=Some((0.0, 0.0, 0.0)), fill=None, width=1.0))]
    pub fn draw_rect(
        &self,
        rect: &PyRect,
        color: Option<(f64, f64, f64)>,
        fill: Option<(f64, f64, f64)>,
        width: f64,
    ) -> PyResult<()> {
        let stroke = color.map(|(r, g, b)| [r, g, b]);
        let fill_rgb = fill.map(|(r, g, b)| [r, g, b]);
        self.with_doc_mut(|doc| {
            doc.draw_rect(self.page_index, rect.inner, stroke, fill_rgb, width)
                .map_err(to_py_err)
        })
    }

    /// Rasterizes the page to an RGBA8888 `Pixmap` at `dpi` (releases the GIL).
    #[pyo3(signature = (dpi=72.0))]
    pub fn render(&self, py: Python<'_>, dpi: f64) -> PyResult<crate::pixmap::PyPixmap> {
        let inner = py.allow_threads(|| {
            self.with_doc(|doc, _| doc.render_page(self.page_index, dpi).map_err(to_py_err))
        })?;
        Ok(crate::pixmap::PyPixmap { inner })
    }

    /// PyMuPDF-compatible `get_pixmap(dpi=72.0)`.
    #[pyo3(signature = (*, dpi=72.0))]
    pub fn get_pixmap(&self, py: Python<'_>, dpi: f64) -> PyResult<crate::pixmap::PyPixmap> {
        self.render(py, dpi)
    }

    /// Exports the page as a scalable SVG string.
    pub fn to_svg(&self, py: Python<'_>) -> PyResult<String> {
        py.allow_threads(|| {
            self.with_doc(|doc, _| doc.render_page_svg(self.page_index).map_err(to_py_err))
        })
    }

    /// PyMuPDF alias `get_svg_image()` for `to_svg()`.
    pub fn get_svg_image(&self, py: Python<'_>) -> PyResult<String> {
        self.to_svg(py)
    }

    /// Returns all hyperlinks on the page as a list of dictionaries (`kind`, `from`, `uri`, `page`, `xref`).
    pub fn get_links<'py>(
        &self,
        py: Python<'py>,
    ) -> PyResult<Vec<Bound<'py, pyo3::types::PyDict>>> {
        let links =
            self.with_doc(|doc, _| doc.get_page_links(self.page_index).map_err(to_py_err))?;
        let mut out = Vec::with_capacity(links.len());
        for l in links {
            let d = pyo3::types::PyDict::new(py);
            d.set_item("xref", l.xref)?;
            d.set_item("kind", l.kind)?;
            d.set_item("from", PyRect { inner: l.from })?;
            if let Some(u) = l.uri {
                d.set_item("uri", u)?;
            }
            if let Some(p) = l.page {
                d.set_item("page", p)?;
            }
            out.push(d);
        }
        Ok(out)
    }

    /// Inserts a hyperlink dictionary (`{"from": Rect, "uri": "..."}` or `{"from": Rect, "page": 1}`) onto the page.
    pub fn insert_link(&self, link_dict: &Bound<'_, pyo3::types::PyDict>) -> PyResult<u32> {
        let from_obj = link_dict.get_item("from")?.ok_or_else(|| {
            pyo3::exceptions::PyValueError::new_err("link_dict missing 'from' Rect")
        })?;
        let from_rect = extract_rect(&from_obj)?;
        let uri: Option<String> = match link_dict.get_item("uri")? {
            Some(v) if !v.is_none() => Some(v.extract()?),
            _ => None,
        };
        let target_page: Option<usize> = match link_dict.get_item("page")? {
            Some(v) if !v.is_none() => Some(v.extract()?),
            _ => None,
        };
        self.with_doc_mut(|doc| {
            doc.insert_page_link(self.page_index, from_rect, uri.as_deref(), target_page)
                .map_err(to_py_err)
        })
    }

    /// Deletes a hyperlink from the page given its `link_dict` (as returned by `get_links()`).
    pub fn delete_link(&self, link_dict: &Bound<'_, pyo3::types::PyDict>) -> PyResult<()> {
        let xref: Option<u32> = match link_dict.get_item("xref")? {
            Some(v) if !v.is_none() => v.extract().ok(),
            _ => None,
        };
        let from_rect: Option<pdfrune::Rect> = match link_dict.get_item("from")? {
            Some(v) if !v.is_none() => extract_rect(&v).ok(),
            _ => None,
        };
        self.with_doc_mut(|doc| {
            doc.delete_page_link(self.page_index, xref, from_rect)
                .map_err(to_py_err)
        })
    }

    /// Returns a list of `(xref, smask, width, height, bpc, colorspace, alt, name, filter)` tuples for images on the page.
    #[pyo3(signature = (full=false))]
    pub fn get_images(&self, full: bool) -> PyResult<Vec<ImageInfoTuple>> {
        let _ = full;
        let imgs =
            self.with_doc(|doc, _| doc.get_page_images(self.page_index).map_err(to_py_err))?;
        Ok(imgs
            .into_iter()
            .map(|i| {
                (
                    i.xref,
                    i.smask,
                    i.width,
                    i.height,
                    i.bpc,
                    i.colorspace,
                    i.alt_colorspace,
                    i.name,
                    i.filter,
                )
            })
            .collect())
    }

    /// Inserts an image onto the page at `rect` from `filename`, `stream` (PNG bytes), or `pixmap`.
    #[pyo3(signature = (rect, *, filename=None, stream=None, pixmap=None))]
    pub fn insert_image(
        &self,
        rect: &Bound<'_, PyAny>,
        filename: Option<std::path::PathBuf>,
        stream: Option<Vec<u8>>,
        pixmap: Option<&crate::pixmap::PyPixmap>,
    ) -> PyResult<u32> {
        let target_rect = extract_rect(rect)?;
        if let Some(pix) = pixmap {
            let inner = pix.inner.clone();
            return self.with_doc_mut(|doc| {
                doc.insert_image_pixmap(self.page_index, target_rect, &inner)
                    .map_err(to_py_err)
            });
        }
        let bytes = if let Some(b) = stream {
            b
        } else if let Some(path) = filename {
            std::fs::read(&path).map_err(|e| {
                pyo3::exceptions::PyValueError::new_err(format!("Failed to read {:?}: {e}", path))
            })?
        } else {
            return Err(pyo3::exceptions::PyValueError::new_err(
                "insert_image requires 'filename', 'stream', or 'pixmap'",
            ));
        };
        self.with_doc_mut(|doc| {
            doc.insert_image_bytes(self.page_index, target_rect, &bytes)
                .map_err(to_py_err)
        })
    }

    /// Returns all annotations on the page as a list of dictionaries.
    pub fn annots<'py>(&self, py: Python<'py>) -> PyResult<Vec<Bound<'py, pyo3::types::PyDict>>> {
        let list =
            self.with_doc(|doc, _| doc.get_page_annots(self.page_index).map_err(to_py_err))?;
        let mut out = Vec::with_capacity(list.len());
        for a in list {
            out.push(annot_to_pydict(py, a)?);
        }
        Ok(out)
    }

    /// Adds a yellow highlight annotation (`/Subtype /Highlight`) over `rect`.
    pub fn add_highlight_annot<'py>(
        &self,
        py: Python<'py>,
        rect: &Bound<'py, PyAny>,
    ) -> PyResult<Bound<'py, pyo3::types::PyDict>> {
        let r = extract_rect(rect)?;
        let info = self.with_doc_mut(|doc| {
            doc.add_page_annot(self.page_index, "Highlight", r, None, [1.0, 1.0, 0.0], None)
                .map_err(to_py_err)
        })?;
        annot_to_pydict(py, info)
    }

    /// Adds a text (sticky note) annotation (`/Subtype /Text`) at `point`.
    pub fn add_text_annot<'py>(
        &self,
        py: Python<'py>,
        point: &crate::rect::PyPoint,
        text: &str,
    ) -> PyResult<Bound<'py, pyo3::types::PyDict>> {
        let r = pdfrune::Rect::new(
            point.inner.x,
            point.inner.y,
            point.inner.x + 20.0,
            point.inner.y + 20.0,
        );
        let info = self.with_doc_mut(|doc| {
            doc.add_page_annot(
                self.page_index,
                "Text",
                r,
                Some(text),
                [1.0, 0.9, 0.2],
                None,
            )
            .map_err(to_py_err)
        })?;
        annot_to_pydict(py, info)
    }

    /// Adds a rectangle annotation (`/Subtype /Square`) over `rect`.
    pub fn add_rect_annot<'py>(
        &self,
        py: Python<'py>,
        rect: &Bound<'py, PyAny>,
    ) -> PyResult<Bound<'py, pyo3::types::PyDict>> {
        let r = extract_rect(rect)?;
        let info = self.with_doc_mut(|doc| {
            doc.add_page_annot(self.page_index, "Square", r, None, [1.0, 0.0, 0.0], None)
                .map_err(to_py_err)
        })?;
        annot_to_pydict(py, info)
    }

    /// Deletes an annotation dictionary (as returned by `annots()` or `add_*_annot()`) from the page.
    pub fn delete_annot(&self, annot: &Bound<'_, pyo3::types::PyDict>) -> PyResult<()> {
        let xref: u32 = annot
            .get_item("xref")?
            .ok_or_else(|| pyo3::exceptions::PyValueError::new_err("annot dict missing 'xref'"))?
            .extract()?;
        self.with_doc_mut(|doc| {
            doc.delete_page_annot(self.page_index, xref)
                .map_err(to_py_err)
        })
    }

    /// Adds a redaction annotation (`/Subtype /Redact`) over `rect`.
    #[pyo3(signature = (rect, text=None, fill=(0.0, 0.0, 0.0)))]
    pub fn add_redact_annot<'py>(
        &self,
        py: Python<'py>,
        rect: &Bound<'py, PyAny>,
        text: Option<&str>,
        fill: (f64, f64, f64),
    ) -> PyResult<Bound<'py, pyo3::types::PyDict>> {
        let r = extract_rect(rect)?;
        let info = self.with_doc_mut(|doc| {
            doc.add_page_annot(
                self.page_index,
                "Redact",
                r,
                None,
                [fill.0, fill.1, fill.2],
                text,
            )
            .map_err(to_py_err)
        })?;
        annot_to_pydict(py, info)
    }

    /// Applies all pending redaction annotations on the page, permanently scrubbing underlying text.
    pub fn apply_redactions(&self) -> PyResult<usize> {
        self.with_doc_mut(|doc| {
            doc.apply_page_redactions(self.page_index)
                .map_err(to_py_err)
        })
    }

    /// Returns all interactive AcroForm `/Widget` fields on the page.
    pub fn widgets<'py>(&self, py: Python<'py>) -> PyResult<Vec<Bound<'py, pyo3::types::PyDict>>> {
        let list =
            self.with_doc(|doc, _| doc.get_page_widgets(self.page_index).map_err(to_py_err))?;
        let mut out = Vec::with_capacity(list.len());
        for w in list {
            out.push(widget_to_pydict(py, w)?);
        }
        Ok(out)
    }

    /// Adds an interactive AcroForm `/Widget` field to the page.
    pub fn add_widget<'py>(
        &self,
        py: Python<'py>,
        widget: &Bound<'py, pyo3::types::PyDict>,
    ) -> PyResult<Bound<'py, pyo3::types::PyDict>> {
        let field_name: String = widget
            .get_item("field_name")?
            .ok_or_else(|| pyo3::exceptions::PyValueError::new_err("widget missing 'field_name'"))?
            .extract()?;
        let field_type: String = match widget.get_item("field_type")? {
            Some(v) if !v.is_none() => v.extract()?,
            _ => "text".to_string(),
        };
        let rect_obj = widget
            .get_item("rect")?
            .ok_or_else(|| pyo3::exceptions::PyValueError::new_err("widget missing 'rect'"))?;
        let rect = extract_rect(&rect_obj)?;
        let field_value: String = match widget.get_item("field_value")? {
            Some(v) if !v.is_none() => v.extract()?,
            _ => String::new(),
        };

        let info = self.with_doc_mut(|doc| {
            doc.add_page_widget(
                self.page_index,
                &field_name,
                &field_type,
                rect,
                &field_value,
            )
            .map_err(to_py_err)
        })?;
        widget_to_pydict(py, info)
    }

    /// Updates the value of an AcroForm field on the page by `field_name`.
    pub fn set_field_value(&self, field_name: &str, value: &str) -> PyResult<bool> {
        self.with_doc_mut(|doc| {
            doc.set_page_field_value(self.page_index, field_name, value)
                .map_err(to_py_err)
        })
    }

    /// Detects structured tables on the page and returns a list of `Table` objects.
    pub fn find_tables(&self, py: Python<'_>) -> PyResult<Vec<PyTable>> {
        let tables = py.allow_threads(|| {
            self.with_doc(|doc, _| doc.find_page_tables(self.page_index).map_err(to_py_err))
        })?;
        Ok(tables.into_iter().map(|inner| PyTable { inner }).collect())
    }

    fn __repr__(&self) -> String {
        format!("Page({})", self.page_index)
    }
}

/// Structured table detected on a PDF page.
#[pyclass(name = "Table", module = "pdfrune")]
#[derive(Debug, Clone)]
pub struct PyTable {
    pub inner: pdfrune::DetectedTable,
}

#[pymethods]
impl PyTable {
    /// Number of rows in the detected table.
    #[getter]
    pub fn row_count(&self) -> usize {
        self.inner.row_count()
    }

    /// Number of columns in the detected table.
    #[getter]
    pub fn col_count(&self) -> usize {
        self.inner.col_count()
    }

    /// Bounding box `Rect` enclosing the table.
    #[getter]
    pub fn bbox(&self) -> PyRect {
        PyRect {
            inner: self.inner.bbox,
        }
    }

    /// First row of the table as column headers.
    #[getter]
    pub fn header(&self) -> Vec<String> {
        self.inner.header()
    }

    /// Extracts the 2D table cells as `list[list[str]]`.
    pub fn extract(&self) -> Vec<Vec<String>> {
        self.inner.extract()
    }

    /// Exports the table as a GitHub-Flavored Markdown table string.
    pub fn to_markdown(&self) -> String {
        self.inner.to_markdown()
    }

    /// Exports data rows (excluding header) as a list of `dict[header, cell_value]`.
    pub fn to_dicts<'py>(&self, py: Python<'py>) -> PyResult<Vec<Bound<'py, pyo3::types::PyDict>>> {
        let rows = self.inner.to_dicts();
        let mut out = Vec::with_capacity(rows.len());
        for row in rows {
            let d = pyo3::types::PyDict::new(py);
            for (k, v) in row {
                d.set_item(k, v)?;
            }
            out.push(d);
        }
        Ok(out)
    }

    fn __repr__(&self) -> String {
        format!(
            "Table(rows={}, cols={})",
            self.row_count(),
            self.col_count()
        )
    }
}

fn annot_to_pydict<'py>(
    py: Python<'py>,
    a: pdfrune::AnnotInfo,
) -> PyResult<Bound<'py, pyo3::types::PyDict>> {
    let d = pyo3::types::PyDict::new(py);
    d.set_item("xref", a.xref)?;
    d.set_item("type", a.subtype)?;
    d.set_item("rect", PyRect { inner: a.rect })?;
    d.set_item("contents", a.contents)?;
    d.set_item("color", (a.color[0], a.color[1], a.color[2]))?;
    if let Some(ot) = a.overlay_text {
        d.set_item("overlay_text", ot)?;
    }
    Ok(d)
}

fn widget_to_pydict<'py>(
    py: Python<'py>,
    w: pdfrune::WidgetInfo,
) -> PyResult<Bound<'py, pyo3::types::PyDict>> {
    let d = pyo3::types::PyDict::new(py);
    d.set_item("xref", w.xref)?;
    d.set_item("field_name", w.field_name)?;
    d.set_item("field_type", w.field_type)?;
    d.set_item("field_value", w.field_value)?;
    d.set_item("rect", PyRect { inner: w.rect })?;
    Ok(d)
}

pub type ImageInfoTuple = (u32, u32, u32, u32, u8, String, String, String, String);

pub(crate) fn extract_rect(obj: &Bound<'_, PyAny>) -> PyResult<pdfrune::Rect> {
    if let Ok(r) = obj.extract::<PyRect>() {
        return Ok(r.inner);
    }
    if let Ok((x0, y0, x1, y1)) = obj.extract::<(f64, f64, f64, f64)>() {
        return Ok(pdfrune::Rect::new(x0, y0, x1, y1));
    }
    Err(pyo3::exceptions::PyTypeError::new_err(
        "Expected Rect or 4-tuple of floats",
    ))
}
