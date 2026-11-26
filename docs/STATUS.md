# Status

**Current milestone:** M0, Foundation (plan approved 2026-11-27; in progress, task list in `docs/plans/m0.md`)
**Last session:** 2026-11-27

## Done

- Build spec v2 (`docs/GRACE_SPEC.md`) and design guide (`docs/DESIGN.md`) written, 2026-11-26
- Mascot master render added to `assets/brand/grace-mascot.png`; logo added as `assets/brand/logo.jpeg`
- 2026-11-27: dev tools installed (MSVC Build Tools 2022, Rust 1.100.0, GitHub CLI 2.106.0 signed in, Blender 5.3.0, pnpm 12.23.0; Node 22.22.2, Git 2.55 already present)
- 2026-11-27: project moved out of OneDrive to `C:\dev\grace` (OneDrive would sync build output; short path with no spaces)
- 2026-11-27: read-only discovery done (findings below)

## Decisions

- **Mascot is a live, interactive 3D model** (maintainer, 2026-11-27). Replaces DESIGN.md's "CSS only, no live 3D". Only a 2D render exists, so the model is built by script in Blender (`grace.blend` -> `grace.glb`, flat colours, target under 300 KB) and drawn with three.js. Small sizes use still renders of the same model. Update DESIGN.md and spec budgets in the same change.

- **Free to build, ship and use** (maintainer, 2026-11-27): permissive open-source dependencies only, free CI and hosting, no paid services. New rule R16 in AGENTS.md; spec §4 principle 9.
- **Made with the M0 plan (2026-11-27):**
  - public repo `smitkohale/grace`, created at the end of M0 after one more check with the maintainer;
  - Apache-2.0 license;
  - Windows signing through the SignPath Foundation (free for open source); macOS unsigned with the "Open Anyway" step;
  - website at `smitkohale.github.io/grace`;
  - the leaf logo on the website and README only, with the app icon from Grace's head;
  - small boxy arms, hidden at rest.
  - See `docs/adr/0001-stack-mascot-free.md`.

## Discovery findings (2026-11-27)

| Tool | Installed | Sessions |
|---|---|---|
| Claude Code | Desktop app (Code tab); `claude` CLI not on PATH | `~/.claude/projects/<encoded-path>/<id>.jsonl`, 3 files (2.4–20 MB), all v2.1.330 |
| Codex | data folder present; `codex` not on PATH | `~/.codex/sessions/YYYY/MM/DD/rollout-*.jsonl`, 11 files, no `.zst` yet; index is `state_5.sqlite` (spec says `state.db`) |
| Antigravity CLI | `agy` 1.1.5 | `~/.gemini/antigravity-cli/` (`history.jsonl`, conversation cache); IDE data in `~/.gemini/antigravity/brain/` |
| Ollama | no (API not responding) | n/a |
| Cursor | no | n/a |

Credential files seen by name only and never opened: `~/.codex/auth.json`, `~/.gemini/oauth_creds.json`, `~/.gemini/google_accounts.json`.

**Claude Code 2.1.330 entry types** (counted across 3 transcripts, no content read):
- Messages: `user`, `assistant` (content blocks `text`, `thinking`, `tool_use`, `tool_result`, `image`)
- Request records (delivery-check evidence), content-addressed:
  - `api-request`: `params` (model, betas, thinking, effort), `shapeHash`, `tail` (blob hashes), `keep`, `cacheControl`, `requestDigest`
  - `api-request-shape`: `shape.system` (system prompt blocks) + `shape.tools`, keyed by `shapeHash`
  - `api-request-blob`: one message keyed by `hash`
- Instruction evidence: attachment `instructions` (`files[]` with `path`, `type: "Project"`, `content`), attachment `prompt_snapshot` (`systemPrompt[]`)
- Compaction: `system` / `compact_boundary` with `compactMetadata` (`trigger`, `preTokens`, `postTokens`, preserved segment), then a `user` entry with `isCompactSummary`, plus `compact_file_reference` attachments
- Resume / fork: `derived-from` (`sessions[]` with `id`, `via`); `relocated` (cwd change)
- Hooks: `system` / `stop_hook_summary`, attachment `hook_additional_context` (`hookEvent`, `hookName`)
- Other attachments: `environment`, `model`, `date`, `file`, `edited_text_file`, `skill_listing`, `deferred_tools_*`, `mcp_instructions_delta`, `agent_listing_delta`, `auto_mode`, `session_context`, `credential_org`, `queued_command`, `silent_turn_reminder`, `thinking_drop`, `remote_session_change`
- Bookkeeping: `custom-title`, `agent-name`, `last-prompt`, `queue-operation`, `mode`, `tools-used`, `data-flow-records`, `atis-latch`, `file-history-snapshot`, `file-history-delta`, `cost-state`

**Spec corrections to make in the M0 commit (R14):**
- Task lists in Desktop sessions come from `mcp__ccd_turn__update_plan`, not TodoWrite/TaskCreate (none seen). Adapter must support all three.
- Request snapshots are split into shape + blobs, deduplicated by hash. Rebuilding a full request needs the blob chain; confirm how `tail`/`keep` cover earlier messages.
- Codex index file is `state_5.sqlite`, not `state.db`.
- `claude` CLI may be absent when only the Desktop app is used: connecting Claude Code can't rely on `claude plugin install`.

## Next

1. Build M0 by following `docs/plans/m0.md` and ticking its boxes.

## Open questions

- Name on registries: `grace-ai` or `getgrace`? (Not needed until the first published package.)
- When to apply for free Windows signing (after the first public releases).
