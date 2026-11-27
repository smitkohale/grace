import { useEffect, useRef, useState } from "react";
import modelUrl from "../../../assets/brand/mascot/grace.glb?url";
import { cn } from "../lib/cn";
import { t } from "../strings/en";
import { GraceGlyph } from "./GraceGlyph";
import type { MascotState } from "./motion";
import type { GraceScene } from "./scene";

export type { MascotState } from "./motion";

export interface GraceMascotProps {
  state: MascotState;
  size: "hero" | "card";
  className?: string;
}

function supportsWebGL(): boolean {
  try {
    const canvas = document.createElement("canvas");
    return Boolean(canvas.getContext("webgl2") ?? canvas.getContext("webgl"));
  } catch {
    return false;
  }
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
  const canvasRef = useRef<HTMLCanvasElement>(null);
  const holderRef = useRef<HTMLElement | null>(null);
  const sceneRef = useRef<GraceScene | null>(null);
  const stateRef = useRef(state);
  const [mode, setMode] = useState<"loading" | "live" | "flat">("loading");
  const reducedMotion = usePrefersReducedMotion();
  const interactive = size === "hero";

  stateRef.current = state;

  useEffect(() => {
    const canvas = canvasRef.current;
    if (!canvas || !supportsWebGL()) {
      setMode("flat");
      return;
    }
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
      <canvas
        ref={canvasRef}
        className={cn(
          "absolute inset-0 size-full touch-none transition-opacity duration-500",
          mode === "live" ? "opacity-100" : "opacity-0",
        )}
      />
      {mode !== "live" && <GraceGlyph size={size === "hero" ? 120 : 64} />}
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
