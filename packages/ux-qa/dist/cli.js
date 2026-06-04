#!/usr/bin/env node
import { buildReceipt } from "../src/receipts/index.js";

const receipt = buildReceipt({
  config: process.env.JANKURAI_UX_QA_CONFIG ?? "agent/ux-qa.toml",
  artifactRoot: process.env.JANKURAI_UX_QA_ARTIFACT_ROOT ?? "target/jankurai/ux-qa",
  requiredStates: [],
  checks: [],
  missing: [],
});

process.stdout.write(`${JSON.stringify(receipt)}\n`);
