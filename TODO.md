# To-Dos

## Release Preparation

- Keep package metadata current for the public crates:
  - `lectito`
  - `lectito-cli`
  - `lectito-wasm`
  - Keep `lectito-api` and `lectito-fixtures` unpublished.
- Add a Rust CI workflow for:
  - `cargo fmt --check` & `cargo check --workspace`
  - `cargo test --workspace`
  - Clippy with denied warnings & Rustdoc warnings
  - Publish dry-runs for public crates.
- Include `wasm-pack test --node` and `wasm-pack build` checks for the
  `bundler`, `web`, and `nodejs` WASM targets.

## API (`crates/api`)

- Add raw HTML extraction endpoints later only if external API users need them.
- Add rate limiting.
- Add a small benchmark command and fixture set for API latency checks.

## Web

The web app has two primary flows:

- Convert a URL through the Render API.
- Convert pasted HTML markup in the browser through WASM.

### Pages

- `/`: landing page with a direct workbench CTA, before/after extraction
  diagram, capability summary, and links to the API and samples.
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

### Implementation Order

- [ ] URL-back lightweight workbench state with `useSearchParams`.
- [ ] Add output copy, download, and fullscreen actions.
- [ ] Add reader-style preview controls for the extracted article.
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

### `examples.txt` Audit

- Fix MDN-style code block rendering. The HTML extractor keeps usable
  `<pre><code>` nodes, but Markdown output can emit broken fences like
  `js```js notranslate`. Normalize language IDs such as `js notranslate`,
  remove sibling language labels, and add a focused MDN fixture.
- Clean app-doc controls from Mintlify-style pages. Current output can include
  duplicated `Copy page` SVG/button text and tab labels without their code
  panels. Extend button/control cleanup and add a fixture for tabbed code docs.
- Improve modern docs root scoring. Mintlify first selects `body.antialiased`
  and cleans it to empty before accepting `main#content-container`. Prefer
  focused `main`/article roots over body-level app shells when both are
  available.
- Tighten web.dev title cleanup. Remove UI suffixes like `Stay organized with
collections Save and categorize content based on your preferences.` from
  metadata titles.
- Decide whether site-profile extraction should still absolutize URLs when
  cleanup is disabled. Wikipedia profile output keeps links such as
  `/wiki/Hermitian_matrix` even when the CLI input is a URL.
- Improve Rustdoc output polish. Remove `Expand description`, strip section
  permalink glyphs such as `§`, and render item definition lists with spacing
  instead of concatenating adjacent entries.
- Add a site rule for `unthread.at` post pages:
  - These are ATProtocol records: `unthread.at/@{handle}/rkey` so we should
    implement a fetcher for these
  - Look into implementing standard.site parsing
    - https://standard.site/
    - https://atproto.com/blog/standard-site-bluesky-timeline

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
