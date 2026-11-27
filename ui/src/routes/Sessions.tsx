import { keepPreviousData, useQuery } from "@tanstack/react-query";
import { Layers, RefreshCw, Search } from "lucide-react";
import { useDeferredValue, useId, useState } from "react";
import { Link } from "wouter";
import { Button } from "../components/ui/button";
import { Card } from "../components/ui/card";
import { Skeleton } from "../components/ui/skeleton";
import { api, type SessionRow } from "../lib/api";
import { relativeTime } from "../lib/time";
import { useScan } from "../lib/useScan";
import { GraceMascot } from "../mascot/GraceMascot";
import { t } from "../strings/en";

function Row({ row }: { row: SessionRow }) {
  return (
    <li>
      <Link
        href={`/sessions/${row.id}`}
        className="grid grid-cols-[minmax(0,1fr)_11rem_8rem_9rem] items-center gap-4 px-4 py-3 hover:bg-paper"
      >
        <span className="min-w-0">
          <span className="block truncate font-medium">{row.title ?? t.sessions.untitled}</span>
          {row.last_prompt && <span className="block truncate text-muted">{row.last_prompt}</span>}
        </span>
        <span className="truncate text-muted">{row.project_name ?? "–"}</span>
        <span className="text-muted">{relativeTime(row.last_active_at)}</span>
        <span className="flex flex-col text-muted">
          <span>{t.sessions.turns(row.turns)}</span>
          {row.compactions > 0 && (
            <span className="inline-flex items-center gap-1 text-amber-strong">
              <Layers className="size-3.5" aria-hidden="true" />
              {t.sessions.compactions(row.compactions)}
            </span>
          )}
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

  return (
    <div className="mx-auto max-w-6xl px-8 py-8">
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
        <div className="flex items-center gap-2">
          <label htmlFor={searchId} className="sr-only">
            {t.sessions.search}
          </label>
          <div className="relative">
            <Search
              className="pointer-events-none absolute top-2.5 left-3 size-4 text-muted"
              aria-hidden="true"
            />
            <input
              id={searchId}
              type="search"
              value={query}
              onChange={(event) => setQuery(event.target.value)}
              placeholder={t.sessions.searchPlaceholder}
              className="h-9 w-72 rounded-control border border-line bg-surface pr-3 pl-9 text-ink placeholder:text-muted"
            />
          </div>
          <Button variant="secondary" onClick={() => void scan.rescan()} disabled={scan.scanning}>
            <RefreshCw className={scan.scanning ? "animate-spin" : undefined} aria-hidden="true" />
            {t.sessions.refresh}
          </Button>
        </div>
      </header>

      {errors > 0 && <p className="mt-4 text-amber-strong">{t.sessions.scanError(errors)}</p>}
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
          <Card className="overflow-hidden">
            <div
              className="grid grid-cols-[minmax(0,1fr)_11rem_8rem_9rem] gap-4 border-b border-line px-4 py-2 text-xs font-medium uppercase tracking-wide text-muted"
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
