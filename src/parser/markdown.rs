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

/// Extracts the description from a `<meta name="description">` tag.
#[must_use]
pub fn extract_meta_description(html: &str) -> Option<String> {
    let pattern = r#"<meta name="description" content=""#;
    let start = html.find(pattern)?;
    let content_start = start + pattern.len();
    let rest = html.get(content_start..)?;
    let end = rest.find('"')?;
    rest.get(..end).map(decode_html_entities)
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
        remaining = convert_single_link(after_a, &mut result);
    }

    result.push_str(remaining);
    result
}

/// Converts one anchor element, appending Markdown to `result`.
///
/// Returns the remaining HTML after the anchor. When the anchor is
/// malformed, the tag is skipped and its text is left in place.
fn convert_single_link<'a>(after_a: &'a str, result: &mut String) -> &'a str {
    let href = extract_href(after_a).unwrap_or("");

    // Find link text
    if let Some(text_start) = after_a.find('>') {
        let after_text = after_a.get(text_start + 1..).unwrap_or("");
        if let Some(text_end) = after_text.find("</a>") {
            let link_text = strip_html_tags(after_text.get(..text_end).unwrap_or(""));
            push_markdown_link(result, &link_text, href);
            return after_text.get(text_end + 4..).unwrap_or("");
        }
    }

    // Fallback: skip the tag
    after_a
        .find('>')
        .and_then(|end| after_a.get(end + 1..))
        .unwrap_or("")
}

/// Extracts the `href` attribute value from an anchor tag.
fn extract_href(anchor_html: &str) -> Option<&str> {
    let href_start = anchor_html.find("href=\"")?;
    let href_content = anchor_html.get(href_start + 6..)?;
    let href_end = href_content.find('"')?;
    href_content.get(..href_end)
}

/// Appends a Markdown link, or bare text when the target is empty.
fn push_markdown_link(result: &mut String, link_text: &str, href: &str) {
    if href.is_empty() {
        result.push_str(link_text);
    } else {
        result.push('[');
        result.push_str(link_text);
        result.push_str("](");
        result.push_str(href);
        result.push(')');
    }
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
    //! Unit tests for HTML-to-Markdown conversion.
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
        let html =
            r#"<html><head><meta name="description" content="Test description"></head></html>"#;
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
