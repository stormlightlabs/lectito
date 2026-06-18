# Lectito

Lectito is a Rust implementation of Mozilla Readability.

The workspace has three publishable crates:

- `lectito`: Rust library crate, imported as `lectito`
- `lectito-cli`: command-line package that installs the `lectito` binary
- `lectito-wasm`: JavaScript and WebAssembly bindings, imported as
  `lectito_wasm` from Rust

The API service and fixture helpers are workspace-only crates and are not
published.

Run the local fixture helper with:

```sh
cargo run -p lectito-fixtures --bin lectito-fixture -- sample-name
```
