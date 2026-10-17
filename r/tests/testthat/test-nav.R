test_that("Table of Contents and Hyperlinks work", {
  doc <- pdf_new()
  pdf_new_page(doc)
  pdf_new_page(doc)

  # Set and get Table of Contents
  toc_df <- data.frame(
    level = c(1L, 2L),
    title = c("Chapter 1: Foundations", "Section 1.1: Rust Core"),
    page = c(1L, 2L),
    stringsAsFactors = FALSE
  )
  pdf_set_toc(doc, toc_df)

  ret_toc <- pdf_get_toc(doc)
  expect_equal(nrow(ret_toc), 2L)
  expect_equal(ret_toc$title, c("Chapter 1: Foundations", "Section 1.1: Rust Core"))
  expect_equal(ret_toc$page, c(1L, 2L))

  # Links: external URI and internal GoTo
  pdf_insert_link(doc, 1L, c(72, 100, 250, 120), uri = "https://cran.r-project.org")
  pdf_insert_link(doc, 1L, c(72, 140, 250, 160), dest_page = 2L)

  links <- pdf_get_links(doc, 1L)
  expect_s3_class(links, "data.frame")
  expect_equal(nrow(links), 2L)
  expect_true(any(links$uri == "https://cran.r-project.org"))
  expect_true(any(links$page == 2L))

  # Delete link
  pdf_delete_link(doc, 1L, rect = c(72, 100, 250, 120))
  updated_links <- pdf_get_links(doc, 1L)
  expect_equal(nrow(updated_links), 1L)
})
