# Grace: Build Spec v2.0

*Updated 2026-11-26.*

## How to use this spec

1. Keep this file at `docs/GRACE_SPEC.md` in a new repo, next to `docs/DESIGN.md`, `docs/STATUS.md`, `AGENTS.md` and `CLAUDE.md`. All of them come in the starter kit, with the mascot in `assets/brand/`.
2. Open Claude Code in the repo and send:
   > Read AGENTS.md, docs/STATUS.md, docs/GRACE_SPEC.md and docs/DESIGN.md. Then, with my permission, do the read-only discovery in section 13 and propose the Milestone 0 plan. Don't write code until I say OK.
3. Don't paste the spec into the chat. Pasted text gets summarized away when a long conversation is compacted, while instruction files are re-read. That is exactly the problem Grace exists to solve.

---

## 0. Your role (coding agent)

You are the lead engineer building Grace for the people who will use it: developers who work with AI coding agents every day, and anyone who hops between AI chats, local models and accounts.

- Work one milestone at a time (section 14). Start each with a short plan (files, key decisions, risks, demo) and wait for the maintainer's OK.
- **Complex inside, simple outside.** The engine can be sophisticated. Everything a user sees or does, including installing and setting up, must be simple. `docs/DESIGN.md` is the rulebook for that. Read it before any UI, copy or setup work.
- **Light to ship and run.** Stay within the size and speed budgets in section 11.
- Tool formats change often. If this spec is wrong about a tool, trust what you observe, tell the maintainer, and fix the spec in the same commit.
- Explain new concepts in one or two plain sentences the first time they come up.

---

## 1. What Grace is

Grace is a local-first companion for people who work with many AIs. It carries your context to every AI you use, proves your rules actually reached each one, and investigates when a rule gets broken.

- **One line:** Write your rules once. Grace carries them to every AI and proves where any rule got lost: at load, at compaction, or in a hand-off.
- **Second line:** Evicted, overridden or ignored? Grace tells you which.
- **Identity:** an air-crash investigator for your AI. Calm, evidence first, no blame, works with any vendor.
- **Mascot:** Grace is also a small low-poly robot with teal accents who guides users through setup, explains findings and keeps them company (see `docs/DESIGN.md`).

Grace does four jobs:

| Job | What the user gets |
|---|---|
| **Remember** | One context (rules, decisions, tasks, where you left off) that follows you across coding agents (Claude Code, Codex CLI/IDE/app; later Antigravity and Cursor), local models (Ollama) and web chats (ChatGPT, Claude.ai, Gemini), across browsers and accounts. |
| **Prove** | Evidence, from what each AI actually received, that every rule arrived and survived each context change: load, compaction, resume, model switch, hand-off to another AI. |
| **Investigate** | For each broken rule: why, where (the exact turn), a fix, and a saved test so it can't come back silently. |
| **Assist** | "Ask Grace": a chat on any model the user picks (local Ollama, free or paid API) that tracks tasks, shows where you left off, and suggests the best free or paid AI for a task, based on evidence. |

### Glossary

- **Rule**: an instruction the AI must follow, with a stable ID (`R1`).
- **Decision**: a recorded project choice (`D1`), e.g. "PostgreSQL, not MySQL".
- **Boundary**: a moment the AI's context changes: session start, compaction, resume, model switch, hand-off. (UI label: "context change".)
- **Delivery**: whether a rule's text was in the request the AI actually received.
- **Verdict**: Grace's finding on why a rule broke (section 8.3).
- **Hand-off**: moving work from one AI to another (Claude Code → Codex, or to a web chat).
- **Context pack**: a compact, budgeted brief Grace builds for a target AI.
- **Checkpoint**: a git ref Grace creates so "Safe stop" can roll back safely.

---

## 2. The problem

1. **Context doesn't travel.** Switching AI, account, browser or session means re-explaining everything. Hitting a context limit mid-task loses the thread.
2. **Rules break and nobody can tell why.** AIs agree to rules, then break them. Was the rule never loaded, dropped at compaction, cut short, overridden or ignored? Today you can't tell.
3. **People lose track** of what they were doing across many AI sessions.
4. **Choosing an AI is guesswork.** Leaderboards don't reflect your own projects.

Evidence for the README and website (verify each figure against its source before publishing):

- Safety rules retained fell from 53% to 10% as compactions went from 1 to 5 ("compaction cliff"); typed handling kept 96%.
- ARCBench: in 21.1% of runs the agent broke a rule it still had in context; violations rose from 9.6% to 37.8% after compaction.
- Lost in Compaction: only 17% of constraints retained.
- Ollama silently trims prompts longer than its context window, from the front, with only a server-side log line. Its default window is 4,096 tokens on machines with under ~23 GiB of VRAM.
- Claude Code reads AGENTS.md only when no CLAUDE.md applies. A September 2026 check found that in 161 of 283 top repos with an AGENTS.md, Claude Code still read CLAUDE.md instead.
- ImpossibleBench: giving the model an abort option cut GPT-5's cheating from 54% to 9% (basis for "Safe stop").
- I-PASS structured hand-offs cut medical errors by 23% (basis for the context pack format).

---

## 3. What already exists: reuse it, don't rebuild it

| Stage of a rule's life | Existing tools (Nov 2026) | Grace's role |
|---|---|---|
| Write once | AGENTS.md standard; rulesync, Ruler | Reuse AGENTS.md as the carrier format |
| Carry everywhere | Rule-sync tools; memory layers (Supermemory, Mem0/OpenMemory, CORE); session movers (casr, continues, ai-memory); vendor memory import (Claude, Gemini) | Interoperate, don't compete |
| Predict what will load | rulereach, agents-doctor, whichrules, agnix, Claude Code `/doctor prompt-audit` | Optional input. Grace proves instead of predicting |
| **Prove it arrived and survived** | Fragments only: skill-souffleur (Claude Code skills), ctxray (user instructions dropped by compaction) | **Core** |
| **Prove it survived a hand-off to another AI** | None found | **Core: the new part** |
| Check it was followed | RuleProbe (output checks), hookify (guards) | Reuse ideas; ship simple built-in checks |
| **Explain why and where it broke** | agent-triage, Microsoft AgentRx (general agents, not coding agents or instruction files) | **Core**: verdicts + "Find the moment" |
| Suggest the best AI | Public leaderboards | **New angle**: the user's own evidence + a dated catalog |

Integration policy: interoperate through files, CLIs, plugins and MCP, with no hard dependency on any of these tools. Borrow ideas (AgentRx's step-by-step validation log, agent-triage's rule extraction, RuleProbe's AST checks) and respect their licenses.

---

## 4. Principles (non-negotiable)

1. **Simple outside.** Normal use needs no terminal and no file editing. Plain words. Every alert has one recommended action. Every change can be undone.
2. **Local-first and private.** Everything stays on the user's machine. Network calls go only to AI providers the user configures and to the update check. No telemetry.
3. **Evidence first.** Every finding cites file, session and line/turn. Model output is labelled "model opinion" and kept apart from evidence.
4. **Deterministic core.** Parsing, the delivery check, verdicts, "where you left off" and tasks work with no LLM. A model may add an optional opinion only where rules can't settle a case.
5. **Vendor neutral.** The same checks run for every tool. No tool is blamed or favoured.
6. **Light to ship and run.** Small installers, low memory, and no servers for us to run.
7. **The user's files are theirs.** Grace edits only its own managed blocks without asking, and backs up before any change.
8. **Shippable steps.** Every milestone ends with something a real user can install and try.
9. **Free to build, ship and use.** No paid service is needed to build, ship or use Grace. Dependencies are open source under permissive licenses (MIT, Apache-2.0, BSD, ISC, Zlib, OFL; MPL-2.0 only for unmodified files). CI, hosting, updates and distribution use free services. AI stays optional: free local models and free tiers come first, and anything that spends the user's own tokens is opt-in with a cost preview.

---

## 5. Scope and phases

**Phase 1: Prove (Claude Code first)**
- M0 Foundation: an installable app from day one
- M1 Rules and delivery check
- M2 Why it broke

**Phase 2: Remember (across tools)**
- M3 Where you left off: Home, tasks, context graph
- M4 Hand-off: Codex, MCP, "Continue in…" (the flagship demo)

**Phase 3: Assist and reach**
- M5 Ask Grace and Suggest an AI
- M6 Everywhere: web-chat context packs, Ollama proxy, browser extension

**Phase 4: Protect and go deeper**
- M7 Safety: Quick check, Memory check, Safe stop
- M8 Find the moment
- M9 Scoreboard, plus Antigravity and Cursor support

**Releases:** private alpha after M2, public beta after M3, launch after M4 with the hand-off demo.

**Not now:** cloud sync, accounts, teams, mobile apps, payments, background scraping of web chats, our own vector memory database, model training.

---

## 6. Stack

| Part | Choice | Why |
|---|---|---|
| Engine + CLI | **Rust** | Starts in milliseconds, which matters because hooks run on every tool call. Parses 100 MB transcripts quickly. Ships as one small binary per OS with no runtime to install. |
| Desktop app | **Tauri 2** | Uses the operating system's built-in web view instead of bundling a browser, so installers are a few MB and idle memory stays low. Built-in tray, notifications, installers and signed auto-updates. |
| UI | **React + TypeScript + Vite + Tailwind CSS + shadcn/ui (Radix) + React Flow** | Accessible components out of the box. One UI serves the desktop window and `grace serve`. |
| Storage | **SQLite** (rusqlite, bundled, with FTS5) | One local file with full-text search. No server. |
| Rules file | **TOML** via `toml_edit` | Easy to read and edit. Comments and formatting survive Grace's edits. |
| MCP | **rmcp** (official Rust SDK) | The same `grace` binary serves MCP over stdio. |
| Local API and proxy | **axum + tokio** | Serves the UI, the browser extension and the Ollama proxy. |
| Secrets | **keyring** crate | OS keychain on Windows, macOS and Linux. |
| Browser extension | **WXT + TypeScript** (Manifest V3) | One codebase for Chromium browsers; Firefox later. |
| Website | **Astro**, static | Free hosting on GitHub Pages or Cloudflare Pages. |

**Not chosen:** Electron (installers of 100 MB+, high idle memory) and bundled Python (large installers, slow start for hooks).

**Trade-off:** Rust compiles slowly. Use `cargo check` and incremental builds while developing, and let CI build the release installers.

---

## 7. Architecture

### 7.1 Repo layout

```
grace/
  Cargo.toml                  Rust workspace
  crates/
    grace-core/               engine library: adapters, store, keep, manifest, checks, verdict,
                              resume, tasks, packs, llm, advisor, redact
    grace-cli/                the `grace` binary: CLI, hooks, MCP (stdio), local API, `serve`, Ollama proxy
  app/                        Tauri 2 desktop shell (src-tauri); bundles grace-cli as a sidecar
  ui/                         React + TypeScript web UI (desktop window and `grace serve`)
  extension/                  browser extension (WXT), M6
  plugins/
    claude-code/              Claude Code plugin: hooks, MCP server, /grace commands, skill
    codex/                    Codex plugin: MCP server, skill (hooks via config; see section 9)
  site/                       Astro landing page and docs
  assets/brand/               mascot renders and icons
  catalog/models.toml         model catalog (data, not code)
  tests/fixtures/             tiny redacted transcripts
  docs/                       GRACE_SPEC.md, DESIGN.md, STATUS.md, ARCHITECTURE.md, adr/
```

### 7.2 How the pieces fit

- **`grace` binary (grace-cli):**
  - CLI commands for power users and automation.
  - `grace hook <tool> <event>`: the fast path that agent hooks call.
  - `grace mcp`: MCP server over stdio.
  - `grace serve`: local API plus the embedded UI (`ui/dist` via `rust-embed`), for headless or remote machines.
  - `grace proxy ollama` (M6).
- **Desktop app (Tauri):**
  - Tray icon and main window.
  - Runs the background watcher and the local API in-process, using grace-core.
  - Bundles the CLI as a sidecar and installs it to `~/.grace/bin/`, so hooks and plugins can call it by a stable path. Offer to add it to PATH. Verify path handling on Windows.
- **One local HTTP API** (axum, 127.0.0.1, random port, token). Used by the desktop window, `grace serve` and the paired browser extension.
- **One SQLite database** (WAL mode), shared safely by the app and the CLI.

### 7.3 Adapter contract

```rust
pub trait Adapter: Send + Sync {
    fn tool(&self) -> ToolId;                           // ClaudeCode, Codex, ...
    fn discover(&self) -> Result<Vec<SessionRef>>;      // read-only
    /// Streams a session from `state` (None = from the start) and calls `sink`
    /// for every event and context change, in order. Returns the state to resume
    /// from next time, so growing transcripts are read incrementally.
    fn parse(&self, s: &SessionRef, state: Option<ParseState>, redactor: &Redactor,
             sink: &mut dyn FnMut(Parsed) -> Result<()>) -> Result<ParseOutcome>;
}
```

Context changes (boundaries) are detected in the same pass, so a 100 MB transcript is never held in memory. The tool version comes from the entries themselves. Unknown entry types become `Event { kind: Unknown, .. }` and are logged, never fatal. (Refined in M0 from the first draft, which had separate `events` and `boundaries` passes.)

### 7.4 Normalized events

`EventKind`: `UserMessage`, `AssistantMessage`, `ToolCall`, `ToolResult`, `RequestSnapshot` (the exact system prompt and instructions sent, when the source records it), `InstructionLoad`, `Compaction`, `Resume`, `ModelSwitch`, `Handoff`, `TaskUpdate`, `HookRun` (a hook ran: its event, name and result), `Unknown`.

Every event keeps `source_file` (through its session), `line_no`, `turn` and `timestamp`, so any finding can point to its evidence.

### 7.5 Storage

- **Global** `~/.grace/` (override with `GRACE_HOME`): `grace.db`, `config.toml`, `logs/`, `snapshots/`, `backups/` (for undo), `bin/`.
- **Per project** `<repo>/.grace/`: `keep.toml`, `incidents/`, `tests/`. Safe to commit and share.
- **Tables** (first draft, refine in M0): `projects`, `sessions`, `events`, `boundaries`, `rules`, `rule_deliveries`, `decisions`, `tasks`, `incidents`, `checkpoints`, `changes` (every edit Grace made, for undo), `chat_messages`, `captures`, `provider_configs` (no secrets).

### 7.6 Session sources

Verify each on a real machine before coding its adapter: formats change between releases.

**Claude Code**
- Transcripts: `~/.claude/projects/<encoded-project-path>/<session-id>.jsonl` (Windows: `%USERPROFILE%\.claude\projects\...`).
- Since 2.1.300 (Oct 14, 2026), transcripts record each API request. Verified on 2.1.330 (Nov 2026, desktop app) as three content-addressed entry types:
  - `api-request-shape`: the system prompt blocks and the tool list, keyed by `shapeHash`;
  - `api-request-blob`: one message, keyed by `hash`;
  - `api-request`: the call's `params` (model, betas, thinking, effort), its `shapeHash`, and its message list as a delta against an earlier request (`base`, `keep`, plus new blob hashes in `tail`).
  This is the delivery check's main evidence. Attachment `prompt_snapshot` also records the system prompt. For versions before 2.1.300, fall back to instruction attachments and hook logs, and say so in reports.
- Instruction loads are `attachment` entries of type `instructions`: `files[]` with `path`, `type` (`Project` for CLAUDE.md, `AutoMem` for MEMORY.md, …) and `content`.
- Compaction leaves a `system` entry with subtype `compact_boundary` (`compactMetadata`: `trigger` auto/manual, `preTokens`, `postTokens`, preserved segment; `parentUuid` is null and `logicalParentUuid` links back), then a `user` entry with `isCompactSummary`, `compact_file_reference` attachments for re-attached files, and a `derived-from` entry (`sessions[]` with `via: "compact"`). Resume and fork markers were not seen yet; `derived-from` with another `via` is the likely shape.
- The agent's own task lists appear as tool calls: `mcp__ccd_turn__update_plan` in desktop-app sessions, TodoWrite or TaskCreate/TaskUpdate in the CLI. Support all of them.
- User entries carry `turnPosition.promptIndex`, the turn number counted from 1. Desktop-app sessions have `entrypoint: "claude-desktop"`, and the `claude` CLI may not be installed at all.
- Hooks are recorded as `system`/`stop_hook_summary` entries and `hook_*` attachments (`hookEvent`, `hookName`).
- Bookkeeping entries (`custom-title`, `last-prompt`, `cost-state`, `queue-operation`, `file-history-*`, …) carry session metadata such as the title.
- Transcripts include the user's email (`session_context` attachment) and org IDs: redact them on ingest.
- AGENTS.md is loaded only when no CLAUDE.md applies (2.1.277+). `@AGENTS.md` inside CLAUDE.md imports it.
- Hooks to use: SessionStart (matchers startup/resume/clear/compact; can add context), PreCompact, PostCompact, UserPromptSubmit, PreToolUse (can deny), PostToolUse, Stop. InstructionsLoaded exists but can't block and has known bugs, so treat it as a hint only.
- On Windows, hooks have depended on Git Bash. Make sure the hook command works from PowerShell and cmd too.

**Codex (CLI, IDE extension, desktop app)**
- Rollouts: `~/.codex/sessions/YYYY/MM/DD/rollout-*.jsonl`. Newer builds may write Zstandard-compressed `.jsonl.zst`: support both.
- The first line (`session_meta`) includes `instructions` (the AGENTS.md text). `compacted` items mark compaction. `~/.codex/state_5.sqlite` is a metadata index (the number changes between releases; `state_5` in Nov 2026).
- The VS Code extension (`openai.chatgpt`) writes rollouts to the same folder with `originator: "codex_vscode"`. No `.zst` files seen yet (Nov 2026). Check whether the desktop app writes there too.
- Hooks have been GA since May 2026 (SessionStart, PreToolUse, PermissionRequest, PostToolUse, PreCompact, PostCompact, UserPromptSubmit, Stop and more), configured in `hooks.json` or `config.toml`.
- Never open `~/.codex/auth.json`.

**Ollama**
- Local API at `http://localhost:11434`, plus an OpenAI-compatible `/v1` API.
- The default context window depends on VRAM (4,096 tokens below ~23 GiB). Prompts that are too long get trimmed from the front, silently, with only a server log line. The proxy (M6) makes this visible.

**Web chats (ChatGPT, Claude.ai, Gemini)**
- Grace can't see what these apps send to their models, so it can't prove delivery there. Support them with context packs and, later, the browser extension (user-initiated only).

**Later:**
- Antigravity: the CLI is `agy` (on Windows under `%LOCALAPPDATA%\agy\bin`) and inherits hooks from Gemini CLI. Data lives in `~/.gemini/antigravity/` (IDE) and `~/.gemini/antigravity-cli/` (CLI), with `conversations/`, `brain/` and `knowledge/` folders. Transcript format still unverified.
- Cursor: local storage format unverified.

---

## 8. Feature specs

Each feature lists what it does, what the user sees (screens are defined in `docs/DESIGN.md`), and acceptance criteria. CLI commands exist for power users; the app is the main way in.

### 8.1 Rules (Keep), M1

Rules and decisions live in one small project file, `.grace/keep.toml`. Users normally edit rules in the app, in plain words.

```toml
version = 1
project = "my-app"

[[rules]]
id = "R1"
text = "Use pnpm, never npm."
pinned = true                      # re-send after compaction
applies_to = ["**"]                # path globs
check = { type = "bash_forbid", pattern = '\bnpm\s+(install|i|ci|run)\b' }

[[rules]]
id = "R2"
text = "Never edit files in migrations/."
check = { type = "path_forbid", glob = "migrations/**" }

[[decisions]]
id = "D1"
date = 2026-11-26
text = "Use PostgreSQL, not MySQL."
```

Grace renders the rules into a managed block in `AGENTS.md`, so every agent that reads AGENTS.md gets them:

```markdown
<!-- grace:begin (managed by Grace; edit rules in the Grace app or .grace/keep.toml) -->
## Project rules
- [R1] Use pnpm, never npm.
- [R2] Never edit files in migrations/.
<!-- grace:end -->
```

- If a `CLAUDE.md` exists, offer to add `@AGENTS.md` to it so Claude Code also loads the block. Then confirm with the delivery check rather than assuming.
- On first use, Grace imports bullet rules from existing AGENTS.md/CLAUDE.md files, gives them IDs, and shows them for confirmation.
- **User sees:** the Rules tab. Rules are cards in plain words; "Advanced" reveals the automatic check.
- CLI: `grace init`, `grace keep add "text" [--pin] [--check ...]`, `grace keep list`, `grace keep sync`, `grace decide "text"`.
- **Acceptance:** rendering is idempotent; text outside the markers is never touched; CRLF files work; tests cover import, add and sync.

### 8.2 Delivery check (Manifest), M1

Checks, without any model, that every active rule reached the AI after each context change.

- **Boundaries:** session start, each compaction, resume, model switch and (M4) hand-off.
- At each boundary, take the first request after it (the request snapshot, or instruction attachments on older versions) and classify each rule:
  - `reached`: exact match after normalizing whitespace and markdown
  - `cut_short`: only a prefix is present
  - `changed`: fuzzy match below exact
  - `lost`: not present
- Also list which instruction files loaded versus which Grace expected, and flag shadowing (e.g. CLAUDE.md present, so AGENTS.md is skipped).
- **Live mode:** hooks trigger a check right after each compaction. If a pinned rule is lost, Grace sends one notification.
- **User sees:** on the Rules tab, one status cube per connected tool for every rule (reached, cut short, lost, not checked yet).
- CLI: `grace manifest [--last | --session ID] [--project PATH] [--json]`. The exit code is non-zero when a pinned rule is lost, so it works in CI.
- **Acceptance:**
  - A fixture with a compaction that drops a rule is flagged at the right boundary.
  - A fixture where CLAUDE.md shadows AGENTS.md shows the AGENTS.md rules as lost, with the reason.
  - A 100 MB transcript is processed in under 10 s with modest memory.

### 8.3 Why it broke (Probable Cause), M2

For each broken rule: why, where, the fix, and a saved test.

**Finding violations**
1. Deterministic checks on tool calls. Start with four types: `bash_forbid`, `path_forbid`, `text_forbid` (in edits) and `require_before` (e.g. tests before commit).
2. The user flags it: a "This broke a rule" button on any session turn, or `grace flag R1 --last --turn 41 "it ran npm install"`.
3. An optional model judge for rules without checks (labelled as opinion).

**Decision procedure (no model needed)**
1. The rule is absent from the request at the violation turn:
   - It was never present in this session → **Never loaded**. Explain why: wrong file, CLAUDE.md shadowing AGENTS.md, path scope, or a tool that doesn't read that file.
   - It was present earlier → **Lost at a context change**. Name it: compaction, resume or model switch. A hand-off gives **Lost in hand-off**.
2. Only part of the rule is present → **Cut short** (file too long, nested file loaded only partly, Ollama window).
3. The rule is present, but a later instruction conflicts (user message, another instruction file, tool output, agent memory) → **Overridden**. Show both texts side by side.
4. A safety check (hook) for this rule was configured but the tool call went through → **Safety check didn't fire** (hook didn't run, failed open, or had the wrong matcher). Show the hook logs.
5. Otherwise → **Ignored** (present, no conflict, no safety check).

Each verdict carries a confidence level with reasons, evidence links (file, line, turn) and a one-sentence summary in plain words.

**Fixes** (shown in plain words with the exact diff on request; applied only after a yes; always undoable)

| Verdict | Fix |
|---|---|
| Never loaded | Move or import the rule to where this tool reads it |
| Lost at a context change | Pin it, and re-send it after compaction via a SessionStart(compact) or PostCompact hook |
| Lost in hand-off | Raise the pack budget or mark the rule as always included |
| Cut short | Shorten or split the file; for Ollama, raise the context window |
| Overridden | Show the conflict and record which wins as a Decision |
| Safety check didn't fire | Repair the hook config and add a hook self-test |
| Ignored | Add a blocking safety check (PreToolUse deny) and a Quick check |

**Outputs**
- Incident report `.grace/incidents/<date>-<rule>.md` in a no-blame format: Summary, Timeline, Evidence, Probable Cause, Contributing factors, Fix, Test.
- Regression test `.grace/tests/<incident>.toml` (rule, minimal transcript slice, expected verdict), run with `grace test`.
- **User sees:** the "Why it broke" screen: one sentence, the moment on a timeline, a "Fix it" button with undo, and "Show evidence" for the transcript lines.
- CLI: `grace why R1 [--last]`, `grace incidents`, `grace test`.
- **Acceptance:** one fixture per verdict, all classified correctly; every report cites line numbers.

### 8.4 Where you left off: Home, tasks, context graph (M3)

Shows what you worked on and where you left off, across tools.

- **Project card:**
  - last session: tool, when, how long
  - what you last asked
  - files changed (from tool calls and `git diff --stat`)
  - open tasks and open problems
  - a suggested next step (rule-based; an optional model summary can be added later)
- **Tasks:** one list merging the agents' own task lists (Claude Code's task tools, Codex's plan updates) with tasks the user adds. Each task links back to its session.
- **Context graph:** the last 7 days, with nodes for project, session, task, file, decision, rule and problem. Clicking a node shows its evidence. Keep it small and readable.
- **Brief on start (opt-in per project):** a SessionStart hook injects a "where you left off" brief of at most 300 tokens into a new agent session. This is how context follows you between tools.
- **User sees:** the Home screen, the Tasks tab, the Overview tab and the tray summary.
- CLI: `grace resume [project]`, `grace tasks`.
- **Acceptance:** Home renders in under 1 s with 30 days of sessions; graph links are correct on fixtures; everything works with no model.

### 8.5 Hand-off: Codex, MCP, "Continue in…" (M4), the flagship

- Codex adapter as described in 7.6, including `.jsonl.zst`.
- **Hand-off:**
  - builds a context pack (rules with IDs, decisions, task state, last request, recent files) within a token budget;
  - puts it where the target loads it (the managed block, or a file it references);
  - records a hand-off boundary.
- When the target session starts, the delivery check confirms that every rule and decision arrived. Anything missing gets the verdict **Lost in hand-off**.
- One project timeline across Claude Code and Codex sessions.
- **MCP server** (`grace mcp`) with tools `get_context`, `list_rules`, `add_decision`, `add_task`, `report_violation` and `safe_stop` (a stub until M7). Shipped inside the Claude Code and Codex plugins.
- **User sees:** "Continue in Codex" on the project card, and the hand-off on the timeline.
- CLI: `grace handoff --to codex [--from claude-code] [--last]`.
- **Demo to record:** Claude Code follows R1 → "Continue in Codex" → Codex breaks R1 → Grace shows the exact step and turn where R1 got lost, and fixes it.
- **Acceptance:** the demo runs end to end on a clean machine; a fixture covers a pack that loses a rule.

### 8.6 Ask Grace and Suggest an AI (M5)

**Providers**
- No AI (default): everything core still works.
- Ollama (local, free).
- Any OpenAI-compatible endpoint via base URL + key. This covers many free and paid services.
- Optional native Anthropic and Gemini adapters.
- Keys live in the OS keychain, never in plain config files. Use the user's own API keys only; never reuse an agent's login tokens.

**Chat** (a drawer available on every screen; `grace chat` in the CLI)
- Answers from Grace's own data (full-text search over sessions, tasks, decisions and problems), with links to sources.
- Creates and updates tasks and suggests next steps.
- Before anything goes to a cloud model, shows exactly what will be sent and redacts secrets.

**Suggest an AI** (a screen; `grace suggest "task" [--free] [--local]` in the CLI)

Ranked options in two clearly separated parts:
1. *Catalog facts* from `catalog/models.toml`: provider, free/paid, price, context window, good-at tags, RAM needed to run locally, `last_verified` and a source URL. This is data, never hardcoded. Flag entries more than 60 days old.
2. *From your projects*: how each tool actually did for this user (rule delivery, violations, hand-off losses), from the Scoreboard data.

Grace detects the user's RAM and GPU and suggests only local models that fit.

**Acceptance:** chat works fully offline with Ollama; no secrets in config files; suggestions always show both parts.

### 8.7 Everywhere (M6)

- **Context packs for web chats:** "Continue in ChatGPT / Claude / Gemini" buttons copy a ready-to-paste brief to the clipboard: a profile the user approved, project, rules, decisions, where you left off and the current task, sized for the target. This works in any browser and any account with no integration. CLI: `grace pack --for chatgpt|claude|gemini|generic [--budget N]`.
- **Ollama proxy:** `grace proxy ollama` listens on `127.0.0.1:11435` and forwards to `11434`. It logs redacted requests as sessions, compares prompt size with the model's real context window, and warns when rules will be cut (rules sit at the front, which is what Ollama trims). The app offers to set it up.
- **Browser extension** (`extension/`, WXT, Manifest V3, Chromium first):
  - "Insert context" pastes a pack into the chat box on chatgpt.com, claude.ai and gemini.google.com.
  - "Save this chat" sends the visible conversation to the local Grace API, only when the user clicks.
  - Least permissions (those three sites + localhost), pairing with a code shown in the app, and one adapter per site so a redesign breaks only that adapter.
- **Acceptance:** packs fit their budget; the extension works on all three sites; saved chats show up on Home.

### 8.8 Safety: Quick check, Memory check, Safe stop (M7)

- **Quick check (Readback):** short quiz-style probes generated from rule IDs ("Which rule covers migrations/?"), run in a forked or headless session so the user's real session is untouched (check the installed CLI's resume and fork flags). Opt-in, after compaction or on demand; shows the token cost first.
- **Memory check (Clearance):** watches the memory files agents write for themselves (Claude Code's MEMORY.md auto memory, Codex memories, claude-mem; verify locations). Compares new entries with rules and decisions, flags contradictions ("memory says use npm; R1 says pnpm"), and undoes them from Grace snapshots.
- **Safe stop and retry (Go-Around):** an abort the agent (MCP tool) or the user (button) can trigger when stuck or about to break a rule, instead of cheating. It stops, restores the last Grace checkpoint (git refs under `refs/grace/checkpoints`, made at session start and before risky commands), and re-briefs the agent with rules and task state on the next prompt. It must never lose uncommitted work without an explicit yes.
- **Acceptance:** tests for each, including "Safe stop keeps uncommitted work safe".

### 8.9 Find the moment (Bisect), M8

Finds the turn where the agent committed to breaking the rule.

- Rebuild the exact request at prefix *k* (Claude Code 2.1.300+ records full requests).
- Re-run the same model N times (default 3) and apply the rule's deterministic check to the next action.
- Binary-search for the earliest *k* where the violation shows up in most runs.
- This needs the user's own API key for the same model. Show a cost estimate and a hard cap before running, and cache results.
- **User sees:** "Turn 37: from here, the AI broke R1 in 3 of 3 replays (0 of 3 from turn 36)", plus what changed at turn 37.
- **Acceptance:** with a mock model and a planted turn, it finds the turn within log2(n) + 1 probes.

### 8.10 Scoreboard (Retention Index), M9

- **Local first:** for each tool, model and version, the rule delivery rate across context changes, the violation rate and the hand-off loss rate, from the user's own sessions. This feeds "Suggest an AI".
- **Public later, opt-in only:** anonymized scores (never transcripts), reproducible scenarios, re-run when tools release new versions, published as a static page.

---

## 9. Connecting tools (the setup engine)

The onboarding flow in `docs/DESIGN.md` runs on this engine.

- **Detect** (read-only): known session folders, executables on PATH, versions, the Ollama API, installed browsers.
- **Connect Claude Code:** prefer Grace's Claude Code plugin, which bundles the hooks, the MCP server and the `/grace` commands. The app runs `claude plugin install …` after showing what it adds. Fallback: add the hooks and MCP entry to Claude Code's settings, with a backup. Desktop-only users may not have the `claude` CLI, so the fallback must work on its own.
- **Connect Codex:** install Grace's Codex plugin for the MCP server and skill (`codex plugin marketplace add …`, then install). Codex plugin hooks have been limited to manually installed desktop plugins, so add Grace's hooks to Codex's `hooks.json`/`config.toml` with a backup. Verify current support first.
- **Ollama:** detect it; offer the proxy (M6).
- **Web chats:** context packs (M6) and extension pairing (M6).
- **Every change** is recorded in the `changes` table with a backup, shown in plain words ("Grace will add 2 hooks and 1 tool to Claude Code"), with the exact diff on request, and can be undone from Connections.
- **Disconnect** restores backups and removes plugins. **Remove Grace completely** undoes every connection and deletes Grace's data after confirmation.
- **Plugins as a second way in:** someone who installs Grace's plugin inside Claude Code or Codex first gets a `/grace setup` command that offers to install the app or the CLI, with consent.

---

## 10. Distribution and deployment

Local-first means there is almost nothing to deploy: users run Grace on their own machines.

- **Desktop installers**, built by GitHub Actions (`tauri-action`) on every tagged release:
  - Windows: NSIS installer, per-user, no admin rights.
  - macOS: universal `.dmg`.
  - Linux: `.AppImage` and `.deb`.
- **Auto-update:** Tauri's updater plugin with signed update manifests hosted on GitHub Releases. Check at most once a day. After an update, Grace shows "What's new" in one short card.
- **CLI-only install** for developers and remote machines:
  - prebuilt binaries on GitHub Releases;
  - one-line install scripts (`install.sh`, `install.ps1`);
  - a Homebrew tap;
  - an npm wrapper (`npx <package>`) that downloads the right binary;
  - Scoop and winget later.
- **Agent plugins:** a Claude Code plugin with its own marketplace repo, and a Codex plugin with its marketplace. Gemini CLI and Antigravity extensions later.
- **Website:** Astro static site on GitHub Pages (free for public repos) at `smitkohale.github.io/grace`, so no paid domain is needed. The download button picks the visitor's OS. Includes docs and a plain-language privacy page.
- **No servers in v1:** nothing to scale, no hosting bill, no user data on our side. The public Scoreboard (M9) is a static page built by CI.
- **Code signing, kept free:**
  - Windows: sign through the SignPath Foundation's free program for open-source projects (it needs a public repo, an OSI license and CI-built releases). Apply once the first releases exist.
  - macOS: signing needs a paid Apple Developer ID ($99/yr) with no free equivalent, so macOS builds stay unsigned (ad-hoc) and the download page shows the one-time "Open Anyway" step.
  - Linux: no signing needed.
  - Until Windows signing is approved, testers see "unknown publisher" warnings and get short "Run anyway" instructions.
- **Package name:** `grace` is taken on npm, crates.io and PyPI, and `grace-cli` on npm and crates.io. `grace-ai` and `getgrace` were free on all three on 2026-11-26. Pick one before the first release.

---

## 11. Engineering standards

**Budgets** (checked in CI where possible)

| What | Budget |
|---|---|
| Installer per OS | ≤ 20 MB |
| `grace` CLI binary | ≤ 15 MB |
| Hook run time (p95) | ≤ 30 ms |
| App idle in the tray | ≤ 60 MB RAM |
| App with window open | ≤ 200 MB RAM |
| App cold start | ≤ 1 s to first screen |
| UI navigation | ≤ 100 ms |
| 100 MB transcript | parsed in ≤ 10 s |
| Each mascot image in the UI | ≤ 100 KB |

**Code**
- Rust stable (Cargo workspace) and TypeScript in strict mode. pnpm for JS packages.
- Rust dependencies to start: clap, serde, serde_json, rusqlite (bundled), toml_edit, regex, globset, tracing, anyhow, thiserror, dirs.
- Added per milestone: notify, axum, tokio, rust-embed, strsim (M1–M3); zstd, rmcp (M4); reqwest, keyring (M5).
- UI dependencies: react, vite, tailwindcss, shadcn/ui (Radix), @xyflow/react, @tanstack/react-query, lucide-react.
- Ask before adding anything else and note its size impact.
- Cross-platform: Windows, macOS and Linux. CRLF-safe. Windows paths and home folders tested.
- i18n-ready: all UI strings in one place, English first.

**Tests and quality**
- `cargo test` with `insta` snapshots for reports.
- Tiny redacted fixtures, and drift tests: new or unknown entry types are logged, never crash.
- Vitest for UI logic; Playwright against `grace serve` for the key flows (onboarding, rules, "Why it broke").
- GitHub Actions on Ubuntu, Windows and macOS, including a size-budget check.
- rustfmt, clippy with warnings denied, and Biome for TypeScript.

**Docs:** README with the mascot and a 60-second demo GIF, `docs/ARCHITECTURE.md`, `docs/adr/` for decisions, `CHANGELOG.md`, and `docs/STATUS.md` updated every session.

**License:** the maintainer decides (MIT or Apache-2.0 suggested).

---

## 12. Privacy and security

- Transcripts can contain secrets and personal data. Redact on ingest (API keys, tokens, passwords, private keys; emails optional), store only what features need, and offer "Forget this session/project".
- Never open credential files: `~/.codex/auth.json`, `~/.claude/.credentials.json`, `.env`, keychains.
- Nothing leaves the machine unless the user picked a cloud model, and then only after a preview.
- Local API: binds to 127.0.0.1 only, requires a random token, and allows only the app's and the paired extension's origins.
- Extension: least privilege, no remote code, pairing code.
- Signed installers and signed updates.
- Run a short security checklist before every release.

---

## 13. Milestone 0: Foundation (start here)

**Before coding**, with the maintainer's permission:
1. Run read-only discovery on the development machine: which agents are installed (Claude Code, Codex, Ollama, Antigravity, Cursor), their versions and their session folders.
2. Open 2–3 real Claude Code transcripts and list every entry type you find (request snapshots, compaction markers, attachments, task tool calls). Show the list.
3. Propose the repo layout, the Event model, the first DB tables and the first UI routes. Wait for OK.

**Deliver**
1. The monorepo from section 7.1: Rust workspace, Tauri app, React UI with the design tokens and the mascot from `docs/DESIGN.md`, light and dark themes.
2. CI that runs the checks and builds installers for Windows, macOS and Linux on tags, with the size-budget check. Releases go to GitHub as drafts.
3. Core: Event model, SQLite store (WAL) with migrations, redaction with tests, Claude Code adapter v0 (discover, stream-parse, boundaries), tiny redacted fixtures.
4. CLI: `grace doctor` (installed agents, versions, session folders, Ollama status; read-only) and `grace sessions`.
5. App: onboarding steps 1–2 ("Hello" and "Find your tools") and a Sessions list.

**Done when:**
- Installers build in CI for all three OSes within budget.
- On a clean machine, a person goes from download to seeing their Claude Code sessions in under 2 minutes, with no terminal.
- Tests are green and `docs/STATUS.md` is updated.

---

## 14. Milestones and user testing

| # | Milestone | Done when |
|---|---|---|
| M0 | Foundation | Installable app shows your Claude Code sessions |
| M1 | Rules + delivery check | The Rules tab shows a rule lost at compaction, and Grace notifies once |
| M2 | Why it broke | All verdicts correct on fixtures; "Fix it" works with undo; test saved |
| M3 | Where you left off | Home answers "where was I?" across projects in under 1 s |
| M4 | Hand-off | Flagship demo recorded on a clean machine |
| M5 | Ask Grace + Suggest an AI | Offline chat with Ollama; suggestions show catalog facts and your evidence |
| M6 | Everywhere | Packs work on 3 web chats; Ollama proxy warns on cut-off rules; extension paired |
| M7 | Safety | Tests green, including uncommitted-work safety |
| M8 | Find the moment | Finds the planted turn with a mock model |
| M9 | Scoreboard + more tools | Local scoreboard feeds suggestions; Antigravity or Cursor adapter |

**User testing checkpoints** (after M2, M3 and M4): five people who didn't build Grace install it and try the main flow unaided while someone watches. Targets: setup in under 2 minutes, and each of them can answer "where was I?" and "why did the AI break this rule?" from Grace alone. Fix the friction before moving on.

---

## 15. How we'll know people use it

- **Setup:** new users connect at least one tool unaided in under 2 minutes.
- **Value:** they find answers about their own sessions without help.
- **Return:** they open Grace again within a week (ask them; there's no telemetry).
- **Signals:** release downloads, plugin installs, GitHub stars and issues, and an in-app "Send feedback" button that opens a prefilled GitHub issue or form.
- **Telemetry:** none by default. If usage stats are ever added: opt-in, anonymous, documented, and off by default.

---

## 16. Open questions (for the maintainer)

- Name on registries: `grace-ai` or `getgrace`?
- When to apply for free Windows signing (after the first public releases).

Decided on 2026-11-27: license Apache-2.0; public repo `smitkohale/grace`; website on GitHub Pages instead of a paid domain; the free code-signing path in section 10; the mascot's poses come from Grace's own 3D model built in M0 (`assets/brand/mascot/`).
