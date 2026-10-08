# Adding a card

Read this before any card work. It is the procedure for a **standard** card: one whose text is made of
verbs, keywords and filters the engine already has. Anything else is [§6](#6-a-card-the-engine-cannot-express-yet).

It was first written by reading C+ #5 Guy Att, #6 Wrong-House Attacker, #77 Anti-Softlock and the Rush Token, grepping every file that
names a card of each kind, and adding a dummy Classic+ unit (a vanilla (1) 1/1 Human) to a scratch worktree and running every gate;
since v0.3.0 the cards are Rust, and the files and gates below are the Rust ones. The contract a card file is held to is
[`crates/cards/README.md`](../crates/cards/README.md). One step cannot be done on Linux: [§7](#7-what-cannot-be-done-on-linux).

## 1. The files, in the order to touch them

A Classic+ card is shown. Core cards live in `src/scripts/core/`, Classic in `src/scripts/classic/`. `cNNN_slug` is the card's spec
index in three digits after a `c`, then its slug with every `-` and `.` an `_` ([cards README §1](../crates/cards/README.md#1-the-card-file)).

| # | File | What changes | A gate fails without it |
|---|---|---|---|
| 1 | `spec/08-catalog.md` §8.7 (§8.1 Core, §8.6 Classic; `spec/07-tokens.md` for a token) | the card's row: the only source of card text (CLAUDE.md) | none |
| 2 | `crates/cards/catalog.json` | the entry, after the set's highest card index and before its shared tokens (`T-AI-…`); a token follows its card | `catalog check`, `tests/cross/catalog.rs` |
| 3 | `crates/cards/src/scripts/classic_plus/cNNN_slug.rs` | the script and its tests in one file ([§2](#2-templates)) | the build: `build.rs` refuses a catalog id with no file |
| 4 | `crates/cards/tests/cross/catalog.rs` | the card's fixture row in `CLASSIC_PLUS`; `RARITY_COUNTS` and `SET_SIZES` (the totals and row counts are their sums) | itself |
| 5 | `crates/tools/src/catalog.rs` | the set's `cards` and `rarities` in `sets()`, `expected_tag_count` for each tag the card has, and the totals `the_shipped_catalog_passes_every_check` pins | `catalog check`, `cargo test -p jackioh-tools` |
| 6 | `docs/radiant-audit.md` | one row `\| <index> \| <name> \| …`; the Radiant face must meet R275 (about twice the base face; doubling stats alone is not enough for a unit with text) | `tests/cross/radiant_standard.rs` |
| 7 | `BUILD.md` | the card's must-pass row in the M9 (or M4-T4) table | none |
| 8 | `apps/web/src/audio/card-audio.json5` | the card's entry in catalog order, its name in a comment beside its id: a Unit's play and death lines, anything else's cast line, and any effects or attack hook (R655; the file's header says how) | `voice-lines.test.ts`, `voiceData.test.ts` |
| 9 | `crates/cards/flavour.json` | the card's flavour line, `"<id>": { "flavour": "…" }`: one short line in the source notes' voice, no rules words, at most 120 characters (R660); it is not card data, so no fragment claims it | `tests/cross/flavour.rs`, `apps/web/src/cards/flavour.test.tsx` |
| 10 | a pinned pool, when the card joins one | `tests/cross/query.rs` names the pools the spec pins down card by card (the KY pool, the (1) Cost Traps, R35's Legendaries, …) | `tests/cross/query.rs` |
| 11 | `crates/cards/patches/pending/<version>.json` | the fragment, `cargo jackioh patches …` ([§3](#3-the-patch-and-its-order)); the shipped history and the version sites move at promotion, not here | `patches check` |
| 12 | a Legendary or Mythic, or a token printed so: `apps/web/scripts/music/tracks.mjs`, `apps/web/src/audio/music-cards.json`, `assets/music/LICENSES.md` | its intro (R1352): an `INTRO_CARDS` row (its facts, and a `tune` if it is hand-tuned), its `"intro": "intro-<id>"` entry, `pnpm --filter @jackioh/web gen:music` (FluidSynth and ffmpeg, Linux included) with the rendered file and manifest row committed, and its licence row | `music-assets.test.ts` (the daily audio run) |

The catalog and pool counts elsewhere (the web's deck builder, Almanac and patch tests, the e2e component specs) count
`catalog.json` and `patches.json` themselves and need no edit for a new card. The patch-list tests pin only the history shipped
before yours, so a pending-claimed card needs no edit there (R646). What stays hand-kept is the proof: `catalog.rs`'s rows,
`RARITY_COUNTS` and `SET_SIZES`, and the tools' `sets()` and tag counts.

Also grep the Markdown for the stated totals (`268 cards`, `318`) and update them: the READMEs, `BUILD.md`, `REVIEW.md`, `CLAUDE.md`,
`spec/`, `docs/architecture.md`.

**A card of a set that has not shipped** (the Meditative set while issue #496 builds it, R1420) differs in five places:
- Its script goes in `src/scripts/meditative/`.
- Rows 1, 4, 5, 6 and 7 are already written. §8.8, §7, the `MEDITATIVE` fixture, the census in `crates/tools/src/catalog.rs`,
  `docs/radiant-audit.md` and BUILD M10 list every card of the set, so a card part corrects its rows only where its build
  decided otherwise.
- There is no row 11. The patches tool reads the catalog without the set, so its cards need no fragment until the release
  claims the whole set at once. The stated totals above don't move either: they count the shipped sets.
- The set's cards are in no pool that names no set, in no deck and in no random deck. A test that needs them there opens the set
  with the engine testkit's `preview_sets(&[SetName::Meditative])` (one thread, until its guard drops).
- `cargo jackioh fuzz` previews every set, so the card is fuzzed from the day it lands, while the golden traces deal from the
  shipped sets and must not move.
- Its `card-audio.json5` entry is written as for any card, but no voice file is expected for it until the set ships
  (`gen-voice.mjs --check` leaves it out), so [§7](#7-what-cannot-be-done-on-linux)'s rendering is owed by the release, not by the card.
  Its intro (row 12) is the release's too: the intros' totality test reads the sets that ship.
- Tests that count the catalog count the sets that ship (`set_is_open` in Rust, `setShips` in the web), never every entry.
  `docs/meditative-set.md` M3 says the rest.

## 2. Templates

**Catalog entry** (C+ #6). `loc` is the script's lines of code, from `cargo jackioh catalog loc <script path>` once the script is written
([§3](#3-the-patch-and-its-order)); text is the printed text, with `{key}` where a number is declared in `params`. A token has
`"token": true`, `"rarity": "Token"` and the `Token` tag.

```json
"classicplus-006": {
  "id": "classicplus-006", "index": "6", "name": "Wrong-House Attacker", "set": "Classic+",
  "type": "Unit", "tags": ["Human"], "rarity": "Common", "token": false, "cost": 1, "loc": 3,
  "base":    { "attack": 1, "health": 1, "keywords": [{ "kind": "Rush" }, { "kind": "Lifesteal" }, { "kind": "Poisonous" }],
               "text": "Rush, Lifesteal, Poisonous" },
  "radiant": { "attack": 2, "health": 2, "keywords": [{ "kind": "Rush" }, { "kind": "Lifesteal" }, { "kind": "Poisonous" }, { "kind": "Reborn" }],
               "text": "Rush, Lifesteal, Poisonous, Reborn" }
}
```

**Script, keywords only** (`src/scripts/classic_plus/c006_wrong_house_attacker.rs`, its header comment left out). Keywords are catalog
data, so there is nothing to run:

```rust
use jackioh_engine::prelude::*;

pub const ID: &str = "classicplus-006";

pub fn script() -> CardScripts {
    let base = Script::default();
    // The Radiant face adds Reborn and doubles the stats: catalog data only.
    let radiant = base.clone();
    CardScripts { base, radiant }
}
```

**Script with an effect** (`c005_guy_att.rs`). Hooks return `Vec<Effect>` from the engine's effects and never touch state:

```rust
use jackioh_engine::prelude::*;

pub const ID: &str = "classicplus-005";

pub fn script() -> CardScripts {
    CardScripts {
        base: Script {
            cry: Some(hook(|_ctx| vec![destroy_all(json_as(json!({ "side": "self", "rows": ["backrow"] })))])),
            ..Script::default()
        },
        radiant: Script {
            cry: Some(hook(|_ctx| vec![destroy_all(json_as(json!({ "side": "any", "rows": ["backrow"] })))])),
            ..Script::default()
        },
    }
}
```

Read a declared number with `param(ctx, "draw")`, never a literal. Declared targets and modes go in `targets` and `modes`
([cards README §1](../crates/cards/README.md#1-the-card-file)). A card with no script of its own for one face reuses `base`.

**Tests**, at the bottom of the same file (two cases in the shape of C+ #6's). Build every game with the testkit's `scenario()`
after `crate::register_all()`, and name a test after the ruling it pins (`fn r64_the_copy_lands_in_the_leftmost_free_zone`):

```rust
#[cfg(test)]
mod tests {
    use super::ID;
    use jackioh_engine::testkit::*;

    #[test]
    fn is_a_1_1_1_human_unit_radiant_2_2() {
        crate::register_all();
        let def = crate::card_def(ID);
        assert_eq!(def.cost, CardCost::Fixed(1));
        assert_eq!(
            [def.base.attack, def.base.health, def.radiant.attack, def.radiant.health],
            [Some(1), Some(1), Some(2), Some(2)]
        );
    }

    #[test]
    fn radiant_enters_the_field_when_played() {
        crate::register_all();
        let mut s = scenario(json!({ "p1": { "hand": [{ "def": ID, "radiant": true }] } }));
        s.play(ID, json!({}));
        s.expect_in_zone(ID, "field");
    }

    // …one case per behaviour in the spec row, base and radiant separately.
}
```

## 3. The patch, and its order

A change to card data is a patch (R388, R646). A normal patch (`Patch v0.2.X: …`) takes the next number
after the newest card patch on `main`, shipped or pending (R743); a micro patch (`Patch v0.2.Y: …`)
keeps its `Y` until promotion names it with the next letter (R650); never reopen a shipped one. A branch claims its card changes with a pending fragment:

```
cargo jackioh patches <version> <date> "<title>" --source "<issue or PR>" --notes "<what changed>"
```

It writes or updates `crates/cards/patches/pending/<version>.json`, containing the version, title,
sources, notes and every catalog id the patch changes. It never changes the shipped history, its
snapshots or the catalog version. `cargo jackioh patches check` must pass before the branch merges.
The post-merge promotion (`patches-ship.yml`) runs `cargo jackioh patches ship`: it snapshots the
catalog at the fragment's first-parent commit, appends the shipped patch and bumps the catalog version
wherever it is written; the binaries compile it in from `patches.json`.

`loc` is frozen data since v0.3.0: nothing recomputes it, and a refactor of an existing script leaves it as it is. A new card's comes
from `cargo jackioh catalog loc crates/cards/src/scripts/<set>/<file>.rs` (its non-blank, non-comment, non-`use` lines above
`#[cfg(test)]`), written into its entry by hand. In a patch split over several PRs, only the part that changes card data adds the
fragment ([issues-and-patches.md](issues-and-patches.md)).

## 4. Gates, and what a new card breaks that is not your card's fault

Run, from the repo root:

```
cargo fmt --check
cargo clippy --workspace --all-targets -- -D warnings
cargo jackioh catalog check
cargo jackioh patches check
cargo jackioh spec check
cargo test -p jackioh-cards c006_wrong_house_attacker    # your card's own tests first
cargo test --workspace --features jackioh-engine/testkit,jackioh-engine/ts
cargo jackioh fuzz --seeds 200                            # CI's wave; plain `cargo jackioh fuzz` plays 1,000
pnpm --dir apps/web test                                  # the voice lines and the flavour line
```

Replay one fuzz seed with `cargo jackioh fuzz --from <seed> --seeds 1`.

One more card shifts every random draw from the pool (R380), so tests and games that were lucky stop being so:

- **Seed-pinned tests** ("the first Discover offers Bigot, the second Twisted Sorcerer"): `crates/cards/tests/cross/`'s
  `play_choices.rs`, `resolving_face.rs`, `fused_hooks.rs`, `tributes.rs` and `pools_and_randomness.rs`, #98 Heroic Power's own
  tests (`src/scripts/core/c098_heroic_power.rs`, Stitching), and `crates/ai/tests/ai/shadow_ban.rs`. Re-pin by looping seeds in a throwaway test
  (`for n in 0.. { let mut s = scenario(json!({ "seed": format!("craft-{n}"), … })); … }`) until the offers match, and update the
  comment beside the seed.
- **The golden traces** (`crates/engine/tests/golden/games.jsonl`, replayed by `cargo test` and `cargo jackioh golden check`) are games
  recorded from the TypeScript engine. A recorded game that draws from a pool the new card joins can diverge; the failure names the seed,
  the step and the hash. If the divergence is the new card's pool and nothing else, re-record with `cargo jackioh golden bless` and say so
  in the PR: the file is then the Rust engine's own record.
- **The fuzz waves** play seeded games and may now draw a game nobody has played. Four latent bugs came out this way, all fixed: a fused
  card of two ingredients that define `targetChecks` threw from `legalActions` (#104), a client table made the animation runner replay a
  whole event window (#106), `determinize` put a Siphon Squad in a hidden slot the seat's own view rules out (#107), and a deeply nested
  fusion resumed Final Gambit's step against the wrong ingredient (#105). If a wave fails at a seed that has nothing to do with
  your card, print the failing game's state, find which card the panic or the diff names, and file it rather than editing the test.
- **A renamed card** (a patch that changes a `name`) fails `voice-lines.test.ts` until its comment in `card-audio.json5`, the name on the
  line of its id, says the new name (R655).
- **A card patched to Legendary or Mythic** (or a token's `printedRarity`) fails `music-assets.test.ts` until it has row 12's intro, and one
  patched down from either until its `intro` is taken out (R1352).

## 5. Do not read

These look relevant and do not change for a standard card: `crates/cards/src/lib.rs` and `build.rs` (the registry is generated: there is
nothing to wire), the snapshots in `crates/cards/patches/`, `crates/engine/**` (unless the text needs a new verb,
[§6](#6-a-card-the-engine-cannot-express-yet)), `apps/web/src/cards/**` (faces and art are drawn from the catalog; procedural art needs
nothing), `crates/server/src/**` and `crates/ai/src/**` (they read the catalog), `e2e/fixtures/decks/*.json`, `reviews/`, `docs/polish/`,
`docs/v0.3.0/`, and the designer's source notes (`JackiOh_*.md`).

## 6. A card the engine cannot express yet

When the text needs a verb, keyword, filter or board fact that no card uses: find where the most similar existing mechanic lives, add the new
one beside it in that pattern, and test it there. A verb is an effect in `crates/engine/src/effects/` (re-exported by `effects/mod.rs` and the
prelude), a new board fact goes in `crates/engine/src/query.rs`, a new keyword also needs its row in spec §6.1 (the web glossary reads that
table). Engine tests that need the card's behaviour use a fixture script in `crates/engine/tests/rules/fixtures/`. Keep the card's script a
list of those effects (CLAUDE.md rules 4 and 5). A rule the spec does not decide is a new ruling note with a proving test (CLAUDE.md rule 3).
No escalation step.

## 7. What cannot be done on Linux

Every card needs its voice lines **rendered**: `apps/web/src/audio/voice-assets.test.ts` (B35, B36, B37, R501) expects an `.m4a` per line in
`apps/web/public/audio/voice/` and a manifest entry. `pnpm --filter @jackioh/web gen:voice` renders them with macOS `say` and `afconvert` or
Windows SAPI and ffmpeg, and has no Linux backend, so a card added from Linux or CI fails about twenty tests there until a person renders them.
`node apps/web/scripts/gen-voice.mjs --check` lists what is missing. Write the `card-audio.json5` entry (the text needs no tool) and say in the
PR that the audio is owed. A hook that is only an effect renders nothing, so it is never owed. The asset tests run in the daily super run, not
on a pull request.
