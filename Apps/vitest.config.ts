/// <reference types="vitest" />
import path from "node:path";
import react from "@vitejs/plugin-react";
import { defineConfig } from "vitest/config";

// Vitest 配置：仅服务测试运行时，不影响 `pnpm dev`/`pnpm build`。
// 与 vite.config.ts 分离，避免 Tailwind 插件等生产链在测试时被误加载。
export default defineConfig({
  plugins: [react()],
  resolve: {
    alias: {
      "@": path.resolve(__dirname, "./src"),
    },
  },
  test: {
    environment: "jsdom",
    globals: true,
    setupFiles: ["./src/test/setup.ts"],
    include: ["src/**/__tests__/**/*.{test,spec}.{ts,tsx}"],
    // 覆盖率：开启采集但不设阈值，先建立基线（见 execution-plan.md 决策）
    coverage: {
      provider: "v8",
      reporter: ["text", "html", "json-summary"],
      include: ["src/app/**/*.{ts,tsx}"],
      exclude: [
        "src/app/**/*.d.ts",
        "src/test/**",
        "src/**/__tests__/**",
      ],
    },
  },
});