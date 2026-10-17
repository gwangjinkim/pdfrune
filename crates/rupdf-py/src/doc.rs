use std::collections::BTreeMap;
use std::path::PathBuf;
use std::sync::{Arc, RwLock};

use pyo3::exceptions::{PyIndexError, PyRuntimeError, PyTypeError, PyValueError};
use pyo3::prelude::*;
use pyo3::types::{PyList, PySlice};

use rupdf_core::{Document, RupdfError};

use crate::page::PyPage;

pub(crate) struct DocState {
    pub inner: Option<Document>,
}

impl DocState {
    pub fn doc(&self) -> PyResult<&Document> {
        self.inner
            .as_ref()
            .ok_or_else(|| PyValueError::new_err("Document is closed"))
    }

    pub fn doc_mut(&mut self) -> PyResult<&mut Document> {
        self.inner
            .as_mut()
            .ok_or_else(|| PyValueError::new_err("Document is closed"))
    }
}

pub(crate) fn to_py_err(err: RupdfError) -> PyErr {
    match err {
        RupdfError::PageOutOfBounds { index, count } => PyIndexError::new_err(format!(
            "Page index {index} out of bounds (page count: {count})"
        )),
        other => PyValueError::new_err(other.to_string()),
    }
}

/// Python wrapper for a PDF `Document`.
#[pyclass(name = "Document", module = "rupdf")]
#[derive(Clone)]
pub struct PyDocument {
    pub(crate) state: Arc<RwLock<DocState>>,
}

impl PyDocument {
    pub fn from_core(doc: Document) -> Self {
        Self {
            state: Arc::new(RwLock::new(DocState { inner: Some(doc) })),
        }
    }

    pub(crate) fn with_doc<F, R>(&self, f: F) -> PyResult<R>
    where
        F: FnOnce(&Document) -> PyResult<R>,
    {
        let guard = self
            .state
            .read()
            .map_err(|_| PyRuntimeError::new_err("Document lock poisoned"))?;
        f(guard.doc()?)
    }

    pub(crate) fn with_doc_mut<F, R>(&self, f: F) -> PyResult<R>
    where
        F: FnOnce(&mut Document) -> PyResult<R>,
    {
        let mut guard = self
            .state
            .write()
            .map_err(|_| PyRuntimeError::new_err("Document lock poisoned"))?;
        f(guard.doc_mut()?)
    }
}

#[pymethods]
impl PyDocument {
    #[new]
    #[pyo3(signature = (filename=None, *, stream=None))]
    pub fn new(
        py: Python<'_>,
        filename: Option<&Bound<'_, PyAny>>,
        stream: Option<&Bound<'_, PyAny>>,
    ) -> PyResult<Self> {
        open_document(py, filename, stream)
    }

    /// Number of pages in the document.
    #[getter]
    pub fn page_count(&self) -> PyResult<usize> {
        self.with_doc(|doc| Ok(doc.page_count()))
    }

    /// Document metadata dictionary (`title`, `author`, `subject`, `keywords`, `creator`, `producer`, `format`).
    #[getter]
    pub fn metadata(&self) -> PyResult<BTreeMap<String, String>> {
        self.with_doc(|doc| Ok(doc.metadata()))
    }

    /// Updates metadata entries in the document's `/Info` dictionary.
    pub fn set_metadata(&self, metadata: BTreeMap<String, String>) -> PyResult<()> {
        self.with_doc_mut(|doc| {
            doc.set_metadata(&metadata);
            Ok(())
        })
    }

    /// True if the document contains an `/Encrypt` dictionary.
    #[getter]
    pub fn is_encrypted(&self) -> PyResult<bool> {
        self.with_doc(|doc| Ok(doc.is_encrypted))
    }

    /// True if the document is encrypted and has not yet been authenticated.
    #[getter]
    pub fn needs_pass(&self) -> PyResult<bool> {
        self.with_doc(|doc| Ok(doc.is_encrypted && !doc.is_authenticated))
    }

    /// Authenticates an encrypted PDF document with `password`.
    pub fn authenticate(&self, password: &str) -> PyResult<bool> {
        self.with_doc_mut(|doc| doc.authenticate(password).map_err(to_py_err))
    }

    /// True if `close()` has been called on this document.
    #[getter]
    pub fn is_closed(&self) -> PyResult<bool> {
        let guard = self
            .state
            .read()
            .map_err(|_| PyRuntimeError::new_err("Document lock poisoned"))?;
        Ok(guard.inner.is_none())
    }

    /// Loads a page by 0-based index.
    pub fn load_page(&self, page_id: isize) -> PyResult<PyPage> {
        let count = self.page_count()? as isize;
        let resolved = if page_id < 0 {
            count + page_id
        } else {
            page_id
        };
        if resolved < 0 || resolved >= count {
            return Err(PyIndexError::new_err(format!(
                "Page index {page_id} out of range (page count: {count})"
            )));
        }
        Ok(PyPage {
            state: Arc::clone(&self.state),
            page_index: resolved as usize,
        })
    }

    /// Closes the document and releases underlying memory.
    pub fn close(&self) -> PyResult<()> {
        let mut guard = self
            .state
            .write()
            .map_err(|_| PyRuntimeError::new_err("Document lock poisoned"))?;
        guard.inner = None;
        Ok(())
    }

    fn __enter__(slf: PyRef<'_, Self>) -> PyRef<'_, Self> {
        slf
    }

    #[pyo3(signature = (_exc_type=None, _exc_value=None, _traceback=None))]
    fn __exit__(
        &self,
        _exc_type: Option<&Bound<'_, PyAny>>,
        _exc_value: Option<&Bound<'_, PyAny>>,
        _traceback: Option<&Bound<'_, PyAny>>,
    ) -> PyResult<bool> {
        self.close()?;
        Ok(false)
    }

    fn __len__(&self) -> PyResult<usize> {
        self.page_count()
    }

    fn __getitem__<'py>(
        &self,
        py: Python<'py>,
        index: &Bound<'py, PyAny>,
    ) -> PyResult<Bound<'py, PyAny>> {
        if let Ok(slice) = index.downcast::<PySlice>() {
            let count = self.page_count()? as isize;
            let indices = slice.indices(count)?;
            let mut pages = Vec::new();
            let mut i = indices.start;
            while (indices.step > 0 && i < indices.stop) || (indices.step < 0 && i > indices.stop) {
                pages.push(PyPage {
                    state: Arc::clone(&self.state),
                    page_index: i as usize,
                });
                i += indices.step;
            }
            let list = PyList::new(py, pages)?;
            return Ok(list.into_any());
        }

        let idx: isize = index.extract()?;
        let page = self.load_page(idx)?;
        Ok(Bound::new(py, page)?.into_any())
    }

    fn __repr__(&self) -> String {
        match self.page_count() {
            Ok(n) => format!("Document(page_count={n})"),
            Err(_) => "Document(closed)".to_string(),
        }
    }
}

/// Opens a PDF document from a file path, raw bytes, a file-like object, or creates an empty document.
#[pyfunction]
#[pyo3(signature = (filename=None, *, stream=None))]
pub fn open_document(
    py: Python<'_>,
    filename: Option<&Bound<'_, PyAny>>,
    stream: Option<&Bound<'_, PyAny>>,
) -> PyResult<PyDocument> {
    if let Some(stm) = stream {
        let bytes = extract_bytes_or_read(stm)?;
        let doc = py
            .allow_threads(|| Document::from_bytes(&bytes))
            .map_err(to_py_err)?;
        return Ok(PyDocument::from_core(doc));
    }

    if let Some(arg) = filename {
        // Support bytes passed directly as first positional argument: `rupdf.open(b"%PDF...")`
        if let Ok(bytes) = arg.extract::<Vec<u8>>() {
            let doc = py
                .allow_threads(|| Document::from_bytes(&bytes))
                .map_err(to_py_err)?;
            return Ok(PyDocument::from_core(doc));
        }

        // Support file-like objects with `.read()`
        if arg.hasattr("read")? {
            let bytes = extract_bytes_or_read(arg)?;
            let doc = py
                .allow_threads(|| Document::from_bytes(&bytes))
                .map_err(to_py_err)?;
            return Ok(PyDocument::from_core(doc));
        }

        // Support `str` or `os.PathLike` (`pathlib.Path`)
        if let Ok(path) = arg.extract::<PathBuf>() {
            let doc = py
                .allow_threads(|| Document::open(&path))
                .map_err(to_py_err)?;
            return Ok(PyDocument::from_core(doc));
        }

        return Err(PyTypeError::new_err(
            "Expected str, PathLike, bytes, or file-like object in rupdf.open()",
        ));
    }

    Ok(PyDocument::from_core(Document::new()))
}

fn extract_bytes_or_read(obj: &Bound<'_, PyAny>) -> PyResult<Vec<u8>> {
    if let Ok(bytes) = obj.extract::<Vec<u8>>() {
        return Ok(bytes);
    }
    if obj.hasattr("read")? {
        let data = obj.call_method0("read")?;
        return data.extract::<Vec<u8>>();
    }
    Err(PyTypeError::new_err(
        "Stream must be bytes, bytearray, or a binary file-like object with .read()",
    ))
}
