export const requiredProofStates = [
  { id: "loading", label: "Loading" },
  { id: "empty", label: "Empty" },
  { id: "error", label: "Error" },
  { id: "permission-denied", label: "Permission denied" },
  { id: "success", label: "Success" },
] as const;

export const geometryRuntimeReceipt = "target/jankurai/ux-qa/geometry.json";
export const geometryRuntimeHost = "apps/web";
