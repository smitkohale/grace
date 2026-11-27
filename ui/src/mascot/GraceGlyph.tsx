// The flat glyph of Grace's face (two square eyes): readable at 16–32 px,
// where a 3D render isn't (DESIGN §5). Used for the sidebar, favicon and tray.

export function GraceGlyph({ size = 28, title }: { size?: number; title?: string }) {
  return (
    <svg
      width={size}
      height={size}
      viewBox="0 0 32 32"
      role={title ? "img" : undefined}
      aria-hidden={title ? undefined : true}
      aria-label={title}
    >
      <rect
        x="2"
        y="3"
        width="28"
        height="26"
        rx="5"
        // Her face stays light in dark mode too, so the black eyes still show.
        fill="#FFFBF7"
        stroke="var(--teal-strong)"
        strokeWidth="2"
      />
      <rect x="8" y="11" width="6" height="9" rx="1" fill="#35312E" />
      <rect x="18" y="11" width="6" height="9" rx="1" fill="#35312E" />
      <rect x="10" y="13" width="2" height="5" fill="#FCC067" />
      <rect x="20" y="13" width="2" height="5" fill="#FCC067" />
    </svg>
  );
}
