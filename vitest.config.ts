import { defineConfig } from "vitest/config";

// Frontend unit tests run against pure TypeScript modules (no Svelte component
// compilation needed), so a lightweight Node environment keeps them fast.
export default defineConfig({
  test: {
    include: ["src/**/*.{test,spec}.ts"],
    environment: "node",
    reporters: "default",
    coverage: {
      provider: "v8",
      // Scope coverage to the pure-logic modules that unit tests target
      // (Svelte components / stores need a browser environment and are covered
      // by type-checking + the Rust engine tests instead).
      include: [
        "src/features/ladder/lib/**/*.ts",
        "src/features/ladder/elements/_shared/id.ts",
        "src/shared/lib/demoProgram.ts",
      ],
      exclude: ["src/**/*.test.ts", "src/**/*.d.ts"],
      reporter: ["text-summary", "lcov"],
    },
  },
});
