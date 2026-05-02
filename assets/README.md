# assets

- `icon.svg` is the source of truth for the app icon (512x512 logical, multi-color, used in the in-app brand mark).
- `icon.png` is a 512x512 PNG of the same artwork, used as the input to Tauri's icon generator.

## Regenerating the icons

Tauri's CLI generates every platform-specific icon from a single PNG. After editing `icon.png` (or `icon.svg` and re-exporting it to PNG), run:

```bash
npx tauri icon assets/icon.png
```

That repopulates [`src-tauri/icons/`](../src-tauri/icons/) with `icon.ico` (Windows), `icon.icns` (macOS), and the various PNG sizes Tauri needs for Linux / mobile bundles. Those files are checked in, so a fresh clone has working icons without any extra steps. Don't hand-edit anything under `src-tauri/icons/` — it's generated output.
