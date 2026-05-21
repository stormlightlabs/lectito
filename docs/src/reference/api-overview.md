# API Overview

Public exports from `lectito_core`:

The crate exposes the extraction API, output structs, diagnostics, errors, and
Markdown helpers. Internal parser, scoring, cleanup, and recovery modules remain
private.

```rust
pub use config::{Article, ReadabilityOptions, ReadableOptions};
pub use diagnostics::{
    AttemptDiagnostic, CandidateDiagnostic, CandidateSelection,
    CleanupDiagnostic, ContentSelectorDiagnostic, ExtractionDiagnostics,
    ExtractionOutcome, ExtractionReport, FlagDiagnostic, NodeDiagnostic,
    RecoveryDiagnostic,
};
pub use error::Error;
pub use extract::{extract, extract_with_diagnostics};
pub use markdown::{html_to_markdown, markdown_with_toml_frontmatter};
pub use readable::is_probably_readable;
```

## Extraction

Use `extract` for normal application code.

```rust
pub fn extract(
    html: &str,
    base_url: Option<&str>,
    options: &ReadabilityOptions,
) -> Result<Option<Article>, Error>
```

Returns `Ok(Some(article))` when content is found, `Ok(None)` when the document
has no useful article content, and `Err` for invalid input or processing
failures.

Use `extract_with_diagnostics` when you need extraction details in addition to
the article.

```rust
pub fn extract_with_diagnostics(
    html: &str,
    base_url: Option<&str>,
    options: &ReadabilityOptions,
) -> Result<ExtractionReport, Error>
```

Returns the same article result with extraction diagnostics.

## Readability Check

Use `is_probably_readable` before full extraction when you are filtering many
documents.

```rust
pub fn is_probably_readable(
    html: &str,
    options: &ReadableOptions,
) -> Result<bool, Error>
```

Returns a quick readability estimate without full extraction.

## Markdown

The Markdown helpers are available separately for callers that already have a
clean HTML fragment or want CLI-style frontmatter.

```rust
pub fn html_to_markdown(html: &str) -> String
```

Converts HTML fragments to Markdown.

```rust
pub fn markdown_with_toml_frontmatter(
    article: &Article,
    source: Option<&str>,
) -> Result<String, Error>
```

Formats an article as Markdown with TOML frontmatter.
