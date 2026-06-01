import { StrictMode, type ReactNode } from "react";
import { createRoot } from "react-dom/client";

export function mountReactApp(
  container: Element | null,
  node: ReactNode,
  options?: { title?: string },
): void {
  if (container === null) {
    throw new Error("Root element missing");
  }

  if (options?.title) {
    document.title = options.title;
  }

  createRoot(container).render(<StrictMode>{node}</StrictMode>);
}
