export type MediaRetention = "none" | "conservative" | "article" | "all";

export interface ReadabilityOptions {
  maxElemsToParse?: number | null;
  nbTopCandidates?: number;
  charThreshold?: number;
  contentSelector?: string | null;
  siteProfiles?: string[];
  mobileViewportWidth?: number | null;
  classesToPreserve?: string[];
  keepClasses?: boolean;
  disableJsonLd?: boolean;
  linkDensityModifier?: number;
  mediaRetention?: MediaRetention;
}

export interface ReadableOptions {
  minContentLength?: number;
  minScore?: number;
}

export interface MarkdownOptions {
  gfm?: boolean;
  footnotes?: boolean;
  math?: boolean;
  allowRawHtml?: boolean;
}

export type CleanHtmlOptions = ReadabilityOptions;

export interface Article {
  title?: string | null;
  byline?: string | null;
  dir?: string | null;
  lang?: string | null;
  content: string;
  markdown: string;
  text_content: string;
  length: number;
  excerpt?: string | null;
  site_name?: string | null;
  published_time?: string | null;
  image?: string | null;
  domain?: string | null;
  favicon?: string | null;
}

export interface ExtractionReport {
  article: Article | null;
  diagnostics: unknown;
}

export function cleanHtml(html: string, baseUrl?: string | null, options?: CleanHtmlOptions | null): string | null;

export function extract(html: string, baseUrl?: string | null, options?: ReadabilityOptions | null): Article | null;

export function extractWithDiagnostics(
  html: string,
  baseUrl?: string | null,
  options?: ReadabilityOptions | null,
): ExtractionReport;

export function htmlToMarkdown(html: string): string;

export function isProbablyReadable(html: string, options?: ReadableOptions | null): boolean;

export function markdownToHtml(markdown: string, options?: MarkdownOptions | null): string;
