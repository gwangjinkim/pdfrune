test_that("ISO 32000 RC4-128 encryption and authentication work", {
  doc <- pdf_new()
  pdf_new_page(doc)
  pdf_insert_text(doc, 1L, c(72, 100), "Confidential Financial Formula 2026", fontsize = 16.0)

  enc_bytes <- pdf_to_bytes(
    doc,
    deflate = TRUE,
    encryption = "rc4-128",
    user_pw = "secret_user_pw",
    owner_pw = "master_owner_pw"
  )

  # Plaintext string must NOT appear in raw bytes
  ascii_bytes <- enc_bytes[enc_bytes >= as.raw(32) & enc_bytes <= as.raw(126)]
  expect_false(grepl("Confidential Financial Formula", rawToChar(ascii_bytes), fixed = TRUE))

  # Open encrypted PDF
  enc_doc <- pdf_open(bytes = enc_bytes)
  expect_true(pdf_is_encrypted(enc_doc))

  # Wrong password fails
  expect_false(pdf_authenticate(enc_doc, "wrong_password"))

  # Correct password succeeds
  expect_true(pdf_authenticate(enc_doc, "secret_user_pw"))

  # Text is decrypted successfully
  dec_text <- pdf_extract_text(enc_doc, 1L)
  expect_true(grepl("Confidential Financial Formula 2026", dec_text))
})
