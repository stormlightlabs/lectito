# llms.txt

`llms.txt` is a Markdown file that gives language models and agent tools a
curated entry point for a site. Sites usually publish it at `/llms.txt`.

Lectito supports the practical parts of the convention:

- fetching a site's `llms.txt`
- parsing its sections and links
- expanding linked pages into one Markdown context file

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

## When To Use It

Use `llms.txt` when you want agents to start from a small, curated list of
important pages. It works well for docs, public APIs, policy pages, and small
knowledge bases.

Do not expect every model provider or search engine to read it. The reliable use
case is explicit: a developer, tool, or agent asks Lectito to fetch or expand the
file.
