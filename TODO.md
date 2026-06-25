# To-Dos

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
