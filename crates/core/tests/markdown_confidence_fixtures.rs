use lectito_core::{Document, convert_to_markdown, parse};
use std::fs;
use std::path::PathBuf;

fn fixture_root() -> PathBuf {
    PathBuf::from("../../tests/fixtures")
}

fn read_fixture(path: &str) -> String {
    fs::read_to_string(fixture_root().join(path)).unwrap()
}

fn normalize_markdown(value: &str) -> String {
    value
        .replace("\\_", "_")
        .replace("\\[", "[")
        .replace("\\]", "]")
        .split_whitespace()
        .collect::<Vec<_>>()
        .join(" ")
}

fn render_fixture_markdown(path: &str) -> String {
    let html = read_fixture(path);
    let document = Document::parse(&html).expect("fixture document should parse");
    let metadata = document.extract_metadata();

    convert_to_markdown(&html, &metadata, &Default::default()).expect("markdown conversion should succeed")
}

fn extract_fixture_markdown(path: &str) -> String {
    parse(&read_fixture(path))
        .expect("fixture should parse")
        .to_markdown()
        .expect("article markdown conversion should succeed")
}

#[test]
fn markdown_fixture_selects_largest_srcset_image() {
    let markdown = extract_fixture_markdown("extract/markdown/jake-attributes-vs-properties.html");

    assert!(
        markdown.contains("/c/me-cat-C7BOo3uZ.avif"),
        "expected markdown to prefer the <picture> srcset candidate, got:\n{markdown}"
    );
    assert!(
        !markdown.contains("/c/me-cat-b1G8G4y3.jpg"),
        "expected markdown to avoid the fallback <img> source once srcset is resolved, got:\n{markdown}"
    );
}

#[test]
fn markdown_fixture_restructures_link_wrapped_cards() {
    let markdown = render_fixture_markdown("extract/markdown/increment-software-architecture.html");
    let normalized = normalize_markdown(&markdown);
    let has_linked_heading = normalized.contains(
        "### [Software architecture at scale](/software-architecture/architecture-at-scale/)",
    ) || normalized.contains("### [What happens when the pager goes off?](/on-call/when-the-pager-goes-off/)");
    let has_linked_excerpt = normalized.contains(
        "[Leaders at Foursquare, Hulu, and Twitter discuss early architecture decisions, downstream effects, and architectural philosophies.](/software-architecture/architecture-at-scale/)",
    ) || normalized.contains(
        "[To discover the state of incident response across the tech industry, we surveyed over thirty industry leaders",
    );

    assert!(
        has_linked_heading,
        "expected the linked card heading to remain a markdown link, got:\n{markdown}"
    );
    assert!(
        has_linked_excerpt,
        "expected the linked card body text to be preserved, got:\n{markdown}"
    );
}

#[test]
fn markdown_fixture_renders_simple_tables_as_pipe_tables() {
    let markdown = extract_fixture_markdown("extract/markdown/cppreference-operator-comparison.html");

    assert!(
        markdown.contains("| The comparison is deprecated if both lhs and rhs have array type prior to the application of these conversions. | (since C++20) (until C++26) |"),
        "expected at least one simple table to be emitted as a markdown pipe table, got:\n{markdown}"
    );
}

#[test]
fn markdown_fixture_preserves_complex_tables_as_html() {
    let markdown = extract_fixture_markdown("extract/markdown/cppreference-operator-comparison.html");

    assert!(markdown.contains("<table"));
    assert!(markdown.contains("colspan=\"5\""));
    assert!(markdown.contains("The comparison is deprecated if both"));
}

#[test]
fn confidence_fixture_distinguishes_high_medium_and_low_cases() {
    let high = parse(&read_fixture("extract/confidence/matklad-basic-things.html")).expect("high fixture should parse");
    let second_high = parse(&read_fixture("extract/confidence/norvig-21-days.html")).expect("second high fixture should parse");
    let medium = parse(&read_fixture("extract/aggregation/docsrs-tokio.html")).expect("medium fixture should parse");
    let low = parse(&read_fixture("extract/confidence/duckduckgo-home.html")).expect("low fixture should parse");

    assert!(
        high.confidence > 0.6,
        "expected high confidence, got {}",
        high.confidence
    );
    assert!(
        second_high.confidence > 0.6,
        "expected second high confidence, got {}",
        second_high.confidence
    );
    assert!(
        medium.confidence > 0.5 && medium.confidence < high.confidence,
        "expected medium confidence, got {}",
        medium.confidence
    );
    assert!(low.confidence < 0.45, "expected low confidence, got {}", low.confidence);
    assert!(high.confidence > medium.confidence);
    assert!(second_high.confidence > medium.confidence);
    assert!(medium.confidence > low.confidence);
    assert!(high.diagnostics.is_some());
    assert!(second_high.diagnostics.is_some());
    assert!(medium.diagnostics.is_some());
    assert!(low.diagnostics.is_some());

    let high_diagnostics = high.diagnostics.unwrap();
    let second_high_diagnostics = second_high.diagnostics.unwrap();
    let medium_diagnostics = medium.diagnostics.unwrap();
    let low_diagnostics = low.diagnostics.unwrap();

    assert!(high_diagnostics.selected_pass.is_some());
    assert!(second_high_diagnostics.selected_pass.is_some());
    assert!(low_diagnostics.selected_pass.is_some());
    assert!(!high_diagnostics.pass_history.is_empty());
    assert!(!second_high_diagnostics.pass_history.is_empty());
    assert!(!medium_diagnostics.candidate_scores.is_empty());
    assert!(medium_diagnostics.content_word_ratio < high_diagnostics.content_word_ratio);
}
