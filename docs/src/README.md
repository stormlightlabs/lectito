# Lectito

A Rust library and CLI for extracting readable content from web pages.

## What is Lectito?

Lectito implements a content extraction algorithm inspired by Mozilla's Readability.js.
It identifies and extracts the main article content from web pages, removing navigation, sidebars, advertisements, and other clutter.

## Features

- **Content Extraction**: Automatically identifies the main article content
- **Metadata Extraction**: Pulls title, author, date, excerpt, and language
- **Output Formats**: HTML, Markdown, plain text, and JSON
- **URL Fetching**: Built-in async HTTP client with timeout support
- **CLI**: Simple command-line interface for quick extractions
- **Site Configuration**: Optional XPath-based extraction rules for difficult sites

## Use Cases

- **Web Scraping**: Extract clean article content from web pages
- **AI Agents**: Feed readable text to language models
- **Content Analysis**: Analyze article text without HTML noise
- **Archival**: Save clean copies of web content
- **CLI**: Quick article extraction from the terminal

## Quick Links

- **Installation**: See the [Installation Guide](getting-started/installation.md)
- **CLI Usage**: See the [CLI Usage Guide](getting-started/cli-usage.md)
- **Library Usage**: See the [Basic Usage Guide](library/basic-usage.md)
- **API Reference**: ~~See [docs.rs/lectito](https://docs.rs/lectito)~~

## Quick Start

### CLI

```bash
# Install
cargo install lectito-cli

# Extract from URL
lectito https://example.com/article

# Extract from local file
lectito article.html

# Pipe from stdin
curl https://example.com | lectito -
```

### Library

```rs
use lectito_core::parse;

let html = r#"<html><body><article><h1>Title</h1><p>Content</p></article></body></html>"#;
let article = parse(html)?;

println!("Title: {:?}", article.metadata.title);
println!("Content: {}", article.to_markdown()?);
```

## About the Name

"Lectito" is derived from the Latin *legere* (to read) and *lectio* (a reading or selection).

Lectito aims to select and present readable content from the chaos of the modern web.
