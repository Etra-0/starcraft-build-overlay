import { execSync, spawnSync } from "node:child_process";
import { readFile, writeFile } from "node:fs/promises";

const args = process.argv.slice(2);
const bumpType = args[0];
const dryRun = args.includes("--dry-run");
const allowDirty = args.includes("--allow-dirty");

const VALID_BUMPS = new Set(["patch", "minor", "major"]);
if (!VALID_BUMPS.has(bumpType)) {
  console.error("Usage: node scripts/release.mjs <patch|minor|major> [--dry-run] [--allow-dirty]");
  process.exit(1);
}

function git(args, options = {}) {
  return execSync(`git ${args}`, { encoding: "utf8", ...options }).trim();
}

const branch = git("rev-parse --abbrev-ref HEAD");
if (branch !== "main" && !allowDirty) {
  console.error(`Refusing to release: current branch is "${branch}", expected "main".`);
  console.error("Re-run with --allow-dirty if you really mean it (e.g. for a release branch).");
  process.exit(1);
}

const status = git("status --porcelain");
if (status && !allowDirty) {
  console.error("Refusing to release: working tree is not clean. Commit or stash first.");
  console.error(status);
  process.exit(1);
}

const pkgPath = "package.json";
const pkg = JSON.parse(await readFile(pkgPath, "utf8"));
const oldVersion = pkg.version;
const [major, minor, patch] = oldVersion.split(".").map((n) => parseInt(n, 10));
let newVersion;
switch (bumpType) {
  case "patch":
    newVersion = `${major}.${minor}.${patch + 1}`;
    break;
  case "minor":
    newVersion = `${major}.${minor + 1}.0`;
    break;
  case "major":
    newVersion = `${major + 1}.0.0`;
    break;
}

const today = new Date().toISOString().slice(0, 10);

const changelogPath = "CHANGELOG.md";
const changelog = await readFile(changelogPath, "utf8");

// Match the last `## [Unreleased]` heading in the file (anchored to its own
// line, multiline mode). The README-style example block at the top of
// CHANGELOG.md also contains a literal `## [Unreleased]` line, so a naive
// String#replace would clobber the example instead of the real heading.
const headingRegex = /^## \[Unreleased\]\s*$/gm;
const headingMatches = [...changelog.matchAll(headingRegex)];
if (headingMatches.length === 0) {
  console.error('CHANGELOG.md is missing a "## [Unreleased]" section. Add one and re-run.');
  process.exit(1);
}
const lastMatch = headingMatches[headingMatches.length - 1];
const replacement = `## [Unreleased]\n\n_No unreleased changes yet._\n\n## [${newVersion}] - ${today}`;
const updatedChangelog =
  changelog.slice(0, lastMatch.index) +
  replacement +
  changelog.slice(lastMatch.index + lastMatch[0].length);

console.log(`Version: ${oldVersion} -> ${newVersion}`);
console.log(`Tag:     v${newVersion}`);
console.log(`Branch:  ${branch}`);

if (dryRun) {
  console.log("(dry run; no files written, no commit, no tag)");
  process.exit(0);
}

pkg.version = newVersion;
await writeFile(pkgPath, JSON.stringify(pkg, null, 2) + "\n", "utf8");
await writeFile(changelogPath, updatedChangelog, "utf8");

const tauriConfPath = "src-tauri/tauri.conf.json";
const tauriConf = JSON.parse(await readFile(tauriConfPath, "utf8"));
tauriConf.version = newVersion;
await writeFile(tauriConfPath, JSON.stringify(tauriConf, null, 2) + "\n", "utf8");

const cargoTomlPath = "src-tauri/Cargo.toml";
const cargoToml = await readFile(cargoTomlPath, "utf8");
const cargoTomlNext = cargoToml.replace(/^version = "[^"]+"/m, `version = "${newVersion}"`);
if (cargoTomlNext === cargoToml) {
  console.error(`Could not find version field in ${cargoTomlPath}.`);
  process.exit(1);
}
await writeFile(cargoTomlPath, cargoTomlNext, "utf8");

const cargoCheck = spawnSync(
  "cargo",
  ["update", "--workspace", "-p", "bw-build-overlay", "--manifest-path", cargoTomlPath],
  {
    stdio: "inherit"
  }
);
if (cargoCheck.status !== 0) {
  console.warn("cargo update failed; you'll need to refresh src-tauri/Cargo.lock manually.");
}

git(`add ${pkgPath} ${changelogPath} ${tauriConfPath} ${cargoTomlPath} src-tauri/Cargo.lock`);
git(`commit -m "chore(release): v${newVersion}"`);
git(`tag -a v${newVersion} -m "v${newVersion}"`);

console.log("");
console.log(`Created commit and tag v${newVersion}.`);
console.log("Push to publish a GitHub Release (CI will build the EXEs):");
console.log("");
console.log("  git push origin main --follow-tags");
console.log("");

const npmCheck = spawnSync(process.platform === "win32" ? "npm.cmd" : "npm", ["--version"]);
if (npmCheck.status !== 0) {
  console.warn("npm is not on PATH; skipping npm install of the new version metadata.");
} else {
  spawnSync(process.platform === "win32" ? "npm.cmd" : "npm", ["install", "--package-lock-only"], {
    stdio: "inherit"
  });
}
