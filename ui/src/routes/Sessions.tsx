import { keepPreviousData, useQuery } from "@tanstack/react-query";
import { Layers, RefreshCw, Search, TriangleAlert } from "lucide-react";
import { useDeferredValue, useId, useState } from "react";
import { Link } from "wouter";
import { Button } from "../components/ui/button";
import { Card } from "../components/ui/card";
import { Skeleton } from "../components/ui/skeleton";
import { api, type SessionRow } from "../lib/api";
import { relativeTime } from "../lib/time";
import { usePageTitle } from "../lib/usePageTitle";
import { useScan } from "../lib/useScan";
import { GraceMascot } from "../mascot/GraceMascot";
import { t } from "../strings/en";

function Row({ row }: { row: SessionRow }) {
  const title = row.title ?? t.sessions.untitled;
  const lastActive = relativeTime(row.last_active_at);
  const turns = t.sessions.turns(row.turns);
  const compactions = row.compactions > 0 ? t.sessions.compactions(row.compactions) : null;
  return (
    <li>
      <Link
        href={`/sessions/${row.id}`}
        aria-label={t.sessions.rowLabel({
          title,
          lastPrompt: row.last_prompt,
          project: row.project_name,
          lastActive,
          length: compactions ? `${turns}, ${compactions}` : turns,
        })}
        className="grid gap-1 px-4 py-3 hover:bg-paper @2xl:grid-cols-[minmax(0,1fr)_11rem_8rem_9rem] @2xl:items-center @2xl:gap-4"
      >
        <span className="min-w-0">
          <span className="block truncate font-medium">{title}</span>
          {row.last_prompt && <span className="block truncate text-muted">{row.last_prompt}</span>}
        </span>
        {/* One wrapping line when the list is narrow; three columns when it's wide. */}
        <span className="flex flex-wrap gap-x-3 text-muted @2xl:contents">
          <span className="max-w-full min-w-0 truncate">{row.project_name ?? "–"}</span>
          <span>{lastActive}</span>
          <span className="flex flex-wrap gap-x-3 @2xl:flex-col">
            <span>{turns}</span>
            {compactions && (
              <span className="inline-flex items-center gap-1">
                {/* Amber marks the change; the words stay at text contrast (DESIGN §6). */}
                <Layers className="size-3.5 text-amber-strong" aria-hidden="true" />
                {compactions}
              </span>
            )}
          </span>
        </span>
      </Link>
    </li>
  );
}

export function Sessions() {
  const searchId = useId();
  const [query, setQuery] = useState("");
  const deferredQuery = useDeferredValue(query.trim());
  const scan = useScan();
  const sessions = useQuery({
    queryKey: ["sessions", deferredQuery],
    queryFn: () => api.sessions({ q: deferredQuery || undefined, limit: 500 }),
    placeholderData: keepPreviousData,
  });
  const rows = sessions.data ?? [];
  const errors = scan.status?.last?.errors.length ?? 0;
  const firstScan = scan.scanning && rows.length === 0 && !deferredQuery;
  usePageTitle(t.sessions.title);

  return (
    <div className="mx-auto max-w-6xl px-4 py-6 sm:px-8 sm:py-8">
      <header className="flex flex-wrap items-end justify-between gap-4">
        <div>
          <h1 className="font-display text-3xl font-semibold tracking-tight">{t.sessions.title}</h1>
          <p className="text-muted" role="status">
            {scan.scanning && scan.status && scan.status.total > 0
              ? `${t.sessions.reading} ${t.sessions.progress(scan.status.done, scan.status.total)}`
              : sessions.data
                ? t.sessions.count(rows.length)
                : ""}
          </p>
        </div>
        <div className="flex w-full flex-wrap items-center gap-2 sm:w-auto">
          <label htmlFor={searchId} className="sr-only">
            {t.sessions.search}
          </label>
          <div className="relative min-w-0 grow basis-48 sm:grow-0 sm:basis-auto">
            <Search
              className="pointer-events-none absolute top-2.5 left-3 size-4 text-muted"
              aria-hidden="true"
            />
            {/* A muted border, not a decorative one: it marks the field (WCAG 1.4.11). */}
            <input
              id={searchId}
              type="search"
              value={query}
              onChange={(event) => setQuery(event.target.value)}
              placeholder={t.sessions.searchPlaceholder}
              className="h-9 w-full rounded-control border border-muted bg-surface pr-3 pl-9 text-ink placeholder:text-muted sm:w-72"
            />
          </div>
          <Button variant="secondary" onClick={() => void scan.rescan()} disabled={scan.scanning}>
            <RefreshCw className={scan.scanning ? "animate-spin" : undefined} aria-hidden="true" />
            {t.sessions.refresh}
          </Button>
        </div>
      </header>

      {errors > 0 && (
        <p className="mt-4 flex items-center gap-2">
          <TriangleAlert className="size-4 shrink-0 text-amber-strong" aria-hidden="true" />
          {t.sessions.scanError(errors)}
        </p>
      )}
      {(sessions.isError || scan.failed) && (
        <p role="alert" className="mt-4 text-coral">
          {t.sessions.loadError}
        </p>
      )}

      <section className="mt-6" aria-label={t.sessions.title}>
        {sessions.isPending && (
          <div className="space-y-2">
            {[0, 1, 2].map((i) => (
              <Skeleton key={i} className="h-14" />
            ))}
          </div>
        )}

        {sessions.data && rows.length === 0 && (
          <Card className="flex flex-col items-center gap-4 px-6 py-12 text-center">
            <GraceMascot state={firstScan ? "thinking" : "sleeping"} size="card" />
            <p className="max-w-md text-base">
              {firstScan
                ? t.sessions.reading
                : deferredQuery
                  ? t.sessions.noMatch(deferredQuery)
                  : t.sessions.empty}
            </p>
          </Card>
        )}

        {rows.length > 0 && (
          <Card className="@container overflow-hidden">
            <div
              className="hidden grid-cols-[minmax(0,1fr)_11rem_8rem_9rem] gap-4 border-b border-line px-4 py-2 text-xs font-medium uppercase tracking-wide text-muted @2xl:grid"
              aria-hidden="true"
            >
              <span>{t.sessions.columns.session}</span>
              <span>{t.sessions.columns.project}</span>
              <span>{t.sessions.columns.lastActive}</span>
              <span>{t.sessions.columns.length}</span>
            </div>
            <ul className="divide-y divide-line">
              {rows.map((row) => (
                <Row key={row.id} row={row} />
              ))}
            </ul>
          </Card>
        )}
      </section>
    </div>
  );
}
