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
