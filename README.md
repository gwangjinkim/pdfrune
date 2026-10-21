# pdfrune

**Ultra-Fast, Memory-Safe, Permissive PDF Engine in Rust, Python, R & Common Lisp**

[![License: MIT OR Apache-2.0](https://img.shields.io/badge/License-MIT%20OR%20Apache--2.0-blue.svg)](#license)
[![Rust](https://img.shields.io/badge/Rust-2021%20Edition-orange.svg)](crates/pdfrune)
[![Python](https://img.shields.io/badge/Python-3.10%2B-3776AB.svg)](python/pdfrune)
[![CRAN](https://img.shields.io/badge/CRAN-pdfrune-276DC3.svg)](r/)
[![Quicklisp](https://img.shields.io/badge/Quicklisp-cl-pdfrune-darkgreen.svg)](lisp/)
[![Ultralisp](https://img.shields.io/badge/Ultralisp-cl-pdfrune-blueviolet.svg)](lisp/)

`pdfrune` is a clean-room, high-performance PDF engine written in 100% memory-safe Rust ([`pdfrune`](crates/pdfrune)) with zero-overhead Python bindings ([`pdfrune`](python/pdfrune)), CRAN-compliant R bindings ([`r/`](r/)), and ANSI-standard Common Lisp bindings ([`lisp/`](lisp/)) designed in the classic style of Edi Weitz. It provides full functional parity with PyMuPDF (`fitz`) workflows—plus native multi-core parallelism, structured Markdown/HTML export, table detection, SIMD-accelerated search, and AI/RAG semantic chunking—under a **100% permissive `MIT OR Apache-2.0` dual license** with zero AGPL restrictions.

---

## Table of Contents

- [Why `pdfrune`?](#why-pdfrune)
- [Feature Comparison Matrix](#feature-comparison-matrix)
- [Installation & Building](#installation--building)
- [Python Quickstart & Examples](#python-quickstart--examples)
  - [1. Opening, Creating & Inspecting PDFs](#1-opening-creating--inspecting-pdfs)
  - [2. Text, Word, Block & Table Extraction](#2-text-word-block--table-extraction)
  - [3. Rendering to Pixmap (PNG, NumPy, PIL) & SVG](#3-rendering-to-pixmap-png-numpy-pil--svg)
  - [4. Table of Contents (TOC) & Hyperlinks](#4-table-of-contents-toc--hyperlinks)
  - [5. Image Inventory, Extraction & Insertion](#5-image-inventory-extraction--insertion)
  - [6. Annotations, True Redactions & Interactive AcroForms](#6-annotations-true-redactions--interactive-acroforms)
  - [7. Merging, Splitting & Password Encryption (RC4-128)](#7-merging-splitting--password-encryption-rc4-128)
  - [8. AI / RAG Chunker, Markdown/HTML & Multi-Core Batch Processing](#8-ai--rag-chunker-markdownhtml--multi-core-batch-processing)
  - [9. Drop-In PyMuPDF (`fitz`) Compatibility](#9-drop-in-pymupdf-fitz-compatibility)
- [R Package Quickstart & CRAN (`uvr`)](#r-package-quickstart--cran-uvr)
- [Common Lisp Package Quickstart (Quicklisp & Ultralisp)](#common-lisp-package-quickstart-quicklisp--ultralisp)
- [Using `pdfrune` Directly in Rust](#using-pdfrune-directly-in-rust)
- [Documentation](#documentation)
- [Development & Testing](#development--testing)
- [Author & License](#author--license)

---

## Why `pdfrune`?

1. **100% Permissive Licensing (`MIT OR Apache-2.0`)**:
   Unlike MuPDF and PyMuPDF (which are governed by the copyleft AGPL-3.0 license or require commercial licensing), `pdfrune` is built from the ground up against the open **ISO 32000-1 / 32000-2** PDF specifications using only permissively licensed Rust crates. It is completely free for commercial, enterprise, academic, and private software.
2. **Memory Safety & Corrupt-PDF Resilience**:
   Written in safe Rust with zero C/C++ dependencies—eliminating buffer overflows, use-after-free vulnerabilities, and segfaults on malformed inputs. Includes SIMD-accelerated automatic cross-reference (`xref`) table repair when opening truncated or damaged PDFs.
3. **Fearless Multi-Core Parallelism (GIL-Free)**:
   All CPU-intensive operations (parsing, rendering, text layout, regex/SIMD search, Markdown generation, and batch processing) release the Python GIL and scale across all CPU cores via `rayon`.
4. **First-Class Rust & Python APIs**:
   Every capability is implemented in the pure Rust crate `pdfrune` and exposed to Python via PyO3 with zero-copy buffer protocol support (`memoryview`, NumPy, Pillow) and complete static type stubs ([`__init__.pyi`](python/pdfrune/__init__.pyi)).

---

## Feature Comparison Matrix

| Capability | PyMuPDF (`fitz`) | `pdfrune` (Python & Rust) |
| :--- | :--- | :--- |
| **License** | AGPL-3.0 / Commercial | **MIT OR Apache-2.0** (100% Free) |
| **Memory Safety** | C / C++ | **100% Safe Rust** |
| **Open Path / Bytes / Stream** | `fitz.open(...)` | `pdfrune.open(...)` |
| **Metadata Read / Write** | `doc.metadata`, `doc.set_metadata()` | `doc.metadata`, `doc.set_metadata()` |
| **Page Geometry & Rotation** | `mediabox`, `cropbox`, `rotation` | `mediabox`, `cropbox`, `bleedbox`, `trimbox`, `artbox`, `rotation` |
| **Text / Words / Blocks / JSON** | `page.get_text("text" \| "words" \| "blocks" \| "json")` | `extract_text()`, `extract_words()`, `extract_blocks()`, `get_text(...)` |
| **Table Detection** | `page.find_tables()` | `page.find_tables()` (`.extract()`, `.to_markdown()`, `.to_dicts()`, `.bbox`) |
| **2D Rasterization & Fonts** | `page.get_pixmap(dpi=150)` | `page.render(dpi=150)` / `get_pixmap(dpi=150)` (TrueType `/FontFile2` + `/Image` XObjects) |
| **Vector SVG Export** | `page.get_svg_image()` | `page.to_svg()` / `page.get_svg_image()` |
| **Outlines / Bookmarks (TOC)** | `doc.get_toc()`, `doc.set_toc()` | `doc.get_toc()`, `doc.set_toc()` |
| **Hyperlinks (`/URI` & `/GoTo`)** | `page.get_links()`, `page.insert_link()` | `page.get_links()`, `page.insert_link()`, `page.delete_link()` |
| **Images (Extract & Insert)** | `page.get_images()`, `doc.extract_image()`, `page.insert_image()` | `page.get_images()`, `doc.extract_image()`, `page.insert_image()` |
| **Annotations** | `page.annots()`, `add_*_annot()` | `page.annots()`, `add_highlight_annot()`, `add_text_annot()`, `add_rect_annot()`, `delete_annot()` |
| **True Content Redactions** | `add_redact_annot()`, `apply_redactions()` | `add_redact_annot()`, `apply_redactions()` (permanently scrubs underlying stream text) |
| **Interactive Forms (`AcroForm`)** | `page.widgets()`, `add_widget()` | `page.widgets()`, `page.add_widget()`, `page.set_field_value()` |
| **Merge, Split & Page Ops** | `insert_pdf()`, `select()`, `delete_page()` | `pdfrune.merge()`, `insert_pdf()`, `extract_pages()`, `select()`, `delete_page()` |
| **Standard PDF Encryption** | `authenticate()`, `save(encryption=...)` | `doc.authenticate(pw)`, `doc.save(..., encryption="rc4-128", user_pw=..., owner_pw=...)` |
| **Direct Markdown & HTML** | External addon (`pymupdf4llm`) | **Built-in in Rust**: `page.to_markdown()`, `doc.to_markdown()`, `doc.to_html()` |
| **AI / RAG Semantic Chunker** | External | **Built-in in Rust**: `doc.to_rag_chunks(max_chars=500)` with bbox & heading grounding |
| **SIMD / Regex Document Search** | Single-threaded per page | **Parallel SIMD (`aho-corasick` + `regex`)**: `doc.search_all(query, regex=True)` |
| **GIL-Free Batch Processing** | Manual multiprocessing | **Built-in Rust Worker Pool**: `pdfrune.batch_extract()`, `pdfrune.batch_render()` |

---

## Installation & Building

### Prerequisites
- **Rust** (1.75+ toolchain via `rustup`)
- **Python** (3.10+) managed with [`uv`](https://docs.astral.sh/uv/)

### Build & Install in Python Environment

```bash
# Sync dependencies and build the Rust extension module in development/release mode
uv sync
uv run pytest
```

### Use as a Rust Crate

Add `pdfrune` to your `Cargo.toml`:

```toml
[dependencies]
pdfrune = { path = "crates/pdfrune" }
```

---

## Python Quickstart & Examples

```bash
pip install pdfrune
```

### 1. Opening, Creating & Inspecting PDFs

```python
import pdfrune
from pdfrune import Point, Rect

# Create a new PDF document in memory
with pdfrune.open() as doc:
    page = doc.new_page(width=612.0, height=792.0)
    page.insert_text(Point(72.0, 72.0), "Hello from pdfrune!", fontsize=24.0, color=(0.1, 0.2, 0.6))
    page.draw_rect(Rect(72.0, 90.0, 320.0, 140.0), color=(0.1, 0.2, 0.6), fill=(0.9, 0.95, 1.0), width=1.5)

    doc.set_metadata({"title": "Sample Report", "author": "pdfrune Engine"})
    doc.save("report.pdf", deflate=True)

# Open from file path, bytes, or file-like BytesIO stream
with pdfrune.open("report.pdf") as doc:
    print("Pages:", len(doc), "Metadata:", doc.metadata)
    page = doc[0]
    print("MediaBox:", page.mediabox, "Rotation:", page.rotation)
```

### 2. Text, Word, Block & Table Extraction

```python
with pdfrune.open("report.pdf") as doc:
    page = doc[0]

    # Plain text
    text = page.extract_text()

    # Word tuples: (x0, y0, x1, y1, word, block_no, line_no, word_no)
    words = page.extract_words()

    # Block tuples: (x0, y0, x1, y1, text, block_no, block_type)
    blocks = page.extract_blocks()

    # Structured JSON layout (bounding boxes, font names, font sizes)
    layout_json = page.get_text("json")

    # Automatic Table Detection
    for table in page.find_tables():
        print(f"Detected {table.row_count}x{table.col_count} table at {table.bbox}")
        print("Headers:", table.header)
        print("2D Grid:", table.extract())
        print("Records:", table.to_dicts())
        print("Markdown:\n", table.to_markdown())
```

### 3. Rendering to Pixmap (PNG, NumPy, PIL) & SVG

`page.render(dpi=...)` rasterizes vector paths, text (including embedded `/FontFile2` TrueType outlines via `ab_glyph`), and embedded `/Image` XObjects into an RGBA8888 `Pixmap` that implements Python's zero-copy buffer protocol:

```python
import numpy as np

with pdfrune.open("report.pdf") as doc:
    page = doc[0]

    # Rasterize at 150 DPI
    pix = page.render(dpi=150.0)
    pix.save("page0.png")

    # Zero-copy NumPy view (H x W x 4 RGBA uint8)
    arr = np.frombuffer(pix, dtype=np.uint8).reshape(pix.height, pix.width, pix.n)

    # Vector SVG export
    svg_str = page.to_svg()
```

### 4. Table of Contents (TOC) & Hyperlinks

```python
with pdfrune.open("report.pdf") as doc:
    doc.new_page()

    # Hierarchical bookmarks: [level, title, 1-based page number]
    doc.set_toc([
        [1, "Chapter 1: Overview", 1],
        [2, "Section 1.1: Architecture", 2],
    ])
    print(doc.get_toc())

    # Insert external URI link and internal page navigation link
    doc[0].insert_link({"rect": Rect(72, 200, 250, 220), "uri": "https://example.com"})
    doc[0].insert_link({"rect": Rect(72, 230, 250, 250), "page": 1})
    print(doc[0].get_links())
```

### 5. Image Inventory, Extraction & Insertion

```python
with pdfrune.open() as doc:
    page = doc.new_page()

    # Insert PNG/JPEG bytes, file, or Pixmap onto a page rectangle
    xref = page.insert_image(Rect(72, 100, 272, 300), filename="logo.png")

    # List all images on the page: (xref, smask, width, height, bpc, colorspace, alt, name, filter)
    for img_info in page.get_images():
        img_xref = img_info[0]
        extracted = doc.extract_image(img_xref)
        print(extracted["width"], extracted["height"], extracted["ext"], len(extracted["image"]))
```

### 6. Annotations, True Redactions & Interactive AcroForms

```python
with pdfrune.open() as doc:
    page = doc.new_page()
    page.insert_text(Point(72, 100), "Public Summary", fontsize=14)
    page.insert_text(Point(72, 140), "SSN: 999-00-1234", fontsize=14)

    # Annotations
    annot = page.add_highlight_annot(Rect(70, 85, 220, 108))
    note = page.add_text_annot(Point(250, 90), "Reviewed and approved")

    # True Redaction: permanently strips "SSN: 999-00-1234" from the PDF content stream
    page.add_redact_annot(Rect(68, 122, 260, 148), text="[REDACTED]", fill=(0.0, 0.0, 0.0))
    page.apply_redactions()
    assert "999-00-1234" not in page.extract_text()

    # Interactive AcroForm Widgets
    page.add_widget({
        "field_name": "customer_name",
        "field_type": "text",
        "rect": Rect(72, 200, 280, 228),
        "field_value": "Ada Lovelace",
    })
    page.set_field_value("customer_name", "Grace Hopper")
    print(page.widgets())
```

### 7. Merging, Splitting & Password Encryption (RC4-128)

```python
# Encrypt with ISO 32000 Standard Security Handler (128-bit RC4)
with pdfrune.open() as doc:
    page = doc.new_page()
    page.insert_text(Point(72, 100), "Confidential Acquisition Plan", fontsize=16)
    encrypted_pdf = doc.to_bytes(
        deflate=True,
        encryption="rc4-128",
        user_pw="reader_secret",
        owner_pw="owner_secret",
    )

# Open & authenticate encrypted PDF
with pdfrune.open(encrypted_pdf) as enc_doc:
    assert enc_doc.is_encrypted and enc_doc.needs_pass
    assert enc_doc.authenticate("reader_secret") is True
    print(enc_doc[0].extract_text())
```

### 8. AI / RAG Chunker, Markdown/HTML & Multi-Core Batch Processing

```python
# Export full document to structured Markdown or HTML (parallelized via rayon)
with pdfrune.open("report.pdf") as doc:
    md_text = doc.to_markdown()
    html_text = doc.to_html()

    # Semantic chunks with page_number, bbox (x0, y0, x1, y1), and section_heading for LLM/RAG grounding
    chunks = doc.to_rag_chunks(max_chars=500)

    # Parallel SIMD substring or regex search across all pages
    matches = doc.search_all(r"Section \d+\.\d+", regex=True)

# GIL-free multi-document batch extraction and multi-page rendering
texts = pdfrune.batch_extract(["report.pdf", "other.pdf"])
pixmaps = pdfrune.batch_render("report.pdf", dpi=150.0)
```

### 9. Drop-In PyMuPDF (`fitz`) Compatibility

Existing PyMuPDF codebases can migrate with a single import change:

```python
import pdfrune.fitz as fitz

with fitz.open("report.pdf") as doc:
    page = doc.load_page(0)
    text = page.get_text("text")
    words = page.get_text("words")
    rects = page.search_for("Hello")
    pix = page.get_pixmap(dpi=150)
```

---

## R Package Quickstart & CRAN (`uvr`)

The R package for `pdfrune` provides full access to the high-performance Rust PDF engine with idiomatic `pdf_*` functions and complete `roxygen2` documentation:

### Installation via `uvr`

Manage dependencies using [`uvr`](https://github.com/astral-sh/uvr):

```bash
uvr add pdfrune
uvr sync
```

### R Code Examples

```r
library(pdfrune)

# 1. Open and extract plain text or layout words
doc <- pdf_open("report.pdf")
cat("Page count:", pdf_page_count(doc), "\n")
text <- pdf_extract_text(doc, page = 1L)
words <- pdf_extract_words(doc, page = 1L)

# 2. Detect tabular data and convert directly to data.frame
tables <- pdf_find_tables(doc, page = 1L)
if (length(tables) > 0) {
  df <- tables[[1]]$to_dataframe()
  print(head(df))
}

# 3. Rasterize page to RGBA8888 Pixmap & save PNG
pix <- pdf_render(doc, page = 1L, dpi = 150.0)
pix$save("page1.png")

# 4. Outlines (Table of Contents) and Hyperlinks
toc <- pdf_get_toc(doc)
links <- pdf_get_links(doc, page = 1L)

# 5. True Redaction & ISO 32000 Encryption
pdf_add_redact_annot(doc, page = 1L, rect = c(72, 100, 250, 130))
pdf_apply_redactions(doc, page = 1L)
pdf_save(doc, "redacted.pdf", encryption = "rc4-128", user_pw = "secret")
```

---

## Common Lisp Package Quickstart (Quicklisp & Ultralisp)

The Common Lisp package (`:pdfrune` / `:cl-pdfrune`) follows the classic design and documentation style of **Edi Weitz** (`cl-ppcre`, `cl-fad`, `drakma`). It is 100% ANSI-standard Common Lisp and portable across all major implementations (SBCL, CCL, ECL, ABCL, CLISP, Allegro, LispWorks).

### Loading with Quicklisp / Ultralisp

```lisp
(ql:quickload :cl-pdfrune)
```

### Common Lisp Code Examples

```lisp
(in-package :pdfrune)

;; 1. Open and extract plain text or layout words
(with-open-pdf (doc #p"report.pdf")
  (format t "Document has ~D pages.~%" (page-count doc))
  (format t "Page 1 Text:~%~A~%" (extract-text doc 0))
  
  ;; Search across entire document
  (dolist (match (search-document doc "Common Lisp"))
    (format t "Found on page ~D at ~A~%" (getf match :page) (getf match :bbox))))

;; 2. Automatic table detection & Markdown conversion
(with-open-pdf (doc #p"report.pdf")
  (let ((tables (find-tables doc 0)))
    (dolist (tab tables)
      (format t "Found table (~Dx~D):~%" (table-row-count tab) (table-col-count tab))
      (format t "~A~%~%" (table-markdown tab)))))

;; 3. Rasterize page to 300 DPI RGBA Pixmap and save PNG
(with-open-pdf (doc #p"report.pdf")
  (let ((pix (render-page doc 0 :dpi 300.0)))
    (save-pixmap pix #p"page1.png")))

;; 4. Create new PDF, draw vector graphics & TrueType typography
(with-open-pdf (doc (new-pdf))
  (new-page doc :width 612.0 :height 792.0)
  (draw-rect doc 0 '(50.0 700.0 562.0 750.0)
             :fill-color '(0.1 0.2 0.5)
             :stroke-color '(0.0 0.0 0.0)
             :stroke-width 1.5)
  (insert-text doc 0 '(72.0 720.0) "Hello from Common Lisp!"
               :font-size 18.0
               :color '(1.0 1.0 1.0))
  (save-document doc #p"hello_lisp.pdf" :deflate t))

;; 5. Irreversible Redaction & ISO 32000 Encryption (RC4-128)
(with-open-pdf (doc #p"secret.pdf")
  (add-redaction doc 0 '(72.0 100.0 250.0 130.0))
  (apply-redactions doc)
  (save-document doc #p"redacted.pdf"
                 :deflate t
                 :encryption "rc4-128"
                 :user-password "lisp_secret"))
```

---

## Using `pdfrune` Directly in Rust

`pdfrune` is a standalone, zero-Python Rust library suitable for CLI tools, backend microservices, WebAssembly targets, or native desktop apps:

```rust
use pdfrune::{Document, Point, Rect};

fn main() -> pdfrune::Result<()> {
    // 1. Create and populate a PDF document
    let mut doc = Document::new();
    doc.new_page(None, 612.0, 792.0)?;
    doc.insert_text(0, Point::new(72.0, 72.0), "Hello from Rust!", 18.0, [0.0, 0.0, 0.0])?;
    doc.draw_rect(
        0,
        Rect::new(72.0, 90.0, 300.0, 140.0),
        Some([0.1, 0.3, 0.8]),
        Some([0.9, 0.95, 1.0]),
        1.5,
    )?;

    // 2. Extract text, Markdown, RAG chunks, or render to RGBA Pixmap
    let text = doc.extract_page_text(0)?;
    let markdown = doc.to_markdown()?;
    let pixmap = doc.render_page(0, 150.0)?;
    let png_bytes = pixmap.to_png()?;

    // 3. Serialize with compression or RC4-128 encryption
    let pdf_bytes = doc.to_bytes_encrypted(true, Some("user_pw"), Some("owner_pw"))?;
    Ok(())
}
```

---

## Documentation

Detailed guides and API references are located in the [`docs/`](docs/) directory:

- [**Python API Reference**](docs/python-api.md) — Complete reference for `Document`, `Page`, `Pixmap`, `Table`, `Rect`, `Point`, `Matrix`, and module-level functions.
- [**Common Lisp API Reference & Guide**](lisp/README.md) ([docs/lisp-api.md](docs/lisp-api.md)) — Complete reference for Common Lisp macros, functions, conditions, and types in Edi Weitz style.
- [**R Package Reference**](docs/r-api.md) — Complete reference for R functions (`pdf_open`, `pdf_render`, `pdf_find_tables`, `pdf_extract_text`, etc.) and `uvr` usage.
- [**Rust (`pdfrune`) API Reference**](docs/rust-api.md) — Guide to using `pdfrune` directly in Rust applications.
- [**PyMuPDF (`fitz`) Migration Guide**](docs/migration-from-pymupdf.md) — Side-by-side cheat sheet for migrating from PyMuPDF to `pdfrune`.
- [**Engine Architecture & Security**](docs/architecture.md) — Deep dive into the ISO 32000 parser, XREF auto-repair, font CMap engine, `tiny-skia`/`ab_glyph` renderer, true redaction scrubber, and RC4-128 crypto handler.

---

## Development & Testing

All Python workflows use `uv`, R workflows use `uvr`, Common Lisp workflows use ANSI ASDF/Quicklisp, and all Rust crates are checked with `cargo`:

```bash
# Format, lint, and run Rust unit tests
cargo fmt --check
cargo clippy --all-targets -- -D warnings
cargo test

# Run the full Python test suite
uv run pytest

# Run the full R test suite
cd r && uvr run run_tests.R

# Run the full Common Lisp test suite
sbcl --noinform --non-interactive \
  --eval '(ql:quickload :cl-pdfrune-test)' \
  --eval '(asdf:test-system :cl-pdfrune)'
```

---

## Author & License

**Author**: Gwang-Jin Kim (<gwang.jin.kim.phd@gmail.com>)

Dual-licensed under either of:

- **Apache License, Version 2.0** ([LICENSE-APACHE](LICENSE-APACHE) or <http://www.apache.org/licenses/LICENSE-2.0>)
- **MIT License** ([LICENSE-MIT](LICENSE-MIT) or <http://opensource.org/licenses/MIT>)

at your option.
