# rupdf

**Ultra-Fast, Memory-Safe, Permissive PDF Engine in Rust & Python**

`rupdf` is a clean-room, high-performance PDF toolkit written in pure, memory-safe Rust (`rupdf-core`) with ergonomic Python bindings (`rupdf`). It provides full parity with PyMuPDF (`fitz`) workflows plus modern multi-core parallelism, structured Markdown/HTML export, SIMD-accelerated search, and AI/RAG semantic chunking—all under a **100% permissive MIT OR Apache-2.0 dual license** (zero AGPL restrictions).

## Highlights

- **100% Permissive License**: Dual-licensed under `MIT OR Apache-2.0`. Free for commercial, academic, and private use without copyleft constraints.
- **Memory-Safe Rust Core**: Zero segfaults, buffer overflows, or use-after-free bugs on malformed or truncated PDFs, with automatic SIMD cross-reference (`xref`) repair.
- **GIL-Free Multi-Core Parallelism**: Batch extraction, rendering, and search release the Python GIL and scale across CPU cores via `rayon`.
- **Rich Extraction & Layout Analysis**: Extract plain text, words, lines, blocks, JSON layouts, structured tables (`page.find_tables()`), and bounding boxes with CMap (`/ToUnicode`) and Adobe Glyph List decoding.
- **Rasterization & Vector Export**: Render PDF pages—including embedded `/Image` XObjects and TrueType (`/FontFile2`) glyphs—to RGBA8888 `Pixmap`, PNG, PPM, and scalable vector `SVG`.
- **Navigation, Annotations, Redactions & Forms**:
  - Hierarchical Outlines / Table of Contents (`doc.get_toc()`, `doc.set_toc()`) and Hyperlinks (`page.get_links()`, `page.insert_link()`)
  - Image inventory, extraction & insertion (`page.get_images()`, `doc.extract_image()`, `page.insert_image()`)
  - Annotations (`page.annots()`, `add_highlight_annot()`, `add_text_annot()`, `add_rect_annot()`) & true content-stream Redactions (`add_redact_annot()`, `apply_redactions()`)
  - Interactive `/AcroForm` widgets (`page.widgets()`, `page.add_widget()`, `page.set_field_value()`)
- **Document Creation, Manipulation & Encryption**: Create pages, insert text/shapes, merge, split, delete, rotate, crop, and serialize compressed or password-encrypted PDFs (`encryption="rc4-128"`, `doc.authenticate()`).
- **Beyond PyMuPDF Innovations**:
  - `page.to_markdown()` / `doc.to_markdown()` & `to_html()`
  - `doc.to_rag_chunks(max_chars=500)` with page numbers, section headings, and bounding box coordinates
  - `doc.search_all(query, regex=True)` using SIMD `aho-corasick` and `regex` engines
  - `rupdf.batch_extract([...])` & `rupdf.batch_render(doc, dpi=150)`
  - Drop-in `import rupdf.fitz as fitz` compatibility shim

## Quickstart

```python
import rupdf
from rupdf import Point, Rect

# Create or open a PDF document
with rupdf.open() as doc:
    page = doc.new_page(width=612, height=792)
    page.insert_text(Point(72, 72), "Hello from rupdf!", fontsize=24)
    page.draw_rect(Rect(72, 100, 300, 160), color=(0.1, 0.3, 0.8), width=2.0)

    # Extract text, words, blocks, or Markdown
    print(page.extract_text())
    print(page.to_markdown())

    # Search with bounding boxes
    hits = page.search("rupdf")

    # Render to RGBA Pixmap or SVG
    pix = page.render(dpi=150)
    png_bytes = pix.to_png()
    svg_xml = page.to_svg()

    # AI / RAG semantic chunks with bounding boxes
    chunks = doc.to_rag_chunks(max_chars=400)

    # Save with Flate compression
    doc.save("output.pdf", deflate=True)
```

### Drop-in PyMuPDF (`fitz`) Compatibility

```python
import rupdf.fitz as fitz

with fitz.open("output.pdf") as doc:
    page = doc.load_page(0)
    text = page.get_text("text")
    words = page.get_text("words")
    pix = page.get_pixmap(dpi=150)
```

## Development & Verification

Managed with `uv` and Cargo:

```bash
cargo fmt --check
cargo test
cargo clippy --all-targets -- -D warnings
uv run pytest
```

## License

Licensed under either of:

- Apache License, Version 2.0 ([LICENSE-APACHE](LICENSE-APACHE))
- MIT License ([LICENSE-MIT](LICENSE-MIT))

at your option.
