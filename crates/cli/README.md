# lectito-cli

`lectito-cli` is the command-line package for Lectito. It installs a `lectito`
binary backed by the `lectito` Rust library.

Install it from crates.io:

```sh
cargo install lectito-cli
```

Install with the optional PDF renderer when you want to export as PDF with
`--format pdf`:

```sh
cargo install lectito-cli --features pdf
```

Example commands:

```sh
lectito article.html
lectito https://example.com/article --format json --pretty
lectito at://did:plc:abc123/site.standard.document/xyz
lectito article.html --format html
lectito article.html --format pdf --output article.pdf
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

Use `--format html`, `--format text`, or `--format json` when another format
fits better.
Use `--format pdf` after installing with `--features pdf`. PDF output always
writes a file and prints the path. Without `--output`, the file is named
`{hash}.pdf` from the generated PDF contents.
Use `--frontmatter=false` to omit Markdown frontmatter.
Use `--inspect` or `--diagnostic-format pretty` when tuning extraction for a page.

The PDF feature converts the extracted article Markdown into a readable PDF
with built-in fonts. It does not edit existing PDFs, merge files, extract pages,
or expose low-level PDF manipulation commands.
