#' Get the Table of Contents (Outlines / Bookmarks) of a PDF Document
#'
#' @param doc A `pdfrune_document` object.
#' @return A data.frame with columns `level` (integer hierarchy level), `title` (character bookmark title), and `page` (1-based destination page).
#' @export
#' @examples
#' doc <- pdf_new()
#' pdf_new_page(doc)
#' pdf_get_toc(doc)
pdf_get_toc <- function(doc) {
  stopifnot(inherits(doc, "pdfrune_document"))
  raw_json <- .Call(C_pdfrune_get_toc_json, doc$ptr)
  df <- jsonlite::fromJSON(raw_json)
  if (length(df) == 0 || nrow(df) == 0) {
    return(data.frame(level = integer(0), title = character(0), page = integer(0), stringsAsFactors = FALSE))
  }
  as.data.frame(df)
}

#' Set the Table of Contents (Outlines / Bookmarks) of a PDF Document
#'
#' @param doc A `pdfrune_document` object.
#' @param toc A data.frame or list of bookmarks. If a data.frame, must have columns `level`, `title`, `page`.
#' @return The modified `pdfrune_document` (invisibly).
#' @export
#' @examples
#' doc <- pdf_new()
#' pdf_new_page(doc)
#' pdf_new_page(doc)
#' toc <- data.frame(level = c(1L, 2L), title = c("Chapter 1", "Section 1.1"), page = c(1L, 2L))
#' pdf_set_toc(doc, toc)
pdf_set_toc <- function(doc, toc) {
  stopifnot(inherits(doc, "pdfrune_document"))
  if (is.data.frame(toc)) {
    if (!all(c("level", "title", "page") %in% names(toc))) {
      stop("`toc` data.frame must contain columns: `level`, `title`, `page`", call. = FALSE)
    }
  }
  json_str <- jsonlite::toJSON(toc)
  .Call(C_pdfrune_set_toc_json, doc$ptr, as.character(json_str))
  invisible(doc)
}

#' Get Hyperlinks on a PDF Page
#'
#' @param doc A `pdfrune_document` object.
#' @param page 1-based page index. Default is `1L`.
#' @return A data.frame containing link definitions (`kind`, `uri`, `page`, `x0`, `y0`, `x1`, `y1`, `xref`).
#' @export
#' @examples
#' doc <- pdf_new()
#' pdf_new_page(doc)
#' pdf_get_links(doc, 1L)
pdf_get_links <- function(doc, page = 1L) {
  stopifnot(inherits(doc, "pdfrune_document"))
  stopifnot(page >= 1L)
  raw_json <- .Call(C_pdfrune_get_links_json, doc$ptr, as.integer(page - 1L))
  df <- jsonlite::fromJSON(raw_json)
  if (length(df) == 0 || nrow(df) == 0) {
    return(data.frame(
      kind = character(0), uri = character(0), page = integer(0),
      x0 = numeric(0), y0 = numeric(0), x1 = numeric(0), y1 = numeric(0),
      xref = integer(0), stringsAsFactors = FALSE
    ))
  }
  as.data.frame(df)
}

#' Insert a Hyperlink onto a PDF Page
#'
#' @param doc A `pdfrune_document` object.
#' @param page 1-based page index. Default is `1L`.
#' @param rect Numeric vector of length 4 `c(x0, y0, x1, y1)` specifying the link clickable area.
#' @param uri Optional character string destination for external web links (e.g. `"https://example.com"`).
#' @param dest_page Optional 1-based page number for internal jump links.
#' @return The modified `pdfrune_document` (invisibly).
#' @export
#' @examples
#' doc <- pdf_new()
#' pdf_new_page(doc)
#' pdf_insert_link(doc, 1L, c(72, 100, 250, 120), uri = "https://cran.r-project.org")
pdf_insert_link <- function(doc, page = 1L, rect, uri = NULL, dest_page = NULL) {
  stopifnot(inherits(doc, "pdfrune_document"))
  stopifnot(page >= 1L && length(rect) == 4)
  dest <- if (is.null(dest_page)) 0L else as.integer(dest_page)
  u <- if (is.null(uri)) "" else as.character(uri)
  .Call(C_pdfrune_insert_link, doc$ptr, as.integer(page - 1L), as.numeric(rect), u, dest)
  invisible(doc)
}

#' Delete a Hyperlink from a PDF Page
#'
#' @param doc A `pdfrune_document` object.
#' @param page 1-based page index. Default is `1L`.
#' @param xref Optional integer indirect object number of the link annotation.
#' @param rect Optional numeric vector `c(x0, y0, x1, y1)`.
#' @return The modified `pdfrune_document` (invisibly).
#' @export
#' @examples
#' doc <- pdf_new()
#' pdf_new_page(doc)
#' pdf_insert_link(doc, 1L, c(72, 100, 250, 120), uri = "https://example.com")
#' pdf_delete_link(doc, 1L, rect = c(72, 100, 250, 120))
pdf_delete_link <- function(doc, page = 1L, xref = NULL, rect = NULL) {
  stopifnot(inherits(doc, "pdfrune_document"))
  stopifnot(page >= 1L)
  xr <- if (is.null(xref)) 0L else as.integer(xref)
  r <- if (is.null(rect)) numeric(0) else as.numeric(rect)
  .Call(C_pdfrune_delete_link, doc$ptr, as.integer(page - 1L), xr, r)
  invisible(doc)
}
