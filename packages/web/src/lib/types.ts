export type AppMode = "url" | "html";

export type OutputTab = "markdown" | "preview" | "cleaned" | "compare";

export type InspectTab = "metadata" | "diagnostics" | "sanitized";

export type Lang = "html" | "markdown" | "plain" | "json";

export type PipelineOptions = {
  baseUrl: string;
  contentSelector: string;
  charThreshold: number;
  keepClasses: boolean;
  diagnostics: boolean;
};

export type PipelineMetadata = {
  title: string;
  author?: string;
  site?: string;
  published?: string;
  source?: string;
  domain?: string;
  language?: string;
  description?: string;
  image?: string;
  favicon?: string;
  dir?: string;
  length: number;
  excerpt: string;
};

export type PipelineResult = {
  sanitizedHtml: string;
  cleanedHtml: string;
  markdown: string;
  previewHtml: string;
  mode: "article" | "fragment";
  source: AppMode;
  elapsedMs: number;
  metadata: PipelineMetadata;
  diagnostics: string;
};

export type PipelineFailure = { sanitizedHtml: string; message: string; source: AppMode; elapsedMs: number };

export type SavedRun = {
  id: string;
  createdAt: string;
  title: string;
  sourceLabel: string;
  input: string;
  options: PipelineOptions;
  result: PipelineResult;
};

export type ExtractionRequest = { url: string; options: PipelineOptions };

export type ExtractResponse = { article?: Article | null; diagnostics?: unknown; elapsedMs: number };

export type ApiErrorResponse = { error?: { code?: string; message?: string }; message?: string };

export type LectitoModule = {
  default: () => Promise<void>;
  extractWithDiagnostics: (
    html: string,
    baseUrl?: string | null,
    options?: Record<string, unknown>,
  ) => ExtractionReport | null;
  cleanHtml: (html: string, baseUrl?: string | null, options?: Record<string, unknown>) => string | null;
  htmlToMarkdown: (html: string) => string;
  markdownToHtml: (markdown: string) => string;
};

export type ExtractionReport = { article?: Article | null; diagnostics?: unknown };

export type Article = {
  title?: string | null;
  byline?: string | null;
  dir?: string | null;
  lang?: string | null;
  content: string;
  markdown: string;
  text_content?: string;
  textContent?: string;
  length: number;
  excerpt?: string | null;
  site_name?: string | null;
  siteName?: string | null;
  published_time?: string | null;
  publishedTime?: string | null;
  image?: string | null;
  domain?: string | null;
  favicon?: string | null;
};

export const APP_MODES = ["html", "url"] as const;

export function isAppMode(value: unknown): value is AppMode {
  return typeof value === "string" && (APP_MODES as readonly string[]).includes(value);
}

const outputTabs: Set<OutputTab> = new Set(["markdown", "preview", "cleaned", "compare"]);
const inspectTabs: Set<InspectTab> = new Set(["metadata", "diagnostics", "sanitized"]);

export function isOutputTab(value: unknown): value is OutputTab {
  return outputTabs.has(value as OutputTab);
}

export function isInspectTab(value: unknown): value is InspectTab {
  return inspectTabs.has(value as InspectTab);
}
