import { For, Show } from "solid-js";
import type { OutputTab, PipelineFailure, PipelineMetadata, PipelineResult } from "../lib/types";
import { CodeEditor } from "./CodeEditor";

const tabs: Array<{ id: OutputTab; label: string }> = [
  { id: "markdown", label: "Markdown" },
  { id: "cleaned", label: "HTML" },
  { id: "preview", label: "Preview" },
  { id: "metadata", label: "Metadata" },
  { id: "diagnostics", label: "Diagnostics" },
  { id: "sanitized", label: "Sanitized" },
];

function outputValue(result: PipelineResult, tab: OutputTab): string {
  switch (tab) {
    case "markdown": {
      return result.markdown;
    }
    case "cleaned": {
      return result.cleanedHtml;
    }
    case "metadata": {
      return JSON.stringify(result.metadata, null, 2);
    }
    case "diagnostics": {
      return result.diagnostics;
    }
    case "sanitized": {
      return result.sanitizedHtml;
    }
    default: {
      return result.previewHtml;
    }
  }
}

function selectedLabel(tab: OutputTab): string {
  return tabs.find((item) => item.id === tab)?.label ?? "Output";
}

function ResultView(props: { result: PipelineResult; tab: OutputTab }) {
  return (
    <Show
      when={props.tab === "preview"}
      fallback={
        <Show
          when={props.tab === "metadata"}
          fallback={
            <CodeEditor
              value={outputValue(props.result, props.tab)}
              readonly
              language={props.tab === "markdown" ? "markdown" : "html"} />
          }>
          <MetadataView metadata={props.result.metadata} />
        </Show>
      }>
      <article class="preview prose" innerHTML={props.result.previewHtml} aria-label="Markdown article preview" />
    </Show>
  );
}

function FailureView(props: { result: PipelineFailure }) {
  return (
    <div class="failure">
      <p>{props.result.message}</p>
      <Show when={props.result.sanitizedHtml}>
        <CodeEditor value={props.result.sanitizedHtml} readonly language="html" />
      </Show>
    </div>
  );
}

function MetadataView(props: { metadata: PipelineMetadata }) {
  const fields = () =>
    [
      ["Title", props.metadata.title],
      ["Author", props.metadata.author],
      ["Site", props.metadata.site],
      ["Published", props.metadata.published],
      ["Domain", props.metadata.domain],
      ["Language", props.metadata.language],
      ["Length", props.metadata.length.toLocaleString()],
      ["Excerpt", props.metadata.excerpt],
    ].filter((item): item is [string, string] => Boolean(item[1]));

  return (
    <dl class="metadata-list">
      <For each={fields()}>
        {(item) => (
          <div>
            <dt>{item[0]}</dt>
            <dd>{item[1]}</dd>
          </div>
        )}
      </For>
    </dl>
  );
}

type OutputPaneProps = { tab: OutputTab; result?: PipelineResult | PipelineFailure; onTab: (tab: OutputTab) => void };

export function OutputPane(props: OutputPaneProps) {
  return (
    <section class="pane pane--output">
      <div class="pane__header">
        <div>
          <p class="eyebrow">Output</p>
          <h2>{selectedLabel(props.tab)}</h2>
        </div>
        <div class="tabs" role="tablist" aria-label="Output views">
          <For each={tabs}>
            {(tab) => (
              <button
                type="button"
                classList={{ "is-active": props.tab === tab.id }}
                onClick={() => props.onTab(tab.id)}>
                {tab.label}
              </button>
            )}
          </For>
        </div>
      </div>

      <Show when={props.result} fallback={<div class="empty">Run the pipeline to generate output.</div>}>
        {(result) => (
          <Show when={!("message" in result())} fallback={<FailureView result={result() as PipelineFailure} />}>
            <ResultView result={result() as PipelineResult} tab={props.tab} />
          </Show>
        )}
      </Show>
    </section>
  );
}
