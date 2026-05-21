import { Show } from "solid-js";
import type { PipelineFailure, PipelineResult } from "../lib/types";
import { CodeEditor } from "./CodeEditor";

type OutputTab = "sanitized" | "cleaned" | "markdown" | "preview" | "diagnostics";

type Props = { tab: OutputTab; result?: PipelineResult | PipelineFailure; onTab: (tab: OutputTab) => void };

const tabs: Array<{ id: OutputTab; label: string }> = [
  { id: "sanitized", label: "Sanitized" },
  { id: "cleaned", label: "Cleaned" },
  { id: "markdown", label: "Markdown" },
  { id: "preview", label: "Preview" },
  { id: "diagnostics", label: "Diagnostics" },
];

export function OutputPane(props: Props) {
  return (
    <section class="pane pane--output">
      <div class="pane__header">
        <div>
          <p class="eyebrow">Output</p>
          <h2>{selectedLabel(props.tab)}</h2>
        </div>
        <div class="tabs" role="tablist" aria-label="Output views">
          {tabs.map((tab) => (
            <button
              type="button"
              classList={{ "is-active": props.tab === tab.id }}
              onClick={() => props.onTab(tab.id)}>
              {tab.label}
            </button>
          ))}
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

function ResultView(props: { result: PipelineResult; tab: OutputTab }) {
  return (
    <Show
      when={props.tab === "preview"}
      fallback={
        <CodeEditor
          value={outputValue(props.result, props.tab)}
          readonly
          language={props.tab === "markdown" ? "markdown" : "html"} />
      }>
      <article class="preview prose" innerHTML={props.result.previewHtml} aria-label="Markdown article preview" />
    </Show>
  );
}

function FailureView(props: { result: PipelineFailure }) {
  return (
    <div class="failure">
      <p>{props.result.message}</p>
      <CodeEditor value={props.result.sanitizedHtml} readonly language="html" />
    </div>
  );
}

function outputValue(result: PipelineResult, tab: OutputTab): string {
  if (tab === "sanitized") return result.sanitizedHtml;
  if (tab === "cleaned") return result.cleanedHtml;
  if (tab === "markdown") return result.markdown;
  if (tab === "diagnostics") return result.diagnostics;
  return result.previewHtml;
}

function selectedLabel(tab: OutputTab): string {
  return tabs.find((item) => item.id === tab)?.label ?? "Output";
}

export type { OutputTab };
