# Publishing

The workspace has three public crates:

- `lectito`: the Rust library crate
- `lectito-cli`: the CLI package, which installs the `lectito` binary
- `lectito-wasm`: JavaScript and WebAssembly bindings

`lectito-api` and `lectito-fixtures` are workspace-only crates and are marked
`publish = false`.

The fixture crate owns the local `lectito-fixture` helper binary. It is useful
for regression work, but it is not part of the public CLI package.

## Publish Order

Publish `lectito` first. The CLI and WASM crates depend on `lectito` with both
a local `path` and a registry `version`:

```toml
lectito = { path = "../core", version = "0.1.0" }
```

That lets local workspace builds use the source tree while crates.io packages
depend on the published `lectito` version.

The first release sequence is:

```sh
cargo publish -p lectito
```

Wait for `lectito 0.1.0` to appear in the crates.io index, then verify and
publish the dependent crates:

```sh
cargo publish --dry-run -p lectito-cli
cargo publish --dry-run -p lectito-wasm

cargo publish -p lectito-cli
cargo publish -p lectito-wasm
```

When preparing a new version, update the workspace version and each published
crate dependency version together. Dry-run all three packages before publishing.

## Local Verification

Before publishing, run:

```sh
cargo check --workspace
cargo publish --dry-run -p lectito
```

Before `lectito` has been published for a new version, Cargo will reject
`lectito-cli` and `lectito-wasm` dry-runs with `no matching package named
lectito found`. That is expected for the first crate in a release train. Retry
those dry-runs after the library crate is visible in the crates.io index.
