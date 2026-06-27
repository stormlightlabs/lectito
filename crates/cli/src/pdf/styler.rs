use super::parser::Node;

#[derive(Debug, Clone)]
pub struct Style {
    pub font_size: f32,
    pub line_height: f32,
    pub margin_top: f32,
    pub margin_bottom: f32,
    pub is_bold: bool,
    pub is_monospace: bool,
}

impl Style {
    fn heading(level: u8) -> Self {
        let font_size = match level {
            1 => 24.0,
            2 => 20.0,
            3 => 16.0,
            _ => 14.0,
        };

        Self {
            font_size,
            line_height: font_size * 1.2,
            margin_top: font_size * 0.8,
            margin_bottom: font_size * 0.4,
            is_bold: true,
            is_monospace: false,
        }
    }

    fn paragraph() -> Self {
        Self {
            font_size: 12.0,
            line_height: 16.0,
            margin_top: 6.0,
            margin_bottom: 6.0,
            is_bold: false,
            is_monospace: false,
        }
    }

    fn code_block() -> Self {
        Self {
            font_size: 10.0,
            line_height: 14.0,
            margin_top: 8.0,
            margin_bottom: 8.0,
            is_bold: false,
            is_monospace: true,
        }
    }

    fn list_item() -> Self {
        Self {
            font_size: 12.0,
            line_height: 16.0,
            margin_top: 2.0,
            margin_bottom: 2.0,
            is_bold: false,
            is_monospace: false,
        }
    }

    fn block_quote() -> Self {
        Self {
            font_size: 12.0,
            line_height: 16.0,
            margin_top: 6.0,
            margin_bottom: 6.0,
            is_bold: false,
            is_monospace: false,
        }
    }

    fn rule() -> Self {
        Self {
            font_size: 10.0,
            line_height: 12.0,
            margin_top: 8.0,
            margin_bottom: 8.0,
            is_bold: false,
            is_monospace: true,
        }
    }

    fn table_row() -> Self {
        Self {
            font_size: 9.0,
            line_height: 12.0,
            margin_top: 1.0,
            margin_bottom: 1.0,
            is_bold: false,
            is_monospace: true,
        }
    }

    fn note() -> Self {
        Self {
            font_size: 10.0,
            line_height: 14.0,
            margin_top: 4.0,
            margin_bottom: 4.0,
            is_bold: false,
            is_monospace: false,
        }
    }
}

#[derive(Debug, Clone)]
pub struct StyledBlock {
    pub content: String,
    pub style: Style,
    pub is_code_block: bool,
    pub keep_with_next: bool,
}

pub fn apply_styles(nodes: Vec<Node>) -> Vec<StyledBlock> {
    nodes
        .into_iter()
        .map(|node| match node {
            Node::Heading { level, text } => {
                StyledBlock { content: text, style: Style::heading(level), is_code_block: false, keep_with_next: true }
            }
            Node::Paragraph(text) => {
                StyledBlock { content: text, style: Style::paragraph(), is_code_block: false, keep_with_next: false }
            }
            Node::CodeBlock(code) => {
                StyledBlock { content: code, style: Style::code_block(), is_code_block: true, keep_with_next: false }
            }
            Node::ListItem { marker, text } => StyledBlock {
                content: format!("{marker} {text}"),
                style: Style::list_item(),
                is_code_block: false,
                keep_with_next: false,
            },
            Node::BlockQuote(text) => StyledBlock {
                content: format!("> {text}"),
                style: Style::block_quote(),
                is_code_block: false,
                keep_with_next: false,
            },
            Node::Rule => StyledBlock {
                content: "-".repeat(60),
                style: Style::rule(),
                is_code_block: false,
                keep_with_next: false,
            },
            Node::TableRow(text) => {
                StyledBlock { content: text, style: Style::table_row(), is_code_block: false, keep_with_next: false }
            }
            Node::Definition { term, details } => StyledBlock {
                content: format!("{term}: {details}"),
                style: Style::paragraph(),
                is_code_block: false,
                keep_with_next: false,
            },
            Node::Footnote { label, text } => StyledBlock {
                content: format!("[^{label}]: {text}"),
                style: Style::note(),
                is_code_block: false,
                keep_with_next: false,
            },
        })
        .collect()
}
