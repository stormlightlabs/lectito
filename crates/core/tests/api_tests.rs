//! Library API integration tests
use lectito_core::*;

fn get_fixture_path(name: &str) -> String {
    format!("../../tests/fixtures/{}", name)
}

fn get_site_fixture_path(site: &str, name: &str) -> String {
    format!("../../tests/fixtures/sites/{}/{}", site, name)
}

#[test]
fn test_parse_api() {
    let html = std::fs::read_to_string(get_site_fixture_path("wikipedia", "article.html")).unwrap();
    let article = parse(&html).expect("should parse");
    assert!(article.metadata.title.is_some());
    assert!(!article.content.is_empty());
}

#[test]
fn test_parse_with_url() {
    let html = std::fs::read_to_string(get_site_fixture_path("wikipedia", "article.html")).unwrap();
    let article =
        parse_with_url(&html, "https://en.wikipedia.org/wiki/Rust_(programming_language)").expect("should parse");

    assert!(article.source_url.is_some());
    assert_eq!(
        article.source_url.unwrap(),
        "https://en.wikipedia.org/wiki/Rust_(programming_language)"
    );
}

#[test]
fn test_is_probably_readable() {
    let article_html = std::fs::read_to_string(get_site_fixture_path("wikipedia", "article.html")).unwrap();
    let empty_html = std::fs::read_to_string(get_fixture_path("empty_content.html")).unwrap();
    assert!(is_probably_readable(&article_html));
    assert!(!is_probably_readable(&empty_html));
}

#[test]
fn test_article_output_formats() {
    let html = std::fs::read_to_string(get_site_fixture_path("wikipedia", "article.html")).unwrap();
    let article = parse(&html).expect("should parse");

    #[cfg(feature = "markdown")]
    {
        let md = article.to_markdown().unwrap();
        assert!(!md.is_empty());
    }

    let json = article.to_json().unwrap();
    assert!(json.is_object());
    assert!(json.get("metadata").is_some());
    assert!(json.get("content").is_some());

    let text = article.to_text();
    assert!(!text.is_empty());
}

#[test]
fn test_readability_builder() {
    let html = std::fs::read_to_string(get_site_fixture_path("wikipedia", "article.html")).unwrap();
    let config = LectitoConfig::builder().min_score(10.0).char_threshold(300).build();
    let reader = Readability::with_config(config);
    let article = reader.parse(&html).expect("should parse");
    assert!(!article.content.is_empty());
}

#[test]
fn test_article_metadata() {
    let html = std::fs::read_to_string(get_site_fixture_path("wikipedia", "article.html")).unwrap();
    let article = parse(&html).expect("should parse");

    assert!(article.metadata.title.is_some() || !article.content.is_empty());
    assert!(article.word_count > 0);
    assert!(article.reading_time > 0.0);
    assert!(article.length > 0);
}

#[test]
fn test_article_with_source() {
    let html = std::fs::read_to_string(get_site_fixture_path("github", "article.html")).unwrap();
    let url = "https://github.com/torvalds/linux";
    let article = parse_with_url(&html, url).expect("should parse");
    assert_eq!(article.source_url, Some(url.to_string()));
}

#[test]
fn test_document_api() {
    let html = std::fs::read_to_string(get_site_fixture_path("wikipedia", "article.html")).unwrap();
    let doc = Document::parse(&html).expect("should parse");

    let title = doc.title();
    assert!(title.is_some());

    let metadata = doc.extract_metadata();
    assert!(metadata.title.is_some() || metadata.excerpt.is_some());
}

#[test]
fn test_extract_content_api() {
    let html = std::fs::read_to_string(get_site_fixture_path("wikipedia", "article.html")).unwrap();
    let doc = Document::parse(&html).expect("should parse");
    let config = ExtractConfig::default();

    let extracted = extract_content(&doc, &config).expect("should extract");

    assert!(!extracted.content.is_empty());
    assert!(extracted.top_score > 0.0);
    assert!(extracted.element_count > 0);
}

#[test]
fn test_extract_config() {
    let html = std::fs::read_to_string(get_site_fixture_path("wikipedia", "article.html")).unwrap();

    let config = ExtractConfig { char_threshold: 100, max_top_candidates: 10, ..Default::default() };

    let doc = Document::parse(&html).expect("should parse");
    let extracted = extract_content(&doc, &config).expect("should extract");

    assert!(!extracted.content.is_empty());
}

#[test]
fn test_edge_case_empty() {
    let html = std::fs::read_to_string(get_fixture_path("empty_content.html")).unwrap();
    let result = parse(&html);

    match result {
        Ok(article) => assert!(article.content.is_empty() || article.content.len() < 100),
        Err(_) => {
            // Acceptable behavior
        }
    }
}

#[test]
fn test_edge_case_malformed() {
    let html = std::fs::read_to_string(get_fixture_path("malformed_html.html")).unwrap();
    let doc = Document::parse(&html).expect("should parse malformed HTML");

    let content = doc.text_content();
    assert!(!content.is_empty());
}

#[test]
fn test_edge_case_unicode() {
    let html = std::fs::read_to_string(get_fixture_path("unicode_heavy.html")).unwrap();
    let result = parse(&html);

    match result {
        Ok(article) => assert!(article.content.contains("International")),
        Err(_) => {
            // Expected behavior
        }
    }
}

#[test]
fn test_multiple_site_fixtures() {
    let fixtures = vec![
        get_site_fixture_path("wikipedia", "article.html"),
        get_site_fixture_path("github", "article.html"),
    ];

    for fixture_path in fixtures {
        let html = std::fs::read_to_string(&fixture_path).expect("fixture should exist");

        match parse(&html) {
            Ok(article) => {
                assert!(
                    article.metadata.title.is_some() || !article.content.is_empty(),
                    "Fixture {} should have title or content",
                    fixture_path
                );
                assert!(
                    !article.content.is_empty(),
                    "Fixture {} should have content",
                    fixture_path
                );
            }
            Err(_) => {
                // Expected behavior
            }
        }
    }
}

#[cfg(feature = "markdown")]
#[test]
fn test_markdown_feature() {
    let html = std::fs::read_to_string(get_site_fixture_path("wikipedia", "article.html")).unwrap();
    let article = parse(&html).expect("should parse");
    let md = article.to_markdown().unwrap();

    assert!(!md.is_empty());
    assert!(md.contains("Rust"));
}

#[cfg(feature = "siteconfig")]
#[test]
fn test_siteconfig_feature() {
    let mut loader = ConfigLoader::default();
    let _ = loader.load_for_url("https://example.com");
}
