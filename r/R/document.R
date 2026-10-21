#' Open or Create a PDF Document
#'
#' Opens an existing PDF from a file path or raw vector, or creates a new empty PDF.
#'
#' @param path Character string specifying the path to a PDF file on disk. Optional if `bytes` is provided.
#' @param bytes Raw vector containing the in-memory PDF data. Optional if `path` is provided.
#' @return An object of class `pdfrune_document`.
#' @export
#' @examples
#' doc <- pdf_new()
#' pdf_page_count(doc)
pdf_open <- function(path = NULL, bytes = NULL) {
  if (is.null(path) && is.null(bytes)) {
    ptr <- .Call(C_pdfrune_new)
  } else if (!is.null(path)) {
    if (!file.exists(path)) {
      stop("File does not exist: ", path, call. = FALSE)
    }
    raw <- readBin(path, "raw", file.info(path)$size)
    ptr <- .Call(C_pdfrune_open, raw)
  } else if (!is.null(bytes)) {
    if (!is.raw(bytes)) {
      stop("`bytes` must be a raw vector", call. = FALSE)
    }
    ptr <- .Call(C_pdfrune_open, bytes)
  } else {
    stop("Must provide either `path` or `bytes`", call. = FALSE)
  }

  structure(
    list(ptr = ptr, path = path),
    class = "pdfrune_document"
  )
}

#' Create a New In-Memory PDF Document
#'
#' @return An empty `pdfrune_document`.
#' @export
#' @examples
#' doc <- pdf_new()
pdf_new <- function() {
  pdf_open()
}

#' Get the Number of Pages in a PDF Document
#'
#' @param doc A `pdfrune_document` object.
#' @return An integer count of pages.
#' @export
#' @examples
#' doc <- pdf_new()
#' pdf_new_page(doc)
#' pdf_page_count(doc)
pdf_page_count <- function(doc) {
  stopifnot(inherits(doc, "pdfrune_document"))
  .Call(C_pdfrune_page_count, doc$ptr)
}

#' Get PDF Document Metadata
#'
#' @param doc A `pdfrune_document` object.
#' @return A named list of metadata fields (e.g. title, author, subject).
#' @export
#' @examples
#' doc <- pdf_new()
#' pdf_metadata(doc)
pdf_metadata <- function(doc) {
  stopifnot(inherits(doc, "pdfrune_document"))
  raw_json <- .Call(C_pdfrune_get_metadata_json, doc$ptr)
  jsonlite::fromJSON(raw_json)
}

#' Set PDF Document Metadata
#'
#' @param doc A `pdfrune_document` object.
#' @param metadata A named list or named character vector of metadata key-value pairs.
#' @return The modified `pdfrune_document` (invisibly).
#' @export
#' @examples
#' doc <- pdf_new()
#' pdf_set_metadata(doc, list(title = "My Document", author = "Gwang-Jin Kim"))
pdf_set_metadata <- function(doc, metadata) {
  stopifnot(inherits(doc, "pdfrune_document"))
  if (!is.list(metadata) && !is.character(metadata)) {
    stop("`metadata` must be a named list or named character vector", call. = FALSE)
  }
  keys <- names(metadata)
  if (is.null(keys) || any(keys == "")) {
    stop("All metadata entries must have names", call. = FALSE)
  }
  for (k in keys) {
    v <- as.character(metadata[[k]])
    .Call(C_pdfrune_set_metadata, doc$ptr, as.character(k), v)
  }
  invisible(doc)
}

#' Check if a PDF Document is Encrypted
#'
#' @param doc A `pdfrune_document` object.
#' @return A logical value (`TRUE` if encrypted, `FALSE` otherwise).
#' @export
#' @examples
#' doc <- pdf_new()
#' pdf_is_encrypted(doc)
pdf_is_encrypted <- function(doc) {
  stopifnot(inherits(doc, "pdfrune_document"))
  .Call(C_pdfrune_is_encrypted, doc$ptr)
}

#' Authenticate an Encrypted PDF Document
#'
#' @param doc A `pdfrune_document` object.
#' @param password A character string containing the user or owner password.
#' @return A logical value (`TRUE` if authentication succeeded, `FALSE` otherwise).
#' @export
#' @examples
#' doc <- pdf_new()
#' pdf_authenticate(doc, "test")
pdf_authenticate <- function(doc, password) {
  stopifnot(inherits(doc, "pdfrune_document"))
  .Call(C_pdfrune_authenticate, doc$ptr, as.character(password))
}

#' Add a New Page to a PDF Document
#'
#' @param doc A `pdfrune_document` object.
#' @param pno 1-based page position to insert at (`-1L` appends to the end). Default is `-1L`.
#' @param width Width of the page in points (1/72 inch). Default is `612.0` (Letter width).
#' @param height Height of the page in points (1/72 inch). Default is `792.0` (Letter height).
#' @return The 1-based index of the newly created page.
#' @export
#' @examples
#' doc <- pdf_new()
#' pdf_new_page(doc, width = 612, height = 792)
pdf_new_page <- function(doc, pno = -1L, width = 612.0, height = 792.0) {
  stopifnot(inherits(doc, "pdfrune_document"))
  pno_c <- if (pno < 1L) -1L else as.integer(pno - 1L)
  idx <- .Call(C_pdfrune_new_page, doc$ptr, pno_c, as.numeric(width), as.numeric(height))
  as.integer(idx + 1L)
}

#' Delete a Page from a PDF Document
#'
#' @param doc A `pdfrune_document` object.
#' @param page 1-based page index to delete.
#' @return The modified `pdfrune_document` (invisibly).
#' @export
#' @examples
#' doc <- pdf_new()
#' pdf_new_page(doc)
#' pdf_new_page(doc)
#' pdf_delete_page(doc, 1L)
pdf_delete_page <- function(doc, page) {
  stopifnot(inherits(doc, "pdfrune_document"))
  stopifnot(page >= 1L)
  .Call(C_pdfrune_delete_page, doc$ptr, as.integer(page - 1L))
  invisible(doc)
}

#' Retrieve Information About a PDF Page
#'
#' @param doc A `pdfrune_document` object.
#' @param page 1-based page index. Default is `1L`.
#' @return A list with page dimensions (`width`, `height`), `rotation`, `mediabox`, and `cropbox`.
#' @export
#' @examples
#' doc <- pdf_new()
#' pdf_new_page(doc, width = 612, height = 792)
#' pdf_page_info(doc, 1L)
pdf_page_info <- function(doc, page = 1L) {
  stopifnot(inherits(doc, "pdfrune_document"))
  stopifnot(page >= 1L)
  raw_json <- .Call(C_pdfrune_page_info_json, doc$ptr, as.integer(page - 1L))
  jsonlite::fromJSON(raw_json)
}

#' Serialize a PDF Document to Raw Bytes
#'
#' @param doc A `pdfrune_document` object.
#' @param deflate Logical; whether to compress streams using FlateDecode. Default is `TRUE`.
#' @param encryption Optional character string; set to `"rc4-128"` to encrypt. Default is `NULL`.
#' @param user_pw Optional user password string for encryption. Default is `NULL`.
#' @param owner_pw Optional owner password string for encryption. Default is `NULL`.
#' @return A raw vector containing valid ISO 32000 PDF data.
#' @export
#' @examples
#' doc <- pdf_new()
#' pdf_new_page(doc)
#' bytes <- pdf_to_bytes(doc)
#' length(bytes) > 0
pdf_to_bytes <- function(doc, deflate = TRUE, encryption = NULL, user_pw = NULL, owner_pw = NULL) {
  stopifnot(inherits(doc, "pdfrune_document"))
  encrypt_flag <- !is.null(encryption) || !is.null(user_pw) || !is.null(owner_pw)
  u <- if (is.null(user_pw)) "" else as.character(user_pw)
  o <- if (is.null(owner_pw)) u else as.character(owner_pw)
  .Call(C_pdfrune_to_bytes, doc$ptr, as.logical(deflate), as.logical(encrypt_flag), u, o)
}

#' Save a PDF Document to Disk
#'
#' @param doc A `pdfrune_document` object.
#' @param filename File path to save the PDF.
#' @param deflate Logical; whether to compress streams using FlateDecode. Default is `TRUE`.
#' @param encryption Optional character string; set to `"rc4-128"` to encrypt. Default is `NULL`.
#' @param user_pw Optional user password string for encryption. Default is `NULL`.
#' @param owner_pw Optional owner password string for encryption. Default is `NULL`.
#' @return The file path saved (invisibly).
#' @export
#' @examples
#' doc <- pdf_new()
#' pdf_new_page(doc)
#' tmp <- tempfile(fileext = ".pdf")
#' pdf_save(doc, tmp)
#' file.exists(tmp)
pdf_save <- function(doc, filename, deflate = TRUE, encryption = NULL, user_pw = NULL, owner_pw = NULL) {
  bytes <- pdf_to_bytes(doc, deflate = deflate, encryption = encryption, user_pw = user_pw, owner_pw = owner_pw)
  writeBin(bytes, filename)
  invisible(filename)
}

#' Merge Multiple PDF Files into One Document
#'
#' @param files A character vector of PDF file paths or a list of raw vectors.
#' @param output Optional file path to write the merged PDF. If `NULL`, returns a `pdfrune_document`.
#' @return If `output` is `NULL`, a merged `pdfrune_document`. Otherwise, `output` path (invisibly).
#' @export
#' @examples
#' doc1 <- pdf_new()
#' pdf_new_page(doc1)
#' f1 <- tempfile(fileext = ".pdf")
#' pdf_save(doc1, f1)
#'
#' doc2 <- pdf_new()
#' pdf_new_page(doc2)
#' f2 <- tempfile(fileext = ".pdf")
#' pdf_save(doc2, f2)
#'
#' merged <- pdf_merge(c(f1, f2))
#' pdf_page_count(merged)
pdf_merge <- function(files, output = NULL) {
  raw_list <- lapply(files, function(f) {
    if (is.raw(f)) {
      f
    } else if (is.character(f) && file.exists(f)) {
      readBin(f, "raw", file.info(f)$size)
    } else {
      stop("Invalid file entry: ", f, call. = FALSE)
    }
  })
  merged_bytes <- .Call(C_pdfrune_merge_pdfs, raw_list)
  if (!is.null(output)) {
    writeBin(merged_bytes, output)
    invisible(output)
  } else {
    pdf_open(bytes = merged_bytes)
  }
}

#' Print Method for pdfrune Document
#'
#' @param x A `pdfrune_document` object.
#' @param ... Additional arguments (unused).
#' @export
print.pdfrune_document <- function(x, ...) {
  cnt <- pdf_page_count(x)
  enc <- pdf_is_encrypted(x)
  cat("<pdfrune_document>\n")
  cat("  Pages:     ", cnt, "\n", sep = "")
  cat("  Encrypted: ", enc, "\n", sep = "")
  if (!is.null(x$path)) {
    cat("  Path:      ", x$path, "\n", sep = "")
  }
  invisible(x)
}
