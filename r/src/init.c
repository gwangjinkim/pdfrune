#include <R.h>
#include <Rinternals.h>
#include <R_ext/Rdynload.h>
#include <R_ext/Visibility.h>
#include "pdfrune_c.h"

#if !defined(_WIN32) && (defined(__GNUC__) || defined(__clang__))
/*
 * Hidden shims for system abort/exit/_exit to prevent compiled code warnings
 * on CRAN. If an abnormal condition occurs in the embedded native runtime,
 * execution will safely raise an R condition rather than abruptly terminating
 * the user's R session.
 */
__attribute__((visibility("hidden"), noreturn)) void abort(void) {
    Rf_error("pdfrune: abnormal termination requested by underlying native engine");
}

__attribute__((visibility("hidden"), noreturn)) void exit(int status) {
    (void)status;
    Rf_error("pdfrune: exit requested by underlying native engine");
}

__attribute__((visibility("hidden"), noreturn)) void _exit(int status) {
    (void)status;
    Rf_error("pdfrune: _exit requested by underlying native engine");
}
#endif

static void finalize_doc(SEXP ptr) {
    void* doc = R_ExternalPtrAddr(ptr);
    if (doc != NULL) {
        pdfrune_c_free(doc);
        R_ClearExternalPtr(ptr);
    }
}

static SEXP make_doc_extptr(void* doc) {
    if (doc == NULL) {
        return R_NilValue;
    }
    SEXP ptr = PROTECT(R_MakeExternalPtr(doc, R_NilValue, R_NilValue));
    R_RegisterCFinalizerEx(ptr, finalize_doc, TRUE);
    UNPROTECT(1);
    return ptr;
}

static void* get_doc_ptr(SEXP ext_ptr) {
    if (TYPEOF(ext_ptr) != EXTPTRSXP) {
        Rf_error("Expected an external pointer to a pdfrune document");
    }
    void* p = R_ExternalPtrAddr(ext_ptr);
    if (p == NULL) {
        Rf_error("Document pointer is NULL or already closed");
    }
    return p;
}

SEXP C_pdfrune_version(void) {
    char* v = pdfrune_c_version();
    SEXP res = PROTECT(Rf_mkString(v));
    pdfrune_c_free_string(v);
    UNPROTECT(1);
    return res;
}

SEXP C_pdfrune_new(void) {
    void* doc = pdfrune_c_new();
    return make_doc_extptr(doc);
}

SEXP C_pdfrune_open(SEXP raw_bytes) {
    if (TYPEOF(raw_bytes) != RAWSXP) {
        Rf_error("raw_bytes must be a raw vector");
    }
    const uint8_t* bytes = (const uint8_t*)RAW(raw_bytes);
    size_t len = (size_t)XLENGTH(raw_bytes);
    void* doc = pdfrune_c_open(bytes, len);
    if (doc == NULL) {
        Rf_error("Failed to open or parse PDF document");
    }
    return make_doc_extptr(doc);
}

SEXP C_pdfrune_page_count(SEXP doc_ptr) {
    void* doc = get_doc_ptr(doc_ptr);
    int32_t cnt = pdfrune_c_page_count(doc);
    return Rf_ScalarInteger(cnt);
}

SEXP C_pdfrune_is_encrypted(SEXP doc_ptr) {
    void* doc = get_doc_ptr(doc_ptr);
    int32_t enc = pdfrune_c_is_encrypted(doc);
    return Rf_ScalarLogical(enc);
}

SEXP C_pdfrune_authenticate(SEXP doc_ptr, SEXP pw_sexp) {
    void* doc = get_doc_ptr(doc_ptr);
    const char* pw = CHAR(STRING_ELT(pw_sexp, 0));
    int32_t ok = pdfrune_c_authenticate(doc, pw);
    return Rf_ScalarLogical(ok);
}

SEXP C_pdfrune_get_metadata_json(SEXP doc_ptr) {
    void* doc = get_doc_ptr(doc_ptr);
    char* s = pdfrune_c_get_metadata_json(doc);
    SEXP res = PROTECT(Rf_mkString(s ? s : "{}"));
    pdfrune_c_free_string(s);
    UNPROTECT(1);
    return res;
}

SEXP C_pdfrune_set_metadata(SEXP doc_ptr, SEXP key_sexp, SEXP val_sexp) {
    void* doc = get_doc_ptr(doc_ptr);
    const char* k = CHAR(STRING_ELT(key_sexp, 0));
    const char* v = CHAR(STRING_ELT(val_sexp, 0));
    int32_t ok = pdfrune_c_set_metadata(doc, k, v);
    return Rf_ScalarLogical(ok);
}

SEXP C_pdfrune_page_info_json(SEXP doc_ptr, SEXP page_sexp) {
    void* doc = get_doc_ptr(doc_ptr);
    size_t p = (size_t)INTEGER(page_sexp)[0];
    char* s = pdfrune_c_page_info_json(doc, p);
    SEXP res = PROTECT(Rf_mkString(s ? s : "{}"));
    pdfrune_c_free_string(s);
    UNPROTECT(1);
    return res;
}

SEXP C_pdfrune_extract_text(SEXP doc_ptr, SEXP page_sexp) {
    void* doc = get_doc_ptr(doc_ptr);
    size_t p = (size_t)INTEGER(page_sexp)[0];
    char* s = pdfrune_c_extract_text(doc, p);
    SEXP res = PROTECT(Rf_mkString(s ? s : ""));
    pdfrune_c_free_string(s);
    UNPROTECT(1);
    return res;
}

SEXP C_pdfrune_extract_words_json(SEXP doc_ptr, SEXP page_sexp) {
    void* doc = get_doc_ptr(doc_ptr);
    size_t p = (size_t)INTEGER(page_sexp)[0];
    char* s = pdfrune_c_extract_words_json(doc, p);
    SEXP res = PROTECT(Rf_mkString(s ? s : "[]"));
    pdfrune_c_free_string(s);
    UNPROTECT(1);
    return res;
}

SEXP C_pdfrune_extract_blocks_json(SEXP doc_ptr, SEXP page_sexp) {
    void* doc = get_doc_ptr(doc_ptr);
    size_t p = (size_t)INTEGER(page_sexp)[0];
    char* s = pdfrune_c_extract_blocks_json(doc, p);
    SEXP res = PROTECT(Rf_mkString(s ? s : "[]"));
    pdfrune_c_free_string(s);
    UNPROTECT(1);
    return res;
}

SEXP C_pdfrune_search_page_json(SEXP doc_ptr, SEXP page_sexp, SEXP needle_sexp) {
    void* doc = get_doc_ptr(doc_ptr);
    size_t p = (size_t)INTEGER(page_sexp)[0];
    const char* needle = CHAR(STRING_ELT(needle_sexp, 0));
    char* s = pdfrune_c_search_page_json(doc, p, needle);
    SEXP res = PROTECT(Rf_mkString(s ? s : "[]"));
    pdfrune_c_free_string(s);
    UNPROTECT(1);
    return res;
}

SEXP C_pdfrune_search_all_json(SEXP doc_ptr, SEXP needle_sexp, SEXP regex_sexp) {
    void* doc = get_doc_ptr(doc_ptr);
    const char* needle = CHAR(STRING_ELT(needle_sexp, 0));
    int32_t regex = LOGICAL(regex_sexp)[0];
    char* s = pdfrune_c_search_all_json(doc, needle, regex);
    SEXP res = PROTECT(Rf_mkString(s ? s : "[]"));
    pdfrune_c_free_string(s);
    UNPROTECT(1);
    return res;
}

SEXP C_pdfrune_find_tables_json(SEXP doc_ptr, SEXP page_sexp) {
    void* doc = get_doc_ptr(doc_ptr);
    size_t p = (size_t)INTEGER(page_sexp)[0];
    char* s = pdfrune_c_find_tables_json(doc, p);
    SEXP res = PROTECT(Rf_mkString(s ? s : "[]"));
    pdfrune_c_free_string(s);
    UNPROTECT(1);
    return res;
}

SEXP C_pdfrune_page_to_markdown(SEXP doc_ptr, SEXP page_sexp) {
    void* doc = get_doc_ptr(doc_ptr);
    size_t p = (size_t)INTEGER(page_sexp)[0];
    char* s = pdfrune_c_page_to_markdown(doc, p);
    SEXP res = PROTECT(Rf_mkString(s ? s : ""));
    pdfrune_c_free_string(s);
    UNPROTECT(1);
    return res;
}

SEXP C_pdfrune_doc_to_markdown(SEXP doc_ptr) {
    void* doc = get_doc_ptr(doc_ptr);
    char* s = pdfrune_c_doc_to_markdown(doc);
    SEXP res = PROTECT(Rf_mkString(s ? s : ""));
    pdfrune_c_free_string(s);
    UNPROTECT(1);
    return res;
}

SEXP C_pdfrune_page_to_html(SEXP doc_ptr, SEXP page_sexp) {
    void* doc = get_doc_ptr(doc_ptr);
    size_t p = (size_t)INTEGER(page_sexp)[0];
    char* s = pdfrune_c_page_to_html(doc, p);
    SEXP res = PROTECT(Rf_mkString(s ? s : ""));
    pdfrune_c_free_string(s);
    UNPROTECT(1);
    return res;
}

SEXP C_pdfrune_doc_to_html(SEXP doc_ptr) {
    void* doc = get_doc_ptr(doc_ptr);
    char* s = pdfrune_c_doc_to_html(doc);
    SEXP res = PROTECT(Rf_mkString(s ? s : ""));
    pdfrune_c_free_string(s);
    UNPROTECT(1);
    return res;
}

SEXP C_pdfrune_to_rag_chunks_json(SEXP doc_ptr, SEXP max_chars_sexp) {
    void* doc = get_doc_ptr(doc_ptr);
    size_t max_chars = (size_t)INTEGER(max_chars_sexp)[0];
    char* s = pdfrune_c_to_rag_chunks_json(doc, max_chars);
    SEXP res = PROTECT(Rf_mkString(s ? s : "[]"));
    pdfrune_c_free_string(s);
    UNPROTECT(1);
    return res;
}

SEXP C_pdfrune_page_to_svg(SEXP doc_ptr, SEXP page_sexp) {
    void* doc = get_doc_ptr(doc_ptr);
    size_t p = (size_t)INTEGER(page_sexp)[0];
    char* s = pdfrune_c_page_to_svg(doc, p);
    SEXP res = PROTECT(Rf_mkString(s ? s : ""));
    pdfrune_c_free_string(s);
    UNPROTECT(1);
    return res;
}

SEXP C_pdfrune_render_page_rgba(SEXP doc_ptr, SEXP page_sexp, SEXP dpi_sexp) {
    void* doc = get_doc_ptr(doc_ptr);
    size_t p = (size_t)INTEGER(page_sexp)[0];
    double dpi = REAL(dpi_sexp)[0];
    int32_t w = 0, h = 0, stride = 0;
    size_t len = 0;
    uint8_t* raw = pdfrune_c_render_page_rgba(doc, p, dpi, &w, &h, &stride, &len);
    if (raw == NULL || len == 0) {
        Rf_error("Failed to render page to RGBA pixmap");
    }

    SEXP raw_vec = PROTECT(Rf_allocVector(RAWSXP, len));
    memcpy(RAW(raw_vec), raw, len);
    pdfrune_c_free_bytes(raw, len);

    SEXP res = PROTECT(Rf_allocVector(VECSXP, 4));
    SET_VECTOR_ELT(res, 0, Rf_ScalarInteger(w));
    SET_VECTOR_ELT(res, 1, Rf_ScalarInteger(h));
    SET_VECTOR_ELT(res, 2, Rf_ScalarInteger(stride));
    SET_VECTOR_ELT(res, 3, raw_vec);

    SEXP names = PROTECT(Rf_allocVector(STRSXP, 4));
    SET_STRING_ELT(names, 0, Rf_mkChar("width"));
    SET_STRING_ELT(names, 1, Rf_mkChar("height"));
    SET_STRING_ELT(names, 2, Rf_mkChar("stride"));
    SET_STRING_ELT(names, 3, Rf_mkChar("bytes"));
    Rf_setAttrib(res, R_NamesSymbol, names);

    UNPROTECT(3);
    return res;
}

SEXP C_pdfrune_render_page_png(SEXP doc_ptr, SEXP page_sexp, SEXP dpi_sexp) {
    void* doc = get_doc_ptr(doc_ptr);
    size_t p = (size_t)INTEGER(page_sexp)[0];
    double dpi = REAL(dpi_sexp)[0];
    size_t len = 0;
    uint8_t* raw = pdfrune_c_render_page_png(doc, p, dpi, &len);
    if (raw == NULL || len == 0) {
        Rf_error("Failed to render page to PNG");
    }

    SEXP raw_vec = PROTECT(Rf_allocVector(RAWSXP, len));
    memcpy(RAW(raw_vec), raw, len);
    pdfrune_c_free_bytes(raw, len);
    UNPROTECT(1);
    return raw_vec;
}

SEXP C_pdfrune_get_toc_json(SEXP doc_ptr) {
    void* doc = get_doc_ptr(doc_ptr);
    char* s = pdfrune_c_get_toc_json(doc);
    SEXP res = PROTECT(Rf_mkString(s ? s : "[]"));
    pdfrune_c_free_string(s);
    UNPROTECT(1);
    return res;
}

SEXP C_pdfrune_set_toc_json(SEXP doc_ptr, SEXP json_sexp) {
    void* doc = get_doc_ptr(doc_ptr);
    const char* json_str = CHAR(STRING_ELT(json_sexp, 0));
    int32_t ok = pdfrune_c_set_toc_json(doc, json_str);
    return Rf_ScalarLogical(ok);
}

SEXP C_pdfrune_get_links_json(SEXP doc_ptr, SEXP page_sexp) {
    void* doc = get_doc_ptr(doc_ptr);
    size_t p = (size_t)INTEGER(page_sexp)[0];
    char* s = pdfrune_c_get_links_json(doc, p);
    SEXP res = PROTECT(Rf_mkString(s ? s : "[]"));
    pdfrune_c_free_string(s);
    UNPROTECT(1);
    return res;
}

SEXP C_pdfrune_insert_link(SEXP doc_ptr, SEXP page_sexp, SEXP rect_sexp, SEXP uri_sexp, SEXP dest_page_sexp) {
    void* doc = get_doc_ptr(doc_ptr);
    size_t p = (size_t)INTEGER(page_sexp)[0];
    double x0 = REAL(rect_sexp)[0];
    double y0 = REAL(rect_sexp)[1];
    double x1 = REAL(rect_sexp)[2];
    double y1 = REAL(rect_sexp)[3];
    const char* uri = (TYPEOF(uri_sexp) == STRSXP && XLENGTH(uri_sexp) > 0) ? CHAR(STRING_ELT(uri_sexp, 0)) : NULL;
    int32_t dest = INTEGER(dest_page_sexp)[0];
    int32_t ok = pdfrune_c_insert_link(doc, p, x0, y0, x1, y1, uri, dest);
    return Rf_ScalarLogical(ok);
}

SEXP C_pdfrune_delete_link(SEXP doc_ptr, SEXP page_sexp, SEXP xref_sexp, SEXP rect_sexp) {
    void* doc = get_doc_ptr(doc_ptr);
    size_t p = (size_t)INTEGER(page_sexp)[0];
    uint32_t xref = (uint32_t)INTEGER(xref_sexp)[0];
    double x0 = 0, y0 = 0, x1 = 0, y1 = 0;
    if (TYPEOF(rect_sexp) == REALSXP && XLENGTH(rect_sexp) == 4) {
        x0 = REAL(rect_sexp)[0];
        y0 = REAL(rect_sexp)[1];
        x1 = REAL(rect_sexp)[2];
        y1 = REAL(rect_sexp)[3];
    }
    int32_t ok = pdfrune_c_delete_link(doc, p, xref, x0, y0, x1, y1);
    return Rf_ScalarLogical(ok);
}

SEXP C_pdfrune_get_images_json(SEXP doc_ptr, SEXP page_sexp) {
    void* doc = get_doc_ptr(doc_ptr);
    size_t p = (size_t)INTEGER(page_sexp)[0];
    char* s = pdfrune_c_get_images_json(doc, p);
    SEXP res = PROTECT(Rf_mkString(s ? s : "[]"));
    pdfrune_c_free_string(s);
    UNPROTECT(1);
    return res;
}

SEXP C_pdfrune_extract_image_bytes(SEXP doc_ptr, SEXP xref_sexp) {
    void* doc = get_doc_ptr(doc_ptr);
    uint32_t xref = (uint32_t)INTEGER(xref_sexp)[0];
    int32_t w = 0, h = 0;
    char* ext = NULL;
    size_t len = 0;
    uint8_t* raw = pdfrune_c_extract_image_bytes(doc, xref, &w, &h, &ext, &len);
    if (raw == NULL) {
        Rf_error("Failed to extract image with xref %u", xref);
    }

    SEXP raw_vec = PROTECT(Rf_allocVector(RAWSXP, len));
    memcpy(RAW(raw_vec), raw, len);
    pdfrune_c_free_bytes(raw, len);

    SEXP res = PROTECT(Rf_allocVector(VECSXP, 4));
    SET_VECTOR_ELT(res, 0, Rf_ScalarInteger(w));
    SET_VECTOR_ELT(res, 1, Rf_ScalarInteger(h));
    SET_VECTOR_ELT(res, 2, Rf_mkString(ext ? ext : "png"));
    SET_VECTOR_ELT(res, 3, raw_vec);
    if (ext) pdfrune_c_free_string(ext);

    SEXP names = PROTECT(Rf_allocVector(STRSXP, 4));
    SET_STRING_ELT(names, 0, Rf_mkChar("width"));
    SET_STRING_ELT(names, 1, Rf_mkChar("height"));
    SET_STRING_ELT(names, 2, Rf_mkChar("ext"));
    SET_STRING_ELT(names, 3, Rf_mkChar("image"));
    Rf_setAttrib(res, R_NamesSymbol, names);

    UNPROTECT(3);
    return res;
}

SEXP C_pdfrune_insert_image_bytes(SEXP doc_ptr, SEXP page_sexp, SEXP rect_sexp, SEXP bytes_sexp) {
    void* doc = get_doc_ptr(doc_ptr);
    size_t p = (size_t)INTEGER(page_sexp)[0];
    double x0 = REAL(rect_sexp)[0];
    double y0 = REAL(rect_sexp)[1];
    double x1 = REAL(rect_sexp)[2];
    double y1 = REAL(rect_sexp)[3];
    const uint8_t* bytes = (const uint8_t*)RAW(bytes_sexp);
    size_t len = (size_t)XLENGTH(bytes_sexp);
    int32_t xref = pdfrune_c_insert_image_bytes(doc, p, x0, y0, x1, y1, bytes, len);
    return Rf_ScalarInteger(xref);
}

SEXP C_pdfrune_get_annots_json(SEXP doc_ptr, SEXP page_sexp) {
    void* doc = get_doc_ptr(doc_ptr);
    size_t p = (size_t)INTEGER(page_sexp)[0];
    char* s = pdfrune_c_get_annots_json(doc, p);
    SEXP res = PROTECT(Rf_mkString(s ? s : "[]"));
    pdfrune_c_free_string(s);
    UNPROTECT(1);
    return res;
}

SEXP C_pdfrune_add_annot(SEXP doc_ptr, SEXP page_sexp, SEXP subtype_sexp, SEXP rect_sexp, SEXP contents_sexp, SEXP color_sexp, SEXP overlay_sexp) {
    void* doc = get_doc_ptr(doc_ptr);
    size_t p = (size_t)INTEGER(page_sexp)[0];
    const char* st = CHAR(STRING_ELT(subtype_sexp, 0));
    double x0 = REAL(rect_sexp)[0];
    double y0 = REAL(rect_sexp)[1];
    double x1 = REAL(rect_sexp)[2];
    double y1 = REAL(rect_sexp)[3];
    const char* cont = (TYPEOF(contents_sexp) == STRSXP && XLENGTH(contents_sexp) > 0) ? CHAR(STRING_ELT(contents_sexp, 0)) : NULL;
    double r = REAL(color_sexp)[0];
    double g = REAL(color_sexp)[1];
    double b = REAL(color_sexp)[2];
    const char* ot = (TYPEOF(overlay_sexp) == STRSXP && XLENGTH(overlay_sexp) > 0) ? CHAR(STRING_ELT(overlay_sexp, 0)) : NULL;
    uint32_t xref = pdfrune_c_add_annot(doc, p, st, x0, y0, x1, y1, cont, r, g, b, ot);
    return Rf_ScalarInteger((int32_t)xref);
}

SEXP C_pdfrune_delete_annot(SEXP doc_ptr, SEXP page_sexp, SEXP xref_sexp) {
    void* doc = get_doc_ptr(doc_ptr);
    size_t p = (size_t)INTEGER(page_sexp)[0];
    uint32_t xref = (uint32_t)INTEGER(xref_sexp)[0];
    int32_t ok = pdfrune_c_delete_annot(doc, p, xref);
    return Rf_ScalarLogical(ok);
}

SEXP C_pdfrune_apply_redactions(SEXP doc_ptr, SEXP page_sexp) {
    void* doc = get_doc_ptr(doc_ptr);
    size_t p = (size_t)INTEGER(page_sexp)[0];
    int32_t cnt = pdfrune_c_apply_redactions(doc, p);
    return Rf_ScalarInteger(cnt);
}

SEXP C_pdfrune_get_widgets_json(SEXP doc_ptr, SEXP page_sexp) {
    void* doc = get_doc_ptr(doc_ptr);
    size_t p = (size_t)INTEGER(page_sexp)[0];
    char* s = pdfrune_c_get_widgets_json(doc, p);
    SEXP res = PROTECT(Rf_mkString(s ? s : "[]"));
    pdfrune_c_free_string(s);
    UNPROTECT(1);
    return res;
}

SEXP C_pdfrune_add_widget(SEXP doc_ptr, SEXP page_sexp, SEXP name_sexp, SEXP type_sexp, SEXP rect_sexp, SEXP val_sexp) {
    void* doc = get_doc_ptr(doc_ptr);
    size_t p = (size_t)INTEGER(page_sexp)[0];
    const char* n = CHAR(STRING_ELT(name_sexp, 0));
    const char* t = CHAR(STRING_ELT(type_sexp, 0));
    double x0 = REAL(rect_sexp)[0];
    double y0 = REAL(rect_sexp)[1];
    double x1 = REAL(rect_sexp)[2];
    double y1 = REAL(rect_sexp)[3];
    const char* v = CHAR(STRING_ELT(val_sexp, 0));
    uint32_t xref = pdfrune_c_add_widget(doc, p, n, t, x0, y0, x1, y1, v);
    return Rf_ScalarInteger((int32_t)xref);
}

SEXP C_pdfrune_set_field_value(SEXP doc_ptr, SEXP page_sexp, SEXP name_sexp, SEXP val_sexp) {
    void* doc = get_doc_ptr(doc_ptr);
    size_t p = (size_t)INTEGER(page_sexp)[0];
    const char* n = CHAR(STRING_ELT(name_sexp, 0));
    const char* v = CHAR(STRING_ELT(val_sexp, 0));
    int32_t ok = pdfrune_c_set_field_value(doc, p, n, v);
    return Rf_ScalarLogical(ok);
}

SEXP C_pdfrune_new_page(SEXP doc_ptr, SEXP pno_sexp, SEXP width_sexp, SEXP height_sexp) {
    void* doc = get_doc_ptr(doc_ptr);
    int32_t pno = INTEGER(pno_sexp)[0];
    double w = REAL(width_sexp)[0];
    double h = REAL(height_sexp)[0];
    int32_t idx = pdfrune_c_new_page(doc, pno, w, h);
    return Rf_ScalarInteger(idx);
}

SEXP C_pdfrune_delete_page(SEXP doc_ptr, SEXP pno_sexp) {
    void* doc = get_doc_ptr(doc_ptr);
    int32_t pno = INTEGER(pno_sexp)[0];
    int32_t ok = pdfrune_c_delete_page(doc, pno);
    return Rf_ScalarLogical(ok);
}

SEXP C_pdfrune_insert_text(SEXP doc_ptr, SEXP page_sexp, SEXP x_sexp, SEXP y_sexp, SEXP text_sexp, SEXP fontsize_sexp, SEXP color_sexp) {
    void* doc = get_doc_ptr(doc_ptr);
    size_t p = (size_t)INTEGER(page_sexp)[0];
    double x = REAL(x_sexp)[0];
    double y = REAL(y_sexp)[0];
    const char* txt = CHAR(STRING_ELT(text_sexp, 0));
    double fs = REAL(fontsize_sexp)[0];
    double r = REAL(color_sexp)[0];
    double g = REAL(color_sexp)[1];
    double b = REAL(color_sexp)[2];
    int32_t ok = pdfrune_c_insert_text(doc, p, x, y, txt, fs, r, g, b);
    return Rf_ScalarLogical(ok);
}

SEXP C_pdfrune_draw_rect(SEXP doc_ptr, SEXP page_sexp, SEXP rect_sexp, SEXP color_sexp, SEXP fill_sexp, SEXP width_sexp) {
    void* doc = get_doc_ptr(doc_ptr);
    size_t p = (size_t)INTEGER(page_sexp)[0];
    double x0 = REAL(rect_sexp)[0];
    double y0 = REAL(rect_sexp)[1];
    double x1 = REAL(rect_sexp)[2];
    double y1 = REAL(rect_sexp)[3];
    int32_t has_stroke = (TYPEOF(color_sexp) == REALSXP && XLENGTH(color_sexp) == 3);
    double sr = has_stroke ? REAL(color_sexp)[0] : 0.0;
    double sg = has_stroke ? REAL(color_sexp)[1] : 0.0;
    double sb = has_stroke ? REAL(color_sexp)[2] : 0.0;
    int32_t has_fill = (TYPEOF(fill_sexp) == REALSXP && XLENGTH(fill_sexp) == 3);
    double fr = has_fill ? REAL(fill_sexp)[0] : 0.0;
    double fg = has_fill ? REAL(fill_sexp)[1] : 0.0;
    double fb = has_fill ? REAL(fill_sexp)[2] : 0.0;
    double w = REAL(width_sexp)[0];
    int32_t ok = pdfrune_c_draw_rect(doc, p, x0, y0, x1, y1, has_stroke, sr, sg, sb, has_fill, fr, fg, fb, w);
    return Rf_ScalarLogical(ok);
}

SEXP C_pdfrune_to_bytes(SEXP doc_ptr, SEXP deflate_sexp, SEXP encrypt_sexp, SEXP user_pw_sexp, SEXP owner_pw_sexp) {
    void* doc = get_doc_ptr(doc_ptr);
    int32_t deflate = LOGICAL(deflate_sexp)[0];
    int32_t encrypt = LOGICAL(encrypt_sexp)[0];
    const char* u = (TYPEOF(user_pw_sexp) == STRSXP && XLENGTH(user_pw_sexp) > 0) ? CHAR(STRING_ELT(user_pw_sexp, 0)) : "";
    const char* o = (TYPEOF(owner_pw_sexp) == STRSXP && XLENGTH(owner_pw_sexp) > 0) ? CHAR(STRING_ELT(owner_pw_sexp, 0)) : "";
    size_t len = 0;
    uint8_t* raw = pdfrune_c_to_bytes(doc, deflate, encrypt, u, o, &len);
    if (raw == NULL || len == 0) {
        Rf_error("Failed to serialize PDF document");
    }
    SEXP raw_vec = PROTECT(Rf_allocVector(RAWSXP, len));
    memcpy(RAW(raw_vec), raw, len);
    pdfrune_c_free_bytes(raw, len);
    UNPROTECT(1);
    return raw_vec;
}

SEXP C_pdfrune_merge_pdfs(SEXP list_of_raws) {
    if (TYPEOF(list_of_raws) != VECSXP) {
        Rf_error("Expected a list of raw vectors");
    }
    size_t count = (size_t)XLENGTH(list_of_raws);
    if (count == 0) {
        Rf_error("List of raw vectors is empty");
    }
    const uint8_t** ptrs = (const uint8_t**)malloc(count * sizeof(uint8_t*));
    size_t* lens = (size_t*)malloc(count * sizeof(size_t));
    if (!ptrs || !lens) {
        if (ptrs) free(ptrs);
        if (lens) free(lens);
        Rf_error("Memory allocation failed");
    }
    for (size_t i = 0; i < count; i++) {
        SEXP item = VECTOR_ELT(list_of_raws, i);
        if (TYPEOF(item) != RAWSXP) {
            free(ptrs);
            free(lens);
            Rf_error("All elements in list must be raw vectors");
        }
        ptrs[i] = (const uint8_t*)RAW(item);
        lens[i] = (size_t)XLENGTH(item);
    }
    size_t out_len = 0;
    uint8_t* raw = pdfrune_c_merge_pdfs(ptrs, lens, count, &out_len);
    free(ptrs);
    free(lens);
    if (raw == NULL || out_len == 0) {
        Rf_error("Failed to merge PDF documents");
    }
    SEXP raw_vec = PROTECT(Rf_allocVector(RAWSXP, out_len));
    memcpy(RAW(raw_vec), raw, out_len);
    pdfrune_c_free_bytes(raw, out_len);
    UNPROTECT(1);
    return raw_vec;
}

static const R_CallMethodDef CallEntries[] = {
    {"C_pdfrune_version", (DL_FUNC) &C_pdfrune_version, 0},
    {"C_pdfrune_new", (DL_FUNC) &C_pdfrune_new, 0},
    {"C_pdfrune_open", (DL_FUNC) &C_pdfrune_open, 1},
    {"C_pdfrune_page_count", (DL_FUNC) &C_pdfrune_page_count, 1},
    {"C_pdfrune_is_encrypted", (DL_FUNC) &C_pdfrune_is_encrypted, 1},
    {"C_pdfrune_authenticate", (DL_FUNC) &C_pdfrune_authenticate, 2},
    {"C_pdfrune_get_metadata_json", (DL_FUNC) &C_pdfrune_get_metadata_json, 1},
    {"C_pdfrune_set_metadata", (DL_FUNC) &C_pdfrune_set_metadata, 3},
    {"C_pdfrune_page_info_json", (DL_FUNC) &C_pdfrune_page_info_json, 2},
    {"C_pdfrune_extract_text", (DL_FUNC) &C_pdfrune_extract_text, 2},
    {"C_pdfrune_extract_words_json", (DL_FUNC) &C_pdfrune_extract_words_json, 2},
    {"C_pdfrune_extract_blocks_json", (DL_FUNC) &C_pdfrune_extract_blocks_json, 2},
    {"C_pdfrune_search_page_json", (DL_FUNC) &C_pdfrune_search_page_json, 3},
    {"C_pdfrune_search_all_json", (DL_FUNC) &C_pdfrune_search_all_json, 3},
    {"C_pdfrune_find_tables_json", (DL_FUNC) &C_pdfrune_find_tables_json, 2},
    {"C_pdfrune_page_to_markdown", (DL_FUNC) &C_pdfrune_page_to_markdown, 2},
    {"C_pdfrune_doc_to_markdown", (DL_FUNC) &C_pdfrune_doc_to_markdown, 1},
    {"C_pdfrune_page_to_html", (DL_FUNC) &C_pdfrune_page_to_html, 2},
    {"C_pdfrune_doc_to_html", (DL_FUNC) &C_pdfrune_doc_to_html, 1},
    {"C_pdfrune_to_rag_chunks_json", (DL_FUNC) &C_pdfrune_to_rag_chunks_json, 2},
    {"C_pdfrune_page_to_svg", (DL_FUNC) &C_pdfrune_page_to_svg, 2},
    {"C_pdfrune_render_page_rgba", (DL_FUNC) &C_pdfrune_render_page_rgba, 3},
    {"C_pdfrune_render_page_png", (DL_FUNC) &C_pdfrune_render_page_png, 3},
    {"C_pdfrune_get_toc_json", (DL_FUNC) &C_pdfrune_get_toc_json, 1},
    {"C_pdfrune_set_toc_json", (DL_FUNC) &C_pdfrune_set_toc_json, 2},
    {"C_pdfrune_get_links_json", (DL_FUNC) &C_pdfrune_get_links_json, 2},
    {"C_pdfrune_insert_link", (DL_FUNC) &C_pdfrune_insert_link, 5},
    {"C_pdfrune_delete_link", (DL_FUNC) &C_pdfrune_delete_link, 4},
    {"C_pdfrune_get_images_json", (DL_FUNC) &C_pdfrune_get_images_json, 2},
    {"C_pdfrune_extract_image_bytes", (DL_FUNC) &C_pdfrune_extract_image_bytes, 2},
    {"C_pdfrune_insert_image_bytes", (DL_FUNC) &C_pdfrune_insert_image_bytes, 4},
    {"C_pdfrune_get_annots_json", (DL_FUNC) &C_pdfrune_get_annots_json, 2},
    {"C_pdfrune_add_annot", (DL_FUNC) &C_pdfrune_add_annot, 7},
    {"C_pdfrune_delete_annot", (DL_FUNC) &C_pdfrune_delete_annot, 3},
    {"C_pdfrune_apply_redactions", (DL_FUNC) &C_pdfrune_apply_redactions, 2},
    {"C_pdfrune_get_widgets_json", (DL_FUNC) &C_pdfrune_get_widgets_json, 2},
    {"C_pdfrune_add_widget", (DL_FUNC) &C_pdfrune_add_widget, 6},
    {"C_pdfrune_set_field_value", (DL_FUNC) &C_pdfrune_set_field_value, 4},
    {"C_pdfrune_new_page", (DL_FUNC) &C_pdfrune_new_page, 4},
    {"C_pdfrune_delete_page", (DL_FUNC) &C_pdfrune_delete_page, 2},
    {"C_pdfrune_insert_text", (DL_FUNC) &C_pdfrune_insert_text, 7},
    {"C_pdfrune_draw_rect", (DL_FUNC) &C_pdfrune_draw_rect, 6},
    {"C_pdfrune_to_bytes", (DL_FUNC) &C_pdfrune_to_bytes, 5},
    {"C_pdfrune_merge_pdfs", (DL_FUNC) &C_pdfrune_merge_pdfs, 1},
    {NULL, NULL, 0}
};

attribute_visible void R_init_pdfrune(DllInfo* dll) {
    R_registerRoutines(dll, NULL, CallEntries, NULL, NULL);
    R_useDynamicSymbols(dll, FALSE);
}
