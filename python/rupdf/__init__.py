"""rupdf: Ultra-Fast, Memory-Safe, Permissive PDF Engine in Rust & Python."""

from rupdf._rupdf import (
    Document,
    Matrix,
    Page,
    Pixmap,
    Point,
    Rect,
    __version__,
    batch_extract,
    batch_render,
    merge_pdfs as merge,
    open_document as open,
    version,
)
from rupdf.compat import fitz

__all__ = [
    "__version__",
    "version",
    "open",
    "merge",
    "batch_extract",
    "batch_render",
    "fitz",
    "Document",
    "Page",
    "Pixmap",
    "Point",
    "Rect",
    "Matrix",
]
