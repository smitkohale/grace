import { screen } from "@testing-library/react";
import { beforeEach, describe, expect, it, vi } from "vitest";
import { api, type SessionRow } from "../lib/api";
import { renderWithQuery } from "../test/render";
import { Sessions } from "./Sessions";

vi.mock("../lib/api", async (importOriginal) => {
  const actual = await importOriginal<typeof import("../lib/api")>();
  return {
    ...actual,
    api: {
      ...actual.api,
      sessions: vi.fn(),
      startScan: vi.fn(async () => ({ started: true })),
      scanStatus: vi.fn(async () => ({
        running: false,
        done: 0,
        total: 0,
        current: null,
        last: null,
        error: null,
      })),
    },
  };
});
vi.mock("../mascot/GraceMascot", () => ({ GraceMascot: () => <div data-testid="grace" /> }));

const row: SessionRow = {
  id: 1,
  tool: "claude_code",
  session_id: "11111111",
  title: "Fix the sprite sheet",
  project_path: "C:\\Users\\Test User\\Projects\\Maple & Moss",
  project_name: "Maple & Moss",
  started_at: "2026-11-23T07:45:00.000Z",
  last_active_at: "2026-11-23T07:46:41.000Z",
  turns: 2,
  compactions: 1,
  event_count: 17,
  version: "2.1.330",
  entrypoint: "claude-desktop",
  models: ["claude-opus-5-5"],
  source_file: "C:\\x.jsonl",
  first_prompt: "Fix the sprite sheet loader",
  last_prompt: "Now add a test",
};

describe("Sessions", () => {
  beforeEach(() => vi.mocked(api.sessions).mockReset());

  it("teaches the next step when there are no sessions", async () => {
    vi.mocked(api.sessions).mockResolvedValue([]);
    renderWithQuery(<Sessions />);
    expect(
      await screen.findByText("No sessions yet. Start a chat in Claude Code and I'll pick it up."),
    ).toBeInTheDocument();
  });

  it("lists sessions with title, project, length and compactions", async () => {
    vi.mocked(api.sessions).mockResolvedValue([row]);
    renderWithQuery(<Sessions />);
    expect(await screen.findByText("Fix the sprite sheet")).toBeInTheDocument();
    expect(screen.getByText("Maple & Moss")).toBeInTheDocument();
    expect(screen.getByText("2 turns")).toBeInTheDocument();
    expect(screen.getByText("1 compaction")).toBeInTheDocument();
    expect(screen.getByRole("link", { name: /Fix the sprite sheet/ })).toHaveAttribute(
      "href",
      "/sessions/1",
    );
  });
});
