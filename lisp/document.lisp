;;; -*- Mode: LISP; Syntax: COMMON-LISP; Package: PDFRUNE; Base: 10 -*-
;;;
;;; Copyright (c) 2026, Gwang-Jin Kim. All rights reserved.

(in-package :cl-pdfrune)

(defstruct (pdf-document (:constructor %make-pdf-document (pointer &optional path)))
  "Represents an open or in-memory PDF document managed by pdfrune."
  pointer
  path)

(defmethod print-object ((doc pdf-document) stream)
  (print-unreadable-object (doc stream :type t :identity t)
    (if (pdf-document-pointer doc)
        (format stream ":PAGES ~D~@[ ~S~]"
                (ignore-errors (page-count doc))
                (pdf-document-path doc))
        (format stream ":CLOSED"))))

(defun version ()
  "Returns the version of the underlying pdfrune engine as a string."
  (ensure-library-loaded)
  (foreign-string-to-lisp-and-free (%c-version)))

(defun check-open-document (doc)
  "Ensures DOC is a valid, open PDF-DOCUMENT. Signals PDFRUNE-ERROR otherwise."
  (unless (and (pdf-document-p doc) (pdf-document-pointer doc))
    (error 'pdfrune-error :message "The PDF document is closed or invalid.")))

(defun open-pdf (path-or-bytes &key password)
  "Opens a PDF document from PATH-OR-BYTES.
PATH-OR-BYTES may be a pathname, a namestring, or a vector of (UNSIGNED-BYTE 8).
Optional PASSWORD is used for immediate authentication if provided.
Returns a PDF-DOCUMENT structure."
  (ensure-library-loaded)
  (let ((ptr nil)
        (path-str nil))
    (cond
      ((or (stringp path-or-bytes) (pathnamep path-or-bytes))
       (setf path-str (namestring (truename path-or-bytes)))
       (setf ptr (%c-open-path path-str))
       (when (or (null ptr) (cffi:null-pointer-p ptr))
         (error 'pdfrune-file-error
                :pathname path-str
                :message "Failed to open or parse PDF file.")))
      ((typep path-or-bytes '(vector (unsigned-byte 8)))
       (let ((len (length path-or-bytes)))
         (cffi:with-foreign-pointer (foreign-buf len)
           (dotimes (i len)
             (setf (cffi:mem-aref foreign-buf :uint8 i) (aref path-or-bytes i)))
           (setf ptr (%c-open-bytes foreign-buf len))))
       (when (or (null ptr) (cffi:null-pointer-p ptr))
         (error 'pdfrune-file-error
                :pathname "<byte-vector>"
                :message "Failed to parse PDF from memory buffer.")))
      (t
       (error 'pdfrune-error
              :message (format nil "Invalid path-or-bytes: ~S" path-or-bytes))))
    (let ((doc (%make-pdf-document ptr path-str)))
      (when password
        (authenticate-document doc password))
      doc)))

(defun new-pdf ()
  "Creates and returns a new empty in-memory PDF document."
  (ensure-library-loaded)
  (let ((ptr (%c-new)))
    (when (or (null ptr) (cffi:null-pointer-p ptr))
      (error 'pdfrune-error :message "Failed to allocate new PDF document."))
    (%make-pdf-document ptr nil)))

(defun close-pdf (doc)
  "Closes DOC and releases all associated native foreign memory.
Safe to call multiple times."
  (when (and (pdf-document-p doc) (pdf-document-pointer doc))
    (%c-free (pdf-document-pointer doc))
    (setf (pdf-document-pointer doc) nil))
  nil)

(defmacro with-open-pdf ((var path-or-bytes &key password) &body body)
  "Binds VAR to an open PDF document from PATH-OR-BYTES, executes BODY,
and guarantees that the document is closed and its memory released upon exit."
  `(let ((,var (if (pdf-document-p ,path-or-bytes)
                   ,path-or-bytes
                   (open-pdf ,path-or-bytes ,@(when password `(:password ,password))))))
     (unwind-protect
          (progn ,@body)
       (close-pdf ,var))))

(defun page-count (doc)
  "Returns the total number of pages in DOC."
  (check-open-document doc)
  (%c-page-count (pdf-document-pointer doc)))

(defun document-metadata (doc)
  "Returns a property list containing document metadata (e.g. :TITLE, :AUTHOR, :SUBJECT)."
  (check-open-document doc)
  (parse-json-string-and-free (%c-metadata-json (pdf-document-pointer doc))))

(defun set-document-metadata (doc &key title author subject keywords creator producer)
  "Updates the document information dictionary fields for DOC."
  (check-open-document doc)
  (let ((res (%c-set-metadata (pdf-document-pointer doc)
                              (or title "")
                              (or author "")
                              (or subject "")
                              (or keywords "")
                              (or creator "")
                              (or producer ""))))
    (unless (= res 1)
      (error 'pdfrune-error :message "Failed to set document metadata."))
    doc))

(defun save-document (doc path &key (deflate t) encryption user-password owner-password)
  "Saves DOC to PATH with optional Flate compression and RC4-128 encryption.
Optional ENCRYPTION specifies the encryption algorithm (e.g. \"rc4-128\")."
  (check-open-document doc)
  (let* ((namestr (namestring path))
         (encrypt (if (or encryption user-password owner-password) 1 0))
         (res (%c-save (pdf-document-pointer doc)
                       namestr
                       (if deflate 1 0)
                       encrypt
                       (or user-password "")
                       (or owner-password ""))))
    (unless (= res 1)
      (error 'pdfrune-error :message (format nil "Failed to save PDF document to ~S" namestr)))
    doc))

(defun document-to-bytes (doc &key (deflate t) encryption user-password owner-password)
  "Serializes DOC into a freshly allocated Lisp (UNSIGNED-BYTE 8) vector.
Optional ENCRYPTION specifies the encryption algorithm (e.g. \"rc4-128\")."
  (check-open-document doc)
  (let ((encrypt (if (or encryption user-password owner-password) 1 0)))
    (cffi:with-foreign-object (out-len :size)
      (let ((ptr (%c-to-bytes (pdf-document-pointer doc)
                              (if deflate 1 0)
                              encrypt
                              (or user-password "")
                              (or owner-password "")
                              out-len)))
        (when (or (null ptr) (cffi:null-pointer-p ptr))
          (error 'pdfrune-error :message "Failed to serialize PDF document to bytes."))
        (let ((len (cffi:mem-ref out-len :size)))
          (foreign-bytes-to-lisp-and-free ptr len))))))

(defun merge-documents (files-or-bytes-list &key target-path)
  "Concatenates multiple PDF documents (specified as paths or byte vectors).
If TARGET-PATH is given, writes to disk; otherwise returns serialized bytes."
  (ensure-library-loaded)
  (let* ((count (length files-or-bytes-list))
         (byte-arrays
           (mapcar (lambda (item)
                     (cond
                       ((typep item '(vector (unsigned-byte 8)))
                        item)
                       ((or (stringp item) (pathnamep item))
                        (with-open-file (s (truename item) :direction :input :element-type '(unsigned-byte 8))
                          (let ((buf (make-array (file-length s) :element-type '(unsigned-byte 8))))
                            (read-sequence buf s)
                            buf)))
                       (t (error 'pdfrune-error :message (format nil "Cannot merge item ~S" item)))))
                   files-or-bytes-list)))
    (cffi:with-foreign-object (ptrs-array :pointer count)
      (cffi:with-foreign-object (lens-array :size count)
        (let ((allocated-ptrs nil))
          (unwind-protect
               (progn
                 (loop for arr in byte-arrays
                       for idx from 0
                       for len = (length arr)
                       for foreign-ptr = (lisp-bytes-to-foreign arr)
                       do (push foreign-ptr allocated-ptrs)
                          (setf (cffi:mem-aref ptrs-array :pointer idx) foreign-ptr)
                          (setf (cffi:mem-aref lens-array :size idx) len))
                 (cffi:with-foreign-object (out-len :size)
                   (let ((res-ptr (%c-merge-pdfs ptrs-array lens-array count out-len)))
                     (when (or (null res-ptr) (cffi:null-pointer-p res-ptr))
                       (error 'pdfrune-error :message "Failed to merge PDF documents."))
                     (let* ((len (cffi:mem-ref out-len :size))
                            (merged-bytes (foreign-bytes-to-lisp-and-free res-ptr len)))
                       (if target-path
                           (with-open-file (out target-path :direction :output
                                                            :element-type '(unsigned-byte 8)
                                                            :if-exists :supersede)
                             (write-sequence merged-bytes out)
                             merged-bytes)
                           merged-bytes)))))
            (dolist (p allocated-ptrs)
              (cffi:foreign-free p))))))))
