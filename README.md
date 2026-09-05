<div align="center">

<img src="./public/app-icon.svg" width="80" height="80" alt="EzDownload">

# EzDownload

A modern, high-performance desktop media downloader optimized for **Adobe Premiere Pro** workflows and macOS Apple Silicon (Mac mini M4) as well as Windows and Linux.

Built with **Tauri 2 + Rust** and **Vue 3 + TypeScript**. Forked from and attributing [imsyy/yt-dlp-gui](https://github.com/imsyy/yt-dlp-gui).

[![License: MIT](https://img.shields.io/badge/License-MIT-blue.svg)](LICENSE)
[![GitHub Release](https://img.shields.io/github/v/release/rabbi696/EzDownload?color=f0f0f0&labelColor=555555)](https://github.com/rabbi696/EzDownload/releases)
[![Build Status](https://img.shields.io/github/actions/workflow/status/rabbi696/EzDownload/build.yml?branch=master)](https://github.com/rabbi696/EzDownload/actions)

**English** | [简体中文](./README.zh-CN.md)

</div>

---

## Why EzDownload?

While yt-dlp is extremely capable, importing downloaded videos into non-linear video editors like **Adobe Premiere Pro** frequently causes issues:
- Downloading 4K or 1440p YouTube video often yields **AV1 (`av01`)** or **VP9 (`vp09`)** streams in `.mp4` or `.webm` containers, which Premiere Pro cannot import natively without missing codec errors, black screens, or no audio.
- YouTube caps **H.264 (`avc1`)** streams at 1080p, requiring intelligent format selection or lossless editing transcode.
- File integrity checks often rely solely on file extensions rather than true stream inspection.

**EzDownload solves this out of the box:**
- **Premiere Ready (H.264 MP4) Preset**: Downloads true H.264 video with AAC audio merged into an MP4 container (`bv*[vcodec^=avc1]+ba[acodec^=mp4a]`).
- **Codec Intelligence & Warnings**: Alerts you whenever a format uses AV1, VP9, or WebM, clearly explaining why (e.g. 4K stream capped in H.264) and offering one-click alternatives.
- **Post-Download FFprobe Verification**: Runs JSON stream inspection on completed files. Files are **never** labeled Premiere Ready unless confirmed by stream metadata.
- **One-Click Transcoding**: Converts incompatible media to **H.264 MP4** (`libx264` or hardware-accelerated `h264_videotoolbox` on macOS) or **Apple ProRes 422 LT MOV** (`prores_ks` + uncompressed PCM audio) for instant timeline drop-in.
- **Robust Process Management**: Uses process hierarchy signals (`pkill -P <pid>`) preventing orphaned FFmpeg background tasks on cancellation.
- **Apple Silicon Optimized**: Native macOS builds for Mac mini M4, MacBook Pro (M-series), and cross-platform compatibility for Windows and Linux.

## Features

### Premiere Pro Workflow
- **Premiere Ready Default Preset**: Directly compatible timeline video.
- **Codec Warning Banner**: Non-error, helpful guidance when AV1/VP9/WebM is selected.
- **Resolution Capping Explanations**: Seamlessly switch to highest H.264 or download 4K original with auto-transcode.
- **Verified Codec Badges**: Completed cards show ffprobe verified streams (`H.264 / AAC`, `ProRes 422 LT`, or `Incompatible`).
- **In-App Transcoding**: Real-time transcode progress bar, cancel control, and auto-conversion triggers.

### Core Downloader
- Paste video URL and instantly preview title, thumbnail, duration, and formats.
- Choose video quality, audio-only, or video-only downloads.
- Download queue with pause / resume / cancel controls and multi-process concurrency limits.
- Real-time progress with speed and ETA display.
- Playlist support — download all or selected items.
- Configurable concurrent fragments and fragment threading.

### Toolbox & Extras
- **Thumbnail Downloader** — browse and save all available cover images in any resolution.
- **Subtitle Extractor** — download subtitles in SRT / VTT / ASS / LRC with bilingual merge support.
- **Live Chat Archiver** — extract YouTube live chat replay, filter with regex, export as JSON / CSV.
- **Plugin Manager** — install yt-dlp plugins with one click.
- **Browser Extension** — companion helper extension for Chromium browsers.
- Custom filename templates, time clip trimming, SponsorBlock, and proxy support.

## Screenshots

| Home (Dark) | Home (Light) |
|:-:|:-:|
| ![Home](screenshot/home.png) | ![Home Light](screenshot/home-light.png) |

| Download Options | Extra Options |
|:-:|:-:|
| ![Download](screenshot/download.png) | ![Download Other](screenshot/download-other.png) |

| Downloading | Tools |
|:-:|:-:|
| ![Downloading](screenshot/downloading.png) | ![Tools](screenshot/tools.png) |

## Getting Started

### Download

Grab the latest release for your platform from [**Releases**](https://github.com/rabbi696/EzDownload/releases):

| Platform | File |
|----------|------|
| Windows  | `.exe` installer |
| macOS    | `.dmg` |
| Linux    | `.AppImage` / `.deb` / `.rpm` |

#### Arch Linux

Use the x86_64 AppImage from the release page. AppImage uses FUSE 2 on Arch, so install the compatibility package once, make the download executable, and run it:

```bash
sudo pacman -S fuse2
chmod +x YDL.GUI_*.AppImage
./YDL.GUI_*.AppImage
```

If FUSE cannot be enabled in your environment, run the same file with `--appimage-extract-and-run` instead. The AppImage is portable and does not require a system-wide package installation.

### First Launch

1. Open the app and go to **Settings**
2. Click **Download** next to yt-dlp — the binary is fetched automatically
3. *(Optional)* Install **Deno** runtime for full YouTube format support
4. Set your **download directory**
5. Go back to the home page, paste a URL, and start downloading

> [!TIP]
> If you encounter login-required videos, configure Cookie in settings using Netscape format text or a cookie file.

### Command-line automation

An installed YDL GUI instance can be opened from scripts with session-specific inputs:

```bash
yt-dlp-gui --url "https://example.com/video" \
  --cookies "/path/to/cookies.txt" \
  --dir "/path/to/downloads" \
  --yt-dlp-path "/path/to/yt-dlp" \
  --deno-path "/path/to/deno"
```

`--flag=value` is also accepted, and an HTTP(S) URL may be passed without `--url`. Tool path overrides apply only to the current app session and are never updated or replaced by the app. Re-running the command forwards the request to an already-open instance.

## Browser Extension

A companion **YDL GUI Helper** browser extension lives in [`browser-extension/`](./browser-extension/). It sends the current tab's URL and required cookies straight to the desktop app via a local protocol handler (`ytdlp-gui://`) — no copy-paste, no extra cookie export.

### Highlights

- One-click send from the popup, or right-click context menu (`Send page to YDL GUI` / `Download link with YDL GUI` / `Send selected URL to YDL GUI`)
- Action badge lights up automatically on supported video sites
- Auto light / dark theme that follows your system
- Cookies are processed locally — passed straight to the app via the local protocol, never uploaded anywhere

### Install (Chrome / Edge / Brave / Vivaldi etc.)

The extension is bundled with the app — no separate download required.

1. In the app, open **Toolbox → Browser Extension** and click **Open extension folder**.
2. Open `chrome://extensions` (or `edge://extensions`) and turn on **Developer mode** in the top-right.
3. Click **Load unpacked** and select the folder revealed in step 1.
4. Pin the YDL GUI Helper icon next to the address bar.

### Use

1. Open a supported video page (YouTube, Bilibili, Twitch, Vimeo, Twitter/X, TikTok, Instagram, Facebook, Reddit, SoundCloud, etc.).
2. Click the YDL GUI icon, or right-click the page / a video link and choose **Send to YDL GUI**.
3. The desktop app comes to the front automatically with the URL and cookies pre-filled.

> [!NOTE]
> Make sure the YDL GUI desktop app is installed and running for the protocol handler to fire.

## Tech Stack

| Layer | Technology |
|-------|-----------|
| Backend | [Tauri 2](https://tauri.app/) + [Rust](https://www.rust-lang.org/) |
| Frontend | [Vue 3](https://vuejs.org/) + [TypeScript](https://www.typescriptlang.org/) |
| UI | [Naive UI](https://www.naiveui.com/) |
| State | [Pinia](https://pinia.vuejs.org/) with persistence |
| Build | [Vite](https://vitejs.dev/) |
| i18n | [Vue I18n](https://vue-i18n.intlify.dev/) — zh-CN, zh-TW, en-US, ja-JP, ko-KR, es-ES, ru-RU |

## Development

### Prerequisites

- [Node.js](https://nodejs.org/) >= 22
- [pnpm](https://pnpm.io/) >= 11
- [Rust](https://www.rust-lang.org/tools/install)

### Setup

```bash
# Clone the repository
git clone https://github.com/rabbi696/EzDownload.git
cd EzDownload

# Install dependencies
pnpm install

# Run in development mode (Vite + Tauri)
pnpm tauri:dev

# Run tests
pnpm test
cargo test --manifest-path src-tauri/Cargo.toml

# Build for production
pnpm tauri:build
```

## Contributing

Contributions are welcome! Feel free to open an [issue](https://github.com/rabbi696/EzDownload/issues) or submit a pull request.

## License & Attribution

[MIT](LICENSE) &copy; 2026 [rabbi696](https://github.com/rabbi696).

Based on and forked from [yt-dlp-gui](https://github.com/imsyy/yt-dlp-gui) &copy; 2026 [imsyy](https://github.com/imsyy). All original copyrights and MIT terms preserved.
