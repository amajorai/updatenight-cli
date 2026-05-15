# Update Night CLI

[![Release](https://github.com/amajorai/updatenight-cli/actions/workflows/release.yml/badge.svg)](https://github.com/amajorai/updatenight-cli/actions/workflows/release.yml)

Terminal UI for browsing the Update Night catalog of AI dev tools, agent frameworks, MCP servers, and AI news.

## Install

### Option 1 — cargo-binstall (pre-built binary, fastest)

```sh
cargo binstall un
```

Installs the pre-built binary for your platform. Get `cargo-binstall` from [cargo-bins/cargo-binstall](https://github.com/cargo-bins/cargo-binstall) if you don't have it.

### Option 2 — Download binary

Go to [Releases](https://github.com/amajorai/updatenight-cli/releases) and grab the archive for your platform:

| Platform | File |
|----------|------|
| macOS Apple Silicon | `un-aarch64-apple-darwin.tar.gz` |
| macOS Intel | `un-x86_64-apple-darwin.tar.gz` |
| Linux x86_64 | `un-x86_64-unknown-linux-gnu.tar.gz` |
| Linux ARM64 | `un-aarch64-unknown-linux-gnu.tar.gz` |
| Windows x86_64 | `un-x86_64-pc-windows-msvc.zip` |

**macOS / Linux:**

```sh
# Replace <target> with your platform (e.g. aarch64-apple-darwin)
curl -fsSL https://github.com/amajorai/updatenight-cli/releases/latest/download/un-<target>.tar.gz \
  | tar xz -C /tmp
sudo mv /tmp/un /usr/local/bin/un
```

**Windows (PowerShell):**

```powershell
# Run as Administrator
$target = "x86_64-pc-windows-msvc"
$url = "https://github.com/amajorai/updatenight-cli/releases/latest/download/un-$target.zip"
Invoke-WebRequest $url -OutFile "$env:TEMP\un.zip"
Expand-Archive "$env:TEMP\un.zip" -DestinationPath "$env:TEMP\un-bin"
Move-Item "$env:TEMP\un-bin\un.exe" "C:\Windows\System32\un.exe"
```

### Option 3 — Build from source

```sh
cargo install --git https://github.com/amajorai/updatenight-cli --bin un
```

Or clone and build:

```sh
git clone https://github.com/amajorai/updatenight-cli
cd updatenight-cli
cargo build --release
# binary at target/release/un (or un.exe on Windows)
sudo mv target/release/un /usr/local/bin/un
```

## Updating

Re-run whichever install method you used. For `cargo-binstall`:

```sh
cargo binstall un
```

## Usage

```
un           # open the TUI
un login     # authenticate with your Update Night account
un logout    # remove stored credentials
un --help    # show help
```

## Tabs

**Search** -- type to search the catalog. Unauthenticated queries use text search; logged-in users get semantic search.

**News** -- recent AI dev news from the last 7 days with titles, dates, and topics.

**Browse** -- browse entries by kind (Tools, Skills, MCPs) and category. Cycle kinds with left/right arrows, cycle categories with `[` and `]`.

## Keybindings

| Key | Action |
|-----|--------|
| Tab / Shift+Tab | Switch tabs forward/back |
| 1 / 2 / 3 | Jump to Search / News / Browse |
| Down / j | Move down |
| Up / k | Move up |
| Enter | Open detail popup |
| o | Open URL in browser |
| Esc | Close detail popup |
| q / Ctrl+C | Quit |
| Left/Right or h/l | Change kind (Browse tab) |
| [ / ] | Change category (Browse tab) |

## Categories

Agent Frameworks, TypeScript, Python, LLMs, Embeddings, Vector DBs, RAG, MCP Servers, CLIs, SDKs, UI, Testing, Observability, Deployment, Search, Code Gen, Data, Voice, Multimodal, Fine-Tuning, Other.

## Authentication

Run `un login` to start a device authorization flow. The CLI opens your browser to the Update Night authorization page and polls for approval. The token is saved to `~/.config/updatenight/config.json` (Linux/macOS) or `%APPDATA%\updatenight\config.json` (Windows).

Authenticated users get semantic search in addition to text search.

## Configuration

Set `UPDATENIGHT_API_URL` to point at a different API host. Defaults to `https://server.updatenight.com`.

## Related

- [Update Night MCP](https://github.com/amajorai/updatenight-mcp) — MCP server for AI assistants to search the catalog
- [Update Night Skill](https://github.com/amajorai/updatenight-skill) — Claude Code skill for browsing the catalog from any AI agent
