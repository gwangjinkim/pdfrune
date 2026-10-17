test_that("Text, word, block extraction, and search work", {
  doc <- pdf_new()
  pdf_new_page(doc)
  pdf_insert_text(doc, 1L, c(72, 80), "Artificial Intelligence Report 2026", fontsize = 18.0)
  pdf_insert_text(doc, 1L, c(72, 120), "Large language models are rapidly advancing.", fontsize = 12.0)

  # Plain text extraction
  txt <- pdf_extract_text(doc, 1L)
  expect_true(grepl("Artificial Intelligence", txt))
  expect_true(grepl("Large language models", txt))

  # Words extraction
  words <- pdf_extract_words(doc, 1L)
  expect_s3_class(words, "data.frame")
  expect_gt(nrow(words), 0)
  expect_true("Artificial" %in% words$word)
  expect_true(all(c("x0", "y0", "x1", "y1", "word", "block_no", "line_no", "word_no") %in% names(words)))

  # Blocks extraction
  blocks <- pdf_extract_blocks(doc, 1L)
  expect_s3_class(blocks, "data.frame")
  expect_gt(nrow(blocks), 0)
  expect_true(any(grepl("Artificial Intelligence", blocks$text)))

  # Substring search
  hits <- pdf_search(doc, "Intelligence", page = 1L)
  expect_s3_class(hits, "data.frame")
  expect_equal(nrow(hits), 1L)
  expect_equal(hits$page, 1L)

  # Regex search across pages
  reg_hits <- pdf_search(doc, "Report \\d{4}", regex = TRUE)
  expect_s3_class(reg_hits, "data.frame")
  expect_equal(nrow(reg_hits), 1L)
})

test_that("Markdown, HTML export, and RAG chunking work", {
  doc <- pdf_new()
  pdf_new_page(doc)
  pdf_insert_text(doc, 1L, c(72, 80), "Section 1. Executive Summary", fontsize = 16.0)
  pdf_insert_text(doc, 1L, c(72, 120), "All project targets were achieved with low latency.", fontsize = 12.0)

  md <- pdf_to_markdown(doc)
  expect_true(is.character(md))
  expect_true(grepl("Executive Summary", md))

  html <- pdf_to_html(doc)
  expect_true(is.character(html))
  expect_true(grepl("Executive Summary", html))

  chunks <- pdf_to_rag_chunks(doc, max_chars = 300L)
  expect_s3_class(chunks, "data.frame")
  expect_gt(nrow(chunks), 0)
  expect_true(all(c("chunk_id", "page_number", "x0", "y0", "x1", "y1", "text", "char_count") %in% names(chunks)))
  expect_true(any(grepl("Executive Summary", chunks$text)))
})
