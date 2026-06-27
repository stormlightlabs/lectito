---
name: article-reader
description: >-
  Read, extract, search, summarize, cite, or compare web articles using
  article-reader tools. Use when the task includes article URLs, requires
  finding articles on a topic, needs source-grounded notes from web pages, or
  asks what an article says.
---

# Article Reader

Use article-reader tools when a task depends on article content from the web.

## Tool Setup

Use `read_article` and `search_articles` when they are available.

If those tools are unavailable, or the task is to set up article reading in a
workspace, read [references/lectito.md](references/lectito.md) for the Lectito
CLI and MCP-backed implementation notes.

## Workflow

1. If the user gives a URL, call `read_article`.
2. If the user gives a topic but no URL, call `search_articles`, then read the
   most relevant sources.
3. Prefer original and primary sources over summaries.
4. Use extracted title, byline, published date, site name, final URL, and text
   when answering.
5. Cite the source URL for article-specific claims.
6. If extraction fails or the page is not an article, say that plainly and use
   only available metadata or search snippets.
7. Do not bypass paywalls, login walls, bot challenges, or access controls.
8. For long articles, request chunks instead of the whole page at once.

## Output

For summaries, include the source title, URL, main points, and any uncertainty
caused by missing dates, extraction failure, or partial content.

For comparisons, read all sources first, then compare claims by source.
