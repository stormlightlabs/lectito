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

## Packages and docs

Use the package that matches where extraction runs:

The [book](https://lectito.stormlightlabs.org/docs/) covers concepts, CLI usage,
configuration, diagnostics, and reference material.

Rust users can install the [lectito crate](https://crates.io/crates/lectito)
for server-side or native extraction. API docs live on
[docs.rs](https://docs.rs/lectito).

The command-line tool is published as
[lectito-cli](https://crates.io/crates/lectito-cli).

Install it with `cargo install lectito-cli`; the binary is `lectito`.
Install it with `cargo install lectito-cli --features pdf` when you want PDF
output: `lectito --format pdf --output article.pdf`. Without `--output`, PDF
mode writes `{hash}.pdf` and prints the path.

JavaScript apps use
[@stormlightlabs/lectito](https://www.npmx.dev/package/@stormlightlabs/lectito)
for the WASM bindings. Use it when your app already has HTML in the browser or
in Node.

The [source repository](https://github.com/stormlightlabs/lectito) has issues,
examples, and local development notes.

## Browser workbench

The workbench accepts pasted HTML or a live URL.

HTML runs in the browser through WASM; URL extraction uses the web service
described below.

## WASM functions

Use these exports when you already have HTML, such as pasted markup, a browser
extension capture, a fixture, or server-fetched content.

The WASM package returns HTML, Markdown, text, metadata, and diagnostics.

### `extractWithDiagnostics`

Params: `html, baseUrl?, options?`

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

### `cleanHtml`

Params: `html, baseUrl?, options?`

Cleans a fragment without a full readability pass. Use it when you already know
which part of the document you want.

```ts
import { cleanHtml } from "@stormlightlabs/lectito";

const cleaned = cleanHtml(fragment, "https://example.com", {
  keepClasses: false,
});
```

### `htmlToMarkdown`

Params: `html`

Converts cleaned HTML to Markdown for export surfaces.

```ts
import { htmlToMarkdown } from "@stormlightlabs/lectito";

const markdown = htmlToMarkdown(article.content);
```

### `markdownToHtml`

Params: `markdown`

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

## HTTP API

The hosted API runs at `https://lectito.stormlightlabs.org/api/v1/...`. It
fetches pages server-side and returns article data or Markdown.

### `POST /v1/extract`

Fetch a URL and extract its article. Returns HTML, Markdown, text, metadata,
and optional diagnostics.

```bash
curl -X POST https://lectito.stormlightlabs.org/api/v1/extract \
  -H "Content-Type: application/json" \
  -d '{
    "url": "https://example.com/post",
    "options": {
      "charThreshold": 500,
      "contentSelector": "main article",
      "keepClasses": false
    },
    "diagnostics": true
  }'
```

Setting options isn't required.

The web app sends `charThreshold`, `contentSelector`, and `keepClasses`.

The full set includes

- `maxElemsToParse`
- `nbTopCandidates`
- `siteProfiles`
- `mobileViewportWidth`
- `classesToPreserve`
- `disableJsonLd`
- `linkDensityModifier`
- `mediaRetention`

### `POST /v1/evaluate`

Check whether a URL is probably readable without a full extraction pass.

```bash
curl -X POST https://lectito.stormlightlabs.org/api/v1/evaluate \
  -H "Content-Type: application/json" \
  -d '{
    "url": "https://example.com/post",
    "options": {
      "minContentLength": 140,
      "minScore": 20
    }
  }'
```

Returns `{ "readable": true }` or `{ "readable": false }`.

### `POST /v1/transform`

Convert raw HTML to Markdown without fetching or a readability pass.

```bash
curl -X POST https://lectito.stormlightlabs.org/api/v1/transform \
  -H "Content-Type: application/json" \
  -d '{
    "html": "<h1>Title</h1><p>Body.</p>"
  }'
```

Returns `{ "markdown": "..." }` as JSON, or `text/markdown` when the request
sends `Accept: text/markdown`.

> **Note:** The request accepts an `options` field for forward compatibility,
> but the current transform endpoint ignores it.
>
> All Markdown conversion uses default settings.

## Errors

All errors return a structured body and an `x-error-code` response header.

```json
{
  "error": {
    "code": "not_readable",
    "message": "No readable article was found for this URL."
  }
}
```

### Status Codes

Rate-limited requests return `429` with a `Retry-After` header (seconds).

Timeouts return `408`.

Network failures or upstream errors return `502` or `504`.

When diagnostics are enabled, the raw error is available in the diagnostics panel.

## Credits

The editor theme in the [app](/workbench) comes from [tinted-theming](https://tinted-theming.github.io/tinted-gallery/)
and was written by [NNB](https://github.com/NNBnh).

The typography and the core inspiration for this library comes from [Mozilla](https://github.com/mozilla/readability)
