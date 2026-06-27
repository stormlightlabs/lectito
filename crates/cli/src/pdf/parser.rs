use pulldown_cmark::{Event, Options, Parser, Tag, TagEnd};

#[derive(Debug, Clone)]
pub enum Node {
    Heading { level: u8, text: String },
    Paragraph(String),
    CodeBlock(String),
    ListItem { marker: String, text: String },
    BlockQuote(String),
    Rule,
    TableRow(String),
    Definition { term: String, details: String },
    Footnote { label: String, text: String },
}

enum Block {
    Heading { level: u8, text: String },
    Paragraph(String),
    CodeBlock(String),
    BlockQuote(String),
    DefinitionTitle(String),
    DefinitionDetails(String),
    Footnote { label: String, text: String },
}

#[derive(Default)]
struct TableState {
    rows: Vec<Vec<String>>,
    current_row: Vec<String>,
    current_cell: Option<String>,
}

struct ListState {
    next_number: Option<u64>,
}

struct ListItem {
    marker: String,
    text: String,
}

pub fn parse_markdown(markdown: &str) -> Vec<Node> {
    let mut nodes = Vec::new();
    let mut block = None;
    let mut table = None;
    let mut lists: Vec<ListState> = Vec::new();
    let mut items: Vec<ListItem> = Vec::new();
    let mut definition_term = None;
    let mut item_code_depth = 0usize;

    for event in Parser::new_ext(markdown, parser_opts()) {
        match event {
            Event::Start(Tag::Table(_)) => {
                flush_block(&mut nodes, &mut block, &mut definition_term);
                flush_current_item(&mut nodes, &mut items);
                table = Some(TableState::default());
            }
            Event::Start(Tag::TableHead) => {
                if let Some(table) = &mut table {
                    table.current_row.clear();
                }
            }
            Event::Start(Tag::TableRow) => {
                if let Some(table) = &mut table {
                    table.current_row.clear();
                }
            }
            Event::Start(Tag::TableCell) => {
                if let Some(table) = &mut table {
                    table.current_cell = Some(String::new());
                }
            }
            Event::End(TagEnd::TableCell) => {
                if let Some(table) = &mut table
                    && let Some(cell) = table.current_cell.take()
                {
                    table.current_row.push(normalize_text(&cell));
                }
            }
            Event::End(TagEnd::TableRow) => {
                if let Some(table) = &mut table
                    && !table.current_row.iter().all(|cell| cell.trim().is_empty())
                {
                    table.rows.push(std::mem::take(&mut table.current_row));
                }
            }
            Event::End(TagEnd::TableHead) => {
                if let Some(table) = &mut table
                    && !table.current_row.iter().all(|cell| cell.trim().is_empty())
                {
                    table.rows.push(std::mem::take(&mut table.current_row));
                }
            }
            Event::End(TagEnd::Table) => {
                if let Some(table) = table.take() {
                    for row in format_table_rows(table.rows) {
                        push_node(&mut nodes, Node::TableRow(row));
                    }
                }
            }
            Event::Start(Tag::List(start)) => {
                flush_current_item(&mut nodes, &mut items);
                lists.push(ListState { next_number: start });
            }
            Event::End(TagEnd::List(_)) => {
                flush_current_item(&mut nodes, &mut items);
                lists.pop();
            }
            Event::Start(Tag::Item) => {
                items.push(ListItem { marker: next_list_marker(&mut lists), text: String::new() });
            }
            Event::End(TagEnd::Item) => {
                flush_current_item(&mut nodes, &mut items);
                items.pop();
            }
            Event::Start(Tag::Heading { level, .. }) => {
                start_block(
                    &mut block,
                    &items,
                    Block::Heading { level: level as u8, text: String::new() },
                );
            }
            Event::End(TagEnd::Heading(_)) => end_leaf_block(&mut nodes, &mut block, &mut items, &mut definition_term),
            Event::Start(Tag::Paragraph) => {
                start_block(&mut block, &items, Block::Paragraph(String::new()));
            }
            Event::End(TagEnd::Paragraph) => {
                if !items.is_empty() {
                    append_separator(&mut block, &mut items);
                } else if matches!(block, Some(Block::Paragraph(_))) {
                    flush_block(&mut nodes, &mut block, &mut definition_term);
                } else {
                    append_separator(&mut block, &mut items);
                }
            }
            Event::Start(Tag::CodeBlock(_)) if items.is_empty() => {
                start_block(&mut block, &items, Block::CodeBlock(String::new()));
            }
            Event::Start(Tag::CodeBlock(_)) => {
                item_code_depth += 1;
                append_separator(&mut block, &mut items);
            }
            Event::End(TagEnd::CodeBlock) if item_code_depth > 0 => {
                item_code_depth -= 1;
                append_separator(&mut block, &mut items);
            }
            Event::End(TagEnd::CodeBlock) => {
                flush_block(&mut nodes, &mut block, &mut definition_term);
            }
            Event::Start(Tag::BlockQuote(_)) => {
                start_block(&mut block, &items, Block::BlockQuote(String::new()));
            }
            Event::End(TagEnd::BlockQuote(_)) => {
                flush_block(&mut nodes, &mut block, &mut definition_term);
            }
            Event::Start(Tag::DefinitionListTitle) => {
                start_block(&mut block, &items, Block::DefinitionTitle(String::new()));
            }
            Event::End(TagEnd::DefinitionListTitle) => {
                flush_block(&mut nodes, &mut block, &mut definition_term);
            }
            Event::Start(Tag::DefinitionListDefinition) => {
                start_block(&mut block, &items, Block::DefinitionDetails(String::new()));
            }
            Event::End(TagEnd::DefinitionListDefinition) => {
                flush_block(&mut nodes, &mut block, &mut definition_term);
            }
            Event::Start(Tag::FootnoteDefinition(label)) => {
                start_block(
                    &mut block,
                    &items,
                    Block::Footnote { label: label.to_string(), text: String::new() },
                );
            }
            Event::End(TagEnd::FootnoteDefinition) => {
                flush_block(&mut nodes, &mut block, &mut definition_term);
            }
            Event::Rule => {
                flush_block(&mut nodes, &mut block, &mut definition_term);
                flush_current_item(&mut nodes, &mut items);
                push_node(&mut nodes, Node::Rule);
            }
            Event::TaskListMarker(checked) => {
                append_text(&mut table, &mut block, &mut items, if checked { "[x]" } else { "[ ]" });
            }
            Event::FootnoteReference(label) => {
                append_text(&mut table, &mut block, &mut items, &format!("[^{label}]"));
            }
            Event::Start(Tag::Image { .. }) => {
                append_text(&mut table, &mut block, &mut items, "Image:");
            }
            Event::Text(value) | Event::Code(value) | Event::InlineMath(value) | Event::DisplayMath(value) => {
                append_text(&mut table, &mut block, &mut items, &value);
            }
            Event::SoftBreak | Event::HardBreak
                if matches!(block, Some(Block::CodeBlock(_))) || item_code_depth > 0 =>
            {
                append_raw(&mut table, &mut block, &mut items, "\n");
            }
            Event::SoftBreak | Event::HardBreak => {
                append_separator(&mut block, &mut items);
            }
            Event::Html(_) | Event::InlineHtml(_) => {}
            _ => {}
        }
    }

    flush_block(&mut nodes, &mut block, &mut definition_term);
    while !items.is_empty() {
        flush_current_item(&mut nodes, &mut items);
        items.pop();
    }
    if let Some(table) = table {
        for row in format_table_rows(table.rows) {
            push_node(&mut nodes, Node::TableRow(row));
        }
    }
    nodes
}

fn parser_opts() -> Options {
    Options::ENABLE_TABLES
        | Options::ENABLE_FOOTNOTES
        | Options::ENABLE_STRIKETHROUGH
        | Options::ENABLE_TASKLISTS
        | Options::ENABLE_HEADING_ATTRIBUTES
        | Options::ENABLE_YAML_STYLE_METADATA_BLOCKS
        | Options::ENABLE_PLUSES_DELIMITED_METADATA_BLOCKS
        | Options::ENABLE_DEFINITION_LIST
        | Options::ENABLE_SUPERSCRIPT
        | Options::ENABLE_SUBSCRIPT
        | Options::ENABLE_GFM
}

fn start_block(block: &mut Option<Block>, items: &[ListItem], next: Block) {
    if items.is_empty() && block.is_none() {
        *block = Some(next);
    }
}

fn end_leaf_block(
    nodes: &mut Vec<Node>, block: &mut Option<Block>, items: &mut [ListItem], definition_term: &mut Option<String>,
) {
    if !items.is_empty() {
        append_separator(block, items);
    } else {
        flush_block(nodes, block, definition_term);
    }
}

fn flush_block(nodes: &mut Vec<Node>, block: &mut Option<Block>, definition_term: &mut Option<String>) {
    let Some(block) = block.take() else {
        return;
    };

    match block {
        Block::Heading { level, text } => push_node(nodes, Node::Heading { level, text: normalize_text(&text) }),
        Block::Paragraph(text) => push_node(nodes, Node::Paragraph(normalize_text(&text))),
        Block::CodeBlock(text) => push_node(nodes, Node::CodeBlock(normalize_pdf_text(text.trim_end()))),
        Block::BlockQuote(text) => push_node(nodes, Node::BlockQuote(normalize_text(&text))),
        Block::DefinitionTitle(text) => {
            let text = normalize_text(&text);
            if !text.trim().is_empty() {
                *definition_term = Some(text);
            }
        }
        Block::DefinitionDetails(text) => {
            let details = normalize_text(&text);
            let term = definition_term.take().unwrap_or_default();
            push_node(nodes, Node::Definition { term, details });
        }
        Block::Footnote { label, text } => push_node(
            nodes,
            Node::Footnote { label: normalize_text(&label), text: normalize_text(&text) },
        ),
    }
}

fn flush_current_item(nodes: &mut Vec<Node>, items: &mut [ListItem]) {
    let Some(item) = items.last_mut() else {
        return;
    };

    let text = normalize_text(&item.text);
    item.text.clear();
    push_node(nodes, Node::ListItem { marker: item.marker.clone(), text });
}

fn push_node(nodes: &mut Vec<Node>, node: Node) {
    let has_text = match &node {
        Node::Heading { text, .. }
        | Node::Paragraph(text)
        | Node::CodeBlock(text)
        | Node::ListItem { text, .. }
        | Node::BlockQuote(text)
        | Node::TableRow(text)
        | Node::Definition { details: text, .. }
        | Node::Footnote { text, .. } => !text.trim().is_empty(),
        Node::Rule => true,
    };

    if has_text {
        nodes.push(node);
    }
}

fn append_text(table: &mut Option<TableState>, block: &mut Option<Block>, items: &mut [ListItem], text: &str) {
    if let Some(table) = table
        && let Some(cell) = &mut table.current_cell
    {
        push_word(cell, text);
        return;
    }

    if let Some(item) = items.last_mut() {
        push_word(&mut item.text, text);
        return;
    }

    let Some(block) = block else {
        return;
    };
    match block {
        Block::Heading { text: target, .. }
        | Block::Paragraph(target)
        | Block::BlockQuote(target)
        | Block::DefinitionTitle(target)
        | Block::DefinitionDetails(target)
        | Block::Footnote { text: target, .. } => push_word(target, text),
        Block::CodeBlock(target) => target.push_str(text),
    }
}

fn append_raw(table: &mut Option<TableState>, block: &mut Option<Block>, items: &mut [ListItem], text: &str) {
    if let Some(table) = table
        && let Some(cell) = &mut table.current_cell
    {
        cell.push_str(text);
        return;
    }

    if let Some(item) = items.last_mut() {
        item.text.push_str(text);
        return;
    }

    if let Some(Block::CodeBlock(target)) = block {
        target.push_str(text);
    }
}

fn append_separator(block: &mut Option<Block>, items: &mut [ListItem]) {
    if let Some(item) = items.last_mut() {
        push_word(&mut item.text, " ");
        return;
    }

    let Some(block) = block else {
        return;
    };
    match block {
        Block::CodeBlock(text) => text.push('\n'),
        Block::Heading { text, .. }
        | Block::Paragraph(text)
        | Block::BlockQuote(text)
        | Block::DefinitionTitle(text)
        | Block::DefinitionDetails(text)
        | Block::Footnote { text, .. } => push_word(text, " "),
    }
}

fn next_list_marker(lists: &mut [ListState]) -> String {
    let indent = "  ".repeat(lists.len().saturating_sub(1));
    let Some(list) = lists.last_mut() else {
        return "-".to_string();
    };

    match &mut list.next_number {
        Some(next_number) => {
            let marker = format!("{indent}{next_number}.");
            *next_number += 1;
            marker
        }
        None => format!("{indent}-"),
    }
}

fn format_table_rows(rows: Vec<Vec<String>>) -> Vec<String> {
    let column_count = rows.iter().map(Vec::len).max().unwrap_or(0);
    if column_count == 0 {
        return Vec::new();
    }

    let mut widths = vec![0; column_count];
    for row in &rows {
        for (idx, cell) in row.iter().enumerate() {
            widths[idx] = widths[idx].max(cell.chars().count().min(32));
        }
    }

    rows.into_iter()
        .map(|row| {
            let cells = (0..column_count)
                .map(|idx| {
                    let cell = row.get(idx).map(String::as_str).unwrap_or_default();
                    pad_cell(cell, widths[idx])
                })
                .collect::<Vec<_>>();
            format!("| {} |", cells.join(" | "))
        })
        .collect()
}

fn pad_cell(cell: &str, width: usize) -> String {
    let truncated = if cell.chars().count() > 32 {
        format!("{}...", cell.chars().take(29).collect::<String>())
    } else {
        cell.to_string()
    };
    format!("{truncated:<width$}")
}

fn push_word(target: &mut String, value: &str) {
    if value.trim().is_empty() {
        return;
    }
    if !target.ends_with(char::is_whitespace) && !target.is_empty() {
        target.push(' ');
    }
    target.push_str(value);
}

fn normalize_text(text: &str) -> String {
    normalize_pdf_text(&text.split_whitespace().collect::<Vec<_>>().join(" "))
}

fn normalize_pdf_text(text: &str) -> String {
    text.replace('®', "(R)")
        .replace('©', "(C)")
        .replace('™', "(TM)")
        .replace(['–', '—'], "-")
        .replace('’', "'")
        .replace('“', "\"")
        .replace('”', "\"")
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn parses_common_blocks() {
        let nodes = parse_markdown("# Title\n\nBody.\n\n- One\n\n```rust\nfn main() {}\n```");

        assert!(matches!(nodes[0], Node::Heading { level: 1, .. }));
        assert!(matches!(nodes[1], Node::Paragraph(_)));
        assert!(matches!(nodes[2], Node::ListItem { .. }));
        assert!(matches!(nodes[3], Node::CodeBlock(_)));
    }

    #[test]
    fn skips_empty_list_items() {
        let nodes = parse_markdown("- \n- [Docs](https://example.com)\n");

        assert_eq!(nodes.len(), 1);
        assert!(matches!(&nodes[0], Node::ListItem { marker, text } if marker == "-" && text == "Docs"));
    }

    #[test]
    fn keeps_card_item_on_one_node_without_trailing_empty_item() {
        let nodes = parse_markdown(
            "- ### Deploy services Connect your repo. - [How deploys work](https://render.com/docs/deploys)\n",
        );

        assert_eq!(nodes.len(), 1);
        assert!(
            matches!(&nodes[0], Node::ListItem { text, .. } if text == "Deploy services Connect your repo. - How deploys work")
        );
    }

    #[test]
    fn emits_parent_item_before_nested_items() {
        let nodes = parse_markdown("- ### Node.js\n  - Express\n  - Fastify\n");

        assert_eq!(nodes.len(), 3);
        assert!(matches!(&nodes[0], Node::ListItem { marker, text } if marker == "-" && text == "Node.js"));
        assert!(matches!(&nodes[1], Node::ListItem { marker, text } if marker == "  -" && text == "Express"));
        assert!(matches!(&nodes[2], Node::ListItem { marker, text } if marker == "  -" && text == "Fastify"));
    }

    #[test]
    fn parses_extended_markdown_blocks() {
        let nodes = parse_markdown(
            r#"# Title {#title}

> quoted **text**

1. [x] first
2. [ ] second

Term
: details

| Name | Value |
| --- | --- |
| CPU | 2 |

---

![diagram](diagram.png)

Footnote ref.[^1]

[^1]: Footnote text.
"#,
        );

        assert!(
            nodes
                .iter()
                .any(|node| matches!(node, Node::BlockQuote(text) if text == "quoted text"))
        );
        assert!(
            nodes
                .iter()
                .any(|node| matches!(node, Node::ListItem { marker, text } if marker == "1." && text == "[x] first"))
        );
        assert!(
            nodes
                .iter()
                .any(|node| matches!(node, Node::ListItem { marker, text } if marker == "2." && text == "[ ] second"))
        );
        assert!(
            nodes.iter().any(
                |node| matches!(node, Node::Definition { term, details } if term == "Term" && details == "details")
            )
        );
        assert!(
            nodes
                .iter()
                .any(|node| matches!(node, Node::TableRow(text) if text.contains("| Name") && text.contains("Value")))
        );
        assert!(nodes.iter().any(|node| matches!(node, Node::Rule)));
        assert!(
            nodes
                .iter()
                .any(|node| matches!(node, Node::Paragraph(text) if text == "Image: diagram"))
        );
        assert!(
            nodes
                .iter()
                .any(|node| matches!(node, Node::Paragraph(text) if text == "Footnote ref. [^1]"))
        );
        assert!(
            nodes
                .iter()
                .any(|node| matches!(node, Node::Footnote { label, text } if label == "1" && text == "Footnote text."))
        );
    }
}
