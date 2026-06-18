import type { PipelineFailure, UrlExtractionRequest } from "../types";

const apiBaseUrl = import.meta.env.VITE_API_BASE_URL as string | undefined;

export async function extractUrlWithApi(request: UrlExtractionRequest): Promise<PipelineFailure> {
  const start = performance.now();
  const trimmedUrl = request.url.trim();

  if (!trimmedUrl) {
    return failure("Enter a URL before running API extraction.", start);
  }

  if (!apiBaseUrl) {
    return failure("Not implemented", start);
  }

  return failure("Not implemented.", start);
}

function failure(message: string, start: number): PipelineFailure {
  return { sanitizedHtml: "", message, source: "url", elapsedMs: Math.max(0, Math.round(performance.now() - start)) };
}
