# WASM API

The npm package `@stormlightlabs/lectito` exposes Lectito to JavaScript through
`wasm-bindgen`.

It supports browser, web worker, bundler, and Node.js use.

```sh
npm install @stormlightlabs/lectito
```

The Rust crate is still named `lectito-wasm`.

## Build Targets

```sh
wasm-pack build crates/wasm --target bundler
wasm-pack build crates/wasm --target web
wasm-pack build crates/wasm --target nodejs
```

`wasm-pack` writes `lectito_wasm.d.ts` with the public TypeScript API.

## Initialization

Bundler builds initialize when imported:

```ts
import { extract } from "@stormlightlabs/lectito";

const article = extract(html, "https://example.com/post");
```

The `web` target needs the async initializer:

```ts
import init, { extract } from "./lectito_wasm.js";

await init();

const article = extract(html, "https://example.com/post");
```

The `nodejs` target initializes when loaded:

```js
const { extract } = require("./lectito_wasm.js");

const article = extract(html, "https://example.com/post");
```

## Functions

```ts
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

## Types

Option fields use camelCase. Returned article fields keep the core Rust snake_case names.

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
```

`mediaRetention` accepts `"none"`, `"conservative"`, `"article"`, or `"all"`.

## Errors

Functions throw JavaScript `Error` objects for invalid base URLs, oversized
documents, option conversion failures, and serialization failures.

## Sanitization

`cleanHtml` performs Lectito article cleanup. It is not a complete
untrusted-HTML security policy.

Browser integrations that accept arbitrary HTML should run a dedicated sanitizer
such as DOMPurify before passing content into Lectito. Sanitize again before
rendering returned HTML when the original input is untrusted.

## Release Checks

Run the WASM tests and build all supported package targets:

```sh
pnpm --dir packages/web exec wasm-pack test --node ../../crates/wasm
pnpm --dir packages/web exec wasm-pack build ../../crates/wasm --target bundler --out-dir ../../target/wasm-pack/bundler
pnpm --dir packages/web exec wasm-pack build ../../crates/wasm --target web --out-dir ../../target/wasm-pack/web
pnpm --dir packages/web exec wasm-pack build ../../crates/wasm --target nodejs --out-dir ../../target/wasm-pack/nodejs
```

The build commands run `wasm-opt`; restricted sandboxes may need permission to
execute it.
