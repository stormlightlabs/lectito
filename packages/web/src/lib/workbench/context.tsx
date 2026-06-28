import { useSettings } from "$lib/settings/context";
import type { WorkbenchStore } from "$lib/workbench/store";
import { createWorkbenchStore } from "$lib/workbench/store";
import { createContext, useContext } from "solid-js";
import type { ParentProps } from "solid-js";

const WorkbenchContext = createContext<WorkbenchStore>();

export function WorkbenchProvider(props: ParentProps) {
  const settings = useSettings();
  const store = createWorkbenchStore(settings.state);
  return <WorkbenchContext.Provider value={store}>{props.children}</WorkbenchContext.Provider>;
}

export function useWorkbench(): WorkbenchStore {
  const context = useContext(WorkbenchContext);
  if (!context) throw new Error("useWorkbench must be used within a WorkbenchProvider");
  return context;
}
