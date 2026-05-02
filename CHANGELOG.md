# Changelog

All notable changes to this project will be documented in this file.

The format is based on [Keep a Changelog](https://keepachangelog.com/en/1.1.0/),
and this project follows [Semantic Versioning](https://semver.org/spec/v2.0.0.html):

- **MAJOR** version when there are incompatible API or data-format changes.
- **MINOR** version when functionality is added in a backwards-compatible way.
- **PATCH** version for backwards-compatible bug fixes.

## How to add a changelog entry

When you make a user-visible change, append a bullet under the appropriate section of `## [Unreleased]`:

```
## [Unreleased]

### Added
- New `Foo` button in the Manager that does Bar.

### Changed
- `Manage > Settings > Rate limit` now defaults to 2300 ms.

### Fixed
- Liquipedia bulk import no longer skips pages whose title contains parentheses.

### Removed
- Dropped support for the legacy v3 builds.json schema.
```

When `npm run release:patch|minor|major` is run, the last `## [Unreleased]` heading is renamed to `## [X.Y.Z] - YYYY-MM-DD` and a fresh empty `## [Unreleased]` block is inserted on top.

---

## [Unreleased]

_No unreleased changes yet._

## [1.0.0] - 2026-05-01

Initial public release. Cross-platform Tauri 2 + Rust desktop overlay for StarCraft: Brood War / Remastered, with a built-in Liquipedia importer and update scanner. Native bundles for Windows, macOS, and Linux.

### Overlay

- Always-on-top desktop overlay for Windows, macOS, and Linux. Runs on top of StarCraft when the game is in **Windowed (Fullscreen)** mode.
- All 9 race matchups (TvT/TvP/TvZ, PvT/PvP/PvZ, ZvT/ZvP/ZvZ) with race-themed colors.
- Build orders shown as a clean paginated list with configurable page size — no per-step "click next".
- Race-color accent stripe on the build card, alternating-row steps list, hairline borders, translucent panels, soft toasts.
- Search, favorites, compact mode (auto-engages on short windows below 560 px tall), opacity slider, configurable default race.
- Cross-platform window opacity: `SetLayeredWindowAttributes` on Windows, `NSWindow setAlphaValue:` on macOS, `gtk_widget_set_opacity` on Linux.
- Global hotkeys: race / opponent / next-prev build / page paging / favorite / compact / hide.
- `F12` and `Ctrl+Shift+I` toggle DevTools in any build; `BW_DEVTOOLS=1` env var auto-opens DevTools on launch.
- Fatal-error red banner inside the overlay window if the renderer fails to boot.
- Custom hex + BW monogram app icon (`assets/icon.svg`); reads cleanly at 16 px and 256 px.

### Liquipedia integration

- `{{build}}` template parser with multi-variant pages, infobox extraction (creator, popularizer, race, matchups), counters and difficulty tags.
- Single-page preview/import and bulk import per race in "common" (curated subset) or "all" modes.
- Updates tab diffs each Liquipedia-sourced build's stored revision id against the latest wiki revision; selectively or wholesale refresh outdated builds while preserving favorite, userNotes, and recentlyUsedAt.
- Custom-edited builds (`customEdited` flag) are protected from refreshes unless explicitly forced.
- Configurable User-Agent and rate-limit (default 2300 ms) for Liquipedia API calls.

### Architecture

- **Tauri 2 + Rust backend.** UI runs in the OS-bundled WebView (WebView2 / WKWebView / WebKitGTK); no Chromium download. Windows installer is ~5–10 MB; idle RAM ~30–80 MB.
- **TypeScript renderer**, bundled by esbuild into a single self-contained ESM file (`dist-frontend/renderer.js`). The renderer uses `@tauri-apps/api/core invoke()` and `@tauri-apps/api/event listen()` to call into Rust.
- **Rust backend** (`src-tauri/src/`) owns: persistent storage (`storage.rs`, `SCHEMA_VERSION = 4`), Liquipedia client + parser + importer (`liquipedia/`), window/opacity/global-shortcut wiring (`window.rs`), and the `#[tauri::command]` surface (`commands.rs`).

### Build + release pipeline

- Per-OS bundles produced by `npm run tauri:build`: NSIS `.exe` + `.msi` (Windows), universal `.dmg` + `.app.tar.gz` (macOS), `.AppImage` + `.deb` (Linux).
- `npm run dev` runs `esbuild --watch` for the renderer plus `npx tauri dev` for the Rust backend; renderer changes hot-reload, Rust changes recompile + relaunch.
- `npm run size` (via [scripts/report-size.mjs](scripts/report-size.mjs)) prints the size of the latest bundle artifacts.
- `scripts/release.mjs`: `npm run release:patch|minor|major` bumps the version across `package.json` / `Cargo.toml` / `tauri.conf.json`, refreshes `Cargo.lock`, rotates the **last** `## [Unreleased]` heading in CHANGELOG.md, commits as `chore(release): vX.Y.Z`, and tags `vX.Y.Z`.
- GitHub Actions CI: a `quality` job (Ubuntu) runs `format:check`, `lint`, `typecheck`, `cargo fmt --check`, `cargo clippy -- -D warnings`, and `cargo test`; a `package` matrix (Windows + macOS + Ubuntu) builds the renderer and runs `cargo build --tests`.
- GitHub Actions Release: tag-push triggers a Windows + macOS + Ubuntu matrix using [`tauri-apps/tauri-action`](https://github.com/tauri-apps/tauri-action), uploads bundles to a draft Release named `BW Build Overlay vX.Y.Z`.
- Node 24 LTS pinned via `package.json` `engines` and `.nvmrc`. Rust pinned via [`rust-toolchain.toml`](rust-toolchain.toml) (`stable` channel, `clippy` + `rustfmt` components).

### Caveats

- **macOS `.app` / `.dmg` are unsigned.** First launch is blocked by Gatekeeper; right-click → Open the first time, or run `xattr -d com.apple.quarantine /Applications/"BW Build Overlay.app"`.
- **Linux `.deb` depends on `libwebkit2gtk-4.1-0`.** AppImage is a self-contained alternative for distros that don't ship that package.
- **Always-on-top vs Windows taskbar.** Tauri 2 has a known issue where clicking the taskbar can drop the overlay behind it. The app installs a focus-loss listener that re-applies `set_always_on_top(true)` to mitigate this; SC:R itself must run in **Windowed (Fullscreen)** mode for any always-on-top window to work.
