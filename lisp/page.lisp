;;; -*- Mode: LISP; Syntax: COMMON-LISP; Package: PDFRUNE; Base: 10 -*-
;;;
;;; Copyright (c) 2026, Gwang-Jin Kim. All rights reserved.

(in-package :cl-pdfrune)

(defun check-page-bounds (doc page-number)
  "Ensures PAGE-NUMBER is within [0, (page-count doc)). Signals PDFRUNE-PAGE-OUT-OF-BOUNDS-ERROR."
  (check-open-document doc)
  (let ((count (page-count doc)))
    (unless (and (integerp page-number) (<= 0 page-number) (< page-number count))
      (error 'pdfrune-page-out-of-bounds-error
             :page-index page-number
             :page-count count))))

(defun page-info (doc page-number)
  "Returns a plist describing the dimensions and boxes of PAGE-NUMBER
in DOC (e.g., :WIDTH, :HEIGHT, :ROTATION, :MEDIABOX, :CROPBOX)."
  (check-page-bounds doc page-number)
  (parse-json-string-and-free
   (%c-page-info-json (pdf-document-pointer doc) page-number)))

(defun new-page (doc &key (pno -1) (width 612.0d0) (height 792.0d0))
  "Appends or inserts a new blank page into DOC.
PNO specifies the 0-based insertion index (-1 appends at the end).
WIDTH and HEIGHT specify the page dimensions in points (default: standard US Letter 612x792)."
  (check-open-document doc)
  (let ((res (%c-new-page (pdf-document-pointer doc)
                          (or pno -1)
                          (coerce width 'double-float)
                          (coerce height 'double-float))))
    (unless (>= res 0)
      (error 'pdfrune-error :message "Failed to create new page in document."))
    res))

(defun delete-page (doc page-number)
  "Deletes PAGE-NUMBER (0-based) from DOC."
  (check-page-bounds doc page-number)
  (let ((res (%c-delete-page (pdf-document-pointer doc) page-number)))
    (unless (= res 1)
      (error 'pdfrune-error :message (format nil "Failed to delete page ~D." page-number)))
    t))

(defun insert-text (doc page-number point text &key (font-size 11.0d0) color)
  "Draws TEXT on PAGE-NUMBER at coordinates POINT '(X Y)'.
Optional FONT-SIZE specifies size in points.
Optional COLOR is a list or vector of '(R G B)' normalized between 0.0 and 1.0."
  (check-page-bounds doc page-number)
  (let* ((pt (coerce point 'list))
         (x (coerce (first pt) 'double-float))
         (y (coerce (second pt) 'double-float))
         (rgb (normalize-color color '(0.0d0 0.0d0 0.0d0)))
         (res (%c-insert-text (pdf-document-pointer doc)
                              page-number
                              x y
                              (string text)
                              (coerce font-size 'double-float)
                              (first rgb)
                              (second rgb)
                              (third rgb))))
    (unless (= res 1)
      (error 'pdfrune-error :message "Failed to insert text on page."))
    t))

(defun draw-rect (doc page-number rect &key stroke-color fill-color (width 1.0d0))
  "Draws a vector rectangle on PAGE-NUMBER at RECT '(X0 Y0 X1 Y1)' with optional stroke and fill."
  (check-page-bounds doc page-number)
  (let* ((r (normalize-rect rect))
         (has-stroke (if stroke-color 1 0))
         (s-rgb (normalize-color stroke-color '(0.0d0 0.0d0 0.0d0)))
         (has-fill (if fill-color 1 0))
         (f-rgb (normalize-color fill-color '(1.0d0 1.0d0 1.0d0)))
         (res (%c-draw-rect (pdf-document-pointer doc)
                            page-number
                            (first r) (second r) (third r) (fourth r)
                            (first s-rgb) (second s-rgb) (third s-rgb) has-stroke
                            (first f-rgb) (second f-rgb) (third f-rgb) has-fill
                            (coerce width 'double-float))))
    (unless (= res 1)
      (error 'pdfrune-error :message "Failed to draw rectangle on page."))
    t))
