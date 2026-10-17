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

    /// Creates and inserts a new blank page with `(width, height)`.
    #[pyo3(signature = (pno=-1, width=612.0, height=792.0))]
    pub fn new_page(&self, pno: isize, width: f64, height: f64) -> PyResult<PyPage> {
        let idx = self.with_doc_mut(|doc| {
            let insert_at = if pno < 0 { None } else { Some(pno as usize) };
            doc.new_page(insert_at, width, height).map_err(to_py_err)
        })?;
        Ok(PyPage {
            state: Arc::clone(&self.state),
            page_index: idx,
        })
    }

    /// Deletes a page at 0-based index `pno` (supports negative indexing).
    pub fn delete_page(&self, pno: isize) -> PyResult<()> {
        self.with_doc_mut(|doc| {
            let count = doc.page_count() as isize;
            let resolved = if pno < 0 { count + pno } else { pno };
            if resolved < 0 || resolved >= count {
                return Err(PyIndexError::new_err(format!(
                    "Page index {pno} out of range (page count: {count})"
                )));
            }
            doc.delete_page(resolved as usize).map_err(to_py_err)
        })
    }

    /// Reorders or filters the document's pages in-place to match `page_indices`.
    pub fn select(&self, page_indices: Vec<usize>) -> PyResult<()> {
        self.with_doc_mut(|doc| doc.select_pages(&page_indices).map_err(to_py_err))
    }

    /// Extracts a subset of pages into a new standalone `Document`.
    pub fn extract_pages(&self, page_indices: Vec<usize>) -> PyResult<PyDocument> {
        let extracted = self.with_doc(|doc| doc.extract_pages(&page_indices).map_err(to_py_err))?;
        Ok(PyDocument::from_core(extracted))
    }

    /// Inserts pages from `other` into this document.
    #[pyo3(signature = (other, from_page=0, to_page=-1, start_at=-1))]
    pub fn insert_pdf(
        &self,
        other: &PyDocument,
        from_page: isize,
        to_page: isize,
        start_at: isize,
    ) -> PyResult<()> {
        let other_doc = other.with_doc(|d| Ok(d.clone()))?;
        self.with_doc_mut(|doc| {
            let other_count = other_doc.page_count() as isize;
            if other_count == 0 {
                return Ok(());
            }
            let fp = if from_page < 0 {
                (other_count + from_page).max(0) as usize
            } else {
                from_page as usize
            };
            let tp = if to_page < 0 {
                (other_count + to_page).max(0) as usize
            } else {
                to_page as usize
            };
            let sa = if start_at < 0 {
                None
            } else {
                Some(start_at as usize)
            };
            doc.insert_pdf(&other_doc, Some(fp), Some(tp), sa)
                .map_err(to_py_err)
        })
    }

    /// Serializes the document to PDF bytes.
    #[pyo3(signature = (*, deflate=false))]
    pub fn to_bytes<'py>(
        &self,
        py: Python<'py>,
        deflate: bool,
    ) -> PyResult<Bound<'py, pyo3::types::PyBytes>> {
        let bytes =
            py.allow_threads(|| self.with_doc(|doc| doc.to_bytes(deflate).map_err(to_py_err)))?;
        Ok(pyo3::types::PyBytes::new(py, &bytes))
    }

    /// Alias for `to_bytes` matching `fitz.Document.write()`.
    #[pyo3(signature = (*, deflate=false))]
    pub fn write<'py>(
        &self,
        py: Python<'py>,
        deflate: bool,
    ) -> PyResult<Bound<'py, pyo3::types::PyBytes>> {
        self.to_bytes(py, deflate)
    }

    /// Saves the document to disk at `filename`.
    #[pyo3(signature = (filename, *, deflate=false))]
    pub fn save(&self, py: Python<'_>, filename: PathBuf, deflate: bool) -> PyResult<()> {
        py.allow_threads(|| self.with_doc(|doc| doc.save(&filename, deflate).map_err(to_py_err)))
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

/// Merges a sequence of `Document` objects or file paths into a single new `Document`.
#[pyfunction]
pub fn merge_pdfs(py: Python<'_>, documents: &Bound<'_, PyAny>) -> PyResult<PyDocument> {
    let mut out = Document::new();
    for item in documents.try_iter()? {
        let item = item?;
        if let Ok(py_doc) = item.extract::<PyDocument>() {
            let cloned = py_doc.with_doc(|d| Ok(d.clone()))?;
            out.insert_pdf(&cloned, None, None, None)
                .map_err(to_py_err)?;
        } else {
            let opened = open_document(py, Some(&item), None)?;
            let cloned = opened.with_doc(|d| Ok(d.clone()))?;
            out.insert_pdf(&cloned, None, None, None)
                .map_err(to_py_err)?;
        }
    }
    Ok(PyDocument::from_core(out))
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
