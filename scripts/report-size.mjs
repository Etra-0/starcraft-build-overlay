// scripts/report-size.mjs
// Walks src-tauri/target/release/bundle/ and prints the size of every Tauri
// bundle artifact (NSIS / MSI / DMG / AppImage / .deb / .app). Run after
// `npm run tauri:build` to confirm the output landed under ~10-15 MB on
// Windows / macOS and ~15 MB on Linux.

import { readdirSync, statSync, existsSync } from "node:fs";
import { join } from "node:path";

const root = "src-tauri/target/release/bundle";
if (!existsSync(root)) {
  console.error(`[size] no ${root}/ folder yet. Run \`npm run tauri:build\` first.`);
  process.exit(1);
}

const ARTIFACT_EXT = /\.(exe|msi|dmg|app|appimage|deb|rpm)$/i;

function walk(dir) {
  const out = [];
  for (const entry of readdirSync(dir, { withFileTypes: true })) {
    const full = join(dir, entry.name);
    if (entry.isDirectory()) {
      if (entry.name.toLowerCase().endsWith(".app")) {
        out.push({ path: full, name: entry.name, dir: true });
        continue;
      }
      out.push(...walk(full));
    } else if (ARTIFACT_EXT.test(entry.name)) {
      out.push({ path: full, name: entry.name, dir: false });
    }
  }
  return out;
}

function dirSize(p) {
  let total = 0;
  for (const entry of readdirSync(p, { withFileTypes: true })) {
    const full = join(p, entry.name);
    if (entry.isDirectory()) total += dirSize(full);
    else total += statSync(full).size;
  }
  return total;
}

const artifacts = walk(root);
if (artifacts.length === 0) {
  console.error("[size] no bundle artifacts found. Did `npm run tauri:build` succeed?");
  process.exit(1);
}

for (const a of artifacts) {
  const bytes = a.dir ? dirSize(a.path) : statSync(a.path).size;
  const mb = (bytes / 1024 / 1024).toFixed(2);
  console.log(`${a.name.padEnd(48)} ${mb.padStart(7)} MB  (${bytes.toLocaleString()} bytes)`);
}
