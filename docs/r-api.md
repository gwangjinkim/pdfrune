# R Package Reference: `pdfrune`

`pdfrune` is an ultra-fast, memory-safe, and permissively licensed PDF engine in Rust with native R bindings. It provides high-performance text extraction, tabular data extraction, rendering to RGBA pixmaps and vector SVG, document manipulation, outlines, hyperlinks, image manipulation, annotations, true content-stream redactions, and ISO 32000 standard encryption.

---

## 1. Installation

### Using `uvr` (Recommended)

`pdfrune` is managed and tested using [`uvr`](https://github.com/astral-sh/uvr). Add and sync in your R project:

```bash
uvr add pdfrune
uvr sync
```

To run an R script in your isolated `uvr` environment:

```bash
uvr run script.R
```

### From CRAN / Source

```r
# From CRAN (once published):
install.packages("pdfrune")

# Or from local source package tarball:
install.packages("pdfrune_0.1.0.tar.gz", repos = NULL, type = "source")
```

System Requirements: **Cargo** (Rust toolchain, 1.75+).

---

## 2. API Overview

All functions are prefixed with `pdf_` for consistent auto-completion.

| Function | Description |
| :--- | :--- |
| `pdf_open(path, bytes)` | Open an existing PDF from file path or raw bytes |
| `pdf_new()` | Create a new blank in-memory PDF document |
| `pdf_page_count(doc)` | Return the total number of pages |
| `pdf_metadata(doc)` | Get document metadata (title, author, subject, etc.) |
| `pdf_set_metadata(doc, ...)` | Update document metadata |
| `pdf_page_info(doc, page)` | Inspect page width, height, mediabox, and cropbox |
| `pdf_new_page(doc, ...)` | Append or insert a new blank page |
| `pdf_delete_page(doc, page)` | Delete a page by 1-based index |
| `pdf_insert_text(doc, ...)` | Insert text at `(x, y)` coordinates |
| `pdf_draw_rect(doc, ...)` | Draw vector rectangles with stroke/fill |
| `pdf_extract_text(doc, page)` | Extract plain text from a page |
| `pdf_extract_words(doc, page)` | Extract words with bounding boxes and line numbers |
| `pdf_extract_blocks(doc, page)` | Extract layout text blocks with bounding boxes |
| `pdf_search(doc, ...)` | Substring or regex search returning bounding boxes |
| `pdf_find_tables(doc, page)` | Auto-detect tables and convert to `data.frame` or Markdown |
| `pdf_to_markdown(doc, page)` | Export page or document to structured Markdown |
| `pdf_to_html(doc, page)` | Export page or document to clean HTML |
| `pdf_to_rag_chunks(doc, ...)` | Semantic chunking with bounding boxes for LLM / RAG |
| `pdf_render(doc, page, dpi)` | Rasterize page to RGBA8888 Pixmap |
| `pdf_to_svg(doc, page)` | Render page to vector SVG XML string |
| `pdf_get_toc(doc)` | Extract hierarchical Table of Contents (outlines) |
| `pdf_set_toc(doc, toc)` | Build or update Table of Contents |
| `pdf_get_links(doc, page)` | Get external URIs and internal jump links |
| `pdf_insert_link(doc, ...)` | Add URI or destination page jump link |
| `pdf_delete_link(doc, ...)` | Remove hyperlink by xref or bounding box |
| `pdf_get_images(doc, page)` | List embedded images on a page |
| `pdf_extract_image(doc, xref)` | Extract decoded image bytes, dimensions, and format |
| `pdf_insert_image(doc, ...)` | Insert image onto page with custom bounding box |
| `pdf_annots(doc, page)` | Read page annotations |
| `pdf_add_highlight_annot(...)` | Add text highlight annotation |
| `pdf_add_text_annot(...)` | Add sticky note text annotation |
| `pdf_add_rect_annot(...)` | Add rectangle markup annotation |
| `pdf_delete_annot(doc, ...)` | Delete annotation |
| `pdf_add_redact_annot(...)` | Stage rectangular area for redaction |
| `pdf_apply_redactions(doc, page)` | Permanently scrub underlying text from content stream |
| `pdf_widgets(doc, page)` | Read interactive AcroForm form fields |
| `pdf_add_widget(doc, ...)` | Add interactive text field, checkbox, or dropdown |
| `pdf_set_field_value(doc, ...)` | Set form field value |
| `pdf_is_encrypted(doc)` | Check if document requires password |
| `pdf_authenticate(doc, pw)` | Authenticate with user or owner password |
| `pdf_save(doc, path, ...)` | Save document to disk with compression/encryption |
| `pdf_to_bytes(doc, ...)` | Serialize document to raw bytes in memory |
| `pdf_merge(files, ...)` | Fast multi-file document concatenation |

---

## 3. Usage Examples

### Reading and Text Extraction

```r
library(pdfrune)

doc <- pdf_open("report.pdf")
cat("Pages:", pdf_page_count(doc), "\n")

# Plain text
txt <- pdf_extract_text(doc, page = 1L)

# Words with bounding boxes
words <- pdf_extract_words(doc, page = 1L)
head(words)

# Structured Markdown export
md <- pdf_to_markdown(doc, page = 1L)
cat(md)
```

### Table Extraction to R `data.frame`

```r
tables <- pdf_find_tables(doc, page = 1L)

for (tbl in tables) {
  df <- tbl$to_dataframe()
  print(head(df))
}
```

### High-Resolution Rendering to PNG

```r
pix <- pdf_render(doc, page = 1L, dpi = 150.0)
pix$save("page_1.png")
```

### Redaction & Form Filling

```r
# Redact sensitive text
pdf_add_redact_annot(doc, page = 1L, rect = c(100, 200, 300, 220), fill = c(0, 0, 0))
pdf_apply_redactions(doc, page = 1L)

# Fill form field
pdf_set_field_value(doc, "TaxID", "999-00-1234")

# Save encrypted copy
pdf_save(doc, "secure.pdf", encryption = "rc4-128", user_pw = "client_secret")
```
