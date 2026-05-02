/**
 * src/renderer/edit-tab.ts
 * Manager > Edit tab: load a build into the form, build a fresh blank
 * build, and save the form back into the store (always marking the result
 * as `customEdited` so subsequent imports won't clobber the user's tweaks).
 */
import { dom } from "./dom.js";
import { store } from "./state.js";
import { ALL_RACES, deriveMatchup, uniqueId } from "../shared/utils.js";
import { renderManagerList, useBuildInOverlay } from "./manager.js";
import { renderOverlay } from "./overlay.js";
import { toastError, toastOk } from "./toast.js";
import type { Build, Difficulty, Opponent, Race } from "../shared/types.js";

interface BlankBuildInputs {
  id: string;
  race: Race;
  opponent: Opponent;
}

export function blankBuild({ id, race, opponent }: BlankBuildInputs): Build {
  const r: Race = race || "Protoss";
  const o: Opponent = opponent || "Terran";
  return {
    id,
    race: r,
    opponent: o,
    matchup: deriveMatchup(r, o),
    name: "New Build",
    variantOf: null,
    tags: ["custom"],
    difficulty: null,
    sourceName: "Manual",
    sourceUrl: "",
    sourcePageTitle: null,
    notes: "",
    userNotes: "",
    customEdited: true,
    favorite: false,
    recentlyUsedAt: null,
    revisionId: null,
    lastImportedAt: null,
    lastCheckedAt: null,
    counters: [],
    counteredBy: [],
    steps: ["8 - Pylon", "10 - Gateway"]
  };
}

export function loadBuildIntoForm(build: Build | null | undefined): void {
  if (!build) return;
  store.selectedManagerBuildId = build.id;
  dom.formId.value = build.id || "";
  dom.formName.value = build.name || "";
  dom.formRace.value = build.race || "Protoss";
  dom.formOpponent.value = build.opponent || "Terran";
  dom.formDifficulty.value = build.difficulty || "";
  dom.formTags.value = (build.tags || []).join(", ");
  dom.formSourceName.value = build.sourceName || "";
  dom.formSourceUrl.value = build.sourceUrl || "";
  dom.formNotes.value = build.notes || "";
  dom.formUserNotes.value = build.userNotes || "";
  dom.formSteps.value = (build.steps || []).join("\n");
  dom.formCustomEdited.checked = !!build.customEdited;
  dom.formFavorite.checked = !!build.favorite;
}

interface FormBuildPayload {
  id: string;
  race: Race;
  opponent: Opponent;
  matchup: ReturnType<typeof deriveMatchup>;
  name: string;
  tags: string[];
  difficulty: Difficulty;
  sourceName: string;
  sourceUrl: string;
  notes: string;
  userNotes: string;
  customEdited: boolean;
  favorite: boolean;
  steps: string[];
}

function isRace(value: string): value is Race {
  return (ALL_RACES as readonly string[]).includes(value);
}

function buildFromForm(): FormBuildPayload {
  const name = dom.formName.value.trim() || "Untitled Build";
  const race: Race = isRace(dom.formRace.value) ? dom.formRace.value : "Protoss";
  const opponent: Opponent = (dom.formOpponent.value as Opponent) || "Terran";
  const id =
    dom.formId.value.trim() ||
    uniqueId(
      `custom-${race}-${opponent}-${name}`,
      store.data.builds.map((b) => b.id)
    );
  const difficultyValue = dom.formDifficulty.value;
  const difficulty: Difficulty =
    difficultyValue === "beginner" ||
    difficultyValue === "intermediate" ||
    difficultyValue === "advanced"
      ? difficultyValue
      : null;
  return {
    id,
    race,
    opponent,
    matchup: deriveMatchup(race, opponent),
    name,
    tags: dom.formTags.value
      .split(",")
      .map((t) => t.trim())
      .filter(Boolean),
    difficulty,
    sourceName: dom.formSourceName.value.trim() || "Manual",
    sourceUrl: dom.formSourceUrl.value.trim(),
    notes: dom.formNotes.value.trim(),
    userNotes: dom.formUserNotes.value,
    customEdited: dom.formCustomEdited.checked,
    favorite: dom.formFavorite.checked,
    steps: dom.formSteps.value
      .split(/\r?\n/)
      .map((s) => s.trim())
      .filter(Boolean)
  };
}

export type SaveDataFn = () => Promise<void>;
export type OpenExternalFn = (url: string) => void;

export function bindEditTabEvents(saveData: SaveDataFn, openExternal: OpenExternalFn): void {
  dom.saveBuildButton.addEventListener("click", async () => {
    try {
      const next = buildFromForm();
      const existing = store.data.builds.find(
        (b) => b.id === store.selectedManagerBuildId || b.id === next.id
      );
      if (existing) {
        Object.assign(existing, next, { customEdited: true });
        store.selectedManagerBuildId = existing.id;
      } else {
        const fresh: Build = {
          ...blankBuild({ id: next.id, race: next.race, opponent: next.opponent }),
          ...next,
          customEdited: true
        };
        store.data.builds.push(fresh);
        store.selectedManagerBuildId = fresh.id;
      }
      await saveData();
      const selected = store.data.builds.find((b) => b.id === store.selectedManagerBuildId);
      loadBuildIntoForm(selected);
      renderManagerList();
      renderOverlay();
      toastOk("Build saved.");
    } catch (err) {
      toastError(`Save failed: ${err instanceof Error ? err.message : String(err)}`);
    }
  });
  dom.useBuildButton.addEventListener("click", () => useBuildInOverlay());
  dom.openSourceFromFormButton.addEventListener("click", () => {
    const url = dom.formSourceUrl.value.trim();
    if (url) openExternal(url);
  });
}
