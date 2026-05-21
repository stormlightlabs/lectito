# Lectito

Lectito is a Rust library and CLI tool for extracting readable article content
from HTML.

Most web pages contain way more than the text a reader came for, like ads,
navigation, related links, comment areas, tracking markup, hidden elements, and
presentation wrappers. Lectito tries to identify the main content root and
return a smaller document that is useful for reading, storage, search, and
conversion.

It returns:

- cleaned article HTML
- Markdown
- plain text
- page metadata
- extraction diagnostics

Lectito is parser-first. The core API accepts HTML and an optional base URL. URL
fetching exists in the CLI for convenience, but the library does not require
network access.

This keeps the library usable in environments that already have HTML available:
crawlers, browser extensions, desktop apps, mobile apps, tests, and offline
archives.

## Main APIs

```rust
use lectito::{extract, ReadabilityOptions};

let html = r#"<article><h1>Title</h1><p>Article text.</p></article>"#;
let article = extract(html, Some("https://example.com/post"), &ReadabilityOptions::default())?;

if let Some(article) = article {
    println!("{}", article.markdown);
}
# Ok::<(), lectito::Error>(())
```

Use `extract_with_diagnostics` when tuning extraction or debugging a bad page.
Use `is_probably_readable` before extraction when you only need a quick yes/no
answer.

## Project Scope

The public API is intentionally small. Callers should depend on the article
result, options, diagnostics, and Markdown helpers rather than internal scoring
or cleanup modules.
