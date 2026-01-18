//! CLI integration tests
use predicates::prelude::*;
use tempfile::TempDir;

fn cmd() -> assert_cmd::Command {
    assert_cmd::cargo::cargo_bin_cmd!("lectito-cli")
}

fn get_fixture_path(name: &str) -> String {
    format!("../../tests/fixtures/{}", name)
}

fn get_site_fixture_path(site: &str, name: &str) -> String {
    format!("../../tests/fixtures/sites/{}/{}", site, name)
}

#[test]
fn test_cli_file_input() {
    cmd()
        .arg(get_site_fixture_path("wikipedia", "article.html"))
        .assert()
        .success();
}

#[test]
fn test_cli_stdin_input() {
    let html = std::fs::read_to_string(get_site_fixture_path("wikipedia", "article.html")).unwrap();
    cmd().arg("-").write_stdin(html).assert().success();
}

#[test]
fn test_cli_markdown_format() {
    cmd()
        .args(["-f", "markdown", &get_site_fixture_path("wikipedia", "article.html")])
        .assert()
        .success()
        .stdout(predicate::str::contains("Rust"));
}

#[test]
fn test_cli_html_format() {
    cmd()
        .args(["-f", "html", &get_site_fixture_path("wikipedia", "article.html")])
        .assert()
        .success()
        .stdout(predicate::str::contains("<h1"));
}

#[test]
fn test_cli_text_format() {
    cmd()
        .args(["-f", "text", &get_site_fixture_path("wikipedia", "article.html")])
        .assert()
        .success()
        .stdout(predicate::str::contains("Rust"));
}

#[test]
fn test_cli_json_format() {
    cmd()
        .args(["-f", "json", &get_site_fixture_path("wikipedia", "article.html")])
        .assert()
        .success()
        .stdout(predicate::str::contains("content"));
}

#[test]
fn test_cli_output_file() {
    let tmp = TempDir::new().unwrap();
    let output = tmp.path().join("output.md");

    cmd()
        .args(["-o", output.to_str().unwrap()])
        .arg(get_site_fixture_path("wikipedia", "article.html"))
        .assert()
        .success();

    assert!(output.exists());
}

#[test]
fn test_cli_metadata_only() {
    cmd()
        .args(["-m", &get_site_fixture_path("wikipedia", "article.html")])
        .assert()
        .success()
        .stdout(predicate::str::contains("title"));
}

#[test]
fn test_cli_metadata_json() {
    cmd()
        .args([
            "-m",
            "--metadata-format",
            "json",
            &get_site_fixture_path("wikipedia", "article.html"),
        ])
        .assert()
        .success()
        .stdout(predicate::str::starts_with("{"));
}

#[test]
fn test_cli_frontmatter() {
    cmd()
        .args(["--frontmatter", &get_site_fixture_path("wikipedia", "article.html")])
        .assert()
        .success()
        .stdout(predicate::str::contains("+++"));
}

#[test]
fn test_cli_invalid_file() {
    cmd().arg("nonexistent.html").assert().failure();
}

#[test]
fn test_cli_empty_content() {
    cmd().arg(get_fixture_path("empty_content.html")).assert().failure();
}

#[test]
fn test_cli_malformed_html() {
    cmd().arg(get_fixture_path("malformed_html.html")).assert().failure();
}

#[test]
fn test_cli_unicode_content() {
    cmd()
        .arg(get_fixture_path("unicode_heavy.html"))
        .assert()
        .success()
        .stdout(predicate::str::contains("International"));
}

#[test]
fn test_cli_verbose() {
    cmd()
        .args(["-v", &get_site_fixture_path("wikipedia", "article.html")])
        .assert()
        .success()
        .stderr(predicate::str::contains("Lectito"));
}

#[test]
fn test_cli_github_fixture() {
    cmd()
        .arg(get_site_fixture_path("github", "article.html"))
        .assert()
        .success()
        .stdout(predicate::str::contains("Linux kernel"));
}


#[test]
fn test_cli_char_threshold() {
    cmd()
        .args([
            "--char-threshold",
            "100",
            &get_site_fixture_path("wikipedia", "article.html"),
        ])
        .assert()
        .success();
}

#[test]
fn test_cli_max_elements() {
    cmd()
        .args([
            "--max-elements",
            "10",
            &get_site_fixture_path("wikipedia", "article.html"),
        ])
        .assert()
        .success();
}

#[test]
fn test_cli_no_images() {
    cmd()
        .args(["--no-images", &get_site_fixture_path("wikipedia", "article.html")])
        .assert()
        .success();
}

#[test]
fn test_cli_references() {
    cmd()
        .args(["--references", &get_site_fixture_path("wikipedia", "article.html")])
        .assert()
        .success()
        .stdout(predicate::str::contains("##"));
}
