use std::path::PathBuf;

use pyo3::exceptions::PyValueError;
use pyo3::prelude::*;
use pyo3::types::PyBytes;

use pdfrune::Pixmap;

use crate::doc::to_py_err;

/// Python wrapper for an RGBA8888 `Pixmap`.
#[pyclass(name = "Pixmap", module = "pdfrune")]
#[derive(Clone)]
pub struct PyPixmap {
    pub inner: Pixmap,
}

#[pymethods]
impl PyPixmap {
    /// Width in pixels.
    #[getter]
    pub fn width(&self) -> u32 {
        self.inner.width
    }

    /// Height in pixels.
    #[getter]
    pub fn height(&self) -> u32 {
        self.inner.height
    }

    /// Number of color components per pixel (4 for RGBA).
    #[getter]
    pub fn n(&self) -> u8 {
        self.inner.channels()
    }

    /// Row stride in bytes (`width * 4`).
    #[getter]
    pub fn stride(&self) -> usize {
        self.inner.stride()
    }

    /// Raw RGBA8888 pixel buffer as Python `bytes`.
    #[getter]
    pub fn samples<'py>(&self, py: Python<'py>) -> Bound<'py, PyBytes> {
        PyBytes::new(py, &self.inner.data)
    }

    /// Returns `(r, g, b, a)` at pixel coordinate `(x, y)`.
    pub fn pixel(&self, x: u32, y: u32) -> PyResult<(u8, u8, u8, u8)> {
        let [r, g, b, a] = self.inner.pixel(x, y).map_err(to_py_err)?;
        Ok((r, g, b, a))
    }

    /// Encodes the pixmap as PNG bytes in memory.
    pub fn to_png<'py>(&self, py: Python<'py>) -> PyResult<Bound<'py, PyBytes>> {
        let bytes = py
            .allow_threads(|| self.inner.to_png())
            .map_err(to_py_err)?;
        Ok(PyBytes::new(py, &bytes))
    }

    /// Encodes the pixmap into the requested image format (`"png"` or `"ppm"`).
    #[pyo3(signature = (output="png"))]
    pub fn tobytes<'py>(&self, py: Python<'py>, output: &str) -> PyResult<Bound<'py, PyBytes>> {
        match output.to_ascii_lowercase().as_str() {
            "png" => self.to_png(py),
            "ppm" | "pnm" => {
                let bytes = py.allow_threads(|| self.inner.to_ppm());
                Ok(PyBytes::new(py, &bytes))
            }
            "raw" | "rgba" => Ok(self.samples(py)),
            other => Err(PyValueError::new_err(format!(
                "Unsupported pixmap output format: {other}"
            ))),
        }
    }

    /// Alias for `tobytes(output)`.
    #[pyo3(signature = (output="png"))]
    pub fn to_bytes<'py>(&self, py: Python<'py>, output: &str) -> PyResult<Bound<'py, PyBytes>> {
        self.tobytes(py, output)
    }

    /// Saves the pixmap as a PNG file at `filename`.
    pub fn save_png(&self, py: Python<'_>, filename: PathBuf) -> PyResult<()> {
        py.allow_threads(|| self.inner.save_png(&filename).map_err(to_py_err))
    }

    /// PyMuPDF-compatible `save(filename)`.
    pub fn save(&self, py: Python<'_>, filename: PathBuf) -> PyResult<()> {
        self.save_png(py, filename)
    }

    fn __repr__(&self) -> String {
        format!(
            "Pixmap(width={}, height={}, n=4)",
            self.inner.width, self.inner.height
        )
    }
}
