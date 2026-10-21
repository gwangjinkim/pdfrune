# Engine Architecture & Clean-Room Design

`pdfrune` is architected as a two-crate Cargo workspace:
- **`crates/pdfrune`**: Pure Rust PDF engine implementing ISO 32000-1 / ISO 32000-2 parsing, layout, rendering, mutation, and cryptography.
- **`crates/pdfrune-py`**: Thin PyO3 bindings exposing `pdfrune` to Python with GIL-released concurrency and zero-copy buffer sharing.

---

## 1. Clean-Room & Permissive Dependency Stack

Every dependency in `pdfrune` is strictly vetted for permissive licensing (`MIT`, `Apache-2.0`, or `BSD`) and high performance:
- **`memchr` & `aho-corasick`**: SIMD-accelerated byte and multi-pattern scanning for fast PDF tokenizing, `xref` repair, and full-document text search.
- **`flate2` (`miniz_oxide`)**: Fast pure-Rust `/FlateDecode` zlib/deflate compression and decompression.
- **`tiny-skia`**: Memory-safe 2D RGBA rasterizer for vector paths, strokes, fills, and clipping.
- **`ab_glyph`**: High-speed TrueType/OpenType (`/FontFile2`) glyph outline parser and rasterizer.
- **`png` & `jpeg-decoder`**: Pure-Rust decoding and encoding for embedded `/Image` XObjects and `Pixmap` exports.
- **`rayon`**: Work-stealing thread pool powering multi-page rendering, Markdown/HTML export, cross-page search, and batch document processing.
- **`pyo3`**: Ergonomic Rust-to-Python bindings with buffer protocol support.

---

## 2. Subsystem Overview

### Parser & Automatic XREF Repair (`crates/pdfrune/src/parser/`)
- **`lexer.rs`**: Zero-allocation byte slice tokenizer for PDF primitives (`Boolean`, `Integer`, `Real`, literal/hex `String`, `Name`, `Array`, `Dict`, `Stream`) and content stream operators.
- **`xref.rs`**: Parses both traditional ASCII cross-reference tables and PDF 1.5+ compressed `/Type /XRef` streams (including PNG predictor filters). If a PDF has a corrupted or truncated `startxref` offset, `scan_repair_xref` performs a full SIMD scan for `obj` headers and reconstructs the object graph automatically.
- **`filter.rs`**: Decodes `/FlateDecode` (with fallback raw-deflate recovery), `/ASCIIHexDecode`, `/ASCII85Decode`, `/RunLengthDecode`, and PNG Up/Sub/Average/Paeth predictors.

### Text & Layout Engine (`crates/pdfrune/src/text/`)
- **`cmap.rs`**: Parses embedded `/ToUnicode` CMap streams (`beginbfchar`, `beginbfrange`), `/Differences` encoding arrays, Adobe Glyph List names, and WinAnsi/MacRoman encodings.
- **`extractor.rs`**: Interprets PDF graphics state (`q`, `Q`, `cm`) and text state (`BT`, `ET`, `Tf`, `Tm`, `Td`, `TD`, `T*`, `TL`, `Tc`, `Tw`, `Tz`, `Tj`, `TJ`, `'`, `"`) to compute exact top-down page coordinates for every glyph and span.
- **`layout.rs`**: Clusters spans into words, visual lines, and reading-order paragraphs/blocks.

### Rendering & Embedded Fonts/Images (`crates/pdfrune/src/render/`)
- **`engine.rs`**: Executes page content streams onto a `tiny-skia` `Pixmap` or vector `SVG` canvas. When a font dictionary references an embedded `/FontFile2` TrueType stream, `ab_glyph` outlines and rasterizes the exact glyph shapes; otherwise a clean vector fallback stroke font ensures readable text on every platform.
- **`image.rs`**: Inventories, extracts, inserts, and composites `/Subtype /Image` XObjects (`DeviceRGB`, `DeviceGray`, `DeviceCMYK`, `/DCTDecode` JPEG, and `/SMask` soft-mask alpha channels).

### Navigation, Annotations, True Redactions & AcroForms (`nav.rs`, `annot.rs`)
- **`nav.rs`**: Traverses and constructs `/Outlines` doubly-linked bookmark trees and page `/Link` annotations (`/URI` and `/GoTo`).
- **`annot.rs`**: Reads and mutates `/Annots` arrays and `/AcroForm` `/Widget` fields (including generating `/AP /N` appearance streams so updated form values render visually). `apply_page_redactions` parses the page's content stream, strips any text operator whose bounding box intersects a `/Redact` rectangle, and paints the redaction fill and overlay label.

### Table Detection & Standard Security Handler (`advanced/table.rs`, `crypto.rs`)
- **`table.rs`**: Groups positioned spans into visual rows, detects multi-column horizontal alignments, and exports structured tables to 2D arrays, dictionaries, or GitHub-Flavored Markdown.
- **`crypto.rs`**: Clean-room implementation of RFC 1321 MD5, RC4 stream cipher, and ISO 32000-1 Standard Security Handler Algorithms 1–7 (`V=1/2, R=2/3`, 40-to-128-bit keys) for authenticating, decrypting, and encrypting PDF objects.
