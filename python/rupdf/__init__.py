"""rupdf: Ultra-Fast, Memory-Safe, Permissive PDF Engine in Rust & Python."""

from rupdf._rupdf import (
    Document,
    Matrix,
    Page,
    Point,
    Rect,
    __version__,
    open_document as open,
    version,
)

__all__ = [
    "__version__",
    "version",
    "open",
    "Document",
    "Page",
    "Point",
    "Rect",
    "Matrix",
]
