;;; -*- Mode: LISP; Syntax: COMMON-LISP; Package: RUPDF; Base: 10 -*-
;;;
;;; Copyright (c) 2026, Gwang-Jin Kim. All rights reserved.

(in-package :rupdf)

(cffi:define-foreign-library librupdf
  (:darwin (:or "librupdf_ffi.dylib" "librupdf.dylib"))
  (:unix (:or "librupdf_ffi.so" "librupdf.so"))
  (:windows (:or "rupdf_ffi.dll" "rupdf.dll"))
  (t (:default "librupdf_ffi")))

(defun find-library-in-candidates ()
  "Searches for the rupdf foreign library file in well-known relative paths."
  (let ((candidates
          (list
           *rupdf-library-path*
           ;; In same directory as asdf system
           (asdf:system-relative-pathname :rupdf "librupdf_ffi.dylib")
           (asdf:system-relative-pathname :rupdf "librupdf_ffi.so")
           (asdf:system-relative-pathname :rupdf "rupdf_ffi.dll")
           ;; In root target/release/ or target/debug/
           (asdf:system-relative-pathname :rupdf "../target/release/librupdf_ffi.dylib")
           (asdf:system-relative-pathname :rupdf "../target/release/librupdf_ffi.so")
           (asdf:system-relative-pathname :rupdf "../target/release/rupdf_ffi.dll")
           (asdf:system-relative-pathname :rupdf "../target/debug/librupdf_ffi.dylib")
           (asdf:system-relative-pathname :rupdf "../target/debug/librupdf_ffi.so")
           (asdf:system-relative-pathname :rupdf "../target/debug/rupdf_ffi.dll")
           ;; In crates/rupdf-ffi/target/release
           (asdf:system-relative-pathname :rupdf "../crates/rupdf-ffi/target/release/librupdf_ffi.dylib")
           (asdf:system-relative-pathname :rupdf "../crates/rupdf-ffi/target/release/librupdf_ffi.so")
           (asdf:system-relative-pathname :rupdf "../crates/rupdf-ffi/target/release/rupdf_ffi.dll"))))
    (dolist (c candidates)
      (when (and c (probe-file c))
        (return (namestring (truename c)))))))

(defun ensure-library-loaded ()
  "Ensures the rupdf foreign library is loaded into the Lisp image."
  (unless *rupdf-library-loaded-p*
    (let ((found (find-library-in-candidates)))
      (if found
          (cffi:load-foreign-library found)
          (cffi:use-foreign-library librupdf)))
    (setf *rupdf-library-loaded-p* t)))

;;; Core / Lifecycle CFFI Definitions

(cffi:defcfun ("rupdf_c_version" %c-version) :pointer)

(cffi:defcfun ("rupdf_c_free_string" %c-free-string) :void
  (s :pointer))

(cffi:defcfun ("rupdf_c_free_bytes" %c-free-bytes) :void
  (b :pointer)
  (len :size))

(cffi:defcfun ("rupdf_c_new" %c-new) :pointer)

(cffi:defcfun ("rupdf_c_open_path" %c-open-path) :pointer
  (path :string))

(cffi:defcfun ("rupdf_c_open_bytes" %c-open-bytes) :pointer
  (bytes :pointer)
  (len :size))

(cffi:defcfun ("rupdf_c_free" %c-free) :void
  (doc :pointer))

(cffi:defcfun ("rupdf_c_page_count" %c-page-count) :int
  (doc :pointer))

(cffi:defcfun ("rupdf_c_metadata_json" %c-metadata-json) :pointer
  (doc :pointer))

(cffi:defcfun ("rupdf_c_set_metadata" %c-set-metadata) :int
  (doc :pointer)
  (title :string)
  (author :string)
  (subject :string)
  (keywords :string)
  (creator :string)
  (producer :string))

(cffi:defcfun ("rupdf_c_is_encrypted" %c-is-encrypted) :int
  (doc :pointer))

(cffi:defcfun ("rupdf_c_authenticate" %c-authenticate) :int
  (doc :pointer)
  (pw :string))

(cffi:defcfun ("rupdf_c_page_info_json" %c-page-info-json) :pointer
  (doc :pointer)
  (page :size))

(cffi:defcfun ("rupdf_c_extract_text" %c-extract-text) :pointer
  (doc :pointer)
  (page :size))

(cffi:defcfun ("rupdf_c_extract_words_json" %c-extract-words-json) :pointer
  (doc :pointer)
  (page :size))

(cffi:defcfun ("rupdf_c_extract_blocks_json" %c-extract-blocks-json) :pointer
  (doc :pointer)
  (page :size))

(cffi:defcfun ("rupdf_c_search_page_json" %c-search-page-json) :pointer
  (doc :pointer)
  (page :size)
  (needle :string))

(cffi:defcfun ("rupdf_c_search_all_json" %c-search-all-json) :pointer
  (doc :pointer)
  (needle :string)
  (regex :int))

(cffi:defcfun ("rupdf_c_find_tables_json" %c-find-tables-json) :pointer
  (doc :pointer)
  (page :size))

(cffi:defcfun ("rupdf_c_page_to_markdown" %c-page-to-markdown) :pointer
  (doc :pointer)
  (page :size))

(cffi:defcfun ("rupdf_c_doc_to_markdown" %c-doc-to-markdown) :pointer
  (doc :pointer))

(cffi:defcfun ("rupdf_c_page_to_html" %c-page-to-html) :pointer
  (doc :pointer)
  (page :size))

(cffi:defcfun ("rupdf_c_to_rag_chunks_json" %c-to-rag-chunks-json) :pointer
  (doc :pointer)
  (max-chars :size))

(cffi:defcfun ("rupdf_c_render_page_rgba" %c-render-page-rgba) :pointer
  (doc :pointer)
  (page :size)
  (dpi :double)
  (out-w :pointer)
  (out-h :pointer)
  (out-stride :pointer)
  (out-len :pointer))

(cffi:defcfun ("rupdf_c_render_page_png" %c-render-page-png) :pointer
  (doc :pointer)
  (page :size)
  (dpi :double)
  (out-len :pointer))

(cffi:defcfun ("rupdf_c_pixmap_to_png" %c-pixmap-to-png) :pointer
  (data :pointer)
  (data-len :size)
  (width :uint32)
  (height :uint32)
  (out-len :pointer))

(cffi:defcfun ("rupdf_c_page_to_svg" %c-page-to-svg) :pointer
  (doc :pointer)
  (page :size))

(cffi:defcfun ("rupdf_c_get_toc_json" %c-get-toc-json) :pointer
  (doc :pointer))

(cffi:defcfun ("rupdf_c_set_toc_json" %c-set-toc-json) :int
  (doc :pointer)
  (json-str :string))

(cffi:defcfun ("rupdf_c_get_links_json" %c-get-links-json) :pointer
  (doc :pointer)
  (page :size))

(cffi:defcfun ("rupdf_c_insert_link" %c-insert-link) :int
  (doc :pointer)
  (page :size)
  (x0 :double)
  (y0 :double)
  (x1 :double)
  (y1 :double)
  (uri :string)
  (dest-page :int))

(cffi:defcfun ("rupdf_c_delete_link" %c-delete-link) :int
  (doc :pointer)
  (page :size)
  (xref :uint32)
  (x0 :double)
  (y0 :double)
  (x1 :double)
  (y1 :double))

(cffi:defcfun ("rupdf_c_get_images_json" %c-get-images-json) :pointer
  (doc :pointer)
  (page :size))

(cffi:defcfun ("rupdf_c_extract_image_bytes" %c-extract-image-bytes) :pointer
  (doc :pointer)
  (xref :uint32)
  (out-w :pointer)
  (out-h :pointer)
  (out-ext :pointer)
  (out-len :pointer))

(cffi:defcfun ("rupdf_c_insert_image" %c-insert-image) :int
  (doc :pointer)
  (page :size)
  (x0 :double)
  (y0 :double)
  (x1 :double)
  (y1 :double)
  (path :string)
  (img-bytes :pointer)
  (img-len :size)
  (overlay :string))

(cffi:defcfun ("rupdf_c_get_annots_json" %c-get-annots-json) :pointer
  (doc :pointer)
  (page :size))

(cffi:defcfun ("rupdf_c_add_annot" %c-add-annot) :uint32
  (doc :pointer)
  (page :size)
  (subtype :string)
  (x0 :double)
  (y0 :double)
  (x1 :double)
  (y1 :double)
  (contents :string)
  (r :double)
  (g :double)
  (b :double)
  (overlay :string))

(cffi:defcfun ("rupdf_c_delete_annot" %c-delete-annot) :int
  (doc :pointer)
  (page :size)
  (xref :uint32))

(cffi:defcfun ("rupdf_c_apply_redactions" %c-apply-redactions) :int
  (doc :pointer)
  (page :size))

(cffi:defcfun ("rupdf_c_get_widgets_json" %c-get-widgets-json) :pointer
  (doc :pointer)
  (page :size))

(cffi:defcfun ("rupdf_c_add_widget" %c-add-widget) :uint32
  (doc :pointer)
  (page :size)
  (name :string)
  (field-type :string)
  (x0 :double)
  (y0 :double)
  (x1 :double)
  (y1 :double)
  (val :string))

(cffi:defcfun ("rupdf_c_set_field_value" %c-set-field-value) :int
  (doc :pointer)
  (page :size)
  (name :string)
  (val :string))

(cffi:defcfun ("rupdf_c_new_page" %c-new-page) :int
  (doc :pointer)
  (pno :int)
  (width :double)
  (height :double))

(cffi:defcfun ("rupdf_c_delete_page" %c-delete-page) :int
  (doc :pointer)
  (pno :int))

(cffi:defcfun ("rupdf_c_insert_text" %c-insert-text) :int
  (doc :pointer)
  (pno :int)
  (x :double)
  (y :double)
  (text :string)
  (fontsize :double)
  (r :double)
  (g :double)
  (b :double))

(cffi:defcfun ("rupdf_c_draw_rect" %c-draw-rect) :int
  (doc :pointer)
  (pno :int)
  (x0 :double)
  (y0 :double)
  (x1 :double)
  (y1 :double)
  (sr :double)
  (sg :double)
  (sb :double)
  (has-stroke :int)
  (fr :double)
  (fg :double)
  (fb :double)
  (has-fill :int)
  (width :double))

(cffi:defcfun ("rupdf_c_to_bytes" %c-to-bytes) :pointer
  (doc :pointer)
  (deflate :int)
  (encrypt :int)
  (user-pw :string)
  (owner-pw :string)
  (out-len :pointer))

(cffi:defcfun ("rupdf_c_save" %c-save) :int
  (doc :pointer)
  (path :string)
  (deflate :int)
  (encrypt :int)
  (user-pw :string)
  (owner-pw :string))

(cffi:defcfun ("rupdf_c_merge_pdfs" %c-merge-pdfs) :pointer
  (docs-bytes :pointer)
  (docs-lens :pointer)
  (count :size)
  (out-len :pointer))
