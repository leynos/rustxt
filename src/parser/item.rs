//! Parser for individual rustdoc item pages (structs, enums, traits, etc.).

use crate::parser::index::ItemKind;
use crate::parser::markdown::{extract_meta_description, html_to_markdown};

/// Parsed documentation for a single item (struct, enum, trait, etc.).
#[derive(Debug, Clone)]
pub struct ItemDoc {
    /// The kind of item.
    pub kind: ItemKind,
    /// The item name.
    pub name: String,
    /// The type signature/declaration.
    pub signature: String,
    /// The item description (from docblock).
    pub description: String,
    /// Methods or associated items.
    pub methods: Vec<MethodDoc>,
    /// Field names (for structs).
    pub fields: Vec<String>,
    /// Variant names (for enums).
    pub variants: Vec<String>,
}

/// Documentation for a method.
#[derive(Debug, Clone)]
pub struct MethodDoc {
    /// Method name.
    pub name: String,
    /// Method signature.
    pub signature: String,
    /// Brief description.
    pub description: String,
}

/// Parses an item page from HTML content.
///
/// Missing or malformed sections degrade to empty results rather than
/// failing, because rustdoc output varies between toolchain versions.
#[must_use]
pub fn parse_item(html: &str, kind: ItemKind) -> ItemDoc {
    let name = extract_item_name(html).unwrap_or_default();
    let description = extract_meta_description(html).unwrap_or_default();
    let signature = extract_signature(html);
    let docblock = extract_item_docblock(html);

    let full_description = if docblock.is_empty() {
        description
    } else {
        docblock
    };

    ItemDoc {
        kind,
        name,
        signature,
        description: full_description,
        methods: extract_methods(html),
        fields: extract_section_names(html, r#"id="fields"#, r#"id="structfield."#),
        variants: extract_section_names(html, r#"id="variants"#, r#"id="variant."#),
    }
}

/// Extracts the item name from the title or heading.
fn extract_item_name(html: &str) -> Option<String> {
    // Look for <h1>Type <span>Name</span>
    // Or extract from title: "Name in crate::module - Rust"
    let start = html.find("<title>")?;
    let title_start = start + 7;
    let rest = html.get(title_start..)?;
    let end = rest.find(" - Rust").or_else(|| rest.find("</title>"))?;
    let title = rest.get(..end)?;
    // Title format: "Name in module - Rust" or just "Name - Rust"
    let name = title.split(" in ").next()?.trim().replace("<wbr>", "");
    Some(name)
}

/// Extracts the type signature/declaration.
fn extract_signature(html: &str) -> String {
    // Look for <pre class="rust item-decl">
    let Some(start) = html.find(r#"class="rust item-decl"#) else {
        return String::new();
    };

    let Some(rest) = html.get(start..) else {
        return String::new();
    };

    let Some(code_start) = rest.find("<code>") else {
        return String::new();
    };

    let Some(code_rest) = rest.get(code_start + 6..) else {
        return String::new();
    };

    let Some(code_end) = code_rest.find("</code>") else {
        return String::new();
    };

    let code_html = code_rest.get(..code_end).unwrap_or("");
    clean_signature(code_html)
}

/// Cleans up a signature by removing HTML tags and normalizing whitespace.
fn clean_signature(html: &str) -> String {
    let mut result = String::with_capacity(html.len());
    let mut in_tag = false;
    let mut last_was_space = false;

    for ch in html.chars() {
        match ch {
            '<' => in_tag = true,
            '>' => in_tag = false,
            _ if in_tag => {}
            _ if ch.is_whitespace() => {
                if !last_was_space {
                    result.push(' ');
                    last_was_space = true;
                }
            }
            _ => {
                result.push(ch);
                last_was_space = false;
            }
        }
    }

    result
        .replace("&lt;", "<")
        .replace("&gt;", ">")
        .replace("&amp;", "&")
        .replace("&quot;", "\"")
        .trim()
        .to_owned()
}

/// Extracts the main docblock content for the item.
fn extract_item_docblock(html: &str) -> String {
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

    // Find the end of this docblock (</div>)
    let content_end = find_matching_close_div(content).unwrap_or_else(|| content.len().min(3000));
    let docblock_html = content.get(..content_end).unwrap_or("");
    html_to_markdown(docblock_html)
}

/// Finds the position of the matching </div> tag, accounting for nesting.
fn find_matching_close_div(html: &str) -> Option<usize> {
    let mut depth = 1;
    let mut pos = 0;

    while pos < html.len() {
        if html.get(pos..)?.starts_with("<div") {
            depth += 1;
            pos += 4;
        } else if html.get(pos..)?.starts_with("</div>") {
            depth -= 1;
            if depth == 0 {
                return Some(pos);
            }
            pos += 6;
        } else {
            pos += 1;
        }
    }

    None
}

/// Extracts the value that follows an `id="prefix"` attribute occurrence.
///
/// Returns the identifier text and the absolute position just past the
/// match, or `None` when no further occurrence exists.
fn next_id_value<'a>(html: &'a str, pattern: &str, pos: usize) -> Option<(&'a str, usize)> {
    let found = html.get(pos..)?.find(pattern)?;
    let abs_start = pos + found;
    let rest = html.get(abs_start..)?;

    let id_rest = rest.get(pattern.len()..)?;
    let id_end = id_rest.find('"')?;
    let value = id_rest.get(..id_end)?;

    Some((value, abs_start + pattern.len()))
}

/// Extracts method documentation.
fn extract_methods(html: &str) -> Vec<MethodDoc> {
    let mut methods = Vec::new();
    let pattern = r#"<section id="method."#;
    let mut pos = 0;

    while let Some(found) = html.get(pos..).and_then(|s| s.find(pattern)) {
        let abs_start = pos + found;
        let Some(rest) = html.get(abs_start..) else {
            break;
        };

        let Some((method_name, _)) = next_id_value(rest, r#"id="method."#, 0) else {
            break;
        };

        methods.push(MethodDoc {
            name: method_name.to_owned(),
            signature: extract_method_signature(rest).unwrap_or_default(),
            description: extract_method_docblock(rest).unwrap_or_default(),
        });

        // Move past this section
        pos = abs_start + pattern.len();
    }

    methods
}

fn extract_method_signature(section_html: &str) -> Option<String> {
    // Look for <h4 class="code-header">
    let start = section_html.find(r#"class="code-header"#)?;
    let rest = section_html.get(start..)?;

    let content_start = rest.find('>')? + 1;
    let content = rest.get(content_start..)?;

    let content_end = content.find("</h4>")?;
    let sig_html = content.get(..content_end)?;

    Some(clean_signature(sig_html))
}

fn extract_method_docblock(section_html: &str) -> Option<String> {
    // Look for docblock after the method signature
    let start = section_html.find(r#"class="docblock"#)?;
    let rest = section_html.get(start..)?;

    let content_start = rest.find('>')? + 1;
    let content = rest.get(content_start..)?;

    // Limit to a reasonable size (first paragraph)
    let end = content
        .find("</p>")
        .map_or_else(|| content.len().min(500), |p| p + 4);

    let docblock_html = content.get(..end)?;
    Some(html_to_markdown(docblock_html))
}

/// Extracts the identifier names within a page section.
///
/// Used for struct fields (`id="structfield.*"` within `id="fields"`) and
/// enum variants (`id="variant.*"` within `id="variants"`).
fn extract_section_names(html: &str, section_anchor: &str, id_pattern: &str) -> Vec<String> {
    let Some(section_pos) = html.find(section_anchor) else {
        return Vec::new();
    };

    let Some(section) = html.get(section_pos..) else {
        return Vec::new();
    };

    // Find the end of the section (next implementations block)
    let section_end = section
        .find(r#"<h2 id="implementations"#)
        .or_else(|| section.find(r#"<h2 id="trait"#))
        .unwrap_or(section.len());

    let Some(section_html) = section.get(..section_end) else {
        return Vec::new();
    };

    let mut names = Vec::new();
    let mut pos = 0;

    while let Some((name, next_pos)) = next_id_value(section_html, id_pattern, pos) {
        names.push(name.to_owned());
        pos = next_pos;
    }

    names
}

#[cfg(test)]
mod tests {
    //! Unit tests for item-page signature and name extraction.
    use super::*;

    #[test]
    fn test_clean_signature() {
        let html = r##"pub trait <a href="#">FluentBuilder</a> { ... }"##;
        let clean = clean_signature(html);
        assert_eq!(clean, "pub trait FluentBuilder { ... }");
    }

    #[test]
    fn test_extract_item_name() {
        let html = r"<title>FluentBuilder in gpui::prelude - Rust</title>";
        let name = extract_item_name(html);
        assert_eq!(name, Some("FluentBuilder".to_owned()));
    }

    #[test]
    fn test_extract_section_names() {
        let html = concat!(
            r#"<h2 id="variants">Variants</h2>"#,
            r#"<section id="variant.Alpha" class="variant">Alpha</section>"#,
            r#"<section id="variant.Beta" class="variant">Beta</section>"#,
            r#"<h2 id="implementations">Implementations</h2>"#,
        );
        let names = extract_section_names(html, r#"id="variants"#, r#"id="variant."#);
        assert_eq!(names, vec!["Alpha".to_owned(), "Beta".to_owned()]);
    }
}
