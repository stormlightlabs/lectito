import { APP_MODES } from "$lib/types";
import { useWorkbench } from "$lib/workbench/context";
import { Trans } from "@lingui/solid/macro";
import { A } from "@solidjs/router";
import { createMemo, createSignal, For, lazy, Show, Suspense } from "solid-js";
import type { AppMode, PipelineOptions } from "../lib/types";
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

type InputToolbarProps = {
  html: string;
  running: boolean;
  onHtml: (html: string) => void;
  onCancel: () => void;
  onReset: () => void;
  onRun: () => void;
};

function InputToolbar(props: InputToolbarProps) {
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
      <A href="/workbench/samples" class="button button--secondary">Samples</A>
      <div class="pane-toolbar__actions">
        <MotionButton type="button" class="button button--secondary" onClick={() => void pasteHtml()}>
          Paste
        </MotionButton>
        <MotionButton type="button" class="button button--primary" disabled={props.running} onClick={props.onRun}>
          <Icon kind="convert" />
          {props.running ? "Converting" : "Convert"}
        </MotionButton>
        <Show when={props.running}>
          <CancelButton onCancel={props.onCancel} />
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
  const [isOpen, setIsOpen] = createSignal(false);
  return (
    <div class="advanced-control">
      <MotionButton
        type="button"
        class="advanced-control__button"
        aria-expanded={isOpen()}
        onClick={() => setIsOpen((open) => !open)}>
        Advanced options
        <span aria-hidden="true">{isOpen() ? "Hide" : "Show"}</span>
      </MotionButton>
      <MotionReveal show={isOpen()} class="advanced-control__panel">
        <OptionsPanel options={props.options} onChangeOpts={props.onOptions} />
      </MotionReveal>
    </div>
  );
}

function ModeSwitch(props: { mode: AppMode; onMode: (mode: AppMode) => void }) {
  return (
    <div class="mode-tabs" role="tablist" aria-label="Input mode">
      <For each={APP_MODES}>
        {(value) => (
          <button
            type="button"
            role="tab"
            aria-selected={props.mode === value}
            classList={{ "is-active": props.mode === value }}
            onClick={() => props.onMode(value)}>
            {value === "html" ? "HTML" : "URL"}
          </button>
        )}
      </For>
    </div>
  );
}

function UrlStatus(props: { running: boolean }) {
  return (
    <span class="field-message">
      <Show when={props.running} fallback={<Trans>Ready.</Trans>}>
        <Trans>Fetching…</Trans>
      </Show>
    </span>
  );
}

function CancelButton(props: { onCancel: () => void }) {
  return (
    <MotionButton
      type="button"
      class="button button--secondary button--icon"
      aria-label="Cancel"
      title="Cancel"
      onClick={props.onCancel}>
      <Icon kind="cancel" />
    </MotionButton>
  );
}

type UrlPanelProps = {
  url: string;
  running: boolean;
  onUrl: (url: string) => void;
  onRun: () => void;
  onCancel: () => void;
};

function UrlPanel(props: UrlPanelProps) {
  const submit = (event: SubmitEvent) => {
    event.preventDefault();
    if (!props.running) props.onRun();
  };

  return (
    <form class="url-panel" onSubmit={submit}>
      <label>
        <span>
          <Trans>Page URL</Trans>
        </span>
        <input
          type="url"
          inputmode="url"
          autocomplete="url"
          placeholder="https://example.com/article"
          value={props.url}
          disabled={props.running}
          onInput={(event) => props.onUrl(event.currentTarget.value)} />
      </label>
      <p>
        <Trans>Enter a page URL and Lectito extracts the article server-side.</Trans>
      </p>
      <div class="url-panel__row">
        <UrlStatus running={props.running} />
        <div class="pane-toolbar__actions">
          <MotionButton type="submit" class="button button--primary" disabled={props.running || !props.url.trim()}>
            <Icon kind="convert" />
            {props.running ? "Converting" : "Convert"}
          </MotionButton>
          <Show when={props.running}>
            <CancelButton onCancel={props.onCancel} />
          </Show>
        </div>
      </div>
    </form>
  );
}

export function InputPane() {
  const workbench = useWorkbench();
  return (
    <section class="pane pane--input">
      <div class="input-stack">
        <header class="pane-toolbar pane-toolbar--mode">
          <ModeSwitch mode={workbench.state.mode} onMode={workbench.setMode} />
        </header>
        <MotionSwap viewKey={workbench.state.mode} class="input-mode">
          <Show
            when={workbench.state.mode === "url"}
            fallback={
              <div class="html-input">
                <InputToolbar
                  html={workbench.state.html}
                  running={workbench.state.running}
                  onHtml={workbench.setHtml}
                  onCancel={workbench.cancelRun}
                  onReset={workbench.resetInput}
                  onRun={() => void workbench.runExtraction()} />
                <HtmlEditor
                  html={workbench.state.html}
                  statusText={workbench.statusText()}
                  onHtml={workbench.setHtml} />
              </div>
            }>
            <UrlPanel
              url={workbench.state.url}
              running={workbench.state.running}
              onUrl={workbench.setUrl}
              onCancel={workbench.cancelRun}
              onRun={() => void workbench.runExtraction()} />
          </Show>
        </MotionSwap>
        <AdvancedOptions options={workbench.state.options} onOptions={workbench.setOptions} />
      </div>
    </section>
  );
}
