//! Markdown-to-PDF rendering for the optional `pdf` CLI feature.

mod layout;
mod parser;
mod renderer;
mod styler;

use std::io;

pub fn markdown_to_pdf(markdown: &str) -> io::Result<Vec<u8>> {
    let nodes = parser::parse_markdown(markdown);
    let blocks = styler::apply_styles(nodes);
    let pages = layout::layout_blocks(&blocks);
    renderer::render_to_pdf(&pages)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn renders_pdf_bytes() {
        let pdf = markdown_to_pdf("# Title\n\nBody text.").expect("render PDF");
        assert!(pdf.starts_with(b"%PDF"), "PDF output should start with a header");
    }
}
