import { createEffect, createSignal, onCleanup, Show } from "solid-js";
import { CodeEditor } from "./components/CodeEditor";
import { OutputPane, type OutputTab } from "./components/OutputPane";
import { runPipeline } from "./lib/pipeline";
import { sampleHtml } from "./lib/sample";
import type { PipelineFailure, PipelineMetadata, PipelineOptions, PipelineResult } from "./lib/types";

const defaultOptions: PipelineOptions = { baseUrl: "", contentSelector: "", charThreshold: 0, keepClasses: false };

export default function App() {
  const [html, setHtml] = createSignal(sampleHtml);
  const [result, setResult] = createSignal<PipelineResult | PipelineFailure>();
  const [tab, setTab] = createSignal<OutputTab>("markdown");
  const [running, setRunning] = createSignal(false);

  createEffect(() => {
    const input = html();
    const timer = globalThis.setTimeout(() => {
      setRunning(true);
      void runPipeline(input, defaultOptions).then((nextResult) => {
        setResult(nextResult);
        setRunning(false);
      });
    }, 350);

    onCleanup(() => globalThis.clearTimeout(timer));
  });

  return (
    <main class="app-shell">
      <header class="app-header">
        <div>
          <p class="eyebrow">Lectito WASM Example</p>
          <h1>HTML cleanup to Markdown</h1>
        </div>
        <div class="status">
          <span class="status__dot" />
          Paste HTML. Read Markdown.
        </div>
      </header>

      <section class="metadata" aria-label="Current conversion metadata">
        <div>
          <span>Status</span>
          <strong>{running() ? "Converting" : "Ready"}</strong>
        </div>
        <Show
          when={result() && !("message" in result()!)}
          fallback={<MetadataFallback />}>
          <MetadataView result={result() as PipelineResult} />
        </Show>
      </section>

      <section class="workspace" aria-label="HTML cleanup workspace">
        <InputPane html={html()} onInput={setHtml} onReset={() => setHtml(sampleHtml)} />
        <OutputPane result={result()} tab={tab()} onTab={setTab} />
      </section>
    </main>
  );
}

function InputPane(props: { html: string; onInput: (html: string) => void; onReset: () => void }) {
  return (
    <section class="pane pane--input">
      <PaneTitle eyebrow="Input" title="Source HTML" onReset={props.onReset} />
      <CodeEditor value={props.html} language="html" onInput={props.onInput} />
    </section>
  );
}

function PaneTitle(props: { eyebrow: string; title: string; onReset: () => void }) {
  return (
    <div class="pane__header">
      <div>
        <p class="eyebrow">{props.eyebrow}</p>
        <h2>{props.title}</h2>
      </div>
      <button type="button" class="secondary-button" onClick={props.onReset}>
        Reset
      </button>
    </div>
  );
}

function MetadataFallback() {
  const metadata: PipelineMetadata = {
    title: "Waiting for HTML",
    length: 0,
    excerpt: "Paste HTML into the left pane.",
  };

  return <MetadataFields metadata={metadata} mode="-" />;
}

function MetadataView(props: { result: PipelineResult }) {
  return <MetadataFields metadata={props.result.metadata} mode={props.result.mode} />;
}

function MetadataFields(props: { metadata: PipelineMetadata; mode: string }) {
  return (
    <>
      <MetadataItem label="Title" value={props.metadata.title} />
      <MetadataItem label="Length" value={props.metadata.length.toLocaleString()} />
      <MetadataItem label="Mode" value={props.mode} />
      <MetadataItem label="Excerpt" value={props.metadata.excerpt} multiline />
    </>
  );
}

function MetadataItem(props: { label: string; value: string; multiline?: boolean }) {
  return (
    <div>
      <span>{props.label}</span>
      <Show when={props.multiline} fallback={<strong>{props.value}</strong>}>
        <p>{props.value}</p>
      </Show>
    </div>
  );
}
