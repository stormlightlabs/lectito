import { createMemo, createSignal, For, lazy, Show, Suspense } from "solid-js";
import type { PipelineOptions } from "../lib/types";
import type { SampleHtml } from "../lib/types";
import type { CodeEditorProps } from "./CodeEditor";
import { OptionsPanel } from "./Options";
import { MotionButton, MotionReveal, MotionSwap } from "./shared/Motion";

const CodeEditor = lazy(async () => {
  const module = await import("./CodeEditor");
  return { default: module.CodeEditor };
});

function EditorFallback(props: Pick<CodeEditorProps, "readonly">) {
  return <div class="editor-loading">{props.readonly ? "Loading output..." : "Loading editor..."}</div>;
}

function HtmlEditor(props: { html: string; onHtml: (html: string) => void }) {
  return (
    <Suspense fallback={<EditorFallback />}>
      <CodeEditor value={props.html} language="html" onInput={props.onHtml} />
    </Suspense>
  );
}

function HtmlSampleSelect(props: { html: string; samples: SampleHtml[]; onHtml: (html: string) => void }) {
  const size = createMemo(() => new Blob([props.html]).size);
  const sizeLabel = createMemo(() => {
    const bytes = size();
    if (bytes < 1024) return `${bytes} B`;
    if (bytes < 1024 * 1024) return `${(bytes / 1024).toFixed(1)} KB`;
    return `${(bytes / 1024 / 1024).toFixed(1)} MB`;
  });

  const importFile = (file?: File) => {
    if (!file) return;
    const reader = new FileReader();
    reader.addEventListener("load", () => props.onHtml(String(reader.result ?? "")));
    reader.readAsText(file);
  };

  const pasteHtml = async () => {
    const value = await navigator.clipboard?.readText();
    if (value) props.onHtml(value);
  };

  return (
    <div class="sample-row sample-row--html">
      <div class="sample-row__control">
        <label for="sample-html">Sample HTML</label>
        <select
          id="sample-html"
          onChange={(event) => props.onHtml(props.samples[Number(event.currentTarget.value)]?.html ?? "")}>
          <For each={props.samples}>{(sample, index) => <option value={index()}>{sample.label}</option>}</For>
        </select>
      </div>
      <div class="sample-row__meta">
        <span>Document size {sizeLabel()}</span>
        <Show when={size() > 500_000}>
          <strong>Large input may take longer to parse.</strong>
        </Show>
      </div>
      <div class="sample-row__actions">
        <MotionButton type="button" class="button button--secondary" onClick={() => void pasteHtml()}>
          Paste
        </MotionButton>
        <label class="button button--secondary">
          Import
          <input type="file" accept=".html,.htm,text/html,text/plain" onChange={(event) => importFile(event.currentTarget.files?.[0])} />
        </label>
        <MotionButton type="button" class="button button--secondary" onClick={() => props.onHtml("")}>
          Clear
        </MotionButton>
      </div>
    </div>
  );
}

function PaneTitle(props: { running: boolean; onReset: () => void; onRun: () => void }) {
  return (
    <div class="pane__header">
      <div>
        <p class="eyebrow">Input</p>
        <h2>Source HTML</h2>
      </div>
      <div class="pane__actions">
        <MotionButton type="button" class="button button--secondary" onClick={props.onReset}>Reset</MotionButton>
        <MotionButton type="button" class="button button--primary" disabled={props.running} onClick={props.onRun}>
          {props.running ? "Converting" : "Convert"}
        </MotionButton>
      </div>
    </div>
  );
}

type InputPaneProps = {
  html: string;
  sampleHtml: SampleHtml[];
  options: PipelineOptions;
  onHtml: (html: string) => void;
  onOptions: (options: PipelineOptions) => void;
  onReset: () => void;
  onRun: () => void;
  running: boolean;
};

function InputMode(
  props: Pick<
    InputPaneProps,
    "html" | "sampleHtml" | "onHtml"
  >,
) {
  return (
    <MotionSwap viewKey="html" class="input-mode">
      <div class="html-input">
        <HtmlSampleSelect html={props.html} samples={props.sampleHtml} onHtml={props.onHtml} />
        <HtmlEditor html={props.html} onHtml={props.onHtml} />
      </div>
    </MotionSwap>
  );
}

function AdvancedOptions(props: Pick<InputPaneProps, "options" | "onOptions">) {
  const [advancedOpen, setAdvancedOpen] = createSignal(false);

  return (
    <div class="advanced-control">
      <MotionButton
        type="button"
        class="advanced-control__button"
        aria-expanded={advancedOpen()}
        onClick={() => setAdvancedOpen((open) => !open)}>
        Advanced options
        <span aria-hidden="true">{advancedOpen() ? "Hide" : "Show"}</span>
      </MotionButton>
      <MotionReveal show={advancedOpen()} class="advanced-control__panel">
        <OptionsPanel options={props.options} onChangeOpts={props.onOptions} />
      </MotionReveal>
    </div>
  );
}

export function InputPane(props: InputPaneProps) {
  return (
    <section class="pane pane--input">
      <PaneTitle running={props.running} onReset={props.onReset} onRun={props.onRun} />
      <div class="input-stack">
        <InputMode
          html={props.html}
          sampleHtml={props.sampleHtml}
          onHtml={props.onHtml} />
        <AdvancedOptions options={props.options} onOptions={props.onOptions} />
      </div>
    </section>
  );
}
