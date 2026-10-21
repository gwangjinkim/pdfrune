;;; -*- Mode: LISP; Syntax: COMMON-LISP; Package: PDFRUNE; Base: 10 -*-
;;;
;;; Copyright (c) 2026, Gwang-Jin Kim. All rights reserved.

(in-package :cl-pdfrune)

(defun extract-text (doc page-number)
  "Extracts all plain text from PAGE-NUMBER in DOC as a string."
  (check-page-bounds doc page-number)
  (foreign-string-to-lisp-and-free
   (%c-extract-text (pdf-document-pointer doc) page-number)))

(defun extract-words (doc page-number)
  "Extracts individual word tokens along with bounding boxes from PAGE-NUMBER in DOC.
Returns a list of property lists (:WORD :X0 :Y0 :X1 :Y1 :BLOCK-NO :LINE-NO :WORD-NO)."
  (check-page-bounds doc page-number)
  (parse-json-string-and-free
   (%c-extract-words-json (pdf-document-pointer doc) page-number)))

(defun extract-blocks (doc page-number)
  "Extracts structured text layout blocks from PAGE-NUMBER in DOC.
Returns a list of property lists (:TEXT :X0 :Y0 :X1 :Y1 :BLOCK-NO :BLOCK-TYPE)."
  (check-page-bounds doc page-number)
  (parse-json-string-and-free
   (%c-extract-blocks-json (pdf-document-pointer doc) page-number)))

(defun search-page (doc page-number needle)
  "Searches for substring NEEDLE on PAGE-NUMBER in DOC.
Returns a list of bounding boxes (:X0 :Y0 :X1 :Y1)."
  (check-page-bounds doc page-number)
  (parse-json-string-and-free
   (%c-search-page-json (pdf-document-pointer doc) page-number (string needle))))

(defun search-document (doc needle &key regex)
  "Searches for NEEDLE across all pages in DOC using multi-core parallel SIMD matching.
If REGEX is true, evaluates NEEDLE as a regular expression pattern.
Returns a list of matches with (:PAGE :X0 :Y0 :X1 :Y1)."
  (check-open-document doc)
  (let ((results (parse-json-string-and-free
                  (%c-search-all-json (pdf-document-pointer doc)
                                     (string needle)
                                     (if regex 1 0)))))
    ;; Map 1-based page numbers from C FFI to 0-based for Lisp
    (mapcar (lambda (m)
              (let ((p (getf m :page)))
                (if p
                    (list* :page (1- p) (alexandria-remove m :page))
                    m)))
            results)))

(defun to-markdown (doc &key page-number)
  "Converts DOC or a specific PAGE-NUMBER into a structured Markdown string."
  (check-open-document doc)
  (if page-number
      (progn
        (check-page-bounds doc page-number)
        (foreign-string-to-lisp-and-free
         (%c-page-to-markdown (pdf-document-pointer doc) page-number)))
      (foreign-string-to-lisp-and-free
       (%c-doc-to-markdown (pdf-document-pointer doc)))))

(defun to-html (doc &key (page-number 0))
  "Converts PAGE-NUMBER in DOC into clean HTML markup."
  (check-page-bounds doc page-number)
  (foreign-string-to-lisp-and-free
   (%c-page-to-html (pdf-document-pointer doc) page-number)))

(defun to-rag-chunks (doc &key (max-chars 500))
  "Generates semantically coherent text chunks from DOC with bounding box
and heading coordinates for Retrieval-Augmented Generation (RAG) and LLM grounding.
Returns a list of property lists (:TEXT :PAGE :BBOX :HEADING :X0 :Y0 :X1 :Y1 :CHUNK-ID)."
  (check-open-document doc)
  (let ((raw-chunks (parse-json-string-and-free
                     (%c-to-rag-chunks-json (pdf-document-pointer doc) (or max-chars 500)))))
    (mapcar (lambda (c)
              (let* ((x0 (getf c :x0))
                     (y0 (getf c :y0))
                     (x1 (getf c :x1))
                     (y1 (getf c :y1))
                     (page (getf c :page-number)))
                (list* :bbox (list x0 y0 x1 y1)
                       :page (if page (1- page) 0)
                       c)))
            raw-chunks)))

(defun alexandria-remove (plist key)
  "Helper to remove KEY and its value from a plist without requiring external libraries."
  (loop for (k v) on plist by #'cddr
        unless (eq k key)
        append (list k v)))
