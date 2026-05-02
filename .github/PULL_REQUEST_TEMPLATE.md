## Summary

<!-- One or two sentences on what this PR does and why. -->

## User-visible changes

<!--
If this PR changes anything a user can feel (overlay behavior, hotkeys, import
flow, build/release pipeline, etc.), add a bullet to `## [Unreleased]` in
CHANGELOG.md under Added / Changed / Fixed / Removed. Pure refactors, docs, or
test-only PRs can use an `### Internal / Tooling` bullet instead and do not
require a release.
-->

- [ ] Added a CHANGELOG entry (user-visible), or
- [ ] Marked as Internal / Tooling in `## [Unreleased]`, or
- [ ] No CHANGELOG entry needed (pure docs / meta change).

## Pre-merge checklist

Run locally before requesting review (see [CONTRIBUTING.md](../CONTRIBUTING.md)):

- [ ] `npm run format:check`
- [ ] `npm run lint`
- [ ] `npm run typecheck`
- [ ] `cargo fmt --all --check --manifest-path src-tauri/Cargo.toml`
- [ ] `cargo clippy --all-targets --manifest-path src-tauri/Cargo.toml -- -D warnings`
- [ ] `npm test`
- [ ] `npm run tauri:build` (only if build/packaging is affected)
- [ ] Renderer smoke (only if renderer changed): `npm run dev`, confirm the
      overlay boots, tabs respond to clicks, and at least one hotkey fires.

Use `gh pr checks --json name,bucket,state,workflow,link` to confirm PR-attached
checks are green before merging — `gh run list` only covers GitHub Actions.

## Risk / rollback

<!--
Blast radius of the change and how to revert if something goes wrong in
production (e.g. revert commit, version pin, schema rollback plan).
-->

## Related issues

<!-- Fixes #N / Refs #N, or "none". -->
