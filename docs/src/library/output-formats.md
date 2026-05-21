# Output Formats

Lectito produces all output formats during extraction.

The formats come from the same cleaned article root. That means callers can
store HTML for fidelity, use Markdown for display or editing, and use plain text
for search without running extraction multiple times.

```rust
let article = extract(html, base_url, &ReadabilityOptions::default())?.unwrap();

let html = article.content;
let markdown = article.markdown;
let text = article.text_content;
```

## HTML

`content` is cleaned article HTML. Scripts, styles, navigation, sidebars, and
other page chrome are removed where possible. Relative URLs are resolved when a
base URL is provided.

Use HTML when you need the closest representation of the extracted article. It
keeps images, links, tables, inline markup, and other structure that can be lost
in plain text.

## Markdown

`markdown` is generated from the cleaned article HTML. It preserves common
reader content:

- headings
- paragraphs
- links and images
- lists
- blockquotes
- code blocks
- tables
- math
- footnotes

The CLI Markdown output includes TOML frontmatter:

```sh
lectito parse article.html --format markdown
```

Markdown is useful when the next step is a reader view, note-taking system,
static archive, or editor. It is also easier to diff in tests than HTML.

## Plain Text

`text_content` is normalized article text. Use it for indexing, previews, and
readability checks.

Plain text should not be treated as a rendering format. It discards links,
images, and most document structure.

## JSON

The CLI can serialize the article:

```sh
lectito parse article.html --format json --pretty
```

JSON is the best CLI format when another program needs metadata and content
together.
