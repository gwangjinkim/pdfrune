#' Get Annotations on a PDF Page
#'
#' @param doc A `pdfrune_document` object.
#' @param page 1-based page index. Default is `1L`.
#' @return A data.frame containing annotation records (`xref`, `type`, `x0`, `y0`, `x1`, `y1`, `contents`, `color`).
#' @export
#' @examples
#' doc <- pdf_new()
#' pdf_new_page(doc)
#' pdf_annots(doc, 1L)
pdf_annots <- function(doc, page = 1L) {
  stopifnot(inherits(doc, "pdfrune_document"))
  stopifnot(page >= 1L)
  raw_json <- .Call(C_pdfrune_get_annots_json, doc$ptr, as.integer(page - 1L))
  df <- jsonlite::fromJSON(raw_json)
  if (length(df) == 0 || nrow(df) == 0) {
    return(data.frame(
      xref = integer(0), type = character(0),
      x0 = numeric(0), y0 = numeric(0), x1 = numeric(0), y1 = numeric(0),
      contents = character(0), overlay_text = character(0), stringsAsFactors = FALSE
    ))
  }
  as.data.frame(df)
}

#' Add a Highlight Annotation to a PDF Page
#'
#' @param doc A `pdfrune_document` object.
#' @param page 1-based page index. Default is `1L`.
#' @param rect Numeric vector `c(x0, y0, x1, y1)`.
#' @return The indirect object number (`xref`) of the created annotation.
#' @export
#' @examples
#' doc <- pdf_new()
#' pdf_new_page(doc)
#' pdf_add_highlight_annot(doc, 1L, c(72, 72, 200, 90))
pdf_add_highlight_annot <- function(doc, page = 1L, rect) {
  stopifnot(inherits(doc, "pdfrune_document") && length(rect) == 4)
  .Call(C_pdfrune_add_annot, doc$ptr, as.integer(page - 1L), "Highlight", as.numeric(rect), "", c(1, 1, 0), "")
}

#' Add a Text (Sticky Note) Annotation to a PDF Page
#'
#' @param doc A `pdfrune_document` object.
#' @param page 1-based page index. Default is `1L`.
#' @param point Numeric vector `c(x, y)` specifying top-left position.
#' @param text Character string note contents.
#' @return The indirect object number (`xref`) of the created annotation.
#' @export
#' @examples
#' doc <- pdf_new()
#' pdf_new_page(doc)
#' pdf_add_text_annot(doc, 1L, c(100, 100), "Review completed")
pdf_add_text_annot <- function(doc, page = 1L, point, text) {
  stopifnot(inherits(doc, "pdfrune_document") && length(point) == 2)
  rect <- c(point[1], point[2], point[1] + 20, point[2] + 20)
  .Call(C_pdfrune_add_annot, doc$ptr, as.integer(page - 1L), "Text", as.numeric(rect), as.character(text), c(1, 0.9, 0.2), "")
}

#' Add a Rectangle Annotation to a PDF Page
#'
#' @param doc A `pdfrune_document` object.
#' @param page 1-based page index. Default is `1L`.
#' @param rect Numeric vector `c(x0, y0, x1, y1)`.
#' @param color Numeric RGB vector `c(r, g, b)` between 0 and 1. Default is red `c(1, 0, 0)`.
#' @return The indirect object number (`xref`) of the created annotation.
#' @export
#' @examples
#' doc <- pdf_new()
#' pdf_new_page(doc)
#' pdf_add_rect_annot(doc, 1L, c(50, 50, 200, 150))
pdf_add_rect_annot <- function(doc, page = 1L, rect, color = c(1, 0, 0)) {
  stopifnot(inherits(doc, "pdfrune_document") && length(rect) == 4 && length(color) == 3)
  .Call(C_pdfrune_add_annot, doc$ptr, as.integer(page - 1L), "Square", as.numeric(rect), "", as.numeric(color), "")
}

#' Delete an Annotation from a PDF Page
#'
#' @param doc A `pdfrune_document` object.
#' @param page 1-based page index. Default is `1L`.
#' @param xref Indirect object number of the annotation to delete.
#' @return The modified `pdfrune_document` (invisibly).
#' @export
#' @examples
#' doc <- pdf_new()
#' pdf_new_page(doc)
#' xref <- pdf_add_highlight_annot(doc, 1L, c(72, 72, 200, 90))
#' pdf_delete_annot(doc, 1L, xref)
pdf_delete_annot <- function(doc, page = 1L, xref) {
  stopifnot(inherits(doc, "pdfrune_document"))
  .Call(C_pdfrune_delete_annot, doc$ptr, as.integer(page - 1L), as.integer(xref))
  invisible(doc)
}

#' Add a Redaction Annotation to a PDF Page
#'
#' Marks a rectangle for true permanent content-stream redaction. Call
#' `pdf_apply_redactions(doc, page)` to permanently scrub underlying text and apply visual fill.
#'
#' @param doc A `pdfrune_document` object.
#' @param page 1-based page index. Default is `1L`.
#' @param rect Numeric vector `c(x0, y0, x1, y1)`.
#' @param text Optional overlay label drawn over the redacted box (e.g. `"[REDACTED]"`).
#' @param fill Numeric RGB vector `c(r, g, b)` for the redaction fill box. Default is black `c(0, 0, 0)`.
#' @return The indirect object number (`xref`) of the redaction annotation.
#' @export
#' @examples
#' doc <- pdf_new()
#' pdf_new_page(doc)
#' pdf_insert_text(doc, 1L, c(72, 100), "SSN: 123-45-6789")
#' pdf_add_redact_annot(doc, 1L, c(70, 85, 250, 115), text = "[CONFIDENTIAL]")
pdf_add_redact_annot <- function(doc, page = 1L, rect, text = NULL, fill = c(0, 0, 0)) {
  stopifnot(inherits(doc, "pdfrune_document") && length(rect) == 4 && length(fill) == 3)
  txt <- if (is.null(text)) "" else as.character(text)
  .Call(C_pdfrune_add_annot, doc$ptr, as.integer(page - 1L), "Redact", as.numeric(rect), "", as.numeric(fill), txt)
}

#' Apply Pending Redactions on a PDF Page
#'
#' Permanently removes all text operators intersecting any `/Redact` annotation from the page's
#' underlying content stream, paints the redaction fill box and overlay label, and deletes the redaction annotation.
#'
#' @param doc A `pdfrune_document` object.
#' @param page 1-based page index. Default is `1L`.
#' @return An integer count of redactions permanently applied.
#' @export
#' @examples
#' doc <- pdf_new()
#' pdf_new_page(doc)
#' pdf_insert_text(doc, 1L, c(72, 100), "Confidential Secret")
#' pdf_add_redact_annot(doc, 1L, c(70, 85, 250, 115))
#' pdf_apply_redactions(doc, 1L)
#' !grepl("Confidential", pdf_extract_text(doc, 1L))
pdf_apply_redactions <- function(doc, page = 1L) {
  stopifnot(inherits(doc, "pdfrune_document"))
  .Call(C_pdfrune_apply_redactions, doc$ptr, as.integer(page - 1L))
}

#' Get Interactive AcroForm Widgets on a PDF Page
#'
#' @param doc A `pdfrune_document` object.
#' @param page 1-based page index. Default is `1L`.
#' @return A data.frame containing form field records (`xref`, `field_name`, `field_type`, `field_value`, `x0`, `y0`, `x1`, `y1`).
#' @export
#' @examples
#' doc <- pdf_new()
#' pdf_new_page(doc)
#' pdf_widgets(doc, 1L)
pdf_widgets <- function(doc, page = 1L) {
  stopifnot(inherits(doc, "pdfrune_document"))
  raw_json <- .Call(C_pdfrune_get_widgets_json, doc$ptr, as.integer(page - 1L))
  df <- jsonlite::fromJSON(raw_json)
  if (length(df) == 0 || nrow(df) == 0) {
    return(data.frame(
      xref = integer(0), field_name = character(0), field_type = character(0),
      field_value = character(0), x0 = numeric(0), y0 = numeric(0), x1 = numeric(0), y1 = numeric(0),
      stringsAsFactors = FALSE
    ))
  }
  as.data.frame(df)
}

#' Add an Interactive AcroForm Field to a PDF Page
#'
#' @param doc A `pdfrune_document` object.
#' @param page 1-based page index. Default is `1L`.
#' @param field_name Character string unique field identifier (e.g. `"full_name"`).
#' @param rect Numeric vector `c(x0, y0, x1, y1)`.
#' @param field_type Field type; `"text"`, `"checkbox"`, `"combobox"`, or `"listbox"`. Default is `"text"`.
#' @param field_value Initial field value. Default is `""`.
#' @return The indirect object number (`xref`) of the created widget.
#' @export
#' @examples
#' doc <- pdf_new()
#' pdf_new_page(doc)
#' pdf_add_widget(doc, 1L, "user_name", c(72, 100, 250, 125), field_value = "Ada")
pdf_add_widget <- function(doc, page = 1L, field_name, rect, field_type = "text", field_value = "") {
  stopifnot(inherits(doc, "pdfrune_document") && length(rect) == 4)
  .Call(C_pdfrune_add_widget, doc$ptr, as.integer(page - 1L), as.character(field_name), as.character(field_type), as.numeric(rect), as.character(field_value))
}

#' Set the Value of an AcroForm Field
#'
#' Updates the value and regenerates the visual appearance stream of a form field.
#'
#' @param doc A `pdfrune_document` object.
#' @param page 1-based page index. Default is `1L`.
#' @param field_name Name of the form field to update.
#' @param value New value character string.
#' @return Logical; `TRUE` if the field was found and updated, `FALSE` otherwise.
#' @export
#' @examples
#' doc <- pdf_new()
#' pdf_new_page(doc)
#' pdf_add_widget(doc, 1L, "user_name", c(72, 100, 250, 125), field_value = "Ada")
#' pdf_set_field_value(doc, 1L, "user_name", "Grace")
pdf_set_field_value <- function(doc, page = 1L, field_name, value) {
  stopifnot(inherits(doc, "pdfrune_document"))
  .Call(C_pdfrune_set_field_value, doc$ptr, as.integer(page - 1L), as.character(field_name), as.character(value))
}

#' Draw Text onto a PDF Page
#'
#' @param doc A `pdfrune_document` object.
#' @param page 1-based page index. Default is `1L`.
#' @param point Numeric vector `c(x, y)` specifying top-down origin.
#' @param text Character string text to insert.
#' @param fontsize Font size in points. Default is `12.0`.
#' @param color Numeric RGB vector `c(r, g, b)` between 0 and 1. Default is black `c(0, 0, 0)`.
#' @return The modified `pdfrune_document` (invisibly).
#' @export
#' @examples
#' doc <- pdf_new()
#' pdf_new_page(doc)
#' pdf_insert_text(doc, 1L, c(72, 72), "Hello from R!", fontsize = 18)
pdf_insert_text <- function(doc, page = 1L, point, text, fontsize = 12.0, color = c(0, 0, 0)) {
  stopifnot(inherits(doc, "pdfrune_document") && length(point) == 2 && length(color) == 3)
  .Call(C_pdfrune_insert_text, doc$ptr, as.integer(page - 1L), as.numeric(point[1]), as.numeric(point[2]), as.character(text), as.numeric(fontsize), as.numeric(color))
  invisible(doc)
}

#' Draw a Rectangle onto a PDF Page
#'
#' @param doc A `pdfrune_document` object.
#' @param page 1-based page index. Default is `1L`.
#' @param rect Numeric vector `c(x0, y0, x1, y1)`.
#' @param color Stroke RGB vector `c(r, g, b)`. Default is black `c(0, 0, 0)`. Set to `NULL` for no stroke.
#' @param fill Optional fill RGB vector `c(r, g, b)`. Default is `NULL` (no fill).
#' @param width Stroke line width. Default is `1.0`.
#' @return The modified `pdfrune_document` (invisibly).
#' @export
#' @examples
#' doc <- pdf_new()
#' pdf_new_page(doc)
#' pdf_draw_rect(doc, 1L, c(72, 100, 300, 180), color = c(0, 0.2, 0.8), fill = c(0.9, 0.95, 1.0))
pdf_draw_rect <- function(doc, page = 1L, rect, color = c(0, 0, 0), fill = NULL, width = 1.0) {
  stopifnot(inherits(doc, "pdfrune_document") && length(rect) == 4)
  clr <- if (is.null(color)) numeric(0) else as.numeric(color)
  fll <- if (is.null(fill)) numeric(0) else as.numeric(fill)
  .Call(C_pdfrune_draw_rect, doc$ptr, as.integer(page - 1L), as.numeric(rect), clr, fll, as.numeric(width))
  invisible(doc)
}

#' Get All Images Referenced on a PDF Page
#'
#' @param doc A `pdfrune_document` object.
#' @param page 1-based page index. Default is `1L`.
#' @return A data.frame of image metadata (`xref`, `smask`, `width`, `height`, `bpc`, `colorspace`, `name`, `filter`).
#' @export
#' @examples
#' doc <- pdf_new()
#' pdf_new_page(doc)
#' pdf_get_images(doc, 1L)
pdf_get_images <- function(doc, page = 1L) {
  stopifnot(inherits(doc, "pdfrune_document"))
  raw_json <- .Call(C_pdfrune_get_images_json, doc$ptr, as.integer(page - 1L))
  df <- jsonlite::fromJSON(raw_json)
  if (length(df) == 0 || nrow(df) == 0) {
    return(data.frame(
      xref = integer(0), smask = integer(0), width = integer(0), height = integer(0),
      bpc = integer(0), colorspace = character(0), name = character(0), filter = character(0),
      stringsAsFactors = FALSE
    ))
  }
  as.data.frame(df)
}

#' Extract an Embedded Image by Indirect Object Number (xref)
#'
#' @param doc A `pdfrune_document` object.
#' @param xref Indirect object number of the image.
#' @return A list with fields `width`, `height`, `ext` (`"png"` or `"jpeg"`), and `image` (raw bytes).
#' @export
#' @examples
#' doc <- pdf_new()
#' pdf_new_page(doc)
pdf_extract_image <- function(doc, xref) {
  stopifnot(inherits(doc, "pdfrune_document"))
  .Call(C_pdfrune_extract_image_bytes, doc$ptr, as.integer(xref))
}

#' Insert an Image onto a PDF Page
#'
#' @param doc A `pdfrune_document` object.
#' @param page 1-based page index. Default is `1L`.
#' @param rect Numeric vector `c(x0, y0, x1, y1)` where the image will be placed.
#' @param filename File path to a PNG or JPEG image. Optional if `bytes` is provided.
#' @param bytes Raw vector of PNG or JPEG image bytes. Optional if `filename` is provided.
#' @return The indirect object number (`xref`) of the created `/Image` XObject.
#' @export
#' @examples
#' doc <- pdf_new()
#' pdf_new_page(doc)
pdf_insert_image <- function(doc, page = 1L, rect, filename = NULL, bytes = NULL) {
  stopifnot(inherits(doc, "pdfrune_document") && length(rect) == 4)
  if (!is.null(filename)) {
    if (!file.exists(filename)) {
      stop("Image file does not exist: ", filename, call. = FALSE)
    }
    raw <- readBin(filename, "raw", file.info(filename)$size)
  } else if (!is.null(bytes)) {
    raw <- bytes
  } else {
    stop("Must provide either `filename` or `bytes`", call. = FALSE)
  }
  .Call(C_pdfrune_insert_image_bytes, doc$ptr, as.integer(page - 1L), as.numeric(rect), raw)
}
