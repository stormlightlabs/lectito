# WASM API

The `lectito-wasm` crate exposes the core `lectito` APIs to JavaScript through
`wasm-bindgen`.

Build targets:

```sh
wasm-pack build crates/wasm --target bundler
wasm-pack build crates/wasm --target web
wasm-pack build crates/wasm --target nodejs
```

## Functions

```ts
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

## Options

The JavaScript API uses camelCase fields and maps them to the Rust options
internally.

```ts
export interface ReadabilityOptions {
  maxElemsToParse?: number | null;
  nbTopCandidates?: number;
  charThreshold?: number;
  contentSelector?: string | null;
  mobileViewportWidth?: number | null;
  classesToPreserve?: string[];
  keepClasses?: boolean;
  disableJsonLd?: boolean;
  linkDensityModifier?: number;
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
```

## Sanitization

`cleanHtml` performs Lectito article cleanup. It is not a complete
untrusted-HTML security policy.

Browser integrations that accept arbitrary HTML should run a dedicated sanitizer
such as DOMPurify before passing content into Lectito, and should sanitize again
before rendering returned HTML when the original input is untrusted.

## Errors

The WASM functions throw JavaScript `Error` objects for invalid base URLs,
oversized documents, serialization failures, and option conversion failures.
