import type {
  ApiErrorKind,
  ApiErrorResponse,
  ExtractionRequest,
  ExtractResponse,
  PipelineFailure,
  PipelineResult,
} from "../types";
import { articleResult, emptyToNull, failure } from "./shared";

export const apiBaseUrl = ((import.meta.env.VITE_API_BASE_URL as string | undefined) || "/api").replace(/\/+$/, "");

/** Read the structured error body without throwing on non-JSON responses. */
async function readErrorBody(response: Response): Promise<ApiErrorResponse | undefined> {
  try {
    return await response.json() as ApiErrorResponse;
  } catch {
    return;
  }
}

function messageFromBody(body: ApiErrorResponse | undefined, status: number): string {
  return body?.error?.message || body?.message || `API request failed with status ${status}.`;
}

/** Parse `Retry-After` (seconds or HTTP-date) into a short human hint. */
function retryAfterHint(response: Response): string | undefined {
  const value = response.headers.get("retry-after");
  if (!value) return;

  const seconds = Number(value);
  if (Number.isFinite(seconds) && seconds > 0) {
    if (seconds < 60) return `${seconds}s`;
    return `${Math.ceil(seconds / 60)} min`;
  }

  const date = new Date(value);
  if (Number.isFinite(date.getTime())) {
    return `at ${date.toLocaleTimeString()}`;
  }
}

/**
 * Capture an actionable, classified failure from a non-OK HTTP response.
 * The raw structured error is stashed in `diagnostics` so the Diagnostics tab
 * can show it when the user has diagnostics enabled.
 */
async function httpFailure(response: Response, start: number, diagnosticsEnabled: boolean): Promise<PipelineFailure> {
  const body = await readErrorBody(response);
  const raw = JSON.stringify({ status: response.status, body }, null, 2);

  if (response.status === 429) {
    const hint = retryAfterHint(response);
    const message = hint ? `Rate limited. Try again in ${hint}.` : "Rate limited. Try again shortly.";
    return failure(message, {
      source: "url",
      start,
      kind: "rate-limited",
      diagnostics: diagnosticsEnabled ? raw : undefined,
    });
  }

  const kind: ApiErrorKind = response.status >= 500 ? "unavailable" : "client-error";
  return failure(messageFromBody(body, response.status), {
    source: "url",
    start,
    kind,
    diagnostics: diagnosticsEnabled ? raw : undefined,
  });
}

export async function extractUrlWithApi(request: ExtractionRequest): Promise<PipelineResult | PipelineFailure> {
  const start = performance.now();
  const trimmedUrl = request.url.trim();
  const wantDiagnostics = request.options.diagnostics;

  if (!trimmedUrl) {
    return failure("Enter a URL before running API extraction.", { source: "url", start });
  }

  try {
    const response = await fetch(`${apiBaseUrl}/v1/extract`, {
      method: "POST",
      headers: { "Content-Type": "application/json" },
      body: JSON.stringify({
        url: trimmedUrl,
        options: {
          charThreshold: request.options.charThreshold,
          contentSelector: emptyToNull(request.options.contentSelector),
          keepClasses: request.options.keepClasses,
        },
        diagnostics: wantDiagnostics,
      }),
    });

    if (!response.ok) {
      return httpFailure(response, start, wantDiagnostics);
    }

    const data = await response.json() as ExtractResponse;
    if (!data.article || data.article.length === 0 || data.article.content.trim().length === 0) {
      return failure("No readable article was found for this URL.", { source: "url", start, kind: "extract-failed" });
    }

    return articleResult({
      article: data.article,
      source: "url",
      start,
      diagnostics: data.diagnostics,
      elapsedMs: data.elapsedMs,
      previewHtml: data.article.content,
    });
  } catch (error) {
    /**
     * `fetch` rejects with a TypeError on network failures and aborts.
     *
     * The worker/proxy converts upstream timeouts into a 504, which we
     * already classify as "unavailable" above.
     *
     * A rejection here means the API itself could not be reached.
     */
    const message = error instanceof Error && error.name === "AbortError"
      ? "The request timed out before the API responded."
      : "Could not reach the extraction API. Check your connection and try again.";
    return failure(message, { source: "url", start, kind: "unavailable" });
  }
}
