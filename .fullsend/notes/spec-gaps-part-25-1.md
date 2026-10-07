# Spec gaps: part 25, chunk 1 (engine tests 2: activate, aiPolicy, audit, boardHistory, callToChaos, callToChaosPlus, comboIndex)

TESTS-IN: 122 of 122 (`it`s ported one `#[test]` each; 20 `describe`s, one `mod` each). No test dropped, skipped or `#[ignore]`d.

## SPEC GAPS

Assertions that could not be written exactly as TS wrote them, each kept as far as Rust can say it:

- `packages/engine/test/callToChaosPlus.test.ts:275` (entry 9, "each deck card replaced one for one"):
  `old.every((card) => card.zone.z === "gone" && findInstance(state, card.id) === undefined)`. The
  `zone.z === "gone"` half reads a TS object the test still held after the card ceased to exist; a Rust
  test holds clones, never a live handle, so a card that is in no pile cannot be read at all. Ported as
  `find_instance(..).is_none()` for each old card (`crates/engine/tests/rules/call_to_chaos_plus.rs`).
- `packages/engine/test/activate.test.ts:670` (R752 alias test):
  `expect(actResult(state, activate("p1", card.id, { modes: ["x"] })).error).not.toBeNull()`. `error` is
  `string | undefined`, never `null`, so the TS assertion holds whatever `reduce` does; it only pins that
  the call returns. Ported as the call alone (it must not panic), with a comment. Asserting
  `error.is_some()` would be stronger than the TS test.
- `packages/engine/test/aiPolicy.test.ts:167`: `expect(first.sink.state).toBe(first.original)` is an
  object-identity check. In Rust the sink borrows the very board it was built over, so the identity holds
  by construction; the ported test returns that board from the playout and asserts on it (the line before,
  `active === "p2"`, is asserted on the same value).
- `packages/engine/test/boardHistory.test.ts:144-146` (R227 rename) and
  `packages/engine/test/activate.test.ts:274-276` (R384 bounce): TS moved a card to the hand and then
  placed that same object on the field, so one object sat in the hand and on the field at once. Rust cannot
  alias it: the port takes the card out of the hand pile first, then renames (`fresh_face_down_id`, whose
  TS doc says "called on a card that is in no pile") and places it once. Every assertion is kept; the only
  difference is that the hand no longer also holds the card, which no assertion reads.

No test reads a `.ts` source file, so none was dropped under #133's rule.
