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

The [book](https://lectito.stormlightlabs.org/) covers concepts, CLI usage,
configuration, diagnostics, and reference material.

Rust users can install the [lectito crate](https://crates.io/crates/lectito)
for server-side or native extraction. API docs live on
[docs.rs](https://docs.rs/lectito).

The command-line tool is published as
[lectito-cli](https://crates.io/crates/lectito-cli).

Install it with `cargo install lectito-cli`; the binary is `lectito`.
Install it with `cargo install lectito-cli --features pdf` when you want to output
to pdf: `lectito --format pdf --output article.pdf`.

JavaScript apps use
[@stormlightlabs/lectito](https://www.npmx.dev/package/@stormlightlabs/lectito)
for the WASM bindings. Use it when your app already has HTML in the browser or
in Node.

The [source repository](https://github.com/stormlightlabs/lectito) has issues,
examples, and local development notes.

## Browser workbench

The workbench does not fetch arbitrary URLs from the browser. It accepts HTML
that you paste, import, or capture elsewhere. For URL extraction, use the Render
API from a server.

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

## Credits

The editor theme in the [app](/workbench) comes from [tinted-theming](https://tinted-theming.github.io/tinted-gallery/)
and was written by [NNB](https://github.com/NNBnh).

The typography and the core inspiration for this library comes from [Mozilla](https://github.com/mozilla/readability)
