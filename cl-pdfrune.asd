;;; -*- Mode: LISP; Syntax: COMMON-LISP; Package: CL-USER; Base: 10 -*-
;;;
;;; Copyright (c) 2026, Gwang-Jin Kim. All rights reserved.
;;;
;;; Redistribution and use in source and binary forms, with or without
;;; modification, are permitted provided that the following conditions
;;; are met:
;;;
;;;   * Redistributions of source code must retain the above copyright
;;;     notice, this list of conditions and the following disclaimer.
;;;
;;;   * Redistributions in binary form must reproduce the above copyright
;;;     notice, this list of conditions and the following disclaimer in
;;;     the documentation and/or other materials provided with the
;;;     distribution.
;;;
;;; THIS SOFTWARE IS PROVIDED BY THE AUTHOR 'AS IS' AND ANY EXPRESSED
;;; OR IMPLIED WARRANTIES, INCLUDING, BUT NOT LIMITED TO, THE IMPLIED
;;; WARRANTIES OF MERCHANTABILITY AND FITNESS FOR A PARTICULAR PURPOSE
;;; ARE DISCLAIMED. IN NO EVENT SHALL THE AUTHOR BE LIABLE FOR ANY
;;; DIRECT, INDIRECT, INCIDENTAL, SPECIAL, EXEMPLARY, OR CONSEQUENTIAL
;;; DAMAGES (INCLUDING, BUT NOT LIMITED TO, PROCUREMENT OF SUBSTITUTE
;;; GOODS OR SERVICES; LOSS OF USE, DATA, OR PROFITS; OR BUSINESS
;;; INTERRUPTION) HOWEVER CAUSED AND ON ANY THEORY OF LIABILITY,
;;; WHETHER IN CONTRACT, STRICT LIABILITY, OR TORT (INCLUDING
;;; NEGLIGENCE OR OTHERWISE) ARISING IN ANY WAY OUT OF THE USE OF THIS
;;; SOFTWARE, EVEN IF ADVISED OF THE POSSIBILITY OF SUCH DAMAGE.

(asdf:defsystem :cl-pdfrune
  :version "0.1.0"
  :description "Ultra-Fast, Memory-Safe, Permissive PDF Engine in Common Lisp"
  :author "Gwang-Jin Kim <gwang.jin.kim.phd@gmail.com>"
  :maintainer "Gwang-Jin Kim <gwang.jin.kim.phd@gmail.com>"
  :license "MIT or Apache-2.0"
  :pathname "lisp"
  :serial t
  :depends-on (:cffi :yason)
  :components ((:file "packages")
               (:file "specials")
               (:file "conditions")
               (:file "ffi")
               (:file "util")
               (:file "document")
               (:file "page")
               (:file "text")
               (:file "table")
               (:file "render")
               (:file "nav")
               (:file "annot")
               (:file "crypto"))
  :in-order-to ((asdf:test-op (asdf:test-op :cl-pdfrune-test))))
