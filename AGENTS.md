# AGENTS.md

Rules for any AI agent working on this repo (Claude Code, Codex or others). Each rule has an ID, in Grace's own format, so Grace can later check whether its own builders kept them.

## Project

Grace carries your context to every AI, proves your rules reached each one, and investigates when a rule breaks. It's built for the people who will use it: complex inside, simple outside.

- Spec: `docs/GRACE_SPEC.md`
- Design (UI, setup, mascot, words): `docs/DESIGN.md`
- Where we left off: `docs/STATUS.md`

## Rules

- [R1] Read `docs/STATUS.md` before starting, and the relevant parts of `docs/GRACE_SPEC.md` and `docs/DESIGN.md` before each milestone.
- [R2] Build only the current milestone. Propose a short plan and wait for the maintainer's OK before starting a new one.
- [R3] Stack: Rust for the engine, CLI and Tauri 2 shell; TypeScript + React for the UI and extension. Support Windows, macOS and Linux.
- [R4] Simple outside: normal users never need a terminal or to edit files. Every alert has one recommended action, and every change Grace makes can be undone.
- [R5] Follow `docs/DESIGN.md` for UI copy, colours, the mascot and accessibility (WCAG 2.2 AA).
- [R6] Stay within the size and speed budgets in `docs/GRACE_SPEC.md` section 11. Ask before adding any dependency and note its size impact.
- [R7] Local-first. No network calls except to AI providers the user configured and the update check. No telemetry.
- [R8] Never open, copy or print credential files (`~/.codex/auth.json`, `~/.claude/.credentials.json`, `.env`, keychains). Redact secrets in anything stored, shown or committed.
- [R9] Never commit real session transcripts. Test fixtures are tiny, redacted samples.
- [R10] Core features (parsing, delivery check, why it broke, where you left off, tasks) must work with no LLM. Label any LLM output "model opinion", separate from evidence.
- [R11] Every finding cites its evidence: file, session and line/turn.
- [R12] Back up before changing any file outside Grace's managed blocks, and show the change and get a yes first.
- [R13] Every feature ships with tests. Run the checks below before saying a step is done.
- [R14] If the spec is wrong about a tool's behaviour, trust what you observe, tell the maintainer, and fix the spec in the same commit.
- [R15] Before ending a session, update `docs/STATUS.md`: what's done, what's next, open questions.
- [R16] Free only: no paid services or SDKs, and only dependencies under permissive open-source licenses (MIT, Apache-2.0, BSD, ISC, Zlib, OFL; MPL-2.0 only unmodified). Grace must stay free to build, ship and use.

## Commands

Confirmed in M0 (CI runs the same; see `.github/workflows/ci.yml`).

- **Rust checks** (engine and CLI): `cargo fmt --all --check && cargo clippy -p grace-core -p grace-cli --all-targets -- -D warnings && cargo test`
- **Desktop app check:** `pnpm -C ui build`, then `cargo clippy -p grace-app -- -D warnings`. Linux also needs WebKitGTK.
- **UI checks:** `pnpm install`, then `pnpm -C ui lint && pnpm -C ui test`
- **End-to-end:** `cargo build -p grace-cli && pnpm -C ui build && pnpm -C ui e2e`. Locally it uses the installed Edge.
- **Desktop app (dev):** `pnpm -C app tauri dev`
- **CLI:** `cargo run -p grace-cli -- --help`
- **Web UI without the desktop app:** `pnpm -C ui build`, then `cargo run -p grace-cli -- serve --open`
- **Mascot model:** `blender -b --factory-startup -P assets/brand/mascot/build_grace.py -- --out assets/brand/mascot/grace.glb`. Add `--compare FILE` for a front view aligned to the master render.
- **Mascot stills and app icon:** `pnpm -C ui build && node ui/scripts/render-stills.mjs`, then `pnpm -C app tauri icon ../assets/brand/mascot/renders/grace-icon-1024.png --output src-tauri/icons`
- **Licenses (R16):** `node scripts/check-licenses.mjs`. CI also runs `cargo deny check licenses`.

## Working agreement

- Plan, wait for OK, then build.
- Small commits with clear messages.
- Explain new concepts in one or two plain sentences the first time they come up.
