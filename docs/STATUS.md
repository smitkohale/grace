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

- **Window RAM budget raised to 250 MB** (maintainer, 2026-11-27), so live 3D Grace fits. WebView2 alone uses about 170 MB, and live Grace adds about 45 MB while she's on screen. Spec §11 updated.

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

## M0 progress (2026-11-27)

Done and committed, with tests green (Rust fmt, clippy and tests; Biome, Vitest; Playwright in Edge):
- Tasks 0–5 in `docs/plans/m0.md`:
  - repo setup and spec fixes;
  - event model and redaction;
  - Claude Code reader (parses 100 MB in 1.9 s);
  - SQLite store with incremental scans;
  - `grace doctor`, `grace sessions`, `grace serve`;
  - the UI: onboarding steps 1–2, Sessions list, session detail.
- Checked against this machine's real transcripts. That found three things, now fixed:
  - turns count from 1;
  - two new attachment types, caught by the drift log;
  - stored sessions are re-read when the parser version changes.

Task 6, 3D mascot, done:
- `assets/brand/mascot/build_grace.py` builds `grace.glb` (177 KB) from pixel measurements of the master render. `--compare` renders an aligned front view for overlays.
- `ui/src/mascot/` shows her live with three.js. It matches the master's camera (about 6° down, long lens), sampled colours and soft studio light with shadows.
- Stills (WebP, 6–28 KB) and the app icon come from `ui/scripts/render-stills.mjs`.
- DESIGN.md §5/§7 and spec §11 updated.
- The repo is public at https://github.com/smitkohale/grace (maintainer said yes on 2026-11-27).

Tasks 7–8, desktop app and CI, done:
- `app/src-tauri` runs the engine in-process. The local API binds a random 127.0.0.1 port with a fresh token, and the window gets both through the `api_info` command. Icons come from Grace's head.
- Checked on this machine through WebView2's debugging port: Find your tools shows the real tools, and Continue opens the Sessions list.
- RAM (debug build; private working set, WebView2 included):
  - about 165–175 MB on Sessions;
  - 215 MB with live 3D Grace on screen;
  - 207 MB on Sessions after onboarding;
  - the engine itself uses 4–10 MB.
  - All within the 250 MB budget, raised from 200 MB by the maintainer so live 3D Grace fits. Measuring also found three GPU-memory leaks in the mascot, now fixed.
- CI (`.github/workflows/ci.yml`) is green on Windows, macOS and Linux:
  - engine and CLI checks;
  - UI lint, tests and build;
  - onboarding end to end;
  - desktop app lint on Linux;
  - license allowlists;
  - CLI size (7.1 MB of 15 MB).
- `release.yml` builds draft installers on `v*` tags.
- Scope note: bundling the `grace` CLI as a sidecar moves to M1, where hooks first need it. M0's app runs the engine in-process.

Task 9, ship:
- Tagged `v0.0.1`. The release workflow built one draft release (private until published) with four installers:

  | Installer | Size |
  |---|---|
  | Windows | 3.86 MB |
  | macOS (universal) | 9.54 MB |
  | Linux `.deb` | 4.73 MB |
  | Linux `.AppImage` | 79.43 MB |

- The AppImage carries its own copy of the Linux web engine, so it can't meet the 20 MB budget. Per R14, the spec and `check-sizes.mjs` now report it without budgeting it. The Linux job of the `v0.0.1` run still shows that size failure; the next tag will be green.

Accessibility pass (2026-11-27), done:
- New end-to-end checks (`ui/e2e/accessibility.spec.ts`) cover every M0 screen, in light and dark:
  - text and input contrast;
  - no sideways scrolling at 480 px (the smallest window at 200% zoom) or 320 px;
  - a skip control that reaches the main content;
  - a title for each screen;
  - clear screen-reader names for session rows;
  - an announced tool count;
  - Grace holding still with reduced motion.
- They found, and these fixes resolve:
  - the "Found" and "Not found" chips and the "1 compaction" label below 4.5:1 contrast;
  - a search-box border at 1.2:1;
  - a Sessions list that cut off its columns at 200% zoom;
  - a skip link that reloaded the page instead of moving focus;
  - every screen titled just "Grace".
- Dark mode also hid the eyes of the sidebar logo; fixed.
- The desktop window can now be zoomed with Ctrl or Cmd and + or −. That isn't tried by hand yet; it ships in the next installer.

## Queue

One piece at a time; nothing here is urgent. "Maintainer" marks the steps only you can do.

**Close M0**
1. Maintainer: install the Windows build from the `v0.0.1` draft in a fresh Windows account. Done when the Sessions list is reached in under 2 minutes with no terminal. Also note how long the window takes to appear (budget 1 s).
2. Maintainer: compare live Grace with the master render and point out any differences.
3. Tag `v0.0.2` with the accessibility fixes, then check that zooming the desktop window works.
4. Publish the release. Then change the README's "Not ready to install yet" to a download link with the "Run anyway" and "Open Anyway" steps.
5. Maintainer: apply for free Windows signing (SignPath Foundation) once a release is public.

**Before the private alpha (after M2)**
- Website (Astro on GitHub Pages): an OS-detecting download button, the unsigned-installer steps, a privacy page.
- Auto-update: Tauri's updater with signed manifests on GitHub Releases, checking at most once a day.
- An in-app "Send feedback" button that opens a prefilled GitHub issue.
- Docs: `CHANGELOG.md`, `docs/ARCHITECTURE.md`, a README demo GIF.
- Pick the registry name before the first published package.
- A short security checklist, run before every release (spec §12).

**Milestones** (each starts with a plan and the maintainer's OK, R2)
- M1 Rules and delivery check. Includes setup steps 3–4, the tray, notifications, hooks, and the CLI installed to `~/.grace/bin`.
- M2 Why it broke. Then the private alpha and the first five-person user test.
- M3 Where you left off. Then the public beta.
- M4 Hand-off (Codex, "Continue in…"). Then the launch.
- M5–M9 as in the spec.

To preview the web UI: `pnpm -C ui build`, then `cargo run -p grace-cli -- serve --open`.

## Open questions

- Name on registries: `grace-ai` or `getgrace`? (Not needed until the first published package.)
- When to apply for free Windows signing (after the first public releases).
