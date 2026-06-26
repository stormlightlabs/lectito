import { getSavedRun } from "$lib/runs";
import { Trans } from "@lingui/solid/macro";
import { useParams } from "@solidjs/router";
import { createEffect, createSignal, Show } from "solid-js";
import { PageShell } from "./PageShell";
import { WorkbenchTabs } from "./WorkbenchTabs";

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
                <h2>{savedRun().title}</h2>
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
