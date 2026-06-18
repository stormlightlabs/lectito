import { createEffect, createSignal, For, onCleanup } from "solid-js";
import { InputPane } from "./components/Input";
import { OutputPane } from "./components/Output";
import { StatusStrip } from "./components/Status";
import { extractUrlWithApi } from "./lib/clients/api";
import { extractHtmlWithWasm } from "./lib/clients/wasm";
import { sampleHtml, sampleUrls } from "./lib/sample";
import type { AppMode, OutputTab, PipelineFailure, PipelineOptions, PipelineResult } from "./lib/types";

const defaultOptions: PipelineOptions = {
  baseUrl: "",
  contentSelector: "",
  charThreshold: 0,
  keepClasses: false,
  diagnostics: true,
};

type AppModeTabsProps = { mode: AppMode; onMode: (mode: AppMode) => void };

function AppModeTabs(props: AppModeTabsProps) {
  const modes: Array<{ id: AppMode; label: string }> = [{ id: "url", label: "URL" }, { id: "html", label: "HTML" }];

  return (
    <div class="mode-tabs" role="tablist" aria-label="Extraction mode">
      <For each={modes}>
        {(item) => (
          <button
            type="button"
            classList={{ "is-active": props.mode === item.id }}
            onClick={() => props.onMode(item.id)}>
            {item.label}
          </button>
        )}
      </For>
    </div>
  );
}

export default function App() {
  const [mode, setMode] = createSignal<AppMode>("html");
  const [html, setHtml] = createSignal(sampleHtml);
  const [url, setUrl] = createSignal(sampleUrls[0] ?? "");
  const [options, setOptions] = createSignal<PipelineOptions>(defaultOptions);
  const [result, setResult] = createSignal<PipelineResult | PipelineFailure>();
  const [tab, setTab] = createSignal<OutputTab>("markdown");
  const [running, setRunning] = createSignal(false);

  createEffect(() => {
    const currentMode = mode();
    const input = currentMode === "html" ? html() : url();
    const currentOptions = options();

    const timer = globalThis.setTimeout(() => {
      setRunning(true);
      const run = currentMode === "html"
        ? extractHtmlWithWasm(input, currentOptions)
        : extractUrlWithApi({ url: input, options: currentOptions });

      void run.then((nextResult) => {
        setResult(nextResult);
        setRunning(false);
      });
    }, currentMode === "html" ? 350 : 150);

    onCleanup(() => globalThis.clearTimeout(timer));
  });

  return (
    <main class="app-shell">
      <header class="app-header">
        <div>
          <p class="eyebrow">Lectito</p>
          <h1>Article Extraction Tools</h1>
        </div>
        <AppModeTabs mode={mode()} onMode={setMode} />
      </header>

      <StatusStrip mode={mode()} running={running()} result={result()} />

      <section class="workspace" aria-label="Extraction workspace">
        <InputPane
          mode={mode()}
          html={html()}
          url={url()}
          options={options()}
          onHtml={setHtml}
          onUrl={setUrl}
          onOptions={setOptions}
          onReset={() => setHtml(sampleHtml)} />
        <OutputPane result={result()} tab={tab()} onTab={setTab} />
      </section>
    </main>
  );
}
