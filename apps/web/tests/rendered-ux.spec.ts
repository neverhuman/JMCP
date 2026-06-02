import { expect, test } from "@playwright/test";
import { AxeBuilder } from "@axe-core/playwright";
import fs from "node:fs/promises";
import path from "node:path";
import { geometryRuntimeReceipt, requiredProofStates } from "../src/geometry-runtime";

const artifactDir = path.resolve(process.cwd(), "../../target/jankurai/ux-qa");
const states = requiredProofStates.map((state) => ({
  ...state,
  fileName: state.id,
}));

test("renders every required proof state with accessibility checks", async ({ page }) => {
  await fs.mkdir(artifactDir, { recursive: true });
  await page.goto("/");
  const geometryEvidence: Array<{
    state: string;
    viewport: { width: number; height: number } | null;
    proofShell: { x: number; y: number; width: number; height: number } | null;
  }> = [];

  for (const state of states) {
    await page.getByRole("button", { name: new RegExp(`^${state.label}$`) }).click();
    await expect(page.locator(".proof-shell")).toHaveAttribute("data-proof-state", state.id);
    await expect(page.getByRole("heading", { name: state.label })).toBeVisible();
    const stateCard = page.locator(".proof-stage .state-card");
    await expect(stateCard).toMatchAriaSnapshot({
      name: `${state.fileName}.aria.yml`,
    });
    await expect(stateCard).toHaveScreenshot(`${state.fileName}.png`, {
      animations: "disabled",
    });

    const accessibility = await new AxeBuilder({ page }).analyze();
    expect(accessibility.violations).toEqual([]);
    geometryEvidence.push({
      state,
      viewport: page.viewportSize(),
      proofShell: await page.locator(".proof-shell").boundingBox(),
    });

    await page.screenshot({
      path: path.join(artifactDir, `apps-web-${state.fileName}.png`),
      fullPage: true,
    });
  }

  await fs.writeFile(
    path.resolve(process.cwd(), "../../", geometryRuntimeReceipt),
    `${JSON.stringify(geometryEvidence, null, 2)}\n`,
  );

  await expect(page.getByRole("heading", { name: "Now", level: 1 })).toBeVisible();
});
