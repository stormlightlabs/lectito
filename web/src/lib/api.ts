import type { ExtractRequest, ExtractResponse, LibraryResponse, LimitsResponse, RateLimitHeaders } from '$lib/types';

type FetchLike = typeof fetch;

export class ApiError extends Error {
  status: number;

  constructor(status: number, message: string) {
    super(message);
    this.name = 'ApiError';
    this.status = status;
  }
}

export interface ApiResult<T> {
  data: T;
  rateLimit: RateLimitHeaders;
}

function buildQuery(params: Record<string, string | number | boolean | null | undefined>) {
  const search = new URLSearchParams();

  for (const [key, value] of Object.entries(params)) {
    if (value === undefined || value === null || value === '') continue;
    search.set(key, String(value));
  }

  const query = search.toString();
  return query ? `?${query}` : '';
}

function parseRateLimit(headers: Headers): RateLimitHeaders {
  const parseHeader = (name: string) => {
    const value = headers.get(name);
    return value ? Number(value) : undefined;
  };

  return {
    limit: parseHeader('x-ratelimit-limit'),
    remaining: parseHeader('x-ratelimit-remaining'),
    reset: parseHeader('x-ratelimit-reset')
  };
}

async function request<T>(fetcher: FetchLike, path: string, init?: RequestInit): Promise<ApiResult<T>> {
  const response = await fetcher(path, init);
  const rateLimit = parseRateLimit(response.headers);

  if (!response.ok) {
    let message = `${response.status} ${response.statusText}`;

    try {
      const payload = (await response.json()) as { error?: string };
      if (payload.error) {
        message = payload.error;
      }
    } catch {
      // Ignore JSON parsing failures for non-JSON error bodies.
    }

    throw new ApiError(response.status, message);
  }

  return { data: (await response.json()) as T, rateLimit };
}

export function getLibrary(
  fetcher: FetchLike,
  params: {
    page?: number;
    per_page?: number;
    sort?: string;
    q?: string;
    domain?: string;
    date_from?: string;
    date_to?: string;
  } = {}
) {
  return request<LibraryResponse>(fetcher, `/api/v1/library${buildQuery(params)}`);
}

export function getLibraryArticle(fetcher: FetchLike, id: string) {
  return request<ExtractResponse>(fetcher, `/api/v1/library/${id}`);
}

export function extractArticle(fetcher: FetchLike, body: ExtractRequest) {
  return request<ExtractResponse>(fetcher, '/api/v1/extract', {
    method: 'POST',
    headers: { 'content-type': 'application/json' },
    body: JSON.stringify(body)
  });
}

export function extractArticleByUrl(
  fetcher: FetchLike,
  params: {
    url: string;
    format: ExtractRequest['format'];
    include_frontmatter?: boolean;
    include_references?: boolean;
    strip_images?: boolean;
  }
) {
  return request<ExtractResponse>(fetcher, `/api/v1/extract${buildQuery(params)}`);
}

export function getLimits(fetcher: FetchLike) {
  return request<LimitsResponse>(fetcher, '/api/v1/limits');
}

export function getApiErrorMessage(error: unknown) {
  if (error instanceof ApiError) {
    return error.message;
  }

  if (error instanceof Error) {
    return error.message;
  }

  return 'An unexpected error occurred.';
}
