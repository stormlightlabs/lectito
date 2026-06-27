# Lectito Reference

Use this reference only when article-reader tools are unavailable or when a
workspace needs to set them up.

## What Lectito Provides

Lectito extracts readable article content from HTML. It can return cleaned HTML,
Markdown, plain text, metadata, and diagnostics. It is useful as the backend for
article-reading tools because it keeps extraction deterministic and separate
from model reasoning.

## CLI Use

Install the CLI from crates.io when working outside the repo:

```sh
cargo install lectito-cli
```

Use the local workspace binary when working inside this repo:

```sh
cargo run -p lectito-cli -- https://example.com/article --format markdown
```

Useful formats:

```sh
lectito https://example.com/article --format markdown
lectito https://example.com/article --format text
lectito https://example.com/article --format json --pretty
lectito inspect https://example.com/article --json --pretty
```

## MCP Backend Expectations

An MCP-backed article reader should expose the Lectito-powered extraction behind
the `read_article` tool. The skill should not ask the model to reimplement
article extraction or manually scrape page chrome.

Expected `read_article` behavior:

- Accept an absolute `http` or `https` URL.
- Fetch the page with redirect, timeout, and response-size limits.
- Reject private-network targets by default.
- Run Lectito extraction.
- Return title, byline, site name, published time, final URL, excerpt, content
  length, truncation state, and content.
- Support bounded reads with `offset` and `maxChars`.

Expected `search_articles` behavior:

- Search for candidate article URLs.
- Return title, URL, and snippet.
- Keep result limits small.

## Fallback Rules

If MCP tools are not available but the CLI is available, use the CLI to read a
specific URL. If only search is missing, use another approved search mechanism
to find URLs, then run Lectito on the chosen sources.

Do not install Lectito during normal article-reading tasks unless setup is
explicitly part of the request or the required tool/CLI is missing.
