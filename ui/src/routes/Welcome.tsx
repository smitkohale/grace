import { useLocation } from "wouter";
import { OnboardingLayout } from "../components/OnboardingLayout";
import { Button } from "../components/ui/button";
import { GraceMascot } from "../mascot/GraceMascot";
import { t } from "../strings/en";

/** Onboarding step 1: Grace waves hello (DESIGN §2). */
export function Welcome() {
  const [, navigate] = useLocation();
  return (
    <OnboardingLayout step={1} total={2}>
      <div className="grid items-center gap-10 md:grid-cols-[minmax(0,1fr)_minmax(0,1.1fr)]">
        <GraceMascot state="hello" size="hero" className="mx-auto" />
        <div>
          <h1 className="font-display text-4xl font-semibold tracking-tight">{t.welcome.title}</h1>
          <p className="mt-3 max-w-md text-lg text-muted">{t.welcome.body}</p>
          <Button size="lg" className="mt-8" onClick={() => navigate("/welcome/tools")}>
            {t.welcome.start}
          </Button>
        </div>
      </div>
    </OnboardingLayout>
  );
}
