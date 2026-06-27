use super::styler::{Style, StyledBlock};

#[derive(Debug, Clone)]
pub struct Glyph {
    pub ch: char,
    pub x: f32,
    pub y: f32,
    pub font_size: f32,
    pub is_bold: bool,
    pub is_monospace: bool,
}

#[derive(Debug, Clone)]
pub struct PositionedLine {
    pub glyphs: Vec<Glyph>,
}

#[derive(Debug, Clone)]
pub struct Page {
    pub lines: Vec<PositionedLine>,
}

struct LayoutEngine {
    page_width: f32,
    page_height: f32,
    margin: f32,
    current_y: f32,
    pages: Vec<Page>,
}

impl LayoutEngine {
    fn new(page_width: f32, page_height: f32, margin: f32) -> Self {
        Self { page_width, page_height, margin, current_y: margin, pages: vec![Page { lines: Vec::new() }] }
    }

    fn should_page_break(&self, additional_height: f32) -> bool {
        self.current_y + additional_height > self.page_height - self.margin
    }

    fn usable_height(&self) -> f32 {
        self.page_height - (2.0 * self.margin)
    }

    fn new_page(&mut self) {
        self.current_y = self.margin;
        self.pages.push(Page { lines: Vec::new() });
    }

    fn current_page_mut(&mut self) -> &mut Page {
        self.pages.last_mut().expect("LayoutEngine always has a page")
    }

    fn char_width(&self, style: &Style) -> f32 {
        if style.is_monospace { style.font_size * 0.6 } else { style.font_size * 0.5 }
    }

    fn measure_block_height(&self, block: &StyledBlock) -> f32 {
        let line_count = if block.is_code_block {
            block.content.lines().count().max(1)
        } else {
            self.wrapped_line_count(&block.content, &block.style)
        };

        block.style.margin_top + block.style.margin_bottom + (line_count as f32 * block.style.line_height)
    }

    fn wrapped_line_count(&self, text: &str, style: &Style) -> usize {
        let max_width = self.page_width - (2.0 * self.margin);
        let char_width = self.char_width(style);
        let mut line_width = 0.0;
        let mut count = 0;

        for word in text.split_whitespace() {
            let word_width = word.len() as f32 * char_width;
            let space_width = if line_width == 0.0 { 0.0 } else { char_width };

            if line_width > 0.0 && line_width + space_width + word_width > max_width {
                count += 1;
                line_width = 0.0;
            }

            if line_width > 0.0 {
                line_width += space_width;
            }
            line_width += word_width;
        }

        if line_width > 0.0 { count + 1 } else { count }
    }

    fn layout_block(&mut self, block: &StyledBlock, block_height: f32) {
        let is_page_start = (self.current_y - self.margin).abs() < f32::EPSILON;
        if self.should_page_break(block_height) && !is_page_start {
            self.new_page();
        }

        self.current_y += block.style.margin_top;

        if block.is_code_block {
            for line in block.content.lines() {
                self.add_line(line, &block.style);
            }
        } else {
            self.add_wrapped_text(&block.content, &block.style);
        }

        self.current_y += block.style.margin_bottom;
    }

    fn add_wrapped_text(&mut self, text: &str, style: &Style) {
        let max_width = self.page_width - (2.0 * self.margin);
        let char_width = self.char_width(style);
        let mut current_line = String::new();
        let mut line_width = 0.0;

        for word in text.split_whitespace() {
            let word_width = word.len() as f32 * char_width;
            let space_width = char_width;

            if line_width + word_width + space_width > max_width && !current_line.is_empty() {
                self.add_line(&current_line, style);
                current_line.clear();
                line_width = 0.0;
            }

            if !current_line.is_empty() {
                current_line.push(' ');
                line_width += space_width;
            }

            current_line.push_str(word);
            line_width += word_width;
        }

        if !current_line.is_empty() {
            self.add_line(&current_line, style);
        }
    }

    fn add_line(&mut self, text: &str, style: &Style) {
        if text.is_empty() {
            return;
        }

        if self.should_page_break(style.line_height) {
            self.new_page();
        }

        let line = self.create_line(text, style, self.current_y);
        self.current_page_mut().lines.push(line);
        self.current_y += style.line_height;
    }

    fn create_line(&self, text: &str, style: &Style, y: f32) -> PositionedLine {
        let char_width = self.char_width(style);
        let mut glyphs = Vec::new();
        let mut x = self.margin;

        for ch in text.chars() {
            glyphs.push(Glyph {
                ch,
                x,
                y,
                font_size: style.font_size,
                is_bold: style.is_bold,
                is_monospace: style.is_monospace,
            });
            x += char_width;
        }

        PositionedLine { glyphs }
    }
}

pub fn layout_blocks(blocks: &[StyledBlock]) -> Vec<Page> {
    let mut engine = LayoutEngine::new(595.0, 842.0, 50.0);
    let mut idx = 0;

    while idx < blocks.len() {
        let block = &blocks[idx];
        let block_height = engine.measure_block_height(block);

        if block.keep_with_next
            && let Some(next_block) = blocks.get(idx + 1)
        {
            let next_height = engine.measure_block_height(next_block);
            let combined_height = block_height + next_height;
            if combined_height <= engine.usable_height() && engine.should_page_break(combined_height) {
                engine.new_page();
            }
        }

        engine.layout_block(block, block_height);
        idx += 1;
    }

    engine.pages
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn wraps_long_lines() {
        let blocks = vec![StyledBlock {
            content: "This is a very long line that should wrap across multiple rendered PDF lines.".to_string(),
            style: Style {
                font_size: 24.0,
                line_height: 28.0,
                margin_top: 0.0,
                margin_bottom: 0.0,
                is_bold: false,
                is_monospace: false,
            },
            is_code_block: false,
            keep_with_next: false,
        }];

        let line_count: usize = layout_blocks(&blocks).iter().map(|page| page.lines.len()).sum();
        assert!(line_count > 1, "long text should wrap");
    }
}
