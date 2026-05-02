/**
 * src/renderer/import-tab.ts
 * Manager > Import tab: single-page preview/import (drops the parsed
 * variant into the Edit form) and bulk import per race in "common" or
 * "all" modes. Streams progress messages from the main process into the
 * import log box.
 */
import { api } from "./api.js";
import { dom } from "./dom.js";
import { store } from "./state.js";
import { renderManagerList, setTab } from "./manager.js";
import { loadBuildIntoForm } from "./edit-tab.js";
import { renderOverlay } from "./overlay.js";
import { toast, toastError, toastOk } from "./toast.js";
import type { BulkImportMode, ImportMergeResult, Race } from "../shared/types.js";

function appendLog(message: string): void {
  const time = new Date().toLocaleTimeString();
  dom.importLog.textContent += `[${time}] ${message}\n`;
  dom.importLog.scrollTop = dom.importLog.scrollHeight;
}

function lockBulkButtons(locked: boolean): void {
  dom.importCommonButton.disabled = locked;
  dom.importAllButton.disabled = locked;
  dom.previewImportButton.disabled = locked;
  dom.importNowButton.disabled = locked;
}

function selectedBulkRaces(): Race[] {
  return dom.bulkRaceCheckboxes
    .filter((c) => c.checked)
    .map((c) => c.dataset.raceBulk as Race)
    .filter((r): r is Race => r === "Terran" || r === "Protoss" || r === "Zerg");
}

export type ReloadBuildsFn = () => Promise<void>;

export function bindImportTabEvents(reloadBuilds: ReloadBuildsFn): void {
  dom.previewImportButton.addEventListener("click", async () => {
    const input = dom.importInput.value.trim();
    if (!input) {
      toastError("Paste a Liquipedia URL or page title first.");
      return;
    }
    try {
      lockBulkButtons(true);
      appendLog(`Preview: ${input}`);
      const variants = await api.previewLiquipediaPage(input);
      if (!variants?.length) {
        toast("No build templates found on that page.", "warn");
        return;
      }
      const first = variants[0];
      if (!first) return;
      const existingIds = new Set(store.data.builds.map((b) => b.id));
      let candidate = first.id;
      let i = 2;
      while (existingIds.has(candidate)) candidate = `${first.id}-${i++}`;
      first.id = candidate;
      store.selectedManagerBuildId = first.id;
      loadBuildIntoForm(first);
      setTab("edit");
      appendLog(
        `Loaded preview: "${first.name}" (+${variants.length - 1} other variant(s) on the page).`
      );
      toast(`Preview loaded. ${variants.length} variant(s) detected. Click Save to keep.`, "ok");
    } catch (err) {
      const message = err instanceof Error ? err.message : String(err);
      appendLog(`ERROR: ${message}`);
      toastError(message);
    } finally {
      lockBulkButtons(false);
    }
  });

  dom.importNowButton.addEventListener("click", async () => {
    const input = dom.importInput.value.trim();
    if (!input) {
      toastError("Paste a Liquipedia URL or page title first.");
      return;
    }
    try {
      lockBulkButtons(true);
      appendLog(`Importing: ${input}`);
      const result = await api.importLiquipediaPage(input, {
        updateExisting: dom.updateExistingCheckbox.checked
      });
      await reloadBuilds();
      renderManagerList();
      renderOverlay();
      const counts = result.results.reduce<Record<ImportMergeResult, number>>(
        (acc, r) => {
          acc[r] = (acc[r] || 0) + 1;
          return acc;
        },
        { added: 0, updated: 0, skipped: 0, "skipped-custom": 0 }
      );
      const summary = Object.entries(counts)
        .filter(([, v]) => v > 0)
        .map(([k, v]) => `${v} ${k}`)
        .join(", ");
      appendLog(`Done. ${summary}.`);
      toastOk(`Imported ${result.totalVariants} variant(s).`);
    } catch (err) {
      const message = err instanceof Error ? err.message : String(err);
      appendLog(`ERROR: ${message}`);
      toastError(message);
    } finally {
      lockBulkButtons(false);
    }
  });

  const runBulk = async (mode: BulkImportMode): Promise<void> => {
    const races = selectedBulkRaces();
    if (!races.length) {
      toastError("Select at least one race.");
      return;
    }
    const label = `${mode === "all" ? "all" : "common"} ${races.join(" + ")}`;
    if (!confirm(`Import ${label} builds? Liquipedia is rate-limited so this can take a while.`))
      return;
    try {
      lockBulkButtons(true);
      appendLog(`Starting bulk import: ${label}`);
      const result = await api.bulkImport({
        mode,
        races,
        updateExisting: dom.updateExistingCheckbox.checked
      });
      await reloadBuilds();
      renderManagerList();
      renderOverlay();
      appendLog(
        `Done. From ${result.totalDiscovered} discovered, processed ${result.selected}. Added ${result.added}, updated ${result.updated}, skipped ${result.skipped}, failed ${result.failed}. Variants: ${result.variantsTotal}.`
      );
      toastOk(
        `Bulk import: +${result.added} new, ${result.updated} updated, ${result.failed} failed.`
      );
    } catch (err) {
      const message = err instanceof Error ? err.message : String(err);
      appendLog(`ERROR: ${message}`);
      toastError(message);
    } finally {
      lockBulkButtons(false);
    }
  };

  dom.importCommonButton.addEventListener("click", () => {
    void runBulk("common");
  });
  dom.importAllButton.addEventListener("click", () => {
    void runBulk("all");
  });
  api.onLiquipediaProgress((message) => appendLog(message));
}
