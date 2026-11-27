// Runs inside the page (pass it to `page.evaluate`), so it must stay
// self-contained. Returns every piece of visible text below the WCAG 2.2 AA
// contrast minimum (1.4.3: 4.5:1, or 3:1 for large text) and every text input
// whose border is below 3:1 against what surrounds it (1.4.11). Colours are
// resolved by the browser itself, so any CSS colour syntax works, and fades
// (`opacity`) are taken into account. Text on images or gradients is skipped.
export function contrastProblems(): string[] {
  type Rgba = { r: number; g: number; b: number; a: number };
  const canvas = document.createElement("canvas");
  canvas.width = 1;
  canvas.height = 1;
  const ctx = canvas.getContext("2d", { willReadFrequently: true });
  if (!ctx) return ["no 2D canvas to resolve colours"];
  const rgba = (css: string): Rgba => {
    ctx.clearRect(0, 0, 1, 1);
    ctx.fillStyle = "#000";
    ctx.fillStyle = css;
    ctx.fillRect(0, 0, 1, 1);
    const [r = 0, g = 0, b = 0, a = 0] = ctx.getImageData(0, 0, 1, 1).data;
    return { r, g, b, a: a / 255 };
  };
  const over = (top: Rgba, bottom: Rgba): Rgba => ({
    r: top.r * top.a + bottom.r * (1 - top.a),
    g: top.g * top.a + bottom.g * (1 - top.a),
    b: top.b * top.a + bottom.b * (1 - top.a),
    a: 1,
  });
  const white: Rgba = { r: 255, g: 255, b: 255, a: 1 };
  // The colour behind an element: its own background and its ancestors',
  // stacked until one is opaque. Null when an image or gradient is involved.
  const backgroundOf = (element: Element): Rgba | null => {
    const layers: Rgba[] = [];
    for (let node: Element | null = element; node; node = node.parentElement) {
      const style = getComputedStyle(node);
      if (style.backgroundImage !== "none") return null;
      const colour = rgba(style.backgroundColor);
      if (colour.a > 0) {
        layers.push(colour);
        if (colour.a >= 1) break;
      }
    }
    return layers.reduceRight((below, layer) => over(layer, below), white);
  };
  const luminance = ({ r, g, b }: Rgba) => {
    const channel = (value: number) => {
      const s = value / 255;
      return s <= 0.03928 ? s / 12.92 : ((s + 0.055) / 1.055) ** 2.4;
    };
    return 0.2126 * channel(r) + 0.7152 * channel(g) + 0.0722 * channel(b);
  };
  const ratio = (x: Rgba, y: Rgba) => {
    const [high, low] = [luminance(x), luminance(y)].sort((p, q) => q - p) as [number, number];
    return (high + 0.05) / (low + 0.05);
  };
  // Foreground and background as they reach the screen, after any fades.
  const onScreen = (element: Element, colour: string) => {
    const background = backgroundOf(element);
    if (!background) return null;
    let fore = over(rgba(colour), background);
    let back = background;
    for (let node: Element | null = element; node; node = node.parentElement) {
      const opacity = Number.parseFloat(getComputedStyle(node).opacity);
      if (opacity < 1) {
        const behind = node.parentElement ? backgroundOf(node.parentElement) : white;
        if (!behind) return null;
        fore = over({ ...fore, a: opacity }, behind);
        back = over({ ...back, a: opacity }, behind);
      }
    }
    return { fore, back };
  };

  const problems: string[] = [];
  const checked = new Set<Element>();
  const walker = document.createTreeWalker(document.body, NodeFilter.SHOW_TEXT);
  while (walker.nextNode()) {
    const text = walker.currentNode.textContent?.trim();
    const element = walker.currentNode.parentElement;
    if (!text || !element || checked.has(element)) continue;
    checked.add(element);
    const style = getComputedStyle(element);
    const box = element.getBoundingClientRect();
    if (style.visibility === "hidden" || box.width <= 1 || box.height <= 1) continue;
    if (element.closest("button:disabled, [aria-disabled='true']")) continue;
    const colours = onScreen(element, style.color);
    if (!colours) continue;
    const size = Number.parseFloat(style.fontSize);
    const bold = Number.parseInt(style.fontWeight, 10) >= 700;
    const needed = size >= 24 || (size >= 18.66 && bold) ? 3 : 4.5;
    const actual = ratio(colours.fore, colours.back);
    if (actual < needed)
      problems.push(`"${text.slice(0, 40)}": ${actual.toFixed(2)}:1, needs ${needed}:1`);
  }

  for (const input of document.querySelectorAll("input:not([type=hidden]), textarea, select")) {
    const around = input.parentElement ? backgroundOf(input.parentElement) : white;
    if (!around) continue;
    const border = over(rgba(getComputedStyle(input).borderTopColor), around);
    const actual = ratio(border, around);
    if (actual < 3)
      problems.push(
        `border of ${input.tagName.toLowerCase()} "${input.getAttribute("placeholder") ?? input.id}": ${actual.toFixed(2)}:1, needs 3:1`,
      );
  }
  return problems;
}
