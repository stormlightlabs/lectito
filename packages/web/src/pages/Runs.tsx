import { clearRuns, deleteRun, exportRuns, importRuns, listSavedRuns, parseImportedRuns } from "$lib/runs";
import type { SavedRun } from "$lib/types";
import { Trans } from "@lingui/solid/macro";
import { A } from "@solidjs/router";
import { createSignal, For, onMount, Show } from "solid-js";
import { PageShell } from "./PageShell";
import { WorkbenchTabs } from "./WorkbenchTabs";

function ImportButton(props: { onFile: (file?: File) => void; ref?: (el: HTMLInputElement) => void }) {
  return (
    <label class="button button--secondary">
      <Trans>Import</Trans>
      <input
        ref={props.ref}
        class="visually-hidden-file"
        type="file"
        accept="application/json,.json"
        onChange={(event) => props.onFile(event.currentTarget.files?.[0])} />
    </label>
  );
}

function ExportButton(props: { onExport: () => void }) {
  return (
    <button type="button" class="button button--secondary" onClick={props.onExport}>
      <Trans>Export all</Trans>
    </button>
  );
}

function ClearAllButton(props: { onClear: () => void }) {
  return (
    <button type="button" class="button button--ghost" onClick={props.onClear}>
      <Trans>Clear all</Trans>
    </button>
  );
}

export function RunsPage() {
  const [runs, setRuns] = createSignal<SavedRun[]>([]);
  let fileInput: HTMLInputElement | undefined;

  const refresh = () => listSavedRuns().then(setRuns);

  onMount(() => {
    void refresh();
  });

  const remove = async (id: string) => {
    const next = await deleteRun(id);
    setRuns(next);
  };

  const clearAll = async () => {
    await clearRuns();
    setRuns([]);
  };

  const downloadAll = () => {
    const blob = new Blob([exportRuns(runs())], { type: "application/json;charset=utf-8" });
    const link = document.createElement("a");
    link.href = URL.createObjectURL(blob);
    link.download = "lectito-runs.json";
    link.click();
    URL.revokeObjectURL(link.href);
  };

  const importFile = async (file?: File) => {
    if (!file) return;
    try {
      const text = await file.text();
      const parsed = parseImportedRuns(text);
      const next = await importRuns(parsed);
      setRuns(next);
    } catch {
      // ignore malformed files
    }
    if (fileInput) fileInput.value = "";
  };

  return (
    <PageShell eyebrow="Workbench" title="Runs" headerBefore={<WorkbenchTabs />} variant="workbench">
      <Show
        when={runs().length > 0}
        fallback={
          <div class="runs-empty">
            <p>
              <Trans>No saved runs yet.</Trans>
            </p>
            <p class="runs-empty__hint">
              <Trans>Runs are stored locally in your browser and disposable.</Trans>
            </p>
            <div class="runs-empty__actions">
              <ImportButton onFile={(file) => void importFile(file)} />
            </div>
          </div>
        }>
        <div class="runs-toolbar">
          <div class="runs-toolbar__actions">
            <ExportButton onExport={() => void downloadAll()} />
            <ImportButton
              onFile={(file) => void importFile(file)}
              ref={(el) => {
                fileInput = el;
              }} />
          </div>
          <ClearAllButton onClear={() => void clearAll()} />
        </div>
        <div class="run-list">
          <For each={runs()}>
            {(run) => (
              <div class="run-list__item">
                <A class="run-list__link" href={`/workbench/runs/${run.id}`}>
                  <span>{run.title}</span>
                  <strong>{run.sourceLabel}</strong>
                  <em>{new Date(run.createdAt).toLocaleString()}</em>
                  <small>{run.result.metadata.length.toLocaleString()} chars · {run.result.elapsedMs}ms</small>
                </A>
                <button
                  type="button"
                  class="run-list__delete"
                  aria-label="Delete run"
                  title="Delete"
                  onClick={() => void remove(run.id)}>
                  ×
                </button>
              </div>
            )}
          </For>
        </div>
      </Show>
    </PageShell>
  );
}
