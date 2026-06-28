import DOMPurify from "dompurify";
import type { AppMode, Article, PipelineFailure, PipelineMetadata, PipelineResult } from "../types";

const sanitizeOptions = { ADD_ATTR: ["target"] };

const domParser = new DOMParser();

export function emptyToNull(value: string): string | null {
  const trimmed = value.trim();
  return trimmed.length > 0 ? trimmed : null;
}

export function elapsedSince(start: number, fallbackMs?: number): number {
  return Number.isFinite(fallbackMs)
    ? Math.max(0, Math.round(fallbackMs ?? 0))
    : Math.max(0, Math.round(performance.now() - start));
}

export function failure(message: string, source: AppMode, start: number): PipelineFailure {
  return { sanitizedHtml: "", message, source, elapsedMs: elapsedSince(start) };
}

export function firstLine(value: string): string {
  return value.trim().split(/\n+/)[0] ?? "";
}

export function firstHeading(html: string): string {
  return domParser.parseFromString(html, "text/html").querySelector("h1, h2, h3")?.textContent?.trim() ?? "";
}

export function textContent(html: string): string {
  return domParser.parseFromString(html, "text/html").body.textContent?.replaceAll(/\s+/g, " ").trim() ?? "";
}

export function sanitizeHtml(html: string): string {
  return DOMPurify.sanitize(html, sanitizeOptions);
}

export function diagnosticsText(diagnostics: unknown): string {
  return diagnostics ? JSON.stringify(diagnostics, null, 2) : "Diagnostics disabled.";
}

export function metadataFromArticle(
  article: Article,
  sourceMetadata: Partial<PipelineMetadata> = {},
): PipelineMetadata {
  const siteName = article.site_name ?? article.siteName;
  const publishedTime = article.published_time ?? article.publishedTime;
  const articleText = article.text_content ?? article.textContent ?? "";

  return {
    title: article.title || sourceMetadata.title || firstHeading(article.content) || "Untitled",
    author: article.byline || sourceMetadata.author,
    site: siteName || sourceMetadata.site,
    published: publishedTime || sourceMetadata.published,
    source: sourceMetadata.source,
    domain: article.domain || sourceMetadata.domain,
    language: article.lang || sourceMetadata.language,
    description: article.excerpt || sourceMetadata.description,
    image: article.image || sourceMetadata.image,
    favicon: article.favicon || sourceMetadata.favicon,
    dir: article.dir || sourceMetadata.dir,
    length: article.length,
    excerpt: article.excerpt || sourceMetadata.excerpt || firstLine(articleText),
  };
}

export function metadataFromHtml(html: string): Partial<PipelineMetadata> {
  const document = new DOMParser().parseFromString(html, "text/html");
  const canonical = attr(document, "link[rel='canonical']", "href");
  const url = canonical ? safeUrl(canonical) : undefined;

  return {
    title: meta(document, "og:title") || meta(document, "twitter:title") || document.title.trim() || firstHeading(html),
    author: metaName(document, "author") || meta(document, "article:author"),
    site: meta(document, "og:site_name"),
    published: meta(document, "article:published_time") || metaName(document, "date"),
    source: canonical,
    domain: url?.hostname,
    language: document.documentElement.lang || undefined,
    description: meta(document, "og:description") || metaName(document, "description"),
    image: meta(document, "og:image") || meta(document, "twitter:image"),
    favicon: attr(document, "link[rel~='icon']", "href"),
    dir: document.documentElement.dir || undefined,
  };
}

export function markdownWithFrontmatter(markdown: string, metadata: PipelineMetadata): string {
  const frontmatter = [
    tomlField("title", metadata.title),
    tomlField("author", metadata.author),
    tomlField("site", metadata.site),
    tomlField("published", metadata.published),
    tomlField("source", metadata.source),
    tomlField("domain", metadata.domain),
    tomlField("language", metadata.language),
    tomlField("description", metadata.description || metadata.excerpt),
    tomlField("image", metadata.image),
    tomlField("favicon", metadata.favicon),
    tomlField("dir", metadata.dir),
    `length = ${metadata.length}`,
  ].filter(Boolean).join("\n");

  return `+++\n${frontmatter}\n+++\n\n${markdown}`;
}

type ArticleParams = {
  article: Article;
  source: AppMode;
  start: number;
  diagnostics: unknown;
  sourceMetadata?: Partial<PipelineMetadata>;
  elapsedMs?: number;
  previewHtml: string;
  sanitizedHtml?: string;
};

export function articleResult(params: ArticleParams): PipelineResult {
  const metadata = metadataFromArticle(params.article, params.sourceMetadata);
  const bodyMarkdown = params.article.markdown;

  return {
    sanitizedHtml: params.sanitizedHtml ?? sanitizeHtml(params.article.content),
    cleanedHtml: params.article.content,
    markdown: markdownWithFrontmatter(bodyMarkdown, metadata),
    previewHtml: sanitizeHtml(params.previewHtml),
    mode: "article",
    source: params.source,
    elapsedMs: elapsedSince(params.start, params.elapsedMs),
    metadata,
    diagnostics: diagnosticsText(params.diagnostics),
  };
}

function tomlField(name: string, value?: string | null): string {
  const trimmed = value?.trim();
  if (!trimmed) return "";
  return `${name} = ${JSON.stringify(trimmed)}`;
}

function meta(document: Document, property: string): string | undefined {
  return attr(document, `meta[property="${property}"]`, "content");
}

function metaName(document: Document, name: string): string | undefined {
  return attr(document, `meta[name="${name}"]`, "content");
}

function attr(document: Document, selector: string, attribute: string): string | undefined {
  return document.querySelector(selector)?.getAttribute(attribute)?.trim() || undefined;
}

function safeUrl(value: string): URL | undefined {
  try {
    return new URL(value);
  } catch {
    return undefined;
  }
}
