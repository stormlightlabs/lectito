import DOMPurify from "dompurify";
import type {
  Article,
  LectitoModule,
  PipelineFailure,
  PipelineMetadata,
  PipelineOptions,
  PipelineResult,
} from "../types";

let LECTITO_MOD: Promise<LectitoModule> | undefined;

const WAS_MOD_URL = "/lectito-wasm/lectito_wasm.js";

function fragmentResult(
  mod: LectitoModule,
  markup: string,
  srcMetadata: Partial<PipelineMetadata>,
  note: string,
  start: number,
): PipelineResult {
  const text = textContent(markup);
  const bodyMarkdown = mod.htmlToMarkdown(markup);
  const metadata: PipelineMetadata = {
    ...srcMetadata,
    title: srcMetadata.title || firstHeading(markup) || "HTML fragment",
    length: text.length,
    excerpt: srcMetadata.excerpt || srcMetadata.description || firstLine(text),
  };

  return {
    sanitizedHtml: markup,
    cleanedHtml: markup,
    markdown: markdownWithFrontmatter(bodyMarkdown, metadata),
    previewHtml: DOMPurify.sanitize(mod.markdownToHtml(bodyMarkdown)),
    mode: "fragment",
    source: "html",
    elapsedMs: elapsedSince(start),
    metadata,
    diagnostics: JSON.stringify({ fallback: "fragment", note }, null, 2),
  };
}

async function loadLectito(): Promise<LectitoModule> {
  LECTITO_MOD ??= browserImport(WAS_MOD_URL).then(async (module: LectitoModule) => {
    await module.default();
    return module;
  });

  return LECTITO_MOD;
}

function browserImport(url: string): Promise<LectitoModule> {
  const importer = new Function("url", "return import(url)") as (url: string) => Promise<LectitoModule>;
  return importer(url);
}

function emptyToNull(value: string): string | null {
  const trimmed = value.trim();
  return trimmed.length > 0 ? trimmed : null;
}

function elapsedSince(start: number): number {
  return Math.max(0, Math.round(performance.now() - start));
}

function firstLine(value: string): string {
  return value.trim().split(/\n+/)[0] ?? "";
}

function firstHeading(html: string): string {
  return new DOMParser().parseFromString(html, "text/html").querySelector("h1, h2, h3")?.textContent?.trim() ?? "";
}

function textContent(html: string): string {
  return new DOMParser().parseFromString(html, "text/html").body.textContent?.replaceAll(/\s+/g, " ").trim() ?? "";
}

function metadataFromArticle(article: Article, sourceMetadata: Partial<PipelineMetadata>): PipelineMetadata {
  return {
    title: article.title || sourceMetadata.title || firstHeading(article.content) || "Untitled",
    author: article.byline || sourceMetadata.author,
    site: article.siteName || sourceMetadata.site,
    published: article.publishedTime || sourceMetadata.published,
    domain: article.domain || sourceMetadata.domain,
    language: article.lang || sourceMetadata.language,
    description: article.excerpt || sourceMetadata.description,
    image: article.image || sourceMetadata.image,
    favicon: article.favicon || sourceMetadata.favicon,
    dir: article.dir || sourceMetadata.dir,
    length: article.length,
    excerpt: article.excerpt || sourceMetadata.excerpt || firstLine(article.textContent),
  };
}

function metadataFromHtml(html: string): Partial<PipelineMetadata> {
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

function markdownWithFrontmatter(markdown: string, metadata: PipelineMetadata): string {
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

export async function extractHtmlWithWasm(
  html: string,
  opts: PipelineOptions,
): Promise<PipelineResult | PipelineFailure> {
  const start = performance.now();
  const sourceMetadata = metadataFromHtml(html);
  const sanitizedHtml = DOMPurify.sanitize(html, { ADD_ATTR: ["target"] });

  try {
    const lectito = await loadLectito();
    const report = lectito.extractWithDiagnostics(sanitizedHtml, emptyToNull(opts.baseUrl), {
      charThreshold: opts.charThreshold,
      contentSelector: emptyToNull(opts.contentSelector),
      keepClasses: opts.keepClasses,
    });
    const article = report?.article;

    if (!article || article.length === 0 || article.content.trim().length === 0) {
      return fragmentResult(
        lectito,
        sanitizedHtml,
        sourceMetadata,
        "No readable article was found; converted the sanitized HTML fragment.",
        start,
      );
    }

    const bodyMarkdown = article.markdown || lectito.htmlToMarkdown(article.content);
    const metadata = metadataFromArticle(article, sourceMetadata);

    return {
      sanitizedHtml,
      cleanedHtml: article.content,
      markdown: markdownWithFrontmatter(bodyMarkdown, metadata),
      previewHtml: DOMPurify.sanitize(lectito.markdownToHtml(bodyMarkdown)),
      mode: "article",
      source: "html",
      elapsedMs: elapsedSince(start),
      metadata,
      diagnostics: opts.diagnostics ? JSON.stringify(report.diagnostics ?? {}, null, 2) : "Diagnostics disabled.",
    };
  } catch (error) {
    if (LECTITO_MOD) {
      const lectito = await LECTITO_MOD;
      return fragmentResult(
        lectito,
        sanitizedHtml,
        sourceMetadata,
        error instanceof Error ? error.message : "Extraction failed; converted the sanitized HTML fragment.",
        start,
      );
    }

    return {
      sanitizedHtml,
      message: error instanceof Error ? error.message : "Lectito failed before producing output.",
      source: "html",
      elapsedMs: elapsedSince(start),
    };
  }
}
