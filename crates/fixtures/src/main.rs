use std::collections::BTreeMap;
use std::path::{Path, PathBuf};

use anyhow::Context;
use clap::Parser;
use lectito::{Article, ReadabilityOptions, ReadableOptions, extract, is_probably_readable};

/// Inspect fixture extraction behavior.
#[derive(Debug, Parser)]
#[command(name = "corpus")]
struct Args {
    /// Fixture name or path to a fixture directory.
    path: Option<PathBuf>,

    /// Check every fixture and print aggregate quality counts.
    #[arg(long)]
    all: bool,

    /// Base URL to use while extracting the fixture.
    #[arg(long)]
    url: Option<String>,

    /// Directory where expected and actual fixture output should be written.
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

    fn write_diff(
        &self, diff_dir: &Path, fixture_name: &str, expected_content: &str, article: &Article,
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
        std::fs::write(dir.join("expected.txt"), &self.expected_text)?;
        std::fs::write(dir.join("actual.txt"), &self.actual_text)?;
        std::fs::write(dir.join("expected-tags.txt"), self.expected_tag_sequence.join("\n"))?;
        std::fs::write(dir.join("actual-tags.txt"), self.actual_tag_sequence.join("\n"))?;
        Ok(())
    }
}

#[derive(Debug)]
struct FixtureReport {
    readable_matches: bool,
    metadata_mismatches: Vec<String>,
    content_report: Option<ContentReport>,
}

impl FixtureReport {
    fn print(&self) {
        println!("readable: {}", if self.readable_matches { "pass" } else { "mismatch" });
        println!(
            "metadata: {}",
            if self.metadata_mismatches.is_empty() { "pass" } else { "mismatch" }
        );
        for mismatch in &self.metadata_mismatches {
            println!("  - {mismatch}");
        }

        if let Some(report) = &self.content_report {
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
    }
}

#[derive(Default)]
struct CorpusSummary {
    total: usize,
    readable_pass: usize,
    metadata_pass: usize,
    text_pass: usize,
    tags_pass: usize,
    metadata_fields: BTreeMap<String, usize>,
}

impl CorpusSummary {
    fn record(&mut self, report: &FixtureReport) {
        self.total += 1;
        if report.readable_matches {
            self.readable_pass += 1;
        }
        if report.metadata_mismatches.is_empty() {
            self.metadata_pass += 1;
        }
        if let Some(content) = &report.content_report {
            if content.text_matches {
                self.text_pass += 1;
            }
            if content.tags_match {
                self.tags_pass += 1;
            }
        }
        for mismatch in &report.metadata_mismatches {
            let field = mismatch.split(':').next().unwrap_or("unknown").to_string();
            *self.metadata_fields.entry(field).or_insert(0) += 1;
        }
    }

    fn print(&self) {
        println!("fixtures: {}", self.total);
        println!("readable: {}/{} pass", self.readable_pass, self.total);
        println!("metadata: {}/{} pass", self.metadata_pass, self.total);
        println!("content text: {}/{} pass", self.text_pass, self.total);
        println!("content tags: {}/{} pass", self.tags_pass, self.total);
        if !self.metadata_fields.is_empty() {
            println!("metadata mismatch fields:");
            for (field, count) in &self.metadata_fields {
                println!("  - {field}: {count}");
            }
        }
    }
}

fn main() -> anyhow::Result<()> {
    let args = Args::parse();
    run(&args)
}

fn run(args: &Args) -> anyhow::Result<()> {
    if args.all {
        return run_all(args);
    }

    let path = args
        .path
        .as_deref()
        .context("fixture name/path is required unless --all is set")?;
    let fixture = load_fixture_arg(path)?;
    let report = inspect_fixture(fixture, args.url.as_deref())?;

    report.print();

    if let (Some(diff_dir), Some(content_report)) = (&args.diff_dir, &report.content_report) {
        let fixture = load_fixture_arg(path)?;
        let article = extract(&fixture.source, args.url.as_deref(), &ReadabilityOptions::default())?
            .context("expected extracted article when writing fixture diff")?;
        content_report.write_diff(diff_dir, &fixture.name, &fixture.expected_content, &article)?;
        println!("diff: {}", diff_dir.display());
    }

    Ok(())
}

fn run_all(args: &Args) -> anyhow::Result<()> {
    let mut summary = CorpusSummary::default();
    for fixture in lectito_fixtures::load_all().context("failed to load fixture corpus")? {
        let report = inspect_fixture(fixture, args.url.as_deref())?;
        summary.record(&report);
    }

    summary.print();
    Ok(())
}

fn inspect_fixture(fixture: lectito_fixtures::Fixture, url: Option<&str>) -> anyhow::Result<FixtureReport> {
    let expected_readable = fixture
        .expected_metadata
        // FIXME: "readerable" is the upstream fixture field name.
        .get("readerable")
        .and_then(serde_json::Value::as_bool)
        .unwrap_or(false);
    let actual_readable = is_probably_readable(&fixture.source, &ReadableOptions::default())?;
    let article = extract(&fixture.source, url, &ReadabilityOptions::default())?;
    let metadata_mismatches = article
        .as_ref()
        .map(|article| metadata_mismatches(&fixture.expected_metadata, article))
        .unwrap_or_else(|| vec!["article: expected extracted article, got none".to_string()]);
    let content_report = article
        .as_ref()
        .map(|article| ContentReport::new(&fixture.expected_content, &article.content));

    Ok(FixtureReport { readable_matches: actual_readable == expected_readable, metadata_mismatches, content_report })
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
            expected?;
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
