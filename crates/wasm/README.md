# lectito-wasm

This crate will provide the JavaScript and WebAssembly packaging surface for
Lectito. It is intentionally a stub while the core crate API is settled.

## Goals

- Run Lectito in browsers, web workers, Node.js, and bundler-based web apps.
- Expose article extraction, HTML cleanup, HTML-to-Markdown, and
  Markdown-to-HTML through a small JavaScript API.
- Keep the Rust core crate as the source of truth for parsing, cleanup,
  metadata, diagnostics, and Markdown behavior.
- Make browser demos possible without a backend service.

## Sanitization Guidance

Lectito cleanup is optimized for readable article extraction. It should not be
treated as a complete untrusted-HTML security policy. Browser integrations that
accept arbitrary HTML should run a dedicated sanitizer such as DOMPurify before
passing content into Lectito, or before rendering any HTML returned by Lectito.

## Proposed Package Shape

Build with `wasm-pack`:

```sh
wasm-pack build crates/wasm --target bundler
wasm-pack build crates/wasm --target web
wasm-pack build crates/wasm --target nodejs
```

Key dependencies:

- `wasm-bindgen` for JavaScript bindings.
- `serde-wasm-bindgen` for structured option/result conversion.
- `console_error_panic_hook` for useful browser panic diagnostics in debug
  builds.
- `lectito` as the core Rust library crate.

## Proposed JavaScript API

```ts
export type MediaRetention = "none" | "conservative" | "article" | "all";

export interface ReadabilityOptions {
  maxElemsToParse?: number;
  nbTopCandidates?: number;
  charThreshold?: number;
  contentSelector?: string;
  siteProfiles?: string[];
  mobileViewportWidth?: number | null;
  classesToPreserve?: string[];
  keepClasses?: boolean;
  disableJsonLd?: boolean;
  linkDensityModifier?: number;
  mediaRetention?: MediaRetention;
}

export interface ReadableOptions {
  minContentLength?: number;
  minScore?: number;
}

export interface MarkdownOptions {
  gfm?: boolean;
  footnotes?: boolean;
  math?: boolean;
  allowRawHtml?: boolean;
}

export type CleanHtmlOptions = ReadabilityOptions;

export interface Article {
  title?: string;
  byline?: string;
  dir?: string;
  lang?: string;
  content: string;
  markdown: string;
  textContent: string;
  length: number;
  excerpt?: string;
  siteName?: string;
  publishedTime?: string;
  image?: string;
  domain?: string;
  favicon?: string;
}

export function extract(
  html: string,
  baseUrl?: string | null,
  options?: ReadabilityOptions,
): Article | null;

export function extractWithDiagnostics(
  html: string,
  baseUrl?: string | null,
  options?: ReadabilityOptions,
): unknown;

export function isProbablyReadable(
  html: string,
  options?: ReadableOptions,
): boolean;

export function cleanHtml(
  html: string,
  baseUrl?: string | null,
  options?: CleanHtmlOptions,
): string | null;

export function htmlToMarkdown(html: string): string;

export function markdownToHtml(
  markdown: string,
  options?: MarkdownOptions,
): string;
```

`cleanHtml` returns Lectito's cleaned article HTML, which is optimized for
readable content extraction. It does not sanitize arbitrary HTML. Browser apps
should run DOMPurify or a similar sanitizer before calling `cleanHtml`, and
again before rendering any returned HTML if the original input is untrusted.

Errors should throw JavaScript `Error` objects for invalid base URLs, oversized
documents, serialization failures, and option conversion failures.

## Example App Plan

Create a small browser example under this crate, for example:

```text
crates/wasm/examples/codemirror-cleanup/
```

The example should:

- Use a dual-pane CodeMirror layout for HTML input and generated output.
- Run an explicit sanitize step.
- Run Lectito cleanup/extraction on the sanitized HTML.
- Render three synchronized outputs:
  - sanitized HTML
  - cleaned article HTML
  - Markdown
- Include options for `baseUrl`, `contentSelector`, `charThreshold`, and
  `keepClasses`.
- Show extraction metadata and a compact diagnostics panel.
- Run entirely in the browser with the WASM package loaded as an ES module.

The intended processing flow is:

```text
CodeMirror HTML input
  -> sanitize HTML
  -> clean/extract article HTML with Lectito
  -> convert cleaned HTML to Markdown
  -> preview outputs
```

The UI should make the sanitize-vs-cleanup distinction visible so users do not
assume article cleanup is a complete XSS policy.
