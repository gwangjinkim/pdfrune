# pdfrune Common Lisp API Reference

The `pdfrune` package (nickname `:cl-pdfrune`) provides high-performance, ANSI-compliant Common Lisp bindings for the `pdfrune` engine.

All code is strictly portable across ANSI Common Lisp implementations (SBCL, CCL, ECL, ABCL, CLISP, Allegro, LispWorks).

---

## Package Summary

```lisp
(in-package :pdfrune)
;; or
(use-package :pdfrune)
```

Exported symbols include:

### Document Management

| Function / Macro | Description |
| :--- | :--- |
| `(with-open-pdf (var source &key password) &body body)` | Safe lexical scoping macro ensuring document destruction on exit |
| `(open-pdf source &key password)` | Opens PDF from pathname, string, or byte vector |
| `(new-pdf)` | Creates an empty in-memory PDF document |
| `(close-pdf doc)` | Frees native PDF memory immediately |
| `(page-count doc)` | Returns total number of pages |
| `(document-metadata doc)` | Returns metadata plist |
| `(set-document-metadata doc &key title author ...)` | Updates metadata fields in the document information catalog |
| `(save-document doc path &key (deflate t) encryption ...)` | Saves document to file system |
| `(document-to-bytes doc &key (deflate t) encryption ...)` | Serializes document to `(vector (unsigned-byte 8))` |
| `(merge-documents files-or-bytes-list &key target-path)` | Fast concatenation of multiple PDF files |

### Page Operations & Content Drawing

| Function | Description |
| :--- | :--- |
| `(page-info doc page-num)` | Returns page dimensions, mediabox, and cropbox |
| `(new-page doc &key (width 612.0) (height 792.0))` | Appends a new blank page |
| `(delete-page doc page-num)` | Removes a page from the document |
| `(insert-text doc page-num point text &key font-size color)` | Draws text at `(x y)` |
| `(draw-rect doc page-num rect &key fill-color stroke-color stroke-width)` | Draws a vector rectangle `(x0 y0 x1 y1)` |

### Text, Words, Blocks & Search

| Function | Description |
| :--- | :--- |
| `(extract-text doc page-num)` | Extracts clean plain text |
| `(extract-words doc page-num)` | Extracts words with coordinates and line numbers |
| `(extract-blocks doc page-num)` | Extracts layout blocks with structural bounding boxes |
| `(search-page doc page-num needle)` | Returns list of bounding boxes matching `needle` on page |
| `(search-document doc needle)` | Fast multi-page search returning `(:page p :bbox (...))` |
| `(to-markdown doc &optional page-num)` | Formats page or document as clean Markdown |
| `(to-html doc page-num)` | Converts page layout into HTML |
| `(to-rag-chunks doc &key (chunk-size 500) (overlap 50))` | Generates semantic chunks with page and bounding box metadata |

### Table Detection

| Function | Description |
| :--- | :--- |
| `(find-tables doc page-num)` | Detects tabular data structures and returns `pdf-table` structs |
| `(table-row-count table)` | Number of rows |
| `(table-col-count table)` | Number of columns |
| `(table-bbox table)` | Bounding box `(x0 y0 x1 y1)` |
| `(table-header table)` | List of string headers |
| `(table-rows table)` | List of lists of cell text |
| `(table-markdown table)` | Pre-formatted Markdown table string |

### Rendering & Vector Export

| Function | Description |
| :--- | :--- |
| `(render-page doc page-num &key (dpi 150.0))` | Rasterizes page to a `pixmap` struct |
| `(pixmap-width pix)` / `(pixmap-height pix)` | Pixel dimensions |
| `(pixmap-samples pix)` | Raw 32-bit RGBA pixel byte vector |
| `(pixmap-to-png pix)` | Encodes pixmap into PNG byte vector |
| `(save-pixmap pix path)` | Saves pixmap to PNG file on disk |
| `(page-to-svg doc page-num)` | Generates clean SVG markup |

### Annotations, Redactions & Forms

| Function | Description |
| :--- | :--- |
| `(page-annotations doc page-num)` | Lists all annotations on a page |
| `(add-highlight-annotation doc page-num rect &key color content)` | Highlights text in `rect` |
| `(add-text-annotation doc page-num point text &key color)` | Creates sticky note annotation |
| `(add-rect-annotation doc page-num rect &key stroke-color fill-color)` | Draws markup rectangle |
| `(delete-annotation doc page-num index)` | Removes annotation |
| `(add-redaction doc page-num rect &key fill-color)` | Stages a redaction zone |
| `(apply-redactions doc)` | Strips redacted text from content streams permanently |
| `(page-widgets doc page-num)` | Lists interactive AcroForm fields |
| `(add-widget doc page-num name type rect &optional default-val)` | Creates a new form field |
| `(set-widget-value doc name value)` | Populates form field value |

### Navigation & Outlines

| Function | Description |
| :--- | :--- |
| `(document-toc doc)` | Retrieves outline bookmarks hierarchy |
| `(set-document-toc doc entries)` | Sets document outline hierarchy |
| `(page-links doc page-num)` | Extracts external URIs and internal page links |
| `(insert-link doc page-num rect &key uri target-page)` | Inserts a clickable link |
| `(delete-link doc page-num index)` | Removes link at index |

### Security & Password Authentication

| Function | Description |
| :--- | :--- |
| `(document-encrypted-p doc)` | Checks if PDF is encrypted |
| `(authenticate-document doc password)` | Unlocks encrypted PDF using password |
