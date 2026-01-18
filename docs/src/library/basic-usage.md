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

## Working with the Article

The `Article` struct contains all extracted information:

### Metadata

```rs
use lectito_core::parse;

let html = "<html>...</html>";
let article = parse(html)?;

// Access metadata
if let Some(title) = article.metadata.title {
    println!("Title: {}", title);
}

if let Some(author) = article.metadata.author {
    println!("Author: {}", author);
}

if let Some(date) = article.metadata.published_date {
    println!("Published: {}", date);
}

// Get excerpt
if let Some(excerpt) = article.metadata.excerpt {
    println!("Excerpt: {}", excerpt);
}
```

### Content Access

```rs
use lectito_core::parse;

let html = "<html>...</html>";
let article = parse(html)?;

// Get cleaned HTML
let html_content = &article.content;

// Get plain text
let text = article.to_text();

// Get Markdown
let markdown = article.to_markdown()?;

// Get JSON
let json = article.to_json()?;
```

## Readability API

For more control, use the `Readability` API:

```rs
use lectito_core::{Readability, ReadabilityConfig};

fn main() -> Result<(), Box<dyn std::error::Error>> {
    let html = "<html>...</html>";

    // Use default config
    let reader = Readability::new();
    let article = reader.parse(html)?;

    // Or with custom config
    let config = ReadabilityConfig::builder()
        .min_score(25.0)
        .char_threshold(500)
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
        Err(LectitoError::NotReaderable { score, threshold }) => {
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

    // Relative links are now resolved correctly
    Ok(())
}
```

### Check if Content is Readable

Before parsing, check if content meets readability thresholds:

```rs
use lectito_core::is_probably_readable;

fn main() -> Result<(), Box<dyn std::error::Error>> {
    let html = "<html>...</html>";

    if is_probably_readable(html) {
        println!("Content is readable");
    } else {
        println!("Content may not be readable");
    }

    Ok(())
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

## Integrations

### With reqwest

```rs
use lectito_core::parse;
use reqwest::Client;

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    let client = Client::new();
    let response = client.get("https://example.com/article")
        .send()
        .await?;

    let html = response.text().await?;
    let article = parse(&html)?;

    println!("Title: {:?}", article.metadata.title);

    Ok(())
}
```

### With Scraper

If you're already using `scraper`, you can integrate:

```rs
use lectito_core::parse;

fn main() -> Result<(), Box<dyn std::error::Error>> {
    let html = "<html>...</html>";
    let article = parse(html)?;

    // Work with the article's HTML content
    println!("Cleaned HTML: {}", article.content);

    Ok(())
}
```

## Next Steps

- [Configuration](configuration.md) - Advanced configuration options
- [Async vs Sync](async-vs-sync.md) - Understanding async APIs
- [Output Formats](output-formats.md) - Detailed format documentation
