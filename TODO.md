# To-Dos

## Repository Replacement

Context: this rewrite should eventually replace `stormlightlabs/lectito` as a
focused parser/extractor project. The old hosted API and web app are not part of
the important product surface; the core extractor/parser, CLI, fixtures, and
docs are.

### Drop Or Archive Legacy Surfaces

- Remove or archive the old API/server workspace pieces from the GitHub project.
- Remove or archive the old web application from the GitHub project.
- Remove public documentation that describes hosted API behavior as a supported product.
- Keep replacement scope centered on `lectito-core`, `lectito-cli`, fixtures, and mdBook docs.

### Public API Reset

- Document the new public API around `extract`, `extract_with_diagnostics`, `is_probably_readable`, `Article`, `ReadabilityOptions`, and `ReadableOptions`.
- Do not add compatibility shims for the old `parse`, `Readability`, or builder-style API.
- Keep internal scoring, cleanup, recovery, and serialization modules private unless there is a concrete consumer need.

### Markdown First-Class Output

- Treat Markdown as a core output beside cleaned HTML and plain text.
- Ensure `Article` documentation clearly explains `content`, `markdown`, `text_content`, metadata, and length fields.
- Make CLI examples show Markdown output directly.
- Keep Markdown conversion behavior covered by golden tests for headings, links, images, tables, code, math, footnotes, and frontmatter.

### Release Preparation

- Update package metadata for the replacement repository.
- Keep MPL-2.0 as the project license.
- Add CI for formatting, tests, and linting before takeover.
- Write a changelog entry explaining that the project was rewritten around the core extractor/parser and that the old hosted API/web app are no longer supported surfaces.

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
