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

### CLI

Install the binary via cargo:

```bash
cargo install lectito-cli
```

~~Or download pre-built binaries from the [GitHub Releases](https://github.com/stormlightlabs/lectito/releases) page.~~

### Library

Add `lectito` to your `Cargo.toml`:

```toml
[dependencies]
lectito = "0.1"
```

With specific features:

```toml
[dependencies]
lectito = { version = "0.1", default-features = false, features = ["fetch", "markdown"] }
```

## Quick Start

### CLI Usage

Extract an article from a URL:

```bash
lectito https://example.com/article
```

From a local file:

```bash
lectito article.html
```

From stdin:

```bash
curl https://example.com | lectito -
```

Output options:

```bash
# Save to file
lectito https://example.com/article -o article.md

# JSON output with metadata
lectito https://example.com/article --json

# Plain text only
lectito https://example.com/article --format text

# Include frontmatter and reference links
lectito https://example.com/article --frontmatter --references
```

### Library Usage

Parse HTML from a string:

```rs
use lectito::parse;

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
```

Fetch and parse from a URL:

```rs
use lectito::fetch_and_parse;

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    let article = fetch_and_parse("https://example.com/article").await?;
    println!("Extracted {} words", article.word_count);
    Ok(())
}
```

Convert to different output formats:

```rs
use lectito::{parse, formatters::convert_to_markdown};

let html = "<h1>Title</h1><p>Content here</p>";
let article = parse(html)?;

// Get as Markdown with frontmatter
let markdown = convert_to_markdown(&article.content, &article.metadata, &Default::default())?;

// Get as plain text
let text = article.to_text();

// Get as structured JSON
let json = article.to_json()?;
```

## Feature Flags

| Feature | Default | Description |
| ------- | ------- | ----------- |
| `fetch` | Yes | Enable async URL fetching with reqwest |
| `markdown` | Yes | Enable Markdown output conversion |
| `siteconfig` | Yes | Enable site configuration support (XPath rules) |
| `json` | Always | JSON output support (always enabled for Article serialization) |
| `full` | No | Enable all features |

Disable default features and select only what you need:

```toml
[dependencies]
lectito = { version = "0.1", default-features = false, features = ["fetch"] }
```

## Configuration

Use the builder pattern for advanced extraction configuration:

```rs
use lectito::{Readability, ReadabilityConfig};

let config = ReadabilityConfig::builder()
    .min_score(25.0)
    .char_threshold(500)
    .max_top_candidates(10)
    .build();

let reader = Readability::with_config(config);
let article = reader.parse(html)?;
```

### CLI Configuration

The CLI supports various configuration options:

```bash
lectito https://example.com/article \
  --timeout 60 \
  --user-agent "MyBot/1.0" \
  --char-threshold 1000 \
  --max-elements 10 \
  --no-images
```

For sites that need custom extraction rules, use site configurations:

```bash
lectito https://example.com/article --config-dir /path/to/configs
```

## License

MPL-2.0

## Related Projects

- [Thunderus AI Agent](https://github.com/stormlightlabs/thunderus) - AI agent that uses Lectito as a fetch tool

- [Mccabre](https://github.com/stormlightlabs/mccabre) - Code analysis tool
