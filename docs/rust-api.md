# Rust Crate Reference (`rupdf-core`)

`rupdf-core` is a pure, 100% safe Rust crate with zero Python or C/C++ dependencies. Every feature exposed in the Python `rupdf` library is implemented directly in `rupdf-core`.

---

## Adding `rupdf-core` to Your Rust Project

```toml
[dependencies]
rupdf-core = { path = "crates/rupdf-core" }
```

---

## Core Types & Modules

| Module / Type | Description |
| :--- | :--- |
| `rupdf_core::Document` | High-level PDF document handle (`open`, `from_bytes`, `new`, page tree, mutation, encryption, rendering). |
| `rupdf_core::PageInfo` | Page dimensions, `/MediaBox`, `/CropBox`, `/BleedBox`, `/TrimBox`, `/ArtBox`, and rotation. |
| `rupdf_core::Rect`, `Point`, `Matrix` | 2D geometry primitives and affine transformations. |
| `rupdf_core::Pixmap` | RGBA8888 pixel buffer with PNG and PPM encoders. |
| `rupdf_core::DetectedTable` | Table structure detector (`rows`, `bbox`, `extract`, `to_markdown`, `to_dicts`). |
| `rupdf_core::TocEntry`, `LinkInfo`, `LinkKind` | Hierarchical `/Outlines` bookmarks and `/Link` destinations. |
| `rupdf_core::AnnotInfo`, `WidgetInfo` | PDF annotations, redactions, and interactive `/AcroForm` widgets. |
| `rupdf_core::PageImageInfo`, `ExtractedImage` | Embedded `/Image` XObject metadata and decoded image buffers. |
| `rupdf_core::RagChunk` | Semantic text chunk with page number, section heading, and bounding box coordinates. |
| `rupdf_core::parser::*` | Low-level SIMD byte lexer, PDF DOM (`PdfObject`, `PdfDict`, `PdfStream`), XREF parser/repair, and stream filters. |

---

## Common Rust Workflows

### 1. Parsing & Repairing PDFs

```rust
use rupdf_core::Document;

fn inspect_pdf(path: &str) -> rupdf_core::Result<()> {
    // Automatically repairs broken xref tables if needed
    let doc = Document::open(path)?;
    println!("Pages: {}", doc.page_count());
    println!("Metadata: {:?}", doc.metadata());

    for i in 0..doc.page_count() {
        let text = doc.extract_page_text(i)?;
        println!("Page {} text ({} bytes)", i + 1, text.len());
    }
    Ok(())
}
```

### 2. Creating Pages, Vector Graphics, Text & Images

```rust
use rupdf_core::{Document, Point, Rect};

fn create_invoice() -> rupdf_core::Result<Vec<u8>> {
    let mut doc = Document::new();
    doc.new_page(None, 612.0, 792.0)?;

    doc.insert_text(0, Point::new(72.0, 60.0), "INVOICE #2026-01", 20.0, [0.1, 0.2, 0.5])?;
    doc.draw_rect(
        0,
        Rect::new(72.0, 80.0, 540.0, 82.0),
        Some([0.1, 0.2, 0.5]),
        Some([0.1, 0.2, 0.5]),
        1.0,
    )?;

    // Serialize with Flate compression
    doc.to_bytes(true)
}
```

### 3. Table Detection, Markdown & RAG Chunks

```rust
use rupdf_core::Document;

fn analyze_layout(doc: &Document) -> rupdf_core::Result<()> {
    // Detect tables on page 0
    for table in doc.find_page_tables(0)? {
        println!("Table ({}x{}):\n{}", table.row_count(), table.col_count(), table.to_markdown());
    }

    // Parallel Markdown export across all pages
    let md = doc.to_markdown()?;

    // Semantic chunks for vector databases
    for chunk in doc.to_rag_chunks(500)? {
        println!(
            "[Page {} @ {:?}] ({:?}): {}",
            chunk.page_number, chunk.bbox, chunk.section_heading, chunk.text
        );
    }
    Ok(())
}
```

### 4. True Content-Stream Redaction & RC4-128 Encryption

```rust
use rupdf_core::{Document, Rect};

fn redact_and_encrypt(mut doc: Document) -> rupdf_core::Result<Vec<u8>> {
    // Add a redaction annotation over sensitive coordinates
    doc.add_page_annot(
        0,
        "Redact",
        Rect::new(70.0, 120.0, 280.0, 150.0),
        None,
        [0.0, 0.0, 0.0],
        Some("[REDACTED]"),
    )?;

    // Permanently scrub matching text operators from the underlying content stream
    let scrubbed = doc.apply_page_redactions(0)?;
    println!("Applied {} redactions", scrubbed);

    // Encrypt all streams and strings using ISO 32000 RC4-128
    doc.to_bytes_encrypted(true, Some("user_secret"), Some("owner_secret"))
}
```
