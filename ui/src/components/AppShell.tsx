import { List, Monitor, Moon, Sun } from "lucide-react";
import { type ReactNode, useState } from "react";
import { Link, useRoute } from "wouter";
import { cn } from "../lib/cn";
import { setThemeChoice, type ThemeChoice, themeChoice } from "../lib/theme";
import { GraceGlyph } from "../mascot/GraceGlyph";
import { t } from "../strings/en";

function NavLink({
  href,
  icon: Icon,
  children,
}: {
  href: string;
  icon: typeof List;
  children: ReactNode;
}) {
  const [active] = useRoute(`${href}/*?`);
  return (
    <Link
      href={href}
      aria-current={active ? "page" : undefined}
      className={cn(
        "flex items-center gap-2 rounded-control px-3 py-2 text-ink hover:bg-paper",
        active && "bg-paper font-medium",
      )}
    >
      <Icon className="size-4 text-teal-strong" aria-hidden="true" />
      {children}
    </Link>
  );
}

const NEXT: Record<ThemeChoice, ThemeChoice> = { system: "light", light: "dark", dark: "system" };
const ICON = { system: Monitor, light: Sun, dark: Moon } as const;

function ThemeToggle() {
  const [choice, setChoice] = useState<ThemeChoice>(themeChoice);
  const Icon = ICON[choice];
  return (
    <button
      type="button"
      onClick={() => {
        const next = NEXT[choice];
        setThemeChoice(next);
        setChoice(next);
      }}
      className="flex items-center gap-2 rounded-control px-3 py-2 text-muted hover:bg-paper hover:text-ink md:w-full"
    >
      <Icon className="size-4" aria-hidden="true" />
      {t.nav.theme[choice]}
    </button>
  );
}

export function AppShell({ children }: { children: ReactNode }) {
  return (
    // A sidebar on wide windows; a top bar on narrow ones, including the
    // smallest window at 200% zoom (WCAG 1.4.10).
    <div className="flex min-h-screen flex-col md:flex-row">
      {/* A button, not a "#main" link: the URL hash holds the route. */}
      <button
        type="button"
        onClick={() => document.getElementById("main")?.focus()}
        className="sr-only focus:not-sr-only focus:absolute focus:left-2 focus:top-2 focus:z-10 focus:rounded-control focus:bg-surface focus:px-3 focus:py-2"
      >
        {t.skipToContent}
      </button>
      <aside
        className="flex shrink-0 flex-wrap items-center gap-x-2 border-b border-line bg-surface px-2 md:w-56 md:flex-col md:flex-nowrap md:items-stretch md:border-r md:border-b-0 md:px-0"
        aria-label={t.nav.label}
      >
        <div className="flex items-center gap-2 px-2 py-3 md:px-4 md:py-4">
          <GraceGlyph size={28} />
          <span className="font-display text-lg font-semibold">{t.appName}</span>
        </div>
        <nav aria-label={t.nav.main} className="md:px-2">
          <ul className="flex gap-1 md:block md:space-y-1">
            <li>
              <NavLink href="/sessions" icon={List}>
                {t.nav.sessions}
              </NavLink>
            </li>
          </ul>
        </nav>
        <div className="ml-auto md:mt-auto md:ml-0 md:p-2">
          <ThemeToggle />
        </div>
      </aside>
      <main id="main" tabIndex={-1} className="min-w-0 flex-1 outline-none">
        {children}
      </main>
    </div>
  );
}
