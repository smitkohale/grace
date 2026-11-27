import { expect, test } from "@playwright/test";

test.describe.configure({ mode: "serial" });

test("first run: hello, find your tools, then your sessions", async ({ page }) => {
  await page.goto("/#token=e2e-token");
  await expect(page.getByRole("heading", { name: "Hi, I'm Grace." })).toBeVisible();
  await expect(page).not.toHaveURL(/token=/);

  await page.getByRole("button", { name: "Let's go" }).click();
  await expect(
    page.getByRole("heading", { name: "Let me look for the AI tools you use." }),
  ).toBeVisible();
  // Finding tools asks each installed tool for its version, which can take a
  // few seconds on a cold start.
  await expect(page.getByRole("heading", { name: "Claude Code" })).toBeVisible({
    timeout: 20_000,
  });
  await expect(page.getByRole("switch", { name: "Read Claude Code sessions" })).toBeChecked();
  await expect(page.getByRole("heading", { name: "Ollama" })).toBeVisible();

  await page.getByRole("button", { name: "Continue" }).click();
  const session = page.getByRole("link", { name: /Fix the sprite sheet/ });
  await expect(session).toBeVisible({ timeout: 20_000 });

  await session.click();
  await expect(page.getByRole("heading", { name: "Context changes" })).toBeVisible();
  await expect(page.getByText("Session started", { exact: true })).toBeVisible();
  await expect(page.getByText("Loaded CLAUDE.md (Project)")).toBeVisible();
});

test("the welcome step works with the keyboard alone", async ({ page }) => {
  await page.goto("/#/welcome");
  const start = page.getByRole("button", { name: "Let's go" });
  for (let i = 0; i < 4 && !(await start.evaluate((el) => el === document.activeElement)); i++) {
    await page.keyboard.press("Tab");
  }
  await expect(start).toBeFocused();
  const outline = await start.evaluate((el) => getComputedStyle(el).outlineStyle);
  expect(outline).not.toBe("none");
  await page.keyboard.press("Enter");
  await expect(
    page.getByRole("heading", { name: "Let me look for the AI tools you use." }),
  ).toBeVisible();
});
