# Basic Usage

Learn the fundamentals of using Lectito as a library.

## Simple Parsing

The easiest way to extract content is with the `parse` function:

```rs
use lectito_core::{parse, Article};

fn main() -> Result<(), Box<dyn std::error::Error>> {
    let html = r#"
        <!DOCTYPE html>
        <html>
            <head><title>My Article</title></head>
            <body>
                <article>
                    <h1>Article Title</h1>
                    <p>This is the article content.</p>
                </article>
            </body>
        </html>
    "#;

    let article: Article = parse(html)?;

    println!("Title: {:?}", article.metadata.title);
    println!("Confidence: {:.2}", article.confidence);
    println!("Content: {}", article.to_markdown()?);

    Ok(())
}
```

## Fetching and Parsing

For URLs, use the `fetch_and_parse` function:

```rs
use lectito_core::fetch_and_parse;

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    let url = "https://example.com/article";
    let article = fetch_and_parse(url).await?;

    println!("Title: {:?}", article.metadata.title);
    println!("Author: {:?}", article.metadata.author);
    println!("Word count: {}", article.word_count);

    Ok(())
}
```

This helper requires the `fetch` feature.

## Working with the Article

The `Article` struct contains the extracted content, metadata, and derived metrics.

### Metadata

```rs
use lectito_core::parse;

let html = "<html>...</html>";
let article = parse(html)?;

if let Some(title) = article.metadata.title {
    println!("Title: {}", title);
}

if let Some(author) = article.metadata.author {
    println!("Author: {}", author);
}

if let Some(date) = article.metadata.date {
    println!("Published: {}", date);
}

if let Some(excerpt) = article.metadata.excerpt {
    println!("Excerpt: {}", excerpt);
}
```

### Content Access

```rs
use lectito_core::parse;

let html = "<html>...</html>";
let article = parse(html)?;

let html_content = &article.content;
let text = article.to_text();
let markdown = article.to_markdown()?;
let json = article.to_json()?;
```

`to_markdown()` requires the `markdown` feature.

## Readability API

For more control, use the `Readability` API:

```rs
use lectito_core::{Readability, ReadabilityConfig};

fn main() -> Result<(), Box<dyn std::error::Error>> {
    let html = "<html>...</html>";

    let reader = Readability::new();
    let article = reader.parse(html)?;

    let config = ReadabilityConfig::builder()
        .min_score(25.0)
        .char_threshold(500)
        .nb_top_candidates(8)
        .build();

    let reader = Readability::with_config(config);
    let article = reader.parse(html)?;

    Ok(())
}
```

## Error Handling

Lectito returns `Result<T, LectitoError>`. Handle errors appropriately:

```rs
use lectito_core::{parse, LectitoError};

fn extract_article(html: &str) -> Result<String, String> {
    match parse(html) {
        Ok(article) => Ok(article.to_markdown().unwrap_or_default()),
        Err(LectitoError::NotReadable { score, threshold }) => {
            Err(format!("Content not readable: score {} < threshold {}", score, threshold))
        }
        Err(LectitoError::InvalidUrl(msg)) => {
            Err(format!("Invalid URL: {}", msg))
        }
        Err(e) => Err(format!("Extraction failed: {}", e)),
    }
}
```

## Common Patterns

### Parse with URL Context

When you have the URL, provide it for better relative link resolution:

```rs
use lectito_core::{parse_with_url, Article};

fn main() -> Result<(), Box<dyn std::error::Error>> {
    let html = "<html>...</html>";
    let url = "https://example.com/article";

    let article: Article = parse_with_url(html, url)?;

    assert_eq!(article.source_url.as_deref(), Some(url));
    Ok(())
}
```

### Check if Content is Probably Readable

For a quick pre-check:

```rs
use lectito_core::is_probably_readable;

fn main() {
    let html = "<html>...</html>";

    if is_probably_readable(html) {
        println!("Content looks readable");
    } else {
        println!("Content may not be readable");
    }
}
```

### Working with Documents

For lower-level DOM manipulation:

```rs
use lectito_core::{Document, Element};

fn main() -> Result<(), Box<dyn std::error::Error>> {
    let html = "<html><body><p>Hello</p></body></html>";

    let doc = Document::parse(html)?;
    let elements: Vec<Element> = doc.select("p")?;

    for element in elements {
        println!("Text: {}", element.text());
    }

    Ok(())
}
```
