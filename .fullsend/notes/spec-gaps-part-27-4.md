# Spec gaps: part 27, chunk 4 (cards cross tests: paused-sequences, play-choices, plays-and-casts)

TESTS-IN: 64 of 64 (`it`s ported one `#[test]` each: paused-sequences 45, play-choices 7,
plays-and-casts 12; 54 `describe`s, one `mod` each). No test dropped, skipped, `#[ignore]`d or weakened.

## SPEC GAPS

Nothing was left out. Two TS constructs have no Rust counterpart and are ported as close as Rust says it:

- `packages/cards/test/plays-and-casts.test.ts:292` (`LONG_PLAY_TIMEOUT_MS = 60_000`, passed as vitest's
  per-test timeout at line 341): a Rust `#[test]` has no timeout, so the R58/R350 Call to Chaos test runs
  without one (`crates/cards/tests/cross/plays_and_casts.rs`). Its comment is kept.
- `packages/cards/test/plays-and-casts.test.ts:277` and `:331` (`expect(() => g.play(…)).not.toThrow()`):
  ported as the plain call, which panics, failing the test, on any refusal or engine panic — the same
  assertion, without a wrapper.

No test reads a `.ts` source file, so none was dropped under #133's rule.
