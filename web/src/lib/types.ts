export type PipelineOptions = { baseUrl: string; contentSelector: string; charThreshold: number; keepClasses: boolean };

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
  metadata: PipelineMetadata;
  diagnostics: string;
};

export type PipelineFailure = { sanitizedHtml: string; message: string };

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
  textContent: string;
  length: number;
  excerpt?: string | null;
  siteName?: string | null;
  publishedTime?: string | null;
  image?: string | null;
  domain?: string | null;
  favicon?: string | null;
};
