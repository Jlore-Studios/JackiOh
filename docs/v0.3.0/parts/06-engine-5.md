# v0.3.0 (part 6 of 40): engine 5: effect verbs, first half

Part 6 of 40 of #306 (the v0.3.0 Rust rewrite). The plan, the interface freeze and the port map are in `docs/v0.3.0/` (on `staging` once part 1 has run; until then on the branch `claude/relaxed-archimedes-i5fdah`): [README.md](../README.md) (how the run works), [SURFACE.md](../SURFACE.md) (every boundary), [PORT-MAP.md](../PORT-MAP.md) (every file), and the research inventories written for this plan: research A = [`research/A-engine.md`](../research/A-engine.md), B = [`B-ai.md`](../research/B-ai.md), C = [`C-server.md`](../research/C-server.md), D = [`D-ci-spec.md`](../research/D-ci-spec.md).

| | |
|---|---|
| Wave | Wave 1 (full send) |
| Starts after | part 1 |
| Branch | `staging` (push straight to it; no pull request) |
| Builder | a Claude Code cloud session, Sonnet or stronger |
| Notes | `.fullsend/notes/part-06.md` and `.assumptions` (SURFACE §16) |

Unlabelled on purpose: #306 holds labels until a person says the run is ready. The Plan below is between the night bot's plan markers, so if a person later queues this issue for a bot, it builds from this plan without planning again.

<!-- jackioh-bot:plan -->
## Plan

_Written by Claude (Opus 5.5) for #306's planning session. Edit it here to change it._

### 1. Goal

Port the first half of the effect verbs (`effects/*`, alphabetical, as the table lists) from TypeScript to Rust, file for file (the table below), by SURFACE.md §4's rules, so that every function keeps its name (snake_cased), its module, its behaviour and its RNG draws. These modules are called by every other engine part, the cards and the AI by the names §4.2 gives them.

**How you work (fullsend builder rules, in priority order; full text in `.claude/skills/fullsend/agents/builder.md`).**
1. Never run `cargo`, `rustc`, `rustfmt`, `clippy`, `pnpm`, `tsc` or any test runner. Nothing on `staging` compiles until Wave 3; every error you would see is someone else's unwritten file. Count any you ran as `BUILDS-RUN`.
2. Write only the files in your Files to touch table and your two notes files. Read anything in `packages/`, `apps/`, `docs/`, `SPEC.md` (the TypeScript is your spec); do not read other parts' Rust under `crates/`: it is half-written.
3. Match SURFACE.md exactly at every boundary: paths by §4.1, names by §4.2, types by §4.3, the shapes of §5–§14.
4. No stubs: no `todo!()`, `unimplemented!()`, placeholder bodies or `// TODO`. Port the real body; if you truly cannot, leave the function out and list it under GAPS.
5. Duplicate on purpose: a small helper another part probably writes, write your own private copy.
6. Call what you wish existed: another module's function by its TS name snake_cased at its TS module's Rust path.
7. Don't ask questions: decide, and record the decision in your notes.
8. Stop when Done when holds.

**Pushing.** README §3.2: `git fetch origin staging && git checkout -B work origin/staging`, write, commit only your files, `git pull --rebase origin staging && git push origin HEAD:staging` (retry with back-off). Commit often. A session that is cut off leaves its work on `staging`; the next session for this part starts at the first file in the table that is missing or empty.

### 2. Files to touch

| File | new/change/delete | What |
|---|---|---|
| `crates/engine/src/effects/add_to_hand.rs` | new | port of `packages/engine/src/effects/addToHand.ts` (186 lines). |
| `crates/engine/src/effects/after_check.rs` | new | port of `packages/engine/src/effects/afterCheck.ts` (80 lines). |
| `crates/engine/src/effects/animate.rs` | new | port of `packages/engine/src/effects/animate.ts` (34 lines). |
| `crates/engine/src/effects/brittle.rs` | new | port of `packages/engine/src/effects/brittle.ts` (68 lines). |
| `crates/engine/src/effects/buff.rs` | new | port of `packages/engine/src/effects/buff.ts` (184 lines). |
| `crates/engine/src/effects/card_scope.rs` | new | port of `packages/engine/src/effects/cardScope.ts` (128 lines). |
| `crates/engine/src/effects/cast.rs` | new | port of `packages/engine/src/effects/cast.ts` (221 lines). |
| `crates/engine/src/effects/choose.rs` | new | port of `packages/engine/src/effects/choose.ts` (753 lines). |
| `crates/engine/src/effects/choose_where.rs` | new | port of `packages/engine/src/effects/chooseWhere.ts` (65 lines). |
| `crates/engine/src/effects/coins.rs` | new | port of `packages/engine/src/effects/coins.ts` (114 lines). |
| `crates/engine/src/effects/combat.rs` | new | port of `packages/engine/src/effects/combat.ts` (295 lines). |
| `crates/engine/src/effects/cost.rs` | new | port of `packages/engine/src/effects/cost.ts` (80 lines). |
| `crates/engine/src/effects/counters.rs` | new | port of `packages/engine/src/effects/counters.ts` (100 lines). |
| `crates/engine/src/effects/cry.rs` | new | port of `packages/engine/src/effects/cry.ts` (41 lines). |
| `crates/engine/src/effects/damage.rs` | new | port of `packages/engine/src/effects/damage.ts` (96 lines). |
| `crates/engine/src/effects/datacenter.rs` | new | port of `packages/engine/src/effects/datacenter.ts` (103 lines). |
| `crates/engine/src/effects/delay.rs` | new | port of `packages/engine/src/effects/delay.ts` (311 lines). |
| `crates/engine/src/effects/destroy.rs` | new | port of `packages/engine/src/effects/destroy.ts` (104 lines). |
| `crates/engine/src/effects/draw.rs` | new | port of `packages/engine/src/effects/draw.ts` (59 lines). |
| `crates/engine/src/effects/draw_while.rs` | new | port of `packages/engine/src/effects/drawWhile.ts` (31 lines). |
| `crates/engine/src/effects/each.rs` | new | port of `packages/engine/src/effects/each.ts` (32 lines). |
| `crates/engine/src/effects/enchant.rs` | new | port of `packages/engine/src/effects/enchant.ts` (47 lines). |
| `crates/engine/src/effects/flicker.rs` | new | port of `packages/engine/src/effects/flicker.ts` (67 lines). |
| `crates/engine/src/effects/fruit.rs` | new | port of `packages/engine/src/effects/fruit.ts` (195 lines). |
| `crates/engine/src/effects/fuse.rs` | new | port of `packages/engine/src/effects/fuse.ts` (450 lines). |
| `crates/engine/src/effects/give.rs` | new | port of `packages/engine/src/effects/give.ts` (125 lines). |
| `crates/engine/src/effects/hand_exile.rs` | new | port of `packages/engine/src/effects/handExile.ts` (49 lines). |
| `crates/engine/src/effects/heal.rs` | new | port of `packages/engine/src/effects/heal.ts` (47 lines). |
| `crates/engine/src/effects/health.rs` | new | port of `packages/engine/src/effects/health.ts` (44 lines). |
| `crates/engine/src/effects/kill_credit.rs` | new | port of `packages/engine/src/effects/killCredit.ts` (49 lines). |

Do **not** touch: any `Cargo.toml`, `lib.rs`, `mod.rs` or `main.rs` (part 1's), other parts' files, `packages/` and `apps/server/` (read only; part 37 deletes them).

### 3. Steps

1. Read SURFACE.md §3–§6 fully, then each TS file in your table (and, read-only, any TS file it imports, to know what it calls).
2. Each TS constructor `name(args) => ({ kind: "…", apply(ctx) { … }, expand?(ctx, memo) { … } })` becomes `pub fn name(args: Args) -> Effect` returning `Effect::new("…", move |ctx| { … })` (plus `.with_expand(…)` where TS has `expand`), SURFACE §6.6.
3. Every argument type (`TargetSpec`, `BoardScope`, `PlayerSpec`, `DamageEffectArgs`, …) derives `Serialize, Deserialize, Clone, Debug, PartialEq` with camelCase serde, and `Default` when all fields are optional, so cards can build them with `json_as(json!(…))` (SURFACE §6.6). Variant names of TS string unions by §4.3.
4. Exported constants (`EXILE_ZONE_ORDER`, `DELAYED_HOOK`, `SWAP_ROWS`, …) stay `pub const` in the same file. Prompt answerers registered by an effect file (plague placement, fuse onto) are plain `pub fn`s that `prompts.rs` calls by name.
5. For each row: write the Rust file. Port every function, exported or not, in TS order; keep the TS header comment as `//!` lines and every inline comment that states a rule or cites a ruling (`R113`, `§10.3`) — `spec check` reads the R-ids (§15).
6. Constants that TS declared at the top of your files now live in `crate::config` (part 1 moved them, SURFACE §6.4); use them from there.
7. A function another module provides is called by its TS name snake_cased at its TS module's path (`crate::zones::place_on_field`), taking `&GameState`, `&mut GameState` or `&mut EngineSink` by §6.5. Don't look at whether it exists yet.
8. Push after every 3–5 files (README §3.2).

### 4. Tests

- Unit tests are not yours: parts 24–27 port the engine's TS tests. Add none.
- Part 32 compiles and tests these files in Wave 3.

### 5. Done when

- [ ] Every destination in the table holds the full port of its TS file (no function missing that TS exported).
- [ ] No `todo!`, `unimplemented!`, placeholder bodies or `// TODO` in your files.
- [ ] `.fullsend/notes/part-06.md` and `.assumptions` committed; `BUILDS-RUN: 0`.

### 6. Risks

- SURFACE §4.4's semantics (stable sorts, insertion-ordered maps, presence of optional fields, RNG order) are where a port goes quietly wrong; the golden traces catch it in Wave 3, at the cost of a debugging session.

<!-- /jackioh-bot:plan -->
