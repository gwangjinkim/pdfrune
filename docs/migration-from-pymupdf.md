# Migrating from PyMuPDF (`fitz`) to `pdfrune`

`pdfrune` is designed so existing Python codebases using PyMuPDF (`import fitz` or `import pymupdf`) can switch with minimal or zero changes while moving from an AGPL-3.0 dependency to a **100% permissive `MIT OR Apache-2.0`** Rust engine.

---

## Option 1: Drop-In Compatibility Shim

Replace `import fitz` with:

```python
import pdfrune.fitz as fitz
```

All standard `fitz.open(...)`, `doc.load_page(i)`, `page.get_text(...)`, `page.search_for(...)`, `page.get_pixmap(...)`, `doc.get_toc()`, `page.get_links()`, `page.get_images()`, `page.annots()`, `page.widgets()`, and `page.find_tables()` calls work out of the box.

---

## Option 2: Idiomatic `pdfrune` API Cheat Sheet

| Operation | PyMuPDF (`fitz`) | `pdfrune` |
| :--- | :--- | :--- |
| **Open file / bytes** | `fitz.open("a.pdf")` / `fitz.open(stream=b, filetype="pdf")` | `pdfrune.open("a.pdf")` / `pdfrune.open(b)` |
| **Create empty PDF** | `fitz.open()` | `pdfrune.open()` |
| **Extract plain text** | `page.get_text("text")` | `page.extract_text()` (or `page.get_text("text")`) |
| **Extract words** | `page.get_text("words")` | `page.extract_words()` (or `page.get_text("words")`) |
| **Extract blocks** | `page.get_text("blocks")` | `page.extract_blocks()` (or `page.get_text("blocks")`) |
| **Search text** | `page.search_for("term")` | `page.search("term")` (or `page.search_for("term")`) |
| **Cross-document search** | Manual loop over pages | `doc.search_all("term", regex=True)` (parallel SIMD) |
| **Render page to image** | `page.get_pixmap(dpi=150)` | `page.render(dpi=150)` (or `page.get_pixmap(dpi=150)`) |
| **Export SVG** | `page.get_svg_image()` | `page.to_svg()` (or `page.get_svg_image()`) |
| **Export Markdown** | `pymupdf4llm.to_markdown(doc)` | `doc.to_markdown()` / `page.to_markdown()` |
| **Detect tables** | `page.find_tables()` | `page.find_tables()` |
| **Table of Contents** | `doc.get_toc()` / `doc.set_toc(toc)` | `doc.get_toc()` / `doc.set_toc(toc)` |
| **Hyperlinks** | `page.get_links()` / `page.insert_link(d)` | `page.get_links()` / `page.insert_link(d)` |
| **Images** | `page.get_images()`, `doc.extract_image(xref)`, `page.insert_image(rect, ...)` | Identical |
| **Annotations** | `page.annots()`, `page.add_highlight_annot(rect)` | Identical |
| **Redactions** | `page.add_redact_annot(rect)`, `page.apply_redactions()` | Identical |
| **AcroForm Widgets** | `page.widgets()`, `page.add_widget(w)` | Identical + `page.set_field_value(name, val)` |
| **Save encrypted PDF** | `doc.save("out.pdf", encryption=..., user_pw="...")` | `doc.save("out.pdf", encryption="rc4-128", user_pw="...")` |
| **Authenticate PDF** | `doc.authenticate("pw")` | `doc.authenticate("pw")` |
