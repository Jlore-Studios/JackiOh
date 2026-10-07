# Spec gaps: part 25, chunk 3 (engine tests 2: heroPower, kyTest, lastBoards, modifiers, papaya, pauses, perfectHand)

TESTS-IN: 115 of 117 (`it`s ported one `#[test]` each; 22 `describe`s, one `mod` each). Two tests of
`pauses.test.ts` are not ported (below). None is skipped, `#[ignore]`d or weakened beyond what is listed.

## SPEC GAPS

### Tests not ported
- `packages/engine/test/pauses.test.ts:354` "§9.3 resumes a three-step sequence at the step after the one
  that asked" and `packages/engine/test/pauses.test.ts:374` "§9.3 finishes a pause nested inside an owed
  sequence before the sequence's own tail". Both drive the file's fixture engine sequence (`driveSequence`,
  hook `__pausesTestSequence`) and then answer, so `drainWork` must run the owed `__pausesTestSequence`
  item; only TS's `registerWorkHandler(SEQUENCE_HOOK, …)` (`pauses.test.ts:145`) made that hook runnable.
  SURFACE §6.6 drops the registration hooks (`work.rs` dispatches by a `match` over the engine's own
  hooks, then a card's own continuation), so a test cannot add an engine sequence; an owed item under an
  unknown hook panics "no handler for owed work". The R113 ordering these two prove for an engine sequence
  is proved for card continuations by the file's other tests. Part 31: either a testkit hook to register a
  test-only work handler (thread-local, like the registries) or a fixture sequence the dispatcher knows.

### Tests adapted, every assertion kept
- `packages/engine/test/pauses.test.ts:577` "§9.3 stops a sequence that keeps owing work instead of
  spinning": TS registered `__pausesTestLoop` whose handler owed its own item again. Ported with a fixture
  card (`pz-looper`) whose `resume` step "again" owes that same continuation again; the drain must still
  panic "did not drain in {MAX_WORK_STEPS} steps".
- `packages/engine/test/pauses.test.ts:565` "§9.3 throws rather than drop work whose sequence registered no
  handler": TS cleared the default handler (`registerDefaultWorkHandler(undefined)`, `:569`) for the test.
  Rust has no registry to clear; the default arm runs a card's own step only when its script has that
  step, and the item names no card, so the same panic is asserted with the default arm in place.
- `packages/engine/test/modifiers.test.ts:128`, `:134`, `:192`: the fixtures' delayed steps are
  `Script.activate` with `Resume.hook` "activate". SURFACE §7.2 does not port `Script.activate`, and
  `Script::hook_named` has no "activate"; the port uses `Script.delayed` and hook "delayed" (TS's
  `DELAYED_HOOK`, the name the engine's own delays use). Every assertion is unchanged.

### Assertions written as far as the frozen types allow
- `packages/engine/test/lastBoards.test.ts:53`: the input `{ defId: "fx-5" } as never` has no `radiant`; a
  `LastBoardEntry` requires one (part 1's frozen `state.rs`), so the port passes `radiant: false`, the face
  the freeze writes for it. The extra `damage: 4` key on `:52` is kept in the JSON input (serde drops it, as
  the freeze does). The expected frozen boards are TS's.
- `packages/engine/test/lastBoards.test.ts:87`: p1's list also holds `null as never` and `{ radiant: true }
  as never` (no `defId`), which a `LastBoardInput` cannot hold. The port freezes the entries it can hold
  (`nope`, `FUSED`, and p2's digest entry) and expects TS's `{ p1: [{ defId: FUSED, radiant: true }] }`; the
  freeze's tolerance of malformed entries is untested in Rust (a typed input cannot carry them; a JSON
  boundary that parses into `LastBoardInput` would reject them before the freeze).
- `packages/engine/test/lastBoards.test.ts:253`: `expect(registeredScripts()[NESTED]).toBeDefined()`. Rust
  registers no fused scripts (SURFACE §6.6: composed on lookup from `state.transient_defs`), so there is no
  registry entry to find; the port asserts the transient definitions (TS's lines above) and calls
  `script_of(&state, NESTED)`, which must compose without panicking. Whether the composed scripts are
  non-empty cannot be told here: the ingredients are vanilla fixture units with no scripts.
- `packages/engine/test/papaya.test.ts:294`, `:304`, `:306`, `:260`, `:331`: `card.zone.z` read off live
  TS objects after the curve resolved. Ported by re-reading each card by id (`find_instance(..).zone.z()`);
  for the token that ceased to exist (`:294`, "gone") the card is asserted absent from every zone.
- `packages/engine/test/papaya.test.ts:337`, `:339`, `:340`: `cardsOnCurve({ state, controller: "p1" }, …)`
  takes TS's `Pick<EffectContext, "state" | "controller">`; ported as `cards_on_curve(&state, P1, …)`.
- `packages/engine/test/perfectHand.test.ts:81`: `ALL_MANA = Number.POSITIVE_INFINITY` is `i32::MAX / 2`
  (an `i32` has no infinity; any amount at least the max refreshes fully, R364).
- `packages/engine/test/heroPower.test.ts`, `pauses.test.ts`: object-identity `toBe` on cards and work items
  is value equality (`assert_eq!`).

No test reads a `.ts` source file, so none was dropped under #133's rule.
