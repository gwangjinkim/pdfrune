;;; -*- Mode: LISP; Syntax: COMMON-LISP; Package: PDFRUNE; Base: 10 -*-
;;;
;;; Copyright (c) 2026, Gwang-Jin Kim. All rights reserved.

(in-package :cl-pdfrune)

(defun document-toc (doc)
  "Returns the hierarchical Table of Contents (bookmarks / outlines) of DOC
as a list of property lists (:LEVEL :TITLE :PAGE)."
  (check-open-document doc)
  (parse-json-string-and-free
   (%c-get-toc-json (pdf-document-pointer doc))))

(defun set-document-toc (doc toc)
  "Replaces the Table of Contents of DOC with entries in TOC.
TOC must be a list of plists containing :LEVEL, :TITLE, and :PAGE."
  (check-open-document doc)
  (let* ((json-str
           (with-output-to-string (s)
             (write-char #\[ s)
             (loop for item in toc
                   for first = t then nil
                   for level = (or (getf item :level) 1)
                   for title = (or (getf item :title) "")
                   for page = (or (getf item :page) 1)
                   unless first do (write-char #\, s)
                   do (format s "{\"level\":~D,\"title\":~S,\"page\":~D}" level title page))
             (write-char #\] s)))
         (res (%c-set-toc-json (pdf-document-pointer doc) json-str)))
    (unless (= res 1)
      (error 'pdfrune-error :message "Failed to set Table of Contents."))
    doc))

(defun page-links (doc page-number)
  "Returns all hyperlinks on PAGE-NUMBER in DOC as a list of property lists
\(:KIND :URI :PAGE :X0 :Y0 :X1 :Y1 :XREF)."
  (check-page-bounds doc page-number)
  (parse-json-string-and-free
   (%c-get-links-json (pdf-document-pointer doc) page-number)))

(defun insert-link (doc page-number rect &key uri dest-page)
  "Inserts a clickable hyperlink on PAGE-NUMBER over the area RECT '(X0 Y0 X1 Y1)'.
URI specifies an external web target (e.g. \"https://common-lisp.net\").
DEST-PAGE specifies an internal target page number (1-based)."
  (check-page-bounds doc page-number)
  (let* ((r (normalize-rect rect))
         (res (%c-insert-link (pdf-document-pointer doc)
                              page-number
                              (first r) (second r) (third r) (fourth r)
                              (or uri "")
                              (or dest-page 0))))
    (unless (= res 1)
      (error 'pdfrune-error :message "Failed to insert hyperlink."))
    t))

(defun delete-link (doc page-number &key xref rect)
  "Deletes a hyperlink from PAGE-NUMBER by indirect object number XREF or bounding RECT."
  (check-page-bounds doc page-number)
  (let* ((r (if rect (normalize-rect rect) '(0.0d0 0.0d0 0.0d0 0.0d0)))
         (res (%c-delete-link (pdf-document-pointer doc)
                              page-number
                              (or xref 0)
                              (first r) (second r) (third r) (fourth r))))
    (unless (= res 1)
      (error 'pdfrune-error :message "Failed to delete hyperlink."))
    t))
