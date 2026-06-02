export function buildReceipt({ config, artifactRoot, requiredStates, checks, missing }) {
  return {
    schema: "https://schemas.neverhuman.ai/jmcp/repair-queue.schema.json",
    ok: missing.length === 0,
    config,
    artifactRoot,
    requiredStates,
    checks,
    missing,
    generatedBy: "@jankurai/ux-qa",
  };
}
