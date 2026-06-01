import { loadEnv } from "vite";
import { defineConfig } from "vitest/config";
import react from "@vitejs/plugin-react";

export default defineConfig(({ mode }) => {
  const env = loadEnv(mode, ".", "");
  const portValue = env.JMCP_COCKPIT_PORT ?? "15873";
  if (!/^[0-9]+$/.test(portValue)) {
    throw new Error(`JMCP_COCKPIT_PORT must be numeric: ${portValue}`);
  }
  const safePort = parseInt(portValue, 10);
  if (safePort > 65535) {
    throw new Error(`JMCP_COCKPIT_PORT is outside the valid TCP port range: ${portValue}`);
  }
  const protectedPorts = [2224, 8787, 8799, 8929, 18787, 18788, 19800];

  if (protectedPorts.some((port) => port === safePort)) {
    throw new Error(`JMCP_COCKPIT_PORT must not use Jeryu protected port ${safePort}`);
  }

  return {
    plugins: [react()],
    server: {
      host: env.JMCP_COCKPIT_HOST ?? "127.0.0.1",
      port: safePort,
      strictPort: true,
    },
    preview: {
      host: env.JMCP_COCKPIT_HOST ?? "127.0.0.1",
      port: safePort,
      strictPort: true,
    },
    test: {
      environment: "jsdom",
      setupFiles: "./src/setupTests.ts",
    },
  };
});
