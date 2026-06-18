# To-Dos

## Release Preparation

- Keep package metadata current for the public crates:
  - `lectito`
  - `lectito-cli`
  - `lectito-wasm`
- Add CI
- Keep Markdown conversion behavior covered by golden tests for headings, links,
  images, tables, code, math, footnotes, and frontmatter.
- Keep `lectito-api` and `lectito-fixtures` unpublished.

## API (`crates/api`)

- Add raw HTML extraction endpoints later only if external API users need them.
- Add rate limiting.
- Add a small benchmark command and fixture set for API latency checks.

## Web

The web app has two primary flows:

- Convert a URL through the Render API.
- Convert pasted HTML markup in the browser through WASM.

### Pages

- `/workbench`: the main extraction workspace.
- `/history`: saved local runs, with source, status, elapsed time, length,
  title, timestamp, and options.
- `/runs/:id`: one saved extraction result, with output, metadata, diagnostics,
  and options.
- `/samples`: a browsable gallery of curated fixtures and known edge cases.
- `/api`: API playground and reference examples.
- `/settings`: API base URL, default mode, output preferences, and history
  settings.

### Controls

- Expand the collapsible sidebar with saved runs and sample shortcuts.
- Add a command bar with convert, cancel, reset, copy, download, save run,
  share view link, and open result actions.
- Add output actions for copying Markdown, copying HTML, copying metadata JSON,
  downloading files, opening preview in a new tab, toggling line wrap, and
  fullscreen output.
- Improve input controls with URL validation, optional paste-from-clipboard,
  HTML file import, clear input, recent URLs, searchable samples, document size,
  and large-input warnings.
- Group advanced options by extraction, media, metadata, styling, site rules,
  and debug settings.
- Add presets for default, strict article, keep media, debug, and preserve
  styling.
- Make diagnostics readable with a summary, fallback reason, warnings, timing,
  candidate details, and sanitized-vs-cleaned comparison.
- Add a compact metadata summary above the output tabs.
- Add a compare view for source HTML, sanitized HTML, cleaned article HTML, and
  Markdown.
- Add a resizable layout with collapsible input, fullscreen preview/output, and
  persistent layout preference.
- Finish accessible tab behavior with `role="tab"`, `aria-selected`,
  `aria-controls`, tab panels, and keyboard navigation.
- Keep failed input intact and show field-level recovery messages.

### Router

- Keep the route definition in `web/src/App.tsx`.
- Put route wrappers and pages in `web/src/pages/*.tsx`.
- Use URL search params for lightweight workbench state such as mode, output
  tab, inspect tab, selected sample, and option preset.
- Do not put pasted HTML in the URL.

### Developer Tools

- Add a small smoke test that loads the app and runs one WASM extraction.
- Add a smoke test for URL mode against a mocked API response.

### Build And Deploy

- Keep `pnpm build`, `pnpm lint`, and `pnpm build:wasm` green.
- Make production builds work without a local API.

### Implementation Order

- [ ] URL-back lightweight workbench state with `useSearchParams`.
- [ ] Add output copy, download, and fullscreen actions.
- [ ] Add run history and `/runs/:id`.
  - [ ] Back with Dexie.js
- [ ] Add grouped presets and better advanced options.
- [ ] Add the sample gallery.
- [ ] Improve tab and command accessibility.
- [ ] Add `/api` behavior and `/settings` persistence.

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

## Markdown Conversion

### Clean Reference Site Chrome

- Remove skip links, "from Wikipedia" boilerplate, edit links,
  table-of-contents blocks, and infoboxes when extracting reference pages.
- Preserve equations, tables, footnotes, and citation references while removing
  navigation chrome.
- Remove heading permalink/edit anchors but keep the heading text.

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
