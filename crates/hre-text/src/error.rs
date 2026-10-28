//! Error types for text normalization and analysis.

/// Errors occurring during text validation and processing.
#[derive(Debug, thiserror::Error, PartialEq, Eq)]
pub enum TextError {
    /// Input text violates bounds (empty or exceeds 32768 bytes).
    #[error("validation error: {0}")]
    Validation(String),

    /// Text processing or backend inference failed.
    #[error("analysis error: {0}")]
    Analysis(String),
}
