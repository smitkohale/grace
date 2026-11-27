import { useMutation, useQueryClient } from "@tanstack/react-query";
import type { ReactNode } from "react";
import { useLocation } from "wouter";
import { api } from "../lib/api";
import { t } from "../strings/en";
import { Button } from "./ui/button";

/** First-run steps: each one is skippable (DESIGN §2). */
export function OnboardingLayout({
  step,
  total,
  children,
}: {
  step: number;
  total: number;
  children: ReactNode;
}) {
  const [, navigate] = useLocation();
  const queryClient = useQueryClient();
  const skip = useMutation({
    mutationFn: () => api.saveConfig({ onboarding_done: true }),
    onSettled: async () => {
      await queryClient.invalidateQueries({ queryKey: ["config"] });
      navigate("/sessions");
    },
  });

  return (
    <div className="min-h-screen bg-paper">
      <header className="mx-auto flex max-w-5xl items-center justify-between px-8 pt-6 text-muted">
        <p>{t.onboarding.step(step, total)}</p>
        <Button variant="link" onClick={() => skip.mutate()} disabled={skip.isPending}>
          {t.onboarding.skip}
        </Button>
      </header>
      <main className="mx-auto max-w-5xl px-8 pb-12 pt-6">{children}</main>
    </div>
  );
}
