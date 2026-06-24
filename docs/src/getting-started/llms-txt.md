# llms.txt

`llms.txt` is a Markdown file that gives language models and agent tools a
curated entry point for a site. Sites usually publish it at `/llms.txt`.

Lectito supports the practical parts of the convention:

- fetching a site's `llms.txt`
- parsing its sections and links
- expanding linked pages into one Markdown context file
- crawling a bounded set of pages to generate an `llms.txt` index

It does not treat `llms.txt` as access control. Use `robots.txt`, HTTP
authorization, and normal server controls for that.

## File Shape

A small file looks like this:

```md
# Example Docs

> Documentation for Example's public API.

Use the current API reference when generated examples disagree with older blog
posts.

## Docs

- [Quick start](https://example.com/docs/quick-start.md): First integration
  steps.
- [API reference](https://example.com/docs/api.md): Endpoint and object
  reference.

## Optional

- [Changelog](https://example.com/docs/changelog.md)
```

Lectito expects:

- one H1 title
- an optional blockquote summary
- optional notes before the first H2
- H2 sections containing Markdown links

The `Optional` section has special handling. `lectito llms expand` skips those
links by default so the generated context stays smaller.

## Fetch

Fetch a site's `llms.txt`:

```sh
lectito llms fetch https://example.com
```

For bare site URLs, Lectito requests `/llms.txt`. Explicit URLs are used as
given:

```sh
lectito llms fetch https://example.com/docs/llms.txt
```

You can write the result to a file:

```sh
lectito llms fetch https://example.com --output llms.txt
```

## Parse

Parse an `llms.txt` file into JSON:

```sh
lectito llms parse llms.txt --pretty
```

This is useful for checking whether section names, optional links, and notes are
being read as expected.

## Expand

Expand linked resources into one Markdown file:

```sh
lectito llms expand llms.txt --output llms-full.txt
```

Lectito keeps Markdown resources unchanged. When a linked resource looks like
HTML, Lectito extracts the readable article and inserts the extracted Markdown.
For remote links, Lectito checks the HTTP `Content-Type` header before falling
back to URL suffixes and simple Markdown markers.

Each resource is separated and labeled:

```md
---
# Source: Quick start
URL: https://example.com/docs/quick-start.md
Notes: First integration steps.
...
```

Use `--include-optional` to include the `Optional` section:

```sh
lectito llms expand llms.txt --include-optional --output llms-full.txt
```

Use `--max-links` when you want a smaller bundle:

```sh
lectito llms expand llms.txt --max-links 10
```

## Generate

Generate an `llms.txt` file from a seed page:

```sh
lectito llms generate https://example.com/docs/ --output llms.txt
```

The crawler is intentionally bounded. For URL seeds, Lectito follows
same-origin links only. For local HTML files, it follows relative local links.
Assets such as images, stylesheets, scripts, PDFs, archives, and feeds are
skipped.

You can also generate from a sitemap:

```sh
lectito llms generate --sitemap https://example.com/sitemap.xml \
  --output llms.txt
```

Or discover sitemaps from a URL seed:

```sh
lectito llms generate https://example.com --discover \
  --output llms.txt
```

Discovery reads `Sitemap:` lines from `robots.txt`. When no sitemap is listed
there, Lectito tries `/sitemap.xml`.

Sitemap indexes are supported. Lectito reads child sitemaps up to
`--max-sitemaps`, then fetches page URLs up to `--max-pages`:

```sh
lectito llms generate --sitemap https://example.com/sitemap.xml \
  --max-sitemaps 10 \
  --max-pages 100 \
  --output llms.txt
```

Remote sitemap generation keeps sitemap and page URLs on the same origin as the
sitemap input. Local sitemap files may list any absolute page URL.

By default, generation fetches up to 25 pages and follows links up to depth 2:

```sh
lectito llms generate https://example.com/docs/ \
  --max-pages 10 \
  --max-depth 1
```

Use `--filter` for the common path and glob cases. Prefix a pattern with `!` to
exclude it:

```sh
lectito llms generate --sitemap https://example.com/sitemap.xml \
  --filter /docs/ \
  --filter '!/docs/archive/' \
  --filter '!*/drafts/*'
```

Patterns that start with `/` match URL paths. Plain path values are prefixes.
Path patterns with `*` or `?` are globs. Other glob patterns match the full URL.

Use `--delay` to wait between page fetches:

```sh
lectito llms generate https://example.com/docs/ --delay 250
```

Remote generation checks `robots.txt` before fetching page URLs. Lectito keeps
the existing browser-like user agent for HTTP requests, but evaluates robots
rules as `Lectito` unless you pass another token:

```sh
lectito llms generate https://example.com/docs/ \
  --robots-agent LectitoDocsBot
```

Use `--ignore-robots` only when you explicitly want to bypass those checks:

```sh
lectito llms generate https://example.com/docs/ --ignore-robots
```

Only pages that produce readable article content are included. Each accepted
page becomes one link in the generated file. Lectito uses the extracted title as
the link label and the extracted excerpt as the link note.

Set the generated title, summary, or section name when the defaults are too
generic:

```sh
lectito llms generate https://example.com/docs/ \
  --title "Example Docs" \
  --summary "Public documentation for Example." \
  --section "Guides" \
  --output llms.txt
```

## When To Use It

Use `llms.txt` when you want agents to start from a small, curated list of
important pages. It works well for docs, public APIs, policy pages, and small
knowledge bases.

Do not expect every model provider or search engine to read it. The reliable use
case is explicit: a developer, tool, or agent asks Lectito to fetch or expand the
file.
