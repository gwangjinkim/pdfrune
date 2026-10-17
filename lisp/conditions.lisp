;;; -*- Mode: LISP; Syntax: COMMON-LISP; Package: RUPDF; Base: 10 -*-
;;;
;;; Copyright (c) 2026, Gwang-Jin Kim. All rights reserved.

(in-package :rupdf)

(define-condition rupdf-error (error)
  ((message :initarg :message
            :reader rupdf-error-message
            :initform "Unknown rupdf error."))
  (:report (lambda (condition stream)
             (format stream "rupdf error: ~A" (rupdf-error-message condition))))
  (:documentation "Superclass for all errors signaled by the rupdf engine."))

(define-condition rupdf-file-error (rupdf-error)
  ((pathname :initarg :pathname
             :reader rupdf-error-pathname
             :initform nil))
  (:report (lambda (condition stream)
             (format stream "rupdf could not open or process file ~S: ~A"
                     (rupdf-error-pathname condition)
                     (rupdf-error-message condition))))
  (:documentation "Signaled when a PDF file cannot be opened, parsed, or read."))

(define-condition rupdf-crypto-error (rupdf-error)
  ()
  (:report (lambda (condition stream)
             (format stream "rupdf cryptography error: ~A"
                     (rupdf-error-message condition))))
  (:documentation "Signaled when an operation fails due to document encryption or invalid credentials."))

(define-condition rupdf-page-out-of-bounds-error (rupdf-error)
  ((page-index :initarg :page-index
               :reader rupdf-error-page-index
               :initform 0)
   (page-count :initarg :page-count
               :reader rupdf-error-page-count
               :initform 0))
  (:report (lambda (condition stream)
             (format stream "Page index ~D is out of bounds (document has ~D page~:P)."
                     (rupdf-error-page-index condition)
                     (rupdf-error-page-count condition))))
  (:documentation "Signaled when an invalid page number is requested."))
