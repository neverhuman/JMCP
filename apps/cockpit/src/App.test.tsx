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

  it("opens the approvals slice from the first screen", async () => {
    const user = userEvent.setup();
    render(<App />);

    await user.click(screen.getByRole("button", { name: "Approvals" }));

    expect(screen.getByRole("heading", { name: "Approvals", level: 1 })).toBeInTheDocument();
    expect(screen.getByText("Allow bridge to request a temporary write lease")).toBeInTheDocument();
  });

  it("shows the Jeryu ecosystem tool graph", async () => {
    const user = userEvent.setup();
    render(<App />);

    await user.click(screen.getByRole("button", { name: "Tools/Data" }));

    expect(screen.getByRole("heading", { name: /tools across/i })).toBeInTheDocument();
    expect(screen.getAllByText("jeryu.repo.adopt").length).toBeGreaterThan(0);
    expect(screen.getByText("Needs Attention")).toBeInTheDocument();
  });
});
