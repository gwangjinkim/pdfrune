#ifndef PDFRUNE_C_H
#define PDFRUNE_C_H

#include <stddef.h>
#include <stdint.h>

// Document lifecycle
void* pdfrune_c_new(void);
void* pdfrune_c_open(const uint8_t* bytes, size_t len);
void pdfrune_c_free(void* doc);
char* pdfrune_c_version(void);
int32_t pdfrune_c_page_count(void* doc);
int32_t pdfrune_c_is_encrypted(void* doc);
int32_t pdfrune_c_authenticate(void* doc, const char* password);
char* pdfrune_c_get_metadata_json(void* doc);
int32_t pdfrune_c_set_metadata(void* doc, const char* key, const char* val);

// Page & Layout Extraction
char* pdfrune_c_page_info_json(void* doc, size_t page);
char* pdfrune_c_extract_text(void* doc, size_t page);
char* pdfrune_c_extract_words_json(void* doc, size_t page);
char* pdfrune_c_extract_blocks_json(void* doc, size_t page);
char* pdfrune_c_search_page_json(void* doc, size_t page, const char* needle);
char* pdfrune_c_search_all_json(void* doc, const char* needle, int32_t regex);
char* pdfrune_c_find_tables_json(void* doc, size_t page);
char* pdfrune_c_page_to_markdown(void* doc, size_t page);
char* pdfrune_c_doc_to_markdown(void* doc);
char* pdfrune_c_page_to_html(void* doc, size_t page);
char* pdfrune_c_doc_to_html(void* doc);
char* pdfrune_c_to_rag_chunks_json(void* doc, size_t max_chars);

// Rendering & Vector Export
char* pdfrune_c_page_to_svg(void* doc, size_t page);
uint8_t* pdfrune_c_render_page_rgba(void* doc, size_t page, double dpi, int32_t* out_w, int32_t* out_h, int32_t* out_stride, size_t* out_len);
uint8_t* pdfrune_c_render_page_png(void* doc, size_t page, double dpi, size_t* out_len);

// Outlines, Links, Images
char* pdfrune_c_get_toc_json(void* doc);
int32_t pdfrune_c_set_toc_json(void* doc, const char* json_str);
char* pdfrune_c_get_links_json(void* doc, size_t page);
int32_t pdfrune_c_insert_link(void* doc, size_t page, double x0, double y0, double x1, double y1, const char* uri, int32_t dest_page);
int32_t pdfrune_c_delete_link(void* doc, size_t page, uint32_t xref, double x0, double y0, double x1, double y1);
char* pdfrune_c_get_images_json(void* doc, size_t page);
uint8_t* pdfrune_c_extract_image_bytes(void* doc, uint32_t xref, int32_t* out_w, int32_t* out_h, char** out_ext, size_t* out_len);
int32_t pdfrune_c_insert_image_bytes(void* doc, size_t page, double x0, double y0, double x1, double y1, const uint8_t* bytes, size_t len);

// Annotations, Redactions, Widgets
char* pdfrune_c_get_annots_json(void* doc, size_t page);
uint32_t pdfrune_c_add_annot(void* doc, size_t page, const char* subtype, double x0, double y0, double x1, double y1, const char* contents, double r, double g, double b, const char* overlay);
int32_t pdfrune_c_delete_annot(void* doc, size_t page, uint32_t xref);
int32_t pdfrune_c_apply_redactions(void* doc, size_t page);
char* pdfrune_c_get_widgets_json(void* doc, size_t page);
uint32_t pdfrune_c_add_widget(void* doc, size_t page, const char* name, const char* field_type, double x0, double y0, double x1, double y1, const char* val);
int32_t pdfrune_c_set_field_value(void* doc, size_t page, const char* name, const char* val);

// Document Mutation & Saving
int32_t pdfrune_c_new_page(void* doc, int32_t pno, double width, double height);
int32_t pdfrune_c_delete_page(void* doc, int32_t pno);
int32_t pdfrune_c_insert_text(void* doc, size_t page, double x, double y, const char* text, double fontsize, double r, double g, double b);
int32_t pdfrune_c_draw_rect(void* doc, size_t page, double x0, double y0, double x1, double y1, int32_t has_stroke, double sr, double sg, double sb, int32_t has_fill, double fr, double fg, double fb, double width);
uint8_t* pdfrune_c_to_bytes(void* doc, int32_t deflate, int32_t encrypt, const char* user_pw, const char* owner_pw, size_t* out_len);
uint8_t* pdfrune_c_merge_pdfs(const uint8_t** docs_bytes, const size_t* docs_lens, size_t count, size_t* out_len);

// Memory free
void pdfrune_c_free_string(char* s);
void pdfrune_c_free_bytes(uint8_t* b, size_t len);

#endif // PDFRUNE_C_H
