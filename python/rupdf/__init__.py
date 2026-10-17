"""rupdf: Ultra-Fast, Memory-Safe, Permissive PDF Engine in Rust & Python."""

from rupdf._rupdf import (
    Document,
    Matrix,
    Page,
    Point,
    Rect,
    __version__,
    merge_pdfs as merge,
    open_document as open,
    version,
)

__all__ = [
    "__version__",
    "version",
    "open",
    "merge",
    "Document",
    "Page",
    "Point",
    "Rect",
    "Matrix",
]
