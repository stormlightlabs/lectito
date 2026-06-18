import { InputPane } from "$components/Input";
import { OutputPane } from "$components/Output";
import { MotionButton } from "$components/shared/Motion";
import { StatusStrip } from "$components/Status";
import { extractUrlWithApi } from "$lib/clients/api";
import { extractHtmlWithWasm } from "$lib/clients/wasm";
import { sampleHtml, sampleHtmlFixtures, sampleUrls } from "$lib/sample";
import type { AppMode, InspectTab, OutputTab, PipelineFailure, PipelineOptions, PipelineResult } from "$lib/types";
import { createSignal, For } from "solid-js";

const defaultOptions: PipelineOptions = {
  baseUrl: "",
  contentSelector: "",
  charThreshold: 0,
  keepClasses: false,
  diagnostics: false,
};

type AppModeTabsProps = { mode: AppMode; onMode: (mode: AppMode) => void };

const modes: Array<{ id: AppMode; label: string }> = [{ id: "url", label: "URL" }, { id: "html", label: "HTML" }];

function AppModeTabs(props: AppModeTabsProps) {
  return (
    <div class="mode-tabs" role="tablist" aria-label="Extraction mode">
      <For each={modes}>
        {(item) => (
          <MotionButton
            type="button"
            classList={{ "is-active": props.mode === item.id }}
            onClick={() => props.onMode(item.id)}>
            {item.label}
          </MotionButton>
        )}
      </For>
    </div>
  );
}

export function WorkbenchPage() {
  const [mode, setMode] = createSignal<AppMode>("html");
  const [html, setHtml] = createSignal(sampleHtml);
  const [url, setUrl] = createSignal(sampleUrls[0]?.url ?? "");
  const [options, setOptions] = createSignal<PipelineOptions>(defaultOptions);
  const [result, setResult] = createSignal<PipelineResult | PipelineFailure>();
  const [tab, setTab] = createSignal<OutputTab>("markdown");
  const [inspectTab, setInspectTab] = createSignal<InspectTab>("metadata");
  const [inspectOpen, setInspectOpen] = createSignal(false);
  const [running, setRunning] = createSignal(false);
  let runId = 0;

  const setAppMode = (nextMode: AppMode) => {
    setMode(nextMode);
    setResult(undefined);
    setTab("markdown");
    setInspectOpen(false);
  };

  const runExtraction = async () => {
    const currentMode = mode();
    const input = currentMode === "html" ? html() : url();
    const currentOptions = options();
    const currentRun = ++runId;

    setRunning(true);
    const nextResult =
      await (currentMode === "html"
        ? extractHtmlWithWasm(input, currentOptions)
        : extractUrlWithApi({ url: input, options: currentOptions }));

    if (currentRun === runId) {
      setResult(nextResult);
      setRunning(false);
    }
  };

  return (
    <main class="app-shell">
      <header class="app-header">
        <div>
          <p class="eyebrow">Lectito</p>
          <h1>Extract clean articles</h1>
        </div>
        <AppModeTabs mode={mode()} onMode={setAppMode} />
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
          sampleHtml={sampleHtmlFixtures}
          sampleUrls={sampleUrls}
          onOptions={setOptions}
          onReset={() => setHtml(sampleHtml)}
          onRun={() => void runExtraction()}
          running={running()} />
        <OutputPane
          result={result()}
          tab={tab()}
          inspectTab={inspectTab()}
          inspectOpen={inspectOpen()}
          onTab={setTab}
          onInspectTab={setInspectTab}
          onToggleInspect={() => setInspectOpen((open) => !open)} />
      </section>

      <footer class="app-footer">
        <span>Lectito web workbench</span>
        <span>URL extraction uses the configured API; HTML extraction runs locally in WASM.</span>
      </footer>
    </main>
  );
}
