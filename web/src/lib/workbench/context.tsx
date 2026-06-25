import { sampleHtmlFixtures } from "$lib/sample";
import { useSettings } from "$lib/settings/context";
import type { WorkbenchStore } from "$lib/workbench/store";
import { createWorkbenchStore } from "$lib/workbench/store";
import { createContext, useContext } from "solid-js";
import type { ParentProps } from "solid-js";

export type WorkbenchContextValue = WorkbenchStore & { sampleHtml: typeof sampleHtmlFixtures };

const WorkbenchContext = createContext<WorkbenchContextValue>();

export function WorkbenchProvider(props: ParentProps) {
  const settings = useSettings();
  const store = createWorkbenchStore(settings.state);
  const value: WorkbenchContextValue = { ...store, sampleHtml: sampleHtmlFixtures };
  return <WorkbenchContext.Provider value={value}>{props.children}</WorkbenchContext.Provider>;
}

export function useWorkbench(): WorkbenchContextValue {
  const context = useContext(WorkbenchContext);
  if (!context) throw new Error("useWorkbench must be used within a WorkbenchProvider");
  return context;
}
