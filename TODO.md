# To-Dos

## Release Preparation

- Update package metadata for the public crates:
  - `lectito`
  - `lectito-cli`
  - generated WASM package metadata
- Add CI
- Keep Markdown conversion behavior covered by golden tests for headings, links,
  images, tables, code, math, footnotes, and frontmatter.
- Keep `lectito-fixtures` unpublished.
- Fix path-only dependencies before publishing `lectito-cli` or `lectito-wasm`.

## API (`crates/api`)

Build a small HTTP API around the core `lectito` crate. Keep it as an adapter:
request validation, extraction, response formatting, and operational limits.

### First Pass

- Add a new `lectito-api` workspace crate under `crates/api`.
- Use `axum` with `tokio`, `tower-http`, `serde`, and `tracing`.
- Add these routes:
  - `GET /healthz`
  - `POST /v1/extract-url`
  - `POST /v1/readable`
  - `POST /v1/markdown`
- Fetch URLs server-side for the web app's URL conversion flow.
- Keep raw HTML extraction as an internal/API test path only if it stays useful.
- For URL extraction, use the final fetched URL as the extraction base URL so
  relative links and site profiles work.
- Map API options directly to `ReadabilityOptions`, `ReadableOptions`, and
  `MarkdownOptions`.
- Return camelCase JSON so the API shape matches the WASM JavaScript API.
- Return structured errors:
  - `invalid_request`
  - `document_too_large`
  - `fetch_failed`
  - `unsupported_content_type`
  - `extract_failed`
  - `markdown_failed`
  - `internal_error`

### API Routing

- `POST /v1/extract-url` should accept:
  - `url`
  - `options`
  - `diagnostics`
- `POST /v1/extract-url` should fetch the URL, pass the final response URL as
  the extraction base URL, and return:
  - `article`
  - `diagnostics` when requested
  - `elapsedMs`
- `POST /v1/readable` should accept a URL and return only
  `{ "readable": true | false }`.
- `POST /v1/markdown` should convert a provided HTML fragment to Markdown.
- Keep response article fields aligned with the Rust `Article` struct and the
  WASM `Article` type.
- Reject non-HTTP(S) URLs, private-network targets, oversized responses, and
  unsupported content types before extraction.
- Follow redirects only within the configured redirect limit.

### Render Deployment

- Add a `Dockerfile` for Render.
  - Bind to the `PORT` environment variable.
- Add a request body limit before handlers run.
- Add a fetched response size limit before reading the full response body.
- Add request timeout handling.
- Add CORS with configured allowed origins.
- Add JSON logs with method, path, status, elapsed time, and error code.
- Add smoke tests for `/healthz`, `/v1/extract-url`, and `/v1/readable`.

---

- Add raw HTML extraction endpoints later only if external API users need them.
- Add rate limiting.
- Add a small benchmark command & fixture set for API latency checks.

## Web App (`web/`)

The web app has two primary flows:

- Convert a URL through the Render API.
- Convert pasted HTML markup in the browser through WASM.

### First Pass

- Rename the app package from `lectito-examplei` to a real `lectito-web`.
- Split the current pipeline into:
  - `src/lib/clients/api.ts`
  - `src/lib/clients/wasm.ts`
  - `src/lib/types.ts`

- Add a mode control (tabbed UI)
  - URL
  - HTML
- In URL mode, send the URL to the API and render the returned article.
- In HTML mode, run extraction locally through WASM.
- Keep DOMPurify in the browser before rendering previews.
- Keep the generated WASM package under `public/lectito-wasm` until the package
  story is settled.

### Main Screen

- Left side:
  - URL input for API mode
  - HTML input editor for WASM mode
  - optional `baseUrl` field for HTML mode
  - extraction options
- Right side:
  - Markdown output
  - cleaned HTML output
  - rendered preview
  - metadata
  - diagnostics
- Status strip:
  - current mode
  - elapsed time
  - article or fallback result
  - text length
  - error state

### Shared Behavior

- URL mode and HTML mode should use the same option names where the underlying
  extractor supports them.
- URL mode and HTML mode should produce the same output tabs.
- Empty or failed extraction should show a clear fallback state.
- Preview rendering must sanitize returned HTML before inserting it into the DOM.
- Diagnostics should stay available, but not dominate the default UI.

### Developer Tools

- Add sample URLs and sample HTML fixtures for quick manual checks.
- Add a small smoke test that loads the app and runs one WASM extraction.
- Add a smoke test for URL mode against a mocked API response.

### Build And Deploy

- Keep `pnpm build`, `pnpm lint`, and `pnpm build:wasm` green.
- Add an environment variable for the API base URL.
- Make production builds work without a local API.
- Keep the WASM chunk lazy-loaded so the initial app shell stays small.

## WASM And Browser Safety

- Keep `sanitize_html` out of core unless the project adopts a real sanitizer
  policy.
- Browser integrations should use DOMPurify or similar before rendering
  arbitrary HTML.
- Add WASM smoke tests for extraction, HTML-to-Markdown, and Markdown-to-HTML.
- Measure release package size after adding real exports.
- Add package metadata and a copied license file to the generated WASM package
  before treating it as publishable.

## Extraction Quality

Context: current extraction is strong for many article pages, but the remaining
edge cases usually fall into wrong-root selection, over-included chrome, or
metadata/header cleanup.

### Current Known Regressions

- Fix dropped inline text/link output such as `changed in , Cargo, and Clippy`.
- Tighten Wikipedia/reference-page profiles so sidebars and navigation tables do
  not survive article extraction.
- Add fixture tests for both cases before changing broader scoring behavior.

### Retry Short Or Suspicious Extractions

- If extracted text is far below the page's best content signals, retry with
  relaxed removal settings.
- Retry without unlikely-candidate stripping when the first result is under a
  useful word threshold.
- Retry with hidden-element removal disabled when the first result is extremely
  short.
- Prefer a larger focused subtree when the current result is only notes,
  metadata, or a single step.

### Clean Reference Site Chrome

- Remove skip links, "from Wikipedia" boilerplate, edit links,
  table-of-contents blocks, and infoboxes when extracting reference pages.
- Preserve equations, tables, footnotes, and citation references while removing
  navigation chrome.
- Remove heading permalink/edit anchors but keep the heading text.

## Markdown Conversion

### Markdown Cleanup Edge Cases

- Strip `<wbr>` without introducing spaces.
- Remove empty links like `[](url)` while preserving images.
- Add a space between sentence exclamation marks and image markdown so
  `Yey!![img]` does not become ambiguous markdown.
- Continue removing duplicate leading title headings before markdown output.

### Expand Test Coverage

- Add focused Rust tests in `crates/core/src/markdown.rs` for each feature class
  above.
- Add representative fixtures before broad implementation:
  - `elements--data-table`
  - `elements--complex-tables`
  - `elements--srcset-normalization`
  - `elements--embedded-videos`
  - `math--katex`
  - `math--mathjax-svg`
  - `footnotes--numeric-anchor-id`
  - `footnotes--google-docs-ftnt`
