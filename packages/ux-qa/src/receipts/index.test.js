import test from "node:test";
import assert from "node:assert/strict";
import { buildReceipt } from "./index.js";

test("buildReceipt marks complete evidence as ok", () => {
  const receipt = buildReceipt({
    config: "agent/ux-qa.toml",
    artifactRoot: "target/jankurai/ux-qa",
    requiredStates: ["loading", "success"],
    checks: ["loading.aria.yml", "success-chromium-linux.png"],
    missing: [],
  });

  assert.equal(receipt.ok, true);
  assert.equal(receipt.generatedBy, "@jankurai/ux-qa");
  assert.deepEqual(receipt.requiredStates, ["loading", "success"]);
});

test("buildReceipt preserves missing evidence details", () => {
  const receipt = buildReceipt({
    config: "agent/ux-qa.toml",
    artifactRoot: "target/jankurai/ux-qa",
    requiredStates: ["empty"],
    checks: [],
    missing: ["empty.aria.yml"],
  });

  assert.equal(receipt.ok, false);
  assert.deepEqual(receipt.missing, ["empty.aria.yml"]);
});
