# API Overview

Lectito has two public API targets:

- [Rust Crate API](crate-api.md) for native Rust applications, CLIs, and server
  integrations.
- [WASM API](wasm-api.md) for browser, web worker, bundler, and Node.js
  integrations.

Both targets use the same core extractor and Markdown conversion logic. The Rust
crate is the source of truth; the WASM crate maps that API into JavaScript
types and camelCase option names.
