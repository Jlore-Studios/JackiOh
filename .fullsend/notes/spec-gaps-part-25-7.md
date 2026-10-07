# Spec gaps: part 25, chunk 7 (engine tests 2: state check, stays, traps, triggers, Twice Forward)

TESTS-IN: 62 of 62 TS `it`s ported (statecheck 16, stays 7, trap-cardresolved 4, trap-window-pause 4,
trigger-zones 8, triggers 8, twiceForward 15); 0 dropped.

## SPEC GAPS

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
