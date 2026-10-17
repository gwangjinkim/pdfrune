"""Drop-in PyMuPDF (`fitz`) compatibility layer powered by `rupdf`."""

import sys

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

# Register `rupdf.fitz` in `sys.modules` pointing to this exact module object
# so `import rupdf.fitz as fitz` and `from rupdf.compat import fitz` are identical.
sys.modules["rupdf.fitz"] = sys.modules[__name__]

__all__ = [
    "__version__",
    "version",
    "open",
    "merge",
    "batch_extract",
    "batch_render",
    "Document",
    "Page",
    "Pixmap",
    "Point",
    "Rect",
    "Matrix",
]
