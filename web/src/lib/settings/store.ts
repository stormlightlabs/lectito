import { db } from "$lib/db";
import type { PipelineOptions } from "$lib/types";
import { defaultOptions, type LayoutMode, layoutModes } from "$lib/workbench/store";
import { createStore } from "solid-js/store";

export type Settings = { options: PipelineOptions; layout: LayoutMode };

export const defaultSettings: Settings = { options: defaultOptions, layout: "split" };

const SETTINGS_KEY = "defaults";

function isLayout(value: unknown): value is LayoutMode {
  return typeof value === "string" && (layoutModes as readonly string[]).includes(value);
}

/**
 * normalize merges stored partial values onto the hard-coded defaults
 * so a missing field or an older row never breaks the workbench.
 *
 * We always returns fresh objects so the shared `defaultOptions` const is never aliased
 * into a reactive store.
 */
function normalize(raw: unknown): Settings {
  const value = raw && typeof raw === "object" ? raw as Partial<Settings> : {};
  return { options: { ...defaultOptions, ...value.options }, layout: isLayout(value.layout) ? value.layout : "split" };
}

export async function loadSettings(): Promise<Settings> {
  const row = await db.settings.get(SETTINGS_KEY);
  return normalize(row?.value);
}

export async function saveSettings(settings: Settings): Promise<void> {
  await db.settings.put({ key: SETTINGS_KEY, value: settings, updatedAt: new Date().toISOString() });
}

export type SettingsStore = ReturnType<typeof createSettingsStore>;

/**
 * Holds the *committed* (last saved) settings.
 *
 * The workbench snapshots these at creation; the Settings page keeps its own draft
 * and calls `commit` on Save so unsaved edits never reach the workbench.
 */
export function createSettingsStore(initial: Settings) {
  const [state, setState] = createStore<Settings>({ options: { ...initial.options }, layout: initial.layout });

  const commit = async (next: Settings) => {
    await saveSettings(next);
    setState({ options: { ...next.options }, layout: next.layout });
  };

  return { state, commit };
}
