use std::io;

use pdf_writer::{Content, Finish, Name, Pdf, Rect, Ref, Str};

use super::layout::Page;

const PAGE_WIDTH: f32 = 595.0;
const PAGE_HEIGHT: f32 = 842.0;

pub fn render_to_pdf(pages: &[Page]) -> io::Result<Vec<u8>> {
    let mut pdf = Pdf::new();

    let catalog_id = Ref::new(1);
    let page_tree_id = Ref::new(2);
    let font_regular_id = Ref::new(3);
    let font_bold_id = Ref::new(4);
    let font_mono_id = Ref::new(5);

    let mut next_id = 6;
    let mut page_ids = Vec::new();
    let mut content_ids = Vec::new();

    for _ in 0..pages.len() {
        page_ids.push(Ref::new(next_id));
        next_id += 1;
        content_ids.push(Ref::new(next_id));
        next_id += 1;
    }

    pdf.catalog(catalog_id).pages(page_tree_id);
    pdf.pages(page_tree_id)
        .kids(page_ids.iter().copied())
        .count(pages.len() as i32);

    pdf.type1_font(font_regular_id).base_font(Name(b"Helvetica"));
    pdf.type1_font(font_bold_id).base_font(Name(b"Helvetica-Bold"));
    pdf.type1_font(font_mono_id).base_font(Name(b"Courier"));

    for (page_idx, page_content) in pages.iter().enumerate() {
        let page_id = page_ids[page_idx];
        let content_id = content_ids[page_idx];

        let mut page = pdf.page(page_id);
        page.media_box(Rect::new(0.0, 0.0, PAGE_WIDTH, PAGE_HEIGHT));
        page.parent(page_tree_id);
        page.contents(content_id);

        let mut resources = page.resources();
        resources
            .fonts()
            .pair(Name(b"F1"), font_regular_id)
            .pair(Name(b"F2"), font_bold_id)
            .pair(Name(b"F3"), font_mono_id);
        resources.finish();
        page.finish();

        let mut content = Content::new();
        for line in &page_content.lines {
            if line.glyphs.is_empty() {
                continue;
            }

            let text: String = line.glyphs.iter().map(|glyph| glyph.ch).collect();
            let first_glyph = &line.glyphs[0];
            let font_name = if first_glyph.is_bold {
                Name(b"F2")
            } else if first_glyph.is_monospace {
                Name(b"F3")
            } else {
                Name(b"F1")
            };

            content.begin_text();
            content.set_font(font_name, first_glyph.font_size);
            content.set_text_matrix([1.0, 0.0, 0.0, 1.0, first_glyph.x, PAGE_HEIGHT - first_glyph.y]);
            content.show(Str(text.as_bytes()));
            content.end_text();
        }

        pdf.stream(content_id, &content.finish());
    }

    Ok(pdf.finish())
}
