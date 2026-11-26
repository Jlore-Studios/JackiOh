import comments from "@eslint-community/eslint-plugin-eslint-comments";
import js from "@eslint/js";
import { defineConfig } from "eslint/config";
import reactHooks from "eslint-plugin-react-hooks";
import regexp from "eslint-plugin-regexp";
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
    // A suppression says why and expires (CLAUDE.md rule 15, #4978): a directive names its rules and
    // gives its reason after ` -- `, and one that suppresses nothing fails.
    linterOptions: { reportUnusedDisableDirectives: "error", reportUnusedInlineConfigs: "error" },
    plugins: { "@eslint-community/eslint-comments": comments },
    rules: {
      "@eslint-community/eslint-comments/require-description": "error",
      "@eslint-community/eslint-comments/no-unlimited-disable": "error",
      // The hooks rules below have no inline way out: the fix is the code (`useEffectEvent`, a key, a
      // value derived during render), never a disable that the next agent copies.
      "@eslint-community/eslint-comments/no-restricted-disable": ["error", "react-hooks/*"],
    },
  },
  {
    // React's own hooks rules, the React Compiler's included, all errors (#4978): agents get effects
    // wrong more than anything else in the client. `eslint-suppressions.json` holds the violations
    // already in the tree, so a new one fails CI, and so does a fixed one left unpruned
    // (`pnpm exec eslint --prune-suppressions apps/web apps/edge apps/admin e2e`): the list only
    // shrinks. Never add to it.
    files: ["apps/web/src/**/*.{ts,tsx}", "apps/admin/src/**/*.{ts,tsx}", "e2e/**/*.{ts,tsx}"],
    plugins: { "react-hooks": reactHooks },
    rules: Object.fromEntries(
      Object.keys(reactHooks.configs["recommended-latest"].rules).map((rule) => [rule, "error"]),
    ),
  },
  {
    // A test that proves something did not happen moves a fake clock past the moment it would
    // have (`vi.useFakeTimers`, `vi.advanceTimersByTimeAsync`); a wait in real time passes whenever
    // the bug comes a little later or the machine runs slow (#1490). A zero-delay flush stays.
    files: ["apps/web/**/*.test.{ts,tsx}", "apps/admin/**/*.test.{ts,tsx}"],
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
    // Item 7 of #3897 (#4009): a regex that backtracks super-linearly on crafted input freezes
    // whatever runs it, and another player's text (chat, lobby and deck names, deck codes,
    // usernames) reaches the browser and the Worker. These two rules hold every regex over the
    // client, the Worker, the admin app and e2e to linear time.
    files: ["apps/web/**/*.{ts,tsx,js}", "apps/edge/**/*.ts", "apps/admin/**/*.ts", "e2e/**/*.{ts,tsx}"],
    plugins: { regexp },
    rules: {
      "regexp/no-super-linear-backtracking": "error",
      "regexp/no-super-linear-move": "error",
    },
  },
  {
    // Item 13 of #3897 (#4013): a secret made with a predictable generator can be guessed.
    // Nothing unguessable uses `Math.random` anywhere the client, the Worker or the admin app
    // runs it; anything that needs to be unguessable uses `crypto.getRandomValues` or
    // `crypto.randomUUID`. The files that use it today are allowlisted below, each with why
    // its value is not a secret.
    files: ["apps/web/src/**/*.{ts,tsx,js}", "apps/edge/**/*.{ts,tsx}", "apps/admin/**/*.{ts,tsx}"],
    rules: {
      "no-restricted-properties": [
        "error",
        {
          object: "Math",
          property: "random",
          message:
            "Nothing unguessable uses Math.random: use crypto.getRandomValues or crypto.randomUUID (#4013).",
        },
      ],
    },
  },
  {
    // #4013 allowlist: today's `Math.random` uses, each cosmetic or timing, never a secret.
    files: [
      // Sound variation and trigger-line picks (R655, R1088): heard, not a secret.
      "apps/web/src/audio/cues.ts",
      // Cosmetic trigger-line rng (R1088): heard, not a secret.
      "apps/web/src/audio/director.ts",
      // Effect pitch variation (R655): heard, not a secret.
      "apps/web/src/audio/engine.ts",
      // Local practice portraits and persona (R642, R645): shown, not a secret.
      "apps/web/src/practice/emotes.ts",
      // Reconnect jitter (R1441) delays only, and nonces are per-seat dedup keys (R4013000): neither a secret.
      "apps/web/src/game/net.ts",
      // Viewer-local cosmetic outcomes (R3348001): seen only, not a secret.
      "apps/web/src/game/arena/play.ts",
      // Reconnect jitter (R1441): timing only, not a secret.
      "apps/web/src/together/socialSocket.ts",
      // Reconnect jitter (R1441): timing only, not a secret.
      "apps/web/src/duos/socket.ts",
      // Local non-daily run seed (DR2): no opponent, nothing to guess.
      "apps/web/src/dungeon/seed.ts",
      // Decorative landing fan deal (R374): layout only, not a secret.
      "apps/web/src/routes/landing.tsx",
    ],
    rules: {
      "no-restricted-properties": "off",
    },
  },
  {
    // Item 17 of #3897 (#4016): a player's text rendered as a template or as HTML (template
    // injection). The daily mail is plain text and the unsubscribe page prints fixed strings, so
    // no raw-HTML sink is needed anywhere the client, the Worker or the admin app runs: neither
    // React's raw-markup escape hatch nor assignments to `innerHTML`/`outerHTML`, neither
    // `insertAdjacentHTML` nor `document.write`, nor the `Function` constructor. A future need
    // adds a commented exception.
    files: ["apps/web/src/**/*.{ts,tsx,js}", "apps/edge/**/*.ts", "apps/admin/**/*.{ts,tsx}"],
    ignores: ["**/*.test.{ts,tsx}"],
    rules: {
      "no-restricted-syntax": [
        "error",
        {
          selector: 'JSXAttribute[name.name="dangerouslySetInnerHTML"]',
          message:
            "No raw HTML in JSX: render text, not markup, so a player's text cannot become markup (#4016).",
        },
        {
          selector: "AssignmentExpression[left.property.name='innerHTML']",
          message:
            "No innerHTML assignment: render text, not markup, so a player's text cannot become markup (#4016).",
        },
        {
          selector: "AssignmentExpression[left.property.name='outerHTML']",
          message:
            "No outerHTML assignment: render text, not markup, so a player's text cannot become markup (#4016).",
        },
        {
          selector: 'NewExpression[callee.name="Function"]',
          message:
            "No new Function: a player's text must never become code (#4016).",
        },
      ],
      "no-restricted-properties": [
        "error",
        {
          property: "insertAdjacentHTML",
          message:
            "No insertAdjacentHTML: render text, not markup, so a player's text cannot become markup (#4016).",
        },
        {
          object: "document",
          property: "write",
          message:
            "No document.write: render text, not markup, so a player's text cannot become markup (#4016).",
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
