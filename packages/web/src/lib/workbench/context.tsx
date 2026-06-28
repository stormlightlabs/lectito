import { useSettings } from "$lib/settings/context";
import type { SavedRun } from "$lib/types";
import type { WorkbenchStore } from "$lib/workbench/store";
import { createWorkbenchStore } from "$lib/workbench/store";
import { createContext, useContext } from "solid-js";
import type { ParentProps } from "solid-js";

const WorkbenchContext = createContext<WorkbenchStore>();

/**
 * Transient holder for a run queued for reopening. The Run detail page sets
 * this, navigates to `/workbench`, and the freshly-mounted `WorkbenchProvider`
 * consumes and clears it.
 *
 * We cannot pass a full run through the URL (too large), so this bridges the two
 * routes without polluting search params.
 */
let pendingRun: SavedRun | undefined;

export function queueRunForWorkbench(run: SavedRun): void {
  pendingRun = run;
}

export function WorkbenchProvider(props: ParentProps) {
  const settings = useSettings();
  const store = createWorkbenchStore(settings.state);
  if (pendingRun) {
    store.loadRun(pendingRun);
    pendingRun = undefined;
  }
  return <WorkbenchContext.Provider value={store}>{props.children}</WorkbenchContext.Provider>;
}

export function useWorkbench(): WorkbenchStore {
  const context = useContext(WorkbenchContext);
  if (!context) throw new Error("useWorkbench must be used within a WorkbenchProvider");
  return context;
}
