use lectito_core::{Document, link_density, parse};
use std::fs;
use std::path::PathBuf;

fn fixture_root() -> PathBuf {
    PathBuf::from("../../tests/fixtures")
}

fn read_fixture(path: &str) -> String {
    fs::read_to_string(fixture_root().join(path)).unwrap()
}

#[test]
fn layout_table_fixture_unwraps_into_normal_flow() {
    let html = read_fixture("extract/tables/layout-fallback.html");
    let article = parse(&html).expect("fixture should parse");

    assert_eq!(article.metadata.title.as_deref(), Some("Quarterly report"));
    assert!(article.content.contains("steady subscription growth"));
    assert!(!article.content.contains("<table"));
}

#[test]
fn data_table_fixture_preserves_structured_table_markup() {
    let html = read_fixture("extract/tables/preserve-data-table.html");
    let article = parse(&html).expect("fixture should parse");

    assert_eq!(article.metadata.title.as_deref(), Some("Standings 2026"));
    assert!(article.content.contains("<table"));
    assert!(article.content.contains("<caption>American League standings</caption>"));
    assert!(article.content.contains("<thead>"));
    assert!(!article.content.contains("<h1>Standings 2026</h1>"));
}

#[test]
fn title_similarity_fixture_cleans_json_ld_title_and_strips_duplicate_heading() {
    let html = read_fixture("metadata/title-similarity.html");
    let article = parse(&html).expect("fixture should parse");
    let doc = Document::parse(&html).expect("fixture document should parse");

    assert_eq!(doc.extract_title().as_deref(), Some("The Last Interview"));
    assert_eq!(article.metadata.title.as_deref(), Some("The Last Interview"));
    assert!(!article.content.contains("<h2>The Last Interview</h2>"));
    assert!(
        article
            .content
            .contains("The interview opens with a practical question")
    );
}

#[test]
fn clustered_sibling_fixture_aggregates_sections_under_shared_ancestor() {
    let html = read_fixture("extract/aggregation/propublica-investigation.html");
    let article = parse(&html).expect("fixture should parse");

    assert!(article.content.contains("Section one opens the investigation"));
    assert!(article.content.contains("Section two follows the same investigation"));
    assert!(article.content.contains("Section three closes the investigation"));
}

#[test]
fn hash_link_fixture_deweights_fragment_navigation_density() {
    let html = read_fixture("extract/aggregation/docsrs-tokio.html");
    let doc = Document::parse(&html).expect("fixture document should parse");
    let content = doc
        .select(".docs-body")
        .expect("selector should be valid")
        .into_iter()
        .next()
        .expect("fixture should contain docs body");

    assert!(link_density(&content) < 0.5);

    let article = parse(&html).expect("fixture should parse");
    assert!(article.content.contains("Tokio gives you the building blocks"));
}

#[test]
fn figure_heavy_fixture_keeps_figure_content() {
    let html = read_fixture("extract/aggregation/aeon-figure-heavy.html");
    let article = parse(&html).expect("fixture should parse");

    assert!(
        article
            .content
            .contains("A diagram caption that should survive cleanup")
    );
    assert!(article.content.contains("The essay continues after the figure"));
}

#[test]
fn sibling_refinement_fixture_keeps_matching_lrb_sections() {
    let html = read_fixture("extract/aggregation/lrb-paper.html");
    let article = parse(&html).expect("fixture should parse");

    assert!(article.content.contains("Opening section from the review essay"));
    assert!(article.content.contains("Follow-up section from the review essay"));
}
