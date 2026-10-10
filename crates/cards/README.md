# `jackioh-cards`

318 catalog entries in the three shipped sets, Core (100 cards and 11 tokens), Classic (90 cards and
Glitch, a hidden token) and Classic+ (78 cards and 38 tokens), 268 cards and 50 tokens in all, and the
Meditative set (102 cards and 30 tokens, spec §8.8) joining the catalog part by part before it ships
(R1420: no pool that names no set, no deck and no random deck holds its cards until the release, and a
test opens it with the testkit's `preview_sets`); one script file
per entry with its tests inside it; the catalog query every random pool goes through; and the
catalog's patch history. This file is the contract between the card files. The spec (`spec/08-catalog.md`
for every card, `spec/07-tokens.md` for the tokens) is the only source of card text; when it and this
file disagree, the spec wins. [`docs/ADDING_CARDS.md`](../../docs/ADDING_CARDS.md) is the procedure
for adding, changing or removing a card (CLAUDE.md rule 10).

```
crates/cards
├── catalog.json        the card data, proved against spec §8 by tests/cross/catalog.rs
├── flavour.json        flavour lines and artist credits, not card data (R660)
├── chinese.json        Simplified Chinese names and faces, not card data (R1303)
├── chinese-terms.json  the frame's Chinese words: types, tags, rarities, keywords, labels (R1303)
├── patches/            the catalog's history (R388, R646): patches.json, one snapshot per patch,
│                       shipped.json, index.json, and pending/ (one fragment per patch being built)
├── build.rs            generates the registry from src/scripts/** and compiles the catalog version in
├── clippy.toml         the purity lint (CLAUDE.md rule 4), the engine's file
├── src/lib.rs          CATALOG, CATALOG_IDS, CATALOG_VERSION, card_def, register_all, the registry
├── src/query.rs        spec §5.1's catalog.query, the only random-pool source
├── src/ky_test_bank.rs C+ #42 KY's Test's problems (R420)
├── src/scripts/
│   ├── core/           one Core card or token per file
│   ├── classic/        one Classic card per file
│   ├── classic_plus/   one Classic+ card or token per file
│   └── meditative/     one Meditative card or token per file (the set ships with #496's release, R1420)
└── tests/cards.rs      the cross-card tests: tests/cross/<x>.rs (catalog, query, references, radiant
                        standard, flavour, condition_active, preview, invariants, …)
```

## 1. The card file

**One card, one file, its tests inside it** (CLAUDE.md rule 6). The file is
`src/scripts/<set>/<name>.rs`: the spec §5 index in three digits with a `c` in front, then the slug,
every `-` and `.` an `_`.

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
  contents or a hand's cards. A declared number (`params`) is not a preview.

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
| `zone_cards(state, player, zone)` / `zone_count(…)` | one off-field pile (`OffFieldZone`: hand, library, graveyard, exile) as a copy, `library[0]` on top; its size |
| `cards_played_this_turn(state, player)` | §10.5 step 4's counter, counting the card being played |
| `unspent_mana_of(state, player)`, `max_mana_of(state, player)` | the mana a player holds now; §2.3's max mana |
| `played_ids_this_turn`, `played_earlier`, `was_played_this_turn` | the turn's plays, §6.2's Combo count, the one-shot gate of §5.1's "return to hand" spells |
| `active_units_of`, `dormant_units_of`, `card_at`, `slots_of`, `slot_of` | the field, by lane (§3.2, R13) |
| `face_of`, `stats_with_buffs`, `unit_view` | a unit through the §10.4 layers, never off the instance |
| `def_of`, `printed_cost`, `effective_cost`, `query_cost`, `own_cost`, `cost_now` | a definition and its costs (R65, R386, R396, R455) |
| `tags_of(state, card)` | the card's tags now — its definition's plus what effects granted (R923); every instance-level tag read goes through it |
| `lucky_on(state, card)` | the card's Lucky X now — its printed Lucky as tuning leaves it plus every Lucky it was given (R1438); every luck-based roll a card makes reads it |
| `highest_permanent_cost(state, player)` | the highest R396 cost among the permanents acting on a side, `None` with none (R901) |
| `base_stats_of(state, card)` | §10.4 layer 1: the stat override as its face wears it, else the printed stats; no buffs, tuning or auras (R1181) |
| `find_instance(state, id)` | an instance id, wherever the card has since landed (R98) |
| `instance_of(ctx, spec)` | the card a `TargetSpec` names on the stay the run aimed at (R174) |
| `recalled(ctx, key)` | what the running card remembers under a key (`remember`'s write; R102) |
| `killer_of(state, card)` | the Unit that destroyed a card, while it still acts (R42, R361) |
| `after_attack_of(ctx)` | in an `after_attack` hook, its combat's facts (R426) |
| `after_attacked_of(ctx)` | in an `after_attacked` hook, its combat's facts (R1026) |
| `param(ctx, key)` | the running card's current value of a declared number (`params`, R386) |
| `subsystems::activation_paid(ctx)` | what an Activate paid (R384, C #21) |

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
- `loc`: the lines of code of the card's script. Frozen data since v0.3.0: nothing recomputes it, C
  #48 and C+ #44, #45 read it, and a card added since takes its number from
  `cargo jackioh catalog loc <script path>`.
- A face's own `type` (C+ #22 Blood Moon's Radiant face is a Field Trap) and `xStats` (C+ #69).

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
`REGISTRY` (id → `script`) into `$OUT_DIR/registry.rs`. There is nothing to wire and nothing to
regenerate by hand: drop the file in its folder and build. The build fails, naming the path, on a file
without `ID`, an `ID` the catalog lacks, two files claiming one id, a file outside a set folder, a
file name that is not a Rust identifier, or a catalog id with no file. It also compiles in
`CATALOG_VERSION`, the newest entry of `patches/patches.json`.

`register_all()` hands the catalog and every script to the engine, once per process; the WASM
module, the server and the tools call it at start. A card test calls `crate::register_all()` first.

## 5. Tests

A card's tests are the `#[cfg(test)] mod tests` at the bottom of its file, built on
`jackioh_engine::testkit::scenario(json!({ "p1": {…}, "p2": {…} }))` and its verbs (`play`,
`attack`, `answer`, `end_turn`, `start_turn`, `activate`, `expect_in_zone`, `expect_stats`,
`expect_events`, `expect_health`, `expect_refused`, …; `crates/engine/src/testkit/scenario.rs` says
what each does). For **base and radiant separately**: one test per behaviour in the card's spec §8 row
and every must-pass case in its BUILD table row (M4-T4 for Core, M9 for Classic and Classic+). A
test that proves a ruling is named after it: `fn r64_the_copy_lands_in_the_leftmost_free_zone`
(CLAUDE.md rule 3).

A card that declares `condition_met` proves both answers in `tests/cross/condition_active.rs`, and
one that declares `preview` proves its values in `tests/cross/preview.rs`; each file also pins the
set of cards that declare the hook to its ruling's list, so a new hook fails until its proof and the
ruling's list are updated.

```
cargo test -p jackioh-cards                     # every card and cross-card test
cargo test -p jackioh-cards c043                # card 043 of every set that has one
cargo test -p jackioh-cards classic_plus::      # one set's cards
cargo test -p jackioh-cards --test cards        # the cross-card tests only
cargo clippy -p jackioh-cards --all-targets -- -D warnings
```

## 6. Patches and the catalog version (R388, R646)

Every change to card data is a patch, and every patch is kept.

- `patches/patches.json` lists every shipped patch in ship order (`{ version, date, title, source,
  notes }`); `patches/<version>.json` is the whole catalog as that patch left it; `shipped.json`
  records each patch's commit and snapshot blob; `index.json` maps each card to the versions it
  changed in. Shipped snapshots are never amended.
- A branch that changes card data claims it with a pending fragment:
  `cargo jackioh patches <version> <date> "<title>" --source "<issue or PR>" --notes "<what changed>"`
  writes `patches/pending/<version>.json` (`--cards` names the ids; otherwise they are diffed from
  the newest shipped snapshot). `cargo jackioh patches check` (CI's `rust (checks, fuzz 200)` job) fails naming
  the card when a change is unclaimed or claimed twice, a claimed card did not change, a version is
  neither a bare patch number nor a micro `vA.B.Y` (R650), or a fragment's title, sources or notes is
  empty.
- After the merge, `.github/workflows/patches-ship.yml` runs `cargo jackioh patches ship`: it
  appends each fragment in first-parent order, snapshots the catalog, records `shipped.json`, deletes
  the fragment and bumps the catalog version at every site that carries it (`render.yaml`,
  `crates/server/.env.example`, and `apps/web/.env.production` when it names one). The binaries take
  the version from `patches.json` at compile time; `cargo jackioh catalog-version` prints it.
- A version is opaque (R105): nothing parses or orders one, and the order of patches is
  `patches.json`'s. A snapshot keeps a card's data, not its script.

## 7. Flavour (R660)

`flavour.json` is keyed by catalog id, `{ "flavour"?, "artist"? }`. It is not card data: an edit to it
is no patch and claims no fragment. `tests/cross/flavour.rs` holds every key to a catalog entry, every
entry to a flavour line, and each line to one trimmed line under its cap (120 characters, 60 for an
artist); the client's test also refuses rules words. Real art is credited when it lands
(`apps/web/src/cards/art/ART.md`).

## 8. Chinese text (R1303)

`chinese.json` is keyed by catalog id, `{ name, base, radiant, previews? }`: the name and both faces
in Simplified Chinese, one Chinese line per English line. It is not card data: an edit to it is no
patch and claims no fragment. Each face keeps exactly its English face's `{key}`s (an English
`{key|one|many}` becomes `{key}` plus a measure word), names every `refs` card by its Chinese name,
and carries `previews` exactly where the script declares `preview`. `chinese-terms.json` holds the
frame's words: types, tags, rarities, keywords, labels, Radiant, Created and the glossary.
`tests/cross/chinese.rs` proves both files. Every card added later adds its Chinese entry in the
same change ([`docs/ADDING_CARDS.md`](../../docs/ADDING_CARDS.md)).
