import { cleanup, render, screen } from "@testing-library/react";
import userEvent from "@testing-library/user-event";
import { afterEach, beforeEach, describe, expect, it, vi } from "vitest";
import App from "./App";
import { views } from "./fixtures";

beforeEach(() => {
  vi.stubGlobal("fetch", vi.fn(() => Promise.reject(new Error("test api unavailable"))));
});

afterEach(() => {
  vi.unstubAllGlobals();
  cleanup();
});

describe("JMCP cockpit", () => {
  it("renders every required dashboard view in navigation", () => {
    render(<App />);

    for (const view of views) {
      expect(screen.getByRole("button", { name: view.label })).toBeInTheDocument();
    }
  });

  it("shows the attention inbox on the first screen", () => {
    render(<App />);

    expect(screen.getByText("AP-88")).toBeInTheDocument();
    expect(screen.getByText("Quarantine the bridge until the service card lands.")).toBeInTheDocument();
    expect(screen.getByText("The adapter still lacks an evidence-backed write lease.")).toBeInTheDocument();
  });

  it("opens the memory slice with promotion and quarantine drill-down", async () => {
    const user = userEvent.setup();
    render(<App />);

    await user.click(screen.getByRole("button", { name: "Memory" }));

    expect(screen.getByRole("heading", { name: "Memory", level: 1 })).toBeInTheDocument();
    expect(screen.getByText("ML-219")).toBeInTheDocument();
    expect(screen.getByText("Adapters that emit raw webhooks stay quarantined until wrapped in JCP envelopes.")).toBeInTheDocument();
    expect(screen.getByText("Incident / quarantine")).toBeInTheDocument();
  });

  it("opens the voice/text slice with transcript and confirmation details", async () => {
    const user = userEvent.setup();
    render(<App />);

    await user.click(screen.getByRole("button", { name: "Voice/Text" }));

    expect(screen.getByRole("heading", { name: "Voice/Text", level: 1 })).toBeInTheDocument();
    expect(screen.getByText("Approve deployment request")).toBeInTheDocument();
    expect(screen.getByText("approve the deployment with token alpha")).toBeInTheDocument();
    expect(screen.getAllByText("response required").length).toBeGreaterThan(0);
  });

  it("shows the Universe view with repo scores and placement rows", async () => {
    const user = userEvent.setup();
    render(<App />);

    await user.click(screen.getByRole("button", { name: "Universe" }));

    expect(screen.getByRole("heading", { name: /observed coverage/i })).toBeInTheDocument();
    expect(screen.getAllByText("Jeryu").length).toBeGreaterThan(0);
    expect(screen.getByText("Placement Rows")).toBeInTheDocument();
  });

  it("shows the Telegram approval backplane with token and lineage details", async () => {
    const user = userEvent.setup();
    render(<App />);

    await user.click(screen.getByRole("button", { name: "Approvals" }));

    expect(screen.getByRole("heading", { name: "Approvals", level: 1 })).toBeInTheDocument();
    expect(screen.getByText("Telegram backplane")).toBeInTheDocument();
    expect(screen.getByText("sha256:bridge-alpha")).toBeInTheDocument();
    expect(screen.getByText("challenge.AP-88")).toBeInTheDocument();
  });
});
