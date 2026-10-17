# rupdf

**Ultra-Fast, Memory-Safe, Permissive PDF Engine in Rust & Python**

`rupdf` is a clean-room, high-performance PDF toolkit written in pure, memory-safe Rust (`rupdf-core`) with ergonomic Python bindings (`rupdf`). It provides full parity with PyMuPDF (`fitz`) workflows plus modern multi-core parallelism, structured Markdown/HTML export, SIMD-accelerated search, and AI/RAG semantic chunking—all under a **100% permissive MIT OR Apache-2.0 dual license** (zero AGPL restrictions).

## Features

- **100% Permissive License**: Dual-licensed under `MIT OR Apache-2.0`. Free for commercial, academic, and private use.
- **Memory-Safe Rust Core**: Zero segfaults, buffer overflows, or use-after-free bugs on malformed PDFs.
- **GIL-Free Multi-Core Parallelism**: Batch extraction, rendering, and search scale linearly across CPU cores via `rayon`.
- **Rich Extraction & Layout Analysis**: Extract plain text, words, lines, blocks, tables, and bounding boxes.
- **Rasterization & Vector Export**: Render PDF pages to RGBA `Pixmap`, PNG, SVG, and zero-copy NumPy buffers.
- **Document Manipulation**: Merge, split, insert, delete, rotate, crop, and serialize PDFs cleanly.
- **Beyond PyMuPDF**:
  - `page.to_markdown()` / `doc.to_markdown()` & `to_html()`
  - `doc.to_rag_chunks()` with exact page & bounding box metadata
  - `rupdf.batch_extract()` & `rupdf.batch_render()`
  - Drop-in `import rupdf.fitz as fitz` compatibility shim

## License

Licensed under either of:

- Apache License, Version 2.0 ([LICENSE-APACHE](LICENSE-APACHE))
- MIT License ([LICENSE-MIT](LICENSE-MIT))

at your option.
