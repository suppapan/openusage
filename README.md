# Track all your AI coding subscriptions in one place

See your usage at a glance from your menu bar. No digging through dashboards.

![OpenUsage Screenshot](screenshot.png)

## Download

[**Download the latest release**](https://github.com/robinebers/openusage/releases/latest) (macOS, Windows, Linux)

The app auto-updates. Install once and you're set.

## What It Does

OpenUsage lives in your menu bar and shows you how much of your AI coding subscriptions you've used. Progress bars, badges, and clear labels. No mental math required.

- **One glance.** All your AI tools, one panel.
- **Always up-to-date.** Refreshes automatically on a schedule you pick.
- **Global shortcut.** Toggle the panel from anywhere with a customizable keyboard shortcut.
- **Lightweight.** Opens instantly, stays out of your way.
- **Plugin-based.** New providers get added without updating the whole app.
- **[Local HTTP API](docs/local-http-api.md).** Other apps can read your usage data from `127.0.0.1:6736`.
- **[Proxy support](docs/proxy.md).** Route provider HTTP requests through a SOCKS5 or HTTP proxy.
- **Cross-platform.** Runs on macOS, Windows, and Linux (Ubuntu/Debian).
- **[Multi-machine sync](#multi-machine-sync).** Aggregate usage from multiple machines into one dashboard via a lightweight relay server.

## Supported Providers

- [**Amp**](docs/providers/amp.md) / free tier, bonus, credits
- [**Antigravity**](docs/providers/antigravity.md) / all models
- [**Claude**](docs/providers/claude.md) / session, weekly, peak/off-peak, extra usage, local token usage (ccusage)
- [**Codex**](docs/providers/codex.md) / session, weekly, reviews, credits
- [**Copilot**](docs/providers/copilot.md) / premium, chat, completions
- [**Cursor**](docs/providers/cursor.md) / credits, total usage, auto usage, API usage, on-demand, CLI auth
- [**Factory / Droid**](docs/providers/factory.md) / standard, premium tokens
- [**Gemini**](docs/providers/gemini.md) / pro, flash, workspace/free/paid tier
- [**JetBrains AI Assistant**](docs/providers/jetbrains-ai-assistant.md) / quota, remaining
- [**Kiro**](docs/providers/kiro.md) / credits, bonus credits, overages
- [**Kimi Code**](docs/providers/kimi.md) / session, weekly
- [**MiniMax**](docs/providers/minimax.md) / coding plan session
- [**OpenCode Go**](docs/providers/opencode-go.md) / 5h, weekly, monthly spend limits
- [**Windsurf**](docs/providers/windsurf.md) / prompt credits, flex credits
- [**Z.ai**](docs/providers/zai.md) / session, weekly, web searches

Community contributions welcome.

Want a provider that's not listed? [Open an issue.](https://github.com/robinebers/openusage/issues/new)

## Open Source, Community Driven

OpenUsage is built by its users. Hundreds of people use it daily, and the project grows through community contributions: new providers, bug fixes, and ideas.

I maintain the project as a guide and quality gatekeeper, but this is your app as much as mine. If something is missing or broken, the best way to get it fixed is to contribute by opening an issue, or submitting a PR.

Plugins are currently bundled as we build our the API, but soon will be made flexible so you can build and load their own.

<a href="https://www.star-history.com/?repos=robinebers%2Fopenusage&type=date&legend=top-left">
 <picture>
   <source media="(prefers-color-scheme: dark)" srcset="https://api.star-history.com/chart?repos=robinebers/openusage&type=date&theme=dark&legend=top-left" />
   <source media="(prefers-color-scheme: light)" srcset="https://api.star-history.com/chart?repos=robinebers/openusage&type=date&legend=top-left" />
   <img alt="Star History Chart" src="https://api.star-history.com/chart?repos=robinebers/openusage&type=date&legend=top-left" />
 </picture>
</a>

### How to Contribute

- **Add a provider.** Each one is just a plugin. See the [Plugin API](docs/plugins/api.md).
- **Fix a bug.** PRs welcome. Provide before/after screenshots.
- **Request a feature.** [Open an issue](https://github.com/robinebers/openusage/issues/new) and make your case.

Keep it simple. No feature creep, no AI-generated commit messages, test your changes.

## Cross-Platform Support

OpenUsage runs on all major desktop platforms:

| Platform | Format | Notes |
|----------|--------|-------|
| macOS (Apple Silicon) | `.dmg` | Floating menu bar panel |
| macOS (Intel) | `.dmg` | Floating menu bar panel |
| Windows | `.msi` / `.exe` (NSIS) | System tray app |
| Linux (Ubuntu/Debian) | `.deb` / `.AppImage` | System tray app |

On macOS, the app uses a native floating panel anchored to the menu bar. On Windows and Linux, it uses a system tray icon with a popup window that hides when it loses focus.

## Multi-Machine Sync

Track usage across multiple machines from a single dashboard. Run the full app on your main machine and lightweight agents on your servers.

### Architecture

```
┌─────────────────┐     ┌──────────────┐     ┌─────────────────┐
│  macOS Dashboard │◄───►│  Relay Server │◄─��──│ Ubuntu Agent    │
│  (full tray app) │pull │  (Docker/VPS) │push │  (headless CLI) │
└─────────────────┘     └──────────────┘     └─────────────────┘
```

### 1. Deploy the Relay Server

The relay is a lightweight HTTP server that brokers data between machines. Deploy it on any server with a public IP (VPS, home server, etc.):

```bash
# Using Docker
docker build -t openusage-relay -f crates/openusage-relay/Dockerfile .
docker run -d -p 8090:8090 --name openusage-relay openusage-relay

# Or build from source
cargo run -p openusage-relay
```

The relay listens on port `8090` by default. Set `RELAY_PORT` env var to change it.

**Relay API endpoints:**

| Method | Path | Auth | Description |
|--------|------|------|-------------|
| `POST` | `/v1/push` | Bearer token | Agent pushes usage snapshots |
| `GET` | `/v1/pull` | Bearer token | Dashboard pulls all machine data |
| `DELETE` | `/v1/machines/:id` | Bearer token | Remove a stale machine |
| `GET` | `/v1/health` | None | Health check |

### 2. Configure the Dashboard

In the OpenUsage desktop app:

1. Open **Settings**
2. Scroll to **Multi-Machine Sync**
3. Toggle **Enable sync**
4. Enter your relay URL (e.g. `http://your-server:8090`)
5. Click **Generate Token** and copy it

The dashboard will automatically poll the relay every 60 seconds and display remote machine data.

### 3. Install the Agent on Remote Machines (one-liner)

Paste one of these on the remote machine — it downloads the binary, prompts for the token and relay URL, and installs as a background service.

**Linux / macOS:**
```bash
curl -fsSL https://github.com/suppapan/openusage/releases/download/v0.7.0/install-agent.sh | bash
```

**Windows (PowerShell as user):**
```powershell
iwr https://github.com/suppapan/openusage/releases/download/v0.7.0/install-agent.ps1 | iex
```

Or non-interactive (pass token + relay as flags):

```bash
# Linux / macOS
curl -fsSL https://github.com/suppapan/openusage/releases/download/v0.7.0/install-agent.sh | bash -s -- \
  --token YOUR_SYNC_TOKEN \
  --relay https://relay.example.com:8090
```

```powershell
# Windows PowerShell
$env:OPENUSAGE_TOKEN = "YOUR_SYNC_TOKEN"
$env:OPENUSAGE_RELAY = "https://relay.example.com:8090"
iwr https://github.com/suppapan/openusage/releases/download/v0.7.0/install-agent.ps1 | iex
```

**What the installer does:**
- Downloads the platform-appropriate `openusage-agent` binary from the latest release
- Installs to `/usr/local/bin` (Linux/macOS) or `%LOCALAPPDATA%\OpenUsage` (Windows)
- Registers a background service: `systemd` (Linux), `launchd` (macOS), or a Scheduled Task (Windows)
- Starts pushing data to the relay every 5 minutes

**Installer options:**

| Flag | Default | Notes |
|------|---------|-------|
| `--token` | — | Sync token from dashboard (prompted if omitted) |
| `--relay` | — | Relay URL (prompted if omitted) |
| `--machine-name` | hostname | Display name shown in the dashboard |
| `--interval` | `300` | Push interval in seconds |
| `--no-service` | — | Install the binary only; don't register a service |

### Manual agent run (without installer)

Build from source and run directly:

```bash
cargo build -p openusage-agent --release

# Runs in foreground, reads from local OpenUsage HTTP API on port 6736
openusage-agent \
  --token YOUR_TOKEN_HERE \
  --relay https://relay.example.com:8090
```

**Agent data sources:**

- **Local API** (default): Reads from a running OpenUsage instance's HTTP API at `http://127.0.0.1:6736`. The agent just forwards cached data.
- **Cache file**: Pass `--cache-file <PATH>` to read directly from `usage-api-cache.json`. Useful when you want to skip HTTP.

### 4. View Aggregated Data

Once agents are pushing data, the dashboard overview page shows two tabs:

- **This Machine** — your local usage (default view)
- **All Machines** — usage from all connected machines, grouped by machine with badges showing machine names and last-seen timestamps

### Security Notes

- The sync token is a shared secret. Treat it like a password.
- The relay stores data in memory only (lost on restart; agents re-push within their next interval).
- Stale machine entries are automatically cleaned up after 24 hours of inactivity.
- Revoking a token in the dashboard makes the old token's data inaccessible. Agents using the old token will push to an orphaned room that gets cleaned up automatically.

## Built Entirely with AI

Not a single line of code in this project was read or written by hand. 100% AI-generated, AI-reviewed, AI-shipped — using [Cursor](https://cursor.com), [Claude Code](https://docs.anthropic.com/en/docs/claude-code), and [Codex CLI](https://github.com/openai/codex).

OpenUsage is a real-world example of what I teach in the [AI Builder's Blueprint](https://itsbyrob.in/EBDqgJ6) — a proven process for building and shipping software with AI, no coding background required.

## Sponsors

OpenUsage is supported by our sponsors. Become a sponsor to get your logo here and on [openusage.ai](https://openusage.ai).

[Become a Sponsor](https://github.com/sponsors/robinebers)

<!-- Add sponsor logos here -->

## Credits

Inspired by [CodexBar](https://github.com/steipete/CodexBar) by [@steipete](https://github.com/steipete). Same idea, very different approach.

## License

[MIT](LICENSE)

---

<details>
<summary><strong>Build from source</strong></summary>

> **Warning**: The `main` branch may not be stable. It is merged directly without staging, so users are advised to use tagged versions for stable builds. Tagged versions are fully tested while `main` may contain unreleased features.

### Prerequisites

- [Rust](https://rustup.rs/) (stable)
- [Bun](https://bun.sh/) (latest)
- Platform-specific dependencies:
  - **macOS**: Xcode Command Line Tools
  - **Linux**: `sudo apt install libwebkit2gtk-4.1-dev libgtk-3-dev libayatana-appindicator3-dev librsvg2-dev patchelf libdbus-1-dev libsoup-3.0-dev`
  - **Windows**: MSVC build tools

### Build the Desktop App

```bash
bun install
bun run bundle:plugins
bun tauri build
```

### Build the Relay Server

```bash
cargo build -p openusage-relay --release
# Binary at target/release/openusage-relay
```

### Build the Agent CLI

```bash
cargo build -p openusage-agent --release
# Binary at target/release/openusage-agent
```

### Run Tests

```bash
bun test              # Frontend tests
cargo test -p openusage-shared  # Shared types tests
```
