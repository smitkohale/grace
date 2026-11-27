import { useEffect, useRef, useState } from "react";
import modelUrl from "../../../assets/brand/mascot/grace.glb?url";
import still256 from "../../../assets/brand/mascot/renders/grace-256.webp?url";
import still512 from "../../../assets/brand/mascot/renders/grace-512.webp?url";
import { cn } from "../lib/cn";
import { t } from "../strings/en";
import type { MascotState } from "./motion";
import type { GraceScene } from "./scene";

export type { MascotState } from "./motion";

export interface GraceMascotProps {
  state: MascotState;
  size: "hero" | "card";
  className?: string;
}

let webglWorks: boolean | undefined;

/**
 * Whether this browser can draw WebGL. Checked once per page, and the check's
 * own context is released at once: each live context holds GPU memory, and
 * browsers cap how many a page may have.
 */
function supportsWebGL(): boolean {
  if (webglWorks === undefined) {
    try {
      const probe = document.createElement("canvas");
      const gl = probe.getContext("webgl2") ?? probe.getContext("webgl");
      webglWorks = Boolean(gl);
      gl?.getExtension("WEBGL_lose_context")?.loseContext();
    } catch {
      webglWorks = false;
    }
  }
  return webglWorks;
}

function usePrefersReducedMotion(): boolean {
  const query = "(prefers-reduced-motion: reduce)";
  const [reduced, setReduced] = useState(() => window.matchMedia?.(query).matches ?? false);
  useEffect(() => {
    const media = window.matchMedia?.(query);
    if (!media) return;
    const onChange = () => setReduced(media.matches);
    media.addEventListener("change", onChange);
    return () => media.removeEventListener("change", onChange);
  }, []);
  return reduced;
}

/**
 * Grace, live in 3D on her warm card (DESIGN §5). She follows the pointer,
 * blinks, perks up on hover, and waves when clicked (or Enter at hero size).
 * She pauses when hidden, holds still with reduced motion, and falls back to
 * a flat picture without WebGL. She never carries information that isn't
 * also in text.
 */
export function GraceMascot({ state, size, className }: GraceMascotProps) {
  const stageRef = useRef<HTMLDivElement>(null);
  const holderRef = useRef<HTMLElement | null>(null);
  const sceneRef = useRef<GraceScene | null>(null);
  const stateRef = useRef(state);
  const [mode, setMode] = useState<"loading" | "live" | "flat">("loading");
  const reducedMotion = usePrefersReducedMotion();
  const interactive = size === "hero";

  stateRef.current = state;

  useEffect(() => {
    const stage = stageRef.current;
    if (!stage || !supportsWebGL()) {
      setMode("flat");
      return;
    }
    // A fresh canvas for every mount: leaving the screen releases the old
    // canvas's GPU memory, and a released canvas can't draw again (React's
    // StrictMode mounts twice in development).
    const canvas = document.createElement("canvas");
    canvas.className = "absolute inset-0 size-full touch-none";
    stage.append(canvas);
    let cancelled = false;
    let scene: GraceScene | null = null;
    import("./scene")
      .then(({ createGraceScene }) =>
        createGraceScene(canvas, { modelUrl, state: stateRef.current, reducedMotion }),
      )
      .then((created) => {
        if (cancelled) {
          created.dispose();
          return;
        }
        scene = created;
        sceneRef.current = created;
        setMode("live");
      })
      .catch(() => {
        if (!cancelled) setMode("flat");
      });
    return () => {
      cancelled = true;
      scene?.dispose();
      sceneRef.current = null;
      canvas.remove();
    };
  }, [reducedMotion]);

  useEffect(() => {
    sceneRef.current?.setState(state);
  }, [state]);

  // Pause when the window is hidden or she's scrolled out of view.
  useEffect(() => {
    const holder = holderRef.current;
    if (!holder || mode !== "live") return;
    let onScreen = true;
    const update = () =>
      sceneRef.current?.setPaused(!onScreen || document.visibilityState === "hidden");
    const observer = new IntersectionObserver((entries) => {
      onScreen = entries.some((entry) => entry.isIntersecting);
      update();
    });
    observer.observe(holder);
    document.addEventListener("visibilitychange", update);
    return () => {
      observer.disconnect();
      document.removeEventListener("visibilitychange", update);
    };
  }, [mode]);

  const classes = cn(
    "mascot-card relative grid select-none place-items-center overflow-hidden rounded-card",
    size === "hero" ? "aspect-square w-full max-w-sm" : "size-40",
    mode === "live" && "cursor-grab active:cursor-grabbing",
    className,
  );
  const visuals = (
    <>
      <div
        ref={stageRef}
        className={cn(
          "absolute inset-0 transition-opacity duration-500",
          mode === "live" ? "opacity-100" : "opacity-0",
        )}
      />
      {mode !== "live" && (
        // A still of the same model: shown at once, and kept without WebGL.
        <img
          src={size === "hero" ? still512 : still256}
          alt=""
          className="absolute inset-0 size-full object-contain"
          draggable={false}
        />
      )}
    </>
  );
  const keepRef = (element: HTMLElement | null) => {
    holderRef.current = element;
  };

  // At hero size she's a real button: click, tap, Enter or Space makes her wave.
  if (interactive) {
    return (
      <button
        type="button"
        ref={keepRef}
        aria-label={`${t.mascot[state]}. ${t.mascot.press}`}
        onClick={() => sceneRef.current?.press()}
        className={classes}
      >
        {visuals}
      </button>
    );
  }
  return (
    <div ref={keepRef} role="img" aria-label={t.mascot[state]} className={classes}>
      {visuals}
    </div>
  );
}
