import { useEffect } from "react";
import { t } from "../strings/en";

/** Names the screen in the window title, which screen readers announce (WCAG 2.4.2). */
export function usePageTitle(screen: string | undefined): void {
  useEffect(() => {
    document.title = screen ? t.pageTitle(screen) : t.appName;
  }, [screen]);
}
