;;; -*- Mode: LISP; Syntax: COMMON-LISP; Package: PDFRUNE; Base: 10 -*-
;;;
;;; Copyright (c) 2026, Gwang-Jin Kim. All rights reserved.

(in-package :cl-pdfrune)

(defun foreign-string-to-lisp-and-free (ptr)
  "Converts a foreign C string PTR to a Lisp string and frees PTR.
Returns an empty string if PTR is null."
  (if (or (null ptr) (cffi:null-pointer-p ptr))
      ""
      (unwind-protect
           (cffi:foreign-string-to-lisp ptr)
        (%c-free-string ptr))))

(defun foreign-bytes-to-lisp-and-free (ptr len)
  "Copies LEN bytes from foreign memory PTR into a newly allocated
Lisp (SIMPLE-ARRAY (UNSIGNED-BYTE 8) (*)) vector, and frees PTR."
  (if (or (null ptr) (cffi:null-pointer-p ptr) (zerop len))
      (make-array 0 :element-type '(unsigned-byte 8))
      (unwind-protect
           (let ((vec (make-array len :element-type '(unsigned-byte 8))))
             (dotimes (i len vec)
               (setf (aref vec i) (cffi:mem-aref ptr :uint8 i))))
        (%c-free-bytes ptr len))))

(defun lisp-bytes-to-foreign (vector)
  "Allocates a foreign buffer of unsigned-byte 8 containing the bytes of VECTOR.
Caller is responsible for freeing with CFFI:FOREIGN-FREE."
  (let* ((len (length vector))
         (ptr (cffi:foreign-alloc :uint8 :count len)))
    (dotimes (i len ptr)
      (setf (cffi:mem-aref ptr :uint8 i) (aref vector i)))))

(defun parse-json-plist (json-string)
  "Parses JSON-STRING into a Lisp property list using YASON."
  (if (or (null json-string) (string= json-string ""))
      nil
      (let ((yason:*parse-object-as* :plist)
            (yason:*parse-object-key-fn*
              (lambda (key)
                (intern (substitute #\- #\_ (string-upcase key)) :keyword))))
        (yason:parse json-string))))

(defun parse-json-string-and-free (ptr)
  "Converts foreign string PTR to Lisp, frees PTR, and parses it as a JSON plist or list."
  (let ((str (foreign-string-to-lisp-and-free ptr)))
    (parse-json-plist str)))

(defun normalize-rect (rect)
  "Converts RECT into a list of 4 float numbers (X0 Y0 X1 Y1)."
  (let ((list (coerce rect 'list)))
    (unless (= (length list) 4)
      (error 'pdfrune-error :message (format nil "Invalid rect: ~S (expected 4 numbers)" rect)))
    (mapcar (lambda (x) (coerce x 'double-float)) list)))

(defun normalize-color (color &optional (default '(0.0d0 0.0d0 0.0d0)))
  "Converts COLOR into a list of 3 float numbers (R G B) between 0.0 and 1.0."
  (if (null color)
      default
      (let ((list (coerce color 'list)))
        (unless (= (length list) 3)
          (error 'pdfrune-error :message (format nil "Invalid color: ~S (expected 3 numbers [R G B])" color)))
        (mapcar (lambda (x) (coerce x 'double-float)) list))))
