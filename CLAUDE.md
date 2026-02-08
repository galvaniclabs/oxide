# CLAUDE.md — Project Context for Oxide IDE

## What Is This Project?

Oxide is a modern, terminal-native IDE built in Rust. It is NOT a text editor, NOT a Vim/Neovim fork, NOT a chat agent. It is a full IDE that runs in the terminal with a modern UX.

**Repo:** github.com/galvaniclabs/oxide
**Company:** Galvanic Labs
**Product domain:** oxideide.dev
**License:** Apache 2.0

## Core Philosophy — Read This First

These are non-negotiable design decisions. Do not deviate from them:

1. **No modal editing by default.** Oxide uses normal insert-mode editing like VSCode, IntelliJ, Sublime. The user types and text appears. No normal/insert/visual modes. No `i` to enter insert mode. Vim/Helix/Emacs keymaps are available as OPTIONAL configurations, never the default.

2. **Mouse is first-class.** Click to place cursor. Click-drag to select. Scroll wheel works. Click file tree to open files. Click gutter to set breakpoints. Right-click for context menus. Modern terminals fully support mouse events — use them.

3. **Standard keybindings.** Default keybindings follow IntelliJ/VSCode conventions:
   - `Ctrl+S` save, `Ctrl+Z` undo, `Ctrl+Shift+Z` redo
   - `Ctrl+C/V/X` copy/paste/cut
   - `Ctrl+F` find, `Ctrl+Shift+F` find in project
   - `Ctrl+P` quick open file, `Ctrl+Shift+P` command palette
   - `Ctrl+G` go to line, `F12` go to definition
   - `F5` start debug, `F9` toggle breakpoint
   - `Ctrl+D` select next occurrence (multi-cursor)
   - `Ctrl+/` toggle comment
   - `Alt+↑/↓` move line up/down
   - `` Ctrl+` `` toggle terminal
   - `Ctrl+L` toggle agent pane
   - `Tab` accept LLM suggestion

4. **Zero config.** Opening a project directory should just work — language detection, syntax highlighting, git integration, LSP autostart. No config files required for basic usage.

5. **Built for the masses.** This is not for terminal power users. It's for the 50M+ developers who use VSCode/JetBrains and are being pulled into the terminal by LLM coding agents. Every UX decision should ask: "Would a VSCode user feel at home?"

6. **LLM agents are first-class IDE participants.** The IDE exposes its primitives (debugger, diff viewer, git, test runner, LSP diagnostics) as tools that LLM agents can invoke via the IDE Toolbox Protocol (MCP server on localhost). The LLM doesn't just read files — it can set breakpoints, inspect variables, stage hunks, run tests.

7. **We don't own the AI — we make the IDE legible to any AI.** Oxide does NOT bundle an LLM. It does NOT compete with Claude Code, OpenCode, or Aider. It exposes an MCP server that any agent connects to. The agent runs in its own process; Oxide gives it eyes and hands inside the IDE.

8. **Oxide is the single pane of glass.** The developer never leaves Oxide to interact with their AI agent. Agents run inside Oxide (via tmux-managed panes) and connect to the IDE via MCP on localhost. Editor, file tree, agent terminal — one window, shared state, no tmux tab switching, no alt-tabbing.

## Architecture Overview

```
┌─ tmux session (invisible to user) ──────────────────────┐
│                                                          │
│  ┌─ Oxide TUI (tmux pane 1) ─────────────────────────┐  │
│  │                                                    │  │
│  │  ┌─────────┬──────────────────────┐                │  │
│  │  │  File   │      Editor          │                │  │
│  │  │  Tree   │                      │                │  │
│  │  │         │  [diffs, highlights, │                │  │
│  │  │         │   breakpoints]       │                │  │
│  │  └─────────┴──────────────────────┘                │  │
│  │  Status Bar               MCP: localhost:4322 ●    │  │
│  └────────────────────────────────────────────────────┘  │
│                         ↕ MCP                            │
│  ┌─ Agent pane (tmux pane 2, toggle Ctrl+L) ──────────┐  │
│  │  $ claude                                          │  │
│  │  Claude> I'll fix that type error...               │  │
│  │  [calls ide.diagnostics.list()]                    │  │
│  │  [calls ide.editor.replaceRange()]                 │  │
│  └────────────────────────────────────────────────────┘  │
│                                                          │
└──────────────────────────────────────────────────────────┘

Underlying architecture:
┌─────────────────────────────────────────────┐
│             Oxide Core (Rust)                │
│   LSP · DAP · Tree-sitter · Git · Keymap    │
├──────────────┬──────────────────────────────┤
│  Extension   │   MCP Server                  │
│  Host Bridge │   localhost:4322              │
│  (Node.js)   │   ↕ tools    ↕ events (SSE)  │
└──────────────┴──────────────────────────────┘
```

### IDE as Server

Oxide core is the source of truth for all IDE state. Two types of clients consume it:

1. **The TUI** — renders state for the human
2. **LLM agents** — read and mutate state via MCP

Both operate on the same underlying state. Agent calls `ide.editor.goToLine(file, 42)` → cursor moves in TUI in real time. Human clicks gutter to set breakpoint → agent sees it via `ide.debug.listBreakpoints()`. Same workspace, same session, same truth.

### Process Model

Oxide runs as TWO+ processes:

1. **Main process (Rust):** TUI rendering, editor core, LSP/DAP management, git, MCP server (localhost:4322), keymap engine, config engine, all core logic.

2. **Extension host (Node.js):** Separate process. Runs VSCode-compatible extensions. Communicates with main process via JSON-RPC over Unix domain sockets (named pipes on Windows). Extensions import a JS-side shim that implements the `vscode.*` API and proxies calls over IPC to the Rust process.

3. **LLM agents (external):** NOT managed by Oxide. Claude Code, OpenCode, Aider, or any MCP client connects to the MCP server on localhost:4322. Agent disconnecting does NOT affect the IDE. Process isolation at every boundary.

A misbehaving extension or disconnecting agent CANNOT crash the IDE. This isolation is by design.

### Tmux as Infrastructure Layer

Oxide uses tmux as its process and pane management layer. Tmux is a **dependency** — it ships with or is required by the Oxide distribution. The user never interacts with tmux directly. Oxide programmatically controls tmux sessions, windows, and panes via `tmux` CLI commands. The user experiences one unified UI; tmux is invisible plumbing.

Why tmux:
- Battle-tested PTY handling — we don't build a terminal emulator
- Real pane management for embedding agent CLIs alongside the IDE
- Session persistence — agent keeps running if you detach/reattach
- Available everywhere on macOS and Linux. Windows requires WSL (acceptable for Phase 1)

Implementation rules:
- Oxide starts a tmux session with a custom config (no user `.tmux.conf` interference)
- Tmux prefix key is rebound or disabled — Oxide owns all keybindings
- The user never sees tmux chrome, status bar, or key hints
- All tmux interaction happens via Oxide's own commands (`Ctrl+L` to toggle agent pane, etc.)
- If tmux is not installed, Oxide runs in IDE-only mode and tells the user to install tmux for agent integration

Note: zellij (Rust-native) is an alternative to evaluate later, but tmux has ubiquitous install base and is the right choice for now.

### Editor Core

Custom-built in Rust. NOT embedding Neovim or any other editor.

- **Text buffer:** `ropey` crate (rope data structure for O(log n) edits)
- **Syntax highlighting:** tree-sitter with Rust bindings
- **Code intelligence:** LSP client (tower-lsp / lsp-types)
- **Debugging:** DAP client
- **Git:** gitoxide (pure Rust, no libgit2)

### Extension Host

The extension host is a separate Node.js process that provides VSCode extension compatibility.

**Two shims:**
- **JS-side shim:** A JavaScript module that extensions import as `vscode`. Implements the VSCode Extension API interface by serializing calls and sending them over IPC. Extensions don't know they're not in VSCode.
- **Rust-side shim:** Receives serialized API calls from the JS shim and maps them to Oxide's TUI primitives and core services.

**IPC protocol:** JSON-RPC 2.0 over Unix domain sockets. This is what VSCode itself uses internally. Messages look like:
```json
{
  "jsonrpc": "2.0",
  "id": 42,
  "method": "window/showQuickPick",
  "params": {
    "items": ["Option A", "Option B"],
    "options": { "placeHolder": "Select" }
  }
}
```

**Extension compatibility tiers:**
- Tier 1 (Phase 1): LSP, DAP, tree views, command palette, status bar, file watchers, task providers
- Tier 2 (Phase 2): Webview-based extensions adapted to TUI equivalents
- Tier 3 (unsupported): Fundamentally visual extensions (live preview, Figma, etc.)

### IDE Toolbox Protocol

Oxide runs an MCP server on `localhost:4322` (HTTP + SSE transport). This is how LLM agents interact with the IDE. The server starts automatically with Oxide — no opt-in, no auth (localhost-only).

Claude Code users connect by adding to `.mcp.json`:
```json
{
  "mcpServers": {
    "oxide": {
      "url": "http://localhost:4322/mcp"
    }
  }
}
```

Why HTTP on localhost instead of Unix sockets for agents:
- Cross-platform (Windows has no Unix sockets)
- Any HTTP client can connect (curl for debugging, browser for future web UI)
- SSE provides real-time event streaming (diagnostics changes, test results)
- MCP spec supports HTTP+SSE natively
- Unix sockets still used internally for extension host IPC (Rust↔Node.js)

**Phase 1 tool surface (minimum viable — 8 tools):**
```
ide.editor.getOpenFile()
ide.editor.getOpenFiles()
ide.editor.getContent(file, startLine?, endLine?)
ide.editor.goToLine(file, line)
ide.editor.replaceRange(file, startLine, endLine, newText)
ide.diagnostics.list(file?)
ide.symbols.search(query)
ide.file.list(directory?)
```

**Full tool schema (Phase 2+):**
```
ide.editor.getOpenFile()
ide.editor.getOpenFiles()
ide.editor.getContent(file, startLine?, endLine?)
ide.editor.goToLine(file, line)
ide.editor.getSelection()
ide.editor.replaceRange(file, startLine, endLine, newText)
ide.editor.insertText(text, position)
ide.diff.show(before, after)
ide.diff.stageHunk(file, hunkIndex)
ide.debug.setBreakpoint(file, line)
ide.debug.removeBreakpoint(id)
ide.debug.listBreakpoints()
ide.debug.inspectVariable(name)
ide.debug.stepOver()
ide.debug.stepInto()
ide.debug.continue()
ide.debug.getCallStack()
ide.test.run(file, testName?)
ide.test.getResults()
ide.git.status()
ide.git.stageFile(file)
ide.git.unstageFile(file)
ide.git.diff(file?)
ide.git.commit(message)
ide.terminal.run(command)
ide.search.find(query, scope)
ide.symbols.search(query)
ide.file.list(directory?)
ide.diagnostics.list(file?)
```

### Agent Actions Render Natively

When an agent performs an action via MCP, Oxide renders the action visually in the editor, the same way a human action would appear.

- `ide.editor.replaceRange()` → Editor highlights the changed region. (Phase 2: inline diff view with accept/reject controls.)
- `ide.editor.goToLine()` → Cursor moves, viewport scrolls, line is briefly highlighted.
- `ide.debug.setBreakpoint()` → Gutter breakpoint indicator appears in real time.
- `ide.diagnostics.list()` → No visual change (read-only), but if the agent then acts on diagnostics, the fix is shown as a diff.
- `ide.test.run()` → Test results panel opens or updates with results (Phase 2).
- `ide.git.stageFile()` → Git status in status bar updates, file tree marker changes (Phase 2).

The principle: **the IDE is the visual feedback layer for agent actions.** The agent operates through MCP. Oxide renders what the agent did. The developer sees everything happening in the editor — not in the agent's terminal output.

### Bidirectional Communication

**Agent → IDE (via MCP tools):** Agent calls tools, IDE state changes, TUI renders the effect.

**IDE → Agent (via MCP notifications/events):** When the developer acts in the IDE, the MCP server emits SSE events that the agent can subscribe to. The agent has real-time awareness of what the developer is doing.

Examples:
- Developer opens a file → agent receives `ide.event.fileOpened`
- Developer selects code and hits "send to agent" → selected code is sent to the agent's input
- Developer gets a new LSP diagnostic → agent receives `ide.event.diagnosticChanged`
- Developer places cursor on a function → agent receives `ide.event.cursorMoved`

Phase 1 scope: agent→IDE only (tool calls). IDE→agent events are Phase 2 — they require careful design around what events to emit (too noisy = agent gets confused, too sparse = agent lacks context).

## Two-Mode UX Model

Oxide has two primary modes the user can toggle between:

### Mode 1: IDE Only (default on launch)

```
┌──────────────────────────────────────────┐
│ Tabs                                     │
├────────┬─────────────────────────────────┤
│        │                                 │
│  File  │         Editor                  │
│  Tree  │                                 │
│        │                                 │
│        │                                 │
├────────┴─────────────────────────────────┤
│ Status Bar                               │
└──────────────────────────────────────────┘
```

Standard IDE. Full screen. No agent pane visible. The MCP server is still running — agents CAN connect and operate, but there's no visible agent UI. In this mode, the agent powers subtle augmentation: inline suggestions, completions, background diagnostics enrichment. The developer is driving.

### Mode 2: Agent + IDE (toggle with `Ctrl+L`)

```
┌──────────────────────────────────────────────────┐
│ Tabs                                             │
├───────────┬──────────────────────────┬───────────┤
│           │                          │           │
│   File    │       Editor             │   Agent   │
│   Tree    │                          │   Pane    │
│           │   [shows diffs,          │  (Claude  │
│           │    replacements,         │   Code /  │
│           │    breakpoints,          │  OpenCode │
│           │    test results          │   etc.)   │
│           │    in real time]         │           │
├───────────┴──────────────────────────┴───────────┤
│ Status Bar                                       │
└──────────────────────────────────────────────────┘
```

The agent pane is a real tmux pane running Claude Code, OpenCode, or any CLI agent.

Key behaviors:
- `Ctrl+L` toggles the agent pane open/closed
- The agent pane is a real terminal — the user can type in it, interact with the agent directly
- The agent pane auto-spawns the user's configured agent (e.g., `claude` CLI) or opens as a blank terminal
- Agent pane position defaults to right, configurable to left/bottom
- If an agent triggers a visual IDE action while in IDE-only mode, Oxide shows a non-intrusive notification but does NOT force-open the agent pane

## Tech Stack

| Component | Crate / Technology | Version |
|-----------|-------------------|---------|
| Language | Rust | stable, latest edition |
| TUI framework | ratatui | latest |
| Terminal backend | crossterm | latest |
| Text buffer | ropey | latest |
| Syntax highlighting | tree-sitter | latest Rust bindings |
| Async runtime | tokio | latest, full features |
| Serialization | serde + serde_json | latest |
| LSP types | lsp-types | latest |
| DAP | custom on dap-types or similar | — |
| Git | gitoxide (gix) | latest |
| Extension IPC | JSON-RPC 2.0 (custom impl) | — |
| LLM protocol | MCP-compatible | — |
| MCP server | MCP over HTTP+SSE (custom impl) | — |
| HTTP server | axum | latest |

## Workspace Structure

This is a Cargo workspace. Structure as follows:

```
oxide/
├── Cargo.toml                 # Workspace root
├── CLAUDE.md                  # This file
├── README.md                  # Public README
├── FOUNDING_DOC.md            # Full vision document
├── LICENSE                    # Apache 2.0
│
├── crates/
│   ├── oxide-core/            # Editor core: buffer, cursor, selection, undo/redo
│   │   ├── Cargo.toml
│   │   └── src/
│   │       ├── lib.rs
│   │       ├── buffer.rs      # Rope-based text buffer
│   │       ├── cursor.rs      # Cursor position, movement
│   │       ├── selection.rs   # Selection model (shift+arrow, click-drag, multi-cursor)
│   │       ├── edit.rs        # Edit operations, undo/redo stack
│   │       └── keymap.rs      # Keymap engine, configurable bindings
│   │
│   ├── oxide-tui/             # TUI rendering and layout
│   │   ├── Cargo.toml
│   │   └── src/
│   │       ├── lib.rs
│   │       ├── app.rs         # Main application state and event loop
│   │       ├── layout.rs      # Panel layout, splitting, resizing
│   │       ├── editor.rs      # Editor widget (renders buffer)
│   │       ├── file_tree.rs   # File tree sidebar
│   │       ├── tabs.rs        # Tab bar
│   │       ├── status_bar.rs  # Status bar
│   │       ├── command_palette.rs  # Ctrl+Shift+P command palette
│   │       ├── terminal.rs    # Integrated terminal pane
│   │       ├── search.rs      # Find / find-in-project UI
│   │       └── theme.rs       # Color theme system
│   │
│   ├── oxide-lsp/             # LSP client manager
│   │   ├── Cargo.toml
│   │   └── src/
│   │       ├── lib.rs
│   │       ├── client.rs      # LSP client implementation
│   │       ├── manager.rs     # Auto-detect language, start/stop servers
│   │       └── capabilities.rs
│   │
│   ├── oxide-dap/             # DAP client for debugging
│   │   ├── Cargo.toml
│   │   └── src/
│   │       ├── lib.rs
│   │       ├── client.rs
│   │       ├── session.rs
│   │       └── types.rs
│   │
│   ├── oxide-git/             # Git integration via gitoxide
│   │   ├── Cargo.toml
│   │   └── src/
│   │       ├── lib.rs
│   │       ├── status.rs
│   │       ├── diff.rs
│   │       ├── staging.rs
│   │       └── log.rs
│   │
│   ├── oxide-extensions/      # Extension host bridge (Rust side)
│   │   ├── Cargo.toml
│   │   └── src/
│   │       ├── lib.rs
│   │       ├── host.rs        # Spawn/manage Node.js extension host process
│   │       ├── ipc.rs         # JSON-RPC IPC transport
│   │       ├── shim.rs        # Rust-side API shim (maps vscode.* calls to TUI)
│   │       └── registry.rs    # Extension discovery and loading
│   │
│   ├── oxide-toolbox/         # IDE Toolbox Protocol (MCP server)
│   │   ├── Cargo.toml
│   │   └── src/
│   │       ├── lib.rs
│   │       ├── protocol.rs    # MCP-compatible tool definitions
│   │       ├── server.rs      # HTTP+SSE server on localhost:4322
│   │       ├── handlers.rs    # Tool invocation handlers
│   │       └── events.rs      # SSE event stream (diagnostics, state changes)
│   │
│   └── oxide-config/          # Configuration and settings
│       ├── Cargo.toml
│       └── src/
│           ├── lib.rs
│           ├── settings.rs    # User settings (theme, font, keymap)
│           └── detection.rs   # Project type / language auto-detection
│
├── src/
│   └── main.rs                # Binary entry point, wires everything together
│
├── extension-host/            # Node.js extension host (separate process)
│   ├── package.json
│   └── src/
│       ├── index.js           # Extension host entry point
│       ├── vscode-shim.js     # JS-side vscode.* API implementation
│       └── ipc.js             # JSON-RPC client (connects to Rust process)
│
└── tests/
    ├── integration/
    └── fixtures/
```

## Coding Standards

### Rust

- **Edition:** 2024 (or latest stable)
- **Formatting:** `cargo fmt` — always. No exceptions.
- **Linting:** `cargo clippy` — treat warnings as errors in CI.
- **Error handling:** Use `thiserror` for library crates, `anyhow` for the binary. No `.unwrap()` in production code except where a panic is truly impossible and justified with a comment.
- **Async:** Use `tokio` for all async work. The TUI event loop, LSP/DAP communication, extension host IPC, and LLM API calls are all async.
- **Testing:** Unit tests in each module. Integration tests in `tests/`. Use `insta` for snapshot testing of TUI rendering where appropriate.
- **Documentation:** Public APIs must have doc comments. Internal functions should have comments explaining *why*, not *what*.

### Naming

- Crate names: `oxide-*` (e.g., `oxide-core`, `oxide-tui`)
- Module names: snake_case
- Types: PascalCase
- Functions: snake_case
- Constants: SCREAMING_SNAKE_CASE
- Keep names short and clear. `buf` not `text_buffer_instance`.

### Architecture Rules

- **No global mutable state.** Pass state explicitly.
- **The TUI crate depends on core, never the reverse.** Core knows nothing about rendering.
- **Extension host communication is always async and never blocks the TUI thread.**
- **Every IDE primitive that exists in the TUI must also be exposed in the IDE Toolbox Protocol.** If a human can do it, an LLM agent should be able to do it programmatically.
- **Keybindings are data, not code.** All keybindings are defined in a keymap data structure that can be swapped at runtime. Never hardcode a shortcut in a widget.
- **MCP server is default-on.** Starts with Oxide, binds to localhost:4322. No opt-in friction.
- **Core is agent-agnostic.** No Claude-specific, no OpenCode-specific code in core. MCP protocol is the only interface.

## Build and Run

```bash
# Build
cargo build

# Run (opens current directory)
cargo run

# Run on a specific project
cargo run -- /path/to/project

# Run tests
cargo test

# Format
cargo fmt

# Lint
cargo clippy -- -D warnings
```

## Current Phase

**Phase 1 — Foundation** (we are here)

Priority order:
1. Cargo workspace skeleton with all crates compiling (even if mostly empty)
2. TUI shell: basic ratatui app with panel layout, status bar, tab bar
3. Editor core: rope buffer, cursor, basic editing (type, delete, select, copy/paste, undo/redo)
4. Mouse support: click to position cursor, drag to select, scroll
5. File tree: read directory, click to open file in editor
6. Tmux session management: Oxide launches inside a tmux session it controls. `Ctrl+L` splits a pane for the agent CLI. Oxide manages pane creation/destruction/resizing programmatically.
7. Syntax highlighting: tree-sitter integration
8. Command palette: Ctrl+Shift+P fuzzy finder
9. LSP integration: start language server, show completions and diagnostics
10. MCP server on localhost:4322: expose Phase 1 tool surface (8 tools). Ship `.mcp.json` snippet in README.
11. Extension host: spawn Node.js process, basic IPC, run a simple extension

## What NOT to Build (Yet)

- No DAP / debugging (Phase 2)
- No git panel (Phase 2)
- No diff viewer (Phase 2)
- No test runner (Phase 2)
- No embedded LLM chat pane (agents connect externally via MCP)
- No auth on MCP server (localhost-only, not needed)
- No IDE→agent events / SSE notifications (Phase 2)
- No multi-user / collaboration (Phase 4)
- No enterprise features (Phase 4)
- No settings UI (use config file for now)
- No extension marketplace
- No auto-update mechanism

## Key Technical Decisions Already Made

| Decision | Choice | Rationale |
|----------|--------|-----------|
| Language | Rust | Performance, safety, enterprise security |
| Editor approach | Custom from scratch | Need full control over non-modal UX |
| TUI framework | ratatui | Most mature Rust TUI; contribute upstream if we outgrow it |
| Text buffer | ropey (rope) | O(log n) edits, handles large files |
| Syntax | tree-sitter | Industry standard, AST-level accuracy |
| Extension compat | VSCode extensions | 50K+ extension ecosystem |
| Extension host | Separate Node.js process | Process isolation, extensions expect Node.js |
| IPC | JSON-RPC over Unix sockets | What VSCode uses, proven at scale |
| Git | gitoxide | Pure Rust, no C dependency |
| Async | tokio | Industry standard for Rust async |
| LLM protocol | MCP | Standard for tool use, agent-agnostic |
| Default keybindings | IntelliJ/VSCode style | Familiar to 50M+ developers |
| Modal editing | Optional, not default | Building for the masses |
| MCP transport | HTTP+SSE on localhost:4322 | Cross-platform, debuggable, MCP-native |
| HTTP server | axum | Best Rust HTTP framework, tokio-native |
| Tmux | Invisible infrastructure layer | Battle-tested PTY/pane management, ubiquitous |
| LLM integration | MCP server, no bundled LLM | Agent-agnostic, works with any MCP client |

## Founder Context

The founder (hlpr98) has deep systems-level experience with terminal editor architecture. During GSoC 2019, he worked on Neovim's internals — specifically converting Neovim from a multi-threaded architecture to a multi-process system, separating the TUI from the core process and making TUI an RPC client (PR #10071). He has intimate knowledge of editor process models, IPC design, terminal rendering, and msgpack RPC. Trust his architectural guidance on systems-level decisions.

## Questions? Ambiguity?

If you encounter ambiguity in a design decision, ask these questions in order:

1. **Would a VSCode user expect this behavior?** If yes, do that.
2. **Does this expose the feature to LLM agents via the Toolbox Protocol?** If not, plan for it.
3. **Is this the simplest approach that works?** If not, simplify.
4. **Is this in scope for Phase 1?** If not, skip it and leave a TODO.
