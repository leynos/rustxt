//! HTML parsing module for rustdoc documentation.
//!
//! This module provides utilities for parsing rustdoc HTML files and
//! converting them to structured data and Markdown.

pub mod index;
pub mod item;
pub mod markdown;

use std::path::Path;

use cap_std::ambient_authority;
use cap_std::fs::Dir;

use crate::error::ParseError;

use index::ItemEntry;
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
/// Returns `ParseError` if the documentation root or its index cannot be read.
pub fn parse_crate_docs(
    docs_root: &Path,
    crate_name: &str,
    version: &str,
) -> Result<CrateDocs, ParseError> {
    // Scope filesystem access to the documentation root via a capability
    // handle; item pages are read relative to this directory only.
    let dir = Dir::open_ambient_dir(docs_root, ambient_authority())?;

    let index_html = dir.read_to_string("index.html")?;
    let crate_index = index::parse_index(&index_html, crate_name, version);

    // Parse key items (limit to avoid overwhelming output)
    let mut items = collect_items(&dir, &crate_index.structs, ItemKind::Struct, 10);
    items.extend(collect_items(&dir, &crate_index.traits, ItemKind::Trait, 5));
    items.extend(collect_items(&dir, &crate_index.enums, ItemKind::Enum, 5));

    Ok(CrateDocs {
        index: crate_index,
        items,
    })
}

/// Parses up to `limit` item pages, skipping entries that cannot be read.
fn collect_items(dir: &Dir, entries: &[ItemEntry], kind: ItemKind, limit: usize) -> Vec<ItemDoc> {
    entries
        .iter()
        .take(limit)
        .filter_map(|entry| dir.read_to_string(&entry.path).ok())
        .map(|html| item::parse_item(&html, kind))
        .collect()
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

    push_header(&mut output, &docs.index.name, &docs.index.version);
    push_overview(&mut output, &docs.index);
    push_entry_section(&mut output, "Modules", &docs.index.modules, usize::MAX);
    push_entry_section(&mut output, "Structs", &docs.index.structs, 15);
    push_entry_section(&mut output, "Traits", &docs.index.traits, 10);
    push_entry_section(&mut output, "Enums", &docs.index.enums, 10);
    push_entry_section(&mut output, "Functions", &docs.index.functions, 10);
    push_entry_section(&mut output, "Type Aliases", &docs.index.types, 10);
    push_entry_section(&mut output, "Constants", &docs.index.constants, 10);
    push_entry_section(&mut output, "Macros", &docs.index.macros, 10);
    push_item_details(&mut output, &docs.items);

    output
}

/// Appends the `# name vversion` header.
fn push_header(output: &mut String, name: &str, version: &str) {
    output.push_str("# ");
    output.push_str(name);
    output.push_str(" v");
    output.push_str(version);
    output.push_str("\n\n");
}

/// Appends the crate overview and docblock sections.
fn push_overview(output: &mut String, index: &CrateIndex) {
    if !index.description.is_empty() {
        output.push_str("## Overview\n\n");
        output.push_str(&index.description);
        output.push_str("\n\n");
    }

    // Docblock if different from description
    if !index.docblock.is_empty() && index.docblock != index.description {
        output.push_str(&index.docblock);
        output.push_str("\n\n");
    }
}

/// Appends a `## heading` section listing up to `limit` entries.
fn push_entry_section(output: &mut String, heading: &str, entries: &[ItemEntry], limit: usize) {
    if entries.is_empty() {
        return;
    }

    output.push_str("## ");
    output.push_str(heading);
    output.push_str("\n\n");

    for entry in entries.iter().take(limit) {
        output.push_str("- **");
        output.push_str(&entry.name);
        output.push_str("**");
        if !entry.description.is_empty() {
            output.push_str(": ");
            output.push_str(&entry.description);
        }
        output.push('\n');
    }

    output.push('\n');
}

/// Appends the detailed documentation for each parsed item.
fn push_item_details(output: &mut String, items: &[ItemDoc]) {
    if items.is_empty() {
        return;
    }

    output.push_str("## Key Types (Detailed)\n\n");
    for item in items {
        output.push_str("### ");
        output.push_str(item.kind.label());
        output.push(' ');
        output.push_str(&item.name);
        output.push_str("\n\n");

        if !item.signature.is_empty() {
            output.push_str("```rust\n");
            output.push_str(&item.signature);
            output.push_str("\n```\n\n");
        }

        if !item.description.is_empty() {
            output.push_str(&item.description);
            output.push_str("\n\n");
        }

        push_name_list(output, "**Fields:** ", &item.fields);
        push_name_list(output, "**Variants:** ", &item.variants);
        push_method_summary(output, &item.methods);
    }
}

/// Appends a labelled, comma-separated list of names, if any.
fn push_name_list(output: &mut String, label: &str, names: &[String]) {
    if names.is_empty() {
        return;
    }

    output.push_str(label);
    output.push_str(&names.join(", "));
    output.push_str("\n\n");
}

/// Appends a bullet list summarizing up to five methods.
fn push_method_summary(output: &mut String, methods: &[item::MethodDoc]) {
    if methods.is_empty() {
        return;
    }

    output.push_str("**Key methods:**\n");
    for method in methods.iter().take(5) {
        output.push_str("- `");
        if method.signature.is_empty() {
            output.push_str(&method.name);
        } else {
            output.push_str(&method.signature);
        }
        output.push('`');
        if !method.description.is_empty() {
            let brief = method
                .description
                .lines()
                .next()
                .unwrap_or(&method.description);
            output.push_str(": ");
            output.push_str(brief);
        }
        output.push('\n');
    }
    output.push('\n');
}
