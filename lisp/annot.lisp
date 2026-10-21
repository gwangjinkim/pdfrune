;;; -*- Mode: LISP; Syntax: COMMON-LISP; Package: PDFRUNE; Base: 10 -*-
;;;
;;; Copyright (c) 2026, Gwang-Jin Kim. All rights reserved.

(in-package :cl-pdfrune)

;;; Image Handling

(defun page-images (doc page-number)
  "Returns a list of embedded image dictionaries on PAGE-NUMBER in DOC."
  (check-page-bounds doc page-number)
  (parse-json-string-and-free
   (%c-get-images-json (pdf-document-pointer doc) page-number)))

(defun extract-image (doc xref)
  "Extracts an embedded image object identified by indirect object number XREF.
Returns a plist (:SAMPLES :WIDTH :HEIGHT :FORMAT)."
  (check-open-document doc)
  (cffi:with-foreign-object (out-w :int)
    (cffi:with-foreign-object (out-h :int)
      (cffi:with-foreign-object (out-ext :pointer)
        (cffi:with-foreign-object (out-len :size)
          (let ((ptr (%c-extract-image-bytes (pdf-document-pointer doc)
                                             xref
                                             out-w
                                             out-h
                                             out-ext
                                             out-len)))
            (when (or (null ptr) (cffi:null-pointer-p ptr))
              (error 'pdfrune-error :message (format nil "Failed to extract image with xref ~D." xref)))
            (let* ((w (cffi:mem-ref out-w :int))
                   (h (cffi:mem-ref out-h :int))
                   (len (cffi:mem-ref out-len :size))
                   (ext-ptr (cffi:mem-ref out-ext :pointer))
                   (ext (foreign-string-to-lisp-and-free ext-ptr))
                   (samples (foreign-bytes-to-lisp-and-free ptr len)))
              (list :samples samples
                    :width w
                    :height h
                    :format ext))))))))

(defun insert-image (doc page-number rect &key filename stream pixmap (overlay "foreground"))
  "Inserts an image onto PAGE-NUMBER at location RECT '(X0 Y0 X1 Y1)'.
Source may be specified as FILENAME (path), STREAM (byte vector), or PIXMAP structure."
  (check-page-bounds doc page-number)
  (let ((r (normalize-rect rect))
        (path-str (if filename (namestring (truename filename)) ""))
        (bytes-ptr (cffi:null-pointer))
        (bytes-len 0)
        (allocated nil))
    (cond
      ((typep stream '(vector (unsigned-byte 8)))
       (setf bytes-len (length stream)
             bytes-ptr (lisp-bytes-to-foreign stream)
             allocated t))
      ((pixmap-p pixmap)
       (let ((png (pixmap-to-png pixmap)))
         (setf bytes-len (length png)
               bytes-ptr (lisp-bytes-to-foreign png)
               allocated t))))
    (unwind-protect
         (let ((res (%c-insert-image (pdf-document-pointer doc)
                                     page-number
                                     (first r) (second r) (third r) (fourth r)
                                     path-str
                                     bytes-ptr
                                     bytes-len
                                     (or overlay "foreground"))))
           (unless (= res 1)
             (error 'pdfrune-error :message "Failed to insert image on page."))
           t)
      (when allocated
        (cffi:foreign-free bytes-ptr)))))

;;; Annotations

(defun page-annotations (doc page-number)
  "Returns all annotations present on PAGE-NUMBER in DOC as a list of property lists."
  (check-page-bounds doc page-number)
  (let ((raw (parse-json-string-and-free
              (%c-get-annots-json (pdf-document-pointer doc) page-number))))
    (mapcar (lambda (a)
              (let ((type (getf a :type)))
                (list* :kind type a)))
            raw)))

(defun add-highlight-annotation (doc page-number rect &key (text "") color)
  "Adds a text highlight annotation over RECT on PAGE-NUMBER in DOC."
  (check-page-bounds doc page-number)
  (let ((r (normalize-rect rect))
        (rgb (normalize-color color '(1.0d0 1.0d0 0.0d0))))
    (let ((res (%c-add-annot (pdf-document-pointer doc)
                             page-number
                             "Highlight"
                             (first r) (second r) (third r) (fourth r)
                             (or text "")
                             (first rgb) (second rgb) (third rgb)
                             "")))
      (unless (> res 0)
        (error 'pdfrune-error :message "Failed to add highlight annotation."))
      res)))

(defun add-text-annotation (doc page-number point &key (text "") color)
  "Adds a sticky note text annotation at POINT '(X Y) on PAGE-NUMBER in DOC."
  (check-page-bounds doc page-number)
  (let* ((pt (coerce point 'list))
         (x (coerce (first pt) 'double-float))
         (y (coerce (second pt) 'double-float))
         (rgb (normalize-color color '(1.0d0 1.0d0 0.0d0))))
    (let ((res (%c-add-annot (pdf-document-pointer doc)
                             page-number
                             "Text"
                             x y (+ x 24.0d0) (+ y 24.0d0)
                             (or text "")
                             (first rgb) (second rgb) (third rgb)
                             "")))
      (unless (> res 0)
        (error 'pdfrune-error :message "Failed to add text annotation."))
      res)))

(defun add-rect-annotation (doc page-number rect &key color (width 1.0d0))
  "Adds a square/rectangle markup annotation over RECT on PAGE-NUMBER in DOC."
  (declare (ignore width))
  (check-page-bounds doc page-number)
  (let ((r (normalize-rect rect))
        (rgb (normalize-color color '(1.0d0 0.0d0 0.0d0))))
    (let ((res (%c-add-annot (pdf-document-pointer doc)
                             page-number
                             "Square"
                             (first r) (second r) (third r) (fourth r)
                             ""
                             (first rgb) (second rgb) (third rgb)
                             "")))
      (unless (> res 0)
        (error 'pdfrune-error :message "Failed to add rect annotation."))
      res)))

(defun delete-annotation (doc page-number xref)
  "Deletes the annotation with indirect object number XREF from PAGE-NUMBER in DOC."
  (check-page-bounds doc page-number)
  (let ((res (%c-delete-annot (pdf-document-pointer doc) page-number xref)))
    (unless (= res 1)
      (error 'pdfrune-error :message "Failed to delete annotation."))
    t))

;;; Redactions

(defun add-redaction (doc page-number rect &key (fill '(0.0d0 0.0d0 0.0d0)))
  "Stages rectangular area RECT '(X0 Y0 X1 Y1)' on PAGE-NUMBER for permanent redaction."
  (check-page-bounds doc page-number)
  (let ((r (normalize-rect rect))
        (rgb (normalize-color fill '(0.0d0 0.0d0 0.0d0))))
    (let ((res (%c-add-annot (pdf-document-pointer doc)
                             page-number
                             "Redact"
                             (first r) (second r) (third r) (fourth r)
                             ""
                             (first rgb) (second rgb) (third rgb)
                             "")))
      (unless (> res 0)
        (error 'pdfrune-error :message "Failed to stage redaction annotation."))
      res)))

(defun apply-redactions (doc page-number)
  "Applies all staged redactions on PAGE-NUMBER, permanently scrubbing the text
and graphics underneath from the underlying PDF content stream."
  (check-page-bounds doc page-number)
  (let ((res (%c-apply-redactions (pdf-document-pointer doc) page-number)))
    (unless (>= res 0)
      (error 'pdfrune-error :message "Failed to apply redactions."))
    t))

;;; Interactive Form Fields (AcroForm)

(defun page-widgets (doc page-number)
  "Returns all interactive AcroForm form fields on PAGE-NUMBER in DOC."
  (check-page-bounds doc page-number)
  (let ((raw (parse-json-string-and-free
              (%c-get-widgets-json (pdf-document-pointer doc) page-number))))
    (mapcar (lambda (w)
              (list* :name (getf w :field-name)
                     :type (getf w :field-type)
                     :value (getf w :field-value)
                     w))
            raw)))

(defun add-widget (doc page-number rect name field-type &key (value ""))
  "Adds an interactive form widget (e.g. \"text\", \"checkbox\", \"choice\") to PAGE-NUMBER."
  (check-page-bounds doc page-number)
  (let ((r (normalize-rect rect)))
    (let ((res (%c-add-widget (pdf-document-pointer doc)
                              page-number
                              (string name)
                              (string field-type)
                              (first r) (second r) (third r) (fourth r)
                              (or value ""))))
      (unless (> res 0)
        (error 'pdfrune-error :message "Failed to add form widget."))
      res)))

(defun set-widget-value (doc name value &key (page-number 0))
  "Sets the value of the interactive form field named NAME on PAGE-NUMBER in DOC."
  (check-page-bounds doc page-number)
  (let ((res (%c-set-field-value (pdf-document-pointer doc)
                                 page-number
                                 (string name)
                                 (string value))))
    (unless (= res 1)
      (error 'pdfrune-error :message (format nil "Failed to set field value for ~S." name)))
    t))
