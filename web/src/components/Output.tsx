import { createMemo, createSignal, For, lazy, Show, Suspense } from "solid-js";
import type { InspectTab, OutputTab, PipelineFailure, PipelineMetadata, PipelineResult } from "../lib/types";
import type { CodeEditorProps } from "./CodeEditor";
import { MotionButton, MotionReveal, MotionSwap } from "./shared/Motion";

const tabs: Array<{ id: OutputTab; label: string }> = [{ id: "markdown", label: "Markdown" }, {
  id: "preview",
  label: "Preview",
}, { id: "cleaned", label: "HTML" }, { id: "compare", label: "Compare" }];

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

function tabPanelId(tab: string) {
  return `panel-${tab}`;
}

function tabButtonId(tab: string) {
  return `tab-${tab}`;
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

function CompareView(props: { result: PipelineResult; sourceHtml: string }) {
  const panes = () => [
    { label: "Source HTML", value: props.sourceHtml, language: "html" as const },
    { label: "Sanitized HTML", value: props.result.sanitizedHtml, language: "html" as const },
    { label: "Cleaned article HTML", value: props.result.cleanedHtml, language: "html" as const },
    { label: "Markdown", value: props.result.markdown, language: "markdown" as const },
  ];

  return (
    <div class="compare-grid">
      <For each={panes()}>
        {(pane) => (
          <section class="compare-pane">
            <h3>{pane.label}</h3>
            <LazyCodeEditor value={pane.value} readonly language={pane.language} />
          </section>
        )}
      </For>
    </div>
  );
}

function ResultView(props: { result: PipelineResult; sourceHtml: string; tab: OutputTab }) {
  const [readerSize, setReaderSize] = createSignal("regular");
  const [readerWidth, setReaderWidth] = createSignal("measure");

  return (
    <Show
      when={props.tab === "preview"}
      fallback={
        <Show
          when={props.tab === "compare"}
          fallback={
            <LazyCodeEditor
              value={outputValue(props.result, props.tab)}
              readonly
              language={props.tab === "markdown" ? "markdown" : "html"} />
          }>
          <CompareView result={props.result} sourceHtml={props.sourceHtml} />
        </Show>
      }>
      <div class="reader-view">
        <div class="reader-controls" aria-label="Reader controls">
          <button
            type="button"
            classList={{ "is-active": readerSize() === "regular" }}
            onClick={() => setReaderSize("regular")}>
            Regular
          </button>
          <button
            type="button"
            classList={{ "is-active": readerSize() === "large" }}
            onClick={() => setReaderSize("large")}>
            Large
          </button>
          <button
            type="button"
            classList={{ "is-active": readerWidth() === "measure" }}
            onClick={() => setReaderWidth("measure")}>
            Measure
          </button>
          <button
            type="button"
            classList={{ "is-active": readerWidth() === "wide" }}
            onClick={() => setReaderWidth("wide")}>
            Wide
          </button>
        </div>
        <article
          class="preview prose"
          classList={{ "is-large": readerSize() === "large", "is-wide": readerWidth() === "wide" }}
          innerHTML={props.result.previewHtml}
          aria-label="Markdown article preview" />
      </div>
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

function MetadataSummary(props: { result: PipelineResult }) {
  const metadata = () => props.result.metadata;

  return (
    <div class="metadata-summary" aria-label="Result metadata summary">
      <span>{metadata().title || "Untitled"}</span>
      <strong>{metadata().domain || metadata().site || "No domain"}</strong>
      <strong>{metadata().length.toLocaleString()} chars</strong>
      <strong>{props.result.elapsedMs}ms</strong>
    </div>
  );
}

function DiagnosticsView(props: { diagnostics: string }) {
  const parsed = createMemo(() => {
    try {
      const value = JSON.parse(props.diagnostics);
      return value && typeof value === "object" ? value as Record<string, unknown> : undefined;
    } catch {
      return undefined;
    }
  });
  const fallbackReason = () => String(parsed()?.fallback ?? parsed()?.reason ?? "Not reported");
  const warnings = () => {
    const value = parsed()?.warnings;
    return Array.isArray(value) ? value.map(String) : [];
  };
  const timing = () => parsed()?.timing ?? parsed()?.elapsedMs;
  const candidates = () => parsed()?.candidates ?? parsed()?.candidate;

  return (
    <div class="diagnostics-view">
      <section>
        <h3>Summary</h3>
        <p>{props.diagnostics === "Diagnostics disabled." ? props.diagnostics : "Diagnostics data is available for this run."}</p>
      </section>
      <section>
        <h3>Fallback reason</h3>
        <p>{fallbackReason()}</p>
      </section>
      <section>
        <h3>Warnings</h3>
        <Show when={warnings().length > 0} fallback={<p>No warnings reported.</p>}>
          <ul>
            <For each={warnings()}>{(warning) => <li>{warning}</li>}</For>
          </ul>
        </Show>
      </section>
      <section>
        <h3>Timing</h3>
        <pre>{JSON.stringify(timing() ?? "Not reported", null, 2)}</pre>
      </section>
      <section>
        <h3>Candidate details</h3>
        <pre>{JSON.stringify(candidates() ?? "Not reported", null, 2)}</pre>
      </section>
      <section>
        <h3>Raw diagnostics</h3>
        <LazyCodeEditor value={props.diagnostics} readonly language="plain" />
      </section>
    </div>
  );
}

function SanitizedComparison(props: { result: PipelineResult }) {
  return (
    <div class="compare-grid compare-grid--two">
      <section class="compare-pane">
        <h3>Sanitized HTML</h3>
        <LazyCodeEditor value={props.result.sanitizedHtml} readonly language="html" />
      </section>
      <section class="compare-pane">
        <h3>Cleaned article HTML</h3>
        <LazyCodeEditor value={props.result.cleanedHtml} readonly language="html" />
      </section>
    </div>
  );
}

function InspectPanel(props: { result: PipelineResult; tab: InspectTab; onTab: (tab: InspectTab) => void }) {
  const onKeyDown = (event: KeyboardEvent, tab: InspectTab) => {
    const index = inspectTabs.findIndex((item) => item.id === tab);
    const nextIndex = event.key === "ArrowRight" ? index + 1 : event.key === "ArrowLeft" ? index - 1 : index;
    if (nextIndex === index) return;
    event.preventDefault();
    props.onTab(inspectTabs[(nextIndex + inspectTabs.length) % inspectTabs.length].id);
  };

  return (
    <aside class="inspect-panel" aria-label="Extraction details">
      <div class="inspect-panel__tabs" role="tablist" aria-label="Inspect output">
        <For each={inspectTabs}>
          {(tab) => (
            <MotionButton
              type="button"
              id={tabButtonId(`inspect-${tab.id}`)}
              role="tab"
              aria-selected={props.tab === tab.id}
              aria-controls={tabPanelId(`inspect-${tab.id}`)}
              tabindex={props.tab === tab.id ? 0 : -1}
              classList={{ "is-active": props.tab === tab.id }}
              onKeyDown={(event) => onKeyDown(event, tab.id)}
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
            <Show
              when={props.tab === "diagnostics"}
              fallback={<SanitizedComparison result={props.result} />}>
              <DiagnosticsView diagnostics={inspectValue(props.result, props.tab)} />
            </Show>
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
  sourceHtml: string;
  layout: "split" | "wide-output" | "input-collapsed";
  fullscreen: boolean;
  onInspectTab: (tab: InspectTab) => void;
  onToggleInspect: () => void;
  onLayout: (layout: "split" | "wide-output" | "input-collapsed") => void;
  onToggleFullscreen: () => void;
  onCopyMarkdown: () => void;
  onCopyHtml: () => void;
  onCopyMetadata: () => void;
  onDownload: () => void;
  onOpenPreview: () => void;
};

export function OutputPane(props: OutputPaneProps) {
  const resultValue = () => props.result && !("message" in props.result) ? props.result : undefined;
  const onKeyDown = (event: KeyboardEvent, tab: OutputTab) => {
    const index = tabs.findIndex((item) => item.id === tab);
    const nextIndex = event.key === "ArrowRight" ? index + 1 : event.key === "ArrowLeft" ? index - 1 : index;
    if (nextIndex === index) return;
    event.preventDefault();
    props.onTab(tabs[(nextIndex + tabs.length) % tabs.length].id);
  };

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
                  id={tabButtonId(tab.id)}
                  role="tab"
                  aria-selected={props.tab === tab.id}
                  aria-controls={tabPanelId(tab.id)}
                  tabindex={props.tab === tab.id ? 0 : -1}
                  classList={{ "is-active": props.tab === tab.id }}
                  onKeyDown={(event) => onKeyDown(event, tab.id)}
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

      <Show when={resultValue()}>
        {(result) => <MetadataSummary result={result()} />}
      </Show>

      <Show when={resultValue()}>
        <div class="output-actions" aria-label="Output actions">
          <MotionButton type="button" class="button button--secondary" onClick={props.onCopyMarkdown}>
            Copy Markdown
          </MotionButton>
          <MotionButton type="button" class="button button--secondary" onClick={props.onCopyHtml}>
            Copy HTML
          </MotionButton>
          <MotionButton type="button" class="button button--secondary" onClick={props.onCopyMetadata}>
            Copy metadata
          </MotionButton>
          <MotionButton type="button" class="button button--secondary" onClick={props.onDownload}>
            Download
          </MotionButton>
          <MotionButton type="button" class="button button--secondary" onClick={props.onOpenPreview}>
            Open preview
          </MotionButton>
          <MotionButton type="button" class="button button--secondary" onClick={props.onToggleFullscreen}>
            {props.fullscreen ? "Exit fullscreen" : "Fullscreen"}
          </MotionButton>
          <select
            aria-label="Layout"
            value={props.layout}
            onChange={(event) => props.onLayout(event.currentTarget.value as OutputPaneProps["layout"])}>
            <option value="split">Split layout</option>
            <option value="wide-output">Wide output</option>
            <option value="input-collapsed">Collapse input</option>
          </select>
        </div>
      </Show>

      <div class="output-layout" classList={{ "has-inspect": props.inspectOpen }}>
        <Show
          when={props.result}
          fallback={<div class="empty">Convert a URL or HTML document to generate output.</div>}>
          {(result) => (
            <Show when={!("message" in result())} fallback={<FailureView result={result() as PipelineFailure} />}>
              <MotionSwap viewKey={props.tab} class="output-view">
                <div
                  id={tabPanelId(props.tab)}
                  role="tabpanel"
                  aria-labelledby={tabButtonId(props.tab)}
                  class="output-tabpanel">
                  <ResultView result={result() as PipelineResult} sourceHtml={props.sourceHtml} tab={props.tab} />
                </div>
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
