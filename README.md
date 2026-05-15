# Update Night CLI

Terminal UI for browsing the Update Night catalog of AI dev tools, skills, and MCP servers.

## Install

```
cargo install --git https://github.com/amajorai/updatenight-cli
```

Or build from source:

```
git clone https://github.com/amajorai/updatenight-cli
cd updatenight-cli
cargo build --release
# binary at target/release/un (or un.exe on Windows)
```

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
| Left/Right or h/l | Change kind (Browse tab) |
| [ / ] | Change category (Browse tab) |

## Categories

Agent Frameworks, TypeScript, Python, LLMs, Embeddings, Vector DBs, RAG, MCP Servers, CLIs, SDKs, UI, Testing, Observability, Deployment, Search, Code Gen, Data, Voice, Multimodal, Fine-Tuning, Other.

## Authentication

Run `un login` to start a device authorization flow. The CLI opens your browser to the Update Night authorization page and polls for approval. The token is saved to `~/.config/updatenight/config.json` (Linux/macOS) or `%APPDATA%\updatenight\config.json` (Windows).

## Configuration

Set `UPDATENIGHT_API_URL` to point at a different API host. Defaults to `https://server.updatenight.com`.

## Related

- [Update Night MCP](https://github.com/amajorai/updatenight-mcp) -- MCP server for AI assistants to search the catalog
- [Update Night Skill](https://github.com/amajorai/updatenight-skill) -- Claude Code skill for browsing the catalog from any AI agent
