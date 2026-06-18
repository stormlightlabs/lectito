import { Show } from "solid-js";
import type { AppMode, PipelineOptions } from "../lib/types";
import { CodeEditor } from "./CodeEditor";
import { OptionsPanel } from "./Options";

function UrlInput(props: { url: string; onUrl: (url: string) => void }) {
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
      <p>API mode is wired to the final client shape and left stubbed for now.</p>
    </div>
  );
}

function PaneTitle(props: { mode: AppMode; onReset: () => void }) {
  return (
    <div class="pane__header">
      <div>
        <p class="eyebrow">Input</p>
        <h2>{props.mode === "html" ? "Source HTML" : "Article URL"}</h2>
      </div>
      <Show when={props.mode === "html"}>
        <button type="button" class="button button--secondary" onClick={props.onReset}>Reset</button>
      </Show>
    </div>
  );
}

type InputPaneProps = {
  mode: AppMode;
  html: string;
  url: string;
  options: PipelineOptions;
  onHtml: (html: string) => void;
  onUrl: (url: string) => void;
  onOptions: (options: PipelineOptions) => void;
  onReset: () => void;
};

export function InputPane(props: InputPaneProps) {
  return (
    <section class="pane pane--input">
      <PaneTitle mode={props.mode} onReset={props.onReset} />
      <div class="input-stack">
        <Show when={props.mode === "html"} fallback={<UrlInput url={props.url} onUrl={props.onUrl} />}>
          <CodeEditor value={props.html} language="html" onInput={props.onHtml} />
        </Show>
        <OptionsPanel options={props.options} mode={props.mode} onChangeOpts={props.onOptions} />
      </div>
    </section>
  );
}
