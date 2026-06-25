# @stormlightlabs/lectito

JavaScript and WebAssembly bindings for Lectito.

The npm package runs Lectito's article extraction, HTML cleanup, readability
checks, HTML-to-Markdown conversion, and Markdown-to-HTML rendering in
JavaScript applications.

The Rust crate is named `lectito-wasm`.

The npm package is published as `@stormlightlabs/lectito`.

## Installation

```sh
npm install @stormlightlabs/lectito
```

## Usage

Import the package from a bundler-based app:

```ts
import { extract } from "@stormlightlabs/lectito";

const article = extract(html, "https://example.com/post", {
  charThreshold: 0,
  mediaRetention: "article",
});
```

## API

```ts
export type MediaRetention = "none" | "conservative" | "article" | "all";

export interface ReadabilityOptions {
  maxElemsToParse?: number | null;
  nbTopCandidates?: number;
  charThreshold?: number;
  contentSelector?: string | null;
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
  title?: string | null;
  byline?: string | null;
  dir?: string | null;
  lang?: string | null;
  content: string;
  markdown: string;
  text_content: string;
  length: number;
  excerpt?: string | null;
  site_name?: string | null;
  published_time?: string | null;
  image?: string | null;
  domain?: string | null;
  favicon?: string | null;
}

export interface ExtractionReport {
  article: Article | null;
  diagnostics: unknown;
}

export function extract(
  html: string,
  baseUrl?: string | null,
  options?: ReadabilityOptions | null,
): Article | null;

export function extractWithDiagnostics(
  html: string,
  baseUrl?: string | null,
  options?: ReadabilityOptions | null,
): ExtractionReport;

export function isProbablyReadable(html: string, options?: ReadableOptions | null): boolean;

export function cleanHtml(
  html: string,
  baseUrl?: string | null,
  options?: CleanHtmlOptions | null,
): string | null;

export function htmlToMarkdown(html: string): string;

export function markdownToHtml(markdown: string, options?: MarkdownOptions | null): string;
```

The JavaScript API uses camelCase option fields.

Returned article fields keep the core Rust snake_case names.
`mediaRetention` accepts `"none"`, `"conservative"`, `"article"`, or `"all"`.

## Errors

Functions throw JavaScript `Error` objects for invalid base URLs, oversized
documents, option conversion failures, and serialization failures.

## Sanitization

`cleanHtml` performs Lectito article cleanup. It is not a complete security
policy for untrusted HTML.

Browser integrations that accept arbitrary HTML should run a dedicated
sanitizer such as DOMPurify before passing content into Lectito.

Sanitize again before rendering returned HTML when the original input is untrusted.
