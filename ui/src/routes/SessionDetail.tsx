import { useInfiniteQuery } from "@tanstack/react-query";
import {
  ArrowLeft,
  Bot,
  Camera,
  CircleHelp,
  CornerDownLeft,
  FileText,
  Flag,
  Layers,
  ListChecks,
  type LucideIcon,
  MessageSquare,
  RotateCcw,
  Shuffle,
  Webhook,
  Wrench,
} from "lucide-react";
import { Link } from "wouter";
import { Button } from "../components/ui/button";
import { Card } from "../components/ui/card";
import { Skeleton } from "../components/ui/skeleton";
import { ApiError, api, type BoundaryKind, type BoundaryRow, type EventKind } from "../lib/api";
import { dateTime, relativeTime } from "../lib/time";
import { t } from "../strings/en";

const EVENT_ICON: Record<EventKind, LucideIcon> = {
  user_message: MessageSquare,
  assistant_message: Bot,
  tool_call: Wrench,
  tool_result: CornerDownLeft,
  request_snapshot: Camera,
  instruction_load: FileText,
  compaction: Layers,
  resume: RotateCcw,
  model_switch: Shuffle,
  handoff: RotateCcw,
  task_update: ListChecks,
  hook_run: Webhook,
  unknown: CircleHelp,
};

const BOUNDARY_ICON: Record<BoundaryKind, LucideIcon> = {
  session_start: Flag,
  compaction: Layers,
  resume: RotateCcw,
  model_switch: Shuffle,
  handoff: RotateCcw,
};

function boundaryDetail(boundary: BoundaryRow): string | null {
  const d = boundary.detail;
  switch (boundary.kind) {
    case "compaction":
      return t.detail.compaction(
        String(d.trigger ?? "auto"),
        typeof d.pre_tokens === "number" ? d.pre_tokens : undefined,
        typeof d.post_tokens === "number" ? d.post_tokens : undefined,
      );
    case "model_switch":
      return `${String(d.from)} → ${String(d.to)}`;
    default:
      return null;
  }
}

export function SessionDetail({ id }: { id: number }) {
  const detail = useInfiniteQuery({
    queryKey: ["session", id],
    queryFn: ({ pageParam }) => api.session(id, pageParam),
    initialPageParam: undefined as number | undefined,
    getNextPageParam: (page) => page.next_after ?? undefined,
  });

  const first = detail.data?.pages[0];
  const session = first?.session;
  const events = detail.data?.pages.flatMap((page) => page.events) ?? [];
  const notFound = detail.error instanceof ApiError && detail.error.status === 404;

  return (
    <div className="mx-auto max-w-5xl px-8 py-8">
      <Link
        href="/sessions"
        className="inline-flex items-center gap-1 text-teal-strong hover:underline"
      >
        <ArrowLeft className="size-4" aria-hidden="true" />
        {t.detail.back}
      </Link>

      {detail.isPending && (
        <div className="mt-6 space-y-3">
          <Skeleton className="h-9 w-2/3" />
          <Skeleton className="h-5 w-1/2" />
          <Skeleton className="h-40" />
        </div>
      )}
      {detail.isError && (
        <p role="alert" className="mt-6 text-coral">
          {notFound ? t.detail.notFound : t.sessions.loadError}
        </p>
      )}

      {session && first && (
        <>
          <header className="mt-4">
            <h1 className="font-display text-3xl font-semibold tracking-tight">
              {session.title ?? t.sessions.untitled}
            </h1>
            <p className="mt-1 text-muted">
              {[
                session.project_name,
                session.last_active_at
                  ? `${t.detail.lastActive} ${relativeTime(session.last_active_at)}`
                  : null,
                t.sessions.turns(session.turns),
                session.models.join(", ") || null,
                session.version ? t.detail.version(session.version) : null,
              ]
                .filter(Boolean)
                .join(" · ")}
            </p>
            <details className="mt-2 text-muted">
              <summary className="cursor-pointer">{t.detail.source}</summary>
              <p className="mt-1 font-mono text-xs break-all">{session.source_file}</p>
            </details>
          </header>

          <section className="mt-8" aria-labelledby="context-changes">
            <h2 id="context-changes" className="font-display text-lg font-semibold">
              {t.detail.contextChanges}
            </h2>
            <p className="text-muted">{t.detail.contextChangesHelp}</p>
            <ol className="mt-3 space-y-2">
              {first.boundaries.map((boundary) => {
                const Icon = BOUNDARY_ICON[boundary.kind];
                const extra = boundaryDetail(boundary);
                return (
                  <li key={`${boundary.kind}-${boundary.event_seq}-${boundary.line_no}`}>
                    <Card className="flex items-center gap-3 px-4 py-3">
                      <Icon
                        className={
                          boundary.kind === "compaction"
                            ? "size-5 text-amber-strong"
                            : "size-5 text-teal-strong"
                        }
                        aria-hidden="true"
                      />
                      <span className="font-medium">{t.detail.boundary[boundary.kind]}</span>
                      {extra && <span className="text-muted">{extra}</span>}
                      <span className="ml-auto font-mono text-xs text-muted">
                        {dateTime(boundary.timestamp)} · {t.detail.line(boundary.line_no)}
                      </span>
                    </Card>
                  </li>
                );
              })}
            </ol>
          </section>

          <section className="mt-8" aria-labelledby="what-happened">
            <h2 id="what-happened" className="font-display text-lg font-semibold">
              {t.detail.events}
            </h2>
            <Card className="mt-3 overflow-hidden">
              <ol className="divide-y divide-line">
                {events.map((event) => {
                  const Icon = EVENT_ICON[event.kind] ?? CircleHelp;
                  return (
                    <li key={event.seq} className="flex items-start gap-3 px-4 py-2.5">
                      <Icon
                        className="mt-0.5 size-4 shrink-0 text-teal-strong"
                        aria-hidden="true"
                      />
                      <span className="min-w-0 flex-1 break-words">{event.summary}</span>
                      <span className="shrink-0 font-mono text-xs text-muted">
                        {event.turn !== null && `${t.detail.turn(event.turn)} · `}
                        {t.detail.line(event.line_no)}
                      </span>
                    </li>
                  );
                })}
              </ol>
            </Card>
            {detail.hasNextPage && (
              <Button
                variant="secondary"
                className="mt-3"
                onClick={() => void detail.fetchNextPage()}
                disabled={detail.isFetchingNextPage}
              >
                {t.detail.more}
              </Button>
            )}
          </section>
        </>
      )}
    </div>
  );
}
