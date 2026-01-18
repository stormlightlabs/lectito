# Quick Start

Get started with Lectito in minutes.

## CLI Quick Start

### Basic Usage

Extract content from a URL:

```bash
lectito https://example.com/article
```

Extract from a local file:

```bash
lectito article.html
```

Extract from stdin:

```bash
curl https://example.com | lectito -
```

### Save to File

```bash
lectito https://example.com/article -o article.md
```

### Change Output Format

```bash
# JSON output
lectito https://example.com/article --format json

# Plain text output
lectito https://example.com/article --format text
```

### Set Timeout

For slow-loading sites:

```bash
lectito https://example.com/article --timeout 60
```

## Library Quick Start

### Add Dependency

Add to `Cargo.toml`:

```toml
[dependencies]
lectito-core = "0.1"
```

### Parse HTML String

```rs
use lectito_core::parse;

fn main() -> Result<(), Box<dyn std::error::Error>> {
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
    println!("Content: {}", article.to_markdown()?);

    Ok(())
}
```

### Fetch and Parse URL

```rs
use lectito_core::fetch_and_parse;

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    let article = fetch_and_parse("https://example.com/article").await?;

    println!("Title: {:?}", article.metadata.title);
    println!("Word count: {}", article.word_count);

    Ok(())
}
```

### Convert to Different Formats

```rs
use lectito_core::parse;

fn main() -> Result<(), Box<dyn std::error::Error>> {
    let html = "<h1>Title</h1><p>Content here</p>";
    let article = parse(html)?;

    // Markdown with frontmatter
    let markdown = article.to_markdown()?;
    println!("{}", markdown);

    // Plain text
    let text = article.to_text();
    println!("{}", text);

    // Structured JSON
    let json = article.to_json()?;
    println!("{}", json);

    Ok(())
}
```

## Common Patterns

### Handle Errors

```rs
use lectito_core::{parse, LectitoError};

match parse("<html>...</html>") {
    Ok(article) => println!("Title: {:?}", article.metadata.title.unwrap()),
    Err(LectitoError::NotReaderable { score, threshold }) => {
        eprintln!("Content not readable: score {} < threshold {}", score, threshold);
    }
    Err(e) => eprintln!("Error: {}", e),
}
```

### Configure Extraction

```rs
use lectito_core::{Readability, ReadabilityConfig};

fn main() -> Result<(), Box<dyn std::error::Error>> {
    let config = ReadabilityConfig::builder()
        .min_score(25.0)
        .char_threshold(500)
        .preserve_images(true)
        .build();

    let reader = Readability::with_config(config);
    let article = reader.parse("<html>...</html>")?;

    Ok(())
}
```

## What's Next?

- [CLI Usage](cli-usage.md) - Full CLI command reference
- [Library Guide](../library/basic-usage.md) - In-depth library documentation
- [Configuration](../library/configuration.md) - Advanced configuration options
- [Concepts](../concepts/how-it-works.md) - How the algorithm works
