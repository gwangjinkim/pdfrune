# rupdf — Ultra-Fast, Memory-Safe PDF Engine for Common Lisp

> **Author**: Gwang-Jin Kim \<gwang.jin.kim.phd@gmail.com\>  
> **License**: [MIT](../LICENSE-MIT) or [Apache-2.0](../LICENSE-APACHE)  
> **Status**: Ready for Quicklisp & Ultralisp  
> **Standards Compliance**: ANSI Common Lisp (portable across SBCL, CCL, ECL, ABCL, CLISP, Allegro, LispWorks)

---

### Abstract

`rupdf` is a clean-room, high-performance, and permissively licensed Common Lisp PDF library modeled after the design philosophy of Edi Weitz's libraries (`cl-ppcre`, `cl-fad`, `drakma`). Built on top of the memory-safe Rust `rupdf-core` engine via CFFI, it provides Common Lisp developers with full PDF capabilities:

* **High-Speed Text Extraction & Layout**: extract plain text, word bounding boxes, structural layout blocks, and SIMD text search.
* **Modern Document Understanding**: automated table detection with Markdown export, page/document Markdown & HTML rendering, and AI/RAG semantic chunking with bounding box grounding.
* **High-Resolution Rasterization & Vector Output**: multi-DPI RGBA pixmaps, PNG serialization, and crisp SVG vector exports.
* **Document Manipulation & Assembly**: create new PDFs from scratch, merge multiple PDFs, delete/reorder pages, draw vector rectangles, and insert TrueType typography.
* **Outlines & Hyperlinks**: extract and build hierarchical Table of Contents (bookmarks), internal page links (`/GoTo`), and external web URIs.
* **Annotations & Redaction**: highlight, sticky text notes, square markups, and irreversible PDF redaction that physically strips sensitive glyphs from underlying content streams.
* **Interactive AcroForms**: inspect form widgets, add text fields, checkboxes, dropdowns, and programmatically populate values.
* **Standard PDF Cryptography**: open and decrypt RC4-128 and AES-128 password-protected PDFs, or save encrypted documents.

---

## Contents

1. [Installation & Requirements](#installation--requirements)
2. [Shared Library Setup](#shared-library-setup)
3. [Quickstart Tutorial](#quickstart-tutorial)
4. [Dictionary & API Reference](#dictionary--api-reference)
   - [Document Lifecycle & Scope](#document-lifecycle--scope)
   - [Page Geometry & Mutation](#page-geometry--mutation)
   - [Text, Words, Blocks & Search](#text-words-blocks--search)
   - [Table Detection & Markdown](#table-detection--markdown)
   - [Rendering (Pixmap, PNG, SVG)](#rendering-pixmap-png-svg)
   - [Table of Contents & Hyperlinks](#table-of-contents--hyperlinks)
   - [Image Extraction & Insertion](#image-extraction--insertion)
   - [Annotations & Redactions](#annotations--redactions)
   - [AcroForm Interactive Widgets](#acroform-interactive-widgets)
   - [Security & Encryption](#security--encryption)
   - [Conditions & Errors](#conditions--errors)
5. [Running Tests](#running-tests)
6. [License](#license)

---

## Installation & Requirements

`rupdf` requires an ANSI-compliant Common Lisp implementation with `CFFI` and `YASON`.

### Ultralisp & Quicklisp

Once published or cloned to your `~/quicklisp/local-projects/`:

```lisp
(ql:quickload :rupdf)
```

### Manual ASDF

Symlink `rupdf.asd` and `rupdf-test.asd` to your ASDF source registry, then load:

```lisp
(asdf:load-system :rupdf)
```

---

## Shared Library Setup

`rupdf` binds to the native C shared library `librupdf_ffi` (or `rupdf_ffi.dll` on Windows).

By default, `rupdf` searches:
1. `rupdf:*rupdf-library-path*` (if explicitly set)
2. Build directories: `target/release/`, `target/debug/`
3. Standard system locations (`/usr/local/lib`, `/opt/homebrew/lib`, `/usr/lib`)

To build the native shared library from the repository root:

```bash
cargo build --release -p rupdf-ffi
```

To point Common Lisp to a custom library path before loading:

```lisp
(setf rupdf:*rupdf-library-path* #p"/opt/rupdf/librupdf_ffi.dylib")
(asdf:load-system :rupdf)
```

---

## Quickstart Tutorial

### Reading Text and Searching

```lisp
(rupdf:with-open-pdf (doc #p"paper.pdf")
  (format t "Document has ~D pages.~%" (rupdf:page-count doc))
  
  ;; Extract plain text from page 0
  (format t "Page 1 Text:~%~A~%" (rupdf:extract-text doc 0))
  
  ;; Search for matches across the entire document
  (dolist (match (rupdf:search-document doc "Common Lisp"))
    (format t "Found on page ~D at bbox ~A~%"
            (getf match :page)
            (getf match :bbox))))
```

### Creating and Modifying Documents

```lisp
(rupdf:with-open-pdf (doc (rupdf:new-pdf))
  ;; Add a standard letter page (612 x 792 pt)
  (rupdf:new-page doc :width 612.0 :height 792.0)
  
  ;; Draw decorative rectangle
  (rupdf:draw-rect doc 0 '(50.0 700.0 562.0 750.0)
                   :fill-color '(0.1 0.2 0.5)
                   :stroke-color '(0.0 0.0 0.0)
                   :stroke-width 1.5)
                   
  ;; Insert text
  (rupdf:insert-text doc 0 '(72.0 720.0) "Hello from Common Lisp & Rust!"
                     :font-size 18.0
                     :color '(1.0 1.0 1.0))
                     
  ;; Save to disk with Flate compression
  (rupdf:save-document doc #p"hello.pdf" :deflate t))
```

### Table Detection and Markdown

```lisp
(rupdf:with-open-pdf (doc #p"financial_report.pdf")
  (let ((tables (rupdf:find-tables doc 0)))
    (dolist (tab tables)
      (format t "Detected ~Dx~D table:~%"
              (rupdf:table-row-count tab)
              (rupdf:table-col-count tab))
      (format t "~A~%~%" (rupdf:table-markdown tab)))))
```

### Rasterization & PNG Export

```lisp
(rupdf:with-open-pdf (doc #p"manual.pdf")
  ;; Render page 0 at 300 DPI
  (let ((pix (rupdf:render-page doc 0 :dpi 300.0)))
    (format t "Rendered pixmap: ~Dx~D px~%"
            (rupdf:pixmap-width pix)
            (rupdf:pixmap-height pix))
    ;; Save as PNG
    (rupdf:save-pixmap pix #p"page1.png")))
```

---

## Dictionary & API Reference

### Document Lifecycle & Scope

#### `with-open-pdf (var path-or-bytes &key password) &body body`
Macro. Opens a document from pathname, string, or `(vector (unsigned-byte 8))`, binds it to `VAR`, evaluates `BODY`, and guarantees `close-pdf` is called upon normal or non-local exit.

#### `open-pdf (path-or-bytes &key password)`
Opens and parses a PDF document. Returns a `pdf-document` struct.

#### `new-pdf ()`
Creates a new in-memory empty PDF document.

#### `close-pdf (doc)`
Closes the document and immediately frees native Rust resources. Idempotent.

#### `page-count (doc)`
Returns the integer count of pages in `DOC`.

#### `document-metadata (doc)`
Returns a plist of metadata (`:TITLE`, `:AUTHOR`, `:SUBJECT`, `:KEYWORDS`, `:CREATOR`, `:PRODUCER`, `:FORMAT`, etc.).

#### `set-document-metadata (doc &key title author subject keywords creator producer)`
Updates document metadata in the PDF information dictionary.

#### `save-document (doc path &key (deflate t) encryption user-password owner-password)`
Writes the document to disk.

#### `document-to-bytes (doc &key (deflate t) encryption user-password owner-password)`
Serializes the document to an in-memory `(simple-array (unsigned-byte 8) (*))`.

#### `merge-documents (files-or-bytes-list &key target-path)`
Concatenates multiple documents into a single PDF. Returns serialized bytes or writes to `TARGET-PATH`.

---

### Page Geometry & Mutation

#### `page-info (doc page-number)`
Returns a plist containing page dimensions (`:WIDTH`, `:HEIGHT`, `:ROTATION`, `:MEDIABOX`, `:CROPBOX`).

#### `new-page (doc &key (width 612.0) (height 792.0))`
Appends a blank page. Returns the new 0-based page index.

#### `delete-page (doc page-number)`
Deletes the page at `PAGE-NUMBER`.

#### `insert-text (doc page-number point text &key (font-size 12.0) color)`
Draws `TEXT` at `POINT` `(x y)`.

#### `draw-rect (doc page-number rect &key fill-color stroke-color (stroke-width 1.0))`
Draws a vector rectangle `(x0 y0 x1 y1)`.

---

### Text, Words, Blocks & Search

#### `extract-text (doc page-number)`
Returns extracted plain text string.

#### `extract-words (doc page-number)`
Returns a list of plists `(:TEXT "..." :BBOX (x0 y0 x1 y1) :WORD-NO n :LINE-NO m)`.

#### `extract-blocks (doc page-number)`
Returns a list of plists `(:TEXT "..." :BBOX (x0 y0 x1 y1) :BLOCK-TYPE "text"|"image")`.

#### `search-page (doc page-number needle)`
Searches for `NEEDLE` and returns a list of bounding boxes `(list x0 y0 x1 y1)`.

#### `search-document (doc needle)`
Searches across all pages. Returns a list of plists `(:PAGE index :BBOX (x0 y0 x1 y1))`.

#### `to-markdown (doc &optional page-number)`
Converts visual layout into clean Markdown. If `PAGE-NUMBER` is omitted, converts all pages.

#### `to-html (doc page-number)`
Converts page layout into semantic HTML.

#### `to-rag-chunks (doc &key (chunk-size 500) (overlap 50))`
Splits document text into LLM/RAG-ready chunks with `:PAGE` and `:BBOX` metadata.

---

### Table Detection & Markdown

#### `find-tables (doc page-number)`
Detects tabular layouts on `PAGE-NUMBER` using geometric boundary analysis. Returns a list of `pdf-table` structures.

#### `pdf-table` Accessors:
* `(table-row-count table)`
* `(table-col-count table)`
* `(table-bbox table)` -> `(x0 y0 x1 y1)`
* `(table-header table)` -> list of column strings
* `(table-rows table)` -> list of row cell lists
* `(table-markdown table)` -> formatted Markdown table

---

### Rendering (Pixmap, PNG, SVG)

#### `render-page (doc page-number &key (dpi 150.0))`
Rasterizes `PAGE-NUMBER` to a `pixmap` structure.
* `(pixmap-width pix)`
* `(pixmap-height pix)`
* `(pixmap-stride pix)`
* `(pixmap-samples pix)` -> `(simple-array (unsigned-byte 8) (*))` RGBA buffer.

#### `save-pixmap (pixmap path)`
Encodes and saves `PIXMAP` as a PNG file.

#### `pixmap-to-png (pixmap)`
Encodes `PIXMAP` directly into an in-memory PNG byte vector.

#### `page-to-svg (doc page-number)`
Exports vector page representation as an SVG string.

---

### Table of Contents & Hyperlinks

#### `document-toc (doc)`
Returns hierarchical outline entries: `(:LEVEL n :TITLE "..." :PAGE index)`.

#### `set-document-toc (doc toc-entries)`
Replaces document outlines with `TOC-ENTRIES`.

#### `page-links (doc page-number)`
Returns hyperlinks on the page: `(:BBOX (x0 y0 x1 y1) :KIND "uri"|"goto" :URI "..." :PAGE index)`.

#### `insert-link (doc page-number rect &key uri target-page)`
Adds a clickable hyperlink.

#### `delete-link (doc page-number link-index)`
Removes the link at `LINK-INDEX`.

---

### Image Extraction & Insertion

#### `page-images (doc page-number)`
Returns inventory of embedded images: `(:XREF id :NAME "..." :WIDTH w :HEIGHT h :COLORSPACE "...")`.

#### `extract-image (doc xref)`
Extracts raw image bytes and metadata.

#### `insert-image (doc page-number rect &key path bytes)`
Embeds an image into `PAGE-NUMBER` within `RECT`.

---

### Annotations & Redactions

#### `page-annotations (doc page-number)`
Returns annotations on the page: `(:INDEX i :TYPE "..." :RECT (...) :CONTENT "...")`.

#### `add-highlight-annotation (doc page-number rect &key color content)`
Adds a text highlight annotation.

#### `add-text-annotation (doc page-number point text &key color)`
Adds a sticky note annotation.

#### `add-rect-annotation (doc page-number rect &key stroke-color fill-color)`
Adds a geometric square annotation.

#### `delete-annotation (doc page-number annot-index)`
Removes the annotation at `ANNOT-INDEX`.

#### `add-redaction (doc page-number rect &key fill-color)`
Marks `RECT` for irreversible redaction.

#### `apply-redactions (doc)`
Physically sanitizes the content stream, scrubbing redacted text and drawing black redaction rectangles.

---

### AcroForm Interactive Widgets

#### `page-widgets (doc page-number)`
Returns interactive form fields: `(:NAME "..." :TYPE "text"|"checkbox"|"choice" :VALUE "..." :RECT (...))`.

#### `add-widget (doc page-number name field-type rect &optional default-value)`
Creates an interactive form field.

#### `set-widget-value (doc field-name new-value)`
Updates the value of form field `FIELD-NAME`.

---

### Security & Encryption

#### `document-encrypted-p (doc)`
Returns `T` if `DOC` is encrypted with standard PDF security (RC4 / AES).

#### `authenticate-document (doc password)`
Authenticates with user or owner password. Returns `T` on success.

---

### Conditions & Errors

* `rupdf-error`: Root condition for all `rupdf` errors.
* `rupdf-file-error`: Signaled when a PDF file cannot be opened or parsed. Reader: `rupdf-error-pathname`.
* `rupdf-crypto-error`: Signaled when password authentication fails or an encrypted stream cannot be read.
* `rupdf-page-out-of-bounds-error`: Signaled when a page index is outside `[0, page-count)`. Readers: `rupdf-error-page-index`, `rupdf-error-page-count`.

---

## Running Tests

Run the full test suite with ASDF / Quicklisp:

```lisp
(asdf:test-system :rupdf)
```

Or from the command line:

```bash
sbcl --noinform --non-interactive \
  --eval '(ql:quickload :rupdf-test)' \
  --eval '(asdf:test-system :rupdf)'
```

---

## Author & License

* **Author**: Gwang-Jin Kim \<gwang.jin.kim.phd@gmail.com\>
* **License**: Dual licensed under [MIT](../LICENSE-MIT) or [Apache-2.0](../LICENSE-APACHE).
