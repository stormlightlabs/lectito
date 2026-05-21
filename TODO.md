# To-Dos

## Release Preparation

- Update package metadata for the replacement repository.
- Add CI for formatting, tests, and linting.
- Write a changelog entry explaining that the project was rewritten around the
  core extractor/parser and that the old hosted API/web app are no longer supported
  surfaces.
- Keep Markdown conversion behavior covered by golden tests for headings, links,
  images, tables, code, math, footnotes, and frontmatter.

## Public API And WASM

- Rename the public Rust library package from `lectito-core` to `lectito`.
- Keep the installed CLI binary name `lectito`; consider `lec` only as an
  optional alias.
- Add core APIs:
  - `markdown_to_html(markdown, options)`
  - `MarkdownOptions`
  - `clean_article_html(html, base_url, options)`
- Keep `sanitize_html` out of core unless the project adopts a real sanitizer
  policy. Browser examples should recommend DOMPurify or similar before
  rendering arbitrary HTML.
- Add `lectito-wasm` at `crates/wasm` with `wasm-bindgen` exports for:
  - `extract`
  - `extractWithDiagnostics`
  - `isProbablyReadable`
  - `cleanHtml`
  - `htmlToMarkdown`
  - `markdownToHtml`
- Add `serde-wasm-bindgen`, `console_error_panic_hook`, and `wasm-bindgen-test`
  when the exports are implemented.
- Use camelCase JS fields and map them to Rust structs internally.
- Add Rust wrapper DTOs or serde derives for options, articles, and diagnostics
  as needed by WASM.
- Add a browser example with dual-pane CodeMirror input and the pipeline:
  HTML input -> DOMPurify sanitize -> Lectito cleanup -> Markdown output.
- Include example controls for base URL, content selector, char threshold, and
  class preservation.
- Add WASM smoke tests for extraction, HTML-to-Markdown, and Markdown-to-HTML.
- Add npm package metadata once naming and distribution are settled.
- Measure release package size after adding real exports.

## Extraction Quality

Context: current extraction is strong for many article pages, but the remaining
edge cases usually fall into wrong-root selection, over-included chrome, or
metadata/header cleanup.

### Retry Short Or Suspicious Extractions

- If extracted text is far below the page's best content signals, retry with relaxed removal settings.
- Retry without unlikely-candidate stripping when the first result is under a useful word threshold.
- Retry with hidden-element removal disabled when the first result is extremely short.
- Prefer a larger focused subtree when the current result is only notes, metadata, or a single step.

### Clean Reference Site Chrome

- Remove skip links, "from Wikipedia" boilerplate, edit links, table-of-contents blocks, and infoboxes when extracting reference pages.
- Preserve equations, tables, footnotes, and citation references while removing navigation chrome.
- Remove heading permalink/edit anchors but keep the heading text.

## Markdown Conversion

### Markdown Cleanup Edge Cases

- Strip `<wbr>` without introducing spaces.
- Remove empty links like `[](url)` while preserving images.
- Add a space between sentence exclamation marks and image markdown so `Yey!![img]` does not become ambiguous markdown.
- Continue removing duplicate leading title headings before markdown output.

### Expand Test Coverage

- Add focused Rust tests in `crates/core/src/markdown.rs` for each feature class above.
- Add representative fixtures before broad implementation:
  - `elements--data-table`
  - `elements--complex-tables`
  - `elements--srcset-normalization`
  - `elements--embedded-videos`
  - `math--katex`
  - `math--mathjax-svg`
  - `footnotes--numeric-anchor-id`
  - `footnotes--google-docs-ftnt`
