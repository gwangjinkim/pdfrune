;;; -*- Mode: LISP; Syntax: COMMON-LISP; Package: PDFRUNE; Base: 10 -*-
;;;
;;; Copyright (c) 2026, Gwang-Jin Kim. All rights reserved.

(in-package :cl-pdfrune)

(defstruct (pdf-table (:constructor %make-pdf-table)
                      (:conc-name table-))
  "Represents a tabular data layout extracted from a PDF page."
  row-count
  col-count
  bbox
  header
  rows
  markdown)

(defmethod print-object ((tbl pdf-table) stream)
  (print-unreadable-object (tbl stream :type t :identity t)
    (format stream "~Dx~D ROWS"
            (table-row-count tbl)
            (table-col-count tbl))))

(defun table-to-markdown (table)
  "Returns the Markdown table representation of TABLE."
  (table-markdown table))

(defun find-tables (doc page-number)
  "Automatically detects tabular structures on PAGE-NUMBER in DOC.
Returns a list of PDF-TABLE structures."
  (check-page-bounds doc page-number)
  (let ((raw-list (parse-json-string-and-free
                   (%c-find-tables-json (pdf-document-pointer doc) page-number))))
    (mapcar (lambda (item)
              (%make-pdf-table
               :row-count (getf item :row-count)
               :col-count (getf item :col-count)
               :bbox (getf item :bbox)
               :header (getf item :header)
               :rows (getf item :rows)
               :markdown (getf item :markdown)))
            raw-list)))
