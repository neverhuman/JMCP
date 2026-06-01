import { expect, test } from "@playwright/test";
import { AxeBuilder } from "@axe-core/playwright";
import fs from "node:fs/promises";
import path from "node:path";

const artifactDir = path.resolve(process.cwd(), "../../target/jankurai/ux-qa");

test("renders every required proof state with accessibility checks", async ({ page }) => {
  await fs.mkdir(artifactDir, { recursive: true });
  await page.goto("/");

  for (const state of [
    "Loading",
    "Empty",
    "Error",
    "Permission denied",
    "Success",
  ] as const) {
    await page.getByRole("button", { name: new RegExp(`^${state}$`) }).click();
    await expect(page.getByRole("heading", { name: state })).toBeVisible();

    const accessibility = await new AxeBuilder({ page }).analyze();
    expect(accessibility.violations).toEqual([]);

    const fileName = state.toLowerCase().replace(/[^a-z]+/g, "-");
    await page.screenshot({
      path: path.join(artifactDir, `apps-web-${fileName}.png`),
      fullPage: true,
    });
  }

  await expect(page.getByRole("heading", { name: "Now", level: 1 })).toBeVisible();
});
