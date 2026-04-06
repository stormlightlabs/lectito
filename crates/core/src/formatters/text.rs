use crate::metadata::Metadata;
use crate::{Result, utils};
use scraper::{ElementRef, Html};

const BLOCK_ELEMENTS: [&str; 13] = [
    "p",
    "div",
    "h1",
    "h2",
    "h3",
    "h4",
    "h5",
    "h6",
    "li",
    "blockquote",
    "pre",
    "td",
    "th",
];

/// Configuration for plain text output
#[derive(Debug, Clone, Default)]
pub struct TextConfig {
    /// Preserve paragraph structure with double newlines
    pub preserve_paragraphs: bool,

    /// Wrap lines at specified width (0 = no wrapping)
    pub line_width: usize,

    /// Include metadata header
    pub include_header: bool,
}

/// Plain text formatter for converting HTML to readable plain text
pub struct TextFormatter {
    config: TextConfig,
}

impl TextFormatter {
    pub fn new(config: TextConfig) -> Self {
        Self { config }
    }

    /// Convert HTML content to plain text
    pub fn convert(&self, html: &str, metadata: &Metadata) -> Result<String> {
        convert_to_text(html, metadata, &self.config)
    }
}

/// Convert HTML content to plain text with specified configuration
pub fn convert_to_text(html: &str, metadata: &Metadata, config: &TextConfig) -> Result<String> {
    let mut output = String::new();

    if config.include_header {
        output.push_str(&generate_header(metadata));
        output.push_str("\n\n");
    }

    let text = html_to_text(html, config.preserve_paragraphs);

    let final_text = if config.line_width > 0 { wrap_text(&text, config.line_width) } else { text };

    output.push_str(&final_text);

    Ok(output.trim().to_string())
}

pub(crate) fn html_to_text(html: &str, preserve_paragraphs: bool) -> String {
    if preserve_paragraphs {
        extract_text_with_paragraphs(html).unwrap_or_else(|_| extract_plain_text(html))
    } else {
        extract_plain_text(html)
    }
}

/// Generate a header from metadata
fn generate_header(metadata: &Metadata) -> String {
    let mut header = String::new();

    if let Some(title) = &metadata.title {
        header.push_str(title);
        header.push('\n');
        header.push_str(&"=".repeat(title.len()));
        header.push('\n');
    }

    let mut meta_parts = Vec::new();

    if let Some(author) = &metadata.author {
        meta_parts.push(format!("By: {}", author));
    }

    if let Some(date) = &metadata.date {
        meta_parts.push(format!("Date: {}", date));
    }

    if let Some(site) = &metadata.site_name {
        meta_parts.push(format!("Site: {}", site));
    }

    if !meta_parts.is_empty() {
        header.push_str(&meta_parts.join(" | "));
        header.push('\n');
    }

    header
}

/// Extract plain text from HTML, stripping all tags
fn extract_plain_text(html: &str) -> String {
    html_to_text_with_blocks(html).join(" ")
}

/// Extract text from HTML while preserving paragraph structure
fn extract_text_with_paragraphs(html: &str) -> Result<String> {
    Ok(html_to_text_with_blocks(html).join("\n\n"))
}

fn html_to_text_with_blocks(html: &str) -> Vec<String> {
    let document = Html::parse_document(html);
    let root = document
        .select(&scraper::Selector::parse("body").unwrap())
        .next()
        .unwrap_or_else(|| document.root_element());

    let mut blocks = Vec::<String>::new();
    collect_text_blocks(root, &mut blocks);

    if blocks.is_empty() {
        let fallback = normalize_block_text(&root.text().collect::<String>());
        if !fallback.is_empty() {
            blocks.push(fallback);
        }
    }

    blocks
}

fn collect_text_blocks(element: ElementRef<'_>, blocks: &mut Vec<String>) {
    let mut inline_buffer = String::new();
    collect_text_segments(element, blocks, &mut inline_buffer);
    flush_inline_buffer(&mut inline_buffer, blocks);
}

fn collect_text_segments(element: ElementRef<'_>, blocks: &mut Vec<String>, inline_buffer: &mut String) {
    for child in element.children() {
        if let Some(text) = child.value().as_text() {
            append_inline_text(inline_buffer, &text.text);
            continue;
        }

        let Some(child_element) = ElementRef::wrap(child) else {
            continue;
        };

        if is_block_element(&child_element) {
            flush_inline_buffer(inline_buffer, blocks);

            if has_block_descendants(&child_element) {
                collect_text_blocks(child_element, blocks);
            } else {
                push_text_block(&child_element.text().collect::<String>(), blocks);
            }
        } else {
            collect_text_segments(child_element, blocks, inline_buffer);
        }
    }
}

fn push_text_block(text: &str, blocks: &mut Vec<String>) {
    let text = normalize_block_text(text);
    if text.is_empty() {
        return;
    }

    if blocks
        .last()
        .is_some_and(|last| utils::normalize_whitespace(last) == utils::normalize_whitespace(&text))
    {
        return;
    }

    blocks.push(text);
}

fn flush_inline_buffer(inline_buffer: &mut String, blocks: &mut Vec<String>) {
    if inline_buffer.trim().is_empty() {
        inline_buffer.clear();
        return;
    }

    push_text_block(inline_buffer, blocks);
    inline_buffer.clear();
}

fn append_inline_text(buffer: &mut String, text: &str) {
    if !buffer.is_empty() && !buffer.ends_with(char::is_whitespace) {
        buffer.push(' ');
    }
    buffer.push_str(text);
}

fn has_block_descendants(element: &ElementRef<'_>) -> bool {
    element
        .descendants()
        .skip(1)
        .filter_map(ElementRef::wrap)
        .any(|child| is_block_element(&child))
}

fn is_block_element(element: &ElementRef<'_>) -> bool {
    BLOCK_ELEMENTS.contains(&element.value().name())
}

fn normalize_block_text(text: &str) -> String {
    let mut normalized = String::new();
    let mut previous_was_whitespace = false;

    for ch in text.trim().chars() {
        let is_breaking_whitespace = ch.is_whitespace() && ch != '\u{00A0}';
        if is_breaking_whitespace {
            if !previous_was_whitespace {
                normalized.push(' ');
                previous_was_whitespace = true;
            }
        } else {
            normalized.push(ch);
            previous_was_whitespace = false;
        }
    }

    normalized.trim().to_string()
}

/// Wrap text to specified line width
fn wrap_text(text: &str, width: usize) -> String {
    if width == 0 {
        return text.to_string();
    }

    let mut result = Vec::new();
    let mut current_line = String::new();
    let mut current_length = 0;

    for word in text.split_whitespace() {
        let word_len = word.len();

        if current_length == 0 {
            current_line.push_str(word);
            current_length = word_len;
        } else if current_length + 1 + word_len <= width {
            current_line.push(' ');
            current_line.push_str(word);
            current_length += 1 + word_len;
        } else {
            result.push(current_line);
            current_line = word.to_string();
            current_length = word_len;
        }
    }

    if !current_line.is_empty() {
        result.push(current_line);
    }

    let paragraphs: Vec<&str> = text.split("\n\n").collect();
    if paragraphs.len() > 1 {
        paragraphs
            .iter()
            .map(|p| {
                p.lines()
                    .map(|line| {
                        let words: Vec<&str> = line.split_whitespace().collect();
                        if words.is_empty() { String::new() } else { wrap_words(&words, width) }
                    })
                    .collect::<Vec<_>>()
                    .join("\n")
            })
            .collect::<Vec<_>>()
            .join("\n\n")
    } else {
        result.join("\n")
    }
}

/// Wrap a slice of words to specified width
fn wrap_words(words: &[&str], width: usize) -> String {
    let mut lines = Vec::new();
    let mut current_line = Vec::new();
    let mut current_length = 0;

    for &word in words {
        let word_len = word.len();

        if current_length == 0 {
            current_line.push(word);
            current_length = word_len;
        } else if current_length + 1 + word_len <= width {
            current_length += 1 + word_len;
            current_line.push(word);
        } else {
            lines.push(current_line.join(" "));
            current_line = vec![word];
            current_length = word_len;
        }
    }

    if !current_line.is_empty() {
        lines.push(current_line.join(" "));
    }

    lines.join("\n")
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_extract_plain_text() {
        let html = r#"<h1>Title</h1><p>This is a paragraph.</p>"#;
        let text = extract_plain_text(html);
        assert!(text.contains("Title"));
        assert!(text.contains("This is a paragraph."));
    }

    #[test]
    fn test_extract_plain_text_strips_tags() {
        let html = r#"<p>Text with <strong>bold</strong> and <em>italic</em>.</p>"#;
        let text = extract_plain_text(html);
        assert!(!text.contains("<strong>"));
        assert!(!text.contains("<em>"));
        assert!(text.contains("bold"));
        assert!(text.contains("italic"));
    }

    #[test]
    fn test_extract_text_with_paragraphs() {
        let html = r#"
            <p>First paragraph with some content.</p>
            <p>Second paragraph with more content.</p>
        "#;

        let result = extract_text_with_paragraphs(html);
        assert!(result.is_ok());
        let text = result.unwrap();
        assert!(text.contains("First paragraph"));
        assert!(text.contains("Second paragraph"));
        assert!(text.contains("\n\n"));
    }

    #[test]
    fn test_extract_text_with_headings() {
        let html = r#"<h1>Main Title</h1><p>Content goes here.</p>"#;
        let result = extract_text_with_paragraphs(html);
        assert!(result.is_ok());
        let text = result.unwrap();
        assert_eq!(text, "Main Title\n\nContent goes here.");
    }

    #[test]
    fn test_extract_text_with_mixed_container_content() {
        let html = r#"
            <article>
                <div>Here's a demo: <a href="https://example.com">https://example.com</a><p>Second block.</p></div>
            </article>
        "#;

        let result = extract_text_with_paragraphs(html);
        assert!(result.is_ok());
        let text = result.unwrap();
        assert_eq!(text, "Here's a demo: https://example.com\n\nSecond block.");
    }

    #[test]
    fn test_generate_header_with_all_fields() {
        let metadata = Metadata {
            title: Some("Test Article".to_string()),
            author: Some("John Doe".to_string()),
            date: Some("2024-01-15".to_string()),
            site_name: Some("Example Site".to_string()),
            ..Default::default()
        };

        let header = generate_header(&metadata);
        assert!(header.contains("Test Article"));
        assert!(header.contains("==="));
        assert!(header.contains("By: John Doe"));
        assert!(header.contains("Date: 2024-01-15"));
        assert!(header.contains("Site: Example Site"));
    }

    #[test]
    fn test_generate_header_with_title_only() {
        let metadata = Metadata { title: Some("Solo Title".to_string()), ..Default::default() };

        let header = generate_header(&metadata);
        assert!(header.contains("Solo Title"));
        assert!(header.contains("==="));
        assert!(!header.contains("By:"));
        assert!(!header.contains("Date:"));
    }

    #[test]
    fn test_convert_to_text_basic() {
        let html = r#"<p>Simple paragraph with some text.</p>"#;
        let metadata = Metadata::default();
        let config = TextConfig::default();

        let result = convert_to_text(html, &metadata, &config);
        assert!(result.is_ok());
        let text = result.unwrap();
        assert!(text.contains("Simple paragraph"));
    }

    #[test]
    fn test_convert_to_text_with_header() {
        let html = r#"<p>Content here.</p>"#;
        let metadata = Metadata {
            title: Some("Test Title".to_string()),
            author: Some("Test Author".to_string()),
            ..Default::default()
        };
        let config = TextConfig { include_header: true, ..Default::default() };

        let result = convert_to_text(html, &metadata, &config);
        assert!(result.is_ok());
        let text = result.unwrap();
        assert!(text.contains("Test Title"));
        assert!(text.contains("Test Author"));
        assert!(text.contains("Content here"));
    }

    #[test]
    fn test_convert_to_text_preserve_paragraphs() {
        let html = r#"
            <p>First paragraph.</p>
            <p>Second paragraph.</p>
            <p>Third paragraph.</p>
        "#;

        let metadata = Metadata::default();
        let config = TextConfig { preserve_paragraphs: true, ..Default::default() };

        let result = convert_to_text(html, &metadata, &config);
        assert!(result.is_ok());
        let text = result.unwrap();
        assert!(text.contains("First paragraph"));
        assert!(text.contains("Second paragraph"));
        assert!(text.contains("Third paragraph"));
    }

    #[test]
    fn test_wrap_words() {
        let words = vec!["hello", "world", "this", "is", "a", "test"];
        let wrapped = wrap_words(&words, 10);
        assert!(wrapped.contains('\n'));
    }

    #[test]
    fn test_wrap_text_with_narrow_width() {
        let text = "This is a long line that should be wrapped at a smaller width";
        let wrapped = wrap_text(text, 20);
        assert!(wrapped.contains('\n'));
    }

    #[test]
    fn test_wrap_text_with_zero_width() {
        let text = "This is a line";
        let wrapped = wrap_text(text, 0);
        assert_eq!(wrapped, text);
    }

    #[test]
    fn test_text_formatter() {
        let html = r#"<p>Test content for formatter.</p>"#;
        let metadata = Metadata::default();
        let config = TextConfig::default();
        let formatter = TextFormatter::new(config.clone());

        let result = formatter.convert(html, &metadata);
        assert!(result.is_ok());

        let direct_result = convert_to_text(html, &metadata, &config);
        assert!(direct_result.is_ok());

        assert_eq!(result.unwrap(), direct_result.unwrap());
    }

    #[test]
    fn test_convert_to_text_with_lists() {
        let html = r#"
            <ul>
                <li>First item</li>
                <li>Second item</li>
                <li>Third item</li>
            </ul>
        "#;

        let metadata = Metadata::default();
        let config = TextConfig { preserve_paragraphs: true, ..Default::default() };

        let result = convert_to_text(html, &metadata, &config);
        assert!(result.is_ok());
        let text = result.unwrap();
        assert!(text.contains("First item"));
        assert!(text.contains("Second item"));
        assert!(text.contains("Third item"));
    }

    #[test]
    fn test_extract_text_with_blockquotes() {
        let html = r#"<blockquote>This is a quoted text.</blockquote>"#;
        let result = extract_text_with_paragraphs(html);
        assert!(result.is_ok());
        let text = result.unwrap();
        assert!(text.contains("quoted text"));
    }
}
