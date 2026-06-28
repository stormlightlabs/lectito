import { type Context, Hono } from "hono";

type App = { Bindings: Env };

const ALLOWED_METHODS = new Set(["GET", "POST", "OPTIONS"]);
const ALLOWED_HEADERS = "content-type, authorization";
const ALLOWED_METHODS_HEADER = "GET, POST, OPTIONS";
const WEB_APP_ORIGIN = "https://lectito.stormlightlabs.org";
const ALLOWED_ORIGINS = new Set([WEB_APP_ORIGIN, "http://localhost:5173", "http://127.0.0.1:5173"]);
const HOP_BY_HOP_HEADERS = new Set([
  "connection",
  "keep-alive",
  "proxy-authenticate",
  "proxy-authorization",
  "te",
  "trailer",
  "transfer-encoding",
  "upgrade",
  "host",
]);

const app = new Hono<App>();

function corsHeaders(requestOrigin: string | null): Headers {
  const allowedOrigin = requestOrigin !== null && ALLOWED_ORIGINS.has(requestOrigin) ? requestOrigin : WEB_APP_ORIGIN;

  return new Headers({
    "access-control-allow-origin": allowedOrigin,
    "access-control-allow-methods": ALLOWED_METHODS_HEADER,
    "access-control-allow-headers": ALLOWED_HEADERS,
    "access-control-max-age": "86400",
    "vary": "Origin",
  });
}

function withCors(headers: Headers, requestOrigin: string | null): Headers {
  const next = new Headers(headers);
  for (const [key, value] of corsHeaders(requestOrigin)) {
    next.set(key, value);
  }
  return next;
}

function jsonError(requestOrigin: string | null, status: number, code: string, message: string): Response {
  const headers = corsHeaders(requestOrigin);
  headers.set("content-type", "application/json; charset=utf-8");
  return new Response(JSON.stringify({ error: { code, message } }), { status, headers });
}

function upstreamUrl(requestUrl: URL, apiOrigin: string): URL {
  const upstream = new URL(apiOrigin);
  upstream.pathname = requestUrl.pathname.replace(/^\/api(?=\/|$)/, "") || "/";
  upstream.search = requestUrl.search;
  return upstream;
}

function upstreamHeaders(request: Request): Headers {
  const headers = new Headers(request.headers);
  for (const header of HOP_BY_HOP_HEADERS) {
    headers.delete(header);
  }

  headers.set("x-forwarded-host", new URL(request.url).host);
  headers.set("x-forwarded-proto", "https");
  headers.set("x-lectito-proxy", "cloudflare-worker");
  return headers;
}

async function proxy(c: Context<App>): Promise<Response> {
  const method = c.req.method.toUpperCase();
  const requestOrigin = c.req.header("Origin") ?? null;

  if (!ALLOWED_METHODS.has(method)) {
    return jsonError(requestOrigin, 405, "method_not_allowed", "Method not allowed.");
  }

  if (method === "OPTIONS") {
    return new Response(null, { status: 204, headers: corsHeaders(requestOrigin) });
  }

  const controller = new AbortController();
  const timeoutMs = Number.parseInt(c.env.UPSTREAM_TIMEOUT_MS, 10);
  const timeout = setTimeout(() => controller.abort(), Number.isFinite(timeoutMs) ? timeoutMs : 25_000);

  try {
    const requestUrl = new URL(c.req.url);
    const target = upstreamUrl(requestUrl, c.env.API_ORIGIN);
    const upstreamRequest = new Request(target, {
      body: method === "GET" ? null : c.req.raw.body,
      headers: upstreamHeaders(c.req.raw),
      method,
      redirect: "manual",
      signal: controller.signal,
    });

    const response = await fetch(upstreamRequest);
    return new Response(response.body, {
      headers: withCors(response.headers, requestOrigin),
      status: response.status,
      statusText: response.statusText,
    });
  } catch (error) {
    if (error instanceof DOMException && error.name === "AbortError") {
      return jsonError(requestOrigin, 504, "upstream_timeout", "The API origin timed out.");
    }

    console.error(JSON.stringify({ error: error instanceof Error ? error.message : "Unknown proxy error" }));
    return jsonError(requestOrigin, 502, "upstream_error", "The API origin could not be reached.");
  } finally {
    clearTimeout(timeout);
  }
}

app.all("/api/*", proxy);
app.all("*", (c) => {
  const requestOrigin = c.req.header("Origin") ?? null;
  return jsonError(requestOrigin, 404, "not_found", "Not found.");
});

export default app;
