# lectito-cli

`lectito-cli` is the command-line package for Lectito. It installs a `lectito`
binary backed by the `lectito` Rust library.

Install it from crates.io:

```sh
cargo install lectito-cli
```

Example commands:

```sh
lectito article.html
lectito https://example.com/article --json --pretty
lectito at://did:plc:abc123/site.standard.document/xyz
lectito article.html --html
lectito readable article.html
lectito inspect article.html
lectito article.html --timeout 10
```

The CLI can read from files, stdin, URLs, and renderable Standard.site document
AT URIs. URL fetching and ATProto resolution are CLI conveniences. The
`lectito` library accepts HTML supplied by the caller.

For web pages that advertise `rel="site.standard.document"`, the CLI uses the
linked ATProto record when it can render it. Otherwise it extracts the fetched
HTML.

Markdown with TOML frontmatter is the default output.

Use `--html`, `--text`, or `--json` when another format fits better.
Use `--inspect` or `--diagnostic-format pretty` when tuning extraction for a page.
