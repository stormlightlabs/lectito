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
- [x] Preserve rich-text facets when rendering Standard.site content records.
- [x] Resolve blob images into usable image URLs.
- [x] Render footnotes from publisher block records.
- [x] Render embedded Standard.site posts.
- [x] Render Bluesky post embeds.
- [x] Render web bookmark and web embed blocks.
- [ ] Render tables from publisher block records.
- [ ] Render math blocks.
- [ ] Keep image captions and alt text when the publisher supplies both.
- [ ] Add frozen fixtures for Leaflet, pckt, and Offprint records.
- [ ] Report Standard.site resolution and rendering warnings in diagnostics.
