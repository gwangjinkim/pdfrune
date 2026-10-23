# Scanned-page raster rendering repair

The renderer now decodes DCTDecode/JPEG, JPXDecode/JPEG2000 and embedded JBIG2
images (including referenced JBIG2Globals). Document-aware image decoding applies
SMask transparency before source-over compositing, with affine image placement.
Unsupported or invalid image streams return a RenderError instead of silently
omitting the image. Raw grayscale/RGB/CMYK samples handle packed row padding and
Decode inversion. A 64-million-pixel image limit bounds decoded image buffers;
soft-mask recursion is limited to eight levels.

PDF text rendering modes 3 (invisible OCR) and 7 (clip-only) are not painted in
raster or SVG output. Text extraction still returns their text and positions.
Other clipping semantics are not newly implemented by this repair.

New codecs are pure Rust, MIT OR Apache-2.0, with codec SIMD features disabled.
They do not introduce PDFium, MuPDF, a C codec library or an external rendering
process. Rust 1.92 or newer is required by the JPEG2000/JBIG2 dependencies.
THIRD_PARTY_NOTICES.md preserves the new dependency license texts.

Regression inputs in crates/pdfrune/tests/fixtures are generated, not extracted
book pages. Tests exercise JPEG/JPX rendering, JBIG2 soft-mask polarity,
invisible text extraction/rendering, q/Q rendering-mode restoration, affine
mirroring, packed sample rows and Decode inversion, and unsupported image errors.

Real-file validation uses the user's local Archive.org books 10289757bsb
(DCTDecode scans) and makrobiotikoderd00hufeuoft (JPX background/foreground,
JBIG2 SMask, invisible OCR). Those books are not bundled with this patch.
The examples/render_scan.rs helper renders a zero-based page index to PNG.

This addresses these scanned-page failures. It is not a claim of complete PDF
rendering conformance: arbitrary color profiles, image masks, clipping, page
rotation, advanced transparency and all font types need separate coverage.
