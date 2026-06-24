pub mod code;
mod footnotes;
mod frontmatter;
mod math;
mod media;
mod tables;

use comrak::options::{Extension, Parse};
pub use frontmatter::markdown_with_toml_frontmatter;

use comrak::markdown_to_html as comrak_markdown_to_html;
use comrak::{Arena, Options};
use comrak::{escape_commonmark_link_destination, format_commonmark, parse_document};
use kuchiki::NodeRef;
use kuchiki::traits::TendrilSink;

use super::{dom, patterns, serialize};
use crate::MarkdownOptions;

#[derive(Clone, Copy)]
pub struct RenderContext {
    in_pre: bool,
    list_depth: usize,
}

/// Convert a cleaned HTML fragment to Markdown.
///
/// This helper accepts fragments, not full documents. Full extraction already
/// populates [`crate::Article::markdown`].
pub fn html_to_markdown(html: &str) -> String {
    let document = kuchiki::parse_html().one(format!("<html><body>{html}</body></html>"));
    let body = dom::select_nodes(&document, "body")
        .into_iter()
        .next()
        .unwrap_or(document);
    let footnotes = footnotes::FootnoteContext::extract(&body);
    let mut output = render_children(&body, RenderContext { in_pre: false, list_depth: 0 });
    output.push_str(&footnotes.render_defs());
    output = normalize_markdown(&output);
    fmt_with_comrak(&output)
}

/// Render Markdown to HTML with the provided options.
pub fn markdown_to_html(markdown: &str, options: &MarkdownOptions) -> String {
    comrak_markdown_to_html(markdown, &comrak_opts(options))
}

pub fn normalize_markdown(value: &str) -> String {
    let mut output = String::new();
    let mut blank_count = 0;
    let mut in_fenced_code = false;
    for line in value.lines() {
        let line = line.trim_end();
        if line.trim_start().starts_with("```") || line.trim_start().starts_with("~~~") {
            blank_count = 0;
            in_fenced_code = !in_fenced_code;
            output.push_str(line.trim_start());
            output.push('\n');
            continue;
        }
        if in_fenced_code {
            output.push_str(line);
            output.push('\n');
            continue;
        }
        if line.trim().is_empty() {
            blank_count += 1;
            if blank_count <= 1 {
                output.push('\n');
            }
        } else {
            blank_count = 0;
            output.push_str(line);
            output.push('\n');
        }
    }
    output.trim().to_string()
}

// TODO: instance method on RenderContext
pub fn render_children(node: &NodeRef, ctx: RenderContext) -> String {
    let mut output = String::new();
    for child in node.children() {
        output.push_str(&render_node(&child, ctx));
    }
    output
}

// TODO: instance method on RenderContext
fn render_node(node: &NodeRef, ctx: RenderContext) -> String {
    match node.as_text() {
        Some(text) => {
            let text = text.borrow();
            if ctx.in_pre { text.to_string() } else { patterns::normalize_spaces(&text) }
        }
        None => match code::is_highlighter_chrome(node) {
            true => String::new(),
            false => match dom::node_name(node).as_str() {
                "h1" => block(format!("# {}", inline_children(node, ctx))),
                "h2" => block(format!("## {}", inline_children(node, ctx))),
                "h3" => block(format!("### {}", inline_children(node, ctx))),
                "h4" => block(format!("#### {}", inline_children(node, ctx))),
                "h5" => block(format!("##### {}", inline_children(node, ctx))),
                "h6" => block(format!("###### {}", inline_children(node, ctx))),
                "p" => block(inline_children(node, ctx)),
                "br" => "  \n".to_string(),
                "strong" | "b" => format!("**{}**", inline_children(node, ctx)),
                "em" | "i" => format!("*{}*", inline_children(node, ctx)),
                "mark" => wrap_inline("==", inline_children(node, ctx)),
                "del" | "s" | "strike" => wrap_inline("~~", inline_children(node, ctx)),
                "sup"
                    if dom::class_id_string(node)
                        .to_ascii_lowercase()
                        .split_whitespace()
                        .any(|token| token == "reference") =>
                {
                    inline_children(node, ctx)
                }
                "sup" | "sub" | "svg" => serialize::serialize_node(node).unwrap_or_else(|_| render_children(node, ctx)),
                "code" if !ctx.in_pre => format!("`{}`", inline_children(node, ctx).replace('`', "\\`")),
                "pre" => code::render_code_block(node, ctx),
                "math" | "mjx-container" => math::render_math(node, ctx).unwrap_or_else(|| render_children(node, ctx)),
                "script" => math::render_math(node, ctx).unwrap_or_default(),
                "span" | "img" if math::render_math(node, ctx).is_some() => {
                    math::render_math(node, ctx).unwrap_or_default()
                }
                "iframe" | "video" | "audio" | "object" | "embed" => media::render_embed(node).unwrap_or_default(),
                "a" => {
                    let label = inline_children(node, ctx);
                    if is_heading_permalink(node, &label) {
                        String::new()
                    } else if label.is_empty() {
                        preserved_empty_inline(node)
                    } else if let Some(href) = dom::attr(node, "href") {
                        format!("[{}]({})", label, escape_commonmark_link_destination(&href))
                    } else {
                        label
                    }
                }
                "img" => media::render_image(node),
                "picture" => media::render_picture(node),
                "source" => String::new(),
                "figure" => media::render_figure(node, ctx).unwrap_or_else(|| block(render_children(node, ctx))),
                "blockquote" => match media::render_embed(node) {
                    Some(embed) => block(embed),
                    None => block(
                        normalize_markdown(&render_children(node, ctx))
                            .lines()
                            .map(|line| if line.trim().is_empty() { ">".to_string() } else { format!("> {line}") })
                            .collect::<Vec<_>>()
                            .join("\n"),
                    ),
                },
                "ul" => render_list(node, false, ctx),
                "ol" => render_list(node, true, ctx),
                "li" => block(inline_children(node, ctx)),
                "dl" => render_definition_list(node, ctx),
                "table" => code::render_code_table(node, ctx).unwrap_or_else(|| tables::render_table(node, ctx)),
                "div" => code::render_code_container(node, ctx).unwrap_or_else(|| render_children(node, ctx)),
                "section" | "article" | "main" | "body" => render_children(node, ctx),
                "figcaption" => block(inline_children(node, ctx)),
                "hr" => "\n\n---\n\n".to_string(),
                _ => render_children(node, ctx),
            },
        },
    }
}

// TODO: instance method on RenderContext
fn render_definition_list(node: &NodeRef, ctx: RenderContext) -> String {
    let mut output = String::new();
    for child in node.children().filter(|child| child.as_element().is_some()) {
        match dom::node_name(&child).as_str() {
            "dt" => output.push_str(&block(format!("**{}**", inline_children(&child, ctx)))),
            "dd" => output.push_str(&block(inline_children(&child, ctx))),
            _ => output.push_str(&render_node(&child, ctx)),
        }
    }
    output
}

// TODO: instance method on RenderContext
fn inline_children(node: &NodeRef, ctx: RenderContext) -> String {
    patterns::normalize_spaces(render_children(node, ctx).trim())
}

// TODO: instance method on RenderContext
fn render_list(node: &NodeRef, ordered: bool, ctx: RenderContext) -> String {
    let mut output = String::new();
    let indent = "  ".repeat(ctx.list_depth);
    let mut index = 1;
    for child in node.children().filter(|child| dom::node_name(child) == "li") {
        let marker = if ordered {
            let marker = format!("{index}.");
            index += 1;
            marker
        } else {
            "-".to_string()
        };
        let (label, nested) = render_list_item(&child, RenderContext { list_depth: ctx.list_depth + 1, ..ctx });
        output.push_str(&format!("{indent}{marker} {label}\n"));
        for line in nested.lines().filter(|line| !line.trim().is_empty()) {
            output.push_str(line);
            output.push('\n');
        }
    }
    output.push('\n');
    output
}

// TODO: instance method on RenderContext
fn render_list_item(node: &NodeRef, ctx: RenderContext) -> (String, String) {
    let mut label = String::new();
    let mut nested = String::new();

    for child in node.children() {
        match dom::node_name(&child).as_str() {
            "ul" => nested.push_str(&render_list(&child, false, ctx)),
            "ol" => nested.push_str(&render_list(&child, true, ctx)),
            _ => label.push_str(&render_node(&child, ctx)),
        }
    }

    (patterns::normalize_spaces(normalize_markdown(&label).trim()), nested)
}

fn block(value: String) -> String {
    let value = value.trim();
    if value.is_empty() { String::new() } else { format!("\n\n{value}\n\n") }
}

fn wrap_inline(marker: &str, value: String) -> String {
    if value.is_empty() { String::new() } else { format!("{marker}{value}{marker}") }
}

fn preserved_empty_inline(node: &NodeRef) -> String {
    let prev_text = node
        .previous_sibling()
        .and_then(|sibling| sibling.as_text().map(|text| text.borrow().to_string()))
        .unwrap_or_default();
    let next_text = node
        .next_sibling()
        .and_then(|sibling| sibling.as_text().map(|text| text.borrow().to_string()))
        .unwrap_or_default();

    if prev_text.ends_with(char::is_whitespace) && next_text.starts_with(char::is_whitespace) {
        " ".to_string()
    } else {
        String::new()
    }
}

fn is_heading_permalink(node: &NodeRef, label: &str) -> bool {
    let label = label.trim();
    if !(label.is_empty() || matches!(label, "#" | "¶" | "§" | "Permalink")) {
        return false;
    }

    let attrs = dom::class_id_string(node).to_ascii_lowercase();
    attrs.contains("anchor")
        || attrs.contains("permalink")
        || dom::attr(node, "href").is_some_and(|href| href.starts_with('#'))
}

fn fmt_with_comrak(markdown: &str) -> String {
    let arena = Arena::new();
    let opts = Options {
        extension: Extension {
            footnotes: true,
            math_dollars: true,
            strikethrough: true,
            table: true,
            ..Default::default()
        },
        parse: Parse { leave_footnote_definitions: true, ..Default::default() },
        ..Default::default()
    };

    let root = parse_document(&arena, markdown, &opts);
    let mut output = String::new();
    match format_commonmark(root, &opts, &mut output) {
        Err(_) => markdown.to_string(),
        Ok(_) => output.trim().to_string(),
    }
}

fn comrak_opts(opts: &MarkdownOptions) -> Options<'static> {
    let mut comrak = Options::default();

    if opts.gfm {
        comrak.extension.autolink = true;
        comrak.extension.strikethrough = true;
        comrak.extension.table = true;
        comrak.extension.tagfilter = true;
        comrak.extension.tasklist = true;
        comrak.parse.tasklist_in_table = true;
    }

    if opts.footnotes {
        comrak.extension.footnotes = true;
        comrak.extension.inline_footnotes = true;
    }

    if opts.math {
        comrak.extension.math_code = true;
        comrak.extension.math_dollars = true;
    }

    comrak.render.r#unsafe = opts.allow_raw_html;
    comrak
}

#[cfg(test)]
mod tests {
    use super::{html_to_markdown, markdown_to_html};
    use crate::{MarkdownOptions, ReadabilityOptions, extract};

    #[test]
    fn converts_representative_article_html() {
        let markdown = html_to_markdown(
            r#"<div><h1>Title</h1><p>Hello <strong>bold</strong> <a href="https://example.com">link</a>.</p><ul><li>One</li><li>Two</li></ul><pre><code>let x = 1;</code></pre></div>"#,
        );

        assert!(markdown.contains("# Title"));
        assert!(markdown.contains("Hello **bold** [link](https://example.com)."));
        assert!(markdown.contains("- One"));
        assert!(markdown.contains("    let x = 1;"));
    }

    #[test]
    fn preserves_nested_list_indentation() {
        let markdown = html_to_markdown(
            r#"<ul><li>Parent<ul><li>Child<ul><li>Grandchild</li></ul></li></ul></li><li>Sibling</li></ul>"#,
        );

        assert!(markdown.contains("- Parent\n  - Child\n    - Grandchild"), "{markdown}");
        assert!(markdown.contains("- Sibling"), "{markdown}");
    }

    #[test]
    fn converts_simple_tables_to_pipe_tables() {
        let markdown = html_to_markdown(
            r#"<table><thead><tr><th>Name</th><th>Value</th></tr></thead><tbody><tr><td>A</td><td>x|y</td></tr><tr><td>B</td><td><a href="https://example.com">z</a></td></tr></tbody></table>"#,
        );

        assert!(markdown.contains("| Name | Value |"));
        assert!(markdown.contains("| --- | --- |"));
        assert!(markdown.contains(r"| A | x\|y |"));
        assert!(markdown.contains("| B | [z](https://example.com) |"));
    }

    #[test]
    fn spaces_definition_lists() {
        let markdown = html_to_markdown(
            r#"<dl><dt><a href="struct.Value.html">Value</a></dt><dd>A borrowed value.</dd><dt><a href="trait.Serialize.html">Serialize</a></dt><dd>Serializes data.</dd></dl>"#,
        );

        assert!(
            markdown.contains("**[Value](struct.Value.html)**\n\nA borrowed value.\n\n**[Serialize](trait.Serialize.html)**\n\nSerializes data."),
            "{markdown}"
        );
    }

    #[test]
    fn renders_gfm_markdown_to_html_by_default() {
        let html = markdown_to_html(
            "| Done | Item |\n| --- | --- |\n| yes | ~~ship it~~ |\n\n- [x] publish\n",
            &MarkdownOptions::default(),
        );

        assert!(html.contains("<table>"), "{html}");
        assert!(html.contains("type=\"checkbox\""), "{html}");
        assert!(html.contains("<del>ship it</del>"), "{html}");
    }

    #[test]
    fn omits_raw_html_when_markdown_html_is_not_allowed() {
        let html = markdown_to_html("<script>alert(1)</script>\n", &MarkdownOptions::default());

        assert!(!html.contains("<script>"), "{html}");
        assert!(html.contains("raw HTML omitted"), "{html}");
    }

    #[test]
    fn allows_raw_html_when_configured() {
        let html = markdown_to_html(
            "<span data-x=\"1\">ok</span>\n",
            &MarkdownOptions { allow_raw_html: true, ..MarkdownOptions::default() },
        );

        assert!(html.contains("<span data-x=\"1\">ok</span>"), "{html}");
    }

    #[test]
    fn unwraps_layout_tables() {
        let markdown = html_to_markdown(
            r#"<table role="presentation"><tr><td><p>Left</p></td><td><p>Right <strong>side</strong></p></td></tr></table>"#,
        );

        assert!(!markdown.contains("| --- |"));
        assert!(markdown.contains("Left"));
        assert!(markdown.contains("Right **side**"));
    }

    #[test]
    fn preserves_spanning_tables_as_html() {
        let markdown =
            html_to_markdown(r#"<table><tr><th colspan="2">Group</th></tr><tr><td>A</td><td>B</td></tr></table>"#);

        assert!(markdown.contains("<table>"));
        assert!(markdown.contains("colspan=\"2\""));
        assert!(markdown.contains("<td>A</td>"));
    }

    #[test]
    fn converts_numeric_footnotes() {
        let markdown = html_to_markdown(
            r##"<p>See<sup><a id="fnref:1" href="#fn:1">1</a></sup>.</p><ol><li id="fn:1">A note <a href="#fnref:1">↩</a></li></ol>"##,
        );

        assert!(markdown.contains("See[^1]."), "{markdown}");
        assert!(markdown.contains("[^1]:\n    A note"), "{markdown}");
        assert!(!markdown.contains("↩"), "{markdown}");
        assert!(!markdown.contains("[1](#fn:1)"), "{markdown}");
    }

    #[test]
    fn converts_mediawiki_citations() {
        let markdown = html_to_markdown(
            r##"<p>Claim<sup class="reference"><a href="#cite_note-smith-2">[2]</a></sup>.</p><ol class="references"><li id="cite_note-smith-2"><span class="mw-cite-backlink"><a href="#cite_ref-smith-2">^</a></span>Smith source</li></ol>"##,
        );

        assert!(markdown.contains("Claim[^2]."), "{markdown}");
        assert!(markdown.contains("[^2]:\n    Smith source"), "{markdown}");
        assert!(!markdown.contains("cite_note"), "{markdown}");
    }

    #[test]
    fn converts_absolute_mediawiki_citation_fragments() {
        let markdown = html_to_markdown(
            r##"<p>Claim<sup class="reference"><a href="https://en.wikipedia.org/wiki/Mozilla#cite_note-smith-2">[2]</a></sup>.</p><ol class="references"><li id="cite_note-smith-2"><span class="mw-cite-backlink"><a href="#cite_ref-smith-2">^</a></span>Smith source</li></ol>"##,
        );

        assert!(markdown.contains("Claim[^2]."), "{markdown}");
        assert!(markdown.contains("[^2]:\n    Smith source"), "{markdown}");
        assert!(!markdown.contains("<sup"), "{markdown}");
        assert!(!markdown.contains("\\^"), "{markdown}");
    }

    #[test]
    fn converts_google_docs_footnotes() {
        let markdown = html_to_markdown(
            r##"<p>Word<a id="ftnt_ref1" href="#ftnt1">[1]</a></p><div id="ftnt1"><p><a href="#ftnt_ref1">[1]</a> Google Docs note.</p></div>"##,
        );

        assert!(markdown.contains("Word[^1]"), "{markdown}");
        assert!(markdown.contains("[^1]:\n    Google Docs note."), "{markdown}");
        assert!(!markdown.contains("ftnt_ref1"), "{markdown}");
    }

    #[test]
    fn converts_mathml_to_latex() {
        let markdown = html_to_markdown(
            r#"<p>When <math><mi>a</mi><mo>≠</mo><mn>0</mn></math>, solve <math display="block"><mi>x</mi><mo>=</mo><mfrac><mrow><mo>−</mo><mi>b</mi></mrow><mrow><mn>2</mn><mi>a</mi></mrow></mfrac></math></p>"#,
        );

        assert!(markdown.contains("When $a \\ne 0$"), "{markdown}");
        assert!(markdown.contains("$$\nx = \\frac{- b}{2 a}\n$$"), "{markdown}");
    }

    #[test]
    fn extracts_katex_annotations_and_latex_attrs() {
        let markdown = html_to_markdown(
            r#"<p><span class="katex"><span class="katex-mathml"><math><semantics><mrow></mrow><annotation encoding="application/x-tex">E=mc^2</annotation></semantics></math></span></span> and <span data-latex="\alpha+\beta"></span></p>"#,
        );

        assert!(markdown.contains("$E=mc^2$"), "{markdown}");
        assert!(markdown.contains("$\\alpha+\\beta$"), "{markdown}");
    }

    #[test]
    fn unwraps_equation_tables_without_flattening_math() {
        let markdown = html_to_markdown(
            r#"<table><tr><td><math display="block"><mi>y</mi><mo>=</mo><msup><mi>x</mi><mn>2</mn></msup></math></td></tr></table>"#,
        );

        assert!(markdown.contains("$$\ny = x^{2}\n$$"), "{markdown}");
        assert!(!markdown.contains("| --- |"), "{markdown}");
        assert!(!markdown.contains("<table>"), "{markdown}");
    }

    #[test]
    fn preserves_inline_highlight_and_strikethrough_semantics() {
        let markdown = html_to_markdown(
            r#"<p>Use <mark>exact match</mark>, not <del>approximate</del>, <s>old</s>, or <strike>stale</strike> terms.</p>"#,
        );

        assert!(
            markdown.contains("Use ==exact match==, not ~~approximate~~, ~~old~~, or ~~stale~~ terms."),
            "{markdown}"
        );
    }

    #[test]
    fn preserves_non_footnote_superscript_and_subscript_semantics() {
        let markdown = html_to_markdown(r#"<p>Area is 10<sup>2</sup> m<sup>2</sup>; water is H<sub>2</sub>O.</p>"#);

        assert!(
            markdown.contains("Area is 10<sup>2</sup> m<sup>2</sup>; water is H<sub>2</sub>O."),
            "{markdown}"
        );
    }

    #[test]
    fn unwraps_unlinked_mediawiki_reference_markers() {
        let markdown = html_to_markdown(r#"<p>Book<sup class="reference nowrap"><span>: 227</span></sup>.</p>"#);

        assert!(markdown.contains("Book: 227."), "{markdown}");
        assert!(!markdown.contains("<sup"), "{markdown}");
    }

    #[test]
    fn preserves_inline_svg_semantics() {
        let markdown = html_to_markdown(
            r#"<p>Status <svg viewBox="0 0 10 10" role="img" aria-label="circle"><circle cx="5" cy="5" r="4"></circle></svg> active.</p>"#,
        );

        assert!(markdown.contains("<svg "), "{markdown}");
        assert!(markdown.contains(r#"viewBox="0 0 10 10""#), "{markdown}");
        assert!(markdown.contains(r#"role="img""#), "{markdown}");
        assert!(markdown.contains(r#"aria-label="circle""#), "{markdown}");
        assert!(
            markdown.contains(r#"<circle cx="5" cy="5" r="4"></circle>"#),
            "{markdown}"
        );
        assert!(markdown.contains("active."), "{markdown}");
    }

    #[test]
    fn removes_heading_permalink_anchors() {
        let markdown =
            html_to_markdown(r##"<h2><a class="heading-anchor" href="#overview">#</a> Overview</h2><p>Body.</p>"##);

        assert!(markdown.contains("## Overview"), "{markdown}");
        assert!(!markdown.contains("[#](#overview)"), "{markdown}");
    }

    #[test]
    fn fixture_preserves_inline_semantics_in_article_markdown() {
        let fixture = lectito_fixtures::load_fixture("inline-semantic-elements").unwrap();
        let article = extract(
            &fixture.source,
            Some("https://developer.mozilla.org/en-US/docs/Web/HTML/Element/mark"),
            &ReadabilityOptions::default(),
        )
        .unwrap()
        .unwrap();

        assert!(
            article.markdown.contains("==highlight matching terms=="),
            "{}",
            article.markdown
        );
        assert!(article.markdown.contains("~~removed wording~~"), "{}", article.markdown);
        assert!(article.markdown.contains("~~obsolete labels~~"), "{}", article.markdown);
        assert!(article.markdown.contains("H<sub>2</sub>O"), "{}", article.markdown);
        assert!(
            article.markdown.contains("10<sup>2</sup> m<sup>2</sup>"),
            "{}",
            article.markdown
        );
        assert!(article.markdown.contains("<svg "), "{}", article.markdown);
        assert!(
            article
                .markdown
                .contains("[Video](https://www.youtube.com/watch?v=LtOGa5M8AuU)"),
            "{}",
            article.markdown
        );
    }

    #[test]
    fn prefers_largest_srcset_candidate_and_preserves_image_title() {
        let markdown = html_to_markdown(
            r#"<p><img src="small.jpg" srcset="https://cdn.example.com/image,w_400.jpg 400w, https://cdn.example.com/image,w_1600.jpg 1600w" alt="Hero" title="Launch view"></p>"#,
        );

        assert!(
            markdown.contains(r#"![Hero](https://cdn.example.com/image,w_1600.jpg "Launch view")"#),
            "{markdown}"
        );
        assert!(!markdown.contains("small.jpg"), "{markdown}");
    }

    #[test]
    fn normalizes_picture_lazy_and_placeholder_images() {
        let markdown = html_to_markdown(
            r#"<picture><source data-srcset="wide.webp 1200w, wide@2x.webp 2400w"><img src="data:image/gif;base64,R0lGODlhAQABAAAAACw=" data-src="fallback.jpg" alt="Wide"></picture>"#,
        );

        assert!(markdown.contains("![Wide](wide@2x.webp)"), "{markdown}");
        assert!(!markdown.contains("data:image"), "{markdown}");
    }

    #[test]
    fn converts_image_figures_with_captions_but_leaves_content_wrappers() {
        let image_figure = html_to_markdown(
            r#"<figure><img src="photo.jpg" alt="Photo"><figcaption>Photo caption <em>here</em>.</figcaption></figure>"#,
        );

        assert!(
            image_figure.contains("![Photo](photo.jpg)\n\nPhoto caption *here*."),
            "{image_figure}"
        );

        let wrapper = html_to_markdown(
            r#"<figure><h2>Section</h2><p>Intro text.</p><img src="photo.jpg" alt="Photo"><figcaption>Caption</figcaption></figure>"#,
        );

        assert!(wrapper.contains("## Section"), "{wrapper}");
        assert!(wrapper.contains("Intro text."), "{wrapper}");
        assert!(wrapper.contains("![Photo](photo.jpg)"), "{wrapper}");
    }

    #[test]
    fn converts_figures_with_wrapped_images() {
        let markdown = html_to_markdown(
            r#"<figure><p><a href="photo.html"><picture><source srcset="photo-large.jpg 1200w"><img src="photo-small.jpg" alt="Photo"></picture></a></p><figcaption>Wrapped caption.</figcaption></figure>"#,
        );

        assert!(
            markdown.contains("![Photo](photo-large.jpg)\n\nWrapped caption."),
            "{markdown}"
        );
        assert!(!markdown.contains("photo-small.jpg"), "{markdown}");
        assert!(!markdown.contains("photo.html"), "{markdown}");
    }

    #[test]
    fn converts_youtube_and_twitter_embeds() {
        let markdown = html_to_markdown(
            r#"<iframe src="https://www.youtube.com/embed/dQw4w9WgXcQ"></iframe><blockquote class="twitter-tweet"><p>Tweet</p><a href="https://twitter.com/example/status/12345">May 1</a></blockquote>"#,
        );

        assert!(
            markdown.contains("[Video](https://www.youtube.com/watch?v=dQw4w9WgXcQ)"),
            "{markdown}"
        );
        assert!(
            markdown.contains("[Video](https://twitter.com/example/status/12345)"),
            "{markdown}"
        );
        assert!(!markdown.contains("> Tweet"), "{markdown}");
    }
}
