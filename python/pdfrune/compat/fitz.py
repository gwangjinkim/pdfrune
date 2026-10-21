"""Drop-in PyMuPDF (`fitz`) compatibility layer powered by `pdfrune`."""

import sys

from pdfrune._pdfrune import (
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
    merge_pdfs as merge,
    open_document as open,
    version,
)

# Register `pdfrune.fitz` in `sys.modules` pointing to this exact module object
# so `import pdfrune.fitz as fitz` and `from pdfrune.compat import fitz` are identical.
sys.modules["pdfrune.fitz"] = sys.modules[__name__]

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
    "Table",
]
