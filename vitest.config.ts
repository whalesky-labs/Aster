import { defineConfig } from "vitest/config";

export default defineConfig({
  test: {
    environment: "jsdom",
    include: ["src/**/*.test.ts", "src/**/*.test.tsx"],
    coverage: {
      provider: "v8",
      reporter: ["text", "json-summary"],
      include: [
        "src/shared/lib/editorWindowConfig.ts",
        "src/shared/lib/localDate.ts",
        "src/features/connections/connectionWizardUtils.ts",
        "src/features/app/refreshTargets.ts",
      ],
      thresholds: {
        branches: 85,
        functions: 90,
        lines: 90,
        statements: 90,
      },
    },
  },
});
