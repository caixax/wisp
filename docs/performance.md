# Performance measurements

Wisp has two hard budgets: a usable window in under 2 seconds from a cold
start, and under 200 MB of idle RAM including WebView2. Every chapter 1 sprint
ends with a fresh measurement so regressions are caught while they are still
cheap to fix.

## Method

- Release build (`pnpm tauri build --no-bundle`), not a dev build.
- **Startup** is measured from process start until the main window handle
  exists. That is slightly earlier than first paint, so treat it as a lower
  bound rather than a precise "time to interactive".
- **Memory** covers `wisp.exe` and every WebView2 process descended from it,
  sampled 10 seconds after the window appears with the app idle on its default
  screen. Two numbers are reported because they mean different things:
  - **Private working set** — RAM that belongs only to Wisp. This is what Task
    Manager shows for an app, and it is the number the 200 MB budget refers to.
  - **Total working set** — the same processes including shared Edge/WebView2
    pages. Those pages are counted once per process and are shared with any
    other WebView2 app and with Edge itself, so this figure overstates what
    Wisp actually costs the machine. It is tracked anyway as an upper bound.
- Measurements are taken on the development machine described below, which is
  the "normal PC" the targets are judged against.

Reproduce with the script kept alongside these notes:

```powershell
powershell -ExecutionPolicy Bypass -File docs/measure-performance.ps1
```

## Machine

|      |                             |
| ---- | --------------------------- |
| OS   | Windows 10 Pro 22H2 (19045) |
| Rust | 1.98.0 (MSVC)               |
| Node | 24.19.0                     |

## Results

| Sprint       | Date       | Startup to window | Idle private WS | Idle total WS | Processes | `wisp.exe` |
| ------------ | ---------- | ----------------- | --------------- | ------------- | --------- | ---------- |
| 0 — skeleton | 2026-10-01 | 77 ms             | 96 MB           | 317 MB        | 7         | 3.95 MB    |

Budget: **< 2 s** startup, **< 200 MB** idle private working set.

### Notes per sprint

**Sprint 0.** Baseline: an empty shell with a sidebar, three placeholder
screens and a settings page. No Legendary runtime, no network calls, no cover
art. Later sprints are only comparable against this if the app is measured
idle on its default screen.

- 77 ms is the window handle appearing, not first paint. The frontend bundle
  is 282 KB (89 KB gzipped) and renders from local files, so the gap between
  the two is small, but "under 2 s" should be re-checked with a real library
  of games loaded in sprint 2.
- Six of the seven processes belong to WebView2 (browser, GPU, network,
  renderer, utility, crashpad). That split is fixed by the runtime; the only
  real lever is how much the renderer allocates.
- Private working set has 104 MB of headroom against the budget. Cover art
  caching (sprint 2) and the download queue (sprint 3) are the two features
  most likely to eat into it.
