# Development

This repo is a Rust workspace for Lectito, a Rust implementation of Mozilla
Readability.

## Project Layout

### Lib & CLI

- `crates/core`
  - the `lectito` library. This is where extraction, readability
    checks, metadata handling, and Markdown conversion live.
- `crates/cli`
  - the `lectito` command-line interface. It wraps the core library
    and adds file, stdin, URL, and ATProto input handling.
- `crates/wasm`
  - WebAssembly bindings for browser and JavaScript callers.
- `crates/fixtures`
  - shared fixture loader, corpus data, and review helper.
  - these are workspace-only. They are not published as a crate.

### Web App

- `crates/api`: API service code.
- `web`: Solid and Vite web app.

## Web App

The web app lives in `web/`. Run web commands from that directory:

```sh
cd web
pnpm dev
pnpm lint
pnpm build
pnpm format
```

The app uses Solid, Vite, Lingui, and the local WebAssembly package.

Rebuild the WASM package when Rust changes affect browser behavior:

```sh
cd web
pnpm build:wasm
```

Run WASM crate tests through `wasm-pack`:

```sh
pnpm --dir web exec wasm-pack test --node ../crates/wasm
```

The WASM tests live in `crates/wasm` and use `wasm-bindgen-test`. Use this
path for Rust exports that cross the WebAssembly boundary.

Keep message files current when user-facing web text changes:

```sh
cd web
pnpm messages:extract
pnpm messages:compile
```

## Fixtures

The large corpus of extraction fixtures live under:

```text
crates/fixtures/samples/test-pages/<case>/
```

Each case has three files:

```text
source.html
expected.html
expected-metadata.json
```

`source.html` is the input page.

`expected.html` is the expected extracted article content.

`expected-metadata.json` holds expected metadata and the upstream `readerable` flag.

ATProto fixtures used by CLI tests live under:

```text
crates/fixtures/atproto/
```

## Reviewing Corpus Behavior

Use the corpus helper as the default review tool:

```sh
cargo run -p lectito-fixtures --bin corpus -- <case>
```

For a corpus-wide quality audit:

```sh
cargo run -p lectito-fixtures --bin corpus -- --all
```

The aggregate audit reports readability, metadata, normalized text, and tag
sequence pass counts.

It also prints a metadata-field mismatch histogram so publisher metadata drift
can be reviewed without inspecting every fixture by hand.

For a diffable review, write expected and actual output files:

```sh
cargo run -p lectito-fixtures --bin corpus -- <case> \
  --diff-dir target/fixture-review
```

The helper reports:

- readability match or mismatch
- metadata match or mismatch
- normalized text match or mismatch
- tag sequence match or mismatch
- expected and actual text lengths
- expected and actual tag counts

With `--diff-dir`, it writes:

```text
target/fixture-review/<case>/
  expected.html
  actual.html
  expected.txt
  actual.txt
  expected-tags.txt
  actual-tags.txt
```

Recommended Review Order:

1. readability result
2. metadata fields
3. normalized text
4. tag sequence
5. extracted HTML
6. raw source HTML
