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

## Release Prep

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
