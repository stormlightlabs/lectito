# CLI Usage

Complete reference for the `lectito` command-line tool.

## Basic Syntax

```bash
lectito [OPTIONS] <INPUT>
```

The `INPUT` can be:

- A URL (starts with `http://` or `https://`)
- A local file path
- `-` to read from stdin

## Examples

### URL Extraction

```bash
lectito https://example.com/article
```

### Local File

```bash
lectito article.html
```

### Stdin Pipe

```bash
curl https://example.com | lectito -
cat page.html | lectito -
wget -qO- https://example.com | lectito -
```

## Options

### `-o, --output <FILE>`

Write output to a file instead of stdout.

```bash
lectito https://example.com/article -o article.md
```

### `-f, --format <FORMAT>`

Specify output format. Available formats:

| Format             | Description                         |
| ------------------ | ----------------------------------- |
| `markdown` or `md` | Markdown (default)                  |
| `json`             | Structured JSON                     |
| `text` or `txt`    | Plain text                          |
| `html`             | Cleaned HTML                        |

```bash
lectito https://example.com/article -f json
```

### `--timeout <SECONDS>`

HTTP request timeout in seconds (default: 30).

```bash
lectito https://example.com/article --timeout 60
```

### `--user-agent <USER_AGENT>`

Custom User-Agent header.

```bash
lectito https://example.com/article --user-agent "MyBot/1.0"
```

### `--config <PATH>`

Path to site configuration file (TOML format).

```bash
lectito https://example.com/article --config site-config.toml
```

### `-v, --verbose`

Enable verbose debug logging.

```bash
lectito https://example.com/article -v
```

### `-h, --help`

Display help information.

```bash
lectito --help
```

### `-V, --version`

Display version information.

```bash
lectito --version
```

## Common Workflows

### Extract and Save Article

```bash
lectito https://example.com/article -o articles/article.md
```

### Batch Processing Multiple URLs

```bash
while read url; do
    lectito "$url" -o "articles/$(date +%s).md"
done < urls.txt
```

### Extract to JSON for Processing

```bash
lectito https://example.com/article --format json | jq '.metadata.title'
```

### Extract from Multiple Files

```bash
for file in articles/*.html; do
    lectito "$file" -o "processed/$(basename "$file" .html).md"
done
```

### Custom Timeout for Slow Sites

```bash
lectito https://slow-site.com/article --timeout 120
```

## Output Formats

### Markdown (Default)

Output includes TOML frontmatter with metadata (when `--frontmatter` is used):

```markdown
+++
title = "Article Title"
author = "John Doe"
date = "2025-01-17"
excerpt = "A brief description..."
+++

# Article Title

Article content here...
```

### JSON

Structured output with all metadata:

```json
{
    "metadata": {
        "title": "Article Title",
        "author": "John Doe",
        "date": "2025-01-17",
        "excerpt": "A brief description..."
    },
    "content": "<div>...</div>",
    "text_content": "Article content here...",
    "word_count": 500
}
```

### Plain Text

Just the article text without formatting:

```text
Article Title

Article content here...
```

## Exit Codes

| Code | Meaning                                    |
| ---- | ------------------------------------------ |
| 0    | Success                                    |
| 1    | Error (invalid URL, network failure, etc.) |

## Error Handling

The CLI will print error messages to stderr:

```bash
lectito https://invalid-domain-xyz.com
# Error: failed to fetch URL: dns error: failed to lookup address information
```

For content that isn't readable:

```bash
lectito https://example.com/page
# Error: content not readable: score 15.2 < threshold 20.0
```

## Tips

1. **Use timeouts**: Set appropriate timeouts to avoid hanging
2. **Batch operations**: Process multiple URLs in parallel
3. **Save to file**: Use `-o` to avoid terminal rendering overhead
4. **JSON for parsing**: Use JSON output when processing with other tools

## Next Steps

- [Configuration](../library/configuration.md) - Advanced configuration options
- [Output Formats](../library/output-formats.md) - Detailed format documentation
- [Concepts](../concepts/how-it-works.md) - Understanding the algorithm
