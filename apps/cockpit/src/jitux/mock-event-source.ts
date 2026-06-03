export class MockEventSource {
  static instances: MockEventSource[] = [];

  readonly listeners = new Map<string, Set<EventListener>>();
  closed = false;

  constructor(readonly url: string) {
    MockEventSource.instances.push(this);
  }

  addEventListener(type: string, listener: EventListener): void {
    const listeners = this.listeners.get(type) ?? new Set<EventListener>();
    listeners.add(listener);
    this.listeners.set(type, listeners);
  }

  removeEventListener(type: string, listener: EventListener): void {
    this.listeners.get(type)?.delete(listener);
  }

  emit(type: string, data: unknown): void {
    if (this.closed) {
      return;
    }
    const payload = typeof data === "string" ? data : JSON.stringify(data);
    const event = { data: payload } as MessageEvent<string>;
    for (const listener of this.listeners.get(type) ?? []) {
      listener(event);
    }
  }

  emitFrame(frame: unknown, type = "jitux.frame"): void {
    this.emit(type, frame);
  }

  emitError(): void {
    if (this.closed) {
      return;
    }
    const event = new Event("error");
    for (const listener of this.listeners.get("error") ?? []) {
      listener(event as Event);
    }
  }

  close(): void {
    this.closed = true;
  }

  static reset(): void {
    MockEventSource.instances = [];
  }
}

