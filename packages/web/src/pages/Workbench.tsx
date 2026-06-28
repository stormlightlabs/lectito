import { InputPane } from "$components/Input";
import { OutputPane } from "$components/Output";
import { useWorkbench, WorkbenchProvider } from "$lib/workbench/context";
import { Trans } from "@lingui/solid/macro";
import { A } from "@solidjs/router";
import { Show } from "solid-js";
import { WorkbenchTabs } from "./WorkbenchTabs";

function WorkbenchHeader() {
  return (
    <header class="app-header app-header--workbench">
      <div class="app-header__main">
        <p class="eyebrow">Workbench</p>
        <div class="app-header__note">
          <Trans>
            <p>Paste HTML or switch to URL mode to extract a live page.</p>
          </Trans>
          <Trans>
            <p>
              {/* eslint-disable-next-line react/jsx-max-depth */}
              See the <A href="/api-docs">API docs</A> for programmatic access.
            </p>
          </Trans>
        </div>
      </div>
    </header>
  );
}

function WorkbenchContent() {
  const { state, hasOutput } = useWorkbench();

  return (
    <main
      class="app-shell app-shell--workbench"
      classList={{ "has-output-fullscreen": state.fullscreen && hasOutput() }}>
      <WorkbenchTabs />
      <WorkbenchHeader />

      <section
        class="workspace"
        classList={{ [`workspace--${state.layout}`]: hasOutput(), "workspace--input-only": !hasOutput() }}
        aria-label="Extraction workspace">
        <Show when={!hasOutput() || state.layout !== "input-collapsed"}>
          <InputPane />
        </Show>
        <Show when={hasOutput()}>
          <OutputPane />
        </Show>
      </section>
    </main>
  );
}

export function WorkbenchPage() {
  return (
    <WorkbenchProvider>
      <WorkbenchContent />
    </WorkbenchProvider>
  );
}
