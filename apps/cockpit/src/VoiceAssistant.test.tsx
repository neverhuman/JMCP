import { cleanup, render, screen } from "@testing-library/react";
import userEvent from "@testing-library/user-event";
import { afterEach, beforeEach, describe, expect, it, vi } from "vitest";
import VoiceAssistant from "./components/VoiceAssistant";

// jsdom has no navigator.mediaDevices, so the widget renders its no-microphone
// affordance — which is exactly the path a desktop Mac (no built-in mic) hits.
// The typed command box must still be present and drive the agent. fetch is
// stubbed to reject so a submitted turn fails fast without touching the network.
beforeEach(() => {
  vi.stubGlobal("fetch", vi.fn(() => Promise.reject(new Error("test api unavailable"))));
});

afterEach(() => {
  vi.unstubAllGlobals();
  cleanup();
});

describe("VoiceAssistant widget", () => {
  let savedMediaDevices: MediaDevices | undefined;
  let savedSecureContext: boolean | undefined;
  let savedMediaRecorder: typeof MediaRecorder | undefined;

  beforeEach(() => {
    savedMediaDevices = navigator.mediaDevices;
    savedSecureContext = window.isSecureContext;
    savedMediaRecorder = window.MediaRecorder;
  });

  afterEach(() => {
    Object.defineProperty(navigator, "mediaDevices", {
      configurable: true,
      value: savedMediaDevices,
    });
    Object.defineProperty(window, "isSecureContext", {
      configurable: true,
      value: savedSecureContext,
    });
    Object.defineProperty(window, "MediaRecorder", {
      configurable: true,
      value: savedMediaRecorder,
    });
  });

  it("offers a type-to-talk box even without a microphone", () => {
    render(<VoiceAssistant />);
    expect(screen.getByText(/No microphone/i)).toBeInTheDocument();
    expect(screen.getByLabelText("Type a command for JMCP")).toBeInTheDocument();
    expect(screen.getByRole("button", { name: "Send" })).toBeInTheDocument();
  });

  it("disables Send until there is text, and clears the box on submit", async () => {
    const user = userEvent.setup();
    render(<VoiceAssistant />);
    const input = screen.getByLabelText("Type a command for JMCP");
    const send = screen.getByRole("button", { name: "Send" });

    expect(send).toBeDisabled();
    await user.type(input, "how is JMCP doing?");
    expect(send).toBeEnabled();

    await user.click(send);
    expect(input).toHaveValue("");
  });

  it("shows a useful message when Chrome reports no microphone device", async () => {
    const user = userEvent.setup();
    Object.defineProperty(window, "isSecureContext", {
      configurable: true,
      value: true,
    });
    Object.defineProperty(window, "MediaRecorder", {
      configurable: true,
      value: class MediaRecorderDouble {
        static isTypeSupported() {
          return true;
        }
      },
    });
    Object.defineProperty(navigator, "mediaDevices", {
      configurable: true,
      value: {
        enumerateDevices: vi.fn(() => Promise.resolve([])),
        getUserMedia: vi.fn(() =>
          Promise.reject(new DOMException("Requested device not found", "NotFoundError")),
        ),
      },
    });

    render(<VoiceAssistant />);
    await user.click(screen.getByRole("button", { name: "Start voice assistant" }));

    expect(await screen.findByText(/Chrome cannot see a microphone input/i)).toBeInTheDocument();
  });
});
