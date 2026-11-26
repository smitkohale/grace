# ADR 0001: Stack, a live 3D mascot, and free-only

*Status: accepted, 2026-11-27 (Milestone 0 plan approved by the maintainer).*

## Context

Grace starts from `docs/GRACE_SPEC.md` and `docs/DESIGN.md`. Before M0 the maintainer added two requirements:

1. The mascot must be a real, interactive 3D model, not CSS-animated images.
2. Everything must be free to build, ship and use, because many people will use Grace.

Discovery on the development machine also showed that Claude Code 2.1.330 records every API request in the transcript, which the delivery check (M1) depends on.

## Decision

- **Stack** as in spec §6: Rust (engine, CLI), Tauri 2 (desktop), React + TypeScript (UI), SQLite (storage). The axum API lives in `grace-core` behind a `server` feature, so `grace serve` and the desktop app share one API.
- **Adapters stream.** `Adapter::parse` reads a session from a saved position and reports events and context changes in one pass. Growing transcripts are read incrementally, and a 100 MB file is never held in memory.
- **Mascot:** the only source is a 2D render, so a Blender script (`assets/brand/mascot/build_grace.py`) builds a low-poly model from simple shapes. It is exported to `grace.glb` (flat colours, no textures, ≤ 300 KB) and drawn live with three.js, loaded lazily. Small spots use still renders of the same model. Motion stays subtle, pauses when hidden, and turns off with "reduce motion". The script is GPL-3.0-or-later because it runs inside Blender. It is a dev tool and not shipped; the model and renders are ours.
- **Free only (AGENTS.md R16):**
  - permissive open-source dependencies only;
  - a public GitHub repo, for unlimited free CI on all three OSes, free GitHub Pages and eligibility for free open-source code signing;
  - Apache-2.0 license;
  - Windows signing through the SignPath Foundation; macOS unsigned with the "Open Anyway" step, because Apple's signing is paid;
  - website at `smitkohale.github.io/grace`;
  - AI features stay optional, with free options first.
- **Logo:** the leaf marks the website and README. The app icon and tray use Grace herself.

## Consequences

- three.js adds about 170 KB gzipped, only on screens that show Grace.
- Until Windows signing is approved, testers see an "unknown publisher" warning. macOS users always need one extra step.
- The repo is public from the first push.
