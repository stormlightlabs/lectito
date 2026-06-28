import type { ApiErrorResponse, ExtractionRequest, ExtractResponse, PipelineFailure, PipelineResult } from "../types";
import { articleResult, emptyToNull, failure } from "./shared";

export const apiBaseUrl = ((import.meta.env.VITE_API_BASE_URL as string | undefined) || "/api").replace(/\/+$/, "");

async function errorMessage(response: Response): Promise<string> {
  try {
    const body = await response.json() as ApiErrorResponse;
    return body.error?.message || body.message || `API request failed with ${response.status}.`;
  } catch {
    return `API request failed with ${response.status}.`;
  }
}

export async function extractUrlWithApi(request: ExtractionRequest): Promise<PipelineResult | PipelineFailure> {
  const start = performance.now();
  const trimmedUrl = request.url.trim();

  if (!trimmedUrl) {
    return failure("Enter a URL before running API extraction.", "url", start);
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
        diagnostics: request.options.diagnostics,
      }),
    });

    if (!response.ok) {
      return failure(await errorMessage(response), "url", start);
    }

    const data = await response.json() as ExtractResponse;
    if (!data.article || data.article.length === 0 || data.article.content.trim().length === 0) {
      return failure("No readable article was found for this URL.", "url", start);
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
    return failure(error instanceof Error ? error.message : "URL extraction failed.", "url", start);
  }
}
