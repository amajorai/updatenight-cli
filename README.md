# updatenight-cli

Terminal UI for browsing the Update Night catalog of AI dev tools, skills, and MCP servers.

## Usage

```
un           # open the TUI
un login     # authenticate with your Update Night account
un logout    # remove stored credentials
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

## Categories

Agent Frameworks, TypeScript, Python, LLMs, Embeddings, Vector DBs, RAG, MCP Servers, CLIs, SDKs, UI, Testing, Observability, Deployment, Search, Code Gen, Data, Voice, Multimodal, Fine-Tuning, Other.

## Authentication

Run `un login` to start a device authorization flow. The CLI will open your browser to the Update Night authorization page and poll for approval. The token is saved to `~/.config/updatenight/config.json` (Linux/macOS) or `%APPDATA%\updatenight\config.json` (Windows).

## Building

```
cargo build --release
```

The binary will be at `target/release/un` (or `un.exe` on Windows).

## Configuration

Set `UPDATENIGHT_API_URL` to point at a different API host (defaults to `https://server.updatenight.com`).
