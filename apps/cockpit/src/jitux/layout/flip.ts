type FlipOptions = {
  durationMs?: number;
  easing?: string;
};

export function runFlip(elements: HTMLElement[], mutate: () => void, options: FlipOptions = {}): void {
  const first = new Map<HTMLElement, DOMRect>();
  for (const element of elements) {
    first.set(element, element.getBoundingClientRect());
  }

  mutate();

  const animate = () => {
    const duration = options.durationMs ?? 180;
    const easing = options.easing ?? "cubic-bezier(0.2, 0.8, 0.2, 1)";
    for (const element of elements) {
      const before = first.get(element);
      if (!before) {
        continue;
      }
      const after = element.getBoundingClientRect();
      const dx = before.left - after.left;
      const dy = before.top - after.top;
      if (dx === 0 && dy === 0) {
        continue;
      }
      element.animate(
        [
          { transform: `translate3d(${dx}px, ${dy}px, 0)` },
          { transform: "translate3d(0, 0, 0)" },
        ],
        { duration, easing },
      );
    }
  };

  if (typeof requestAnimationFrame === "function") {
    requestAnimationFrame(animate);
    return;
  }
  animate();
}
