<p align="center">
  <img src="./assets/banner.svg" alt="Oxide — A modern IDE that runs in your terminal" width="100%"/>
</p>

<p align="center">
  <a href="https://oxideide.dev">Website</a> ·
  <a href="#architecture">Architecture</a> ·
  <a href="#roadmap">Roadmap</a> ·
  <a href="#contributing">Contributing</a>
</p>

<p align="center">
  <a href="https://github.com/galvaniclabs/oxide/blob/main/LICENSE"><img src="https://img.shields.io/badge/license-Apache%202.0-blue.svg" alt="License"></a>
  <a href="https://github.com/galvaniclabs/oxide"><img src="https://img.shields.io/github/stars/galvaniclabs/oxide?style=social" alt="GitHub Stars"></a>
</p>

---

Oxide is a terminal-native IDE built in Rust. It combines the speed and composability of the terminal with the UX of a modern editor — familiar keybindings, full mouse support, VSCode extension compatibility, and an MCP server that gives any LLM agent eyes and hands inside your IDE.

No modal editing. No config files. No bundled AI. It just works — and so does your agent.

## Why

Developers are increasingly living in the terminal — SSH sessions, Claude Code, OpenCode, tmux workflows. But every terminal editor either demands you learn modal editing or forces you to assemble 50 plugins before you can be productive.

Meanwhile, LLM coding agents can read and write your files, but they can't set a breakpoint, inspect a variable, view a diff, or run your tests. They're powerful but blind to the IDE.

Oxide fixes both problems:

- **For developers:** A real IDE in the terminal with the keybindings you already know
- **For LLM agents:** An MCP server on localhost that exposes every IDE primitive — editors, diagnostics, symbols, files — as tools any agent can call
- **For the workflow:** One window. Your editor on the left, your agent on the right. Shared state. The agent fixes code and you see the diff appear in real time

## Principles

**For the masses, not for terminal nerds.** Normal editing by default. `Ctrl+S` to save. `Ctrl+Z` to undo. Click to place your cursor. If you want Vim keybindings, they're available — but they're not the default.

**We don't own the AI — we make the IDE legible to any AI.** Oxide does NOT bundle an LLM. It runs an MCP server that any agent (Claude Code, OpenCode, Aider) connects to. The agent runs in its own process; Oxide gives it eyes and hands inside the IDE.

**Oxide is the single pane of glass.** Your editor, file tree, and agent terminal live in one window. `Ctrl+L` opens a real terminal pane running your agent of choice. The agent calls MCP tools, and you see the results render in the editor in real time. No alt-tabbing.

**Extensions are the ecosystem.** Oxide runs VSCode extensions via a compatible extension host. We don't rewrite language support for 50 languages — we run the extensions that already do it.

**Terminal is an implementation detail.** We say "IDE", not "terminal editor." The terminal is how we deliver speed and universal access. It's not the identity.

## Architecture

<p align="center">
  <img src="./assets/architecture.svg" alt="Oxide Architecture" width="100%"/>
</p>

**IDE as Server.** Oxide core is the source of truth for all IDE state. The TUI renders it for humans. LLM agents read and mutate it via MCP. Same workspace, same session, same truth — agent calls `ide.editor.goToLine(file, 42)` and the cursor moves in your editor in real time.

**MCP Server on localhost.** Oxide runs an MCP server on `localhost:4322` (HTTP + SSE) that starts automatically. Any MCP-compatible agent connects with zero config. No auth needed — it's localhost-only.

**Tmux under the hood.** Oxide uses tmux as invisible infrastructure for pane management. `Ctrl+L` splits a real terminal pane running your agent CLI. You never see tmux chrome — Oxide owns the UX.

**Editor core.** Custom-built in Rust. Tree-sitter for syntax highlighting, rope data structure for efficient editing, LSP for intellisense.

**Extension Host.** Separate Node.js process running a VSCode API compatibility shim. Extensions think they're running in VSCode. Communication via JSON-RPC over Unix domain sockets.

## Connect Your Agent

Oxide's MCP server starts automatically on `localhost:4322`. Point your agent at it:

**Claude Code** — add to your project's `.mcp.json`:
```json
{
  "mcpServers": {
    "oxide": {
      "url": "http://localhost:4322/mcp"
    }
  }
}
```

**Any MCP client** — connect to `http://localhost:4322/mcp` (HTTP + SSE transport).

**Phase 1 tools available:**
| Tool | Description |
|------|-------------|
| `ide.editor.getOpenFile()` | Get the currently focused file |
| `ide.editor.getOpenFiles()` | List all open files |
| `ide.editor.getContent(file)` | Read file content with optional line range |
| `ide.editor.goToLine(file, line)` | Move cursor, scroll viewport |
| `ide.editor.replaceRange(file, ...)` | Edit code (renders as highlighted change in editor) |
| `ide.diagnostics.list(file?)` | Get LSP errors and warnings |
| `ide.symbols.search(query)` | Search for symbols across the project |
| `ide.file.list(directory?)` | List files in the workspace |

## Default Keybindings

Oxide ships with familiar keybindings. No modes. No leader keys.

| Action | Keybinding |
|--------|-----------|
| Command Palette | `Ctrl+Shift+P` |
| Quick Open File | `Ctrl+P` |
| Save | `Ctrl+S` |
| Undo / Redo | `Ctrl+Z` / `Ctrl+Shift+Z` |
| Find in Project | `Ctrl+Shift+F` |
| Go to Definition | `F12` |
| Toggle Terminal | ``Ctrl+` `` |
| Start Debugging | `F5` |
| Multi-cursor | `Ctrl+D` |
| Toggle Agent Pane | `Ctrl+L` |

Optional keymaps: Vim, Helix, Emacs, Sublime, IntelliJ

## Roadmap

- [ ] **Phase 1 — Foundation:** Editor core, TUI shell, standard keybindings, mouse, file tree, tmux agent pane (`Ctrl+L`), syntax highlighting, LSP, MCP server (8 tools on localhost:4322), extension host
- [ ] **Phase 2 — IDE Power:** DAP debugging, diff viewer, git panel, test runner, full MCP tool schema, IDE-to-agent SSE events, inline diff accept/reject, embedded chat pane, multi-cursor
- [ ] **Phase 3 — LLM Deep Integration:** Agent-invocable debugger/diff/git/tests, inline suggestions, bidirectional context awareness
- [ ] **Phase 4 — Enterprise:** Client/server architecture, multi-user sessions, SSO, audit logging, air-gapped deployment

## Tech Stack

| Component | Technology |
|-----------|-----------|
| Core | Rust |
| TUI | ratatui + crossterm |
| Syntax | tree-sitter |
| Text Buffer | ropey (rope) |
| Async | tokio |
| MCP Server | HTTP + SSE via axum |
| LSP | tower-lsp / lsp-types |
| Git | gitoxide |
| Extension Host | Node.js (separate process) |
| Extension IPC | JSON-RPC over Unix domain sockets |
| Pane Management | tmux (invisible layer) |

## Contributing

Oxide is in early development. We're building in the open and welcome contributors.

If you're interested in helping, start by reading the [founding document](./FOUNDING_DOC.md) for full context on the vision, architecture, and technical decisions.

Areas where help is most valuable right now:
- **Editor core** — text buffer, cursor management, selection, undo/redo
- **TUI layout** — ratatui-based windowing, panels, command palette
- **MCP server** — axum-based HTTP+SSE server, tool handler implementation
- **Tmux integration** — programmatic session/pane management, agent pane lifecycle
- **Tree-sitter integration** — syntax highlighting, code folding
- **VSCode API research** — mapping which API surfaces the top extensions use

Open an issue or discussion to say hello before diving in.

## License

Apache 2.0 — see [LICENSE](./LICENSE)

---

<p align="center">
  <strong>Oxide</strong> is built by <a href="https://galvaniclabs.com">Galvanic Labs</a>
</p>
