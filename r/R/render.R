#' Render a PDF Page to an RGBA Pixmap
#'
#' Rasterizes a PDF page at the specified resolution (DPI) into an RGBA8888 pixel buffer.
#'
#' @param doc A `pdfrune_document` object.
#' @param page 1-based page index. Default is `1L`.
#' @param dpi Resolution in dots per inch. Default is `72.0`.
#' @return An object of class `pdfrune_pixmap`.
#' @export
#' @examples
#' doc <- pdf_new()
#' pdf_new_page(doc)
#' pix <- pdf_render(doc, 1L, dpi = 72)
#' pix$width > 0
pdf_render <- function(doc, page = 1L, dpi = 72.0) {
  stopifnot(inherits(doc, "pdfrune_document"))
  stopifnot(page >= 1L)
  res <- .Call(C_pdfrune_render_page_rgba, doc$ptr, as.integer(page - 1L), as.numeric(dpi))

  pix <- structure(
    list(
      width = res$width,
      height = res$height,
      stride = res$stride,
      bytes = res$bytes,
      doc_ptr = doc$ptr,
      page = as.integer(page - 1L),
      dpi = as.numeric(dpi)
    ),
    class = "pdfrune_pixmap"
  )

  pix$to_png <- function() {
    .Call(C_pdfrune_render_page_png, doc$ptr, as.integer(page - 1L), as.numeric(dpi))
  }

  pix$save <- function(filename) {
    png_bytes <- pix$to_png()
    writeBin(png_bytes, filename)
    invisible(filename)
  }

  pix
}

#' Export a PDF Page as Vector SVG
#'
#' @param doc A `pdfrune_document` object.
#' @param page 1-based page index. Default is `1L`.
#' @return A character string containing standalone SVG XML.
#' @export
#' @examples
#' doc <- pdf_new()
#' pdf_new_page(doc)
#' svg <- pdf_to_svg(doc, 1L)
#' nchar(svg) > 0
pdf_to_svg <- function(doc, page = 1L) {
  stopifnot(inherits(doc, "pdfrune_document"))
  stopifnot(page >= 1L)
  .Call(C_pdfrune_page_to_svg, doc$ptr, as.integer(page - 1L))
}

#' Print Method for pdfrune Pixmap
#'
#' @param x A `pdfrune_pixmap` object.
#' @param ... Additional arguments (unused).
#' @export
print.pdfrune_pixmap <- function(x, ...) {
  cat("<pdfrune_pixmap: ", x$width, "x", x$height, " RGBA, ", length(x$bytes), " bytes>\n", sep = "")
  invisible(x)
}
