//! CLI tool for fetching Rust crate documentation in LLM-friendly format.
//!
//! This tool downloads rustdoc HTML from docs.rs, converts it to Markdown,
//! and optionally summarizes it using GPT-4.1 for optimal LLM consumption.

mod error;
mod fetcher;
mod output;
mod parser;
mod summarizer;

use clap::Parser;
use eyre::{Result, WrapErr};

use crate::error::AppError;
use crate::fetcher::{find_docs_root, DocsFetcher};
use crate::output::CrateOutput;
use crate::parser::parse_crate_docs;
use crate::summarizer::Summarizer;

/// Fetch Rust crate documentation formatted for LLM consumption.
#[derive(Parser, Debug)]
#[command(name = "rustxt", version, about)]
struct Args {
    /// Name of the crate to fetch documentation for.
    crate_name: String,

    /// Specific crate version to fetch (defaults to latest).
    #[arg(long = "crate-version")]
    crate_version: Option<String>,

    /// Skip GPT-4.1 summarization (output raw converted docs).
    #[arg(long)]
    no_summary: bool,

    /// Output only the summary (when summarization is enabled).
    #[arg(long)]
    compact: bool,
}

#[tokio::main]
async fn main() -> Result<()> {
    let args = Args::parse();

    // Fetch documentation
    let fetcher = DocsFetcher::new();
    let (temp_dir, version) = fetcher
        .fetch(&args.crate_name, args.crate_version.as_deref())
        .await
        .map_err(AppError::from)
        .wrap_err_with(|| format!("Failed to fetch documentation for '{}'", args.crate_name))?;

    // Find the docs root directory
    let docs_root = find_docs_root(&temp_dir, &args.crate_name).ok_or_else(|| {
        eyre::eyre!(
            "Could not find documentation root for '{}' in downloaded archive",
            args.crate_name
        )
    })?;

    // Parse the documentation
    let crate_docs = parse_crate_docs(&docs_root, &args.crate_name, &version)
        .map_err(AppError::from)
        .wrap_err("Failed to parse documentation HTML")?;

    // Create output
    let output = if args.no_summary {
        CrateOutput::without_summary(&crate_docs)
    } else {
        // Summarize with GPT-4.1
        match Summarizer::from_env() {
            Ok(summarizer) => {
                let summary = summarizer
                    .summarize(&crate_docs)
                    .await
                    .map_err(AppError::from)
                    .wrap_err("Failed to summarize documentation")?;

                CrateOutput::with_summary(&crate_docs, summary)
            }
            Err(crate::error::SummaryError::MissingApiKey) => {
                eprintln!(
                    "Warning: OPENAI_API_KEY not set, skipping summarization. \
                     Use --no-summary to suppress this warning."
                );
                CrateOutput::without_summary(&crate_docs)
            }
            Err(e) => return Err(AppError::from(e).into()),
        }
    };

    // Write output
    if args.compact {
        output::write_compact_output(&output).wrap_err("Failed to write output")?;
    } else {
        output::write_output(&output).wrap_err("Failed to write output")?;
    }

    Ok(())
}
