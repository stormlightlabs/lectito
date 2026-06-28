# API To-Do

Plan for the first hosted API release:

- Run `lectito-api` as the Docker service on Coolify.
- Put Cloudflare in front of it.
- Serve public API traffic from `lectito.stormlightlabs.org/api/*`.
- Keep the browser app and docs on the same hostname.

This keeps the Axum/Tokio API intact while leaving room for a Workers-native
adapter later.

## Public Routing

- `lectito.stormlightlabs.org/` serves the web app.
- `lectito.stormlightlabs.org/docs` serves the mdBook output.
- `lectito.stormlightlabs.org/api/*` proxies to the Coolify API service.

The Cloudflare Worker should strip the `/api` prefix before forwarding:

- `/api/healthz` -> `/healthz`
- `/api/openapi.json` -> `/openapi.json`
- `/api/v1/extract` -> `/v1/extract`
- `/api/v1/evaluate` -> `/v1/evaluate`
- `/api/v1/transform` -> `/v1/transform`

Move the current web `/api` docs page before this goes live. Good targets are
`/api-docs` or a page under `/docs`.

## Coolify Service

- Deploy the root `docker-compose.yml`.
- Keep Redis private on the Compose network.
- Set the service port through the `PORT` environment variable.
- Keep `LECTITO_ALLOWED_ORIGINS` scoped to
  `https://lectito.stormlightlabs.org` for production.
- Keep private-network fetch protection enabled:
  `LECTITO_ALLOW_PRIVATE_NETWORK=false`.
- Start with conservative limits:
  - `LECTITO_MAX_BODY_BYTES=524288`
  - `LECTITO_MAX_FETCH_BYTES=2097152`
  - `LECTITO_REDIRECT_LIMIT=5`
  - `LECTITO_REQUEST_TIMEOUT_SECS=20`
- Add a `/healthz` check in Coolify that uses the existing endpoint.

## Cloudflare Worker Wrapper

- Implement the wrapper in TypeScript using the Workers `fetch` runtime.
- Store the Coolify origin in an environment variable such as `API_ORIGIN`.
- Forward only the methods the API supports: `GET`, `POST`, and `OPTIONS`.
- Return CORS preflight responses at the Worker boundary.
- Strip hop-by-hop headers before forwarding.
- Preserve response status, body, and content type from the origin.
- Add a short upstream timeout so stalled origin requests do not pin Worker
  execution.

The web client should default to same-origin API calls:

```ts
const apiBaseUrl =
  (import.meta.env.VITE_API_BASE_URL as string | undefined) || "/api";
```

## API Product Work

- Keep Redis-backed IP token bucket rate limiting enabled before public launch.
- Add a small benchmark command and fixture set for API latency checks.
- Keep raw HTML extraction endpoints out of the public API until external users
  ask for them. The browser already handles pasted HTML through WASM.
- Keep `/openapi.json` published and update the docs examples to use
  `https://lectito.stormlightlabs.org/api`.
- Add smoke checks for:
  - `/api/healthz`
  - `/api/openapi.json`
  - `/api/v1/transform`
  - `/api/v1/evaluate`
  - `/api/v1/extract`

## Later

- Revisit a Workers-native API only if VPS hosting becomes a real problem.
  The likely shape is a small Workers adapter over `crates/wasm`, not a direct
  port of the Axum crate.
- If the Worker adapter happens, move shared request and response types into a
  small contract layer so Axum, Workers, the web client, and docs do not drift.
