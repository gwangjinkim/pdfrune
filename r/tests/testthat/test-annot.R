test_that("Annotations, true redactions, and AcroForms work", {
  doc <- pdf_new()
  pdf_new_page(doc)
  pdf_insert_text(doc, 1L, c(72, 100), "Confidential Patient SSN: 000-11-2222")
  pdf_insert_text(doc, 1L, c(72, 140), "Red Title", fontsize = 16, color = c(1, 0, 0))

  # Add annotations
  h_xref <- pdf_add_highlight_annot(doc, 1L, c(70, 85, 200, 110))
  expect_gt(h_xref, 0L)
  t_xref <- pdf_add_text_annot(doc, 1L, c(250, 100), "Review Note")
  expect_gt(t_xref, 0L)

  ann_df <- pdf_annots(doc, 1L)
  expect_s3_class(ann_df, "data.frame")
  expect_equal(nrow(ann_df), 2L)

  # Delete annotation
  pdf_delete_annot(doc, 1L, t_xref)
  expect_equal(nrow(pdf_annots(doc, 1L)), 1L)

  # True Redaction: permanently scrubs underlying text from content stream
  pdf_add_redact_annot(doc, 1L, c(68, 85, 320, 115), text = "[REDACTED]")
  count <- pdf_apply_redactions(doc, 1L)
  expect_gte(count, 1L)

  scrubbed_text <- pdf_extract_text(doc, 1L)
  expect_false(grepl("000-11-2222", scrubbed_text))

  # AcroForm Widgets
  w_xref <- pdf_add_widget(doc, 1L, "customer_name", c(72, 200, 280, 225), field_type = "text", field_value = "Ada Lovelace")
  expect_gt(w_xref, 0L)

  widgets <- pdf_widgets(doc, 1L)
  expect_equal(nrow(widgets), 1L)
  expect_equal(widgets$field_name, "customer_name")
  expect_equal(widgets$field_value, "Ada Lovelace")

  # Update widget value
  updated <- pdf_set_field_value(doc, 1L, "customer_name", "Grace Hopper")
  expect_true(updated)
  expect_equal(pdf_widgets(doc, 1L)$field_value, "Grace Hopper")
})

test_that("Image insertion and extraction work", {
  doc <- pdf_new()
  pdf_new_page(doc)

  # Create a tiny 2x2 test PNG in R
  # We can render an empty page to PNG to get valid PNG bytes!
  dummy_doc <- pdf_new()
  pdf_new_page(dummy_doc, width = 100, height = 100)
  png_bytes <- pdf_render(dummy_doc, 1L)$to_png()

  img_xref <- pdf_insert_image(doc, 1L, c(72, 100, 200, 200), bytes = png_bytes)
  expect_gt(img_xref, 0L)

  imgs <- pdf_get_images(doc, 1L)
  expect_equal(nrow(imgs), 1L)

  extracted <- pdf_extract_image(doc, img_xref)
  expect_equal(extracted$width, 100L)
  expect_equal(extracted$height, 100L)
  expect_true(is.raw(extracted$image))
  expect_gt(length(extracted$image), 0)
})
