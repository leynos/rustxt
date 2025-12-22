//! HTML parsing module for rustdoc documentation.
//!
//! This module provides utilities for parsing rustdoc HTML files and
//! converting them to structured data and Markdown.

pub mod index;
pub mod item;
pub mod markdown;

use std::path::Path;

use crate::error::ParseError;

pub use index::{CrateIndex, ItemKind};
pub use item::ItemDoc;

/// Complete parsed documentation for a crate.
#[derive(Debug, Clone)]
pub struct CrateDocs {
    /// The crate index (overview, module list, etc.).
    pub index: CrateIndex,
    /// Parsed documentation for key types.
    pub items: Vec<ItemDoc>,
}

/// Parses all documentation from a rustdoc output directory.
///
/// # Arguments
///
/// * `docs_root` - Path to the crate's documentation root (containing `index.html`).
/// * `crate_name` - The crate name.
/// * `version` - The crate version.
///
/// # Errors
///
/// Returns `ParseError` if files cannot be read or parsed.
pub fn parse_crate_docs(
    docs_root: &Path,
    crate_name: &str,
    version: &str,
) -> Result<CrateDocs, ParseError> {
    // Parse the index
    let index_path = docs_root.join("index.html");
    let crate_index = index::parse_index_file(&index_path, crate_name, version)?;

    // Parse key items (limit to avoid overwhelming output)
    let mut items = Vec::new();

    // Parse important structs (limit to first 10)
    for entry in crate_index.structs.iter().take(10) {
        let item_path = docs_root.join(&entry.path);
        if item_path.exists() {
            if let Ok(item_doc) = item::parse_item_file(&item_path, ItemKind::Struct) {
                items.push(item_doc);
            }
        }
    }

    // Parse important traits (limit to first 5)
    for entry in crate_index.traits.iter().take(5) {
        let item_path = docs_root.join(&entry.path);
        if item_path.exists() {
            if let Ok(item_doc) = item::parse_item_file(&item_path, ItemKind::Trait) {
                items.push(item_doc);
            }
        }
    }

    // Parse important enums (limit to first 5)
    for entry in crate_index.enums.iter().take(5) {
        let item_path = docs_root.join(&entry.path);
        if item_path.exists() {
            if let Ok(item_doc) = item::parse_item_file(&item_path, ItemKind::Enum) {
                items.push(item_doc);
            }
        }
    }

    Ok(CrateDocs {
        index: crate_index,
        items,
    })
}

/// Returns a summary of the crate suitable for LLM consumption.
///
/// This produces a text representation that includes:
/// - Crate name and version
/// - Description
/// - Module overview
/// - Key types with brief descriptions
#[must_use]
pub fn format_docs_summary(docs: &CrateDocs) -> String {
    let mut output = String::new();

    // Header
    output.push_str(&format!("# {} v{}\n\n", docs.index.name, docs.index.version));

    // Description
    if !docs.index.description.is_empty() {
        output.push_str("## Overview\n\n");
        output.push_str(&docs.index.description);
        output.push_str("\n\n");
    }

    // Docblock if different from description
    if !docs.index.docblock.is_empty() && docs.index.docblock != docs.index.description {
        output.push_str(&docs.index.docblock);
        output.push_str("\n\n");
    }

    // Modules
    if !docs.index.modules.is_empty() {
        output.push_str("## Modules\n\n");
        for module in &docs.index.modules {
            output.push_str(&format!("- **{}**", module.name));
            if !module.description.is_empty() {
                output.push_str(&format!(": {}", module.description));
            }
            output.push('\n');
        }
        output.push('\n');
    }

    // Key structs
    if !docs.index.structs.is_empty() {
        output.push_str("## Structs\n\n");
        for s in docs.index.structs.iter().take(15) {
            output.push_str(&format!("- **{}**", s.name));
            if !s.description.is_empty() {
                output.push_str(&format!(": {}", s.description));
            }
            output.push('\n');
        }
        output.push('\n');
    }

    // Key traits
    if !docs.index.traits.is_empty() {
        output.push_str("## Traits\n\n");
        for t in docs.index.traits.iter().take(10) {
            output.push_str(&format!("- **{}**", t.name));
            if !t.description.is_empty() {
                output.push_str(&format!(": {}", t.description));
            }
            output.push('\n');
        }
        output.push('\n');
    }

    // Key enums
    if !docs.index.enums.is_empty() {
        output.push_str("## Enums\n\n");
        for e in docs.index.enums.iter().take(10) {
            output.push_str(&format!("- **{}**", e.name));
            if !e.description.is_empty() {
                output.push_str(&format!(": {}", e.description));
            }
            output.push('\n');
        }
        output.push('\n');
    }

    // Functions
    if !docs.index.functions.is_empty() {
        output.push_str("## Functions\n\n");
        for f in docs.index.functions.iter().take(10) {
            output.push_str(&format!("- **{}**", f.name));
            if !f.description.is_empty() {
                output.push_str(&format!(": {}", f.description));
            }
            output.push('\n');
        }
        output.push('\n');
    }

    // Detailed item documentation
    if !docs.items.is_empty() {
        output.push_str("## Key Types (Detailed)\n\n");
        for item in &docs.items {
            output.push_str(&format!("### {}\n\n", item.name));

            if !item.signature.is_empty() {
                output.push_str("```rust\n");
                output.push_str(&item.signature);
                output.push_str("\n```\n\n");
            }

            if !item.description.is_empty() {
                output.push_str(&item.description);
                output.push_str("\n\n");
            }

            // Methods summary
            if !item.methods.is_empty() {
                output.push_str("**Key methods:**\n");
                for method in item.methods.iter().take(5) {
                    output.push_str(&format!("- `{}`", method.name));
                    if !method.description.is_empty() {
                        let brief = method
                            .description
                            .lines()
                            .next()
                            .unwrap_or(&method.description);
                        output.push_str(&format!(": {brief}"));
                    }
                    output.push('\n');
                }
                output.push('\n');
            }
        }
    }

    output
}
