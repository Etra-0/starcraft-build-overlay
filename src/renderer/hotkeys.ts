/**
 * src/renderer/hotkeys.ts
 * Maps the global-hotkey action strings emitted by the main process
 * (race-*, opp-*, next/prev-build, paging, toggle-favorite, toggle-compact,
 * toggle-window) to renderer state mutations + a re-render. Returns a
 * single handler function used by `api.onHotkey(...)`.
 */
import { api } from "./api.js";
import {
  firstPage,
  nextPage,
  prevPage,
  setOpponent,
  setRace,
  store,
  toggleFavoriteOnCurrent
} from "./state.js";
import { cycleBuild, renderOverlay } from "./overlay.js";
import type { Build, HotkeyAction, Settings } from "../shared/types.js";

export interface HotkeyDeps {
  persistSettings: (partial: Partial<Settings>) => Promise<void> | void;
  persistFavorite: (build: Build) => Promise<void> | void;
}

export function makeHotkeyHandler({
  persistSettings,
  persistFavorite
}: HotkeyDeps): (action: HotkeyAction) => Promise<void> {
  return async function handleHotkey(action: HotkeyAction): Promise<void> {
    switch (action) {
      case "race-terran":
        setRace("Terran");
        renderOverlay();
        break;
      case "race-protoss":
        setRace("Protoss");
        renderOverlay();
        break;
      case "race-zerg":
        setRace("Zerg");
        renderOverlay();
        break;
      case "opp-terran":
        setOpponent("Terran");
        renderOverlay();
        break;
      case "opp-zerg":
        setOpponent("Zerg");
        renderOverlay();
        break;
      case "opp-protoss":
        setOpponent("Protoss");
        renderOverlay();
        break;
      case "opp-random":
        setOpponent("Random");
        renderOverlay();
        break;
      case "next-build":
        cycleBuild(1);
        break;
      case "prev-build":
        cycleBuild(-1);
        break;
      case "next-page":
        nextPage();
        renderOverlay();
        break;
      case "prev-page":
        prevPage();
        renderOverlay();
        break;
      case "first-page":
        firstPage();
        renderOverlay();
        break;
      case "toggle-favorite": {
        const build = toggleFavoriteOnCurrent();
        if (build) {
          await persistFavorite(build);
          renderOverlay();
        }
        break;
      }
      case "toggle-compact":
        store.settings.compactOverlay = !store.settings.compactOverlay;
        await persistSettings({ compactOverlay: store.settings.compactOverlay });
        renderOverlay();
        break;
      case "toggle-window":
        api.toggleWindow();
        break;
      default: {
        const exhaustive: never = action;
        void exhaustive;
        break;
      }
    }
  };
}
