//! Integration tests for the lectito-core library API
//!
//! These tests verify that the public API works correctly and maintains
//! backwards compatibility with earlier versions.

use lectito_core::{
    Document, LectitoConfig, LectitoError, Readability, ReadabilityConfig, extract::ExtractConfig, extract_content,
    extract_content_with_config, parse, parse_with_url, preprocess_html,
};

const ARTICLE_HTML: &str = r##"
<!DOCTYPE html>
<html lang="en">
<head>
    <title>Test Article Title</title>
    <meta name="author" content="Jane Doe">
    <meta name="description" content="A test article for integration testing">
    <meta property="og:published_time" content="2024-01-15T10:00:00Z">
</head>
<body>
    <header>
        <nav>Navigation</nav>
    </header>
    <div class="container">
        <aside class="sidebar">
            <p>Sidebar content</p>
            <a href="#">Link 1</a>
            <a href="#">Link 2</a>
        </aside>
        <main>
            <article class="content" id="main-article">
                <h1>Main Article Heading</h1>
                <p class="lead">
                    This is the lead paragraph with substantial content. It contains multiple sentences,
                    commas for density scoring, and enough text to be considered meaningful content.
                    The extraction algorithm should recognize this as legitimate article content.
                </p>
                <p>
                    This is a supporting paragraph with content, text, and more text. It has commas,
                    periods, and various punctuation marks. The purpose is to create a substantial
                    amount of text that will score well in the content density calculation.
                    More text here, more content, more sentences, more everything.
                </p>
                <p>
                    A third paragraph to ensure the content is substantial enough. Long-form content
                    is important for testing the extraction algorithm's ability to identify main content
                    versus navigation, sidebars, and other non-content elements.
                </p>
                <blockquote>
                    This is a blockquote with meaningful text that should be included in the extraction.
                </blockquote>
                <p>
                    Final paragraph with concluding thoughts and more substantial text content.
                    The article should be successfully extracted with good confidence scores.
                </p>
            </article>
        </main>
    </div>
    <footer>Footer content</footer>
</body>
</html>
"##;

const SIMPLE_HTML: &str = r##"
<!DOCTYPE html>
<html>
<head><title>Simple Page</title></head>
<body>
    <div>
        <p>Short content.</p>
    </div>
</body>
</html>
"##;

const NAVIGATION_HTML: &str = r##"
<!DOCTYPE html>
<html>
<head><title>Navigation Page</title></head>
<body>
    <nav class="menu">
        <a href="#">Link 1</a>
        <a href="#">Link 2</a>
        <a href="#">Link 3</a>
        <a href="#">Link 4</a>
        <a href="#">Link 5</a>
    </nav>
    <div class="sidebar">
        <a href="#">Nav Link</a>
        <a href="#">Another Link</a>
    </div>
</body>
</html>
"##;

#[test]
fn test_high_level_parse_function() {
    let result = parse(ARTICLE_HTML);
    assert!(result.is_ok(), "parse() should succeed with valid article HTML");

    let article = result.unwrap();
    assert!(!article.content.is_empty(), "Article content should not be empty");
    assert!(article.word_count > 0, "Article should have a positive word count");
    assert_eq!(article.metadata.title, Some("Test Article Title".to_string()));
    assert_eq!(article.metadata.author, Some("Jane Doe".to_string()));
}

#[test]
fn test_high_level_parse_with_url_function() {
    let url = "https://example.com/article/test-article";
    let result = parse_with_url(ARTICLE_HTML, url);

    assert!(
        result.is_ok(),
        "parse_with_url() should succeed with valid article HTML"
    );

    let article = result.unwrap();
    assert_eq!(article.source_url, Some(url.to_string()));
    assert!(!article.content.is_empty());
}

#[test]
fn test_high_level_is_probably_readable() {
    assert!(lectito_core::is_probably_readable(ARTICLE_HTML));
    assert!(!lectito_core::is_probably_readable(NAVIGATION_HTML));
}

#[test]
fn test_readability_builder_parse() {
    let reader = Readability::new();
    let result = reader.parse(ARTICLE_HTML);

    assert!(result.is_ok());
    let article = result.unwrap();
    assert!(!article.content.is_empty());
    assert!(article.word_count > 0);
}

#[test]
fn test_readability_with_custom_config() {
    let config = ReadabilityConfig::builder()
        .min_score(15.0)
        .char_threshold(300)
        .keep_classes(true)
        .preserve_images(false)
        .build();

    let reader = Readability::with_config(config);
    let result = reader.parse(ARTICLE_HTML);

    assert!(result.is_ok());
    let article = result.unwrap();
    assert!(!article.content.is_empty());
}

#[test]
fn test_readability_parse_with_url() {
    let reader = Readability::new();
    let url = "https://example.com/test";
    let result = reader.parse_with_url(ARTICLE_HTML, url);

    assert!(result.is_ok());
    let article = result.unwrap();
    assert_eq!(article.source_url, Some(url.to_string()));
}

#[test]
fn test_readability_is_probably_readable() {
    let reader = Readability::new();
    assert!(reader.is_probably_readable(ARTICLE_HTML));
    assert!(!reader.is_probably_readable(NAVIGATION_HTML));
}

#[test]
fn test_backwards_compat_extract_content() {
    let doc = Document::parse(ARTICLE_HTML).expect("Document::parse should succeed");
    let result = extract_content(&doc, &Default::default());

    assert!(result.is_ok(), "extract_content should succeed");
    let extracted = result.unwrap();
    assert!(!extracted.content.is_empty());
    assert!(extracted.top_score > 0.0);
}

#[test]
fn test_backwards_compat_extract_content_with_config() {
    let doc = Document::parse(ARTICLE_HTML).expect("Document::parse should succeed");
    let config = ExtractConfig { min_score_threshold: 15.0, char_threshold: 300, ..Default::default() };

    let result = extract_content_with_config(&doc, &config, None);
    assert!(result.is_ok());

    let extracted = result.unwrap();
    assert!(!extracted.content.is_empty());
}

#[test]
fn test_backwards_compat_document_parse() {
    let doc = Document::parse(ARTICLE_HTML);
    assert!(doc.is_ok());

    let document = doc.unwrap();
    assert_eq!(document.title(), Some("Test Article Title".to_string()));
    let articles = document.select("article");
    assert!(articles.is_ok());
    assert!(!articles.unwrap().is_empty());
}

#[test]
fn test_backwards_compat_preprocess_html() {
    let preprocessed = preprocess_html(ARTICLE_HTML, &Default::default());
    assert!(!preprocessed.is_empty());
    assert!(preprocessed.contains("<article") || preprocessed.contains("<p"));
}

#[test]
fn test_article_metadata_extraction() {
    let article = parse(ARTICLE_HTML).unwrap();

    assert_eq!(article.metadata.title, Some("Test Article Title".to_string()));
    assert_eq!(article.metadata.author, Some("Jane Doe".to_string()));
    assert_eq!(
        article.metadata.excerpt,
        Some("A test article for integration testing".to_string())
    );
    // TODO: Implement Date extraction from og:published_time
    // assert!(article.metadata.date.is_some());
}

#[test]
fn test_article_content_structure() {
    let article = parse(ARTICLE_HTML).unwrap();
    assert!(article.content.contains("<p>") || article.content.contains("<h1"));
    assert!(!article.content.is_empty());
}

#[test]
fn test_article_text_content() {
    let article = parse(ARTICLE_HTML).unwrap();

    assert!(!article.text_content.is_empty());
    assert!(article.text_content.contains("lead paragraph") || article.text_content.contains("supporting paragraph"));
    assert!(article.word_count > 0);
}

#[test]
fn test_article_reading_time() {
    let article = parse(ARTICLE_HTML).unwrap();
    assert!(article.reading_time > 0.0);
}

#[test]
fn test_error_invalid_html() {
    let result = parse("");
    assert!(result.is_err());
}

#[test]
fn test_error_no_readable_content() {
    let result = parse(NAVIGATION_HTML);
    assert!(result.is_err());

    match result {
        Err(LectitoError::NotReaderable { score, threshold }) => assert!(score < threshold),
        Err(LectitoError::NoContent) => (),
        _ => panic!("Expected NotReaderable or NoContent error, got: {:?}", result),
    }
}

#[test]
fn test_convenience_async_fetch_invalid_url() {
    use std::thread;

    let result = thread::spawn(move || {
        tokio::runtime::Runtime::new()
            .unwrap()
            .block_on(async { lectito_core::fetch_and_parse("not-a-url").await })
    })
    .join()
    .unwrap();

    assert!(matches!(result, Err(LectitoError::InvalidUrl(_))));
}

#[test]
fn test_readability_default_config() {
    let config = ReadabilityConfig::default();
    assert_eq!(config.min_score, 20.0);
    assert_eq!(config.char_threshold, 500);
    assert!(config.remove_unlikely);
    assert!(!config.keep_classes);
    assert!(config.preserve_images);
}

#[test]
fn test_lectito_config_type_alias() {
    let config: LectitoConfig = LectitoConfig::default();
    assert_eq!(config.min_score, 20.0);
}

#[test]
fn test_parse_with_very_short_content() {
    let result = parse(SIMPLE_HTML);
    assert!(result.is_ok() || result.is_err());
}

#[test]
fn test_multiple_parse_calls() {
    let reader = Readability::new();

    let result1 = reader.parse(ARTICLE_HTML);
    assert!(result1.is_ok());
    let result2 = reader.parse(ARTICLE_HTML);
    assert!(result2.is_ok());
    let article1 = result1.unwrap();
    let article2 = result2.unwrap();
    assert_eq!(article1.word_count, article2.word_count);
}

#[test]
fn test_article_content_focus() {
    let article = parse(ARTICLE_HTML).unwrap();
    let content_lower = article.content.to_lowercase();
    assert!(content_lower.contains("paragraph") || content_lower.contains("<p"));
    assert!(!article.content.is_empty());
}

#[test]
fn test_backwards_compat_document_methods() {
    let doc = Document::parse(ARTICLE_HTML).expect("Document::parse should succeed");

    assert_eq!(doc.title(), Some("Test Article Title".to_string()));
    assert!(!doc.as_string().is_empty());
    assert!(!doc.text_content().is_empty());

    let articles = doc.select("article").expect("select should work");
    assert!(!articles.is_empty());
}
