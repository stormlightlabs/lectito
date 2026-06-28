import { Icon } from "$components/Icon";
import { JsonViewer } from "$components/JsonViewer";
import { apiBaseUrl } from "$lib/clients/api";
import { type MarkdownBlock, parseMarkdown, type TocItem } from "$lib/markdown";
import { createMemo, createSignal, For, type JSX, Show } from "solid-js";
import apiMarkdown from "./api.md?raw";

type ApiView = "docs" | "spec";

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
      parts.push(link ? <a href={link[2]} rel="external">{link[1]}</a> : token);
    }

    cursor = match.index + token.length;
  }

  if (cursor < text.length) parts.push(text.slice(cursor));
  return parts;
}

function CopyButton(props: { value: string; label: string; showLabel?: boolean }) {
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
      <Show when={props.showLabel}>
        <span>{copied() ? "Copied" : props.label}</span>
      </Show>
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

function ApiViewToggle(props: { view: ApiView; onView: (view: ApiView) => void }) {
  const tabs: Array<{ id: ApiView; label: string }> = [{ id: "docs", label: "Documentation" }, {
    id: "spec",
    label: "OpenAPI spec",
  }];

  return (
    <div class="api-tabs" role="tablist" aria-label="API view">
      <For each={tabs}>
        {(tab) => (
          <button
            type="button"
            role="tab"
            aria-selected={props.view === tab.id}
            classList={{ "is-active": props.view === tab.id }}
            onClick={() => props.onView(tab.id)}>
            {tab.label}
          </button>
        )}
      </For>
    </div>
  );
}

function DocsToc(props: { items: TocItem[] }) {
  return (
    <nav class="api-toc" aria-label="API contents">
      <p>Docs</p>
      <For each={props.items}>
        {(item) => <a classList={{ "is-nested": item.depth > 2 }} href={`#${item.id}`}>{item.text}</a>}
      </For>
    </nav>
  );
}

// TODO: handle "``"/'code' headings
export function ApiPage() {
  const [view, setView] = createSignal<ApiView>("docs");
  const blocks = () => parseMarkdown(apiMarkdown);
  const toc = createMemo(() => {
    const b = blocks();
    return b.filter((block): block is Extract<MarkdownBlock, { kind: "heading" }> =>
      block.kind === "heading" && block.depth > 1
    ).map<TocItem>((heading) => ({ id: heading.id, depth: heading.depth, text: heading.text.replaceAll("`", "") }));
  });

  return (
    <main class="api-page">
      <header class="api-page__bar">
        <p class="eyebrow">API docs</p>
        <div class="api-page__bar-actions">
          <Show when={view() === "docs"}>
            <CopyButton value={apiMarkdown.trim()} label="Copy Markdown" showLabel />
          </Show>
          <ApiViewToggle view={view()} onView={setView} />
        </div>
      </header>
      <Show when={view() === "docs"} fallback={<JsonViewer src={`${apiBaseUrl}/openapi.json`} />}>
        <div class="api-body">
          <DocsToc items={toc()} />
          <MarkdownDocument blocks={blocks()} />
        </div>
      </Show>
    </main>
  );
}
