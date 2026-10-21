;;; -*- Mode: LISP; Syntax: COMMON-LISP; Package: PDFRUNE; Base: 10 -*-
;;;
;;; Copyright (c) 2026, Gwang-Jin Kim. All rights reserved.

(in-package :cl-pdfrune)

(define-condition pdfrune-error (error)
  ((message :initarg :message
            :reader pdfrune-error-message
            :initform "Unknown pdfrune error."))
  (:report (lambda (condition stream)
             (format stream "pdfrune error: ~A" (pdfrune-error-message condition))))
  (:documentation "Superclass for all errors signaled by the pdfrune engine."))

(define-condition pdfrune-file-error (pdfrune-error)
  ((pathname :initarg :pathname
             :reader pdfrune-error-pathname
             :initform nil))
  (:report (lambda (condition stream)
             (format stream "pdfrune could not open or process file ~S: ~A"
                     (pdfrune-error-pathname condition)
                     (pdfrune-error-message condition))))
  (:documentation "Signaled when a PDF file cannot be opened, parsed, or read."))

(define-condition pdfrune-crypto-error (pdfrune-error)
  ()
  (:report (lambda (condition stream)
             (format stream "pdfrune cryptography error: ~A"
                     (pdfrune-error-message condition))))
  (:documentation "Signaled when an operation fails due to document encryption or invalid credentials."))

(define-condition pdfrune-page-out-of-bounds-error (pdfrune-error)
  ((page-index :initarg :page-index
               :reader pdfrune-error-page-index
               :initform 0)
   (page-count :initarg :page-count
               :reader pdfrune-error-page-count
               :initform 0))
  (:report (lambda (condition stream)
             (format stream "Page index ~D is out of bounds (document has ~D page~:P)."
                     (pdfrune-error-page-index condition)
                     (pdfrune-error-page-count condition))))
  (:documentation "Signaled when an invalid page number is requested."))
