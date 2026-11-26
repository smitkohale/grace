# Grace: Design Guide

*How Grace looks, sounds and feels. Read this before any UI, copy or setup work. Updated 2026-11-26.*

---

## 1. Complex inside, simple outside

Grace's engine is sophisticated. Users should never feel that.

1. **Answer first, evidence on tap.** Every screen leads with one plain sentence. Details sit behind "Show evidence".
2. **One clear next step.** Every alert has one recommended action button, plus "Not now".
3. **Undo everything.** Any change Grace makes can be undone from the same place for 30 days.
4. **No terminal, no file editing.** Everything a normal user needs works in the app. The CLI is for power users and automation.
5. **Safe defaults.** Private by default, no AI required, advanced options under "Advanced".
6. **Calm, not noisy.** Notify only when action is needed, and never more than once per problem.
7. **Fast feel.** Navigation under 100 ms. Slow work runs in the background while Grace shows her "thinking" state.
8. **Accessible.** WCAG 2.2 AA: keyboard navigation, screen-reader labels, visible focus, colour never the only signal, reduced motion respected, usable at 200% zoom.

---

## 2. Setup: two minutes, no terminal

### Install
- The website's download button picks the right installer for the visitor's OS.
- Windows: per-user installer, no admin rights. macOS: drag to Applications. Linux: AppImage or .deb.
- Windows installers are signed through a free open-source signing program, so users don't see "unknown publisher" warnings. macOS builds are unsigned (Apple's signing costs money), so the download page shows the one-time "Open Anyway" step in System Settings.

### First run: four short steps, each skippable, with Grace guiding

1. **Hello.** Grace waves: "Hi, I'm Grace. I keep your AIs on the same page. Setup takes about two minutes." [Let's go]
2. **Find your tools.** "Let me look for the AI tools you use." Grace scans read-only and shows cards: Claude Code ✓, Codex ✓, Ollama ✓, Cursor (not found). Each card says exactly which folder she will read, with a toggle.
3. **Your rules.** Pick a project folder, or one of the recent projects Grace found. Grace imports rules from AGENTS.md or CLAUDE.md as cards. The user ticks the ones that matter and can add new ones in plain words ("Never touch the migrations folder").
4. **Connect.** One toggle per tool: "Grace will add 2 hooks and 1 tool to Claude Code. [Show exact changes]". Then, optionally: "Want me to answer questions and suggest AIs? Pick a brain: No AI (default) · On this computer (Ollama) · Use an API key (free or paid)."

**Done:** the Home screen with the first "Where you left off" card, or a friendly empty state.

### Setup rules
- Nothing changes on disk before the user says yes. Every change is listed in plain words, with the exact diff on request.
- Grace backs up anything she edits. "Disconnect" restores it.
- "Remove Grace completely" (Settings) undoes every connection and deletes Grace's data after confirmation.
- Power users get the same flow in the terminal: `grace init`.
- Agent plugins are a second way in: Grace's Claude Code or Codex plugin offers to install the app, with consent.

---

## 3. Screens

**Layout:** a sidebar (Home · Projects · Problems · Suggest an AI · Connections), the main area, and the Ask Grace drawer on the right. Minimum window 960 × 640.

### Home: "Where you left off"
- Grace greets the user with today's status: "All rules reached your AIs today" or "1 thing needs a look".
- One card per active project: last tool and when, what you last asked, files changed, open tasks, problems.
- Buttons: **Continue in…** (Claude Code, Codex, ChatGPT, Claude, Gemini) and **Open project**.

### Project
- Tabs: **Overview** (timeline + small context graph), **Rules**, **Tasks**, **Problems**, **Sessions**.

### Rules tab
- Each rule is a row with one status cube per connected tool: reached, cut short, lost, not checked yet (section 6).
- Add or edit rules in plain words. "Advanced" reveals the automatic check (e.g. "block `npm install`").

### Why it broke
- One sentence first: "Codex used npm even though R1 says pnpm. R1 never reached Codex: the hand-off pack was cut short."
- A verdict chip (e.g. "Lost in hand-off") with a one-line explanation.
- A timeline marking the exact moment.
- **Fix it** (with undo) as the main button. **Show evidence** expands the transcript lines. **Find the moment** shows a cost estimate first if it needs an API key.
- Small print: "I saved a test so this can't come back silently."

### Ask Grace (drawer)
- Available from every screen, with a model picker.
- Before anything goes to a cloud model, a preview shows exactly what leaves the computer.

### Suggest an AI
- "What do you want to do?" plus toggles: Free only · Runs on my computer.
- Ranked cards, each with a "Why" section, a "From your projects" section and a "Last checked" date.

### Connections (Settings)
- Every tool, Connected or Not connected, with what Grace changed and an Undo button.
- Ollama status, browser extension pairing, AI providers and keys (kept in the OS keychain), privacy, and "Remove Grace completely".

### Tray / menu bar
- A status line, Open Grace, Copy context for…, Pause Grace, Quit.

### Notifications
- Only for problems that need action, one per problem, and they can be turned off. Clicking one opens "Why it broke".

### Browser extension popup (M6)
- "Insert context" and "Save this chat", with Grace's small avatar.

### Empty states
- Teach the next step, with Grace: "No sessions yet. Start a chat in Claude Code and I'll pick it up."

---

## 4. Words

Use plain labels in the UI. The aviation names are the brand: use them on the website, in docs and in tooltips.

| UI label | Brand name | Internal term |
|---|---|---|
| Rules | Keep | `keep.toml` |
| Delivery check | Manifest | `rule_deliveries` |
| Why it broke | Probable Cause | `verdict` |
| Find the moment | Bisect | `bisect` |
| Quick check | Readback | `readback` |
| Memory check | Clearance | `clearance` |
| Safe stop and retry | Go-Around | `safe_stop` |
| Scoreboard | Retention Index | `retention` |
| Context change | Boundary | `boundary` |
| Continue in… | Hand-off | `handoff` |

**Verdict labels:** Never loaded · Lost at a context change (say which: compaction, resume, model switch) · Lost in hand-off · Cut short · Overridden · Safety check didn't fire · Ignored.

### Writing rules
- Short sentences, everyday words, active voice.
- Say what happened, why, and what to do, in that order.
- Never blame the user or any AI vendor. Describe, don't judge: "R1 didn't reach Codex", not "Codex failed you".
- Numbers over adjectives: "3 of 6 rules", not "some rules".
- Usually no exclamation marks; never more than one per screen.
- Errors say what went wrong, whether data is safe, and what to try.

---

## 5. Grace, the mascot

**Look:** a small, calm, low-poly robot. White angular hair with teal streaks and a single strand standing up on top, square black eyes lit with warm amber, teal plus marks on her cheeks, a black ribbon, a black scarf with a teal diamond emblem, black boots with teal bands, and a tail that trails small floating teal cubes.

**The cubes are the story.** They are the pieces of context Grace carries between AIs, and the same cube is the UI's status symbol (section 6). The mascot and the product say one thing.

**Personality:** an air-crash investigator in a tiny body. Calm, precise, kind, evidence first, never blames. Helpful, not chatty.

**Voice:** first person, short, warm, plain.

| Moment | Grace says |
|---|---|
| Welcome | "Hi, I'm Grace. I keep your AIs on the same page." |
| All clear | "All 6 rules reached Claude Code today." |
| Problem | "R1 didn't make it to Codex after the hand-off. Want me to fix it?" |
| Working | "Reading your Claude Code sessions…" |
| Fixed | "Done. I'll re-send R1 after every compaction. [Undo]" |
| Error | "I couldn't read Codex's session files. They may have moved after an update. Your data is safe. [Show details]" |

### States (poses)

| State | Where | Pose |
|---|---|---|
| Hello | Onboarding, About | Waving |
| Thinking | Scans, loading | Cubes circling her |
| All clear | Home with no problems | Relaxed, eyes softly curved |
| Found something | Problem alerts | Top strand up, amber eyes brighter |
| Investigating | Why it broke | Holding a small clipboard |
| Carrying | Hand-off in progress | Tail cubes streaming to one side |
| Sleeping | Paused, or nothing yet | Eyes closed, cubes resting |

**Where she appears:** onboarding, empty states, the Home greeting, problem alerts, the chat drawer avatar, the tray menu header, the extension popup and the website. Data screens (Rules, Sessions) stay clean.

**Motion:** subtle. A 2 px bob, a blink every few seconds, slowly floating cubes. CSS only, no live 3D. Motion turns off when the system's "reduce motion" setting is on, and pauses when the window is hidden.

**Don't:** stretch or recolour her, place her on busy backgrounds, give her long paragraphs, or show her in front of an error. Problems get a clear message first; Grace supports it.

### Assets (`assets/brand/`)
- **Now:** `grace-mascot.png` (the master render) and `grace-mascot-720.webp` (light version for the UI). The render's background is a warm gradient, so place her on a card with the same gradient (`--mascot-bg`) and she blends in until cut-outs exist. Use that light card in dark mode too.
- **Needed from the 3D source:**
  - each pose above as a transparent PNG (2048 px master), exported to WebP at 512, 256 and 128 px, with the same camera and lighting as the master;
  - a head-only crop for the app icon (1024 px);
  - a flat, single-colour tray glyph (SVG), such as the two square eyes or the diamond emblem, because a 3D render is unreadable at 16–22 px.
- Keep every UI image at or under 100 KB.
- **Logo:** the terracotta leaf (`logo.jpeg`) marks the website and README only. The app icon, installer and tray use Grace herself (a head render and the flat eye glyph).

---

## 6. Visual system

### Palette (sampled from the mascot render)

```css
:root {
  --paper: #F7F1EB;        /* app background */
  --surface: #FFFBF7;      /* cards */
  --ink: #35312E;          /* text and icons (11.5:1 on paper) */
  --muted: #6F665F;        /* secondary text (5.0:1 on paper) */
  --line: #E6DCD3;         /* decorative borders */
  --teal: #55C9CD;         /* brand fills: primary buttons, reached cubes */
  --teal-strong: #137879;  /* links, teal text, cube outlines, focus ring (4.7:1 on paper) */
  --amber: #FCC067;        /* "needs a look" fills */
  --amber-strong: #A86A12; /* amber outlines and icons (4.0:1 on paper) */
  --coral: #C2412D;        /* lost / error text, icons, outlines (4.6:1 on paper) */
  --on-accent: #1F1D1B;    /* text on teal or amber fills (8.5:1 on teal) */
  --mascot-bg: linear-gradient(180deg, #DCD1C8 0%, #F3E5DB 100%);
  --radius-card: 8px;
  --radius-control: 6px;
}

[data-theme="dark"] {
  --paper: #1F1D1B;
  --surface: #2A2724;
  --ink: #F3ECE6;          /* 14.4:1 on paper */
  --muted: #B3A9A1;        /* 7.3:1 */
  --line: #3A3632;
  --teal: #55C9CD;         /* 8.5:1 on paper */
  --teal-strong: #7AD8DB;  /* 10.1:1 */
  --amber: #FCC067;
  --amber-strong: #FCC067; /* 10.3:1 */
  --coral: #F08A76;        /* 6.9:1 */
  --on-accent: #1F1D1B;
}
```

**Rules**
- On light backgrounds, `--teal` is for fills only. As text it's 1.8:1, so use `--teal-strong` for links and teal text.
- Primary buttons: `--teal` fill with `--on-accent` text in both themes.
- Status shapes and focus rings use the `-strong` colours, so they meet the 3:1 contrast needed for non-text elements.

### Status cubes

| Status | Look | Colours | Label |
|---|---|---|---|
| Reached | Solid cube | `--teal` fill, `--teal-strong` outline | "Reached" |
| Cut short | Half-filled cube | `--amber` fill, `--amber-strong` outline | "Cut short" |
| Lost | Hollow cube | `--coral` outline | "Lost" |
| Not checked | Dashed cube | `--muted` outline | "Not checked yet" |

Always pair shape and colour with the label or a tooltip.

### Shape, type and icons
- **Shape:** 8 px card corners, 6 px controls, flat surfaces with one soft shadow level. Mascot cards may use one chamfered corner to echo her low-poly style.
- **Type:** Inter (UI), Space Grotesk (headings), JetBrains Mono (evidence and code). All are open-licensed and self-hosted, so the app works offline. Base size 14 px, line height 1.5.
- **Icons:** Lucide, 1.5 px stroke.

---

## 7. Performance feel

- The window appears within 1 s. Home renders from the local database immediately, then refreshes.
- Anything slower than 300 ms gets a skeleton loader and Grace's "thinking" state.
- Long jobs (the first scan, "Find the moment") run in the background, with progress in the tray.
- Images are lazy-loaded; each mascot image is at most 100 KB.
- The extension and the website use the same tokens, cubes and voice.
