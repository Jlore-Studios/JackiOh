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
| `run_max_hp`, `run_curse_points`, `run_artifact_points` | the run's max HP, its curse points, an artifact's points |
| `run_score`, `run_embers` | the score so far, and the Embers a score pays (DR9) |
| `run_placeholder_content()` | pseudo cards and temporary bosses, artifacts, blessings, events and curses (`src/placeholder.json`) until the designer's lists land |
| `run_catalog_content(&CardDefs, &[CustomCard])` | the content a run on the cards that ship deals from: every card a deck may hold (`deckable`) and the custom cards given, the starting cards `RUN_STARTER_CARDS`, and the placeholder's enemies (their pseudo boss cards dropped), passives, events and curses (DR14) |
| `run_fight_setup(&RunState, &RunContent) -> Option<RunFightSetup>` | in `RunPhase::Fight`, the game to start: its seed (`"{run seed}-{row}-{col}"`), both decks, the human's marks (Radiant and buffed cards) and both handicaps, the human's with the run's HP, the enemy's its Presence's tier with its health (DR5); `None` in any other phase |
| `run_fight_report(&ReplayInput, PlayerId) -> Result<RunAction, _>` | the `RunAction::FightOver` a finished game sends for that seat, folded from its record: the win, the hero's final health, the damage dealt to the enemy's side, the healing on its own, its draws and the turns; a game with no result, or a setup the engine refuses, is refused |
| `run_level_cost(level)` | the Embers from `level` to the next (`RUN_LEVEL_EMBERS`), `None` at the ladder's top |
| `run_constants()` | the numbers the screens show (the slot ladder, the starting deck's size, its artifacts' count and points, the tiers' points), so the client states none (rule 9) |

## What the caller does

- **Content.** The caller passes a `RunContent`: the pool, the starting cards, the enemies, the passives,
  the events and the curses. A Sandbox caller adds the player's custom cards to the pool.
- **Unlocks.** The caller keeps `RunUnlocks` (the level, the extracted cards and artifacts, and in the
  Campaign the account's collection) and adds a finished run's `extracted_cards` and
  `extracted_artifacts` to it, and its `run_embers(run_score(…))`.
- **Fights.** In `RunPhase::Fight` the caller starts the game from `run_fight_setup` (the human P1, the
  enemy P2), plays it against `run_enemy`, and sends the `RunAction::FightOver` that `run_fight_report`
  folds from the finished game: the hero's health at the end and what the score reads (damage dealt,
  healing done, cards drawn, turns). A game with scripted custom cards is reported under the host that
  lends their scripts, as it is folded.

## What it does not do yet

Combat artifacts and curses change no fight (M4), a ghost's deck is random until snapshots exist (M7),
and the extended campaign is not built (M8). The starting cards, the enemies, the artifacts, the
blessings, the events and the curses are placeholders until the designer's lists land (M9). CI does not
hold the generated `Run*.ts` to the Rust until `ci.yml` runs `cargo test -p jackioh-run --features ts`
and fails on its diff, which only a person can add.

## Tests

`src/tests.rs`, on the placeholder content: seeds, the map's shape, enemy decks, the starting deck's
slots, ownership and artifact points, a whole run played out and its file folded back, extraction,
packs, campfires, shops and the score. `src/fight_tests.rs`, on the real catalog (`jackioh-cards`, a
dev-dependency): the catalog's content, the starting cards, a fight's setup and seed, a scripted game's
report and the levels' costs (DR14's B5 to B11). The `ts` feature writes the public types to
`apps/web/src/wire/generated/`, which `apps/web/src/wire/run.ts` re-exports as `@jackioh/run`; commit
any diff.

```
cargo test -p jackioh-run
cargo test -p jackioh-run --features ts
cargo clippy -p jackioh-run --all-targets -- -D warnings
```
