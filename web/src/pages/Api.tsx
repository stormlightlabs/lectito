import { Icon } from "$components/Icon";
import { type MarkdownBlock, parseMarkdown, type TocItem } from "$lib/markdown";
import { createMemo, createSignal, For, type JSX } from "solid-js";
import apiMarkdown from "./api.md?raw";

function renderInline(text: string): Array<string | JSX.Element> {
  const parts: Array<string | JSX.Element> = [];
  const pattern = /(`[^`]+`|\[[^\]]+\]\([^)]+\))/g;
  let cursor = 0;
  let match: RegExpExecArray | null;

  while ((match = pattern.exec(text))) {
    if (match.index > cursor) parts.push(text.slice(cursor, match.index));

    const token = match[0];
    if (token.startsWith("`")) {
      parts.push(<code>{token.slice(1, -1)}</code>);
    } else {
      const link = token.match(/^\[([^\]]+)\]\(([^)]+)\)$/);
      parts.push(link ? <a href={link[2]}>{link[1]}</a> : token);
    }

    cursor = match.index + token.length;
  }

  if (cursor < text.length) parts.push(text.slice(cursor));
  return parts;
}

function CopyButton(props: { value: string; label: string }) {
  const [copied, setCopied] = createSignal(false);
  let timer: number | undefined;

  const copy = async () => {
    await navigator.clipboard?.writeText(props.value);
    setCopied(true);
    if (timer) globalThis.clearTimeout(timer);
    timer = globalThis.setTimeout(() => setCopied(false), 1400);
  };

  return (
    <button type="button" class="api-copy" onClick={() => void copy()} title={copied() ? "Copied" : props.label}>
      <Icon kind="copy" />
    </button>
  );
}

function MarkdownDocument(props: { blocks: MarkdownBlock[] }) {
  return (
    <article class="api-markdown prose">
      <For each={props.blocks}>
        {(block) => {
          switch (block.kind) {
            case "heading": {
              if (block.depth === 1) return <h1 id={block.id}>{renderInline(block.text)}</h1>;
              if (block.depth === 2) return <h2 id={block.id}>{renderInline(block.text)}</h2>;
              return <h3 id={block.id}>{renderInline(block.text)}</h3>;
            }
            case "paragraph": {
              return <p>{renderInline(block.text)}</p>;
            }
            case "list": {
              return (
                <ul>
                  <For each={block.items}>{(item) => <li>{renderInline(item)}</li>}</For>
                </ul>
              );
            }
            case "code": {
              return (
                <figure class="api-code">
                  <figcaption>
                    <span>example</span>
                    <span>
                      <strong>{block.language}</strong>
                      <CopyButton value={block.code} label="Copy" />
                    </span>
                  </figcaption>
                  <pre><code>{block.code}</code></pre>
                </figure>
              );
            }
          }
        }}
      </For>
    </article>
  );
}

export function ApiPage() {
  const blocks = () => parseMarkdown(apiMarkdown);
  // TODO: handle "``"/'code' headings
  const toc = createMemo(() => {
    const b = blocks();
    return b.filter((block): block is Extract<MarkdownBlock, { kind: "heading" }> =>
      block.kind === "heading" && block.depth > 1
    ).map<TocItem>((heading) => ({ id: heading.id, depth: heading.depth, text: heading.text.replaceAll("`", "") }));
  });

  return (
    <main class="api-page">
      <div class="api-body">
        <nav class="api-toc" aria-label="API contents">
          <header class="api-page__bar">
            <p class="eyebrow">API docs</p>
            <CopyButton value={apiMarkdown.trim()} label="Copy Markdown" />
          </header>
          <p>Docs</p>
          <For each={toc()}>
            {(item) => <a classList={{ "is-nested": item.depth > 2 }} href={`#${item.id}`}>{item.text}</a>}
          </For>
        </nav>
        <MarkdownDocument blocks={blocks()} />
      </div>
    </main>
  );
}
