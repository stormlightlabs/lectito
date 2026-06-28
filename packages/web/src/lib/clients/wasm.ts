import type { LectitoModule, PipelineFailure, PipelineMetadata, PipelineOptions, PipelineResult } from "../types";
import {
  articleResult,
  elapsedSince,
  emptyToNull,
  firstHeading,
  firstLine,
  markdownWithFrontmatter,
  metadataFromHtml,
  sanitizeHtml,
  textContent,
} from "./shared";

let LECTITO_MOD: Promise<LectitoModule> | undefined;

const WAS_MOD_URL = "/lectito-wasm/lectito_wasm.js";

type FragmentOptions = { markup: string; srcMetadata: Partial<PipelineMetadata>; note: string; start: number };

function fragmentResult(mod: LectitoModule, opts: FragmentOptions): PipelineResult {
  const text = textContent(opts.markup);
  const bodyMarkdown = mod.htmlToMarkdown(opts.markup);
  const metadata: PipelineMetadata = {
    ...opts.srcMetadata,
    title: opts.srcMetadata.title || firstHeading(opts.markup) || "HTML fragment",
    length: text.length,
    excerpt: opts.srcMetadata.excerpt || opts.srcMetadata.description || firstLine(text),
  };

  return {
    sanitizedHtml: opts.markup,
    cleanedHtml: opts.markup,
    markdown: markdownWithFrontmatter(bodyMarkdown, metadata),
    previewHtml: sanitizeHtml(mod.markdownToHtml(bodyMarkdown)),
    mode: "fragment",
    source: "html",
    elapsedMs: elapsedSince(opts.start),
    metadata,
    diagnostics: JSON.stringify({ fallback: "fragment", note: opts.note }, null, 2),
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

export async function extractHtmlWithWasm(
  html: string,
  opts: PipelineOptions,
): Promise<PipelineResult | PipelineFailure> {
  const start = performance.now();
  const srcMetadata = metadataFromHtml(html);
  const sanitizedHtml = sanitizeHtml(html);

  try {
    const lectito = await loadLectito();
    const report = lectito.extractWithDiagnostics(sanitizedHtml, emptyToNull(opts.baseUrl), {
      charThreshold: opts.charThreshold,
      contentSelector: emptyToNull(opts.contentSelector),
      keepClasses: opts.keepClasses,
    });
    const article = report?.article;

    if (!article || article.length === 0 || article.content.trim().length === 0) {
      return fragmentResult(lectito, {
        markup: sanitizedHtml,
        srcMetadata,
        note: "No readable article was found; converted the sanitized HTML fragment.",
        start,
      });
    }

    const bodyMarkdown = article.markdown || lectito.htmlToMarkdown(article.content);

    return articleResult({
      article: { ...article, markdown: bodyMarkdown },
      source: "html",
      start,
      diagnostics: opts.diagnostics ? report.diagnostics ?? {} : undefined,
      sourceMetadata: srcMetadata,
      previewHtml: lectito.markdownToHtml(bodyMarkdown),
      sanitizedHtml,
    });
  } catch (error) {
    if (LECTITO_MOD) {
      const lectito = await LECTITO_MOD;
      return fragmentResult(lectito, {
        markup: sanitizedHtml,
        srcMetadata,
        note: error instanceof Error ? error.message : "Extraction failed; converted the sanitized HTML fragment.",
        start,
      });
    }

    return {
      sanitizedHtml,
      message: error instanceof Error ? error.message : "Lectito failed before producing output.",
      source: "html",
      elapsedMs: elapsedSince(start),
    };
  }
}
