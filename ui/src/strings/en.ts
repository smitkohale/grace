// Every string the UI shows, in one place (spec §11: i18n-ready).
// Voice and wording follow docs/DESIGN.md §4 and §5.

const plural = (n: number, one: string, many: string) => `${n} ${n === 1 ? one : many}`;

export const en = {
  appName: "Grace",
  skipToContent: "Skip to content",
  start: {
    unreachable:
      "I can't reach Grace's engine right now. Your data is safe. Try again in a moment.",
    retry: "Try again",
  },
  nav: {
    label: "Grace",
    main: "Main",
    sessions: "Sessions",
    theme: {
      system: "Theme: match system",
      light: "Theme: light",
      dark: "Theme: dark",
    },
  },
  onboarding: {
    step: (n: number, total: number) => `Step ${n} of ${total}`,
    skip: "Skip setup",
    back: "Back",
  },
  welcome: {
    title: "Hi, I'm Grace.",
    body: "I keep your AIs on the same page. Setup takes about two minutes.",
    start: "Let's go",
  },
  tools: {
    title: "Let me look for the AI tools you use.",
    subtitle: "I only read files. Nothing on your computer changes.",
    looking: "Looking for your AI tools…",
    found: "Found",
    notFound: "Not found",
    willRead: "I'll read",
    laterUpdate: "Found. Supported in a later update.",
    toggle: (name: string) => `Read ${name} sessions`,
    sessions: (n: number) => plural(n, "session", "sessions"),
    version: (v: string) => `Version ${v}`,
    continue: "Continue",
    error: "I couldn't look for your tools. Your data is safe. Try again.",
    retry: "Try again",
    saveError: "I couldn't save your choices. Your data is safe. Try again.",
  },
  sessions: {
    title: "Sessions",
    count: (n: number) => plural(n, "session", "sessions"),
    search: "Search sessions",
    searchPlaceholder: "Search by title, prompt or project",
    empty: "No sessions yet. Start a chat in Claude Code and I'll pick it up.",
    noMatch: (q: string) => `No sessions match "${q}".`,
    reading: "Reading your Claude Code sessions…",
    progress: (done: number, total: number) => `${done} of ${total}`,
    refresh: "Check for new sessions",
    untitled: "Untitled session",
    turns: (n: number) => plural(n, "turn", "turns"),
    compactions: (n: number) => plural(n, "compaction", "compactions"),
    loadError: "I couldn't load your sessions. Your data is safe. Try again.",
    scanError: (n: number) =>
      `I couldn't read ${plural(n, "session", "sessions")}. The rest are up to date.`,
    columns: {
      session: "Session",
      project: "Project",
      lastActive: "Last active",
      length: "Length",
    },
  },
  detail: {
    back: "All sessions",
    contextChanges: "Context changes",
    contextChangesHelp:
      "Moments the AI's context changed: when the session started, compactions, model switches and resumes.",
    events: "What happened",
    turn: (n: number) => `Turn ${n}`,
    line: (n: number) => `line ${n}`,
    more: "Show more",
    source: "Source file",
    models: "Model",
    version: (v: string) => `Claude Code ${v}`,
    started: "Started",
    lastActive: "Last active",
    notFound: "I couldn't find this session. It may have been deleted.",
    boundary: {
      session_start: "Session started",
      compaction: "Context compacted",
      resume: "Continued from an earlier session",
      model_switch: "Model changed",
      handoff: "Handed off",
    },
    compaction: (trigger: string, before?: number, after?: number) =>
      before !== undefined && after !== undefined
        ? `${trigger === "auto" ? "Automatic" : "Manual"}: ${tokens(before)} → ${tokens(after)} tokens`
        : trigger === "auto"
          ? "Automatic"
          : "Manual",
  },
  mascot: {
    hello: "Grace, a small robot, waving hello",
    thinking: "Grace, thinking, with cubes circling her",
    idle: "Grace, a small robot with white hair and teal accents",
    sleeping: "Grace, resting with her eyes closed",
  },
  time: {
    justNow: "just now",
    minutes: (n: number) => (n === 1 ? "1 minute ago" : `${n} minutes ago`),
    hours: (n: number) => (n === 1 ? "1 hour ago" : `${n} hours ago`),
    yesterday: "yesterday",
    days: (n: number) => `${n} days ago`,
    months: (n: number) => (n === 1 ? "1 month ago" : `${n} months ago`),
  },
} as const;

function tokens(n: number): string {
  return n >= 1000 ? `${Math.round(n / 1000)}k` : String(n);
}

export const t = en;
