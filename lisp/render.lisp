;;; -*- Mode: LISP; Syntax: COMMON-LISP; Package: PDFRUNE; Base: 10 -*-
;;;
;;; Copyright (c) 2026, Gwang-Jin Kim. All rights reserved.

(in-package :cl-pdfrune)

(defstruct (pixmap (:constructor %make-pixmap))
  "Represents a rendered 2D raster image with RGBA pixel samples."
  width
  height
  stride
  samples)

(defmethod print-object ((pix pixmap) stream)
  (print-unreadable-object (pix stream :type t :identity t)
    (format stream "~Dx~D RGBA"
            (pixmap-width pix)
            (pixmap-height pix))))

(defun render-page (doc page-number &key (dpi *default-dpi*))
  "Renders PAGE-NUMBER of DOC to an RGBA8888 PIXMAP at resolution DPI."
  (check-page-bounds doc page-number)
  (cffi:with-foreign-object (out-w :int)
    (cffi:with-foreign-object (out-h :int)
      (cffi:with-foreign-object (out-stride :int)
        (cffi:with-foreign-object (out-len :size)
          (let ((ptr (%c-render-page-rgba (pdf-document-pointer doc)
                                          page-number
                                          (coerce dpi 'double-float)
                                          out-w
                                          out-h
                                          out-stride
                                          out-len)))
            (when (or (null ptr) (cffi:null-pointer-p ptr))
              (error 'pdfrune-error :message "Failed to rasterize page to pixmap."))
            (let ((w (cffi:mem-ref out-w :int))
                  (h (cffi:mem-ref out-h :int))
                  (stride (cffi:mem-ref out-stride :int))
                  (len (cffi:mem-ref out-len :size)))
              (%make-pixmap
               :width w
               :height h
               :stride stride
               :samples (foreign-bytes-to-lisp-and-free ptr len)))))))))

(defun pixmap-to-png (pix)
  "Encodes PIX (a PIXMAP structure) into a PNG byte vector."
  (unless (pixmap-p pix)
    (error 'pdfrune-error :message "Expected a PIXMAP structure."))
  (let* ((data (pixmap-samples pix))
         (len (length data)))
    (cffi:with-foreign-pointer (foreign-data len)
      (dotimes (i len)
        (setf (cffi:mem-aref foreign-data :uint8 i) (aref data i)))
      (cffi:with-foreign-object (out-len :size)
        (let ((ptr (%c-pixmap-to-png foreign-data
                                     len
                                     (pixmap-width pix)
                                     (pixmap-height pix)
                                     out-len)))
          (when (or (null ptr) (cffi:null-pointer-p ptr))
            (error 'pdfrune-error :message "Failed to encode pixmap to PNG."))
          (let ((png-len (cffi:mem-ref out-len :size)))
            (foreign-bytes-to-lisp-and-free ptr png-len)))))))

(defun save-pixmap (pix path)
  "Saves PIX to file PATH in PNG format."
  (let ((png-bytes (pixmap-to-png pix)))
    (with-open-file (out path :direction :output
                              :element-type '(unsigned-byte 8)
                              :if-exists :supersede)
      (write-sequence png-bytes out)
      path)))

(defun page-to-svg (doc page-number)
  "Exports vector graphics of PAGE-NUMBER in DOC as an SVG XML string."
  (check-page-bounds doc page-number)
  (foreign-string-to-lisp-and-free
   (%c-page-to-svg (pdf-document-pointer doc) page-number)))
