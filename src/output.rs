//! Output formatting for the final documentation.

use crate::parser::CrateDocs;

/// Represents the final output to be written to stdout.
pub struct CrateOutput {
    /// The crate name.
    pub name: String,
    /// The crate version.
    pub version: String,
    /// The GPT-4.1 generated summary (if enabled).
    pub summary: Option<String>,
    /// The raw documentation (overview, modules, types).
    pub documentation: String,
}

impl CrateOutput {
    /// Creates output with summarization.
    #[must_use]
    pub fn with_summary(docs: &CrateDocs, summary: String) -> Self {
        Self {
            name: docs.index.name.clone(),
            version: docs.index.version.clone(),
            summary: Some(summary),
            documentation: crate::parser::format_docs_summary(docs),
        }
    }

    /// Creates output without summarization.
    #[must_use]
    pub fn without_summary(docs: &CrateDocs) -> Self {
        Self {
            name: docs.index.name.clone(),
            version: docs.index.version.clone(),
            summary: None,
            documentation: crate::parser::format_docs_summary(docs),
        }
    }

    /// Formats the output as Markdown.
    #[must_use]
    pub fn to_markdown(&self) -> String {
        let mut output = String::new();

        self.push_header(&mut output);

        // Summary section (if available)
        if let Some(summary) = &self.summary {
            output.push_str("## Summary\n\n");
            output.push_str(summary);
            output.push_str("\n\n---\n\n");
        }

        // Full documentation
        output.push_str("## Documentation\n\n");
        output.push_str(&self.documentation);

        output
    }

    /// Formats the output as a compact version (summary only if available).
    #[must_use]
    pub fn to_compact_markdown(&self) -> String {
        self.summary.as_ref().map_or_else(
            || self.to_markdown(),
            |summary| {
                let mut output = String::new();
                self.push_header(&mut output);
                output.push_str(summary);
                output
            },
        )
    }

    /// Appends the `# name vversion` header.
    fn push_header(&self, output: &mut String) {
        output.push_str("# ");
        output.push_str(&self.name);
        output.push_str(" v");
        output.push_str(&self.version);
        output.push_str("\n\n");
    }
}

/// Writes output to stdout.
///
/// # Errors
///
/// Returns an I/O error if writing fails.
pub fn write_output(output: &CrateOutput) -> std::io::Result<()> {
    use std::io::Write;

    let mut stdout = std::io::stdout().lock();
    writeln!(stdout, "{}", output.to_markdown())?;
    Ok(())
}

/// Writes compact output to stdout (summary only when available).
///
/// # Errors
///
/// Returns an I/O error if writing fails.
pub fn write_compact_output(output: &CrateOutput) -> std::io::Result<()> {
    use std::io::Write;

    let mut stdout = std::io::stdout().lock();
    writeln!(stdout, "{}", output.to_compact_markdown())?;
    Ok(())
}

#[cfg(test)]
mod tests {
    //! Unit tests for output formatting with and without summaries.
    use super::*;
    use crate::parser::{CrateDocs, CrateIndex};

    fn make_test_docs() -> CrateDocs {
        CrateDocs {
            index: CrateIndex {
                name: "test_crate".to_owned(),
                version: "1.0.0".to_owned(),
                description: "A test crate".to_owned(),
                docblock: String::new(),
                modules: vec![],
                structs: vec![],
                enums: vec![],
                traits: vec![],
                functions: vec![],
                types: vec![],
                constants: vec![],
                macros: vec![],
            },
            items: vec![],
        }
    }

    #[test]
    fn test_output_with_summary() {
        let docs = make_test_docs();
        let output = CrateOutput::with_summary(&docs, "This is a summary.".to_owned());
        let md = output.to_markdown();

        assert!(md.contains("# test_crate v1.0.0"));
        assert!(md.contains("## Summary"));
        assert!(md.contains("This is a summary."));
    }

    #[test]
    fn test_output_without_summary() {
        let docs = make_test_docs();
        let output = CrateOutput::without_summary(&docs);
        let md = output.to_markdown();

        assert!(md.contains("# test_crate v1.0.0"));
        assert!(!md.contains("## Summary"));
    }
}
