export {};

declare global {
  interface Window {
    setTimeout(handler: TimerHandler, timeout?: number, ...arguments_: any[]): ReturnType<typeof setTimeout>;
    clearTimeout(handle?: ReturnType<typeof setTimeout>): void;
  }
}
