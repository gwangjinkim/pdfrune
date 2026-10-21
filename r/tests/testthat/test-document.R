test_that("Document creation, page management, and metadata work", {
  doc <- pdf_new()
  expect_s3_class(doc, "pdfrune_document")
  expect_equal(pdf_page_count(doc), 0L)

  # Add pages
  p1 <- pdf_new_page(doc, width = 612.0, height = 792.0)
  expect_equal(p1, 1L)
  expect_equal(pdf_page_count(doc), 1L)

  p2 <- pdf_new_page(doc, width = 612.0, height = 792.0)
  expect_equal(p2, 2L)
  expect_equal(pdf_page_count(doc), 2L)

  # Page info
  info1 <- pdf_page_info(doc, 1L)
  expect_equal(info1$width, 612.0)
  expect_equal(info1$height, 792.0)
  expect_equal(info1$rotation, 0L)

  # Delete page
  pdf_delete_page(doc, 2L)
  expect_equal(pdf_page_count(doc), 1L)

  # Metadata
  pdf_set_metadata(doc, list(title = "R pdfrune Test", author = "Gwang-Jin Kim"))
  meta <- pdf_metadata(doc)
  expect_equal(meta$title, "R pdfrune Test")
  expect_equal(meta$author, "Gwang-Jin Kim")

  # Serialization roundtrip
  bytes <- pdf_to_bytes(doc, deflate = TRUE)
  expect_true(is.raw(bytes))
  expect_gt(length(bytes), 0)

  doc_reopened <- pdf_open(bytes = bytes)
  expect_equal(pdf_page_count(doc_reopened), 1L)
  reopened_meta <- pdf_metadata(doc_reopened)
  expect_equal(reopened_meta$title, "R pdfrune Test")
})

test_that("pdf_merge combines multiple PDF files", {
  d1 <- pdf_new()
  pdf_new_page(d1)
  pdf_insert_text(d1, 1L, c(72, 72), "Document 1")
  f1 <- tempfile(fileext = ".pdf")
  pdf_save(d1, f1)

  d2 <- pdf_new()
  pdf_new_page(d2)
  pdf_insert_text(d2, 1L, c(72, 72), "Document 2")
  f2 <- tempfile(fileext = ".pdf")
  pdf_save(d2, f2)

  merged <- pdf_merge(c(f1, f2))
  expect_equal(pdf_page_count(merged), 2L)
  t1 <- pdf_extract_text(merged, 1L)
  t2 <- pdf_extract_text(merged, 2L)
  expect_true(grepl("Document 1", t1))
  expect_true(grepl("Document 2", t2))
})
