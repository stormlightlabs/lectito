# To-Dos

## MCP Article Reader

- [x] Add a `lectito-mcp` binary crate.
- [x] Use stdio transport for the first MCP server.
- [x] Expose a `search_articles` tool:
  - [x] Adapt the DuckDuckGo HTML search client and parser from `gremlin`
  - [x] Return title, URL, and snippet.
  - [x] Cap results to a small limit, probably 10 but make configurable.
- [x] Expose a `read_article` tool:
  - [x] Fetch a public `http` or `https` URL.
  - [x] Reject private-network targets by default.
  - [x] Enforce redirect, timeout, and response-size limits.
  - [x] Check for HTML-like content types.
  - [x] Run `lectito::extract_with_diagnostics` directly.
  - [x] Return title, byline, site name, published time, final URL, excerpt,
        content length, truncation state, and content.
  - [x] Support `format`, `offset`, and `maxChars` arguments.
- [x] Return both text content and structured content from MCP tool calls.
- [x] Report malformed requests as protocol errors.
- [x] Report fetch, extraction, and unreadable-page failures as tool results with
      `isError: true`.
- [x] Keep logs on stderr so stdout contains only MCP messages.
- [x] Do not add persistence, browser rendering, summaries, caching, or extra
      tools until usage proves they are needed.

## PDF CLI Feature

- [x] Add an optional `pdf` feature to `crates/cli`.
  - [x] Keep `default = []`.
  - [x] Add optional `pdf-writer` and `pulldown-cmark` dependencies.
- [x] Port only the Markdown-to-PDF path from `picopdf`.
  - [x] Add a `pulldown-cmark` event adapter plus styler, layout, and renderer
        code in a small `crates/cli/src/pdf` module.
  - [x] Start with built-in PDF fonts and leave custom font flags out of the
        first version.
- [x] Expose PDF as an extract output format.
  - [x] Add `OutputFormat::Pdf` behind `#[cfg(feature = "pdf")]`.
  - [x] Update `--format` help text when the feature is enabled.
  - [x] Reuse the extracted article Markdown as the PDF source.
- [x] Write PDF output as bytes.
  - [x] Keep string formats on the existing `echo::render_article` path.
  - [x] Write PDF bytes to `--output` with `fs::write`.
  - [x] Write PDF bytes to stdout with `io::stdout().write_all`.
  - [x] Preserve the current `--inspect` and `--diagnostic-format` behavior.
- [x] Add focused tests.
  - [x] Check that `--format pdf` parses with `--features pdf`.
  - [x] Check that the PDF renderer returns bytes starting with `%PDF`.
  - [x] Cover common Markdown blocks in the PDF parser tests.
  - [x] Keep non-PDF builds compiling without PDF dependencies.
- [x] Update docs.
  - [x] Document installation with `cargo install lectito-cli --features pdf`.
  - [x] Add `pdf` to CLI output-format docs as an optional feature.
  - [x] Mention that PDF manipulation tools are out of scope for now.
- [x] Verify the CLI after the Rust changes.
  - [x] Run `cargo test -p lectito-cli`.
  - [x] Run `cargo test -p lectito-cli --features pdf`.
  - [x] Run `cargo run -p lectito-cli --features pdf -- https://render.com/docs \
    --format pdf -o .sandbox/render/pdf/render-docs.pdf`.

## Release Prep

- Keep package metadata current for the public crates:
  - `lectito`
  - `lectito-cli`
  - `lectito-mcp`
  - `lectito-wasm`
  - Keep `lectito-api` and `lectito-fixtures` unpublished.
- Keep `release.md` current with the version bump checklist for every workspace
  crate.
- Add a Rust CI workflow for:
  - `cargo fmt --check` & `cargo check --workspace`
  - `cargo test --workspace`
  - Clippy with denied warnings & Rustdoc warnings
  - Publish dry-runs for public crates.
- Include `wasm-pack test --node` and `wasm-pack build` checks for the
  `bundler`, `web`, and `nodejs` WASM targets.
