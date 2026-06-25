import { listSavedRuns } from "$lib/runs";
import { A } from "@solidjs/router";
import { createSignal, For, onMount, Show } from "solid-js";
import { PageShell } from "./PageShell";
import { WorkbenchTabs } from "./WorkbenchTabs";

export function RunsPage() {
  const [runs, setRuns] = createSignal<Awaited<ReturnType<typeof listSavedRuns>>>([]);

  onMount(() => {
    void listSavedRuns().then(setRuns);
  });

  return (
    <PageShell eyebrow="Workbench" title="Runs" headerBefore={<WorkbenchTabs />} variant="workbench">
      <Show when={runs().length > 0} fallback={<p>No saved runs yet.</p>}>
        <div class="run-list">
          <For each={runs()}>
            {(run) => (
              <A class="run-list__item" href={`/workbench/runs/${run.id}`}>
                <span>{run.title}</span>
                <strong>{run.sourceLabel}</strong>
                <em>{new Date(run.createdAt).toLocaleString()}</em>
                <small>{run.result.metadata.length.toLocaleString()} chars · {run.result.elapsedMs}ms</small>
              </A>
            )}
          </For>
        </div>
      </Show>
    </PageShell>
  );
}
