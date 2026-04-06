use lectito_core::{Document, parse};
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
