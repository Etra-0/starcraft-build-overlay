/**
 * src/renderer/manager.ts
 * Drives the Build Manager modal: tab switching, the filtered build list
 * on the left (with favorite / update / custom-edited badges), and the
 * new / duplicate / delete / use-in-overlay actions.
 */
import { api } from "./api.js";
import { dom } from "./dom.js";
import { selectedManagerBuild, setBuildId, setOpponent, setRace, store } from "./state.js";
import { deriveMatchup, matchesQuery, uniqueId } from "../shared/utils.js";
import { renderOverlay } from "./overlay.js";
import { toastError, toastOk } from "./toast.js";
import { blankBuild, loadBuildIntoForm } from "./edit-tab.js";
import type { Build } from "../shared/types.js";
import type { SaveDataFn } from "./edit-tab.js";

export type TabName = "edit" | "import" | "updates" | "settings";

export function setTab(name: TabName): void {
  const tabs: Array<[TabName, HTMLButtonElement, HTMLElement]> = [
    ["edit", dom.editTabButton, dom.editTab],
    ["import", dom.importTabButton, dom.importTab],
    ["updates", dom.updatesTabButton, dom.updatesTab],
    ["settings", dom.settingsTabButton, dom.settingsTab]
  ];
  for (const [id, button, panel] of tabs) {
    button.classList.toggle("active", id === name);
    panel.classList.toggle("active", id === name);
  }
}

function buildMatchups(): string[] {
  const set = new Set<string>();
  for (const b of store.data.builds) set.add(b.matchup || deriveMatchup(b.race, b.opponent));
  return [...set].sort();
}

function syncMatchupFilter(): void {
  const current = dom.managerMatchupFilter.value || "All";
  dom.managerMatchupFilter.innerHTML = "";
  const all = document.createElement("option");
  all.value = "All";
  all.textContent = "All matchups";
  dom.managerMatchupFilter.appendChild(all);
  for (const m of buildMatchups()) {
    const opt = document.createElement("option");
    opt.value = m;
    opt.textContent = m;
    dom.managerMatchupFilter.appendChild(opt);
  }
  dom.managerMatchupFilter.value = current;
}

export function renderManagerList(): void {
  syncMatchupFilter();
  const raceFilter = dom.managerRaceFilter.value;
  const matchupFilter = dom.managerMatchupFilter.value;
  const search = dom.managerSearch.value;

  const builds = store.data.builds
    .filter((b) => raceFilter === "All" || b.race === raceFilter)
    .filter(
      (b) =>
        matchupFilter === "All" ||
        (b.matchup || deriveMatchup(b.race, b.opponent)) === matchupFilter
    )
    .filter((b) =>
      matchesQuery(
        `${b.name} ${b.tags?.join(" ") || ""} ${b.matchup || ""} ${b.difficulty || ""}`,
        search
      )
    )
    .sort((a, b) => {
      const matchupCmp = (a.matchup || "").localeCompare(b.matchup || "");
      if (matchupCmp !== 0) return matchupCmp;
      return a.name.localeCompare(b.name);
    });

  dom.managerBuildList.innerHTML = "";
  dom.managerEmptyState.hidden = builds.length > 0;
  const pendingIds = new Set(store.pendingUpdates.outdated.map((u) => u.buildId));

  for (const build of builds) {
    const wrapper = document.createElement("button");
    wrapper.className = `build-list-item ${build.id === store.selectedManagerBuildId ? "active" : ""}`;
    const titleRow = document.createElement("div");
    titleRow.className = "item-title-row";

    const title = document.createElement("div");
    title.className = "item-title";
    title.textContent = build.name;
    titleRow.appendChild(title);

    if (build.favorite) {
      const star = document.createElement("span");
      star.textContent = "\u2605";
      star.style.color = "var(--update)";
      titleRow.appendChild(star);
    }
    if (pendingIds.has(build.id)) {
      const upd = document.createElement("span");
      upd.textContent = "\u21bb";
      upd.style.color = "var(--update)";
      upd.title = "Update available on Liquipedia";
      titleRow.appendChild(upd);
    }
    if (build.customEdited) {
      const ce = document.createElement("span");
      ce.textContent = "\u270e";
      ce.style.color = "var(--muted)";
      ce.title = "Custom edited - protected from refresh";
      titleRow.appendChild(ce);
    }

    const subtitle = document.createElement("div");
    subtitle.className = "item-subtitle";
    const matchup = document.createElement("span");
    matchup.className = "chip";
    matchup.textContent = build.matchup || deriveMatchup(build.race, build.opponent);
    matchup.style.borderColor = "var(--border)";
    matchup.style.background = "transparent";
    matchup.style.color = "var(--accent)";
    subtitle.appendChild(matchup);
    if (build.difficulty) {
      const diff = document.createElement("span");
      diff.className = "chip subtle";
      diff.textContent = build.difficulty;
      subtitle.appendChild(diff);
    }
    if (build.sourceName === "Liquipedia") {
      const src = document.createElement("span");
      src.className = "chip subtle";
      src.textContent = "Liquipedia";
      subtitle.appendChild(src);
    }

    wrapper.append(titleRow, subtitle);
    wrapper.addEventListener("click", () => {
      store.selectedManagerBuildId = build.id;
      loadBuildIntoForm(build);
      setTab("edit");
      renderManagerList();
    });
    dom.managerBuildList.appendChild(wrapper);
  }
}

export async function newManualBuild(saveData: SaveDataFn): Promise<void> {
  const filterRace = dom.managerRaceFilter.value;
  const race = filterRace !== "All" ? (filterRace as Build["race"]) : store.state.race;
  const opponent = store.state.opponent;
  const id = uniqueId(
    `custom-${race}-${opponent}-build`,
    store.data.builds.map((b) => b.id)
  );
  const build = blankBuild({ id, race, opponent });
  store.data.builds.push(build);
  store.selectedManagerBuildId = build.id;
  loadBuildIntoForm(build);
  await saveData();
  renderManagerList();
  renderOverlay();
  toastOk("New build created.");
}

export async function duplicateSelectedBuild(saveData: SaveDataFn): Promise<void> {
  const build = selectedManagerBuild();
  if (!build) return;
  const copy: Build = JSON.parse(JSON.stringify(build));
  copy.id = uniqueId(
    `${copy.id}-copy`,
    store.data.builds.map((b) => b.id)
  );
  copy.name = `${copy.name} (Copy)`;
  copy.tags = [...(copy.tags || []).filter((t) => t !== "imported"), "custom"];
  copy.customEdited = true;
  copy.revisionId = null;
  copy.lastImportedAt = null;
  copy.lastCheckedAt = null;
  copy.variantOf = null;
  store.data.builds.push(copy);
  store.selectedManagerBuildId = copy.id;
  await saveData();
  loadBuildIntoForm(copy);
  renderManagerList();
  toastOk("Duplicated.");
}

export async function deleteSelectedBuild(saveData: SaveDataFn): Promise<void> {
  const build = selectedManagerBuild();
  if (!build) return;
  if (!confirm(`Delete "${build.name}"?`)) return;
  store.data.builds = store.data.builds.filter((b) => b.id !== build.id);
  if (store.state.buildId === build.id) {
    store.state.buildId = null;
    store.state.page = 0;
  }
  store.selectedManagerBuildId = store.data.builds[0]?.id ?? null;
  await saveData();
  loadBuildIntoForm(selectedManagerBuild());
  renderManagerList();
  renderOverlay();
  toastOk("Deleted.");
}

export function useBuildInOverlay(): void {
  const build = selectedManagerBuild();
  if (!build) return;
  setRace(build.race);
  setOpponent(build.opponent);
  setBuildId(build.id);
  renderOverlay();
  toastOk(`Switched overlay to ${build.name}.`);
}

export function bindManagerListEvents(saveData: SaveDataFn): void {
  dom.managerRaceFilter.addEventListener("change", renderManagerList);
  dom.managerMatchupFilter.addEventListener("change", renderManagerList);
  dom.managerSearch.addEventListener("input", renderManagerList);
  dom.newBuildButton.addEventListener("click", () =>
    newManualBuild(saveData).catch((e) => toastError(e instanceof Error ? e.message : String(e)))
  );
  dom.duplicateBuildButton.addEventListener("click", () =>
    duplicateSelectedBuild(saveData).catch((e) =>
      toastError(e instanceof Error ? e.message : String(e))
    )
  );
  dom.deleteBuildButton.addEventListener("click", () =>
    deleteSelectedBuild(saveData).catch((e) =>
      toastError(e instanceof Error ? e.message : String(e))
    )
  );
  dom.backupButton.addEventListener("click", async () => {
    try {
      await api.backupData();
      toastOk("Backup created.");
    } catch (err) {
      toastError(`Backup failed: ${err instanceof Error ? err.message : String(err)}`);
    }
  });
  dom.openDataFolderButton.addEventListener("click", () => api.openDataFolder());
  dom.editTabButton.addEventListener("click", () => setTab("edit"));
  dom.importTabButton.addEventListener("click", () => setTab("import"));
  dom.updatesTabButton.addEventListener("click", () => setTab("updates"));
  dom.settingsTabButton.addEventListener("click", () => setTab("settings"));
  dom.managerCloseButton.addEventListener("click", () => dom.managerDialog.close());
}
