pub mod modify;
pub mod writer;

pub use modify::merge_documents;
pub use writer::{save_document, serialize_document, serialize_document_encrypted};
