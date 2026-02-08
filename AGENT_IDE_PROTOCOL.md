# AGENT_IDE_PROTOCOL.md — Agent ↔ IDE Interaction Reference

## Purpose

This document is the single source of truth for how LLM coding agents interact with Oxide IDE. It defines the protocol, tools, events, behavioral contracts, and configuration for every supported agent. Both Oxide developers and agent users must treat this as the canonical reference.

**Audience:**
- Oxide contributors (implement against this spec)
- Agent developers (integrate against this spec)
- End users (configure their agents to work with Oxide)

---

## 1. Core Principle

Oxide is an IDE that agents operate, not an AI product.

Oxide does not bundle an LLM. It does not compete with Claude Code, OpenCode, Aider, or any coding agent. Oxide exposes its IDE primitives — editor, diagnostics, debugger, git, tests — via an MCP-compatible server. Any agent that speaks MCP can connect and operate the IDE alongside the developer.

The agent reasons and converses in its own process. Oxide gives the agent eyes (read IDE state) and hands (mutate IDE state) inside the developer's workspace. The developer sees every agent action rendered natively in the IDE — diffs, cursor movements, breakpoints, test results — not as text in a chat log.

---

## 2. Connection

### MCP Server

Oxide starts an MCP server automatically on launch.

| Property | Value |
|----------|-------|
| Transport | HTTP + SSE (MCP Streamable HTTP) |
| Default address | `localhost:4322` |
| Endpoint | `http://localhost:4322/mcp` |
| Auth | None (localhost-only binding) |
| Startup | Automatic with Oxide. No opt-in. |
| Lifecycle | Server runs as long as Oxide is running. Agent disconnect does not affect the IDE. |

### Agent Configuration

#### Claude Code

File: `.mcp.json` in project root

```json
{
  "mcpServers": {
    "oxide": {
      "url": "http://localhost:4322/mcp"
    }
  }
}
```

#### OpenCode

File: `opencode.json` in project root

```json
{
  "$schema": "https://opencode.ai/config.json",
  "mcp": {
    "oxide": {
      "type": "remote",
      "url": "http://localhost:4322/mcp",
      "enabled": true
    }
  }
}
```

#### Aider

Aider does not natively support MCP yet. When it does, the same `localhost:4322/mcp` endpoint applies. Until then, Aider operates via file system only and does not benefit from IDE integration.

#### Any MCP Client

Any tool that implements the MCP client spec can connect to `http://localhost:4322/mcp` and discover tools automatically via the standard `tools/list` method. No Oxide-specific SDK, plugin, or adapter required.

---

## 3. Tool Surface

### Naming Convention

All tools use the `ide.*` namespace. Tools are grouped by domain:

| Namespace | Domain |
|-----------|--------|
| `ide.editor.*` | File editing, navigation, buffer state |
| `ide.diagnostics.*` | LSP diagnostics (errors, warnings, hints) |
| `ide.symbols.*` | Symbol search, go-to-definition |
| `ide.file.*` | File system operations scoped to workspace |
| `ide.debug.*` | DAP debugger control |
| `ide.test.*` | Test runner |
| `ide.git.*` | Git operations |
| `ide.diff.*` | Diff viewer, hunk staging |
| `ide.terminal.*` | Terminal command execution |
| `ide.search.*` | Text search across workspace |

### Phase 1 Tools (Minimum Viable)

These 8 tools ship first. They are sufficient for an agent to meaningfully read and write code through the IDE.

#### `ide.editor.getOpenFile`

Returns the currently focused file path and cursor position.

```json
{
  "name": "ide.editor.getOpenFile",
  "description": "Get the file path, cursor position, and visible range of the currently focused editor tab.",
  "inputSchema": {},
  "outputSchema": {
    "file": "string — absolute path",
    "cursor": { "line": "number", "column": "number" },
    "visibleRange": { "startLine": "number", "endLine": "number" }
  }
}
```

**Why agents should use this:** Tells the agent what the developer is looking at right now. No built-in agent tool provides this context.

#### `ide.editor.getOpenFiles`

Returns all open tabs with their file paths and dirty (unsaved) status.

```json
{
  "name": "ide.editor.getOpenFiles",
  "description": "List all files currently open in editor tabs, including whether they have unsaved changes.",
  "inputSchema": {},
  "outputSchema": {
    "files": [
      {
        "file": "string — absolute path",
        "dirty": "boolean — true if unsaved changes exist"
      }
    ]
  }
}
```

**Why agents should use this:** Agents can understand the developer's working set — which files they're actively editing. No built-in agent tool provides this.

#### `ide.editor.getContent`

Reads file content from the live editor buffer, including unsaved changes.

```json
{
  "name": "ide.editor.getContent",
  "description": "Read content from the editor's live buffer. Unlike reading from disk, this includes unsaved changes. Use this instead of file read tools when the file is open in the IDE.",
  "inputSchema": {
    "file": "string — absolute path",
    "startLine": "number (optional) — 1-indexed, inclusive",
    "endLine": "number (optional) — 1-indexed, inclusive"
  },
  "outputSchema": {
    "content": "string",
    "totalLines": "number",
    "dirty": "boolean"
  }
}
```

**Why agents should prefer this over built-in `view`/`read_file`:** The developer may have unsaved edits. Disk reads miss those. This reads the live buffer — the truth the developer sees on screen.

#### `ide.editor.goToLine`

Moves the cursor and viewport to a specific file and line.

```json
{
  "name": "ide.editor.goToLine",
  "description": "Open a file (if not already open), move the cursor to the specified line, and scroll the viewport to show it. The line is briefly highlighted in the IDE so the developer sees where the agent navigated.",
  "inputSchema": {
    "file": "string — absolute path",
    "line": "number — 1-indexed"
  }
}
```

**Visual effect in IDE:** File opens (or tab activates), viewport scrolls, line is highlighted briefly. The developer sees the navigation happen in real time.

#### `ide.editor.replaceRange`

Replaces a range of lines in an open file.

```json
{
  "name": "ide.editor.replaceRange",
  "description": "Replace lines in an open file. Use this instead of str_replace or write_file — it shows the change as an inline diff in the IDE that the developer can see, review, and accept/reject. The edit is applied to the live buffer (not saved to disk until the developer saves).",
  "inputSchema": {
    "file": "string — absolute path",
    "startLine": "number — 1-indexed, inclusive",
    "endLine": "number — 1-indexed, inclusive",
    "newText": "string — replacement content (may be more or fewer lines)"
  },
  "outputSchema": {
    "applied": "boolean",
    "newRange": { "startLine": "number", "endLine": "number" }
  }
}
```

**Visual effect in IDE:**
- Phase 1: Edit is applied, changed lines are highlighted briefly.
- Phase 2+: Edit is shown as an inline diff with accept/reject controls. Developer must approve before the buffer is mutated.

**Why agents should prefer this over `str_replace`:** The developer sees the change in context, in the editor, with syntax highlighting. `str_replace` modifies the file on disk silently — the developer has to notice via file watcher reload.

#### `ide.diagnostics.list`

Returns LSP diagnostics for a file or the entire workspace.

```json
{
  "name": "ide.diagnostics.list",
  "description": "Get LSP diagnostics (errors, warnings, hints) for a specific file or all open files. Use this instead of running the compiler — it returns structured, real-time diagnostics from the language server.",
  "inputSchema": {
    "file": "string (optional) — absolute path. If omitted, returns diagnostics for all open files."
  },
  "outputSchema": {
    "diagnostics": [
      {
        "file": "string",
        "line": "number",
        "column": "number",
        "endLine": "number",
        "endColumn": "number",
        "severity": "error | warning | info | hint",
        "message": "string",
        "code": "string (optional)",
        "source": "string — e.g. 'rust-analyzer', 'typescript'"
      }
    ]
  }
}
```

**Why agents should prefer this over `cargo check`/`tsc`:** Instant, structured, already parsed. No shell invocation, no output parsing. Includes warnings and hints that compiler output may omit.

#### `ide.symbols.search`

Searches for symbols (functions, types, variables) across the workspace.

```json
{
  "name": "ide.symbols.search",
  "description": "Search for symbols (functions, structs, classes, variables, etc.) by name across the workspace. Uses the LSP workspace symbol search. More precise than grep for code navigation.",
  "inputSchema": {
    "query": "string — symbol name or partial match"
  },
  "outputSchema": {
    "symbols": [
      {
        "name": "string",
        "kind": "string — function, struct, class, variable, etc.",
        "file": "string — absolute path",
        "line": "number",
        "containerName": "string (optional) — parent scope"
      }
    ]
  }
}
```

**Why agents should prefer this over `grep`/`ripgrep`:** Semantic search, not text search. Finds `struct Buffer` even if grep would match comments and strings. Understands language structure.

#### `ide.file.list`

Lists files in the workspace, respecting .gitignore and project structure.

```json
{
  "name": "ide.file.list",
  "description": "List files and directories in the workspace. Respects .gitignore and IDE exclude patterns. Returns file metadata including git status.",
  "inputSchema": {
    "directory": "string (optional) — relative to workspace root. Defaults to root.",
    "recursive": "boolean (optional) — default false"
  },
  "outputSchema": {
    "entries": [
      {
        "name": "string",
        "path": "string — relative to workspace root",
        "type": "file | directory",
        "gitStatus": "string (optional) — modified, added, untracked, etc."
      }
    ]
  }
}
```

**Why agents should prefer this over `ls`/`find`:** Respects .gitignore, includes git status, scoped to workspace. No accidental traversal into node_modules or target/.

### Phase 2 Tools (Full IDE Surface)

These ship after Phase 1 core is stable.

#### Debug (`ide.debug.*`)

| Tool | Description |
|------|-------------|
| `ide.debug.setBreakpoint(file, line)` | Set a breakpoint. Gutter indicator appears in IDE. |
| `ide.debug.removeBreakpoint(id)` | Remove a breakpoint by ID. |
| `ide.debug.listBreakpoints()` | List all active breakpoints. |
| `ide.debug.launch(config?)` | Start a debug session. |
| `ide.debug.continue()` | Resume execution. |
| `ide.debug.stepOver()` | Step over current line. |
| `ide.debug.stepInto()` | Step into function call. |
| `ide.debug.stepOut()` | Step out of current function. |
| `ide.debug.inspectVariable(name)` | Read a variable's value at current breakpoint. |
| `ide.debug.getCallStack()` | Get the current call stack. |
| `ide.debug.evaluate(expression)` | Evaluate an expression in the current debug context. |

#### Test (`ide.test.*`)

| Tool | Description |
|------|-------------|
| `ide.test.discover(file?)` | List available tests in a file or workspace. |
| `ide.test.run(file?, testName?)` | Run a specific test or all tests. |
| `ide.test.getResults()` | Get results of the last test run. |

#### Git (`ide.git.*`)

| Tool | Description |
|------|-------------|
| `ide.git.status()` | Get working tree status. |
| `ide.git.diff(file?)` | Get diff for a file or all changes. |
| `ide.git.stageFile(file)` | Stage a file. |
| `ide.git.unstageFile(file)` | Unstage a file. |
| `ide.git.commit(message)` | Commit staged changes. |
| `ide.git.log(count?)` | Get recent commit history. |

#### Diff (`ide.diff.*`)

| Tool | Description |
|------|-------------|
| `ide.diff.show(file, before, after)` | Open a side-by-side diff view in the IDE. |
| `ide.diff.stageHunk(file, hunkIndex)` | Stage a specific hunk from a diff. |
| `ide.diff.discardHunk(file, hunkIndex)` | Discard a specific hunk. |

#### Terminal (`ide.terminal.*`)

| Tool | Description |
|------|-------------|
| `ide.terminal.run(command)` | Execute a command in the IDE's integrated terminal. Returns output. |

#### Search (`ide.search.*`)

| Tool | Description |
|------|-------------|
| `ide.search.find(query, scope?)` | Text search across workspace. Scope: file, directory, or workspace. |
| `ide.search.replace(query, replacement, scope?)` | Find and replace across workspace. Requires developer confirmation in IDE. |

---

## 4. IDE → Agent Events (Phase 2)

The MCP server can emit SSE events that agents subscribe to. This gives agents real-time awareness of what the developer is doing in the IDE.

**Transport:** SSE stream on the same MCP connection.

### Events

| Event | Payload | Description |
|-------|---------|-------------|
| `ide.event.fileOpened` | `{ file }` | Developer opened a file. |
| `ide.event.fileClosed` | `{ file }` | Developer closed a file. |
| `ide.event.fileSaved` | `{ file }` | Developer saved a file. |
| `ide.event.cursorMoved` | `{ file, line, column }` | Developer moved cursor. **Debounced** — emitted at most once per 500ms. |
| `ide.event.selectionChanged` | `{ file, startLine, endLine, text }` | Developer selected text. |
| `ide.event.diagnosticChanged` | `{ file, diagnostics[] }` | LSP diagnostics changed (new error, warning resolved, etc.). |
| `ide.event.breakpointChanged` | `{ action, file, line }` | Developer added/removed a breakpoint via gutter click. |
| `ide.event.testCompleted` | `{ file, testName, passed, duration }` | A test finished running. |
| `ide.event.gitStatusChanged` | `{ files[] }` | Working tree status changed. |

### Noise Control

Events must be carefully tuned. An agent receiving 100 `cursorMoved` events per second is useless and expensive (each event may trigger LLM reasoning tokens). Rules:

- `cursorMoved` is debounced to at most 1 event per 500ms.
- `diagnosticChanged` is debounced to at most 1 event per 2 seconds (LSP diagnostics can flicker during typing).
- Events are only emitted for files the agent has expressed interest in (via a subscription mechanism, TBD in Phase 2 design).
- Agents can opt out of specific event types.

---

## 5. Behavioral Contract: Agent Tool Preferences

When Oxide is running and an agent is connected via MCP, the agent **should prefer** Oxide's tools over its built-in equivalents. This preference must be communicated to the agent via instruction files (CLAUDE.md, AGENTS.md) because agents do not automatically prefer MCP tools over built-ins.

### Preference Table

| Action | Instead of (built-in) | Use (Oxide MCP) | Why |
|--------|----------------------|-----------------|-----|
| Read file content | `view`, `read_file`, `cat` | `ide.editor.getContent` | Reads live buffer including unsaved changes |
| Edit file | `str_replace`, `write_file` | `ide.editor.replaceRange` | Shows inline diff in IDE, developer sees change in real time |
| List files | `ls`, `find`, `list_files` | `ide.file.list` | Respects .gitignore, includes git status |
| Check errors | `cargo check`, `tsc`, `pylint` | `ide.diagnostics.list` | Instant, structured, no shell overhead, includes all severities |
| Find symbol | `grep`, `ripgrep` | `ide.symbols.search` | Semantic search, language-aware, no false matches |
| Navigate to code | N/A | `ide.editor.goToLine` | Developer sees navigation in IDE, cursor moves visually |
| Set breakpoint | "add a breakpoint on line 42" (text instruction) | `ide.debug.setBreakpoint` | Actually sets it. Gutter updates. Developer doesn't have to do anything. |
| Run tests | `cargo test`, `pytest` via shell | `ide.test.run` | Structured results, IDE test panel updates |
| Stage file | `git add` via shell | `ide.git.stageFile` | IDE git panel updates in real time |

### When to fall back to built-ins

Agents should still use built-in tools when:
- Oxide is not running (MCP connection fails)
- The operation is not covered by an Oxide tool (e.g., complex shell scripting)
- The file is not open in the IDE (e.g., generating a new file — use built-in `write_file`, then `ide.editor.goToLine` to open it)

---

## 6. Instruction Files Oxide Ships

When the user runs Oxide for the first time in a project (or via an `oxide init` command), Oxide can generate these files. These are **not auto-generated without user consent** — Oxide prompts: "Generate agent config files for this project? (y/n)"

### CLAUDE.md section (appended, not overwritten)

```markdown
## Oxide IDE Integration

This project uses Oxide IDE. When Oxide is running, an MCP server is available at localhost:4322.

### Tool Preferences

When the Oxide MCP server is connected, prefer these tools over built-in equivalents:

- **Read files:** Use `ide.editor.getContent` instead of `view` — reads the live editor buffer, including unsaved changes.
- **Edit files:** Use `ide.editor.replaceRange` instead of `str_replace` — the developer sees your edit as an inline diff in the IDE in real time.
- **Check errors:** Use `ide.diagnostics.list` instead of running the compiler — returns structured LSP diagnostics instantly.
- **Find symbols:** Use `ide.symbols.search` instead of `grep` — semantic, language-aware symbol search.
- **List files:** Use `ide.file.list` instead of `ls` — respects .gitignore, includes git status.
- **Navigate:** Use `ide.editor.goToLine` to move the developer's cursor to relevant code — they see it happen in the IDE.

### Why

The developer is looking at the IDE. When you use Oxide tools, your actions are visible in the editor — diffs appear inline, the cursor moves, diagnostics update. When you use built-in tools, the developer sees nothing until they check the file. Oxide tools make your work visible and reviewable.

### Fallback

If the Oxide MCP server is not connected, fall back to your built-in tools normally.
```

### AGENTS.md section (appended, not overwritten)

```markdown
## Oxide IDE Integration

This project uses Oxide IDE with an MCP server on localhost:4322.

When the Oxide MCP server is connected, prefer its tools over built-in equivalents:

- Use `ide.editor.getContent` instead of reading files from disk (captures unsaved changes).
- Use `ide.editor.replaceRange` instead of `edit` tool (shows inline diff in the IDE).
- Use `ide.diagnostics.list` instead of running the compiler (instant, structured LSP diagnostics).
- Use `ide.symbols.search` instead of `grep` (semantic symbol search).
- Use `ide.file.list` instead of `ls` (respects .gitignore, includes git status).
- Use `ide.editor.goToLine` to navigate the developer's cursor to relevant code.

Your actions through Oxide tools are visible to the developer in real time. Use them so the developer can see and review your work as it happens.

If the Oxide MCP server is unavailable, fall back to built-in tools.
```

### .mcp.json (Claude Code)

```json
{
  "mcpServers": {
    "oxide": {
      "url": "http://localhost:4322/mcp"
    }
  }
}
```

### opencode.json (OpenCode — merged into existing if present)

```json
{
  "$schema": "https://opencode.ai/config.json",
  "mcp": {
    "oxide": {
      "type": "remote",
      "url": "http://localhost:4322/mcp",
      "enabled": true
    }
  }
}
```

---

## 7. UX Modes and Agent Visibility

### Mode 1: IDE Only (default)

```
┌──────────────────────────────────────────┐
│ Tabs                                     │
├────────┬─────────────────────────────────┤
│  File  │         Editor                  │
│  Tree  │                                 │
├────────┴─────────────────────────────────┤
│ Status Bar              MCP: ● connected │
└──────────────────────────────────────────┘
```

- MCP server is running. Agents can connect and operate.
- No agent pane visible. Agent actions (edits, navigation) render in the editor silently.
- Status bar shows MCP connection status: `● connected` (green) / `○ no agent` (grey).
- If an agent performs an action, the editor shows it (highlighted edit, cursor jump) but does not force-open the agent pane.

### Mode 2: Agent + IDE (toggle `Ctrl+L`)

```
┌──────────────────────────────────────────────────┐
│ Tabs                                             │
├───────────┬──────────────────────────┬───────────┤
│   File    │       Editor             │   Agent   │
│   Tree    │                          │   Pane    │
│           │   [inline diffs,         │  (tmux)   │
│           │    highlights,           │           │
│           │    breakpoints]          │           │
├───────────┴──────────────────────────┴───────────┤
│ Status Bar              MCP: ● connected         │
└──────────────────────────────────────────────────┘
```

- Agent pane is a tmux-managed terminal pane running the user's chosen agent CLI.
- `Ctrl+L` toggles the agent pane open/closed.
- Agent pane position defaults to right. Configurable: left, right, bottom.
- Agent actions render in the editor in real time. The agent pane shows the agent's reasoning/conversation. The editor shows the code-level effects.
- File tree stays on the left (standard convention).

### What the developer sees when the agent acts

| Agent action | IDE visual effect |
|--------------|-------------------|
| `ide.editor.replaceRange` | Changed lines highlighted (Phase 1). Inline diff with accept/reject (Phase 2+). |
| `ide.editor.goToLine` | Cursor moves, viewport scrolls, line briefly highlighted. |
| `ide.debug.setBreakpoint` | Red dot appears in gutter. |
| `ide.debug.removeBreakpoint` | Red dot removed from gutter. |
| `ide.diagnostics.list` | No visual change (read-only). |
| `ide.test.run` | Test panel opens/updates with results. |
| `ide.git.stageFile` | File tree git status marker updates. |
| `ide.diff.show` | Diff view opens in editor area. |

---

## 8. Security and Trust Model

### Localhost Only

The MCP server binds to `127.0.0.1` only. It is not accessible from the network. No authentication is required.

### Agent Trust

The agent has full read/write access to the workspace via MCP tools — the same access the developer has. This is intentional. The agent is a tool the developer chose to run. Oxide does not second-guess the developer's choice of agent.

However:

- **Destructive write operations** (replaceRange, git commit, file delete) should have visual confirmation in the IDE in Phase 2+. The agent proposes, the developer approves.
- **Read operations** (getContent, diagnostics, symbols) are always allowed without confirmation.
- **Debug operations** (setBreakpoint, stepOver) are always allowed — they're non-destructive.
- **Terminal execution** (`ide.terminal.run`) inherits the developer's shell permissions. Oxide does not sandbox this.

### Future: Workspace Scoping

Phase 3+: Agents can be scoped to specific directories or files. An agent working on the frontend can be restricted from reading backend code. Not in Phase 1 or 2.

---

## 9. Error Handling

### MCP Server Unavailable

If Oxide is not running or the MCP server fails to start:
- Agents fall back to their built-in tools automatically (file read/write, shell).
- No breakage. The agent workflow degrades gracefully.

### Tool Invocation Errors

All tools return structured errors:

```json
{
  "error": {
    "code": "string",
    "message": "string"
  }
}
```

| Error code | Meaning |
|------------|---------|
| `FILE_NOT_FOUND` | The specified file does not exist in the workspace. |
| `FILE_NOT_OPEN` | The file is not open in an editor tab (for buffer-dependent operations). |
| `RANGE_INVALID` | The specified line range is out of bounds. |
| `LSP_NOT_READY` | The language server has not initialized yet. Retry after a short delay. |
| `DEBUG_NOT_ACTIVE` | No active debug session. |
| `NO_WORKSPACE` | No workspace/project is open. |

Agents should handle these gracefully — retry, fall back to built-in tools, or inform the developer.

---

## 10. Versioning

The IDE Toolbox Protocol is versioned. The MCP server reports its version in the `initialize` response.

| Version | Phase | Tools |
|---------|-------|-------|
| `0.1.0` | Phase 1 | 8 tools: editor.getOpenFile, getOpenFiles, getContent, goToLine, replaceRange, diagnostics.list, symbols.search, file.list |
| `0.2.0` | Phase 2 | + debug.*, test.*, git.*, diff.*, terminal.*, search.* |
| `0.3.0` | Phase 2 | + IDE → Agent events (SSE) |
| `1.0.0` | Phase 3 | Stable API. Breaking changes require major version bump. |

Agents should check the version and gracefully handle missing tools in older versions.
