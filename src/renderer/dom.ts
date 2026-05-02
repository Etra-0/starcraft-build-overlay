/**
 * src/renderer/dom.ts
 * Single source of truth for DOM element references. Looks up every id
 * declared in index.html once at module load and exposes them on the
 * `dom` object so the rest of the renderer doesn't repeat
 * `document.getElementById` calls. byId<T> throws on a missing id so
 * callers get a useful stack instead of a silent null-deref later.
 */

function byId<T extends HTMLElement = HTMLElement>(id: string): T {
  const el = document.getElementById(id);
  if (!el) throw new Error(`Missing #${id} in index.html`);
  return el as T;
}

function queryAll<T extends HTMLElement = HTMLElement>(selector: string): T[] {
  return Array.from(document.querySelectorAll(selector)) as T[];
}

export interface Dom {
  body: HTMLElement;
  app: HTMLElement;
  overlayStatus: HTMLElement;
  manageButton: HTMLButtonElement;
  favoriteButton: HTMLButtonElement;
  compactButton: HTMLButtonElement;
  updatesBadgeButton: HTMLButtonElement;
  raceTabs: HTMLElement[];
  oppChips: HTMLElement[];
  buildSearch: HTMLInputElement;
  buildSelect: HTMLSelectElement;
  prevBuildButton: HTMLButtonElement;
  nextBuildButton: HTMLButtonElement;
  buildCard: HTMLElement;
  buildName: HTMLElement;
  matchupChip: HTMLElement;
  difficultyChip: HTMLElement;
  favoritedChip: HTMLElement;
  updateChip: HTMLElement;
  buildTags: HTMLElement;
  sourceButton: HTMLButtonElement;
  buildNotes: HTMLElement;
  buildUserNotes: HTMLElement;
  steps: HTMLElement;
  countersBlock: HTMLElement;
  countersList: HTMLElement;
  counteredByList: HTMLElement;
  prevPageButton: HTMLButtonElement;
  pageIndicator: HTMLElement;
  nextPageButton: HTMLButtonElement;
  toasts: HTMLElement;
  managerDialog: HTMLDialogElement;
  managerCloseButton: HTMLButtonElement;
  managerRaceFilter: HTMLSelectElement;
  managerMatchupFilter: HTMLSelectElement;
  managerSearch: HTMLInputElement;
  managerBuildList: HTMLElement;
  managerEmptyState: HTMLElement;
  newBuildButton: HTMLButtonElement;
  duplicateBuildButton: HTMLButtonElement;
  deleteBuildButton: HTMLButtonElement;
  backupButton: HTMLButtonElement;
  openDataFolderButton: HTMLButtonElement;
  editTabButton: HTMLButtonElement;
  importTabButton: HTMLButtonElement;
  updatesTabButton: HTMLButtonElement;
  settingsTabButton: HTMLButtonElement;
  editTab: HTMLElement;
  importTab: HTMLElement;
  updatesTab: HTMLElement;
  settingsTab: HTMLElement;
  formId: HTMLInputElement;
  formName: HTMLInputElement;
  formRace: HTMLSelectElement;
  formOpponent: HTMLSelectElement;
  formDifficulty: HTMLSelectElement;
  formTags: HTMLInputElement;
  formSourceName: HTMLInputElement;
  formSourceUrl: HTMLInputElement;
  formNotes: HTMLTextAreaElement;
  formUserNotes: HTMLTextAreaElement;
  formSteps: HTMLTextAreaElement;
  formCustomEdited: HTMLInputElement;
  formFavorite: HTMLInputElement;
  saveBuildButton: HTMLButtonElement;
  useBuildButton: HTMLButtonElement;
  openSourceFromFormButton: HTMLButtonElement;
  importInput: HTMLInputElement;
  previewImportButton: HTMLButtonElement;
  importNowButton: HTMLButtonElement;
  importCommonButton: HTMLButtonElement;
  importAllButton: HTMLButtonElement;
  updateExistingCheckbox: HTMLInputElement;
  importLog: HTMLElement;
  bulkRaceCheckboxes: HTMLInputElement[];
  checkUpdatesButton: HTMLButtonElement;
  refreshSelectedUpdatesButton: HTMLButtonElement;
  refreshAllUpdatesButton: HTMLButtonElement;
  updatesSummary: HTMLElement;
  updatesList: HTMLElement;
  settingsUserAgent: HTMLInputElement;
  settingsRateLimit: HTMLInputElement;
  settingsAutoCheck: HTMLInputElement;
  settingsCompactOverlay: HTMLInputElement;
  settingsOpacity: HTMLInputElement;
  settingsPageSize: HTMLInputElement;
  settingsDefaultRace: HTMLSelectElement;
  saveSettingsButton: HTMLButtonElement;
}

export const dom: Dom = {
  body: document.body,
  app: byId("app"),
  overlayStatus: byId("overlayStatus"),
  manageButton: byId<HTMLButtonElement>("manageButton"),
  favoriteButton: byId<HTMLButtonElement>("favoriteButton"),
  compactButton: byId<HTMLButtonElement>("compactButton"),
  updatesBadgeButton: byId<HTMLButtonElement>("updatesBadgeButton"),
  raceTabs: queryAll(".race-tab"),
  oppChips: queryAll(".opp-chip"),
  buildSearch: byId<HTMLInputElement>("buildSearch"),
  buildSelect: byId<HTMLSelectElement>("buildSelect"),
  prevBuildButton: byId<HTMLButtonElement>("prevBuildButton"),
  nextBuildButton: byId<HTMLButtonElement>("nextBuildButton"),
  buildCard: byId("buildCard"),
  buildName: byId("buildName"),
  matchupChip: byId("matchupChip"),
  difficultyChip: byId("difficultyChip"),
  favoritedChip: byId("favoritedChip"),
  updateChip: byId("updateChip"),
  buildTags: byId("buildTags"),
  sourceButton: byId<HTMLButtonElement>("sourceButton"),
  buildNotes: byId("buildNotes"),
  buildUserNotes: byId("buildUserNotes"),
  steps: byId("steps"),
  countersBlock: byId("countersBlock"),
  countersList: byId("countersList"),
  counteredByList: byId("counteredByList"),
  prevPageButton: byId<HTMLButtonElement>("prevPageButton"),
  pageIndicator: byId("pageIndicator"),
  nextPageButton: byId<HTMLButtonElement>("nextPageButton"),
  toasts: byId("toasts"),
  managerDialog: byId<HTMLDialogElement>("managerDialog"),
  managerCloseButton: byId<HTMLButtonElement>("managerCloseButton"),
  managerRaceFilter: byId<HTMLSelectElement>("managerRaceFilter"),
  managerMatchupFilter: byId<HTMLSelectElement>("managerMatchupFilter"),
  managerSearch: byId<HTMLInputElement>("managerSearch"),
  managerBuildList: byId("managerBuildList"),
  managerEmptyState: byId("managerEmptyState"),
  newBuildButton: byId<HTMLButtonElement>("newBuildButton"),
  duplicateBuildButton: byId<HTMLButtonElement>("duplicateBuildButton"),
  deleteBuildButton: byId<HTMLButtonElement>("deleteBuildButton"),
  backupButton: byId<HTMLButtonElement>("backupButton"),
  openDataFolderButton: byId<HTMLButtonElement>("openDataFolderButton"),
  editTabButton: byId<HTMLButtonElement>("editTabButton"),
  importTabButton: byId<HTMLButtonElement>("importTabButton"),
  updatesTabButton: byId<HTMLButtonElement>("updatesTabButton"),
  settingsTabButton: byId<HTMLButtonElement>("settingsTabButton"),
  editTab: byId("editTab"),
  importTab: byId("importTab"),
  updatesTab: byId("updatesTab"),
  settingsTab: byId("settingsTab"),
  formId: byId<HTMLInputElement>("formId"),
  formName: byId<HTMLInputElement>("formName"),
  formRace: byId<HTMLSelectElement>("formRace"),
  formOpponent: byId<HTMLSelectElement>("formOpponent"),
  formDifficulty: byId<HTMLSelectElement>("formDifficulty"),
  formTags: byId<HTMLInputElement>("formTags"),
  formSourceName: byId<HTMLInputElement>("formSourceName"),
  formSourceUrl: byId<HTMLInputElement>("formSourceUrl"),
  formNotes: byId<HTMLTextAreaElement>("formNotes"),
  formUserNotes: byId<HTMLTextAreaElement>("formUserNotes"),
  formSteps: byId<HTMLTextAreaElement>("formSteps"),
  formCustomEdited: byId<HTMLInputElement>("formCustomEdited"),
  formFavorite: byId<HTMLInputElement>("formFavorite"),
  saveBuildButton: byId<HTMLButtonElement>("saveBuildButton"),
  useBuildButton: byId<HTMLButtonElement>("useBuildButton"),
  openSourceFromFormButton: byId<HTMLButtonElement>("openSourceFromFormButton"),
  importInput: byId<HTMLInputElement>("importInput"),
  previewImportButton: byId<HTMLButtonElement>("previewImportButton"),
  importNowButton: byId<HTMLButtonElement>("importNowButton"),
  importCommonButton: byId<HTMLButtonElement>("importCommonButton"),
  importAllButton: byId<HTMLButtonElement>("importAllButton"),
  updateExistingCheckbox: byId<HTMLInputElement>("updateExistingCheckbox"),
  importLog: byId("importLog"),
  bulkRaceCheckboxes: queryAll<HTMLInputElement>("[data-race-bulk]"),
  checkUpdatesButton: byId<HTMLButtonElement>("checkUpdatesButton"),
  refreshSelectedUpdatesButton: byId<HTMLButtonElement>("refreshSelectedUpdatesButton"),
  refreshAllUpdatesButton: byId<HTMLButtonElement>("refreshAllUpdatesButton"),
  updatesSummary: byId("updatesSummary"),
  updatesList: byId("updatesList"),
  settingsUserAgent: byId<HTMLInputElement>("settingsUserAgent"),
  settingsRateLimit: byId<HTMLInputElement>("settingsRateLimit"),
  settingsAutoCheck: byId<HTMLInputElement>("settingsAutoCheck"),
  settingsCompactOverlay: byId<HTMLInputElement>("settingsCompactOverlay"),
  settingsOpacity: byId<HTMLInputElement>("settingsOpacity"),
  settingsPageSize: byId<HTMLInputElement>("settingsPageSize"),
  settingsDefaultRace: byId<HTMLSelectElement>("settingsDefaultRace"),
  saveSettingsButton: byId<HTMLButtonElement>("saveSettingsButton")
};
