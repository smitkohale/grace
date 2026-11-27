import { fireEvent, screen } from "@testing-library/react";
import { describe, expect, it, vi } from "vitest";
import type { DoctorReport } from "../lib/api";
import { renderWithQuery } from "../test/render";
import { FindTools } from "./FindTools";

const { report } = vi.hoisted(() => {
  const tool = (over: Partial<DoctorReport["tools"][number]>) => ({
    tool: "claude_code" as const,
    name: "Claude Code",
    found: true,
    version: null,
    data_dir: null,
    sessions: null,
    executable: null,
    readable_now: false,
    notes: [],
    ...over,
  });
  const report: DoctorReport = {
    grace_home: "C:\\Users\\Test User\\.grace",
    grace_home_exists: false,
    tools: [
      tool({
        version: "2.1.330",
        sessions: 3,
        data_dir: "C:\\Users\\Test User\\.claude\\projects",
        readable_now: true,
      }),
      tool({
        tool: "codex",
        name: "Codex",
        sessions: 11,
        data_dir: "C:\\Users\\Test User\\.codex\\sessions",
      }),
      tool({ tool: "ollama", name: "Ollama", found: false }),
    ],
  };
  return { report };
});

vi.mock("../lib/api", async (importOriginal) => {
  const actual = await importOriginal<typeof import("../lib/api")>();
  return {
    ...actual,
    api: {
      ...actual.api,
      doctor: vi.fn(async () => report),
      config: vi.fn(async () => ({ onboarding_done: false, redact_emails: true, tools: {} })),
    },
  };
});
vi.mock("../mascot/GraceMascot", () => ({ GraceMascot: () => <div data-testid="grace" /> }));

describe("FindTools", () => {
  it("shows one card per tool, naming the folder Grace reads", async () => {
    renderWithQuery(<FindTools />);
    expect(await screen.findByRole("heading", { name: "Claude Code" })).toBeInTheDocument();
    expect(screen.getByRole("heading", { name: "Codex" })).toBeInTheDocument();
    expect(screen.getByRole("heading", { name: "Ollama" })).toBeInTheDocument();
    expect(screen.getByText("C:\\Users\\Test User\\.claude\\projects")).toBeInTheDocument();
    expect(screen.getByText("Version 2.1.330 · 3 sessions")).toBeInTheDocument();
  });

  it("lets people turn Claude Code reading off", async () => {
    renderWithQuery(<FindTools />);
    const toggle = await screen.findByRole("switch", { name: "Read Claude Code sessions" });
    expect(toggle).toHaveAttribute("aria-checked", "true");
    fireEvent.click(toggle);
    expect(toggle).toHaveAttribute("aria-checked", "false");
  });

  it("says plainly what isn't found or isn't supported yet", async () => {
    renderWithQuery(<FindTools />);
    expect(await screen.findByText("Not found")).toBeInTheDocument();
    expect(screen.getByText("Found. Supported in a later update.")).toBeInTheDocument();
    expect(screen.getAllByRole("switch")).toHaveLength(1);
  });
});
