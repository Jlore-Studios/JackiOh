# v0.3.0 port map

Every tracked file under `packages/`, `apps/server/` and `ladder/` (plus `scripts/catalog-version.mjs`), the part that owns it, what happens to it, and where it lands. Generated from the tree at the plan's commit; a file added to `main` after that is ported by the part that owns its directory. Actions: **port** (rewrite in Rust at the destination), **port-inline** (a card test that becomes the `mod tests` of its card file), **split** (part of it is ported, the rest dropped, as noted), **copy** (byte for byte; the original is deleted by part 37), **move** (`git mv`), **merge** (its contents go into a file that already exists at the destination), **convert** (data in a new format), **delete** (by part 37 unless noted).

Paths follow [SURFACE.md](SURFACE.md) §4.1. Line counts are the TypeScript's.

| Part | Title | Files | TS lines to port |
|---|---|---|---|
| 1 | bootstrap: staging, the Cargo workspace and the type freeze | 68 | 4,258 |
| 2 | engine 1: the model and the board | 28 | 4,371 |
| 3 | engine 2: combat, damage and the resolution loop | 12 | 6,686 |
| 4 | engine 3: the play pipeline and casting | 10 | 5,604 |
| 5 | engine 4: turn, setup, reduce, view, validator, wire helpers and the testkit | 28 | 9,830 |
| 6 | engine 5: effect verbs, first half | 30 | 4,158 |
| 7 | engine 6: effect verbs, second half | 30 | 4,099 |
| 8 | engine 7: subsystems | 20 | 5,777 |
| 9 | cards lane 1: Core #1–#48 | 97 | 10,809 |
| 10 | cards lane 2: Core #49–#81 | 70 | 10,616 |
| 11 | cards lane 3: Core #82–#100 and the Core tokens to Ghoul | 49 | 10,607 |
| 12 | cards lane 4: Core tokens Rush to Sheep, Classic #1–#32 | 68 | 10,765 |
| 13 | cards lane 5: Classic #33–#68 | 72 | 10,739 |
| 14 | cards lane 6: Classic #69–#90 and tokens, Classic+ #1–#18 | 98 | 10,595 |
| 15 | cards lane 7: Classic+ #19–#49 | 87 | 10,662 |
| 16 | cards lane 8: Classic+ #50–#78 and the AI tokens | 94 | 9,536 |
| 17 | the AI in Rust (generation 0) | 46 | 9,376 |
| 18 | server 1: the HTTP API and auth | 44 | 15,885 |
| 19 | server 2: the match lifecycle and the WebSocket | 34 | 16,005 |
| 20 | server 3: persistence, migrations, CLIs and the Docker deploy | 53 | 17,610 |
| 21 | the WASM bindings and the web client on them | 3 | 1,165 |
| 22 | the jackioh CLI: fuzz, catalog, patches, gates, sweep, stats | 28 | 7,512 |
| 23 | golden traces recorded from the TypeScript engine | 2 | 213 |
| 24 | engine tests 1: effect verbs and fixtures | 77 | 20,170 |
| 25 | engine tests 2: rulings, subsystems, prompts and triggers | 37 | 19,577 |
| 26 | engine tests 3: view, turn, setup, combat | 63 | 18,662 |
| 27 | engine tests 4: play pipeline and cross-card rules | 49 | 21,475 |
| 28 | the spec graph and structural spec checks (#133) | 1 | 4,565 |
| 29 | the training arena, the promotion gate and the lane prompts | 1 | 129 |
| 37 | cull: delete the TypeScript, rewrite the docs | 79 | 0 |


## Part 1: bootstrap: staging, the Cargo workspace and the type freeze

| TypeScript | Lines | Action | Rust | Note |
|---|---|---|---|---|
| `apps/server/src/db/migrations/0001_profiles_and_invites.sql` | 738 | copy | `crates/server/migrations/0001_profiles_and_invites.sql` | byte for byte (their checksums are pinned); original deleted by part 37 |
| `apps/server/src/db/migrations/0002_collection.sql` | 489 | copy | `crates/server/migrations/0002_collection.sql` | byte for byte (their checksums are pinned); original deleted by part 37 |
| `apps/server/src/db/migrations/0003_loadouts.sql` | 525 | copy | `crates/server/migrations/0003_loadouts.sql` | byte for byte (their checksums are pinned); original deleted by part 37 |
| `apps/server/src/db/migrations/0004_matches.sql` | 932 | copy | `crates/server/migrations/0004_matches.sql` | byte for byte (their checksums are pinned); original deleted by part 37 |
| `apps/server/src/db/migrations/0005_service_role_reads_auth_users.sql` | 43 | copy | `crates/server/migrations/0005_service_role_reads_auth_users.sql` | byte for byte (their checksums are pinned); original deleted by part 37 |
| `apps/server/src/db/migrations/0006_redeem_ip_lock.sql` | 201 | copy | `crates/server/migrations/0006_redeem_ip_lock.sql` | byte for byte (their checksums are pinned); original deleted by part 37 |
| `apps/server/src/db/migrations/0007_decks_and_trios.sql` | 625 | copy | `crates/server/migrations/0007_decks_and_trios.sql` | byte for byte (their checksums are pinned); original deleted by part 37 |
| `apps/server/src/db/migrations/0008_queue_modes.sql` | 145 | copy | `crates/server/migrations/0008_queue_modes.sql` | byte for byte (their checksums are pinned); original deleted by part 37 |
| `apps/server/src/db/migrations/0009_series.sql` | 153 | copy | `crates/server/migrations/0009_series.sql` | byte for byte (their checksums are pinned); original deleted by part 37 |
| `apps/server/src/db/migrations/0010_jlockeed_tag.sql` | 41 | copy | `crates/server/migrations/0010_jlockeed_tag.sql` | byte for byte (their checksums are pinned); original deleted by part 37 |
| `apps/server/src/db/migrations/0011_tutorial_progress.sql` | 256 | copy | `crates/server/migrations/0011_tutorial_progress.sql` | byte for byte (their checksums are pinned); original deleted by part 37 |
| `apps/server/src/db/migrations/0012_account_deletion.sql` | 219 | copy | `crates/server/migrations/0012_account_deletion.sql` | byte for byte (their checksums are pinned); original deleted by part 37 |
| `apps/server/src/db/migrations/0013_retention_purge.sql` | 111 | copy | `crates/server/migrations/0013_retention_purge.sql` | byte for byte (their checksums are pinned); original deleted by part 37 |
| `apps/server/src/db/migrations/0014_game_records.sql` | 113 | copy | `crates/server/migrations/0014_game_records.sql` | byte for byte (their checksums are pinned); original deleted by part 37 |
| `apps/server/src/db/migrations/0015_classic_sets_tags.sql` | 46 | copy | `crates/server/migrations/0015_classic_sets_tags.sql` | byte for byte (their checksums are pinned); original deleted by part 37 |
| `apps/server/src/db/migrations/0016_catalog_growth_grants.sql` | 73 | copy | `crates/server/migrations/0016_catalog_growth_grants.sql` | byte for byte (their checksums are pinned); original deleted by part 37 |
| `apps/server/src/db/migrations/0017_last_boards.sql` | 77 | copy | `crates/server/migrations/0017_last_boards.sql` | byte for byte (their checksums are pinned); original deleted by part 37 |
| `apps/server/src/db/migrations/0018_player_settings.sql` | 230 | copy | `crates/server/migrations/0018_player_settings.sql` | byte for byte (their checksums are pinned); original deleted by part 37 |
| `apps/server/src/db/migrations/0019_hero_portraits.sql` | 253 | copy | `crates/server/migrations/0019_hero_portraits.sql` | byte for byte (their checksums are pinned); original deleted by part 37 |
| `apps/server/src/db/migrations/0020_plague_tag.sql` | 38 | copy | `crates/server/migrations/0020_plague_tag.sql` | byte for byte (their checksums are pinned); original deleted by part 37 |
| `apps/server/src/db/migrations/0021_player_stats.sql` | 33 | copy | `crates/server/migrations/0021_player_stats.sql` | byte for byte (their checksums are pinned); original deleted by part 37 |
| `apps/server/src/db/migrations/0022_ranked_ladder.sql` | 472 | copy | `crates/server/migrations/0022_ranked_ladder.sql` | byte for byte (their checksums are pinned); original deleted by part 37 |
| `apps/server/src/db/migrations/0023_rematch.sql` | 35 | copy | `crates/server/migrations/0023_rematch.sql` | byte for byte (their checksums are pinned); original deleted by part 37 |
| `apps/server/src/db/migrations/0024_glitch_boards.sql` | 93 | copy | `crates/server/migrations/0024_glitch_boards.sql` | byte for byte (their checksums are pinned); original deleted by part 37 |
| `apps/server/src/db/migrations/0025_patch_retcon.sql` | 88 | copy | `crates/server/migrations/0025_patch_retcon.sql` | byte for byte (their checksums are pinned); original deleted by part 37 |
| `apps/server/src/db/migrations/0026_catalyst_prime_acclaimed_tags.sql` | 39 | copy | `crates/server/migrations/0026_catalyst_prime_acclaimed_tags.sql` | byte for byte (their checksums are pinned); original deleted by part 37 |
| `packages/ai/src/index.ts` | 24 | port | `crates/ai/src/lib.rs` |  |
| `packages/cards/catalog.json` | 9772 | copy | `crates/cards/catalog.json` | byte for byte; original deleted by part 37 |
| `packages/cards/flavour.json` | 956 | copy | `crates/cards/flavour.json` | byte for byte; original deleted by part 37 |
| `packages/cards/patches/index.json` | 1512 | copy | `crates/cards/patches/index.json` | byte for byte; original deleted by part 37 |
| `packages/cards/patches/patches.json` | 6936 | copy | `crates/cards/patches/patches.json` | byte for byte; original deleted by part 37 |
| `packages/cards/patches/shipped.json` | 112 | copy | `crates/cards/patches/shipped.json` | byte for byte; original deleted by part 37 |
| `packages/cards/patches/v0.1.0.json` | 2594 | copy | `crates/cards/patches/v0.1.0.json` | byte for byte; original deleted by part 37 |
| `packages/cards/patches/v0.1.0b.json` | 2609 | copy | `crates/cards/patches/v0.1.0b.json` | byte for byte; original deleted by part 37 |
| `packages/cards/patches/v0.1.0c.json` | 2630 | copy | `crates/cards/patches/v0.1.0c.json` | byte for byte; original deleted by part 37 |
| `packages/cards/patches/v0.1.0d.json` | 2709 | copy | `crates/cards/patches/v0.1.0d.json` | byte for byte; original deleted by part 37 |
| `packages/cards/patches/v0.1.1.json` | 2733 | copy | `crates/cards/patches/v0.1.1.json` | byte for byte; original deleted by part 37 |
| `packages/cards/patches/v0.2.0.json` | 9774 | copy | `crates/cards/patches/v0.2.0.json` | byte for byte; original deleted by part 37 |
| `packages/cards/patches/v0.2.1.json` | 9774 | copy | `crates/cards/patches/v0.2.1.json` | byte for byte; original deleted by part 37 |
| `packages/cards/patches/v0.2.10.json` | 9750 | copy | `crates/cards/patches/v0.2.10.json` | byte for byte; original deleted by part 37 |
| `packages/cards/patches/v0.2.10b.json` | 9759 | copy | `crates/cards/patches/v0.2.10b.json` | byte for byte; original deleted by part 37 |
| `packages/cards/patches/v0.2.10c.json` | 9759 | copy | `crates/cards/patches/v0.2.10c.json` | byte for byte; original deleted by part 37 |
| `packages/cards/patches/v0.2.10d.json` | 9761 | copy | `crates/cards/patches/v0.2.10d.json` | byte for byte; original deleted by part 37 |
| `packages/cards/patches/v0.2.11.json` | 9772 | copy | `crates/cards/patches/v0.2.11.json` | byte for byte; original deleted by part 37 |
| `packages/cards/patches/v0.2.2.json` | 9808 | copy | `crates/cards/patches/v0.2.2.json` | byte for byte; original deleted by part 37 |
| `packages/cards/patches/v0.2.3.json` | 10024 | copy | `crates/cards/patches/v0.2.3.json` | byte for byte; original deleted by part 37 |
| `packages/cards/patches/v0.2.4.json` | 10030 | copy | `crates/cards/patches/v0.2.4.json` | byte for byte; original deleted by part 37 |
| `packages/cards/patches/v0.2.5.json` | 9814 | copy | `crates/cards/patches/v0.2.5.json` | byte for byte; original deleted by part 37 |
| `packages/cards/patches/v0.2.6.json` | 9814 | copy | `crates/cards/patches/v0.2.6.json` | byte for byte; original deleted by part 37 |
| `packages/cards/patches/v0.2.7.json` | 9814 | copy | `crates/cards/patches/v0.2.7.json` | byte for byte; original deleted by part 37 |
| `packages/cards/patches/v0.2.7b.json` | 9814 | copy | `crates/cards/patches/v0.2.7b.json` | byte for byte; original deleted by part 37 |
| `packages/cards/patches/v0.2.8.json` | 9836 | copy | `crates/cards/patches/v0.2.8.json` | byte for byte; original deleted by part 37 |
| `packages/cards/patches/v0.2.8b.json` | 9836 | copy | `crates/cards/patches/v0.2.8b.json` | byte for byte; original deleted by part 37 |
| `packages/cards/patches/v0.2.9.json` | 9836 | copy | `crates/cards/patches/v0.2.9.json` | byte for byte; original deleted by part 37 |
| `packages/cards/src/catalog-data.ts` | 44 | port | `crates/cards/src/lib.rs` | catalog_json(), catalog_version() |
| `packages/cards/src/index.ts` | 103 | port | `crates/cards/src/lib.rs` | registry include + register_all (SURFACE §7.4) |
| `packages/engine/src/config.ts` | 469 | port | `crates/engine/src/config.rs` | type freeze (SURFACE §6.4) |
| `packages/engine/src/effects/index.ts` | 382 | port | `crates/engine/src/effects/mod.rs` | barrel: `pub mod x; pub use x::*;` |
| `packages/engine/src/index.ts` | 70 | port | `crates/engine/src/lib.rs` | module tree + re-exports |
| `packages/engine/src/rng.ts` | 86 | port | `crates/engine/src/rng.rs` | type freeze (SURFACE §6.4) |
| `packages/engine/src/script.ts` | 577 | port | `crates/engine/src/script.rs` | type freeze (SURFACE §6.4) |
| `packages/engine/src/state.ts` | 1012 | port | `crates/engine/src/state.rs` | type freeze (SURFACE §6.4) |
| `packages/engine/src/subsystems/index.ts` | 43 | port | `crates/engine/src/subsystems/mod.rs` | barrel |
| `packages/shared/src/actions.ts` | 97 | port | `crates/engine/src/wire/actions.rs` | types + tiny helpers (opponentOf, keywordKey, fillParams, …); TS copy of the helpers by part 21 |
| `packages/shared/src/catalog-types.ts` | 406 | port | `crates/engine/src/wire/catalog_types.rs` | types + tiny helpers (opponentOf, keywordKey, fillParams, …); TS copy of the helpers by part 21 |
| `packages/shared/src/events.ts` | 498 | port | `crates/engine/src/wire/events.rs` | types + tiny helpers (opponentOf, keywordKey, fillParams, …); TS copy of the helpers by part 21 |
| `packages/shared/src/index.ts` | 8 | port | `crates/engine/src/wire/mod.rs` |  |
| `packages/shared/src/view.ts` | 439 | port | `crates/engine/src/wire/view.rs` | types + tiny helpers (opponentOf, keywordKey, fillParams, …); TS copy of the helpers by part 21 |

## Part 2: engine 1: the model and the board

| TypeScript | Lines | Action | Rust | Note |
|---|---|---|---|---|
| `packages/engine/src/animated.ts` | 225 | port | `crates/engine/src/animated.rs` |  |
| `packages/engine/src/announce.ts` | 81 | port | `crates/engine/src/announce.rs` |  |
| `packages/engine/src/bookSwap.ts` | 35 | port | `crates/engine/src/book_swap.rs` |  |
| `packages/engine/src/brittle.ts` | 92 | port | `crates/engine/src/brittle.rs` |  |
| `packages/engine/src/brittleCount.ts` | 98 | port | `crates/engine/src/brittle_count.rs` |  |
| `packages/engine/src/carriers.ts` | 73 | port | `crates/engine/src/carriers.rs` |  |
| `packages/engine/src/castOnDrawNow.ts` | 13 | port | `crates/engine/src/cast_on_draw_now.rs` |  |
| `packages/engine/src/catalog.ts` | 344 | port | `crates/engine/src/catalog.rs` | registry is a OnceLock; transient defs read from state; no digest table |
| `packages/engine/src/drawComplete.ts` | 42 | port | `crates/engine/src/draw_complete.rs` |  |
| `packages/engine/src/enchantments.ts` | 58 | port | `crates/engine/src/enchantments.rs` |  |
| `packages/engine/src/faces.ts` | 31 | port | `crates/engine/src/faces.rs` |  |
| `packages/engine/src/killCredit.ts` | 23 | port | `crates/engine/src/kill_credit.rs` |  |
| `packages/engine/src/layers.ts` | 324 | port | `crates/engine/src/layers.rs` |  |
| `packages/engine/src/marks.ts` | 92 | port | `crates/engine/src/marks.rs` |  |
| `packages/engine/src/numbers.ts` | 193 | port | `crates/engine/src/numbers.rs` |  |
| `packages/engine/src/ownLibrary.ts` | 89 | port | `crates/engine/src/own_library.rs` |  |
| `packages/engine/src/ownership.ts` | 112 | port | `crates/engine/src/ownership.rs` |  |
| `packages/engine/src/params.ts` | 193 | port | `crates/engine/src/params.rs` |  |
| `packages/engine/src/plague.ts` | 107 | port | `crates/engine/src/plague.rs` |  |
| `packages/engine/src/preview.ts` | 100 | port | `crates/engine/src/preview.rs` |  |
| `packages/engine/src/query.ts` | 348 | port | `crates/engine/src/query.rs` |  |
| `packages/engine/src/restrictions.ts` | 152 | port | `crates/engine/src/restrictions.rs` | registerAttackBar not ported |
| `packages/engine/src/scripts.ts` | 130 | port | `crates/engine/src/scripts.rs` |  |
| `packages/engine/src/stays.ts` | 285 | port | `crates/engine/src/stays.rs` |  |
| `packages/engine/src/temporary.ts` | 33 | port | `crates/engine/src/temporary.rs` |  |
| `packages/engine/src/timesPlayed.ts` | 24 | port | `crates/engine/src/times_played.rs` |  |
| `packages/engine/src/tuning.ts` | 200 | port | `crates/engine/src/tuning.rs` |  |
| `packages/engine/src/zones.ts` | 874 | port | `crates/engine/src/zones.rs` |  |

## Part 3: engine 2: combat, damage and the resolution loop

| TypeScript | Lines | Action | Rust | Note |
|---|---|---|---|---|
| `packages/engine/src/combat.ts` | 1043 | port | `crates/engine/src/combat.rs` |  |
| `packages/engine/src/damage.ts` | 427 | port | `crates/engine/src/damage.rs` |  |
| `packages/engine/src/modifiers.ts` | 240 | port | `crates/engine/src/modifiers.rs` |  |
| `packages/engine/src/prompts.ts` | 974 | port | `crates/engine/src/prompts.rs` | answerer registry → one `match` |
| `packages/engine/src/replacements.ts` | 584 | port | `crates/engine/src/replacements.rs` | `converting` moves to EngineSink |
| `packages/engine/src/resolve.ts` | 206 | port | `crates/engine/src/resolve.rs` |  |
| `packages/engine/src/stateCheck.ts` | 662 | port | `crates/engine/src/state_check.rs` |  |
| `packages/engine/src/targeting.ts` | 166 | port | `crates/engine/src/targeting.rs` |  |
| `packages/engine/src/targetingPoint.ts` | 172 | port | `crates/engine/src/targeting_point.rs` |  |
| `packages/engine/src/traps.ts` | 766 | port | `crates/engine/src/traps.rs` |  |
| `packages/engine/src/triggers.ts` | 853 | port | `crates/engine/src/triggers.rs` |  |
| `packages/engine/src/work.ts` | 593 | port | `crates/engine/src/work.rs` | handler registry → one `match` on resume.hook (SURFACE §6.6) |

## Part 4: engine 3: the play pipeline and casting

| TypeScript | Lines | Action | Rust | Note |
|---|---|---|---|---|
| `packages/engine/src/costRules.ts` | 248 | port | `crates/engine/src/cost_rules.rs` |  |
| `packages/engine/src/cryTrigger.ts` | 304 | port | `crates/engine/src/cry_trigger.rs` |  |
| `packages/engine/src/draw.ts` | 632 | port | `crates/engine/src/draw.rs` |  |
| `packages/engine/src/echo.ts` | 292 | port | `crates/engine/src/echo.rs` |  |
| `packages/engine/src/graveyardPlay.ts` | 177 | port | `crates/engine/src/graveyard_play.rs` |  |
| `packages/engine/src/mana.ts` | 223 | port | `crates/engine/src/mana.rs` |  |
| `packages/engine/src/playChoices.ts` | 1413 | port | `crates/engine/src/play_choices.rs` |  |
| `packages/engine/src/playCounts.ts` | 69 | port | `crates/engine/src/play_counts.rs` |  |
| `packages/engine/src/playSteps.ts` | 2059 | port | `crates/engine/src/play_steps.rs` |  |
| `packages/engine/src/randomCast.ts` | 187 | port | `crates/engine/src/random_cast.rs` |  |

## Part 5: engine 4: turn, setup, reduce, view, validator, wire helpers and the testkit

| TypeScript | Lines | Action | Rust | Note |
|---|---|---|---|---|
| `packages/cards/test/_glow.ts` | 35 | port | `crates/engine/src/testkit/glow.rs` |  |
| `packages/cards/test/_harness.test.ts` | 664 | port | `crates/engine/src/testkit/scenario.rs` | as `#[cfg(test)] mod tests` of the testkit |
| `packages/cards/test/_harness.ts` | 1149 | port | `crates/engine/src/testkit/scenario.rs` | SURFACE §8 |
| `packages/cards/test/_invariants.ts` | 258 | port | `crates/engine/src/testkit/invariants.rs` |  |
| `packages/cards/test/game-summary.test.ts` | 221 | port | `crates/cards/tests/cross/game_summary.rs` |  |
| `packages/cards/test/invariants.test.ts` | 55 | port | `crates/cards/tests/cross/invariants.rs` |  |
| `packages/engine/src/condition.ts` | 70 | port | `crates/engine/src/condition.rs` |  |
| `packages/engine/src/counterWarning.ts` | 50 | port | `crates/engine/src/counter_warning.rs` |  |
| `packages/engine/src/gameOver.ts` | 23 | port | `crates/engine/src/game_over.rs` |  |
| `packages/engine/src/gameSummary.ts` | 186 | port | `crates/engine/src/game_summary.rs` |  |
| `packages/engine/src/instanceView.ts` | 62 | port | `crates/engine/src/instance_view.rs` |  |
| `packages/engine/src/reduce.ts` | 567 | port | `crates/engine/src/reduce.rs` | drops the `rng` argument and syncFusedScripts |
| `packages/engine/src/replay.ts` | 83 | port | `crates/engine/src/replay.rs` |  |
| `packages/engine/src/setup.ts` | 605 | port | `crates/engine/src/setup.rs` |  |
| `packages/engine/src/turn.ts` | 842 | port | `crates/engine/src/turn.rs` |  |
| `packages/engine/src/viewFor.ts` | 1177 | port | `crates/engine/src/view_for.rs` |  |
| `packages/shared/src/aim.ts` | 90 | port | `crates/engine/src/wire/aim.rs` | TS copy kept for the web as apps/web/src/wire/aim.ts (part 21) |
| `packages/shared/src/codes.ts` | 358 | port | `crates/engine/src/wire/codes.rs` | TS copy kept for the web as apps/web/src/wire/codes.ts (part 21) |
| `packages/shared/src/emotes.ts` | 139 | port | `crates/engine/src/wire/emotes.rs` | TS copy kept for the web as apps/web/src/wire/emotes.ts (part 21) |
| `packages/shared/src/stats.ts` | 442 | port | `crates/engine/src/wire/stats.rs` | TS copy kept for the web as apps/web/src/wire/stats.ts (part 21) |
| `packages/shared/test/codes.test.ts` | 486 | port | `crates/engine/src/wire/codes.rs` | as `#[cfg(test)] mod tests` |
| `packages/shared/test/emotes.test.ts` | 196 | port | `crates/engine/src/wire/emotes.rs` | as `#[cfg(test)] mod tests` |
| `packages/shared/test/fixtures/code-input-cases.ts` | 461 | convert | `crates/engine/tests/fixtures/code-input-cases.json` | one fixture for the Rust and the web tests |
| `packages/shared/test/stats.test.ts` | 284 | port | `crates/engine/src/wire/stats.rs` | as `#[cfg(test)] mod tests` |
| `packages/validator/src/index.ts` | 526 | port | `crates/engine/src/validator.rs` | the one validator; the web calls it through WASM |
| `packages/validator/test/drafts.test.ts` | 284 | port | `crates/engine/tests/rules/validator_drafts.rs` |  |
| `packages/validator/test/fixtures/loadouts.ts` | 247 | port | `crates/engine/tests/rules/fixtures/validator_loadouts.rs` |  |
| `packages/validator/test/validator.test.ts` | 270 | port | `crates/engine/tests/rules/validator.rs` |  |

## Part 6: engine 5: effect verbs, first half

| TypeScript | Lines | Action | Rust | Note |
|---|---|---|---|---|
| `packages/engine/src/effects/addToHand.ts` | 186 | port | `crates/engine/src/effects/add_to_hand.rs` |  |
| `packages/engine/src/effects/afterCheck.ts` | 80 | port | `crates/engine/src/effects/after_check.rs` |  |
| `packages/engine/src/effects/animate.ts` | 34 | port | `crates/engine/src/effects/animate.rs` |  |
| `packages/engine/src/effects/brittle.ts` | 68 | port | `crates/engine/src/effects/brittle.rs` |  |
| `packages/engine/src/effects/buff.ts` | 184 | port | `crates/engine/src/effects/buff.rs` |  |
| `packages/engine/src/effects/cardScope.ts` | 128 | port | `crates/engine/src/effects/card_scope.rs` |  |
| `packages/engine/src/effects/cast.ts` | 221 | port | `crates/engine/src/effects/cast.rs` |  |
| `packages/engine/src/effects/choose.ts` | 753 | port | `crates/engine/src/effects/choose.rs` |  |
| `packages/engine/src/effects/chooseWhere.ts` | 65 | port | `crates/engine/src/effects/choose_where.rs` |  |
| `packages/engine/src/effects/coins.ts` | 114 | port | `crates/engine/src/effects/coins.rs` |  |
| `packages/engine/src/effects/combat.ts` | 295 | port | `crates/engine/src/effects/combat.rs` |  |
| `packages/engine/src/effects/cost.ts` | 80 | port | `crates/engine/src/effects/cost.rs` |  |
| `packages/engine/src/effects/counters.ts` | 100 | port | `crates/engine/src/effects/counters.rs` |  |
| `packages/engine/src/effects/cry.ts` | 41 | port | `crates/engine/src/effects/cry.rs` |  |
| `packages/engine/src/effects/damage.ts` | 96 | port | `crates/engine/src/effects/damage.rs` |  |
| `packages/engine/src/effects/datacenter.ts` | 103 | port | `crates/engine/src/effects/datacenter.rs` |  |
| `packages/engine/src/effects/delay.ts` | 311 | port | `crates/engine/src/effects/delay.rs` |  |
| `packages/engine/src/effects/destroy.ts` | 104 | port | `crates/engine/src/effects/destroy.rs` |  |
| `packages/engine/src/effects/draw.ts` | 59 | port | `crates/engine/src/effects/draw.rs` |  |
| `packages/engine/src/effects/drawWhile.ts` | 31 | port | `crates/engine/src/effects/draw_while.rs` |  |
| `packages/engine/src/effects/each.ts` | 32 | port | `crates/engine/src/effects/each.rs` |  |
| `packages/engine/src/effects/enchant.ts` | 47 | port | `crates/engine/src/effects/enchant.rs` |  |
| `packages/engine/src/effects/flicker.ts` | 67 | port | `crates/engine/src/effects/flicker.rs` |  |
| `packages/engine/src/effects/fruit.ts` | 195 | port | `crates/engine/src/effects/fruit.rs` |  |
| `packages/engine/src/effects/fuse.ts` | 450 | port | `crates/engine/src/effects/fuse.rs` |  |
| `packages/engine/src/effects/give.ts` | 125 | port | `crates/engine/src/effects/give.rs` |  |
| `packages/engine/src/effects/handExile.ts` | 49 | port | `crates/engine/src/effects/hand_exile.rs` |  |
| `packages/engine/src/effects/heal.ts` | 47 | port | `crates/engine/src/effects/heal.rs` |  |
| `packages/engine/src/effects/health.ts` | 44 | port | `crates/engine/src/effects/health.rs` |  |
| `packages/engine/src/effects/killCredit.ts` | 49 | port | `crates/engine/src/effects/kill_credit.rs` |  |

## Part 7: engine 6: effect verbs, second half

| TypeScript | Lines | Action | Rust | Note |
|---|---|---|---|---|
| `packages/engine/src/effects/lastBoard.ts` | 104 | port | `crates/engine/src/effects/last_board.rs` |  |
| `packages/engine/src/effects/library.ts` | 94 | port | `crates/engine/src/effects/library.rs` |  |
| `packages/engine/src/effects/libraryCopies.ts` | 41 | port | `crates/engine/src/effects/library_copies.rs` |  |
| `packages/engine/src/effects/locks.ts` | 145 | port | `crates/engine/src/effects/locks.rs` |  |
| `packages/engine/src/effects/loseHealth.ts` | 14 | port | `crates/engine/src/effects/lose_health.rs` |  |
| `packages/engine/src/effects/mana.ts` | 63 | port | `crates/engine/src/effects/mana.rs` |  |
| `packages/engine/src/effects/memory.ts` | 38 | port | `crates/engine/src/effects/memory.rs` |  |
| `packages/engine/src/effects/move.ts` | 398 | port | `crates/engine/src/effects/move_.rs` |  |
| `packages/engine/src/effects/perks.ts` | 43 | port | `crates/engine/src/effects/perks.rs` |  |
| `packages/engine/src/effects/plague.ts` | 232 | port | `crates/engine/src/effects/plague.rs` |  |
| `packages/engine/src/effects/playerMods.ts` | 43 | port | `crates/engine/src/effects/player_mods.rs` |  |
| `packages/engine/src/effects/position.ts` | 35 | port | `crates/engine/src/effects/position.rs` |  |
| `packages/engine/src/effects/radiant.ts` | 257 | port | `crates/engine/src/effects/radiant.rs` |  |
| `packages/engine/src/effects/randomPicks.ts` | 49 | port | `crates/engine/src/effects/random_picks.rs` |  |
| `packages/engine/src/effects/reveal.ts` | 27 | port | `crates/engine/src/effects/reveal.rs` |  |
| `packages/engine/src/effects/rotate.ts` | 53 | port | `crates/engine/src/effects/rotate.rs` |  |
| `packages/engine/src/effects/rounds.ts` | 69 | port | `crates/engine/src/effects/rounds.rs` |  |
| `packages/engine/src/effects/shuffleCard.ts` | 48 | port | `crates/engine/src/effects/shuffle_card.rs` |  |
| `packages/engine/src/effects/shuffleInto.ts` | 69 | port | `crates/engine/src/effects/shuffle_into.rs` |  |
| `packages/engine/src/effects/shuffleRandom.ts` | 38 | port | `crates/engine/src/effects/shuffle_random.rs` |  |
| `packages/engine/src/effects/split.ts` | 47 | port | `crates/engine/src/effects/split.rs` |  |
| `packages/engine/src/effects/statuses.ts` | 48 | port | `crates/engine/src/effects/statuses.rs` |  |
| `packages/engine/src/effects/steal.ts` | 118 | port | `crates/engine/src/effects/steal.rs` |  |
| `packages/engine/src/effects/summon.ts` | 553 | port | `crates/engine/src/effects/summon.rs` |  |
| `packages/engine/src/effects/summonThis.ts` | 21 | port | `crates/engine/src/effects/summon_this.rs` |  |
| `packages/engine/src/effects/swap.ts` | 270 | port | `crates/engine/src/effects/swap.rs` |  |
| `packages/engine/src/effects/targets.ts` | 250 | port | `crates/engine/src/effects/targets.rs` |  |
| `packages/engine/src/effects/transform.ts` | 319 | port | `crates/engine/src/effects/transform.rs` |  |
| `packages/engine/src/effects/tune.ts` | 569 | port | `crates/engine/src/effects/tune.rs` |  |
| `packages/engine/src/effects/turnEnd.ts` | 44 | port | `crates/engine/src/effects/turn_end.rs` |  |

## Part 8: engine 7: subsystems

| TypeScript | Lines | Action | Rust | Note |
|---|---|---|---|---|
| `packages/engine/src/subsystems/activate.ts` | 568 | port | `crates/engine/src/subsystems/activate.rs` |  |
| `packages/engine/src/subsystems/aiPolicy.ts` | 205 | port | `crates/engine/src/subsystems/ai_policy.rs` |  |
| `packages/engine/src/subsystems/audit.ts` | 40 | port | `crates/engine/src/subsystems/audit.rs` |  |
| `packages/engine/src/subsystems/boardHistory.ts` | 194 | port | `crates/engine/src/subsystems/board_history.rs` |  |
| `packages/engine/src/subsystems/callToChaos.ts` | 394 | port | `crates/engine/src/subsystems/call_to_chaos.rs` |  |
| `packages/engine/src/subsystems/callToChaosPlus.ts` | 139 | port | `crates/engine/src/subsystems/call_to_chaos_plus.rs` |  |
| `packages/engine/src/subsystems/comboIndex.ts` | 266 | port | `crates/engine/src/subsystems/combo_index.rs` |  |
| `packages/engine/src/subsystems/copiedText.ts` | 136 | port | `crates/engine/src/subsystems/copied_text.rs` |  |
| `packages/engine/src/subsystems/fuse.ts` | 1171 | port | `crates/engine/src/subsystems/fuse.rs` | no syncFusedScripts, no global registry: fused scripts built on lookup (SURFACE §6.6) |
| `packages/engine/src/subsystems/glitch.ts` | 132 | port | `crates/engine/src/subsystems/glitch.rs` |  |
| `packages/engine/src/subsystems/heroPower.ts` | 590 | port | `crates/engine/src/subsystems/hero_power.rs` |  |
| `packages/engine/src/subsystems/kyTest.ts` | 110 | port | `crates/engine/src/subsystems/ky_test.rs` |  |
| `packages/engine/src/subsystems/lastBoards.ts` | 106 | port | `crates/engine/src/subsystems/last_boards.rs` |  |
| `packages/engine/src/subsystems/lethal.ts` | 172 | port | `crates/engine/src/subsystems/lethal.rs` |  |
| `packages/engine/src/subsystems/papaya.ts` | 141 | port | `crates/engine/src/subsystems/papaya.rs` |  |
| `packages/engine/src/subsystems/perfectHand.ts` | 49 | port | `crates/engine/src/subsystems/perfect_hand.rs` |  |
| `packages/engine/src/subsystems/quests.ts` | 544 | port | `crates/engine/src/subsystems/quests.rs` |  |
| `packages/engine/src/subsystems/rotation.ts` | 197 | port | `crates/engine/src/subsystems/rotation.rs` |  |
| `packages/engine/src/subsystems/scorer.ts` | 503 | port | `crates/engine/src/subsystems/scorer.rs` | `dryRunning` moves to EngineSink (SURFACE §6.5) |
| `packages/engine/src/subsystems/twiceForward.ts` | 120 | port | `crates/engine/src/subsystems/twice_forward.rs` |  |

## Part 9: cards lane 1: Core #1–#48

| TypeScript | Lines | Action | Rust | Note |
|---|---|---|---|---|
| `packages/cards/src/query.ts` | 110 | port | `crates/cards/src/query.rs` | used by 083 Transmogulate and 067 Zoomerbin Oomen |
| `packages/cards/src/scripts/001-big-d-fender.ts` | 36 | port | `crates/cards/src/scripts/core/c001_big_d_fender.rs` | cards lane 1 |
| `packages/cards/src/scripts/002-bigot.ts` | 30 | port | `crates/cards/src/scripts/core/c002_bigot.rs` | cards lane 1 |
| `packages/cards/src/scripts/003-right-house-defender.ts` | 27 | port | `crates/cards/src/scripts/core/c003_right_house_defender.rs` | cards lane 1 |
| `packages/cards/src/scripts/004-gary-the-gambler.ts` | 39 | port | `crates/cards/src/scripts/core/c004_gary_the_gambler.rs` | cards lane 1 |
| `packages/cards/src/scripts/005-stockpile.ts` | 24 | port | `crates/cards/src/scripts/core/c005_stockpile.rs` | cards lane 1 |
| `packages/cards/src/scripts/006-mana-well.ts` | 34 | port | `crates/cards/src/scripts/core/c006_mana_well.rs` | cards lane 1 |
| `packages/cards/src/scripts/007-jewelosco-scarab.ts` | 65 | port | `crates/cards/src/scripts/core/c007_jewelosco_scarab.rs` | cards lane 1 |
| `packages/cards/src/scripts/008-mr-vanilla.ts` | 16 | port | `crates/cards/src/scripts/core/c008_mr_vanilla.rs` | cards lane 1 |
| `packages/cards/src/scripts/009-moths-to-the-flame.ts` | 37 | port | `crates/cards/src/scripts/core/c009_moths_to_the_flame.rs` | cards lane 1 |
| `packages/cards/src/scripts/010-rapid-replenish.ts` | 54 | port | `crates/cards/src/scripts/core/c010_rapid_replenish.rs` | cards lane 1 |
| `packages/cards/src/scripts/011-tempo-timmy.ts` | 18 | port | `crates/cards/src/scripts/core/c011_tempo_timmy.rs` | cards lane 1 |
| `packages/cards/src/scripts/012-duplicating-felinors.ts` | 40 | port | `crates/cards/src/scripts/core/c012_duplicating_felinors.rs` | cards lane 1 |
| `packages/cards/src/scripts/013-jlockeed-shredder-10.ts` | 40 | port | `crates/cards/src/scripts/core/c013_jlockeed_shredder_10.rs` | cards lane 1 |
| `packages/cards/src/scripts/014-jlockeeds-weapons.ts` | 39 | port | `crates/cards/src/scripts/core/c014_jlockeeds_weapons.rs` | cards lane 1 |
| `packages/cards/src/scripts/015-me-and-mr-token.ts` | 33 | port | `crates/cards/src/scripts/core/c015_me_and_mr_token.rs` | cards lane 1 |
| `packages/cards/src/scripts/016-hit-job.ts` | 45 | port | `crates/cards/src/scripts/core/c016_hit_job.rs` | cards lane 1 |
| `packages/cards/src/scripts/017-flood.ts` | 70 | port | `crates/cards/src/scripts/core/c017_flood.rs` | cards lane 1 |
| `packages/cards/src/scripts/018-bread-and-butter.ts` | 126 | port | `crates/cards/src/scripts/core/c018_bread_and_butter.rs` | cards lane 1 |
| `packages/cards/src/scripts/019-midrange-menace.ts` | 37 | port | `crates/cards/src/scripts/core/c019_midrange_menace.rs` | cards lane 1 |
| `packages/cards/src/scripts/020-pointmaster.ts` | 22 | port | `crates/cards/src/scripts/core/c020_pointmaster.rs` | cards lane 1 |
| `packages/cards/src/scripts/021-hinder.ts` | 42 | port | `crates/cards/src/scripts/core/c021_hinder.rs` | cards lane 1 |
| `packages/cards/src/scripts/022-carnivorous-cube.ts` | 159 | port | `crates/cards/src/scripts/core/c022_carnivorous_cube.rs` | cards lane 1 |
| `packages/cards/src/scripts/023-reoccurring-dream.ts` | 81 | port | `crates/cards/src/scripts/core/c023_reoccurring_dream.rs` | cards lane 1 |
| `packages/cards/src/scripts/024-efficiency-dividend.ts` | 106 | port | `crates/cards/src/scripts/core/c024_efficiency_dividend.rs` | cards lane 1 |
| `packages/cards/src/scripts/025-4-mana-7-7.ts` | 22 | port | `crates/cards/src/scripts/core/c025_4_mana_7_7.rs` | cards lane 1 |
| `packages/cards/src/scripts/026-glowy-jelly-bean.ts` | 40 | port | `crates/cards/src/scripts/core/c026_glowy_jelly_bean.rs` | cards lane 1 |
| `packages/cards/src/scripts/027-blood-ridden-glowy-jelly-bean.ts` | 44 | port | `crates/cards/src/scripts/core/c027_blood_ridden_glowy_jelly_bean.rs` | cards lane 1 |
| `packages/cards/src/scripts/028-knockoff-temu-glowy-jelly-bean.ts` | 44 | port | `crates/cards/src/scripts/core/c028_knockoff_temu_glowy_jelly_bean.rs` | cards lane 1 |
| `packages/cards/src/scripts/029-giga-glowy-jelly-bean.ts` | 60 | port | `crates/cards/src/scripts/core/c029_giga_glowy_jelly_bean.rs` | cards lane 1 |
| `packages/cards/src/scripts/030-archivist.ts` | 97 | port | `crates/cards/src/scripts/core/c030_archivist.rs` | cards lane 1 |
| `packages/cards/src/scripts/031-kys-math-equation.ts` | 134 | port | `crates/cards/src/scripts/core/c031_kys_math_equation.rs` | cards lane 1 |
| `packages/cards/src/scripts/032-prem-panther.ts` | 32 | port | `crates/cards/src/scripts/core/c032_prem_panther.rs` | cards lane 1 |
| `packages/cards/src/scripts/033-unstable-clone-machine.ts` | 88 | port | `crates/cards/src/scripts/core/c033_unstable_clone_machine.rs` | cards lane 1 |
| `packages/cards/src/scripts/034-collateral-damage.ts` | 67 | port | `crates/cards/src/scripts/core/c034_collateral_damage.rs` | cards lane 1 |
| `packages/cards/src/scripts/035-lunar-eclipse.ts` | 81 | port | `crates/cards/src/scripts/core/c035_lunar_eclipse.rs` | cards lane 1 |
| `packages/cards/src/scripts/036-magic-jammed.ts` | 50 | port | `crates/cards/src/scripts/core/c036_magic_jammed.rs` | cards lane 1 |
| `packages/cards/src/scripts/037-gravedigger.ts` | 71 | port | `crates/cards/src/scripts/core/c037_gravedigger.rs` | cards lane 1 |
| `packages/cards/src/scripts/038-quickstriker.ts` | 57 | port | `crates/cards/src/scripts/core/c038_quickstriker.rs` | cards lane 1 |
| `packages/cards/src/scripts/039-recycling-initiative.ts` | 141 | port | `crates/cards/src/scripts/core/c039_recycling_initiative.rs` | cards lane 1 |
| `packages/cards/src/scripts/040-echoes-of-the-forgotten.ts` | 72 | port | `crates/cards/src/scripts/core/c040_echoes_of_the_forgotten.rs` | cards lane 1 |
| `packages/cards/src/scripts/041-sheepish.ts` | 90 | port | `crates/cards/src/scripts/core/c041_sheepish.rs` | cards lane 1 |
| `packages/cards/src/scripts/042-eugenics.ts` | 55 | port | `crates/cards/src/scripts/core/c042_eugenics.rs` | cards lane 1 |
| `packages/cards/src/scripts/043-big-felinor.ts` | 47 | port | `crates/cards/src/scripts/core/c043_big_felinor.rs` | cards lane 1 |
| `packages/cards/src/scripts/044-true-strike.ts` | 51 | port | `crates/cards/src/scripts/core/c044_true_strike.rs` | cards lane 1 |
| `packages/cards/src/scripts/045-deft-duelist.ts` | 21 | port | `crates/cards/src/scripts/core/c045_deft_duelist.rs` | cards lane 1 |
| `packages/cards/src/scripts/046-suppressive-aura.ts` | 66 | port | `crates/cards/src/scripts/core/c046_suppressive_aura.rs` | cards lane 1 |
| `packages/cards/src/scripts/047-fig-of-life.ts` | 37 | port | `crates/cards/src/scripts/core/c047_fig_of_life.rs` | cards lane 1 |
| `packages/cards/src/scripts/048-5pek-controller.ts` | 51 | port | `crates/cards/src/scripts/core/c048_5pek_controller.rs` | cards lane 1 |
| `packages/cards/test/001-big-d-fender.test.ts` | 65 | port-inline | `crates/cards/src/scripts/core/c001_big_d_fender.rs` | `#[cfg(test)] mod tests` of the card file |
| `packages/cards/test/002-bigot.test.ts` | 62 | port-inline | `crates/cards/src/scripts/core/c002_bigot.rs` | `#[cfg(test)] mod tests` of the card file |
| `packages/cards/test/003-right-house-defender.test.ts` | 146 | port-inline | `crates/cards/src/scripts/core/c003_right_house_defender.rs` | `#[cfg(test)] mod tests` of the card file |
| `packages/cards/test/004-gary-the-gambler.test.ts` | 130 | port-inline | `crates/cards/src/scripts/core/c004_gary_the_gambler.rs` | `#[cfg(test)] mod tests` of the card file |
| `packages/cards/test/005-stockpile.test.ts` | 74 | port-inline | `crates/cards/src/scripts/core/c005_stockpile.rs` | `#[cfg(test)] mod tests` of the card file |
| `packages/cards/test/006-mana-well.test.ts` | 95 | port-inline | `crates/cards/src/scripts/core/c006_mana_well.rs` | `#[cfg(test)] mod tests` of the card file |
| `packages/cards/test/007-jewelosco-scarab.test.ts` | 147 | port-inline | `crates/cards/src/scripts/core/c007_jewelosco_scarab.rs` | `#[cfg(test)] mod tests` of the card file |
| `packages/cards/test/008-mr-vanilla.test.ts` | 103 | port-inline | `crates/cards/src/scripts/core/c008_mr_vanilla.rs` | `#[cfg(test)] mod tests` of the card file |
| `packages/cards/test/009-moths-to-the-flame.test.ts` | 125 | port-inline | `crates/cards/src/scripts/core/c009_moths_to_the_flame.rs` | `#[cfg(test)] mod tests` of the card file |
| `packages/cards/test/010-rapid-replenish.test.ts` | 153 | port-inline | `crates/cards/src/scripts/core/c010_rapid_replenish.rs` | `#[cfg(test)] mod tests` of the card file |
| `packages/cards/test/011-tempo-timmy.test.ts` | 103 | port-inline | `crates/cards/src/scripts/core/c011_tempo_timmy.rs` | `#[cfg(test)] mod tests` of the card file |
| `packages/cards/test/012-duplicating-felinors.test.ts` | 153 | port-inline | `crates/cards/src/scripts/core/c012_duplicating_felinors.rs` | `#[cfg(test)] mod tests` of the card file |
| `packages/cards/test/013-jlockeed-shredder-10.test.ts` | 108 | port-inline | `crates/cards/src/scripts/core/c013_jlockeed_shredder_10.rs` | `#[cfg(test)] mod tests` of the card file |
| `packages/cards/test/014-jlockeeds-weapons.test.ts` | 143 | port-inline | `crates/cards/src/scripts/core/c014_jlockeeds_weapons.rs` | `#[cfg(test)] mod tests` of the card file |
| `packages/cards/test/015-me-and-mr-token.test.ts` | 89 | port-inline | `crates/cards/src/scripts/core/c015_me_and_mr_token.rs` | `#[cfg(test)] mod tests` of the card file |
| `packages/cards/test/016-hit-job.test.ts` | 183 | port-inline | `crates/cards/src/scripts/core/c016_hit_job.rs` | `#[cfg(test)] mod tests` of the card file |
| `packages/cards/test/017-flood.test.ts` | 206 | port-inline | `crates/cards/src/scripts/core/c017_flood.rs` | `#[cfg(test)] mod tests` of the card file |
| `packages/cards/test/018-bread-and-butter.test.ts` | 205 | port-inline | `crates/cards/src/scripts/core/c018_bread_and_butter.rs` | `#[cfg(test)] mod tests` of the card file |
| `packages/cards/test/019-midrange-menace.test.ts` | 131 | port-inline | `crates/cards/src/scripts/core/c019_midrange_menace.rs` | `#[cfg(test)] mod tests` of the card file |
| `packages/cards/test/020-pointmaster.test.ts` | 105 | port-inline | `crates/cards/src/scripts/core/c020_pointmaster.rs` | `#[cfg(test)] mod tests` of the card file |
| `packages/cards/test/021-hinder.test.ts` | 256 | port-inline | `crates/cards/src/scripts/core/c021_hinder.rs` | `#[cfg(test)] mod tests` of the card file |
| `packages/cards/test/022-carnivorous-cube.test.ts` | 246 | port-inline | `crates/cards/src/scripts/core/c022_carnivorous_cube.rs` | `#[cfg(test)] mod tests` of the card file |
| `packages/cards/test/023-reoccurring-dream.test.ts` | 160 | port-inline | `crates/cards/src/scripts/core/c023_reoccurring_dream.rs` | `#[cfg(test)] mod tests` of the card file |
| `packages/cards/test/024-efficiency-dividend.test.ts` | 233 | port-inline | `crates/cards/src/scripts/core/c024_efficiency_dividend.rs` | `#[cfg(test)] mod tests` of the card file |
| `packages/cards/test/025-4-mana-7-7.test.ts` | 138 | port-inline | `crates/cards/src/scripts/core/c025_4_mana_7_7.rs` | `#[cfg(test)] mod tests` of the card file |
| `packages/cards/test/026-glowy-jelly-bean.test.ts` | 111 | port-inline | `crates/cards/src/scripts/core/c026_glowy_jelly_bean.rs` | `#[cfg(test)] mod tests` of the card file |
| `packages/cards/test/027-blood-ridden-glowy-jelly-bean.test.ts` | 123 | port-inline | `crates/cards/src/scripts/core/c027_blood_ridden_glowy_jelly_bean.rs` | `#[cfg(test)] mod tests` of the card file |
| `packages/cards/test/028-knockoff-temu-glowy-jelly-bean.test.ts` | 152 | port-inline | `crates/cards/src/scripts/core/c028_knockoff_temu_glowy_jelly_bean.rs` | `#[cfg(test)] mod tests` of the card file |
| `packages/cards/test/029-giga-glowy-jelly-bean.test.ts` | 129 | port-inline | `crates/cards/src/scripts/core/c029_giga_glowy_jelly_bean.rs` | `#[cfg(test)] mod tests` of the card file |
| `packages/cards/test/030-archivist.test.ts` | 140 | port-inline | `crates/cards/src/scripts/core/c030_archivist.rs` | `#[cfg(test)] mod tests` of the card file |
| `packages/cards/test/031-kys-math-equation.test.ts` | 229 | port-inline | `crates/cards/src/scripts/core/c031_kys_math_equation.rs` | `#[cfg(test)] mod tests` of the card file |
| `packages/cards/test/032-prem-panther.test.ts` | 315 | port-inline | `crates/cards/src/scripts/core/c032_prem_panther.rs` | `#[cfg(test)] mod tests` of the card file |
| `packages/cards/test/033-unstable-clone-machine.test.ts` | 313 | port-inline | `crates/cards/src/scripts/core/c033_unstable_clone_machine.rs` | `#[cfg(test)] mod tests` of the card file |
| `packages/cards/test/034-collateral-damage.test.ts` | 175 | port-inline | `crates/cards/src/scripts/core/c034_collateral_damage.rs` | `#[cfg(test)] mod tests` of the card file |
| `packages/cards/test/035-lunar-eclipse.test.ts` | 152 | port-inline | `crates/cards/src/scripts/core/c035_lunar_eclipse.rs` | `#[cfg(test)] mod tests` of the card file |
| `packages/cards/test/036-magic-jammed.test.ts` | 135 | port-inline | `crates/cards/src/scripts/core/c036_magic_jammed.rs` | `#[cfg(test)] mod tests` of the card file |
| `packages/cards/test/037-gravedigger.test.ts` | 180 | port-inline | `crates/cards/src/scripts/core/c037_gravedigger.rs` | `#[cfg(test)] mod tests` of the card file |
| `packages/cards/test/038-quickstriker.test.ts` | 292 | port-inline | `crates/cards/src/scripts/core/c038_quickstriker.rs` | `#[cfg(test)] mod tests` of the card file |
| `packages/cards/test/039-recycling-initiative.test.ts` | 330 | port-inline | `crates/cards/src/scripts/core/c039_recycling_initiative.rs` | `#[cfg(test)] mod tests` of the card file |
| `packages/cards/test/040-echoes-of-the-forgotten.test.ts` | 243 | port-inline | `crates/cards/src/scripts/core/c040_echoes_of_the_forgotten.rs` | `#[cfg(test)] mod tests` of the card file |
| `packages/cards/test/041-sheepish.test.ts` | 263 | port-inline | `crates/cards/src/scripts/core/c041_sheepish.rs` | `#[cfg(test)] mod tests` of the card file |
| `packages/cards/test/042-eugenics.test.ts` | 169 | port-inline | `crates/cards/src/scripts/core/c042_eugenics.rs` | `#[cfg(test)] mod tests` of the card file |
| `packages/cards/test/043-big-felinor.test.ts` | 134 | port-inline | `crates/cards/src/scripts/core/c043_big_felinor.rs` | `#[cfg(test)] mod tests` of the card file |
| `packages/cards/test/044-true-strike.test.ts` | 162 | port-inline | `crates/cards/src/scripts/core/c044_true_strike.rs` | `#[cfg(test)] mod tests` of the card file |
| `packages/cards/test/045-deft-duelist.test.ts` | 160 | port-inline | `crates/cards/src/scripts/core/c045_deft_duelist.rs` | `#[cfg(test)] mod tests` of the card file |
| `packages/cards/test/046-suppressive-aura.test.ts` | 210 | port-inline | `crates/cards/src/scripts/core/c046_suppressive_aura.rs` | `#[cfg(test)] mod tests` of the card file |
| `packages/cards/test/047-fig-of-life.test.ts` | 128 | port-inline | `crates/cards/src/scripts/core/c047_fig_of_life.rs` | `#[cfg(test)] mod tests` of the card file |
| `packages/cards/test/048-5pek-controller.test.ts` | 157 | port-inline | `crates/cards/src/scripts/core/c048_5pek_controller.rs` | `#[cfg(test)] mod tests` of the card file |

## Part 10: cards lane 2: Core #49–#81

| TypeScript | Lines | Action | Rust | Note |
|---|---|---|---|---|
| `packages/cards/src/scripts/049-snom-bunny-mind-control.ts` | 50 | port | `crates/cards/src/scripts/core/c049_snom_bunny_mind_control.rs` | cards lane 2 |
| `packages/cards/src/scripts/050-k-pop-fanatic.ts` | 184 | port | `crates/cards/src/scripts/core/c050_k_pop_fanatic.rs` | cards lane 2 |
| `packages/cards/src/scripts/051-1-kys-empty-notebook.ts` | 42 | port | `crates/cards/src/scripts/core/c051_1_kys_empty_notebook.rs` | cards lane 2 |
| `packages/cards/src/scripts/051-kys-private-tutor.ts` | 277 | port | `crates/cards/src/scripts/core/c051_kys_private_tutor.rs` | cards lane 2 |
| `packages/cards/src/scripts/052-silly-silas.ts` | 93 | port | `crates/cards/src/scripts/core/c052_silly_silas.rs` | cards lane 2 |
| `packages/cards/src/scripts/053-reno.ts` | 51 | port | `crates/cards/src/scripts/core/c053_reno.rs` | cards lane 2 |
| `packages/cards/src/scripts/054-straaza.ts` | 71 | port | `crates/cards/src/scripts/core/c054_straaza.rs` | cards lane 2 |
| `packages/cards/src/scripts/055-lava-golem.ts` | 54 | port | `crates/cards/src/scripts/core/c055_lava_golem.rs` | cards lane 2 |
| `packages/cards/src/scripts/056-jilliax.ts` | 33 | port | `crates/cards/src/scripts/core/c056_jilliax.rs` | cards lane 2 |
| `packages/cards/src/scripts/057-conjure-ky.ts` | 51 | port | `crates/cards/src/scripts/core/c057_conjure_ky.rs` | cards lane 2 |
| `packages/cards/src/scripts/058-rush-token-farm.ts` | 48 | port | `crates/cards/src/scripts/core/c058_rush_token_farm.rs` | cards lane 2 |
| `packages/cards/src/scripts/059-unbiased-immigration.ts` | 59 | port | `crates/cards/src/scripts/core/c059_unbiased_immigration.rs` | cards lane 2 |
| `packages/cards/src/scripts/060-bear-honeypot.ts` | 134 | port | `crates/cards/src/scripts/core/c060_bear_honeypot.rs` | cards lane 2 |
| `packages/cards/src/scripts/061-prejudiced-postdoc.ts` | 68 | port | `crates/cards/src/scripts/core/c061_prejudiced_postdoc.rs` | cards lane 2 |
| `packages/cards/src/scripts/062-friend-of-felinors.ts` | 36 | port | `crates/cards/src/scripts/core/c062_friend_of_felinors.rs` | cards lane 2 |
| `packages/cards/src/scripts/063-plastic-surgery.ts` | 49 | port | `crates/cards/src/scripts/core/c063_plastic_surgery.rs` | cards lane 2 |
| `packages/cards/src/scripts/064-gifted-program.ts` | 40 | port | `crates/cards/src/scripts/core/c064_gifted_program.rs` | cards lane 2 |
| `packages/cards/src/scripts/065-1-spikey-pillow.ts` | 61 | port | `crates/cards/src/scripts/core/c065_1_spikey_pillow.rs` | cards lane 2 |
| `packages/cards/src/scripts/065-masochism-mask.ts` | 112 | port | `crates/cards/src/scripts/core/c065_masochism_mask.rs` | cards lane 2 |
| `packages/cards/src/scripts/066-the-rock.ts` | 50 | port | `crates/cards/src/scripts/core/c066_the_rock.rs` | cards lane 2 |
| `packages/cards/src/scripts/067-zoomerbin-oomen.ts` | 76 | port | `crates/cards/src/scripts/core/c067_zoomerbin_oomen.rs` | cards lane 2 |
| `packages/cards/src/scripts/068-twisted-sorcerer.ts` | 89 | port | `crates/cards/src/scripts/core/c068_twisted_sorcerer.rs` | cards lane 2 |
| `packages/cards/src/scripts/069-call-to-arms.ts` | 52 | port | `crates/cards/src/scripts/core/c069_call_to_arms.rs` | cards lane 2 |
| `packages/cards/src/scripts/070-spiteful-stab.ts` | 88 | port | `crates/cards/src/scripts/core/c070_spiteful_stab.rs` | cards lane 2 |
| `packages/cards/src/scripts/071-intern-stimmy.ts` | 82 | port | `crates/cards/src/scripts/core/c071_intern_stimmy.rs` | cards lane 2 |
| `packages/cards/src/scripts/072-reminisce.ts` | 68 | port | `crates/cards/src/scripts/core/c072_reminisce.rs` | cards lane 2 |
| `packages/cards/src/scripts/073-anti-oneshot-armor.ts` | 49 | port | `crates/cards/src/scripts/core/c073_anti_oneshot_armor.rs` | cards lane 2 |
| `packages/cards/src/scripts/074-adaptive-ui.ts` | 83 | port | `crates/cards/src/scripts/core/c074_adaptive_ui.rs` | cards lane 2 |
| `packages/cards/src/scripts/075-infinite-reserves.ts` | 41 | port | `crates/cards/src/scripts/core/c075_infinite_reserves.rs` | cards lane 2 |
| `packages/cards/src/scripts/076-field-of-dreams.ts` | 64 | port | `crates/cards/src/scripts/core/c076_field_of_dreams.rs` | cards lane 2 |
| `packages/cards/src/scripts/077-professor-curvature.ts` | 52 | port | `crates/cards/src/scripts/core/c077_professor_curvature.rs` | cards lane 2 |
| `packages/cards/src/scripts/078-fullsend.ts` | 100 | port | `crates/cards/src/scripts/core/c078_fullsend.rs` | cards lane 2 |
| `packages/cards/src/scripts/079-twinspell.ts` | 37 | port | `crates/cards/src/scripts/core/c079_twinspell.rs` | cards lane 2 |
| `packages/cards/src/scripts/080-zao-gao.ts` | 71 | port | `crates/cards/src/scripts/core/c080_zao_gao.rs` | cards lane 2 |
| `packages/cards/src/scripts/081-radiant-saintess.ts` | 89 | port | `crates/cards/src/scripts/core/c081_radiant_saintess.rs` | cards lane 2 |
| `packages/cards/test/049-snom-bunny-mind-control.test.ts` | 182 | port-inline | `crates/cards/src/scripts/core/c049_snom_bunny_mind_control.rs` | `#[cfg(test)] mod tests` of the card file |
| `packages/cards/test/050-k-pop-fanatic.test.ts` | 737 | port-inline | `crates/cards/src/scripts/core/c050_k_pop_fanatic.rs` | `#[cfg(test)] mod tests` of the card file |
| `packages/cards/test/051-1-kys-empty-notebook.test.ts` | 131 | port-inline | `crates/cards/src/scripts/core/c051_1_kys_empty_notebook.rs` | `#[cfg(test)] mod tests` of the card file |
| `packages/cards/test/051-kys-private-tutor.test.ts` | 460 | port-inline | `crates/cards/src/scripts/core/c051_kys_private_tutor.rs` | `#[cfg(test)] mod tests` of the card file |
| `packages/cards/test/052-silly-silas.test.ts` | 258 | port-inline | `crates/cards/src/scripts/core/c052_silly_silas.rs` | `#[cfg(test)] mod tests` of the card file |
| `packages/cards/test/053-reno.test.ts` | 83 | port-inline | `crates/cards/src/scripts/core/c053_reno.rs` | `#[cfg(test)] mod tests` of the card file |
| `packages/cards/test/054-straaza.test.ts` | 234 | port-inline | `crates/cards/src/scripts/core/c054_straaza.rs` | `#[cfg(test)] mod tests` of the card file |
| `packages/cards/test/055-lava-golem.test.ts` | 300 | port-inline | `crates/cards/src/scripts/core/c055_lava_golem.rs` | `#[cfg(test)] mod tests` of the card file |
| `packages/cards/test/056-jilliax.test.ts` | 177 | port-inline | `crates/cards/src/scripts/core/c056_jilliax.rs` | `#[cfg(test)] mod tests` of the card file |
| `packages/cards/test/057-conjure-ky.test.ts` | 159 | port-inline | `crates/cards/src/scripts/core/c057_conjure_ky.rs` | `#[cfg(test)] mod tests` of the card file |
| `packages/cards/test/058-rush-token-farm.test.ts` | 182 | port-inline | `crates/cards/src/scripts/core/c058_rush_token_farm.rs` | `#[cfg(test)] mod tests` of the card file |
| `packages/cards/test/059-unbiased-immigration.test.ts` | 230 | port-inline | `crates/cards/src/scripts/core/c059_unbiased_immigration.rs` | `#[cfg(test)] mod tests` of the card file |
| `packages/cards/test/060-bear-honeypot.test.ts` | 476 | port-inline | `crates/cards/src/scripts/core/c060_bear_honeypot.rs` | `#[cfg(test)] mod tests` of the card file |
| `packages/cards/test/061-prejudiced-postdoc.test.ts` | 191 | port-inline | `crates/cards/src/scripts/core/c061_prejudiced_postdoc.rs` | `#[cfg(test)] mod tests` of the card file |
| `packages/cards/test/062-friend-of-felinors.test.ts` | 92 | port-inline | `crates/cards/src/scripts/core/c062_friend_of_felinors.rs` | `#[cfg(test)] mod tests` of the card file |
| `packages/cards/test/063-plastic-surgery.test.ts` | 153 | port-inline | `crates/cards/src/scripts/core/c063_plastic_surgery.rs` | `#[cfg(test)] mod tests` of the card file |
| `packages/cards/test/064-gifted-program.test.ts` | 190 | port-inline | `crates/cards/src/scripts/core/c064_gifted_program.rs` | `#[cfg(test)] mod tests` of the card file |
| `packages/cards/test/065-1-spikey-pillow.test.ts` | 134 | port-inline | `crates/cards/src/scripts/core/c065_1_spikey_pillow.rs` | `#[cfg(test)] mod tests` of the card file |
| `packages/cards/test/065-masochism-mask.test.ts` | 156 | port-inline | `crates/cards/src/scripts/core/c065_masochism_mask.rs` | `#[cfg(test)] mod tests` of the card file |
| `packages/cards/test/066-the-rock.test.ts` | 183 | port-inline | `crates/cards/src/scripts/core/c066_the_rock.rs` | `#[cfg(test)] mod tests` of the card file |
| `packages/cards/test/067-zoomerbin-oomen.test.ts` | 249 | port-inline | `crates/cards/src/scripts/core/c067_zoomerbin_oomen.rs` | `#[cfg(test)] mod tests` of the card file |
| `packages/cards/test/068-twisted-sorcerer.test.ts` | 167 | port-inline | `crates/cards/src/scripts/core/c068_twisted_sorcerer.rs` | `#[cfg(test)] mod tests` of the card file |
| `packages/cards/test/069-call-to-arms.test.ts` | 205 | port-inline | `crates/cards/src/scripts/core/c069_call_to_arms.rs` | `#[cfg(test)] mod tests` of the card file |
| `packages/cards/test/070-spiteful-stab.test.ts` | 227 | port-inline | `crates/cards/src/scripts/core/c070_spiteful_stab.rs` | `#[cfg(test)] mod tests` of the card file |
| `packages/cards/test/071-intern-stimmy.test.ts` | 174 | port-inline | `crates/cards/src/scripts/core/c071_intern_stimmy.rs` | `#[cfg(test)] mod tests` of the card file |
| `packages/cards/test/072-reminisce.test.ts` | 217 | port-inline | `crates/cards/src/scripts/core/c072_reminisce.rs` | `#[cfg(test)] mod tests` of the card file |
| `packages/cards/test/073-anti-oneshot-armor.test.ts` | 228 | port-inline | `crates/cards/src/scripts/core/c073_anti_oneshot_armor.rs` | `#[cfg(test)] mod tests` of the card file |
| `packages/cards/test/074-adaptive-ui.test.ts` | 218 | port-inline | `crates/cards/src/scripts/core/c074_adaptive_ui.rs` | `#[cfg(test)] mod tests` of the card file |
| `packages/cards/test/075-infinite-reserves.test.ts` | 165 | port-inline | `crates/cards/src/scripts/core/c075_infinite_reserves.rs` | `#[cfg(test)] mod tests` of the card file |
| `packages/cards/test/076-field-of-dreams.test.ts` | 136 | port-inline | `crates/cards/src/scripts/core/c076_field_of_dreams.rs` | `#[cfg(test)] mod tests` of the card file |
| `packages/cards/test/077-professor-curvature.test.ts` | 209 | port-inline | `crates/cards/src/scripts/core/c077_professor_curvature.rs` | `#[cfg(test)] mod tests` of the card file |
| `packages/cards/test/078-fullsend.test.ts` | 290 | port-inline | `crates/cards/src/scripts/core/c078_fullsend.rs` | `#[cfg(test)] mod tests` of the card file |
| `packages/cards/test/079-twinspell.test.ts` | 201 | port-inline | `crates/cards/src/scripts/core/c079_twinspell.rs` | `#[cfg(test)] mod tests` of the card file |
| `packages/cards/test/080-zao-gao.test.ts` | 304 | port-inline | `crates/cards/src/scripts/core/c080_zao_gao.rs` | `#[cfg(test)] mod tests` of the card file |
| `packages/cards/test/081-radiant-saintess.test.ts` | 314 | port-inline | `crates/cards/src/scripts/core/c081_radiant_saintess.rs` | `#[cfg(test)] mod tests` of the card file |

## Part 11: cards lane 3: Core #82–#100 and the Core tokens to Ghoul

| TypeScript | Lines | Action | Rust | Note |
|---|---|---|---|---|
| `packages/cards/src/scripts/082-kys-trial.ts` | 93 | port | `crates/cards/src/scripts/core/c082_kys_trial.rs` | cards lane 3 |
| `packages/cards/src/scripts/083-transmogulate.ts` | 146 | port | `crates/cards/src/scripts/core/c083_transmogulate.rs` | cards lane 3 |
| `packages/cards/src/scripts/084-going-long.ts` | 50 | port | `crates/cards/src/scripts/core/c084_going_long.rs` | cards lane 3 |
| `packages/cards/src/scripts/085-unlicensed-experimentation.ts` | 176 | port | `crates/cards/src/scripts/core/c085_unlicensed_experimentation.rs` | cards lane 3 |
| `packages/cards/src/scripts/086-miss-mrow.ts` | 40 | port | `crates/cards/src/scripts/core/c086_miss_mrow.rs` | cards lane 3 |
| `packages/cards/src/scripts/087-pocket-chaos.ts` | 99 | port | `crates/cards/src/scripts/core/c087_pocket_chaos.rs` | cards lane 3 |
| `packages/cards/src/scripts/088-twisting-nether.ts` | 67 | port | `crates/cards/src/scripts/core/c088_twisting_nether.rs` | cards lane 3 |
| `packages/cards/src/scripts/089-corpse-eater.ts` | 89 | port | `crates/cards/src/scripts/core/c089_corpse_eater.rs` | cards lane 3 |
| `packages/cards/src/scripts/090-1-cn-virus.ts` | 85 | port | `crates/cards/src/scripts/core/c090_1_cn_virus.rs` | cards lane 3 |
| `packages/cards/src/scripts/090-cn-viral-injection.ts` | 52 | port | `crates/cards/src/scripts/core/c090_cn_viral_injection.rs` | cards lane 3 |
| `packages/cards/src/scripts/091-fed-fauci.ts` | 95 | port | `crates/cards/src/scripts/core/c091_fed_fauci.rs` | cards lane 3 |
| `packages/cards/src/scripts/092-felinor-fiender.ts` | 87 | port | `crates/cards/src/scripts/core/c092_felinor_fiender.rs` | cards lane 3 |
| `packages/cards/src/scripts/093-1-combo-fodder.ts` | 55 | port | `crates/cards/src/scripts/core/c093_1_combo_fodder.rs` | cards lane 3 |
| `packages/cards/src/scripts/093-combo-index.ts` | 96 | port | `crates/cards/src/scripts/core/c093_combo_index.rs` | cards lane 3 |
| `packages/cards/src/scripts/094-genns-greed.ts` | 80 | port | `crates/cards/src/scripts/core/c094_genns_greed.rs` | cards lane 3 |
| `packages/cards/src/scripts/095-1-chaos-golem.ts` | 41 | port | `crates/cards/src/scripts/core/c095_1_chaos_golem.rs` | cards lane 3 |
| `packages/cards/src/scripts/095-call-to-chaos.ts` | 41 | port | `crates/cards/src/scripts/core/c095_call_to_chaos.rs` | cards lane 3 |
| `packages/cards/src/scripts/096-my-pawn.ts` | 130 | port | `crates/cards/src/scripts/core/c096_my_pawn.rs` | cards lane 3 |
| `packages/cards/src/scripts/097-zephyrs.ts` | 83 | port | `crates/cards/src/scripts/core/c097_zephyrs.rs` | cards lane 3 |
| `packages/cards/src/scripts/098-heroic-power.ts` | 62 | port | `crates/cards/src/scripts/core/c098_heroic_power.rs` | cards lane 3 |
| `packages/cards/src/scripts/099-craft-a-card.ts` | 120 | port | `crates/cards/src/scripts/core/c099_craft_a_card.rs` | cards lane 3 |
| `packages/cards/src/scripts/100-ceaseless-void.ts` | 103 | port | `crates/cards/src/scripts/core/c100_ceaseless_void.rs` | cards lane 3 |
| `packages/cards/src/scripts/t-bread.ts` | 37 | port | `crates/cards/src/scripts/core/t_bread.rs` | cards lane 3 |
| `packages/cards/src/scripts/t-coin.ts` | 38 | port | `crates/cards/src/scripts/core/t_coin.rs` | cards lane 3 |
| `packages/cards/src/scripts/t-felinor.ts` | 31 | port | `crates/cards/src/scripts/core/t_felinor.rs` | cards lane 3 |
| `packages/cards/src/scripts/t-ghoul.ts` | 29 | port | `crates/cards/src/scripts/core/t_ghoul.rs` | cards lane 3 |
| `packages/cards/test/082-kys-trial.test.ts` | 179 | port-inline | `crates/cards/src/scripts/core/c082_kys_trial.rs` | `#[cfg(test)] mod tests` of the card file |
| `packages/cards/test/083-transmogulate.test.ts` | 267 | port-inline | `crates/cards/src/scripts/core/c083_transmogulate.rs` | `#[cfg(test)] mod tests` of the card file |
| `packages/cards/test/084-going-long.test.ts` | 262 | port-inline | `crates/cards/src/scripts/core/c084_going_long.rs` | `#[cfg(test)] mod tests` of the card file |
| `packages/cards/test/085-unlicensed-experimentation.test.ts` | 408 | port-inline | `crates/cards/src/scripts/core/c085_unlicensed_experimentation.rs` | `#[cfg(test)] mod tests` of the card file |
| `packages/cards/test/086-miss-mrow.test.ts` | 213 | port-inline | `crates/cards/src/scripts/core/c086_miss_mrow.rs` | `#[cfg(test)] mod tests` of the card file |
| `packages/cards/test/087-pocket-chaos.test.ts` | 355 | port-inline | `crates/cards/src/scripts/core/c087_pocket_chaos.rs` | `#[cfg(test)] mod tests` of the card file |
| `packages/cards/test/088-twisting-nether.test.ts` | 181 | port-inline | `crates/cards/src/scripts/core/c088_twisting_nether.rs` | `#[cfg(test)] mod tests` of the card file |
| `packages/cards/test/089-corpse-eater.test.ts` | 248 | port-inline | `crates/cards/src/scripts/core/c089_corpse_eater.rs` | `#[cfg(test)] mod tests` of the card file |
| `packages/cards/test/090-cn-viral-injection.test.ts` | 607 | port-inline | `crates/cards/src/scripts/core/c090_cn_viral_injection.rs` | `#[cfg(test)] mod tests` of the card file |
| `packages/cards/test/091-fed-fauci.test.ts` | 198 | port-inline | `crates/cards/src/scripts/core/c091_fed_fauci.rs` | `#[cfg(test)] mod tests` of the card file |
| `packages/cards/test/092-felinor-fiender.test.ts` | 310 | port-inline | `crates/cards/src/scripts/core/c092_felinor_fiender.rs` | `#[cfg(test)] mod tests` of the card file |
| `packages/cards/test/093-combo-index.test.ts` | 971 | port-inline | `crates/cards/src/scripts/core/c093_combo_index.rs` | `#[cfg(test)] mod tests` of the card file |
| `packages/cards/test/094-genns-greed.test.ts` | 418 | port-inline | `crates/cards/src/scripts/core/c094_genns_greed.rs` | `#[cfg(test)] mod tests` of the card file |
| `packages/cards/test/095-call-to-chaos.test.ts` | 738 | port-inline | `crates/cards/src/scripts/core/c095_call_to_chaos.rs` | `#[cfg(test)] mod tests` of the card file |
| `packages/cards/test/096-my-pawn.test.ts` | 470 | port-inline | `crates/cards/src/scripts/core/c096_my_pawn.rs` | `#[cfg(test)] mod tests` of the card file |
| `packages/cards/test/097-zephyrs.test.ts` | 381 | port-inline | `crates/cards/src/scripts/core/c097_zephyrs.rs` | `#[cfg(test)] mod tests` of the card file |
| `packages/cards/test/098-heroic-power.test.ts` | 450 | port-inline | `crates/cards/src/scripts/core/c098_heroic_power.rs` | `#[cfg(test)] mod tests` of the card file |
| `packages/cards/test/099-craft-a-card.test.ts` | 504 | port-inline | `crates/cards/src/scripts/core/c099_craft_a_card.rs` | `#[cfg(test)] mod tests` of the card file |
| `packages/cards/test/100-ceaseless-void.test.ts` | 440 | port-inline | `crates/cards/src/scripts/core/c100_ceaseless_void.rs` | `#[cfg(test)] mod tests` of the card file |
| `packages/cards/test/t-bread.test.ts` | 251 | port-inline | `crates/cards/src/scripts/core/t_bread.rs` | `#[cfg(test)] mod tests` of the card file |
| `packages/cards/test/t-coin.test.ts` | 320 | port-inline | `crates/cards/src/scripts/core/t_coin.rs` | `#[cfg(test)] mod tests` of the card file |
| `packages/cards/test/t-felinor.test.ts` | 234 | port-inline | `crates/cards/src/scripts/core/t_felinor.rs` | `#[cfg(test)] mod tests` of the card file |
| `packages/cards/test/t-ghoul.test.ts` | 177 | port-inline | `crates/cards/src/scripts/core/t_ghoul.rs` | `#[cfg(test)] mod tests` of the card file |

## Part 12: cards lane 4: Core tokens Rush to Sheep, Classic #1–#32

| TypeScript | Lines | Action | Rust | Note |
|---|---|---|---|---|
| `packages/cards/src/scripts/classic/001-curse-of-the-forgotten-classic.ts` | 90 | port | `crates/cards/src/scripts/classic/c001_curse_of_the_forgotten_classic.rs` | cards lane 4 |
| `packages/cards/src/scripts/classic/002-the-trickster.ts` | 37 | port | `crates/cards/src/scripts/classic/c002_the_trickster.rs` | cards lane 4 |
| `packages/cards/src/scripts/classic/003-book-of-heal.ts` | 37 | port | `crates/cards/src/scripts/classic/c003_book_of_heal.rs` | cards lane 4 |
| `packages/cards/src/scripts/classic/004-palantir.ts` | 86 | port | `crates/cards/src/scripts/classic/c004_palantir.rs` | cards lane 4 |
| `packages/cards/src/scripts/classic/005-tesla.ts` | 65 | port | `crates/cards/src/scripts/classic/c005_tesla.rs` | cards lane 4 |
| `packages/cards/src/scripts/classic/006-cloaked-toe-cracker.ts` | 50 | port | `crates/cards/src/scripts/classic/c006_cloaked_toe_cracker.rs` | cards lane 4 |
| `packages/cards/src/scripts/classic/007-infiniscepter.ts` | 76 | port | `crates/cards/src/scripts/classic/c007_infiniscepter.rs` | cards lane 4 |
| `packages/cards/src/scripts/classic/008-pickle.ts` | 144 | port | `crates/cards/src/scripts/classic/c008_pickle.rs` | cards lane 4 |
| `packages/cards/src/scripts/classic/009-income-tax.ts` | 86 | port | `crates/cards/src/scripts/classic/c009_income_tax.rs` | cards lane 4 |
| `packages/cards/src/scripts/classic/010-exile.ts` | 89 | port | `crates/cards/src/scripts/classic/c010_exile.rs` | cards lane 4 |
| `packages/cards/src/scripts/classic/011-mind-melt.ts` | 52 | port | `crates/cards/src/scripts/classic/c011_mind_melt.rs` | cards lane 4 |
| `packages/cards/src/scripts/classic/012-book-of-blood.ts` | 33 | port | `crates/cards/src/scripts/classic/c012_book_of_blood.rs` | cards lane 4 |
| `packages/cards/src/scripts/classic/013-boots-on-the-ground.ts` | 39 | port | `crates/cards/src/scripts/classic/c013_boots_on_the_ground.rs` | cards lane 4 |
| `packages/cards/src/scripts/classic/014-shadowstep.ts` | 97 | port | `crates/cards/src/scripts/classic/c014_shadowstep.rs` | cards lane 4 |
| `packages/cards/src/scripts/classic/015-nose-hunter.ts` | 47 | port | `crates/cards/src/scripts/classic/c015_nose_hunter.rs` | cards lane 4 |
| `packages/cards/src/scripts/classic/016-book-of-flame.ts` | 32 | port | `crates/cards/src/scripts/classic/c016_book_of_flame.rs` | cards lane 4 |
| `packages/cards/src/scripts/classic/017-counterspell.ts` | 65 | port | `crates/cards/src/scripts/classic/c017_counterspell.rs` | cards lane 4 |
| `packages/cards/src/scripts/classic/018-glitch-in-the-system.ts` | 81 | port | `crates/cards/src/scripts/classic/c018_glitch_in_the_system.rs` | cards lane 4 |
| `packages/cards/src/scripts/classic/019-lizards-breath.ts` | 79 | port | `crates/cards/src/scripts/classic/c019_lizards_breath.rs` | cards lane 4 |
| `packages/cards/src/scripts/classic/020-the-power-to-punish.ts` | 75 | port | `crates/cards/src/scripts/classic/c020_the_power_to_punish.rs` | cards lane 4 |
| `packages/cards/src/scripts/classic/021-turtinator.ts` | 43 | port | `crates/cards/src/scripts/classic/c021_turtinator.rs` | cards lane 4 |
| `packages/cards/src/scripts/classic/022-mid-runner.ts` | 88 | port | `crates/cards/src/scripts/classic/c022_mid_runner.rs` | cards lane 4 |
| `packages/cards/src/scripts/classic/023-devils-pact.ts` | 61 | port | `crates/cards/src/scripts/classic/c023_devils_pact.rs` | cards lane 4 |
| `packages/cards/src/scripts/classic/024-book-of-knowledge.ts` | 24 | port | `crates/cards/src/scripts/classic/c024_book_of_knowledge.rs` | cards lane 4 |
| `packages/cards/src/scripts/classic/025-lag-in-the-system.ts` | 63 | port | `crates/cards/src/scripts/classic/c025_lag_in_the_system.rs` | cards lane 4 |
| `packages/cards/src/scripts/classic/026-rapid-draw.ts` | 44 | port | `crates/cards/src/scripts/classic/c026_rapid_draw.rs` | cards lane 4 |
| `packages/cards/src/scripts/classic/027-pestilent-slime.ts` | 28 | port | `crates/cards/src/scripts/classic/c027_pestilent_slime.rs` | cards lane 4 |
| `packages/cards/src/scripts/classic/028-second-wind.ts` | 67 | port | `crates/cards/src/scripts/classic/c028_second_wind.rs` | cards lane 4 |
| `packages/cards/src/scripts/classic/029-book-of-vital-kill.ts` | 41 | port | `crates/cards/src/scripts/classic/c029_book_of_vital_kill.rs` | cards lane 4 |
| `packages/cards/src/scripts/classic/030-recycle.ts` | 57 | port | `crates/cards/src/scripts/classic/c030_recycle.rs` | cards lane 4 |
| `packages/cards/src/scripts/classic/031-cookie-guild.ts` | 35 | port | `crates/cards/src/scripts/classic/c031_cookie_guild.rs` | cards lane 4 |
| `packages/cards/src/scripts/classic/032-felinor-feelings.ts` | 74 | port | `crates/cards/src/scripts/classic/c032_felinor_feelings.rs` | cards lane 4 |
| `packages/cards/src/scripts/t-rush.ts` | 46 | port | `crates/cards/src/scripts/core/t_rush.rs` | cards lane 4 |
| `packages/cards/src/scripts/t-sheep.ts` | 34 | port | `crates/cards/src/scripts/core/t_sheep.rs` | cards lane 4 |
| `packages/cards/test/classic/001-curse-of-the-forgotten-classic.test.ts` | 249 | port-inline | `crates/cards/src/scripts/classic/c001_curse_of_the_forgotten_classic.rs` | `#[cfg(test)] mod tests` of the card file |
| `packages/cards/test/classic/002-the-trickster.test.ts` | 206 | port-inline | `crates/cards/src/scripts/classic/c002_the_trickster.rs` | `#[cfg(test)] mod tests` of the card file |
| `packages/cards/test/classic/003-book-of-heal.test.ts` | 127 | port-inline | `crates/cards/src/scripts/classic/c003_book_of_heal.rs` | `#[cfg(test)] mod tests` of the card file |
| `packages/cards/test/classic/004-palantir.test.ts` | 416 | port-inline | `crates/cards/src/scripts/classic/c004_palantir.rs` | `#[cfg(test)] mod tests` of the card file |
| `packages/cards/test/classic/005-tesla.test.ts` | 299 | port-inline | `crates/cards/src/scripts/classic/c005_tesla.rs` | `#[cfg(test)] mod tests` of the card file |
| `packages/cards/test/classic/006-cloaked-toe-cracker.test.ts` | 229 | port-inline | `crates/cards/src/scripts/classic/c006_cloaked_toe_cracker.rs` | `#[cfg(test)] mod tests` of the card file |
| `packages/cards/test/classic/007-infiniscepter.test.ts` | 314 | port-inline | `crates/cards/src/scripts/classic/c007_infiniscepter.rs` | `#[cfg(test)] mod tests` of the card file |
| `packages/cards/test/classic/008-pickle.test.ts` | 330 | port-inline | `crates/cards/src/scripts/classic/c008_pickle.rs` | `#[cfg(test)] mod tests` of the card file |
| `packages/cards/test/classic/009-income-tax.test.ts` | 388 | port-inline | `crates/cards/src/scripts/classic/c009_income_tax.rs` | `#[cfg(test)] mod tests` of the card file |
| `packages/cards/test/classic/010-exile.test.ts` | 324 | port-inline | `crates/cards/src/scripts/classic/c010_exile.rs` | `#[cfg(test)] mod tests` of the card file |
| `packages/cards/test/classic/011-mind-melt.test.ts` | 243 | port-inline | `crates/cards/src/scripts/classic/c011_mind_melt.rs` | `#[cfg(test)] mod tests` of the card file |
| `packages/cards/test/classic/012-book-of-blood.test.ts` | 168 | port-inline | `crates/cards/src/scripts/classic/c012_book_of_blood.rs` | `#[cfg(test)] mod tests` of the card file |
| `packages/cards/test/classic/013-boots-on-the-ground.test.ts` | 247 | port-inline | `crates/cards/src/scripts/classic/c013_boots_on_the_ground.rs` | `#[cfg(test)] mod tests` of the card file |
| `packages/cards/test/classic/014-shadowstep.test.ts` | 299 | port-inline | `crates/cards/src/scripts/classic/c014_shadowstep.rs` | `#[cfg(test)] mod tests` of the card file |
| `packages/cards/test/classic/015-nose-hunter.test.ts` | 270 | port-inline | `crates/cards/src/scripts/classic/c015_nose_hunter.rs` | `#[cfg(test)] mod tests` of the card file |
| `packages/cards/test/classic/016-book-of-flame.test.ts` | 124 | port-inline | `crates/cards/src/scripts/classic/c016_book_of_flame.rs` | `#[cfg(test)] mod tests` of the card file |
| `packages/cards/test/classic/017-counterspell.test.ts` | 271 | port-inline | `crates/cards/src/scripts/classic/c017_counterspell.rs` | `#[cfg(test)] mod tests` of the card file |
| `packages/cards/test/classic/018-glitch-in-the-system.test.ts` | 278 | port-inline | `crates/cards/src/scripts/classic/c018_glitch_in_the_system.rs` | `#[cfg(test)] mod tests` of the card file |
| `packages/cards/test/classic/019-lizards-breath.test.ts` | 229 | port-inline | `crates/cards/src/scripts/classic/c019_lizards_breath.rs` | `#[cfg(test)] mod tests` of the card file |
| `packages/cards/test/classic/020-the-power-to-punish.test.ts` | 387 | port-inline | `crates/cards/src/scripts/classic/c020_the_power_to_punish.rs` | `#[cfg(test)] mod tests` of the card file |
| `packages/cards/test/classic/021-turtinator.test.ts` | 275 | port-inline | `crates/cards/src/scripts/classic/c021_turtinator.rs` | `#[cfg(test)] mod tests` of the card file |
| `packages/cards/test/classic/022-mid-runner.test.ts` | 230 | port-inline | `crates/cards/src/scripts/classic/c022_mid_runner.rs` | `#[cfg(test)] mod tests` of the card file |
| `packages/cards/test/classic/023-devils-pact.test.ts` | 337 | port-inline | `crates/cards/src/scripts/classic/c023_devils_pact.rs` | `#[cfg(test)] mod tests` of the card file |
| `packages/cards/test/classic/024-book-of-knowledge.test.ts` | 157 | port-inline | `crates/cards/src/scripts/classic/c024_book_of_knowledge.rs` | `#[cfg(test)] mod tests` of the card file |
| `packages/cards/test/classic/025-lag-in-the-system.test.ts` | 276 | port-inline | `crates/cards/src/scripts/classic/c025_lag_in_the_system.rs` | `#[cfg(test)] mod tests` of the card file |
| `packages/cards/test/classic/026-rapid-draw.test.ts` | 184 | port-inline | `crates/cards/src/scripts/classic/c026_rapid_draw.rs` | `#[cfg(test)] mod tests` of the card file |
| `packages/cards/test/classic/027-pestilent-slime.test.ts` | 162 | port-inline | `crates/cards/src/scripts/classic/c027_pestilent_slime.rs` | `#[cfg(test)] mod tests` of the card file |
| `packages/cards/test/classic/028-second-wind.test.ts` | 293 | port-inline | `crates/cards/src/scripts/classic/c028_second_wind.rs` | `#[cfg(test)] mod tests` of the card file |
| `packages/cards/test/classic/029-book-of-vital-kill.test.ts` | 189 | port-inline | `crates/cards/src/scripts/classic/c029_book_of_vital_kill.rs` | `#[cfg(test)] mod tests` of the card file |
| `packages/cards/test/classic/030-recycle.test.ts` | 197 | port-inline | `crates/cards/src/scripts/classic/c030_recycle.rs` | `#[cfg(test)] mod tests` of the card file |
| `packages/cards/test/classic/031-cookie-guild.test.ts` | 209 | port-inline | `crates/cards/src/scripts/classic/c031_cookie_guild.rs` | `#[cfg(test)] mod tests` of the card file |
| `packages/cards/test/classic/032-felinor-feelings.test.ts` | 292 | port-inline | `crates/cards/src/scripts/classic/c032_felinor_feelings.rs` | `#[cfg(test)] mod tests` of the card file |
| `packages/cards/test/t-rush.test.ts` | 277 | port-inline | `crates/cards/src/scripts/core/t_rush.rs` | `#[cfg(test)] mod tests` of the card file |
| `packages/cards/test/t-sheep.test.ts` | 224 | port-inline | `crates/cards/src/scripts/core/t_sheep.rs` | `#[cfg(test)] mod tests` of the card file |

## Part 13: cards lane 5: Classic #33–#68

| TypeScript | Lines | Action | Rust | Note |
|---|---|---|---|---|
| `packages/cards/src/scripts/classic/033-joro.ts` | 45 | port | `crates/cards/src/scripts/classic/c033_joro.rs` | cards lane 5 |
| `packages/cards/src/scripts/classic/034-ancient-acquisition.ts` | 30 | port | `crates/cards/src/scripts/classic/c034_ancient_acquisition.rs` | cards lane 5 |
| `packages/cards/src/scripts/classic/035-prep.ts` | 44 | port | `crates/cards/src/scripts/classic/c035_prep.rs` | cards lane 5 |
| `packages/cards/src/scripts/classic/036-burn.ts` | 67 | port | `crates/cards/src/scripts/classic/c036_burn.rs` | cards lane 5 |
| `packages/cards/src/scripts/classic/037-last-hurrah.ts` | 33 | port | `crates/cards/src/scripts/classic/c037_last_hurrah.rs` | cards lane 5 |
| `packages/cards/src/scripts/classic/038-jackiestan-auctioneer.ts` | 75 | port | `crates/cards/src/scripts/classic/c038_jackiestan_auctioneer.rs` | cards lane 5 |
| `packages/cards/src/scripts/classic/039-outbreak.ts` | 79 | port | `crates/cards/src/scripts/classic/c039_outbreak.rs` | cards lane 5 |
| `packages/cards/src/scripts/classic/040-mc-tech.ts` | 89 | port | `crates/cards/src/scripts/classic/c040_mc_tech.rs` | cards lane 5 |
| `packages/cards/src/scripts/classic/041-state-of-the-game.ts` | 27 | port | `crates/cards/src/scripts/classic/c041_state_of_the_game.rs` | cards lane 5 |
| `packages/cards/src/scripts/classic/042-transmutable-toxins.ts` | 63 | port | `crates/cards/src/scripts/classic/c042_transmutable_toxins.rs` | cards lane 5 |
| `packages/cards/src/scripts/classic/043-plague-nuke.ts` | 94 | port | `crates/cards/src/scripts/classic/c043_plague_nuke.rs` | cards lane 5 |
| `packages/cards/src/scripts/classic/044-back-from-the-gy.ts` | 63 | port | `crates/cards/src/scripts/classic/c044_back_from_the_gy.rs` | cards lane 5 |
| `packages/cards/src/scripts/classic/045-nature-titan.ts` | 58 | port | `crates/cards/src/scripts/classic/c045_nature_titan.rs` | cards lane 5 |
| `packages/cards/src/scripts/classic/046-divine-favor.ts` | 48 | port | `crates/cards/src/scripts/classic/c046_divine_favor.rs` | cards lane 5 |
| `packages/cards/src/scripts/classic/047-recurring-felinor.ts` | 39 | port | `crates/cards/src/scripts/classic/c047_recurring_felinor.rs` | cards lane 5 |
| `packages/cards/src/scripts/classic/048-hired-shrimp.ts` | 38 | port | `crates/cards/src/scripts/classic/c048_hired_shrimp.rs` | cards lane 5 |
| `packages/cards/src/scripts/classic/049-anti-greed-machine.ts` | 33 | port | `crates/cards/src/scripts/classic/c049_anti_greed_machine.rs` | cards lane 5 |
| `packages/cards/src/scripts/classic/050-voidwalker.ts` | 32 | port | `crates/cards/src/scripts/classic/c050_voidwalker.rs` | cards lane 5 |
| `packages/cards/src/scripts/classic/051-back-breaker.ts` | 34 | port | `crates/cards/src/scripts/classic/c051_back_breaker.rs` | cards lane 5 |
| `packages/cards/src/scripts/classic/052-final-gambit.ts` | 43 | port | `crates/cards/src/scripts/classic/c052_final_gambit.rs` | cards lane 5 |
| `packages/cards/src/scripts/classic/053-plague-crawler.ts` | 67 | port | `crates/cards/src/scripts/classic/c053_plague_crawler.rs` | cards lane 5 |
| `packages/cards/src/scripts/classic/054-rewind.ts` | 28 | port | `crates/cards/src/scripts/classic/c054_rewind.rs` | cards lane 5 |
| `packages/cards/src/scripts/classic/055-book-of-wildfire.ts` | 42 | port | `crates/cards/src/scripts/classic/c055_book_of_wildfire.rs` | cards lane 5 |
| `packages/cards/src/scripts/classic/056-spell-tyrant.ts` | 41 | port | `crates/cards/src/scripts/classic/c056_spell_tyrant.rs` | cards lane 5 |
| `packages/cards/src/scripts/classic/057-echo.ts` | 49 | port | `crates/cards/src/scripts/classic/c057_echo.rs` | cards lane 5 |
| `packages/cards/src/scripts/classic/058-common-resources.ts` | 39 | port | `crates/cards/src/scripts/classic/c058_common_resources.rs` | cards lane 5 |
| `packages/cards/src/scripts/classic/059-plague-doctor.ts` | 91 | port | `crates/cards/src/scripts/classic/c059_plague_doctor.rs` | cards lane 5 |
| `packages/cards/src/scripts/classic/060-pile-on.ts` | 24 | port | `crates/cards/src/scripts/classic/c060_pile_on.rs` | cards lane 5 |
| `packages/cards/src/scripts/classic/061-plague-bringer-goliath.ts` | 41 | port | `crates/cards/src/scripts/classic/c061_plague_bringer_goliath.rs` | cards lane 5 |
| `packages/cards/src/scripts/classic/062-living-bomb.ts` | 45 | port | `crates/cards/src/scripts/classic/c062_living_bomb.rs` | cards lane 5 |
| `packages/cards/src/scripts/classic/063-crop-dusting.ts` | 51 | port | `crates/cards/src/scripts/classic/c063_crop_dusting.rs` | cards lane 5 |
| `packages/cards/src/scripts/classic/064-malzahars-recycler.ts` | 66 | port | `crates/cards/src/scripts/classic/c064_malzahars_recycler.rs` | cards lane 5 |
| `packages/cards/src/scripts/classic/065-ace-in-the-hole.ts` | 51 | port | `crates/cards/src/scripts/classic/c065_ace_in_the_hole.rs` | cards lane 5 |
| `packages/cards/src/scripts/classic/066-eu-striker.ts` | 73 | port | `crates/cards/src/scripts/classic/c066_eu_striker.rs` | cards lane 5 |
| `packages/cards/src/scripts/classic/067-felinor-feeler.ts` | 35 | port | `crates/cards/src/scripts/classic/c067_felinor_feeler.rs` | cards lane 5 |
| `packages/cards/src/scripts/classic/068-small-card-lobbyist.ts` | 37 | port | `crates/cards/src/scripts/classic/c068_small_card_lobbyist.rs` | cards lane 5 |
| `packages/cards/test/classic/033-joro.test.ts` | 316 | port-inline | `crates/cards/src/scripts/classic/c033_joro.rs` | `#[cfg(test)] mod tests` of the card file |
| `packages/cards/test/classic/034-ancient-acquisition.test.ts` | 246 | port-inline | `crates/cards/src/scripts/classic/c034_ancient_acquisition.rs` | `#[cfg(test)] mod tests` of the card file |
| `packages/cards/test/classic/035-prep.test.ts` | 164 | port-inline | `crates/cards/src/scripts/classic/c035_prep.rs` | `#[cfg(test)] mod tests` of the card file |
| `packages/cards/test/classic/036-burn.test.ts` | 137 | port-inline | `crates/cards/src/scripts/classic/c036_burn.rs` | `#[cfg(test)] mod tests` of the card file |
| `packages/cards/test/classic/037-last-hurrah.test.ts` | 202 | port-inline | `crates/cards/src/scripts/classic/c037_last_hurrah.rs` | `#[cfg(test)] mod tests` of the card file |
| `packages/cards/test/classic/038-jackiestan-auctioneer.test.ts` | 270 | port-inline | `crates/cards/src/scripts/classic/c038_jackiestan_auctioneer.rs` | `#[cfg(test)] mod tests` of the card file |
| `packages/cards/test/classic/039-outbreak.test.ts` | 253 | port-inline | `crates/cards/src/scripts/classic/c039_outbreak.rs` | `#[cfg(test)] mod tests` of the card file |
| `packages/cards/test/classic/040-mc-tech.test.ts` | 261 | port-inline | `crates/cards/src/scripts/classic/c040_mc_tech.rs` | `#[cfg(test)] mod tests` of the card file |
| `packages/cards/test/classic/041-state-of-the-game.test.ts` | 214 | port-inline | `crates/cards/src/scripts/classic/c041_state_of_the_game.rs` | `#[cfg(test)] mod tests` of the card file |
| `packages/cards/test/classic/042-transmutable-toxins.test.ts` | 191 | port-inline | `crates/cards/src/scripts/classic/c042_transmutable_toxins.rs` | `#[cfg(test)] mod tests` of the card file |
| `packages/cards/test/classic/043-plague-nuke.test.ts` | 250 | port-inline | `crates/cards/src/scripts/classic/c043_plague_nuke.rs` | `#[cfg(test)] mod tests` of the card file |
| `packages/cards/test/classic/044-back-from-the-gy.test.ts` | 257 | port-inline | `crates/cards/src/scripts/classic/c044_back_from_the_gy.rs` | `#[cfg(test)] mod tests` of the card file |
| `packages/cards/test/classic/045-nature-titan.test.ts` | 226 | port-inline | `crates/cards/src/scripts/classic/c045_nature_titan.rs` | `#[cfg(test)] mod tests` of the card file |
| `packages/cards/test/classic/046-divine-favor.test.ts` | 156 | port-inline | `crates/cards/src/scripts/classic/c046_divine_favor.rs` | `#[cfg(test)] mod tests` of the card file |
| `packages/cards/test/classic/047-recurring-felinor.test.ts` | 221 | port-inline | `crates/cards/src/scripts/classic/c047_recurring_felinor.rs` | `#[cfg(test)] mod tests` of the card file |
| `packages/cards/test/classic/048-hired-shrimp.test.ts` | 283 | port-inline | `crates/cards/src/scripts/classic/c048_hired_shrimp.rs` | `#[cfg(test)] mod tests` of the card file |
| `packages/cards/test/classic/049-anti-greed-machine.test.ts` | 246 | port-inline | `crates/cards/src/scripts/classic/c049_anti_greed_machine.rs` | `#[cfg(test)] mod tests` of the card file |
| `packages/cards/test/classic/050-voidwalker.test.ts` | 287 | port-inline | `crates/cards/src/scripts/classic/c050_voidwalker.rs` | `#[cfg(test)] mod tests` of the card file |
| `packages/cards/test/classic/051-back-breaker.test.ts` | 162 | port-inline | `crates/cards/src/scripts/classic/c051_back_breaker.rs` | `#[cfg(test)] mod tests` of the card file |
| `packages/cards/test/classic/052-final-gambit.test.ts` | 439 | port-inline | `crates/cards/src/scripts/classic/c052_final_gambit.rs` | `#[cfg(test)] mod tests` of the card file |
| `packages/cards/test/classic/053-plague-crawler.test.ts` | 328 | port-inline | `crates/cards/src/scripts/classic/c053_plague_crawler.rs` | `#[cfg(test)] mod tests` of the card file |
| `packages/cards/test/classic/054-rewind.test.ts` | 257 | port-inline | `crates/cards/src/scripts/classic/c054_rewind.rs` | `#[cfg(test)] mod tests` of the card file |
| `packages/cards/test/classic/055-book-of-wildfire.test.ts` | 391 | port-inline | `crates/cards/src/scripts/classic/c055_book_of_wildfire.rs` | `#[cfg(test)] mod tests` of the card file |
| `packages/cards/test/classic/056-spell-tyrant.test.ts` | 245 | port-inline | `crates/cards/src/scripts/classic/c056_spell_tyrant.rs` | `#[cfg(test)] mod tests` of the card file |
| `packages/cards/test/classic/057-echo.test.ts` | 298 | port-inline | `crates/cards/src/scripts/classic/c057_echo.rs` | `#[cfg(test)] mod tests` of the card file |
| `packages/cards/test/classic/058-common-resources.test.ts` | 226 | port-inline | `crates/cards/src/scripts/classic/c058_common_resources.rs` | `#[cfg(test)] mod tests` of the card file |
| `packages/cards/test/classic/059-plague-doctor.test.ts` | 195 | port-inline | `crates/cards/src/scripts/classic/c059_plague_doctor.rs` | `#[cfg(test)] mod tests` of the card file |
| `packages/cards/test/classic/060-pile-on.test.ts` | 239 | port-inline | `crates/cards/src/scripts/classic/c060_pile_on.rs` | `#[cfg(test)] mod tests` of the card file |
| `packages/cards/test/classic/061-plague-bringer-goliath.test.ts` | 291 | port-inline | `crates/cards/src/scripts/classic/c061_plague_bringer_goliath.rs` | `#[cfg(test)] mod tests` of the card file |
| `packages/cards/test/classic/062-living-bomb.test.ts` | 281 | port-inline | `crates/cards/src/scripts/classic/c062_living_bomb.rs` | `#[cfg(test)] mod tests` of the card file |
| `packages/cards/test/classic/063-crop-dusting.test.ts` | 266 | port-inline | `crates/cards/src/scripts/classic/c063_crop_dusting.rs` | `#[cfg(test)] mod tests` of the card file |
| `packages/cards/test/classic/064-malzahars-recycler.test.ts` | 308 | port-inline | `crates/cards/src/scripts/classic/c064_malzahars_recycler.rs` | `#[cfg(test)] mod tests` of the card file |
| `packages/cards/test/classic/065-ace-in-the-hole.test.ts` | 260 | port-inline | `crates/cards/src/scripts/classic/c065_ace_in_the_hole.rs` | `#[cfg(test)] mod tests` of the card file |
| `packages/cards/test/classic/066-eu-striker.test.ts` | 217 | port-inline | `crates/cards/src/scripts/classic/c066_eu_striker.rs` | `#[cfg(test)] mod tests` of the card file |
| `packages/cards/test/classic/067-felinor-feeler.test.ts` | 102 | port-inline | `crates/cards/src/scripts/classic/c067_felinor_feeler.rs` | `#[cfg(test)] mod tests` of the card file |
| `packages/cards/test/classic/068-small-card-lobbyist.test.ts` | 240 | port-inline | `crates/cards/src/scripts/classic/c068_small_card_lobbyist.rs` | `#[cfg(test)] mod tests` of the card file |

## Part 14: cards lane 6: Classic #69–#90 and tokens, Classic+ #1–#18

| TypeScript | Lines | Action | Rust | Note |
|---|---|---|---|---|
| `packages/cards/src/scripts/classic-plus/001-doom-shroom.ts` | 27 | port | `crates/cards/src/scripts/classic_plus/c001_doom_shroom.rs` | cards lane 6 |
| `packages/cards/src/scripts/classic-plus/002-groom-shroom.ts` | 49 | port | `crates/cards/src/scripts/classic_plus/c002_groom_shroom.rs` | cards lane 6 |
| `packages/cards/src/scripts/classic-plus/003-second-amendment-snake.ts` | 30 | port | `crates/cards/src/scripts/classic_plus/c003_second_amendment_snake.rs` | cards lane 6 |
| `packages/cards/src/scripts/classic-plus/004-juhan-biggest-bat.ts` | 15 | port | `crates/cards/src/scripts/classic_plus/c004_juhan_biggest_bat.rs` | cards lane 6 |
| `packages/cards/src/scripts/classic-plus/005-guy-att.ts` | 15 | port | `crates/cards/src/scripts/classic_plus/c005_guy_att.rs` | cards lane 6 |
| `packages/cards/src/scripts/classic-plus/006-wrong-house-attacker.ts` | 13 | port | `crates/cards/src/scripts/classic_plus/c006_wrong_house_attacker.rs` | cards lane 6 |
| `packages/cards/src/scripts/classic-plus/007-the-house.ts` | 27 | port | `crates/cards/src/scripts/classic_plus/c007_the_house.rs` | cards lane 6 |
| `packages/cards/src/scripts/classic-plus/008-withering-storm.ts` | 37 | port | `crates/cards/src/scripts/classic_plus/c008_withering_storm.rs` | cards lane 6 |
| `packages/cards/src/scripts/classic-plus/009-silence.ts` | 18 | port | `crates/cards/src/scripts/classic_plus/c009_silence.rs` | cards lane 6 |
| `packages/cards/src/scripts/classic-plus/010-new-wraps.ts` | 21 | port | `crates/cards/src/scripts/classic_plus/c010_new_wraps.rs` | cards lane 6 |
| `packages/cards/src/scripts/classic-plus/011-anime-armor.ts` | 14 | port | `crates/cards/src/scripts/classic_plus/c011_anime_armor.rs` | cards lane 6 |
| `packages/cards/src/scripts/classic-plus/012-1-devour.ts` | 35 | port | `crates/cards/src/scripts/classic_plus/c012_1_devour.rs` | cards lane 6 |
| `packages/cards/src/scripts/classic-plus/012-2-death-boil.ts` | 34 | port | `crates/cards/src/scripts/classic_plus/c012_2_death_boil.rs` | cards lane 6 |
| `packages/cards/src/scripts/classic-plus/012-3-fluffy-grip.ts` | 19 | port | `crates/cards/src/scripts/classic_plus/c012_3_fluffy_grip.rs` | cards lane 6 |
| `packages/cards/src/scripts/classic-plus/012-4-powder-spray.ts` | 17 | port | `crates/cards/src/scripts/classic_plus/c012_4_powder_spray.rs` | cards lane 6 |
| `packages/cards/src/scripts/classic-plus/012-5-anti-waffle-shell.ts` | 28 | port | `crates/cards/src/scripts/classic_plus/c012_5_anti_waffle_shell.rs` | cards lane 6 |
| `packages/cards/src/scripts/classic-plus/012-6-frozen-wastes.ts` | 46 | port | `crates/cards/src/scripts/classic_plus/c012_6_frozen_wastes.rs` | cards lane 6 |
| `packages/cards/src/scripts/classic-plus/012-7-legion-of-the-hungry.ts` | 32 | port | `crates/cards/src/scripts/classic_plus/c012_7_legion_of_the_hungry.rs` | cards lane 6 |
| `packages/cards/src/scripts/classic-plus/012-8-frostspatula.ts` | 60 | port | `crates/cards/src/scripts/classic_plus/c012_8_frostspatula.rs` | cards lane 6 |
| `packages/cards/src/scripts/classic-plus/012-the-mother-pancake.ts` | 20 | port | `crates/cards/src/scripts/classic_plus/c012_the_mother_pancake.rs` | cards lane 6 |
| `packages/cards/src/scripts/classic-plus/013-mommy-barker.ts` | 18 | port | `crates/cards/src/scripts/classic_plus/c013_mommy_barker.rs` | cards lane 6 |
| `packages/cards/src/scripts/classic-plus/014-forever.ts` | 24 | port | `crates/cards/src/scripts/classic_plus/c014_forever.rs` | cards lane 6 |
| `packages/cards/src/scripts/classic-plus/015-conjure-rush-token.ts` | 25 | port | `crates/cards/src/scripts/classic_plus/c015_conjure_rush_token.rs` | cards lane 6 |
| `packages/cards/src/scripts/classic-plus/016-conjure-rush-token.ts` | 25 | port | `crates/cards/src/scripts/classic_plus/c016_conjure_rush_token.rs` | cards lane 6 |
| `packages/cards/src/scripts/classic-plus/017-conjure-rush-token.ts` | 25 | port | `crates/cards/src/scripts/classic_plus/c017_conjure_rush_token.rs` | cards lane 6 |
| `packages/cards/src/scripts/classic-plus/018-gullible-treatler.ts` | 37 | port | `crates/cards/src/scripts/classic_plus/c018_gullible_treatler.rs` | cards lane 6 |
| `packages/cards/src/scripts/classic/069-plague-charger.ts` | 52 | port | `crates/cards/src/scripts/classic/c069_plague_charger.rs` | cards lane 6 |
| `packages/cards/src/scripts/classic/070-book-of-plague.ts` | 21 | port | `crates/cards/src/scripts/classic/c070_book_of_plague.rs` | cards lane 6 |
| `packages/cards/src/scripts/classic/071-lane-eater.ts` | 54 | port | `crates/cards/src/scripts/classic/c071_lane_eater.rs` | cards lane 6 |
| `packages/cards/src/scripts/classic/072-grand-counterspell.ts` | 54 | port | `crates/cards/src/scripts/classic/c072_grand_counterspell.rs` | cards lane 6 |
| `packages/cards/src/scripts/classic/073-nurse-cleaver.ts` | 22 | port | `crates/cards/src/scripts/classic/c073_nurse_cleaver.rs` | cards lane 6 |
| `packages/cards/src/scripts/classic/074-corpse-plantation.ts` | 36 | port | `crates/cards/src/scripts/classic/c074_corpse_plantation.rs` | cards lane 6 |
| `packages/cards/src/scripts/classic/075-argusland.ts` | 20 | port | `crates/cards/src/scripts/classic/c075_argusland.rs` | cards lane 6 |
| `packages/cards/src/scripts/classic/076-plague-bringer.ts` | 24 | port | `crates/cards/src/scripts/classic/c076_plague_bringer.rs` | cards lane 6 |
| `packages/cards/src/scripts/classic/077-anti-magic-monkey.ts` | 27 | port | `crates/cards/src/scripts/classic/c077_anti_magic_monkey.rs` | cards lane 6 |
| `packages/cards/src/scripts/classic/078-mutate-spell.ts` | 50 | port | `crates/cards/src/scripts/classic/c078_mutate_spell.rs` | cards lane 6 |
| `packages/cards/src/scripts/classic/079-risky-die.ts` | 60 | port | `crates/cards/src/scripts/classic/c079_risky_die.rs` | cards lane 6 |
| `packages/cards/src/scripts/classic/080-boom-big-max.ts` | 30 | port | `crates/cards/src/scripts/classic/c080_boom_big_max.rs` | cards lane 6 |
| `packages/cards/src/scripts/classic/081-the-power-to-thrive.ts` | 36 | port | `crates/cards/src/scripts/classic/c081_the_power_to_thrive.rs` | cards lane 6 |
| `packages/cards/src/scripts/classic/082-sheeople.ts` | 25 | port | `crates/cards/src/scripts/classic/c082_sheeople.rs` | cards lane 6 |
| `packages/cards/src/scripts/classic/083-flame-lance.ts` | 33 | port | `crates/cards/src/scripts/classic/c083_flame_lance.rs` | cards lane 6 |
| `packages/cards/src/scripts/classic/084-lockdown.ts` | 55 | port | `crates/cards/src/scripts/classic/c084_lockdown.rs` | cards lane 6 |
| `packages/cards/src/scripts/classic/085-king-wagtoggle.ts` | 30 | port | `crates/cards/src/scripts/classic/c085_king_wagtoggle.rs` | cards lane 6 |
| `packages/cards/src/scripts/classic/086-genn.ts` | 19 | port | `crates/cards/src/scripts/classic/c086_genn.rs` | cards lane 6 |
| `packages/cards/src/scripts/classic/087-plague-chalice.ts` | 57 | port | `crates/cards/src/scripts/classic/c087_plague_chalice.rs` | cards lane 6 |
| `packages/cards/src/scripts/classic/088-siphon-squad.ts` | 50 | port | `crates/cards/src/scripts/classic/c088_siphon_squad.rs` | cards lane 6 |
| `packages/cards/src/scripts/classic/089-paul-allens-ghost.ts` | 34 | port | `crates/cards/src/scripts/classic/c089_paul_allens_ghost.rs` | cards lane 6 |
| `packages/cards/src/scripts/classic/090-in-too-deep.ts` | 255 | port | `crates/cards/src/scripts/classic/c090_in_too_deep.rs` | cards lane 6 |
| `packages/cards/src/scripts/classic/t-glitch-glitch.ts` | 22 | port | `crates/cards/src/scripts/classic/t_glitch_glitch.rs` | cards lane 6 |
| `packages/cards/test/classic-plus/001-doom-shroom.test.ts` | 160 | port-inline | `crates/cards/src/scripts/classic_plus/c001_doom_shroom.rs` | `#[cfg(test)] mod tests` of the card file |
| `packages/cards/test/classic-plus/002-groom-shroom.test.ts` | 169 | port-inline | `crates/cards/src/scripts/classic_plus/c002_groom_shroom.rs` | `#[cfg(test)] mod tests` of the card file |
| `packages/cards/test/classic-plus/003-second-amendment-snake.test.ts` | 221 | port-inline | `crates/cards/src/scripts/classic_plus/c003_second_amendment_snake.rs` | `#[cfg(test)] mod tests` of the card file |
| `packages/cards/test/classic-plus/004-juhan-biggest-bat.test.ts` | 140 | port-inline | `crates/cards/src/scripts/classic_plus/c004_juhan_biggest_bat.rs` | `#[cfg(test)] mod tests` of the card file |
| `packages/cards/test/classic-plus/005-guy-att.test.ts` | 190 | port-inline | `crates/cards/src/scripts/classic_plus/c005_guy_att.rs` | `#[cfg(test)] mod tests` of the card file |
| `packages/cards/test/classic-plus/006-wrong-house-attacker.test.ts` | 91 | port-inline | `crates/cards/src/scripts/classic_plus/c006_wrong_house_attacker.rs` | `#[cfg(test)] mod tests` of the card file |
| `packages/cards/test/classic-plus/007-the-house.test.ts` | 160 | port-inline | `crates/cards/src/scripts/classic_plus/c007_the_house.rs` | `#[cfg(test)] mod tests` of the card file |
| `packages/cards/test/classic-plus/008-withering-storm.test.ts` | 206 | port-inline | `crates/cards/src/scripts/classic_plus/c008_withering_storm.rs` | `#[cfg(test)] mod tests` of the card file |
| `packages/cards/test/classic-plus/009-silence.test.ts` | 184 | port-inline | `crates/cards/src/scripts/classic_plus/c009_silence.rs` | `#[cfg(test)] mod tests` of the card file |
| `packages/cards/test/classic-plus/010-new-wraps.test.ts` | 167 | port-inline | `crates/cards/src/scripts/classic_plus/c010_new_wraps.rs` | `#[cfg(test)] mod tests` of the card file |
| `packages/cards/test/classic-plus/011-anime-armor.test.ts` | 211 | port-inline | `crates/cards/src/scripts/classic_plus/c011_anime_armor.rs` | `#[cfg(test)] mod tests` of the card file |
| `packages/cards/test/classic-plus/012-1-devour.test.ts` | 104 | port-inline | `crates/cards/src/scripts/classic_plus/c012_1_devour.rs` | `#[cfg(test)] mod tests` of the card file |
| `packages/cards/test/classic-plus/012-2-death-boil.test.ts` | 75 | port-inline | `crates/cards/src/scripts/classic_plus/c012_2_death_boil.rs` | `#[cfg(test)] mod tests` of the card file |
| `packages/cards/test/classic-plus/012-3-fluffy-grip.test.ts` | 110 | port-inline | `crates/cards/src/scripts/classic_plus/c012_3_fluffy_grip.rs` | `#[cfg(test)] mod tests` of the card file |
| `packages/cards/test/classic-plus/012-4-powder-spray.test.ts` | 71 | port-inline | `crates/cards/src/scripts/classic_plus/c012_4_powder_spray.rs` | `#[cfg(test)] mod tests` of the card file |
| `packages/cards/test/classic-plus/012-5-anti-waffle-shell.test.ts` | 71 | port-inline | `crates/cards/src/scripts/classic_plus/c012_5_anti_waffle_shell.rs` | `#[cfg(test)] mod tests` of the card file |
| `packages/cards/test/classic-plus/012-6-frozen-wastes.test.ts` | 126 | port-inline | `crates/cards/src/scripts/classic_plus/c012_6_frozen_wastes.rs` | `#[cfg(test)] mod tests` of the card file |
| `packages/cards/test/classic-plus/012-7-legion-of-the-hungry.test.ts` | 105 | port-inline | `crates/cards/src/scripts/classic_plus/c012_7_legion_of_the_hungry.rs` | `#[cfg(test)] mod tests` of the card file |
| `packages/cards/test/classic-plus/012-8-frostspatula.test.ts` | 228 | port-inline | `crates/cards/src/scripts/classic_plus/c012_8_frostspatula.rs` | `#[cfg(test)] mod tests` of the card file |
| `packages/cards/test/classic-plus/012-the-mother-pancake.test.ts` | 96 | port-inline | `crates/cards/src/scripts/classic_plus/c012_the_mother_pancake.rs` | `#[cfg(test)] mod tests` of the card file |
| `packages/cards/test/classic-plus/013-mommy-barker.test.ts` | 89 | port-inline | `crates/cards/src/scripts/classic_plus/c013_mommy_barker.rs` | `#[cfg(test)] mod tests` of the card file |
| `packages/cards/test/classic-plus/014-forever.test.ts` | 176 | port-inline | `crates/cards/src/scripts/classic_plus/c014_forever.rs` | `#[cfg(test)] mod tests` of the card file |
| `packages/cards/test/classic-plus/015-conjure-rush-token.test.ts` | 169 | port-inline | `crates/cards/src/scripts/classic_plus/c015_conjure_rush_token.rs` | `#[cfg(test)] mod tests` of the card file |
| `packages/cards/test/classic-plus/016-conjure-rush-token.test.ts` | 102 | port-inline | `crates/cards/src/scripts/classic_plus/c016_conjure_rush_token.rs` | `#[cfg(test)] mod tests` of the card file |
| `packages/cards/test/classic-plus/017-conjure-rush-token.test.ts` | 110 | port-inline | `crates/cards/src/scripts/classic_plus/c017_conjure_rush_token.rs` | `#[cfg(test)] mod tests` of the card file |
| `packages/cards/test/classic-plus/018-gullible-treatler.test.ts` | 200 | port-inline | `crates/cards/src/scripts/classic_plus/c018_gullible_treatler.rs` | `#[cfg(test)] mod tests` of the card file |
| `packages/cards/test/classic/069-plague-charger.test.ts` | 173 | port-inline | `crates/cards/src/scripts/classic/c069_plague_charger.rs` | `#[cfg(test)] mod tests` of the card file |
| `packages/cards/test/classic/070-book-of-plague.test.ts` | 237 | port-inline | `crates/cards/src/scripts/classic/c070_book_of_plague.rs` | `#[cfg(test)] mod tests` of the card file |
| `packages/cards/test/classic/071-lane-eater.test.ts` | 175 | port-inline | `crates/cards/src/scripts/classic/c071_lane_eater.rs` | `#[cfg(test)] mod tests` of the card file |
| `packages/cards/test/classic/072-grand-counterspell.test.ts` | 222 | port-inline | `crates/cards/src/scripts/classic/c072_grand_counterspell.rs` | `#[cfg(test)] mod tests` of the card file |
| `packages/cards/test/classic/073-nurse-cleaver.test.ts` | 63 | port-inline | `crates/cards/src/scripts/classic/c073_nurse_cleaver.rs` | `#[cfg(test)] mod tests` of the card file |
| `packages/cards/test/classic/074-corpse-plantation.test.ts` | 269 | port-inline | `crates/cards/src/scripts/classic/c074_corpse_plantation.rs` | `#[cfg(test)] mod tests` of the card file |
| `packages/cards/test/classic/075-argusland.test.ts` | 271 | port-inline | `crates/cards/src/scripts/classic/c075_argusland.rs` | `#[cfg(test)] mod tests` of the card file |
| `packages/cards/test/classic/076-plague-bringer.test.ts` | 217 | port-inline | `crates/cards/src/scripts/classic/c076_plague_bringer.rs` | `#[cfg(test)] mod tests` of the card file |
| `packages/cards/test/classic/077-anti-magic-monkey.test.ts` | 179 | port-inline | `crates/cards/src/scripts/classic/c077_anti_magic_monkey.rs` | `#[cfg(test)] mod tests` of the card file |
| `packages/cards/test/classic/078-mutate-spell.test.ts` | 430 | port-inline | `crates/cards/src/scripts/classic/c078_mutate_spell.rs` | `#[cfg(test)] mod tests` of the card file |
| `packages/cards/test/classic/079-risky-die.test.ts` | 179 | port-inline | `crates/cards/src/scripts/classic/c079_risky_die.rs` | `#[cfg(test)] mod tests` of the card file |
| `packages/cards/test/classic/080-boom-big-max.test.ts` | 107 | port-inline | `crates/cards/src/scripts/classic/c080_boom_big_max.rs` | `#[cfg(test)] mod tests` of the card file |
| `packages/cards/test/classic/081-the-power-to-thrive.test.ts` | 175 | port-inline | `crates/cards/src/scripts/classic/c081_the_power_to_thrive.rs` | `#[cfg(test)] mod tests` of the card file |
| `packages/cards/test/classic/082-sheeople.test.ts` | 221 | port-inline | `crates/cards/src/scripts/classic/c082_sheeople.rs` | `#[cfg(test)] mod tests` of the card file |
| `packages/cards/test/classic/083-flame-lance.test.ts` | 121 | port-inline | `crates/cards/src/scripts/classic/c083_flame_lance.rs` | `#[cfg(test)] mod tests` of the card file |
| `packages/cards/test/classic/084-lockdown.test.ts` | 184 | port-inline | `crates/cards/src/scripts/classic/c084_lockdown.rs` | `#[cfg(test)] mod tests` of the card file |
| `packages/cards/test/classic/085-king-wagtoggle.test.ts` | 106 | port-inline | `crates/cards/src/scripts/classic/c085_king_wagtoggle.rs` | `#[cfg(test)] mod tests` of the card file |
| `packages/cards/test/classic/086-genn.test.ts` | 43 | port-inline | `crates/cards/src/scripts/classic/c086_genn.rs` | `#[cfg(test)] mod tests` of the card file |
| `packages/cards/test/classic/087-plague-chalice.test.ts` | 350 | port-inline | `crates/cards/src/scripts/classic/c087_plague_chalice.rs` | `#[cfg(test)] mod tests` of the card file |
| `packages/cards/test/classic/088-siphon-squad.test.ts` | 210 | port-inline | `crates/cards/src/scripts/classic/c088_siphon_squad.rs` | `#[cfg(test)] mod tests` of the card file |
| `packages/cards/test/classic/089-paul-allens-ghost.test.ts` | 269 | port-inline | `crates/cards/src/scripts/classic/c089_paul_allens_ghost.rs` | `#[cfg(test)] mod tests` of the card file |
| `packages/cards/test/classic/090-in-too-deep.test.ts` | 782 | port-inline | `crates/cards/src/scripts/classic/c090_in_too_deep.rs` | `#[cfg(test)] mod tests` of the card file |
| `packages/cards/test/classic/t-glitch-glitch.test.ts` | 104 | port-inline | `crates/cards/src/scripts/classic/t_glitch_glitch.rs` | `#[cfg(test)] mod tests` of the card file |

## Part 15: cards lane 7: Classic+ #19–#49

| TypeScript | Lines | Action | Rust | Note |
|---|---|---|---|---|
| `packages/cards/src/kyTestBank.ts` | 108 | port | `crates/cards/src/ky_test_bank.rs` | with C+ #42 KY's Test |
| `packages/cards/src/scripts/classic-plus/019-1-top-loser.ts` | 14 | port | `crates/cards/src/scripts/classic_plus/c019_1_top_loser.rs` | cards lane 7 |
| `packages/cards/src/scripts/classic-plus/019-2-jungle-loser.ts` | 56 | port | `crates/cards/src/scripts/classic_plus/c019_2_jungle_loser.rs` | cards lane 7 |
| `packages/cards/src/scripts/classic-plus/019-3-mid-loser.ts` | 33 | port | `crates/cards/src/scripts/classic_plus/c019_3_mid_loser.rs` | cards lane 7 |
| `packages/cards/src/scripts/classic-plus/019-4-support-loser.ts` | 24 | port | `crates/cards/src/scripts/classic_plus/c019_4_support_loser.rs` | cards lane 7 |
| `packages/cards/src/scripts/classic-plus/019-5-bot-loser.ts` | 38 | port | `crates/cards/src/scripts/classic_plus/c019_5_bot_loser.rs` | cards lane 7 |
| `packages/cards/src/scripts/classic-plus/019-league-of-losers.ts` | 49 | port | `crates/cards/src/scripts/classic_plus/c019_league_of_losers.rs` | cards lane 7 |
| `packages/cards/src/scripts/classic-plus/020-mushroom-power.ts` | 22 | port | `crates/cards/src/scripts/classic_plus/c020_mushroom_power.rs` | cards lane 7 |
| `packages/cards/src/scripts/classic-plus/021-whirlwind.ts` | 17 | port | `crates/cards/src/scripts/classic_plus/c021_whirlwind.rs` | cards lane 7 |
| `packages/cards/src/scripts/classic-plus/022-blood-moon.ts` | 16 | port | `crates/cards/src/scripts/classic_plus/c022_blood_moon.rs` | cards lane 7 |
| `packages/cards/src/scripts/classic-plus/023-dropshipping.ts` | 38 | port | `crates/cards/src/scripts/classic_plus/c023_dropshipping.rs` | cards lane 7 |
| `packages/cards/src/scripts/classic-plus/024-crushing-walls.ts` | 28 | port | `crates/cards/src/scripts/classic_plus/c024_crushing_walls.rs` | cards lane 7 |
| `packages/cards/src/scripts/classic-plus/025-soul-shot.ts` | 52 | port | `crates/cards/src/scripts/classic_plus/c025_soul_shot.rs` | cards lane 7 |
| `packages/cards/src/scripts/classic-plus/026-tommy-tempo.ts` | 22 | port | `crates/cards/src/scripts/classic_plus/c026_tommy_tempo.rs` | cards lane 7 |
| `packages/cards/src/scripts/classic-plus/027-zephrys-zealotism.ts` | 38 | port | `crates/cards/src/scripts/classic_plus/c027_zephrys_zealotism.rs` | cards lane 7 |
| `packages/cards/src/scripts/classic-plus/028-nuestro-hogar-nuestras-tumbas.ts` | 24 | port | `crates/cards/src/scripts/classic_plus/c028_nuestro_hogar_nuestras_tumbas.rs` | cards lane 7 |
| `packages/cards/src/scripts/classic-plus/029-portal-to-the-past.ts` | 23 | port | `crates/cards/src/scripts/classic_plus/c029_portal_to_the_past.rs` | cards lane 7 |
| `packages/cards/src/scripts/classic-plus/030-felinor-fuser.ts` | 71 | port | `crates/cards/src/scripts/classic_plus/c030_felinor_fuser.rs` | cards lane 7 |
| `packages/cards/src/scripts/classic-plus/031-fusion-lab.ts` | 54 | port | `crates/cards/src/scripts/classic_plus/c031_fusion_lab.rs` | cards lane 7 |
| `packages/cards/src/scripts/classic-plus/032-1-execute.ts` | 38 | port | `crates/cards/src/scripts/classic_plus/c032_1_execute.rs` | cards lane 7 |
| `packages/cards/src/scripts/classic-plus/032-2-brawl.ts` | 56 | port | `crates/cards/src/scripts/classic_plus/c032_2_brawl.rs` | cards lane 7 |
| `packages/cards/src/scripts/classic-plus/032-3-blade-storm.ts` | 34 | port | `crates/cards/src/scripts/classic_plus/c032_3_blade_storm.rs` | cards lane 7 |
| `packages/cards/src/scripts/classic-plus/032-otherworldly-removal.ts` | 27 | port | `crates/cards/src/scripts/classic_plus/c032_otherworldly_removal.rs` | cards lane 7 |
| `packages/cards/src/scripts/classic-plus/033-ivory-tower.ts` | 66 | port | `crates/cards/src/scripts/classic_plus/c033_ivory_tower.rs` | cards lane 7 |
| `packages/cards/src/scripts/classic-plus/034-memory-leak.ts` | 52 | port | `crates/cards/src/scripts/classic_plus/c034_memory_leak.rs` | cards lane 7 |
| `packages/cards/src/scripts/classic-plus/035-rollback.ts` | 29 | port | `crates/cards/src/scripts/classic_plus/c035_rollback.rs` | cards lane 7 |
| `packages/cards/src/scripts/classic-plus/036-1-bone-storm.ts` | 19 | port | `crates/cards/src/scripts/classic_plus/c036_1_bone_storm.rs` | cards lane 7 |
| `packages/cards/src/scripts/classic-plus/036-conjure-bones.ts` | 30 | port | `crates/cards/src/scripts/classic_plus/c036_conjure_bones.rs` | cards lane 7 |
| `packages/cards/src/scripts/classic-plus/037-wardrum.ts` | 98 | port | `crates/cards/src/scripts/classic_plus/c037_wardrum.rs` | cards lane 7 |
| `packages/cards/src/scripts/classic-plus/038-1-solarius-prime.ts` | 26 | port | `crates/cards/src/scripts/classic_plus/c038_1_solarius_prime.rs` | cards lane 7 |
| `packages/cards/src/scripts/classic-plus/038-solarius.ts` | 23 | port | `crates/cards/src/scripts/classic_plus/c038_solarius.rs` | cards lane 7 |
| `packages/cards/src/scripts/classic-plus/039-book-worm.ts` | 41 | port | `crates/cards/src/scripts/classic_plus/c039_book_worm.rs` | cards lane 7 |
| `packages/cards/src/scripts/classic-plus/040-appropriations.ts` | 65 | port | `crates/cards/src/scripts/classic_plus/c040_appropriations.rs` | cards lane 7 |
| `packages/cards/src/scripts/classic-plus/041-kys-constant.ts` | 46 | port | `crates/cards/src/scripts/classic_plus/c041_kys_constant.rs` | cards lane 7 |
| `packages/cards/src/scripts/classic-plus/042-1-kys-gift.ts` | 32 | port | `crates/cards/src/scripts/classic_plus/c042_1_kys_gift.rs` | cards lane 7 |
| `packages/cards/src/scripts/classic-plus/042-kys-test.ts` | 18 | port | `crates/cards/src/scripts/classic_plus/c042_kys_test.rs` | cards lane 7 |
| `packages/cards/src/scripts/classic-plus/043-ai-slop.ts` | 29 | port | `crates/cards/src/scripts/classic_plus/c043_ai_slop.rs` | cards lane 7 |
| `packages/cards/src/scripts/classic-plus/044-simplicity-audit.ts` | 50 | port | `crates/cards/src/scripts/classic_plus/c044_simplicity_audit.rs` | cards lane 7 |
| `packages/cards/src/scripts/classic-plus/045-complexity-audit.ts` | 50 | port | `crates/cards/src/scripts/classic_plus/c045_complexity_audit.rs` | cards lane 7 |
| `packages/cards/src/scripts/classic-plus/046-1-felinor-flagbearer-prime.ts` | 40 | port | `crates/cards/src/scripts/classic_plus/c046_1_felinor_flagbearer_prime.rs` | cards lane 7 |
| `packages/cards/src/scripts/classic-plus/046-felinor-flagbearer.ts` | 49 | port | `crates/cards/src/scripts/classic_plus/c046_felinor_flagbearer.rs` | cards lane 7 |
| `packages/cards/src/scripts/classic-plus/047-joggs-box.ts` | 27 | port | `crates/cards/src/scripts/classic_plus/c047_joggs_box.rs` | cards lane 7 |
| `packages/cards/src/scripts/classic-plus/048-jlockheeds-lobbyist.ts` | 27 | port | `crates/cards/src/scripts/classic_plus/c048_jlockheeds_lobbyist.rs` | cards lane 7 |
| `packages/cards/src/scripts/classic-plus/049-jay-fungus.ts` | 22 | port | `crates/cards/src/scripts/classic_plus/c049_jay_fungus.rs` | cards lane 7 |
| `packages/cards/test/classic-plus/019-1-top-loser.test.ts` | 219 | port-inline | `crates/cards/src/scripts/classic_plus/c019_1_top_loser.rs` | `#[cfg(test)] mod tests` of the card file |
| `packages/cards/test/classic-plus/019-2-jungle-loser.test.ts` | 221 | port-inline | `crates/cards/src/scripts/classic_plus/c019_2_jungle_loser.rs` | `#[cfg(test)] mod tests` of the card file |
| `packages/cards/test/classic-plus/019-3-mid-loser.test.ts` | 191 | port-inline | `crates/cards/src/scripts/classic_plus/c019_3_mid_loser.rs` | `#[cfg(test)] mod tests` of the card file |
| `packages/cards/test/classic-plus/019-4-support-loser.test.ts` | 146 | port-inline | `crates/cards/src/scripts/classic_plus/c019_4_support_loser.rs` | `#[cfg(test)] mod tests` of the card file |
| `packages/cards/test/classic-plus/019-5-bot-loser.test.ts` | 206 | port-inline | `crates/cards/src/scripts/classic_plus/c019_5_bot_loser.rs` | `#[cfg(test)] mod tests` of the card file |
| `packages/cards/test/classic-plus/019-league-of-losers.test.ts` | 178 | port-inline | `crates/cards/src/scripts/classic_plus/c019_league_of_losers.rs` | `#[cfg(test)] mod tests` of the card file |
| `packages/cards/test/classic-plus/020-mushroom-power.test.ts` | 122 | port-inline | `crates/cards/src/scripts/classic_plus/c020_mushroom_power.rs` | `#[cfg(test)] mod tests` of the card file |
| `packages/cards/test/classic-plus/021-whirlwind.test.ts` | 176 | port-inline | `crates/cards/src/scripts/classic_plus/c021_whirlwind.rs` | `#[cfg(test)] mod tests` of the card file |
| `packages/cards/test/classic-plus/022-blood-moon.test.ts` | 251 | port-inline | `crates/cards/src/scripts/classic_plus/c022_blood_moon.rs` | `#[cfg(test)] mod tests` of the card file |
| `packages/cards/test/classic-plus/023-dropshipping.test.ts` | 221 | port-inline | `crates/cards/src/scripts/classic_plus/c023_dropshipping.rs` | `#[cfg(test)] mod tests` of the card file |
| `packages/cards/test/classic-plus/024-crushing-walls.test.ts` | 163 | port-inline | `crates/cards/src/scripts/classic_plus/c024_crushing_walls.rs` | `#[cfg(test)] mod tests` of the card file |
| `packages/cards/test/classic-plus/025-soul-shot.test.ts` | 167 | port-inline | `crates/cards/src/scripts/classic_plus/c025_soul_shot.rs` | `#[cfg(test)] mod tests` of the card file |
| `packages/cards/test/classic-plus/026-tommy-tempo.test.ts` | 248 | port-inline | `crates/cards/src/scripts/classic_plus/c026_tommy_tempo.rs` | `#[cfg(test)] mod tests` of the card file |
| `packages/cards/test/classic-plus/027-zephrys-zealotism.test.ts` | 194 | port-inline | `crates/cards/src/scripts/classic_plus/c027_zephrys_zealotism.rs` | `#[cfg(test)] mod tests` of the card file |
| `packages/cards/test/classic-plus/028-nuestro-hogar-nuestras-tumbas.test.ts` | 138 | port-inline | `crates/cards/src/scripts/classic_plus/c028_nuestro_hogar_nuestras_tumbas.rs` | `#[cfg(test)] mod tests` of the card file |
| `packages/cards/test/classic-plus/029-portal-to-the-past.test.ts` | 282 | port-inline | `crates/cards/src/scripts/classic_plus/c029_portal_to_the_past.rs` | `#[cfg(test)] mod tests` of the card file |
| `packages/cards/test/classic-plus/030-felinor-fuser.test.ts` | 186 | port-inline | `crates/cards/src/scripts/classic_plus/c030_felinor_fuser.rs` | `#[cfg(test)] mod tests` of the card file |
| `packages/cards/test/classic-plus/031-fusion-lab.test.ts` | 206 | port-inline | `crates/cards/src/scripts/classic_plus/c031_fusion_lab.rs` | `#[cfg(test)] mod tests` of the card file |
| `packages/cards/test/classic-plus/032-1-execute.test.ts` | 101 | port-inline | `crates/cards/src/scripts/classic_plus/c032_1_execute.rs` | `#[cfg(test)] mod tests` of the card file |
| `packages/cards/test/classic-plus/032-2-brawl.test.ts` | 153 | port-inline | `crates/cards/src/scripts/classic_plus/c032_2_brawl.rs` | `#[cfg(test)] mod tests` of the card file |
| `packages/cards/test/classic-plus/032-3-blade-storm.test.ts` | 179 | port-inline | `crates/cards/src/scripts/classic_plus/c032_3_blade_storm.rs` | `#[cfg(test)] mod tests` of the card file |
| `packages/cards/test/classic-plus/032-otherworldly-removal.test.ts` | 77 | port-inline | `crates/cards/src/scripts/classic_plus/c032_otherworldly_removal.rs` | `#[cfg(test)] mod tests` of the card file |
| `packages/cards/test/classic-plus/033-ivory-tower.test.ts` | 186 | port-inline | `crates/cards/src/scripts/classic_plus/c033_ivory_tower.rs` | `#[cfg(test)] mod tests` of the card file |
| `packages/cards/test/classic-plus/034-memory-leak.test.ts` | 209 | port-inline | `crates/cards/src/scripts/classic_plus/c034_memory_leak.rs` | `#[cfg(test)] mod tests` of the card file |
| `packages/cards/test/classic-plus/035-rollback.test.ts` | 715 | port-inline | `crates/cards/src/scripts/classic_plus/c035_rollback.rs` | `#[cfg(test)] mod tests` of the card file |
| `packages/cards/test/classic-plus/036-1-bone-storm.test.ts` | 133 | port-inline | `crates/cards/src/scripts/classic_plus/c036_1_bone_storm.rs` | `#[cfg(test)] mod tests` of the card file |
| `packages/cards/test/classic-plus/036-conjure-bones.test.ts` | 125 | port-inline | `crates/cards/src/scripts/classic_plus/c036_conjure_bones.rs` | `#[cfg(test)] mod tests` of the card file |
| `packages/cards/test/classic-plus/037-wardrum.test.ts` | 388 | port-inline | `crates/cards/src/scripts/classic_plus/c037_wardrum.rs` | `#[cfg(test)] mod tests` of the card file |
| `packages/cards/test/classic-plus/038-1-solarius-prime.test.ts` | 254 | port-inline | `crates/cards/src/scripts/classic_plus/c038_1_solarius_prime.rs` | `#[cfg(test)] mod tests` of the card file |
| `packages/cards/test/classic-plus/038-solarius.test.ts` | 220 | port-inline | `crates/cards/src/scripts/classic_plus/c038_solarius.rs` | `#[cfg(test)] mod tests` of the card file |
| `packages/cards/test/classic-plus/039-book-worm.test.ts` | 189 | port-inline | `crates/cards/src/scripts/classic_plus/c039_book_worm.rs` | `#[cfg(test)] mod tests` of the card file |
| `packages/cards/test/classic-plus/040-appropriations.test.ts` | 284 | port-inline | `crates/cards/src/scripts/classic_plus/c040_appropriations.rs` | `#[cfg(test)] mod tests` of the card file |
| `packages/cards/test/classic-plus/041-kys-constant.test.ts` | 200 | port-inline | `crates/cards/src/scripts/classic_plus/c041_kys_constant.rs` | `#[cfg(test)] mod tests` of the card file |
| `packages/cards/test/classic-plus/042-1-kys-gift.test.ts` | 192 | port-inline | `crates/cards/src/scripts/classic_plus/c042_1_kys_gift.rs` | `#[cfg(test)] mod tests` of the card file |
| `packages/cards/test/classic-plus/042-kys-test.test.ts` | 363 | port-inline | `crates/cards/src/scripts/classic_plus/c042_kys_test.rs` | `#[cfg(test)] mod tests` of the card file |
| `packages/cards/test/classic-plus/043-ai-slop.test.ts` | 152 | port-inline | `crates/cards/src/scripts/classic_plus/c043_ai_slop.rs` | `#[cfg(test)] mod tests` of the card file |
| `packages/cards/test/classic-plus/044-simplicity-audit.test.ts` | 180 | port-inline | `crates/cards/src/scripts/classic_plus/c044_simplicity_audit.rs` | `#[cfg(test)] mod tests` of the card file |
| `packages/cards/test/classic-plus/045-complexity-audit.test.ts` | 144 | port-inline | `crates/cards/src/scripts/classic_plus/c045_complexity_audit.rs` | `#[cfg(test)] mod tests` of the card file |
| `packages/cards/test/classic-plus/046-1-felinor-flagbearer-prime.test.ts` | 145 | port-inline | `crates/cards/src/scripts/classic_plus/c046_1_felinor_flagbearer_prime.rs` | `#[cfg(test)] mod tests` of the card file |
| `packages/cards/test/classic-plus/046-felinor-flagbearer.test.ts` | 177 | port-inline | `crates/cards/src/scripts/classic_plus/c046_felinor_flagbearer.rs` | `#[cfg(test)] mod tests` of the card file |
| `packages/cards/test/classic-plus/047-joggs-box.test.ts` | 366 | port-inline | `crates/cards/src/scripts/classic_plus/c047_joggs_box.rs` | `#[cfg(test)] mod tests` of the card file |
| `packages/cards/test/classic-plus/048-jlockheeds-lobbyist.test.ts` | 147 | port-inline | `crates/cards/src/scripts/classic_plus/c048_jlockheeds_lobbyist.rs` | `#[cfg(test)] mod tests` of the card file |
| `packages/cards/test/classic-plus/049-jay-fungus.test.ts` | 147 | port-inline | `crates/cards/src/scripts/classic_plus/c049_jay_fungus.rs` | `#[cfg(test)] mod tests` of the card file |

## Part 16: cards lane 8: Classic+ #50–#78 and the AI tokens

| TypeScript | Lines | Action | Rust | Note |
|---|---|---|---|---|
| `packages/cards/src/scripts/classic-plus/050-adaptive-growth.ts` | 46 | port | `crates/cards/src/scripts/classic_plus/c050_adaptive_growth.rs` | cards lane 8 |
| `packages/cards/src/scripts/classic-plus/051-jlockheeds-j15-fighter.ts` | 20 | port | `crates/cards/src/scripts/classic_plus/c051_jlockheeds_j15_fighter.rs` | cards lane 8 |
| `packages/cards/src/scripts/classic-plus/052-jlockheeds-permanent-defense-contract.ts` | 50 | port | `crates/cards/src/scripts/classic_plus/c052_jlockheeds_permanent_defense_contract.rs` | cards lane 8 |
| `packages/cards/src/scripts/classic-plus/053-book-of-tokens.ts` | 44 | port | `crates/cards/src/scripts/classic_plus/c053_book_of_tokens.rs` | cards lane 8 |
| `packages/cards/src/scripts/classic-plus/054-book-of-books.ts` | 25 | port | `crates/cards/src/scripts/classic_plus/c054_book_of_books.rs` | cards lane 8 |
| `packages/cards/src/scripts/classic-plus/055-book-of-greed.ts` | 22 | port | `crates/cards/src/scripts/classic_plus/c055_book_of_greed.rs` | cards lane 8 |
| `packages/cards/src/scripts/classic-plus/056-book-of-pain.ts` | 21 | port | `crates/cards/src/scripts/classic_plus/c056_book_of_pain.rs` | cards lane 8 |
| `packages/cards/src/scripts/classic-plus/057-book-of-stats.ts` | 26 | port | `crates/cards/src/scripts/classic_plus/c057_book_of_stats.rs` | cards lane 8 |
| `packages/cards/src/scripts/classic-plus/058-fruit-basket.ts` | 22 | port | `crates/cards/src/scripts/classic_plus/c058_fruit_basket.rs` | cards lane 8 |
| `packages/cards/src/scripts/classic-plus/059-all-purpose-apple.ts` | 35 | port | `crates/cards/src/scripts/classic_plus/c059_all_purpose_apple.rs` | cards lane 8 |
| `packages/cards/src/scripts/classic-plus/060-doctors-orders.ts` | 28 | port | `crates/cards/src/scripts/classic_plus/c060_doctors_orders.rs` | cards lane 8 |
| `packages/cards/src/scripts/classic-plus/061-bauble-bubble.ts` | 29 | port | `crates/cards/src/scripts/classic_plus/c061_bauble_bubble.rs` | cards lane 8 |
| `packages/cards/src/scripts/classic-plus/062-kys-papaya.ts` | 23 | port | `crates/cards/src/scripts/classic_plus/c062_kys_papaya.rs` | cards lane 8 |
| `packages/cards/src/scripts/classic-plus/063-fruit-tree.ts` | 39 | port | `crates/cards/src/scripts/classic_plus/c063_fruit_tree.rs` | cards lane 8 |
| `packages/cards/src/scripts/classic-plus/064-mulch-muncher.ts` | 29 | port | `crates/cards/src/scripts/classic_plus/c064_mulch_muncher.rs` | cards lane 8 |
| `packages/cards/src/scripts/classic-plus/065-1-rotten-grape.ts` | 25 | port | `crates/cards/src/scripts/classic_plus/c065_1_rotten_grape.rs` | cards lane 8 |
| `packages/cards/src/scripts/classic-plus/065-2-normal-grape.ts` | 42 | port | `crates/cards/src/scripts/classic_plus/c065_2_normal_grape.rs` | cards lane 8 |
| `packages/cards/src/scripts/classic-plus/065-3-large-grape.ts` | 33 | port | `crates/cards/src/scripts/classic_plus/c065_3_large_grape.rs` | cards lane 8 |
| `packages/cards/src/scripts/classic-plus/065-4-golden-grape.ts` | 56 | port | `crates/cards/src/scripts/classic_plus/c065_4_golden_grape.rs` | cards lane 8 |
| `packages/cards/src/scripts/classic-plus/065-5-mythic-grape.ts` | 35 | port | `crates/cards/src/scripts/classic_plus/c065_5_mythic_grape.rs` | cards lane 8 |
| `packages/cards/src/scripts/classic-plus/065-two-grapes.ts` | 30 | port | `crates/cards/src/scripts/classic_plus/c065_two_grapes.rs` | cards lane 8 |
| `packages/cards/src/scripts/classic-plus/066-vine-of-grapes.ts` | 26 | port | `crates/cards/src/scripts/classic_plus/c066_vine_of_grapes.rs` | cards lane 8 |
| `packages/cards/src/scripts/classic-plus/067-pear.ts` | 36 | port | `crates/cards/src/scripts/classic_plus/c067_pear.rs` | cards lane 8 |
| `packages/cards/src/scripts/classic-plus/068-organic-produce.ts` | 29 | port | `crates/cards/src/scripts/classic_plus/c068_organic_produce.rs` | cards lane 8 |
| `packages/cards/src/scripts/classic-plus/069-buff-billy.ts` | 25 | port | `crates/cards/src/scripts/classic_plus/c069_buff_billy.rs` | cards lane 8 |
| `packages/cards/src/scripts/classic-plus/070-chaos-machine.ts` | 28 | port | `crates/cards/src/scripts/classic_plus/c070_chaos_machine.rs` | cards lane 8 |
| `packages/cards/src/scripts/classic-plus/071-book-of-buff.ts` | 20 | port | `crates/cards/src/scripts/classic_plus/c071_book_of_buff.rs` | cards lane 8 |
| `packages/cards/src/scripts/classic-plus/072-book-of-nerf.ts` | 25 | port | `crates/cards/src/scripts/classic_plus/c072_book_of_nerf.rs` | cards lane 8 |
| `packages/cards/src/scripts/classic-plus/073-1-classic-golem.ts` | 59 | port | `crates/cards/src/scripts/classic_plus/c073_1_classic_golem.rs` | cards lane 8 |
| `packages/cards/src/scripts/classic-plus/073-call-to-chaos-classic-edition.ts` | 26 | port | `crates/cards/src/scripts/classic_plus/c073_call_to_chaos_classic_edition.rs` | cards lane 8 |
| `packages/cards/src/scripts/classic-plus/074-twice-forward-one-step-backwards.ts` | 42 | port | `crates/cards/src/scripts/classic_plus/c074_twice_forward_one_step_backwards.rs` | cards lane 8 |
| `packages/cards/src/scripts/classic-plus/075-1-j-lease-j-jungle-ex-plorer-pack.ts` | 40 | port | `crates/cards/src/scripts/classic_plus/c075_1_j_lease_j_jungle_ex_plorer_pack.rs` | cards lane 8 |
| `packages/cards/src/scripts/classic-plus/075-j-lease-j-jungle-ex-plorer.ts` | 30 | port | `crates/cards/src/scripts/classic_plus/c075_j_lease_j_jungle_ex_plorer.rs` | cards lane 8 |
| `packages/cards/src/scripts/classic-plus/076-1-brother-ping.ts` | 43 | port | `crates/cards/src/scripts/classic_plus/c076_1_brother_ping.rs` | cards lane 8 |
| `packages/cards/src/scripts/classic-plus/076-brother-lar.ts` | 23 | port | `crates/cards/src/scripts/classic_plus/c076_brother_lar.rs` | cards lane 8 |
| `packages/cards/src/scripts/classic-plus/077-anti-softlock.ts` | 39 | port | `crates/cards/src/scripts/classic_plus/c077_anti_softlock.rs` | cards lane 8 |
| `packages/cards/src/scripts/classic-plus/078-claudes-datacenter.ts` | 36 | port | `crates/cards/src/scripts/classic_plus/c078_claudes_datacenter.rs` | cards lane 8 |
| `packages/cards/src/scripts/classic-plus/t-ai-01-helpful-assistant.ts` | 28 | port | `crates/cards/src/scripts/classic_plus/t_ai_01_helpful_assistant.rs` | cards lane 8 |
| `packages/cards/src/scripts/classic-plus/t-ai-02-scaling-law.ts` | 27 | port | `crates/cards/src/scripts/classic_plus/t_ai_02_scaling_law.rs` | cards lane 8 |
| `packages/cards/src/scripts/classic-plus/t-ai-03-hallucination.ts` | 27 | port | `crates/cards/src/scripts/classic_plus/t_ai_03_hallucination.rs` | cards lane 8 |
| `packages/cards/src/scripts/classic-plus/t-ai-04-chain-of-thought.ts` | 26 | port | `crates/cards/src/scripts/classic_plus/t_ai_04_chain_of_thought.rs` | cards lane 8 |
| `packages/cards/src/scripts/classic-plus/t-ai-05-autocomplete.ts` | 34 | port | `crates/cards/src/scripts/classic_plus/t_ai_05_autocomplete.rs` | cards lane 8 |
| `packages/cards/src/scripts/classic-plus/t-ai-06-datacenter-fire.ts` | 42 | port | `crates/cards/src/scripts/classic_plus/t_ai_06_datacenter_fire.rs` | cards lane 8 |
| `packages/cards/src/scripts/classic-plus/t-ai-07-alignment-tax.ts` | 27 | port | `crates/cards/src/scripts/classic_plus/t_ai_07_alignment_tax.rs` | cards lane 8 |
| `packages/cards/src/scripts/classic-plus/t-ai-08-rate-limit.ts` | 43 | port | `crates/cards/src/scripts/classic_plus/t_ai_08_rate_limit.rs` | cards lane 8 |
| `packages/cards/src/scripts/classic-plus/t-ai-09-refusal.ts` | 60 | port | `crates/cards/src/scripts/classic_plus/t_ai_09_refusal.rs` | cards lane 8 |
| `packages/cards/src/scripts/classic-plus/t-ai-10-fine-tuning.ts` | 24 | port | `crates/cards/src/scripts/classic_plus/t_ai_10_fine_tuning.rs` | cards lane 8 |
| `packages/cards/test/classic-plus/050-adaptive-growth.test.ts` | 158 | port-inline | `crates/cards/src/scripts/classic_plus/c050_adaptive_growth.rs` | `#[cfg(test)] mod tests` of the card file |
| `packages/cards/test/classic-plus/051-jlockheeds-j15-fighter.test.ts` | 120 | port-inline | `crates/cards/src/scripts/classic_plus/c051_jlockheeds_j15_fighter.rs` | `#[cfg(test)] mod tests` of the card file |
| `packages/cards/test/classic-plus/052-jlockheeds-permanent-defense-contract.test.ts` | 225 | port-inline | `crates/cards/src/scripts/classic_plus/c052_jlockheeds_permanent_defense_contract.rs` | `#[cfg(test)] mod tests` of the card file |
| `packages/cards/test/classic-plus/053-book-of-tokens.test.ts` | 127 | port-inline | `crates/cards/src/scripts/classic_plus/c053_book_of_tokens.rs` | `#[cfg(test)] mod tests` of the card file |
| `packages/cards/test/classic-plus/054-book-of-books.test.ts` | 163 | port-inline | `crates/cards/src/scripts/classic_plus/c054_book_of_books.rs` | `#[cfg(test)] mod tests` of the card file |
| `packages/cards/test/classic-plus/055-book-of-greed.test.ts` | 128 | port-inline | `crates/cards/src/scripts/classic_plus/c055_book_of_greed.rs` | `#[cfg(test)] mod tests` of the card file |
| `packages/cards/test/classic-plus/056-book-of-pain.test.ts` | 135 | port-inline | `crates/cards/src/scripts/classic_plus/c056_book_of_pain.rs` | `#[cfg(test)] mod tests` of the card file |
| `packages/cards/test/classic-plus/057-book-of-stats.test.ts` | 118 | port-inline | `crates/cards/src/scripts/classic_plus/c057_book_of_stats.rs` | `#[cfg(test)] mod tests` of the card file |
| `packages/cards/test/classic-plus/058-fruit-basket.test.ts` | 125 | port-inline | `crates/cards/src/scripts/classic_plus/c058_fruit_basket.rs` | `#[cfg(test)] mod tests` of the card file |
| `packages/cards/test/classic-plus/059-all-purpose-apple.test.ts` | 118 | port-inline | `crates/cards/src/scripts/classic_plus/c059_all_purpose_apple.rs` | `#[cfg(test)] mod tests` of the card file |
| `packages/cards/test/classic-plus/060-doctors-orders.test.ts` | 156 | port-inline | `crates/cards/src/scripts/classic_plus/c060_doctors_orders.rs` | `#[cfg(test)] mod tests` of the card file |
| `packages/cards/test/classic-plus/061-bauble-bubble.test.ts` | 188 | port-inline | `crates/cards/src/scripts/classic_plus/c061_bauble_bubble.rs` | `#[cfg(test)] mod tests` of the card file |
| `packages/cards/test/classic-plus/062-kys-papaya.test.ts` | 392 | port-inline | `crates/cards/src/scripts/classic_plus/c062_kys_papaya.rs` | `#[cfg(test)] mod tests` of the card file |
| `packages/cards/test/classic-plus/063-fruit-tree.test.ts` | 196 | port-inline | `crates/cards/src/scripts/classic_plus/c063_fruit_tree.rs` | `#[cfg(test)] mod tests` of the card file |
| `packages/cards/test/classic-plus/064-mulch-muncher.test.ts` | 167 | port-inline | `crates/cards/src/scripts/classic_plus/c064_mulch_muncher.rs` | `#[cfg(test)] mod tests` of the card file |
| `packages/cards/test/classic-plus/065-1-rotten-grape.test.ts` | 111 | port-inline | `crates/cards/src/scripts/classic_plus/c065_1_rotten_grape.rs` | `#[cfg(test)] mod tests` of the card file |
| `packages/cards/test/classic-plus/065-2-normal-grape.test.ts` | 209 | port-inline | `crates/cards/src/scripts/classic_plus/c065_2_normal_grape.rs` | `#[cfg(test)] mod tests` of the card file |
| `packages/cards/test/classic-plus/065-3-large-grape.test.ts` | 143 | port-inline | `crates/cards/src/scripts/classic_plus/c065_3_large_grape.rs` | `#[cfg(test)] mod tests` of the card file |
| `packages/cards/test/classic-plus/065-4-golden-grape.test.ts` | 173 | port-inline | `crates/cards/src/scripts/classic_plus/c065_4_golden_grape.rs` | `#[cfg(test)] mod tests` of the card file |
| `packages/cards/test/classic-plus/065-5-mythic-grape.test.ts` | 115 | port-inline | `crates/cards/src/scripts/classic_plus/c065_5_mythic_grape.rs` | `#[cfg(test)] mod tests` of the card file |
| `packages/cards/test/classic-plus/065-two-grapes.test.ts` | 220 | port-inline | `crates/cards/src/scripts/classic_plus/c065_two_grapes.rs` | `#[cfg(test)] mod tests` of the card file |
| `packages/cards/test/classic-plus/066-vine-of-grapes.test.ts` | 116 | port-inline | `crates/cards/src/scripts/classic_plus/c066_vine_of_grapes.rs` | `#[cfg(test)] mod tests` of the card file |
| `packages/cards/test/classic-plus/067-pear.test.ts` | 152 | port-inline | `crates/cards/src/scripts/classic_plus/c067_pear.rs` | `#[cfg(test)] mod tests` of the card file |
| `packages/cards/test/classic-plus/068-organic-produce.test.ts` | 196 | port-inline | `crates/cards/src/scripts/classic_plus/c068_organic_produce.rs` | `#[cfg(test)] mod tests` of the card file |
| `packages/cards/test/classic-plus/069-buff-billy.test.ts` | 164 | port-inline | `crates/cards/src/scripts/classic_plus/c069_buff_billy.rs` | `#[cfg(test)] mod tests` of the card file |
| `packages/cards/test/classic-plus/070-chaos-machine.test.ts` | 228 | port-inline | `crates/cards/src/scripts/classic_plus/c070_chaos_machine.rs` | `#[cfg(test)] mod tests` of the card file |
| `packages/cards/test/classic-plus/071-book-of-buff.test.ts` | 132 | port-inline | `crates/cards/src/scripts/classic_plus/c071_book_of_buff.rs` | `#[cfg(test)] mod tests` of the card file |
| `packages/cards/test/classic-plus/072-book-of-nerf.test.ts` | 171 | port-inline | `crates/cards/src/scripts/classic_plus/c072_book_of_nerf.rs` | `#[cfg(test)] mod tests` of the card file |
| `packages/cards/test/classic-plus/073-1-classic-golem.test.ts` | 206 | port-inline | `crates/cards/src/scripts/classic_plus/c073_1_classic_golem.rs` | `#[cfg(test)] mod tests` of the card file |
| `packages/cards/test/classic-plus/073-call-to-chaos-classic-edition.test.ts` | 386 | port-inline | `crates/cards/src/scripts/classic_plus/c073_call_to_chaos_classic_edition.rs` | `#[cfg(test)] mod tests` of the card file |
| `packages/cards/test/classic-plus/074-twice-forward-one-step-backwards.test.ts` | 329 | port-inline | `crates/cards/src/scripts/classic_plus/c074_twice_forward_one_step_backwards.rs` | `#[cfg(test)] mod tests` of the card file |
| `packages/cards/test/classic-plus/075-1-j-lease-j-jungle-ex-plorer-pack.test.ts` | 142 | port-inline | `crates/cards/src/scripts/classic_plus/c075_1_j_lease_j_jungle_ex_plorer_pack.rs` | `#[cfg(test)] mod tests` of the card file |
| `packages/cards/test/classic-plus/075-j-lease-j-jungle-ex-plorer.test.ts` | 122 | port-inline | `crates/cards/src/scripts/classic_plus/c075_j_lease_j_jungle_ex_plorer.rs` | `#[cfg(test)] mod tests` of the card file |
| `packages/cards/test/classic-plus/076-1-brother-ping.test.ts` | 214 | port-inline | `crates/cards/src/scripts/classic_plus/c076_1_brother_ping.rs` | `#[cfg(test)] mod tests` of the card file |
| `packages/cards/test/classic-plus/076-brother-lar.test.ts` | 112 | port-inline | `crates/cards/src/scripts/classic_plus/c076_brother_lar.rs` | `#[cfg(test)] mod tests` of the card file |
| `packages/cards/test/classic-plus/077-anti-softlock.test.ts` | 166 | port-inline | `crates/cards/src/scripts/classic_plus/c077_anti_softlock.rs` | `#[cfg(test)] mod tests` of the card file |
| `packages/cards/test/classic-plus/078-claudes-datacenter.test.ts` | 128 | port-inline | `crates/cards/src/scripts/classic_plus/c078_claudes_datacenter.rs` | `#[cfg(test)] mod tests` of the card file |
| `packages/cards/test/classic-plus/t-ai-01-helpful-assistant.test.ts` | 129 | port-inline | `crates/cards/src/scripts/classic_plus/t_ai_01_helpful_assistant.rs` | `#[cfg(test)] mod tests` of the card file |
| `packages/cards/test/classic-plus/t-ai-02-scaling-law.test.ts` | 112 | port-inline | `crates/cards/src/scripts/classic_plus/t_ai_02_scaling_law.rs` | `#[cfg(test)] mod tests` of the card file |
| `packages/cards/test/classic-plus/t-ai-03-hallucination.test.ts` | 157 | port-inline | `crates/cards/src/scripts/classic_plus/t_ai_03_hallucination.rs` | `#[cfg(test)] mod tests` of the card file |
| `packages/cards/test/classic-plus/t-ai-04-chain-of-thought.test.ts` | 146 | port-inline | `crates/cards/src/scripts/classic_plus/t_ai_04_chain_of_thought.rs` | `#[cfg(test)] mod tests` of the card file |
| `packages/cards/test/classic-plus/t-ai-05-autocomplete.test.ts` | 199 | port-inline | `crates/cards/src/scripts/classic_plus/t_ai_05_autocomplete.rs` | `#[cfg(test)] mod tests` of the card file |
| `packages/cards/test/classic-plus/t-ai-06-datacenter-fire.test.ts` | 143 | port-inline | `crates/cards/src/scripts/classic_plus/t_ai_06_datacenter_fire.rs` | `#[cfg(test)] mod tests` of the card file |
| `packages/cards/test/classic-plus/t-ai-07-alignment-tax.test.ts` | 127 | port-inline | `crates/cards/src/scripts/classic_plus/t_ai_07_alignment_tax.rs` | `#[cfg(test)] mod tests` of the card file |
| `packages/cards/test/classic-plus/t-ai-08-rate-limit.test.ts` | 185 | port-inline | `crates/cards/src/scripts/classic_plus/t_ai_08_rate_limit.rs` | `#[cfg(test)] mod tests` of the card file |
| `packages/cards/test/classic-plus/t-ai-09-refusal.test.ts` | 223 | port-inline | `crates/cards/src/scripts/classic_plus/t_ai_09_refusal.rs` | `#[cfg(test)] mod tests` of the card file |
| `packages/cards/test/classic-plus/t-ai-10-fine-tuning.test.ts` | 119 | port-inline | `crates/cards/src/scripts/classic_plus/t_ai_10_fine_tuning.rs` | `#[cfg(test)] mod tests` of the card file |

## Part 17: the AI in Rust (generation 0)

| TypeScript | Lines | Action | Rust | Note |
|---|---|---|---|---|
| `packages/ai/src/baselines.ts` | 66 | port | `crates/ai/src/baselines.rs` |  |
| `packages/ai/src/candidates.ts` | 195 | port | `crates/ai/src/candidates.rs` |  |
| `packages/ai/src/config.ts` | 381 | port | `crates/ai/src/config.rs` | minus EMOTE_TRIGGERS, EMOTE_REPLY_KEYS, AI_EMOTE, AI_PERSONAS, which move to apps/web/src/practice/personas.ts (part 21) |
| `packages/ai/src/decide.ts` | 225 | port | `crates/ai/src/decide.rs` |  |
| `packages/ai/src/deck.ts` | 242 | port | `crates/ai/src/deck.rs` |  |
| `packages/ai/src/determinize.ts` | 166 | port | `crates/ai/src/determinize.rs` |  |
| `packages/ai/src/devRun.ts` | 80 | port | `crates/ai/src/dev_run.rs` |  |
| `packages/ai/src/evaluate.ts` | 240 | port | `crates/ai/src/evaluate.rs` |  |
| `packages/ai/src/gate.ts` | 249 | port | `crates/ai/src/gate.rs` |  |
| `packages/ai/src/lethal.ts` | 247 | port | `crates/ai/src/lethal.rs` |  |
| `packages/ai/src/match.ts` | 234 | port | `crates/ai/src/match_.rs` |  |
| `packages/ai/src/mulligan.ts` | 21 | port | `crates/ai/src/mulligan.rs` |  |
| `packages/ai/src/observe.ts` | 373 | port | `crates/ai/src/observe.rs` |  |
| `packages/ai/src/reply.ts` | 234 | port | `crates/ai/src/reply.rs` |  |
| `packages/ai/src/search.ts` | 143 | port | `crates/ai/src/search.rs` |  |
| `packages/ai/src/shadowBan.ts` | 53 | port | `crates/ai/src/shadow_ban.rs` |  |
| `packages/ai/src/simulate.ts` | 212 | port | `crates/ai/src/simulate.rs` |  |
| `packages/ai/src/sweep.ts` | 450 | port | `crates/ai/src/sweep.rs` |  |
| `packages/ai/src/types.ts` | 65 | port | `crates/ai/src/types.rs` |  |
| `packages/ai/test/_support.ts` | 212 | port | `crates/ai/tests/ai/support.rs` |  |
| `packages/ai/test/activate.test.ts` | 200 | port | `crates/ai/tests/ai/activate.rs` |  |
| `packages/ai/test/answer-key.test.ts` | 90 | port | `crates/ai/tests/ai/answer_key.rs` |  |
| `packages/ai/test/decide.test.ts` | 403 | port | `crates/ai/tests/ai/decide.rs` |  |
| `packages/ai/test/deck.test.ts` | 275 | port | `crates/ai/tests/ai/deck.rs` |  |
| `packages/ai/test/determinize-shown-cost.test.ts` | 105 | port | `crates/ai/tests/ai/determinize_shown_cost.rs` |  |
| `packages/ai/test/dev-run.test.ts` | 97 | port | `crates/ai/tests/ai/dev_run.rs` |  |
| `packages/ai/test/evaluate-v020.test.ts` | 258 | port | `crates/ai/tests/ai/evaluate_v020.rs` |  |
| `packages/ai/test/evaluate.test.ts` | 223 | port | `crates/ai/tests/ai/evaluate.rs` |  |
| `packages/ai/test/lethal.test.ts` | 188 | port | `crates/ai/tests/ai/lethal.rs` |  |
| `packages/ai/test/match-refusal.test.ts` | 120 | port | `crates/ai/tests/ai/match_refusal.rs` |  |
| `packages/ai/test/match.test.ts` | 294 | port | `crates/ai/tests/ai/match_.rs` |  |
| `packages/ai/test/observe-instance-data.test.ts` | 43 | port | `crates/ai/tests/ai/observe_instance_data.rs` |  |
| `packages/ai/test/observe.test.ts` | 663 | port | `crates/ai/tests/ai/observe.rs` |  |
| `packages/ai/test/prompts-v020.test.ts` | 267 | port | `crates/ai/tests/ai/prompts_v020.rs` |  |
| `packages/ai/test/puzzles.test.ts` | 273 | port | `crates/ai/tests/ai/puzzles.rs` |  |
| `packages/ai/test/redact-announce.test.ts` | 40 | port | `crates/ai/tests/ai/redact_announce.rs` |  |
| `packages/ai/test/redact-backrow-piles.test.ts` | 49 | port | `crates/ai/tests/ai/redact_backrow_piles.rs` |  |
| `packages/ai/test/redact-board-history.test.ts` | 30 | port | `crates/ai/tests/ai/redact_board_history.rs` |  |
| `packages/ai/test/redact-fusion.test.ts` | 51 | port | `crates/ai/tests/ai/redact_fusion.rs` |  |
| `packages/ai/test/redact-last-boards.test.ts` | 25 | port | `crates/ai/tests/ai/redact_last_boards.rs` |  |
| `packages/ai/test/redact-live-face-down.test.ts` | 96 | port | `crates/ai/tests/ai/redact_live_face_down.rs` |  |
| `packages/ai/test/reply.test.ts` | 170 | port | `crates/ai/tests/ai/reply.rs` |  |
| `packages/ai/test/search.test.ts` | 337 | port | `crates/ai/tests/ai/search.rs` |  |
| `packages/ai/test/shadowBan.test.ts` | 432 | port | `crates/ai/tests/ai/shadow_ban.rs` |  |
| `packages/ai/test/surface.test.ts` | 319 | port | `crates/ai/tests/ai/surface.rs` |  |
| `packages/ai/test/tutorial-tier.test.ts` | 240 | port | `crates/ai/tests/ai/tutorial_tier.rs` |  |

## Part 18: server 1: the HTTP API and auth

| TypeScript | Lines | Action | Rust | Note |
|---|---|---|---|---|
| `apps/server/src/api/auth.ts` | 898 | port | `crates/server/src/api/auth.rs` | provider half → crates/server/src/auth.rs |
| `apps/server/src/api/catalog.ts` | 276 | port | `crates/server/src/api/catalog.rs` |  |
| `apps/server/src/api/codes.ts` | 424 | port | `crates/server/src/api/codes.rs` |  |
| `apps/server/src/api/collection.ts` | 192 | port | `crates/server/src/api/collection.rs` |  |
| `apps/server/src/api/cors.ts` | 160 | port | `crates/server/src/api/cors.rs` |  |
| `apps/server/src/api/crypto.ts` | 106 | port | `crates/server/src/api/crypto.rs` |  |
| `apps/server/src/api/decks.ts` | 674 | port | `crates/server/src/api/decks.rs` |  |
| `apps/server/src/api/deps.ts` | 78 | delete | — | defaults fold into config.rs; ROOM_CODE_TTL_SECONDS added |
| `apps/server/src/api/e2e.ts` | 301 | port | `crates/server/src/api/e2e.rs` |  |
| `apps/server/src/api/http.ts` | 709 | port | `crates/server/src/api/http.rs` |  |
| `apps/server/src/api/loadout-validator.ts` | 71 | delete | — | handlers call jackioh_engine::validator directly |
| `apps/server/src/api/ranked.ts` | 474 | port | `crates/server/src/api/ranked.rs` |  |
| `apps/server/src/api/retention.ts` | 26 | port | `crates/server/src/api/retention.rs` |  |
| `apps/server/src/api/settings.ts` | 151 | port | `crates/server/src/api/settings.rs` |  |
| `apps/server/src/api/stats.ts` | 432 | port | `crates/server/src/api/stats.rs` |  |
| `apps/server/src/api/tutorial.ts` | 121 | port | `crates/server/src/api/tutorial.rs` |  |
| `apps/server/src/config.ts` | 782 | port | `crates/server/src/config.rs` |  |
| `apps/server/src/env.ts` | 349 | port | `crates/server/src/env.rs` |  |
| `apps/server/src/index.ts` | 401 | port | `crates/server/src/app.rs` | router, background loops, boot; `main.rs` is part 1's |
| `apps/server/src/ranked/glicko2.ts` | 147 | port | `crates/server/src/ranked/glicko2.rs` |  |
| `apps/server/src/ranked/ladder.ts` | 286 | port | `crates/server/src/ranked/ladder.rs` |  |
| `apps/server/src/ranked/season.ts` | 106 | port | `crates/server/src/ranked/season.rs` |  |
| `apps/server/test/api/account.test.ts` | 254 | port | `crates/server/tests/api/account.rs` |  |
| `apps/server/test/api/auth.test.ts` | 984 | port | `crates/server/tests/api/auth.rs` |  |
| `apps/server/test/api/catalog.test.ts` | 421 | port | `crates/server/tests/api/catalog.rs` |  |
| `apps/server/test/api/client-address.test.ts` | 637 | port | `crates/server/tests/api/client_address.rs` |  |
| `apps/server/test/api/code-input-parity.test.ts` | 268 | port | `crates/server/tests/api/code_input_parity.rs` |  |
| `apps/server/test/api/codes.test.ts` | 807 | port | `crates/server/tests/api/codes.rs` |  |
| `apps/server/test/api/collection.test.ts` | 333 | port | `crates/server/tests/api/collection.rs` |  |
| `apps/server/test/api/cors.test.ts` | 250 | port | `crates/server/tests/api/cors.rs` |  |
| `apps/server/test/api/decks.test.ts` | 1032 | port | `crates/server/tests/api/decks.rs` |  |
| `apps/server/test/api/e2e.test.ts` | 670 | port | `crates/server/tests/api/e2e.rs` |  |
| `apps/server/test/api/ranked.test.ts` | 354 | port | `crates/server/tests/api/ranked.rs` |  |
| `apps/server/test/api/rate-limit.test.ts` | 237 | port | `crates/server/tests/api/rate_limit.rs` |  |
| `apps/server/test/api/redeem-feedback.test.ts` | 673 | port | `crates/server/tests/api/redeem_feedback.rs` |  |
| `apps/server/test/api/retention.test.ts` | 69 | port | `crates/server/tests/api/retention.rs` |  |
| `apps/server/test/api/settings.test.ts` | 233 | port | `crates/server/tests/api/settings.rs` |  |
| `apps/server/test/api/stats.test.ts` | 509 | port | `crates/server/tests/api/stats.rs` |  |
| `apps/server/test/api/tutorial.test.ts` | 174 | port | `crates/server/tests/api/tutorial.rs` |  |
| `apps/server/test/env-deployed-commit.test.ts` | 41 | port | `crates/server/tests/api/env_deployed_commit.rs` |  |
| `apps/server/test/fakes/deps.ts` | 494 | port | `crates/server/tests/support/deps.rs` |  |
| `apps/server/test/ranked/glicko2.test.ts` | 125 | port | `crates/server/tests/api/glicko2.rs` |  |
| `apps/server/test/ranked/ladder.test.ts` | 239 | port | `crates/server/tests/api/ladder.rs` |  |
| `apps/server/test/ranked/season.test.ts` | 66 | port | `crates/server/tests/api/season.rs` |  |

## Part 19: server 2: the match lifecycle and the WebSocket

| TypeScript | Lines | Action | Rust | Note |
|---|---|---|---|---|
| `apps/server/src/api/game-records.ts` | 85 | port | `crates/server/src/api/game_records.rs` |  |
| `apps/server/src/api/queue.ts` | 578 | port | `crates/server/src/api/queue.rs` |  |
| `apps/server/src/api/rematch.ts` | 276 | port | `crates/server/src/api/rematch.rs` |  |
| `apps/server/src/api/results.ts` | 325 | port | `crates/server/src/api/results.rs` |  |
| `apps/server/src/api/series-rules.ts` | 654 | port | `crates/server/src/api/series_rules.rs` |  |
| `apps/server/src/api/series.ts` | 570 | port | `crates/server/src/api/series.rs` |  |
| `apps/server/src/match/actor.ts` | 917 | port | `crates/server/src/actor/match_actor.rs` |  |
| `apps/server/src/match/clock.ts` | 299 | port | `crates/server/src/actor/clock.rs` |  |
| `apps/server/src/match/contracts.ts` | 158 | port | `crates/server/src/actor/contracts.rs` |  |
| `apps/server/src/match/engine.real.ts` | 103 | port | `crates/server/src/actor/engine.rs` | merged with engine.ts: plain functions over `jackioh_engine` |
| `apps/server/src/match/engine.ts` | 198 | port | `crates/server/src/actor/engine.rs` | plain functions over `jackioh_engine`; no Engine trait, no dynamic import |
| `apps/server/src/match/protocol.ts` | 532 | port | `crates/server/src/actor/protocol.rs` |  |
| `apps/server/src/match/registry.ts` | 195 | port | `crates/server/src/actor/registry.rs` |  |
| `apps/server/src/match/rooms.ts` | 350 | port | `crates/server/src/actor/rooms.rs` |  |
| `apps/server/src/match/wsServer.ts` | 341 | port | `crates/server/src/actor/ws_server.rs` |  |
| `apps/server/test/api/game-records.test.ts` | 236 | port | `crates/server/tests/api/game_records.rs` |  |
| `apps/server/test/api/queue.test.ts` | 1241 | port | `crates/server/tests/api/queue.rs` |  |
| `apps/server/test/api/rematch.test.ts` | 638 | port | `crates/server/tests/api/rematch.rs` |  |
| `apps/server/test/api/results.test.ts` | 579 | port | `crates/server/tests/api/results.rs` |  |
| `apps/server/test/api/series-rules.test.ts` | 799 | port | `crates/server/tests/api/series_rules.rs` |  |
| `apps/server/test/api/series.test.ts` | 661 | port | `crates/server/tests/api/series.rs` |  |
| `apps/server/test/fakes/engine.ts` | 659 | port | `crates/server/tests/support/engine.rs` | test cards as real scripts (testkit override); no ScriptedEngine |
| `apps/server/test/fakes/socket.ts` | 91 | port | `crates/server/tests/support/socket.rs` |  |
| `apps/server/test/match/actor.test.ts` | 1991 | port | `crates/server/tests/actor/match_actor.rs` |  |
| `apps/server/test/match/aim.test.ts` | 232 | port | `crates/server/tests/actor/aim.rs` |  |
| `apps/server/test/match/clock.test.ts` | 451 | port | `crates/server/tests/actor/clock.rs` |  |
| `apps/server/test/match/dealt-deck.test.ts` | 128 | port | `crates/server/tests/actor/dealt_deck.rs` |  |
| `apps/server/test/match/engine.real.test.ts` | 198 | port | `crates/server/tests/actor/engine_real.rs` |  |
| `apps/server/test/match/glitch.test.ts` | 324 | port | `crates/server/tests/actor/glitch.rs` |  |
| `apps/server/test/match/last-boards.test.ts` | 225 | port | `crates/server/tests/actor/last_boards.rs` |  |
| `apps/server/test/match/recovery.test.ts` | 622 | port | `crates/server/tests/actor/recovery.rs` |  |
| `apps/server/test/match/rooms.test.ts` | 704 | port | `crates/server/tests/actor/rooms.rs` |  |
| `apps/server/test/match/series-recovery.test.ts` | 488 | port | `crates/server/tests/actor/series_recovery.rs` |  |
| `apps/server/test/match/ws-server.test.ts` | 157 | port | `crates/server/tests/actor/ws_server.rs` |  |

## Part 20: server 3: persistence, migrations, CLIs and the Docker deploy

| TypeScript | Lines | Action | Rust | Note |
|---|---|---|---|---|
| `apps/server/.env.example` | 70 | copy | `crates/server/.env.example` |  |
| `apps/server/src/api/e2e-store.ts` | 767 | port | `crates/server/src/db/fake.rs` | one FakeStore for unit tests and E2E=1 |
| `apps/server/src/api/memory-stores.ts` | 866 | port | `crates/server/src/db/fake.rs` | one FakeStore for unit tests and E2E=1 |
| `apps/server/src/api/ports.ts` | 1272 | port | `crates/server/src/db/store.rs` | Db/Tx enums, every Store method, row types; Timers/Logger/Ids/Hashes go (SURFACE §11) |
| `apps/server/src/db/card-stats.ts` | 138 | port | `crates/server/src/cli/card_stats.rs` |  |
| `apps/server/src/db/import-dev-records.ts` | 86 | port | `crates/server/src/cli/import_dev_records.rs` |  |
| `apps/server/src/db/migrate.ts` | 156 | port | `crates/server/src/db/migrate.rs` | checksum and REWRITTEN exactly (SURFACE §11) |
| `apps/server/src/db/mint-code.ts` | 113 | port | `crates/server/src/cli/mint_code.rs` |  |
| `apps/server/src/db/season-start.ts` | 109 | port | `crates/server/src/cli/season_start.rs` |  |
| `apps/server/src/db/seed-accounts.ts` | 286 | port | `crates/server/src/cli/seed_accounts.rs` |  |
| `apps/server/src/db/seed-catalog.ts` | 187 | port | `crates/server/src/cli/seed_catalog.rs` |  |
| `apps/server/src/db/store.ts` | 2979 | port | `crates/server/src/db/pg.rs` | sqlx; keep every SET LOCAL role/claim and app.* call |
| `apps/server/test/db/bootstrap.sql` | 57 | port | `crates/server/tests/db/bootstrap.sql` |  |
| `apps/server/test/db/card-stats.test.ts` | 157 | port | `crates/server/tests/store/card_stats.rs` |  |
| `apps/server/test/db/contract.memory.test.ts` | 13 | delete | — | TS pool/dynamic-import plumbing, or a one-line runner folded into the contract |
| `apps/server/test/db/contract.postgres.spec.ts` | 10 | delete | — | TS pool/dynamic-import plumbing, or a one-line runner folded into the contract |
| `apps/server/test/db/contract.ts` | 2251 | port | `crates/server/tests/store/contract.rs` | run against FakeStore always and PgStore when DATABASE_URL is set |
| `apps/server/test/db/grants.sql` | 11 | port | `crates/server/tests/db/grants.sql` |  |
| `apps/server/test/db/harness.ts` | 236 | delete | — | TS pool/dynamic-import plumbing, or a one-line runner folded into the contract |
| `apps/server/test/db/loadstore-boot.ts` | 45 | delete | — | TS pool/dynamic-import plumbing, or a one-line runner folded into the contract |
| `apps/server/test/db/migrations-pinned.test.ts` | 75 | port | `crates/server/tests/store/migrations_pinned.rs` |  |
| `apps/server/test/db/mint-code.test.ts` | 85 | port | `crates/server/tests/store/mint_code.rs` |  |
| `apps/server/test/db/pool-error.test.ts` | 156 | delete | — | TS pool/dynamic-import plumbing, or a one-line runner folded into the contract |
| `apps/server/test/db/postgres.spec.ts` | 743 | port | `crates/server/tests/store/postgres.rs` |  |
| `apps/server/test/db/redeem-race.memory.test.ts` | 13 | delete | — | TS pool/dynamic-import plumbing, or a one-line runner folded into the contract |
| `apps/server/test/db/redeem-race.postgres.spec.ts` | 10 | delete | — | TS pool/dynamic-import plumbing, or a one-line runner folded into the contract |
| `apps/server/test/db/redeem-race.ts` | 245 | port | `crates/server/tests/store/redeem_race.rs` | run against FakeStore always and PgStore when DATABASE_URL is set |
| `apps/server/test/db/run.sh` | 74 | port | `crates/server/tests/db/run.sh` |  |
| `apps/server/test/db/season-start.test.ts` | 119 | port | `crates/server/tests/store/season_start.rs` |  |
| `apps/server/test/db/seed-accounts.test.ts` | 58 | port | `crates/server/tests/store/seed_accounts.rs` |  |
| `apps/server/test/db/seed-catalog.spec.ts` | 106 | port | `crates/server/tests/store/seed_catalog.rs` |  |
| `apps/server/test/db/seed-catalog.test.ts` | 121 | port | `crates/server/tests/store/seed_catalog.rs` |  |
| `apps/server/test/db/store-guards.test.ts` | 32 | delete | — | TS pool/dynamic-import plumbing, or a one-line runner folded into the contract |
| `apps/server/test/db/vitest.config.ts` | 31 | delete | — | TS pool/dynamic-import plumbing, or a one-line runner folded into the contract |
| `apps/server/test/deploy/rehearse.sh` | 205 | port | `crates/server/tests/deploy/rehearse.sh` | rehearses the Docker image and render.yaml |
| `apps/server/test/fakes/store.ts` | 546 | delete | — | FakeStore is the fake |
| `apps/server/test/sql/00_supabase_stub.sql` | 39 | copy | `crates/server/tests/sql/00_supabase_stub.sql` | unchanged |
| `apps/server/test/sql/01_schema_invariants.sql` | 932 | copy | `crates/server/tests/sql/01_schema_invariants.sql` | unchanged |
| `apps/server/test/sql/02_rls_as_client.sql` | 943 | copy | `crates/server/tests/sql/02_rls_as_client.sql` | unchanged |
| `apps/server/test/sql/03_match_lifecycle.sql` | 1408 | copy | `crates/server/tests/sql/03_match_lifecycle.sql` | unchanged |
| `apps/server/test/sql/03b_legacy_loadout_seed.sql` | 102 | copy | `crates/server/tests/sql/03b_legacy_loadout_seed.sql` | unchanged |
| `apps/server/test/sql/04_decks_and_series.sql` | 802 | copy | `crates/server/tests/sql/04_decks_and_series.sql` | unchanged |
| `apps/server/test/sql/05_tutorial_progress.sql` | 222 | copy | `crates/server/tests/sql/05_tutorial_progress.sql` | unchanged |
| `apps/server/test/sql/06_account_deletion.sql` | 218 | copy | `crates/server/tests/sql/06_account_deletion.sql` | unchanged |
| `apps/server/test/sql/07_retention_purge.sql` | 80 | copy | `crates/server/tests/sql/07_retention_purge.sql` | unchanged |
| `apps/server/test/sql/08_game_records.sql` | 85 | copy | `crates/server/tests/sql/08_game_records.sql` | unchanged |
| `apps/server/test/sql/09_catalog_growth.sql` | 81 | copy | `crates/server/tests/sql/09_catalog_growth.sql` | unchanged |
| `apps/server/test/sql/10_last_boards.sql` | 215 | copy | `crates/server/tests/sql/10_last_boards.sql` | unchanged |
| `apps/server/test/sql/11_player_settings.sql` | 256 | copy | `crates/server/tests/sql/11_player_settings.sql` | unchanged |
| `apps/server/test/sql/12_ranked.sql` | 475 | copy | `crates/server/tests/sql/12_ranked.sql` | unchanged |
| `apps/server/test/sql/13_glitch.sql` | 114 | copy | `crates/server/tests/sql/13_glitch.sql` | unchanged |
| `apps/server/test/sql/14_patch_retcon.sql` | 151 | copy | `crates/server/tests/sql/14_patch_retcon.sql` | unchanged |
| `apps/server/test/sql/run.sh` | 151 | copy | `crates/server/tests/sql/run.sh` | then its migrations path points at crates/server/migrations |

## Part 21: the WASM bindings and the web client on them

| TypeScript | Lines | Action | Rust | Note |
|---|---|---|---|---|
| `packages/ai/src/personas.ts` | 362 | copy | `apps/web/src/practice/personas.ts` | cosmetic emote personas stay TypeScript |
| `packages/ai/test/personas.test.ts` | 779 | copy | `apps/web/src/practice/personas.test.ts` | drop l.756–776 (reads packages/ai source) |
| `packages/cards/src/flavour.ts` | 24 | merge | `apps/web/src/cards/flavour.ts` | the `CardFlavour` type and the two caps move into the web's existing file; the original goes with `packages/` (part 37) |

## Part 22: the jackioh CLI: fuzz, catalog, patches, gates, sweep, stats

| TypeScript | Lines | Action | Rust | Note |
|---|---|---|---|---|
| `packages/ai/scripts/gate-merge.ts` | 70 | port | `crates/tools/src/gate.rs` |  |
| `packages/ai/scripts/stats.ts` | 121 | port | `crates/tools/src/stats.rs` |  |
| `packages/ai/scripts/sweep.ts` | 272 | port | `crates/tools/src/sweep.rs` |  |
| `packages/ai/scripts/trace.ts` | 97 | port | `crates/tools/src/trace.rs` |  |
| `packages/ai/test/gate-greedy.test.ts` | 172 | port | `crates/tools/src/gate.rs` | the gates become `cargo jackioh gate` (smoke run as a #[test] with 4 games) |
| `packages/ai/test/gate-hard-easy.test.ts` | 123 | port | `crates/tools/src/gate.rs` | the gates become `cargo jackioh gate` (smoke run as a #[test] with 4 games) |
| `packages/ai/test/gate-perf.test.ts` | 145 | port | `crates/tools/src/gate.rs` | the gates become `cargo jackioh gate` (smoke run as a #[test] with 4 games) |
| `packages/ai/test/gate-random.test.ts` | 152 | port | `crates/tools/src/gate.rs` | the gates become `cargo jackioh gate` (smoke run as a #[test] with 4 games) |
| `packages/cards/scripts/naming.ts` | 249 | port | `crates/tools/src/patches.rs` |  |
| `packages/cards/scripts/patch.ts` | 64 | port | `crates/tools/src/patches.rs` |  |
| `packages/cards/scripts/patches-io.ts` | 418 | port | `crates/tools/src/patches.rs` |  |
| `packages/cards/scripts/patches.ts` | 372 | port | `crates/tools/src/patches.rs` |  |
| `packages/cards/scripts/validate-catalog.ts` | 655 | port | `crates/tools/src/catalog.rs` |  |
| `packages/cards/scripts/versions.ts` | 39 | port | `crates/tools/src/patches.rs` |  |
| `packages/cards/test/card-text.test.ts` | 689 | port | `crates/cards/tests/cross/card_text.rs` |  |
| `packages/cards/test/catalog.test.ts` | 820 | port | `crates/cards/tests/cross/catalog.rs` |  |
| `packages/cards/test/flavour.test.ts` | 59 | port | `crates/cards/tests/cross/flavour.rs` |  |
| `packages/cards/test/fuzz-handicap.test.ts` | 154 | port | `crates/tools/src/fuzz.rs` | the fuzz loop becomes `cargo jackioh fuzz`; its tests seeds 1–20 as `#[cfg(test)]` |
| `packages/cards/test/fuzz.test.ts` | 555 | port | `crates/tools/src/fuzz.rs` | the fuzz loop becomes `cargo jackioh fuzz`; its tests seeds 1–20 as `#[cfg(test)]` |
| `packages/cards/test/params.test.ts` | 128 | port | `crates/cards/tests/cross/params.rs` |  |
| `packages/cards/test/patches-ship.test.ts` | 545 | port | `crates/tools/src/patches.rs` |  |
| `packages/cards/test/patches.test.ts` | 437 | port | `crates/tools/src/patches.rs` |  |
| `packages/cards/test/pools-and-randomness.test.ts` | 157 | port | `crates/cards/tests/cross/pools_and_randomness.rs` |  |
| `packages/cards/test/query.test.ts` | 402 | port | `crates/cards/tests/cross/query.rs` |  |
| `packages/cards/test/radiant-standard.test.ts` | 103 | port | `crates/cards/tests/cross/radiant_standard.rs` |  |
| `packages/cards/test/references.test.ts` | 157 | port | `crates/cards/tests/cross/references.rs` |  |
| `packages/cards/test/registry.test.ts` | 331 | port | `crates/cards/tests/cross/registry.rs` |  |
| `packages/cards/test/versions.test.ts` | 26 | port | `crates/tools/src/patches.rs` |  |

## Part 23: golden traces recorded from the TypeScript engine

| TypeScript | Lines | Action | Rust | Note |
|---|---|---|---|---|
| `packages/cards/test/fixtures/01-hotseat-full-game.json` | 336 | copy | `crates/engine/tests/golden/01-hotseat-full-game.json` |  |
| `packages/cards/test/hotseat-replay.test.ts` | 213 | port | `crates/engine/tests/golden.rs` | fold of 01-hotseat-full-game.json must hash "a798906b" |

## Part 24: engine tests 1: effect verbs and fixtures

| TypeScript | Lines | Action | Rust | Note |
|---|---|---|---|---|
| `packages/engine/test/effects-after-check.test.ts` | 229 | port | `crates/engine/tests/rules/effects_after_check.rs` |  |
| `packages/engine/test/effects-animate.test.ts` | 76 | port | `crates/engine/tests/rules/effects_animate.rs` |  |
| `packages/engine/test/effects-boardwide.test.ts` | 698 | port | `crates/engine/tests/rules/effects_boardwide.rs` |  |
| `packages/engine/test/effects-brittle.test.ts` | 87 | port | `crates/engine/tests/rules/effects_brittle.rs` |  |
| `packages/engine/test/effects-buff.test.ts` | 298 | port | `crates/engine/tests/rules/effects_buff.rs` |  |
| `packages/engine/test/effects-cardScope.test.ts` | 100 | port | `crates/engine/tests/rules/effects_card_scope.rs` |  |
| `packages/engine/test/effects-cast-chaos.test.ts` | 80 | port | `crates/engine/tests/rules/effects_cast_chaos.rs` |  |
| `packages/engine/test/effects-cast.test.ts` | 496 | port | `crates/engine/tests/rules/effects_cast.rs` |  |
| `packages/engine/test/effects-choose-where.test.ts` | 72 | port | `crates/engine/tests/rules/effects_choose_where.rs` |  |
| `packages/engine/test/effects-choose.test.ts` | 455 | port | `crates/engine/tests/rules/effects_choose.rs` |  |
| `packages/engine/test/effects-combat.test.ts` | 639 | port | `crates/engine/tests/rules/effects_combat.rs` |  |
| `packages/engine/test/effects-core.test.ts` | 784 | port | `crates/engine/tests/rules/effects_core.rs` |  |
| `packages/engine/test/effects-cost.test.ts` | 203 | port | `crates/engine/tests/rules/effects_cost.rs` |  |
| `packages/engine/test/effects-counters.test.ts` | 200 | port | `crates/engine/tests/rules/effects_counters.rs` |  |
| `packages/engine/test/effects-cry.test.ts` | 248 | port | `crates/engine/tests/rules/effects_cry.rs` |  |
| `packages/engine/test/effects-damage.test.ts` | 209 | port | `crates/engine/tests/rules/effects_damage.rs` |  |
| `packages/engine/test/effects-datacenter.test.ts` | 262 | port | `crates/engine/tests/rules/effects_datacenter.rs` |  |
| `packages/engine/test/effects-delay.test.ts` | 555 | port | `crates/engine/tests/rules/effects_delay.rs` |  |
| `packages/engine/test/effects-destroy.test.ts` | 338 | port | `crates/engine/tests/rules/effects_destroy.rs` |  |
| `packages/engine/test/effects-drawWhile.test.ts` | 128 | port | `crates/engine/tests/rules/effects_draw_while.rs` |  |
| `packages/engine/test/effects-each.test.ts` | 110 | port | `crates/engine/tests/rules/effects_each.rs` |  |
| `packages/engine/test/effects-enchant.test.ts` | 115 | port | `crates/engine/tests/rules/effects_enchant.rs` |  |
| `packages/engine/test/effects-flicker.test.ts` | 224 | port | `crates/engine/tests/rules/effects_flicker.rs` |  |
| `packages/engine/test/effects-fruit.test.ts` | 356 | port | `crates/engine/tests/rules/effects_fruit.rs` |  |
| `packages/engine/test/effects-give.test.ts` | 353 | port | `crates/engine/tests/rules/effects_give.rs` |  |
| `packages/engine/test/effects-hand-exile.test.ts` | 88 | port | `crates/engine/tests/rules/effects_hand_exile.rs` |  |
| `packages/engine/test/effects-heal.test.ts` | 182 | port | `crates/engine/tests/rules/effects_heal.rs` |  |
| `packages/engine/test/effects-health.test.ts` | 105 | port | `crates/engine/tests/rules/effects_health.rs` |  |
| `packages/engine/test/effects-library.test.ts` | 600 | port | `crates/engine/tests/rules/effects_library.rs` |  |
| `packages/engine/test/effects-locks.test.ts` | 199 | port | `crates/engine/tests/rules/effects_locks.rs` |  |
| `packages/engine/test/effects-move.test.ts` | 492 | port | `crates/engine/tests/rules/effects_move.rs` |  |
| `packages/engine/test/effects-perks.test.ts` | 99 | port | `crates/engine/tests/rules/effects_perks.rs` |  |
| `packages/engine/test/effects-plague-random-cast.test.ts` | 86 | port | `crates/engine/tests/rules/effects_plague_random_cast.rs` |  |
| `packages/engine/test/effects-plague.test.ts` | 464 | port | `crates/engine/tests/rules/effects_plague.rs` |  |
| `packages/engine/test/effects-plus-c.test.ts` | 290 | port | `crates/engine/tests/rules/effects_plus_c.rs` |  |
| `packages/engine/test/effects-radiant.test.ts` | 265 | port | `crates/engine/tests/rules/effects_radiant.rs` |  |
| `packages/engine/test/effects-random.test.ts` | 720 | port | `crates/engine/tests/rules/effects_random.rs` |  |
| `packages/engine/test/effects-reveal.test.ts` | 47 | port | `crates/engine/tests/rules/effects_reveal.rs` |  |
| `packages/engine/test/effects-shuffle-card.test.ts` | 106 | port | `crates/engine/tests/rules/effects_shuffle_card.rs` |  |
| `packages/engine/test/effects-split.test.ts` | 87 | port | `crates/engine/tests/rules/effects_split.rs` |  |
| `packages/engine/test/effects-statuses.test.ts` | 101 | port | `crates/engine/tests/rules/effects_statuses.rs` |  |
| `packages/engine/test/effects-steal.test.ts` | 192 | port | `crates/engine/tests/rules/effects_steal.rs` |  |
| `packages/engine/test/effects-summon-copies.test.ts` | 382 | port | `crates/engine/tests/rules/effects_summon_copies.rs` |  |
| `packages/engine/test/effects-summon.test.ts` | 401 | port | `crates/engine/tests/rules/effects_summon.rs` |  |
| `packages/engine/test/effects-summonThis.test.ts` | 231 | port | `crates/engine/tests/rules/effects_summon_this.rs` |  |
| `packages/engine/test/effects-swap.test.ts` | 306 | port | `crates/engine/tests/rules/effects_swap.rs` |  |
| `packages/engine/test/effects-targets.test.ts` | 320 | port | `crates/engine/tests/rules/effects_targets.rs` |  |
| `packages/engine/test/effects-transform.test.ts` | 372 | port | `crates/engine/tests/rules/effects_transform.rs` |  |
| `packages/engine/test/effects-tune.test.ts` | 649 | port | `crates/engine/tests/rules/effects_tune.rs` |  |
| `packages/engine/test/effects-turnEnd.test.ts` | 294 | port | `crates/engine/tests/rules/effects_turn_end.rs` |  |
| `packages/engine/test/fixtures/activate.ts` | 393 | port | `crates/engine/tests/rules/fixtures/activate.rs` | test-only scripts/catalog, registered through testkit |
| `packages/engine/test/fixtures/boardHistory.ts` | 57 | port | `crates/engine/tests/rules/fixtures/board_history.rs` | test-only scripts/catalog, registered through testkit |
| `packages/engine/test/fixtures/callToChaosPlus.ts` | 81 | port | `crates/engine/tests/rules/fixtures/call_to_chaos_plus.rs` | test-only scripts/catalog, registered through testkit |
| `packages/engine/test/fixtures/catalog.ts` | 78 | port | `crates/engine/tests/rules/fixtures/catalog.rs` | test-only scripts/catalog, registered through testkit |
| `packages/engine/test/fixtures/combat.ts` | 408 | port | `crates/engine/tests/rules/fixtures/combat.rs` | test-only scripts/catalog, registered through testkit |
| `packages/engine/test/fixtures/copiedText.ts` | 136 | port | `crates/engine/tests/rules/fixtures/copied_text.rs` | test-only scripts/catalog, registered through testkit |
| `packages/engine/test/fixtures/corePatches.ts` | 86 | port | `crates/engine/tests/rules/fixtures/core_patches.rs` | test-only scripts/catalog, registered through testkit |
| `packages/engine/test/fixtures/damage-combat.ts` | 510 | port | `crates/engine/tests/rules/fixtures/damage_combat.rs` | test-only scripts/catalog, registered through testkit |
| `packages/engine/test/fixtures/datacenter.ts` | 63 | port | `crates/engine/tests/rules/fixtures/datacenter.rs` | test-only scripts/catalog, registered through testkit |
| `packages/engine/test/fixtures/field.ts` | 301 | port | `crates/engine/tests/rules/fixtures/field.rs` | test-only scripts/catalog, registered through testkit |
| `packages/engine/test/fixtures/fruit.ts` | 71 | port | `crates/engine/tests/rules/fixtures/fruit.rs` | test-only scripts/catalog, registered through testkit |
| `packages/engine/test/fixtures/generation.ts` | 428 | port | `crates/engine/tests/rules/fixtures/generation.rs` | test-only scripts/catalog, registered through testkit |
| `packages/engine/test/fixtures/harness.ts` | 107 | port | `crates/engine/tests/rules/fixtures/harness.rs` | test-only scripts/catalog, registered through testkit |
| `packages/engine/test/fixtures/instanceData.ts` | 361 | port | `crates/engine/tests/rules/fixtures/instance_data.rs` | test-only scripts/catalog, registered through testkit |
| `packages/engine/test/fixtures/killCredit.ts` | 96 | port | `crates/engine/tests/rules/fixtures/kill_credit.rs` | test-only scripts/catalog, registered through testkit |
| `packages/engine/test/fixtures/kyTest.ts` | 63 | port | `crates/engine/tests/rules/fixtures/ky_test.rs` | test-only scripts/catalog, registered through testkit |
| `packages/engine/test/fixtures/lastBoards.ts` | 78 | port | `crates/engine/tests/rules/fixtures/last_boards.rs` | test-only scripts/catalog, registered through testkit |
| `packages/engine/test/fixtures/papaya.ts` | 70 | port | `crates/engine/tests/rules/fixtures/papaya.rs` | test-only scripts/catalog, registered through testkit |
| `packages/engine/test/fixtures/playPipelineA.ts` | 352 | port | `crates/engine/tests/rules/fixtures/play_pipeline_a.rs` | test-only scripts/catalog, registered through testkit |
| `packages/engine/test/fixtures/playPipelineB.ts` | 384 | port | `crates/engine/tests/rules/fixtures/play_pipeline_b.rs` | test-only scripts/catalog, registered through testkit |
| `packages/engine/test/fixtures/promptHarness.ts` | 125 | port | `crates/engine/tests/rules/fixtures/prompt_harness.rs` | test-only scripts/catalog, registered through testkit |
| `packages/engine/test/fixtures/prompts.ts` | 649 | port | `crates/engine/tests/rules/fixtures/prompts.rs` | test-only scripts/catalog, registered through testkit |
| `packages/engine/test/fixtures/quests.ts` | 266 | port | `crates/engine/tests/rules/fixtures/quests.rs` | test-only scripts/catalog, registered through testkit |
| `packages/engine/test/fixtures/rng-child.ts` | 9 | port | `crates/engine/tests/rules/fixtures/rng_child.rs` | test-only scripts/catalog, registered through testkit |
| `packages/engine/test/fixtures/scripts.ts` | 253 | port | `crates/engine/tests/rules/fixtures/scripts.rs` | test-only scripts/catalog, registered through testkit |
| `packages/engine/test/fixtures/turn.ts` | 285 | port | `crates/engine/tests/rules/fixtures/turn.rs` | test-only scripts/catalog, registered through testkit |
| `packages/engine/test/fixtures/twiceForward.ts` | 67 | port | `crates/engine/tests/rules/fixtures/twice_forward.rs` | test-only scripts/catalog, registered through testkit |

## Part 25: engine tests 2: rulings, subsystems, prompts and triggers

| TypeScript | Lines | Action | Rust | Note |
|---|---|---|---|---|
| `packages/engine/test/activate.test.ts` | 827 | port | `crates/engine/tests/rules/activate.rs` |  |
| `packages/engine/test/aiPolicy.test.ts` | 256 | port | `crates/engine/tests/rules/ai_policy.rs` |  |
| `packages/engine/test/audit.test.ts` | 99 | port | `crates/engine/tests/rules/audit.rs` |  |
| `packages/engine/test/boardHistory.test.ts` | 317 | port | `crates/engine/tests/rules/board_history.rs` |  |
| `packages/engine/test/callToChaos.test.ts` | 682 | port | `crates/engine/tests/rules/call_to_chaos.rs` |  |
| `packages/engine/test/callToChaosPlus.test.ts` | 339 | port | `crates/engine/tests/rules/call_to_chaos_plus.rs` |  |
| `packages/engine/test/comboIndex.test.ts` | 522 | port | `crates/engine/tests/rules/combo_index.rs` |  |
| `packages/engine/test/copied-text.test.ts` | 353 | port | `crates/engine/tests/rules/copied_text.rs` |  |
| `packages/engine/test/corePatches.test.ts` | 390 | port | `crates/engine/tests/rules/core_patches.rs` |  |
| `packages/engine/test/death-pause.test.ts` | 504 | port | `crates/engine/tests/rules/death_pause.rs` |  |
| `packages/engine/test/delayed-kinds.test.ts` | 270 | port | `crates/engine/tests/rules/delayed_kinds.rs` |  |
| `packages/engine/test/fuse-registry.test.ts` | 196 | port | `crates/engine/tests/rules/fuse_registry.rs` |  |
| `packages/engine/test/fuse-variants.test.ts` | 540 | port | `crates/engine/tests/rules/fuse_variants.rs` |  |
| `packages/engine/test/fuse.test.ts` | 615 | port | `crates/engine/tests/rules/fuse.rs` |  |
| `packages/engine/test/glitch.test.ts` | 218 | port | `crates/engine/tests/rules/glitch.rs` |  |
| `packages/engine/test/heroPower.test.ts` | 528 | port | `crates/engine/tests/rules/hero_power.rs` |  |
| `packages/engine/test/kyTest.test.ts` | 282 | port | `crates/engine/tests/rules/ky_test.rs` |  |
| `packages/engine/test/lastBoards.test.ts` | 283 | port | `crates/engine/tests/rules/last_boards.rs` |  |
| `packages/engine/test/modifiers.test.ts` | 573 | port | `crates/engine/tests/rules/modifiers.rs` |  |
| `packages/engine/test/papaya.test.ts` | 420 | port | `crates/engine/tests/rules/papaya.rs` |  |
| `packages/engine/test/pauses.test.ts` | 589 | port | `crates/engine/tests/rules/pauses.rs` |  |
| `packages/engine/test/perfectHand.test.ts` | 257 | port | `crates/engine/tests/rules/perfect_hand.rs` |  |
| `packages/engine/test/prompt-kinds.test.ts` | 601 | port | `crates/engine/tests/rules/prompt_kinds.rs` |  |
| `packages/engine/test/prompts.test.ts` | 909 | port | `crates/engine/tests/rules/prompts.rs` |  |
| `packages/engine/test/quests.test.ts` | 658 | port | `crates/engine/tests/rules/quests.rs` |  |
| `packages/engine/test/rulings-a.test.ts` | 1385 | port | `crates/engine/tests/rules/rulings_a.rs` |  |
| `packages/engine/test/rulings-b.test.ts` | 1682 | port | `crates/engine/tests/rules/rulings_b.rs` |  |
| `packages/engine/test/rulings-c.test.ts` | 2539 | port | `crates/engine/tests/rules/rulings_c.rs` |  |
| `packages/engine/test/scorer.test.ts` | 264 | port | `crates/engine/tests/rules/scorer.rs` |  |
| `packages/engine/test/start-of-opponent-turn.test.ts` | 77 | port | `crates/engine/tests/rules/start_of_opponent_turn.rs` |  |
| `packages/engine/test/statecheck.test.ts` | 641 | port | `crates/engine/tests/rules/statecheck.rs` |  |
| `packages/engine/test/stays.test.ts` | 98 | port | `crates/engine/tests/rules/stays.rs` |  |
| `packages/engine/test/trap-cardresolved.test.ts` | 261 | port | `crates/engine/tests/rules/trap_cardresolved.rs` |  |
| `packages/engine/test/trap-window-pause.test.ts` | 349 | port | `crates/engine/tests/rules/trap_window_pause.rs` |  |
| `packages/engine/test/trigger-zones.test.ts` | 366 | port | `crates/engine/tests/rules/trigger_zones.rs` |  |
| `packages/engine/test/triggers.test.ts` | 432 | port | `crates/engine/tests/rules/triggers.rs` |  |
| `packages/engine/test/twiceForward.test.ts` | 255 | port | `crates/engine/tests/rules/twice_forward.rs` |  |

## Part 26: engine tests 3: view, turn, setup, combat

| TypeScript | Lines | Action | Rust | Note |
|---|---|---|---|---|
| `packages/engine/test/after-attack.test.ts` | 135 | port | `crates/engine/tests/rules/after_attack.rs` |  |
| `packages/engine/test/animated.test.ts` | 473 | port | `crates/engine/tests/rules/animated.rs` |  |
| `packages/engine/test/auto-end-turn.test.ts` | 161 | port | `crates/engine/tests/rules/auto_end_turn.rs` |  |
| `packages/engine/test/backrow-death.test.ts` | 73 | port | `crates/engine/tests/rules/backrow_death.rs` |  |
| `packages/engine/test/backrow-piles.test.ts` | 438 | port | `crates/engine/tests/rules/backrow_piles.rs` |  |
| `packages/engine/test/brittle.test.ts` | 395 | port | `crates/engine/tests/rules/brittle.rs` |  |
| `packages/engine/test/carried-damage.test.ts` | 66 | port | `crates/engine/tests/rules/carried_damage.rs` |  |
| `packages/engine/test/combat-positions.test.ts` | 347 | port | `crates/engine/tests/rules/combat_positions.rs` |  |
| `packages/engine/test/combat-resolution.test.ts` | 303 | port | `crates/engine/tests/rules/combat_resolution.rs` |  |
| `packages/engine/test/combat-validation.test.ts` | 428 | port | `crates/engine/tests/rules/combat_validation.rs` |  |
| `packages/engine/test/combat.property.test.ts` | 465 | port | `crates/engine/tests/rules/combat_property.rs` |  |
| `packages/engine/test/conditionActive.test.ts` | 825 | port | `crates/engine/tests/rules/condition_active.rs` | drop l.770–825 (the R195 prose block, #133); keep the behaviour tests |
| `packages/engine/test/config.test.ts` | 54 | port | `crates/engine/tests/rules/config.rs` |  |
| `packages/engine/test/control-change.property.test.ts` | 309 | port | `crates/engine/tests/rules/control_change_property.rs` |  |
| `packages/engine/test/control-change.test.ts` | 477 | port | `crates/engine/tests/rules/control_change.rs` |  |
| `packages/engine/test/damage-pipeline.test.ts` | 180 | port | `crates/engine/tests/rules/damage_pipeline.rs` |  |
| `packages/engine/test/damage.test.ts` | 476 | port | `crates/engine/tests/rules/damage.rs` |  |
| `packages/engine/test/destroyed-face.test.ts` | 32 | port | `crates/engine/tests/rules/destroyed_face.rs` |  |
| `packages/engine/test/endgame.test.ts` | 177 | port | `crates/engine/tests/rules/endgame.rs` |  |
| `packages/engine/test/faces.test.ts` | 145 | port | `crates/engine/tests/rules/faces.rs` |  |
| `packages/engine/test/game-summary.test.ts` | 277 | port | `crates/engine/tests/rules/game_summary.rs` |  |
| `packages/engine/test/generation-replay.test.ts` | 122 | port | `crates/engine/tests/rules/generation_replay.rs` |  |
| `packages/engine/test/glow-facts.test.ts` | 218 | port | `crates/engine/tests/rules/glow_facts.rs` |  |
| `packages/engine/test/handicap.test.ts` | 1303 | port | `crates/engine/tests/rules/handicap.rs` |  |
| `packages/engine/test/hotseat.smoke.test.ts` | 32 | port | `crates/engine/tests/rules/hotseat_smoke.rs` |  |
| `packages/engine/test/instance-data.test.ts` | 262 | port | `crates/engine/tests/rules/instance_data.rs` |  |
| `packages/engine/test/kill-credit.test.ts` | 111 | port | `crates/engine/tests/rules/kill_credit.rs` |  |
| `packages/engine/test/layers.test.ts` | 682 | port | `crates/engine/tests/rules/layers.rs` |  |
| `packages/engine/test/lethal.test.ts` | 206 | port | `crates/engine/tests/rules/lethal.rs` |  |
| `packages/engine/test/library-copies.test.ts` | 119 | port | `crates/engine/tests/rules/library_copies.rs` |  |
| `packages/engine/test/mulligan-concurrent.test.ts` | 339 | port | `crates/engine/tests/rules/mulligan_concurrent.rs` |  |
| `packages/engine/test/ownLibrary.test.ts` | 253 | port | `crates/engine/tests/rules/own_library.rs` |  |
| `packages/engine/test/params.test.ts` | 190 | port | `crates/engine/tests/rules/params.rs` |  |
| `packages/engine/test/pools.test.ts` | 172 | port | `crates/engine/tests/rules/pools.rs` |  |
| `packages/engine/test/preview-ids.test.ts` | 97 | port | `crates/engine/tests/rules/preview_ids.rs` |  |
| `packages/engine/test/preview.test.ts` | 616 | port | `crates/engine/tests/rules/preview.rs` |  |
| `packages/engine/test/query.test.ts` | 372 | port | `crates/engine/tests/rules/query.rs` |  |
| `packages/engine/test/recruit-variants.test.ts` | 232 | port | `crates/engine/tests/rules/recruit_variants.rs` |  |
| `packages/engine/test/reduce.test.ts` | 185 | port | `crates/engine/tests/rules/reduce.rs` |  |
| `packages/engine/test/replacements.test.ts` | 685 | port | `crates/engine/tests/rules/replacements.rs` |  |
| `packages/engine/test/replay-scripted.test.ts` | 414 | port | `crates/engine/tests/rules/replay_scripted.rs` |  |
| `packages/engine/test/replay.test.ts` | 33 | port | `crates/engine/tests/rules/replay.rs` |  |
| `packages/engine/test/restrictions.test.ts` | 262 | port | `crates/engine/tests/rules/restrictions.rs` |  |
| `packages/engine/test/rng.test.ts` | 98 | port | `crates/engine/tests/rules/rng.rs` |  |
| `packages/engine/test/rotation.test.ts` | 366 | port | `crates/engine/tests/rules/rotation.rs` |  |
| `packages/engine/test/rounds.test.ts` | 143 | port | `crates/engine/tests/rules/rounds.rs` |  |
| `packages/engine/test/self-tribute.test.ts` | 132 | port | `crates/engine/tests/rules/self_tribute.rs` |  |
| `packages/engine/test/setup-aside.test.ts` | 524 | port | `crates/engine/tests/rules/setup_aside.rs` |  |
| `packages/engine/test/setup.test.ts` | 237 | port | `crates/engine/tests/rules/setup.rs` |  |
| `packages/engine/test/shuffle-random.test.ts` | 98 | port | `crates/engine/tests/rules/shuffle_random.rs` |  |
| `packages/engine/test/state.test.ts` | 77 | port | `crates/engine/tests/rules/state.rs` |  |
| `packages/engine/test/targeting.test.ts` | 465 | port | `crates/engine/tests/rules/targeting.rs` |  |
| `packages/engine/test/temporary.test.ts` | 107 | port | `crates/engine/tests/rules/temporary.rs` |  |
| `packages/engine/test/transform-variants.test.ts` | 194 | port | `crates/engine/tests/rules/transform_variants.rs` |  |
| `packages/engine/test/tribute-zones.test.ts` | 199 | port | `crates/engine/tests/rules/tribute_zones.rs` |  |
| `packages/engine/test/tribute.test.ts` | 569 | port | `crates/engine/tests/rules/tribute.rs` |  |
| `packages/engine/test/turn-cap.test.ts` | 65 | port | `crates/engine/tests/rules/turn_cap.rs` |  |
| `packages/engine/test/turn-wiring.test.ts` | 359 | port | `crates/engine/tests/rules/turn_wiring.rs` |  |
| `packages/engine/test/turn.test.ts` | 514 | port | `crates/engine/tests/rules/turn.rs` |  |
| `packages/engine/test/view-marks.test.ts` | 173 | port | `crates/engine/tests/rules/view_marks.rs` |  |
| `packages/engine/test/viewFor.test.ts` | 818 | port | `crates/engine/tests/rules/view_for.rs` |  |
| `packages/engine/test/windfury.test.ts` | 151 | port | `crates/engine/tests/rules/windfury.rs` |  |
| `packages/engine/test/zones.test.ts` | 232 | port | `crates/engine/tests/rules/zones.rs` |  |

## Part 27: engine tests 4: play pipeline and cross-card rules

| TypeScript | Lines | Action | Rust | Note |
|---|---|---|---|---|
| `packages/cards/test/after-resolution.test.ts` | 420 | port | `crates/cards/tests/cross/after_resolution.rs` |  |
| `packages/cards/test/combat-windows.test.ts` | 852 | port | `crates/cards/tests/cross/combat_windows.rs` |  |
| `packages/cards/test/condition-active.test.ts` | 959 | port | `crates/cards/tests/cross/condition_active.rs` |  |
| `packages/cards/test/control-change-carry.test.ts` | 104 | port | `crates/cards/tests/cross/control_change_carry.rs` |  |
| `packages/cards/test/control-change.test.ts` | 599 | port | `crates/cards/tests/cross/control_change.rs` |  |
| `packages/cards/test/costs-and-mana.test.ts` | 225 | port | `crates/cards/tests/cross/costs_and_mana.rs` |  |
| `packages/cards/test/deaths-and-reborn.test.ts` | 482 | port | `crates/cards/tests/cross/deaths_and_reborn.rs` |  |
| `packages/cards/test/echo-and-exile.test.ts` | 230 | port | `crates/cards/tests/cross/echo_and_exile.rs` |  |
| `packages/cards/test/forced-attacks.test.ts` | 141 | port | `crates/cards/tests/cross/forced_attacks.rs` |  |
| `packages/cards/test/fuse-registry.test.ts` | 74 | port | `crates/cards/tests/cross/fuse_registry.rs` |  |
| `packages/cards/test/fused-hooks.test.ts` | 637 | port | `crates/cards/tests/cross/fused_hooks.rs` |  |
| `packages/cards/test/fused-nested-resume.test.ts` | 53 | port | `crates/cards/tests/cross/fused_nested_resume.rs` |  |
| `packages/cards/test/fused-target-checks.test.ts` | 59 | port | `crates/cards/tests/cross/fused_target_checks.rs` |  |
| `packages/cards/test/game-over.test.ts` | 122 | port | `crates/cards/tests/cross/game_over.rs` |  |
| `packages/cards/test/hand-returns.test.ts` | 321 | port | `crates/cards/tests/cross/hand_returns.rs` |  |
| `packages/cards/test/hidden-information.test.ts` | 1474 | port | `crates/cards/tests/cross/hidden_information.rs` |  |
| `packages/cards/test/lasting-effects.test.ts` | 222 | port | `crates/cards/tests/cross/lasting_effects.rs` |  |
| `packages/cards/test/my-pawn.test.ts` | 387 | port | `crates/cards/tests/cross/my_pawn.rs` |  |
| `packages/cards/test/paused-sequences.test.ts` | 2175 | port | `crates/cards/tests/cross/paused_sequences.rs` |  |
| `packages/cards/test/play-choices.test.ts` | 363 | port | `crates/cards/tests/cross/play_choices.rs` |  |
| `packages/cards/test/plays-and-casts.test.ts` | 342 | port | `crates/cards/tests/cross/plays_and_casts.rs` |  |
| `packages/cards/test/preview.test.ts` | 1383 | port | `crates/cards/tests/cross/preview.rs` |  |
| `packages/cards/test/re-entry.test.ts` | 964 | port | `crates/cards/tests/cross/re_entry.rs` |  |
| `packages/cards/test/resolving-face.test.ts` | 286 | port | `crates/cards/tests/cross/resolving_face.rs` |  |
| `packages/cards/test/self-generation.test.ts` | 162 | port | `crates/cards/tests/cross/self_generation.rs` |  |
| `packages/cards/test/setup-and-mulligan.test.ts` | 540 | port | `crates/cards/tests/cross/setup_and_mulligan.rs` |  |
| `packages/cards/test/stacks-and-reborn.test.ts` | 369 | port | `crates/cards/tests/cross/stacks_and_reborn.rs` |  |
| `packages/cards/test/tributes.test.ts` | 371 | port | `crates/cards/tests/cross/tributes.rs` |  |
| `packages/cards/test/trigger-stays.test.ts` | 606 | port | `crates/cards/tests/cross/trigger_stays.rs` |  |
| `packages/cards/test/turn-clock-and-legality.test.ts` | 481 | port | `crates/cards/tests/cross/turn_clock_and_legality.rs` |  |
| `packages/cards/test/turn-stages.test.ts` | 689 | port | `crates/cards/tests/cross/turn_stages.rs` |  |
| `packages/cards/test/vanilla-and-positions.test.ts` | 266 | port | `crates/cards/tests/cross/vanilla_and_positions.rs` |  |
| `packages/engine/test/announce.test.ts` | 488 | port | `crates/engine/tests/rules/announce.rs` |  |
| `packages/engine/test/cost-rules.test.ts` | 322 | port | `crates/engine/tests/rules/cost_rules.rs` |  |
| `packages/engine/test/counterWarning.test.ts` | 140 | port | `crates/engine/tests/rules/counter_warning.rs` |  |
| `packages/engine/test/draw-complete.test.ts` | 180 | port | `crates/engine/tests/rules/draw_complete.rs` |  |
| `packages/engine/test/draw-limit.test.ts` | 322 | port | `crates/engine/tests/rules/draw_limit.rs` |  |
| `packages/engine/test/draw-pause.test.ts` | 421 | port | `crates/engine/tests/rules/draw_pause.rs` |  |
| `packages/engine/test/draw.test.ts` | 195 | port | `crates/engine/tests/rules/draw.rs` |  |
| `packages/engine/test/echo.test.ts` | 411 | port | `crates/engine/tests/rules/echo.rs` |  |
| `packages/engine/test/graveyard-play.test.ts` | 338 | port | `crates/engine/tests/rules/graveyard_play.rs` |  |
| `packages/engine/test/mana-before-play.test.ts` | 95 | port | `crates/engine/tests/rules/mana_before_play.rs` |  |
| `packages/engine/test/mana.test.ts` | 183 | port | `crates/engine/tests/rules/mana.rs` |  |
| `packages/engine/test/overflow-events.test.ts` | 297 | port | `crates/engine/tests/rules/overflow_events.rs` |  |
| `packages/engine/test/play-pipeline-b-replay.test.ts` | 141 | port | `crates/engine/tests/rules/play_pipeline_b_replay.rs` |  |
| `packages/engine/test/play-step3.test.ts` | 240 | port | `crates/engine/tests/rules/play_step3.rs` |  |
| `packages/engine/test/playChoices-filters.test.ts` | 703 | port | `crates/engine/tests/rules/play_choices_filters.rs` |  |
| `packages/engine/test/playChoices.test.ts` | 464 | port | `crates/engine/tests/rules/play_choices.rs` |  |
| `packages/engine/test/playCounts.test.ts` | 177 | port | `crates/engine/tests/rules/play_counts.rs` |  |

## Part 28: the spec graph and structural spec checks (#133)

| TypeScript | Lines | Action | Rust | Note |
|---|---|---|---|---|
| `packages/engine/test/rulings.test.ts` | 4565 | split | `crates/engine/tests/rules/rulings_config.rs` | only the per-row `expect(config.X).toBe(…)` asserts, as `fn r<n>_…`; the provenIn index becomes spec/rulings/*.md `proven_in`; the 23 source-regex pins are dropped (#133) |

## Part 29: the training arena, the promotion gate and the lane prompts

| TypeScript | Lines | Action | Rust | Note |
|---|---|---|---|---|
| `packages/ai/scripts/duel.ts` | 129 | port | `crates/tools/src/arena.rs` |  |

## Part 37: cull: delete the TypeScript, rewrite the docs

| TypeScript | Lines | Action | Rust | Note |
|---|---|---|---|---|
| `apps/server/README.md` | 496 | delete | — | README content moves to crates/server/README.md (part 37) |
| `apps/server/package.json` | 37 | delete | — |  |
| `apps/server/test/validator-single-source.test.ts` | 245 | delete | — | one crate is one source |
| `apps/server/tsconfig.json` | 18 | delete | — |  |
| `apps/server/vitest.config.ts` | 8 | delete | — |  |
| `ladder/DECISIONS.md` | 18 | delete | — | the Rust arena and training lanes replace it (part 29) |
| `ladder/README.md` | 132 | delete | — | the Rust arena and training lanes replace it (part 29) |
| `ladder/agents/__init__.py` | 0 | delete | — | the Rust arena and training lanes replace it (part 29) |
| `ladder/agents/random/__init__.py` | 0 | delete | — | the Rust arena and training lanes replace it (part 29) |
| `ladder/agents/random/agent.py` | 33 | delete | — | the Rust arena and training lanes replace it (part 29) |
| `ladder/arena/__init__.py` | 0 | delete | — | the Rust arena and training lanes replace it (part 29) |
| `ladder/arena/bridge.py` | 83 | delete | — | the Rust arena and training lanes replace it (part 29) |
| `ladder/arena/runner.py` | 187 | delete | — | the Rust arena and training lanes replace it (part 29) |
| `ladder/arena/types.py` | 24 | delete | — | the Rust arena and training lanes replace it (part 29) |
| `ladder/audit/__init__.py` | 0 | delete | — | the Rust arena and training lanes replace it (part 29) |
| `ladder/audit/gates.py` | 63 | delete | — | the Rust arena and training lanes replace it (part 29) |
| `ladder/audit/shadowban.py` | 124 | delete | — | the Rust arena and training lanes replace it (part 29) |
| `ladder/champions/1/.gitkeep` | 0 | delete | — | the Rust arena and training lanes replace it (part 29) |
| `ladder/champions/2/.gitkeep` | 0 | delete | — | the Rust arena and training lanes replace it (part 29) |
| `ladder/common.py` | 18 | delete | — | the Rust arena and training lanes replace it (part 29) |
| `ladder/config.yaml` | 16 | delete | — | the Rust arena and training lanes replace it (part 29) |
| `ladder/hall_of_fame/.gitkeep` | 0 | delete | — | the Rust arena and training lanes replace it (part 29) |
| `ladder/history/.gitkeep` | 0 | delete | — | the Rust arena and training lanes replace it (part 29) |
| `ladder/llm/__init__.py` | 0 | delete | — | the Rust arena and training lanes replace it (part 29) |
| `ladder/llm/client.py` | 74 | delete | — | the Rust arena and training lanes replace it (part 29) |
| `ladder/llm/providers/__init__.py` | 0 | delete | — | the Rust arena and training lanes replace it (part 29) |
| `ladder/llm/providers/anthropic.py` | 21 | delete | — | the Rust arena and training lanes replace it (part 29) |
| `ladder/llm/providers/base.py` | 49 | delete | — | the Rust arena and training lanes replace it (part 29) |
| `ladder/llm/providers/google.py` | 44 | delete | — | the Rust arena and training lanes replace it (part 29) |
| `ladder/llm/providers/mock.py` | 44 | delete | — | the Rust arena and training lanes replace it (part 29) |
| `ladder/llm/providers/openai.py` | 25 | delete | — | the Rust arena and training lanes replace it (part 29) |
| `ladder/prompts/proposer.md` | 52 | delete | — | the Rust arena and training lanes replace it (part 29) |
| `ladder/proposals/.gitkeep` | 0 | delete | — | the Rust arena and training lanes replace it (part 29) |
| `ladder/proposer/__init__.py` | 0 | delete | — | the Rust arena and training lanes replace it (part 29) |
| `ladder/proposer/propose.py` | 220 | delete | — | the Rust arena and training lanes replace it (part 29) |
| `ladder/pyproject.toml` | 16 | delete | — | the Rust arena and training lanes replace it (part 29) |
| `ladder/schemas/spec.schema.json` | 68 | delete | — | the Rust arena and training lanes replace it (part 29) |
| `ladder/tests/__init__.py` | 0 | delete | — | the Rust arena and training lanes replace it (part 29) |
| `ladder/tests/test_arena.py` | 117 | delete | — | the Rust arena and training lanes replace it (part 29) |
| `ladder/tests/test_gates.py` | 72 | delete | — | the Rust arena and training lanes replace it (part 29) |
| `ladder/tests/test_llm.py` | 68 | delete | — | the Rust arena and training lanes replace it (part 29) |
| `ladder/tests/test_proposer.py` | 105 | delete | — | the Rust arena and training lanes replace it (part 29) |
| `ladder/tests/test_shadowban.py` | 102 | delete | — | the Rust arena and training lanes replace it (part 29) |
| `packages/ai/README.md` | 253 | delete | — |  |
| `packages/ai/package.json` | 20 | delete | — |  |
| `packages/ai/scripts/arena-bridge.ts` | 153 | delete | — | the Rust arena replaces the Node bridge |
| `packages/ai/scripts/bench.ts` | 124 | delete | — | superseded by `arena` |
| `packages/ai/scripts/oracle.ts` | 72 | delete | — | dev tool, not ported |
| `packages/ai/test/_shard.ts` | 62 | delete | — |  |
| `packages/ai/test/arena-bridge.test.ts` | 98 | delete | — |  |
| `packages/ai/test/setup.ts` | 7 | delete | — |  |
| `packages/ai/tsconfig.json` | 9 | delete | — |  |
| `packages/ai/vitest.config.ts` | 13 | delete | — |  |
| `packages/cards/README.md` | 611 | delete | — | README content moves to crates/cards/README.md (part 37) |
| `packages/cards/package.json` | 21 | delete | — |  |
| `packages/cards/scripts/gen-loc.ts` | 155 | delete | — | loc frozen; `cargo jackioh catalog loc` prints a new card's count |
| `packages/cards/scripts/gen-registry.ts` | 203 | delete | — | build.rs (SURFACE §7.4) |
| `packages/cards/scripts/missing-tests.ts` | 265 | delete | — | build.rs (SURFACE §7.4) |
| `packages/cards/src/scripts/_generated.ts` | 654 | delete | — | build.rs generates the registry (SURFACE §7.4) |
| `packages/cards/test/globalSetup.ts` | 36 | delete | — | registry generation moved to build.rs |
| `packages/cards/test/loc.test.ts` | 64 | delete | — | loc is frozen data (SURFACE §7.5) |
| `packages/cards/tsconfig.json` | 8 | delete | — |  |
| `packages/cards/vitest.config.ts` | 11 | delete | — |  |
| `packages/engine/package.json` | 14 | delete | — |  |
| `packages/engine/scripts/rulings-coverage.ts` | 140 | delete | — | replaced by `cargo jackioh spec check` (part 28) |
| `packages/engine/test/fixtures/lint/date-now.ts` | 2 | delete | — | ESLint probe; clippy.toml replaces the lint (SURFACE §3) |
| `packages/engine/test/fixtures/lint/math-random.ts` | 2 | delete | — | ESLint probe; clippy.toml replaces the lint (SURFACE §3) |
| `packages/engine/test/fixtures/lint/new-date.ts` | 2 | delete | — | ESLint probe; clippy.toml replaces the lint (SURFACE §3) |
| `packages/engine/test/lint-ban.test.ts` | 76 | delete | — | tests the ESLint purity rule |
| `packages/engine/tsconfig.json` | 4 | delete | — |  |
| `packages/engine/vitest.config.ts` | 8 | delete | — |  |
| `packages/shared/package.json` | 9 | delete | — |  |
| `packages/shared/test/events.test.ts` | 78 | delete | — | reads its own source; the generated TS types replace it |
| `packages/shared/vitest.config.ts` | 8 | delete | — |  |
| `packages/validator/package.json` | 13 | delete | — |  |
| `packages/validator/src/config.ts` | 14 | delete | — | re-exported DECK_SIZE/MAX_COPIES; use crate::config |
| `packages/validator/tsconfig.json` | 4 | delete | — |  |
| `packages/validator/vitest.config.ts` | 8 | delete | — |  |
| `scripts/catalog-version.mjs` | 35 | delete | — | the server compiles the version in; `cargo jackioh catalog-version` for scripts |
