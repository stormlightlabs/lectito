use kuchiki::NodeRef;
use kuchiki::traits::TendrilSink;
use scraper::Html;
use serde_json::Value;
use url::Url;

use super::config::{ExtractFlags, ReadabilityOptions};
use super::error::Result;
use super::extract::{ExtractAttempt, element_count, prep_document, serialize_roots};
use super::metadata::{Metadata, clean_metadata_value, decode_html_entities, normalize_byline};
use super::regexes::RegexPattern;
use super::{dom, patterns, shared};

pub fn extract_json_ld(html: &str) -> Metadata {
    let document = Html::parse_document(html);
    let script_selector = patterns::selector(r#"script[type="application/ld+json"]"#);

    for script in document.select(&script_selector) {
        let content = script.text().collect::<String>();
        let content = content.trim().trim_start_matches("<![CDATA[").trim_end_matches("]]>");
        let Ok(value) = serde_json::from_str::<Value>(content) else {
            continue;
        };
        if let Some(article) = find_json_ld_article(&value) {
            return metadata_from_json_ld(article);
        }
    }

    Metadata::default()
}

pub fn apply_schema_fallback(
    html: &str, attempt: ExtractAttempt, metadata: &Metadata, opts: &ReadabilityOptions, flags: ExtractFlags,
    base_url: Option<&Url>,
) -> Result<ExtractAttempt> {
    let Some(schema_text) = metadata.schema_text.as_deref() else {
        return Ok(attempt);
    };
    let normalized_schema = normalized_match_text(schema_text);
    if normalized_schema.chars().count() < 80 {
        return Ok(attempt);
    }

    let extracted_len = normalized_match_text(&attempt.text_content).chars().count();
    let schema_len = normalized_schema.chars().count();
    let document = kuchiki::parse_html().one(html);
    prep_document(&document, opts, flags);
    if let Some(root) = smallest_schema_match(&document, &normalized_schema) {
        let matched_len = normalized_match_text(&dom::inner_text(&root)).chars().count();
        let (fallback, _) = serialize_roots(vec![root], opts, flags, base_url, metadata)?;
        if fallback.text_len > attempt.text_len
            || (matched_len >= schema_len.saturating_mul(4) / 5 && extracted_len > matched_len.saturating_mul(6) / 5)
        {
            return Ok(fallback);
        }
        return Ok(attempt);
    }

    if schema_len <= extracted_len.saturating_add(40) && schema_len <= extracted_len.saturating_mul(6) / 5 {
        return Ok(attempt);
    }

    let escaped = shared::escape_html(schema_text);
    let document = kuchiki::parse_html().one(format!("<html><body><article><p>{escaped}</p></article></body></html>"));
    let Some(root) = dom::select_nodes(&document, "article").into_iter().next() else {
        return Ok(attempt);
    };
    let (fallback, _) = serialize_roots(vec![root], opts, flags, base_url, metadata)?;
    if fallback.text_len > attempt.text_len { Ok(fallback) } else { Ok(attempt) }
}

fn find_json_ld_article(value: &Value) -> Option<&Value> {
    match value {
        Value::Array(items) => items.iter().find_map(find_json_ld_article),
        Value::Object(map) => {
            if let Some(graph) = map.get("@graph").and_then(Value::as_array)
                && let Some(article) = graph.iter().find_map(find_json_ld_article)
            {
                return Some(article);
            }

            if map.get("@type").is_some_and(json_ld_type_is_article) { Some(value) } else { None }
        }
        _ => None,
    }
}

fn json_ld_type_is_article(value: &Value) -> bool {
    match value {
        Value::String(kind) => RegexPattern::JsonLdArticleType
            .to_regex()
            .is_match(kind.trim_start_matches("https://schema.org/")),
        Value::Array(kinds) => kinds.iter().any(json_ld_type_is_article),
        _ => false,
    }
}

fn metadata_from_json_ld(value: &Value) -> Metadata {
    Metadata {
        title: string_field(value, "name").or_else(|| string_field(value, "headline")),
        byline: byline_from_json_ld(value.get("author")),
        excerpt: string_field(value, "description"),
        site_name: value
            .get("publisher")
            .and_then(|publisher| string_field(publisher, "name"))
            .or_else(|| {
                value
                    .get("isPartOf")
                    .and_then(|is_part_of| string_field(is_part_of, "name"))
            }),
        published_time: string_field(value, "datePublished"),
        image: image_from_json_ld(value.get("image")),
        domain: None,
        favicon: None,
        schema_text: string_field(value, "articleBody").or_else(|| string_field(value, "text")),
        lang: None,
        dir: None,
    }
}

fn byline_from_json_ld(value: Option<&Value>) -> Option<String> {
    let raw = match value? {
        Value::String(author) => Some(author.trim().to_string()),
        Value::Object(_) => string_field(value?, "name"),
        Value::Array(authors) => {
            let names: Vec<_> = authors
                .iter()
                .filter_map(|author| string_field(author, "name"))
                .collect();
            (!names.is_empty()).then(|| names.join(", "))
        }
        _ => None,
    }?;
    normalize_byline(&raw)
}

fn image_from_json_ld(value: Option<&Value>) -> Option<String> {
    match value? {
        Value::String(image) => clean_metadata_value(image),
        Value::Object(_) => string_field(value?, "url").or_else(|| string_field(value?, "@id")),
        Value::Array(images) => images.iter().find_map(|image| image_from_json_ld(Some(image))),
        _ => None,
    }
}

fn string_field(value: &Value, field: &str) -> Option<String> {
    value
        .get(field)?
        .as_str()
        .map(str::trim)
        .filter(|value| !value.is_empty())
        .map(decode_html_entities)
}

fn smallest_schema_match(document: &NodeRef, normalized_schema: &str) -> Option<NodeRef> {
    let mut matches = Vec::new();
    for node in dom::select_nodes(document, "article, main, section, div, p") {
        let text = normalized_match_text(&dom::inner_text(&node));
        let text_len = text.chars().count();
        if text_len < 80 {
            continue;
        }
        if text.contains(normalized_schema) || normalized_schema.contains(&text) {
            matches.push((text_len, element_count(&node), node));
        }
    }
    matches.sort_by_key(|(text_len, element_count, _)| (*text_len, *element_count));
    matches.into_iter().map(|(_, _, node)| node).next()
}

fn normalized_match_text(value: &str) -> String {
    patterns::normalize_spaces(value.trim()).to_lowercase()
}

#[cfg(test)]
mod tests {
    use crate::config::ReadabilityOptions;
    use crate::extract::extract;

    #[test]
    fn schema_text_fallback_uses_smallest_matching_subtree() {
        let schema_text = "This is the target post content with enough words to trigger schema text fallback. It includes several sentences so it is clearly better than the short article summary. More detail appears here to make the target post the right extraction root.";
        let html = format!(
            r#"
            <html><head>
                <title>Feed Page</title>
                <script type="application/ld+json">{{"@type":"SocialMediaPosting","text":"{schema_text}"}}</script>
            </head><body>
                <article><p>Short article summary.</p></article>
                <div id="feed">
                    <div class="post"><p>First post in the feed with different content entirely.</p></div>
                    <div class="post" id="target"><p>{schema_text}</p></div>
                    <div class="post"><p>Third post with different content.</p></div>
                </div>
            </body></html>
            "#
        );
        let article = extract(
            &html,
            None,
            &ReadabilityOptions { char_threshold: 0, ..Default::default() },
        )
        .unwrap()
        .unwrap();

        assert!(article.text_content.contains("target post content"));
        assert!(!article.text_content.contains("First post in the feed"));
        assert!(!article.text_content.contains("Third post"));
    }

    #[test]
    fn schema_text_fallback_escapes_raw_schema_text_without_dom_match() {
        let schema_text = "Safe schema text with enough words to trigger fallback. <script>alert('xss')</script> More text here pads the word count above what the tiny body extracts from the page.";
        let html = format!(
            r#"
            <html><head>
                <title>No Match</title>
                <script type="application/ld+json">{{"@type":"SocialMediaPosting","text":"{}"}}</script>
            </head><body><div><p>Tiny.</p></div></body></html>
            "#,
            schema_text.replace("</script>", "<\\/script>")
        );
        let article = extract(
            &html,
            None,
            &ReadabilityOptions { char_threshold: 0, ..Default::default() },
        )
        .unwrap()
        .unwrap();

        assert!(article.content.contains("Safe schema text"));
        assert!(!article.content.contains("<script>"));
        assert!(!article.content.contains("</script>"));
    }
}
