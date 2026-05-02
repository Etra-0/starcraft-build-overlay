/**
 * src/shared/types.ts
 * Single source of truth for project types shared across the Rust backend
 * and the TypeScript renderer. The OverlayAPI interface here is the
 * canonical IPC contract; src-tauri/src/commands.rs implements it and the
 * renderer consumes it via src/renderer/api.ts.
 */

export type Race = "Terran" | "Protoss" | "Zerg";
export type Opponent = "Terran" | "Protoss" | "Zerg" | "Random";
export type Matchup = `${"T" | "P" | "Z"}v${"T" | "P" | "Z" | "R"}`;
export type Difficulty = "beginner" | "intermediate" | "advanced" | null;
export type SourceName = "Liquipedia" | "Manual" | string;

export interface Build {
  id: string;
  race: Race;
  opponent: Opponent;
  matchup: Matchup;
  name: string;
  variantOf: string | null;
  tags: string[];
  difficulty: Difficulty;
  sourceName: SourceName;
  sourceUrl: string;
  sourcePageTitle: string | null;
  notes: string;
  userNotes: string;
  customEdited: boolean;
  favorite: boolean;
  recentlyUsedAt: string | null;
  revisionId: number | null;
  lastImportedAt: string | null;
  lastCheckedAt: string | null;
  counters: string[];
  counteredBy: string[];
  steps: string[];
}

export interface BuildsData {
  version: number;
  lastUpdated: string;
  builds: Build[];
}

export interface Settings {
  version?: number;
  liquipediaUserAgent: string;
  rateLimitMs: number;
  compactOverlay: boolean;
  overlayOpacity: number;
  autoCheckUpdatesOnLaunch: boolean;
  pageSize: number;
  defaultRace: Race;
}

export interface ImportOptions {
  updateExisting?: boolean;
}

export type ImportMergeResult = "added" | "updated" | "skipped" | "skipped-custom";

export interface ImportSinglePageResult {
  pageTitle: string;
  addedOrUpdated: Build[];
  results: ImportMergeResult[];
  totalVariants: number;
}

export type BulkImportMode = "common" | "all";

export interface BulkImportOptions {
  races?: Race[];
  mode?: BulkImportMode;
  updateExisting?: boolean;
}

export interface BulkImportResult {
  mode: BulkImportMode;
  races: Race[];
  selected: number;
  totalDiscovered: number;
  added: number;
  updated: number;
  skipped: number;
  failed: number;
  variantsTotal: number;
}

export type UpdateReason = "newer-revision" | "no-stored-rev" | "missing-on-server" | "up-to-date";

export interface UpdateInfo {
  buildId: string;
  name: string;
  matchup: Matchup;
  sourcePageTitle: string;
  sourceUrl: string;
  currentRevId: number | null;
  latestRevId: number | null;
  latestTimestamp: string | null;
  outdated: boolean;
  reason: UpdateReason;
  customEdited: boolean;
}

export interface CheckUpdatesResult {
  checked: number;
  outdated: UpdateInfo[];
  all: UpdateInfo[];
}

export interface RefreshBuildsOptions {
  force?: boolean;
}

export interface RefreshBuildsFailure {
  buildId: string;
  error: string;
}

export interface RefreshBuildsResult {
  refreshed: string[];
  failed: RefreshBuildsFailure[];
  skippedCustom: string[];
}

export interface UserDataPaths {
  userBuildsPath: string;
  settingsPath: string;
  userData: string;
}

export type HotkeyAction =
  | "race-terran"
  | "race-protoss"
  | "race-zerg"
  | "opp-terran"
  | "opp-zerg"
  | "opp-protoss"
  | "opp-random"
  | "next-build"
  | "prev-build"
  | "next-page"
  | "prev-page"
  | "first-page"
  | "toggle-favorite"
  | "toggle-compact"
  | "toggle-window";

export interface ParsedVariant {
  variantName: string;
  heading: string | null;
  steps: string[];
}

export interface ParsedInfobox {
  name?: string;
  race?: Race | null;
  matchups?: string[];
  creator?: string;
  popularized?: string;
}

export interface ParsedLiquipediaPage {
  infobox: ParsedInfobox | null;
  playerRace: Race | null;
  opponent: Opponent | null;
  difficulty: Difficulty;
  counters: string[];
  counteredBy: string[];
  variants: ParsedVariant[];
}

export interface OverlayAPI {
  getBuilds(): Promise<BuildsData>;
  saveBuilds(builds: BuildsData): Promise<BuildsData>;
  getSettings(): Promise<Settings>;
  saveSettings(settings: Partial<Settings>): Promise<Settings>;
  previewLiquipediaPage(input: string): Promise<Build[]>;
  importLiquipediaPage(input: string, options: ImportOptions): Promise<ImportSinglePageResult>;
  bulkImport(options: BulkImportOptions): Promise<BulkImportResult>;
  checkForUpdates(): Promise<CheckUpdatesResult>;
  refreshBuild(buildId: string): Promise<Build>;
  refreshBuilds(buildIds: string[], options: RefreshBuildsOptions): Promise<RefreshBuildsResult>;
  backupData(): Promise<string>;
  openDataFolder(): void;
  getUserPaths(): Promise<UserDataPaths>;
  close(): void;
  toggleWindow(): void;
  setOpacity(value: number): void;
  openExternal(url: string): void;
  onHotkey(callback: (action: HotkeyAction) => void): void;
  onLiquipediaProgress(callback: (message: string) => void): void;
}
