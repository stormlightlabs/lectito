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

## Atproto

- URLS:
  - https://standard.site/
  - https://atproto.com/blog/standard-site-bluesky-timeline
  - https://jola.dev/posts/publishing-your-blog
- [ ] Preserve rich-text facets when rendering Standard.site content records.
- [ ] Resolve blob images into usable image URLs.
- [ ] Render footnotes from publisher block records.
- [ ] Render embedded Standard.site posts.
- [ ] Render Bluesky post embeds.
- [ ] Render web bookmark and web embed blocks.
- [ ] Render tables from publisher block records.
- [ ] Render math blocks.
- [ ] Keep image captions and alt text when the publisher supplies both.
- [ ] Add frozen fixtures for Leaflet, pckt, and Offprint records.
- [ ] Report Standard.site resolution and rendering warnings in diagnostics.

## Markdown Conversion

### Clean Reference Site Chrome

- [ ] Remove reference-page chrome from Wikipedia extraction.
  - URL: https://en.wikipedia.org/wiki/Mozilla
  - Remove skip links, "from Wikipedia" boilerplate, edit links, table-of-contents blocks, and infoboxes.
- [ ] Preserve equations, tables, footnotes, and citation references while removing navigation chrome.
  - URL: https://en.wikipedia.org/wiki/Hermitian_matrix
- [ ] Remove heading permalink/edit anchors but keep the heading text.
  - URL: https://sre.google/sre-book/table-of-contents/

### Markdown Cleanup Edge Cases

- [ ] Strip `<wbr>` without introducing spaces.
  - URL:
    https://developer.mozilla.org/en-US/docs/Learn_web_development/Core/Structuring_content/HTML_images
- [ ] Remove empty links like `[](url)` while preserving images.
  - URL: https://web.dev/articles/responsive-images
- [ ] Add a space between sentence exclamation marks and image markdown so
      `Yey!![img]` does not become ambiguous markdown.
  - URL: https://web.dev/articles/responsive-images
- [ ] Continue removing duplicate leading title headings before markdown output.
  - URL: https://www.paulgraham.com/makersschedule.html

### Expand Test Coverage

- [ ] Add focused Rust tests in `crates/core/src/markdown.rs` for each feature class above.
- [ ] Add an `elements--data-table` fixture.
  - URL: https://en.wikipedia.org/wiki/Hermitian_matrix
- [ ] Add an `elements--complex-tables` fixture.
  - URL: https://www.rfc-editor.org/rfc/rfc7540
- [ ] Add an `elements--srcset-normalization` fixture.
  - URL: https://web.dev/articles/responsive-images
- [ ] Add an `elements--embedded-videos` fixture.
  - URL: https://developer.mozilla.org/en-US/docs/Learn_web_development/Core/Structuring_content/HTML_images
- [ ] Add a `math--katex` fixture.
  - URL: https://www.intmath.com/cg5/katex-mathjax-comparison.php
- [ ] Add a `math--mathjax-svg` fixture.
  - URL: https://mathjax.github.io/MathJax-demos-web/input/tex-mml2chtml.html
- [ ] Add a `footnotes--numeric-anchor-id` fixture.
  - URL: https://math.meta.stackexchange.com/questions/5020/mathjax-basic-tutorial-and-quick-reference
- [ ] Add a `footnotes--google-docs-ftnt` fixture.
  - URL: https://stackprinter.appspot.com/export?question=5020&service=math.meta.stackexchange&language=en&hideAnswers=false&width=640
