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
      // Same-origin proxy to the local on-box voice stack (ASR/TTS/LLM) so the
      // browser needs no CORS and audio never leaves the machine.
      proxy: {
        "/asr": {
          target: env.VITE_ASR_TARGET ?? "http://127.0.0.1:18878",
          changeOrigin: true,
          rewrite: (path: string) => path.replace(/^\/asr/, ""),
        },
        "/tts": {
          target: env.VITE_TTS_TARGET ?? "http://127.0.0.1:18901",
          changeOrigin: true,
          rewrite: (path: string) => path.replace(/^\/tts/, ""),
        },
        "/llm": {
          target: env.VITE_LLM_TARGET ?? "http://127.0.0.1:18902",
          changeOrigin: true,
          rewrite: (path: string) => path.replace(/^\/llm/, ""),
        },
        // JMCP control-plane API, so the voice agent's tools can read status and
        // take actions same-origin (no CORS; stays on the box).
        "/jmcp": {
          target: env.VITE_JMCP_TARGET ?? "http://127.0.0.1:18877",
          changeOrigin: true,
          rewrite: (path: string) => path.replace(/^\/jmcp/, ""),
        },
      },
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
