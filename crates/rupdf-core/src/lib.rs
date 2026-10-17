pub mod advanced;
pub mod document;
pub mod error;
pub mod geometry;
pub mod mutate;
pub mod page;
pub mod parser;
pub mod render;
pub mod text;

pub use advanced::{
    batch_extract_bytes, batch_render_pages, doc_to_html, doc_to_markdown, doc_to_rag_chunks,
    page_to_html, page_to_markdown, run_benchmark, search_document, BenchmarkMetrics, RagChunk,
};
pub use document::Document;
pub use error::{Result, RupdfError};
pub use geometry::{Matrix, Point, Rect};
pub use mutate::{merge_documents, save_document, serialize_document};
pub use page::Page;
pub use render::{render_page, render_page_svg, Pixmap};
pub use text::{PageLayout, TextBlock, TextLine, TextWord};

/// Returns the version of the `rupdf-core` engine.
pub fn version() -> &'static str {
    env!("CARGO_PKG_VERSION")
}

#[cfg(test)]
mod tests {
    use std::io::Write;

    use flate2::write::ZlibEncoder;
    use flate2::Compression;

    use super::*;
    use crate::parser::filter::{
        decode_ascii85, decode_ascii_hex, decode_flate, decode_run_length,
    };
    use crate::parser::lexer::Lexer;
    use crate::parser::object::{PdfDictExt, PdfObject};

    #[test]
    fn test_version() {
        assert_eq!(version(), "0.1.0");
    }

    #[test]
    fn test_lexer_primitives_and_dicts() {
        let input = br#"
            % PDF comment line
            <<
                /Type /Page
                /Count 42
                /Scale 1.25
                /Active true
                /Empty null
                /Title (Hello \(World\) \101)
                /Hex <48656C6C6F>
                /NameWithHex /A#20B
                /Box [ 0 0 612.0 792 ]
                /Parent 2 0 R
            >>
        "#;
        let mut lexer = Lexer::new(input);
        let obj = lexer.parse_object().expect("should parse dict");
        let dict = obj.as_dict().expect("should be dict");

        assert_eq!(dict.get_name_str(b"Type"), Some("Page"));
        assert_eq!(dict.get_int(b"Count"), Some(42));
        assert_eq!(dict.get_f64(b"Scale"), Some(1.25));
        assert_eq!(
            dict.get(b"Active".as_slice()).and_then(PdfObject::as_bool),
            Some(true)
        );
        assert_eq!(dict.get(b"Empty".as_slice()), Some(&PdfObject::Null));
        assert_eq!(
            dict.get_string_decoded(b"Title").as_deref(),
            Some("Hello (World) A")
        );
        assert_eq!(dict.get_string_decoded(b"Hex").as_deref(), Some("Hello"));
        assert_eq!(dict.get_name_str(b"NameWithHex"), Some("A B"));
        assert_eq!(dict.get_ref(b"Parent"), Some((2, 0)));
        assert_eq!(
            dict.get(b"Box".as_slice()).and_then(PdfObject::as_rect),
            Some([0.0, 0.0, 612.0, 792.0])
        );
    }

    #[test]
    fn test_stream_filters() {
        // FlateDecode
        let raw = b"BT /F1 12 Tf 72 720 Td (Hello rupdf!) Tj ET";
        let mut enc = ZlibEncoder::new(Vec::new(), Compression::default());
        enc.write_all(raw).unwrap();
        let compressed = enc.finish().unwrap();
        let decompressed = decode_flate(&compressed).unwrap();
        assert_eq!(decompressed, raw);

        // ASCIIHexDecode
        let hex_decoded = decode_ascii_hex(b"48 65 6c 6c 6f>").unwrap();
        assert_eq!(hex_decoded, b"Hello");

        // ASCII85Decode
        let a85_decoded = decode_ascii85(b"<~87cURD]i,\"Ebo80~>").unwrap();
        assert_eq!(a85_decoded, b"Hello World!");

        // RunLengthDecode
        let rl_data = [4u8, b'A', b'B', b'C', b'D', b'E', 253u8, b'Z', 128u8];
        let rl_decoded = decode_run_length(&rl_data).unwrap();
        assert_eq!(rl_decoded, b"ABCDEZZZZ");
    }

    #[test]
    fn test_parse_valid_and_broken_xref_pdf() {
        let sample_pdf = b"%PDF-1.7\n\
1 0 obj\n<< /Type /Catalog /Pages 2 0 R >>\nendobj\n\
2 0 obj\n<< /Type /Pages /Kids [3 0 R] /Count 1 /MediaBox [0 0 612 792] >>\nendobj\n\
3 0 obj\n<< /Type /Page /Parent 2 0 R /Rotate 90 /Contents 4 0 R >>\nendobj\n\
4 0 obj\n<< /Length 44 >>\nstream\nBT /F1 12 Tf 72 720 Td (Hello World) Tj ET\nendstream\nendobj\n\
5 0 obj\n<< /Title (Test PDF) /Author (rupdf) >>\nendobj\n\
trailer\n<< /Root 1 0 R /Info 5 0 R /Size 6 >>\n\
startxref\n999999\n%%EOF";

        // Even with a deliberately wrong startxref offset (999999), automatic repair recovers the document!
        let doc = Document::from_bytes(sample_pdf).expect("should parse and repair PDF");
        assert_eq!(doc.page_count(), 1);
        let page = doc.page(0).unwrap();
        assert_eq!(page.mediabox, Rect::new(0.0, 0.0, 612.0, 792.0));
        assert_eq!(page.rotation, 90);
        // Rotated 90 deg -> width is 792, height is 612
        assert_eq!(page.width(), 792.0);
        assert_eq!(page.height(), 612.0);

        let meta = doc.metadata();
        assert_eq!(meta.get("title").map(String::as_str), Some("Test PDF"));
        assert_eq!(meta.get("author").map(String::as_str), Some("rupdf"));

        let content = doc.page_content_bytes(0).unwrap();
        assert!(content.starts_with(b"BT /F1 12 Tf"));
    }

    #[test]
    fn test_end_to_end_core_pipeline_and_benchmark() {
        let metrics = run_benchmark(5).expect("benchmark should succeed");
        assert_eq!(metrics.pages, 5);
        assert!(metrics.pages_per_sec_extract > 0.0);
        assert!(metrics.pages_per_sec_render > 0.0);
    }
}
