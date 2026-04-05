# Lectito API & Web App

`lectito-server` is the Axum backend for the web app. It exposes the API under
`/api/v1` and serves the built frontend from `web/dist` by default.

## Requirements

- PostgreSQL
- Rust toolchain
- `pnpm` for the frontend

## Run It

From the repository root:

```sh
pnpm --dir web install
pnpm --dir web build

export DATABASE_URL=postgres://localhost/lectito
cargo run -p lectito-server
```

The server listens on `0.0.0.0:3000` by default, runs migrations on startup,
and serves both the API and the built web app at `http://localhost:3000`.

## Web App Development

For frontend-only work, you can run:

```sh
pnpm --dir web dev
```

That starts the Vite dev server for the Svelte app. API requests to `/api/*`
are proxied to `http://127.0.0.1:3000` by default, so run `lectito-server` in a
separate terminal for a fully working local stack. Set `LECTITO_SERVER_URL` if
your backend listens elsewhere.

## Useful Environment Variables

- `DATABASE_URL` (required): PostgreSQL connection string
- `LISTEN_ADDR`: socket address to bind, defaults to `0.0.0.0:3000`
- `WEB_DIR`: frontend build directory, defaults to `web/dist`
