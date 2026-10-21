# Python API Reference (`pdfrune`)

This document describes every class, method, and function exported by the `pdfrune` Python package.

---

## Module-Level Functions

### `pdfrune.open(filename=None, *, stream=None, filetype=None) -> Document`
Opens an existing PDF document or creates a new empty PDF document.
- **`filename`**: `str | PathLike | bytes | BinaryIO | None`
  - If `None` (and `stream` is `None`), creates a new empty in-memory PDF document.
  - If a `str` or `PathLike`, reads the PDF file from disk.
  - If `bytes` or `bytearray`, parses the PDF directly from memory.
  - If a file-like object with `.read()`, reads the bytes and parses the PDF.
- **`stream`**: Optional `bytes | bytearray | BinaryIO` containing PDF data (PyMuPDF `fitz.open(stream=..., filetype="pdf")` compatible).

### `pdfrune.merge(documents: list[Document | str | PathLike]) -> Document`
Combines multiple `Document` instances or PDF file paths into a single new `Document` preserving all pages and resources.

### `pdfrune.batch_extract(inputs: list[Document | str | PathLike | bytes]) -> list[str]`
Extracts plain text from a batch of PDF documents concurrently across all CPU cores using `rayon` with the Python GIL released.

### `pdfrune.batch_render(source: Document | str | PathLike | bytes, *, dpi: float = 72.0) -> list[Pixmap]`
Renders all pages of a PDF document in parallel across CPU cores with the Python GIL released, returning a list of `Pixmap` objects in page order.

### `pdfrune.benchmark(pages: int = 50) -> dict[str, float | int]`
Synthesizes a `pages`-page PDF in Rust and measures multi-core throughput (`parse_ms`, `extract_ms`, `search_ms`, `markdown_ms`, `render_ms`, `pages_per_sec_extract`, `pages_per_sec_render`).

### `pdfrune.version() -> str`
Returns the version string of the underlying `pdfrune` engine.

---

## `Document`

Represents an open or newly created PDF document. Supports context managers (`with pdfrune.open(...) as doc:`), indexing (`doc[0]`, `doc[-1]`), slicing (`doc[1:4]`), `len(doc)`, and iteration (`for page in doc:`).

### Properties
- **`page_count -> int`**: Number of pages in the document.
- **`metadata -> dict[str, str]`**: Dictionary of document `/Info` metadata (`title`, `author`, `subject`, `keywords`, `creator`, `producer`, `creationDate`, `modDate`).
- **`is_encrypted -> bool`**: `True` if the PDF contains an `/Encrypt` security dictionary.
- **`needs_pass -> bool`**: `True` if the document is encrypted and requires authentication via `doc.authenticate(password)`.
- **`is_closed -> bool`**: `True` if `doc.close()` has been called.

### Page & Document Methods
- **`load_page(page_id: int) -> Page`**: Loads a 0-based page (supports negative indexing).
- **`new_page(pno: int = -1, width: float = 612.0, height: float = 792.0) -> Page`**: Creates and inserts a blank page at `pno` (`-1` appends to the end).
- **`delete_page(pno: int) -> None`**: Deletes the page at index `pno`.
- **`select(page_indices: list[int]) -> None`**: Retains only the specified page indices in the given order.
- **`extract_pages(page_indices: list[int]) -> Document`**: Clones the specified pages into a new `Document`.
- **`insert_pdf(other: Document, from_page: int = 0, to_page: int = -1, start_at: int = -1) -> None`**: Copies a range of pages and their referenced objects from `other` into `self`.
- **`set_metadata(metadata: dict[str, str]) -> None`**: Updates the `/Info` dictionary fields.
- **`authenticate(password: str) -> bool`**: Verifies a user or owner password against the `/Encrypt` dictionary and decrypts all PDF streams and strings in-place.

### Navigation, Images, Search & Export
- **`get_toc() -> list[list[int | str]]`**: Extracts the hierarchical Table of Contents (`/Outlines`) as `[[level, title, page_1based], ...]`.
- **`set_toc(toc: list[list[int | str] | tuple[int, str, int]]) -> None`**: Builds a hierarchical `/Outlines` tree in the document catalog.
- **`extract_image(xref: int) -> dict[str, object]`**: Extracts an embedded `/Image` XObject by object ID, returning `{"ext": "png" | "jpeg", "image": bytes, "width": int, "height": int, "colorspace": int, "bpc": int}`.
- **`search_all(query: str, *, regex: bool = False) -> list[tuple[int, list[Rect]]]`**: Searches all pages in parallel using SIMD substring matching (`aho-corasick`) or regular expressions (`regex`), returning `(page_index, [Rect, ...])` pairs.
- **`to_markdown() -> str`**: Converts all pages to structured Markdown (parallelized across pages).
- **`to_html() -> str`**: Converts all pages to semantic HTML5 (parallelized across pages).
- **`to_rag_chunks(max_chars: int = 500) -> list[dict[str, object]]`**: Splits the document into semantically coherent chunks for LLM/RAG pipelines, each containing `chunk_id`, `page_number`, `bbox`, `text`, `char_count`, and `section_heading`.

### Serialization & Saving
- **`to_bytes(*, deflate: bool = False, encryption: str | None = None, user_pw: str | None = None, owner_pw: str | None = None) -> bytes`**: Serializes the document to valid ISO 32000 PDF bytes. When `encryption="rc4-128"` or `user_pw`/`owner_pw` is provided, encrypts all streams and strings using 128-bit RC4 (`V=2, R=3`).
- **`write(...) -> bytes`**: Alias for `to_bytes(...)`.
- **`save(filename: str | PathLike, *, deflate: bool = False, encryption: str | None = None, user_pw: str | None = None, owner_pw: str | None = None) -> None`**: Writes the serialized PDF to disk.
- **`close() -> None`**: Releases the underlying Rust document memory immediately.

---

## `Page`

Represents a single page in a `Document`.

### Geometry & Rotation Properties
- **`number -> int`**: 0-based page index.
- **`rect -> Rect`**: Effective visible page box starting at `(0, 0)` after applying cropbox and rotation.
- **`mediabox -> Rect`** (get/set): Page `/MediaBox`.
- **`cropbox -> Rect`** (get/set): Page `/CropBox`.
- **`bleedbox -> Rect`**, **`trimbox -> Rect`**, **`artbox -> Rect`**: Additional PDF page boxes.
- **`width -> float`**, **`height -> float`**: Effective page dimensions in points (1/72 inch).
- **`rotation -> int`** (get/set) & **`set_rotation(deg: int)`**: Page rotation in degrees (`0`, `90`, `180`, `270`).

### Text, Layout & Table Extraction
- **`extract_text() -> str`**: Extracts plain text in visual reading order.
- **`extract_words() -> list[tuple[float, float, float, float, str, int, int, int]]`**: Returns `(x0, y0, x1, y1, word, block_no, line_no, word_no)` for every word on the page.
- **`extract_blocks() -> list[tuple[float, float, float, float, str, int, int]]`**: Returns `(x0, y0, x1, y1, text, block_no, block_type)` for each paragraph/block.
- **`get_text(option: str = "text") -> str | list[tuple]`**: PyMuPDF-compatible selector supporting `"text"`, `"words"`, `"blocks"`, `"json"`, `"markdown"`, and `"html"`.
- **`search(needle: str) -> list[Rect]`** / **`search_for(needle: str) -> list[Rect]`**: Case-insensitive substring search returning bounding boxes.
- **`find_tables() -> list[Table]`**: Automatically detects multi-column tabular layouts on the page and returns [`Table`](#table) instances.
- **`to_markdown() -> str`** / **`to_html() -> str`**: Exports the single page to Markdown or HTML.

### Drawing, Text & Image Insertion
- **`insert_text(point: Point, text: str, fontsize: float = 12.0, color: tuple[float, float, float] = (0.0, 0.0, 0.0)) -> None`**: Appends text at top-down `point` `(x, y)`.
- **`draw_rect(rect: Rect, color: tuple[float, float, float] | None = (0.0, 0.0, 0.0), fill: tuple[float, float, float] | None = None, width: float = 1.0) -> None`**: Draws a stroked and/or filled rectangle.
- **`get_images() -> list[tuple[int, int, int, int, int, str, str, str, str]]`**: Returns `(xref, smask, width, height, bpc, colorspace, alt, name, filter)` for all `/Image` XObjects on the page.
- **`insert_image(rect: Rect | tuple, *, filename=None, stream=None, pixmap=None) -> int`**: Embeds a PNG, JPEG, or `Pixmap` as an `/Image` XObject and draws it inside `rect`. Returns the allocated object `xref`.

### Rendering
- **`render(dpi: float = 72.0) -> Pixmap`** / **`get_pixmap(*, dpi: float = 72.0) -> Pixmap`**: Rasterizes the page at `dpi` into an RGBA8888 `Pixmap`.
- **`to_svg() -> str`** / **`get_svg_image() -> str`**: Renders the page as a standalone SVG XML string.

### Hyperlinks, Annotations, Redactions & AcroForms
- **`get_links() -> list[dict[str, object]]`**: Returns all `/Link` annotations on the page (`{"kind": "uri" | "goto", "rect": Rect, "uri": str, "page": int, "xref": int}`).
- **`insert_link(link: dict[str, object]) -> None`**: Inserts an external (`{"rect": ..., "uri": "https://..."}`) or internal (`{"rect": ..., "page": 1}`) link.
- **`delete_link(link: dict[str, object]) -> None`**: Removes a link annotation by `xref` or `rect`.
- **`annots() -> list[dict[str, object]]`**: Returns all page annotations (`xref`, `type`, `rect`, `contents`, `color`, `overlay_text`).
- **`add_highlight_annot(rect) -> dict`**, **`add_text_annot(point, text) -> dict`**, **`add_rect_annot(rect) -> dict`**: Creates standard PDF annotations.
- **`delete_annot(annot: dict) -> None`**: Deletes an annotation from the page's `/Annots` array.
- **`add_redact_annot(rect, text=None, fill=(0.0, 0.0, 0.0)) -> dict`**: Marks a rectangular area for redaction.
- **`apply_redactions() -> int`**: Permanently removes all text spans intersecting any `/Redact` annotation from the page's content stream, draws the redaction fill/overlay text, and deletes the `/Redact` annotations. Returns the number of redactions applied.
- **`widgets() -> list[dict[str, object]]`**: Lists all interactive `/AcroForm` `/Widget` fields on the page (`xref`, `field_name`, `field_type`, `field_value`, `rect`).
- **`add_widget(widget: dict[str, object]) -> dict[str, object]`**: Creates an interactive form field (`field_type` in `"text"`, `"checkbox"`, `"combobox"`, `"listbox"`).
- **`set_field_value(field_name: str, value: str) -> bool`**: Updates the value and appearance stream of a form widget by `field_name`.

---

## `Table`

Returned by `page.find_tables()`.

- **`row_count -> int`**: Number of rows.
- **`col_count -> int`**: Number of columns.
- **`bbox -> Rect`**: Bounding box enclosing the entire table.
- **`header -> list[str]`**: List of column header strings from the first row.
- **`extract() -> list[list[str]]`**: Returns all rows and cells as a 2D list of strings.
- **`to_markdown() -> str`**: Formats the table as a GitHub-Flavored Markdown table.
- **`to_dicts() -> list[dict[str, str]]`**: Maps each data row to a dictionary keyed by `header`.

---

## `Pixmap`

RGBA8888 pixel buffer supporting Python's zero-copy buffer protocol (`memoryview(pix)`, `np.frombuffer(pix, dtype=np.uint8)`).

- **`width -> int`**, **`height -> int`**, **`n -> int`** (`4` for RGBA), **`stride -> int`**, **`samples -> bytes`**.
- **`pixel(x: int, y: int) -> tuple[int, int, int, int]`**: Returns `(r, g, b, a)` at `(x, y)`.
- **`to_png() -> bytes`** / **`to_bytes(output: str = "png") -> bytes`** / **`tobytes(output: str = "png") -> bytes`**: Encodes as `"png"` or `"ppm"`.
- **`save(filename: str | PathLike) -> None`** / **`save_png(filename: str | PathLike) -> None`**: Saves the image to disk.

---

## Geometry Primitives (`Point`, `Rect`, `Matrix`)

- **`Point(x: float = 0.0, y: float = 0.0)`**: 2D coordinate with `.x`, `.y`, `.distance_to(other)`, indexing, and tuple unpacking.
- **`Rect(x0: float = 0.0, y0: float = 0.0, x1: float = 0.0, y1: float = 0.0)`**: Axis-aligned bounding box with `.width`, `.height`, `.area`, `.is_empty`, `.normalize()`, `.contains(pt)`, `.intersects(other)`, `.intersect(other)`, `.union(other)`, `.to_tuple()`.
- **`Matrix(a=1.0, b=0.0, c=0.0, d=1.0, e=0.0, f=0.0)`**: 3x3 affine transformation matrix with `.multiply(other)` and `.transform_point(pt)`.
