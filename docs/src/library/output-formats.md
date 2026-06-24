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

Markdown cleanup also strips zero-width break hints, drops empty links, keeps
images intact, and removes duplicate title headings before rendering.

The CLI Markdown output includes TOML frontmatter:

```sh
lectito article.html
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
lectito article.html --format json --pretty
```

JSON is the best CLI format when another program needs metadata and content
together.

## Quality Expectations

| Output     | Best use                                        | Expect                                                                                                           | Do not expect                                                                   |
| ---------- | ----------------------------------------------- | ---------------------------------------------------------------------------------------------------------------- | ------------------------------------------------------------------------------- |
| Markdown   | Reader views, notes, archives, editing          | Good preservation of headings, paragraphs, links, images, lists, blockquotes, code, tables, math, and footnotes. | Byte-for-byte source fidelity or every custom widget.                           |
| HTML       | Rendering or post-processing extracted articles | The closest structural view of the cleaned article root, with links and media kept according to options.         | A complete sanitizer policy or the original page layout.                        |
| Text       | Search, previews, indexing, basic summaries     | Normalized article text with block boundaries for headings, paragraphs, lists, code, and definition lists.       | A rich rendering format with links, images, or full table structure.            |
| JSON       | Programmatic CLI integrations                   | Metadata plus HTML, Markdown, text, length, and source-related fields in one object.                             | Stable values for publisher metadata when source pages disagree or omit fields. |
| `inspect`  | Debugging extraction choices                    | Selected root, candidate scores, cleanup counts, recovery data, and site-rule information.                       | A user-facing article format.                                                   |
| `readable` | Cheap filtering before full extraction          | A boolean estimate using text length, visibility, class/id hints, and link density.                              | The same answer full extraction would produce on every borderline page.         |
