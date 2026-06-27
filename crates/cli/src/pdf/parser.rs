use pulldown_cmark::{Event, Parser, Tag, TagEnd};

#[derive(Debug, Clone)]
pub enum Node {
    Heading { level: u8, text: String },
    Paragraph(String),
    CodeBlock(String),
    ListItem(String),
}

pub fn parse_markdown(markdown: &str) -> Vec<Node> {
    let parser = Parser::new(markdown);
    let mut nodes = Vec::new();
    let mut text = String::new();
    let mut heading = None;
    let mut in_paragraph = false;
    let mut in_code_block = false;
    let mut in_list_item = false;

    for event in parser {
        match event {
            Event::Start(Tag::Heading { level, .. }) => {
                heading = Some(level as u8);
                text.clear();
            }
            Event::Start(Tag::Paragraph) => {
                in_paragraph = true;
                text.clear();
            }
            Event::Start(Tag::CodeBlock(_)) => {
                in_code_block = true;
                text.clear();
            }
            Event::Start(Tag::Item) => {
                in_list_item = true;
                text.clear();
            }
            Event::End(TagEnd::Heading(_)) => {
                if let Some(level) = heading.take() {
                    nodes.push(Node::Heading { level, text: text.clone() });
                    text.clear();
                }
            }
            Event::End(TagEnd::Paragraph) => {
                if in_paragraph {
                    nodes.push(Node::Paragraph(text.clone()));
                    text.clear();
                    in_paragraph = false;
                }
            }
            Event::End(TagEnd::CodeBlock) => {
                if in_code_block {
                    nodes.push(Node::CodeBlock(text.clone()));
                    text.clear();
                    in_code_block = false;
                }
            }
            Event::End(TagEnd::Item) => {
                if in_list_item {
                    nodes.push(Node::ListItem(text.clone()));
                    text.clear();
                    in_list_item = false;
                }
            }
            Event::Text(value) | Event::Code(value) => text.push_str(&value),
            Event::SoftBreak | Event::HardBreak if in_code_block => text.push('\n'),
            Event::SoftBreak | Event::HardBreak => text.push(' '),
            _ => {}
        }
    }

    nodes
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn parses_common_blocks() {
        let nodes = parse_markdown("# Title\n\nBody.\n\n- One\n\n```rust\nfn main() {}\n```");

        assert!(matches!(nodes[0], Node::Heading { level: 1, .. }));
        assert!(matches!(nodes[1], Node::Paragraph(_)));
        assert!(matches!(nodes[2], Node::ListItem(_)));
        assert!(matches!(nodes[3], Node::CodeBlock(_)));
    }
}
