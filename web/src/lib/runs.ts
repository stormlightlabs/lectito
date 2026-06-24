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

export function listSavedRuns(): SavedRun[] {
  return readStoredRuns();
}

export function getSavedRun(id: string): SavedRun | undefined {
  return readStoredRuns().find((run) => run.id === id);
}

export function saveRun(run: SavedRun): SavedRun[] {
  const runs = readStoredRuns().filter((item) => item.id !== run.id);
  const nextRuns = [run, ...runs].slice(0, MAX_RUNS);
  writeStoredRuns(nextRuns);
  return nextRuns;
}
