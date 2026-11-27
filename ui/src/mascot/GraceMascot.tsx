import { cn } from "../lib/cn";
import { t } from "../strings/en";
import { GraceGlyph } from "./GraceGlyph";

export type MascotState = "hello" | "thinking" | "idle" | "sleeping";

export interface GraceMascotProps {
  state: MascotState;
  size: "hero" | "card";
  className?: string;
}

/**
 * Grace on her warm card. The live 3D model replaces this placeholder in the
 * mascot step of M0 (docs/plans/m0.md, Task 6).
 */
export function GraceMascot({ state, size, className }: GraceMascotProps) {
  return (
    <div
      role="img"
      aria-label={t.mascot[state]}
      className={cn(
        "mascot-card relative grid place-items-center overflow-hidden rounded-card",
        size === "hero" ? "aspect-square w-full max-w-sm" : "size-40",
        className,
      )}
    >
      <GraceGlyph size={size === "hero" ? 120 : 64} />
    </div>
  );
}
