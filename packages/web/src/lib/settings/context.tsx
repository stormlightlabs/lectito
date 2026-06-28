import type { Settings } from "$lib/settings/store";
import { createSettingsStore } from "$lib/settings/store";
import { createContext, useContext } from "solid-js";
import type { ParentProps } from "solid-js";

export type SettingsContextValue = ReturnType<typeof createSettingsStore>;

const SettingsContext = createContext<SettingsContextValue>();

export function SettingsProvider(props: ParentProps & { initial: Settings }) {
  const store = createSettingsStore(props.initial);
  return <SettingsContext.Provider value={store}>{props.children}</SettingsContext.Provider>;
}

export function useSettings(): SettingsContextValue {
  const context = useContext(SettingsContext);
  if (!context) throw new Error("useSettings must be used within a SettingsProvider");
  return context;
}
