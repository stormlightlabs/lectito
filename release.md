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
  pnpm --dir packages/web exec wasm-pack test --node ../../crates/wasm
  pnpm --dir packages/web exec wasm-pack build ../../crates/wasm --target bundler --out-dir ../../target/wasm-pack/bundler
  pnpm --dir packages/web exec wasm-pack build ../../crates/wasm --target web --out-dir ../../target/wasm-pack/web
  pnpm --dir packages/web exec wasm-pack build ../../crates/wasm --target nodejs --out-dir ../../target/wasm-pack/nodejs
  ```

- Inspect package contents:

  ```sh
  cargo package --allow-dirty --list -p lectito
  cargo package --allow-dirty --list -p lectito-cli
  cargo package --allow-dirty --list -p lectito-mcp
  cargo package --allow-dirty --list -p lectito-wasm
  ```

## API

This section covers the public, hosted web-service.

Public routing:

- `https://lectito.stormlightlabs.org/` serves the Cloudflare Pages web app.
- `https://lectito.stormlightlabs.org/docs/*` serves the mdBook output from
  Cloudflare Pages.
- `https://lectito.stormlightlabs.org/api/*` is handled at Cloudflare and
  proxied to the API origin.
- The API origin, Render or Coolify, stays behind Cloudflare. Do not publish it
  as the public API URL in docs or examples.

Build the public site as one Cloudflare Pages artifact:

```sh
pnpm --dir packages/web run build:pages
```

Configure Cloudflare Pages with `packages/web` as the project root, that build
command, and `dist` as the output directory.

Before the public site goes live:

- Keep the web client default API base URL as `/api`.
- Update API examples to use
  `https://lectito.stormlightlabs.org/api/v1/...`.
- Fix stale endpoint names in the web API docs. The current API uses
  `/v1/extract`, `/v1/evaluate`, and `/v1/transform`.
- Confirm `/v1/transform` honors the documented Markdown options before
  documenting those options as public behavior.

Deploy the API origin:

- Use `crates/api/Dockerfile` with the repository root as the Docker build
  context.
- Configure the health check path as `/healthz`.
- If using Coolify, set the app domain to
  `https://lectito-api.stormlightlabs.org`, set the exposed port to `3000`,
  and keep the Cloudflare public API route at
  `https://lectito.stormlightlabs.org/api/*`.
- Upgrade the Render instance before treating the hosted API as production
  infrastructure with latency or availability expectations.
- Set production environment variables:

  ```text
  LECTITO_ALLOWED_ORIGINS=https://lectito.stormlightlabs.org
  LECTITO_ALLOW_PRIVATE_NETWORK=false
  LECTITO_MAX_BODY_BYTES=524288
  LECTITO_MAX_FETCH_BYTES=2097152
  LECTITO_REDIRECT_LIMIT=5
  LECTITO_REQUEST_TIMEOUT_SECS=20
  ```

Configure the Cloudflare API proxy:

- Match `/api/*` before the static app fallback.
- Strip the `/api` prefix before forwarding to the origin:
  `/api/v1/extract` becomes `/v1/extract`.
- Forward only `GET`, `POST`, and `OPTIONS`.
- Return CORS preflight responses at Cloudflare.
- Drop hop-by-hop headers before forwarding.
- Preserve status, response body, and content type from Render.
- Add a short upstream timeout so stalled origin requests do not pin Worker
  execution.
- Add a private `API_ORIGIN` variable for the Coolify origin URL.

Rate limit aggressively at Cloudflare:

- Start with Cloudflare WAF rate limiting rules for `/api/*`.
- Use IP-based counting for the unauthenticated public API. This is blunt and
  can affect shared NATs, but it is acceptable for the first public release if
  the product goal is to protect the origin rather than maximize anonymous
  throughput.
- Exempt or loosen limits for `GET /api/healthz` and `GET /api/openapi.json`.
- Start with separate rules by endpoint class:
  - `/api/v1/extract` and `/api/v1/evaluate`: low allowance, because they fetch
    upstream URLs.
  - `/api/v1/transform`: higher allowance, because it only transforms caller
    supplied HTML.
  - `/api/*`: a broader backstop rule for bursts across all endpoints.
- Initial limits to test:
  - `POST /api/v1/extract`: 5 requests per minute per IP, 60-second mitigation.
  - `POST /api/v1/evaluate`: 10 requests per minute per IP, 60-second
    mitigation.
  - `POST /api/v1/transform`: 30 requests per minute per IP, 60-second
    mitigation.
  - All `POST /api/*`: 45 requests per minute per IP, 60-second mitigation.
  - `GET /api/openapi.json`: 60 requests per minute per IP, log before block if
    Cloudflare plan support allows it.
- Return `429` with a small JSON response for blocked API calls:

  ```json
  { "error": { "code": "rate_limited", "message": "Too many requests." } }
  ```

- Log or count rate-limited requests so the first limits can be tuned from real
  traffic.

Use Cloudflare's Worker Rate Limiting binding when the proxy needs code-level
limits, such as separate limits by `IP + route`, API key, user tier, or request
class. The binding supports simple 10-second or 60-second windows and is fast
enough to run inside the proxy Worker. It is eventually consistent and local to
the Cloudflare location.

Hosted API smoke checks:

```sh
hurl --test scripts/api/healthz.hurl
hurl --test scripts/api/openapi.hurl
hurl --test scripts/api/transform.hurl
hurl --test scripts/api/evaluate.hurl
hurl --test scripts/api/extract.hurl
```

After deploy, verify the public routes:

- `GET https://lectito.stormlightlabs.org/docs/`
- `GET https://lectito.stormlightlabs.org/api/healthz`
- `GET https://lectito.stormlightlabs.org/api/openapi.json`
- `POST https://lectito.stormlightlabs.org/api/v1/transform`
- `POST https://lectito.stormlightlabs.org/api/v1/evaluate`
- `POST https://lectito.stormlightlabs.org/api/v1/extract`

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
pnpm --dir packages/web exec wasm-pack build ../../crates/wasm --target bundler --out-dir ../../target/wasm-pack/bundler
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
