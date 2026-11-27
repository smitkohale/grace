import { Check, Minus } from "lucide-react";
import { useId } from "react";
import type { ToolStatus } from "../lib/api";
import { cn } from "../lib/cn";
import { t } from "../strings/en";
import { Card } from "./ui/card";
import { Switch } from "./ui/switch";

export function ToolCard({
  status,
  enabled,
  onToggle,
}: {
  status: ToolStatus;
  enabled?: boolean;
  onToggle?: (enabled: boolean) => void;
}) {
  const switchId = useId();
  const headingId = useId();
  const canRead = status.found && status.readable_now;

  return (
    <Card className="flex flex-col gap-3 p-4">
      <article aria-labelledby={headingId} className="contents">
        <div className="flex items-start justify-between gap-3">
          <div>
            <h2 id={headingId} className="font-display text-base font-semibold">
              {status.name}
            </h2>
            {status.found && (status.version || status.sessions !== null) && (
              <p className="text-muted">
                {[
                  status.version ? t.tools.version(status.version) : null,
                  status.sessions !== null ? t.tools.sessions(status.sessions) : null,
                ]
                  .filter(Boolean)
                  .join(" · ")}
              </p>
            )}
          </div>
          <span
            className={cn(
              "inline-flex items-center gap-1 rounded-full px-2 py-0.5 text-xs font-medium",
              // Light tints keep the small label at 4.5:1 or more (WCAG 1.4.3).
              status.found ? "bg-teal/10 text-teal-strong" : "bg-line/60 text-muted",
            )}
          >
            {status.found ? (
              <Check className="size-3.5" aria-hidden="true" />
            ) : (
              <Minus className="size-3.5" aria-hidden="true" />
            )}
            {status.found ? t.tools.found : t.tools.notFound}
          </span>
        </div>

        {status.found && status.data_dir && (
          <div>
            <p className="text-xs font-medium uppercase tracking-wide text-muted">
              {t.tools.willRead}
            </p>
            <p className="font-mono text-xs break-all text-ink">{status.data_dir}</p>
          </div>
        )}

        {canRead && onToggle && (
          <div className="mt-auto flex items-center gap-3 pt-1">
            <Switch id={switchId} checked={enabled} onCheckedChange={onToggle} />
            <label htmlFor={switchId} className="cursor-pointer">
              {t.tools.toggle(status.name)}
            </label>
          </div>
        )}
        {status.found && !status.readable_now && (
          <p className="mt-auto text-muted">{t.tools.laterUpdate}</p>
        )}
      </article>
    </Card>
  );
}
