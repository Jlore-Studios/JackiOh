# Spec gaps: part 25, chunk 4 (engine tests 2: prompt-kinds, prompts, quests)

TESTS-IN: 72 of 72 (`it`s ported one `#[test]` each: prompt-kinds 27, prompts 18, quests 27; 16 `describe`s,
one `mod` each). No test dropped, skipped or `#[ignore]`d. No test reads a `.ts` source file.

## SPEC GAPS

None that stops a test. Assertions written in a Rust form because TS asserted on a live object or a
JS-only property, each kept as far as Rust can say it:

- `packages/engine/test/prompts.test.ts:387,390` (`deepValues(…)` holds no `function`): a closure cannot be
  a field of a serde type, so `crates/engine/tests/rules/prompts.rs` asserts the pending prompt and the
  parked work are plain data that serialise and read back equal.
- `prompts.test.ts:428,483,735` (`expect(state.pending).toBe(pending)`) and `:885` (`expect(bad.state).toBe(
  played.state)`): object identity, ported as equality. `:484` (`expect(sink.state).toBe(state)`): ported as
  `std::ptr::eq` on the sink's state and the board it borrows.
- `prompts.test.ts:707` (R98, `card.memory.sawSelf` undefined on the object TS still held after removing it
  from `resolving`): ported as the dropped clone's memory, the card being in no zone, and no `sawSelf`
  anywhere in the serialised state.
- `packages/engine/test/prompt-kinds.test.ts:191` (`expect(() => act(…)).toThrow()`): ported as
  `reduce(…).error.is_some()`, which is when the harness's `act` throws.
- `prompt-kinds.test.ts:564-566` (`const { sink } = answerKeys(…)`, then `sink.events`): a Rust sink cannot
  outlive the call, so the fixture answers its events (`AnswerResult.events`), which the port pushes.
- `packages/engine/test/quests.test.ts:501,518` (`questBookOf(fused)`): called as `quest_book_of(&state,
  &fused)`, since a fused card's script is composed from the state on lookup (SURFACE §6.6). A signature
  question for part 31, not a dropped assertion.
