//! Parser for rustdoc crate index pages.

use std::path::Path;

use crate::error::ParseError;
use crate::parser::markdown::{extract_meta_description, html_to_markdown};

/// Represents a parsed crate index page.
#[derive(Debug, Clone)]
pub struct CrateIndex {
    /// The crate name.
    pub name: String,
    /// The crate version.
    pub version: String,
    /// The crate description from the meta tag.
    pub description: String,
    /// The full docblock content converted to Markdown.
    pub docblock: String,
    /// Modules in the crate.
    pub modules: Vec<ItemEntry>,
    /// Structs in the crate.
    pub structs: Vec<ItemEntry>,
    /// Enums in the crate.
    pub enums: Vec<ItemEntry>,
    /// Traits in the crate.
    pub traits: Vec<ItemEntry>,
    /// Functions in the crate.
    pub functions: Vec<ItemEntry>,
    /// Type aliases in the crate.
    pub types: Vec<ItemEntry>,
    /// Constants in the crate.
    pub constants: Vec<ItemEntry>,
    /// Macros in the crate.
    pub macros: Vec<ItemEntry>,
}

/// An entry in an item list (struct, enum, trait, etc.).
#[derive(Debug, Clone)]
pub struct ItemEntry {
    /// The item name.
    pub name: String,
    /// Brief description of the item.
    pub description: String,
    /// Relative path to the item's HTML file.
    pub path: String,
    /// The kind of item (struct, enum, trait, etc.).
    pub kind: ItemKind,
}

/// The kind of documentation item.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ItemKind {
    /// A module.
    Module,
    /// A struct.
    Struct,
    /// An enum.
    Enum,
    /// A trait.
    Trait,
    /// A function.
    Function,
    /// A type alias.
    Type,
    /// A constant.
    Constant,
    /// A macro.
    Macro,
    /// An attribute macro.
    AttrMacro,
    /// A derive macro.
    DeriveMacro,
}

impl ItemKind {
    /// Returns the section ID used in rustdoc HTML.
    #[must_use]
    pub const fn section_id(self) -> &'static str {
        match self {
            Self::Module => "modules",
            Self::Struct => "structs",
            Self::Enum => "enums",
            Self::Trait => "traits",
            Self::Function => "functions",
            Self::Type => "types",
            Self::Constant => "constants",
            Self::Macro => "macros",
            Self::AttrMacro => "attributes",
            Self::DeriveMacro => "derives",
        }
    }
}

/// Parses a crate index HTML file.
///
/// # Errors
///
/// Returns `ParseError` if the HTML structure is invalid or required
/// elements are missing.
pub fn parse_index(html: &str, crate_name: &str, version: &str) -> Result<CrateIndex, ParseError> {
    let description = extract_meta_description(html).unwrap_or_default();
    let docblock = extract_docblock_content(html);
    let modules = extract_items(html, ItemKind::Module);
    let structs = extract_items(html, ItemKind::Struct);
    let enums = extract_items(html, ItemKind::Enum);
    let traits = extract_items(html, ItemKind::Trait);
    let functions = extract_items(html, ItemKind::Function);
    let types = extract_items(html, ItemKind::Type);
    let constants = extract_items(html, ItemKind::Constant);
    let macros = extract_items(html, ItemKind::Macro);

    Ok(CrateIndex {
        name: crate_name.to_owned(),
        version: version.to_owned(),
        description,
        docblock,
        modules,
        structs,
        enums,
        traits,
        functions,
        types,
        constants,
        macros,
    })
}

/// Parses a crate index from an HTML file path.
///
/// # Errors
///
/// Returns `ParseError` if the file cannot be read or parsed.
pub fn parse_index_file(
    path: &Path,
    crate_name: &str,
    version: &str,
) -> Result<CrateIndex, ParseError> {
    let html = std::fs::read_to_string(path)?;
    parse_index(&html, crate_name, version)
}

/// Extracts the main docblock content from the HTML.
fn extract_docblock_content(html: &str) -> String {
    // Look for <details class="toggle top-doc" open>
    let pattern = r#"class="toggle top-doc"#;
    let Some(start) = html.find(pattern) else {
        return String::new();
    };

    let rest = match html.get(start..) {
        Some(r) => r,
        None => return String::new(),
    };

    // Find the docblock div inside
    let docblock_pattern = r#"class="docblock"#;
    let Some(docblock_start) = rest.find(docblock_pattern) else {
        return String::new();
    };

    let docblock_rest = match rest.get(docblock_start..) {
        Some(r) => r,
        None => return String::new(),
    };

    // Find the start of the div content (after the >)
    let Some(content_start) = docblock_rest.find('>') else {
        return String::new();
    };

    let content = match docblock_rest.get(content_start + 1..) {
        Some(c) => c,
        None => return String::new(),
    };

    // Find the end of this docblock - look for </details> to get the whole block
    let content_end = content
        .find("</details>")
        .or_else(|| content.find("</div></details>"))
        .unwrap_or(content.len().min(5000));

    let docblock_html = content.get(..content_end).unwrap_or("");
    html_to_markdown(docblock_html)
}

/// Extracts items of a given kind from the HTML using string-based parsing.
fn extract_items(html: &str, kind: ItemKind) -> Vec<ItemEntry> {
    let section_id = kind.section_id();
    let mut items = Vec::new();

    // Find the section start
    let section_pattern = format!(r#"id="{section_id}""#);
    let Some(section_start) = html.find(&section_pattern) else {
        return items;
    };

    // Find the item-table after this section
    let rest = match html.get(section_start..) {
        Some(r) => r,
        None => return items,
    };

    let Some(table_start) = rest.find(r#"class="item-table"#) else {
        return items;
    };

    let table_rest = match rest.get(table_start..) {
        Some(r) => r,
        None => return items,
    };

    // Find the end of this section (next h2 or end of main content)
    let table_end = table_rest
        .find(r#"<h2 id="#)
        .or_else(|| table_rest.find("</section>"))
        .unwrap_or(table_rest.len());

    let table_html = table_rest.get(..table_end).unwrap_or("");

    // Parse dt/dd pairs
    let mut pos = 0;
    while let Some(dt_start) = table_html.get(pos..).and_then(|s| s.find("<dt")) {
        let abs_dt_start = pos + dt_start;
        let dt_rest = match table_html.get(abs_dt_start..) {
            Some(r) => r,
            None => break,
        };

        // Find the end of this dt
        let Some(dt_end) = dt_rest.find("</dt>") else {
            break;
        };

        let dt_content = dt_rest.get(..dt_end).unwrap_or("");

        // Extract href and name from the anchor
        if let Some((name, href)) = extract_link_from_dt(dt_content) {
            // Look for the dd after this dt
            let dd_start_pos = abs_dt_start + dt_end + 5; // 5 = len("</dt>")
            let dd_rest = table_html.get(dd_start_pos..).unwrap_or("");

            let description = if let Some(dd_start) = dd_rest.find("<dd>") {
                let dd_content = dd_rest.get(dd_start + 4..).unwrap_or("");
                if let Some(dd_end) = dd_content.find("</dd>") {
                    strip_html_tags(dd_content.get(..dd_end).unwrap_or(""))
                } else {
                    String::new()
                }
            } else {
                String::new()
            };

            items.push(ItemEntry {
                name,
                description,
                path: href,
                kind,
            });
        }

        pos = abs_dt_start + dt_end + 5;
    }

    items
}

fn extract_link_from_dt(dt_html: &str) -> Option<(String, String)> {
    // Find href="..."
    let href_start = dt_html.find(r#"href=""#)? + 6;
    let href_rest = dt_html.get(href_start..)?;
    let href_end = href_rest.find('"')?;
    let href = href_rest.get(..href_end)?;

    // Find the link text (between > and </a>)
    // Look for the last > before </a>
    let anchor_end = dt_html.rfind("</a>")?;
    let before_anchor = dt_html.get(..anchor_end)?;
    let text_start = before_anchor.rfind('>')? + 1;
    let name = before_anchor.get(text_start..)?;

    // Clean up the name (remove <wbr> tags, etc.)
    let clean_name = name
        .replace("<wbr>", "")
        .replace("&lt;", "<")
        .replace("&gt;", ">")
        .replace("&amp;", "&");

    Some((clean_name, href.to_owned()))
}

fn strip_html_tags(html: &str) -> String {
    let mut result = String::with_capacity(html.len());
    let mut in_tag = false;

    for ch in html.chars() {
        match ch {
            '<' => in_tag = true,
            '>' => in_tag = false,
            _ if !in_tag => result.push(ch),
            _ => {}
        }
    }

    result
        .replace("&lt;", "<")
        .replace("&gt;", ">")
        .replace("&amp;", "&")
        .replace("&quot;", "\"")
        .replace("&#39;", "'")
        .trim()
        .to_owned()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_extract_link_from_dt() {
        let dt = r#"<dt><a class="trait" href="trait.FluentBuilder.html" title="trait">FluentBuilder</a></dt>"#;
        let result = extract_link_from_dt(dt);
        assert_eq!(
            result,
            Some((
                "FluentBuilder".to_owned(),
                "trait.FluentBuilder.html".to_owned()
            ))
        );
    }

    #[test]
    fn test_strip_html_tags() {
        let html = "A <code>helper</code> trait for building.";
        let result = strip_html_tags(html);
        assert_eq!(result, "A helper trait for building.");
    }
}
