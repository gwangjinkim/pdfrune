pub mod markdown;
pub mod rag;
pub mod search;
pub mod table;

pub use markdown::{doc_to_html, doc_to_markdown, page_to_html, page_to_markdown};
pub use rag::{doc_to_rag_chunks, RagChunk};
pub use search::{
    batch_extract_bytes, batch_render_pages, run_benchmark, search_document, BenchmarkMetrics,
};
pub use table::{find_page_tables, DetectedTable};
