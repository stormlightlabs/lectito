use std::io;

use kuchiki::NodeRef;

use super::error::{Error, Result};
use super::{dom, patterns};

pub fn serialize_node(node: &NodeRef) -> Result<String> {
    let mut bytes = Vec::new();
    node.serialize(&mut bytes)
        .map_err(|_: io::Error| Error::Serialization)?;
    String::from_utf8(bytes).map_err(|_| Error::Serialization)
}

pub fn serialize_children(node: &NodeRef) -> Result<String> {
    let mut html = String::new();
    for child in node.children() {
        html.push_str(&serialize_node(&child)?);
    }
    Ok(html)
}

pub fn text_content(nodes: &[NodeRef]) -> String {
    let mut text = String::new();
    for node in nodes {
        append_text_content(node, &mut text);
        ensure_blank_line(&mut text);
    }
    normalize_plain_text(&text)
}

fn append_text_content(node: &NodeRef, output: &mut String) {
    if let Some(text) = node.as_text() {
        push_text(output, &text.borrow());
        return;
    }

    let tag = dom::node_name(node);
    let block = is_block_boundary(&tag);
    if block {
        ensure_line_break(output);
    }
    if tag == "pre" {
        push_pre_text(output, &node.text_contents());
        ensure_line_break(output);
        return;
    }

    for child in node.children() {
        append_text_content(&child, output);
    }

    if tag == "br" {
        ensure_line_break(output);
    } else if block {
        ensure_line_break(output);
    }
}

fn push_text(output: &mut String, text: &str) {
    if text.trim().is_empty() {
        if !output.ends_with(char::is_whitespace) {
            output.push(' ');
        }
        return;
    }

    let normalized = patterns::normalize_spaces(text);
    let starts_with_space = normalized.starts_with(char::is_whitespace);
    let ends_with_space = normalized.ends_with(char::is_whitespace);
    let trimmed = normalized.trim();

    if starts_with_space && needs_space(output) {
        output.push(' ');
    }
    output.push_str(trimmed);
    if ends_with_space {
        output.push(' ');
    }
}

fn is_block_boundary(tag: &str) -> bool {
    matches!(
        tag,
        "address"
            | "article"
            | "aside"
            | "blockquote"
            | "dd"
            | "details"
            | "div"
            | "dl"
            | "dt"
            | "figcaption"
            | "figure"
            | "footer"
            | "h1"
            | "h2"
            | "h3"
            | "h4"
            | "h5"
            | "h6"
            | "header"
            | "hr"
            | "li"
            | "main"
            | "nav"
            | "ol"
            | "p"
            | "pre"
            | "section"
            | "table"
            | "tbody"
            | "td"
            | "tfoot"
            | "th"
            | "thead"
            | "tr"
            | "ul"
    )
}

fn push_pre_text(output: &mut String, text: &str) {
    for line in text.trim_matches('\n').lines() {
        let line = line.trim_end();
        output.push_str(line);
        output.push('\n');
    }
}

fn ensure_line_break(output: &mut String) {
    while output.ends_with(' ') {
        output.pop();
    }
    if !output.is_empty() && !output.ends_with('\n') {
        output.push('\n');
    }
}

fn ensure_blank_line(output: &mut String) {
    ensure_line_break(output);
    if !output.is_empty() && !output.ends_with("\n\n") {
        output.push('\n');
    }
}

fn needs_space(output: &str) -> bool {
    output
        .chars()
        .next_back()
        .is_some_and(|ch| !ch.is_whitespace() && !matches!(ch, '(' | '[' | '{' | '/' | '-'))
}

fn normalize_plain_text(text: &str) -> String {
    let mut output = String::new();
    let mut blank = false;

    for line in text.lines() {
        let line = patterns::normalize_spaces(line).trim().to_string();
        if line.is_empty() {
            if !blank && !output.is_empty() {
                output.push('\n');
                blank = true;
            }
            continue;
        }
        if !output.is_empty() && !output.ends_with('\n') {
            output.push('\n');
        }
        output.push_str(&line);
        blank = false;
    }

    output.trim().to_string()
}

#[cfg(test)]
mod tests {
    use kuchiki::traits::TendrilSink;

    use super::*;

    #[test]
    fn text_content_preserves_block_boundaries() {
        let document = kuchiki::parse_html().one(
            r#"<main>
                <h2>Heading</h2>
                <p>Paragraph with <code>inline()</code> code.</p>
                <pre><code>let x = 1;
let y = 2;</code></pre>
                <ul><li>First</li><li>Second</li></ul>
                <dl><dt>Term</dt><dd>Definition</dd></dl>
            </main>"#,
        );
        let body = dom::select_nodes(&document, "body");

        assert_eq!(
            text_content(&body),
            "Heading\nParagraph with inline() code.\nlet x = 1;\nlet y = 2;\nFirst\nSecond\nTerm\nDefinition"
        );
    }
}
