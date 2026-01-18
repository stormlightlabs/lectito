# Lectito

A fetch tool to download, parse and extract content from web pages using a
Readability-inspired algorithm and set of heuristics.

<!-- markdownlint-disable MD033 -->
<details>
<summary>How it works</summary>

Lectito implements a content extraction algorithm inspired by Mozilla's [Readability.js](https://github.com/mozilla/readability):

1. **Preprocessing**: Removes scripts, styles, comments, and unlikely content candidates
2. **Scoring**: Analyzes elements based on tag names, class/ID patterns, content density, and link density
3. **Selection**: Identifies the highest-scoring content candidate, preferring semantic containers when scores are close
4. **Sibling Inclusion**: Adds related content based on score thresholds, link density, and shared parent headers
5. **Cleanup**: Removes empty nodes, fixes relative URLs, and applies formatting rules

~~For a deeper dive into the algorithm, see the [How It Works](https://stormlightlabs.github.io/lectito/concepts/how-it-works.html) documentation.~~

</details>

## Features

- **Content Extraction**: Extracts the main article content from navigation, sidebars, and advertisements
- **Multiple Output Formats**: HTML, Markdown, plain text, and JSON
- **Site Configuration**: Optional XPath-based extraction rules for difficult sites
- **CLI and Library**: Use as a command-line tool or as a Rust library

## Documentation

- **User Guide**: ~~[Available online](https://stormlightlabs.github.io/lectito/)~~
- **API Reference**: ~~[docs.rs/lectito](https://docs.rs/lectito)~~
- **Changelog**: [CHANGELOG.md](CHANGELOG.md)

## Installation

### CLI Tool

For installation and usage of the `lectito` CLI tool, see the cli's [README](crates/cli/README.md).

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

### Library Usage

Parse HTML from a string:

```rs
use lectito_core::{Document, extract_content};

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

let doc = Document::parse(html)?;
let extracted = extract_content(&doc, &Default::default())?;
let metadata = doc.extract_metadata();
println!("Title: {:?}", metadata.title);
```

Fetch and parse from a URL:

```rs
use lectito_core::{Document, fetch_url, extract_content};

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    let html = fetch_url("https://example.com/article", &Default::default()).await?;
    let doc = Document::parse(&html)?;
    let extracted = extract_content(&doc, &Default::default())?;
    Ok(())
}
```

Convert to different output formats:

```rs
use lectito_core::{Document, convert_to_markdown};

let html = "<h1>Title</h1><p>Content here</p>";
let doc = Document::parse(html)?;
let extracted = extract_content(&doc, &Default::default())?;
let metadata = doc.extract_metadata();

// Get as Markdown with frontmatter
let markdown = convert_to_markdown(&extracted.content, &metadata, &Default::default())?;
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

For library usage, use the `ExtractConfig` for advanced extraction configuration:

```rs
use lectito_core::{Document, ExtractConfig, extract_content};

let config = ExtractConfig {
    char_threshold: 500,
    max_top_candidates: 10,
    ..Default::default()
};

let extracted = extract_content(&doc, &config)?;
```

## License

MPL-2.0

## Related Projects

- [Thunderus AI Agent](https://github.com/stormlightlabs/thunderus) - AI agent that uses Lectito as a fetch tool

- [Mccabre](https://github.com/stormlightlabs/mccabre) - Code analysis tool
