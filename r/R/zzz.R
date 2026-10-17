#' @useDynLib rupdf, .registration = TRUE
#' @importFrom jsonlite fromJSON toJSON
NULL

#' Return rupdf Core Engine Version
#'
#' @return A character string representing the version of the underlying Rust engine.
#' @export
#' @examples
#' pdf_version()
pdf_version <- function() {
  .Call(C_rupdf_version)
}
