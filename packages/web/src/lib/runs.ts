import { db } from "./db";
import type { SavedRun } from "./types";

const STORAGE_KEY = "lectito.savedRuns";
const MAX_RUNS = 25;

function readStoredRuns(): SavedRun[] {
  if (typeof localStorage === "undefined") return [];

  try {
    const value = localStorage.getItem(STORAGE_KEY);
    if (!value) return [];
    const parsed = JSON.parse(value);
    return Array.isArray(parsed) ? parsed : [];
  } catch {
    return [];
  }
}

function writeStoredRuns(runs: SavedRun[]) {
  localStorage.setItem(STORAGE_KEY, JSON.stringify(runs.slice(0, MAX_RUNS)));
}

async function listRunsNewestFirst(): Promise<SavedRun[]> {
  const runs = await db.runs.orderBy("createdAt").toArray();
  return runs.toReversed();
}

export async function listSavedRuns(): Promise<SavedRun[]> {
  const runs = await listRunsNewestFirst();
  if (runs.length > 0) return runs;

  const storedRuns = readStoredRuns();
  if (storedRuns.length > 0) {
    await db.runs.bulkPut(storedRuns);
  }
  return storedRuns;
}

export async function getSavedRun(id: string): Promise<SavedRun | undefined> {
  const run = await db.runs.get(id);
  if (run) return run;

  const storedRun = readStoredRuns().find((item) => item.id === id);
  if (storedRun) {
    await db.runs.put(storedRun);
  }
  return storedRun;
}

export async function saveRun(run: SavedRun): Promise<SavedRun[]> {
  await db.runs.put(run);
  const runs = await listRunsNewestFirst();
  const nextRuns = runs.slice(0, MAX_RUNS);
  const staleRuns = runs.slice(MAX_RUNS);

  if (staleRuns.length > 0) {
    await db.runs.bulkDelete(staleRuns.map((item) => item.id));
  }

  writeStoredRuns(nextRuns);
  return nextRuns;
}

export async function deleteRun(id: string): Promise<SavedRun[]> {
  await db.runs.delete(id);
  const runs = await listRunsNewestFirst();
  writeStoredRuns(runs.slice(0, MAX_RUNS));
  return runs.slice(0, MAX_RUNS);
}

export async function clearRuns(): Promise<void> {
  await db.runs.clear();
  writeStoredRuns([]);
}

/** Serialize runs for export. */
export function exportRuns(runs: SavedRun[]): string {
  return JSON.stringify(runs, null, 2);
}

/** Parse an imported runs file, returning valid `SavedRun` rows only. */
export function parseImportedRuns(text: string): SavedRun[] {
  const parsed = JSON.parse(text);
  if (!Array.isArray(parsed)) return [];

  return parsed.filter((item): item is SavedRun => {
    if (!item || typeof item !== "object") return false;
    const run = item as Partial<SavedRun>;
    return Boolean(run.id && run.createdAt && run.result && run.options);
  });
}

/** Merge imported runs into the store, skipping duplicates by id. */
export async function importRuns(runs: SavedRun[]): Promise<SavedRun[]> {
  const existing = await listRunsNewestFirst();
  const ids = new Set(existing.map((item) => item.id));
  const fresh = runs.filter((item) => !ids.has(item.id));

  if (fresh.length > 0) {
    await db.runs.bulkPut(fresh);
  }

  const merged = await listRunsNewestFirst();
  const nextRuns = merged.slice(0, MAX_RUNS);
  writeStoredRuns(nextRuns);
  return nextRuns;
}
