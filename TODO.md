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

## Extraction Quality

### `examples.txt` Audit

- [x] Fix MDN-style code block rendering.
      URL:
  - https://developer.mozilla.org/en-US/docs/Web/JavaScript/Reference/Global_Objects/Proxy
  - Keep usable `<pre><code>` nodes, normalize language IDs such as
    `js notranslate`, remove sibling language labels, and add a focused fixture.
- [x] Clean app-doc controls from Mintlify-style pages.
  - URL: https://www.mintlify.com/docs/create/code
  - Remove duplicated `Copy page` SVG/button text and tab labels without their
    code panels. Extend button/control cleanup and add a fixture for tabbed code
    docs.
- [x] Improve modern docs root scoring.
  - URL: https://mintlify.com/docs/code
  - Prefer focused `main`/article roots over body-level app shells when both are
    available. Mintlify can select `body.antialiased` and clean it to empty
    before accepting `main#content-container`.
- [x] Tighten web.dev title cleanup.
  - URL: https://web.dev/articles/responsive-images
  - Remove UI suffixes such as `Stay organized with collections Save and
categorize content based on your preferences.` from metadata titles.
- [x] Decide whether site-profile extraction should still absolutize URLs when
      cleanup is disabled.
  - URL: https://en.wikipedia.org/wiki/Hermitian_matrix
  - Wikipedia profile output can keep links such as `/wiki/Hermitian_matrix` even
    when the CLI input is a URL.
- [x] Improve Rustdoc output polish.
  - URL: https://docs.rs/serde/latest/serde/
  - Remove `Expand description`, strip section permalink glyphs such as `§`, and
    render item definition lists with spacing instead of concatenating adjacent
    entries.
- [ ] Add a site rule for `unthread.at` post pages.
  - URL: https://unthread.at/@desertthunder.dev/3mlgpk65xzw23
  - These are ATProtocol records: `unthread.at/@{handle}/rkey`.
  - Implement a fetcher for them and check whether standard.site parsing applies:
    - https://standard.site/
    - https://atproto.com/blog/standard-site-bluesky-timeline
    - https://jola.dev/posts/publishing-your-blog

## Atproto

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

### Retry Short Or Suspicious Extractions

- [ ] Retry with relaxed removal settings when extracted text is far below the
      page's best content signals.
  - URL: https://www.royalroad.com/fiction/63759/super-supportive/chapter/1449598/one-hundred-two-what-kind-of-wordchain
- [ ] Retry without unlikely-candidate stripping when the first result is under
      a useful word threshold.
  - URL: http://www.ehow.com/how_2042752_build-terrarium.html
- [ ] Retry with hidden-element removal disabled when the first result is
      extremely short.
  - URL: https://www.aclu.org/blog/privacy-technology/internet-privacy/facebook-tracking-me-even-though-im-not-facebook
- [ ] Prefer a larger focused subtree when the current result is only notes,
      metadata, or a single step.
  - URL: https://sport.aktualne.cz/fotbal/zahranici/west-ham-hrozi-gigantum-okouzlil-i-linekera-souckovu-praci-j/r~8fa032ba3add11ec8a900cc47ab5f122/

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
