#' @useDynLib pdfrune, .registration = TRUE
#' @importFrom jsonlite fromJSON toJSON
NULL

if (getRversion() >= "2.15.1") {
  utils::globalVariables(c(
    "C_pdfrune_version",
    "C_pdfrune_new",
    "C_pdfrune_open",
    "C_pdfrune_page_count",
    "C_pdfrune_is_encrypted",
    "C_pdfrune_authenticate",
    "C_pdfrune_get_metadata_json",
    "C_pdfrune_set_metadata",
    "C_pdfrune_page_info_json",
    "C_pdfrune_extract_text",
    "C_pdfrune_extract_words_json",
    "C_pdfrune_extract_blocks_json",
    "C_pdfrune_search_page_json",
    "C_pdfrune_search_all_json",
    "C_pdfrune_find_tables_json",
    "C_pdfrune_page_to_markdown",
    "C_pdfrune_doc_to_markdown",
    "C_pdfrune_page_to_html",
    "C_pdfrune_doc_to_html",
    "C_pdfrune_to_rag_chunks_json",
    "C_pdfrune_page_to_svg",
    "C_pdfrune_render_page_rgba",
    "C_pdfrune_render_page_png",
    "C_pdfrune_get_toc_json",
    "C_pdfrune_set_toc_json",
    "C_pdfrune_get_links_json",
    "C_pdfrune_insert_link",
    "C_pdfrune_delete_link",
    "C_pdfrune_get_images_json",
    "C_pdfrune_extract_image_bytes",
    "C_pdfrune_insert_image_bytes",
    "C_pdfrune_get_annots_json",
    "C_pdfrune_add_annot",
    "C_pdfrune_delete_annot",
    "C_pdfrune_apply_redactions",
    "C_pdfrune_get_widgets_json",
    "C_pdfrune_add_widget",
    "C_pdfrune_set_field_value",
    "C_pdfrune_new_page",
    "C_pdfrune_delete_page",
    "C_pdfrune_insert_text",
    "C_pdfrune_draw_rect",
    "C_pdfrune_to_bytes",
    "C_pdfrune_merge_pdfs"
  ))
}

#' Return pdfrune Core Engine Version
#'
#' @return A character string representing the version of the underlying Rust engine.
#' @export
#' @examples
#' pdf_version()
pdf_version <- function() {
  .Call(C_pdfrune_version)
}
