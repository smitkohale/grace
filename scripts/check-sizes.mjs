// Size budgets from docs/GRACE_SPEC.md section 11.
//   node scripts/check-sizes.mjs cli <path-to-grace-binary>     (15 MB)
//   node scripts/check-sizes.mjs installers <target-dir>        (20 MB each; AppImage reported only)
import { existsSync, readdirSync, statSync } from "node:fs";
import { join } from "node:path";

const MB = 1024 * 1024;
const [mode, path] = process.argv.slice(2);
const report = (file, size, budget) => {
  const ok = size <= budget;
  console.log(`${ok ? "ok  " : "OVER"} ${(size / MB).toFixed(2).padStart(6)} MB / ${budget / MB} MB  ${file}`);
  return ok;
};

let ok = true;
if (mode === "cli") {
  const file = existsSync(path) ? path : `${path}.exe`;
  ok = report(file, statSync(file).size, 15 * MB);
} else if (mode === "installers") {
  const found = [];
  const walk = (dir) => {
    for (const entry of readdirSync(dir, { withFileTypes: true })) {
      const full = join(dir, entry.name);
      if (entry.isDirectory()) {
        if (!["deps", "build", "incremental", ".fingerprint"].includes(entry.name)) walk(full);
      } else if (full.includes("bundle") && /\.(exe|msi|dmg|AppImage|deb)$/.test(entry.name)) {
        found.push(full);
      }
    }
  };
  walk(path);
  if (found.length === 0) {
    console.error(`No installers found under ${path}`);
    process.exit(1);
  }
  for (const file of found) {
    const size = statSync(file).size;
    if (file.endsWith(".AppImage")) {
      // An AppImage carries its own copy of the Linux web engine (WebKitGTK),
      // so the budget can't apply. Report it so growth stays visible.
      console.log(`info ${(size / MB).toFixed(2).padStart(6)} MB  ${file} (bundles WebKitGTK; not budgeted)`);
      continue;
    }
    ok = report(file, size, 20 * MB) && ok;
  }
} else {
  console.error("usage: check-sizes.mjs cli <binary> | installers <target-dir>");
  process.exit(2);
}
process.exit(ok ? 0 : 1);
