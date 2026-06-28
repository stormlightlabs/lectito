import { extractUrlWithApi } from "$lib/clients/api";
import { extractHtmlWithWasm } from "$lib/clients/wasm";
import { saveRun } from "$lib/runs";
import { sampleHtml } from "$lib/sample";
import { findSample } from "$lib/samples";
import type { AppMode, InspectTab, OutputTab, PipelineFailure, PipelineOptions, PipelineResult } from "$lib/types";
import { isAppMode, isInspectTab, isOutputTab } from "$lib/types";
import { useSearchParams } from "@solidjs/router";
import { createEffect, createMemo } from "solid-js";
import { createStore, produce } from "solid-js/store";

export const layoutModes = ["split", "wide-output", "input-collapsed"] as const;

export type LayoutMode = typeof layoutModes[number];

type SelectedOutput = { value: string; extension: string; type: string };

export const defaultOptions: PipelineOptions = {
  baseUrl: "",
  contentSelector: "",
  charThreshold: 0,
  keepClasses: false,
  diagnostics: false,
};

function isLayoutMode(value: unknown): value is LayoutMode {
  return layoutModes.includes(value as LayoutMode);
}

function describeStatus(running: boolean, result?: PipelineResult | PipelineFailure): string {
  if (running) return "Converting";
  if (!result) return "Waiting";
  return "message" in result ? "Error" : "Ready";
}

function shareView() {
  void navigator.clipboard?.writeText(globalThis.location.href);
}

function sourceLabelForUrl(url: string): string {
  try {
    const { hostname } = new URL(url.trim());
    return hostname || url.trim() || "URL";
  } catch {
    return url.trim() || "URL";
  }
}

function selectedOutput(result: PipelineResult, tab: OutputTab): SelectedOutput {
  switch (tab) {
    case "preview": {
      return { value: result.previewHtml, extension: "html", type: "text/html" };
    }
    case "cleaned": {
      return { value: result.cleanedHtml, extension: "html", type: "text/html" };
    }
    case "compare": {
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
    default: {
      return { value: result.markdown, extension: "md", type: "text/markdown" };
    }
  }
}

export type WorkbenchState = {
  mode: AppMode;
  html: string;
  url: string;
  options: PipelineOptions;
  result?: PipelineResult | PipelineFailure;
  tab: OutputTab;
  inspectTab: InspectTab;
  inspectOpen: boolean;
  layout: LayoutMode;
  fullscreen: boolean;
  running: boolean;
};

export type WorkbenchStore = ReturnType<typeof createWorkbenchStore>;

/**
 * `defaults` are the user's saved settings (loaded at bootstrap and provided
 * via the settings context). The workbench snapshots them once at creation so
 * a running session is unaffected by later edits in the Settings page.
 *
 * Lightweight view state stays in the URL so views are shareable; persisted
 * defaults (options + default layout) live in the Dexie-backed settings store.
 */
export function createWorkbenchStore(defaults: { options: PipelineOptions; layout: LayoutMode }) {
  const [params, setParams] = useSearchParams();
  const initialMode = isAppMode(params.mode) ? params.mode : "html";
  const initialTab = isOutputTab(params.tab) ? params.tab : "markdown";
  const initialInspectTab = isInspectTab(params.inspect) ? params.inspect : "metadata";
  const initialLayout = isLayoutMode(params.layout) ? params.layout : defaults.layout;
  const sampleParam = Array.isArray(params.sample) ? params.sample[0] : params.sample;
  const initialHtml = findSample(sampleParam)?.html ?? sampleHtml;

  const [state, setState] = createStore<WorkbenchState>({
    mode: initialMode,
    html: initialHtml,
    url: "",
    options: { ...defaults.options },
    result: undefined,
    tab: initialTab,
    inspectTab: initialInspectTab,
    inspectOpen: params.inspectOpen === "1",
    layout: initialLayout,
    fullscreen: params.fullscreen === "1",
    running: false,
  });

  let runId = 0;

  createEffect(() => {
    setParams({
      mode: state.mode === "html" ? undefined : state.mode,
      tab: state.tab === "markdown" ? undefined : state.tab,
      inspect: state.inspectTab === "metadata" ? undefined : state.inspectTab,
      inspectOpen: state.inspectOpen ? "1" : undefined,
      layout: state.layout === defaults.layout ? undefined : state.layout,
      fullscreen: state.fullscreen ? "1" : undefined,
    }, { replace: true });
  });

  const statusText = createMemo(() => describeStatus(state.running, state.result));

  const resultValue = (): PipelineResult | undefined =>
    state.result && !("message" in state.result) ? state.result : undefined;

  const hasOutput = () => Boolean(state.result);

  const setHtml = (html: string) => setState("html", html);
  const setUrl = (url: string) => setState("url", url);
  const setMode = (mode: AppMode) => setState("mode", mode);
  const setOptions = (options: PipelineOptions) => setState("options", options);
  const setTab = (tab: OutputTab) => setState("tab", tab);
  const setInspectTab = (tab: InspectTab) => setState("inspectTab", tab);
  const toggleInspect = () => setState("inspectOpen", (open) => !open);
  const setLayout = (layout: LayoutMode) => setState("layout", layout);
  const toggleFullscreen = () => setState("fullscreen", (fullscreen) => !fullscreen);

  const runExtraction = async () => {
    const currentOptions = state.options;
    const currentRun = ++runId;

    setState("running", true);
    const nextResult = state.mode === "url"
      ? await extractUrlWithApi({ url: state.url, options: currentOptions })
      : await extractHtmlWithWasm(state.html, currentOptions);

    if (currentRun === runId) {
      setState(produce((s) => {
        s.result = nextResult;
        s.running = false;
      }));
    }
  };

  const cancelRun = () => {
    runId += 1;
    setState("running", false);
  };

  const resetInput = () => {
    setState(produce((s) => {
      s.result = undefined;
      s.html = sampleHtml;
    }));
  };

  const copySelected = () => {
    const current = resultValue();
    if (!current) return;
    void navigator.clipboard?.writeText(selectedOutput(current, state.tab).value);
  };

  const copyHtml = () => {
    const current = resultValue();
    if (current) void navigator.clipboard?.writeText(current.cleanedHtml);
  };

  const copyMetadata = () => {
    const current = resultValue();
    if (current) void navigator.clipboard?.writeText(JSON.stringify(current.metadata, null, 2));
  };

  const downloadSelected = () => {
    const current = resultValue();
    if (!current) return;
    const selected = selectedOutput(current, state.tab);
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
    const isUrl = state.mode === "url";
    void saveRun({
      id: crypto.randomUUID(),
      createdAt: new Date().toISOString(),
      title: current.metadata.title || (isUrl ? state.url : "Untitled extraction"),
      sourceLabel: isUrl ? sourceLabelForUrl(state.url) : "Pasted HTML",
      input: isUrl ? state.url : state.html,
      options: state.options,
      result: current,
    });
  };

  const openResult = () => {
    const current = resultValue();
    if (!current) return;
    const blob = new Blob([current.previewHtml], { type: "text/html;charset=utf-8" });
    window.open(URL.createObjectURL(blob), "_blank", "noopener,noreferrer");
  };

  return {
    state,
    statusText,
    resultValue,
    hasOutput,
    setHtml,
    setUrl,
    setMode,
    setOptions,
    setTab,
    setInspectTab,
    toggleInspect,
    setLayout,
    toggleFullscreen,
    runExtraction,
    cancelRun,
    resetInput,
    copySelected,
    copyHtml,
    copyMetadata,
    downloadSelected,
    saveCurrentRun,
    openResult,
    shareView,
  };
}
