// R16: everything Grace ships must be free, under permissive open-source licenses.
// Checks the UI's production dependencies (dev tools aren't shipped).
import { execSync } from "node:child_process";

const ALLOWED = new Set([
  "MIT",
  "MIT-0",
  "Apache-2.0",
  "BSD-2-Clause",
  "BSD-3-Clause",
  "ISC",
  "0BSD",
  "CC0-1.0",
  "Zlib",
  "OFL-1.1",
  "BlueOak-1.0.0",
  "Unicode-3.0",
  "Unlicense",
  "MPL-2.0",
]);

function allowed(expression) {
  const clean = expression.replace(/[()]/g, " ").trim();
  if (clean.includes(" OR ")) return clean.split(" OR ").some((part) => allowed(part));
  if (clean.includes(" AND ")) return clean.split(" AND ").every((part) => allowed(part));
  return ALLOWED.has(clean);
}

const output = execSync("pnpm -C ui licenses list --prod --json", { encoding: "utf8" });
const byLicense = JSON.parse(output);
let bad = 0;
for (const [license, packages] of Object.entries(byLicense)) {
  const names = packages.map((p) => `${p.name}@${p.versions?.join(",") ?? p.version ?? ""}`);
  if (allowed(license)) {
    console.log(`ok   ${license}: ${names.length} package(s)`);
  } else {
    bad += names.length;
    console.log(`DENY ${license}: ${names.join(", ")}`);
  }
}
process.exit(bad ? 1 : 0);
