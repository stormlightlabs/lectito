use std::collections::HashSet;

use scraper::{ElementRef, Html};

use super::patterns::{self, MAYBE_CANDIDATE, UNLIKELY_CANDIDATES};
use super::{config::ReadableOptions, error::Result};

pub fn is_probably_readable(html: &str, options: &ReadableOptions) -> Result<bool> {
    let document = Html::parse_document(html);

    let text_selector = patterns::selector("p, pre, article");
    let br_selector = patterns::selector("div > br");
    let list_paragraph_selector = patterns::selector("li p");

    let mut nodes = Vec::new();
    let mut seen = HashSet::new();
    let list_paragraphs: HashSet<_> = document
        .select(&list_paragraph_selector)
        .map(|node| node.id())
        .collect();

    for node in document.select(&text_selector) {
        if seen.insert(node.id()) {
            nodes.push(node);
        }
    }

    for br in document.select(&br_selector) {
        if let Some(parent) = br.parent().and_then(ElementRef::wrap) {
            if seen.insert(parent.id()) {
                nodes.push(parent);
            }
        }
    }

    let mut score = 0.0_f32;

    for node in nodes {
        if !is_node_visible(&node) {
            continue;
        }

        let match_string = format!(
            "{} {}",
            node.value().attr("class").unwrap_or_default(),
            node.value().attr("id").unwrap_or_default()
        );
        if UNLIKELY_CANDIDATES.is_match(&match_string) && !MAYBE_CANDIDATE.is_match(&match_string) {
            continue;
        }

        if list_paragraphs.contains(&node.id()) {
            continue;
        }

        let text_length = node.text().collect::<String>().trim().encode_utf16().count();
        if text_length < options.min_content_length {
            continue;
        }

        score += ((text_length - options.min_content_length) as f32).sqrt();
        if score > options.min_score {
            return Ok(true);
        }
    }

    Ok(false)
}

fn is_node_visible(node: &ElementRef<'_>) -> bool {
    let element = node.value();

    if element.has_class("fallback-image", scraper::CaseSensitivity::AsciiCaseInsensitive) {
        return element.attr("hidden").is_none() && !patterns::has_display_none(element.attr("style"));
    }

    !patterns::has_display_none(element.attr("style"))
        && element.attr("hidden").is_none()
        && element.attr("aria-hidden") != Some("true")
}

#[cfg(test)]
mod tests {
    use serde::Deserialize;

    use super::*;

    #[derive(Deserialize)]
    struct ExpectedMetadata {
        readerable: bool,
    }

    #[test]
    fn default_thresholds_match_upstream_shape() {
        let very_small_doc = r#"<html><p id="main">hello there</p></html>"#;
        let small_doc = format!(r#"<html><p id="main">{}</p></html>"#, "hello there ".repeat(11));
        let large_doc = format!(r#"<html><p id="main">{}</p></html>"#, "hello there ".repeat(12));
        let very_large_doc = format!(r#"<html><p id="main">{}</p></html>"#, "hello there ".repeat(50));

        let options = ReadableOptions::default();

        assert!(!is_probably_readable(very_small_doc, &options).unwrap());
        assert!(!is_probably_readable(&small_doc, &options).unwrap());
        assert!(!is_probably_readable(&large_doc, &options).unwrap());
        assert!(is_probably_readable(&very_large_doc, &options).unwrap());
    }

    #[test]
    fn options_control_content_length_and_score() {
        let small_doc = format!(r#"<html><p id="main">{}</p></html>"#, "hello there ".repeat(11));
        let large_doc = format!(r#"<html><p id="main">{}</p></html>"#, "hello there ".repeat(12));

        assert!(
            is_probably_readable(&small_doc, &ReadableOptions { min_content_length: 120, min_score: 0.0 },).unwrap()
        );
        assert!(
            !is_probably_readable(&large_doc, &ReadableOptions { min_content_length: 200, min_score: 0.0 },).unwrap()
        );
        assert!(
            is_probably_readable(&large_doc, &ReadableOptions { min_content_length: 0, min_score: 11.5 },).unwrap()
        );
    }

    #[test]
    fn skips_hidden_unlikely_and_list_paragraphs() {
        let options = ReadableOptions { min_content_length: 0, min_score: 0.0 };

        assert!(
            !is_probably_readable(r#"<html><p hidden>this paragraph is long enough</p></html>"#, &options).unwrap()
        );
        assert!(
            !is_probably_readable(
                r#"<html><p style="display: none">this paragraph is long enough</p></html>"#,
                &options,
            )
            .unwrap()
        );
        assert!(
            !is_probably_readable(
                r#"<html><p class="comment">this paragraph is long enough</p></html>"#,
                &options
            )
            .unwrap()
        );
        assert!(
            !is_probably_readable(
                r#"<html><li><p>this paragraph is long enough</p></li></html>"#,
                &options
            )
            .unwrap()
        );
    }

    #[test]
    fn matches_upstream_fixture_metadata() {
        let mut checked = 0;
        let mut mismatches = Vec::new();

        for fixture in lectito_fixtures::load_all().unwrap() {
            let metadata: ExpectedMetadata = serde_json::from_value(fixture.expected_metadata).unwrap();
            let actual = is_probably_readable(&fixture.source, &ReadableOptions::default()).unwrap();
            checked += 1;

            if actual != metadata.readerable {
                mismatches.push(format!(
                    "{}: expected {}, got {}",
                    fixture.name, metadata.readerable, actual
                ));
            }
        }

        assert!(checked > 0);
        assert!(
            mismatches.is_empty(),
            "readable fixture mismatches:\n{}",
            mismatches.join("\n")
        );
    }
}
