test_that("Table detection and conversions work", {
  doc <- pdf_new()
  pdf_new_page(doc, width = 612.0, height = 792.0)

  pdf_insert_text(doc, 1L, c(72.0, 60.0), "Quarterly Financial Summary", fontsize = 18.0)

  rows <- list(
    c("Quarter", "Revenue", "Growth"),
    c("Q1-2026", "$12.4M", "+18%"),
    c("Q2-2026", "$15.1M", "+22%"),
    c("Q3-2026", "$19.8M", "+31%")
  )
  y <- 120.0
  for (r in rows) {
    pdf_insert_text(doc, 1L, c(72.0, y), r[1], fontsize = 12.0)
    pdf_insert_text(doc, 1L, c(220.0, y), r[2], fontsize = 12.0)
    pdf_insert_text(doc, 1L, c(360.0, y), r[3], fontsize = 12.0)
    y <- y + 28.0
  }

  tabs <- pdf_find_tables(doc, 1L)
  expect_equal(length(tabs), 1L)

  tbl <- tabs[[1]]
  expect_s3_class(tbl, "pdfrune_table")
  expect_equal(tbl$row_count, 4L)
  expect_equal(tbl$col_count, 3L)
  expect_equal(tbl$header, c("Quarter", "Revenue", "Growth"))

  df <- tbl$to_dataframe()
  expect_s3_class(df, "data.frame")
  expect_equal(nrow(df), 3L)
  expect_equal(colnames(df), c("Quarter", "Revenue", "Growth"))
  expect_equal(df$Quarter, c("Q1-2026", "Q2-2026", "Q3-2026"))
  expect_equal(df$Revenue, c("$12.4M", "$15.1M", "$19.8M"))

  md <- tbl$to_markdown()
  expect_true(grepl("\\| Quarter \\| Revenue \\| Growth \\|", md))
  expect_true(grepl("\\| Q1-2026 \\| \\$12.4M \\| \\+18% \\|", md))
})
