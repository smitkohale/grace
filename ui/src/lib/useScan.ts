import { useQueryClient } from "@tanstack/react-query";
import { useCallback, useEffect, useRef, useState } from "react";
import { api, type ScanJob } from "./api";

const POLL_MS = 400;

/**
 * Reads new session data now and whenever the window comes back into view,
 * then refreshes the sessions list. Scans are incremental, so this is cheap.
 */
export function useScan() {
  const queryClient = useQueryClient();
  const [status, setStatus] = useState<ScanJob | null>(null);
  const [failed, setFailed] = useState(false);
  const busy = useRef(false);

  const rescan = useCallback(async () => {
    if (busy.current) return;
    busy.current = true;
    setFailed(false);
    try {
      await api.startScan();
      for (;;) {
        const next = await api.scanStatus();
        setStatus(next);
        if (!next.running) break;
        await new Promise((resolve) => setTimeout(resolve, POLL_MS));
      }
      await queryClient.invalidateQueries({ queryKey: ["sessions"] });
    } catch {
      setFailed(true);
    } finally {
      busy.current = false;
    }
  }, [queryClient]);

  useEffect(() => {
    void rescan();
    const onVisible = () => {
      if (document.visibilityState === "visible") void rescan();
    };
    document.addEventListener("visibilitychange", onVisible);
    return () => document.removeEventListener("visibilitychange", onVisible);
  }, [rescan]);

  return { status, failed, scanning: status?.running ?? busy.current, rescan };
}
