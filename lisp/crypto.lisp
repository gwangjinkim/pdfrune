;;; -*- Mode: LISP; Syntax: COMMON-LISP; Package: PDFRUNE; Base: 10 -*-
;;;
;;; Copyright (c) 2026, Gwang-Jin Kim. All rights reserved.

(in-package :cl-pdfrune)

(defun document-encrypted-p (doc)
  "Returns T if DOC is encrypted with a password, NIL otherwise."
  (check-open-document doc)
  (= (%c-is-encrypted (pdf-document-pointer doc)) 1))

(defun authenticate-document (doc password)
  "Attempts to authenticate DOC using PASSWORD.
Returns T if password matches user or owner password, NIL otherwise."
  (check-open-document doc)
  (= (%c-authenticate (pdf-document-pointer doc) (string password)) 1))
