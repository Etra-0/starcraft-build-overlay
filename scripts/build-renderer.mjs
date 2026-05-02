// scripts/build-renderer.mjs
// Builds src/renderer/* into dist-frontend/, the directory Tauri loads from
// (frontendDist in src-tauri/tauri.conf.json). Bundles the renderer with
// esbuild, then copies the static assets (index.html, styles.css, icon)
// alongside the bundle so relative URLs in the HTML keep working.

import { build, context } from "esbuild";
import { mkdir, copyFile } from "node:fs/promises";
import { join } from "node:path";

const watch = process.argv.includes("--watch");
const isProduction = process.env.NODE_ENV === "production";
const outDir = "dist-frontend";

await mkdir(outDir, { recursive: true });
await mkdir(join(outDir, "assets"), { recursive: true });

const options = {
  entryPoints: ["src/renderer/index.ts"],
  bundle: true,
  format: "esm",
  target: "chrome120",
  platform: "browser",
  outfile: join(outDir, "renderer.js"),
  sourcemap: !isProduction,
  minify: isProduction,
  logLevel: "info",
  legalComments: "none"
};

async function copyStaticAssets() {
  await copyFile("index.html", join(outDir, "index.html"));
  await copyFile("styles.css", join(outDir, "styles.css"));
  await copyFile("assets/icon.png", join(outDir, "assets/icon.png"));
}

if (watch) {
  await copyStaticAssets();
  const ctx = await context(options);
  await ctx.watch();
  console.log(`[esbuild] watching src/renderer/... (output: ${outDir}/renderer.js)`);
} else {
  await build(options);
  await copyStaticAssets();
  console.log(`[esbuild] built ${outDir}/renderer.js + copied static assets.`);
}
