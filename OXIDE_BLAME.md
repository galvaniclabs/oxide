# OXIDE_BLAME.md — Provenance Tracking & Blame Architecture

## Overview

Oxide blame answers: **who wrote this line, how, and why?** — at sub-commit granularity.

Where `git blame` tells you which commit introduced a line, `oxide blame` tells you:
- **Who**: human or agent (and which agent)
- **How**: keystroke, MCP tool call, or terminal command
- **Why**: in response to a diagnostic, a prompt, a test failure, or unprompted

This is a Phase 1 deliverable. It proves "Oxide as the language of IDE for agents" and gives enterprises the audit trail to scale agents with confidence.

## How git blame works (our design influence)

Understanding git blame's performance characteristics is essential — oxide blame must be equally fast.

### The algorithm: blame passing

Git blame works **backward**, not forward:

1. **Start**: All lines in the current file are unblamed — assigned to a virtual "working tree" suspect
2. **Walk**: For each commit in reverse topological order, diff the file against its parent
3. **Pass blame**: Lines **identical** between child and parent get re-attributed (passed) to the parent. Lines that **differ** stay attributed to the child — that's where they were introduced
4. **Terminate early**: Once every line has a final attribution (no more suspects), stop

The key data structure is the **blame_entry** — a contiguous range of lines with the same attribution:

```
struct blame_entry {
    commit,        // the suspect/final commit
    orig_line,     // line number in the suspect's version
    final_line,    // line number in the current file
    num_lines,     // contiguous run length
    next,          // linked list
}
```

The **scoreboard** holds all entries — a linked list of blame ranges covering every line.

### Why git blame is fast

1. **History simplification (TREESAME)**: Only visits commits where the file's blob OID actually changed. The commit-graph file accelerates this with precomputed generation numbers.

2. **Shrinking work set**: Each iteration, lines get finalized. The suspect set monotonically decreases. Once empty, the walk terminates. A file where the top half is old and the bottom half is new — top half finalizes from recent commits, only the bottom needs deep history.

3. **Diff, not reconstruction**: Never reconstructs the full file at each revision. Computes diffs between adjacent versions. Cost is O(n*d) where d is edit distance — and between adjacent commits, d is usually tiny.

4. **Pack file delta chains**: Historical blobs are stored as deltas. Reading the file at commit N-1 when you have commit N is cheap — just applying a delta.

5. **Contiguous ranges, not per-line**: A 500-line file might have ~20 blame entries. Operations work on ranges.

### Complexity

- Walk: O(C) commits where C = commits touching this file (not total repo commits)
- Per commit: O(d) diff where d = edit distance (not file size)
- Total: O(C * d_avg) — fast because d_avg is small and C << total commits

## Oxide blame: what's different

| | Git blame | Oxide blame |
|---|---|---|
| Granularity | Per commit | Per action (keystroke, MCP call, terminal cmd) |
| Attribution | Commit SHA | (actor, method, trigger, timestamp) |
| When computed | Post-hoc, on demand | Recorded live, queried on demand |
| Data source | Git history | Session journal + git history |

Git can work backward because commits are immutable snapshots with diffs. Oxide blame needs a **forward-recording, backward-queryable** design.

## Architecture: five layers

```
┌─────────────────────────────────────────────────────────┐
│  Layer 5: Query (oxide blame <file>)                    │
│  Walk commits, load trace files, merge with git blame   │
├─────────────────────────────────────────────────────────┤
│  Layer 4: Commit Crystallization                        │
│  .oxide/traces/<sha>.jsonl — delta-only, checked in git │
├─────────────────────────────────────────────────────────┤
│  Layer 3: Incremental Maintenance                       │
│  Lazy offset tree — O(log n) per edit                   │
├─────────────────────────────────────────────────────────┤
│  Layer 2: Line Attribution Index (blame index)          │
│  Contiguous ranges — same insight as git blame_entry    │
├─────────────────────────────────────────────────────────┤
│  Layer 1: Session Journal (append-only)                 │
│  Every mutation recorded as it happens — O(1) per edit  │
└─────────────────────────────────────────────────────────┘
```

### Layer 1: Session Journal (forward recording)

During editing, every mutation is recorded as it happens:

```rust
struct JournalEntry {
    seq: u64,                    // monotonic sequence number
    timestamp: Instant,
    actor: Actor,                // Human | Agent(agent_id)
    method: Method,              // Keystroke | McpCall(tool) | Terminal(cmd)
    trigger: Option<Trigger>,    // Diagnostic | Prompt(text) | TestFailure
    edit: EditOp,                // (file, range, old_text, new_text)
}

enum Actor {
    Human,
    Agent { id: String },        // e.g., "claude", "opencode"
}

enum Method {
    Keystroke,
    McpCall { tool: String },    // e.g., "editor.replaceRange"
    Terminal { command: String }, // observed from agent's tmux pane
}

enum Trigger {
    Diagnostic { code: String, message: String },
    Prompt { text: String },
    TestFailure { test: String },
    Unprompted,
}
```

**Append-only**. No indexing overhead during editing. Push to a `Vec<JournalEntry>` (or ring buffer if memory-bounded).

**Performance constraint**: Recording must be **O(1) amortized** per edit. A keystroke adds ~100 bytes. No blocking, no I/O on the hot path.

**Three data streams feed the journal:**
1. MCP tool calls (agent → IDE): directly captured by the MCP server handlers
2. Terminal activity (agent's tmux pane): observed via `tmux pipe-pane` / `capture-pane`
3. Human IDE actions (keystrokes/clicks): captured by the keymap engine

### Layer 2: Line Attribution Index (the blame structure)

The in-memory data structure for per-line attribution — equivalent to git's blame scoreboard, but maintained incrementally.

```rust
struct LineAttribution {
    actor: Actor,
    method: Method,
    trigger: Option<Trigger>,
    journal_seq: u64,           // points back to journal entry
    timestamp: Instant,
}

struct BlameRange {
    start_line: u32,            // inclusive
    end_line: u32,              // exclusive
    attribution: LineAttribution,
}

struct BlameIndex {
    // Per-file, maps line ranges to attributions
    files: HashMap<PathBuf, Vec<BlameRange>>,
}
```

Key insight borrowed from git: **contiguous ranges, not per-line storage**. When a human types 15 lines in a row, that's one `BlameRange`. When an agent does `replaceRange` for 50 lines, one `BlameRange`.

### Layer 3: Incremental Maintenance

When an edit happens, the blame index is updated:

```
Edit: file.rs, lines 10-15 replaced with 8 new lines

Before:
  [1-9:   human/keystroke]
  [10-15: agent/mcp]        ← being replaced
  [16-50: human/keystroke]

After:
  [1-9:   human/keystroke]   ← unchanged
  [10-17: <new attribution>] ← 8 new lines, new entry
  [18-52: human/keystroke]   ← shifted by (8-6)=+2, same attribution
```

Operations per edit:
1. **Split** any range partially overlapping the edit region — O(1), at most 2 ranges split
2. **Remove** ranges fully inside the edit region — O(k) where k = ranges in edit region
3. **Insert** new range for the new text — O(1)
4. **Shift** all ranges after the edit by the line delta — O(m) where m = ranges after edit

The O(m) shift is the bottleneck. Solution: **lazy offset tree**.

#### Lazy offset tree (recommended approach)

Use an augmented B-tree where each node stores a **relative offset** rather than an absolute line number. Shifts become O(log n) — update one node, offsets propagate lazily on traversal.

This is the same approach used by:
- VSCode's piece table markers
- CodeMirror's range sets
- ropey's internal rope structure

Because we already use `ropey` for the text buffer, we can **colocate blame metadata on the rope** — each leaf chunk carries its blame attribution. Edits to the rope naturally maintain the blame index. No separate data structure to synchronize.

Alternative: interval tree with lazy propagation — O(log n) per edit, O(log n + k) query. More complex, less natural fit with the rope.

### Layer 4: Commit Crystallization

When the user commits, the blame index for modified files is **crystallized** into a trace file:

```
.oxide/traces/<commit-sha>.jsonl
```

Each line is a blame range for lines **changed in that commit** (delta-only):

```json
{"file":"src/main.rs","start":10,"end":17,"actor":"agent:claude","method":"mcp:editor.replaceRange","trigger":"diagnostic:E0308","seq":4523,"ts":1708617600}
{"file":"src/main.rs","start":42,"end":45,"actor":"human","method":"keystroke","seq":4531,"ts":1708617720}
```

**Only the diff is stored** — lines unchanged from the parent commit don't need entries. They inherit from the parent commit's trace, exactly like git blame inherits from parent commits.

This is the equivalent of git's "finalize blame entries" — once committed, attribution is immutable.

**Storage estimate**: ~100 bytes per blame range. A commit touching 200 lines across 5 files with 30 blame ranges ≈ 3KB. A year of daily commits ≈ 1MB. Negligible.

#### Sidecar storage

Full session history (including reverts, failures, exploratory edits) is persisted to `~/.oxide/history/` as a local sidecar. This is NOT checked into git — it's the developer's private edit history. Trace files are the curated, commit-aligned subset.

### Layer 5: Querying (oxide blame)

`oxide blame <file>` reconstructs full per-line attribution:

1. Start with current file content, all lines unblamed
2. Load trace file for HEAD commit — attribute lines changed in that commit
3. Walk parent commits, loading their trace files — attribute lines changed in each
4. For any line reaching a commit with no trace file (pre-Oxide history), fall back to git blame: `(human, git-commit, no-trigger)`
5. Terminate when all lines are attributed

**This is exactly git blame's algorithm**, but reading from trace files instead of computing diffs. Because trace files store pre-computed attributions (not raw diffs), it's **faster than git blame** — no diff computation needed, just range matching.

**Complexity**: O(C) where C = commits with trace files in file history. Each commit's trace lookup is O(log k) binary search over sorted ranges. Total: **O(C * log k)**.

### Graceful degradation

For commits **before** Oxide was adopted, we fall back to git blame seamlessly:
- Pre-Oxide commits: standard git blame attribution (author, commit, date)
- Post-Oxide commits: rich attribution (actor, method, trigger, timestamp, sub-commit granularity)

No migration needed. It just gets richer over time.

## Git metadata: commit trailers

Commits made through Oxide carry lightweight metadata as git trailers:

```
fix: resolve type mismatch in parser

Oxide-Agent: claude
Oxide-Authored: human:35%,agent:65%
Oxide-Segments: human:1-9,42-45 agent:10-41
```

These are informational — the trace files are the source of truth. Trailers give a quick summary without needing to parse trace files.

## Performance summary

| Operation | Complexity | Notes |
|-----------|-----------|-------|
| Record edit | O(1) amortized | Append to journal |
| Update blame index | O(log n) | Lazy offset tree |
| Crystallize on commit | O(changed_ranges) | Delta-only trace |
| Query `oxide blame` | O(C * log k) | Walk commits, binary search ranges |
| Storage per commit | ~100 bytes/range | Negligible |

## Data flow

```
Editing session:
  keystroke/MCP call/terminal cmd
       │
       ▼
  Session Journal (append-only, in-memory)
       │
       ▼
  Blame Index (lazy offset tree, in-memory)
       │
       ├──► ~/.oxide/history/ (full sidecar, local-only)
       │
       ▼ (on git commit)
  .oxide/traces/<sha>.jsonl (delta-only, checked into git)
       │
       ▼ (on query)
  oxide blame <file> (walk traces + git blame fallback)
```

## Agent session detection

Oxide needs to know when an agent session starts and ends to properly scope journal entries and blame attribution.

**Primary signal: MCP connection lifecycle**

When an agent connects to the MCP server on localhost:4322 and calls `initialize` / tool discovery, that's a session start. When the connection drops (SSE stream closes, HTTP keepalive times out), that's session end. This is clean, protocol-level, and works for any MCP client — no agent-specific code.

```
Agent connects → MCP initialize → session_id assigned → journal entries tagged with session
Agent disconnects → session ends → journal entries for that session are closed
```

**Fallback signal: tmux pane lifecycle**

When the agent pane is spawned (process starts in tmux), that's a weaker session start signal. Less precise — the agent might not use MCP immediately, or the user might be typing in the agent pane before the agent responds. But it catches the case where an agent is running but hasn't connected via MCP yet.

MCP connection is the authoritative signal. Tmux lifecycle is the fallback.

## Competitive context: Entire (entireio/cli)

Entire is a Go CLI that hooks into Claude Code / Gemini / OpenCode to capture agent sessions as git checkpoints. It validates that teams want agent provenance. Key architectural differences:

**Why Oxide blame doesn't need what Entire builds:**

- **Session rewind**: Entire's core feature is checkpoint-based rewind (restore working tree to a previous agent turn). This is unnecessary in Oxide — git handles rewind (`git stash`, `git reset`), agents have their own undo (Claude Code `/undo`), and the IDE has native undo/redo. Entire needs rewind because it's external and can't control the editor. We *are* the editor.

- **Transcript parsing**: Entire parses agent-specific transcript formats (Claude JSONL, Gemini JSON, OpenCode plugin output) to reconstruct what happened. Oxide captures the full agent terminal via `tmux pipe-pane`/`capture-pane` — agent-agnostic, no format-specific parsers to maintain.

- **Agent-specific hook adapters**: Entire maintains separate hook implementations for each agent (Claude hooks, Gemini hooks, OpenCode plugin). Oxide uses MCP — any agent that speaks MCP gets provenance tracking for free. Zero agent-specific code.

**The fundamental difference:**

Entire reconstructs provenance from outside the IDE by observing transcripts after the fact. Oxide records provenance from inside — every edit flows through our buffer, attribution is captured at the source. Entire is a security camera watching the building. Oxide is the building knowing who opened every door.

**What Entire validates for us:**

- Teams want attribution percentages on commits (`human:35%,agent:65%`) — we already have this in our commit trailers design.
- The market for agent provenance is real and enterprises are willing to adopt tooling for audit trails.

## Open questions

1. **Blame index colocation**: Should blame metadata live on the rope (ropey leaf chunks) or as a separate parallel structure? Colocation is elegant but couples blame to the buffer implementation.

2. **Merge commits**: When a merge commit has multiple parents, trace files need to handle the union of changes. Follow git blame's approach — process each parent independently, pass blame to whichever parent the line matches.

3. **Rebase/squash**: When commits are rewritten, trace file SHAs become orphaned. Options: (a) rewrite trace file names during rebase (git post-rewrite hook), (b) store a SHA mapping table, (c) accept that pre-rebase granularity is lost (pragmatic).

4. **Agent attribution from terminal observation**: Matching terminal output (from `tmux capture-pane`) back to file edits is fuzzy. Phase 1 can use heuristics (timestamp correlation, file watcher events). Phase 2 can use structured agent output if agents adopt conventions.

5. **Ring buffer vs unbounded journal**: For long editing sessions, the journal could grow large. A ring buffer with crystallization checkpoints (flush to sidecar periodically) bounds memory without losing data.
