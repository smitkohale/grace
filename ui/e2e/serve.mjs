// Starts `grace serve` for end-to-end tests, against a throwaway home folder
// (with a space in its name) holding two synthetic Claude Code sessions, one
// of them compacted.
// Needs `cargo build -p grace-cli` and `pnpm build` first.
import { spawn } from "node:child_process";
import { cpSync, mkdirSync, mkdtempSync } from "node:fs";
import { tmpdir } from "node:os";
import { join, resolve } from "node:path";

const port = process.argv[2] ?? "4329";
const repo = resolve(import.meta.dirname, "../..");
const root = mkdtempSync(join(tmpdir(), "grace e2e "));
const home = join(root, "Test User");
const project = join(home, ".claude", "projects", "C--Users-Test-User-Projects-Maple---Moss");
mkdirSync(project, { recursive: true });
mkdirSync(join(root, "empty"));
cpSync(
  join(repo, "tests", "fixtures", "claude_code", "basic.jsonl"),
  join(project, "11111111-1111-4111-8111-111111111111.jsonl"),
);
cpSync(
  join(repo, "tests", "fixtures", "claude_code", "compaction.jsonl"),
  join(project, "22222222-2222-4222-8222-222222222222.jsonl"),
);

const exe = join(repo, "target", "debug", process.platform === "win32" ? "grace.exe" : "grace");
const child = spawn(exe, ["serve", "--port", port, "--token", "e2e-token"], {
  stdio: "inherit",
  env: {
    ...process.env,
    PATH: join(root, "empty"),
    GRACE_HOME: join(root, "grace"),
    GRACE_DISCOVERY_HOME: home,
    CLAUDE_CONFIG_DIR: join(home, ".claude"),
    CODEX_HOME: join(home, ".codex"),
    LOCALAPPDATA: join(root, "local"),
    GRACE_OLLAMA_ADDR: "127.0.0.1:9",
  },
});

for (const signal of ["SIGINT", "SIGTERM"]) {
  process.on(signal, () => {
    child.kill();
    process.exit(0);
  });
}
child.on("exit", (code) => process.exit(code ?? 0));
