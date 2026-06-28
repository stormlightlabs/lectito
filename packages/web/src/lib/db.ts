import Dexie, { type EntityTable } from "dexie";
import type { SavedRun } from "./types";

export type AppSetting = { key: string; value: unknown; updatedAt: string };

export const db = new Dexie("lectito") as Dexie & {
  settings: EntityTable<AppSetting, "key">;
  runs: EntityTable<SavedRun, "id">;
};

db.version(1).stores({ settings: "key, updatedAt", runs: "id, createdAt" });
