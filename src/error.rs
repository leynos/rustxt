//! Error types for the rustxt application.

use std::io;

/// Errors that can occur when fetching documentation from docs.rs.
#[derive(Debug, thiserror::Error)]
pub enum FetchError {
    /// Failed to make HTTP request to docs.rs.
    #[error("HTTP request failed: {0}")]
    Http(#[from] reqwest::Error),

    /// The requested crate or version was not found on docs.rs.
    #[error("Crate '{name}' version '{version}' not found on docs.rs")]
    CrateNotFound {
        /// The crate name that was not found.
        name: String,
        /// The version that was not found.
        version: String,
    },

    /// Failed to extract the ZIP archive.
    #[error("ZIP extraction failed: {0}")]
    ZipExtraction(#[from] async_zip::error::ZipError),

    /// I/O error during file operations.
    #[error("I/O error: {0}")]
    Io(#[from] io::Error),
}

/// Errors that can occur when parsing rustdoc HTML.
///
/// Parsing itself degrades gracefully on malformed HTML, so the only
/// failure mode is reading documentation files from disk.
#[derive(Debug, thiserror::Error)]
pub enum ParseError {
    /// I/O error when reading HTML files.
    #[error("Failed to read file: {0}")]
    Io(#[from] io::Error),
}

/// Errors that can occur during GPT-4.1 summarization.
#[derive(Debug, thiserror::Error)]
pub enum SummaryError {
    /// `OpenAI` API returned an error.
    #[error("OpenAI API error: {0}")]
    ApiError(String),

    /// Missing API key in environment.
    #[error("OPENAI_API_KEY environment variable not set")]
    MissingApiKey,

    /// `OpenAI` client error.
    #[error("OpenAI client error: {0}")]
    Client(#[from] async_openai::error::OpenAIError),
}

/// Top-level application error type.
#[derive(Debug, thiserror::Error)]
pub enum AppError {
    /// Error fetching documentation.
    #[error(transparent)]
    Fetch(#[from] FetchError),

    /// Error parsing HTML.
    #[error(transparent)]
    Parse(#[from] ParseError),

    /// Error during summarization.
    #[error(transparent)]
    Summary(#[from] SummaryError),

    /// I/O error.
    #[error("I/O error: {0}")]
    Io(#[from] io::Error),
}
