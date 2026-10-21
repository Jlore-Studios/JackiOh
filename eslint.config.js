import js from "@eslint/js";
import { defineConfig } from "eslint/config";
import tseslint from "typescript-eslint";

export default defineConfig(
  {
    ignores: [
      "**/node_modules/**",
      "**/dist/**",
      "**/coverage/**",
      // wasm-bindgen's glue, written by scripts/build-wasm.sh and gitignored (SURFACE §10.2).
      "apps/web/src/wasm/pkg/**",
      // Agent worktrees are whole checkouts of this same repo (see scripts/worktree.sh). Linting
      // them lints every file twice and reports another agent's in-progress work as this tree's.
      ".claude/**",
    ],
  },
  js.configs.recommended,
  tseslint.configs.recommended,
  {
    rules: {
      "@typescript-eslint/no-unused-vars": [
        "error",
        { argsIgnorePattern: "^_", varsIgnorePattern: "^_", ignoreRestSiblings: true },
      ],
    },
  },
  {
    // A test that proves something did not happen moves a fake clock past the moment it would
    // have (`vi.useFakeTimers`, `vi.advanceTimersByTimeAsync`); a wait in real time passes whenever
    // the bug comes a little later or the machine runs slow (#1490). A zero-delay flush stays.
    files: ["apps/web/**/*.test.{ts,tsx}"],
    rules: {
      "no-restricted-syntax": [
        "error",
        {
          selector:
            'NewExpression[callee.name="Promise"] CallExpression:matches([callee.name="setTimeout"], [callee.property.name="setTimeout"])[arguments.length>1]:not([arguments.1.value=0])',
          message:
            "No real-time wait in a web test: use vi.useFakeTimers and advance past the component's own constant (#1490).",
        },
      ],
    },
  },
  {
    // Plain `.mjs` scripts are Node tools, not library code: they legitimately use `console`, the
    // web globals Node exposes, and the process APIs.
    files: ["**/*.mjs"],
    languageOptions: {
      globals: {
        console: "readonly",
        process: "readonly",
        URL: "readonly",
        URLSearchParams: "readonly",
        TextEncoder: "readonly",
        TextDecoder: "readonly",
        fetch: "readonly",
        Buffer: "readonly",
        __dirname: "readonly",
        __filename: "readonly",
      },
    },
  },
);
