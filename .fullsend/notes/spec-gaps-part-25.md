# Part 25: spec gaps (all chunks)

TESTS-IN: 627 (the sum of the chunks below)

## SPEC GAPS

### From `spec-gaps-part-25-1.md`

#### Spec gaps: part 25, chunk 1 (engine tests 2: activate, aiPolicy, audit, boardHistory, callToChaos, callToChaosPlus, comboIndex)

TESTS-IN: 122 of 122 (`it`s ported one `#[test]` each; 20 `describe`s, one `mod` each). No test dropped, skipped or `#[ignore]`d.

#### SPEC GAPS

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

### From `spec-gaps-part-25-2.md`

#### Spec gaps: part 25 (engine tests 2), chunk 2 of 7
TESTS-IN: 109 (copied_text 20, core_patches 15, death_pause 8, delayed_kinds 11, fuse_registry 5,
fuse_variants 17, fuse 18, glitch 15), every TS `it` ported; none dropped.

#### SPEC GAPS
Assertions TS states that SURFACE.md makes inexpressible as written. Each test is ported whole; only
the assertion below is restated in Rust's terms, and why.

- `packages/engine/test/fuse-registry.test.ts:103` (`markers`): reads `registeredScripts()[defId]`
  directly. SURFACE §6.6: a fused id is never registered; its scripts are composed on lookup from the
  state (`scripts::script_of`). Rust's `markers` reads `script_of(state, def_id)`.
- `fuse-registry.test.ts:156` `expect(markers(copy, id)).toEqual([])` and `:157`
  `syncFusedScripts(copy)`: `syncFusedScripts` is not ported (SURFACE §6.6), and `script_of` composes
  the fusion at every lookup, so "nothing before the rebuild" cannot happen. Restated as
  `registered_scripts().get(id).is_none()` (the precondition TS's `[]` showed: the registry holds
  nothing for the id); the post-sync assertion runs unchanged off `script_of`.
- `fuse-registry.test.ts:165` `expect(markers(first, id)).toEqual([])` after `drop(id)`: the same,
  restated as `registered_scripts().get(id).is_none()`.
- `fuse-variants.test.ts:135` and `:140` `fusedIdSpecs(id)`: TS's answer changed from `null` to the
  specs once `legalActions` had taught the process's digest table the id. No digest table in Rust
  (SURFACE §6.6; catalog readers take `Option<&GameState>`, part 2.2): `:135` is
  `fused_id_specs(None, &id)` (no state: a digest is no fused id) and `:140` is
  `fused_id_specs(Some(&round), &id)`. `:142`/`:154` `scriptsFor(id)` is `script_of(&round, id)`.
- `fuse.test.ts:455`, `:488`, `:591`, `:592` and `fuse-variants.test.ts:223`
  `expect(<ingredient>.zone).toEqual({ z: "gone", player })`: TS read the live object `fuse` set to
  `gone`. Rust's `FuseArgs` takes owned copies and a gone card is in no pile, so nothing holds its
  zone after the call. Restated as `find_instance(state, id).is_none()` (in no pile at all, R86); the
  `player` of the gone zone is not observable.
- `glitch.test.ts:68`, `:69`, `:83`, `:95` `pickGenerated(rng, pool, { systemPlays })` and `:177`
  `seatPlayedBy({ seatSwaps: 2 }, "p1")`: TS passed bare object literals; part 2.2 made `GlitchOdds`
  the state and SURFACE §6.1 gives `seat_played_by(&GameState, ..)`. Each passes a clone of the test's
  state with only that field set (`system_plays`, `seat_swaps`).

### From `spec-gaps-part-25-3.md`

#### Spec gaps: part 25, chunk 3 (engine tests 2: heroPower, kyTest, lastBoards, modifiers, papaya, pauses, perfectHand)

TESTS-IN: 115 of 117 (`it`s ported one `#[test]` each; 22 `describe`s, one `mod` each). Two tests of
`pauses.test.ts` are not ported (below). None is skipped, `#[ignore]`d or weakened beyond what is listed.

#### SPEC GAPS

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

### From `spec-gaps-part-25-4.md`

#### Spec gaps: part 25, chunk 4 (engine tests 2: prompt-kinds, prompts, quests)

TESTS-IN: 72 of 72 (`it`s ported one `#[test]` each: prompt-kinds 27, prompts 18, quests 27; 16 `describe`s,
one `mod` each). No test dropped, skipped or `#[ignore]`d. No test reads a `.ts` source file.

#### SPEC GAPS

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

### From `spec-gaps-part-25-5.md`

#### Spec gaps: part 25, chunk 5 (engine tests 2: rulings-a, rulings-b)

TESTS-IN: 82 of 82 (`it`s ported one `#[test]` each: 40 from `rulings-a.test.ts`, 42 from
`rulings-b.test.ts`; 2 `describe`s, one `mod` each). No test dropped, skipped or `#[ignore]`d. Neither
file reads a `.ts` source, so nothing was dropped under #133's rule.

#### SPEC GAPS

Assertions that could not be written exactly as TS wrote them, each kept as far as Rust can say it:

- `packages/engine/test/rulings-b.test.ts:1570` (R79, "leaves the clocks to the server"):
  `expect(Object.keys(engineConfig).filter((key) => SERVER_CONSTANTS.includes(key))).toEqual([])` —
  that `config.ts` exports none of the 27 server constant names. Rust has no run-time list of a module's
  items, and reading `crates/engine/src/config.rs` as text is the source-reading #133 drops. **Not
  ported**; the rest of R79 (both timeouts, the disconnect loss, the ceiling draw, `clockMs` with and
  without a clock) is. `SERVER_CONSTANTS` itself, which fed only this assertion, is not ported either;
  `rulings_b.rs` says why where it stood. A structural check (`cargo jackioh spec check`, part 28, or a
  grep in CI) could hold the same line.
- `rulings-b.test.ts:1292` (R73, "health and armor are separate fields"):
  `expect(Object.keys(state.players.p1.hero)).toEqual(["health", "armor"])`. The key *order* is a JS
  object's declaration order; serde_json without `preserve_order` hands back sorted keys. Ported as an
  exhaustive destructuring `let HeroState { health: _, armor: _ } = …` (exactly these two fields, checked
  by the compiler) plus the serialised hero's sorted key list `["armor", "health"]`. Field order is not
  asserted.
- `rulings-b.test.ts:1370` (R75): `typeof def.set === "string" && def.set.length > 0` — the
  `typeof` half is the type itself in Rust (`set: SetName`); ported as `!def.set.as_str().is_empty()`.

### From `spec-gaps-part-25-6.md`

#### Spec gaps: part 25, chunk 6 (engine tests 2: rulings-c, scorer, start-of-opponent-turn)

TESTS-IN: 65 of 65 (`rulings-c.test.ts` 55 `it`s in 15 `describe`s, `scorer.test.ts` 9 in 1,
`start-of-opponent-turn.test.ts` 1 in 1; one `#[test]` per `it`, one `mod` per `describe`). No test
dropped, skipped or `#[ignore]`d. No test reads a `.ts` source file, so none was dropped under #133's rule.

#### SPEC GAPS

Assertions that could not be written exactly as TS wrote them, each kept as far as Rust can say it:

- `packages/engine/test/rulings-c.test.ts:1161` (R102 multi-target): `expect(shared.zone).toEqual({ z:
  "gone", player: "p1" })` reads the TS object the test still held after the fusion consumed it. A Rust
  test holds a copy; a card that is in no pile cannot be read back from the state, and `fuse` returns only
  the kept instance. The two assertions after it (`findInstance(...) === undefined`, the lane empty) are
  kept and say the same thing from the state's side. The copy handed to the second fusion has its zone set
  to `gone` by the test, as TS's shared object had been by the engine.
- `packages/engine/test/rulings-c.test.ts:1190` (R102 consumed): `expect(eaten.zone).toEqual({ z: "gone",
  player: "p1" })`, the same: ported as `find_instance(&state, &eaten.id).is_none()` beside the kept
  lane, graveyard and exile assertions.
- `packages/engine/test/rulings-c.test.ts:2154-2155` (R135): `expect(Object.keys(effects)).toContain(
  "exileMatching")` / `"drawFromLibrary"`. Rust has no runtime list of a module's exports; the port names
  `effects::exile_matching` and `effects::draw_from_library` as values, which compiles only if both verbs
  exist (a compile-time form of the same check).
- `packages/engine/test/rulings-c.test.ts:1854` (R126): `typeof scriptsFor(...).base.resume?.later ===
  "function"` is `resume.contains_key("later")`: every value of `Script.resume` is a `Hook`.
- `packages/engine/test/rulings-c.test.ts:2444` (R151): `expect(powerOf(card)).toBe(kept)` is an identity
  check on a `HERO_POWERS` table entry. Ported as the same power by name (compared as JSON) and `x`.
- `packages/engine/test/rulings-c.test.ts:2517, 2524` (R155): TS pushed one object into the graveyard (or
  exile) without taking it out of the hand, so one card sat in two piles and a flag written to it showed
  in both. Rust cannot alias it: the hand copy's zone is set and a copy pushed to the second pile; the
  assertion checks that no copy of that id in hand, graveyard or exile is flagged.
- `packages/engine/test/rulings-c.test.ts:1345-1346` (R113): `expect(() => runWorkItem(...)).toThrow(/…/)`
  is a `catch_unwind` around the call (SURFACE §4.4.9: a TS throw on an impossible state is a `panic!`
  with the same message), matched with `contains`. `not.toThrow()` (lines 1856, 1875, 1906) is the call
  itself: a panic fails the test.

### From `spec-gaps-part-25-7.md`

#### Spec gaps: part 25, chunk 7 (engine tests 2: state check, stays, traps, triggers, Twice Forward)

TESTS-IN: 62 of 62 TS `it`s ported (statecheck 16, stays 7, trap-cardresolved 4, trap-window-pause 4,
trigger-zones 8, triggers 8, twiceForward 15); 0 dropped.

#### SPEC GAPS

No test was dropped. These assertions are written against the nearest Rust observable, none weaker on
what the engine does to the state:

- `statecheck.test.ts:638`: `expect(() => stateCheck(sinkFor(state))).toThrow(/did not settle in 100
  passes/)`. SURFACE §4.4.9 makes a TS throw on an impossible state a `panic!` with the same message,
  so the port catches the panic (`std::panic::catch_unwind`) and checks the message contains the text.
  If part 31 makes the cap a returned `EngineError` instead, the assertion becomes a match on it.
- `twiceForward.test.ts:135`: `expect(second.card.id).toBeDefined()` — a Rust `String` is always
  defined; ported as "the id is not empty".
- `trap-window-pause.test.ts:284`: `toEqual({ fired: [], paused: true })` on a `TrapDispatch` compares
  its two fields. `trap-cardresolved.test.ts:235`, `:246` and `trigger-zones.test.ts:216`, `:241`, `:287`,
  `:309`, `:316`, `:326`: TS's `toEqual([])` on `trapsWatching(...)` and `triggerHoldersWithHook(...)`
  (lists of structs holding scripts, which have no `PartialEq`) is `.is_empty()`.
- `trap-window-pause.test.ts:280`: `resumeAt({ defId, step })` takes TS's argument object; SURFACE fixes
  no Rust name for that anonymous type, so the port builds it with `json_as(json!({ "defId", "step" }))`,
  which needs the argument type to derive `Deserialize` (camelCase). Part 31: if it does not, a struct
  literal of whatever part 3 named it.
- `twiceForward.test.ts:21` imports `TWICE_FORWARD_SCRIPTS` (a TS `const` record of closures); Rust calls
  it `fixtures::twice_forward::twice_forward_scripts()`, since closures cannot be a `const`. Part 24's
  fixture file must use that name (or part 31 renames the call).

