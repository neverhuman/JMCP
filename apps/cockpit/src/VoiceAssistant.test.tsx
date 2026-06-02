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
});
