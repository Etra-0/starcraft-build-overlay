/**
 * src/renderer/index.ts
 * Renderer entry point bundled by esbuild into one ESM file. Boots the
 * overlay: loads builds + settings via the preload bridge, wires every
 * tab/module's event handlers, mounts the global "/" search hotkey, hooks
 * up the main-process hotkey channel, and shows a fatal red banner if
 * boot fails.
 */
import { api } from "./api.js";
import { dom } from "./dom.js";
import { currentBuild, loadLocalState, setOpponent, setRace, store } from "./state.js";
import { bindOverlayEvents, renderOverlay } from "./overlay.js";
import { bindManagerListEvents, renderManagerList } from "./manager.js";
import { bindEditTabEvents, loadBuildIntoForm } from "./edit-tab.js";
import { bindImportTabEvents } from "./import-tab.js";
import { bindUpdatesTabEvents, renderUpdatesList } from "./updates-tab.js";
import { bindSettingsTabEvents, loadSettingsIntoForm } from "./settings-tab.js";
import { makeHotkeyHandler } from "./hotkeys.js";
import { toast, toastError } from "./toast.js";
import type { Build, Settings } from "../shared/types.js";

console.info("[bw-overlay] renderer booted");

function showFatal(message: string): void {
  let banner = document.getElementById("fatalBanner");
  if (!banner) {
    banner = document.createElement("div");
    banner.id = "fatalBanner";
    banner.style.cssText =
      "position:fixed;top:0;left:0;right:0;z-index:9999;background:#7a1f1f;color:#fff;padding:12px 16px;font:14px Segoe UI,sans-serif;line-height:1.4;box-shadow:0 4px 12px rgba(0,0,0,.5);";
    document.body.appendChild(banner);
  }
  banner.innerHTML = `<strong>Renderer error:</strong> ${String(message)}<br><small>Press F12 to open DevTools for details.</small>`;
}

async function reloadBuilds(): Promise<void> {
  store.data = await api.getBuilds();
}

async function saveData(): Promise<void> {
  try {
    store.data = await api.saveBuilds(store.data);
  } catch (err) {
    const message = err instanceof Error ? err.message : String(err);
    toastError(`Save failed: ${message}`);
    throw err;
  }
}

async function persistFavorite(_build: Build): Promise<void> {
  try {
    store.data = await api.saveBuilds(store.data);
  } catch (err) {
    const message = err instanceof Error ? err.message : String(err);
    toastError(`Could not save favorite: ${message}`);
  }
}

async function persistSettings(partial: Partial<Settings>): Promise<void> {
  try {
    store.settings = await api.saveSettings({ ...store.settings, ...partial });
  } catch (err) {
    const message = err instanceof Error ? err.message : String(err);
    toastError(`Settings save failed: ${message}`);
  }
}

function openExternal(url: string): void {
  if (url) api.openExternal(url);
}

function openManager(): void {
  store.selectedManagerBuildId = currentBuild()?.id ?? store.selectedManagerBuildId;
  loadBuildIntoForm(currentBuild());
  loadSettingsIntoForm();
  renderManagerList();
  renderUpdatesList();
  dom.managerDialog.showModal();
}

function bindGlobalKeys(): void {
  document.addEventListener("keydown", (e) => {
    const target = e.target as Element | null;
    const tag = target?.tagName?.toLowerCase() ?? "";
    const inField = ["input", "textarea", "select"].includes(tag);
    if (e.key === "/" && !inField && !e.ctrlKey && !e.metaKey && !e.altKey) {
      e.preventDefault();
      dom.buildSearch.focus();
    }
  });
}

async function boot(): Promise<void> {
  loadLocalState();
  try {
    store.data = await api.getBuilds();
    store.settings = await api.getSettings();
  } catch (err) {
    const message = err instanceof Error ? err.message : String(err);
    dom.buildName.textContent = "Error loading data";
    dom.buildNotes.hidden = false;
    dom.buildNotes.textContent = message;
    showFatal(`Failed to load data from main process: ${message}`);
    return;
  }

  if (!store.state.race) store.state.race = store.settings.defaultRace || "Protoss";
  if (!currentBuild()) {
    const first =
      store.data.builds.find((b) => b.race === store.state.race) || store.data.builds[0];
    if (first) {
      setRace(first.race);
      setOpponent(first.opponent);
      store.state.buildId = first.id;
    }
  }
  store.selectedManagerBuildId = store.state.buildId || store.data.builds[0]?.id || null;

  bindOverlayEvents({
    openExternal,
    toggleCompact: async () => {
      store.settings.compactOverlay = !store.settings.compactOverlay;
      await persistSettings({ compactOverlay: store.settings.compactOverlay });
      renderOverlay();
    },
    persistFavorite
  });

  bindManagerListEvents(saveData);
  bindEditTabEvents(saveData, openExternal);
  bindImportTabEvents(reloadBuilds);
  bindUpdatesTabEvents();
  bindSettingsTabEvents();
  dom.manageButton.addEventListener("click", openManager);
  bindGlobalKeys();
  api.onHotkey(makeHotkeyHandler({ persistSettings, persistFavorite }));
  api.setOpacity(Number(store.settings.overlayOpacity) || 1);
  loadSettingsIntoForm();
  renderOverlay();

  if (store.settings.autoCheckUpdatesOnLaunch) {
    setTimeout(async () => {
      try {
        const result = await api.checkForUpdates();
        store.pendingUpdates.all = result.all;
        store.pendingUpdates.outdated = result.outdated;
        store.pendingUpdates.lastChecked = new Date().toISOString();
        renderUpdatesList();
        renderOverlay();
        if (result.outdated.length) {
          toast(`${result.outdated.length} build update(s) available on Liquipedia.`, "warn", 5000);
        }
      } catch (err) {
        console.warn("Auto update check failed:", err);
      }
    }, 1500);
  }
}

boot().catch((err: unknown) => {
  console.error(err);
  const message = err instanceof Error ? err.message : String(err);
  if (dom.buildName) {
    dom.buildName.textContent = "Error loading overlay";
    dom.buildNotes.hidden = false;
    dom.buildNotes.textContent = message;
  }
  showFatal(`Boot failed: ${message}`);
});
