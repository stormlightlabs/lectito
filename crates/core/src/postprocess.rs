use super::metadata::text_similarity;
use super::parse::{Document, Element};
use super::scoring::hash_only_link_coefficient;
use regex::Regex;
use std::collections::HashSet;
use std::sync::OnceLock;
use url::Url;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum TableKind {
    Data,
    Layout,
}

/// Configuration for HTML post-processing cleanup
#[derive(Debug, Clone)]
pub struct PostProcessConfig {
    /// Whether to remove empty nodes
    pub remove_empty_nodes: bool,
    /// Maximum passes for removing empty nodes
    pub max_empty_node_passes: usize,
    /// Whether to remove nodes with high link density
    pub remove_high_link_density: bool,
    /// Maximum link density threshold (0.0 to 1.0)
    pub max_link_density: f64,
    /// Whether to clean up nested DIVs with single children
    pub clean_nested_divs: bool,
    /// Whether to remove conditional comments
    pub remove_conditional_comments: bool,
    /// Whether to strip all images
    pub strip_images: bool,
    /// Whether to keep class attributes (default: false)
    pub keep_classes: bool,
    /// Whether to run the HTML standardization pass
    pub standardize_html: bool,
    /// Whether to remove content-pattern tails and metadata after scoring
    pub remove_content_patterns: bool,
    /// Custom strip patterns (class/ID regex)
    pub strip_patterns: Option<String>,
    /// Base URL for converting relative URLs
    pub base_url: Option<Url>,
}

impl Default for PostProcessConfig {
    fn default() -> Self {
        Self {
            remove_empty_nodes: true,
            max_empty_node_passes: 10,
            remove_high_link_density: true,
            max_link_density: 0.5,
            clean_nested_divs: true,
            remove_conditional_comments: true,
            strip_images: false,
            keep_classes: false,
            standardize_html: true,
            remove_content_patterns: true,
            strip_patterns: None,
            base_url: None,
        }
    }
}

/// Post-process extracted HTML by cleaning up remaining unwanted content
pub fn postprocess_html(html: &str, config: &PostProcessConfig) -> String {
    let mut processed = html.to_string();

    if config.remove_conditional_comments {
        processed = remove_conditional_comments(&processed);
    }

    if config.strip_images {
        processed = strip_images(&processed);
    }

    if config.standardize_html {
        processed = standardize_html(&processed);
        processed = rewrite_tables(&processed);
    }

    processed = strip_unwanted_attributes(&processed, config.keep_classes);

    processed = remove_doc_chrome_nodes(&processed);
    processed = remove_doc_chrome_text_blocks(&processed);

    if config.remove_content_patterns {
        processed = remove_content_patterns(&processed);
    }

    if config.remove_empty_nodes {
        processed = remove_empty_nodes(&processed, config.max_empty_node_passes);
    }

    if config.remove_high_link_density {
        processed = remove_high_link_density_nodes(&processed, config.max_link_density);
    }

    if let Some(patterns) = &config.strip_patterns {
        processed = strip_patterns(&processed, patterns);
    }

    if config.clean_nested_divs {
        processed = clean_nested_divs(&processed);
    }

    if let Some(base_url) = &config.base_url {
        processed = fix_relative_urls(&processed, base_url);
    }

    processed
}

/// Remove Internet Explorer conditional comments
///
/// IE conditional comments have the format:
/// <!--[if condition]>...<![endif]-->
/// <!--[if !IE]>...<![endif]-->
fn remove_conditional_comments(html: &str) -> String {
    let re = Regex::new(r#"(?s)<!--\[if[^\]]*\]>.*?<!\[endif\]-->|<!--<!\[if[^\]]*\]>.*?<!\[endif\]-->"#).unwrap();
    re.replace_all(html, "").to_string()
}

/// Strip all image tags from HTML
fn strip_images(html: &str) -> String {
    let re = Regex::new(r#"<img[^>]*>"#).unwrap();
    re.replace_all(html, "").to_string()
}

/// Strip all class attributes from HTML
fn strip_classes(html: &str) -> String {
    let re = Regex::new(r#"\s+class=["'][^"']*["']"#).unwrap();
    re.replace_all(html, "").to_string()
}

fn escape_html(value: &str) -> String {
    value
        .replace('&', "&amp;")
        .replace('<', "&lt;")
        .replace('>', "&gt;")
        .replace('"', "&quot;")
        .replace('\'', "&#39;")
}

fn table_selector_attrs(table: &Element<'_>) -> String {
    [
        table.attr("class").unwrap_or(""),
        table.attr("id").unwrap_or(""),
        table.attr("role").unwrap_or(""),
        table.attr("summary").unwrap_or(""),
    ]
    .join(" ")
    .to_lowercase()
}

fn table_cell_span(cell: &Element<'_>, attr: &str) -> usize {
    cell.attr(attr)
        .and_then(|value| value.parse::<usize>().ok())
        .filter(|value| *value > 0)
        .unwrap_or(1)
}

fn table_dimensions(table: &Element<'_>) -> (usize, usize) {
    let rows = table.select("tr").unwrap_or_default();
    let row_count = rows.len();
    let max_cols = rows
        .iter()
        .map(|row| {
            row.select("th, td")
                .unwrap_or_default()
                .iter()
                .map(|cell| table_cell_span(cell, "colspan"))
                .sum::<usize>()
        })
        .max()
        .unwrap_or(0);

    (row_count, max_cols)
}

pub(crate) fn classify_table_element(table: &Element<'_>) -> TableKind {
    let mut data_signals = 0usize;
    let mut layout_signals = 0usize;

    let role = table.attr("role").unwrap_or("");
    if matches!(role, "presentation" | "none") {
        layout_signals += 3;
    }
    if matches!(role, "table" | "grid" | "treegrid") {
        data_signals += 2;
    }

    let attrs = table_selector_attrs(table);
    if attrs.contains("layout") || attrs.contains("wrapper") || attrs.contains("grid") {
        layout_signals += 1;
    }
    if attrs.contains("schedule") || attrs.contains("standings") || attrs.contains("stats") {
        data_signals += 1;
    }

    let captions = table.select("caption").unwrap_or_default();
    let thead = table.select("thead").unwrap_or_default();
    let tfoot = table.select("tfoot").unwrap_or_default();
    let headers = table.select("th").unwrap_or_default();
    let nested_tables = table.select("table").unwrap_or_default().len().saturating_sub(1);
    let (row_count, col_count) = table_dimensions(table);
    let cell_count = row_count.saturating_mul(col_count);

    if !captions.is_empty() {
        data_signals += 2;
    }
    if !thead.is_empty() || !tfoot.is_empty() {
        data_signals += 2;
    }
    if !headers.is_empty() {
        data_signals += 2;
    }
    if headers
        .iter()
        .any(|cell| cell.attr("scope").is_some() || cell.attr("abbr").is_some())
        || table
            .select("td[headers], th[headers]")
            .ok()
            .is_some_and(|cells| !cells.is_empty())
    {
        data_signals += 1;
    }

    if row_count >= 2 && col_count >= 2 {
        data_signals += 2;
    }
    if row_count >= 4 && col_count >= 2 {
        data_signals += 1;
    }
    if cell_count >= 8 {
        data_signals += 1;
    }

    if nested_tables > 0 {
        layout_signals += 2;
    }
    if row_count <= 1 || col_count <= 1 {
        layout_signals += 1;
    }
    if table
        .attr("width")
        .and_then(|value| value.parse::<usize>().ok())
        .is_some_and(|value| value > 400)
        || table
            .attr("align")
            .is_some_and(|value| value.eq_ignore_ascii_case("center"))
    {
        layout_signals += 1;
    }
    if headers.is_empty() && captions.is_empty() && thead.is_empty() && tfoot.is_empty() && cell_count <= 4 {
        layout_signals += 1;
    }

    if data_signals >= layout_signals { TableKind::Data } else { TableKind::Layout }
}

fn unwrap_layout_table(table: &Element<'_>) -> String {
    let mut parts = Vec::new();

    for row in table.select("tr").unwrap_or_default() {
        for cell in row.select("th, td").unwrap_or_default() {
            let cell_html = rewrite_tables(&cell.inner_html()).trim().to_string();
            if !cell_html.is_empty() {
                parts.push(cell_html);
            }
        }
    }

    if parts.is_empty() {
        let fallback = rewrite_tables(&table.inner_html());
        fallback.trim().to_string()
    } else {
        parts.join("\n")
    }
}

fn rewrite_tables(html: &str) -> String {
    if !html.contains("<table") {
        return html.to_string();
    }

    let Ok(doc) = Document::parse(html) else {
        return html.to_string();
    };

    let mut replacements = Vec::new();
    for table in doc.select("table").unwrap_or_default() {
        if classify_table_element(&table) == TableKind::Layout {
            replacements.push((table.outer_html(), unwrap_layout_table(&table)));
        }
    }

    replace_html_snippets(html.to_string(), replacements)
}

fn allowed_attributes() -> &'static HashSet<&'static str> {
    static ATTRS: OnceLock<HashSet<&'static str>> = OnceLock::new();
    ATTRS.get_or_init(|| {
        [
            "alt",
            "allow",
            "allowfullscreen",
            "aria-label",
            "checked",
            "colspan",
            "controls",
            "data-callout",
            "data-callout-title",
            "data-lang",
            "data-latex",
            "dir",
            "display",
            "frameborder",
            "headers",
            "height",
            "href",
            "kind",
            "label",
            "lang",
            "poster",
            "role",
            "rowspan",
            "src",
            "srclang",
            "srcset",
            "title",
            "type",
            "width",
            "xmlns",
        ]
        .into_iter()
        .collect()
    })
}

fn code_language_regex() -> &'static Regex {
    static REGEX: OnceLock<Regex> = OnceLock::new();
    REGEX.get_or_init(|| Regex::new(r"(?i)(?:language|lang|highlight-source|brush)[-:=\s]+([a-z0-9_+-]+)").unwrap())
}

fn reference_fragment_regex() -> &'static Regex {
    static REGEX: OnceLock<Regex> = OnceLock::new();
    REGEX.get_or_init(|| Regex::new(r"(?i)(?:cite_(?:note|ref)-|fnref:?|footnote-|reference-|fn:?|^r|^b)").unwrap())
}

fn reference_label_regex() -> &'static Regex {
    static REGEX: OnceLock<Regex> = OnceLock::new();
    REGEX.get_or_init(|| Regex::new(r"\d+[A-Za-z0-9_-]*").unwrap())
}

fn replace_html_snippets(mut html: String, mut replacements: Vec<(String, String)>) -> String {
    replacements.sort_by_key(|(old, _)| std::cmp::Reverse(old.len()));
    for (old, new) in replacements {
        if old.is_empty() || old == new {
            continue;
        }
        if let Some(idx) = html.find(&old) {
            html.replace_range(idx..idx + old.len(), &new);
        }
    }
    html
}

fn standardize_html(html: &str) -> String {
    let mut processed = html.to_string();
    processed = standardize_code_blocks(&processed);
    processed = standardize_callouts(&processed);
    processed = normalize_footnotes(&processed);
    processed = normalize_math(&processed);
    processed = flatten_wrapper_divs(&processed);
    processed
}

fn normalize_language(value: &str) -> Option<String> {
    let trimmed = value.trim().trim_matches(|c: char| c == '"' || c == '\'');
    if trimmed.is_empty() {
        return None;
    }

    let lower = trimmed.to_lowercase();
    let candidate = code_language_regex()
        .captures(&lower)
        .and_then(|caps| caps.get(1).map(|m| m.as_str().to_string()))
        .unwrap_or(lower);
    let cleaned = candidate
        .chars()
        .filter(|ch| ch.is_ascii_alphanumeric() || matches!(ch, '_' | '+' | '-'))
        .collect::<String>();

    (!cleaned.is_empty()).then_some(cleaned)
}

fn detect_code_language(pre: &crate::parse::Element<'_>, code: Option<&crate::parse::Element<'_>>) -> Option<String> {
    let candidates = [
        pre.attr("data-lang"),
        pre.attr("data-language"),
        pre.attr("lang"),
        pre.attr("class"),
        code.and_then(|element| element.attr("data-lang")),
        code.and_then(|element| element.attr("data-language")),
        code.and_then(|element| element.attr("lang")),
        code.and_then(|element| element.attr("class")),
    ];

    candidates.into_iter().flatten().find_map(normalize_language)
}

fn standardize_code_blocks(html: &str) -> String {
    if !html.contains("<pre") && !html.contains("<code") {
        return html.to_string();
    }

    let Ok(doc) = Document::parse(html) else {
        return html.to_string();
    };

    let mut replacements = Vec::new();

    for pre in doc.select("pre").unwrap_or_default() {
        let nested_code = pre.select("code").unwrap_or_default();
        let replacement = if let Some(code) = nested_code.first() {
            let language = detect_code_language(&pre, Some(code));
            let class_attr = language
                .as_ref()
                .map(|lang| format!(r#" class="language-{}""#, lang))
                .unwrap_or_default();
            format!(r#"<pre><code{}>{}</code></pre>"#, class_attr, code.inner_html())
        } else {
            let language = detect_code_language(&pre, None);
            let class_attr = language
                .as_ref()
                .map(|lang| format!(r#" class="language-{}""#, lang))
                .unwrap_or_default();
            format!(r#"<pre><code{}>{}</code></pre>"#, class_attr, pre.inner_html())
        };

        replacements.push((pre.outer_html(), replacement));
    }

    replace_html_snippets(html.to_string(), replacements)
}

fn normalize_callout_type(value: &str) -> String {
    match value.to_ascii_lowercase().as_str() {
        "danger" | "error" => "warning".to_string(),
        "success" => "tip".to_string(),
        "secondary" | "info" => "note".to_string(),
        "" => "note".to_string(),
        other => other.to_string(),
    }
}

fn remove_nested_snippet(mut html: String, snippet: Option<String>) -> String {
    if let Some(snippet) = snippet
        && !snippet.is_empty()
        && let Some(idx) = html.find(&snippet)
    {
        html.replace_range(idx..idx + snippet.len(), "");
    }
    html.trim().to_string()
}

fn build_callout_html(callout_type: &str, title: &str, body_html: &str) -> String {
    let heading = if title.is_empty() {
        String::new()
    } else {
        format!(r#"<p><strong>{}</strong></p>"#, escape_html(title))
    };
    format!(
        r#"<blockquote data-callout="{}">{}{}</blockquote>"#,
        callout_type,
        heading,
        body_html.trim()
    )
}

fn standardize_callouts(html: &str) -> String {
    if !html.contains("markdown-alert") && !html.contains("callout") && !html.contains("alert-") {
        return html.to_string();
    }

    let Ok(doc) = Document::parse(html) else {
        return html.to_string();
    };

    let mut replacements = Vec::new();

    for alert in doc.select(".markdown-alert").unwrap_or_default() {
        let classes = alert.attr("class").unwrap_or("");
        let callout_type = classes
            .split_whitespace()
            .find_map(|class_name| class_name.strip_prefix("markdown-alert-"))
            .map(normalize_callout_type)
            .unwrap_or_else(|| "note".to_string());
        let title_element = alert
            .select(".markdown-alert-title")
            .unwrap_or_default()
            .first()
            .cloned();
        let title = title_element
            .as_ref()
            .map(|element| normalize_text(&element.text()))
            .unwrap_or_else(|| callout_type.to_ascii_uppercase());
        let body_html = remove_nested_snippet(alert.inner_html(), title_element.map(|element| element.outer_html()));
        replacements.push((
            alert.outer_html(),
            build_callout_html(&callout_type, &title, &body_html),
        ));
    }

    for alert in doc
        .select("aside[class*=\"callout\"], div.alert[class*=\"alert-\"]")
        .unwrap_or_default()
    {
        let classes = alert.attr("class").unwrap_or("");
        let callout_type = classes
            .split_whitespace()
            .find_map(|class_name| {
                class_name
                    .strip_prefix("callout-")
                    .or_else(|| class_name.strip_prefix("alert-"))
                    .map(normalize_callout_type)
            })
            .unwrap_or_else(|| "note".to_string());
        let title_element = alert
            .select(".alert-heading, .alert-title, .callout-title")
            .unwrap_or_default()
            .first()
            .cloned();
        let title = title_element
            .as_ref()
            .map(|element| normalize_text(&element.text()))
            .unwrap_or_else(|| {
                let mut chars = callout_type.chars();
                chars
                    .next()
                    .map(|first| first.to_ascii_uppercase().to_string() + chars.as_str())
                    .unwrap_or_else(|| "Note".to_string())
            });
        let body_html = remove_nested_snippet(alert.inner_html(), title_element.map(|element| element.outer_html()));
        replacements.push((
            alert.outer_html(),
            build_callout_html(&callout_type, &title, &body_html),
        ));
    }

    replace_html_snippets(html.to_string(), replacements)
}

fn ensure_math_xmlns(math_html: String) -> String {
    if math_html.contains("xmlns=") {
        math_html
    } else {
        math_html.replacen("<math", r#"<math xmlns="http://www.w3.org/1998/Math/MathML""#, 1)
    }
}

fn build_math_html(content: &str, display: &str) -> String {
    let escaped = escape_html(content);
    format!(
        r#"<math xmlns="http://www.w3.org/1998/Math/MathML" display="{}" data-latex="{}">{}</math>"#,
        display, escaped, escaped
    )
}

fn normalize_math(html: &str) -> String {
    if !html.contains("katex")
        && !html.contains("MathJax")
        && !html.contains("mjx-container")
        && !html.contains(r#"type="math/"#)
    {
        return html.to_string();
    }

    let Ok(doc) = Document::parse(html) else {
        return html.to_string();
    };

    let mut replacements = Vec::new();

    for script in doc.select(r#"script[type^="math/"]"#).unwrap_or_default() {
        let text = normalize_text(&script.text());
        if !text.is_empty() {
            replacements.push((script.outer_html(), build_math_html(&text, "inline")));
        }
    }

    for element in doc
        .select(".katex, .MathJax, .MathJax_Display, mjx-container")
        .unwrap_or_default()
    {
        let display = if element
            .attr("class")
            .is_some_and(|classes| classes.contains("display") || classes.contains("Display"))
            || element
                .attr("display")
                .is_some_and(|value| value.eq_ignore_ascii_case("block"))
        {
            "block"
        } else {
            "inline"
        };

        let replacement = if let Some(math) = element.select("math").unwrap_or_default().first() {
            ensure_math_xmlns(math.outer_html())
        } else if let Some(annotation) = element
            .select(r#"annotation[encoding="application/x-tex"]"#)
            .unwrap_or_default()
            .first()
        {
            build_math_html(&normalize_text(&annotation.text()), display)
        } else if let Some(label) = element.attr("aria-label") {
            build_math_html(label, display)
        } else {
            let text = normalize_text(&element.text());
            if text.is_empty() {
                continue;
            }
            build_math_html(&text, display)
        };

        replacements.push((element.outer_html(), replacement));
    }

    replace_html_snippets(html.to_string(), replacements)
}

fn normalize_reference_id(value: &str) -> Option<String> {
    let fragment = value.rsplit('#').next().unwrap_or(value).trim().to_lowercase();
    if fragment.is_empty() {
        return None;
    }
    let stripped = reference_fragment_regex().replace_all(&fragment, "").to_string();
    let cleaned = stripped
        .chars()
        .filter(|ch| ch.is_ascii_alphanumeric() || matches!(ch, '_' | '-'))
        .collect::<String>();
    (!cleaned.is_empty()).then_some(cleaned)
}

fn reference_label(text: &str, fallback: &str) -> String {
    reference_label_regex()
        .find(text)
        .map(|m| m.as_str().to_string())
        .unwrap_or_else(|| fallback.to_string())
}

fn build_footnote_ref_html(id: &str, label: &str) -> String {
    format!(
        r##"<sup id="fnref:{}"><a href="#fn:{}">[{}]</a></sup>"##,
        id,
        id,
        escape_html(label)
    )
}

fn standardize_footnote_item(item: &crate::parse::Element<'_>, position: usize) -> String {
    let raw_id = item
        .attr("id")
        .and_then(normalize_reference_id)
        .or_else(|| item.attr("data-counter").and_then(normalize_reference_id))
        .unwrap_or_else(|| position.to_string());
    let label = reference_label(&item.text(), &raw_id);
    let mut content_html = item.inner_html();
    if let Ok(backrefs) = item.select("a[href^=\"#fnref\"], a[href*=\"cite_ref\"], a.footnote-backref") {
        for backref in backrefs {
            content_html = remove_nested_snippet(content_html, Some(backref.outer_html()));
        }
    }
    content_html = Regex::new(r#"(?is)^\s*(?:<sup[^>]*>.*?</sup>\s*)?(?:\[\s*)?\d+[A-Za-z0-9_-]*(?:\s*\])?[:.)-]?\s*"#)
        .unwrap()
        .replace(&content_html, "")
        .to_string();
    if !content_html.trim_start().starts_with('<') || !content_html.contains("<p") {
        content_html = format!("<p>{}</p>", content_html.trim());
    }

    format!(
        r##"<li id="fn:{}">{}<a href="#fnref:{}" class="footnote-backref">↩</a></li>"##,
        raw_id,
        content_html.trim(),
        raw_id
    )
    .replace(&format!("[{}]", raw_id), &format!("[{}]", label))
}

fn normalize_footnotes(html: &str) -> String {
    if !html.contains("cite_note")
        && !html.contains("cite_ref")
        && !html.contains("references")
        && !html.contains("footnote")
        && !html.contains("doc-endnotes")
        && !html.contains("doc-footnotes")
    {
        return html.to_string();
    }

    let Ok(doc) = Document::parse(html) else {
        return html.to_string();
    };

    let mut replacements = Vec::new();

    for reference in doc
        .select(
            "sup.reference, sup[id^=\"fnr\"], sup.footnoteref, a[href*=\"cite_note\"], a[href*=\"cite_ref\"], a[href^=\"#fn\"], a[href^=\"#footnote\"], a[role=\"doc-biblioref\"], a[id^=\"fnref\"], span.footnote-link",
        )
        .unwrap_or_default()
    {
        let raw_id = reference
            .attr("href")
            .and_then(normalize_reference_id)
            .or_else(|| reference.attr("id").and_then(normalize_reference_id))
            .or_else(|| normalize_reference_id(&normalize_text(&reference.text())));
        let Some(id) = raw_id else {
            continue;
        };
        let label = reference_label(&reference.text(), &id);
        replacements.push((reference.outer_html(), build_footnote_ref_html(&id, &label)));
    }

    for container in doc
        .select(
            "ol.references, ol.footnotes, div.footnotes, section.footnotes, section[role=\"doc-endnotes\"], section[role=\"doc-footnotes\"], section[role=\"doc-bibliography\"], div[role=\"doc-endnotes\"], div[role=\"doc-footnotes\"], div[role=\"doc-bibliography\"], #footnotes",
        )
        .unwrap_or_default()
    {
        let items = container.select("li").unwrap_or_default();
        if items.is_empty() {
            continue;
        }

        let normalized_items = items
            .iter()
            .enumerate()
            .map(|(index, item)| standardize_footnote_item(item, index + 1))
            .collect::<Vec<_>>()
            .join("");
        let replacement = format!(r#"<section id="footnotes"><ol>{}</ol></section>"#, normalized_items);
        replacements.push((container.outer_html(), replacement));
    }

    replace_html_snippets(html.to_string(), replacements)
}

fn flatten_wrapper_divs(html: &str) -> String {
    let mut result = clean_nested_divs(html);
    let wrapper_re = Regex::new(
        r#"(?is)<div(?:\s[^>]*?(?:class|id)=["'][^"']*(?:container|content-area|content-wrapper|inner|layout|outer|wrapper)[^"']*["'][^>]*)>\s*((?:<(?:div|section|article|p|h[1-6]|ul|ol|pre|blockquote|figure|table)[^>]*>.*?</(?:div|section|article|p|h[1-6]|ul|ol|pre|blockquote|figure|table)>\s*)+)</div>"#,
    )
    .unwrap();

    for _ in 0..4 {
        let next = wrapper_re.replace_all(&result, "$1").to_string();
        if next == result {
            break;
        }
        result = next;
    }

    result
}

fn is_allowed_id(value: &str) -> bool {
    value == "footnotes" || value.starts_with("fn:") || value.starts_with("fnref:")
}

fn filter_allowed_classes(value: &str) -> String {
    value
        .split_whitespace()
        .filter(|class_name| class_name.starts_with("language-") || *class_name == "footnote-backref")
        .collect::<Vec<_>>()
        .join(" ")
}

fn strip_unwanted_attributes(html: &str, keep_classes: bool) -> String {
    let mut output = String::new();
    let mut rewriter = lol_html::HtmlRewriter::new(
        lol_html::Settings {
            element_content_handlers: vec![lol_html::element!("*", move |el| {
                let attrs = el
                    .attributes()
                    .iter()
                    .map(|attr| (attr.name(), attr.value()))
                    .collect::<Vec<_>>();

                for (name, value) in attrs {
                    if name == "class" {
                        if keep_classes {
                            continue;
                        }
                        let filtered = filter_allowed_classes(&value);
                        if filtered.is_empty() {
                            el.remove_attribute("class");
                        } else {
                            el.set_attribute("class", &filtered).ok();
                        }
                        continue;
                    }

                    if name == "id" {
                        if !is_allowed_id(&value) {
                            el.remove_attribute("id");
                        }
                        continue;
                    }

                    if !allowed_attributes().contains(name.as_str()) {
                        el.remove_attribute(&name);
                    }
                }
                Ok(())
            })],
            ..Default::default()
        },
        |c: &[u8]| {
            output.push_str(&String::from_utf8_lossy(c));
        },
    );

    if rewriter.write(html.as_bytes()).is_err() || rewriter.end().is_err() {
        return if keep_classes { html.to_string() } else { strip_classes(html) };
    }

    if output.is_empty() { html.to_string() } else { output }
}

fn content_root_selector() -> &'static str {
    "#__lectito_content_root__"
}

fn word_regex() -> &'static Regex {
    static REGEX: OnceLock<Regex> = OnceLock::new();
    REGEX.get_or_init(|| Regex::new(r"\b[\w'-]+\b").unwrap())
}

fn boundary_date_regex() -> &'static Regex {
    static REGEX: OnceLock<Regex> = OnceLock::new();
    REGEX.get_or_init(|| {
        Regex::new(
            r"(?i)\b(?:Jan(?:uary)?|Feb(?:ruary)?|Mar(?:ch)?|Apr(?:il)?|May|Jun(?:e)?|Jul(?:y)?|Aug(?:ust)?|Sep(?:t(?:ember)?)?|Oct(?:ober)?|Nov(?:ember)?|Dec(?:ember)?)\b",
        )
        .unwrap()
    })
}

fn read_time_regex() -> &'static Regex {
    static REGEX: OnceLock<Regex> = OnceLock::new();
    REGEX.get_or_init(|| Regex::new(r"(?i)\b\d+\s*(?:min|minute)s?\s+read\b").unwrap())
}

fn byline_regex() -> &'static Regex {
    static REGEX: OnceLock<Regex> = OnceLock::new();
    REGEX.get_or_init(|| Regex::new(r"(?i)^by\s+\S").unwrap())
}

fn from_wikipedia_regex() -> &'static Regex {
    static REGEX: OnceLock<Regex> = OnceLock::new();
    REGEX.get_or_init(|| Regex::new(r"(?i)^from wikipedia(?:, the free encyclopedia)?$").unwrap())
}

fn newsletter_regex() -> &'static Regex {
    static REGEX: OnceLock<Regex> = OnceLock::new();
    REGEX.get_or_init(|| {
        Regex::new(
            r"(?i)\bsubscribe\b[\s\S]{0,40}\bnewsletter\b|\bnewsletter\b[\s\S]{0,40}\bsubscribe\b|\bsign[- ]up\b[\s\S]{0,80}\b(?:newsletter|email alert)\b",
        )
        .unwrap()
    })
}

fn trailing_heading_regex() -> &'static Regex {
    static REGEX: OnceLock<Regex> = OnceLock::new();
    REGEX.get_or_init(|| {
        Regex::new(
            r"(?i)^(see also|references|notes|external links|further reading|bibliography|sources|citations|related (?:posts?|articles?|content|stories|reads?|reading)|you (?:might|may|could) (?:also )?(?:like|enjoy|be interested in)|read (?:next|more|also)|more (?:from|articles?|posts?|like this)|about (?:the )?author)$",
        )
        .unwrap()
    })
}

fn boilerplate_regex() -> &'static Regex {
    static REGEX: OnceLock<Regex> = OnceLock::new();
    REGEX.get_or_init(|| {
        Regex::new(
            r"(?i)^(this (?:article|story|piece) (?:appeared|was published|originally appeared) in|a version of this (?:article|story) (?:appeared|was published) in|originally (?:published|appeared) (?:in|on|at)|any re-?use permitted|©\s*(?:copyright\s+)?\d{4}|comments?|leave a (?:comment|reply))",
        )
        .unwrap()
    })
}

fn count_words(text: &str) -> usize {
    word_regex().find_iter(text).count()
}

fn normalize_text(text: &str) -> String {
    text.split_whitespace().collect::<Vec<_>>().join(" ")
}

fn title_word_count(text: &str) -> usize {
    normalize_text(text).split_whitespace().count()
}

fn should_strip_title_heading(text: &str, title: &str) -> bool {
    let normalized = normalize_text(text);
    !normalized.is_empty() && text_similarity(&normalized, title) > 0.75
}

pub(crate) fn dedupe_title_headings(html: &str, title: Option<&str>) -> String {
    let Some(title) = title.map(normalize_text).filter(|title| !title.is_empty()) else {
        return html.to_string();
    };

    let wrapped = format!(r#"<div id="__lectito_title_root__">{}</div>"#, html);
    let Ok(doc) = Document::parse(&wrapped) else {
        return html.to_string();
    };

    let mut snippets = Vec::new();
    let title_words = title_word_count(&title);

    for child in doc.select("#__lectito_title_root__ > *").unwrap_or_default() {
        let tag = child.tag_name();
        let child_text = normalize_text(&child.text());
        let child_words = title_word_count(&child_text);

        let remove_child = match tag.as_str() {
            "h1" | "h2" => should_strip_title_heading(&child_text, &title),
            "header" => {
                let heading = child
                    .select("h1, h2")
                    .unwrap_or_default()
                    .first()
                    .cloned()
                    .map(|heading| normalize_text(&heading.text()))
                    .unwrap_or_default();
                !heading.is_empty()
                    && should_strip_title_heading(&heading, &title)
                    && child_words <= title_words.saturating_add(12).max(20)
            }
            _ => false,
        };

        if remove_child {
            snippets.push(child.outer_html());
            continue;
        }

        if child_words > 20 {
            break;
        }
    }

    let full_text = normalize_text(
        &doc.select("#__lectito_title_root__")
            .unwrap_or_default()
            .first()
            .map(|root| root.text())
            .unwrap_or_default(),
    );

    for heading in doc
        .select("#__lectito_title_root__ h1, #__lectito_title_root__ h2")
        .unwrap_or_default()
    {
        let heading_text = normalize_text(&heading.text());
        if !should_strip_title_heading(&heading_text, &title) {
            continue;
        }

        let text_position = full_text.find(&heading_text).unwrap_or(usize::MAX);
        let words_before =
            if text_position == usize::MAX { usize::MAX } else { title_word_count(&full_text[..text_position]) };

        if words_before <= 20 {
            let heading_html = heading.outer_html();
            if !snippets.iter().any(|snippet| snippet == &heading_html) {
                snippets.push(heading_html);
            }
        }
    }

    if snippets.is_empty() {
        return html.to_string();
    }

    let result = remove_snippets(wrapped, snippets);
    if let Ok(doc) = Document::parse(&result)
        && let Ok(mut nodes) = doc.select("#__lectito_title_root__")
        && let Some(root) = nodes.drain(..).next()
    {
        root.inner_html()
    } else {
        html.to_string()
    }
}

fn remove_snippets(mut html: String, mut snippets: Vec<String>) -> String {
    snippets.sort_by_key(|b| std::cmp::Reverse(b.len()));
    for snippet in snippets {
        if snippet.is_empty() {
            continue;
        }
        if let Some(idx) = html.find(&snippet) {
            html.replace_range(idx..idx + snippet.len(), "");
        }
    }
    html
}

fn is_boundary_metadata(text: &str, words: usize, pos: usize, full_len: usize) -> bool {
    if words == 0 {
        return false;
    }

    let near_start = pos <= 700;
    let near_end = full_len.saturating_sub(pos + text.len()) <= 250;

    (from_wikipedia_regex().is_match(text) && pos <= 200)
        || (byline_regex().is_match(text) && words <= 12 && near_start)
        || (boundary_date_regex().is_match(text) && read_time_regex().is_match(text) && words <= 12 && near_start)
        || (newsletter_regex().is_match(text) && words <= 60 && (near_start || near_end))
}

fn remove_content_patterns(html: &str) -> String {
    let wrapped = format!(r#"<div id="__lectito_content_root__">{}</div>"#, html);
    let Ok(doc) = Document::parse(&wrapped) else {
        return html.to_string();
    };

    let root_selector = content_root_selector();
    let root = doc.select(root_selector).ok().and_then(|els| els.into_iter().next());
    let Some(root) = root else {
        return html.to_string();
    };

    let top_level_selector = format!("{} > *", root_selector);
    let top_level = doc.select(&top_level_selector).unwrap_or_default();
    if top_level.is_empty() {
        return html.to_string();
    }

    let full_text = normalize_text(&root.text());
    let mut snippets = Vec::new();
    let mut words_before = 0usize;
    let mut truncate_from: Option<String> = None;

    for child in &top_level {
        let text = normalize_text(&child.text());
        let words = count_words(&text);
        let tag = child.tag_name();
        let pos = full_text.find(&text).unwrap_or(usize::MAX);

        if words_before <= 60 && is_boundary_metadata(&text, words, pos, full_text.len()) {
            snippets.push(child.outer_html());
            continue;
        }

        if matches!(tag.as_str(), "h2" | "h3" | "h4" | "h5" | "h6")
            && words_before >= 8
            && trailing_heading_regex().is_match(&text)
        {
            truncate_from = Some(child.outer_html());
            break;
        }

        if words_before >= 8 && words <= 20 && boilerplate_regex().is_match(&text) {
            truncate_from = Some(child.outer_html());
            break;
        }

        words_before += words;
    }

    if truncate_from.is_none() {
        let heading_selector = format!(
            "{} h2, {} h3, {} h4, {} h5, {} h6",
            root_selector, root_selector, root_selector, root_selector, root_selector
        );
        for heading in doc.select(&heading_selector).unwrap_or_default() {
            let text = normalize_text(&heading.text());
            if !trailing_heading_regex().is_match(&text) {
                continue;
            }
            if let Some(pos) = full_text.find(&text)
                && count_words(&full_text[..pos]) >= 8
            {
                truncate_from = Some(heading.outer_html());
                break;
            }
        }
    }

    let candidate_selector = format!(
        "{} p, {} div, {} span, {} time",
        root_selector, root_selector, root_selector, root_selector
    );
    for element in doc.select(&candidate_selector).unwrap_or_default() {
        let text = normalize_text(&element.text());
        let words = count_words(&text);
        if words == 0 || words > 15 {
            continue;
        }
        if let Some(pos) = full_text.find(&text)
            && is_boundary_metadata(&text, words, pos, full_text.len())
        {
            snippets.push(element.outer_html());
        }
    }

    let mut result = remove_snippets(wrapped, snippets);

    if let Some(marker) = truncate_from
        && let Some(start_idx) = result.find(&marker)
        && let Some(end_idx) = result.rfind("</div>")
        && start_idx < end_idx
    {
        result.replace_range(start_idx..end_idx, "");
    }

    if let Ok(doc) = Document::parse(&result)
        && let Ok(elements) = doc.select(root_selector)
        && let Some(root) = elements.into_iter().next()
    {
        root.inner_html()
    } else {
        html.to_string()
    }
}

/// Remove empty nodes from HTML
///
/// A node is considered empty if it has no text content or only whitespace.
/// This iteratively removes empty nodes until none remain.
fn remove_empty_nodes(html: &str, max_passes: usize) -> String {
    let mut result = html.to_string();
    let tags = [
        "div", "p", "span", "section", "article", "aside", "nav", "header", "footer",
    ];
    let protected_figure_children = protected_figure_children(html);

    let mut passes = 0;
    loop {
        let mut modified = false;
        let prev_result = result.clone();

        for tag in tags {
            let empty_re = Regex::new(&format!(r#"(?is)<{}(?:\s[^>]*)?>\s*(?:<br\s*/?>\s*)*</{}>"#, tag, tag)).unwrap();
            let whitespace_re = Regex::new(&format!(r#"(?is)<{}(?:\s[^>]*)?>\s*</{}>"#, tag, tag)).unwrap();

            result = empty_re
                .replace_all(&result, |caps: &regex::Captures| {
                    let full = caps.get(0).map(|m| m.as_str()).unwrap_or("");
                    if protected_figure_children.contains(full) { full.to_string() } else { String::new() }
                })
                .to_string();
            result = whitespace_re
                .replace_all(&result, |caps: &regex::Captures| {
                    let full = caps.get(0).map(|m| m.as_str()).unwrap_or("");
                    if protected_figure_children.contains(full) { full.to_string() } else { String::new() }
                })
                .to_string();
        }

        if result != prev_result {
            modified = true;
        }

        if !modified {
            break;
        }
        passes += 1;
        if passes >= max_passes {
            break;
        }
    }

    result
}

/// Remove doc-site chrome blocks like sidebars, TOCs, and breadcrumbs.
fn remove_doc_chrome_nodes(html: &str) -> String {
    let pattern = Regex::new(
        r"(?i)(toc|table[-_ ]of[-_ ]contents|on[-_ ]this[-_ ]page|breadcrumbs?|breadcrumb|sidebar|sidenav|navigation|page[-_ ]nav|pagination|pager|edit[-_ ]on[-_ ]github|edit[-_ ]this[-_ ]page)",
    )
    .unwrap();

    let tags = ["nav", "aside", "div", "section", "ul", "ol"];
    let mut result = html.to_string();

    for tag in tags {
        let class_re = Regex::new(&format!(
            r#"<{}((?:\s[^>]*?)?\s+class=["']([^"']*)["'][^>]*)>(.*?)</{}>"#,
            tag, tag
        ))
        .unwrap();

        result = class_re
            .replace_all(&result, |caps: &regex::Captures| {
                let classes = caps.get(2).map(|m| m.as_str()).unwrap_or("");
                if classes.split_whitespace().any(|c| pattern.is_match(c)) {
                    String::new()
                } else {
                    caps.get(0).map(|m| m.as_str()).unwrap_or("").to_string()
                }
            })
            .to_string();

        let id_re = Regex::new(&format!(
            r#"<{}((?:\s[^>]*?)?\s+id=["']([^"']*)["'][^>]*)>(.*?)</{}>"#,
            tag, tag
        ))
        .unwrap();

        result = id_re
            .replace_all(&result, |caps: &regex::Captures| {
                let id = caps.get(2).map(|m| m.as_str()).unwrap_or("");
                if pattern.is_match(id) {
                    String::new()
                } else {
                    caps.get(0).map(|m| m.as_str()).unwrap_or("").to_string()
                }
            })
            .to_string();
    }

    result
}

/// Remove doc-site utility blocks by text content.
fn remove_doc_chrome_text_blocks(html: &str) -> String {
    let text_pattern = Regex::new(r"(?i)(edit on github|ask about this page|copy for llm|view as markdown)").unwrap();
    let tags = ["div", "p", "span", "a", "li"];
    let mut result = html.to_string();

    for tag in tags {
        let element_re = Regex::new(&format!(r#"<{}[^>]*>(.*?)</{}>"#, tag, tag)).unwrap();
        result = element_re
            .replace_all(&result, |caps: &regex::Captures| {
                let inner_html = caps.get(1).map(|m| m.as_str()).unwrap_or("");
                let text = strip_tags(inner_html);
                let trimmed = text.trim();
                let word_count = trimmed.split_whitespace().count();
                if trimmed.len() <= 200 && word_count <= 10 && text_pattern.is_match(trimmed) {
                    String::new()
                } else {
                    caps.get(0).map(|m| m.as_str()).unwrap_or("").to_string()
                }
            })
            .to_string();
    }

    let literal_re = Regex::new(r"(?i)view as markdown").unwrap();
    literal_re.replace_all(&result, "").to_string()
}

/// Remove nodes with high link density
///
/// Link density is the ratio of link text to total text.
/// Nodes above the threshold are removed as they're likely navigation/menus.
fn remove_high_link_density_nodes(html: &str, max_density: f64) -> String {
    let density_threshold = max_density;
    let mut result = html.to_string();

    let tags = ["div", "p", "section", "article", "aside", "nav", "li"];
    let protected_figure_children = protected_figure_children(html);

    for tag in tags {
        let re = Regex::new(&format!(r#"(?is)<{}(?:\s[^>]*)?>(.*?)</{}\s*>"#, tag, tag)).unwrap();

        result = re
            .replace_all(&result, |caps: &regex::Captures| {
                let full = caps.get(0).map(|m| m.as_str()).unwrap_or("");
                if protected_figure_children.contains(full) {
                    return full.to_string();
                }

                let inner_html = caps.get(1).map(|m| m.as_str()).unwrap_or("");
                let text_content = strip_tags(inner_html);
                let text_length = text_content.chars().count();

                if text_length == 0 {
                    return full.to_string();
                }

                let link_text_length = extract_link_text_length(inner_html);
                let link_density = link_text_length / text_length as f64;

                if link_density > density_threshold { String::new() } else { full.to_string() }
            })
            .to_string();
    }

    result
}

/// Clean up nested DIVs with single children
///
/// If a DIV contains only another DIV as its direct child,
/// unwrap the outer DIV to reduce nesting.
fn clean_nested_divs(html: &str) -> String {
    let mut result = html.to_string();
    let nested_div_re = Regex::new(r#"<div\s[^>]*>\s*<div\s[^>]*>(.*?)</div\s*>\s*</div\s*>"#).unwrap();

    let mut max_iterations = 10;
    let mut modified = true;

    while modified && max_iterations > 0 {
        let prev_result = result.clone();
        result = nested_div_re.replace_all(&result, r#"<div>$1</div>"#).to_string();
        modified = result != prev_result;
        max_iterations -= 1;
    }

    result
}

/// Remove elements matching strip patterns (class/ID regex)
///
/// Removes elements whose class or id attributes match the given regex pattern,
/// preserving the inner content.
fn strip_patterns(html: &str, patterns: &str) -> String {
    let re = match Regex::new(patterns) {
        Ok(regex) => regex,
        Err(_) => return html.to_string(),
    };

    let mut result = html.to_string();
    let tags = [
        "div", "p", "span", "section", "article", "aside", "nav", "header", "footer",
    ];

    for tag in tags {
        let element_re = Regex::new(&format!(
            r#"<{}((?:\s[^>]*?)?\s+class=["']([^"']*)["'][^>]*)>(.*?)</{}>"#,
            tag, tag
        ))
        .unwrap();

        result = element_re
            .replace_all(&result, |caps: &regex::Captures| {
                let classes = caps.get(2).map(|m| m.as_str()).unwrap_or("");
                let content = caps.get(3).map(|m| m.as_str()).unwrap_or("");

                let should_remove = classes.split_whitespace().any(|c| re.is_match(c));

                if should_remove {
                    content.to_string()
                } else {
                    caps.get(0).map(|m| m.as_str()).unwrap_or("").to_string()
                }
            })
            .to_string();

        let id_re = Regex::new(&format!(
            r#"<{}((?:\s[^>]*?)?\s+id=["']([^"']*)["'][^>]*)>(.*?)</{}>"#,
            tag, tag
        ))
        .unwrap();

        result = id_re
            .replace_all(&result, |caps: &regex::Captures| {
                let id = caps.get(2).map(|m| m.as_str()).unwrap_or("");
                let content = caps.get(3).map(|m| m.as_str()).unwrap_or("");

                if re.is_match(id) {
                    content.to_string()
                } else {
                    caps.get(0).map(|m| m.as_str()).unwrap_or("").to_string()
                }
            })
            .to_string();
    }

    result
}

/// Fix remaining relative URLs to absolute URLs
fn fix_relative_urls(html: &str, base_url: &Url) -> String {
    let mut output = String::new();
    let mut rewriter = lol_html::HtmlRewriter::new(
        lol_html::Settings {
            element_content_handlers: vec![
                lol_html::element!("a", |el| {
                    if let Some(href) = el.get_attribute("href")
                        && let Ok(absolute) = base_url.join(&href)
                    {
                        el.set_attribute("href", absolute.as_str()).ok();
                    }
                    Ok(())
                }),
                lol_html::element!("img", |el| {
                    if let Some(src) = el.get_attribute("src")
                        && let Ok(absolute) = base_url.join(&src)
                    {
                        el.set_attribute("src", absolute.as_str()).ok();
                    }
                    Ok(())
                }),
            ],
            ..Default::default()
        },
        |c: &[u8]| {
            output.push_str(&String::from_utf8_lossy(c));
        },
    );

    match rewriter.write(html.as_bytes()) {
        Ok(_) => {}
        Err(_) => return html.to_string(),
    }

    match rewriter.end() {
        Ok(_) => {}
        Err(_) => return html.to_string(),
    }

    if output.is_empty() { html.to_string() } else { output }
}

/// Strip HTML tags from a string, keeping only text content
fn strip_tags(html: &str) -> String {
    let re = Regex::new(r"<[^>]*>").unwrap();
    re.replace_all(html, "").to_string()
}

/// Extract the total length of link text from HTML
fn extract_link_text_length(html: &str) -> f64 {
    let link_re = Regex::new(r#"(?is)<a\b[^>]*>(.*?)</a>"#).unwrap();
    let href_re = Regex::new(r#"(?is)\bhref=["']([^"']*)["']"#).unwrap();
    link_re
        .captures_iter(html)
        .map(|cap| {
            let full = cap.get(0).map(|m| m.as_str()).unwrap_or("");
            let href = href_re
                .captures(full)
                .and_then(|href_cap| href_cap.get(1).map(|m| m.as_str()));
            let text = cap.get(1).map(|m| m.as_str()).unwrap_or("");
            strip_tags(text).chars().count() as f64 * hash_only_link_coefficient(href)
        })
        .sum()
}

fn protected_figure_children(html: &str) -> HashSet<String> {
    let Ok(doc) = Document::parse(html) else {
        return HashSet::new();
    };

    doc.select("figure *")
        .unwrap_or_default()
        .into_iter()
        .map(|element| element.outer_html())
        .collect()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_remove_conditional_comments() {
        let html = r#"
            <html>
                <body>
                    <!--[if IE]>
                    <div>IE specific content</div>
                    <![endif]-->
                    <div>Normal content</div>
                    <!--[if !IE]>
                    <div>Non-IE content</div>
                    <![endif]-->
                </body>
            </html>
        "#;

        let result = remove_conditional_comments(html);
        assert!(!result.contains("[if IE]"));
        assert!(!result.contains("[endif]"));
        assert!(result.contains("Normal content"));
    }

    #[test]
    fn test_strip_images() {
        let html = r#"
            <html>
                <body>
                    <p>Text before</p>
                    <img src="test.jpg" alt="Test">
                    <p>Text after</p>
                </body>
            </html>
        "#;

        let result = strip_images(html);
        assert!(!result.contains("<img"));
        assert!(result.contains("Text before"));
        assert!(result.contains("Text after"));
    }

    #[test]
    fn test_remove_empty_nodes() {
        let html = r#"
            <html>
                <body>
                    <div></div>
                    <p>Content</p>
                    <span>   </span>
                    <section>Text</section>
                </body>
            </html>
        "#;

        let result = remove_empty_nodes(html, 10);
        assert!(result.contains("Content"));
        assert!(result.contains("Text"));
    }

    #[test]
    fn test_remove_high_link_density_nodes() {
        let html = r##"
            <html>
                <body>
                    <div class="nav">
                        <a href="#">Link 1</a>
                        <a href="#">Link 2</a>
                        <a href="#">Link 3</a>
                    </div>
                    <div class="content">
                        <p>This is substantial text content with links.
                        <a href="#">Small link</a>
                        More text here to ensure low link density.</p>
                    </div>
                </body>
            </html>
        "##;

        let result = remove_high_link_density_nodes(html, 0.5);
        assert!(result.contains("content"));
    }

    #[test]
    fn test_clean_nested_divs() {
        let html = r#"<div class="outer"><div class="inner">Content</div></div>"#;
        let result = clean_nested_divs(html);
        assert!(result.contains("Content"));
        assert!(result.contains("<div>"));
    }

    #[test]
    fn test_dedupe_title_headings_removes_nested_h1() {
        let html =
            r#"<article><h1>Standings 2026</h1><p>Body copy with enough detail to remain after cleanup.</p></article>"#;
        let result = dedupe_title_headings(html, Some("Standings 2026"));

        assert!(!result.contains("<h1>Standings 2026</h1>"));
        assert!(result.contains("Body copy with enough detail"));
    }

    #[test]
    fn test_dedupe_title_headings_removes_nested_h2() {
        let html = r#"<article><h2>The Last Interview</h2><p>Body copy with enough detail to remain after cleanup.</p></article>"#;
        let result = dedupe_title_headings(html, Some("The Last Interview"));

        assert!(!result.contains("<h2>The Last Interview</h2>"));
        assert!(result.contains("Body copy with enough detail"));
    }

    #[test]
    fn test_strip_patterns() {
        let html = r#"
            <html>
                <body>
                    <div class="advertisement">Ad content</div>
                    <div class="sidebar">Sidebar</div>
                    <div class="main">Main content</div>
                </body>
            </html>
        "#;

        let result = strip_patterns(html, r"(?i)(ad|advertisement)");
        assert!(!result.contains("advertisement"));
        assert!(result.contains("Sidebar"));
        assert!(result.contains("Main content"));
    }

    #[test]
    fn test_fix_relative_urls() {
        let base = Url::parse("https://example.com/blog/").unwrap();
        let html = r#"
            <html>
                <body>
                    <a href="/about">About</a>
                    <img src="image.jpg" />
                </body>
            </html>
        "#;

        let result = fix_relative_urls(html, &base);
        assert!(result.contains("href=\"https://example.com/about\""));
        assert!(result.contains("src=\"https://example.com/blog/image.jpg\""));
    }

    #[test]
    fn test_postprocess_full_pipeline() {
        let html = r#"
            <html>
                <body>
                    <!--[if IE]>
                    <div>IE content</div>
                    <![endif]-->
                    <div></div>
                    <div class="main">Content</div>
                </body>
            </html>
        "#;

        let config = PostProcessConfig::default();
        let result = postprocess_html(html, &config);

        assert!(!result.contains("[if IE]"));
        assert!(result.contains("Content"));
    }

    #[test]
    fn test_strip_tags() {
        let html = r#"<p>This is <strong>bold</strong> text</p>"#;
        let result = strip_tags(html);
        assert_eq!(result, "This is bold text");
    }

    #[test]
    fn test_extract_link_text_length() {
        let html = r##"<a href="#">Link text</a> and <a href="#">Another</a>"##;
        let length = extract_link_text_length(html);
        assert!((length - 4.8).abs() < f64::EPSILON);
    }

    #[test]
    fn test_remove_high_link_density_nodes_preserves_figure_children() {
        let html = r#"
            <figure>
                <p><a href="https://example.com/credit">Illustration credit</a></p>
            </figure>
        "#;

        let result = remove_high_link_density_nodes(html, 0.1);
        assert!(result.contains("Illustration credit"));
    }

    #[test]
    fn test_remove_empty_nodes_preserves_figure_children() {
        let html = r#"
            <figure>
                <div> </div>
            </figure>
        "#;

        let result = remove_empty_nodes(html, 10);
        assert!(result.contains("<div> </div>"));
    }

    #[test]
    fn test_postprocess_config_default() {
        let config = PostProcessConfig::default();
        assert!(config.remove_empty_nodes);
        assert!(config.remove_high_link_density);
        assert_eq!(config.max_link_density, 0.5);
        assert!(config.clean_nested_divs);
        assert!(config.remove_conditional_comments);
        assert!(!config.strip_images);
        assert!(!config.keep_classes);
        assert!(config.remove_content_patterns);
        assert!(config.strip_patterns.is_none());
    }

    #[test]
    fn test_strip_classes() {
        let html = r#"<div class="container main">Content</div><p class="text">Text</p>"#;
        let result = strip_classes(html);
        assert!(!result.contains("class="));
        assert!(result.contains("<div>Content</div>"));
        assert!(result.contains("<p>Text</p>"));
    }

    #[test]
    fn test_keep_classes_true() {
        let html = r#"<div class="container">Content</div>"#;
        let config = PostProcessConfig { keep_classes: true, ..Default::default() };
        let result = postprocess_html(html, &config);
        assert!(result.contains("class="));
    }

    #[test]
    fn test_keep_classes_false() {
        let html = r#"<div class="container">Content</div>"#;
        let config = PostProcessConfig { keep_classes: false, ..Default::default() };
        let result = postprocess_html(html, &config);
        assert!(!result.contains("class="));
    }

    #[test]
    fn test_strip_images_with_attributes() {
        let html = r#"<img src="test.jpg" alt="Test" class="image" width="100" />"#;
        let result = strip_images(html);
        assert!(!result.contains("<img"));
        assert!(!result.contains("src="));
        assert!(!result.contains("alt="));
    }

    #[test]
    fn test_remove_empty_nodes_nested() {
        let html = r#"<div><p></p><span>Content</span></div>"#;
        let result = remove_empty_nodes(html, 10);
        assert!(!result.contains("<p></p>"));
        assert!(result.contains("Content"));
    }

    #[test]
    fn test_remove_content_patterns_truncates_wikipedia_sections() {
        let html = r#"
            <h1>Rust</h1>
            <p>Rust is a systems programming language.</p>
            <p>It emphasizes safety and performance.</p>
            <h2>See also</h2>
            <ul><li><a href="/wiki/Go_(programming_language)">Go</a></li></ul>
            <h2>References</h2>
            <ol><li>Reference</li></ol>
        "#;

        let result = remove_content_patterns(html);
        assert!(result.contains("systems programming language"));
        assert!(!result.contains("See also"));
        assert!(!result.contains("References"));
    }

    #[test]
    fn test_remove_content_patterns_removes_leading_metadata() {
        let html = r#"
            <p>From Wikipedia, the free encyclopedia</p>
            <p>By Jane Doe</p>
            <p>March 4, 2026 · 3 min read</p>
            <p>Rust is a systems programming language focused on safety.</p>
            <p>It prevents memory bugs without a garbage collector.</p>
        "#;

        let result = remove_content_patterns(html);
        assert!(result.contains("systems programming language focused on safety"));
        assert!(!result.contains("From Wikipedia"));
        assert!(!result.contains("By Jane Doe"));
        assert!(!result.contains("3 min read"));
    }

    #[test]
    fn test_postprocess_standardizes_code_blocks() {
        let html = r#"<pre class="lang-rust">fn main() { println!("hi"); }</pre>"#;
        let result = postprocess_html(html, &PostProcessConfig::default());
        assert!(result.contains(r#"<code class="language-rust">"#));
    }

    #[test]
    fn test_postprocess_standardizes_github_callouts() {
        let html = r#"
            <div class="markdown-alert markdown-alert-note">
                <p class="markdown-alert-title">Note</p>
                <p>Important context.</p>
            </div>
        "#;
        let result = postprocess_html(html, &PostProcessConfig::default());
        assert!(result.contains(r#"data-callout="note""#));
        assert!(result.contains("Important context."));
        assert!(!result.contains("markdown-alert-title"));
    }

    #[test]
    fn test_postprocess_normalizes_footnotes() {
        let html = r##"
            <p>Reference<a href="#cite_note-1">1</a></p>
            <ol class="references">
                <li id="cite_note-1">First source</li>
            </ol>
        "##;
        let result = postprocess_html(html, &PostProcessConfig::default());
        assert!(result.contains(r#"id="fnref:1""#));
        assert!(result.contains(r#"<section id="footnotes"><ol>"#));
        assert!(result.contains(r#"id="fn:1""#));
    }

    #[test]
    fn test_postprocess_normalizes_math_scripts() {
        let html = r#"<p><script type="math/tex">x^2 + y^2</script></p>"#;
        let result = postprocess_html(html, &PostProcessConfig::default());
        assert!(result.contains("<math"));
        assert!(result.contains("data-latex=\"x^2 + y^2\""));
    }
}
