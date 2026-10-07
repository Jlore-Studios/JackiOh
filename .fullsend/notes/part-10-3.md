# Slice: part 10 (cards lane 2: Core #49–#81), chunk 3 of 4 (#398): Core #66–#77
BUILDS-RUN: 0

No `cargo`, `rustc`, `rustfmt`, `clippy`, `pnpm`, `tsc` or test runner was run. One `python3` scratch
script (outside the repo) re-indented the tests of #66–#68 into their `describe`'s `mod`.

## FILES
Each holds the whole port of its TS script (every function in TS order, the header as `//!`, every
rule- or ruling-citing comment) and, below it, its TS test file as `#[cfg(test)] mod tests` (the test
header kept above it, one `mod` per `describe`, one `#[test]` per `it`, ruling tokens leading):
- `crates/cards/src/scripts/core/c066_the_rock.rs` — the night bot's half-done port (0180138), finished
  in place (see Decisions).
- `crates/cards/src/scripts/core/c067_zoomerbin_oomen.rs` — new; uses `crate::query::{CardQuery, TRAP_TYPES}`.
- `crates/cards/src/scripts/core/c068_twisted_sorcerer.rs` — new.
- `crates/cards/src/scripts/core/c069_call_to_arms.rs` — new.
- `crates/cards/src/scripts/core/c070_spiteful_stab.rs` — new.
- `crates/cards/src/scripts/core/c071_intern_stimmy.rs` — new.
- `crates/cards/src/scripts/core/c072_reminisce.rs` — new.
- `crates/cards/src/scripts/core/c073_anti_oneshot_armor.rs` — new.
- `crates/cards/src/scripts/core/c074_adaptive_ui.rs` — new.
- `crates/cards/src/scripts/core/c075_infinite_reserves.rs` — new.
- `crates/cards/src/scripts/core/c076_field_of_dreams.rs` — new.
- `crates/cards/src/scripts/core/c077_professor_curvature.rs` — the night bot's half-done port, finished.
No `todo!`, `unimplemented!` or `// TODO` in any of them. No function or `it` left out.

## SURFACE
Every file: `pub const ID`, `pub fn script() -> CardScripts` (SURFACE §7.1). Nothing else is public.

## DEPENDS-ON
Called by TS name snake_cased at the TS module's Rust path (rule 6), with these shapes:
- Effects (`jackioh_engine::effects`, imported by name so a glob collision cannot shadow them), each
  taking its argument struct built with `json_as(json!({ …TS literal… }))`: `summon_random`, `damage`,
  `recruit` (+ the named type `RecruitFilter`, `Deserialize + Serialize`), `discover_from_graveyard`,
  `exile`, `add_to_hand`, `set_cost_mod`, `set_cost_override`, `draw`, `heal`, `summon`,
  `discard_hand`, `add_player_modifier`.
- Read helpers through the prelude: `zones::{slot_of(&GameState, &CardInstance) -> Option<ZoneSlot { lane: i32, .. }>,
  OffFieldZone::{Hand, Library, Exile}}`, `query::{hero_of(&GameState, PlayerId) -> HeroView { health, .. },
  zone_count(&GameState, PlayerId, OffFieldZone) -> i32}`, `wire::opponent_of`, `config::HERO_HEALTH`.
- Script types (part 1, frozen): `hook`, `condition_hook` (for `condition_met` and `preview`),
  `TriggerDef::new(..).with_when(..)`, `TargetDecl::target`, `StaticFlags`, `PreviewValue`,
  `ConditionZone::Hand`, `FaceKind`, `CardDef::face`, `EffectContext::live_self`.
- Cards crate: `crate::register_all()`, `crate::card_def(&str) -> CardDef` (part 1's lib.rs),
  `crate::query::{CardQuery (= CatalogQueryArgs, Serialize + Deserialize), query(&CardQuery) ->
  Vec<&'static CardDef>, TRAP_TYPES: &[CardType]}` (part 9.1's notes).
- Testkit (part 5.1's notes): `scenario(Value) -> Scenario`; steps and assertions returning
  `&mut Scenario`; `unit`/`backrow(seat, lane: i32) -> Option<CardInstance>`, `hand`/`pile -> Vec<CardInstance>`,
  `card(ref) -> &CardInstance`, `stats(ref)`, `state()`, `state_mut()`, `events()`, `last_events()`,
  `view(seat) -> PlayerView`; `expect_refused_with(|s| s.play(..), "text")`; `hand_glows(&Scenario,
  &str, PlayerId)` (glow.rs); `legal_actions(&GameState, PlayerId)`, `find_instance_mut`.

## GAPS
- Nothing left out. The names above are other parts'; the likeliest misses for part 31:
  - `crate::query::CardQuery` must be `Serialize` (#67 writes the pool into `summon_random`'s JSON) —
    part 9.1's notes say `CatalogQueryArgs` is.
  - `effects::RecruitFilter` must be `Serialize` (#69 writes it into `recruit`'s JSON).
  - `effects::summon_random`'s args must accept `{ query, player, lane, radiant }` (TS's literal).
  - `jackioh_engine::testkit::hand_glows(s, instance_id, viewer)` with the viewer explicit (part 5.3).
- No `Scenario::card_mut`: #66's R46 test writes `marked_destroyed` through
  `find_instance_mut(s.state_mut(), &id)` (TS wrote `s.card(ROCK).markedDestroyed = true`).

## Decisions
- The bot's #66 and #77 were kept where right and fixed where they broke the testkit's shapes (part
  5.1): every test registers the shipped cards first (`crate::register_all()`, the TS globalSetup's
  job; folded into each file's `board`/`setup` helper); `expect_refused*` closures return the
  `&mut Scenario` (`|s| s.play(..)`); `unit()` is already owned (no `.cloned()`); `s.card_mut` replaced
  as above; TS chains kept as chains; `toMatchObject` as a recursive `matches_object` on JSON; the R69
  test name carries the whole TS title; #66–#68's tests wrapped in their one `describe`'s `mod`.
- TS regexes as hand checks (SURFACE §8, no regex crate): `/[Tt]ribute/` → contains `"ribute"`;
  `/target/i` → contains `"target"` (the engine's refusals write it lower-case); `/X must be at least 1/`
  is literal.
- Views, events, prompts and legal actions are compared as JSON (`serde_json::to_value`) wherever TS
  compared against an object literal or read a field off a union member (`e.type === "summoned" &&
  e.row === "backrow"`, `pending.options[i].selection.instanceId`, `action.x`), so the tests depend on
  the wire shape TS pinned, not on Rust variant or field names.
- A TS module constant built from the catalog (`cardDef("core-t-ghoul").id`, `cardDef("core-072").id`)
  is a plain `const &str` (part 11's convention); a module constant `targets: TargetDecl[]` is a private
  `fn targets()`; `FACES = { base, radiant }` (#70) is two `const Stab`s picked by `FaceKind`;
  #74's `per` object is a private `Per` struct; #72's `price: () => Effect` is a generic `Fn` parameter.
- #70's preview label (`def[face].text`) is read once in `script()` from `crate::card_def(ID).face(face)`.
- #71's `TrapTrigger` is a plain `TriggerDef` (`with_when`), since part 1's `TriggerDef` carries `when`;
  its header comment says so instead of "typed as TrapTrigger. Reported."
- #67 reads `ctx.live_self()` for `slot_of` (TS read the live `ctx.self`), as parts 6–7 decided.
- Integer arithmetic: #70's `Math.floor(missing / step)` is `/` on a non-negative `i32` (§4.4.4);
  #74's `Math.max(0, Math.trunc(ctx.x))` is `ctx.x.max(0)`.
- Seeds, fixtures and expected numbers are TS's, unchanged.
