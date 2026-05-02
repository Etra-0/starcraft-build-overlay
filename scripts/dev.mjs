// scripts/dev.mjs
// Dev orchestrator for the Tauri build. Spawns esbuild in watch mode so
// dist-frontend/renderer.js stays fresh, waits for the first bundle to
// land, then launches `tauri dev` with BW_DEVTOOLS=1 so the webview opens
// devtools on startup.

import { spawn } from "node:child_process";
import { mkdir } from "node:fs/promises";
import { existsSync } from "node:fs";
import path from "node:path";

const isWindows = process.platform === "win32";

await mkdir("dist-frontend", { recursive: true });

const children = [];

function spawnChild(name, command, args, env = {}) {
  const child = spawn(command, args, {
    stdio: ["ignore", "pipe", "pipe"],
    env: { ...process.env, ...env },
    shell: isWindows
  });
  child.stdout.on("data", (data) => process.stdout.write(`[${name}] ${data}`));
  child.stderr.on("data", (data) => process.stderr.write(`[${name}] ${data}`));
  child.on("exit", (code, signal) => {
    console.log(`[${name}] exited (code=${code}, signal=${signal})`);
  });
  children.push({ name, child });
  return child;
}

function shutdown(code = 0) {
  for (const { child } of children) {
    if (!child.killed) {
      try {
        child.kill();
      } catch {
        // best-effort shutdown
      }
    }
  }
  process.exit(code);
}

process.on("SIGINT", () => shutdown(0));
process.on("SIGTERM", () => shutdown(0));

console.log("[dev] starting esbuild watch on src/renderer/...");
spawnChild("esbuild", process.execPath, ["scripts/build-renderer.mjs", "--watch"]);

const target = path.join("dist-frontend", "renderer.js");
const start = Date.now();
const TIMEOUT_MS = 60_000;
while (!existsSync(target) && Date.now() - start < TIMEOUT_MS) {
  await new Promise((r) => setTimeout(r, 250));
}
if (!existsSync(target)) {
  console.error("[dev] timed out waiting for esbuild first build.");
  shutdown(1);
}

console.log("[dev] launching tauri dev with auto-DevTools...");
const tauri = spawnChild("tauri", isWindows ? "npx.cmd" : "npx", ["tauri", "dev"], {
  BW_DEVTOOLS: "1"
});

tauri.on("exit", (code) => shutdown(code ?? 0));
