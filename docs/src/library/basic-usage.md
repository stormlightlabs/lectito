# Basic Usage

Use `extract` when you want article content.

The function does not fetch the page. Pass it the HTML you want parsed. This is
usually cleaner in applications because networking, caching, cookies, and
browser rendering are application concerns.

```rust
use lectito::{extract, ReadabilityOptions};

let options = ReadabilityOptions::default();
let article = extract(html, Some("https://example.com/post"), &options)?;

match article {
    Some(article) => println!("{}", article.text_content),
    None => eprintln!("no article content found"),
}
# Ok::<(), lectito::Error>(())
```

The base URL is optional. Pass it when the document contains relative links,
images, or metadata URLs.

When extraction succeeds, Lectito returns `Some(Article)`. When the page parses
but does not contain a useful article, it returns `None`. Reserve error handling
for invalid base URLs, configured size limits, and serialization failures.

## Article Output

`Article` contains the extracted content in several forms:

```rust
if let Some(article) = article {
    println!("{}", article.content);
    println!("{}", article.markdown);
    println!("{}", article.text_content);
}
```

Use `extract_with_diagnostics` when you need to see how extraction chose a root.
Diagnostics are meant for development and regression work. Most application code
should call `extract`.

```rust
use lectito::{extract_with_diagnostics, ReadabilityOptions};

let report = extract_with_diagnostics(html, base_url, &ReadabilityOptions::default())?;

if let Some(article) = report.article {
    println!("{}", article.markdown);
}

eprintln!("{:?}", report.diagnostics.outcome);

# Ok::<(), lectito::Error>(())
```
