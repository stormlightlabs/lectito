import { InputPane } from "$components/Input";
import { OutputPane } from "$components/Output";
import { MotionButton } from "$components/shared/Motion";
import { StatusStrip } from "$components/Status";
import { extractHtmlWithWasm } from "$lib/clients/wasm";
import { saveRun } from "$lib/runs";
import { sampleHtml, sampleHtmlFixtures } from "$lib/sample";
import type { InspectTab, OutputTab, PipelineFailure, PipelineOptions, PipelineResult } from "$lib/types";
import { A, useSearchParams } from "@solidjs/router";
import { createEffect, createSignal, Show } from "solid-js";
import { WorkbenchTabs } from "./WorkbenchTabs";

const defaultOptions: PipelineOptions = {
  baseUrl: "",
  contentSelector: "",
  charThreshold: 0,
  keepClasses: false,
  diagnostics: false,
};

const outputTabs: OutputTab[] = ["markdown", "preview", "cleaned", "compare"];
const inspectTabs: InspectTab[] = ["metadata", "diagnostics", "sanitized"];
const layoutModes = ["split", "wide-output", "input-collapsed"] as const;

type LayoutMode = typeof layoutModes[number];

function isOutputTab(value: unknown): value is OutputTab {
  return outputTabs.includes(value as OutputTab);
}

function isInspectTab(value: unknown): value is InspectTab {
  return inspectTabs.includes(value as InspectTab);
}

function isLayoutMode(value: unknown): value is LayoutMode {
  return layoutModes.includes(value as LayoutMode);
}

function selectedOutput(result: PipelineResult, tab: OutputTab): { value: string; extension: string; type: string } {
  if (tab === "cleaned") return { value: result.cleanedHtml, extension: "html", type: "text/html" };
  if (tab === "preview") return { value: result.previewHtml, extension: "html", type: "text/html" };
  if (tab === "compare") {
    return {
      value: JSON.stringify(
        {
          sanitizedHtml: result.sanitizedHtml,
          cleanedHtml: result.cleanedHtml,
          markdown: result.markdown,
          metadata: result.metadata,
        },
        null,
        2,
      ),
      extension: "json",
      type: "application/json",
    };
  }
  return { value: result.markdown, extension: "md", type: "text/markdown" };
}

function CommandBar(
  props: {
    running: boolean;
    hasResult: boolean;
    onRun: () => void;
    onCancel: () => void;
    onReset: () => void;
    onCopy: () => void;
    onDownload: () => void;
    onSave: () => void;
    onShare: () => void;
    onOpen: () => void;
  },
) {
  return (
    <div class="command-bar" aria-label="Workbench commands">
      <MotionButton type="button" class="button button--primary" disabled={props.running} onClick={props.onRun}>
        {props.running ? "Converting" : "Convert"}
      </MotionButton>
      <MotionButton type="button" class="button button--secondary" disabled={!props.running} onClick={props.onCancel}>
        Cancel
      </MotionButton>
      <MotionButton type="button" class="button button--secondary" onClick={props.onReset}>Reset</MotionButton>
      <span class="command-bar__rule" aria-hidden="true" />
      <MotionButton type="button" class="button button--secondary" disabled={!props.hasResult} onClick={props.onCopy}>
        Copy
      </MotionButton>
      <MotionButton
        type="button"
        class="button button--secondary"
        disabled={!props.hasResult}
        onClick={props.onDownload}>
        Download
      </MotionButton>
      <MotionButton type="button" class="button button--secondary" disabled={!props.hasResult} onClick={props.onSave}>
        Save run
      </MotionButton>
      <MotionButton type="button" class="button button--secondary" onClick={props.onShare}>Share view</MotionButton>
      <MotionButton type="button" class="button button--secondary" disabled={!props.hasResult} onClick={props.onOpen}>
        Open result
      </MotionButton>
    </div>
  );
}

export function WorkbenchPage() {
  const [params, setParams] = useSearchParams();
  const initialTab = isOutputTab(params.tab) ? params.tab : "markdown";
  const initialInspectTab = isInspectTab(params.inspect) ? params.inspect : "metadata";
  const initialLayout = isLayoutMode(params.layout) ? params.layout : "split";
  const [html, setHtml] = createSignal(sampleHtml);
  const [options, setOptions] = createSignal<PipelineOptions>(defaultOptions);
  const [result, setResult] = createSignal<PipelineResult | PipelineFailure>();
  const [tab, setTab] = createSignal<OutputTab>(initialTab);
  const [inspectTab, setInspectTab] = createSignal<InspectTab>(initialInspectTab);
  const [inspectOpen, setInspectOpen] = createSignal(params.inspectOpen === "1");
  const [layout, setLayout] = createSignal<LayoutMode>(
    isLayoutMode(localStorage.getItem("lectito.layout"))
      ? localStorage.getItem("lectito.layout") as LayoutMode
      : initialLayout,
  );
  const [fullscreenOutput, setFullscreenOutput] = createSignal(params.fullscreen === "1");
  const [running, setRunning] = createSignal(false);
  let runId = 0;

  createEffect(() => {
    setParams({
      tab: tab() === "markdown" ? undefined : tab(),
      inspect: inspectTab() === "metadata" ? undefined : inspectTab(),
      inspectOpen: inspectOpen() ? "1" : undefined,
      layout: layout() === "split" ? undefined : layout(),
      fullscreen: fullscreenOutput() ? "1" : undefined,
    }, { replace: true });
    localStorage.setItem("lectito.layout", layout());
  });

  const runExtraction = async () => {
    const currentOptions = options();
    const currentRun = ++runId;

    setRunning(true);
    const nextResult = await extractHtmlWithWasm(html(), currentOptions);

    if (currentRun === runId) {
      setResult(nextResult);
      setRunning(false);
    }
  };

  const resetInput = () => {
    setResult(undefined);
    setHtml(sampleHtml);
  };

  const cancelRun = () => {
    runId += 1;
    setRunning(false);
  };

  const resultValue = () => {
    const current = result();
    return current && !("message" in current) ? current : undefined;
  };
  const hasOutput = () => Boolean(result());

  const copyText = async (value: string) => {
    await navigator.clipboard?.writeText(value);
  };

  const copySelected = () => {
    const current = resultValue();
    if (!current) return;
    void copyText(selectedOutput(current, tab()).value);
  };

  const downloadSelected = () => {
    const current = resultValue();
    if (!current) return;
    const selected = selectedOutput(current, tab());
    const blob = new Blob([selected.value], { type: `${selected.type};charset=utf-8` });
    const link = document.createElement("a");
    link.href = URL.createObjectURL(blob);
    link.download = `lectito-result.${selected.extension}`;
    link.click();
    URL.revokeObjectURL(link.href);
  };

  const saveCurrentRun = () => {
    const current = resultValue();
    if (!current) return;
    saveRun({
      id: crypto.randomUUID(),
      createdAt: new Date().toISOString(),
      title: current.metadata.title || "Untitled extraction",
      sourceLabel: "Pasted HTML",
      input: html(),
      options: options(),
      result: current,
    });
  };

  const shareView = () => {
    void copyText(window.location.href);
  };

  const openResult = () => {
    const current = resultValue();
    if (!current) return;
    const blob = new Blob([current.previewHtml], { type: "text/html;charset=utf-8" });
    window.open(URL.createObjectURL(blob), "_blank", "noopener,noreferrer");
  };

  return (
    <main class="app-shell" classList={{ "has-output-fullscreen": fullscreenOutput() && hasOutput() }}>
      <WorkbenchTabs />
      <header class="app-header app-header--workbench">
        <div class="app-header__main">
          <div>
            <p class="eyebrow">Workbench</p>
            <h1>Extract pasted HTML</h1>
            <p class="app-header__note">
              Paste HTML here. Use the <A href="/api">API docs</A> for server-side URL extraction.
            </p>
          </div>
          <CommandBar
            running={running()}
            hasResult={Boolean(resultValue())}
            onRun={() => void runExtraction()}
            onCancel={cancelRun}
            onReset={resetInput}
            onCopy={copySelected}
            onDownload={downloadSelected}
            onSave={saveCurrentRun}
            onShare={shareView}
            onOpen={openResult} />
        </div>
      </header>

      <StatusStrip running={running()} result={result()} />

      <section
        class="workspace"
        classList={{ [`workspace--${layout()}`]: hasOutput(), "workspace--input-only": !hasOutput() }}
        aria-label="Extraction workspace">
        <Show when={!hasOutput() || layout() !== "input-collapsed"}>
          <InputPane
            html={html()}
            options={options()}
            onHtml={setHtml}
            sampleHtml={sampleHtmlFixtures}
            onOptions={setOptions}
            onReset={resetInput}
            onRun={() => void runExtraction()}
            running={running()} />
        </Show>
        <Show when={hasOutput()}>
          <OutputPane
            result={result()}
            tab={tab()}
            inspectTab={inspectTab()}
            inspectOpen={inspectOpen()}
            sourceHtml={html()}
            layout={layout()}
            fullscreen={fullscreenOutput()}
            onTab={setTab}
            onInspectTab={setInspectTab}
            onToggleInspect={() => setInspectOpen((open) => !open)}
            onLayout={setLayout}
            onToggleFullscreen={() => setFullscreenOutput((fullscreen) => !fullscreen)}
            onCopyMarkdown={() => {
              const current = resultValue();
              if (current) void copyText(current.markdown);
            }}
            onCopyHtml={() => {
              const current = resultValue();
              if (current) void copyText(current.cleanedHtml);
            }}
            onCopyMetadata={() => {
              const current = resultValue();
              if (current) void copyText(JSON.stringify(current.metadata, null, 2));
            }}
            onDownload={downloadSelected}
            onOpenPreview={openResult} />
        </Show>
      </section>
    </main>
  );
}
