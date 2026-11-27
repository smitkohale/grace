// Client for Grace's local API. In the desktop app the address and token come
// from the app itself; under `grace serve` the token arrives in the URL fragment.

export type ToolId = "claude_code" | "codex" | "antigravity" | "ollama" | "cursor";

export interface ToolStatus {
  tool: ToolId;
  name: string;
  found: boolean;
  version: string | null;
  data_dir: string | null;
  sessions: number | null;
  executable: string | null;
  readable_now: boolean;
  notes: string[];
}

export interface DoctorReport {
  grace_home: string;
  grace_home_exists: boolean;
  tools: ToolStatus[];
}

export interface Config {
  onboarding_done: boolean;
  redact_emails: boolean;
  tools: Record<string, { enabled: boolean }>;
}

export interface ScanReport {
  sessions_found: number;
  parsed: number;
  skipped: number;
  removed: number;
  events_added: number;
  unknown: Record<string, number>;
  errors: string[];
  duration_ms: number;
}

export interface ScanJob {
  running: boolean;
  done: number;
  total: number;
  current: string | null;
  last: ScanReport | null;
  error: string | null;
}

export interface SessionRow {
  id: number;
  tool: ToolId;
  session_id: string;
  title: string | null;
  project_path: string | null;
  project_name: string | null;
  started_at: string | null;
  last_active_at: string | null;
  turns: number;
  compactions: number;
  event_count: number;
  version: string | null;
  entrypoint: string | null;
  models: string[];
  source_file: string;
  first_prompt: string | null;
  last_prompt: string | null;
}

export type EventKind =
  | "user_message"
  | "assistant_message"
  | "tool_call"
  | "tool_result"
  | "request_snapshot"
  | "instruction_load"
  | "compaction"
  | "resume"
  | "model_switch"
  | "handoff"
  | "task_update"
  | "hook_run"
  | "unknown";

export type BoundaryKind = "session_start" | "compaction" | "resume" | "model_switch" | "handoff";

export interface EventRow {
  seq: number;
  kind: EventKind;
  label: string;
  line_no: number;
  turn: number | null;
  timestamp: string | null;
  model: string | null;
  summary: string;
  detail: Record<string, unknown>;
}

export interface BoundaryRow {
  kind: BoundaryKind;
  event_seq: number;
  line_no: number;
  timestamp: string | null;
  detail: Record<string, unknown>;
}

export interface SessionDetail {
  session: SessionRow;
  boundaries: BoundaryRow[];
  events: EventRow[];
  next_after: number | null;
}

export class ApiError extends Error {
  constructor(
    readonly status: number,
    message: string,
  ) {
    super(message);
  }
}

const TOKEN_KEY = "grace-token";

/** Moves `#token=…` from the address bar into session storage. */
export function captureTokenFromHash(): void {
  const match = window.location.hash.match(/token=([0-9A-Za-z_-]+)/);
  if (match?.[1]) {
    sessionStorage.setItem(TOKEN_KEY, match[1]);
    window.history.replaceState(null, "", `${window.location.pathname}#/`);
  }
}

interface ApiInfo {
  baseUrl: string;
  token: string;
}

let info: Promise<ApiInfo> | null = null;

async function resolveInfo(): Promise<ApiInfo> {
  if ("__TAURI_INTERNALS__" in window) {
    const { invoke } = await import("@tauri-apps/api/core");
    const fromApp = await invoke<{ base_url: string; token: string }>("api_info");
    return { baseUrl: fromApp.base_url, token: fromApp.token };
  }
  return { baseUrl: "", token: sessionStorage.getItem(TOKEN_KEY) ?? "" };
}

async function request<T>(path: string, init?: RequestInit): Promise<T> {
  info ??= resolveInfo();
  const { baseUrl, token } = await info;
  const response = await fetch(`${baseUrl}${path}`, {
    ...init,
    headers: {
      Authorization: `Bearer ${token}`,
      "Content-Type": "application/json",
      ...init?.headers,
    },
  });
  if (!response.ok) {
    throw new ApiError(response.status, await response.text());
  }
  return (await response.json()) as T;
}

export interface SessionsParams {
  q?: string;
  project?: string;
  limit?: number;
}

export const api = {
  doctor: () => request<DoctorReport>("/api/doctor"),
  config: () => request<Config>("/api/config"),
  saveConfig: (patch: Partial<Config>) =>
    request<Config>("/api/config", { method: "PUT", body: JSON.stringify(patch) }),
  startScan: () => request<{ started: boolean }>("/api/scan", { method: "POST" }),
  scanStatus: () => request<ScanJob>("/api/scan"),
  sessions: (params: SessionsParams = {}) => {
    const query = new URLSearchParams();
    if (params.q) query.set("q", params.q);
    if (params.project) query.set("project", params.project);
    if (params.limit) query.set("limit", String(params.limit));
    return request<SessionRow[]>(`/api/sessions?${query}`);
  },
  session: (id: number, after?: number) =>
    request<SessionDetail>(`/api/sessions/${id}${after === undefined ? "" : `?after=${after}`}`),
};
