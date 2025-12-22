//! Parser for individual rustdoc item pages (structs, enums, traits, etc.).

use std::path::Path;

use crate::error::ParseError;
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
    /// Fields (for structs).
    pub fields: Vec<FieldDoc>,
    /// Variants (for enums).
    pub variants: Vec<VariantDoc>,
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

/// Documentation for a struct field.
#[derive(Debug, Clone)]
pub struct FieldDoc {
    /// Field name.
    pub name: String,
    /// Field type.
    pub field_type: String,
    /// Field description.
    pub description: String,
}

/// Documentation for an enum variant.
#[derive(Debug, Clone)]
pub struct VariantDoc {
    /// Variant name.
    pub name: String,
    /// Variant signature (including fields if any).
    pub signature: String,
    /// Variant description.
    pub description: String,
}

/// Parses an item page from HTML content.
///
/// # Errors
///
/// Returns `ParseError` if the HTML structure is invalid.
pub fn parse_item(html: &str, kind: ItemKind) -> Result<ItemDoc, ParseError> {
    let name = extract_item_name(html).unwrap_or_default();
    let description = extract_meta_description(html).unwrap_or_default();
    let signature = extract_signature(html);
    let docblock = extract_item_docblock(html);

    let full_description = if docblock.is_empty() {
        description
    } else {
        docblock
    };

    let methods = extract_methods(html);
    let fields = extract_fields(html);
    let variants = extract_variants(html);

    Ok(ItemDoc {
        kind,
        name,
        signature,
        description: full_description,
        methods,
        fields,
        variants,
    })
}

/// Parses an item page from a file path.
///
/// # Errors
///
/// Returns `ParseError` if the file cannot be read or parsed.
pub fn parse_item_file(path: &Path, kind: ItemKind) -> Result<ItemDoc, ParseError> {
    let html = std::fs::read_to_string(path)?;
    parse_item(&html, kind)
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
    let name = title
        .split(" in ")
        .next()?
        .trim()
        .replace("<wbr>", "");
    Some(name)
}

/// Extracts the type signature/declaration.
fn extract_signature(html: &str) -> String {
    // Look for <pre class="rust item-decl">
    let pattern = r#"class="rust item-decl"#;
    let Some(start) = html.find(pattern) else {
        return String::new();
    };

    let rest = match html.get(start..) {
        Some(r) => r,
        None => return String::new(),
    };

    let Some(code_start) = rest.find("<code>") else {
        return String::new();
    };

    let code_rest = match rest.get(code_start + 6..) {
        Some(r) => r,
        None => return String::new(),
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

    // Find the end of this docblock (</div>)
    let content_end = find_matching_close_div(content).unwrap_or(content.len().min(3000));
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

/// Extracts method documentation.
fn extract_methods(html: &str) -> Vec<MethodDoc> {
    let mut methods = Vec::new();

    // Look for method sections: <section id="method.name">
    let pattern = r#"<section id="method."#;
    let mut pos = 0;

    while let Some(section_start) = html.get(pos..).and_then(|s| s.find(pattern)) {
        let abs_start = pos + section_start;
        let rest = match html.get(abs_start..) {
            Some(r) => r,
            None => break,
        };

        // Extract method name from id="method.name"
        let id_start = r#"id="method."#.len();
        let id_rest = match rest.get(id_start..) {
            Some(r) => r,
            None => break,
        };

        let id_end = match id_rest.find('"') {
            Some(e) => e,
            None => break,
        };

        let method_name = match id_rest.get(..id_end) {
            Some(n) => n.to_owned(),
            None => break,
        };

        // Find the method signature in <h4 class="code-header">
        let sig = extract_method_signature(rest).unwrap_or_default();

        // Find the method docblock
        let desc = extract_method_docblock(rest).unwrap_or_default();

        methods.push(MethodDoc {
            name: method_name,
            signature: sig,
            description: desc,
        });

        // Move past this section
        pos = abs_start + 20; // Skip past the section start
    }

    methods
}

fn extract_method_signature(section_html: &str) -> Option<String> {
    // Look for <h4 class="code-header">
    let pattern = r#"class="code-header"#;
    let start = section_html.find(pattern)?;
    let rest = section_html.get(start..)?;

    let content_start = rest.find('>')? + 1;
    let content = rest.get(content_start..)?;

    let content_end = content.find("</h4>")?;
    let sig_html = content.get(..content_end)?;

    Some(clean_signature(sig_html))
}

fn extract_method_docblock(section_html: &str) -> Option<String> {
    // Look for docblock after the method signature
    let pattern = r#"class="docblock"#;
    let start = section_html.find(pattern)?;
    let rest = section_html.get(start..)?;

    let content_start = rest.find('>')? + 1;
    let content = rest.get(content_start..)?;

    // Limit to a reasonable size (first paragraph)
    let end = content
        .find("</p>")
        .map(|p| p + 4)
        .unwrap_or_else(|| content.len().min(500));

    let docblock_html = content.get(..end)?;
    Some(html_to_markdown(docblock_html))
}

/// Extracts field documentation for structs.
fn extract_fields(html: &str) -> Vec<FieldDoc> {
    let mut fields = Vec::new();

    // Look for <h2 id="fields">
    let fields_section = match html.find(r#"id="fields"#) {
        Some(pos) => match html.get(pos..) {
            Some(s) => s,
            None => return fields,
        },
        None => return fields,
    };

    // Find the end of fields section (next h2 or implementations)
    let section_end = fields_section
        .find(r#"<h2 id="implementations"#)
        .or_else(|| fields_section.find(r#"<h2 id="trait"#))
        .unwrap_or(fields_section.len());

    let fields_html = match fields_section.get(..section_end) {
        Some(h) => h,
        None => return fields,
    };

    // Parse structfield spans
    let pattern = r#"id="structfield."#;
    let mut pos = 0;

    while let Some(field_start) = fields_html.get(pos..).and_then(|s| s.find(pattern)) {
        let abs_start = pos + field_start;
        let rest = match fields_html.get(abs_start..) {
            Some(r) => r,
            None => break,
        };

        // Extract field name
        let id_start = r#"id="structfield."#.len();
        let id_rest = match rest.get(id_start..) {
            Some(r) => r,
            None => break,
        };

        let id_end = match id_rest.find('"') {
            Some(e) => e,
            None => break,
        };

        let field_name = match id_rest.get(..id_end) {
            Some(n) => n.to_owned(),
            None => break,
        };

        fields.push(FieldDoc {
            name: field_name,
            field_type: String::new(),
            description: String::new(),
        });

        pos = abs_start + 20;
    }

    fields
}

/// Extracts variant documentation for enums.
fn extract_variants(html: &str) -> Vec<VariantDoc> {
    let mut variants = Vec::new();

    // Look for <h2 id="variants">
    let variants_section = match html.find(r#"id="variants"#) {
        Some(pos) => match html.get(pos..) {
            Some(s) => s,
            None => return variants,
        },
        None => return variants,
    };

    // Find the end of variants section
    let section_end = variants_section
        .find(r#"<h2 id="implementations"#)
        .or_else(|| variants_section.find(r#"<h2 id="trait"#))
        .unwrap_or(variants_section.len());

    let variants_html = match variants_section.get(..section_end) {
        Some(h) => h,
        None => return variants,
    };

    // Parse variant sections
    let pattern = r#"id="variant."#;
    let mut pos = 0;

    while let Some(variant_start) = variants_html.get(pos..).and_then(|s| s.find(pattern)) {
        let abs_start = pos + variant_start;
        let rest = match variants_html.get(abs_start..) {
            Some(r) => r,
            None => break,
        };

        // Extract variant name
        let id_start = r#"id="variant."#.len();
        let id_rest = match rest.get(id_start..) {
            Some(r) => r,
            None => break,
        };

        let id_end = match id_rest.find('"') {
            Some(e) => e,
            None => break,
        };

        let variant_name = match id_rest.get(..id_end) {
            Some(n) => n.to_owned(),
            None => break,
        };

        variants.push(VariantDoc {
            name: variant_name,
            signature: String::new(),
            description: String::new(),
        });

        pos = abs_start + 15;
    }

    variants
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_clean_signature() {
        let html = r##"pub trait <a href="#">FluentBuilder</a> { ... }"##;
        let clean = clean_signature(html);
        assert_eq!(clean, "pub trait FluentBuilder { ... }");
    }

    #[test]
    fn test_extract_item_name() {
        let html = r#"<title>FluentBuilder in gpui::prelude - Rust</title>"#;
        let name = extract_item_name(html);
        assert_eq!(name, Some("FluentBuilder".to_owned()));
    }
}
