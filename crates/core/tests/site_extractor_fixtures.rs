use lectito_core::article::Article;
use lectito_core::siteextractors::{ExtractorOutcome, ExtractorRegistry};
use lectito_core::{ConfigLoaderBuilder, Document, FetchConfig, Readability, ReadabilityConfig};
use serde::Deserialize;
use std::fs;
use std::path::{Path, PathBuf};
use url::Url;

fn fixture_root() -> PathBuf {
    PathBuf::from("../../tests/fixtures/extractors")
}

fn site_config_root() -> PathBuf {
    PathBuf::from("../../site_configs")
}

#[derive(Debug, Deserialize)]
struct FixtureManifest {
    url: String,
    mode: String,
    title: Option<String>,
    author: Option<String>,
    date: Option<String>,
    min_words: usize,
    must_contain: Vec<String>,
    must_not_contain: Vec<String>,
}

#[test]
fn site_extractor_fixture_suite() {
    let root = fixture_root();
    let mut cases = Vec::new();

    for group in fs::read_dir(&root).unwrap() {
        let group = group.unwrap();
        if !group.file_type().unwrap().is_dir() {
            continue;
        }

        for case in fs::read_dir(group.path()).unwrap() {
            let case = case.unwrap();
            if case.file_type().unwrap().is_dir() {
                cases.push(case.path());
            }
        }
    }

    cases.sort();
    assert!(!cases.is_empty(), "fixture suite should not be empty");

    for case_dir in cases {
        run_case(&case_dir);
    }
}

#[test]
fn fetch_flow_applies_site_config_headers() {
    use std::io::{Read, Write};
    use std::net::TcpListener;
    use std::sync::{Arc, Mutex};

    let listener = TcpListener::bind("127.0.0.1:0").unwrap();
    let addr = listener.local_addr().unwrap();
    let seen_header = Arc::new(Mutex::new(false));
    let seen_header_bg = seen_header.clone();

    std::thread::spawn(move || {
        if let Ok((mut stream, _)) = listener.accept() {
            let mut buffer = [0; 4096];
            let n = stream.read(&mut buffer).unwrap();
            let request = String::from_utf8_lossy(&buffer[..n]);
            if request.to_ascii_lowercase().contains("x-test: applied") {
                *seen_header_bg.lock().unwrap() = true;
            }

            let response = concat!(
                "HTTP/1.1 200 OK\r\n",
                "Content-Type: text/html; charset=utf-8\r\n",
                "Content-Length: 87\r\n",
                "\r\n",
                "<html><head><title>Header test</title></head><body><article><p>Header OK here.</p></article></body></html>"
            );
            stream.write_all(response.as_bytes()).unwrap();
        }
    });

    let temp_dir = tempfile::TempDir::new().unwrap();
    fs::write(
        temp_dir.path().join("127.0.0.1.txt"),
        "http_header(X-Test): applied\nbody: //article\n",
    )
    .unwrap();

    let loader = ConfigLoaderBuilder::new().custom_dir(temp_dir.path()).build();
    let reader = Readability::with_config_and_loader(ReadabilityConfig::default(), loader);
    let url = format!("http://{}/article", addr);
    let rt = tokio::runtime::Runtime::new().unwrap();
    let article = rt
        .block_on(reader.fetch_and_parse_with_config(&url, &FetchConfig::default()))
        .unwrap();

    assert!(article.content.contains("Header OK"));
    assert!(
        *seen_header.lock().unwrap(),
        "expected custom site_config header to be applied"
    );
}

fn run_case(case_dir: &Path) {
    let manifest: FixtureManifest =
        serde_json::from_str(&fs::read_to_string(case_dir.join("manifest.json")).unwrap()).unwrap();
    let input_html = fs::read_to_string(case_dir.join("input.html")).unwrap();
    let expected_html = fs::read_to_string(case_dir.join("expected.html")).unwrap();
    let expected_markdown = fs::read_to_string(case_dir.join("expected.md")).unwrap();
    let article = load_article(case_dir, &manifest, &input_html);

    if let Some(title) = manifest.title.as_deref() {
        assert_eq!(article.metadata.title.as_deref(), Some(title), "{}", case_dir.display());
    }
    if let Some(author) = manifest.author.as_deref() {
        assert_eq!(
            article.metadata.author.as_deref(),
            Some(author),
            "{}",
            case_dir.display()
        );
    }
    if let Some(date) = manifest.date.as_deref() {
        assert_eq!(article.metadata.date.as_deref(), Some(date), "{}", case_dir.display());
    }

    assert!(
        article.word_count >= manifest.min_words,
        "{} expected at least {} words, got {}",
        case_dir.display(),
        manifest.min_words,
        article.word_count
    );

    let text = normalize_ws(&article.to_text());
    for needle in manifest.must_contain {
        assert!(
            text.contains(&normalize_ws(&needle)),
            "{} missing {needle}",
            case_dir.display()
        );
    }
    for needle in manifest.must_not_contain {
        assert!(
            !text.contains(&normalize_ws(&needle)),
            "{} unexpectedly contained {needle}",
            case_dir.display()
        );
    }

    assert_eq!(
        normalize_html(&article.content),
        normalize_html(&expected_html),
        "{} html mismatch",
        case_dir.display()
    );
    assert_eq!(
        normalize_markdown_body(&article.to_markdown().unwrap()),
        normalize_markdown_body(&expected_markdown),
        "{} markdown mismatch",
        case_dir.display()
    );
}

fn load_article(case_dir: &Path, manifest: &FixtureManifest, input_html: &str) -> Article {
    match manifest.mode.as_str() {
        "selector_hint" | "html_override" => Readability::new().parse_with_url(input_html, &manifest.url).unwrap(),
        "generic" => {
            let config_dir = case_dir.join("configs");
            let loader = ConfigLoaderBuilder::new().custom_dir(config_dir).build();
            let reader = Readability::with_config_and_loader(ReadabilityConfig::default(), loader);
            reader.parse_with_url(input_html, &manifest.url).unwrap()
        }
        "site_config" => {
            let loader = ConfigLoaderBuilder::new().custom_dir(site_config_root()).build();
            let reader = Readability::with_config_and_loader(ReadabilityConfig::default(), loader);
            reader.parse_with_url(input_html, &manifest.url).unwrap()
        }
        "async_override" => {
            let url = Url::parse(&manifest.url).unwrap();
            let doc = Document::parse_with_base_url(input_html, Some(url.clone())).unwrap();
            let registry = ExtractorRegistry::new();
            let rt = tokio::runtime::Runtime::new().unwrap();
            let outcome = rt
                .block_on(registry.extract_async(&doc, &url, &FetchConfig::default()))
                .unwrap()
                .expect("async extractor should produce content");
            outcome_to_article(&doc, &url, outcome)
        }
        other => panic!("unsupported fixture mode {other} for {}", case_dir.display()),
    }
}

fn outcome_to_article(doc: &Document, url: &Url, outcome: ExtractorOutcome) -> Article {
    match outcome {
        ExtractorOutcome::Selector { selector } => {
            let html = doc
                .select(&selector)
                .unwrap()
                .into_iter()
                .map(|element| element.outer_html())
                .collect::<Vec<_>>()
                .join("\n");
            Article::from_document(doc, html, Some(url.to_string()))
        }
        ExtractorOutcome::Html(outcome) => {
            let content_html = outcome.content_html;
            let metadata_patch = outcome.metadata_patch;
            Article::from_document_with_metadata(doc, content_html, Some(url.to_string()), &metadata_patch)
        }
    }
}

fn normalize_html(value: &str) -> String {
    normalize_ws(
        &value
            .replace("=\"\"", "")
            .replace('>', "> ")
            .replace('<', " <")
            .replace("&quot;", "\"")
            .replace("&amp;", "&"),
    )
}

fn normalize_markdown_body(value: &str) -> String {
    let value = value.replace("\\_", "_").replace("\\[", "[").replace("\\]", "]");
    let mut lines = value.lines();
    if matches!(lines.next(), Some("+++")) {
        for line in &mut lines {
            if line.trim() == "+++" {
                break;
            }
        }
    } else {
        lines = value.lines();
    }

    normalize_ws(&lines.collect::<Vec<_>>().join("\n"))
}

fn normalize_ws(value: &str) -> String {
    value.split_whitespace().collect::<Vec<_>>().join(" ")
}
