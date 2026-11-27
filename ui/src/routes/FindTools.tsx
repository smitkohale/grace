import { useMutation, useQuery, useQueryClient } from "@tanstack/react-query";
import { useState } from "react";
import { useLocation } from "wouter";
import { OnboardingLayout } from "../components/OnboardingLayout";
import { ToolCard } from "../components/ToolCard";
import { Button } from "../components/ui/button";
import { Skeleton } from "../components/ui/skeleton";
import { api } from "../lib/api";
import { usePageTitle } from "../lib/usePageTitle";
import { GraceMascot } from "../mascot/GraceMascot";
import { t } from "../strings/en";

/** Onboarding step 2: Grace looks for AI tools, read-only (DESIGN §2). */
export function FindTools() {
  const [, navigate] = useLocation();
  const queryClient = useQueryClient();
  const doctor = useQuery({ queryKey: ["doctor"], queryFn: api.doctor });
  const config = useQuery({ queryKey: ["config"], queryFn: api.config });
  const [claudeChoice, setClaudeChoice] = useState<boolean | null>(null);
  const claudeEnabled = claudeChoice ?? config.data?.tools.claude_code?.enabled ?? true;
  usePageTitle(t.tools.pageTitle);

  const finish = useMutation({
    mutationFn: async () => {
      await api.saveConfig({
        onboarding_done: true,
        tools: { claude_code: { enabled: claudeEnabled } },
      });
      if (claudeEnabled) await api.startScan();
    },
    onSuccess: async () => {
      await queryClient.invalidateQueries({ queryKey: ["config"] });
      navigate("/sessions");
    },
  });

  return (
    <OnboardingLayout step={2} total={2}>
      <div className="flex items-center gap-6">
        <GraceMascot
          state={doctor.isPending ? "thinking" : "idle"}
          size="card"
          className="shrink-0"
        />
        <div>
          <h1 className="font-display text-3xl font-semibold tracking-tight">{t.tools.title}</h1>
          <p className="mt-2 text-muted">{t.tools.subtitle}</p>
        </div>
      </div>

      <section className="mt-8" aria-busy={doctor.isPending}>
        {/* One live region for the whole search, so the result is announced too. */}
        <p className="mb-3 text-muted" role="status">
          {doctor.isPending
            ? t.tools.looking
            : doctor.data
              ? t.tools.foundCount(
                  doctor.data.tools.filter((tool) => tool.found).length,
                  doctor.data.tools.length,
                )
              : ""}
        </p>
        {doctor.isPending && (
          <div className="grid gap-4 md:grid-cols-2">
            {[0, 1, 2, 3].map((i) => (
              <Skeleton key={i} className="h-32" />
            ))}
          </div>
        )}
        {doctor.isError && (
          <div role="alert" className="flex items-center gap-4">
            <p>{t.tools.error}</p>
            <Button variant="secondary" onClick={() => doctor.refetch()}>
              {t.tools.retry}
            </Button>
          </div>
        )}
        {doctor.data && (
          <div className="grid gap-4 md:grid-cols-2">
            {doctor.data.tools.map((tool) => (
              <ToolCard
                key={tool.tool}
                status={tool}
                enabled={tool.tool === "claude_code" ? claudeEnabled : undefined}
                onToggle={tool.tool === "claude_code" ? setClaudeChoice : undefined}
              />
            ))}
          </div>
        )}
      </section>

      <footer className="mt-8 flex items-center gap-3">
        <Button
          size="lg"
          onClick={() => finish.mutate()}
          disabled={!doctor.data || finish.isPending}
        >
          {t.tools.continue}
        </Button>
        <Button variant="ghost" size="lg" onClick={() => navigate("/welcome")}>
          {t.onboarding.back}
        </Button>
        {finish.isError && (
          <p role="alert" className="text-coral">
            {t.tools.saveError}
          </p>
        )}
      </footer>
    </OnboardingLayout>
  );
}
