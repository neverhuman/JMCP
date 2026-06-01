import { cleanup, render, screen } from "@testing-library/react";
import userEvent from "@testing-library/user-event";
import { afterEach, describe, expect, it } from "vitest";
import App from "./App";
import { views } from "./fixtures";

afterEach(() => {
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
});
