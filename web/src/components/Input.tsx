import { useWorkbench } from "$lib/workbench/context";
import { createMemo, createSignal, For, lazy, Show, Suspense } from "solid-js";
import type { PipelineOptions, SampleHtml } from "../lib/types";
import type { CodeEditorProps } from "./CodeEditor";
import { Icon } from "./Icon";
import { OptionsPanel } from "./Options";
import { MotionButton, MotionReveal, MotionSwap } from "./shared/Motion";

const CodeEditor = lazy(async () => {
  const module = await import("./CodeEditor");
  return { default: module.CodeEditor };
});

function EditorFallback(props: Pick<CodeEditorProps, "readonly">) {
  return <div class="editor-loading">{props.readonly ? "Loading output..." : "Loading editor..."}</div>;
}

function HtmlEditor(props: { html: string; statusText: string; onHtml: (html: string) => void }) {
  return (
    <Suspense fallback={<EditorFallback />}>
      <CodeEditor value={props.html} language="html" statusText={props.statusText} onInput={props.onHtml} />
    </Suspense>
  );
}

function InputToolbar(
  props: {
    html: string;
    running: boolean;
    samples: SampleHtml[];
    onHtml: (html: string) => void;
    onCancel: () => void;
    onReset: () => void;
    onRun: () => void;
  },
) {
  const size = createMemo(() => new Blob([props.html]).size);
  let fileInput: HTMLInputElement | undefined;

  const importFile = async (file?: File) => {
    if (!file) return;
    props.onHtml(await file.text());
    if (fileInput) fileInput.value = "";
  };

  const pasteHtml = async () => {
    const value = await navigator.clipboard?.readText();
    if (value) props.onHtml(value);
  };

  return (
    <div class="pane-toolbar pane-toolbar--input">
      <label class="pane-toolbar__select">
        <select
          id="sample-html"
          onChange={(event) => props.onHtml(props.samples[Number(event.currentTarget.value)]?.html ?? "")}>
          {/* TODO: we should add a readonly -- Samples -- entry */}
          <For each={props.samples}>{(sample, index) => <option value={index()}>{sample.label}</option>}</For>
        </select>
      </label>
      <div class="pane-toolbar__actions">
        <MotionButton type="button" class="button button--secondary" onClick={() => void pasteHtml()}>
          Paste
        </MotionButton>
        <MotionButton type="button" class="button button--primary" disabled={props.running} onClick={props.onRun}>
          <Icon kind="convert" />
          {props.running ? "Converting" : "Convert"}
        </MotionButton>
        <Show when={props.running}>
          <MotionButton
            type="button"
            class="button button--secondary button--icon"
            aria-label="Cancel"
            title="Cancel"
            onClick={props.onCancel}>
            <Icon kind="cancel" />
          </MotionButton>
        </Show>
        <details class="overflow-menu">
          <summary class="button button--secondary button--icon" aria-label="More input actions" title="More actions">
            <Icon kind="more" />
          </summary>
          <div class="overflow-menu__panel">
            <MotionButton type="button" onClick={() => fileInput?.click()}>Import HTML</MotionButton>
            <MotionButton type="button" onClick={props.onReset}>Reset sample</MotionButton>
            <MotionButton type="button" onClick={() => props.onHtml("")}>Clear editor</MotionButton>
          </div>
        </details>
        <input
          ref={(element) => {
            fileInput = element;
          }}
          class="visually-hidden-file"
          type="file"
          accept=".html,.htm,text/html,text/plain"
          onChange={(event) => void importFile(event.currentTarget.files?.[0])} />
      </div>
      <Show when={size() > 500_000}>
        <p class="pane-toolbar__warning">Large input may take longer to parse.</p>
      </Show>
    </div>
  );
}

function AdvancedOptions(props: { options: PipelineOptions; onOptions: (options: PipelineOptions) => void }) {
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

export function InputPane() {
  const workbench = useWorkbench();

  return (
    <section class="pane pane--input">
      <div class="input-stack">
        <MotionSwap viewKey="html" class="input-mode">
          <div class="html-input">
            <InputToolbar
              html={workbench.state.html}
              running={workbench.state.running}
              samples={workbench.sampleHtml}
              onHtml={workbench.setHtml}
              onCancel={workbench.cancelRun}
              onReset={workbench.resetInput}
              onRun={() => void workbench.runExtraction()} />
            <HtmlEditor html={workbench.state.html} statusText={workbench.statusText()} onHtml={workbench.setHtml} />
          </div>
        </MotionSwap>
        <AdvancedOptions options={workbench.state.options} onOptions={workbench.setOptions} />
      </div>
    </section>
  );
}
