import { expect, type Locator, type Page, test } from "@playwright/test";
import { contrastProblems } from "./contrast";

// WCAG 2.2 AA checks for every M0 screen (AGENTS.md R5, DESIGN.md §1).
// Keyboard-only use of the first run is covered in onboarding.spec.ts.

const auth = { Authorization: "Bearer e2e-token" };
const spriteSheet = (page: Page) => page.getByRole("link", { name: /Fix the sprite sheet/ });

async function open(page: Page, hash: string) {
  await page.goto("/#token=e2e-token");
  await page.evaluate((target) => {
    location.hash = target;
  }, hash);
}

const screens: {
  name: string;
  go: (page: Page) => Promise<void>;
  ready: (page: Page) => Locator;
}[] = [
  {
    name: "Hello",
    go: (page) => open(page, "#/welcome"),
    ready: (page) => page.getByRole("heading", { name: "Hi, I'm Grace." }),
  },
  {
    name: "Find your tools",
    go: (page) => open(page, "#/welcome/tools"),
    ready: (page) => page.getByRole("switch", { name: "Read Claude Code sessions" }),
  },
  {
    name: "Sessions",
    go: (page) => open(page, "#/sessions"),
    ready: (page) => page.getByText("1 compaction"),
  },
  {
    name: "Session",
    go: async (page) => {
      await open(page, "#/sessions");
      await spriteSheet(page).click({ timeout: 20_000 });
    },
    ready: (page) => page.getByRole("heading", { name: "Context changes" }),
  },
];

// The fixture sessions are read through the API, so onboarding stays
// unfinished for onboarding.spec.ts whichever file runs first.
test.beforeAll(async ({ request }) => {
  await request.post("/api/scan", { headers: auth });
  await expect
    .poll(
      async () => {
        const response = await request.get("/api/sessions", { headers: auth });
        return ((await response.json()) as unknown[]).length;
      },
      { timeout: 30_000 },
    )
    .toBe(2);
});

for (const colorScheme of ["light", "dark"] as const) {
  test(`text and inputs have enough contrast (${colorScheme})`, async ({ page }) => {
    await page.emulateMedia({ colorScheme });
    const found: Record<string, string[]> = {};
    for (const screen of screens) {
      await screen.go(page);
      await expect(screen.ready(page)).toBeVisible({ timeout: 20_000 });
      const problems = await page.evaluate(contrastProblems);
      if (problems.length > 0) found[screen.name] = problems;
    }
    expect(found).toEqual({});
  });
}

// 480 px is the app's smallest window at 200% zoom; 320 px is WCAG's reflow
// test (a 1280 px window at 400%).
for (const width of [480, 320]) {
  test(`every screen fits ${width} px without sideways scrolling`, async ({ page }) => {
    await page.setViewportSize({ width, height: 640 });
    const overflows: Record<string, number> = {};
    for (const screen of screens) {
      await screen.go(page);
      await expect(screen.ready(page)).toBeAttached({ timeout: 20_000 });
      const overflow = await page.evaluate(
        () => document.documentElement.scrollWidth - document.documentElement.clientWidth,
      );
      if (overflow > 0) overflows[screen.name] = overflow;
    }
    expect(overflows, "pixels each screen scrolls sideways").toEqual({});
    await open(page, "#/sessions");
    const row = spriteSheet(page);
    await expect(row.getByText("Fix the sprite sheet", { exact: true })).toBeInViewport();
    await expect(row.getByText("2 turns", { exact: true })).toBeInViewport();
  });
}

test("Skip to content moves focus to the main content", async ({ page }) => {
  await open(page, "#/sessions");
  await expect(spriteSheet(page)).toBeVisible({ timeout: 20_000 });
  await page.keyboard.press("Tab");
  await expect(page.getByRole("button", { name: "Skip to content" })).toBeFocused();
  await page.keyboard.press("Enter");
  await expect(page.getByRole("main")).toBeFocused();
  await expect(page).toHaveURL(/#\/sessions$/);
});

test("each screen has its own title", async ({ page }) => {
  const titles: Record<string, string> = {};
  for (const screen of screens) {
    await screen.go(page);
    await expect(screen.ready(page)).toBeVisible({ timeout: 20_000 });
    titles[screen.name] = await page.title();
  }
  expect(titles).toEqual({
    Hello: "Hello · Grace",
    "Find your tools": "Find your tools · Grace",
    Sessions: "Sessions · Grace",
    Session: "Fix the sprite sheet · Grace",
  });
});

test("each session in the list reads clearly to screen readers", async ({ page }) => {
  await open(page, "#/sessions");
  await expect(spriteSheet(page)).toHaveAccessibleName(
    /^Fix the sprite sheet\. Last prompt: Now add a test\. Project: Maple & Moss\. Last active [^.]+\. 2 turns$/,
    { timeout: 20_000 },
  );
});

test("Find your tools says what it found", async ({ page }) => {
  await open(page, "#/welcome/tools");
  await expect(page.getByRole("status")).toHaveText("Found 1 of 5 tools.", { timeout: 20_000 });
});

test("with reduced motion, Grace holds still", async ({ page }) => {
  await page.addInitScript(() => {
    const counted = window as Window & { frames_requested?: number };
    counted.frames_requested = 0;
    const request = window.requestAnimationFrame.bind(window);
    window.requestAnimationFrame = (callback) => {
      counted.frames_requested = (counted.frames_requested ?? 0) + 1;
      return request(callback);
    };
  });
  await page.emulateMedia({ reducedMotion: "reduce" });
  await open(page, "#/welcome");
  await expect(page.getByRole("heading", { name: "Hi, I'm Grace." })).toBeVisible();
  // Live Grace replaces her still picture when WebGL works; without WebGL
  // she stays a picture, which holds still anyway.
  await page
    .locator(".mascot-card img")
    .waitFor({ state: "detached", timeout: 10_000 })
    .catch(() => {});
  const frames = () =>
    page.evaluate(() => (window as Window & { frames_requested?: number }).frames_requested ?? 0);
  const before = await frames();
  await page.waitForTimeout(1_500);
  expect(await frames()).toBe(before);
});
