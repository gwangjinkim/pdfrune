test_that("Page rendering to RGBA pixmap and SVG work", {
  doc <- pdf_new()
  pdf_new_page(doc, width = 612.0, height = 792.0)
  pdf_insert_text(doc, 1L, c(72, 72), "Vector Rendering Test", fontsize = 20.0)
  pdf_draw_rect(doc, 1L, c(72, 100, 300, 160), color = c(0.1, 0.3, 0.8), fill = c(0.9, 0.95, 1.0), width = 2.0)

  # Render to RGBA pixmap at 72 DPI
  pix <- pdf_render(doc, 1L, dpi = 72.0)
  expect_s3_class(pix, "pdfrune_pixmap")
  expect_equal(pix$width, 612L)
  expect_equal(pix$height, 792L)
  expect_equal(length(pix$bytes), 612L * 792L * 4L)

  # Convert to PNG
  png_bytes <- pix$to_png()
  expect_true(is.raw(png_bytes))
  expect_gt(length(png_bytes), 0)
  # PNG signature: 89 50 4E 47
  expect_equal(as.integer(png_bytes[1:4]), c(137L, 80L, 78L, 71L))

  # Save PNG to disk
  tmp <- tempfile(fileext = ".png")
  pix$save(tmp)
  expect_true(file.exists(tmp))
  expect_gt(file.info(tmp)$size, 0)

  # Export SVG
  svg <- pdf_to_svg(doc, 1L)
  expect_true(is.character(svg))
  expect_true(grepl("<svg", svg))
  expect_true(grepl("</svg>", svg))
})
