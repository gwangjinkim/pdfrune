#' @useDynLib rupdf, .registration = TRUE
#' @importFrom jsonlite fromJSON toJSON
NULL

if (getRversion() >= "2.15.1") {
  utils::globalVariables(c(
    "C_rupdf_version",
    "C_rupdf_new",
    "C_rupdf_open",
    "C_rupdf_page_count",
    "C_rupdf_is_encrypted",
    "C_rupdf_authenticate",
    "C_rupdf_get_metadata_json",
    "C_rupdf_set_metadata",
    "C_rupdf_page_info_json",
    "C_rupdf_extract_text",
    "C_rupdf_extract_words_json",
    "C_rupdf_extract_blocks_json",
    "C_rupdf_search_page_json",
    "C_rupdf_search_all_json",
    "C_rupdf_find_tables_json",
    "C_rupdf_page_to_markdown",
    "C_rupdf_doc_to_markdown",
    "C_rupdf_page_to_html",
    "C_rupdf_doc_to_html",
    "C_rupdf_to_rag_chunks_json",
    "C_rupdf_page_to_svg",
    "C_rupdf_render_page_rgba",
    "C_rupdf_render_page_png",
    "C_rupdf_get_toc_json",
    "C_rupdf_set_toc_json",
    "C_rupdf_get_links_json",
    "C_rupdf_insert_link",
    "C_rupdf_delete_link",
    "C_rupdf_get_images_json",
    "C_rupdf_extract_image_bytes",
    "C_rupdf_insert_image_bytes",
    "C_rupdf_get_annots_json",
    "C_rupdf_add_annot",
    "C_rupdf_delete_annot",
    "C_rupdf_apply_redactions",
    "C_rupdf_get_widgets_json",
    "C_rupdf_add_widget",
    "C_rupdf_set_field_value",
    "C_rupdf_new_page",
    "C_rupdf_delete_page",
    "C_rupdf_insert_text",
    "C_rupdf_draw_rect",
    "C_rupdf_to_bytes",
    "C_rupdf_merge_pdfs"
  ))
}

#' Return rupdf Core Engine Version
#'
#' @return A character string representing the version of the underlying Rust engine.
#' @export
#' @examples
#' pdf_version()
pdf_version <- function() {
  .Call(C_rupdf_version)
}
