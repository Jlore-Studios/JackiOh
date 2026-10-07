# Spec gaps: part 26, chunk 4 of 7 (engine tests 3: lethal, library-copies, mulligan-concurrent, ownLibrary, params, pools, preview-ids, preview, query, recruit-variants, reduce)

TESTS-IN: 112 of 112 (`it`s ported one `#[test]` each; 30 `describe`s, one `mod` each). No test dropped, skipped or `#[ignore]`d.

## SPEC GAPS

No test was left unported. Assertions that could not be written exactly as TS wrote them, each kept as far as Rust can say it:

- `packages/engine/test/reduce.test.ts:56` (`dedupes a repeated nonce`): `expect(second.state).toBe(first.state)` is object identity. A Rust value has no identity; ported as `assert_eq!(second.state, first.state)` (`crates/engine/tests/rules/reduce.rs`).
- `packages/engine/test/pools.test.ts:156-157` (R387 `excludingDefId`): `expect(excludingDefId(asked, …)).toBe(asked)` is identity ("handed back as it was"). Ported as equality of the serialised queries (`crates/engine/tests/rules/pools.rs`).
- `packages/engine/test/preview.test.ts:530` (R280 "handed no rng and no event sink"): `Object.keys(ctx).sort()` equal to the six keys. A Rust struct has no runtime key list; the recording hook destructures `ConditionContext` exhaustively (no `..`), which compiles only while the context is exactly those six fields, and the test asserts the recorded key list on every call (`crates/engine/tests/rules/preview.rs`, `record`).
- `packages/engine/test/preview.test.ts:557-562`, `preview-ids.test.ts:90-96`, `query.test.ts:120-128, 169-180, 314-326` (the "hands back a copy" / "the view holds its own array" tests): TS mutated the returned array or object and checked the state or a later view. In Rust the returned values are owned, so these hold by construction; each test still mutates its copy and asserts the state and the next read unchanged.
- `packages/engine/test/params.test.ts:152-156, 165-168, 178-180` (`param(ctx, key)` on plain objects and its `toThrow`s): TS handed `param` object literals shaped like a context (`{ state, self: null, radiant, defId }`, `{ ...ctx, data }`). Ported as an `EffectContext` (`make_context` or `EffectContext::new`) with those fields set. The TS throws are panics with TS's message (SURFACE §4.4.9), caught with `std::panic::catch_unwind` and matched on the message text.
- `packages/engine/test/reduce.test.ts:122` (the walk's `30_000` ms timeout): a Rust test has no default timeout to raise; the comment is kept with a note, every assertion is ported.
- `packages/engine/test/preview.test.ts:153-164`, `preview-ids.test.ts:42-51` (`beforeAll`/`afterAll` save and restore of the registries, `beforeEach` `mockClear`): the registries are the testkit's per-thread override and each `#[test]` runs on its own thread, so there is nothing to restore; the mock logs are thread-locals that start empty. No assertion lived in those hooks.

No test reads a `.ts` source file, so none was dropped under #133's rule.
