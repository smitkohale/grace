// Light and dark themes. Grace follows the system unless the user picks one.

export type ThemeChoice = "system" | "light" | "dark";

const KEY = "grace-theme";
const darkQuery = () => window.matchMedia?.("(prefers-color-scheme: dark)");

export function themeChoice(): ThemeChoice {
  const saved = localStorage.getItem(KEY);
  return saved === "light" || saved === "dark" ? saved : "system";
}

export function applyTheme(choice: ThemeChoice = themeChoice()): void {
  const dark = choice === "dark" || (choice === "system" && Boolean(darkQuery()?.matches));
  document.documentElement.dataset.theme = dark ? "dark" : "light";
}

export function setThemeChoice(choice: ThemeChoice): void {
  if (choice === "system") localStorage.removeItem(KEY);
  else localStorage.setItem(KEY, choice);
  applyTheme(choice);
}

export function initTheme(): void {
  applyTheme();
  darkQuery()?.addEventListener("change", () => applyTheme());
}
