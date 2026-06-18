import { For, lazy, Show, Suspense } from "solid-js";
import type { InspectTab, Lang, OutputTab, PipelineFailure, PipelineMetadata, PipelineResult } from "../lib/types";
import type { CodeEditorProps } from "./CodeEditor";
import { MotionButton, MotionReveal, MotionSwap } from "./shared/Motion";

const tabs: Array<{ id: OutputTab; label: string }> = [{ id: "markdown", label: "Markdown" }, {
  id: "preview",
  label: "Preview",
}, { id: "cleaned", label: "HTML" }];

const inspectTabs: Array<{ id: InspectTab; label: string }> = [{ id: "metadata", label: "Metadata" }, {
  id: "diagnostics",
  label: "Diagnostics",
}, { id: "sanitized", label: "Sanitized" }];

function outputValue(result: PipelineResult, tab: OutputTab): string {
  switch (tab) {
    case "markdown": {
      return result.markdown;
    }
    case "cleaned": {
      return result.cleanedHtml;
    }
    default: {
      return result.previewHtml;
    }
  }
}

function selectedLabel(tab: OutputTab): string {
  return tabs.find((item) => item.id === tab)?.label ?? "Output";
}

function inspectValue(result: PipelineResult, tab: InspectTab): string {
  switch (tab) {
    case "metadata": {
      return JSON.stringify(result.metadata, null, 2);
    }
    case "diagnostics": {
      return result.diagnostics;
    }
    case "sanitized": {
      return result.sanitizedHtml;
    }
  }
}

function inspectLanguage(tab: InspectTab): Lang {
  return tab === "metadata" || tab === "diagnostics" ? "plain" : "html";
}

const CodeEditor = lazy(async () => {
  const module = await import("./CodeEditor");
  return { default: module.CodeEditor };
});

function EditorFallback(props: Pick<CodeEditorProps, "readonly">) {
  return <div class="editor-loading">{props.readonly ? "Loading output..." : "Loading editor..."}</div>;
}

function LazyCodeEditor(props: CodeEditorProps) {
  return (
    <Suspense fallback={<EditorFallback readonly={props.readonly} />}>
      <CodeEditor {...props} />
    </Suspense>
  );
}

function ResultView(props: { result: PipelineResult; tab: OutputTab }) {
  return (
    <Show
      when={props.tab === "preview"}
      fallback={
        <LazyCodeEditor
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
      <Show when={props.result.sanitizedHtml}>
        <LazyCodeEditor value={props.result.sanitizedHtml} readonly language="html" />
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

function InspectPanel(props: { result: PipelineResult; tab: InspectTab; onTab: (tab: InspectTab) => void }) {
  return (
    <aside class="inspect-panel" aria-label="Extraction details">
      <div class="inspect-panel__tabs" role="tablist" aria-label="Inspect output">
        <For each={inspectTabs}>
          {(tab) => (
            <MotionButton
              type="button"
              classList={{ "is-active": props.tab === tab.id }}
              onClick={() => props.onTab(tab.id)}>
              {tab.label}
            </MotionButton>
          )}
        </For>
      </div>
      <MotionSwap viewKey={props.tab} class="inspect-panel__body">
        <Show
          when={props.tab === "metadata"}
          fallback={
            <LazyCodeEditor
              value={inspectValue(props.result, props.tab)}
              readonly
              language={inspectLanguage(props.tab)} />
          }>
          <MetadataView metadata={props.result.metadata} />
        </Show>
      </MotionSwap>
    </aside>
  );
}

type OutputPaneProps = {
  tab: OutputTab;
  result?: PipelineResult | PipelineFailure;
  onTab: (tab: OutputTab) => void;
  inspectTab: InspectTab;
  inspectOpen: boolean;
  onInspectTab: (tab: InspectTab) => void;
  onToggleInspect: () => void;
};

export function OutputPane(props: OutputPaneProps) {
  return (
    <section class="pane pane--output">
      <div class="pane__header">
        <div>
          <p class="eyebrow">Output</p>
          <h2>{selectedLabel(props.tab)}</h2>
        </div>
        <div class="pane__header-controls">
          <div class="tabs" role="tablist" aria-label="Output views">
            <For each={tabs}>
              {(tab) => (
                <MotionButton
                  type="button"
                  classList={{ "is-active": props.tab === tab.id }}
                  onClick={() => props.onTab(tab.id)}>
                  {tab.label}
                </MotionButton>
              )}
            </For>
          </div>
          <MotionButton
            type="button"
            class="button button--secondary"
            disabled={!props.result || "message" in props.result}
            aria-expanded={props.inspectOpen}
            onClick={props.onToggleInspect}>
            Inspect
          </MotionButton>
        </div>
      </div>

      <div class="output-layout" classList={{ "has-inspect": props.inspectOpen }}>
        <Show
          when={props.result}
          fallback={<div class="empty">Convert a URL or HTML document to generate output.</div>}>
          {(result) => (
            <Show when={!("message" in result())} fallback={<FailureView result={result() as PipelineFailure} />}>
              <MotionSwap viewKey={props.tab} class="output-view">
                <ResultView result={result() as PipelineResult} tab={props.tab} />
              </MotionSwap>
              <MotionReveal show={props.inspectOpen} class="inspect-reveal">
                <InspectPanel result={result() as PipelineResult} tab={props.inspectTab} onTab={props.onInspectTab} />
              </MotionReveal>
            </Show>
          )}
        </Show>
      </div>
    </section>
  );
}
