# Lectito

A Rust library and CLI for extracting readable content from web pages using a
Readability-inspired algorithm and a growing set of extraction heuristics.

<!-- markdownlint-disable MD033 -->
<details>
<summary>How it works</summary>

Lectito implements a content extraction algorithm inspired by Mozilla's [Readability.js](https://github.com/mozilla/readability):

1. **Preprocessing**: Removes scripts, styles, comments, and unlikely content candidates
2. **Scoring**: Analyzes elements based on tag names, class/ID patterns, content density, and link density
3. **Selection**: Identifies the highest-scoring content candidate, preferring semantic containers when scores are close
4. **Sibling Inclusion**: Adds related content based on score thresholds, link density, and shared parent headers
5. **Cleanup**: Removes empty nodes, fixes relative URLs, and applies formatting rules
6. **Retry and Overrides**: Retries extraction with looser settings when needed and can use site-specific extractors or site configs for difficult pages

For a deeper dive into the algorithm, see [How It Works](docs/src/concepts/how-it-works.md).

</details>

## Features

- **Content Extraction**: Extracts the main article content from navigation, sidebars, and advertisements
- **Multiple Output Formats**: HTML, Markdown, plain text, and JSON
- **Confidence and Diagnostics**: Returns a confidence score and optional extraction diagnostics
- **Site Configuration**: Optional XPath-based extraction rules for difficult sites
- **Site Extractors**: Built-in handling for sites like GitHub, docs.rs, Reddit, Hacker News, Substack, and YouTube
- **CLI and Library**: Use as a command-line tool or as a Rust library

## Documentation

- **User Guide**: [docs/src/README.md](docs/src/README.md)
- **API Overview**: [docs/src/reference/api-overview.md](docs/src/reference/api-overview.md)
- **CLI Guide**: [docs/src/getting-started/cli-usage.md](docs/src/getting-started/cli-usage.md)
- **Changelog**: [CHANGELOG.md](CHANGELOG.md)

## Installation

### CLI Tool

For installation and usage of the `lectito` CLI tool, see the cli's [README](crates/cli/README.md).

Common CLI options include:

- `--frontmatter` and `--references` for Markdown exports
- `--json` or `--format json` for structured output
- `--metadata-only --metadata-format json|toml` for metadata extraction
- `--config-dir` for site configuration overrides

### Library

Add `lectito-core` to your `Cargo.toml`:

```toml
[dependencies]
lectito-core = "1.0"
```

With specific features:

```toml
[dependencies]
lectito-core = { version = "1.0", default-features = false, features = ["fetch", "markdown"] }
```

## Quick Start

See [cli/README.md](crates/cli/README.md) for CLI usage examples.

```sh
lectito https://example.com/article --frontmatter --references
```

### Library Usage

Parse HTML from a string:

```rs
use lectito_core::parse;

let html = r#"
    <!DOCTYPE html>
    <html>
        <head><title>My Article</title></head>
        <body>
            <article>
                <h1>Article Title</h1>
                <p>This is the article content with plenty of text.</p>
            </article>
        </body>
    </html>
"#;

let article = parse(html)?;
println!("Title: {:?}", article.metadata.title);
println!("Confidence: {:.2}", article.confidence);
```

Fetch and parse from a URL:

```rs
use lectito_core::fetch_and_parse;

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    let article = fetch_and_parse("https://example.com/article").await?;
    println!("{:?}", article.metadata.title);
    Ok(())
}
```

This helper requires the `fetch` feature.

Convert to different output formats:

```rs
use lectito_core::{parse, MarkdownConfig};

let article = parse("<h1>Title</h1><p>Content here</p>")?;

let markdown = article.to_markdown_with_config(&MarkdownConfig {
    include_frontmatter: true,
    include_references: true,
    strip_images: false,
    include_title_heading: true,
})?;
let text = article.to_text();
let json = article.to_json()?;
```

## Feature Flags

| Feature      | Default | Description                                                    |
| ------------ | ------- | -------------------------------------------------------------- |
| `fetch`      | Yes     | Enable async URL fetching with reqwest                         |
| `markdown`   | Yes     | Enable Markdown output conversion                              |
| `siteconfig` | Yes     | Enable site configuration support (XPath rules)                |
| `json`       | Always  | JSON output support (always enabled for Article serialization) |
| `full`       | No      | Enable all features                                            |

Disable default features and select only what you need:

```toml
[dependencies]
lectito-core = { version = "1.0", default-features = false, features = ["fetch"] }
```

## Configuration

See [crates/cli/README.md](crates/cli/README.md) for CLI configuration options.

For library usage, `ReadabilityConfig` is the main public configuration surface:

```rs
use lectito_core::{Readability, ReadabilityConfig};

let config = ReadabilityConfig::builder()
    .min_score(20.0)
    .char_threshold(500)
    .nb_top_candidates(8)
    .preserve_images(true)
    .preserve_video_embeds(true)
    .build();

let reader = Readability::with_config(config);
let article = reader.parse("<html>...</html>")?;
```

## License

MPL-2.0

## Related Projects

- [Thunderus AI Agent](https://github.com/stormlightlabs/thunderus) - AI agent that uses Lectito as a fetch tool

- [Mccabre](https://github.com/stormlightlabs/mccabre) - Code analysis tool
