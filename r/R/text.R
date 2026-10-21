#' Extract Plain Text from a PDF Document or Page
#'
#' @param doc A `pdfrune_document` object.
#' @param page Optional 1-based page index. If `NULL` (default), extracts and concatenates text across all pages.
#' @return A character vector containing the extracted plain text in visual reading order.
#' @export
#' @examples
#' doc <- pdf_new()
#' pdf_new_page(doc)
#' pdf_insert_text(doc, 1L, c(72, 72), "Hello from R!")
#' pdf_extract_text(doc, 1L)
pdf_extract_text <- function(doc, page = NULL) {
  stopifnot(inherits(doc, "pdfrune_document"))
  cnt <- pdf_page_count(doc)
  if (cnt == 0) return("")

  if (!is.null(page)) {
    stopifnot(page >= 1L && page <= cnt)
    .Call(C_pdfrune_extract_text, doc$ptr, as.integer(page - 1L))
  } else {
    vapply(seq_len(cnt), function(p) {
      .Call(C_pdfrune_extract_text, doc$ptr, as.integer(p - 1L))
    }, character(1))
  }
}

#' Extract Words with Bounding Boxes from a PDF Page
#'
#' @param doc A `pdfrune_document` object.
#' @param page 1-based page index. Default is `1L`.
#' @return A data.frame with columns `x0`, `y0`, `x1`, `y1`, `word`, `block_no`, `line_no`, `word_no`.
#' @export
#' @examples
#' doc <- pdf_new()
#' pdf_new_page(doc)
#' pdf_insert_text(doc, 1L, c(72, 72), "Quick brown fox")
#' pdf_extract_words(doc, 1L)
pdf_extract_words <- function(doc, page = 1L) {
  stopifnot(inherits(doc, "pdfrune_document"))
  stopifnot(page >= 1L)
  raw_json <- .Call(C_pdfrune_extract_words_json, doc$ptr, as.integer(page - 1L))
  res <- jsonlite::fromJSON(raw_json)
  if (length(res) == 0 || nrow(res) == 0) {
    return(data.frame(
      x0 = numeric(0), y0 = numeric(0), x1 = numeric(0), y1 = numeric(0),
      word = character(0), block_no = integer(0), line_no = integer(0), word_no = integer(0),
      stringsAsFactors = FALSE
    ))
  }
  as.data.frame(res)
}

#' Extract Paragraph Blocks with Bounding Boxes from a PDF Page
#'
#' @param doc A `pdfrune_document` object.
#' @param page 1-based page index. Default is `1L`.
#' @return A data.frame with columns `x0`, `y0`, `x1`, `y1`, `text`, `block_no`, `block_type`.
#' @export
#' @examples
#' doc <- pdf_new()
#' pdf_new_page(doc)
#' pdf_insert_text(doc, 1L, c(72, 72), "First paragraph")
#' pdf_extract_blocks(doc, 1L)
pdf_extract_blocks <- function(doc, page = 1L) {
  stopifnot(inherits(doc, "pdfrune_document"))
  stopifnot(page >= 1L)
  raw_json <- .Call(C_pdfrune_extract_blocks_json, doc$ptr, as.integer(page - 1L))
  res <- jsonlite::fromJSON(raw_json)
  if (length(res) == 0 || nrow(res) == 0) {
    return(data.frame(
      x0 = numeric(0), y0 = numeric(0), x1 = numeric(0), y1 = numeric(0),
      text = character(0), block_no = integer(0), block_type = integer(0),
      stringsAsFactors = FALSE
    ))
  }
  as.data.frame(res)
}

#' Search for Text Across a Page or Entire PDF Document
#'
#' @param doc A `pdfrune_document` object.
#' @param needle Character string to search for (substring or regex pattern).
#' @param page Optional 1-based page index. If `NULL` (default), searches across all pages in parallel.
#' @param regex Logical; if `TRUE`, interprets `needle` as a regular expression. Default is `FALSE`.
#' @return A data.frame with matching bounding box coordinates (`page`, `x0`, `y0`, `x1`, `y1`).
#' @export
#' @examples
#' doc <- pdf_new()
#' pdf_new_page(doc)
#' pdf_insert_text(doc, 1L, c(72, 72), "Revenue was 15M in 2026")
#' pdf_search(doc, "Revenue")
pdf_search <- function(doc, needle, page = NULL, regex = FALSE) {
  stopifnot(inherits(doc, "pdfrune_document"))
  needle <- as.character(needle)

  if (!is.null(page)) {
    stopifnot(page >= 1L)
    raw_json <- .Call(C_pdfrune_search_page_json, doc$ptr, as.integer(page - 1L), needle)
    df <- jsonlite::fromJSON(raw_json)
    if (length(df) == 0 || nrow(df) == 0) {
      return(data.frame(page = integer(0), x0 = numeric(0), y0 = numeric(0), x1 = numeric(0), y1 = numeric(0)))
    }
    df$page <- as.integer(page)
    df[, c("page", "x0", "y0", "x1", "y1")]
  } else {
    raw_json <- .Call(C_pdfrune_search_all_json, doc$ptr, needle, as.logical(regex))
    df <- jsonlite::fromJSON(raw_json)
    if (length(df) == 0 || nrow(df) == 0) {
      return(data.frame(page = integer(0), x0 = numeric(0), y0 = numeric(0), x1 = numeric(0), y1 = numeric(0)))
    }
    as.data.frame(df)
  }
}

#' Convert a PDF Page or Full Document to Structured Markdown
#'
#' @param doc A `pdfrune_document` object.
#' @param page Optional 1-based page index. If `NULL` (default), converts the full document.
#' @return A character string containing GitHub-Flavored Markdown.
#' @export
#' @examples
#' doc <- pdf_new()
#' pdf_new_page(doc)
#' pdf_insert_text(doc, 1L, c(72, 72), "Title Heading")
#' pdf_to_markdown(doc)
pdf_to_markdown <- function(doc, page = NULL) {
  stopifnot(inherits(doc, "pdfrune_document"))
  if (!is.null(page)) {
    stopifnot(page >= 1L)
    .Call(C_pdfrune_page_to_markdown, doc$ptr, as.integer(page - 1L))
  } else {
    .Call(C_pdfrune_doc_to_markdown, doc$ptr)
  }
}

#' Convert a PDF Page or Full Document to Semantic HTML
#'
#' @param doc A `pdfrune_document` object.
#' @param page Optional 1-based page index. If `NULL` (default), converts the full document.
#' @return A character string containing semantic HTML5.
#' @export
#' @examples
#' doc <- pdf_new()
#' pdf_new_page(doc)
#' pdf_insert_text(doc, 1L, c(72, 72), "Title Heading")
#' pdf_to_html(doc)
pdf_to_html <- function(doc, page = NULL) {
  stopifnot(inherits(doc, "pdfrune_document"))
  if (!is.null(page)) {
    stopifnot(page >= 1L)
    .Call(C_pdfrune_page_to_html, doc$ptr, as.integer(page - 1L))
  } else {
    .Call(C_pdfrune_doc_to_html, doc$ptr)
  }
}

#' Generate Semantic RAG Chunks from a PDF Document
#'
#' Produces semantically coherent text chunks coupled with accurate page numbers,
#' bounding box coordinates, and section headings for vector database grounding.
#'
#' @param doc A `pdfrune_document` object.
#' @param max_chars Maximum character length per chunk. Default is `500L`.
#' @return A data.frame with columns `chunk_id`, `page_number`, `x0`, `y0`, `x1`, `y1`, `section_heading`, `text`, `char_count`.
#' @export
#' @examples
#' doc <- pdf_new()
#' pdf_new_page(doc)
#' pdf_insert_text(doc, 1L, c(72, 72), "Executive Summary\nAll objectives achieved.")
#' pdf_to_rag_chunks(doc, max_chars = 300L)
pdf_to_rag_chunks <- function(doc, max_chars = 500L) {
  stopifnot(inherits(doc, "pdfrune_document"))
  raw_json <- .Call(C_pdfrune_to_rag_chunks_json, doc$ptr, as.integer(max_chars))
  df <- jsonlite::fromJSON(raw_json)
  if (length(df) == 0 || nrow(df) == 0) {
    return(data.frame(
      chunk_id = integer(0), page_number = integer(0),
      x0 = numeric(0), y0 = numeric(0), x1 = numeric(0), y1 = numeric(0),
      text = character(0), char_count = integer(0), section_heading = character(0),
      stringsAsFactors = FALSE
    ))
  }
  as.data.frame(df)
}
