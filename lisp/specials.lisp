;;; -*- Mode: LISP; Syntax: COMMON-LISP; Package: PDFRUNE; Base: 10 -*-
;;;
;;; Copyright (c) 2026, Gwang-Jin Kim. All rights reserved.

(in-package :cl-pdfrune)

(defvar *pdfrune-library-path* nil
  "The explicit file pathname or string to the pdfrune foreign shared library
\(e.g., 'libpdfrune_ffi.dylib', 'libpdfrune_ffi.so', or 'pdfrune_ffi.dll').
If NIL, CFFI will look in default search directories and ASDF system directories.")

(defvar *default-dpi* 150.0d0
  "Default rasterization resolution in dots per inch (DPI).")

(defvar *pdfrune-library-loaded-p* nil
  "Internal flag indicating whether the CFFI foreign library has been loaded.")
