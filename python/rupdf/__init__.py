"""rupdf: Ultra-Fast, Memory-Safe, Permissive PDF Engine in Rust & Python."""

from rupdf._rupdf import (
    Document,
    Matrix,
    Page,
    Pixmap,
    Point,
    Rect,
    Table,
    __version__,
    batch_extract,
    batch_render,
    benchmark,
    merge_pdfs as merge,
    open_document as open,
    version,
)
from rupdf.compat import fitz

__all__ = [
    "__version__",
    "version",
    "benchmark",
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
    "Table",
]
