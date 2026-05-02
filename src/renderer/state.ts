/**
 * src/renderer/state.ts
 * In-memory store for builds, settings, transient UI state, and pending
 * Liquipedia updates. Persists the lightweight UI slice (race / opponent /
 * buildId / page / search) to localStorage; everything else is owned by
 * the main process and reloaded via IPC.
 */
import { ALL_OPPONENTS, ALL_RACES, clamp } from "../shared/utils.js";
import type { Build, BuildsData, Opponent, Race, Settings, UpdateInfo } from "../shared/types.js";

const STORAGE_KEY = "bw-build-overlay-state-v4";

interface UIState {
  race: Race;
  opponent: Opponent;
  buildId: string | null;
  page: number;
  search: string;
}

interface PendingUpdates {
  outdated: UpdateInfo[];
  all: UpdateInfo[];
  lastChecked: string | null;
}

export interface Store {
  data: BuildsData;
  settings: Settings;
  state: UIState;
  selectedManagerBuildId: string | null;
  pendingUpdates: PendingUpdates;
}

export const store: Store = {
  data: { version: 4, lastUpdated: "", builds: [] },
  settings: {
    liquipediaUserAgent: "",
    rateLimitMs: 2300,
    compactOverlay: false,
    overlayOpacity: 1,
    autoCheckUpdatesOnLaunch: false,
    pageSize: 25,
    defaultRace: "Protoss"
  },
  state: { race: "Protoss", opponent: "Terran", buildId: null, page: 0, search: "" },
  selectedManagerBuildId: null,
  pendingUpdates: { outdated: [], all: [], lastChecked: null }
};

function isRace(value: unknown): value is Race {
  return typeof value === "string" && (ALL_RACES as readonly string[]).includes(value);
}

function isOpponent(value: unknown): value is Opponent {
  return typeof value === "string" && (ALL_OPPONENTS as readonly string[]).includes(value);
}

export function loadLocalState(): void {
  try {
    const raw = localStorage.getItem(STORAGE_KEY);
    if (!raw) return;
    const saved = JSON.parse(raw) as Partial<UIState>;
    store.state.race = isRace(saved.race) ? saved.race : "Protoss";
    store.state.opponent = isOpponent(saved.opponent) ? saved.opponent : "Terran";
    store.state.buildId = typeof saved.buildId === "string" ? saved.buildId : null;
    store.state.page = Number(saved.page ?? 0) || 0;
    store.state.search = typeof saved.search === "string" ? saved.search : "";
  } catch (err) {
    console.warn("Failed to restore local UI state:", err);
  }
}

export function saveLocalState(): void {
  try {
    localStorage.setItem(STORAGE_KEY, JSON.stringify(store.state));
  } catch (err) {
    console.warn("Failed to persist local UI state:", err);
  }
}

export function setRace(race: Race): void {
  if (!isRace(race)) return;
  store.state.race = race;
  document.body.dataset.race = race;
  const builds = buildsForCurrentMatchup();
  if (!builds.find((b) => b.id === store.state.buildId))
    store.state.buildId = builds[0]?.id ?? null;
  store.state.page = 0;
  saveLocalState();
}

export function setOpponent(opponent: Opponent): void {
  if (!isOpponent(opponent)) return;
  store.state.opponent = opponent;
  const builds = buildsForCurrentMatchup();
  if (!builds.find((b) => b.id === store.state.buildId))
    store.state.buildId = builds[0]?.id ?? null;
  store.state.page = 0;
  saveLocalState();
}

export function setBuildId(id: string | null): void {
  store.state.buildId = id;
  store.state.page = 0;
  const build = currentBuild();
  if (build) build.recentlyUsedAt = new Date().toISOString();
  saveLocalState();
}

export function setSearch(query: string): void {
  store.state.search = String(query || "");
  saveLocalState();
}

export function setPage(page: number): void {
  const total = totalPages();
  store.state.page = clamp(Math.floor(page), 0, Math.max(0, total - 1));
  saveLocalState();
}

export function nextPage(): void {
  setPage(store.state.page + 1);
}

export function prevPage(): void {
  setPage(store.state.page - 1);
}

export function firstPage(): void {
  setPage(0);
}

export function buildsForCurrentMatchup(): Build[] {
  return store.data.builds.filter(
    (b) => b.race === store.state.race && b.opponent === store.state.opponent
  );
}

export function currentBuild(): Build | null {
  return (
    store.data.builds.find((b) => b.id === store.state.buildId) ||
    buildsForCurrentMatchup()[0] ||
    null
  );
}

export function selectedManagerBuild(): Build | null {
  return store.data.builds.find((b) => b.id === store.selectedManagerBuildId) || currentBuild();
}

export function totalPages(): number {
  const build = currentBuild();
  if (!build) return 1;
  const size = Math.max(1, Number(store.settings.pageSize || 25));
  return Math.max(1, Math.ceil((build.steps?.length || 0) / size));
}

export interface PageEntry {
  step: string;
  index: number;
}

export function pageRange(): PageEntry[] {
  const build = currentBuild();
  if (!build) return [];
  const size = Math.max(1, Number(store.settings.pageSize || 25));
  const start = store.state.page * size;
  return (build.steps || [])
    .slice(start, start + size)
    .map((step, i) => ({ step, index: start + i }));
}

export function toggleFavoriteOnCurrent(): Build | null {
  const b = currentBuild();
  if (!b) return null;
  b.favorite = !b.favorite;
  return b;
}
