# `jackioh-cards`

The catalog entries of the seven shipped sets, Core (spec §8.1–§8.5), Classic (§8.6, and Glitch, a
hidden token), Classic+ (§8.7), Meditative (§8.8), Meditative+ (§8.9, shipped in v0.3.15 by #1187),
Stinkyscape (§8.10, the Dormant Starship among its cards, shipped by #1884, R1857000) and Training
Grounds (§8.11, shipped by #2204, R2194000), each set's counts written once in `src/census.rs` (#2581).
A set the catalog holds before it ships, as Meditative was while issue #496 built it, Meditative+
while issue #1173 did (R1178000), Stinkyscape while issue #1836 did, and Training Grounds while tracker
#2193 did, is in no pool that names no set, no deck and no random deck until the patch that lists it in `SHIPPED_SETS`
(R1420), counts toward none of the census's totals, and a test opens it with the testkit's `preview_sets`; one script file per entry with its tests inside it; the catalog query every random pool goes through; and the
catalog's patch history. This file is the contract between the card files. The spec (`spec/08-catalog.md`
for every card, `spec/07-tokens.md` for the tokens) is the only source of card text; when it and this
file disagree, the spec wins. [`docs/ADDING_CARDS.md`](../../docs/ADDING_CARDS.md) is the procedure
for adding, changing or removing a card (CLAUDE.md rule 10).

```
crates/cards
├── catalog.json        the card data, proved against spec §8 by tests/cross/catalog.rs
├── card-text.yaml      every card's words but its rules, edited by hand: flavour, artist, voice lines,
│                       Chinese, and the portraits' emote lines (#2498); not card data
├── flavour.json        flavour lines and artist credits (R660), generated from card-text.yaml
├── chinese.json        Simplified Chinese names and faces (R1303), generated from card-text.yaml
├── chinese-terms.json  the frame's Chinese words: types, tags, rarities, keywords, labels (R1303)
├── patches/            the catalog's history (R388, R646): patches.json, one snapshot per patch,
│                       win_rates/ (each patch's win-rate table, R1329000), shipped.json, index.json,
│                       and pending/ (one fragment per patch being built)
├── build.rs            generates the registry from src/scripts/**, compiles the catalog version in,
│                       cuts every card's code block from its script (R1183004), and counts every
│                       script's code lines into the BUSTED and complicated lists (R1846000)
├── clippy.toml         the purity lint (CLAUDE.md rule 4), the engine's file
├── src/lib.rs          CATALOG, CATALOG_IDS, CATALOG_VERSION, card_def, register_all, the registry
├── src/query.rs        spec §5.1's catalog.query, the only random-pool source
├── src/census.rs       each set's cards, tokens, rarities and tag counts (#2581)
├── src/ky_test_bank.rs C+ #42 KY's Test's problems (R420)
├── src/scripts/
│   ├── core/           one Core card or token per file
│   ├── classic/        one Classic card per file
│   ├── classic_plus/   one Classic+ card or token per file
│   ├── meditative/     one Meditative card or token per file (shipped in v0.3.5, R1420)
│   ├── meditative_plus/ one Meditative+ card or token per file (shipped in v0.3.15, R1178000)
│   ├── stinkyscape/    one Stinkyscape card or token per file (shipped by #1884, R1857000)
│   └── training_grounds/ one Training Grounds card or token per file (shipped by #2204, R2194000)
└── tests/cards.rs      the cross-card tests: tests/cross/<x>.rs (catalog, query, references, radiant
                        standard, flavour, condition_active, preview, invariants, …)
```

## 1. The card file

**One card, one file, its tests inside it** (CLAUDE.md rule 6). The file is
`src/scripts/<set>/<name>.rs`: the spec §5 index in three digits with a `c` in front, then the slug,
every `-` and `.` an `_`. The build holds the folder and the number, not the slug (§4): a few slugs
differ from the card's name on purpose (`core/c097_zephyrs.rs` for Zephrys).

| Card | File |
| --- | --- |
| #2 Bigot (`core-002`) | `src/scripts/core/c002_bigot.rs` |
| #51.1 KY's Empty Notebook (`core-051-1`) | `src/scripts/core/c051_1_kys_empty_notebook.rs` |
| Rush Token (`core-t-rush`) | `src/scripts/core/t_rush.rs` |
| C #43 Plague Nuke (`classic-043`) | `src/scripts/classic/c043_plague_nuke.rs` |
| C+ #12.1 Devour (`classicplus-012-1`) | `src/scripts/classic_plus/c012_1_devour.rs` |
| T-AI-1 Helpful Assistant (`classicplus-t-ai-01`) | `src/scripts/classic_plus/t_ai_01_helpful_assistant.rs` |

A card's id is its set and its index (`core-043`, `classic-043`, `classicplus-043`); a token a card
defines adds its number (`core-051-1`), a shared token is named (`core-t-rush`). An index is unique
only within its set, so look a card up by id, or by `(set, index)` with `card_def_by_index`.

The file names its id and builds both faces:

```rust
//! SPEC §8.1 #2 Bigot — 6/1 → 12/2 Unit, Human, cost 2. (what the card does, and why)

use jackioh_engine::effects::{destroy, destroy_all};
use jackioh_engine::prelude::*;

pub const ID: &str = "core-002";

pub fn script() -> CardScripts {
    let base = Script {
        targets: vec![TargetDecl::target(1, 1, json!({ "side": "enemy", "of": ["unit"], "notTags": ["Human"] }))],
        cry: Some(hook(|_ctx| vec![destroy(json_as(json!({ "target": { "of": "chosen" } })))])),
        ..Script::default()
    };
    let radiant = Script {
        cry: Some(hook(|_ctx| vec![destroy_all(json_as(json!({ "side": "enemy", "rows": ["units"], "notTags": ["Human"] })))])),
        ..Script::default()
    };
    CardScripts { base, radiant }
}

#[cfg(test)]
mod tests {
    use jackioh_engine::testkit::*;

    #[test]
    fn destroys_the_chosen_enemy_non_human_unit() { /* … */ }
}
```

- **`pub const ID` is mandatory.** `build.rs` reads that line. The definition comes from the
  catalog by `ID`; never restate stats, cost, tags or rarity in a script.
- **Both faces are required.** A card whose Radiant face differs only in what the engine reads off
  the catalog (stats, keywords, a config multiple) runs the same script on both:
  `CardScripts { radiant: base.clone(), base }`, and says so in a comment.
- **`Script`** (`crates/engine/src/script.rs`, spec §10.9) is a struct of optional hooks and
  declarations: `cost`, `cry`, `death`, `start_of_game`, `enters_hand`, `resume`, `delayed`, `set_stat`,
  `start_of_turn`, `end_of_turn`, `aura`, `triggers`, `on_play_hook`, `hand_triggers`,
  `static_flags`, `targets`, `modes`, `condition_met`, `preview`, `activations` and the rest; fill
  what the card needs and end with `..Script::default()`. A Spell's resolution hangs off `cry`.
  `resume` is the named continuation a prompt answer re-enters (R113), `delayed` the hook a
  scheduled effect lands on (R126), `set_stat` layer 2's stat hook (R116), `activations` a card's
  Activate abilities (R384).
- **`condition_met`** is R195's yellow glow: a pure read of the card's printed condition now, built
  on the same local predicate its own resolution uses. It never writes, never draws from the rng and
  never reads `state.active` (it reads `your_turn`).
- **`preview`** is R280's number a formula comes to now: `{ label, value }` pairs, each label an
  exact substring of the running face's text, each value what the card's own resolution would deal
  or gain. It reads only public facts and the card's own face and counters, never a library's
  contents or a hand's cards. A declared number (`params`) is not a preview. R2476000: a card that
  remembers something shows it the same way, a count through `count_preview`, a remembered card by
  name through `card_preview` (the dash, `NOTHING_REMEMBERED`, before there is one), read with the
  same function its resolution reads it with; what only its owner may know stays in their hand.

**Purity (CLAUDE.md rules 4 and 5).** A hook reads its `EffectContext` and returns `Vec<Effect>`;
it never assigns to the state, never calls an engine mutator and never reaches for a clock or the
OS. Every effect comes from `jackioh_engine::effects` (re-exported by the prelude): that module is
the whole vocabulary of a card file. A verb that is missing is missing from the engine; add it there
with its own engine test. A card-specific system (C #57 Echo's copied text, C #90's quests, C+ #29's
last boards, C+ #35's snapshots, C+ #42's problems, C+ #62's curve, C+ #27's perfect hand) is an
engine module under `crates/engine/src/subsystems/` with its own engine tests through fixture
scripts, never logic in the card file. This crate's `clippy.toml` is the engine's, so `cargo clippy`
refuses a clock, a file or an environment read here too.

**The read surface.** A hook may read the state to compute an effect's argument, but only through
the engine's read helpers, never a `GameState` field: no script names `state.players[…]` above its
`#[cfg(test)]` (a test below it may read the state to assert). Every one is in the prelude:

| Read | Answers |
| --- | --- |
| `hero_of(state, player)` | `HeroView { health, armor }`, copied |
| `starting_health_of(state, player)` | the health the player's hero started with: its handicap's `heroHealth`, else `HERO_HEALTH` (R290, R1866002) |
| `zone_cards(state, player, zone)` / `zone_count(…)` | one off-field pile (`OffFieldZone`: hand, library, graveyard, exile) as a copy, `library[0]` on top; its size |
| `cards_played_this_turn(state, player)` | §10.5 step 4's counter, counting the card being played |
| `unspent_mana_of(state, player)`, `max_mana_of(state, player)` | the mana a player holds now; §2.3's max mana |
| `played_ids_this_turn`, `played_earlier`, `was_played_this_turn` | the turn's plays, §6.2's Combo count, the one-shot gate of §5.1's "return to hand" spells |
| `active_units_of`, `dormant_units_of`, `card_at`, `slots_of`, `slot_of` | the field, by lane (§3.2, R13) |
| `units_on_field_of`, `is_slumbering`, `slumber_turns_of` | the Units filling a side's zones, Dormant ones included, which `active_units_of` leaves out; whether a card is Dormant and its countdown (§6.1, R1839000) |
| `building_a_ship`, `dormant_starship_of`, `is_dormant_starship`, `launched_starship_of`, `is_launched_starship`, `starship_pieces_of`, `launch_cost`, `when_launched` | the Starship (§6.1, §6.2, R1852000–R1852006): whether a player has a Dormant Starship and which, a player's launched Starship, the pieces a Starship holds, what a launch costs now, and a piece's "When launched" trigger; the price rule is the verb `starship_launch_costs` |
| `face_of`, `stats_with_buffs`, `unit_view` | a unit through the §10.4 layers, never off the instance |
| `def_of`, `printed_cost`, `effective_cost`, `query_cost`, `own_cost`, `cost_now` | a definition and its costs (R65, R386, R396, R455) |
| `tags_of(state, card)` | the card's tags now — its definition's plus what effects granted (R923); every instance-level tag read goes through it |
| `lucky_on(state, card)` | the card's Lucky X now — its printed Lucky as tuning leaves it plus every Lucky it was given (R1438); every luck-based roll a card makes reads it |
| `highest_permanent_cost(state, player)` | the highest R396 cost among the permanents acting on a side, `None` with none (R901) |
| `base_stats_of(state, card)` | §10.4 layer 1: the stat override as its face wears it, else the printed stats; no buffs, tuning or auras (R1181) |
| `credit_used_this_turn(state, player)` | whether the player borrowed mana this turn — Jlarna's base face reads it (R1225) |
| `armor_on_field(state)` | every point of Armor on the field, Units through §10.4 and both heroes (R1064) |
| `find_instance(state, id)` | an instance id, wherever the card has since landed (R98) |
| `instance_of(ctx, spec)` | the card a `TargetSpec` names on the stay the run aimed at (R174) |
| `recalled(ctx, key)` | what the running card remembers under a key (`remember`'s write; R102) |
| `killer_of(state, card)` | the Unit that destroyed a card, while it still acts (R42, R361) |
| `when_attacks_of(ctx)` | in a `when_attacks` hook, its attack's facts, before any damage (R2285000) |
| `after_attack_of(ctx)` | in an `after_attack` hook, its combat's facts; it runs only for a surviving attacker (R2285001), or for any with `after_attack_even_if_gone`, `survived` false for a gone one (R426, R2396000) |
| `after_attacked_of(ctx)` | in an `after_attacked` hook, its combat's facts (R1026) |
| `param(ctx, key)` | the running card's current value of a declared number (`params`, R386) |
| `traded_card_of(ctx)` | in a trigger on `traded`, the card that Trade put into a deck (R1864003) |
| `luck_of(state, player)` | the player's Luck for best-of rolls (R987, Meditative #40) |
| `player_spell_damage(state, player)` | the player's Spell Damage, the one sum §4.4 step 0 reads: Units, auras for one player or both, "this turn" modifiers (R1180000, R1180001) |
| `heal_multiplier_of(state, player)` | how many times as large a heal of the player's hero or Units is now (R1180003, Meditative+ #7) |
| `inflation_chance_of(card, printed)`, `next_inflation_chance(chance)` | the Inflation chance a card rolls at (the one it carries, else its printed one) and the one the copy it makes carries (R1848006, Stinkyscape #153) |
| `freeze::frozen_forever(card)` | whether a Unit is Frozen forever, which never thaws (R1848000) |
| `subsystems::feng_shui::{last_element, element_preview}` | the last-played elements and their `preview` lines (R982, Meditative #40) |
| `subsystems::activation_paid(ctx)` | what an Activate paid (R384, C #21) |
| `subsystems::completed_stage_of(ctx, event)`, `subsystems::quest_stages_of(state, card)`, `subsystems::quest_memory_of(card)` | the stage (1 for the first) a staged quest's own `questCompleted` names, its stages, and its line: the stage it stands at and its wait to hear back (R1854000, R1854004, Stinkyscape #7, #112) |

Board facts live in `crates/engine/src/query.rs`; if the one you need is missing, add it there and
test it there. Randomness goes through `ctx`'s rng and every pool through `catalog.query` (§3).

**Play-time choices (R81).** Zone, X, embiggen, Tribute and the targets and modes a card needs for
its own play travel in the `play` action. Declare them (`targets: vec![TargetDecl::target(…)]`,
`modes`) so `legal_actions` and the client can build the pickers without running the script; the pick
arrives in `ctx.targets` or `ctx.modes`. An Activate declares its own the same way. Every choice made
during resolution (Discover, a chained step, an Echo repeat, a trigger) is a `PendingChoice` opened
by a `choose*` or `discover*` effect (spec §10.6).

**Instance memory.** Anything a card must remember lives on the instance, written by the `remember`
and `remember_random` effects, so Fuse, copies and replay stay trivial (spec §10.1).

## 2. The catalog entry

`catalog.json` is the printed card. `radiant.text` is the Radiant face written out in full (R277),
every Radiant face meets R275's standard (`tests/cross/radiant_standard.rs` against
`docs/radiant-audit.md`), and `refs` lists every card or token the entry's texts name (R279,
`tests/cross/references.rs`). The fields patch v0.2.0 added are all data:

- `printedRarity`: the rarity printed on a token, for display only; a token's `rarity` stays
  `"Token"`, so no pool finds one by rarity.
- `params`: the numbers a Nerf, a Buff and KY's Constant may change (R386), per face, written
  `{key}` in the text. A script reads `param(ctx, key)`, never a literal. Players read Nerf and Buff
  for the engine's Degrade and Upgrade, whose verbs keep their names (`degrade`, `upgrade`, R1320).
  `cry: true` marks a number only the card's Cry reads, which no Nerf or Buff draws on the field
  (R1748000), and `cry: "base"` or `"radiant"` one only that face's Cry reads while the other face's
  lasting text reads it too (R2261001); `catalog check` holds both to the text. A number a Spell or a Field Spell prints is
  declared, or `tests/cross/params.rs` names why it stays fixed (R1748003).
- `dealsDamage: true`: a Spell or a Field Spell that deals damage, which a Buff may give Pierce or
  Lifesteal (R1748001). `usesPlague: true`: a Field Spell that uses the Plague Counters on itself, on
  which a Buff places one and from which a Nerf removes one on the field (R1748002).
- `snapshotOf`: set on no card since issue #2511 (R2511001). The catalogs shipped from v0.5.5 to
  v0.5.7 set it on Training Grounds #41–#60, the Core card each copies, and a replay folded under
  one still reads it (one Discover offers one of the pair, R2204001); `catalog check` refuses it now.
  Those twenty are the Legacy cards, named `Legacy: <Core name>` and tagged Legacy (R2511000), each
  its own card beside its Core card; its script is a copy of the Core card's code, not a re-export
  (R2195003).
- `loc`: the lines of code of the card's script. Frozen data since v0.3.0: nothing recomputes it, C
  #48 and C+ #44, #45 read it, and a card added since takes its number from
  `cargo jackioh catalog loc <script path>`.
- A face's own `type` (C+ #22 Blood Moon's Radiant face is a Field Trap) and `xStats` (C+ #69).

`flavor` (R1437000) is what kind of thing the card is, one of the engine's fifteen `Flavor` names,
chosen by `docs/polish/8-flavor.md`. It picks the card's animations and nothing else: no rule, pool or
AI reads it, `catalog check` holds it to the fifteen, and a patch reads the catalog without it, so a
flavor-only change needs no fragment.

Tags are the designer's as written (`Jlockeed` is a pool as well as a filter, R278). No two cards
share a name (R381). Texts follow the house style (R432, R373): "Nerf" and "Buff", never "Degrade" or
"Upgrade" (R1320).

## 3. `catalog.query`, the only random pool (spec §5.1)

```rust
use crate::query::*;   // in a card file that draws from a pool

catalog.query(&json_as(json!({ "type": "Unit", "cost": 2 })))     // every matching non-token card
catalog.pool("core-057", &json_as(json!({ "tags": ["KY"] })))       // the same, your own id excluded (R387)
catalog.cost(def)                                                   // R65's out-of-play cost
TRAP_TYPES                                                          // [Trap, Field Trap]
```

`src/query.rs` is a thin wrapper over the engine's one filter (`crates/engine/src/catalog.rs`). Its
guarantees, proved by `tests/cross/query.rs`, which names every pool the spec pins down with the exact
arguments it needs: no tokens unless the query asks for them (R382's Fruit pool holds the Grapes);
never the generating card when you use `pool` (R387); costs read out of play (R65); a deterministic
order, set by set and each by index, so a seeded pick replays (§9.3, R60). Never sort, filter or
dedupe a pool yourself afterwards. `"type": "Trap"` drops the Field Traps; where the spec says a
Field Trap counts as a Trap, pass `TRAP_TYPES`.

## 4. The registry

`build.rs` walks `src/scripts/<set>/*.rs`, reads each `pub const ID`, and writes the module tree and
`REGISTRY` (id → `script`) into `$OUT_DIR/registry.rs`, and every card's code block but a token's
into `$OUT_DIR/code_blocks.rs` (`CODE_BLOCKS`, R1183004: its script's lines of code less its `ID`
line, cut to the engine's `CODE_BLOCK_MAX_LINES`, which `register_all` hands to the engine for
Meditative+ #25's code Discover), and the card metrics into `$OUT_DIR/card_metrics.rs` (R1846000:
`CODE_LINES`, every script's lines above its tests that hold code, comments and blank lines left out;
`BUSTED`, the ids over the engine's `BUSTED_CODE_LINES`, and `COMPLICATED`, over its
`COMPLICATED_CODE_LINES`, which the catalog query's `busted` and `complicated` read). There is nothing to wire and nothing to
regenerate by hand: drop the file in its folder and build. The build fails, naming the path, on a file
without `ID`, an `ID` the catalog lacks, two files claiming one id, a file outside a set folder, a
file name that is not a Rust identifier, a file outside its `ID`'s set's folder or named for another
card's number, or a catalog id with no file. It also compiles in `CATALOG_VERSION`, the newest entry
of `patches/patches.json`.

`register_all()` hands the catalog and every script to the engine, once per process, and where the
older shipped catalogs are found (`register_shipped`); the WASM module, the server and the tools
call it at start. A card test calls `crate::register_all()` first.

R767000: `build.rs` also writes `$OUT_DIR/history.json`, how each shipped version but the newest
differs from `catalog.json`, and `src/history.rs` rebuilds a version's catalog from it the first
time a replay of a game played on it asks (`shipped_catalog`, `carried_versions`). Its scripts are
the build's own, except a script kept for that version in `KEPT_SCRIPTS` (§6). The browser's
module (a `wasm32` target) carries only the versions from `build.rs`'s `BROWSER_CARRIED_FROM` on:
it folds only practice games, and one folds on a carried version only with the setup it kept.

## 5. Tests

A card's tests are the `#[cfg(test)] mod tests` at the bottom of its file, built on
`jackioh_engine::testkit::scenario(json!({ "p1": {…}, "p2": {…} }))` and its verbs (`play`,
`attack`, `answer`, `end_turn`, `start_turn`, `activate`, `expect_in_zone`, `expect_stats`,
`expect_events`, `expect_health`, `expect_refused`, …; `crates/engine/src/testkit/scenario.rs` says
what each does). `expect_refused` and `expect_refused_with` pass only on a refusal, `reduce`'s or an
answer the open prompt does not offer: any other panic in the step (a reference that matches nothing,
`answer` with no prompt open) fails the test, so refuse the step while the board still holds what the
rule needs (#1247). For **base and radiant separately**: one test per behavior in the card's spec §8 row
and every must-pass case in its BUILD table row (M4-T4 for Core, M9 for Classic and Classic+). A
test that proves a ruling is named after it: `fn r64_the_copy_lands_in_the_leftmost_free_zone`
(CLAUDE.md rule 3). A test that only reads printed data back with no action (cost, stats, type, tags
or rarity, which `tests/cross/catalog.rs` proves for every entry) proves nothing a card test should,
so write none (#1125). A "no tuned numbers" clause still gets its
`params.is_none()` test: `catalog.rs` does not assert `params`.

A card that declares `condition_met` proves both answers in `tests/cross/condition_active.rs`, and
one that declares `preview` proves its values in `tests/cross/preview.rs`; each file also pins the
set of cards that declare the hook to its ruling's list, so a new hook fails until its proof and the
ruling's list are updated.

```
cargo test -p jackioh-cards                     # every card and cross-card test
cargo test -p jackioh-cards c043                # card 043 of every set that has one
cargo test -p jackioh-cards classic_plus::      # one set's cards
cargo test -p jackioh-cards --lib -- --skip scripts::classic:: --skip scripts::classic_plus:: --skip scripts::meditative::  # one binary, CI's four shards (#882): Core's skips the other sets, so it runs every unit test outside them too
cargo test -p jackioh-cards --lib -- scripts::classic::  # CI shard (#882)
cargo test -p jackioh-cards --lib -- scripts::classic_plus::  # CI shard (#882)
cargo test -p jackioh-cards --lib -- scripts::meditative::  # CI shard (#882)
cargo test -p jackioh-cards --lib -- scripts::meditative_plus::  # the Meditative+ cards (Core's shard runs them too, its skips not naming them)
cargo test -p jackioh-cards --lib -- scripts::stinkyscape::  # the Stinkyscape cards (Core's shard runs them too)
cargo test -p jackioh-cards --test cards        # the cross-card tests only
cargo clippy -p jackioh-cards --all-targets -- -D warnings
```

## 6. Patches and the catalog version (R388, R646)

Every change to card data is a patch, and every patch is kept.

- `patches/patches.json` lists every shipped patch in ship order (`{ version, date, title, source,
  notes, player, tier, plays }`, `player` the note players read on the Patch notes page, R724001;
  `tier` and `plays`, the families that play differently, recorded from R1366000 on);
  `patches/<version>.json` is the whole catalog as that patch left it; `patches/win_rates/<version>.json`
  the win-rate table it shipped with (R1329000; none for a patch before v0.3.15, which shipped with
  the empty one); `shipped.json` records each patch's commit, snapshot blob and
  table blob (`winRatesBlob`); `index.json` maps each card to the versions it changed in. Shipped
  snapshots and tables are never amended. `aliases.json` is the version retcon's map (#1371,
  R1371000), each old name to its new one: only a reader of a file without the `versions` marker
  consults it (`src/versions.rs`, `current_version_name`), and the history's names are the new
  ones. Older docs name versions the old way (`docs/issues-and-patches.md`, Version retcon).
- `patches/technical.json` is the technical log (R1369000): the changes that shipped no card data, a
  list in logged order of `{ version, commit, pr?, title, kind, player? }`, `version` the newest
  shipped X when it was logged, `commit` its key, `kind` `app`, `ai` or `internal`, and `player` the
  line the Patch notes page shows under that X (App and AI only). `cargo jackioh patches log
  <base>..<head> [--bodies <path>]` appends to it (`crates/tools/src/technical.rs`'s header has the
  rest); nothing else writes it, and `cargo jackioh patches check` holds the whole file to that
  shape on every run.
- A branch that changes a card claims it with a pending fragment:
  `cargo jackioh patches "<title>" --source "<issue or PR>" --notes "<what changed>"` writes
  `patches/pending/<slug>.json`, named after the title, with no version (R1366000; `--cards` names
  the ids, otherwise they are diffed from the newest shipped snapshot; `--plays <ids>` claims cards
  whose script or engine rule changed how they play with no catalog change, `--rule` a rule every
  game meets, both as `cargo jackioh classify` finds them, which CI's `rust (classify)` holds them to,
  R1370000). Its `player` note is written into the fragment by hand, since no flag
  sets it, and kept when the command runs again (`docs/issues-and-patches.md`, Patch notes players
  read). A patch that refreshes `data/win_rates.json` claims it too: the command writes
  `"winRates": true` when the table's rows differ from the newest shipped version's and no other
  fragment claims them, and a fragment may claim the table and no card (R1329000).
  `cargo jackioh patches check` (CI's `rust (checks, fuzz 200)` job) fails naming
  the card when a change is unclaimed or claimed twice, a claimed card did not change, a `plays` id
  is no shipped card, a fragment claims nothing, or a fragment's title, sources or notes is empty, when a fragment's or a shipped patch's player note is missing, empty or names a ruling,
  an issue or a § (R724001), and when the win-rate table's rows changed unclaimed, are claimed twice,
  or are claimed unchanged (R1329000).
- After the merge, `.github/workflows/patches-ship.yml` runs `cargo jackioh patches ship`: it
  appends each fragment in first-parent order with its player note, named from its tier (a major
  `v0.(Z+1).0`, a patch `v0.Z.(X+1)`, a micro the next letter, R1366000) with the tier and `plays`
  recorded, snapshots the patch before it
  with the cards the fragment claims as the catalog holds them (R1280000), writes its win-rate table
  (`data/win_rates.json` from the fragment that claims it on, the patch before it's until then,
  R1329000), records `shipped.json`, deletes
  the fragment and bumps the catalog version at every site that carries it (`render.yaml`,
  `crates/server/.env.example`, and `apps/web/.env.production` when it names one). The binaries take
  the version from `patches.json` at compile time; `cargo jackioh catalog-version` prints it.
- A version is opaque (R105): nothing parses or orders one, and the order of patches is
  `patches.json`'s. A snapshot keeps a card's data, not its script.
- `patch_history()` (`src/patch_history.rs`, R1367000) reads `patches.json` as a card's statistics
  window does: each version's tier and the families that play in it, as recorded, or, for a version
  shipped before tiers were recorded, from its `changes` (`WORDING_FIELDS`, `PATCH_MIN_CARDS`,
  `PRE_TIER_MAJORS`). The server and the tools hand it to the engine's `windowed_card_stats`.
- Every shipped version but the newest is carried for replays (R767000): its definitions rebuilt
  from its snapshot, its scripts the build's own, and the win-rate table it shipped with (R1266000:
  its `win_rates/<version>.json`, or the empty table with no file, R1329000). A patch
  that changes what a card's script does, not only a number its definition holds, keeps the script
  it replaces in `src/history.rs`'s `KEPT_SCRIPTS` as `(ID, the newest shipped version that ran it,
  a fn building that card's scripts)`; without one, a replay that script changes is refused by its
  final hash.
  `cargo jackioh patches check` proves every carried version holds its snapshot's cards, in order,
  each with a script, and its table's rows.

## 7. Flavour (R660)

A card's words other than its rules are written in one file, `card-text.yaml` (#2498): one entry per
catalog card and token, in catalog order with the card's name in a comment beside its id, holding its
`flavour` and `artist`, its `voice` lines (R655, spoken in the voices of
`apps/web/src/audio/card-audio.json5`) and its `zh` (§8), and an `emotes` section with the six
portraits' lines (R644). This crate parses no YAML (CLAUDE.md rule 4):
`cargo test -p jackioh-tools --test export_card_text` writes `flavour.json`, `chinese.json` and
`apps/web/src/audio/voice-text.json` from it, each headed by a `"//"` entry that says so, and fails
while one is stale; commit what it writes. It also holds the YAML's ids to the catalog's, in its order,
and each name comment to the card's name.

`flavour.json` is keyed by catalog id, `{ "flavour"?, "artist"? }`. It is not card data: an edit to it
is no patch and claims no fragment. `tests/cross/flavour.rs` holds every key to a catalog entry, every
entry to a flavour line, and each line to one trimmed line under its cap (120 characters, 60 for an
artist); the client's test also refuses rules words. Real art is credited when it lands
(`apps/web/src/cards/art/ART.md`).

## 8. Chinese text (R1303)

`chinese.json`, written from each entry's `zh` in `card-text.yaml` (§7), is keyed by catalog id,
`{ name, base, radiant, previews? }`: the name and both faces in Simplified Chinese, one Chinese line
per English line. It is not card data: an edit to it is no
patch and claims no fragment. Each face keeps exactly its English face's `{key}`s (an English
`{key|one|many}` becomes `{key}` plus a measure word), names every `refs` card by its Chinese name,
and carries `previews` exactly where the script declares `preview`. `chinese-terms.json` holds the
frame's words: types, tags, rarities, keywords, labels, Radiant, Created and the glossary.
`tests/cross/chinese.rs` proves both files. Every card added later adds its Chinese entry in the
same change ([`docs/ADDING_CARDS.md`](../../docs/ADDING_CARDS.md)).
