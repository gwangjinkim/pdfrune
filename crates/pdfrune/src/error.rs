use thiserror::Error;

/// Unified error type for all `pdfrune` operations.
#[derive(Debug, Error)]
pub enum PdfruneError {
    #[error("I/O error: {0}")]
    Io(#[from] std::io::Error),

    #[error("Invalid PDF header or file format: {0}")]
    InvalidFormat(String),

    #[error("Syntax error at byte offset {offset}: {message}")]
    SyntaxError { offset: usize, message: String },

    #[error("Missing or invalid XREF table: {0}")]
    XrefError(String),

    #[error("Object ({0}, {1}) not found")]
    ObjectNotFound(u32, u16),

    #[error("Stream decompression error ({filter}): {message}")]
    StreamError { filter: String, message: String },

    #[error("Page index {index} out of bounds (page count: {count})")]
    PageOutOfBounds { index: usize, count: usize },

    #[error("Encryption / password error: {0}")]
    EncryptionError(String),

    #[error("Rendering error: {0}")]
    RenderError(String),

    #[error("Invalid operation or argument: {0}")]
    InvalidArgument(String),
}

pub type Result<T> = std::result::Result<T, PdfruneError>;
