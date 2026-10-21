#' Detect Structured Tables on a PDF Page
#'
#' Automatically detects multi-column tabular layouts on a page and extracts
#' headers, rows, cells, and bounding boxes.
#'
#' @param doc A `pdfrune_document` object.
#' @param page 1-based page index. Default is `1L`.
#' @return A list of `pdfrune_table` objects.
#' @export
#' @examples
#' doc <- pdf_new()
#' pdf_new_page(doc)
#' pdf_insert_text(doc, 1L, c(72, 100), "Quarter")
#' pdf_insert_text(doc, 1L, c(200, 100), "Revenue")
#' pdf_insert_text(doc, 1L, c(72, 130), "Q1-2026")
#' pdf_insert_text(doc, 1L, c(200, 130), "$10M")
#' tabs <- pdf_find_tables(doc, 1L)
#' length(tabs)
pdf_find_tables <- function(doc, page = 1L) {
  stopifnot(inherits(doc, "pdfrune_document"))
  stopifnot(page >= 1L)
  raw_json <- .Call(C_pdfrune_find_tables_json, doc$ptr, as.integer(page - 1L))
  parsed <- jsonlite::fromJSON(raw_json, simplifyVector = FALSE)
  if (length(parsed) == 0) {
    return(list())
  }

  lapply(parsed, function(t) {
    structure(
      list(
        row_count = t$row_count,
        col_count = t$col_count,
        bbox = unlist(t$bbox),
        header = unlist(t$header),
        rows = t$rows,
        markdown = t$markdown,
        extract = function() {
          t$rows
        },
        to_dataframe = function() {
          if (length(t$rows) <= 1) {
            return(data.frame())
          }
          cols <- as.character(unlist(t$header))
          mat <- do.call(rbind, lapply(t$rows[-1], unlist))
          df <- as.data.frame(mat, stringsAsFactors = FALSE)
          colnames(df) <- cols
          rownames(df) <- NULL
          df
        },
        to_markdown = function() {
          t$markdown
        }
      ),
      class = "pdfrune_table"
    )
  })
}

#' Print Method for pdfrune Table
#'
#' @param x A `pdfrune_table` object.
#' @param ... Additional arguments (unused).
#' @export
print.pdfrune_table <- function(x, ...) {
  cat("<pdfrune_table: ", x$row_count, " rows x ", x$col_count, " cols>\n", sep = "")
  cat("BBox: [", paste(round(x$bbox, 1), collapse = ", "), "]\n", sep = "")
  if (nzchar(x$markdown)) {
    cat(x$markdown, "\n")
  }
  invisible(x)
}
