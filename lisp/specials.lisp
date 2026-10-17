;;; -*- Mode: LISP; Syntax: COMMON-LISP; Package: RUPDF; Base: 10 -*-
;;;
;;; Copyright (c) 2026, Gwang-Jin Kim. All rights reserved.

(in-package :rupdf)

(defvar *rupdf-library-path* nil
  "The explicit file pathname or string to the rupdf foreign shared library
\(e.g., 'librupdf_ffi.dylib', 'librupdf_ffi.so', or 'rupdf_ffi.dll').
If NIL, CFFI will look in default search directories and ASDF system directories.")

(defvar *default-dpi* 150.0d0
  "Default rasterization resolution in dots per inch (DPI).")

(defvar *rupdf-library-loaded-p* nil
  "Internal flag indicating whether the CFFI foreign library has been loaded.")
