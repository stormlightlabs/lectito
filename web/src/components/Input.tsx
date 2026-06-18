import { createSignal, For, lazy, Show, Suspense } from "solid-js";
import type { AppMode, PipelineOptions } from "../lib/types";
import type { SampleHtml, SampleUrl } from "../lib/types";
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

function UrlInput(props: { url: string; samples: SampleUrl[]; onUrl: (url: string) => void }) {
  return (
    <div class="url-panel">
      <label for="article-url">URL</label>
      <div class="url-panel__row">
        <input
          id="article-url"
          type="url"
          value={props.url}
          placeholder="https://example.com/article"
          onInput={(event) => props.onUrl(event.currentTarget.value)} />
      </div>
      <label for="sample-url">Sample URL</label>
      <select id="sample-url" onChange={(event) => props.onUrl(event.currentTarget.value)}>
        <For each={props.samples}>{(sample) => <option value={sample.url}>{sample.label}</option>}</For>
      </select>
    </div>
  );
}

function HtmlSampleSelect(props: { samples: SampleHtml[]; onHtml: (html: string) => void }) {
  return (
    <div class="sample-row">
      <label for="sample-html">Sample HTML</label>
      <select
        id="sample-html"
        onChange={(event) => props.onHtml(props.samples[Number(event.currentTarget.value)]?.html ?? "")}>
        <For each={props.samples}>{(sample, index) => <option value={index()}>{sample.label}</option>}</For>
      </select>
    </div>
  );
}

function PaneTitle(props: { mode: AppMode; running: boolean; onReset: () => void; onRun: () => void }) {
  return (
    <div class="pane__header">
      <div>
        <p class="eyebrow">Input</p>
        <h2>{props.mode === "html" ? "Source HTML" : "Article URL"}</h2>
      </div>
      <div class="pane__actions">
        <Show when={props.mode === "html"}>
          <MotionButton type="button" class="button button--secondary" onClick={props.onReset}>Reset</MotionButton>
        </Show>
        <MotionButton type="button" class="button button--primary" disabled={props.running} onClick={props.onRun}>
          {props.running ? "Converting" : "Convert"}
        </MotionButton>
      </div>
    </div>
  );
}

type InputPaneProps = {
  mode: AppMode;
  html: string;
  url: string;
  sampleHtml: SampleHtml[];
  sampleUrls: SampleUrl[];
  options: PipelineOptions;
  onHtml: (html: string) => void;
  onUrl: (url: string) => void;
  onOptions: (options: PipelineOptions) => void;
  onReset: () => void;
  onRun: () => void;
  running: boolean;
};

function InputMode(
  props: Pick<InputPaneProps, "html" | "url" | "mode" | "sampleHtml" | "sampleUrls" | "onHtml" | "onUrl">,
) {
  return (
    <MotionSwap viewKey={props.mode} class="input-mode">
      <Show
        when={props.mode === "html"}
        fallback={<UrlInput url={props.url} samples={props.sampleUrls} onUrl={props.onUrl} />}>
        <div class="html-input">
          <HtmlSampleSelect samples={props.sampleHtml} onHtml={props.onHtml} />
          <HtmlEditor html={props.html} onHtml={props.onHtml} />
        </div>
      </Show>
    </MotionSwap>
  );
}

function AdvancedOptions(props: Pick<InputPaneProps, "mode" | "options" | "onOptions">) {
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
        <OptionsPanel options={props.options} mode={props.mode} onChangeOpts={props.onOptions} />
      </MotionReveal>
    </div>
  );
}

export function InputPane(props: InputPaneProps) {
  return (
    <section class="pane pane--input">
      <PaneTitle mode={props.mode} running={props.running} onReset={props.onReset} onRun={props.onRun} />
      <div class="input-stack">
        <InputMode
          mode={props.mode}
          html={props.html}
          url={props.url}
          sampleHtml={props.sampleHtml}
          sampleUrls={props.sampleUrls}
          onHtml={props.onHtml}
          onUrl={props.onUrl} />
        <AdvancedOptions mode={props.mode} options={props.options} onOptions={props.onOptions} />
      </div>
    </section>
  );
}
