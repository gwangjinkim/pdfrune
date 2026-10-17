use pyo3::prelude::*;

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
    Ok(())
}
