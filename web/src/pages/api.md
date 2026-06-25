# Lectito API

Run pasted HTML in the browser with WASM. Fetch URLs on a server, then pass the
returned HTML or article data through your own app.

## Quick start

Import the package once. The exported functions accept strings and small option
objects, so the browser path stays local.

```ts
import { extract } from "@stormlightlabs/lectito";

const article = extract(html, "https://example.com/post", {
  charThreshold: 500,
  contentSelector: "main article",
});

console.log(article?.title);
console.log(article?.markdown);
```

## Browser workbench

The workbench does not fetch arbitrary URLs from the browser. It accepts HTML
that you paste, import, or capture elsewhere. For URL extraction, use the Render
API from a server.

## WASM functions

Use these exports when you already have HTML, such as pasted markup, a browser
extension capture, a fixture, or server-fetched content.

### `extractWithDiagnostics(html, baseUrl?, options?)`

Runs article extraction and returns the article plus diagnostic data for
candidates, fallback behavior, warnings, and timing.

```ts
import { extractWithDiagnostics } from "@stormlightlabs/lectito";

const report = extractWithDiagnostics(html, baseUrl, {
  diagnostics: true,
});

console.log(report.article?.content);
console.log(report.diagnostics);
```

### `cleanHtml(html, baseUrl?, options?)`

Cleans a fragment without a full readability pass. Use it when you already know
which part of the document you want.

```ts
import { cleanHtml } from "@stormlightlabs/lectito";

const cleaned = cleanHtml(fragment, "https://example.com", {
  keepClasses: false,
});
```

### `htmlToMarkdown(html)`

Converts cleaned HTML to Markdown for export surfaces.

```ts
import { htmlToMarkdown } from "@stormlightlabs/lectito";

const markdown = htmlToMarkdown(article.content);
```

### `markdownToHtml(markdown)`

Converts Markdown back to HTML for previews and rendering pipelines.

```ts
import { markdownToHtml } from "@stormlightlabs/lectito";

const previewHtml = markdownToHtml(markdown);
```

## Types

Options are optional. Set `baseUrl` when pasted HTML contains relative links or
images.

```ts
type Article = {
  title?: string | null;
  byline?: string | null;
  lang?: string | null;
  content: string;
  markdown: string;
  text_content?: string;
  length: number;
  excerpt?: string | null;
  site_name?: string | null;
  published_time?: string | null;
  image?: string | null;
  domain?: string | null;
  favicon?: string | null;
};

type ReadabilityOptions = {
  baseUrl?: string;
  contentSelector?: string;
  charThreshold?: number;
  keepClasses?: boolean;
  diagnostics?: boolean;
};
```

## Render API

URL extraction belongs on the server. The server can fetch the page, follow
redirects, handle headers, and return article data to the browser.

```bash
curl -X POST https://api.lectito.dev/v1/extract-url \
  -H "Content-Type: application/json" \
  -d '{
    "url": "https://example.com/post",
    "options": {
      "diagnostics": true
    }
  }'
```

## Errors

Treat an empty article as a recoverable extraction miss. Keep the original input
visible and let users retry with diagnostics, another selector, or a server-side
fetch.

```json
{
  "error": {
    "code": "not_readable",
    "message": "No readable article was found for this URL."
  }
}
```
