# rustxt

A command-line tool that fetches Rust crate documentation from
[docs.rs](https://docs.rs) and converts it to clean, LLM-friendly Markdown.
Optionally summarizes documentation using GPT-4.1 for quick comprehension.

## What it does

Instead of dumping a list of links, rustxt gives you **comprehensive, readable documentation**:

- Downloads the full rustdoc HTML archive from docs.rs
- Converts HTML to lightweight Markdown
- Extracts crate overview, modules, structs, traits, enums, and functions
- Parses key type details including method signatures
- Optionally summarizes everything with GPT-4.1 for quick understanding

Perfect for feeding crate documentation into LLMs, offline reading, or just
getting a quick overview of an unfamiliar crate.

## Installation

### From source

```bash
git clone https://github.com/leynos/rustxt
cd rustxt
cargo build --release
```

The binary will be at `target/release/rustxt`.

### Requirements

- Rust 2024 edition (nightly)
- For summarization: OpenAI API key with GPT-4.1 access

## Usage

### Basic usage (no API key needed)

```bash
# Get documentation for the latest version
rustxt --no-summary clap

# Get documentation for a specific version
rustxt --no-summary --crate-version 4.5.0 clap
```

### With GPT-4.1 summarization

```bash
# Set your OpenAI API key
export OPENAI_API_KEY=sk-...

# Get summarized documentation
rustxt clap

# Get only the summary (compact mode)
rustxt --compact clap
```

### Example output

```markdown
# clap v4.5.0

## Summary
Clap is a full-featured command line argument parser for Rust. It supports
both derive-based and builder-based APIs for defining CLI interfaces...

## Documentation

### Overview
Command Line Argument Parser for Rust with support for subcommands,
shell completions, and automatic help generation.

### Modules
- **builder**: Builder API for constructing CLI parsers
- **error**: Error types for argument parsing failures
...

### Structs
- **Command**: The main entry point for building a CLI application
- **Arg**: Represents a command-line argument
...
```

## CLI Options

```text
Usage: rustxt [OPTIONS] <CRATE_NAME>

Arguments:
  <CRATE_NAME>  Name of the crate to fetch documentation for

Options:
      --crate-version <VERSION>  Specific crate version (defaults to latest)
      --no-summary               Skip GPT-4.1 summarization
      --compact                  Output only the summary (requires API key)
  -h, --help                     Print help
  -V, --version                  Print version
```

## How it works

1. **Fetch**: Downloads the rustdoc ZIP archive from `docs.rs/crate/{name}/{version}/download`
2. **Extract**: Unpacks the BZIP2-compressed archive to a temporary directory
3. **Parse**: Walks through the HTML files, extracting:
   - Crate description from meta tags
   - Module structure and descriptions
   - Public types (structs, enums, traits) with their documentation
   - Method signatures and descriptions for key types
4. **Convert**: Transforms HTML docblocks to clean Markdown
5. **Summarize** (optional): Sends the extracted documentation to GPT-4.1
   for a concise, actionable summary
6. **Output**: Formats everything as Markdown to stdout

## Configuration

### OpenAI API Key

For summarization features, set your API key:

```bash
export OPENAI_API_KEY=sk-your-key-here
```

If the API key is not set, rustxt will:

- Work normally with `--no-summary`
- Print a warning and fall back to raw documentation without `--no-summary`

### Model

The tool uses `gpt-4.1` by default, which supports up to 1 million tokens
of context - enough for even the largest crate documentation.

## Use cases

- **LLM context**: Pipe crate docs directly into your AI assistant
- **Offline reading**: Save documentation as Markdown for offline reference
- **Quick learning**: Get GPT-4.1 summaries to understand new crates fast
- **Documentation aggregation**: Build custom documentation collections

## Examples

```bash
# Explore a UI framework
rustxt --no-summary gpui > gpui-docs.md

# Understand async runtime internals
rustxt tokio --compact

# Get full serde documentation with summary
rustxt serde > serde-complete.md

# Check a specific version for migration planning
rustxt --crate-version 0.11 hyper --no-summary
```

## Building

```bash
# Debug build
cargo build

# Release build (recommended)
cargo build --release

# Run tests
cargo test

# Run with strict lints
cargo clippy
```

## Project structure

```text
src/
  main.rs           # CLI entry point and orchestration
  error.rs          # Error types (FetchError, ParseError, SummaryError)
  fetcher.rs        # docs.rs ZIP download and extraction
  output.rs         # Markdown output formatting
  summarizer.rs     # GPT-4.1 API integration
  parser/
    mod.rs          # Parser orchestration
    index.rs        # Crate index page parsing
    item.rs         # Individual type page parsing
    markdown.rs     # HTML-to-Markdown conversion
```

## Dependencies

- **reqwest**: HTTP client for downloading from docs.rs
- **async_zip**: Async ZIP extraction with BZIP2 support
- **async-openai**: OpenAI API client for GPT-4.1
- **clap**: CLI argument parsing
- **tokio**: Async runtime
- **eyre**: Error handling

## Limitations

- Requires network access to docs.rs
- Only works with crates published to crates.io
- GPT-4.1 summarization requires an OpenAI API key
- Very large crates may take a moment to download and parse

## License

ISC

## Contributing

Contributions welcome! Please feel free to submit issues and pull requests.

---

Built with Rust. Documentation powered by [docs.rs](https://docs.rs).
