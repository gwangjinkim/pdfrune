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
  "Searches for the rupdf foreign library file in well-known relative paths and environment variables."
  (let* ((env-path (or (uiop:getenv "RUPDF_LIBRARY_PATH")
                       (uiop:getenv "RUPDF_LIB_PATH")
                       (uiop:getenv "RUPDF_LIB")))
         (candidates
           (remove-if
            #'null
            (list
             *rupdf-library-path*
             (when (and env-path (not (string= env-path "")))
               (pathname env-path))
             ;; In same directory as asdf system
             (asdf:system-relative-pathname :rupdf "librupdf_ffi.dylib")
             (asdf:system-relative-pathname :rupdf "librupdf_ffi.so")
             (asdf:system-relative-pathname :rupdf "rupdf_ffi.dll")
             ;; If loaded from project root
             (asdf:system-relative-pathname :rupdf "target/release/librupdf_ffi.dylib")
             (asdf:system-relative-pathname :rupdf "target/release/librupdf_ffi.so")
             (asdf:system-relative-pathname :rupdf "target/release/rupdf_ffi.dll")
             ;; If loaded from lisp/
             (asdf:system-relative-pathname :rupdf "../target/release/librupdf_ffi.dylib")
             (asdf:system-relative-pathname :rupdf "../target/release/librupdf_ffi.so")
             (asdf:system-relative-pathname :rupdf "../target/release/rupdf_ffi.dll")
             ;; crates/rupdf-ffi/target/release
             (asdf:system-relative-pathname :rupdf "../crates/rupdf-ffi/target/release/librupdf_ffi.dylib")
             (asdf:system-relative-pathname :rupdf "../crates/rupdf-ffi/target/release/librupdf_ffi.so")
             (asdf:system-relative-pathname :rupdf "../crates/rupdf-ffi/target/release/rupdf_ffi.dll")
             ;; Debug builds
             (asdf:system-relative-pathname :rupdf "target/debug/librupdf_ffi.dylib")
             (asdf:system-relative-pathname :rupdf "target/debug/librupdf_ffi.so")
             (asdf:system-relative-pathname :rupdf "../target/debug/librupdf_ffi.dylib")
             (asdf:system-relative-pathname :rupdf "../target/debug/librupdf_ffi.so")
             ;; Common Unix / macOS locations
             #p"/opt/homebrew/lib/librupdf_ffi.dylib"
             #p"/usr/local/lib/librupdf_ffi.dylib"
             #p"/usr/local/lib/librupdf_ffi.so"
             #p"/usr/lib/librupdf_ffi.so"
             (let ((home (uiop:getenv "HOME")))
               (when home
                 (merge-pathnames ".local/lib/librupdf_ffi.so" (uiop:ensure-directory-pathname home))))))))
    (dolist (c candidates)
      (when (and c (probe-file c))
        (return (namestring (truename c)))))))

(defun build-native-library (&key (release t))
  "Attempts to invoke cargo to compile librupdf_ffi. Returns T on success, NIL otherwise."
  (let* ((sys-dir (asdf:system-source-directory :rupdf))
         (root-dir (cond
                     ((probe-file (merge-pathnames "Cargo.toml" sys-dir))
                      sys-dir)
                     ((probe-file (merge-pathnames "../Cargo.toml" sys-dir))
                      (uiop:pathname-parent-directory-pathname sys-dir))
                     (t nil))))
    (unless root-dir
      (warn "Could not locate root Cargo.toml relative to rupdf system directory ~A" sys-dir)
      (return-from build-native-library nil))
    (let ((args (append (list "cargo" "build" "-p" "rupdf-ffi")
                        (when release (list "--release")))))
      (handler-case
          (progn
            (uiop:run-program args
                              :directory root-dir
                              :output :interactive
                              :error-output :interactive)
            t)
        (error (e)
          (warn "Failed to execute cargo build: ~A" e)
          nil)))))

(defun load-rupdf-library (&optional path)
  "Explicitly loads the rupdf foreign library from PATH or candidate directories."
  (when path
    (setf *rupdf-library-path* (pathname path)))
  (let ((found (or (and *rupdf-library-path* (probe-file *rupdf-library-path*))
                   (find-library-in-candidates))))
    (if found
        (progn
          (cffi:load-foreign-library found)
          (setf *rupdf-library-loaded-p* t))
        (cffi:use-foreign-library librupdf))))

(defun ensure-library-loaded ()
  "Ensures the rupdf foreign library is loaded into the Lisp image.
Provides interactive restarts and helpful diagnostics if the native binary is missing."
  (unless *rupdf-library-loaded-p*
    (tagbody
     :retry
       (let ((found (find-library-in-candidates)))
         (if found
             (handler-case
                 (progn
                   (cffi:load-foreign-library found)
                   (setf *rupdf-library-loaded-p* t))
               (error (err)
                 (restart-case
                     (error 'rupdf-error
                            :message (format nil "Failed to load foreign library ~A: ~A" found err))
                   (retry-load ()
                     :report "Retry loading the foreign library."
                     (go :retry))
                   (specify-library-path (path)
                     :report "Specify an alternative path to the library."
                     :interactive (lambda ()
                                    (format *query-io* "Enter path to librupdf_ffi: ")
                                    (finish-output *query-io*)
                                    (list (read-line *query-io*)))
                     (setf *rupdf-library-path* (pathname path))
                     (go :retry)))))
             (handler-case
                 (progn
                   (cffi:use-foreign-library librupdf)
                   (setf *rupdf-library-loaded-p* t))
               (error (err)
                 (restart-case
                     (error 'rupdf-error
                            :message
                            (format nil
                                    "Could not locate the rupdf foreign shared library (librupdf_ffi).~%~%~
                                     To fix this:~%~
                                       1. Compile the Rust FFI library:~%~
                                          cargo build --release -p rupdf-ffi~%~
                                       2. Or set the RUPDF_LIBRARY_PATH environment variable:~%~
                                          export RUPDF_LIBRARY_PATH=/path/to/librupdf_ffi.dylib (or .so)~%~
                                       3. Or set rupdf:*rupdf-library-path* in Lisp:~%~
                                          (setf rupdf:*rupdf-library-path* #p\"/path/to/librupdf_ffi.dylib\")~%~%~
                                     Original error: ~A" err))
                   (retry-load ()
                     :report "Retry searching and loading the rupdf foreign library."
                     (go :retry))
                   (build-native-library ()
                     :report "Attempt to build librupdf_ffi using 'cargo build --release -p rupdf-ffi'."
                     (when (build-native-library :release t)
                       (go :retry)))
                   (specify-library-path (path)
                     :report "Specify an explicit path to librupdf_ffi."
                     :interactive (lambda ()
                                    (format *query-io* "Enter path to librupdf_ffi: ")
                                    (finish-output *query-io*)
                                    (list (read-line *query-io*)))
                     (setf *rupdf-library-path* (pathname path))
                     (go :retry))))))))))

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
