import { toCssTransform, type DeckTransform } from "./layout/deck";

type TransformMap = Record<string, DeckTransform>;

export function createTransformScheduler() {
  const elements = new Map<string, HTMLElement>();
  let pending: TransformMap = {};
  let raf = 0;

  const flush = () => {
    raf = 0;
    const next = pending;
    pending = {};
    for (const [id, transform] of Object.entries(next)) {
      const element = elements.get(id);
      if (!element) {
        continue;
      }
      element.style.transform = toCssTransform(transform);
      element.style.opacity = transform.opacity.toString();
      element.style.willChange = "transform, opacity";
    }
  };

  const requestFlush = () => {
    if (raf !== 0) {
      return;
    }
    if (typeof requestAnimationFrame === "function") {
      raf = requestAnimationFrame(flush);
      return;
    }
    raf = window.setTimeout(flush, 16);
  };

  return {
    register: (id: string, element: HTMLElement | null) => {
      if (element) {
        elements.set(id, element);
        return;
      }
      elements.delete(id);
    },
    schedule: (transforms: TransformMap) => {
      pending = { ...pending, ...transforms };
      requestFlush();
    },
    clear: () => {
      if (raf !== 0) {
        if (typeof cancelAnimationFrame === "function") {
          cancelAnimationFrame(raf);
        } else {
          window.clearTimeout(raf);
        }
      }
      raf = 0;
      pending = {};
      elements.clear();
    },
  };
}

export const deckTransformScheduler = createTransformScheduler();
