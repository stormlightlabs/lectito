export type ExtractFormat = 'html' | 'markdown' | 'text' | 'json';

export type LibrarySort = 'recent' | 'popular' | 'alpha';

export type CachedMetadata = {
  title?: string | null;
  author?: string | null;
  date?: string | null;
  excerpt?: string | null;
  site_name?: string | null;
  language?: string | null;
  word_count?: number | null;
  reading_time_minutes?: number | null;
  image?: string | null;
  favicon?: string | null;
};

export type ExtractResponse = {
  id?: string | null;
  url: string;
  format: ExtractFormat;
  content: string;
  metadata: CachedMetadata;
  cached: boolean;
  extracted_at: string;
};

export type ExtractRequest = {
  url: string;
  format: ExtractFormat;
  include_frontmatter?: boolean;
  include_references?: boolean;
  strip_images?: boolean;
  content_selector?: string | null;
};

export type LibraryItem = {
  id: string;
  url: string;
  domain: string;
  format: ExtractFormat;
  title?: string | null;
  author?: string | null;
  site_name?: string | null;
  favicon?: string | null;
  excerpt?: string | null;
  date?: string | null;
  word_count?: number | null;
  reading_time_minutes?: number | null;
  hit_count: number;
  fetched_at: string;
};

export type TopDomain = { domain: string; count: number };

export type LibraryStats = {
  total_articles: number;
  total_reads: number;
  unique_domains: number;
  total_reading_time_minutes: number;
  top_domains: TopDomain[];
};

export type LibraryResponse = {
  items: LibraryItem[];
  total: number;
  page: number;
  per_page: number;
  stats: LibraryStats;
};

export type LimitsResponse = {
  requests_remaining: number;
  requests_limit: number;
  window_seconds: number;
  reset_at: string;
};

export type RateLimitHeaders = { limit?: number; remaining?: number; reset?: number };

export type HealthResponse = {
  status: 'ok' | 'degraded' | string;
  version: string;
  database: 'ok' | 'unreachable' | string;
};

export type OpenApiReference = { $ref: string };

export type OpenApiSchemaObject = {
  type?: string | string[];
  format?: string;
  enum?: string[];
  default?: unknown;
  description?: string;
  properties?: Record<string, OpenApiSchema>;
  items?: OpenApiSchema;
  required?: string[];
  example?: unknown;
};

export type OpenApiSchema = OpenApiReference | OpenApiSchemaObject;

export type OpenApiParameterObject = {
  name: string;
  in: string;
  required?: boolean;
  description?: string;
  schema?: OpenApiSchema;
};

export type OpenApiParameter = OpenApiReference | OpenApiParameterObject;

export type OpenApiHeaderObject = { description?: string; schema?: OpenApiSchema };

export type OpenApiMediaTypeObject = { schema?: OpenApiSchema; example?: unknown };

export type OpenApiRequestBodyObject = { required?: boolean; content?: Record<string, OpenApiMediaTypeObject> };

export type OpenApiRequestBody = OpenApiReference | OpenApiRequestBodyObject;

export type OpenApiResponseObject = {
  description?: string;
  headers?: Record<string, OpenApiHeaderObject>;
  content?: Record<string, OpenApiMediaTypeObject>;
};

export type OpenApiResponse = OpenApiReference | OpenApiResponseObject;

export type OpenApiOperation = {
  tags?: string[];
  summary?: string;
  description?: string;
  operationId?: string;
  parameters?: OpenApiParameter[];
  requestBody?: OpenApiRequestBody;
  responses: Record<string, OpenApiResponse>;
};

export type OpenApiPathItem = {
  parameters?: OpenApiParameter[];
  get?: OpenApiOperation;
  post?: OpenApiOperation;
  put?: OpenApiOperation;
  patch?: OpenApiOperation;
  delete?: OpenApiOperation;
};

export type OpenApiTag = { name: string; description?: string };

export type OpenApiComponents = {
  schemas?: Record<string, OpenApiSchemaObject>;
  parameters?: Record<string, OpenApiParameterObject>;
  responses?: Record<string, OpenApiResponseObject>;
};

type OpenApiDocumentInfo = { title: string; version: string; description?: string };

export type OpenApiDocument = {
  openapi: string;
  info: OpenApiDocumentInfo;
  tags?: OpenApiTag[];
  paths: Record<string, OpenApiPathItem>;
  components?: OpenApiComponents;
};
