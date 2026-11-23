# `jackioh-run`

The dungeon run (`docs/dungeon-run.md`): a seeded map of fights and tiles strung around plain games, as
the engine's `duos` wraps two of them. No card and no rule of a game knows about a run. This file is
the contract other code relies on; the plan is the design until the mode has its spec section, and
when the two disagree the plan wins and this file is the bug.

It is pure (CLAUDE.md rule 4): it takes only `serde`, `serde_json`, the engine and, behind its `ts`
feature (off by default), `ts-rs`; its `clippy.toml` is the other pure crates' byte for byte, and every
random draw goes through the engine's `Rng`, rebuilt from the run's seed and cursor. Every number is a
constant in `src/config.rs` (rule 9), named by the plan's section.

## Entry points

| Function | Answers |
| --- | --- |
| `run_seed(source)`, `run_daily_seed(date, n)` | a seed of 6 capital letters: from any text (a number the client rolled), or the day's nth run, the same for everyone (DR2) |
| `run_check_setup(&RunSetup, &RunUnlocks, &RunContent)` | why a starting deck and its artifacts are refused, or `Ok` (DR3) |
| `run_create(&RunSetup, &RunUnlocks, &RunContent) -> Result<RunState, _>` | a run at its start: the setup checked, the map dealt from the seed |
| `run_reduce(&RunState, &RunContent, &RunAction) -> Result<RunState, _>` | the run after one action, or why it is refused, the state unchanged |
| `run_fold(&RunFile, &RunContent)` | a run file replayed from its start: how a Campaign run is checked before it pays (DR1) |
| `run_node`, `run_enemy` | the node stood on, and the enemy of the fight being played (what a fight's setup needs) |
| `run_next_cols(&RunState)` | the next row's nodes `Go` may step onto, by index: every node of the first row before the first step, then those the node stood on leads to (its `next`, R3583000) |
| `run_max_hp`, `run_curse_points`, `run_artifact_points` | the run's max HP (`RUN_MAX_HP` plus the wins', campfires' and events' bonus and the passives'), its curse points, an artifact's points |
| `run_score`, `run_score_terms`, `run_embers` | the score so far, its five sums before the percents (`RunScoreTerms`), and the Embers a score pays (DR9) |
| `run_placeholder_content()` | pseudo cards and temporary bosses, artifacts, blessings, events and curses (`src/placeholder.json`) until the designer's lists land |
| `run_catalog_content(&CardDefs)` | the content a run on the cards that ship deals from: every card a deck may hold (`deckable`), the starting cards `RUN_STARTER_CARDS`, and the placeholder's enemies (their pseudo boss cards dropped), passives, events and curses (DR14) |
| `run_fight_setup(&RunState, &RunContent) -> Option<RunFightSetup>` | in `RunPhase::Fight`, the game to start: its seed (`"{run seed}-{row}-{col}"`), both decks, the human's marks (Radiant and buffed cards) and both handicaps, the human's with the run's HP and each Combat artifact held, once per level, the enemy's its tier's row (`RUN_ELITE_TIERS`' for an elite, else `RUN_TIERS`', its health among it, R3586001) and each curse taken (DR5, R2922001); `None` in any other phase or at a tier no table holds |
| `run_fight_report(&ReplayInput, PlayerId) -> Result<RunAction, _>` | the `RunAction::FightOver` a finished game sends for that seat, folded from its record: the win, the hero's final health, the damage dealt to the enemy's side, the healing on its own, its draws and the turns; a game with no result, or a setup the engine refuses, is refused |
| `run_level_cost(level)` | the Embers from `level` to the next (`RUN_LEVEL_EMBERS`), `None` at the ladder's top |
| `run_constants()` | the numbers the screens show (the slot ladder, the starting deck's size, its artifacts' count and points, the tiers' points, the max HP a campfire adds, the cards a campfire removes, the named preset starting decks), so the client states none (rule 9) |
| `run_shop_prices(&RunState, &RunContent) -> RunShopPrices` | what a shop's services cost this run, the ones `apply` spends: the remove price after the shop discount, the Radiant and buff Tickets, each held relic's next level (`None` at `RUN_ARTIFACT_LEVEL_MAX`) and each stock item's Coins after the discount |

## What the caller does

- **Refusals.** A refusal is a sentence the Sandbox's screens show word for word, so it says "relic"
  where the code says artifact (`apps/web/WRITING.md`'s Plain table). A `Go` onto a node the one stood
  on does not lead to is refused with "no path leads there from here" (R3583000).
- **Content.** The caller passes a `RunContent`: the pool, the starting cards, the enemies, the passives,
  the events and the curses.
- **Unlocks.** The caller keeps `RunUnlocks` (the level, the extracted cards and artifacts, and in the
  Campaign the account's collection) and adds a finished run's `extracted_cards` and
  `extracted_artifacts` to it, and its `run_embers(run_score(…))`.
- **Fights.** In `RunPhase::Fight` the caller starts the game from `run_fight_setup` (the human P1, the
  enemy P2), plays it against `run_enemy`, and sends the `RunAction::FightOver` that `run_fight_report`
  folds from the finished game: the hero's health at the end and what the score reads (damage dealt,
  healing done, cards drawn, turns). A won fight adds `RUN_WIN_MAX_HP` to the max and refills HP to it;
  a lost one drops HP to zero and ends the run.

## What it does not do yet

A ghost's deck is random until snapshots exist (M7),
and the extended campaign is not built (M8). The starting cards, the enemies, the artifacts, the
blessings, the events and the curses are placeholders until the designer's lists land (M9). CI holds the
generated `Run*.ts` to the Rust: `jackioh-wasm-core`'s dev-dependencies turn on `ts`, so the `tools, wasm`
group's `cargo test --workspace` writes them and `ci.yml`'s V20 step fails on a diff (#4426).

## Tests

`src/tests.rs`, on the placeholder content: seeds, the map's acts and lone bosses, a step onto a node
the one stood on does not lead to refused and a log holding one that does not fold (R3583000), enemy
decks, the starting deck's slots, ownership and artifact points, a whole run played out and its file folded back, extraction,
packs, campfires, shops, the score and the placeholders' fight effects. `src/fight_tests.rs`, on the
real catalog (`jackioh-cards`, a dev-dependency): the catalog's content, the starting cards, a fight's
setup and seed, the curses and Combat artifacts in it (R2922001), a scripted game's report and the
levels' costs (DR14's B5 to B11). `src/map_tests.rs` deals `MAP_SEEDS` maps on each content and checks each as a
graph: its shape, its edges (none crossing, none wider than a column but a boss's fans), its reach (no
orphan, no dead end) and every route's fights and tiles, by dynamic programming over the whole graph
(R3583000, R3583001). `src/walk_tests.rs` walks whole seeded runs on both contents,
checking bounds, fight setups, each won route's fights and replay. `src/map_fuzz_tests.rs` walks
random-policy map-only runs with stubbed fight reports, firing one illegal action per step and
checking each refusal leaves the state untouched, each acceptance keeps the bounds and each log
replays. `cargo jackioh run-sim` (#3065) plays whole runs' fights to
the end with the AI. The `ts` feature writes the public types to
`apps/web/src/wire/generated/`, which `apps/web/src/wire/run.ts` re-exports as `@jackioh/run`; commit
any diff. `cargo test --workspace` writes them too (`jackioh-wasm-core`'s dev-dependencies turn the feature on),
so a stale file fails CI.

```
cargo test -p jackioh-run
cargo test -p jackioh-run --features ts
cargo clippy -p jackioh-run --all-targets -- -D warnings
```

## The Infinite Assault

The Infinite Assault (`docs/infinite-assault.md` IA0 to IA6, module `src/assault/`): a daily
20-level gauntlet beside the dungeon run, sharing its seeds, its `Rng` use and its fight report.
Every UTC day deals 20 levels, Commanders on 5, 10, 15 and 20, each one game against the AI.

| Function | Answers |
| --- | --- |
| `assault_day_seed(date)` | the day's seed (`assault-YYYY-MM-DD`), or why the date is refused (IA1) |
| `assault_catalog_content(&CardDefs)` | the content the mode deals from: the set's cards by set and tag, with `src/assault.json`'s doctrines and Commanders (IA4) |
| `assault_placeholder_content(&CardDefs)` | the content dealt until the release: legal decks from shipped cards standing in for each doctrine, with `assault.json`'s twelve stand-in Commanders (IA4) |
| `assault_day(date, &AssaultContent)` | the day's deal: its 20 levels, each with its enemy, doctrine, escalations, upgrades, slots, deck, marks and handicap (IA1, IA2, IA4) |
| `assault_create(&AssaultSetup, &AssaultRecord, &AssaultDay)` | an attempt at its start: the setup checked against the day and the record, or why it is refused (IA6) |
| `assault_reduce(&AssaultState, &AssaultDay, &AssaultAction)` | the attempt after one action, or why it is refused, the state unchanged (IA6) |
| `assault_record(&AssaultRecord, &AssaultState)` | the day's record folded with a finished or running attempt: attempts, best, checkpoints and the first repel (IA6) |
| `assault_fold(&AssaultFile, &AssaultDay)` | an attempt file replayed from its start, the error naming the first refused step (IA6) |
| `assault_fight_setup(&AssaultState, &AssaultDay)` | in `Fight`, the game to start: its seed (`{day seed}-L{level}-A{attempt}`), both decks, the human's `HUMAN_HANDICAP` and the level's handicap and marks (IA6) |
| `assault_fight_report(&ReplayInput, PlayerId)` | the `AssaultAction::FightOver` a finished game sends for that seat, folded from its record: a draw at the turn cap is a loss (IA6, IA15) |
| `assault_constants()` | the numbers the screens show (rule 9) |

What the caller does:

- **Content.** Deal from the placeholder (`assault_placeholder_content`) until the release, when
  the browser deals from the catalog's content instead. The file's set and tags are read by name,
  so the mode compiles before C1 lands them.
- **Records.** Start a day's first record as `AssaultRecord { date, ..Default::default() }`, with
  no attempts yet. Fold the state into the record after create and after each action, with
  `assault_record`. Keep the last `ASSAULT_HISTORY_DAYS` days on the device.
- **Attempts.** An attempt starts at level 1 or at a checkpoint the record holds (6, 11 or 16),
  one higher than each beaten Commander. Its deck is chosen per attempt and kept for the whole
  attempt. Leave mid-fight keeps the fight, as a dungeon fight does.
- **Fights.** In `Fight` the caller starts the game from `assault_fight_setup` (the human P1, the
  enemy P2, the enemy marks as P2's), plays it, and sends the `AssaultAction::FightOver` that
  `assault_fight_report` folds from the finished game.
- **Files.** Keep the `AssaultFile` with the catalog version the caller folds under, as a dungeon
  run's file does: nothing here reads it.

What is not built yet (Open question 1): the set's cards cannot start a game yet. `createGame`
refuses cards of a held or single-player set in any deck, so the decks dealt from
`assault_catalog_content` cannot start one until an engine change (C1, the AI part or the
release) lets an Assault enemy's seat hold them. Part A tests `createGame` only on the
placeholder.

The tests and the commands: `src/assault/tests.rs`, on synthetic content, holds the seed, the
determinism, the shape over a year of days, the escalation, the attempt and the fold
([[R4579000]], [[R4579001]], [[R4579002]]); `src/assault/fight_tests.rs`, on the real catalog,
holds the placeholder, the set read by name, every level of a year on `createGame`, every level
of a week folding back to its hash, and the draw reporting a loss ([[R4579001]], [[R4579002]]).

```
cargo test -p jackioh-run assault
cargo test -p jackioh-run --features ts
```
