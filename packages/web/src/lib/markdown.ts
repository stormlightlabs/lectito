/**
 * Tiny Markdown parser for `api.md`.
 *
 * This is not a CommonMark parser.
 *
 * Keep the input to the small subset rendered by `Api.tsx`:
 *
 * - headings with one to three `#` markers
 * - paragraphs separated by blank lines; wrapped paragraph lines are joined
 * - flat unordered lists where every item starts with `- `
 * - fenced code blocks with an optional word-like language tag
 * - inline code spans and Markdown links inside text blocks
 *
 * Nested lists, blockquotes, tables, emphasis, images, ordered lists, HTML, and
 * wrapped list items are treated as plain paragraph text or split into separate
 * blocks.
 */

/** Supported Markdown blocks */
export type MarkdownBlock =
  | { kind: "heading"; depth: number; text: string; id: string }
  | { kind: "paragraph"; text: string }
  | { kind: "list"; items: string[] }
  | { kind: "code"; language: string; code: string };

export type TocItem = { id: string; depth: number; text: string };

function slugify(value: string) {
  return value.replaceAll("`", "").toLowerCase().replaceAll(/[^a-z0-9]+/g, "-").replaceAll(/^-|-$/g, "");
}

export function parseMarkdown(markdown: string): MarkdownBlock[] {
  const blocks: MarkdownBlock[] = [];
  const lines = markdown.trim().split(/\r?\n/);
  let index = 0;

  while (index < lines.length) {
    const line = lines[index] ?? "";

    if (!line.trim()) {
      index += 1;
      continue;
    }

    const fence = line.match(/^```(\w+)?$/);
    if (fence) {
      const code: string[] = [];
      index += 1;
      while (index < lines.length && !lines[index]?.startsWith("```")) {
        code.push(lines[index] ?? "");
        index += 1;
      }
      blocks.push({ kind: "code", language: fence[1] ?? "text", code: code.join("\n") });
      index += 1;
      continue;
    }

    const heading = line.match(/^(#{1,3})\s+(.+)$/);
    if (heading) {
      const text = heading[2] ?? "";
      blocks.push({ kind: "heading", depth: heading[1]?.length ?? 2, text, id: slugify(text) });
      index += 1;
      continue;
    }

    if (line.startsWith("- ")) {
      const items: string[] = [];
      while (index < lines.length && lines[index]?.startsWith("- ")) {
        items.push((lines[index] ?? "").slice(2));
        index += 1;
      }
      blocks.push({ kind: "list", items });
      continue;
    }

    const paragraph: string[] = [line.trim()];
    index += 1;
    while (
      index < lines.length
      && lines[index]?.trim()
      && !lines[index]?.startsWith("#")
      && !lines[index]?.startsWith("```")
      && !lines[index]?.startsWith("- ")
    ) {
      paragraph.push((lines[index] ?? "").trim());
      index += 1;
    }
    blocks.push({ kind: "paragraph", text: paragraph.join(" ") });
  }

  return blocks;
}
