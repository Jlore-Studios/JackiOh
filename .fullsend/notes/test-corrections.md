# Test corrections (fullsend Phase 5)

A ported test fixed to match its TypeScript original, one entry each: the test, what was wrong, and the TS file and line it now matches.

- **part 36, `apps/web/src/cards/rules.test.ts` (`KEYWORDS_NOTE`, `CARD_TYPES_NOTE`)**: part 28 (#133) read the spec notes through `new URL("../../../../spec/….md", import.meta.url)`, which Vite rewrites under vitest's jsdom into an asset URL on the page's origin, so `readFileSync` threw "The URL must be of scheme file" and the two B11 tests failed before reading a row. Now `join(dirname(fileURLToPath(import.meta.url)), "../../../../spec")`, as the TS original read SPEC.md (`origin/main:apps/web/src/cards/rules.test.ts:37`). No assertion changed. The same fix in `apps/web/src/test/setup.ts` (the WASM bytes; part 21, no TS original).
