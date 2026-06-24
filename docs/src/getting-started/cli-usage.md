# CLI Usage

The CLI is designed for inspecting extraction behavior and converting documents
from the terminal.

The root command extracts article content. The CLI also has these subcommands:

- `readable`: check whether a document looks readable
- `inspect`: print extraction metadata and scoring details
- `llms`: fetch, parse, and expand `llms.txt` files

## Extract

Pass a URL, a file path, or `-` for stdin. Markdown with TOML frontmatter is
the default output.

```sh
lectito article.html
lectito https://example.com/article
lectito - < article.html
```

Output formats:

Use HTML, text, or JSON when Markdown is not the right output for the next
tool.

```sh
lectito article.html --html
lectito article.html --text
lectito article.html --json --pretty
lectito article.html --no-frontmatter
lectito article.html --output article.md
```

Useful options:

The defaults work for most article pages. Tune these flags when a page is too
short, too broad, or has a known content container.

```sh
lectito article.html --char-threshold 800
lectito article.html --nb-top-candidates 8
lectito article.html --content-selector article
lectito article.html --base-url https://example.com/post --site-profile example.com.toml
lectito article.html --max-elems-to-parse 10000
lectito article.html --media article
lectito article.html --media none
lectito article.html --keep-classes --preserve-class language-rust
```

`--content-selector` is the strongest extraction hint. Use it when you know the
article root for a page or fixture. Without that flag, the CLI still tries
common article-body containers before falling back to generic scoring.

`--media` accepts `none`, `conservative`, `article`, or `all`. The default is
`article`, which keeps figures/images that appear to be part of the article body.

`--site-profile` can be repeated. Each file must be a TOML site profile. User
profiles take precedence over bundled profiles for the same host.

`--disable-json-ld` turns off JSON-LD metadata extraction and the JSON-LD
article-body fast path. Use it when structured data is stale or misleading.

Diagnostics are written to stderr after the main output to keep keep stdout usable
for the extracted article while still showing debug information in the terminal.

```sh
lectito article.html --diagnostic-format pretty
lectito article.html --diagnostic-format json
```

`--inspect` prints a compact extraction summary to stderr while keeping article
output on stdout:

```sh
lectito article.html --inspect
```

Full extraction has a timeout so unusually large or hostile pages do not hang
the command:

```sh
lectito article.html --timeout 10
```

## Readable

`readable` checks whether the document appears to contain enough article-like
text. It does not return extracted content.

```sh
lectito readable article.html
lectito readable --stdin < article.html
lectito readable https://example.com/article
lectito readable article.html --json --pretty
lectito article.html --readable
```

Thresholds:

```sh
lectito readable article.html --min-content-length 140 --min-score 20
```

## Inspect

`inspect` prints extraction metadata and scoring details without printing the
article body.

```sh
lectito inspect article.html
lectito inspect https://example.com/article
lectito inspect article.html --json --pretty
```

## llms.txt

Use the `llms` subcommands when a site publishes an `llms.txt` file or when
you want to bundle its linked resources into one Markdown context file.

```sh
lectito llms fetch https://example.com
lectito llms parse https://example.com/llms.txt --pretty
lectito llms expand https://example.com/llms.txt --output llms-full.txt
```

`fetch` resolves a bare site URL to `/llms.txt`. `parse` prints structured JSON.
`expand` reads the linked resources, keeps Markdown resources as-is, and runs
HTML resources through Lectito before adding them to the bundle.

Links in the special `Optional` section are skipped unless you pass
`--include-optional`:

```sh
lectito llms expand https://example.com/llms.txt --include-optional
```

See the [llms.txt guide](./llms-txt.md) for the expected file shape and the
tradeoffs.

## Exit Codes

- `0`: article extracted, or readability check returned true
- `1`: no article was extracted, or readability check returned false
- `2`: input, file, or network error
- `3`: extraction, configuration, or extraction timeout error
