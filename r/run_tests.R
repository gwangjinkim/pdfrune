library(testthat)
library(pdfrune)

results <- test_dir("tests/testthat", reporter = "summary")
if (any(as.data.frame(results)$failed > 0)) {
  stop("Some tests failed!", call. = FALSE)
}
cat("\nALL R TESTS PASSED SUCCESSFULLY!\n")
