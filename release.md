# Release Checklist

This repo publishes four crates:

- `lectito`: Rust library crate.
- `lectito-cli`: Cargo package that installs the `lectito` binary.
- `lectito-mcp`: Cargo package that installs the `lectito-mcp` stdio server.
- `lectito-wasm`: Rust crate for JavaScript and WebAssembly bindings.

## Version Checklist

Choose the target version before changing manifests:

```text
target version: 0.x.y
```

Workspace crates:

| Package                 | Publish | Version source              | Release action                         |
| ----------------------- | ------- | --------------------------- | -------------------------------------- |
| `lectito`               | yes     | `workspace.package.version` | Publish first.                         |
| `lectito-cli`           | yes     | `workspace.package.version` | Publish after `lectito` is available.  |
| `lectito-api`           | no      | `workspace.package.version` | Keep in sync for deployed builds.      |
| `lectito-fixtures`      | no      | `workspace.package.version` | Keep in sync for workspace tests.      |
| `lectito-mcp`           | yes     | `workspace.package.version` | Publish after `lectito` is available.  |
| `lectito-wasm`          | yes     | `workspace.package.version` | Publish after `lectito` is available.  |
| `lectito-basic-example` | no      | `0.0.0`                     | Leave unchanged unless it is packaged. |

Version bump steps:

1. Update `workspace.package.version` in `Cargo.toml`.
2. Update `lectito` dependency versions in `crates/cli/Cargo.toml`:
   runtime dependency and build dependency.
3. Update the `lectito` dependency version in `crates/mcp/Cargo.toml`.
4. Update the `lectito` dependency version in `crates/wasm/Cargo.toml`.
5. Leave unpublished path-only dependencies in `lectito-api`, and
   `lectito-fixtures` as path dependencies.
6. Run `cargo check --workspace` once after manifest edits so `Cargo.lock`
   records the new package versions.
7. Confirm README install snippets and docs mention any new feature flags for
   the release. For this release, confirm `lectito-cli --features pdf`.
8. If publishing npm, make the generated package version match the Cargo
   target version before `npm pack` and `npm publish`.
9. Record any intentionally skipped package in the release notes.

## Before Publishing

- Confirm the working tree only contains intended release changes.
- Confirm versions in the workspace and dependent crate manifests.
- Confirm crate metadata: description, license, README, repository, homepage,
  keywords, and categories.
- Run the Rust checks:

  ```sh
  cargo check --workspace
  cargo test --workspace
  cargo doc --no-deps -p lectito
  cargo doc --no-deps -p lectito-cli
  cargo doc --no-deps -p lectito-mcp
  cargo doc --no-deps -p lectito-wasm
  ```

- Run stricter docs checks when changing public APIs:

  ```sh
  cargo rustdoc -p lectito --lib -- -D missing_docs
  cargo rustdoc -p lectito-wasm --lib -- -D missing_docs
  ```

- Run the wasm release checks:

  ```sh
  pnpm --dir web exec wasm-pack test --node ../crates/wasm
  pnpm --dir web exec wasm-pack build ../crates/wasm --target bundler --out-dir ../../target/wasm-pack/bundler
  pnpm --dir web exec wasm-pack build ../crates/wasm --target web --out-dir ../../target/wasm-pack/web
  pnpm --dir web exec wasm-pack build ../crates/wasm --target nodejs --out-dir ../../target/wasm-pack/nodejs
  ```

- Inspect package contents:

  ```sh
  cargo package --allow-dirty --list -p lectito
  cargo package --allow-dirty --list -p lectito-cli
  cargo package --allow-dirty --list -p lectito-mcp
  cargo package --allow-dirty --list -p lectito-wasm
  ```

## Publishing

Publish the library crate before crates that depend on it.

Cargo verifies registry dependencies, so `lectito-cli`, `lectito-mcp`, and
`lectito-wasm` cannot finish packaging until `lectito` exists on crates.io.

1. Dry-run the library crate:

   ```sh
   cargo publish --dry-run -p lectito
   ```

2. Publish the library crate:

   ```sh
   cargo publish -p lectito
   ```

3. Wait for crates.io index propagation.

4. Dry-run and publish the CLI package:

   ```sh
   cargo publish --dry-run -p lectito-cli
   cargo publish -p lectito-cli
   ```

5. Dry-run and publish the MCP package:

   ```sh
   cargo publish --dry-run -p lectito-mcp
   cargo publish -p lectito-mcp
   ```

6. Dry-run and publish the wasm package:

   ```sh
   cargo publish --dry-run -p lectito-wasm
   cargo publish -p lectito-wasm
   ```

## NPM

Publish to npm when the JavaScript/WebAssembly package should be installable
with:

```sh
npm install @stormlightlabs/lectito
```

The crates.io `lectito-wasm` package and the npm `@stormlightlabs/lectito`
package are different artifacts. crates.io gets the Rust crate. npm gets the
generated `wasm-pack` output.

Confirm npm auth before publishing:

```sh
npm whoami
npm org ls stormlightlabs
```

The npm user must belong to the `stormlightlabs` org and have package publish
rights. Scoped public packages must publish with public access.

Build and inspect the bundler package:

```sh
pnpm --dir web exec wasm-pack build ../crates/wasm --target bundler --out-dir ../../target/wasm-pack/bundler
cd target/wasm-pack/bundler
npm pkg set name=@stormlightlabs/lectito publishConfig.access=public
npm pack --dry-run
```

Confirm `package.json` before publishing:

- `name` is `@stormlightlabs/lectito`.
- `version` matches the Rust release version.
- `publishConfig.access` is `public`.
- `license`, `repository`, `homepage`, `types`, and `files` are correct.
- The tarball includes `lectito_wasm_bg.wasm`, JavaScript glue, and
  `lectito_wasm.d.ts`.

Publish:

```sh
npm publish --access public
```

After publishing, test a fresh install in a temporary project before updating
docs to describe npm as live.

## After Publishing

- Confirm crates.io pages render the README for each published crate.
- Confirm docs.rs builds for each published crate.
- Confirm npm renders the README and installs `@stormlightlabs/lectito` when
  publishing the generated wasm package.
- Confirm installation:

  ```sh
  cargo install --force lectito-cli
  lectito --help
  cargo install --force lectito-cli --features pdf
  lectito --help
  cargo install --force lectito-mcp
  printf '%s\n' '{"jsonrpc":"2.0","id":1,"method":"tools/list"}' | lectito-mcp
  ```

- Confirm the generated docs link to the expected public API.
- Tag the release after the crates are visible and installable.
