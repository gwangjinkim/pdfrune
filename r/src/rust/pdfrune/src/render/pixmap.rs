use std::io::Write;
use std::path::Path;

use crate::error::{Result, PdfruneError};

/// RGBA8888 pixel buffer representing a rasterized PDF page or image.
#[derive(Debug, Clone)]
pub struct Pixmap {
    pub width: u32,
    pub height: u32,
    /// Raw RGBA8888 pixels (`width * height * 4` bytes).
    pub data: Vec<u8>,
}

impl Pixmap {
    /// Creates a new `Pixmap` filled with solid white `(255, 255, 255, 255)`.
    pub fn new_white(width: u32, height: u32) -> Self {
        let len = (width as usize) * (height as usize) * 4;
        Self {
            width,
            height,
            data: vec![255u8; len],
        }
    }

    #[inline]
    pub fn channels(&self) -> u8 {
        4
    }

    #[inline]
    pub fn stride(&self) -> usize {
        (self.width as usize) * 4
    }

    #[inline]
    pub fn samples(&self) -> &[u8] {
        &self.data
    }

    /// Returns the `(R, G, B, A)` pixel value at `(x, y)`.
    pub fn pixel(&self, x: u32, y: u32) -> Result<[u8; 4]> {
        if x >= self.width || y >= self.height {
            return Err(PdfruneError::InvalidArgument(format!(
                "Pixel coordinates ({x}, {y}) out of bounds ({}x{})",
                self.width, self.height
            )));
        }
        let idx = ((y as usize) * (self.width as usize) + (x as usize)) * 4;
        Ok([
            self.data[idx],
            self.data[idx + 1],
            self.data[idx + 2],
            self.data[idx + 3],
        ])
    }

    /// Encodes the `Pixmap` as a lossless PNG image in memory.
    pub fn to_png(&self) -> Result<Vec<u8>> {
        let mut buf = Vec::new();
        {
            let mut encoder = png::Encoder::new(&mut buf, self.width, self.height);
            encoder.set_color(png::ColorType::Rgba);
            encoder.set_depth(png::BitDepth::Eight);
            let mut writer = encoder
                .write_header()
                .map_err(|e| PdfruneError::RenderError(e.to_string()))?;
            writer
                .write_image_data(&self.data)
                .map_err(|e| PdfruneError::RenderError(e.to_string()))?;
        }
        Ok(buf)
    }

    /// Encodes the `Pixmap` as a binary PPM (P6) RGB image in memory.
    pub fn to_ppm(&self) -> Vec<u8> {
        let mut out = Vec::with_capacity(32 + (self.width as usize) * (self.height as usize) * 3);
        let _ = write!(out, "P6\n{} {}\n255\n", self.width, self.height);
        for rgba in self.data.chunks_exact(4) {
            out.extend_from_slice(&[rgba[0], rgba[1], rgba[2]]);
        }
        out
    }

    /// Saves the `Pixmap` as a PNG file at `path`.
    pub fn save_png<P: AsRef<Path>>(&self, path: P) -> Result<()> {
        let png_bytes = self.to_png()?;
        std::fs::write(path, png_bytes)?;
        Ok(())
    }
}
