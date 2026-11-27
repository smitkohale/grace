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
      className="flex w-full items-center gap-2 rounded-control px-3 py-2 text-muted hover:bg-paper hover:text-ink"
    >
      <Icon className="size-4" aria-hidden="true" />
      {t.nav.theme[choice]}
    </button>
  );
}

export function AppShell({ children }: { children: ReactNode }) {
  return (
    <div className="flex min-h-screen">
      <a
        href="#main"
        className="sr-only focus:not-sr-only focus:absolute focus:left-2 focus:top-2 focus:z-10 focus:rounded-control focus:bg-surface focus:px-3 focus:py-2"
      >
        {t.skipToContent}
      </a>
      <aside
        className="flex w-56 shrink-0 flex-col border-r border-line bg-surface"
        aria-label={t.nav.label}
      >
        <div className="flex items-center gap-2 px-4 py-4">
          <GraceGlyph size={28} />
          <span className="font-display text-lg font-semibold">{t.appName}</span>
        </div>
        <nav aria-label={t.nav.main} className="px-2">
          <ul className="space-y-1">
            <li>
              <NavLink href="/sessions" icon={List}>
                {t.nav.sessions}
              </NavLink>
            </li>
          </ul>
        </nav>
        <div className="mt-auto p-2">
          <ThemeToggle />
        </div>
      </aside>
      <main id="main" className="min-w-0 flex-1">
        {children}
      </main>
    </div>
  );
}
