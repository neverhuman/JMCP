import { cleanup, render, screen } from "@testing-library/react";
import userEvent from "@testing-library/user-event";
import { afterEach, beforeEach, describe, expect, it, vi } from "vitest";
import App from "./App";
import { views } from "./fixtures";
import { resetDeckStoreForTests } from "./jitux/store";

beforeEach(() => {
  vi.stubGlobal("fetch", vi.fn(() => Promise.reject(new Error("test api unavailable"))));
  resetDeckStoreForTests();
});

afterEach(() => {
  resetDeckStoreForTests();
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

  it("shows the Mission Deck on the first screen", async () => {
    render(<App />);

    expect(await screen.findByLabelText("AIUX Mission Deck")).toBeInTheDocument();
    const rankedDeck = screen.getByLabelText("Ranked Mission Deck");
    expect(rankedDeck).toBeInTheDocument();
    expect(screen.getAllByText("Queue blocker").length).toBeGreaterThan(0);
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
