## Resubmission of pdfrune 0.1.0

This is a resubmission addressing the feedback from the CRAN incoming pre-tests.

### Summary of Fixes:

* **Foreign function calls (WARNING fixed)**:
  - Fixed `pdf_insert_text()` in `R/annot.R` which previously passed 9 arguments to `C_pdfrune_insert_text` instead of 7. The RGB color is now passed as a single length-3 numeric vector as registered in `src/init.c`.

* **Rust compilation (WARNING fixed)**:
  - All dependency Rust crates are now vendored offline inside `src/rust/vendor.tar.xz`.
  - `src/Makevars` and `src/Makevars.win` now compile with `--offline` using vendored sources without downloading anything from the internet.
  - Added `configure`, `configure.win`, and `configure.ucrt` scripts, which check and report `cargo` and `rustc` versions prior to compilation.
  - Added `-j 2` concurrency limit to respect CRAN CPU limits.

* **Compiled code (WARNING fixed)**:
  - Added internal hidden shims in `src/init.c` for `abort`, `exit`, and `_exit` that redirect abnormal termination safely to R's `Rf_error()`, ensuring compiled code cannot abruptly terminate the R session.
  - Cleaned up static library archives and target build directories immediately upon linking so no detritus is left in the build tree.

* **CRAN incoming feasibility (NOTE addressed)**:
  - Created `inst/WORDLIST` containing verified technical terminology: `AGPL`, `AcroForm`, `RGBA`, `rasterization`, `redactions`.
  - Quoted acronyms and proper names in the `DESCRIPTION` file.

### Package Size:
* Source package tarball is ~6.9 MB due to vendored Rust dependencies (`vendor.tar.xz`) required for self-contained, offline CRAN compilation.
