//! HTML to Markdown conversion utilities for rustdoc HTML.
//!
//! This module provides string-based HTML-to-Markdown conversion for rustdoc HTML.
//! It uses simple parsing rather than a full DOM, which is sufficient for the
//! well-structured output of rustdoc.

/// Converts rustdoc HTML to lightweight Markdown.
///
/// This function handles common rustdoc HTML elements and produces clean
/// Markdown suitable for LLM consumption.
#[must_use]
pub fn html_to_markdown(html: &str) -> String {
    let mut result = html.to_owned();

    // Remove script and style tags
    result = remove_tag_content(&result, "script");
    result = remove_tag_content(&result, "style");
    result = remove_tag_content(&result, "noscript");

    // Convert code blocks
    result = result.replace("<pre class=\"rust item-decl\">", "\n```rust\n");
    result = result.replace("<pre>", "\n```\n");
    result = result.replace("</pre>", "\n```\n");

    // Convert inline code
    result = result.replace("<code>", "`");
    result = result.replace("</code>", "`");

    // Convert headers
    result = result.replace("<h1>", "\n# ");
    result = result.replace("</h1>", "\n");
    result = result.replace("<h2>", "\n## ");
    result = result.replace("</h2>", "\n");
    result = result.replace("<h3>", "\n### ");
    result = result.replace("</h3>", "\n");
    result = result.replace("<h4>", "\n#### ");
    result = result.replace("</h4>", "\n");

    // Convert emphasis
    result = result.replace("<strong>", "**");
    result = result.replace("</strong>", "**");
    result = result.replace("<b>", "**");
    result = result.replace("</b>", "**");
    result = result.replace("<em>", "*");
    result = result.replace("</em>", "*");
    result = result.replace("<i>", "*");
    result = result.replace("</i>", "*");

    // Convert paragraphs and line breaks
    result = result.replace("<p>", "\n\n");
    result = result.replace("</p>", "\n\n");
    result = result.replace("<br>", "\n");
    result = result.replace("<br/>", "\n");
    result = result.replace("<br />", "\n");

    // Convert list items
    result = result.replace("<li>", "\n- ");
    result = result.replace("</li>", "");
    result = result.replace("<ul>", "\n");
    result = result.replace("</ul>", "\n");
    result = result.replace("<ol>", "\n");
    result = result.replace("</ol>", "\n");

    // Convert links - simplified approach
    result = convert_links(&result);

    // Strip remaining HTML tags
    result = strip_html_tags(&result);

    // Decode HTML entities
    result = decode_html_entities(&result);

    // Clean up whitespace
    cleanup_markdown(&result)
}

/// Extracts text content from a docblock HTML element.
#[must_use]
pub fn extract_docblock(html: &str) -> String {
    html_to_markdown(html)
}

/// Extracts the description from a `<meta name="description">` tag.
#[must_use]
pub fn extract_meta_description(html: &str) -> Option<String> {
    let pattern = r#"<meta name="description" content=""#;
    let start = html.find(pattern)?;
    let content_start = start + pattern.len();
    let rest = html.get(content_start..)?;
    let end = rest.find('"')?;
    rest.get(..end).map(|s| decode_html_entities(s))
}

/// Extracts the page title from the `<title>` tag.
#[must_use]
pub fn extract_title(html: &str) -> Option<String> {
    let start = html.find("<title>")?;
    let title_start = start + "<title>".len();
    let rest = html.get(title_start..)?;
    let end = rest.find("</title>")?;
    rest.get(..end).map(|s| decode_html_entities(s))
}

/// Removes content within a specific tag, including the tags themselves.
fn remove_tag_content(html: &str, tag: &str) -> String {
    let open_tag = format!("<{tag}");
    let close_tag = format!("</{tag}>");

    let mut result = String::with_capacity(html.len());
    let mut remaining = html;

    while let Some(start) = remaining.find(&open_tag) {
        result.push_str(remaining.get(..start).unwrap_or(""));

        let after_open = remaining.get(start..).unwrap_or("");
        if let Some(end) = after_open.find(&close_tag) {
            remaining = after_open.get(end + close_tag.len()..).unwrap_or("");
        } else {
            // No closing tag found, skip to end of opening tag
            if let Some(tag_end) = after_open.find('>') {
                remaining = after_open.get(tag_end + 1..).unwrap_or("");
            } else {
                break;
            }
        }
    }

    result.push_str(remaining);
    result
}

/// Converts HTML links to Markdown links.
fn convert_links(html: &str) -> String {
    let mut result = String::with_capacity(html.len());
    let mut remaining = html;

    while let Some(start) = remaining.find("<a ") {
        result.push_str(remaining.get(..start).unwrap_or(""));

        let after_a = remaining.get(start..).unwrap_or("");

        // Find href
        let href = if let Some(href_start) = after_a.find("href=\"") {
            let href_content = after_a.get(href_start + 6..).unwrap_or("");
            if let Some(href_end) = href_content.find('"') {
                href_content.get(..href_end).unwrap_or("")
            } else {
                ""
            }
        } else {
            ""
        };

        // Find link text
        if let Some(text_start) = after_a.find('>') {
            let after_text = after_a.get(text_start + 1..).unwrap_or("");
            if let Some(text_end) = after_text.find("</a>") {
                let link_text = strip_html_tags(after_text.get(..text_end).unwrap_or(""));

                if href.is_empty() {
                    result.push_str(&link_text);
                } else {
                    result.push('[');
                    result.push_str(&link_text);
                    result.push_str("](");
                    result.push_str(href);
                    result.push(')');
                }

                remaining = after_text.get(text_end + 4..).unwrap_or("");
                continue;
            }
        }

        // Fallback: skip the tag
        if let Some(end) = after_a.find('>') {
            remaining = after_a.get(end + 1..).unwrap_or("");
        } else {
            break;
        }
    }

    result.push_str(remaining);
    result
}

/// Strips all HTML tags from text.
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
}

/// Decodes HTML entities.
fn decode_html_entities(text: &str) -> String {
    text.replace("&lt;", "<")
        .replace("&gt;", ">")
        .replace("&amp;", "&")
        .replace("&quot;", "\"")
        .replace("&#39;", "'")
        .replace("&apos;", "'")
        .replace("&nbsp;", " ")
}

/// Cleans up Markdown by removing excessive whitespace.
fn cleanup_markdown(text: &str) -> String {
    let mut result = String::with_capacity(text.len());
    let mut newline_count = 0;
    let mut last_was_space = false;

    for ch in text.chars() {
        if ch == '\n' {
            newline_count += 1;
            last_was_space = false;
            if newline_count <= 2 {
                result.push(ch);
            }
        } else if ch.is_whitespace() {
            newline_count = 0;
            if !last_was_space {
                result.push(' ');
                last_was_space = true;
            }
        } else {
            newline_count = 0;
            last_was_space = false;
            result.push(ch);
        }
    }

    result.trim().to_owned()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_simple_paragraph() {
        let html = "<p>Hello world</p>";
        let md = html_to_markdown(html);
        assert!(md.contains("Hello world"));
    }

    #[test]
    fn test_code_inline() {
        let html = "<p>Use <code>foo()</code> here</p>";
        let md = html_to_markdown(html);
        assert!(md.contains("`foo()`"));
    }

    #[test]
    fn test_extract_meta_description() {
        let html = r#"<html><head><meta name="description" content="Test description"></head></html>"#;
        let desc = extract_meta_description(html);
        assert_eq!(desc, Some("Test description".to_owned()));
    }

    #[test]
    fn test_strip_html_tags() {
        let html = "A <code>helper</code> trait for building.";
        let result = strip_html_tags(html);
        assert_eq!(result, "A helper trait for building.");
    }

    #[test]
    fn test_convert_links() {
        let html = r#"See <a href="https://example.com">example</a> for more."#;
        let result = convert_links(html);
        assert!(result.contains("[example](https://example.com)"));
    }
}
