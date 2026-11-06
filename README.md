<p align="center">
  <img src="zbase.png" width="128" alt="zBase icon" />
</p>

<h1 align="center">zBase</h1>

<p align="center">
A fast, native desktop chat client for <a href="https://keybase.io">Keybase</a>, built with Rust and <a href="https://gpui.rs">GPUI</a>.
</p>

<p align="center">
  <img src="docs/screenshot.png" alt="zBase screenshot" />
</p>

zBase is a from-scratch replacement for the official Keybase desktop app. It talks directly to the Keybase daemon over its local msgpack-RPC socket and renders everything through GPUI — the same GPU-accelerated UI framework that powers the [Zed](https://zed.dev) editor. zBase keeps a local message cache for quick navigation and search, with light and dark themes that follow Keybase’s colors.

## Install

The recommended installation is the **signed and notarized macOS app**. You do not need Rust, Homebrew, or separate media libraries.

1. Make sure you have **macOS 15 or later on an Apple Silicon Mac** and [Keybase](https://keybase.io/download) installed and running.
2. Download the Apple Silicon `.dmg` from the [latest GitHub release](https://github.com/cameroncooper/zbase/releases/latest).
3. Open the DMG and drag **zBase.app** to **Applications**, then launch zBase from Applications or Spotlight.

zBase uses the local Keybase service and your existing Keybase account. Keep the Keybase app or `keybase service` running while using zBase. Official DMGs bundle their media libraries; Intel Macs are not supported by these downloads.

### Optional: Homebrew source installation

Homebrew compiles zBase locally and installs its build dependencies. Use this if you prefer a source installation; it is not required for the DMG.

```sh
brew install cameroncooper/tap/zbase
```

After compilation, link the app bundle into Applications, provided there is not already a zBase.app installed there:

```sh
ln -s "$(brew --prefix)/opt/zbase/zBase.app" /Applications/zBase.app
```

## Features

### Chat
- Channels, DMs, and group DMs organized by team/workspace
- Threaded conversations with reply previews, reply counts, and a dedicated right pane
- Send, edit, and delete messages
- Emoji reactions (standard Unicode and custom team emoji)
- Emoji picker with skin tone preference and recents
- File, image, video, and audio attachments with drag-and-drop upload
- Inline video previews decoded via FFmpeg
- Link previews with thumbnails
- Rich inline markdown: **bold**, *italic*, ~~strikethrough~~, ||spoilers||, `code`, code blocks, and block quotes
- Selectable, copyable message text with styled ranges and clickable links
- @mentions, #channel mentions, and @here/@all broadcasts
- Pinned messages per conversation
- Leave eligible team channels from the channel Details panel

### Navigation
- **Recent / Grouped sidebar** — choose **View › Sort by Recent** or **Group by Team** (`Cmd+Option+1` / `Cmd+Option+2`), the pop-up on the sidebar's "Recent" or "Teams" label, or **Settings › Sidebar order**; the choice is saved per account. Recent is the default for new accounts, keeps read conversations in place, and shows shortened team names inline.
- **Grouped Unread** — one-line shortcuts to conversations with unread activity; the open conversation stays listed (marked, not selected twice) until you navigate away
- **Sidebar rows** — names only, or with previews of the newest message, someone typing, your draft or a failed send (Settings › Sidebar rows)
- **Quick Switcher** (`Cmd+K`) — fuzzy search across all conversations, ranked by usage affinity
- **Find in Chat** (`Cmd+F`) — search within the current conversation with match-by-match navigation
- **Global Search** (`Cmd+J`) — full-text search across workspaces with filters (by user, channel, has file, has link, mentions me) and highlighted snippets
- **Command Palette** (`Cmd+Shift+P`)
- Keyboard-driven sidebar navigation (`Alt+Up/Down`)
- Back/forward history (`Cmd+[` / `Cmd+]`)
- Jump to recent conversations (`Cmd+1` through `Cmd+5`)
- Deep link support (`keybase://chat/...`)

### User Profiles
- Avatars, bio, location, and title
- Identity proofs (Twitter, GitHub, etc.) with verification state
- Social graph — followers and following with follow/unfollow
- Team showcase
- Presence indicators (active, away, do not disturb, offline) with status text


### Preferences
- Light, dark, and system-follow theme modes
- Comfortable and compact density
- Reduced motion toggle
- Per-account sidebar section ordering and collapse state
- Remembered window size and position, with temporary space for profiles and threads excluded from your preferred size
- Persistent settings stored in `.zbase/settings.conf`

### Performance
- Built-in benchmark harness (`Cmd+Shift+B`) with per-component timers (render, sidebar, main panel, composer, reducer, backend events)
- CSV export and configurable scenarios via environment variables
- RocksDB local cache for offline-ready conversation and message history
- Tantivy full-text search index with automatic schema migration
- Metadata-only history catch-up and local search indexing avoid downloading every historical attachment

## Usage

Launch **zBase** from Applications or Spotlight. To open the installed app from a terminal:

```sh
open /Applications/zBase.app
```

View › Sort by Recent / Group by Team switches the sidebar order. Opening a profile or thread adds space to a compact window when needed; closing the panel restores its base width.

### Environment variables

| Variable | Description |
|---|---|
| `ZBASE_LOG_LEVEL` | Logging verbosity: `trace`, `debug`, `info`, `warn`, `error` |
| `ZBASE_BENCH_AUTOSTART` | Start the performance benchmark automatically on launch |
| `ZBASE_BENCH_DURATION_SECS` | Benchmark capture duration |
| `ZBASE_BENCH_SCENARIO` | Benchmark scenario name |
| `ZBASE_BENCH_LABEL` | Label for the benchmark run |
| `ZBASE_BENCH_OUTPUT` | Path for benchmark CSV output |

### Keyboard shortcuts

| Shortcut | Action |
|---|---|
| `Cmd+K` | Quick Switcher |
| `Cmd+J` | Global Search |
| `Cmd+F` | Find in Chat |
| `Cmd+N` | New Chat |
| `Cmd+Return` | New Chat: start the conversation |
| `Cmd+B` / `Cmd+I` / `Cmd+E` | Composer: bold / italic / code (also in the **Aa** menu) |
| `Cmd+Shift+P` | Command Palette |
| `Cmd+1` | Home |
| `Cmd+2–5` | Recent conversations |
| `Cmd+[` / `Cmd+]` | Navigate back / forward |
| `Cmd+Option+1` / `Cmd+Option+2` | Sidebar: sort by recent / group by team |
| `Cmd+Shift+\` | Toggle thread pane |
| `Cmd+Shift+M` | Toggle members pane |
| `Cmd+Shift+D` | Toggle details pane |
| `Cmd+Shift+F` | Files pane |
| `Cmd+Shift+A` | Activity |
| `Cmd+,` | Settings |
| `Alt+Up/Down` | Open the previous/next visible sidebar conversation |
| `Escape` | Dismiss overlays |

## Architecture

```
src/
├── app/          # Bootstrap, window management, theme, key bindings
├── domain/       # Core types: messages, conversations, users, profiles
├── models/       # View models (sidebar, timeline, composer, search, settings, …)
├── state/        # Unidirectional store: actions → reducer → state → effects
├── services/     # Backend adapters, RocksDB cache, Tantivy search, sync, uploads
│   └── backends/
│       └── keybase/  # Keybase RPC client and adapter
├── util/         # Fuzzy matching, video decoder, deep links, debounce, perf harness
├── views/        # GPUI components (sidebar, timeline, composer, overlays, profile, …)
└── main.rs
```

The backend layer is trait-based (`ChatBackend`), making it possible to add adapters for other chat providers in the future.

## Building from source

Source builds require macOS 15+, Xcode Command Line Tools, stable Rust with 2024 edition support, and the following Homebrew dependencies:

```sh
xcode-select --install
brew install cmake pkgconf ffmpeg
```

With Rust installed, clone the repository and build the app:

```sh
git clone https://github.com/cameroncooper/zbase.git
cd zbase
scripts/macos/build_app_bundle.sh release
open dist/macos/zBase.app
```

This creates a development app at `dist/macos/zBase.app` using your installed media libraries. For terminal-based development:

```sh
cargo run --locked --release --features gpui/runtime_shaders
```

To prepare a release, run **Actions → Prepare release** on `main` with the next version. Merge its version-bump PR after CI passes, then create and publish the matching release in GitHub. For self-contained packaging, Developer ID signing, notarization, and the full publishing process, see [macOS releases](docs/macos-releases.md).

### Checks before opening a PR

```sh
cargo clippy --locked --all-targets --features gpui/runtime_shaders
cargo test --locked --all-targets --features gpui/runtime_shaders
python3 -m unittest discover -s scripts -p 'test_*.py' -v
python3 -m unittest discover -s scripts/macos -p test_bundle_dylibs.py -v
```

GitHub Actions runs workflow/script linting, Clippy, Rust tests, and macOS packaging fixture tests on PRs and pushes to `main`. PRs skip app bundle creation, archiving, and upload. Pushes to `main` and manual CI runs additionally build a debug app using the dependencies compiled for checks and tests, then verify its minimum OS, architecture, bundled library dependencies, and ad-hoc signatures. The `macos-development-app` artifact from those runs contains an unoptimized development build for inspection; it is not Developer ID signed or notarized. The `macos-cargo-timings` artifact contains Cargo's build timing reports, including on PRs.

Media dependencies are cached as soon as their build succeeds. Rust build outputs are saved even if later validation fails, with compiler-specific cache keys. The separate Release workflow runs tests in release mode and builds the optimized app using the same profile, then signs and notarizes it with release credentials.

Unit and packaging tests do not require Keybase or a logged-in account. Tests requiring a live service remain opt-in. Native interactive UI checks are still performed separately; CI does not verify on-screen rendering or live account synchronization.

## Development tools

- **`keybase_rpc_inspect`** — standalone binary for inspecting live Keybase RPC traffic:
  ```sh
  cargo run --bin keybase_rpc_inspect
  ```
- **Keybase Inspector** (`Cmd+Shift+I`) — in-app RPC debug panel

## License

MIT
