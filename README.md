# Wisp

A fast, lightweight, open source launcher for your Epic Games library.

The official launcher is heavy (around 2 GB of RAM), slow to start, and full of
things most people never asked for. Wisp does a small set of things and tries to
do them well.

> **Status: in development.** Sprints 0 to 2 are done: Wisp sets up its own
> Legendary installation, signs in to Epic, and shows the games you own with
> their cover art and DLC. Installing and launching them arrive in sprints 3
> and 4.

## What Wisp does

- Sign in with your Epic account.
- Show the games you own, and their DLC.
- Download, install, update, verify/repair, move and uninstall games.
- Launch games.
- Sync cloud saves.
- Import games already installed by the official launcher, without
  re-downloading them.

## What Wisp does not do

No store. No friends, no chat, no news, no ads, no telemetry, nothing social.

Wisp is not a piracy tool: it only manages games you legally own on your own
Epic account, and it signs in through Epic's own login page.

## How it works

Wisp is a user interface on top of [Legendary](https://github.com/legendary-gl/legendary),
the open source Epic Games client. Wisp does not reimplement Epic's API.

On first launch it downloads the official Legendary binary (about 17 MB) from
Legendary's GitHub releases and checks it against the SHA-256 that GitHub
publishes for that file. There is nothing to install yourself: no Python, no
administrator rights, nothing added to your PATH. Legendary's configuration is
pinned inside Wisp's own data folder, so an existing Legendary or Heroic setup
is never read or written.

Wisp checks for a newer Legendary at startup and offers it. It never updates
anything on its own. If the binary ever stops working, "Repair runtime" in
settings downloads it again.

- **Frontend:** React + TypeScript + Vite
- **Backend:** Rust, via [Tauri 2](https://tauri.app)
- **Epic client:** Legendary

## Performance goals

| Goal                             | Target                                     |
| -------------------------------- | ------------------------------------------ |
| Cold start to usable window      | < 2 s on a normal PC                       |
| Idle RAM (including WebView2)    | < 200 MB                                   |
| Background processes when closed | none, unless you enable "minimise to tray" |

Measured numbers per sprint live in [docs/performance.md](docs/performance.md).

## Building from source

Requirements:

- [Rust](https://rustup.rs) (stable) with the MSVC toolchain on Windows
- [Node.js](https://nodejs.org) 20 or newer and [pnpm](https://pnpm.io)
- Windows: Visual Studio Build Tools with the "Desktop development with C++"
  workload, and WebView2 (already present on up-to-date Windows 10/11)

```bash
pnpm install
pnpm tauri:dev      # run in development
pnpm tauri:build    # build a release binary
pnpm check          # lint, typecheck, format check, clippy and tests
```

## Project layout

```
src/                    React frontend
  components/           Shared UI pieces
  hooks/                React hooks
  i18n/                 Translations (one JSON file per language)
  lib/                  IPC wrappers, shared types, routing
  pages/                Top-level screens
  styles/               Design tokens and base styles
src-tauri/src/          Rust backend
  platform/             Per-OS paths and behaviour (windows.rs, linux.rs)
  runtime/              The private Legendary installation Wisp downloads
  legendary/            The only module that talks to the Legendary CLI
  auth/                 Signing in to Epic
  library/              Owned games, DLC, metadata, cover art
  downloads/            Download queue and install jobs
  settings/             settings.json
```

Anything platform-specific lives in `platform/` behind a trait, so Linux
support (chapter 2) means adding an implementation, not editing call sites.

## Where Wisp stores data

**Installed (Windows)**

| What                        | Where                                                          |
| --------------------------- | -------------------------------------------------------------- |
| Settings                    | `%APPDATA%\Wisp`                                               |
| Legendary binary + its data | `%LOCALAPPDATA%\Wisp\runtime`, `%LOCALAPPDATA%\Wisp\legendary` |
| Cover art cache             | `%LOCALAPPDATA%\Wisp\cache`                                    |
| Logs                        | `%LOCALAPPDATA%\Wisp\logs`                                     |

**Portable**

Put an empty file named `portable.flag` next to `wisp.exe` and everything
(settings, Legendary, cache, saved data) goes into `.\data\` next to the
executable instead. Nothing is written outside that folder, so Wisp runs from a
USB stick.

## Language support

English and Spanish ship today; Portuguese, French and German arrive in
sprint 5. Wisp follows your system language on first run and falls back to
English. No UI text is hardcoded: everything goes through the translation
files in `src/i18n/locales/`.

## Licence

[GPL-3.0-only](LICENSE), the same licence as Legendary.
