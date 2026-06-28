# To-Dos

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

## Hosted API And Pages Deploy

- [ ] Verify the Coolify API origin directly:

  ```sh
  curl -i https://lectito-api.stormlightlabs.org/healthz
  curl -i https://lectito-api.stormlightlabs.org/openapi.json
  ```

- [ ] Set production API environment variables in Coolify:

  ```text
  LECTITO_ALLOWED_ORIGINS=https://lectito.stormlightlabs.org
  LECTITO_ALLOW_PRIVATE_NETWORK=false
  LECTITO_MAX_BODY_BYTES=524288
  LECTITO_MAX_FETCH_BYTES=2097152
  LECTITO_REDIRECT_LIMIT=5
  LECTITO_REQUEST_TIMEOUT_SECS=20
  ```

- [x] Add Redis to the Coolify API deployment:
  - Run Redis as a private service on the API Docker network.
  - Do not expose the Redis port publicly.
  - Set a small memory cap and rely on key expiry for rate-limit state.
  - Move to Docker Compose in Coolify if that is the cleanest way to keep the
    API and Redis in one deployment.
- [x] Add IP-based Redis token bucket rate limiting to the Docker API:
  - Use one atomic Lua script for refill, decrement, TTL, and retry-after.
  - Key buckets by caller IP and route class.
  - Trust `CF-Connecting-IP` and the leftmost `X-Forwarded-For` only when
    `LECTITO_TRUST_PROXY_HEADERS=true`.
  - Fall back to the socket address for direct traffic.
  - Return structured `429` JSON plus `Retry-After`.
  - Skip `GET /healthz`.
- [x] Start with these API limits:
  - `POST /v1/extract`: 5 requests per minute, burst 5.
  - `POST /v1/evaluate`: 10 requests per minute, burst 10.
  - `POST /v1/transform`: 30 requests per minute, burst 30.
  - All API `POST` requests: 45 requests per minute, burst 45.
  - `GET /openapi.json`: 60 requests per minute, burst 60.
- [x] Add rate-limit environment variables in Coolify after implementation:

  ```text
  LECTITO_RATE_LIMIT_ENABLED=true
  LECTITO_REDIS_URL=redis://lectito-redis:6379
  LECTITO_RATE_LIMIT_PREFIX=lectito:api:rate
  LECTITO_TRUST_PROXY_HEADERS=true
  ```

- [ ] Add the Cloudflare `/api/*` proxy:
  - Match `lectito.stormlightlabs.org/api/*`.
  - Strip `/api` before forwarding.
  - Forward to `https://lectito-api.stormlightlabs.org`.
  - Handle `OPTIONS` before forwarding.
  - Keep the web app API base URL as `/api`.
- [ ] Add the Cloudflare route for the API proxy:

  ```text
  lectito.stormlightlabs.org/api/*
  ```

- [ ] Smoke test public routes:

  ```sh
  curl -i https://lectito.stormlightlabs.org/
  curl -i https://lectito.stormlightlabs.org/docs/
  curl -i https://lectito.stormlightlabs.org/api/healthz
  curl -i https://lectito.stormlightlabs.org/api/openapi.json
  ```

- [ ] Smoke test a public API POST:

  ```sh
  curl -i https://lectito.stormlightlabs.org/api/v1/transform \
    -H "Content-Type: application/json" \
    -d '{"html":"<article><h1>Hello</h1><p>World</p></article>"}'
  ```

- [ ] Smoke test API rate limiting through the public proxy:

  ```sh
  for i in $(seq 1 7); do
    curl -i https://lectito.stormlightlabs.org/api/v1/extract \
      -H "Content-Type: application/json" \
      -d '{"url":"https://example.com"}'
  done
  ```
