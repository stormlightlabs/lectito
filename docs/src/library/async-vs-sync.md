# Async vs Sync

Understanding Lectito's async and synchronous APIs.

## Overview

Lectito provides both synchronous and asynchronous APIs:

| Function            | Async/Sync | Use Case                  |
| ------------------- | ---------- | ------------------------- |
| `parse()`           | Sync       | Parse HTML from string    |
| `parse_with_url()`  | Sync       | Parse with URL context    |
| `fetch_and_parse()` | Async      | Fetch from URL then parse |
| `fetch_url()`       | Async      | Fetch HTML from URL       |

## When to Use Each

### Use Sync APIs When

- You already have the HTML as a string
- You're using your own HTTP client
- Performance is not critical
- You're integrating into synchronous code

### Use Async APIs When

- You need to fetch from URLs
- You're already using async/await
- You want concurrent fetches
- Performance matters for network operations

## Synchronous Parsing

Parse HTML that you already have:

```rs
use lectito_core::parse;

fn main() -> Result<(), Box<dyn std::error::Error>> {
    let html = "<html>...</html>";
    let article = parse(html)?;
    Ok(())
}
```

## Asynchronous Fetching

Fetch and parse in one operation:

```rs
use lectito_core::fetch_and_parse;

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    let url = "https://example.com/article";
    let article = fetch_and_parse(url).await?;
    Ok(())
}
```

## Manual Fetch and Parse

Use your own HTTP client:

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

    Ok(())
}
```

## Concurrent Fetches

Fetch multiple articles concurrently:

```rs
use lectito_core::fetch_and_parse;
use futures::future::join_all;

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    let urls = vec![
        "https://example.com/article1",
        "https://example.com/article2",
        "https://example.com/article3",
    ];

    let futures: Vec<_> = urls.into_iter()
        .map(|url| fetch_and_parse(url))
        .collect();

    let articles = join_all(futures).await;

    for article in articles {
        match article {
            Ok(a) => println!("Got: {:?}", a.metadata.title),
            Err(e) => eprintln!("Error: {}", e),
        }
    }

    Ok(())
}
```

## Batch Processing

Process URLs with concurrency limits:

```rs
use lectito_core::fetch_and_parse;
use futures::stream::{StreamExt, try_stream};

async fn process_urls(urls: Vec<String>) -> Result<(), Box<dyn std::error::Error>> {
    let stream = try_stream! {
        for url in urls {
            let article = fetch_and_parse(&url).await?;
            yield article;
        }
    };

    let mut stream = stream.buffer_unordered(5); // 5 concurrent requests

    while let Some(article) = stream.next().await {
        println!("Processed: {:?}", article?.metadata.title);
    }

    Ok(())
}
```

## Sync Code in Async Context

If you need to use sync parsing in async code:

```rs
use lectito_core::parse;

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    // Fetch with your async HTTP client
    let html = fetch_html().await?;

    // Parse is sync, but that's fine in async context
    let article = parse(&html)?;

    Ok(())
}

async fn fetch_html() -> Result<String, Box<dyn std::error::Error>> {
    // Your async fetching logic
    Ok(String::from("<html>...</html>"))
}
```

## Performance Considerations

### Parsing (Sync)

Parsing is CPU-bound and runs synchronously:

```rs
use lectito_core::parse;
use std::time::Instant;

fn main() -> Result<(), Box<dyn std::error::Error>> {
    let html = "<html>...</html>";

    let start = Instant::now();
    let article = parse(html)?;
    let duration = start.elapsed();

    println!("Parsed in {:?}", duration);

    Ok(())
}
```

### Fetching (Async)

Fetching is I/O-bound and benefits from async:

```rs
use lectito_core::fetch_and_parse;

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    let start = std::time::Instant::now();
    let article = fetch_and_parse("https://example.com/article").await?;
    let duration = start.elapsed();

    println!("Fetched and parsed in {:?}", duration);

    Ok(())
}
```

## Choosing the Right Approach

| Scenario             | Recommended Approach                              |
| -------------------- | ------------------------------------------------- |
| Have HTML string     | `parse()` (sync)                                  |
| Need to fetch URL    | `fetch_and_parse()` (async)                       |
| Custom HTTP client   | Your client + `parse()` (sync)                    |
| Batch URL processing | `fetch_and_parse()` with concurrent futures       |
| CLI tool             | Depends on your runtime setup                     |
| Web server           | `fetch_and_parse()` (async) for better throughput |

## Feature Flags

To disable async features and reduce dependencies:

```toml
[dependencies]
lectito-core = { version = "0.1", default-features = false, features = ["markdown"] }
```

This removes `reqwest` and `tokio` dependencies. You'll need to fetch HTML yourself.

## Next Steps

- [Output Formats](output-formats.md) - Working with different output formats
- [Configuration](configuration.md) - Advanced configuration options
- [Basic Usage](basic-usage.md) - Core usage patterns
