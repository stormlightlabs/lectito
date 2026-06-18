# Quick Start

## Extract From HTML

Start with `extract` for normal use. It takes the source HTML, an optional base
URL, and `ReadabilityOptions`. The base URL lets Lectito resolve relative links,
images, and metadata URLs in the extracted output.

```rust
use lectito::{extract, ReadabilityOptions};

fn main() -> Result<(), lectito::Error> {
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
use lectito::{is_probably_readable, ReadableOptions};

let readable = is_probably_readable(html, &ReadableOptions::default())?;
# Ok::<(), lectito::Error>(())
```

## CLI

The CLI mirrors the library. The root command extracts content, and `readable`
performs the quick readability check.

```sh
lectito article.html
lectito https://example.com/article --json --pretty
lectito readable article.html
```
