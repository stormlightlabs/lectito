# Output Formats

Work with different output formats: Markdown, JSON, text, and HTML.

## Overview

The `Article` struct provides several ways to render extracted content:

| Method | Format | Requires Feature |
| ------ | ------ | ---------------- |
| `to_markdown()` | Markdown | `markdown` |
| `to_markdown_with_config()` | Markdown with custom options | `markdown` |
| `to_json()` | Serialized `Article` JSON | Always available |
| `to_text()` | Plain text | Always available |
| `content` field | Cleaned HTML | Always available |

## Markdown

Convert an article to Markdown:

```rs
use lectito_core::parse;

fn main() -> Result<(), Box<dyn std::error::Error>> {
    let html = "<html>...</html>";
    let article = parse(html)?;

    let markdown = article.to_markdown()?;
    println!("{}", markdown);

    Ok(())
}
```

### Markdown Configuration

Use `MarkdownConfig` for frontmatter, references, and image handling:

```rs
use lectito_core::{parse, MarkdownConfig};

fn main() -> Result<(), Box<dyn std::error::Error>> {
    let html = "<html>...</html>";
    let article = parse(html)?;

    let config = MarkdownConfig {
        include_frontmatter: true,
        include_references: true,
        strip_images: false,
        include_title_heading: true,
    };

    let markdown = article.to_markdown_with_config(&config)?;
    println!("{}", markdown);

    Ok(())
}
```

### Frontmatter Fields

When `include_frontmatter` is enabled, Lectito can emit fields such as:

```toml
+++
title = "Article Title"
author = "John Doe"
date = "2025-01-17"
site = "Example"
image = "https://example.com/image.jpg"
favicon = "https://example.com/favicon.ico"
excerpt = "A brief description of the article"
word_count = 500
reading_time_minutes = 2.5
+++
```

## JSON

`Article::to_json()` returns a serialized view of the article itself:

```rs
use lectito_core::parse;

fn main() -> Result<(), Box<dyn std::error::Error>> {
    let html = "<html>...</html>";
    let article = parse(html)?;

    let json = article.to_json()?;
    println!("{}", json);

    Ok(())
}
```

### JSON Structure

```json
{
  "content": "<div>Cleaned HTML content...</div>",
  "text_content": "Plain text content...",
  "metadata": {
    "title": "Article Title",
    "author": "John Doe",
    "date": "2025-01-17",
    "excerpt": "A brief description",
    "site_name": "Example",
    "language": "en"
  },
  "length": 1234,
  "word_count": 500,
  "reading_time": 2.5,
  "source_url": "https://example.com/article",
  "confidence": 0.92
}
```

## Plain Text

Extract just the text content:

```rs
use lectito_core::parse;

fn main() -> Result<(), Box<dyn std::error::Error>> {
    let html = "<html>...</html>";
    let article = parse(html)?;

    let text = article.to_text();
    println!("{}", text);

    Ok(())
}
```

Plain text preserves the readable text content without HTML tags.

## HTML

Access the cleaned HTML directly:

```rs
use lectito_core::parse;

fn main() -> Result<(), Box<dyn std::error::Error>> {
    let html = "<html>...</html>";
    let article = parse(html)?;

    let cleaned_html = &article.content;
    println!("{}", cleaned_html);

    Ok(())
}
```

The cleaned HTML:

- removes clutter such as navigation and ads
- keeps the main content structure
- preserves images when `preserve_images` is enabled
- preserves supported embeds when `preserve_video_embeds` is enabled

## Choosing a Format

| Format | Use Case |
| ------ | -------- |
| **Markdown** | Blog posts, docs, static publishing |
| **JSON** | APIs, storage, downstream processing |
| **Text** | Analysis, indexing, search |
| **HTML** | Web display or further HTML processing |
