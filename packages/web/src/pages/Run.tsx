import { queueRunForWorkbench } from "$lib/workbench/context";
import { exportRuns, getSavedRun } from "$lib/runs";
import type { SavedRun } from "$lib/types";
import { Trans } from "@lingui/solid/macro";
import { useNavigate, useParams } from "@solidjs/router";
import { createEffect, createSignal, Show } from "solid-js";
import { PageShell } from "./PageShell";
import { WorkbenchTabs } from "./WorkbenchTabs";

function ExportButton(props: { run: () => SavedRun | undefined }) {
  const download = () => {
    const run = props.run();
    if (!run) return;
    const blob = new Blob([exportRuns([run])], { type: "application/json;charset=utf-8" });
    const link = document.createElement("a");
    link.href = URL.createObjectURL(blob);
    link.download = `lectito-run-${run.id.slice(0, 8)}.json`;
    link.click();
    URL.revokeObjectURL(link.href);
  };

  return (
    <button type="button" class="button button--secondary" onClick={download}>
      <Trans>Export</Trans>
    </button>
  );
}

function ReopenButton(props: { run: () => SavedRun | undefined }) {
  const navigate = useNavigate();

  const reopen = () => {
    const run = props.run();
    if (!run) return;
    queueRunForWorkbench(run);
    navigate("/workbench");
  };

  return (
    <button type="button" class="button button--primary" onClick={reopen}>
      <Trans>Reopen in workbench</Trans>
    </button>
  );
}

export function RunPage() {
  const params = useParams();
  const [run, setRun] = createSignal<Awaited<ReturnType<typeof getSavedRun>>>();
  const [loaded, setLoaded] = createSignal(false);

  createEffect(() => {
    const id = params.id;
    if (!id) return;
    setLoaded(false);
    void getSavedRun(id).then((savedRun) => {
      setRun(() => savedRun);
      setLoaded(true);
    });
  });

  return (
    <PageShell eyebrow="Workbench" title={`Run ${params.id}`} headerBefore={<WorkbenchTabs />} variant="workbench">
      <Show
        when={loaded()}
        fallback={
          <p>
            <Trans>Loading saved run.</Trans>
          </p>
        }>
        <Show
          when={run()}
          fallback={
            <p>
              <Trans>This saved run was not found in local history.</Trans>
            </p>
          }>
          {(savedRun) => (
            <div class="run-detail">
              <section>
                <div class="run-detail__header">
                  <h2>{savedRun().title}</h2>
                  <div class="run-detail__actions">
                    <ReopenButton run={savedRun} />
                    <ExportButton run={savedRun} />
                  </div>
                </div>
                <p>{savedRun().sourceLabel}</p>
                <dl class="metadata-list metadata-list--static">
                  <div>
                    <dt>Status</dt>
                    <dd>{savedRun().result.mode}</dd>
                  </div>
                  <div>
                    <dt>Elapsed</dt>
                    <dd>{savedRun().result.elapsedMs}ms</dd>
                  </div>
                  <div>
                    <dt>Length</dt>
                    <dd>{savedRun().result.metadata.length.toLocaleString()} chars</dd>
                  </div>
                  <div>
                    <dt>Saved</dt>
                    <dd>{new Date(savedRun().createdAt).toLocaleString()}</dd>
                  </div>
                </dl>
              </section>
              <section>
                <h2>Output</h2>
                <pre>{savedRun().result.markdown}</pre>
              </section>
              <section>
                <h2>Metadata</h2>
                <pre>{JSON.stringify(savedRun().result.metadata, null, 2)}</pre>
              </section>
              <section>
                <h2>Diagnostics</h2>
                <pre>{savedRun().result.diagnostics}</pre>
              </section>
              <section>
                <h2>Options</h2>
                <pre>{JSON.stringify(savedRun().options, null, 2)}</pre>
              </section>
            </div>
          )}
        </Show>
      </Show>
    </PageShell>
  );
}
