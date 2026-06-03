export type DeckTransform = {
  x: number;
  y: number;
  z: number;
  rotateX: number;
  rotateY: number;
  scale: number;
  opacity: number;
};

export function getDeckTransform(index: number, active: boolean): DeckTransform {
  if (active) {
    return {
      x: 0,
      y: 0,
      z: 96,
      rotateX: 0,
      rotateY: 0,
      scale: 1,
      opacity: 1,
    };
  }

  const lane = Math.min(index, 19);
  const side = lane % 2 === 0 ? 1 : -1;
  const row = Math.floor((lane - 1) / 2);
  const compressed = lane > 8 ? 0.62 : 1;

  return {
    x: side * (170 + row * 22) * compressed,
    y: 32 + row * 34,
    z: -42 - row * 26,
    rotateX: -3,
    rotateY: side * -16,
    scale: Math.max(0.62, 0.9 - row * 0.05),
    opacity: Math.max(0.38, 0.88 - row * 0.08),
  };
}

export function toCssTransform(transform: DeckTransform): string {
  return `translate3d(${transform.x}px, ${transform.y}px, ${transform.z}px) rotateX(${transform.rotateX}deg) rotateY(${transform.rotateY}deg) scale(${transform.scale})`;
}
