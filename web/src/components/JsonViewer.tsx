import { createEffect, createSignal, lazy, Match, Suspense, Switch } from "solid-js";
import { Icon } from "./Icon";

/**
 * Fetch & then pretty-print so the viewer's foldable regions line
 * up with the structure.
 *
 * The OpenAPI handler already serializes with indentation, but
 * normalizing here keeps the viewer robust to minified payloads.
 */
async function fetchSpec(url: string) {
  const resp = await fetch(url);
  if (!resp.ok) {
    throw new Error(`Request failed with ${resp.status}`);
  }

  const parsed = await resp.json();
  return JSON.stringify(parsed, null, 2);
}

const CodeEditor = lazy(async () => {
  const module = await import("./CodeEditor");
  return { default: module.CodeEditor };
});

function EditorFallback() {
  return <div class="editor-loading">Loading viewer…</div>;
}

export type JsonViewerProps = {
  /** Absolute or base-relative URL of the JSON document to render. */
  src: string;
  /** Label shown for the "view raw" link. Defaults to "Download raw spec". */
  linkLabel?: string;
};

type ViewerState = { status: "loading" } | { status: "error"; message: string } | { status: "ready"; text: string };

/**
 * Fetches a JSON document and renders it in a read-only, foldable,
 * searchable CodeMirror viewer.
 *
 * A link to the raw source is included so consumers can open or download
 * the original spec.
 */
export function JsonViewer(props: JsonViewerProps) {
  const [state, setState] = createSignal<ViewerState>({ status: "loading" });
  const src = () => props.src;
  const linkLabel = () => props.linkLabel ?? "Download raw spec";

  createEffect(() => {
    const specURL = src();
    setState({ status: "loading" });

    fetchSpec(specURL).then((text) => setState({ status: "ready", text })).catch((error: unknown) =>
      setState({ status: "error", message: error instanceof Error ? error.message : "Failed to load JSON." })
    );
  });

  return (
    <div class="json-viewer">
      <div class="json-viewer__bar">
        <a class="json-viewer__link" href={props.src} target="_blank" rel="noreferrer">
          <Icon kind="download" />
          {linkLabel()}
        </a>
      </div>
      <Switch>
        <Match when={state().status === "loading"}>
          <div class="json-viewer__status">Loading…</div>
        </Match>
        <Match when={state().status === "error"}>
          <div class="json-viewer__status json-viewer__status--error">
            {(state() as { status: "error"; message: string }).message}
          </div>
        </Match>
        <Match when={state().status === "ready"}>
          <Suspense fallback={<EditorFallback />}>
            <CodeEditor
              value={(state() as { status: "ready"; text: string }).text}
              language="json"
              readonly
              fold
              search
              statusText="OpenAPI JSON" />
          </Suspense>
        </Match>
      </Switch>
    </div>
  );
}
