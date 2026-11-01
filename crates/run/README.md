# `jackioh-run`

The dungeon run (`docs/dungeon-run.md`): a seeded map of fights and tiles strung around plain games, as
the engine's `duos` wraps two of them. No card and no rule of a game knows about a run. This file is
the contract other code relies on; the plan is the design until the mode has its spec section, and
when the two disagree the plan wins and this file is the bug.

It is pure (CLAUDE.md rule 4): it takes only `serde`, `serde_json` and the engine, its `clippy.toml` is
the other pure crates' byte for byte, and every random draw goes through the engine's `Rng`, rebuilt
from the run's seed and cursor. Every number is a constant in `src/config.rs` (rule 9), named by the
plan's section.

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

## What the caller does

- **Content.** The caller passes a `RunContent`: the pool, the starting cards, the enemies, the passives,
  the events and the curses. A Sandbox caller adds the player's custom cards to the pool.
- **Unlocks.** The caller keeps `RunUnlocks` (the level, the extracted cards and artifacts, and in the
  Campaign the account's collection) and adds a finished run's `extracted_cards` and
  `extracted_artifacts` to it, and its `run_embers(run_score(…))`.
- **Fights.** In `RunPhase::Fight` the caller plays the game against `run_enemy` and sends
  `RunAction::FightOver` with the hero's health at the end and what the score reads (damage dealt, healing
  done, cards drawn, turns).

## What it does not do yet

Fights read none of the run (M2: the carried HP, Looming and Overwhelming Presence as handicaps, Radiant
and buffed cards in the deck), Combat artifacts and curses change no fight (M4), a ghost's deck is random
until snapshots exist (M7), and the extended campaign is not built (M8).

## Tests

`src/tests.rs`, on the placeholder content: seeds, the map's shape, enemy decks, the starting deck's
slots, ownership and artifact points, a whole run played out and its file folded back, extraction,
packs, campfires, shops and the score.

```
cargo test -p jackioh-run
cargo clippy -p jackioh-run --all-targets -- -D warnings
```
