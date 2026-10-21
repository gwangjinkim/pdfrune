;;; -*- Mode: LISP; Syntax: COMMON-LISP; Package: CL-USER; Base: 10 -*-
;;;
;;; Copyright (c) 2026, Gwang-Jin Kim. All rights reserved.

(in-package :cl-user)

(defpackage :cl-pdfrune  (:nicknames :pdfrune)
  (:use :cl)
  (:export
   ;; Specials & Library Management
   #:*pdfrune-library-path*
   #:*default-dpi*
   #:ensure-library-loaded
   #:load-pdfrune-library
   #:build-native-library

   ;; Conditions
   #:pdfrune-error
   #:pdfrune-error-message
   #:pdfrune-file-error
   #:pdfrune-error-pathname
   #:pdfrune-crypto-error
   #:pdfrune-page-out-of-bounds-error
   #:pdfrune-error-page-index
   #:pdfrune-error-page-count

   ;; Types & Structures
   #:pdf-document
   #:pdf-document-p
   #:pixmap
   #:pixmap-p
   #:pixmap-width
   #:pixmap-height
   #:pixmap-stride
   #:pixmap-samples
   #:pdf-table
   #:pdf-table-p
   #:table-row-count
   #:table-col-count
   #:table-bbox
   #:table-header
   #:table-rows
   #:table-markdown
   #:table-to-markdown

   ;; Core Document Lifecycle
   #:version
   #:open-pdf
   #:new-pdf
   #:close-pdf
   #:with-open-pdf
   #:page-count
   #:document-metadata
   #:set-document-metadata
   #:save-document
   #:document-to-bytes
   #:merge-documents

   ;; Page Operations
   #:new-page
   #:delete-page
   #:page-info
   #:insert-text
   #:draw-rect

   ;; Text Extraction & Search
   #:extract-text
   #:extract-words
   #:extract-blocks
   #:search-page
   #:search-document
   #:to-markdown
   #:to-html
   #:to-rag-chunks

   ;; Table Detection
   #:find-tables

   ;; Rendering & SVG
   #:render-page
   #:save-pixmap
   #:pixmap-to-png
   #:page-to-svg

   ;; Navigation & Hyperlinks
   #:document-toc
   #:set-document-toc
   #:page-links
   #:insert-link
   #:delete-link

   ;; Image Handling
   #:page-images
   #:extract-image
   #:insert-image

   ;; Annotations & Redactions
   #:page-annotations
   #:add-highlight-annotation
   #:add-text-annotation
   #:add-rect-annotation
   #:delete-annotation
   #:add-redaction
   #:apply-redactions

   ;; Form Fields (AcroForm)
   #:page-widgets
   #:add-widget
   #:set-widget-value

   ;; Security & Decryption
   #:document-encrypted-p
   #:authenticate-document))
