import { t } from "../strings/en";

/** "5 minutes ago", "yesterday", "3 days ago" … for an RFC 3339 timestamp. */
export function relativeTime(iso: string | null | undefined, now: number = Date.now()): string {
  if (!iso) return "";
  const then = Date.parse(iso);
  if (Number.isNaN(then)) return "";
  const minutes = Math.floor((now - then) / 60_000);
  if (minutes < 1) return t.time.justNow;
  if (minutes < 60) return t.time.minutes(minutes);
  if (minutes < 120) return t.time.hours(1);
  if (minutes < 1440) return t.time.hours(Math.floor(minutes / 60));
  if (minutes < 2880) return t.time.yesterday;
  if (minutes < 43_200) return t.time.days(Math.floor(minutes / 1440));
  return t.time.months(Math.max(1, Math.floor(minutes / 43_200)));
}

/** A readable local date and time, e.g. "23 Nov 2026, 13:15". */
export function dateTime(iso: string | null | undefined): string {
  if (!iso) return "";
  const date = new Date(iso);
  if (Number.isNaN(date.getTime())) return "";
  return date.toLocaleString(undefined, { dateStyle: "medium", timeStyle: "short" });
}
