import { defineConfig } from "vitest/config";
import react from "@vitejs/plugin-react";

// Frontend tests run in jsdom with a mocked Tauri backend (src/test/tauriMock.ts)
// and a stubbed Supabase client, so they need neither the Rust app nor network.
export default defineConfig({
  plugins: [react()],
  test: {
    environment: "jsdom",
    include: ["src/**/*.test.{ts,tsx}"],
    setupFiles: ["src/test/setup.ts"],
    // src/supabase.ts throws without these; the client itself is mocked.
    env: {
      VITE_SUPABASE_URL: "https://test.supabase.co",
      VITE_SUPABASE_ANON_KEY: "test-anon-key",
    },
    testTimeout: 30000,
    restoreMocks: true,
  },
});
