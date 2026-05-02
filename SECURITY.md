# Security policy

## Supported versions

Only the **latest released version** of BW Build Overlay receives security fixes. If you're running an older release, please update before reporting.

## Reporting a vulnerability

Please report vulnerabilities **privately** so they can be triaged before public disclosure.

- Open a private security advisory via the repository's [Security tab → Report a vulnerability](https://github.com/Etra-0/starcraft-build-overlay/security/advisories/new).
- Do **not** open a public issue, PR, or discussion thread for security-sensitive reports.

When reporting, include:

- A clear description of the issue and its impact.
- Steps to reproduce (or a minimal proof-of-concept).
- Affected version(s) and OS.
- Any suggested mitigation, if you have one.

## Response expectations

This is a hobby project maintained by a single author. Best-effort response targets:

- **Acknowledge** within 7 days of report.
- **Fix or mitigation plan** within 30 days for high/critical issues.
- **Coordinated disclosure** once a fix is released; credit will be offered to the reporter unless they request otherwise.

## Scope

**In scope** — security issues in code maintained in this repository:

- Rust backend (`src-tauri/src/**`).
- TypeScript renderer (`src/**`).
- Build / release pipeline (`.github/workflows/**`, `scripts/**`, `tauri.conf.json`).
- Default Tauri capability manifest (`src-tauri/capabilities/**`).

**Out of scope:**

- The Liquipedia MediaWiki API itself — report issues there to [Liquipedia](https://liquipedia.net).
- Third-party dependencies — report upstream (npm crates, Rust crates, GitHub Actions). If a dep advisory affects this project, file an issue here so the version pin can be bumped.
- Bugs without a security impact (overlay glitches, parser edge cases, etc.) — open a regular [issue](https://github.com/Etra-0/starcraft-build-overlay/issues) instead.
- Local-machine privilege escalation that requires the attacker to already be running as the user — the app stores user data under the per-user app-data dir and trusts the local OS account.
