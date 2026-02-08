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

---

## Architecture

```
┌─────────────────────────────────────────────────────────┐
│                     TUI Frontend                        │
│              (Rust — ratatui + crossterm)               │
│                                                         │
│  ┌──────────┐ ┌──────────┐ ┌───────────┐ ┌───────────┐  │
│  │  Editor  │ │ Terminal │ │ LLM Agent │ │   Diff    │  │
│  │  Buffer  │ │  Panes   │ │   Panel   │ │  Viewer   │  │
│  └──────────┘ └──────────┘ └───────────┘ └───────────┘  │
│  ┌──────────┐ ┌──────────┐ ┌───────────┐ ┌───────────┐  │
│  │  File    │ │  Debug   │ │ Git Panel │ │  Search   │  │
│  │  Tree    │ │  Panel   │ │           │ │  Results  │  │
│  └──────────┘ └──────────┘ └───────────┘ └───────────┘  │
│  ┌───────────────────────────────────────────────────┐  │
│  │    Tab Bar  │  Status Bar  │  Command Palette     │  │
│  └───────────────────────────────────────────────────┘  │
├─────────────────────────────────────────────────────────┤
│                 IDE Toolbox Protocol                    │
│            (MCP-compatible tool interface)              │
│                                                         │
│  ide.editor.*  ide.debug.*  ide.diff.*  ide.git.*       │
│  ide.test.*    ide.terminal.*  ide.search.*             │
├─────────────────────────────────────────────────────────┤
│                   Core Services                         │
│              (Rust — async via tokio)                   │
│                                                         │
│  ┌─────────┐ ┌─────────┐ ┌────────┐ ┌───────────────┐   │
│  │   LSP   │ │   DAP   │ │  Git   │ │  File System  │   │
│  │ Manager │ │ Manager │ │ Engine │ │    Watcher    │   │
│  └─────────┘ └─────────┘ └────────┘ └───────────────┘   │
│  ┌─────────┐ ┌─────────┐ ┌────────┐ ┌───────────────┐   │
│  │Tree-sit │ │  Test   │ │ Config │ │   Keymap      │   │
│  │  ter    │ │ Runner  │ │ Engine │ │   Engine      │   │
│  └─────────┘ └─────────┘ └────────┘ └───────────────┘   │
├─────────────────────────────────────────────────────────┤
│              Extension Host Bridge                      │
│            (Rust ←JSON-RPC→ Node.js)                    │
│                                                         │
│  ┌─────────────────────┐   ┌──────────────────────────┐ │
│  │  Rust-side Shim     │   │  Node.js Extension Host  │ │
│  │                     │   │                          │ │
│  │  vscode.window →    │◄─►│  JS-side Shim            │ │
│  │    TUI widgets      │   │  (implements vscode.*)   │ │
│  │  vscode.debug →     │   │                          │ │
│  │    DAP bridge       │   │  ┌────┐ ┌────┐ ┌────┐    │ │
│  │  vscode.languages → │   │  │Ext │ │Ext │ │Ext │    │ │
│  │    LSP bridge       │   │  │ 1  │ │ 2  │ │ N  │    │ │
│  │                     │   │  └────┘ └────┘ └────┘    │ │
│  └─────────────────────┘   └──────────────────────────┘ │
└─────────────────────────────────────────────────────────┘
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

**IDE Toolbox Protocol: MCP-compatible**
Every IDE primitive is exposed as a tool that any LLM agent can invoke:
```
ide.editor.getOpenFile()
ide.editor.goToLine(file, line)
ide.editor.getSelection()
ide.diff.show(before, after)
ide.diff.stageHunk(file, hunkIndex)
ide.debug.setBreakpoint(file, line)
ide.debug.inspectVariable(name)
ide.debug.stepOver()
ide.debug.getCallStack()
ide.test.run(file, testName)
ide.test.getResults()
ide.git.status()
ide.git.stageFile(file)
ide.git.diff(file)
ide.terminal.run(command)
ide.search.find(query, scope)
ide.diagnostics.list(file)
```
This protocol is the moat. Even if someone builds a better TUI, if our protocol becomes the standard for how LLMs talk to IDEs, we win.

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
| Open LLM Agent | `Ctrl+L` |
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
- [ ] LSP integration (completions, diagnostics, go-to-definition)
- [ ] Node.js extension host with Tier 1 API shim
- [ ] Basic LLM agent pane (text chat with file context)
- [ ] Git status in status bar

**Ship it.** Even at this stage, it's the only terminal IDE with modern UX + extension support.

### Phase 2: IDE Power (Months 5-8)
**Goal:** Full IDE feature parity

- [ ] DAP integration (breakpoints, variable inspection, call stack, watch)
- [ ] Native TUI diff viewer
- [ ] Full git panel (stage, commit, branch, merge, log)
- [ ] Test runner integration
- [ ] IDE Toolbox Protocol v1 (expose all primitives to LLM agents)
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
