// Renders Grace's still images from the live 3D scene, so stills and 3D match:
//   assets/brand/mascot/renders/grace-{512,256,128}.webp  (transparent; fallbacks, README)
//   assets/brand/mascot/renders/grace-icon-1024.png       (head on her warm card; app icon)
// Run after `pnpm build`:  node scripts/render-stills.mjs
import { createReadStream, mkdirSync, statSync, writeFileSync } from "node:fs";
import { createServer } from "node:http";
import { extname, join, resolve } from "node:path";
import { chromium } from "@playwright/test";

const dist = resolve(import.meta.dirname, "../dist");
const out = resolve(import.meta.dirname, "../../assets/brand/mascot/renders");
mkdirSync(out, { recursive: true });

const types = {
  ".html": "text/html",
  ".js": "text/javascript",
  ".css": "text/css",
  ".glb": "model/gltf-binary",
  ".woff2": "font/woff2",
  ".svg": "image/svg+xml",
};
const server = createServer((req, res) => {
  const path = join(dist, decodeURIComponent(new URL(req.url, "http://x").pathname));
  try {
    const file = statSync(path).isFile() ? path : join(dist, "index.html");
    res.writeHead(200, { "Content-Type": types[extname(file)] ?? "application/octet-stream" });
    createReadStream(file).pipe(res);
  } catch {
    res.writeHead(404).end();
  }
});
await new Promise((ok) => server.listen(0, "127.0.0.1", ok));
const { port } = server.address();

const browser = await chromium.launch({ channel: process.env.CI ? undefined : "msedge" });
const page = await browser.newPage({
  viewport: { width: 1100, height: 760 },
  deviceScaleFactor: 3,
});
await page.emulateMedia({ reducedMotion: "reduce" }); // a still pose: no blink, no bob
await page.goto(`http://127.0.0.1:${port}/#/welcome`);
await page.addStyleTag({
  content:
    "html, body, #root, main, div, .mascot-card { background: transparent !important; box-shadow: none !important; }",
});
await page.waitForTimeout(3000);
const canvas = page.locator(".mascot-card canvas");
const png = await canvas.screenshot({ omitBackground: true });

// Resize and encode in the page (the browser has a WebP encoder; no extra tools needed).
const files = await page.evaluate(async (pngBase64) => {
  const image = new Image();
  image.src = `data:image/png;base64,${pngBase64}`;
  await image.decode();
  const encode = (canvas, type, quality) => canvas.toDataURL(type, quality).split(",")[1];
  const result = {};
  for (const size of [512, 256, 128]) {
    const c = document.createElement("canvas");
    c.width = size;
    c.height = size;
    const g = c.getContext("2d");
    g.imageSmoothingQuality = "high";
    g.drawImage(image, 0, 0, size, size);
    result[`grace-${size}.webp`] = encode(c, "image/webp", 0.86);
  }
  // App icon: her head and bow on a rounded square with the mascot gradient.
  const icon = document.createElement("canvas");
  icon.width = 1024;
  icon.height = 1024;
  const g = icon.getContext("2d");
  const r = 200;
  g.beginPath();
  g.roundRect(32, 32, 960, 960, r);
  const gradient = g.createLinearGradient(0, 32, 0, 992);
  gradient.addColorStop(0, "#DCD1C8");
  gradient.addColorStop(1, "#F3E5DB");
  g.fillStyle = gradient;
  g.fill();
  g.save();
  g.clip();
  // Her head, from the top strand down to the collar, with the bow.
  const s = image.width;
  g.drawImage(image, s * 0.04, s * 0.08, s * 0.78, s * 0.612, 82, 183, 860, 675);
  g.restore();
  result["grace-icon-1024.png"] = encode(icon, "image/png");
  return result;
}, png.toString("base64"));

for (const [name, base64] of Object.entries(files)) {
  const bytes = Buffer.from(base64, "base64");
  writeFileSync(join(out, name), bytes);
  console.log(`${name}  ${(bytes.length / 1024).toFixed(1)} KB`);
}
await browser.close();
server.close();
