Generated renderer fixtures; no third-party PDF/book content.

- scan.jpg: 16x16 RGB, top eight rows (240,20,10), bottom eight
  (10,30,240), generated using Pillow quality=100, subsampling=0.
- scan.jp2: the same pixels, lossless JPEG2000.
- mask.jbig2: embedded JBIG2 PageInformation + immediate generic MMR region
  + EndOfPage. A 16x16 pattern with eight white pixels followed by eight
  black pixels in every row. Group4 bits: horizontal 001; white run 8
  10011; black run 8 000101; then two vertical-zero codes 11 for each
  subsequent row; EOFB 000000000001 twice, zero-padded to a byte.

These small fixtures exercise the native Rust decoders independently of the
large external Archive.org regression books.
