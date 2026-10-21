pub mod doc;
pub mod page;
pub mod pixmap;
pub mod rect;

use pyo3::prelude::*;

use crate::doc::{batch_extract, batch_render, merge_pdfs, open_document, PyDocument};
use crate::page::{PyPage, PyTable};
use crate::pixmap::PyPixmap;
use crate::rect::{PyMatrix, PyPoint, PyRect};

/// Returns the version of the underlying Rust engine.
#[pyfunction]
fn version() -> &'static str {
    pdfrune::version()
}

/// Runs a multi-core Rust benchmark over `pages` synthesized pages and returns timing metrics.
#[pyfunction]
#[pyo3(signature = (pages=50))]
fn benchmark<'py>(py: Python<'py>, pages: usize) -> PyResult<Bound<'py, pyo3::types::PyDict>> {
    let m = py
        .allow_threads(|| pdfrune::run_benchmark(pages))
        .map_err(crate::doc::to_py_err)?;
    let dict = pyo3::types::PyDict::new(py);
    dict.set_item("pages", m.pages)?;
    dict.set_item("parse_ms", m.parse_ms)?;
    dict.set_item("extract_ms", m.extract_ms)?;
    dict.set_item("search_ms", m.search_ms)?;
    dict.set_item("markdown_ms", m.markdown_ms)?;
    dict.set_item("render_ms", m.render_ms)?;
    dict.set_item("pages_per_sec_extract", m.pages_per_sec_extract)?;
    dict.set_item("pages_per_sec_render", m.pages_per_sec_render)?;
    Ok(dict)
}

/// `_pdfrune` native extension module implemented in Rust.
#[pymodule]
fn _pdfrune(m: &Bound<'_, PyModule>) -> PyResult<()> {
    m.add("__version__", pdfrune::version())?;
    m.add_function(wrap_pyfunction!(version, m)?)?;
    m.add_function(wrap_pyfunction!(benchmark, m)?)?;
    m.add_function(wrap_pyfunction!(open_document, m)?)?;
    m.add_function(wrap_pyfunction!(merge_pdfs, m)?)?;
    m.add_function(wrap_pyfunction!(batch_extract, m)?)?;
    m.add_function(wrap_pyfunction!(batch_render, m)?)?;
    m.add_class::<PyPoint>()?;
    m.add_class::<PyRect>()?;
    m.add_class::<PyMatrix>()?;
    m.add_class::<PyPixmap>()?;
    m.add_class::<PyTable>()?;
    m.add_class::<PyPage>()?;
    m.add_class::<PyDocument>()?;
    Ok(())
}
