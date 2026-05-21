# Quick Start

## Extract From HTML

Start with `extract` for normal use. It takes the source HTML, an optional base
URL, and `ReadabilityOptions`. The base URL lets Lectito resolve relative links,
images, and metadata URLs in the extracted output.

```rust
use lectito_core::{extract, ReadabilityOptions};

fn main() -> Result<(), lectito_core::Error> {
    let html = r#"
        <html>
          <head><title>Example</title></head>
          <body>
            <article>
              <h1>Example</h1>
              <p>This is the article body.</p>
            </article>
          </body>
        </html>
    "#;

    let article = extract(html, Some("https://example.com/article"), &ReadabilityOptions::default())?;

    if let Some(article) = article {
        println!("{:?}", article.title);
        println!("{}", article.markdown);
    }

    Ok(())
}
```

`extract` returns `Ok(None)` when no useful article content is found.
That is different from an error. An empty or navigation-only page can be parsed
successfully and still have no article.

## Check Readability

Use `is_probably_readable` when you only need to decide whether a page is worth
running through full extraction. It is faster and returns a boolean.

```rust
use lectito_core::{is_probably_readable, ReadableOptions};

let readable = is_probably_readable(html, &ReadableOptions::default())?;
# Ok::<(), lectito_core::Error>(())
```

## CLI

The CLI mirrors the library. `parse` extracts content, and `readable` performs
the quick readability check.

```sh
lectito parse article.html --format markdown
lectito parse --url https://example.com/article --format json --pretty
lectito readable article.html
```
