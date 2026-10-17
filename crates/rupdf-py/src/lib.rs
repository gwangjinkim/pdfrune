pub mod doc;
pub mod page;
pub mod rect;

use pyo3::prelude::*;

use crate::doc::{merge_pdfs, open_document, PyDocument};
use crate::page::PyPage;
use crate::rect::{PyMatrix, PyPoint, PyRect};

/// Returns the version of the underlying Rust engine.
#[pyfunction]
fn version() -> &'static str {
    rupdf_core::version()
}

/// `_rupdf` native extension module implemented in Rust.
#[pymodule]
fn _rupdf(m: &Bound<'_, PyModule>) -> PyResult<()> {
    m.add("__version__", rupdf_core::version())?;
    m.add_function(wrap_pyfunction!(version, m)?)?;
    m.add_function(wrap_pyfunction!(open_document, m)?)?;
    m.add_function(wrap_pyfunction!(merge_pdfs, m)?)?;
    m.add_class::<PyPoint>()?;
    m.add_class::<PyRect>()?;
    m.add_class::<PyMatrix>()?;
    m.add_class::<PyPage>()?;
    m.add_class::<PyDocument>()?;
    Ok(())
}
