use std::path::{Path, PathBuf};

use anyhow::Context;
use clap::Args;

use lectito::is_probably_readable;
use lectito::{Article, ReadabilityOptions, ReadableOptions, extract};

#[derive(Debug, Args)]
pub struct FixtureArgs {
    path: PathBuf,
    #[arg(long)]
    url: Option<String>,
    #[arg(long)]
    diff_dir: Option<PathBuf>,
}

#[derive(Debug)]
struct ContentReport {
    text_matches: bool,
    tags_match: bool,
    expected_text: String,
    actual_text: String,
    expected_text_chars: usize,
    actual_text_chars: usize,
    expected_tags: usize,
    actual_tags: usize,
    expected_tag_sequence: Vec<String>,
    actual_tag_sequence: Vec<String>,
}

impl ContentReport {
    fn new(expected_html: &str, actual_html: &str) -> Self {
        let expected_text = lectito_fixtures::normalized_text(expected_html);
        let actual_text = lectito_fixtures::normalized_text(actual_html);
        let expected_tag_sequence = lectito_fixtures::tag_sequence(expected_html);
        let actual_tag_sequence = lectito_fixtures::tag_sequence(actual_html);

        Self {
            text_matches: expected_text == actual_text,
            tags_match: expected_tag_sequence == actual_tag_sequence,
            expected_text_chars: expected_text.chars().count(),
            actual_text_chars: actual_text.chars().count(),
            expected_tags: expected_tag_sequence.len(),
            actual_tags: actual_tag_sequence.len(),
            expected_text,
            actual_text,
            expected_tag_sequence,
            actual_tag_sequence,
        }
    }
}

pub fn run(args: &FixtureArgs) -> anyhow::Result<()> {
    let fixture = load_fixture_arg(&args.path)?;
    let expected_readable = fixture
        .expected_metadata
        .get("readerable")
        .and_then(serde_json::Value::as_bool)
        .unwrap_or(false);
    let actual_readable = is_probably_readable(&fixture.source, &ReadableOptions::default())?;
    let article = extract(&fixture.source, args.url.as_deref(), &ReadabilityOptions::default())?;
    let metadata_mismatches = article
        .as_ref()
        .map(|article| metadata_mismatches(&fixture.expected_metadata, article))
        .unwrap_or_else(|| vec!["article: expected extracted article, got none".to_string()]);
    let content_report = article
        .as_ref()
        .map(|article| ContentReport::new(&fixture.expected_content, &article.content));

    println!(
        "readable: {}",
        if actual_readable == expected_readable { "pass" } else { "mismatch" }
    );
    println!(
        "metadata: {}",
        if metadata_mismatches.is_empty() { "pass" } else { "mismatch" }
    );
    for mismatch in &metadata_mismatches {
        println!("  - {mismatch}");
    }

    if let Some(report) = &content_report {
        println!(
            "content text: {}",
            if report.text_matches { "pass" } else { "mismatch" }
        );
        println!("content tags: {}", if report.tags_match { "pass" } else { "mismatch" });
        println!("expected text chars: {}", report.expected_text_chars);
        println!("actual text chars: {}", report.actual_text_chars);
        println!("expected tags: {}", report.expected_tags);
        println!("actual tags: {}", report.actual_tags);
    } else {
        println!("content text: mismatch");
        println!("content tags: mismatch");
    }

    if let (Some(diff_dir), Some(article), Some(report)) = (&args.diff_dir, &article, &content_report) {
        write_fixture_diff(diff_dir, &fixture.name, &fixture.expected_content, article, report)?;
        println!("diff: {}", diff_dir.display());
    }

    Ok(())
}

fn load_fixture_arg(path: &Path) -> anyhow::Result<lectito_fixtures::Fixture> {
    if path.exists() {
        return lectito_fixtures::load_fixture_path(path)
            .with_context(|| format!("failed to load fixture {}", path.display()));
    }

    let name = path
        .to_str()
        .context("fixture name must be valid UTF-8 when it is not a path")?;
    lectito_fixtures::load_fixture(name).with_context(|| format!("failed to load sample fixture {name}"))
}

fn write_fixture_diff(
    diff_dir: &Path, fixture_name: &str, expected_content: &str, article: &Article, report: &ContentReport,
) -> anyhow::Result<()> {
    let dir = diff_dir.join(
        fixture_name
            .chars()
            .map(|ch| if ch.is_ascii_alphanumeric() || matches!(ch, '-' | '_') { ch } else { '_' })
            .collect::<String>(),
    );
    std::fs::create_dir_all(&dir).with_context(|| format!("failed to create {}", dir.display()))?;
    std::fs::write(dir.join("expected.html"), expected_content)?;
    std::fs::write(dir.join("actual.html"), &article.content)?;
    std::fs::write(dir.join("expected.txt"), &report.expected_text)?;
    std::fs::write(dir.join("actual.txt"), &report.actual_text)?;
    std::fs::write(dir.join("expected-tags.txt"), report.expected_tag_sequence.join("\n"))?;
    std::fs::write(dir.join("actual-tags.txt"), report.actual_tag_sequence.join("\n"))?;
    Ok(())
}

fn metadata_mismatches(expected: &serde_json::Value, article: &Article) -> Vec<String> {
    let checks = [
        ("title", article.title.as_deref()),
        ("byline", article.byline.as_deref()),
        ("dir", article.dir.as_deref()),
        ("excerpt", article.excerpt.as_deref()),
        ("siteName", article.site_name.as_deref()),
        ("publishedTime", article.published_time.as_deref()),
        ("image", article.image.as_deref()),
        ("domain", article.domain.as_deref()),
        ("favicon", article.favicon.as_deref()),
    ];

    checks
        .into_iter()
        .filter_map(|(field, actual)| {
            let expected = expected.get(field);
            if expected.is_none() {
                return None;
            }
            let expected = expected.and_then(serde_json::Value::as_str);
            let matches = match (expected, actual) {
                (Some(expected), Some(actual)) if field == "excerpt" => {
                    lectito_fixtures::normalize_space(expected) == lectito_fixtures::normalize_space(actual)
                }
                (Some(expected), Some(actual)) => expected == actual,
                (None, None) => true,
                _ => false,
            };
            (!matches).then(|| format!("{field}: expected {:?}, got {:?}", expected, actual))
        })
        .collect()
}
