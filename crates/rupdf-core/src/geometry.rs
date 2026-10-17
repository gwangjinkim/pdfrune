/// A 2D coordinate point `(x, y)`.
#[derive(Debug, Clone, Copy, PartialEq, Default)]
pub struct Point {
    pub x: f64,
    pub y: f64,
}

impl Point {
    #[inline]
    pub const fn new(x: f64, y: f64) -> Self {
        Self { x, y }
    }

    #[inline]
    pub fn distance_to(&self, other: Point) -> f64 {
        ((self.x - other.x).powi(2) + (self.y - other.y).powi(2)).sqrt()
    }
}

/// An axis-aligned 2D bounding rectangle `(x0, y0, x1, y1)`.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct Rect {
    pub x0: f64,
    pub y0: f64,
    pub x1: f64,
    pub y1: f64,
}

impl Default for Rect {
    fn default() -> Self {
        Self::new(0.0, 0.0, 612.0, 792.0) // Default US Letter size (8.5 x 11 inches at 72 DPI)
    }
}

impl Rect {
    #[inline]
    pub const fn new(x0: f64, y0: f64, x1: f64, y1: f64) -> Self {
        Self { x0, y0, x1, y1 }
    }

    #[inline]
    pub fn from_array(arr: [f64; 4]) -> Self {
        Self::new(arr[0], arr[1], arr[2], arr[3]).normalize()
    }

    #[inline]
    pub fn to_array(&self) -> [f64; 4] {
        [self.x0, self.y0, self.x1, self.y1]
    }

    /// Ensures `x0 <= x1` and `y0 <= y1`.
    #[inline]
    pub fn normalize(&self) -> Self {
        Self {
            x0: self.x0.min(self.x1),
            y0: self.y0.min(self.y1),
            x1: self.x0.max(self.x1),
            y1: self.y0.max(self.y1),
        }
    }

    #[inline]
    pub fn width(&self) -> f64 {
        (self.x1 - self.x0).abs()
    }

    #[inline]
    pub fn height(&self) -> f64 {
        (self.y1 - self.y0).abs()
    }

    #[inline]
    pub fn area(&self) -> f64 {
        self.width() * self.height()
    }

    #[inline]
    pub fn is_empty(&self) -> bool {
        self.width() <= 0.0 || self.height() <= 0.0
    }

    #[inline]
    pub fn contains_point(&self, p: Point) -> bool {
        let r = self.normalize();
        p.x >= r.x0 && p.x <= r.x1 && p.y >= r.y0 && p.y <= r.y1
    }

    #[inline]
    pub fn intersects(&self, other: &Rect) -> bool {
        let a = self.normalize();
        let b = other.normalize();
        a.x0 < b.x1 && a.x1 > b.x0 && a.y0 < b.y1 && a.y1 > b.y0
    }

    #[inline]
    pub fn intersect(&self, other: &Rect) -> Self {
        let a = self.normalize();
        let b = other.normalize();
        let x0 = a.x0.max(b.x0);
        let y0 = a.y0.max(b.y0);
        let x1 = a.x1.min(b.x1);
        let y1 = a.y1.min(b.y1);
        if x0 <= x1 && y0 <= y1 {
            Self::new(x0, y0, x1, y1)
        } else {
            Self::new(0.0, 0.0, 0.0, 0.0)
        }
    }

    #[inline]
    pub fn union(&self, other: &Rect) -> Self {
        let a = self.normalize();
        let b = other.normalize();
        Self::new(
            a.x0.min(b.x0),
            a.y0.min(b.y0),
            a.x1.max(b.x1),
            a.y1.max(b.y1),
        )
    }
}

/// 2D affine transformation matrix `[a, b, c, d, e, f]` per ISO 32000-1 Section 8.3.3.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct Matrix {
    pub a: f64,
    pub b: f64,
    pub c: f64,
    pub d: f64,
    pub e: f64,
    pub f: f64,
}

impl Default for Matrix {
    fn default() -> Self {
        Self::identity()
    }
}

impl Matrix {
    #[inline]
    pub const fn identity() -> Self {
        Self {
            a: 1.0,
            b: 0.0,
            c: 0.0,
            d: 1.0,
            e: 0.0,
            f: 0.0,
        }
    }

    #[inline]
    pub const fn new(a: f64, b: f64, c: f64, d: f64, e: f64, f: f64) -> Self {
        Self { a, b, c, d, e, f }
    }

    #[inline]
    pub const fn translate(tx: f64, ty: f64) -> Self {
        Self::new(1.0, 0.0, 0.0, 1.0, tx, ty)
    }

    #[inline]
    pub const fn scale(sx: f64, sy: f64) -> Self {
        Self::new(sx, 0.0, 0.0, sy, 0.0, 0.0)
    }

    /// Multiplies `self * other` (`self` premultiplied by `other`).
    #[inline]
    pub fn multiply(&self, other: &Matrix) -> Self {
        Self {
            a: self.a * other.a + self.b * other.c,
            b: self.a * other.b + self.b * other.d,
            c: self.c * other.a + self.d * other.c,
            d: self.c * other.b + self.d * other.d,
            e: self.e * other.a + self.f * other.c + other.e,
            f: self.e * other.b + self.f * other.d + other.f,
        }
    }

    #[inline]
    pub fn transform_point(&self, p: Point) -> Point {
        Point {
            x: p.x * self.a + p.y * self.c + self.e,
            y: p.x * self.b + p.y * self.d + self.f,
        }
    }
}
