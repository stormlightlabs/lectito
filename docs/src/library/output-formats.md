# Output Formats

Work with different output formats: Markdown, JSON, text, and HTML.

## Overview

The `Article` struct provides methods for converting to different formats:

| Method          | Format                    | Requires Feature |
| --------------- | ------------------------- | ---------------- |
| `to_markdown()` | Markdown with frontmatter | `markdown`       |
| `to_json()`     | Structured JSON           | Always available |
| `to_text()`     | Plain text                | Always available |
| `content` field | Cleaned HTML              | Always available |

## Markdown

Convert article to Markdown with YAML frontmatter:

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

### Output Format

```markdown
+++
title = "Article Title"
author = "John Doe"
published_date = "2025-01-17"
excerpt = "A brief description of the article"
word_count = 500
+++

# Article Title

Article content here...

Paragraph with **bold** and _italic_ text.
```

### Customizing Markdown

Use `MarkdownFormatter` for more control:

```rs
use lectito_core::{parse, MarkdownFormatter, MarkdownConfig};

fn main() -> Result<(), Box<dyn std::error::Error>> {
    let html = "<html>...</html>";
    let article = parse(html)?;

    let config = MarkdownConfig {
        frontmatter: true,
        // Add more options as available
    };

    let formatter = MarkdownFormatter::new(config);
    let markdown = formatter.format(&article)?;

    println!("{}", markdown);

    Ok(())
}
```

## JSON

Get structured JSON with all metadata:

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
    "metadata": {
        "title": "Article Title",
        "author": "John Doe",
        "published_date": "2025-01-17",
        "excerpt": "A brief description",
        "language": "en"
    },
    "content": "<div>Cleaned HTML content...</div>",
    "text_content": "Plain text content...",
    "word_count": 500,
    "readability_score": 35.5
}
```

### Parsing JSON

```rs
use lectito_core::parse;
use serde_json::Value;

fn main() -> Result<(), Box<dyn std::error::Error>> {
    let html = "<html>...</html>";
    let article = parse(html)?;

    let json = article.to_json()?;
    let value: Value = serde_json::from_str(&json)?;

    println!("Title: {}", value["metadata"]["title"]);

    Ok(())
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

### Output Format

Plain text includes:

- Headings as lines with `#` prefixes
- Paragraphs separated by blank lines
- List items with `*` or `1.` prefixes
- No HTML tags or markdown syntax

## HTML

Access the cleaned HTML directly:

```rs
use lectito_core::parse;

fn main() -> Result<(), Box<dyn std::error::Error>> {
    let html = "<html>...</html>";
    let article = parse(html)?;

    // Cleaned HTML is in the `content` field
    let cleaned_html = &article.content;
    println!("{}", cleaned_html);

    Ok(())
}
```

### HTML Characteristics

The cleaned HTML:

- Removes clutter (navigation, sidebars, ads)
- Keeps main content structure
- Preserves images (if `preserve_images` is true)
- Removes most scripts and styles
- Maintains heading hierarchy

## Choosing a Format

| Format       | Use Case                                |
| ------------ | --------------------------------------- |
| **Markdown** | Blog posts, documentation, static sites |
| **JSON**     | APIs, databases, further processing     |
| **Text**     | Analysis, indexing, simple display      |
| **HTML**     | Web display, further HTML processing    |

## Format Conversion Examples

### Markdown to File

```rs
use lectito_core::parse;
use std::fs;

fn main() -> Result<(), Box<dyn std::error::Error>> {
    let html = "<html>...</html>";
    let article = parse(html)?;

    let markdown = article.to_markdown()?;
    fs::write("article.md", markdown)?;

    Ok(())
}
```

### JSON for API Response

```rs
use lectito_core::parse;
use warp::Filter;

async fn extract_article(body: String) -> Result<impl warp::Reply, warp::Rejection> {
    let article = parse(&body).unwrap();
    let json = article.to_json().unwrap();
    Ok(warp::reply::json(&json))
}
```

### Text for Analysis

```rs
use lectito_core::parse;

fn analyze_text(html: &str) -> Result<(), Box<dyn std::error::Error>> {
    let article = parse(html)?;
    let text = article.to_text();

    // Analyze word frequency
    let words: Vec<&str> = text.split_whitespace().collect();
    println!("Word count: {}", words.len());

    // Count sentences
    let sentences = text.split(&['.', '!', '?'][..]).count();
    println!("Sentence count: {}", sentences);

    Ok(())
}
```

### HTML for Display

```rs
use lectito_core::parse;

fn display_article(html: &str) -> Result<(), Box<dyn std::error::Error>> {
    let article = parse(html)?;

    // Use in a template
    let rendered = format!(
        r#"
        <!DOCTYPE html>
        <html>
        <head>
            <title>{}</title>
        </head>
        <body>
            <article>{}</article>
        </body>
        </html>
        "#,
        article.metadata.title.unwrap_or_default(),
        article.content
    );

    Ok(())
}
```

## Next Steps

- [Configuration](configuration.md) - Advanced configuration options
- [Basic Usage](basic-usage.md) - Core usage patterns
- [Concepts](../concepts/how-it-works.md) - Understanding the algorithm
