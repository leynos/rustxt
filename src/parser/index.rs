//! Parser for rustdoc crate index pages.

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
        }
    }

    /// Returns a human-readable label for detailed output headings.
    #[must_use]
    pub const fn label(self) -> &'static str {
        match self {
            Self::Module => "Module",
            Self::Struct => "Struct",
            Self::Enum => "Enum",
            Self::Trait => "Trait",
            Self::Function => "Function",
            Self::Type => "Type",
            Self::Constant => "Constant",
            Self::Macro => "Macro",
        }
    }
}

/// Parses a crate index HTML file.
///
/// Malformed or missing sections degrade to empty results rather than
/// failing, because rustdoc output varies between toolchain versions.
#[must_use]
pub fn parse_index(html: &str, crate_name: &str, version: &str) -> CrateIndex {
    CrateIndex {
        name: crate_name.to_owned(),
        version: version.to_owned(),
        description: extract_meta_description(html).unwrap_or_default(),
        docblock: extract_docblock_content(html),
        modules: extract_items(html, ItemKind::Module),
        structs: extract_items(html, ItemKind::Struct),
        enums: extract_items(html, ItemKind::Enum),
        traits: extract_items(html, ItemKind::Trait),
        functions: extract_items(html, ItemKind::Function),
        types: extract_items(html, ItemKind::Type),
        constants: extract_items(html, ItemKind::Constant),
        macros: extract_items(html, ItemKind::Macro),
    }
}

/// Extracts the main docblock content from the HTML.
fn extract_docblock_content(html: &str) -> String {
    // Look for <details class="toggle top-doc" open>
    let Some(start) = html.find(r#"class="toggle top-doc"#) else {
        return String::new();
    };

    let Some(rest) = html.get(start..) else {
        return String::new();
    };

    // Find the docblock div inside
    let Some(docblock_start) = rest.find(r#"class="docblock"#) else {
        return String::new();
    };

    let Some(docblock_rest) = rest.get(docblock_start..) else {
        return String::new();
    };

    // Find the start of the div content (after the >)
    let Some(content_start) = docblock_rest.find('>') else {
        return String::new();
    };

    let Some(content) = docblock_rest.get(content_start + 1..) else {
        return String::new();
    };

    // Find the end of this docblock - look for </details> to get the whole block
    let content_end = content
        .find("</details>")
        .or_else(|| content.find("</div></details>"))
        .unwrap_or_else(|| content.len().min(5000));

    let docblock_html = content.get(..content_end).unwrap_or("");
    html_to_markdown(docblock_html)
}

/// Locates the item-table HTML for a given section, if present.
fn find_item_table(html: &str, section_id: &str) -> Option<String> {
    let section_pattern = format!(r#"id="{section_id}""#);
    let section_start = html.find(&section_pattern)?;
    let rest = html.get(section_start..)?;

    let table_start = rest.find(r#"class="item-table"#)?;
    let table_rest = rest.get(table_start..)?;

    // Find the end of this section (next h2 or end of main content)
    let table_end = table_rest
        .find(r"<h2 id=")
        .or_else(|| table_rest.find("</section>"))
        .unwrap_or(table_rest.len());

    table_rest.get(..table_end).map(ToOwned::to_owned)
}

/// Extracts items of a given kind from the HTML using string-based parsing.
fn extract_items(html: &str, kind: ItemKind) -> Vec<ItemEntry> {
    let Some(table_html) = find_item_table(html, kind.section_id()) else {
        return Vec::new();
    };

    let mut items = Vec::new();
    let mut pos = 0;

    while let Some(found) = table_html.get(pos..).and_then(|s| s.find("<dt")) {
        let abs_start = pos + found;
        let Some(rest) = table_html.get(abs_start..) else {
            break;
        };

        // Find the end of this definition term
        let Some(term_end) = rest.find("</dt>") else {
            break;
        };

        let term_content = rest.get(..term_end).unwrap_or("");
        let next_pos = abs_start + term_end + "</dt>".len();

        // Extract href and name from the anchor
        if let Some((name, href)) = extract_link_from_dt(term_content) {
            let after_term = table_html.get(next_pos..).unwrap_or("");
            items.push(ItemEntry {
                name,
                description: extract_description(after_term),
                path: href,
            });
        }

        pos = next_pos;
    }

    items
}

/// Extracts the `<dd>` description that follows a definition term.
fn extract_description(after_term: &str) -> String {
    let Some(found) = after_term.find("<dd>") else {
        return String::new();
    };

    let content = after_term.get(found + "<dd>".len()..).unwrap_or("");
    content.find("</dd>").map_or_else(String::new, |end| {
        strip_html_tags(content.get(..end).unwrap_or(""))
    })
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
    //! Unit tests for index-page link and description extraction.
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
