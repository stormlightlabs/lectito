# CLI Usage

Reference for the `lectito` command-line tool.

## Basic Syntax

```bash
lectito [OPTIONS] [INPUT]
```

`INPUT` can be:

- a URL starting with `http://` or `https://`
- a local file path
- `-` to read from stdin

## Common Examples

### Extract from a URL

```bash
lectito https://example.com/article
```

### Extract from a File

```bash
lectito article.html
```

### Read from stdin

```bash
curl https://example.com | lectito -
```

## Output Options

### `-o, --output <FILE>`

Write output to a file instead of stdout.

```bash
lectito https://example.com/article -o article.md
```

### `-f, --format <FORMAT>`

Output format. Available values:

| Format             | Description     |
| ------------------ | --------------- |
| `markdown` or `md` | Markdown output |
| `html`             | Cleaned HTML    |
| `text` or `txt`    | Plain text      |
| `json`             | Structured JSON |

```bash
lectito https://example.com/article --format text
```

### `--json`

Force structured JSON output regardless of `--format`.

```bash
lectito https://example.com/article --json
```

### `--references`

Include a reference table in Markdown output or a references array in JSON output.

```bash
lectito https://example.com/article --references
```

### `--frontmatter`

Include TOML frontmatter in Markdown output.

```bash
lectito https://example.com/article --frontmatter
```

### `-m, --metadata-only`

Output metadata only.

```bash
lectito https://example.com/article --metadata-only
```

### `--metadata-format <FORMAT>`

Metadata output format for `--metadata-only`. Supported values: `toml`, `json`.

```bash
lectito https://example.com/article --metadata-only --metadata-format json
```

## Extraction Options

### `--timeout <SECS>`

HTTP timeout in seconds. Default: `30`.

### `--user-agent <UA>`

Custom User-Agent for HTTP requests.

### `-c, --config-dir <DIR>`

Directory containing site configuration files.

### `--char-threshold <NUM>`

Minimum character threshold for content candidates. Default: `500`.

### `--max-elements <NUM>`

Maximum number of top candidates to track. Default: `5`.

### `--no-images`

Strip images from output.

### `-v, --verbose`

Enable verbose logging and timing output.

## Shell Completions

### `--completions <SHELL>`

Generate a completion script for `bash`, `zsh`, `fish`, or `powershell`.

```bash
lectito --completions zsh
```

## Help and Version

```bash
lectito --help
lectito --version
```

## Output Shapes

### Markdown

With `--frontmatter`, Markdown output starts with TOML frontmatter and then the extracted body.

### JSON

`--format json` and `--json` emit structured output with:

- `metadata`
- `content.markdown`
- `content.text`
- `content.html`
- optional `references`

### Metadata-Only

`--metadata-only` emits either:

- TOML metadata
- JSON metadata

without the extracted body.

## Common Workflows

### Save a Markdown export

```bash
lectito https://example.com/article --frontmatter --references -o article.md
```

### Get JSON for downstream processing

```bash
lectito https://example.com/article --json | jq '.metadata.title'
```

### Extract text without images

```bash
lectito https://example.com/article --format text --no-images
```
