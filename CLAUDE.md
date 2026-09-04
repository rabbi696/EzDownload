# CLAUDE.md

This file provides guidance to Claude Code (claude.ai/code) when working with code in this repository.

## Project Overview

**EzDownload** is a desktop application for downloading videos via yt-dlp, optimized for **Adobe Premiere Pro** compatibility and Apple Silicon / cross-platform desktop use. Built with **Tauri 2** (Rust backend) + **Vue 3** (TypeScript frontend). Forked from and attributing `imsyy/yt-dlp-gui`.

## Development Commands

```bash
pnpm install          # Install frontend dependencies
pnpm tauri dev        # Run the full app in development (starts Vite + Rust backend)
pnpm dev              # Run frontend only (Vite dev server on port 5688)
pnpm test             # Run frontend unit tests (node:test)
pnpm typecheck        # Type-check frontend with vue-tsc
pnpm build            # Type-check and build frontend (vue-tsc + vite build)
pnpm tauri build      # Build production app bundle
```

Rust backend builds are handled by Tauri automatically during `pnpm tauri dev` / `pnpm tauri build`. To test or check Rust code independently:
```bash
cd src-tauri && cargo test
cd src-tauri && cargo check
```

## Architecture

### Frontend (`src/`)
- **Vue 3 + TypeScript** with `<script setup>` SFCs
- **Naive UI** component library, auto-imported via `unplugin-vue-components` (NaiveUiResolver)
- **Auto-imports** configured in `vite.config.ts`: Vue, Vue Router, VueUse APIs, and Naive UI composables are available without explicit imports
- **Pinia** for state with `pinia-plugin-persistedstate` for localStorage persistence
- **Path alias**: `@` maps to `src/`
- **Pages**: Home (URL search/batch parse), Pending (format selection & Premiere preset), Downloads (progress, verified badges, transcode actions), Settings (options, Premiere defaults, tools)
- **Premiere Utilities**: `src/utils/formats.ts` classifies H.264/ProRes vs AV1/VP9/WebM, detects 4K resolution capping, and builds Premiere Ready selector strategies
- **Tauri IPC**: Frontend calls Rust commands via `invoke()` from `@tauri-apps/api/core`

### Backend (`src-tauri/src/`)
- `lib.rs` — Tauri app builder, registers all commands and plugins
- `commands/` — Tauri command handlers:
  - `probe.rs` — ffprobe JSON stream analysis, Premiere Pro compatibility verification
  - `transcode.rs` — FFmpeg conversion to H.264 MP4 (`libx264`/`h264_videotoolbox`) and Apple ProRes 422 LT MOV (`prores_ks`), emitting progress and canceling safely
  - `download/` — yt-dlp download process lifecycle, format argument construction (`--remux-video mp4` / Premiere presets), process tree cleanup, output verification
  - `setup.rs` — platform info, yt-dlp/Deno installation management
  - `video.rs` — video info fetching (`-J`), cookie management
- `platform/process.rs` — OS-level process control (Unix `pkill -P <pid>` tree kill preventing orphaned ffmpeg processes)
- `utils.rs` — Path helpers (yt-dlp, FFmpeg, FFprobe, Deno)
- Binaries (yt-dlp, Deno) are downloaded to the Tauri app data directory at runtime, not bundled
- Progress events emitted to frontend via `app.emit()` (e.g., `ytdlp-download-progress`, `deno-download-progress`)
- Download progress uses `--progress-template` (structured JSON) instead of parsing stdout text
- Final output file path retrieved via `--print-to-file after_move:filepath` to avoid Windows GBK encoding issues

### Frontend-Backend Communication
- Tauri commands are invoked from Vue via `invoke<T>("command_name", { args })`
- Real-time progress uses Tauri event system (`app.emit` on Rust side)
- Shared types in `src/types/index.ts` mirror Rust structs in `commands/mod.rs`

## Key Conventions

- Windows builds use `CREATE_NO_WINDOW` flag (0x08000000) on all subprocess spawns to hide console windows
- All yt-dlp commands set `PYTHONUTF8=1` environment variable and use `--ignore-config --color never`
- Deno is optional — used as JS runtime for yt-dlp when installed (`--js-runtimes` flag)
- Cookie support: text (Netscape format saved to file) or direct file path
