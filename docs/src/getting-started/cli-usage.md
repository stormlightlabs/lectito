# CLI Usage

The CLI is designed for inspecting extraction behavior and converting documents
from the terminal. It is also useful for building fixtures because the same
binary can print article output and diagnostics.

The CLI has three commands:

- `parse`: extract article content
- `readable`: check whether a document looks readable
- `fixture`: inspect bundled fixtures

## Parse

`parse` accepts one input source. Use a positional file path, `--input`,
`--stdin`, or `--url`.

```sh
lectito parse article.html
lectito parse --input article.html
lectito parse --stdin < article.html
lectito parse --url https://example.com/article
```

Output formats:

JSON is the default because it preserves the whole article structure. Use
Markdown or text when piping into another tool.

```sh
lectito parse article.html --format json --pretty
lectito parse article.html --format html
lectito parse article.html --format markdown
lectito parse article.html --format text
```

Useful options:

The defaults work for most article pages. Tune these flags when a page is too
short, too broad, or has a known content container.

```sh
lectito parse article.html --char-threshold 800
lectito parse article.html --nb-top-candidates 8
lectito parse article.html --content-selector article
lectito parse article.html --url https://example.com/post --site-profile example.com.toml
lectito parse article.html --max-elems-to-parse 10000
lectito parse article.html --keep-classes --classes-to-preserve language-rust
```

`--site-profile` can be repeated. Each file must be a TOML site profile. User
profiles take precedence over bundled profiles for the same host.

Diagnostics are written to stderr after the main output:

This keeps stdout usable for the extracted article while still showing debug
information in the terminal.

```sh
lectito parse article.html --format markdown --diagnostic-format pretty
lectito parse article.html --diagnostic-format json
```

## Readable

`readable` checks whether the document appears to contain enough article-like
text. It does not return extracted content.

```sh
lectito readable article.html
lectito readable --stdin < article.html
lectito readable --url https://example.com/article
lectito readable article.html --json --pretty
```

Thresholds:

```sh
lectito readable article.html --min-content-length 140 --min-score 20
```
