# Oxide IDE — Founding Document

> **A modern IDE that runs in your terminal.**

**Domain:** oxideide.dev
**Language:** Rust
**License:** Apache 2.0 (open core — free for individuals, paid for enterprise features like SSO, audit logs, air-gapped deployment)
**Founded:** February 2026

---

## The Problem

Developers are being pulled into the terminal by LLM coding agents (Claude Code, OpenCode, Codex CLI) — but there's no IDE waiting for them when they get there.

Today's options force painful tradeoffs:

| Tool | Terminal Native | Modern UX | LLM Native | VSCode Extensions |
|------|:-:|:-:|:-:|:-:|
| VSCode / Cursor | ❌ | ✅ | ✅ | ✅ |
| Zed | ❌ | ✅ | ✅ | ❌ |
| Neovim | ✅ | ❌ (modal) | Plugin | ❌ |
| OpenCode | ✅ | Partial | ✅ | ❌ |
| Helix | ✅ | ❌ (modal) | ❌ | ❌ |
| **Oxide** | **✅** | **✅** | **✅** | **✅** |

Every terminal tool demands developers learn an alien interaction model (modal editing, cryptic keybindings, no mouse) before they can be productive. The terminal's real advantages — speed, composability, SSH-native, low resources, scriptability — are locked behind a gatekeeping UX.

**Oxide strips away the gatekeeping and keeps the advantages.**

---

## The Vision

Oxide is a full-featured IDE that runs in your terminal with:

- **Modern, familiar UX by default** — normal text editing (not modal), IntelliJ/VSCode keybindings, full mouse support, click-to-edit, Ctrl+C/V, Ctrl+Z. No modes. No wall of keybindings to memorize.
- **VSCode extension compatibility** — run the extensions you already use. Language support, debuggers, linters, formatters, themes — they just work.
- **LLM as a first-class IDE citizen** — not a chat sidebar. The IDE's own primitives (debugger, diff viewer, git, test runner) are exposed as tools that LLM agents can invoke and observe. The boundary between "human using IDE" and "LLM using IDE" disappears.
- **Zero config** — open a project, everything works. Language detection, syntax highlighting, git integration, LLM agent. No Lua files. No dotfile ritual.
- **Fast AF** — written in Rust. Starts in milliseconds. Handles million-line files. Renders at terminal refresh rate.

### Who Is This For?

- The 50M+ VSCode/JetBrains developers who've never considered the terminal because the UX was hostile
- Developers increasingly living in the terminal because of Claude Code / OpenCode but tired of switching back to GUI IDEs
- Enterprise teams who need SSH-native, auditable, air-gapped development environments
- Anyone who wants IDE power without IDE weight

### Who Is This NOT For (By Default)?

- Vim purists who want modal editing → available as optional keymap, not the default
- People who want a pure chat agent → use OpenCode/Claude Code directly, or use them *inside* Oxide

---

## Core Design Principles

### 1. For the Masses, Not for Terminal Nerds
Normal editing by default. Mouse works. Standard shortcuts. If your grandma can use Google Docs, the basic editing model of Oxide should feel familiar.

### 2. The LLM Is a Participant, Not a Sidebar
The IDE exposes its primitives (debugger, diff viewer, git state, test runner, file tree, LSP diagnostics) as tools that any LLM agent can invoke. The LLM doesn't just read your code — it can set breakpoints, inspect variables, stage git hunks, run tests, and observe the results.

### 3. Extensions Are the Ecosystem
We don't build language support for 50 languages. We run VSCode extensions that already do this. Our job is the platform, not the plugins.

### 4. Terminal Is an Implementation Detail
We never say "terminal editor." We say "IDE." The terminal is how we deliver speed, composability, and universal access. It's not the identity.

### 5. Open Core
Free for individuals. Paid for enterprise features (SSO, audit logs, air-gapped deployment, team collaboration).

### 6. We Don't Own the AI — We Make the IDE Legible to Any AI
Oxide does NOT bundle an LLM. It does NOT compete with Claude Code, OpenCode, or Aider. It exposes an MCP server that any agent connects to. The agent runs in its own process; Oxide gives it eyes and hands inside the IDE.

### 7. Oxide Is the Single Pane of Glass
The developer never leaves Oxide to interact with their AI agent. Agents run inside Oxide (via tmux-managed panes) and connect to the IDE via MCP on localhost. Editor, file tree, agent terminal — one window, shared state, no tmux tab switching, no alt-tabbing.

---

## Architecture

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

### Key Architectural Decisions

**Editor Core: Custom, built from scratch in Rust**
Not Neovim embed. We need total control over the default UX — normal insert-mode editing, standard keybindings, mouse-first interaction. Embedding Neovim brings too much modal baggage. The editor uses:
- Tree-sitter for syntax highlighting (Rust bindings)
- Rope data structure for efficient text manipulation
- LSP for intellisense, go-to-definition, diagnostics
- DAP for debugging

**TUI: ratatui + crossterm**
ratatui is the most mature Rust TUI framework. If we outgrow it, we contribute upstream. crossterm gives us cross-platform terminal handling.

**Extension Host: Separate Node.js process**
VSCode extensions expect a Node.js runtime. We run the extension host as a separate process communicating over JSON-RPC via Unix domain sockets (named pipes on Windows). This gives us:
- Process isolation (bad extension can't crash IDE)
- No GC pauses in the rendering thread
- Security boundary between extensions and core

**Two Shims:**
- **JS-side shim**: Implements the `vscode.*` API surface. Extensions import this thinking they're in VSCode. It serializes every call and sends it over IPC.
- **Rust-side shim**: Receives serialized calls and maps them to TUI primitives. `window.createTreeView()` → ratatui widget. `debug.startDebugging()` → DAP session.

**IDE as Server**
Oxide core is the source of truth for all IDE state. Two types of clients consume it:
1. **The TUI** — renders state for the human
2. **LLM agents** — read and mutate state via MCP

Both operate on the same underlying state. Agent calls `ide.editor.goToLine(file, 42)` → cursor moves in TUI in real time. Human clicks gutter to set breakpoint → agent sees it via `ide.debug.listBreakpoints()`. Same workspace, same session, same truth.

**IDE Toolbox Protocol: MCP server on localhost**
Oxide runs an MCP server on `localhost:4322` (HTTP + SSE transport). The server starts automatically — no opt-in, no auth (localhost-only). Claude Code users connect by adding to `.mcp.json`:
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

Phase 1 tool surface (minimum viable — 8 tools):
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

Full tool schema (Phase 2+):
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

This protocol is the moat. Even if someone builds a better TUI, if our protocol becomes the standard for how LLMs talk to IDEs, we win.

**Process Model: TWO+ processes**
1. **Main process (Rust):** TUI rendering, editor core, LSP/DAP management, git, MCP server (localhost:4322), keymap engine, config engine, all core logic.
2. **Extension host (Node.js):** Separate process. VSCode-compatible extensions. JSON-RPC over Unix domain sockets.
3. **LLM agents (external):** NOT managed by Oxide. Connect via MCP on localhost:4322. Agent disconnecting does NOT affect the IDE.

A misbehaving extension or disconnecting agent CANNOT crash the IDE. Process isolation at every boundary.

**Tmux as Infrastructure Layer**
Oxide uses tmux as its process and pane management layer. Tmux is a dependency — it ships with or is required by the Oxide distribution. The user never interacts with tmux directly. Oxide programmatically controls tmux sessions, windows, and panes. The user experiences one unified UI; tmux is invisible plumbing.

Why tmux:
- Battle-tested PTY handling — we don't build a terminal emulator
- Real pane management for embedding agent CLIs alongside the IDE
- Session persistence — agent keeps running if you detach/reattach
- Available everywhere on macOS and Linux. Windows requires WSL (acceptable for Phase 1)

Note: zellij (Rust-native) is an alternative to evaluate later, but tmux has ubiquitous install base and is the right choice for now.

**Agent Actions Render Natively in IDE**
When an agent performs an action via MCP, Oxide renders the action visually in the editor:
- `ide.editor.replaceRange()` → Editor highlights the changed region (Phase 2: inline diff with accept/reject)
- `ide.editor.goToLine()` → Cursor moves, viewport scrolls, line is briefly highlighted
- `ide.debug.setBreakpoint()` → Gutter breakpoint indicator appears in real time

The IDE is the visual feedback layer for agent actions. The agent's terminal shows reasoning; the IDE shows code-level effects.

**Bidirectional Communication**
- Agent → IDE (via MCP tools): Agent calls tools, IDE state changes, TUI renders the effect.
- IDE → Agent (via MCP notifications/events): SSE events for developer actions (file opened, cursor moved, diagnostic changed). Phase 2 — requires careful design around event noise.

---

## Two-Mode UX Model

Oxide has two primary modes the user can toggle between:

**Mode 1: IDE Only (default on launch)** — Standard IDE, full screen, no agent pane visible. The MCP server is still running — agents CAN connect and operate, but there's no visible agent UI. The developer is driving.

**Mode 2: Agent + IDE (toggle with `Ctrl+L`)** — The agent pane opens on the right as a real tmux pane running Claude Code, OpenCode, or any CLI agent. The editor is in the middle. File tree stays on the left.

Key behaviors:
- `Ctrl+L` toggles the agent pane open/closed
- The agent pane is a real terminal — the user can type in it, interact with the agent directly
- The agent pane auto-spawns the user's configured agent (e.g., `claude` CLI) or opens as a blank terminal
- Agent pane position defaults to right, configurable to left/bottom
- If an agent triggers a visual IDE action while in IDE-only mode, Oxide shows a non-intrusive notification but does NOT force-open the agent pane

---

## Extension Compatibility Strategy

### Tiered Support

**Tier 1 — Full support (Phase 1)**
Extensions using:
- Language Server Protocol (LSP)
- Debug Adapter Protocol (DAP)
- Tree view contributions
- Command palette contributions
- Status bar items
- File system watchers / Task providers

Coverage: ~60-70% of popular extensions

**Tier 2 — Adapted support (Phase 2)**
Extensions using webviews → mapped to TUI equivalents:
- Diff viewers → native TUI diff
- Markdown preview → terminal renderer
- Settings UI → TUI forms

**Tier 3 — Unsupported (explicit)**
Fundamentally visual extensions (live browser preview, Figma, etc.)

### Compatibility Validation
We will maintain a public compatibility matrix for the top 200 VSCode extensions with CI-tested status badges.

---

## Default Keybindings

Oxide ships with IntelliJ/VSCode-compatible keybindings. No modes. No leader keys.

| Action | Keybinding |
|--------|-----------|
| Command Palette | `Ctrl+Shift+P` |
| Quick Open File | `Ctrl+P` |
| Save | `Ctrl+S` |
| Undo / Redo | `Ctrl+Z` / `Ctrl+Shift+Z` |
| Find | `Ctrl+F` |
| Find in Project | `Ctrl+Shift+F` |
| Go to Definition | `F12` |
| Go to Line | `Ctrl+G` |
| Toggle Terminal | `` Ctrl+` `` |
| Start Debugging | `F5` |
| Toggle Breakpoint | `F9` |
| Multi-cursor | `Ctrl+D` (select next occurrence) |
| Comment Line | `Ctrl+/` |
| Move Line Up/Down | `Alt+↑` / `Alt+↓` |
| Toggle Agent Pane | `Ctrl+L` |
| Accept LLM Suggestion | `Tab` |

**Optional keymaps available:** Vim, Helix, Emacs, Sublime

---

## Phased Roadmap

### Phase 1: Foundation (Months 1-4)
**Goal:** Usable editor with LSP + basic extension support

- [ ] Custom editor core in Rust (tree-sitter, rope, basic editing)
- [ ] ratatui-based TUI shell (file tree, tabs, status bar, command palette)
- [ ] Standard keybindings (IntelliJ/VSCode defaults)
- [ ] Full mouse support (click, drag-select, scroll, context menu)
- [ ] Tmux session management (Oxide launches inside tmux, `Ctrl+L` splits agent pane)
- [ ] LSP integration (completions, diagnostics, go-to-definition)
- [ ] MCP server on localhost:4322 (expose 8 Phase 1 tools, ship `.mcp.json` snippet)
- [ ] Node.js extension host with Tier 1 API shim
- [ ] Git status in status bar

**Ship it.** Even at this stage, it's the only terminal IDE with modern UX + extension support + MCP server for any LLM agent.

### Phase 2: IDE Power (Months 5-8)
**Goal:** Full IDE feature parity

- [ ] DAP integration (breakpoints, variable inspection, call stack, watch)
- [ ] Native TUI diff viewer
- [ ] Full git panel (stage, commit, branch, merge, log)
- [ ] Test runner integration
- [ ] Full IDE Toolbox Protocol (expand from 8 to full tool schema)
- [ ] IDE→agent SSE events (file opened, cursor moved, diagnostic changed)
- [ ] Agent action inline diff with accept/reject controls
- [ ] Embedded LLM chat pane (convenience layer over MCP — optional alternative to tmux agent pane)
- [ ] Expand API shim: tree views, status bar items, task providers
- [ ] Multi-cursor editing
- [ ] Integrated terminal pane
- [ ] Top 100 VSCode extension compatibility testing

### Phase 3: LLM Deep Integration (Months 9-12)
**Goal:** LLM as true IDE participant

- [ ] LLM can invoke debugger, set breakpoints, inspect variables
- [ ] LLM can view and stage diffs
- [ ] LLM can run and observe tests
- [ ] LLM can navigate codebase via LSP (find references, call hierarchy)
- [ ] Agent-agnostic: works with Claude Code, OpenCode, Aider, any MCP-compatible agent
- [ ] Inline code suggestions (ghost text, tab-to-accept)
- [ ] LLM-assisted git commit messages

### Phase 4: Enterprise & Collaboration (Months 13-18)
**Goal:** Production-ready for teams

- [ ] Client/server architecture (run on dev server, control from laptop/mobile)
- [ ] Multi-user sessions (shared terminal, pair programming)
- [ ] SSO / SAML / OIDC authentication
- [ ] Audit logging (who did what, when)
- [ ] Air-gapped deployment (local LLM support)
- [ ] Team configuration sharing (rules files, prompt templates)
- [ ] Extension allowlist/blocklist for enterprise
- [ ] Tier 2 extension support (webview adapters)

---

## Technical Stack

| Component | Technology | Rationale |
|-----------|-----------|-----------|
| Core | Rust | Performance, memory safety, enterprise security story |
| TUI Framework | ratatui + crossterm | Most mature Rust TUI ecosystem |
| Syntax Highlighting | tree-sitter (Rust bindings) | Industry standard, AST-level accuracy |
| Text Buffer | Rope (ropey crate) | O(log n) edits on large files |
| Async Runtime | tokio | Battle-tested, required for concurrent LSP/DAP/IPC |
| Serialization | serde + serde_json | Extension host IPC (JSON-RPC) |
| Extension Host | Node.js (separate process) | VSCode extensions require Node.js |
| IPC | Unix domain sockets + JSON-RPC | What VSCode uses, proven at scale |
| LSP Client | tower-lsp / lsp-types | Typed, async, well-maintained |
| DAP Client | Custom on dap-types | Thinner crate ecosystem, will need some custom work |
| Git | gitoxide (pure Rust) | No libgit2 dependency, faster, safer |
| LLM Protocol | MCP (Model Context Protocol) | Standard for LLM tool use, Claude/OpenCode compatible |
| MCP Server | MCP over HTTP+SSE (custom impl) | Cross-platform, debuggable, MCP-native |
| HTTP Server | axum | Best Rust HTTP framework, tokio-native |
| Pane Management | tmux (invisible layer) | Battle-tested PTY handling, ubiquitous |

---

## Business Model

### Open Source (Free)
- Full IDE functionality
- All keymaps
- Extension support
- Single-user LLM integration
- Community support

### Oxide Pro ($15/user/month)
- Priority LLM model access / hosted agent
- Cloud sync (settings, keymaps, extensions)
- Advanced AI features (codebase-wide refactoring, test generation)
- Priority support

### Oxide Enterprise ($40/user/month)
- Everything in Pro
- SSO / SAML / OIDC
- Audit logging
- Air-gapped deployment
- Extension allowlist/blocklist
- Multi-user collaboration
- Dedicated support & SLA
- Custom LLM provider integration

---

## Competitive Moat

1. **VSCode extension compatibility in a terminal** — nobody else has this
2. **IDE Toolbox Protocol** — the standard for how LLMs interact with development environments
3. **Modern UX without modal editing** — captures the 90% of developers that terminal tools ignore
4. **Rust performance** — genuinely faster than Electron-based IDEs
5. **Founder's Neovim internals expertise** — deep understanding of terminal editor architecture at the systems level

---

## Success Metrics

| Milestone | Target | Timeframe |
|-----------|--------|-----------|
| GitHub stars | 1,000 | Month 1 |
| GitHub stars | 10,000 | Month 4 |
| Monthly active users | 5,000 | Month 6 |
| VSCode extensions tested | 200 | Month 8 |
| Enterprise pilot customers | 3 | Month 12 |
| Monthly active users | 50,000 | Month 18 |
| ARR | $1M | Month 24 |

---

## Open Questions

- [ ] Exact open-source license (MIT? Apache 2.0? BSL?)
- [ ] Should we fork/adapt Eclipse Theia's extension host compatibility layer or build from scratch?
- [ ] Name of the IDE Toolbox Protocol (needs its own identity for standardization)
- [ ] Seed funding strategy vs. bootstrapping
- [ ] Mono-repo or multi-repo?

---

*"The speed and composability of the terminal. The UX of a modern IDE. LLM as a first-class citizen. Your extensions just work."*

**Oxide IDE — A modern IDE that runs in your terminal.**
