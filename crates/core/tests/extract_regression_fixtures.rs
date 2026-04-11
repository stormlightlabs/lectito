use lectito_core::{Document, link_density, parse};
use serde::Deserialize;
use std::fs;
use std::path::PathBuf;

fn fixture_root() -> PathBuf {
    PathBuf::from("../../tests/fixtures")
}

fn read_fixture(path: &str) -> String {
    fs::read_to_string(fixture_root().join(path)).unwrap()
}

fn normalize_ws(value: &str) -> String {
    value.split_whitespace().collect::<Vec<_>>().join(" ")
}

#[derive(Debug, Deserialize)]
struct LiveFixtureManifest {
    url: String,
    fetched_at: String,
}

fn read_live_manifest(path: &str) -> LiveFixtureManifest {
    let manifest_path = format!("extract/live/{path}/manifest.json");
    let raw = read_fixture(&manifest_path);
    serde_json::from_str(&raw).unwrap()
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

#[test]
fn live_web_fixture_suite_keeps_body_and_strips_known_noise() {
    struct Case<'a> {
        fixture_dir: &'a str,
        expected_title: &'a str,
        must_contain: &'a [&'a str],
        must_not_contain: &'a [&'a str],
        min_words: usize,
    }

    let cases = [
        Case {
            fixture_dir: "paulgraham-makers-schedule",
            expected_title: "Maker's Schedule, Manager's Schedule",
            must_contain: &[
                "There are two types of schedule, which I'll call the manager's schedule and the maker's schedule.",
                "Those of us on the maker's schedule are willing to compromise.",
            ],
            must_not_contain: &["Related:"],
            min_words: 900,
        },
        Case {
            fixture_dir: "cloudflare-workers-ai",
            expected_title: "Workers AI: serverless GPU-powered inference on Cloudflare’s global network",
            must_contain: &[
                "Workers AI - making inference just work",
                "A road to global GPU coverage",
            ],
            must_not_contain: &["Birthday Week", "our open positions", "Visit 1.1.1.1"],
            min_words: 1700,
        },
        Case {
            fixture_dir: "rust-1-85",
            expected_title: "Announcing Rust 1.85.0 and Rust 2024",
            must_contain: &["Rust 2024 Edition is now stable", "Contributors to 1.85.0"],
            must_not_contain: &["Click here to be redirected."],
            min_words: 1200,
        },
        Case {
            fixture_dir: "martinfowler-microservices",
            expected_title: "Microservices",
            must_contain: &[
                "suite of small services",
                "you can only make decisions based on the imperfect information",
            ],
            must_not_contain: &["Significant Revisions"],
            min_words: 4000,
        },
    ];

    for case in cases {
        let manifest = read_live_manifest(case.fixture_dir);
        assert!(
            manifest.url.starts_with("https://"),
            "expected https URL for {}",
            case.fixture_dir
        );
        assert!(
            !manifest.fetched_at.trim().is_empty(),
            "expected fetch timestamp for {}",
            case.fixture_dir
        );

        let html = read_fixture(&format!("extract/live/{}/article.html", case.fixture_dir));
        let article = parse(&html).expect("fixture should parse");
        let text = normalize_ws(&article.to_text());

        assert_eq!(
            article.metadata.title.as_deref(),
            Some(case.expected_title),
            "{} title mismatch",
            case.fixture_dir
        );
        assert!(
            article.word_count >= case.min_words,
            "{} expected at least {} words, got {}",
            case.fixture_dir,
            case.min_words,
            article.word_count
        );

        for needle in case.must_contain {
            assert!(
                text.contains(&normalize_ws(needle)),
                "{} missing expected text: {}",
                case.fixture_dir,
                needle
            );
        }
        for needle in case.must_not_contain {
            assert!(
                !text.contains(&normalize_ws(needle)),
                "{} still contains noise marker: {}",
                case.fixture_dir,
                needle
            );
        }
    }
}
