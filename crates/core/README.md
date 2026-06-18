# lectito

`lectito` is the Rust library crate for Lectito. It extracts readable article
content from HTML and returns cleaned HTML, Markdown, plain text, metadata, and
diagnostics.

Add it to a Rust project:

```toml
[dependencies]
lectito = "0.1"
```

Use the library through the `lectito` crate name:

```rust
use lectito::{extract, ReadabilityOptions};

let html = r#"<article><h1>Title</h1><p>Article text.</p></article>"#;
let article = extract(html, Some("https://example.com/post"), &ReadabilityOptions::default())?;

if let Some(article) = article {
    println!("{}", article.markdown);
}
# Ok::<(), lectito::Error>(())
```

The crate does not fetch pages. Pass HTML from your own crawler, browser,
cache, test fixture, or application code.

Extraction starts with structured metadata and known article containers before
falling back to generic readability scoring.

Use `content_selector` when your caller already knows the article root, and use
`extract_with_diagnostics` when you need to debug root selection.
