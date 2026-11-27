import { QueryClient, QueryClientProvider, useQuery } from "@tanstack/react-query";
import { Redirect, Route, Router, Switch } from "wouter";
import { useHashLocation } from "wouter/use-hash-location";
import { AppShell } from "./components/AppShell";
import { Button } from "./components/ui/button";
import { api } from "./lib/api";
import { FindTools } from "./routes/FindTools";
import { SessionDetail } from "./routes/SessionDetail";
import { Sessions } from "./routes/Sessions";
import { Welcome } from "./routes/Welcome";
import { t } from "./strings/en";

const queryClient = new QueryClient({
  defaultOptions: {
    queries: { retry: 1, refetchOnWindowFocus: false, staleTime: 5_000 },
  },
});

/** First screen: onboarding until it's done, then the Sessions list. */
function Start() {
  const config = useQuery({
    queryKey: ["config"],
    queryFn: api.config,
    retry: 4,
    retryDelay: (attempt) => Math.min(500 * 2 ** attempt, 4_000),
  });
  if (config.isPending) return <div className="min-h-screen bg-paper" aria-busy="true" />;
  if (config.isError) {
    // Never restart setup just because Grace's engine answered late.
    return (
      <div className="grid min-h-screen place-items-center bg-paper p-8">
        <div role="alert" className="max-w-md text-center">
          <p className="text-base">{t.start.unreachable}</p>
          <Button className="mt-4" onClick={() => void config.refetch()}>
            {t.start.retry}
          </Button>
        </div>
      </div>
    );
  }
  return <Redirect to={config.data.onboarding_done ? "/sessions" : "/welcome"} />;
}

export function App() {
  return (
    <QueryClientProvider client={queryClient}>
      <Router hook={useHashLocation}>
        <Switch>
          <Route path="/welcome">
            <Welcome />
          </Route>
          <Route path="/welcome/tools">
            <FindTools />
          </Route>
          <Route path="/sessions">
            <AppShell>
              <Sessions />
            </AppShell>
          </Route>
          <Route path="/sessions/:id">
            {(params) => (
              <AppShell>
                <SessionDetail id={Number(params.id)} />
              </AppShell>
            )}
          </Route>
          <Route>
            <Start />
          </Route>
        </Switch>
      </Router>
    </QueryClientProvider>
  );
}
