use std::sync::{Arc, RwLock};

use pyo3::exceptions::PyRuntimeError;
use pyo3::prelude::*;

use crate::doc::{to_py_err, DocState};
use crate::rect::PyRect;

/// Python wrapper for a single PDF `Page`.
#[pyclass(name = "Page", module = "rupdf")]
#[derive(Clone)]
pub struct PyPage {
    pub(crate) state: Arc<RwLock<DocState>>,
    pub(crate) page_index: usize,
}

impl PyPage {
    fn with_doc<F, R>(&self, f: F) -> PyResult<R>
    where
        F: FnOnce(&rupdf_core::Document, &rupdf_core::Page) -> PyResult<R>,
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
        F: FnOnce(&mut rupdf_core::Document) -> PyResult<R>,
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

    #[setter]
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

    fn __repr__(&self) -> String {
        format!("Page({})", self.page_index)
    }
}
