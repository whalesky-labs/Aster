<p align="center">
  <img src="https://avatars.githubusercontent.com/u/277389313?s=200&v=4" width="128" height="128" alt="Aster">
</p>

<h1 align="center">Aster</h1>

<p align="center">
  A desktop client for hotel materials and inventory operations.
</p>

<p align="center">
  Inventory lifecycle · Excel import/export · Local disaster recovery · LAN collaboration
</p>

<p align="center">
  <a href="src-tauri/tauri.conf.json"><img src="https://img.shields.io/badge/Tauri-2-24C8DB?logo=tauri&logoColor=white" alt="Tauri 2"></a>
  <a href="package.json"><img src="https://img.shields.io/badge/React-19-61DAFB?logo=react&logoColor=111111" alt="React 19"></a>
  <a href="package.json"><img src="https://img.shields.io/badge/TypeScript-5-3178C6?logo=typescript&logoColor=white" alt="TypeScript 5"></a>
  <a href="src-tauri/Cargo.toml"><img src="https://img.shields.io/badge/Rust-stable-000000?logo=rust&logoColor=white" alt="Rust stable"></a>
  <a href="src-tauri/Cargo.toml"><img src="https://img.shields.io/badge/SQLite-local-003B57?logo=sqlite&logoColor=white" alt="SQLite local"></a>
  <a href=".github/workflows/build-desktop.yml"><img src="https://img.shields.io/badge/Windows%20%2B%20macOS-desktop-4B5563" alt="Windows and macOS"></a>
</p>

[中文默认](./README.md) | English

Aster is a Windows and macOS desktop client for internal hotel materials operations. It uses local SQLite as the source of truth and supports standalone usage, LAN host/client collaboration, complete inventory workflows, Excel import/export, user permissions, budget approvals, stocktaking, reporting, and local disaster recovery.

See the [Aster execution plan](docs/ASTER_EXECUTION_PLAN.md) for the complete product scope.

## Current Capabilities

- Manage suppliers, material categories, material records, warehouses, departments, users, and role permissions.
- Support inbound, outbound, transfer, adjustment, void, reversal, stocktaking, and inventory ledger workflows.
- Support Excel import/export, report export, budget rules, budget approvals, and business validation.
- Support standalone, host computer, and client computer modes; the host owns the only SQLite database, while clients connect to the host over LAN.
- Provide a connection wizard for enabling host sharing, LAN discovery, manual address entry, pairing-code input, and saved connection credentials.
- Support automatic backups, manual backups, a secondary backup directory, pre-import backups, pre-restore backups, restore validation tokens, rollback on failure, and retention policies.
- Support checking the latest GitHub Releases version online; release builds generate Tauri updater manifests, signed updater packages, and desktop installers.
- Include GitHub Actions builds for Windows and macOS, with the Windows installer produced by a GitHub Windows runner.

## Runtime Modes

| Mode | Use case | Data location |
| --- | --- | --- |
| Standalone | One computer works independently | Local SQLite on the current computer |
| Host computer | One LAN computer shares data for others | SQLite on the host computer |
| Client computer | Other computers connect to the host | Read/write through the host API |

In LAN collaboration, the host computer is the authoritative data source. Client computers do not directly copy the business database, which avoids multi-writer conflicts; local disaster recovery is handled by the host-side backup policy.

## Technical Architecture

| Layer | Current implementation |
| --- | --- |
| Desktop shell | Tauri 2 |
| Frontend | React 19 + TypeScript + Vite |
| Native services | Rust + Tauri commands |
| Local data | SQLite + Rust schema migration |
| File processing | Excel import/export, acceptance evidence, and local backup scripts |
| Sync model | LAN host API + client pairing token |
| Target platforms | Windows x64, macOS Intel, macOS Apple Silicon |

## Project Structure

```text
.
├─ .github/workflows/              GitHub Actions CI and desktop builds
├─ docs/                           Execution docs, recovery runbook, acceptance templates, and evidence
├─ scripts/                        Coverage, release, acceptance archive, and build helper scripts
├─ src/                            React frontend UI and business interactions
├─ src-tauri/                      Tauri config, Rust commands, SQLite, and native capabilities
├─ package.json                    Frontend, build, and acceptance script entry points
└─ tsconfig.json                   TypeScript configuration
```

## Development Commands

Install dependencies:

```bash
npm install
```

Start the frontend dev server:

```bash
npm run dev
```

Start the Tauri desktop app:

```bash
npm run tauri dev
```

Build the frontend:

```bash
npm run build
```

Build the desktop bundle:

```bash
npm run tauri -- build
```

Run a Rust check:

```bash
cd src-tauri
cargo check
```

## Verification And Acceptance

Run full release verification:

```bash
npm run verify:release
```

Run the local automated gate:

```bash
npm run verify:all-local
```

Verify execution-plan coverage:

```bash
npm run verify:coverage
```

Check acceptance readiness:

```bash
npm run verify:readiness
```

`verify:coverage` checks homepage, master data, inventory lifecycle, stocktaking, report export, Excel import, backup and disaster recovery, user permissions, host/client consistency, budget approvals, and cross-platform packaging against the execution plan, then generates a local coverage report.

`verify:release` runs coverage verification first, then generates release evidence under `docs/release-evidence/`. The report includes platform, architecture, tool versions, Tauri bundle configuration summaries, artifact paths, file sizes, file SHA256 values, and directory summaries. This directory is local acceptance output and is not committed to the repository.

`verify:all-local` runs build, manual-acceptance fixture tests, unfinished-marker scans, execution coverage, Rust formatting checks, Rust tests, release packaging, acceptance handoff package generation, readiness checks, handoff package self-checks, and archive generation. It proves only that the current machine passed automated gates; it does not replace real Windows/macOS acceptance.

## Cross-Platform Manual Acceptance

```bash
npm run acceptance:template -- windows
npm run acceptance:template -- macos
npm run acceptance:collect -- windows --force
npm run acceptance:collect -- macos --force
npm run acceptance:status
npm run acceptance:status -- --summary
npm run acceptance:package
npm run acceptance:archive
npm run acceptance:finalize
npm run verify:acceptance-package
npm run verify:manual-acceptance
npm run verify:manual-acceptance -- --strict
```

Manual acceptance records are stored in `docs/manual-acceptance/` and archive evidence for Windows/macOS installation, startup, login, database creation, Excel workflows, backup and restore, and two-way host/client real-machine testing.

`acceptance:package` generates `docs/acceptance-package/` for handing off acceptance docs, the latest local evidence, and cross-device acceptance instructions to testers. `acceptance:archive` generates `docs/acceptance-archives/aster-acceptance-package-*.zip` and a SHA256 archive report. After both Windows and macOS evidence is complete, `acceptance:finalize` runs strict manual acceptance first and refreshes the handoff package only when readiness reaches `ready-for-final-archive`.

## Release And CI

The current Tauri bundle includes Windows NSIS installer metadata and a macOS DMG layout. The Windows installer is generated by the `Build Desktop Bundles` GitHub Actions workflow on a Windows runner; macOS can generate `Aster.app` and DMG either locally or in the workflow.

- `.github/workflows/ci.yml` runs the frontend build, manual-acceptance fixture tests, unfinished-marker scan, execution coverage, Rust formatting checks, and Rust tests.
- `.github/workflows/build-desktop.yml` follows the Liberty desktop build flow: it prepares the version, builds macOS Intel, macOS Apple Silicon, and Windows x64 installers through a matrix, and can optionally publish a GitHub Release.
- When publishing a GitHub Release, the workflow uploads desktop installers, Tauri updater packages, signature files, and `latest.json`; the client reads `latest.json` for online updates.
- The macOS Intel job runs on `macos-15-intel` with the `x86_64-apple-darwin` target and generates `aster-<version>-macos-x86_64.dmg`.
- The macOS Apple Silicon job runs on `macos-15` with the `aarch64-apple-darwin` target and generates `aster-<version>-macos-aarch64.dmg`.
- The Windows job runs `npm run verify:release` on `windows-2022`, generates the installer, and uploads `aster-windows-x64` and `aster-windows-x64-release-evidence` artifacts.
- If the local machine has a `GITHUB_TOKEN` or `GH_TOKEN` with Actions read access, `npm run acceptance:download-windows-artifacts` can download the latest successful `Build Desktop Bundles` Windows artifacts and validate installer SHA256 values through the import script.
- After the branch is pushed to GitHub, `npm run acceptance:run-github-build` can trigger the `Build Desktop Bundles` workflow, poll the result, download Windows artifacts, and import them.

## Documentation

- [Aster execution plan](docs/ASTER_EXECUTION_PLAN.md)
- [Cross-platform acceptance guide](docs/ASTER_CROSS_PLATFORM_ACCEPTANCE.md)
- [Disaster recovery runbook](docs/ASTER_DISASTER_RECOVERY_RUNBOOK.md)
- [Manual acceptance guide](docs/manual-acceptance/README.md)
- [LAN connection wizard design](docs/superpowers/specs/2026-07-02-lan-connection-wizard-design.md)
- [Windows host sync acceptance evidence](docs/manual-acceptance/evidence-windows-2026-07-02/windows-host-sync-2026-07-02.md)
