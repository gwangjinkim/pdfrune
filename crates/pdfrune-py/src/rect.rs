use pyo3::exceptions::PyIndexError;
use pyo3::prelude::*;
use pyo3::types::PyTuple;

use pdfrune::{Matrix, Point, Rect};

/// Python wrapper for a 2D `Point(x, y)`.
#[pyclass(name = "Point", module = "pdfrune")]
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct PyPoint {
    pub inner: Point,
}

#[pymethods]
impl PyPoint {
    #[new]
    #[pyo3(signature = (x=0.0, y=0.0))]
    pub fn new(x: f64, y: f64) -> Self {
        Self {
            inner: Point::new(x, y),
        }
    }

    #[getter]
    pub fn x(&self) -> f64 {
        self.inner.x
    }

    #[setter]
    pub fn set_x(&mut self, val: f64) {
        self.inner.x = val;
    }

    #[getter]
    pub fn y(&self) -> f64 {
        self.inner.y
    }

    #[setter]
    pub fn set_y(&mut self, val: f64) {
        self.inner.y = val;
    }

    pub fn distance_to(&self, other: &PyPoint) -> f64 {
        self.inner.distance_to(other.inner)
    }

    fn __len__(&self) -> usize {
        2
    }

    fn __getitem__(&self, idx: isize) -> PyResult<f64> {
        match idx {
            0 | -2 => Ok(self.inner.x),
            1 | -1 => Ok(self.inner.y),
            _ => Err(PyIndexError::new_err("Point index out of range")),
        }
    }

    fn __eq__(&self, other: &PyPoint) -> bool {
        self.inner == other.inner
    }

    fn __repr__(&self) -> String {
        format!("Point({}, {})", self.inner.x, self.inner.y)
    }
}

/// Python wrapper for an axis-aligned 2D `Rect(x0, y0, x1, y1)`.
#[pyclass(name = "Rect", module = "pdfrune")]
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct PyRect {
    pub inner: Rect,
}

#[pymethods]
impl PyRect {
    #[new]
    #[pyo3(signature = (x0=0.0, y0=0.0, x1=0.0, y1=0.0))]
    pub fn new(x0: f64, y0: f64, x1: f64, y1: f64) -> Self {
        Self {
            inner: Rect::new(x0, y0, x1, y1),
        }
    }

    #[getter]
    pub fn x0(&self) -> f64 {
        self.inner.x0
    }

    #[setter]
    pub fn set_x0(&mut self, val: f64) {
        self.inner.x0 = val;
    }

    #[getter]
    pub fn y0(&self) -> f64 {
        self.inner.y0
    }

    #[setter]
    pub fn set_y0(&mut self, val: f64) {
        self.inner.y0 = val;
    }

    #[getter]
    pub fn x1(&self) -> f64 {
        self.inner.x1
    }

    #[setter]
    pub fn set_x1(&mut self, val: f64) {
        self.inner.x1 = val;
    }

    #[getter]
    pub fn y1(&self) -> f64 {
        self.inner.y1
    }

    #[setter]
    pub fn set_y1(&mut self, val: f64) {
        self.inner.y1 = val;
    }

    #[getter]
    pub fn width(&self) -> f64 {
        self.inner.width()
    }

    #[getter]
    pub fn height(&self) -> f64 {
        self.inner.height()
    }

    #[getter]
    pub fn area(&self) -> f64 {
        self.inner.area()
    }

    #[getter]
    pub fn is_empty(&self) -> bool {
        self.inner.is_empty()
    }

    pub fn normalize(&self) -> PyRect {
        PyRect {
            inner: self.inner.normalize(),
        }
    }

    pub fn contains(&self, pt: &PyPoint) -> bool {
        self.inner.contains_point(pt.inner)
    }

    pub fn intersects(&self, other: &PyRect) -> bool {
        self.inner.intersects(&other.inner)
    }

    pub fn intersect(&self, other: &PyRect) -> PyRect {
        PyRect {
            inner: self.inner.intersect(&other.inner),
        }
    }

    pub fn union(&self, other: &PyRect) -> PyRect {
        PyRect {
            inner: self.inner.union(&other.inner),
        }
    }

    pub fn to_tuple<'py>(&self, py: Python<'py>) -> Bound<'py, PyTuple> {
        PyTuple::new(
            py,
            [self.inner.x0, self.inner.y0, self.inner.x1, self.inner.y1],
        )
        .unwrap()
    }

    fn __len__(&self) -> usize {
        4
    }

    fn __getitem__(&self, idx: isize) -> PyResult<f64> {
        match idx {
            0 | -4 => Ok(self.inner.x0),
            1 | -3 => Ok(self.inner.y0),
            2 | -2 => Ok(self.inner.x1),
            3 | -1 => Ok(self.inner.y1),
            _ => Err(PyIndexError::new_err("Rect index out of range")),
        }
    }

    fn __eq__(&self, other: &PyRect) -> bool {
        self.inner == other.inner
    }

    fn __repr__(&self) -> String {
        format!(
            "Rect({}, {}, {}, {})",
            self.inner.x0, self.inner.y0, self.inner.x1, self.inner.y1
        )
    }
}

/// Python wrapper for a 2D affine `Matrix(a, b, c, d, e, f)`.
#[pyclass(name = "Matrix", module = "pdfrune")]
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct PyMatrix {
    pub inner: Matrix,
}

#[pymethods]
impl PyMatrix {
    #[new]
    #[pyo3(signature = (a=1.0, b=0.0, c=0.0, d=1.0, e=0.0, f=0.0))]
    pub fn new(a: f64, b: f64, c: f64, d: f64, e: f64, f: f64) -> Self {
        Self {
            inner: Matrix::new(a, b, c, d, e, f),
        }
    }

    #[getter]
    pub fn a(&self) -> f64 {
        self.inner.a
    }

    #[getter]
    pub fn b(&self) -> f64 {
        self.inner.b
    }

    #[getter]
    pub fn c(&self) -> f64 {
        self.inner.c
    }

    #[getter]
    pub fn d(&self) -> f64 {
        self.inner.d
    }

    #[getter]
    pub fn e(&self) -> f64 {
        self.inner.e
    }

    #[getter]
    pub fn f(&self) -> f64 {
        self.inner.f
    }

    pub fn multiply(&self, other: &PyMatrix) -> PyMatrix {
        PyMatrix {
            inner: self.inner.multiply(&other.inner),
        }
    }

    pub fn transform_point(&self, pt: &PyPoint) -> PyPoint {
        PyPoint {
            inner: self.inner.transform_point(pt.inner),
        }
    }

    fn __eq__(&self, other: &PyMatrix) -> bool {
        self.inner == other.inner
    }

    fn __repr__(&self) -> String {
        format!(
            "Matrix({}, {}, {}, {}, {}, {})",
            self.inner.a, self.inner.b, self.inner.c, self.inner.d, self.inner.e, self.inner.f
        )
    }
}
