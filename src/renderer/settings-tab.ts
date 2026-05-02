/**
 * src/renderer/settings-tab.ts
 * Manager > Settings tab: load/save Liquipedia User-Agent + rate limit,
 * overlay opacity (live-applied to the BrowserWindow), compact mode,
 * page size, and default race. Persisted via the main-process settings.json.
 */
import { api } from "./api.js";
import { dom } from "./dom.js";
import { store } from "./state.js";
import { toastError, toastOk } from "./toast.js";
import { renderOverlay } from "./overlay.js";
import type { Race, Settings } from "../shared/types.js";

export function loadSettingsIntoForm(): void {
  dom.settingsUserAgent.value = store.settings.liquipediaUserAgent || "";
  dom.settingsRateLimit.value = String(store.settings.rateLimitMs || 2300);
  dom.settingsAutoCheck.checked = !!store.settings.autoCheckUpdatesOnLaunch;
  dom.settingsCompactOverlay.checked = !!store.settings.compactOverlay;
  dom.settingsOpacity.value = String(store.settings.overlayOpacity ?? 1);
  dom.settingsPageSize.value = String(store.settings.pageSize || 25);
  dom.settingsDefaultRace.value = store.settings.defaultRace || "Protoss";
}

export function bindSettingsTabEvents(): void {
  dom.saveSettingsButton.addEventListener("click", async () => {
    try {
      const update: Partial<Settings> = {
        liquipediaUserAgent: dom.settingsUserAgent.value.trim(),
        rateLimitMs: Math.max(2000, Number(dom.settingsRateLimit.value) || 2300),
        autoCheckUpdatesOnLaunch: dom.settingsAutoCheck.checked,
        compactOverlay: dom.settingsCompactOverlay.checked,
        overlayOpacity: Number(dom.settingsOpacity.value) || 1,
        pageSize: Math.max(6, Math.min(60, Number(dom.settingsPageSize.value) || 25)),
        defaultRace: (dom.settingsDefaultRace.value as Race) || "Protoss"
      };
      const next = await api.saveSettings(update);
      store.settings = next;
      api.setOpacity(store.settings.overlayOpacity);
      renderOverlay();
      toastOk("Settings saved.");
    } catch (err) {
      toastError(err instanceof Error ? err.message : String(err));
    }
  });

  dom.settingsOpacity.addEventListener("input", () => {
    api.setOpacity(Number(dom.settingsOpacity.value) || 1);
  });
}
