# Training Grounds — design brief for the implementing agents

2026-10-28 · written from the designer's list of the same day (tracker #2193, `JackiOh_TrainingGrounds_Cards.md`), by
the foundation (#2194)

> **Status: the brief, and the plan of record.** SPEC is still the only source of rules (CLAUDE.md). The set-level
> rules this brief decides are in SPEC with the foundation ([[R2194000]] to [[R2194002]], [[§8.11]] and
> [[§7]]); every card's row goes into §8.11 (its token's also into [[§7]]); each card's mechanics become SPEC when the
> part that builds them lands, with its rulings. This file records, for every card and every new mechanic, what the
> designer wrote (verbatim in the source note), what it means in this engine, every place the text is unclear with the
> reading chosen, and the order the work is built in. It follows `docs/meditative-plus-set.md`'s shape.
>
> Proposed rulings are numbered **TG-P3-1, TG-P4-2, …**, by the part that builds them. They are not `R<n>` rows: the
> part that proves one takes an R-number from its own issue's block (TG8) and names its test after it. **⚠ designer**
> marks a reading the designer (or the owner) should confirm; the build does not wait for it (TG6 lists each with the
> default the build uses).
>
> Every card was read against the engine on `main` at `9be40af` (2026-10-28), before Stinkyscape's release (#2178)
> merged, and re-checked when the foundation merged the release (`f5c99a2`): Stinkyscape now ships, so the one held set
> is Training Grounds, and no card's reading moved. File and line references are as of `9be40af` unless the
> foundation's own change moved them; a part re-checks its own before it builds. The set shipped with its release
> (#2204, [[R2204000]]); where this brief says "while the set is held", it describes the build before then.

## Contents

- [TG0. What the owner asked for](#tg0-what-the-owner-asked-for)
- [TG1. Reading this brief](#tg1-reading-this-brief)
- [TG2. The set: ids, numbers, tags, rarity and the census](#tg2-the-set-ids-numbers-tags-rarity-and-the-census)
- [TG3. Built on main behind R1420](#tg3-built-on-main-behind-r1420)
- [TG4. The systems, and the decisions](#tg4-the-systems-and-the-decisions)
- [TG5. Cards, card by card](#tg5-cards-card-by-card)
- [TG6. Readings for the designer](#tg6-readings-for-the-designer)
- [TG7. Sound, visuals, the AI and the release](#tg7-sound-visuals-the-ai-and-the-release)
- [TG8. The plan: parts, issues, order and ruling blocks](#tg8-the-plan-parts-issues-order-and-ruling-blocks)

---

## TG0. What the owner asked for

The tracker #2193, quoting the owner on 2026-10-28:

> After Stinkyscape lands overhaul the tutorial with these cards and remake it wherein the set of cards they pull from is
> below; PLEASE also add them to the normal game too. Also make sure that the tutorial is not losable & that it explains
> the mechanics to the user in a way where they can discover stuff about it themselves.

The designer's list (`JackiOh_TrainingGrounds_Cards.md`, verbatim) holds:

- **40 numbered cards**, #1–#40, each with a Radiant face, and **one token**, #39.1 Training-Grounds Golem, which #39
  summons. Rarity by number: #1–#15 Common, #16–#27 Rare, #28–#36 Epic, #37–#39 Legendary, #40 Mythic.
- **20 Core cards to snapshot** as #41–#60, "separate entities".
- The rule that **Legendary and Mythic Training Grounds cards do not show up in the tutorial until its last game**.

The owner's decisions, from the tracker:

- **The set ships to the normal game** as the newest set ([[R1371]]), appended last when it ships, with the
  consequences for new accounts' starting rewards, the active set track, the newest-set ribbon and the "newest set"
  deck option.
- **The snapshots keep their Core names**, and a deck may hold **the Core card or its snapshot, not both**. They keep
  their Core rarities, none Legendary or Mythic, and copy the Core script's code into files of their own, frozen so a
  later Core patch does not move them.
- **The tutorial has five lessons**, dealt from Training Grounds cards, Legendary and Mythic only in the fifth. It
  **cannot be lost** (an engine rule: the lesson's human hero cannot fall below 1 health, a handicap field as [[R290]]'s
  `heroHealth`), and its coach lets the player **discover** each mechanic. It replaces [[§9.10]]'s four Core lessons.
- Where the list is silent, Hearthstone semantics (CLAUDE.md rule 3).

## TG1. Reading this brief

- **Numbers.** "§" is SPEC; "TG3" is this brief's section TG3; "TG #N" is Training Grounds card N (BUILD M13 writes "TG
  N"); "S #N" a Stinkyscape card, "M+ #N" a Meditative+ card, "M #N" a Meditative card, "C+ #N", "C #N" and "Core #N"
  (or "#N" in SPEC) the other sets' cards. The parts are named P1 to P13 by the tracker's table (TG8): P1 the
  foundation #2194, P2 #2195 to P13 #2206.
- **A card entry** (TG5) has the heading (set, number, name), the header line (id · cost, type and tags · rarity ·
  stats base → Radiant · the part that builds it, with its issue), the designer's text flattened onto one line (`~~~`
  between the faces, the designer's `{…}` notes kept), and then:
  - **Text** and **Radiant**: the proposed `base.text` and `radiant.text`, house style, params as `{key}`, a line break
    written " / ";
  - **Engine**: the closest shipped card and the verbs of `crates/engine/src/effects/*.rs`, the hooks of `script.rs`,
    the helpers of `query.rs` and `zones.rs`, the rulings it stands on, and **NEW** for anything the engine lacks, with
    the part that builds it;
  - **Rulings**: the readings its builder needs, as proposals TG-P*n*-*k*, or "none new";
  - **Numbers**: the declared params ([[R1748003]]), key, base value, ↑ when more is better for the controller, the
    Radiant value and step where they differ;
  - **Check**: the must-pass clauses, numbered, base face then Radiant face (BUILD M13 lists each as a distinct test),
    then [[R275]]'s verdict, the list's typos and the ⚠ designer items;
  - **Class**: A keywords or one primitive, B composed primitives, C a system a part builds, D a subsystem of its own.
- **Words.** House style as [[R366]], [[R373]], [[R432]] and [[R1175000]] set it: "deck" for the library, "Tribute",
  labelled abilities on lines of their own (Cry:, Death:, Start of turn:, Aura:), "(N) Cost" for a cost as a noun and
  "costs (N)" as a verb, straight apostrophes, never "start of your turn" in a sentence. The designer's "Cry & Death:"
  is written as two labelled lines. The designer's "Training-grounds" and "Tutorial-Grounds" in a card's text are the set,
  written "Training Grounds" ([[R2194001]], TG4 D4); the two names that carry it keep his hyphen (#39, #39.1).

## TG2. The set: ids, numbers, tags, rarity and the census

**The set.** `SetName::TrainingGrounds`, written **`"Training Grounds"`**: the first set name with a space, so every
place that turns a set name into an id segment drops the space (TG4 D10). It follows Stinkyscape in `CATALOG_SETS`.

**Ids.** Card `traininggrounds-NNN` (three digits), the token `traininggrounds-039-1`; index strings `"39"`, `"39.1"`.
Scripts in `crates/cards/src/scripts/training_grounds/cNNN_slug.rs` (`c039_1_training_grounds_golem.rs`):
`crates/cards/build.rs`'s `home_of` (`build.rs:662-676`) finds a set by its folder name with its underscores removed, so
`training_grounds` owns the segment `traininggrounds`.

**Numbers.** The designer's 1 to 40 stand, with no hole and no repeat; #39.1 is the one token; #41–#60 are the snapshots
(P2), numbered in the designer's order:

| # | Core card | # | Core card |
| --- | --- | --- | --- |
| 41 | core-002 Bigot | 51 | core-043 Big Felinor |
| 42 | core-005 Stockpile | 52 | core-049 Snom Bunny Mind Control |
| 43 | core-008 Mr. Vanilla | 53 | core-061 Prejudiced Postdoc |
| 44 | core-012 Duplicating Felinors | 54 | core-066 The Rock |
| 45 | core-014 Jlockeed's Weapons | 55 | core-072 Reminisce |
| 46 | core-016 Hit Job | 56 | core-089 Corpse Eater |
| 47 | core-019 Midrange Menace | 57 | core-039 Recycling Initiative |
| 48 | core-013 Jlockeed Shredder-10 | 58 | core-044 True Strike |
| 49 | core-023 Reoccurring Dream | 59 | core-047 Fig of Life |
| 50 | core-026 Glowy Jelly Bean | 60 | core-058 Rush Token Farm |

**Names.** One name names one card ([[R381]], [[R739]]): none of the 41 names of #1–#40 and #39.1 is a shipped or held
card's (checked against all 710 entries). #39 shares the bare "Call to Chaos" with the five editions, by design (R279's
parenthesis rule). The snapshots shared their Core cards' names by the owner's decision of 2026-10-28 (P2's one exception, TG4); since issue #2511 they are the Legacy cards, each named `Legacy: <its Core card's name>`, so no name is shared (R2511000).
Near names, none a clash: TG #5 Midlane Menace and Core #19 Midrange Menace (also TG #47), TG #12 Fortify and M #22.1
Fortify Mind (TG5 #12's R279 note), TG #26 Path of Exile and C #10 Exile, TG #35 Corpse Recycler and Core #89 Corpse
Eater. The designer's casing is kept ("Group leader", "In a hurry", "Dino-size"), as Core's "Right-house defender" is.

**Types.** The designer's, with #28 Occidental and #29 Oriental, whose header lines name no type, read as **Units**
(each prints stats). #1–#40: **23 Units** (#1–#9, #16–#18, #21–#23, #25, #28–#30, #32, #33, #36, #38), **10 Spells**
(#10–#12, #15, #26, #27, #34, #37, #39, #40), **2 Traps** (#13, #14), **4 Field Spells** (#19, #20, #31, #35), **1 Field
Trap** (#24); the token #39.1 a Unit. The snapshots: 9 Units, 9 Spells, 2 Field Spells, as their Core cards.

**Tags.** The designer's; the set added **no tag** until issue #2511 gave the snapshots **Legacy** (R2511000; the census below is the one before it, and Legacy adds 20) (every other one is in the `Tag` union: Human, Felinor, CN, Jelly Bean, Call
to Chaos, Quickdraw; and the snapshots' Jlockeed and Fruit). #39.1 carries Token. Census of #1–#40 and #39.1: Human 11
(#1, #2, #3, #5, #8, #9, #18, #25, #28, #32, #38), Felinor 4 (#17, #22, #23, #33), CN 1 (#29), Jelly Bean 1 (#27), Call
to Chaos 1 (#39), Quickdraw 1 (#40), Token 1 (#39.1). The snapshots add Human 4, Felinor 2, Jlockeed 2, Jelly Bean 1,
Fruit 1: the set's tags once it ships are Human 15, Felinor 6, Jlockeed 2, Jelly Bean 2, CN 1, Call to Chaos 1,
Quickdraw 1, Fruit 1, Token 1.

**Rarity.** The designer's bands for #1–#40: **15 Common** (#1–#15), **12 Rare** (#16–#27), **9 Epic** (#28–#36),
**3 Legendary** (#37–#39), **1 Mythic** (#40). #39.1 is `rarity: "Token"` with `printedRarity: "Legendary"`, as S
#150.1 Stinkyscape Golem. The snapshots keep their Core rarities: **11 Common** (#41, #42, #43, #45, #46, #47, #48, #54,
#58, #59, #60), **7 Rare** (#44, #49, #50, #51, #52, #53, #55), **2 Epic** (#56, #57). The set: **60 cards and 1
token, 26 Common, 19 Rare, 11 Epic, 3 Legendary, 1 Mythic**.

**Radiant stats.** [[R275]]'s stat half holds on every Unit face as printed, none needing an exception in
`radiant_standard.rs`: the designer often triples (#1 15/6, #4 18/21, #6 9/9, #8 42/14) and #9 gives 14/17. The effect
half is judged card by card in TG5 and `docs/radiant-audit.md`; three faces are the designer's word (#4, #6, #8) and
four go past the band (#10, #11, #18, and #23's four bodies), each kept as printed (TG6).

**The census** (`crates/tools/src/catalog.rs`'s `sets()` and `expected_tag_count`, and
`crates/cards/tests/cross/catalog.rs`'s `TRAINING_GROUNDS` fixture, written once by the foundation from the table in
§8.11): every entry the set holds must be one of these indices, with its row's name, cost, type, tags, rarity and stats.
The foundation writes the rows of all 61 entries; #41–#60 wait for P2 (a held set's row waits for its card).

## TG3. Built on main behind R1420

The set is built on `main`, part by part, behind [[R1420]]'s gate, as Meditative+ ([[R1178000]]) and Stinkyscape
([[R1857000]]) were, until the release (P11, #2204) ships it. While it is held:

- `SetName::TrainingGrounds` is in `CATALOG_SETS` (`crates/engine/src/wire/catalog_types.rs:303`), after Stinkyscape,
  and not in `SHIPPED_SETS` (`:287`).
- A pool that names no set draws from the sets that ship (`crates/engine/src/catalog.rs:653-662`); one that names
  Training Grounds, or a card by id, reaches it. So TG #20, #36, #38 and #39 find the set's cards from the day they land,
  and no other card, AI deck, determinization or random cast meets one until the set ships. #40's "a random card" and
  #39's "Cast a random Call to Chaos" name no set, so they hold no TG card until then.
- [[§9.4]]'s L3 and `create_game` refuse a TG card in a deck (`state.rs:2267`, `validator.rs:357`); no lean names the
  set; the Almanac and the deck builder show no card of it; the server neither seeds nor serves it; no voice file is
  owed for its lines.
- `patches check` and `patches ship` read the catalog without it: **a part adds no pending fragment**, and the release
  claims every entry in one fragment.
- The testkit previews the set on one thread (`preview_sets(&[SetName::TrainingGrounds])`,
  `crates/engine/src/testkit/preview.rs:37`); the fuzz tool previews every set (`crates/tools/src/fuzz.rs:343`
  `FUZZ_POOL`, `:2196`), so every card is fuzzed from the day it lands; the golden traces deal from the shipped sets
  (`SHIPPED_FUZZ_POOL`, `:357`), so no part moves a recorded game.
- `catalog check` holds the set to its shape (TG2's indices, ids, rarity ceilings), keeps it out of the totals and the
  tag census, and prints it as `Training Grounds (not shipped yet, R1420): …`; the cards crate's fixture holds each
  entry to its row and lets a row wait for its card.

**What a part never touches**: the golden traces, the seed-pinned tests, the pending fragments, the shipped totals and
tag counts, and other sets' cards' behaviour. A part whose change moves a golden trace changed something that ships:
it says what and why, or it is a bug.

**What the foundation lays** (P1, #2194, `docs/ADDING_CARDS.md`'s "A card of a set that has not shipped"):
the source list on `main`; this brief; the set (`SetName`, `CATALOG_SETS`, the TypeScript mirrors and
`apps/web/src/wire/catalog.ts`, `build.rs`'s `SETS`); the catalog entries #1–#40 and #39.1 with their `flavor`, `loc`,
params and `dealsDamage`, each with a stub script marked `STUB (#2194)`; the fixture rows, `RARITY_COUNTS`,
`SET_SIZES`, the tools census; §8.11, the §7 row for #39.1, §1's and §8's intros; BUILD M13; the `docs/radiant-audit.md`
rows, `card-audio.json5`, `flavour.json` and `chinese.json` entries; the set-wide rulings; and the gate's tests
(`sets_not_shipped.rs`'s shapes for a held set, and a test that the fuzz pool previews the set while the golden pool
does not).

**What a card part adds**, per card: its script replacing the stub, with its tests (every M13 clause a distinct test,
every `param()` number tuned by one), its `loc` updated (`cargo jackioh catalog loc <path>`), and corrections to its own
§8.11, §7, BUILD M13 and `docs/radiant-audit.md` rows where its build decided a detail differently. New verbs, statuses
and hooks get their [[§6]] or [[§10]] rows and ruling notes in the part that builds them.

## TG4. The systems, and the decisions

One part builds each system, with its engine tests through fixture scripts (`crates/engine/tests/rules/`), its [[§6]]
or [[§10]] rows and its rulings. Read against the engine; the issue's first table is corrected where the read differed
(marked *corrected*).

| System | What it is | Cards | Part |
| --- | --- | --- | --- |
| The held set | `SetName::TrainingGrounds` behind [[R1420]]; the census, the fixture, the stubs; the gate's tests | all | P1 #2194 |
| The snapshots | NEW: one deck holds a Core card or its snapshot, not both: a pair table read by both validators (`state.rs:2213`, `validator.rs:367`, `:681`) and every dealer (`fuzz.rs:391`, the server's `deal_random_deck*` and duos `ai_deck`, `crates/ai/src/deck.rs:655` `build_ai_deck`, the deck builder). The name exception is in `r739_one_name_names_one_card` (`crates/cards/tests/cross/catalog.rs`, *corrected*: not the Meditative+ test `r1178002_…`), `r1303_names_are_unique` (`chinese.rs:385`) and R279's `named_by` (`references.rs:188`), with the Jlockeed (`catalog.rs:8262`) and Jelly Bean (`:8561`) pins | #41–#60 | P2 #2195 |
| An adjacency aura | `AuraEntry.applies` with `zones::slot_of`, `zones::adjacent` and `zones::is_adjacent_card`: a script, no engine change (*corrected*): `layers::aura_sources` (`layers.rs:223`) already takes only a pile's top and `layers.rs:489-494` give a dormant card no aura ([[R13]]). Its ruling and engine test are still owed (R1081000's standing Units) | #21 | P4 #2197 |
| A terminator for a self-recursive Death | Cumulative growth, M #94 Shrinking Felinor's mirror (`base_stats_of` + `summon { statsOverride }`, [[R1181]]): each Trigor is strictly larger, so no fixed aura loops it; growth alone is no terminator, which is R2197004 (a Trigor a Trigor's Death summoned this turn summons nothing, marked as R2206000 says) | #25 | P4 |
| After you summon a Unit | S #23 Warp Disruptor's `arrival()` ([[R1875003]]) turned to its own side or both sides | #18, #30 | P4 |
| Die at the end of this turn | NEW: summoned tokens destroyed at the end of the turn they arrive, whoever's: `summon { rememberAs }`, then a `delay { at: { phase: end, player: THIS_TURN } }` per token watching it (CN-Virus's [[R350]], not [[R241]]), or a `summon` rider. Brittle 1 is not it (`BRITTLE_FIRST_TICK_TURNS` = 2, `brittle.rs:62`), nor Temporary ([[R637]]) | #15 | P5 #2198 |
| Cleave on a board pick | No verb (*corrected*): Core #34 Collateral Damage's Radiant `exile_adjacent_to` already is it, and `adjacent_to` + `set_radiant` for #27; a ruling, the board twin of [[R1563001]] | #26, #27 | P5 |
| A single-target set-stats verb | NEW: `set_stats { target, attack, health }` over the private `set_stats_on` (`effects/health.rs:88`), with `set_stats_all`'s guards ([[R1185002]], [[R23]]) | #34 | P5 |
| An exact copy | NEW: an `add_to_hand { copyOf }` rider carrying what `copy_riders` (`add_to_hand.rs:185`) leaves out: [[R57]]'s field riders (buffs, granted keywords, Vanilla) and the price (`costMod`, `costOverride`) | #37 | P5 |
| A Trap's printed Pierce and Trample | Stated on the Trap's hits (`ignoreArmor`, `trample: true`): `damage.rs:650-656` reads a source's Trample only for an attacking Unit or a Spell. Spec only: one line each in [[§6.1]]'s Pierce and Trample rows (*corrected*: missing from the issue's table) | #13, #14 | P6 #2199 |
| A Field Trap that sets cheaper hand cards to (0) | No new verb (*corrected*): `CardAnnounced`'s `cost_paid` ([[R56]]), `remember`, then `set_cost_override` per hand card under it. A Field Trap fires again ([[§5.1]]), so "On reveal" is every firing | #24 | P6 |
| An attack count across turns | NEW (*corrected*): the count lives in the Field Spell's own `memory` (`remember`, reset by [[R78]]); what is missing is an "after one of your Units attacks" hook for a card that is not the attacker, run after that combat's state check (`Script.after_attack`, `script.rs:1152`, is the attacker's only; the public events are `attackDeclared` and `attackCancelled`) | #31 | P6 |
| Call to Chaos (Training-Grounds Edition) | NEW: its table (`CHAOS_TG_EFFECTS` on `subsystems/call_to_chaos.rs`'s roller) and an every-other-turn rest-of-game add (`for_rest_of_game` runs every own turn, [[R458]]). Not new (*corrected*): "for the rest of the game your Units have X" (`give_units_keyword`, [[R1859007]]), Refresh ([[R364]]), the recast (`cast_random_call_to_chaos`). S #4.6 The Big Crunch's editions are a hard-coded `EDITIONS` list (`c004_6_the_big_crunch.rs`), not R1846003's lists | #39 | P7 #2200 |
| The tutorial seat | A lesson's human hero cannot fall below 1 health (a handicap field as [[R290]]'s `heroHealth`); Legendary and Mythic held out of a lesson's random pools until lesson 5 | tutorial | P8 #2201 |

Everything else maps onto a shipped card's pattern; TG5 names each one.

**Decisions across the set** (these win over any entry in TG5 and over the part issues' tables):

- **D1 The gate.** Training Grounds is held by [[R1420]] until P11 ships it ([[R2194000]], as
  [[R1857000]]); no part adds a pending fragment or moves a golden trace.
- **D2 The newest set.** When it ships it is appended to `SHIPPED_SETS` last and becomes the newest set ([[R1371]]), the
  owner's choice.
- **D3 R275's stat half** holds as printed (TG2); the effect half is judged per card, the designer's numbers kept where
  they go past the band (TG6).
- **D4 The set's name in a card's text.** The designer's "Training-grounds", "Training-Grounds" and "Tutorial-Grounds"
  (#36) are the set, written "Training Grounds" in the text; a pool that names the set (`{ "set": "Training Grounds" }`)
  works while it is held and takes in the snapshots #41–#60 once P2 lands them ([[R2194001]]; the
  owner's issue). It never holds a token unless it says so (#39.1 is in none).
- **D5 Legendary and Mythic** are in every set-named pool in the normal game; only a tutorial lesson before the fifth
  holds them out (P8).
- **D6 The snapshots** are separate entries, frozen at the foundation's commit (P2's TG-P2-1 … TG-P2-5, below TG5's #60).
- **D7 Rulings**: each TG proposal becomes an R-number from its part's block (TG8).
- **D8** The ⚠ readings are the build's defaults (TG6).
- **D9** The §8.11, §7, BUILD M13 and `docs/radiant-audit.md` rows are written by the foundation from this brief; a part
  corrects its own rows when its build decides otherwise, and says so.
- **D10 A set name with a space.** Every place that turns a set name into an id segment drops the space as well as `+`
  and `-`: the cards crate's fixture `set_of_id` (`crates/cards/tests/cross/catalog.rs`, which `landed()` reads), fixed by the
  foundation, `crates/server/tests/api/stats.rs:490` with the release.
- **D11 New state stays out of shipped games' hashes.** A field the set adds to the state, an instance or a view is
  optional and skipped when absent, so a game that never uses it serializes, hashes and replays as before.

## TG5. Cards, card by card

Entries in number order, the token after its card, the snapshots last. Engine names are the Rust ones. Where a decision
in TG4 (D1–D11) differs from an entry, the decision wins.

#### Training Grounds #1 · Spikey Steven
`traininggrounds-001` · (1) Unit, Human · Common · 5/2 → 15/6 · built by P3 (#2196)

> **Designer:** (1) Spikey Steven, Unit, Human, #1 · 5/2 ~~~ 15/6

- **Text:** (none)
- **Radiant:** (none)
- **Engine:** A vanilla Unit: an empty script (`Script::default()` on both faces), as Core #8 Mr. Vanilla (`core-008`, 4/4 → 12/12) and C #86 Genn (`classic-086`, 14/14 → 42/42). The catalog row has `keywords: []` and empty text on both faces.
- **Rulings:** none new.
- **Numbers:** none (stats only).
- **Check:** (1) base: a 5/2 Human with no text and no keyword; (2) it is summoning-sick on its turn (no Rush); (3) Radiant: a 15/6 Human with no text. R275: stat half met (3× both); a vanilla has no text to scale, so the tripled body is the whole upgrade, as `docs/radiant-audit.md` reads #8 Mr. Vanilla, C #86 Genn and M #100 Greaser: **Meets**. Name: no clash in the catalog (Core #65.1 Spikey Pillow is a different name; R381 holds).
- **Class:** A.

#### Training Grounds #2 · Personal Trainer
`traininggrounds-002` · (1) Unit, Human · Common · 3/2 → 6/4 · built by P3 (#2196)

> **Designer:** (1) Personal Trainer, Unit, Human, #2 · 3/2 / **Cry:** Give a different ally **Human** +2/+2 ~~~ 6/4 / **Cry:** Give a friendly **Human** +4/+4

- **Text:** Cry: Give one of your other Humans +{buff}/+{buff}.
- **Radiant:** Cry: Give one of your other Humans +{buff}/+{buff}.
- **Engine:** C+ #57 Book of Stats' shape (`classicplus-057`): one declared target, `TargetDecl::target(1, 1, …)` with `aim: Some(TargetAim::Help)` ([[R656]]), then `buff { target: { of: "chosen" }, attack, health }` (`effects/buff.rs:111`, a permanent layer-4 buff, [[§10.4]]). The filter is Core #61 Prejudiced Postdoc's with `side: "ally"`: `{ "side": "ally", "of": ["unit"], "tags": ["Human"] }`; `tags_of` counts a granted Human tag ([[R923]]). The pick travels in the `play` action and is validated while the Trainer is still in hand ([[R81]], [[R90]], [[§10.5]] step 1), so it can never pick itself: "different" needs no `excludeSelf` (Postdoc's header says the same), though adding `"excludeSelf": true` as Core #22 Carnivorous Cube does is harmless. No other Human on your side: the Cry fizzles and the Unit still enters ([[R90]]).
- **Rulings:** TG-P3-1, [[R2196000]]: the Radiant's "a friendly Human" is read as the base's "another": the pick is made in hand ([[R81]]), so neither face can pick the Trainer, and the Radiant face's upgrade is the doubled stats and the doubled buff. ⚠ designer: if the Radiant was meant to buff itself when it is your only Human, that needs a self-fallback the engine does not have (a Cry with no legal pick fizzles, [[R90]]); default: as written above.
- **Numbers:** `buff` 2 → 4, ↑, step 1, min 1, `cry: true` (C+ #20 Mushroom Power's and S #15 Arg Arg Defender's shape).
- **Check:** (1) base: played with another friendly Human chosen, that Human gets +2/+2 permanently (one `buffed` event); (2) an enemy Human and a friendly non-Human are not legal picks; (3) with no other friendly Human it can be played, the Cry fizzles, the Trainer enters 3/2; (4) a Unit given the Human tag is a legal pick (R923); (5) the buff reads `param("buff")` (a test that tunes it). Radiant: (6) 6/4; (7) the chosen friendly Human gets +4/+4; (8) it cannot pick itself (TG-P3-1). R275: stats doubled; +2/+2 → +4/+4 (2×): **Meets**. Typos: "ally"/"friendly" → "your other Humans"; trailing space after "+2/+2".
- **Class:** A.

#### Training Grounds #3 · Group leader
`traininggrounds-003` · (1) Unit, Human · Common · 4/1 → 8/2 · built by P3 (#2196)

> **Designer:** (1) Group leader, Unit, Human, #3 · 4/1 / **Aura:** Your other units have +1 attack ~~~ 8/2 / **Aura:** Your other units have +2 attack

- **Text:** Aura: Your other Units have +{attack} Attack.
- **Radiant:** Aura: Your other Units have +{attack} Attack.
- **Engine:** C+ #46 Felinor Flagbearer's `felinor_aura(include_self: false)` without the tag test, and Core #65.1 Spikey Pillow's attack-only `StatMod`: `aura_hook` returning one `AuraEntry` whose `applies` is `unit.controller == ctx.self_.controller && in_units_row_or_standing(ctx.state, unit) && unit.id != ctx.self_.id` and whose `mod_` is `StatMod { attack: Some(param("attack")), .. }`. Computed on read ([[§10.4]] layer 5): the +1 goes the instant the leader leaves; a Unit standing in the backrow is one of "your Units" ([[R1081000]]); a card dormant under a Stack pile is not ([[R13]]). Its own side only; the enemy's Units never.
- **Rulings:** none new.
- **Numbers:** `attack` 1 → 2, ↑, step 1, min 1 (Core #14 Jlockeed's Weapons' key).
- **Check:** (1) base: each other friendly Unit on the field has +1 Attack, health unchanged; (2) the leader itself does not (4/1); (3) enemy Units are untouched; (4) a Unit played after it gets the bonus; (5) the bonus goes when the leader leaves the field; (6) the aura reads `param("attack")`. Radiant: (7) 8/2; (8) +2 Attack to each other friendly Unit, not itself. R275: stats doubled; +1 → +2 (2×): **Meets**. Typos: "units"/"attack" → "Units"/"Attack". Name: the designer writes "Group leader" in lower case; house style keeps a designer's casing (Core #3 Right-house defender), ⚠ designer: "Group Leader"? default: as printed.
- **Class:** A.

#### Training Grounds #4 · Boulder Fist Ogre
`traininggrounds-004` · (2) Unit · Common · 6/7 → 18/21 · built by P3 (#2196)

> **Designer:** (2) Boulder Fist Ogre, Unit, #4 · **6/7** / Rush ~~~ **18/21** / Rush

- **Text:** Rush
- **Radiant:** Rush
- **Engine:** Keywords only ([[§6.1]]): an empty script; the catalog prints `keywords: [{ "kind": "Rush" }]` on both faces, as Core T-rush Rush Token (`core-t-rush`) and S #68 Stratos (`stinkyscape-068`, "Keywords only: its script is empty"). Rush lifts sickness for Unit targets only (`combat.rs:487–493`: "Rush cannot hit the hero on its summon turn").
- **Rulings:** none new.
- **Numbers:** none (keywords and stats).
- **Check:** (1) base: 6/7 with Rush; (2) it may attack an enemy Unit the turn it is played; (3) it may not attack the enemy hero that turn; (4) it may attack the hero from its next turn. Radiant: (5) 18/21 with Rush, same (2)–(4). R275: stat half met (3×); effect half: **Designer's word (effect half)**: a keyword-only Unit whose Radiant face adds no keyword, where R275 asks one more or a stronger one ("Rush to Charge"), recorded as `docs/radiant-audit.md` records S #68 Stratos and S #128 Jlockeed's Yamato Cannon; the tripled body stands in for it as it does for #8 Mr. Vanilla. The foundation's radiant-audit row judges it so (#2194). ⚠ designer: R275's own example would make the Radiant face Charge (as Core #11 Tempo Timmy, C #73 Nurse Cleaver); default: Rush as printed. Name: "Boulder Fist Ogre" no clash in the catalog.
- **Class:** A.

#### Training Grounds #5 · Midlane Menace
`traininggrounds-005` · (2) Unit, Human · Common · 3/3 → 6/6 · built by P3 (#2196)

> **Designer:** (2) Midlane Menace, Unit, Human, #5 · 3/3 / **Cry:** if this was placed in the Middle lane gain +5/+5 and **Rush** ~~~ 6/6 / **Cry:** if this was placed in the Middle lane gain +10/+10 and **Rush**

- **Text:** Cry: If this is in midlane, it gains +{buff}/+{buff} and Rush.
- **Radiant:** Cry: If this is in midlane, it gains +{buff}/+{buff} and Rush.
- **Engine:** C #22 Mid Runner's midlane test (`classic-022`, [[R685]]): `slot_of(state, &self_).lane` in `midlane_lanes_of(state, ctx.controller)` and `in_units_row_or_standing` ([[R1081000]]). The board has `UNIT_ZONES` = 5 lanes (`config.rs:117`, [[§3.1]]), so midlane is lane 3, computed by `zones::midlane_lanes` (the centre of an odd count, both centres of an even one), never hardcoded. Then `buff { target: { of: "self" }, attack, health }` and `grant_keyword { target: { of: "self" }, keyword: { kind: "Rush" } }` (`effects/buff.rs:228`; the self-grant as S #91 The Propagation's Wasp grants itself Flying; Core #4 Gary the Gambler grants itself Rush through `flip_coin_keyword`). A granted Rush works the turn it lands: sickness is `summoned_turn == turn` (`combat.rs:203`) and the Rush exemption reads the Unit's layered keywords at the attack (`combat.rs:206–210`, [[§10.4]]), so it may attack an enemy Unit at once, never the hero that turn. The Cry runs only when the card is played or cast ([[R1]]): a summon into midlane, a copy or a Reborn fires nothing.
- **Rulings:** none new. Reading: "placed in the Middle lane" is the lane it is in as its Cry resolves (Mid Runner's reading), which for a played Unit is the zone its player chose. ⚠ designer: none.
- **Numbers:** `buff` 5 → 10, ↑, step 1, min 1, `cry: true` (C+ #57 Book of Stats' 5 → 10).
- **Check:** (1) base played into lane 3: 8/8 with Rush (one `buffed`, one `keywordGranted`); (2) it may attack an enemy Unit that turn; (3) it may not attack the enemy hero that turn; (4) played into lanes 1, 2, 4 or 5: 3/3, no Rush, summoning-sick; (5) the buff is permanent (kept after the turn), and the Rush stays as a granted keyword; (6) a copy or summon of it in lane 3 gains nothing (no Cry, R1); (7) the buff reads `param("buff")`. Radiant: (8) in lane 3: 16/16 with Rush; (9) elsewhere 6/6, nothing. R275: stats doubled; +5/+5 → +10/+10 (2×): **Meets**. Typos: "if" → "If"; "Middle lane" → midlane ([[R685]]'s word, C #22's text); "gain" → "it gains". Name: **near-clash with Core #19 Midrange Menace** (`core-019`) — not the same name, so R381 passes, but one word apart; ⚠ designer (owner) to confirm it is meant.
- **Class:** B.

#### Training Grounds #6 · Chosen by the Sun
`traininggrounds-006` · (1) Unit · Common · 3/3 → 9/9 · built by P3 (#2196)

> **Designer:** (1) Chosen by the Sun, Unit, #6 · **3/3** / **Divine Shield** ~~~ **9/9** / **Divine Shield**

- **Text:** Divine Shield
- **Radiant:** Divine Shield
- **Engine:** Keywords only ([[§6.1]]): an empty script; `keywords: [{ "kind": "Divine Shield" }]` on both faces. Divine Shield negates the first damage instance and is lost ([[§4.4]] step 1). As M #70.1 Ready… I'm's base face, Core #3 Right-house defender.
- **Rulings:** none new.
- **Numbers:** none (keywords and stats).
- **Check:** (1) base: 3/3 with Divine Shield; (2) the first hit deals nothing and removes the shield; (3) the next hit damages it; (4) summoning-sick (no Rush). Radiant: (5) 9/9 with Divine Shield, same (2)–(3). R275: stat half met (3×); effect half: **Designer's word (effect half)**, as #4 (a keyword-only Unit with no keyword more; S #68 Stratos, S #128 Yamato Cannon's record). The foundation's radiant-audit row judges it so (#2194). ⚠ designer: one keyword more on the Radiant (e.g. Reborn, as C+ #6 Wrong-House Attacker's patch did); default: as printed. Name: the pair with #7 Chosen by the Moon; no clash in the catalog.
- **Class:** A.

#### Training Grounds #7 · Chosen by the Moon
`traininggrounds-007` · (1) Unit · Common · 3/3 → 6/6 · built by P3 (#2196)

> **Designer:** (1) Chosen by the Moon, Unit, #7 · **3/3** / **Taunt** / **Armor 3** ~~~ **6/6** / **Taunt** / **Armor 6**

- **Text:** Taunt, Armor 3
- **Radiant:** Taunt, Armor 6
- **Engine:** Keywords only ([[§6.1]]): an empty script; `keywords: [{ "kind": "Taunt" }, { "kind": "Armor", "n": 3 }]`, Radiant `n: 6`. Armor X is the numbered keyword `Keyword::Armor { n }` (`wire/catalog_types.rs:469`), reducing each damage instance by n at [[§4.4]] step 2 and summing with other sources (Defense Position's 1, auras: [[§10.4]]); Pierce skips it ([[R346]]). Printed exactly as M #92 Unan (`meditative-092`, "Armor 3" → "Armor 6") and C+ #19.1 Top Loser (Armor 3 → 6). Taunt is printed, so it has Taunt in either position.
- **Rulings:** none new.
- **Numbers:** none: Armor 3 is a numbered keyword, tuned by the Nerf/Buff X row ([[R386]]), not a param ([[R1748003]]).
- **Check:** (1) base: 3/3 with Taunt and Armor 3; (2) enemies must attack it before a non-Taunt target; (3) a 5-damage hit deals 2; a 3-damage hit deals 0 (`damageAbsorbed`); (4) in Defense Position its Armor is 4; (5) a Pierce hit ignores the Armor. Radiant: (6) 6/6, Taunt, Armor 6; (7) a 7-damage hit deals 1. R275: stats doubled; Armor 3 → 6 (2×), as M #92 Unan's row: **Meets**.
- **Class:** A.

#### Training Grounds #8 · Clobberer
`traininggrounds-008` · (3) Unit, Human · Common · 14/7 → 42/14 · built by P3 (#2196)

> **Designer:** (3) Clobberer, Unit, Human, #8 · 14/7 ~~~ 42/14

- **Text:** (none)
- **Radiant:** (none)
- **Engine:** A vanilla Unit: an empty script, as #1 and C #86 Genn (`classic-086`).
- **Rulings:** none new.
- **Numbers:** none.
- **Check:** (1) base: a 14/7 Human with no text or keyword; (2) summoning-sick on its turn. Radiant: (3) 42/14 with no text. R275: stat half met (attack 3×, health 2×). Effect half: a vanilla's whole upgrade is its body, and the precedents triple both stats (#8 4/4 → 12/12, C #86 14/14 → 42/42, M #100 7/7 → 21/21; Genn's row: "28/28 would meet only the stat half"); here health only doubles: **Designer's word (effect half)**. ⚠ designer: 42/21 would triple both; default: 42/14 as printed. Name: no clash.
- **Class:** A.

#### Training Grounds #9 · Ravager
`traininggrounds-009` · (4) Unit, Human · Common · 7/7 → 14/17 · built by P4 (#2197)

> **Designer:** (4) Ravager, Unit, Human, #9 · 7/7 / **Rush** / On kill gain +5/+5 ~~~ 14/17 / **Rush** / On kill gain +10/+10

- **Text:** Rush / Whenever this destroys a Unit, it gains +{buff}/+{buff}.
- **Radiant:** Rush / Whenever this destroys a Unit, it gains +{buff}/+{buff}.
- **Engine:** C+ #19.5 Bot Loser's kill trigger (`classicplus-019-5`, "Whenever this destroys a Unit, it gains +{attackGain} Attack."), also M #95.1 CN Golem's: `TriggerDef::new(…, &[GameEventType::Destroyed], …)` matching `GameEvent::Destroyed { killer_id: Some(killer), .. }` with `killer == me.id`, returning `buff { target: { of: "self" }, attack, health }`. The killer is [[R42]]'s: the hit (or Poisonous hit) that first dooms the victim, Cleave included; a destroy effect credits nobody. A field trigger: it fires only while the Ravager still stands as the `destroyed` is dispatched, so a mutual combat death gains nothing ([[R1245]]). There is no "on kill" hook in `script.rs`; the trigger is the pattern. Not `query::killer_of` (`query.rs:287`): that reads a dying card's `last_damaged_by` for the victim's own Death hook. Rush as #4.
- **Rulings:** none new. Reading: "On kill" is "Whenever this destroys a Unit" (the shipped wording); a kill on defence (an enemy attacking into it and dying) counts, as for Bot Loser; the enemy hero's death is no Unit's kill (the game ends). ⚠ designer: Hearthstone's Overkill/Honorable Kill fire on your own turn only; default: any turn, as Bot Loser.
- **Numbers:** `buff` 5 → 10, ↑, step 1, min 1 (not `cry`: the trigger reads it on the field).
- **Check:** (1) base: 7/7 with Rush, may attack a Unit the turn it is played, not the hero; (2) attacking and killing a Unit it survives: 12/12 (the damage taken stays); (3) a Unit that attacks it and dies: +5/+5; (4) if it dies in the same combat it gains nothing (R1245); (5) a hit that does not kill: no gain; (6) a hit on a Divine Shield: no gain; (7) two kills: +10/+10; (8) the buff reads `param("buff")`. Radiant: (9) 14/17 with Rush; (10) each kill +10/+10. R275: stat half met (2×; health 17 > 14); +5/+5 → +10/+10 (2×): **Meets**. Typos: "On kill gain" → "Whenever this destroys a Unit, it gains"; ⚠ designer: 17 health is not a doubling of 7 (14/14?); default: 14/17 as printed (it meets the stat half). Name: no clash.
- **Class:** A.

#### Training Grounds #10 · Fan of Knives
`traininggrounds-010` · (1) Spell · Common · built by P5 (#2198)

> **Designer:** (1) Fan of Knives, Spell, #10 · Deal 1 damage to all enemies, draw 1 / **Pierce** ~~~ Deal 2 damage to all enemies, draw 1 / **Pierce** / **Echo 1**

- **Text:** Pierce / Deal {damage} damage to all enemies. Draw {draw|card|cards}.
- **Radiant:** Echo 1, Pierce / Deal {damage} damage to all enemies. Draw {draw|card|cards}.
- **Engine:** S #13 Asphyxiants Cloud less its next-Spell clause (`stinkyscape-013`: `damage_all { amount, side: "enemy", heroes: true }`, then `draw { count }`; its Radiant `StaticFlags { echo: Some(1) }`), with C+ #21 Whirlwind's Pierce (`classicplus-021`: `keywords: [Pierce]` on both faces and `"ignoreArmor": true` in `damage_all`). "All enemies" is every enemy Unit plus the enemy hero, one damage instance each ([[R51]]), the list snapshotted and every hit landing before one state check ([[R59]]); `damage_all`'s doc (`effects/damage.rs:110–123`). Pierce: a Spell's is printed on its face and read off it while it resolves; the effect states it too (`ignoreArmor`), so an Echo repeat with no source left still pierces ([[R346]], Core #44 True Strike), and it skips the enemy hero's Armor as well as a Unit's. Divine Shield still stops a hit. Echo 1 is the static flag, written "Echo 1, Pierce" as S #13's Radiant "Echo 1, Poisonous"; the second resolution deals the damage and draws again ([[§6.2]] Echo, `echo.rs`). Spell Damage raises each hit ([[§4.4]]). "Echo 1" in its text is no reference to C #57 Echo ([[R381]]'s rules-word list). Catalog: `dealsDamage: true` ([[R1748001]]: a Buff can then add only Lifesteal, Pierce being printed).
- **Rulings:** none new.
- **Numbers:** `damage` 1 → 2, ↑, step 1, min 1; `draw` 1 → 1, ↑, step 1, min 1 (S #13's two params). Echo 1 is a numbered keyword, not a param ([[R1748003]]).
- **Check:** (1) base: 1 damage to each enemy Unit and to the enemy hero; (2) no friendly Unit or hero is hit; (3) an enemy Unit with Armor 1 still takes 1 (Pierce), and the enemy hero's Armor is skipped too; (4) a Divine Shield absorbs its hit; (5) Units it takes to 0 die together after the sweep (R59); (6) then its caster draws 1; (7) Spell Damage +1 makes each hit 2; (8) damage and draw read `param()`. Radiant: (9) 2 damage to each enemy and the enemy hero, piercing; (10) draws 1; (11) Echo 1: the whole resolves a second time (4 damage and 2 cards in all), the repeat piercing. R275: damage 2× **and** an added Echo — about 4× the damage and 2× the draw, above R275's 2–2.5× band: **Meets** (over the standard; the designer's word for the excess, ⚠ designer). Typos: the bold comma ("enemies**,** draw 1") → a sentence; "draw 1" → "Draw {draw|card|cards}". Name: no clash in the catalog.
- **Class:** B.

#### Training Grounds #11 · Thwomp
`traininggrounds-011` · (2) Spell · Common · built by P5 (#2198)

> **Designer:** (2) Thwomp, Spell, #11 · Deal 10 damage to a unit ~~~ Deal 100 damage to a unit

- **Text:** Deal {damage} damage to a Unit.
- **Radiant:** Deal {damage} damage to a Unit.
- **Engine:** C #12 Book of Blood's script without its Lifesteal (`classic/c012_book_of_blood.rs`): one `TargetDecl` `{ side: "any", of: ["unit"] }`, then `damage { to: { of: "chosen" }, amount: param("damage") }`; both faces run one script, `param` reads the face's number. C #83 Flame Lance (10, step 2) is the same shape with `trample`. A Spell source, so §4.4 step 0 adds its caster's Spell Damage (`damage::deal_damage` → `spell_damage_for`); Divine Shield (step 1), Armor (step 2), Indestructible (step 4) and a lethal replacement (step 4a, [[R1704001]]) apply. An Immune to Spells or Immutable Unit is never offered ([[R781001]]). Catalog: `dealsDamage: true` ([[R1748001]]). Nothing NEW.
- **Rulings:** none new: "a unit" is a Unit on either side, never a hero ([[R90]]).
- **Numbers:** damage 10 ↑ (Radiant 100), step 2, min 1 ([[R1748003]]; Flame Lance's step for a 10).
- **Check:** (1) it offers every Unit on both sides and no hero; (2) one hit of 10 on the chosen Unit; (3) its caster's Spell Damage raises it (an Arch Mage: 13); (4) Armor lowers it, a Divine Shield takes it whole and is spent; (5) `damage` reads through `param()`, a Buff steps it 2; Radiant: (6) one hit of 100. R275: 100 against 10 (10×), above the 2–2.5× band, the designer's printed number kept as the Stinkyscape brief kept S's 10× (⚠ designer: 20 would be R275's double; 100 reads as "destroy unless shielded"). Typos: none.
- **Class:** A.

#### Training Grounds #12 · Fortify
`traininggrounds-012` · (1) Spell · Common · built by P5 (#2198)

> **Designer:** (1) Fortify, Spell, #12 · Give a unit +3 health, +1 Armor, and **Taunt** ~~~ Give a unit +6 health, +2 Armor, **Taunt** and **Divine Shield**

- **Text:** Give a Unit +{health} Health, +{armor} Armor and Taunt.
- **Radiant:** Give a Unit +{health} Health, +{armor} Armor, Taunt and Divine Shield.
- **Engine:** S #10.3 Power Word Shield's target and buff (`stinkyscape/c010_3_power_word_shield.rs`: `TargetDecl::target(1, 1, { side: "any", of: ["unit"] })`, `buff { target: { of: "chosen" }, health }`, current and max), then S #15 Arg Arg Defender's grants (`grant_keyword { keyword: { kind: "Taunt" } }`, Radiant also `{ kind: "Divine Shield" }`) and `grant_keyword { keyword: { kind: "Armor", n: armor } }` on the chosen Unit. `effects/buff.rs:177` `grant_onto`: Armor and Lucky stack (pushed again, summed with the printed Armor, [[§6.1]] Armor X), Taunt is a set (no second copy), a granted Divine Shield clears `divine_shield_spent`. An Indestructible Unit never has Taunt ([[R347]]). Nothing NEW.
- **Rulings:** none new: "a unit" is either side ([[R90]], as S #10.3); the Armor is the Unit's own Armor X, kept until it leaves the field ([[R78]]).
- **Numbers:** health 3 ↑ (Radiant 6); armor 1 ↑ (Radiant 2), a granted number like M #22 Mind Games' `armor` and C+ #74's `brittleGain`, not the card's own numbered keyword ([[R482]], [[R1748003]]).
- **Check:** (1) it offers any Unit on either side, no hero; (2) the Unit gains +3 Health, current and max; (3) its Armor rises by 1 on top of its printed Armor (a Chosen by the Moon: Armor 4); (4) it gains Taunt, once on a Unit that has it, never on an Indestructible one; (5) both numbers read through `param()`; Radiant: (6) +6 Health; (7) +2 Armor; (8) Taunt; (9) Divine Shield, a spent one coming back. R275: 2× both numbers and a Divine Shield rider (as S #15). Typos: "health" is Health. Name: no clash (M #22.1 Fortify Mind and S #145 Fortify Flesh are other names, [[R381]]), but R279's matcher (`crates/cards/tests/cross/references.rs:160`, a whole-word match) reads M #22 Mind Games' "Add a Fortify Mind" as naming TG #12 once the set ships: P5 adds "Fortify Mind" to `RULES_PHRASES` (`references.rs:60`, [[R961]]'s mechanism) and to the web's twin (`apps/web/src/cards/refs.ts:48`), keeping the designer's name.
- **Class:** B.

#### Training Grounds #13 · Explosive Runes
`traininggrounds-013` · (1) Trap · Common · built by P6 (#2199)

> **Designer:** (1) Explosive Runes, Trap, #13 · When a unit attacks your hero, deal 3 damage to all enemies {Resolves before the attack resolves, such that if the unit attacked your hero with <=3 life an no armor it would die and your hero wouldn't take damage} ~~~ When a unit attacks your hero, deal 6 damage to all enemies. **Pierce**

- **Text:** Reveals when an enemy Unit attacks your hero: Deal {damage} damage to all enemies.
- **Radiant:** Pierce / Reveals when an enemy Unit attacks your hero: Deal {damage} damage to all enemies.
- **Engine:** C+ #1 Doom Shroom's trigger (`classic_plus/c001_doom_shroom.rs`): `TriggerDef` on `GameEventType::AttackDeclared` with `attacks_your_hero` as its `when` (not `forced`, attacker the opponent's, `attack_target_of` = `AttackTarget::Hero` of the controller), in [[§4.2]] step 4's trap window, after the exertion is spent ([[R44]]). Its effect is S #13 Asphyxiants Cloud's `damage_all { amount: param("damage"), side: "enemy", heroes: true }` (`effects/damage.rs:124`: each enemy Unit, then the enemy hero, one §4.4 instance each, the list snapshotted, [[R59]]); the trap is the source (no Spell Damage: step 0 reads only a Spell). Then the state check collects an attacker at 0, and step 5 resolves no combat because the attacker no longer stands as declared ([[R220]]): the hero takes nothing, the designer's note exactly, as Doom Shroom's exile calls the attack off. Radiant: `ignoreArmor: true` stated on the hits ([[R346]]; `damage::pierces` reads the flag first), and Pierce printed on the face. NEW (spec only, P6): [[§6.1]]'s Pierce row names a Unit, a Spell and a Field Spell; add "a Trap that prints it, stated by its effect".
- **Rulings:** TG-P6-1: "a unit attacks your hero" is an enemy Unit's declared attack on it (a forced attack opens no window, [[R121]]; only a forced attack can aim at your own hero); "all enemies" is the enemy hero and every enemy Unit ([[R1181001]]'s reading), the attacker among them; an attacker that survives (Divine Shield, Armor, more than 3 Health, a Reborn return being a new arrival that declared nothing, [[R220]]) goes on only if it still stands as declared; with My Pawn or Doom Shroom set too, [[R68]]'s order.
- **Numbers:** damage 3 ↑ (Radiant 6).
- **Check:** (1) face-down, hidden until it fires ([[R33]], [[R97]]); (2) an enemy Unit's declared attack on your hero reveals it before damage; (3) 3 damage to each enemy Unit and the enemy hero, all before one state check; (4) an attacker with 3 Health, no Armor and no Divine Shield dies and your hero takes no damage, its exertion spent; (5) a 4-Health or Armor 1 attacker survives and its attack resolves; (6) an attack on one of your Units, or a forced attack on your hero, leaves it set; (7) `damage` reads through `param()`; Radiant: (8) 6 damage each; (9) Pierce: an attacker with 6 Health and Armor 3 dies and the hero takes nothing. R275: 6 against 3 (2×) and Pierce, a rider. Typos: "an no armor" is "and no Armor"; "life" is Health. No `dealsDamage` (a Trap, [[R1748001]]). ⚠ designer: the name is Hearthstone's Explosive Trap's effect; see the notes.
- **Class:** B.

#### Training Grounds #14 · Explosive Trap
`traininggrounds-014` · (1) Trap · Common · built by P6 (#2199)

> **Designer:** (1) Explosive Trap, Trap, #14 · After your opponents plays a unit, deal 6 damage to it. **Trample** ~~~ After your opponents plays a unit, deal 12 damage to it. **Trample, Pierce**

- **Text:** Trample / Reveals when your opponent plays a Unit: After it resolves, deal {damage} damage to it.
- **Radiant:** Trample, Pierce / Reveals when your opponent plays a Unit: After it resolves, deal {damage} damage to it.
- **Engine:** Core #41 Sheepish's trigger (`core/c041_sheepish.rs`): `GameEventType::CardResolved` (after the Cry and every Echo, [[R427]]; a play, so not a token, a Recruit or a Reborn, [[R1]], [[R61]]), `when` the player is the opponent and the card a Unit (`def_of(...).type_ == CardType::Unit`); then C #5 Tesla's hit (`classic/c005_tesla.rs`: `damage { to: { of: "instance", instanceId } }`) with `trample: true` stated. A Trap's printed Trample does nothing by itself: `damage.rs:650-656` reads a source's Trample only for an attacking Unit's hit or a Spell source (`restrictions::is_spell_source` is `CardType::Spell` alone), so the stated flag carries it; the excess past the Unit's Health, after its Armor, goes to its controller's hero, the opponent's ([[§4.4]] step 9, [[R1750000]], [[R63]]). Divine Shield (step 1) or Indestructible (step 4) stops the hit and nothing tramples (C #83's note). Radiant: `ignoreArmor: true` ([[R346]]). An Immutable Unit is still hit (a Trap is not one of [[R781001]]'s three types). NEW (spec only, P6): [[§6.1]]'s Trample row ("a Spell that prints it") and Pierce row gain "a Trap that prints it, stated by its effect".
- **Rulings:** TG-P6-2: a Trap's printed Trample (and Pierce) is its text's hits' own, stated by the effect, the excess to the hit Unit's controller's hero. TG-P6-3: a played Unit that is no longer on the field when the play resolves (it left in its own resolution, or an earlier answer took it, [[R174]]) leaves the trap set, the condition in `when` ([[R99]]); Sheepish fires there only for its Radiant rider ([[R120]]). A Unit cast by an effect counts as played ([[R70]]).
- **Numbers:** damage 6 ↑ (Radiant 12), step 2.
- **Check:** (1) face-down until it fires; (2) the opponent plays a Unit: after its Cry resolves the Unit takes 6, the trap consumed; (3) a 2-Health Unit takes 2 and the opponent's hero 4; (4) a 3-Health Armor 2 Unit takes 3 and the hero 1 (Armor first); (5) a Divine Shield Unit loses the shield and nothing tramples; (6) a summoned token, a Recruit, a Spell, a backrow card or your own Unit leaves it set; (7) a played Unit gone before it resolves leaves it set; (8) `damage` reads through `param()`; Radiant: (9) 12 damage; (10) Pierce: an Armor 3 Unit with 4 Health takes 4 and the hero 8. R275: 12 against 6 (2×) and Pierce. Typos: "opponents plays" is "opponent plays". No `dealsDamage` ([[R1748001]]: the mark is refused on a Trap).
- **Class:** B (and the two §6.1 lines).

#### Training Grounds #15 · In a hurry
`traininggrounds-015` · (2) Spell · Common · built by P5 (#2198)

> **Designer:** (2) In a hurry, Spell, #15 · Summon 3 rush tokens that die at the end of this turn {give them brittle 1} ~~~ Summon 3 Radiant rush tokens that die at the end of this turn {give them brittle 1}

- **Text:** Summon {tokens|Rush Token|Rush Tokens}. At the end of this turn, destroy them.
- **Radiant:** Summon {tokens|Radiant Rush Token|Radiant Rush Tokens}. At the end of this turn, destroy them.
- **Engine:** The Rush Token is `core-t-rush` ([[§7]]: 3/3 Rush; Radiant 6/6 Rush, Cleave), summoned one `summon { defId }` per token as C+ #15 Conjure Rush Token does (`classic_plus/c015_conjure_rush_token.rs`), the Radiant face with `radiant: true`, into the leftmost open zones ([[R64]]), fewer on a full row ([[§3.2]]). Brittle 1 is not it: `brittle.rs:62` `brittle_due` is `turn >= since + BRITTLE_FIRST_TICK_TURNS` (2, `config.rs:440`), ticking at its controller's start of turn, so a Brittle 1 token crumbles at the start of your next turn and blocks through the opponent's. Temporary is not it either: a hand keyword, nothing on the field ([[§6.1]], [[R637]]). NEW (P5): "at the end of this turn, destroy them". It composes from shipped verbs: each `summon` with `rememberAs` (`effects/summon.rs:130`, [[R1126]]), then a `for_each_card` over the remembered ids (its `cards` closure runs as the clause begins, S #15's `beside`) making one `delay { at: { phase: "end", player: THIS_TURN }, watch: <id>, step }` each, CN-Virus's "the end of the turn it was cast on, whoever's" (`core/c090_1_cn_virus.rs`, [[R350]]) and the K-Pop Fanatic watch that forgets a card that left the field ([[R174]]), whose step runs `destroy`. P5 may add a `summon` rider instead (a `destroyAtEndOfTurn`), with its [[§6.3]] row and an engine test either way.
- **Rulings:** TG-P5-1: "die at the end of this turn" is a destroy at [[§2.2]]'s end-of-turn delayed-effect point of the turn the Spell resolves, whoever's (after end-of-turn triggers and the end-of-turn trap window, before cleanup, [[R62]]; [[R350]], not [[R241]], which would arm nothing for a cast on the opponent's turn); each token still on the field is destroyed (a destroy: Indestructible refuses it, a Death would fire), and one that left is forgotten ([[R174]]; a token ceases to exist off the field, [[R11]]). The tokens print no Brittle: the designer's {brittle 1} names the end he wants, which Brittle 1 does not give (⚠ designer). Default for a token the opponent stole before the end: still destroyed (⚠ builder: [[R171]] makes a change of control an entry; the watch must not drop it).
- **Numbers:** tokens 3 ↑ (Radiant 3), as C+ #15's `tokens`.
- **Check:** (1) three 3/3 Rush Tokens in the leftmost open unit zones, fewer on a nearly full board, none on a full one; (2) they may attack Units this turn and not the hero (Rush, [[§4.2]] step 2); (3) at the end of this turn each still on the field is destroyed, after end-of-turn triggers and traps; (4) none stands on the opponent's turn; (5) one already dead is not counted again and nothing else is destroyed; (6) cast on the opponent's turn, they go at that turn's end; (7) `tokens` reads through `param()`; Radiant: (8) three Radiant Rush Tokens, 6/6 Rush and Cleave; (9) destroyed the same way. R275: tokens at 2× stats with Cleave (meets, as Rush Token's own Radiant face). Typos: "rush tokens" is Rush Tokens; name "In a hurry" kept as printed (⚠ designer: or "In a Hurry"; shipped names keep the designer's case, Core #3 "Right-house defender").
- **Class:** C (die at end of turn).

#### Training Grounds #16 · Celestial Caller
`traininggrounds-016` · (4) Unit · Rare · 3/3 → 6/6 · built by P3 (#2196)

> **Designer:** (4) Celestial Caller, Unit, #16 · 3/3 · Divine Shield · Taunt · Armor 3 · Cry: Summon a Chosen by the Sun to the left of this and a Chosen by the Moon to the right of this ~~~ 6/6 · Divine Shield · Taunt · Armor 6 · Cry: Summon a Radiant Chosen by the Sun to the left of this and a Radiant Chosen by the Moon to the right of this

- **Text:** Divine Shield, Taunt, Armor 3 / Cry: Summon a Chosen by the Sun to the left of this and a Chosen by the Moon to the right of this.
- **Radiant:** Divine Shield, Taunt, Armor 6 / Cry: Summon a Radiant Chosen by the Sun to the left of this and a Radiant Chosen by the Moon to the right of this.
- **Engine:** Keywords are catalog data (`Keyword::DivineShield`, `Taunt`, `Armor { n: 3 }`). The Cry is M #49 YileGPT's named-lane summon (`meditative/c049_yilegpt_tamed.rs`: `across` reads `Zone::Field { lane }` off `ctx.live_self()` as the Cry resolves, then `summon { defId, lane, radiant }`), with lane − 1 for `traininggrounds-006` and lane + 1 for `traininggrounds-007`, lanes 1-based from the controller's seat ([[§3.1]]; M #61 Joon Jorker's `at.lane + 1` and `at.lane >= zones` bound, [[R1060]], `UNIT_ZONES` 5). `SummonArgs.lane`: an aimed summon fails on an occupied or reserved zone ([[R47]], [[R64]]) and enters a Locked one ([[R688]]). No new verb.
- **Rulings:** TG-P3-2, [[R2196001]]: "to the left of this" is the unit zone one lane lower on this card's side, "to the right" one lane higher ([[R1060]]'s right, mirrored); lane 1 has no left and lane 5 no right, so that summon does nothing and draws nothing ([[R129]]); each summon is aimed and fails alone on an occupied or reserved zone; the Sun first, then the Moon; this card's lane is read as the Cry resolves, and if it has left the field nothing is summoned. The summoned cards are ordinary cards of the set, new and owned by the controller ([[R12]]), with no Cry (a summon is no play, [[R1]]).
- **Numbers:** none (Armor 3 is a numbered keyword, [[R482]]; "a" is no count).
- **Check:** (1) printed Divine Shield, Taunt, Armor 3; (2) played into lane 3 with lanes 2 and 4 open: a Chosen by the Sun (3/3 Divine Shield) in lane 2, a Chosen by the Moon (3/3 Taunt, Armor 3) in lane 4; (3) in lane 1: only the Moon, in lane 2; in lane 5: only the Sun, in lane 4; (4) an occupied lane 2: no Sun, the Moon still lands; (5) a Locked neighbour zone takes its summon; (6) summoned rather than played: no Cry; Radiant: (7) 6/6, Armor 6; (8) a Radiant Chosen by the Sun (9/9) and a Radiant Chosen by the Moon (6/6, Armor 6). R275: stats 2×, Armor 6, and Radiant summons. Typos: none.
- **Class:** B.

#### Training Grounds #17 · Flaming Felinor
`traininggrounds-017` · (2) Unit, Felinor · Rare · 3/6 → 6/12 · built by P3 (#2196)

> **Designer:** (2) Flaming Felinor, Unit, Felinor, #17 · 3/6 · Cry: deal 3 damage to all other units ~~~ 6/12 · Cry: deal 6 damage to all enemy units

- **Text:** Cry: Deal {damage} damage to all other Units.
- **Radiant:** Cry: Deal {damage} damage to all enemy Units.
- **Engine:** `damage_all` (`effects/damage.rs:124`, a `BoardScope` flattened in) with `{ side: "any", excludeSelf: true }` on the base face (Core #100 Ceaseless Void's `exile_all { side: "any", ..., excludeSelf: true }`; `effects/targets.rs:315`) and `{ side: "enemy" }` on the Radiant face; no `heroes`. One hit each, snapshotted, before one state check ([[R59]]). The source is the Unit (its text's hit: no Spell Damage, no Trample, [[R1750000]]; its Pierce or Lifesteal would apply, it has none).
- **Rulings:** none new: "all other units" is every other Unit on the field, both sides, Hearthstone's reading; no hero.
- **Numbers:** damage 3 ↑ (Radiant 6), `cry: true` ([[R1748000]]).
- **Check:** (1) Cry: 3 damage to each other Unit on both sides; (2) not to itself; (3) no hero takes damage; (4) Units it brings to 0 die together at one state check; (5) `damage` reads through `param()`; Radiant: (6) 6/12; (7) 6 damage to each enemy Unit; (8) your other Units are untouched. R275: stats 2×, damage 2× and your side spared (a rider). Typos: "units" is Units.
- **Class:** A.

#### Training Grounds #18 · Knife Juggler
`traininggrounds-018` · (1) Unit, Human · Rare · 3/2 → 6/4 · built by P4 (#2197)

> **Designer:** (1) Knife Juggler, Unit, Human, #18 · 3/2 · Pierce · When you summon a unit deal 1 damage to a random enemy {spawn voice line: put this apple on your head} ~~~ 6/4 · Pierce · When you summon a unit deal 3 damage to a random enemy

- **Text:** Pierce / After you summon a Unit, deal {damage} damage to a random enemy.
- **Radiant:** Pierce / After you summon a Unit, deal {damage} damage to a random enemy.
- **Engine:** No shipped card answers a Unit arriving on its own side. S #23 Warp Disruptor's `arrival()` (`stinkyscape/c023_warp_disruptor.rs`, [[R1875003]]) flipped: `GameEventType::Summoned` (no `exitsFrom`, `Row::Units` or `stands_in_backrow`, [[R1081000]]) or, for a played Unit, `CardResolved` once it resolves, its Cry first (C #5 Tesla's reading), with `controller == ctx.controller` and the instance not this card. The hit is C+ #3 Second Amendment Snake's (`classic_plus/c003_second_amendment_snake.rs`): `damage_split { amount, perHit: amount, among: "enemies" }` (`effects/split.rs`; the enemy hero or a standing enemy Unit, match rng, [[R60]]), or `damage_random_enemies { count: 1, amount }`. Its printed Pierce applies to this hit too: `damage::pierces` (`damage.rs:392`) reads the source's keyword for every hit, unlike Trample ([[R346]], [[R1750000]]). Voice: `card-audio.json5` `play` "Put this apple on your head" (#2194 scope 7).
- **Rulings:** TG-P4-1: "when you summon a unit" is any Unit arriving on your side however summoned (played, a token, a Recruit, a copy, Reborn), a played one after it resolves, Hearthstone's "after you summon"; this card's own arrival does not count, a steal ([[R171]]) or an animating card ([[R383]]) is no summon; "a random enemy" is the enemy hero or an enemy Unit, Hearthstone's; one hit per arrival, each followed by a state check ([[R59]]).
- **Numbers:** damage 1 ↑ (Radiant 3).
- **Check:** (1) printed Pierce; (2) you play a Unit: after its Cry, 1 damage to a random enemy (hero or Unit), drawn from the match rng; (3) a token summoned by your Spell, a Recruit and a Reborn return each trigger it; (4) its own arrival does not; (5) a Unit the opponent summons does not; (6) Pierce: an Armor 2 enemy Unit with 1 Health dies to it; (7) two arrivals (Celestial Caller's Cry) give two hits; (8) `damage` reads through `param()`; Radiant: (9) 6/4; (10) 3 damage per arrival. R275: stats 2×, damage 3× (above the band, the designer's number; ⚠ designer). Typos: "put" is "Put" in the voice line.
- **Class:** B.

#### Training Grounds #19 · Extra Rations
`traininggrounds-019` · (1) Field Spell · Rare · built by P6 (#2199)

> **Designer:** (1) Extra Rations, Field Spell, #19 · Start of turn: Draw 1 ~~~ Start of turn: Draw 2, they cost (1) less

- **Text:** Start of turn: Draw {draw|card|cards}.
- **Radiant:** Start of turn: Draw {draw|card|cards}. They cost ({discount}) less.
- **Engine:** Core #6 Mana Well's `start_of_turn` hook (`core/c006_mana_well.rs`, its controller's start of turn, before the turn's draw, [[R62]]): base `draw { count: param("draw") }`; Radiant M #80 Aluneth's `draw_priced { costMod: -discount }` once per draw (`meditative/c080_aluneth.rs` `draws_discounted`), each pricing only the card it put in the hand (a burned or cast-on-draw card is not discounted, [[§2.4]], [[R58]]). Nothing NEW.
- **Rulings:** none new: "they" are the drawn cards that reached the hand, as Aluneth and M+ #3; the cost change is `costMod` ([[R65]]: never below (0)), kept until the card leaves the hand ([[R766]]).
- **Numbers:** draw 1 ↑ (Radiant 2); discount 1 ↑ (Radiant face only, `tunedOn: "radiant"`, [[R749]], [[R1431]]).
- **Check:** (1) at your start of turn, before your draw, you draw 1; (2) nothing at the opponent's start of turn; (3) nothing on the turn it is played; (4) a full hand burns it, an empty deck takes fatigue; (5) `draw` reads through `param()`; Radiant: (6) draws 2; (7) each costs (1) less; (8) a (0) card stays (0) and the rest of the hand is unchanged. R275: 2 draws at (1) less against 1 (2× and a rider). Typos: none.
- **Class:** B.

#### Training Grounds #20 · Recruitment Posters
`traininggrounds-020` · (2) Field Spell · Rare · built by P6 (#2199)

> **Designer:** (2) Recruitment Posters, Field Spell, #20 · Start of turn: Add a random Training-grounds card to your hand ~~~ Start of turn: Add a random Radiant Training-grounds card to your hand

- **Text:** Start of turn: Add a random Training Grounds card to your hand.
- **Radiant:** Start of turn: Add a random Radiant Training Grounds card to your hand.
- **Engine:** Core #59 Unbiased Immigration's `start_of_turn` (`core/c059_unbiased_immigration.rs`: `add_random_from_catalog { query, count: 1 }`, Radiant `radiant: true`) with C+ #75.1's set-named query (`{ "set": ["Classic", "Classic+"] }`) naming the new set: `{ "set": "Training Grounds" }` (`CatalogQueryArgs.set`, `catalog.rs:476`). `catalog.rs:655-660`: a pool that names a set draws from it, shipped or not, so it works while the set is held ([[R1420]], [[R1857000]]'s words). Tokens are left out by default (`token`/`withTokens` absent, so #39.1 never), and the generator by `excluding_def_id` (`catalog.rs:859`, [[R387]]). Needs `SetName::TrainingGrounds` from the foundation (#2194). Nothing else NEW.
- **Rulings:** The foundation's set-wide ruling ("a random Training-Grounds card" names the set, works while held, takes in the snapshots #41–#60 once part 2 lands). TG-P6-4: never Recruitment Posters itself ([[R387]]: only Call to Chaos names a pool that holds it); Legendary and Mythic cards are in the pool in the normal game, and only a tutorial lesson holds them out (P8).
- **Numbers:** none ("a" is no count, as Core #59).
- **Check:** (1) at your start of turn, before your draw, one random Training Grounds card is added, while the set is held; (2) only cards of the set, never a token, never this card; (3) none at the opponent's start of turn; (4) a full hand burns it ([[§2.4]]); (5) the draw is one match-rng pick ([[R60]]); Radiant: (6) the card is Radiant. R275: a Radiant card against a card, as Core #59 (meets). Typos: "Training-grounds" is the set name Training Grounds, not a tag.
- **Class:** A.

#### Training Grounds #21 · Flame Tongue Totem
`traininggrounds-021` · (1) Unit · Rare · 0/4 → 0/8 · built by P4 (#2197)

> **Designer:** (1) Flame Tongue Totem, Unit, #21 · 0/4 · Adjacent units have +3 attack ~~~ 0/8 · Adjacent units have +6 attack

- **Text:** Aura: Adjacent Units have +{attack} Attack.
- **Radiant:** Aura: Adjacent Units have +{attack} Attack.
- **Engine:** An aura (`Script.aura`, `aura_hook`, `AuraEntry { applies, mod_ }`, crates/engine/src/script.rs:388; [[§10.4]] layer 5) with `StatMod { attack: Some(param("attack")) }`, shaped as C+ #46 Felinor Flagbearer's `felinor_aura` (crates/cards/src/scripts/classic_plus/c046_felinor_flagbearer.rs). No adjacency aura ships today (every one of the 17 `aura_hook` scripts reads side, tags or position; `adjacent_to` is used only by effects: Core #34, Core #16, C+ #20, M #87, S #1). `applies` must read instance data only (layers.rs `mods_of`): the unit's slot off `instance.zone` (`zones::slot_of`, zones.rs:1823), `zones::adjacent(slot)` of the Totem's slot (zones.rs:112: lane N−1 and N+1, same side and row, never across sides), and `zones::is_adjacent_card(state, totem, unit)` (zones.rs:1711, [[R1081000]]: beside a Unit standing in the backrow only another standing Unit counts). Never the Totem itself. Stack piles need no code: `layers::aura_sources` (layers.rs:223) takes only a pile's top, so a Totem beneath a pile projects nothing, and layers.rs:489–494 give a dormant card (and a Dormant one, [[R1839000]]) no aura at all ([[R13]]). A Vanilla'd Totem projects nothing (layers.rs `aura_entries`). Nothing NEW in the engine: a script-only aura.
- **Rulings:** TG-P4-2: "Adjacent Units" are [[§3.1]]'s adjacency of the Totem's own zone: the top Unit of the pile in lane N−1 and in lane N+1 of its side and row, its controller's (an enemy Unit is never adjacent), not itself; a Totem standing in the backrow ([[R1081000]]) reaches only the Units standing beside it there; beneath a Stack it gives nothing and a Unit beneath one gets nothing ([[R13]]). The aura follows the Totem's zone as it stands, so a move, a rotation ([[R14]]) or a steal changes who has it at once.
- **Numbers:** attack 3 ↑ (Radiant 6, step 1, min 1).
- **Check:** (1) a Unit in lane N−1 and one in lane N+1 each read +3 Attack, Health unchanged; (2) the Totem itself reads 0; (3) a Unit two lanes away, an enemy Unit in lane N, and a backrow card read nothing; (4) a Unit beneath a Stack pile next to it gets nothing, and the top does; (5) the bonus leaves when the Totem leaves the field or when the neighbour moves away; (6) a Vanilla'd Totem gives nothing; (7) the number reads through `param()`. Radiant: (8) 0/8, +6 Attack to each neighbour. R275: 0 attack stays 0, health 8 = 2×4, +6 = 2×3: holds. Typos: none ("Adjacent units" → "Adjacent Units", "attack" → "Attack", "Aura:" label added, house style). Name: Hearthstone's card is "Flametongue Totem" (one word); the designer's spacing stands.
- **Class:** A (one aura).

#### Training Grounds #22 · Right-Hand Felinor
`traininggrounds-022` · (2) Unit, Felinor · Rare · 3/3 → 6/6 · built by P3 (#2196)

> **Designer:** (2) Right-Hand Felinor, Unit, Felinor, #22 · 3/3 · **Cry:** if placed to the right of a unit give this and that unit +3/+3 ~~~ 6/6 · **Cry:** if placed to the right of a unit give this and that unit +6/+6

- **Text:** Cry: If this is to the right of a Unit, give this and that Unit +{buff}/+{buff}.
- **Radiant:** Cry: If this is to the right of a Unit, give this and that Unit +{buff}/+{buff}.
- **Engine:** "Right" is a higher lane: [[§3]]'s table numbers a side's zones "Lane 1 to 5, left to right from the owner's seat", and the client draws no mirror: apps/web/src/game/Board.tsx renders `LANES` (contract.ts:46, `[1,2,3,4,5]`) as columns left to right, each column holding the opponent's backrow and units above yours (`FIELD_ROWS`), the same in both seats' clients (no `row-reverse` in the game CSS). So the Cry reads the zone to its left: `zones::slot_of(self)`, then `zones::card_at(state, ZoneSlot { lane: lane − 1, .. })` (zones.rs:226, a pile's top) on its own side and row, kept when it is a Unit acting there (not Dormant, [[R1839000]]; under [[R1081000]] only a standing Unit beside a Felinor standing in the backrow, `is_adjacent_card`). Then `buff { target: self, attack, health }` and `buff { target: { of: instance, instanceId }, … }` (effects/buff.rs:111, a permanent layer-4 buff). In lane 1 there is nothing to its left and nothing happens. Cry fires on a play only ([[R1]]): a copy or summon of it buffs nothing. Closest shipped: Core #12's Cry shape; no shipped Cry reads a neighbour's lane (C+ #20 and M #87 use `adjacent_to`, both sides of the card).
- **Rulings:** TG-P3-3, [[R2196002]]: "placed to the right of a Unit" is read as the Cry resolves: a Unit (the top of its pile) acts in the zone directly to this one's left, lane N−1 of its own side and row, as its owner sees the board. An enemy Unit is never beside it ([[§3.1]]). "That Unit" is that one; if it has left the field by then, nothing happens and the Felinor is not buffed either (the condition is false). Both Units get the buff, the Felinor first.
- **Numbers:** buff 3 ↑ (Radiant 6, step 1, min 1, cry).
- **Check:** (1) played into lane N with a friendly Unit in lane N−1: both gain +3/+3; (2) a Unit only in lane N+1 (to its right): nothing; (3) lane 1: nothing; (4) an enemy Unit in lane N−1 of the other side: nothing; (5) a Unit beneath a Stack in lane N−1 is not it, the pile's top is; (6) summoned or copied (no play): no Cry ([[R1]]); (7) the buff reads through `param()`. Radiant: (8) 6/6, +6/+6 to both. R275: 6/6 = 2×3/3, +6 = 2×3: holds. Typos: "if" capitalised; "placed" read as "is", the Cry checking where the card stands. ⚠ designer: "right" is the owner's right, lane N−1 being the left neighbour (the default); no mirrored reading exists in the client.
- **Class:** B (a lane read and two buffs).

#### Training Grounds #23 · Quadrupling Felinors
`traininggrounds-023` · (3) Unit, Felinor · Rare · 3/4 → 6/8 · built by P3 (#2196)

> **Designer:** (3) Quadrupling Felinors, Unit, Felinor, #23 · 3/4 · **Cry:** Summon 3 copies of this unit ~~~ 6/8 · **Cry:** Summon 3 copies of this unit

- **Text:** Cry: Summon {copies|copy|copies} of this.
- **Radiant:** Cry: Summon {copies|copy|copies} of this.
- **Engine:** Core #12 Duplicating Felinors's Cry (crates/cards/src/scripts/core/c012_duplicating_felinors.rs): `summon_copy(json!({ "of": { "of": "self" }, "player": "self" }))` (effects/summon.rs:668, `SummonCopyArgs` has no count), pushed `param("copies")` times. Each copy is [[R57]]'s: the Radiant flag, buffs, granted keywords, Vanilla state, `statsOverride` and tuning kept, damage and exertion reset; placed by [[R64]] (leftmost empty unlocked zone), one fizzling silently per missing zone ([[§3.2]]). Copies are summons and fire no Cry ([[R1]], [[§6.2]]), so the chain ends after one round (REVIEW A3.6's terminator, as Core #12). A source that has left the field (an answering Trap) copies nothing (summon.rs:676). Nothing NEW.
- **Rulings:** none new ([[R1]], [[R57]], [[R64]]). A Radiant Quadrupling Felinors' copies are Radiant 6/8 ([[R57]]), as Core #12's are.
- **Numbers:** copies 3 ↑ (Radiant 3, step 1, min 1, cry).
- **Check:** (1) on an empty row: four 3/4 Felinors, the copies in the leftmost free zones; (2) the copies' Cries do not fire (exactly three copies); (3) with two free zones left: two copies, no error; (4) a buff on the hand instance is carried, damage is not ([[R57]]); (5) summoned or copied itself: no Cry, no copies; (6) the count reads through `param()`. Radiant: (7) four Radiant 6/8s. R275: stats 2×, the three copies each 2× (Radiant, [[R57]]): holds, as Core #12. Typos: "this unit" → "this" (Core #12's wording). ⚠ designer (balance): four 3/4 bodies for (3) at Rare against Core #12's two for (2); it fills a board in one play (and gives TG #30 Floodgates +4 Armor).
- **Class:** A (Core #12's verb).

#### Training Grounds #24 · Goliath Situation
`traininggrounds-024` · (1) Field Trap · Rare · built by P6 (#2199)

> **Designer:** (1) Goliath Situation, Field Trap, #24 · **Reveals** when your opponent plays a unit, remember its cost. **On reveal:** cards in your hand that cost less than the remembered cost, cost (0) instead. ~~~ **Reveals** when your opponent plays a unit that costs (2) or more, remember its cost. **On reveal:** cards in your hand that cost less than or equal to the remembered cost, cost (0) instead.

- **Text:** Reveals when your opponent plays a Unit: Remember its Cost. Cards in your hand that cost less than that cost ({setCost}).
- **Radiant:** Reveals when your opponent plays a ({minCost}) Cost or more Unit: Remember its Cost. Cards in your hand that cost that much or less cost ({setCost}).
- **Engine:** A Field Trap ([[§5.1]]: "a Trap that stays after firing and can fire again"; traps.rs:12, :298: it stays face-up, [[R33]]), so it fires on every Unit the opponent plays while it is set, face-down first and face-up after. Trigger: `TriggerDef::new(…, &[GameEventType::CardAnnounced], …)` with the condition in `with_when` ([[R99]], [[R61]]), as C #10 Exile's `cheap_play` (crates/cards/src/scripts/classic/c010_exile.rs): `GameEvent::CardAnnounced { player, card_type, cost_paid, .. }` (wire/events.rs:669) with `player` the opponent and `card_type == Unit`; the cost is `cost_paid`, the price paid ([[R56]], [[R70]]: a free cast is 0). Radiant `when` adds `cost_paid >= param("minCost")`. Run: `remember { key, value: cost_paid }` on the trap (effects/memory.rs:35, [[§10.1]]), then for each card in its controller's hand (`zone_cards(…, OffFieldZone::Hand)`) whose `cost_now` (mana.rs:534, [[R65]], [[R396]]) is below it (Radiant: at or below), `set_cost_override { target: { of: instance, instanceId }, cost: setCost, inHandOnly: true }` (effects/cost.rs:121), as M+ #3 Wondrous Wand does. The effect needs only the event's number; `remember` keeps it on the face-up card so the next firing overwrites it and the view/log can show it. The hand cards' `costChanged` stays hidden from the opponent ([[R97]]). Closest shipped: C #10 Exile (trigger and cost), Core #18 Bread and Butter (a Field Trap that fires again). Nothing NEW.
- **Rulings:** TG-P6-5: the Field Trap fires on each Unit the opponent plays, at its announce; each firing is the "reveal" the text names and remembers the newest cost; "its cost" is the cost paid ([[R56]]); "cost less than" compares each hand card's cost as it stands ([[R65]], [[R396]]: an X card in hand costs 0, so it always qualifies), and the cards become (0) by a cost override (a later discount does not take them below 0; one that raises cost still adds, [[R65]]). Cards added to the hand later are untouched. A countered Unit still fired it (the announce happened).
- **Numbers:** setCost 0 ↓ (Radiant 0, min 0); minCost (Radiant only) 2 ↓ (tunedOn radiant, min 1): C #10 Exile declares its threshold the same way.
- **Check:** (1) set face-down; the opponent plays a (4) Unit: the trap reveals and stays face-up; (2) your (1), (2) and (3) hand cards cost (0), your (4) and (5) cards do not; (3) the opponent's later (6) Unit fires it again, reaching (5)s; (4) your own Unit, an opponent's Spell or a summon (no play) fires nothing; (5) a Unit played for less than its printed cost (a discount) is remembered at the price paid; (6) the opponent sees no hand card's cost. Radiant: (7) a (1) Unit leaves it set, a (2) Unit fires it; (8) cards costing exactly the remembered cost become (0) too. R275: (≤ against <) and no firing wasted on a 0–1 cost play, a scope broadened by one cost and a rider; weaker than 2×: ⚠ designer (the base face fires on a (1) Unit making only (0) cards (0) — wasted until a dearer Unit; the Radiant's gate is mostly the ≤). Typos: "unit" → "Unit", "costs (2) or more" → "a (2) Cost or more Unit" (house style), "On reveal:" is not a house label: folded into the Reveals clause. ⚠ designer: "On reveal" read as every firing of the Field Trap (default) and not only its first reveal; a (1) Field Trap that makes most of a hand (0) on every enemy Unit is very strong.
- **Class:** B (a trap trigger, remember, cost overrides).

#### Training Grounds #25 · Trigor
`traininggrounds-025` · (3) Unit, Human · Rare · 3/3 → 6/6 · built by P4 (#2197)

> **Designer:** (3) Trigor, Unit, Human, #25 · **Tribute 3** · 3/3 · Death: summon a **Trigor** with +3/+3 ~~~ **Tribute 3** · 6/6 · Death: summon a **Trigor** with +6/+6

- **Text:** Tribute 3 / Death: Summon a Trigor whose base stats are this one's +{grow}/+{grow}.
- **Radiant:** Tribute 3 / Death: Summon a Trigor whose base stats are this one's +{grow}/+{grow}.
- **Engine:** Tribute 3 is the play's price (`StaticFlags { tribute: Some(3) }`, as C #61 Plague Bringer Goliath's `TRIBUTE_COST`; [[§6.3]], paid at [[§10.5]] step 2; a zone its Tribute empties counts, [[R391]]). Death: M #94 Shrinking Felinor's mirror (crates/cards/src/scripts/meditative/c094_shrinking_felinor.rs): `base_stats_of(ctx.state, &this)` (query.rs:618, layer 1, the `statsOverride` the dying card wore, [[R1181]], [[R78]]) plus `grow`, then `summon { defId: ID, player: self, radiant: false, statsOverride: { attack, health } }` (effects/summon.rs:458): a fresh base Trigor, not a copy ([[R57]]: no buff, granted keyword or tuning carried), into the leftmost open unit zone ([[R64]]), no Cry, no Tribute paid (a summon, [[§6.3]]). A full row summons nothing. REVIEW A3.6 (REVIEW.md line 53) needs a terminator: a Death that summons itself loops within one state check ([[§4.5]], `STATE_CHECK_PASS_CAP`, a panic at state_check.rs:1158) if the new body dies at once, e.g. under Core #46 Suppressive Aura's −N/−N stacked or Embiggened. With the cumulative reading every body has 3 more health than the last, so a fixed aura stops the chain after ⌈N/3⌉ bodies; the literal "fresh Trigor +3/+3" body would be 6/6 every time and loop forever under a −6 aura. Nothing NEW in the engine.
- **Rulings:** TG-P4-3 (⚠ designer): "a Trigor with +3/+3" is a fresh base Trigor whose base stats are the dying one's +3/+3 (cumulative: 3/3, 6/6, 9/9, …), set as a stat override, as R1181 sets Shrinking Felinor's; the Radiant face summons a base body too, at the dying Radiant one's 6/6 +6/+6 = 12/12, continuing +3/+3 on the base face ([[R1828003]]'s pattern). This is the A3.6 terminator: each body is strictly larger, so no fixed aura or damage loops it. Rejected: a fresh 6/6 every time (Hearthstone's literal "summon a X with +3/+3"), which has no terminator under a −6 Health aura.
- **Numbers:** grow 3 ↑ (Radiant 6, step 1, min 1). Tribute 3 is a price (fixed, R1748003's "a price … names").
- **Check:** (1) can't be played without 3 Units to Tribute; (2) destroyed at 3/3: a base Trigor 6/6 arrives in the leftmost open zone; (3) that one destroyed: a 9/9; (4) buffs on the dying one are not carried (base stats only); (5) the new one fires no Cry and pays no Tribute; (6) a full row: none summoned; (7) under an aura that takes 7 Health, the chain stops by itself (6/6 dies, 9/9 survives as 9/2); (8) exiled or bounced: no Death, no summon; (9) the growth reads through `param()`. Radiant: (10) 6/6; its Death summons a base 12/12, whose Death summons 15/15. R275: 6/6 = 2×, +6 against +3: holds. Typos: "Death:" not bolded, "summon" capitalised. ⚠ designer: cumulative growth (TG-P4-3) and the Radiant chain continuing on base bodies; (3) for a 3/3 that costs three Units is weak at base, the value being the chain.
- **Class:** B (M #94's mirror).

#### Training Grounds #26 · Path of Exile
`traininggrounds-026` · (4) Spell · Rare · built by P5 (#2198)

> **Designer:** (4) Path of Exile, Spell, #26 · **Exile** a permanent, draw a card ~~~ **Exile** a permanent, draw 2 cards **Cleave**

- **Text:** Exile a permanent. Draw {draw|card|cards}.
- **Radiant:** Cleave / Exile a permanent. Draw {draw|card|cards}.
- **Engine:** Core #34 Collateral Damage's verbs (crates/cards/src/scripts/core/c034_collateral_damage.rs): one declared target, `TargetDecl::target(1, 1, json!({ "side": "any", "of": ["unit", "backrow"] }))` ([[R81]]); `exile { target: { of: chosen } }` (effects/move_.rs:99: past Indestructible, a unit token ceasing to exist, [[R11]], [[R55]]); then `draw { count: param("draw") }` (effects/draw.rs:23). Radiant: Cleave printed as a keyword on the Spell (`radiant.keywords: [{ kind: Cleave }]`, text "Cleave\n…", as M #18 Expedition12's catalog entry), carried out by `exile_adjacent_to { target: { of: chosen } }` (effects/move_.rs:222) FIRST, while the target still stands in its lane (Core #34's header), then `exile`, then `draw`. `exile_adjacent_to` is [[§3.1]]'s adjacency in the target's own row and side (a backrow target's backrow neighbours), skipping a neighbour Immune to Spells (targets.rs `matches_scope`, B5 E35). Nothing NEW: Cleave on a board pick is Core #34's Radiant already, only the printed word is new for it.
- **Rulings:** TG-P5-2 (shared with #27): a Spell's Cleave on a board pick reaches the permanents adjacent to the pick, [[§3.1]]'s lanes N−1 and N+1 in its row on its side, the board twin of [[R1563001]]'s hand reading and Core #34 Radiant's "the permanents adjacent to it"; a neighbour Immune to Spells is passed by; the pick is still the one declared target ([[R81]]). No `dealsDamage` ([[R1748001]]).
- **Numbers:** draw 1 ↑ (Radiant 2, step 1, min 1).
- **Check:** (1) exiles an enemy Unit, an Indestructible one included; (2) exiles a backrow card, a face-down Trap included; (3) can pick your own permanent; (4) a unit token ceases to exist ([[R11]]); (5) then draws 1; (6) the draw reads through `param()`. Radiant: (7) the pick and the permanents in lanes N−1 and N+1 of its row and side are exiled, nothing across sides or rows; (8) draws 2; (9) the catalog lists Cleave on the Radiant face. R275: draw 2 against 1 and up to three permanents against one: holds. Typos: "draw a card" → "Draw {draw}", "Cleave" set on its own line first (house style, as M #18). Name: "Path of Exile" is a video game's title; no shipped card clashes (C #10 is "Exile").
- **Class:** A (Core #34's verbs).

#### Training Grounds #27 · Hopeful Jelly Bean
`traininggrounds-027` · (4) Spell, Jelly Bean · Rare · built by P5 (#2198)

> **Designer:** (4) Hopeful Jelly Bean, Spell, Jelly Bean, #27 · Make a permanent **Radiant** ~~~ Make a permanent **Radiant** **Cleave**

- **Text:** Make a permanent Radiant.
- **Radiant:** Cleave / Make a permanent Radiant.
- **Engine:** The Make Radiant verb on the field is shipped: `set_radiant { target: { of: chosen } }` (effects/radiant.rs:155, `make_radiant`): the flag only ([[R74]]); on the field the base-stat layer swaps at once, damage and buffs stay, no Cry re-fires ([[R22]]), a Divine Shield only the Radiant face prints comes up (`gain_printed_shield`); a hidden card's change is still reported ([[R97]], [[R177]]). Core #29 GIGA Glowy Jelly Bean's Radiant ("every permanent you control") and Core #28 (`set_radiant_random` with `RadiantZone::Field`) use it on the field already. Target as Core #34's (`{ side: any, of: [unit, backrow] }`, [[R81]]). Radiant: Cleave as #26 (TG-P5-2): the neighbours are computed in the hook with `adjacent_to(ctx, &json_as(json!({ "of": "chosen" })), &BoardScope::default())` (effects/targets.rs:418; rows are not filtered, adjacency stays in the pick's row), each `set_radiant { instanceId }`, M #18 Expedition12's `beside_the_pick` pattern on the board. The Jelly Bean tag exists (`Tag::JellyBean`, wire/catalog_types.rs:200; Core #26–#29). Nothing NEW.
- **Rulings:** TG-P5-2 (as #26). Also: any permanent may be picked, either side, a Radiant one included (left as it is, [[§6.3]] Make Radiant), so no pick check leaks a face-down card's face ([[R177]]); M #18's `notRadiant` check is for its own hand only.
- **Numbers:** none (no number printed).
- **Check:** (1) a base Unit picked becomes Radiant: its stats swap to the Radiant face's, damage and buffs kept; (2) its Cry does not fire again ([[R22]]); (3) a Field Spell picked runs its Radiant text from then on; (4) a face-down Trap of yours picked turns Radiant, the opponent seeing only that a card changed; (5) an enemy permanent may be picked; (6) an already Radiant pick: nothing. Radiant: (7) the permanents beside the pick in its row and side become Radiant too; (8) a neighbour Immune to Spells is passed by. R275: one permanent against up to three (a broader scope): holds; the base face has no number. Typos: "Cleave" placed first on its own line (house style). Tag: Jelly Bean as written.
- **Class:** A (Make Radiant).

#### Training Grounds #28 · Occidental
`traininggrounds-028` · (3) Unit, Human · Epic · 9/5 → 18/10 · built by P3 (#2196)

> **Designer:** (3) Occidental, Human, #28 · 9/5 · **Cry:** Destroy all permanents in the right most lane ~~~ 18/10 · **Cry:** Destroy all enemy permanents in the right most lane

- **Text:** Cry: Destroy all permanents in the rightmost lane.
- **Radiant:** Cry: Destroy all enemy permanents in the rightmost lane.
- **Engine:** C+ #24 Crushing Walls's script (crates/cards/src/scripts/classic_plus/c024_crushing_walls.rs): `cards_in_scope(ctx, { side, rows: [units, backrow] })`, skipping a carried Unit (`is_carried`, [[R446]], [[R653]]), kept when `slot_of(card).lane` is the rightmost lane, then `for_each_card` → `destroy { target: { of: instance } }` (effects/destroy.rs:55); Radiant `side: enemy`. A lane is a column ([[§3]]: "a lane is a column of four zones, two yours and two the opponent's"), and both seats' clients draw lane 1 on the left (Board.tsx, see #22), so "the rightmost lane" is the same column for both players: lane 5, both sides' unit and backrow zones. The rightmost lane is the highest lane read off the board (`state.players[p].units.len()` through a helper, as [[R685]] reads the midlane, `zones::midlane_lanes_of`), 5 today (`UNIT_ZONES`). The tops of piles only ([[R13]]): a card beneath resumes and is not destroyed; Indestructible stays; face-down cards are destroyed; a destroyed card that prints Death fires it ([[§4.5]]). The Occidental is on the field as its Cry resolves, so in lane 5 on the base face it destroys itself. Nothing NEW (Crushing Walls takes `[1, 5]` as a constant; the lane helper is a nicety).
- **Rulings:** TG-P3-4, [[R2196003]] (shared with #29): "the rightmost lane" is lane 5 (the board's last lane, [[R685]]'s reading of the board), both sides, both rows; "the leftmost lane" is lane 1. "All permanents" includes this Unit when it stands there (Hearthstone's "destroy all minions"); the Radiant face's "enemy" never reaches it.
- **Numbers:** none (the lane is a position, not a number).
- **Check:** (1) played in lanes 1–4: both sides' lane-5 Units and backrow cards are destroyed, a face-down one included; (2) lanes 1–4 untouched; (3) an Indestructible one stays; (4) a card beneath a Stack in lane 5 resumes; (5) played into lane 5, it destroys itself as well; (6) a destroyed card's Death fires. Radiant: (7) 18/10; (8) only the enemy's lane-5 Unit and backrow card are destroyed, yours stay, itself included. R275: stats 2×, one side only (no self-harm), as C+ #24's Radiant: holds. Typos: "right most" → "rightmost"; the type line lacks "Unit" (it prints stats: a Unit). ⚠ designer: Occidental (the West, usually left on a map) hits the right lane and Oriental the left; kept as written.
- **Class:** B (C+ #24's sweep).

#### Training Grounds #29 · Oriental
`traininggrounds-029` · (3) Unit, CN · Epic · 5/9 → 10/18 · built by P3 (#2196)

> **Designer:** (3) Oriental, CN, #29 · 5/9 · **Cry:** Destroy all permanents in the left most lane ~~~ 10/18 · **Cry:** Destroy all enemy permanents in the left most lan

- **Text:** Cry: Destroy all permanents in the leftmost lane.
- **Radiant:** Cry: Destroy all enemy permanents in the leftmost lane.
- **Engine:** #28's script with lane 1 (C+ #24 Crushing Walls's verbs: `cards_in_scope`, `is_carried`, `slot_of`, `for_each_card`, `destroy`). Lane 1 is the leftmost for both seats (see #28). CN is a tribal tag ([[R1424]], `Tag::Cn`, wire/catalog_types.rs:182). Nothing NEW.
- **Rulings:** TG-P3-4, [[R2196003]] (as #28): leftmost is lane 1; it destroys itself when played into lane 1 on the base face.
- **Numbers:** none.
- **Check:** (1)–(6) as #28 with lane 1; Radiant: (7) 10/18; (8) only the enemy's lane-1 cards. R275: holds as #28. Typos: "left most lan" → "leftmost lane"; the type line lacks "Unit". Tags: CN only, as written (the designer gave #28 Human and #29 CN; ⚠ designer whether Oriental is also Human).
- **Class:** B (C+ #24's sweep).

#### Training Grounds #30 · Floodgates
`traininggrounds-030` · (3) Unit · Epic · 0/12 → 0/24 · built by P4 (#2197)

> **Designer:** (3) Floodgates, Unit, #30 · 0/12 · **Armor 3** · Whenever a unit is summoned gain **+1 Armor** · Death: destroy all friendly units ~~~ 0/24 · **Armor 6** · Whenever a unit is summoned gain **+2 Armor** · Death: destroy all friendly units

- **Text:** Armor 3 / Whenever a Unit is summoned, this gains +{armorGain} Armor. / Death: Destroy all your Units.
- **Radiant:** Armor 6 / Whenever a Unit is summoned, this gains +{armorGain} Armor. / Death: Destroy all your Units.
- **Engine:** Armor 3 / Armor 6 printed keywords (`{ kind: Armor, n }`, [[§6.1]] Armor X, damage pipeline step 2, [[R1360]]). The trigger: a `TriggerDef` on `Summoned` and `CardResolved`, S #23 Warp Disruptor's `arrival` reading (crates/cards/src/scripts/stinkyscape/c023_warp_disruptor.rs; [[R1875003]], C #5 Tesla): a summon (Recruit, token, copy, Reborn) at once, a played Unit once it resolves, never twice, either side here, a Unit only (a Field Spell's or Trap's placement is no Unit; a Flying Unit placed in the backrow is, [[R1081000]]), never Floodgates itself. Each arrival: `grant_keyword { target: self, keyword: { kind: Armor, n: armorGain } }` (effects/buff.rs:228; Armor stacks, buff.rs:186), a granted keyword kept until it leaves the field ([[R78]]). Death: `destroy_all { side: self }` (effects/destroy.rs:79), its controller as it died ([[R89]]); Indestructible Units stay; their Deaths fire (#25 Trigor's successor arrives after and stays). Nothing NEW.
- **Rulings:** TG-P4-4: "a Unit is summoned" is [[R1875003]]'s arrival on either side, a played Unit included (Hearthstone's "summoned" counts plays), each one once, Floodgates' own arrival not; several arrivals (TG #23's three copies) give one gain each. "Friendly Units" are its controller's Units as it died ([[R89]]).
- **Numbers:** armorGain 1 ↑ (Radiant 2, step 1, min 1). Armor 3 / 6 are keyword numbers (the X row, [[R482]]).
- **Check:** (1) 0/12 with Armor 3: a 5-damage hit deals 2; (2) a Unit you play: +1 Armor (Armor 4); (3) an enemy's played Unit: +1; (4) a token summon, a Recruit and a copy: +1 each; TG #23 played: +4; (5) a Field Spell or Trap placed: nothing; (6) its own arrival: nothing; (7) destroyed: every other Unit you control is destroyed, an Indestructible one staying, the enemy's untouched; (8) bounced or exiled: no Death; (9) the gain reads through `param()`. Radiant: (10) 0/24, Armor 6, +2 per arrival, the same Death. R275: 0 attack stays 0, 24 = 2×12, Armor 6 = 2×3, +2 = 2×1: holds; the Death is the same drawback on both faces. Typos: "unit" → "Unit", "friendly units" → "your Units" (the catalog never says "friendly"), "Death:" bolded and capitalised.
- **Class:** B (an arrival trigger, a stacking keyword grant, a sweep).

#### Training Grounds #31 · Level Up!
`traininggrounds-031` · (3) Field Spell · Epic · built by P6 (#2199)

> **Designer:** (3) Level Up!, Field Spell, #31 · After your units attack 3 times, they become **Radiant** *{start tracking after this is played}* ~~~ After your units attack 2 times, they become **Radiant** *{start tracking after this is played}*

- **Text:** After one of your Units has attacked {attacks|time|times} since this arrived, it becomes Radiant.
- **Radiant:** After one of your Units has attacked {attacks|time|times} since this arrived, it becomes Radiant.
- **Engine:** Make Radiant exists: `set_radiant { instanceId }` (`effects/radiant.rs:155`, §6.3 Make Radiant; Core #95's `make_hand_radiant` builds it per card, `subsystems/call_to_chaos.rs:173`): on the field the base layer swaps, damage and buffs stay, no Cry ([[R22]], [[R74]], [[§5.2]]); an already-Radiant card is untouched. The count can live on this Field Spell's own `memory` (`remember { key, value }`, `effects/memory.rs:35`, writes on self only), a map unit id → attacks, so it starts at 0 when this arrives and is gone when this leaves (R78 resets `memory`): no new state field. NEW (P6): an "after one of your Units attacks" hook for a card that is not the attacker. Today `Script.after_attack` (`script.rs:1152`, facts via `combat::after_attack_of`, `combat.rs:1851`) runs for the attacker alone, and the only public attack events are `attackDeclared` / `attackCancelled` (`wire/events.rs:587`, `GameEventType::AttackDeclared` at :1217; C+ #2 Groom Shroom's and C #45 Nature Titan's `TriggerDef::new(…, &[GameEventType::AttackDeclared], …)`). Counting at `attackDeclared` would make the Unit Radiant (twice the stats) before its third combat's damage, so the Radiant step must run after the combat's state check closes, as `afterAttack` does: either `afterAttack` dispatched to other holders with the attacker's id, or a public after-combat event (the latter touches `wire/events.rs`, a shared surface per CLAUDE.md).
- **Rulings:** TG-P6-6: "your units attack N times" counts each Unit separately (not N attacks among all your Units), every attack it makes while this acts on the field, from its arrival on ("start tracking after this is played"), across turns; a forced attack counts ([[R53]]), an attack called off before it fought does not ([[R44]]); the Unit becomes Radiant after the combat of its Nth attack closes, if it is still on the field ([[R174]]: a Unit that left and came back is a new arrival and starts again at 0). TG-P6-7: the counts are this card's: it leaving the field (R78) resets them, and two Level Ups count independently; an already-Radiant Unit is counted but nothing happens. TG-P6-8: a stolen Unit counts for whoever controls it as it attacks.
- **Numbers:** attacks 3 ↓ (Radiant 2), min 1.
- **Check:** (1) a Unit of yours that attacks 3 times (over several turns) becomes Radiant after the third combat, not before its damage; (2) two different Units attacking once and twice make neither Radiant; (3) attacks made before this arrived do not count; (4) an enemy Unit's attacks never count; (5) a forced attack counts, a cancelled one does not; (6) a Unit that dies in its third attack is not made Radiant; (7) bounce-and-replay restarts that Unit's count; (8) this leaving the field resets every count; (9) already-Radiant Unit: no change. Radiant: (10) 2 attacks suffice. R275: Field Spell scaled by its number, 2 attacks against 3 (1.5×), passes. ⚠ designer: per-Unit count (default) vs a shared count; "attack 3 times" counts across turns (default yes, the designer's note).
- **Class:** C (an after-attack hook for a non-attacker).

#### Training Grounds #32 · Selfless Protector
`traininggrounds-032` · (1) Unit, Human · Epic · 2/1 → 4/2 · built by P4 (#2197)

> **Designer:** (1) Selfless Protector, Unit, Human, #32 · 2/1 · **Death:** give your units **Divine Shield** ~~~ 4/2 · **Cry & Death:** give your units **Divine Shield**

- **Text:** Death: Give your Units Divine Shield.
- **Radiant:** Cry: Give your Units Divine Shield. / Death: Give your Units Divine Shield.
- **Engine:** C+ #12.5 Anti-Waffle Shell's Cry verbatim: `for_each_card { cards: active_units_of(state, controller) ids, each: grant_keyword { target: { of: instance }, keyword: { kind: "Divine Shield" } } }` (`effects/buff.rs:228`; `zones::active_units_of`, `zones.rs:1641`), as the `death` hook (the Radiant face also as `cry`). The Death runs after the state check took this off the field ([[R78]], [[R89]]), so it never reaches itself, nor Units that died in the same check. A granted shield on a Unit that already has one changes nothing; on one whose shield was spent it is up again ([[§10.4]], `buff::grant_to`).
- **Rulings:** none new. Reading: the Radiant Cry's "your Units" includes this Unit (it is on the field as its Cry resolves; Hearthstone's "your minions" includes the source; Anti-Waffle Shell's `active_units_of` would include it).
- **Numbers:** none (no number printed).
- **Check:** (1) Death: every other Unit of yours on the field has Divine Shield; (2) enemy Units do not; (3) Units dying in the same check get none; (4) a pile's lower Unit is not reached ([[R13]]). Radiant: (5) Cry: your Units, this one included, gain Divine Shield; (6) Death as base. R275: 4/2 doubles; the added Cry is a rider, passes. "Cry & Death" written as two labelled lines (house style: one label per line). ⚠ designer: the Radiant Cry includes itself (default yes).
- **Class:** A.

#### Training Grounds #33 · Guerilla Felinor
`traininggrounds-033` · (3) Unit, Felinor · Epic · 4/6 → 8/12 · built by P3 (#2196)

> **Designer:** (3) Guerilla Felinor, Unit, Felinor, #33 · 4/6 · **Cry:** Deal 1 damage to a random enemy unit, repeat 7 times. ~~~ 8/12 · **Pierce** · **Cry:** Deal 1 damage to a random enemy unit, repeat 14 times.

- **Text:** Cry: Deal {damage} damage randomly split among enemy Units.
- **Radiant:** Pierce / Cry: Deal {damage} damage randomly split among enemy Units.
- **Engine:** No new verb: `damage_split { amount, among: "enemyUnits" }` (`effects/split.rs`, `SplitAmong::EnemyUnits`; C+ #3 Second Amendment Snake, S #19 Imprisoned Cosmality's `damage_split(json_as(json!({ "amount": damage, "among": "enemies" })))`): `amount` hits of 1, each drawn from the match rng among the enemy Units still standing as that hit lands (not marked destroyed, above 0 Health), no state check between hits ([[R59]], [[R60]]); with no enemy Unit standing the rest of the hits are lost (no hero). S #11.2 Arcane Missiles reaches the hero too and repeats by Echo (a Spell), so it is not the pattern for a Unit's Cry. Each hit's source is this Unit, so on the Radiant face each hit pierces Armor ([[R346]], as S #19's do). A Divine Shield takes one hit; the next can hit the same Unit again.
- **Rulings:** none new: TG reads "repeat 7 times" as 7 hits in all (the Radiant 14 doubles it).
- **Numbers:** damage 7 ↑ (Radiant 14, step 2), `cry: true` ([[R1748000]]).
- **Check:** (1) 7 hits of 1, each on a random enemy Unit, re-drawn per hit; (2) a Unit killed by an earlier hit is never drawn again; (3) the enemy hero is never hit; (4) no enemy Unit: nothing, and no random number drawn ([[R129]]); (5) a Divine Shield eats one hit. Radiant: (6) 14 hits; (7) Pierce on the Unit and on the Cry's hits (Armor ignored). R275: 8/12 doubles, 14 hits = 2×, plus Pierce. ⚠ designer: "repeat 7 times" = 7 hits (default) or 8; name spelled "Guerilla" (the usual spelling is "Guerrilla"; keep the designer's unless he says otherwise).
- **Class:** A.

#### Training Grounds #34 · Dino-size
`traininggrounds-034` · (3) Spell · Epic · built by P5 (#2198)

> **Designer:** (3) Dino-size, Spell, #34 · Set a units stats to 10/10 ~~~ Set a units stats to 20/20

- **Text:** Set a Unit's Attack and Health to 10/10.
- **Radiant:** Set a Unit's Attack and Health to 20/20.
- **Engine:** M+ #21 Harmony Spirit's set ([[R1185002]]): `set_stats_on` (`effects/health.rs:88`) sets `stats_override`, clears buffs and the stat half of `tuning` (`transform::to_printed_stats`), heals its damage and keeps the X/X-token face (R175, R444); auras of [[§10.4]] layer 5 apply on top. It is private: the only public verb is `set_stats_all { scope, attack, health }` (`health.rs:116`), whose `BoardScope` (`effects/targets.rs:257`) has sides, rows, types, tags, `excludeSelf` but no single instance. NEW (P5): a targeted `set_stats { target, attack, health }` over `set_stats_on`, with `set_stats_all`'s guards (an Immutable Unit untouched, [[R23]]; `numberChanged` "attack"/"health"). A declared target, any Unit on either side ([[R81]]); one Immune to Spells cannot be targeted.
- **Rulings:** none new beyond R1185002 (its "set" is Hearthstone's); either side may be targeted (Hearthstone's Dinosize targets any minion).
- **Numbers:** none: the 10/10 is a number whose better way hangs on the target (an enemy's or yours), which [[R1748003]] fixes for a stated reason ([[R902]]'s case).
- **Check:** (1) the target is 10/10 after it resolves, damage healed; (2) its buffs are gone, an aura still adds; (3) an enemy Unit can be targeted; (4) an Immutable Unit is unchanged; (5) bounced, it is its printed card again ([[R78]]). Radiant: (6) 20/20. R275: 20/20 against 10/10 (2×), passes. Typo: "units stats" is "Unit's stats".
- **Class:** B (a targeted set over an existing primitive).

#### Training Grounds #35 · Corpse Recycler
`traininggrounds-035` · (3) Field Spell · Epic · built by P6 (#2199)

> **Designer:** (3) Corpse Recycler, Field Spell, #35 · Non token units on death summon a **Rush token** ~~~ Non token units on death summon a **Radiant Rush token**

- **Text:** After a non-Token Unit dies, summon a Rush Token for its controller.
- **Radiant:** After a non-Token Unit dies, summon a Radiant Rush Token for its controller.
- **Engine:** A `TriggerDef` on `GameEventType::Destroyed` (as C+ #19.5 Bot Loser, S #150's death watch): the event carries `def_id`, `controller` (who controlled it as it died, [[R172]]), not its lane ([[R89]]: every trigger but a Death reads the event). Keep it when the def is a Unit and `!def.is_token()` (`wire/catalog_types.rs:1133`); then `summon { defId: core-t-rush, player: <that controller>, radiant }` (Core #95's `summon_rush_tokens`, the Core token by index `T-rush`). Placement is [[R64]]'s leftmost free zone, as every Death summon (M #93 Growing Felinor, [[R1180]]); a full row summons nothing. A Rush Token is a Token, so its death summons nothing (no loop). A Field Spell's trigger, so both players' Units answer. Nothing NEW.
- **Rulings:** TG-P6-9: "units" are both players' (the card names no side), each token for the dead Unit's controller (R172); "on death" is after the death (a trigger on `destroyed`, after that Unit's own Death hook, [[R68]]'s order); placement is R64's, not "the zone it died in" (the event carries no lane; Hearthstone puts it in place, a NEW lane on `destroyed` if the designer wants that). A Token fused into a non-token card counts by the fused definition ([[R77]]).
- **Numbers:** none ("a" token; the Token's stats are its card's).
- **Check:** (1) your non-token Unit dies: you get a Rush Token; (2) an enemy's dies: the enemy gets one; (3) a Token dying gives nothing; (4) a full row: nothing; (5) several deaths in one check: one token each, in trigger order; (6) this gone: nothing. Radiant: (7) Radiant Rush Token (6/6, Rush, Cleave). R275: a token at twice the stats with a rider, passes. ⚠ designer: both sides (default) or yours only; same zone vs leftmost (default R64).
- **Class:** A.

#### Training Grounds #36 · Training Dummy
`traininggrounds-036` · (1) Unit · Epic · 0/3 → 0/6 · built by P4 (#2197)

> **Designer:** (1) Training Dummy, Unit, #36 · 0/3 · **Can't be in defense position** · **Death:** Add a random **Radiant Tutorial-Grounds** card to your hand ~~~ 0/6 · **Death:** Add two random **Radiant Tutorial-Grounds** card to your hand

- **Text:** Can't be in Defense Position. / Death: Add {cards|random Radiant Training Grounds card|random Radiant Training Grounds cards} to your hand.
- **Radiant:** Death: Add {cards|random Radiant Training Grounds card|random Radiant Training Grounds cards} to your hand.
- **Engine:** C+ #48 Jlockeed's Lobbyist line for line (0/3 → 0/6, "Can't be in Defense Position." on the base face only, Death adds a random card): `StaticFlags { never_defense: Some(true) }` (C+ #65.1 Spikey Pillow's flag, honoured by the switch action, `legalActions` and an effect's switch, [[R20]]), and `death: add_random_from_catalog { query: { set: "Training Grounds" }, count, radiant: true }` (`effects/add_to_hand.rs`; the query's `set`, `catalog.rs:476`), which excludes this card (R387) and no tokens (the default, [[R382]]); a full hand burns ([[§2.4]]). The pool names its set, so it works while the set is held ([[R1420]], [[R1846001]]).
- **Rulings:** none new. "Tutorial-Grounds" is the Training Grounds set. The pool is every non-token TG card but this one, the snapshot cards #41–#60 included if the brief keeps them TG cards; the tutorial's "Legendary & Mythic do not show up until the last game" is P8's, not this card's.
- **Numbers:** cards 1 ↑ (Radiant 2).
- **Check:** (1) cannot switch to Defense (action refused, an effect's switch leaves it in Attack); (2) Death: one random Radiant TG card to your hand; (3) never itself; (4) hidden from the opponent ([[R97]]). Radiant: (5) two; (6) may be in Defense Position. R275: 0 attack stays 0, 6 Health doubles; 2 cards against 1 and the restriction dropped (a rider, as C+ #48's Radiant face), passes. Typo: "two … card" is "cards". The Radiant face dropping "Can't be in Defense Position" matches C+ #48, so it reads as deliberate (⚠ designer only to confirm).
- **Class:** A.

#### Training Grounds #37 · Seance
`traininggrounds-037` · (2) Spell · Legendary · built by P5 (#2198)

> **Designer:** (2) Seance, Spell, #37 · Add an exact copy of a permanent to your hand *{preserves cost changes}* ~~~ Add an exact copy of a permanent to your hand, it costs (0)

- **Text:** Add an exact copy of a permanent to your hand, keeping its cost changes.
- **Radiant:** Add an exact copy of a permanent to your hand. It costs ({setCost}).
- **Engine:** A declared target, any permanent on the field ([[R81]]; Core #34 Collateral Damage's "a permanent"), a pile's top only ([[R13]]). `add_to_hand { copyOf }` (`effects/add_to_hand.rs:290`, [[R1242]]) makes a fresh card of its definition into the caster's hand, the source staying, with `copy_riders` (`add_to_hand.rs:185`): radiant, chinese, granted tags, `statsOverride`, `armorOverride`, `tuning`. NEW (P5): "exact copy" and "preserves cost changes" — the copy also takes R57's field-copy riders `copy_riders` omits (buffs, granted keywords, the Vanilla state) and the price (`costMod`, `costOverride`, `embiggenCostOverride`, `fixedX`), which a card keeps from the hand onto the field ([[R78]], [[R766]]); damage, exertion, counters and memory reset ([[R57]], [[R385]], [[R429]]). A `copyOf` rider (e.g. `exact: true`) in `AddToHandArgs`. Radiant: `costOverride: setCost`. A full hand burns ([[§2.4]]).
- **Rulings:** TG-P5-3: "exact copy" is R57's copy of a Unit on the field plus its price, and the copy is the caster's (owner and controller); a copy of an enemy card is the caster's ([[R12]]). TG-P5-4: "permanent" is any card on either side's field that Core #34's target admits (a face-down card copied shows its face to the caster alone, [[R97]]); a Spell is never one.
- **Numbers:** setCost 0 ↓ (Radiant face only, `tunedOn: radiant`, [[R749]]).
- **Check:** (1) a buffed, Radiant enemy Unit → your hand holds a Radiant copy with the buffs and granted keywords, undamaged; (2) a Unit that cost (1) less than printed → the copy costs (1) less; (3) the source stays; (4) a Field Spell or Trap can be copied; (5) Brittle and times-played counts are not copied; (6) a full hand burns it. Radiant: (7) the copy costs (0) whatever its price. R275: (0) against the copied price, a cost rider, passes. ⚠ designer: whether "exact" includes damage (default no, R57), and whether an enemy face-down Trap is a legal target.
- **Class:** B (a `copyOf` rider).

#### Training Grounds #38 · Tutorial Master
`traininggrounds-038` · (4) Unit, Human · Legendary · 4/16 → 8/32 · built by P4 (#2197)

> **Designer:** (4) Tutorial Master, Unit, Human, #38 · 4/16 · **Cry & Death:** Add 3 random **Training-Grounds** cards to your hand, they cost (1) less ~~~ 8/32 · **Cry & Death:** Add 3 random **Radiant** **Training-Grounds** cards to your hand, they cost (1) less

- **Text:** Cry: Add {cards|random Training Grounds card|random Training Grounds cards} to your hand. They cost ({discount}) less. / Death: Add {cards|random Training Grounds card|random Training Grounds cards} to your hand. They cost ({discount}) less.
- **Radiant:** Cry: Add {cards|random Radiant Training Grounds card|random Radiant Training Grounds cards} to your hand. They cost ({discount}) less. / Death: Add {cards|random Radiant Training Grounds card|random Radiant Training Grounds cards} to your hand. They cost ({discount}) less.
- **Engine:** `add_random_from_catalog { query: { set: "Training Grounds" }, count, costMod: -discount, radiant }` (`AddRandomFromCatalogArgs.cost_mod`; Core #57 Conjure KY's add of 3, with repeats allowed, [[R60]]) as both `cry` and `death`; the pool never offers this card ([[R387]]) nor a token; each cost floors at (0) ([[R65]]); a full hand burns ([[§2.4]]). Nothing NEW.
- **Rulings:** none new; the pool as #36's.
- **Numbers:** cards 3 ↑; discount 1 ↑ (both read by the Cry and the Death, so no `cry` flag).
- **Check:** (1) Cry: 3 TG cards, each (1) less, never Tutorial Master; (2) Death: the same again; (3) repeats allowed; (4) a (0) card stays (0). Radiant: (5) all Radiant. R275: 8/32 doubles, Radiant cards as the rider, passes.
- **Class:** A.

#### Training Grounds #39 · Call to Chaos (Training-Grounds Edition)
`traininggrounds-039` · (4) Spell, Call to Chaos · Legendary · built by P7 (#2200)

> **Designer:** (4) Call to Chaos (Training-Grounds Edition) Spell, Call to Chaos, Legendary, #39 · ??? *(One of the following random effects)* · Summon 3 Rush tokens, all units in your hand, field, and deck gain **Taunt** and **Divine Shield** · Fill your hand with **Goetzmann Game Design** and refresh 4 mana · Add 4 **Training-Grounds** edition cards to your hand, they cost (0) · Gain 8 mana and draw 2 cards · Summon 6 random **Training-Grounds** permanents · Add 2 **Radiant** **Training-Grounds** edition cards to your hand, they cost (0) · Summon 5 Rush tokens, for the rest of the game your units have **Rush, Trample,** and **Pierce** · Summon a **Training-Grounds Golem** · Now and for the rest of the game, every other turn add a **Call to Chaos** to your hand, it costs (0) · Cast a random **Call to Chaos** ~~~ !!! *(three effects)*

- **Text:** One random effect: Summon 3 Rush Tokens, then give your Units on the field, in your hand and in your deck Taunt and Divine Shield; fill your hand with Goetzmann Game Designs and refresh 4 mana; add 4 random Training Grounds cards to your hand, which cost (0); gain 8 mana and draw 2 cards; summon 6 random Training Grounds permanents; add 2 random Radiant Training Grounds cards to your hand, which cost (0); summon 5 Rush Tokens, and for the rest of the game your Units have Rush, Trample and Pierce; summon a Training-Grounds Golem; add a random Call to Chaos to your hand, which costs (0), now and at the start of every second turn of yours for the rest of the game; cast a random Call to Chaos.
- **Radiant:** Three different random effects, resolved in the order listed: (the same ten clauses).
- **Engine:** The family's one roller: the script is S #150's, `cry: call_to_chaos(CallToChaosArgs { radiant, table: Some(CHAOS_TG_EFFECTS) })` (`subsystems/call_to_chaos.rs:460`), the base face one entry, the Radiant three different ones in list order ([[R423]], `CALL_TO_CHAOS_RADIANT_EFFECTS`), `chaosRolled` naming each entry's `label` to both players first ([[R436]]; each label must be its clause as printed, S #150's `r436_…` test). NEW (P7): `subsystems/call_to_chaos_training_grounds.rs` with `CHAOS_TG_EFFECTS: &[ChaosEffectDef]` (ten entries, each a lazy part, `entry_kind` names), and its numbers in `config.rs` (rule 9). Entry by entry:
  1. `summon` core-t-rush ×3 (Core's `summon_rush_tokens`, base faces), then `grant_keyword_cards { scope: { side: self, zones: [field, hand, library], types: [Unit] }, keyword }` for Taunt and for Divine Shield (`effects/buff.rs:488`; C+ #40 Appropriations, C+ #77 Anti-Softlock), the tokens included; hidden cards change silently ([[R440]]). Exists.
  2. S #88 Siva 2's fill: `hand_cap_of − zone_count(Hand)` × `add_to_hand { defId: traininggrounds-040 }`, then `refresh_mana { amount: 4 }` (`effects/mana.rs:47`, [[R364]]: never past max, nothing at max). Exists.
  3. `add_random_from_catalog { query: { set: "Training Grounds" }, count: 4, costOverride: 0 }`, R387 excluding this card. Exists.
  4. `gain_mana { amount: 8 }` (temporary, may pass max, [[§2.3]]) then `draw { count: 2 }`. Exists.
  5. `summon_random { query: { set: "Training Grounds", type: [Unit, Field Spell, Trap, Field Trap] } }` ×6: each pick independent ([[R60]]), its row by its type, a Trap face-down (Core #95's entries 1 and 9, `summon.rs:728`), drawn only when a zone is open ([[R129]]). Exists.
  6. As 3 with `count: 2, radiant: true`. Exists.
  7. `summon` core-t-rush ×5, then `give_units_keyword` for Rush, Trample and Pierce (`effects/player_mods.rs:238`, `UnitsHaveKeyword`, [[R1859007]], S #137 War Doctrines): every Unit its caster controls now and later, not a Vanilla one, a badge on both seats ([[R169]]). Exists (three modifiers).
  8. `summon` traininggrounds-039-1 (by index in the TG set, as M #95's `summon_token`), [[R64]]. Exists.
  9. NEW (P7): "every other turn". Today's rest-of-game effect is `for_rest_of_game` (`effects/delay.rs:763`, `ModifierKind::StartOfTurnEffect`, [[R458]]; M #95 entry 9's `CHAOS_ETERNAL_HOOK`, [[R1241]]), which runs at every start of its player's turns. TG's needs a step under its own engine hook that adds only on alternate turns: the data carries the caster's `turns_started` when made, the step adds when (now − then) is even (`PlayerState.turns_started`, `state.rs:1281`, extra turns included, [[R846]]). Plus the "now": one `add_to_hand` at once. The add is `add_random_from_catalog { query: { tags: [Call to Chaos] }, costOverride: 0 }`. A cap like `CALL_TO_CHAOS_ETERNAL_CAP` per player (M #95's `eternal_effects_held`) bounds it.
  10. `cast_random_call_to_chaos` (`call_to_chaos.rs:241`, [[R28]], [[R87]], [[R70]]): the base form of any shipped edition, the chain capped by `CALL_TO_CHAOS_CHAIN_CAP`. Exists.
- **Rulings:** TG-P7-1: "Training-Grounds edition cards" (3, 6) and "Training-Grounds permanents" (5) are the Training Grounds set's non-token cards, not Call to Chaos editions; "permanents" are Units, Field Spells, Traps and Field Traps ([[§5.1]]). TG-P7-2: entry 1's Units are the caster's, on the field, in hand and in deck, the 3 tokens included (summoned first). TG-P7-3: entry 9 adds one at once, then at the start of every second turn of the caster's (own turns counted, extra turns included), from a random Call to Chaos of the shipped editions (tag Call to Chaos, [[R380]], [[R1420]]), at (0); several stack up to the cap, and at the cap the entry does nothing (R87's shape, as [[R1241]]); its badge is its label ([[R169]]). TG-P7-4: "refresh 4 mana" is §6.3 Refresh ([[R364]]), not a gain. TG-P7-5: entry 7's three keywords are three `unitsHaveKeyword` modifiers, kept when the card is gone.
- **Numbers:** none declared: a Call to Chaos's numbers are a table the engine holds ([[R1748003]]'s reason, as every edition); `config.rs` constants (rule 9): `CHAOS_TG_RUSH_TOKENS` 3, `CHAOS_TG_REFRESH` 4, `CHAOS_TG_CARDS` 4, `CHAOS_TG_MANA` 8, `CHAOS_TG_DRAW` 2, `CHAOS_TG_PERMANENTS` 6, `CHAOS_TG_RADIANT_CARDS` 2, `CHAOS_TG_ARMY_TOKENS` 5, `CHAOS_TG_EVERY` 2 (names the build's).
- **Check:** (1) the roll names one clause to both players before it resolves; (2)–(11) one test per entry, as S #150's `entry_*` tests: 3 tokens then Taunt and Divine Shield on hand, deck and field Units; a full hand of Goetzmann Game Designs and up to 4 mana back; 4 TG cards at (0), never this; +8 mana over max and 2 draws; 6 TG permanents, Units and backrow cards by type, Traps face-down; 2 Radiant TG cards at (0); 5 tokens and Rush, Trample, Pierce on a Unit played later; the Golem; one Call to Chaos at (0) now, none next turn, one the turn after; a cast of another edition at chain depth 1; (12) every entry is rolled over the seeds. Radiant: (13) three different entries in list order; (14) the recast cast is a base form ([[R28]]). R275: three effects against one, as every edition. The designer's "???" is the base text's slot and "!!!" the Radiant's (both in the editions' standard words). ⚠ designer: "every other turn" read as every second turn of yours (default) and not "each of your turns" (every other turn of the game); "a Call to Chaos" read as a random edition (default) and not this edition; the names keep the designer's hyphen ("Training-Grounds"), the set's name being "Training Grounds".
- **Class:** C (a new edition's table; alternate-turn rest-of-game effect).

#### Training Grounds #39.1 · Training-Grounds Golem
`traininggrounds-039-1` · (4) Unit, Token · Token (printed Legendary) · 10/10 → 20/20 · built by P3 (#2196), with #39's table in P7 (#2200)

> **Designer:** (4) Training-Grounds Golem Unit, Token, Legendary, #39.1 · 10/10 · **Rush, Armor 10, Taunt, Divine Shield, Pierce** ~~~ 20/20 · **Rush, Armor 20, Taunt, Divine Shield, Pierce**

- **Text:** Rush, Armor 10, Taunt, Divine Shield, Pierce
- **Radiant:** Rush, Armor 20, Taunt, Divine Shield, Pierce
- **Engine:** Keywords only, catalog data read by [[§10.4]] layer 1 (`KeywordKind::Rush`, `Armor`, `Taunt`, `DivineShield`, `Pierce`, `wire/catalog_types.rs:555`); an empty script (C #41 State of the Game's shape; Core #95.1 Chaos Golem). Tokens like it: Core #95.1, C+ #73.1, M #95.1, S #150.1.
- **Rulings:** none new.
- **Numbers:** none: Armor 10 (Radiant 20) is a numbered keyword, which a Nerf or a Buff moves as an X ([[R482]]).
- **Check:** (1) 10/10 with the five keywords, can attack Units the turn it lands; (2) Armor 10 takes 10 off each hit; (3) Taunt holds attacks. Radiant: (4) 20/20 and Armor 20. R275: stats double and Armor doubles, passes. Name: the designer's "Training-Grounds Golem", hyphen kept as the tracker writes it (the set is "Training Grounds"; ⚠ designer whether the names drop the hyphen). Its maker is #39's eighth entry, whose text names it, so #39's `refs` lists it ([[R279]]).
- **Class:** A.

#### Training Grounds #40 · Goetzmann Game Design
`traininggrounds-040` · (0) Spell, Quickdraw · Mythic · built by P5 (#2198)

> **Designer:** (0) Goetzmann Game Design, Spell, Quickdraw, #40 · Add a random card to your hand ~~~ Add a random **Radiant** card to your hand

- **Text:** Add a random card to your hand.
- **Radiant:** Add a random Radiant card to your hand.
- **Engine:** Core #59 Unbiased Immigration's add: `add_random_from_catalog { query: { excludeDefId: ID }, count: 1, radiant }` (R387 excludes it anyway): a pool that names no set draws every non-token card of the sets that ship ([[R380]], [[R1420]]), so no TG card until the set ships; a full hand burns ([[§2.4]]). Quickdraw ([[R640]]: replaces an opening draw).
- **Rulings:** none new.
- **Numbers:** none (one card; "a random card" is the text it is built on).
- **Check:** (1) one random non-token card of a shipped set into your hand; (2) never itself; (3) hidden from the opponent ([[R97]]); (4) Quickdraw in the opening hand. Radiant: (5) the card is Radiant. R275: a Radiant card against a plain one, a rider, passes.
- **Class:** A.

### The snapshots, #41–#60 (P2, #2195)

The designer, after #40: "These 20 cards: (SNAP SHOT THEM SO THEY ARE LIKE LABELED \#41,\#42 etc \[they should be separate entities\])". Each entry's Text and Radiant are the Core entry's as they stand at the foundation's commit; stats, keywords, tags, `flavor`, params, `dealsDamage` and `refs` are copied as they are. Each Engine cell names the Core file to copy into `crates/cards/src/scripts/training_grounds/` and what the copy changes or drops.

**What every snapshot shares (the P2 rulings, TG-P2-1 … TG-P2-5; block R2195000…):**

- **TG-P2-1 Identity.** A snapshot is its own entry, `traininggrounds-0NN`, `set: "Training Grounds"`, index "41"…"60",
  with the Core card's name, cost, type, tags, rarity, flavor, stats, keywords, both texts, params and refs **as they stand at
  the foundation's commit**. It does not follow later Core patches (a separate entity, the designer's "[they should be
  separate entities]"); a later patch to a Core card that should reach its snapshot claims both ids. Rarity: the Core rarity
  (the brief's #1–#40 bands say nothing for #41–#60; ⚠ designer). Snapshot census: 11 Common (#41, 42, 43, 45, 46, 47, 48,
  54, 58, 59, 60), 7 Rare (#44, 49, 50, 51, 52, 53, 55), 2 Epic (#56, 57); with #1–#40 the set is 26 Common, 19 Rare,
  11 Epic, 3 Legendary, 1 Mythic over 60 cards, plus the token #39.1.
- **TG-P2-2 The script is a copy.** `pub const ID` changes to the snapshot's id; nothing else in `script()` names its own
  id (every self-reference in these 20 files is `{ "of": "self" }`, `ctx.self_`, or a def id the engine stores from the
  instance, so a copy keeps working under the new id). A Core file's `script_v…` function kept for old replays
  (`KEPT_SCRIPTS`, R767000) is **not** copied: a snapshot has no shipped history (#49, #52). The header comment and the tests
  are rewritten for the snapshot (CLAUDE.md rule 6); the tests are the Core file's tests with the id swapped, plus the
  "plays as" clauses below. Not a re-export (`pub use … core::c002_bigot::script`), so a Core refactor never moves a snapshot.
- **TG-P2-3 One deck, one of the pair.** A deck holds `core-NNN` or its snapshot, never both. Today L3 counts copies by id
  only (`MAX_COPIES = 1`, `crates/engine/src/config.rs:36`; `crates/engine/src/state.rs:2213` `validate_deck_with`;
  `crates/engine/src/validator.rs:367` L3; `validator.rs:681` `check_deck_draft`), so this is NEW (P2): one pair table
  (or a `snapshotOf` field on the TG entry) read by both validators and by every dealer, or a dealt deck is refused:
  `crates/tools/src/fuzz.rs:391` `decks_for_seed_from` (fuzz, duos fuzz; the golden traces deal `SHIPPED_FUZZ_POOL`, which
  holds no TG card while the set is held), `crates/server/src/actor/engine.rs:298`–`346` `deal_random_deck*`,
  `crates/server/src/duos/engine.rs:123` `ai_deck`, `crates/ai/src/deck.rs:655` `build_ai_deck`, and the web deck builder.
- **TG-P2-4 Names.** The exception the owner asked for is not at `crates/cards/tests/cross/catalog.rs` 7561–7584 (that is
  `r1178002_names_its_17_dogpile…`, which compares Meditative+ names only and passes unchanged): it is
  `r739_one_name_names_one_card` in **`crates/cards/tests/cross/catalog.rs`**, which walks every non-token entry of **every** set, held ones
  included, so it fails the day the foundation lands the first snapshot entry. The exception must allow exactly: two
  non-token entries sharing a name when one is `core-NNN`, the other is `traininggrounds-0NN` with NN in 41…60, and the pair
  is the one the snapshot table names (#41↔core-002 … #60↔core-058). Anything else still fails: two snapshots sharing a name, a
  snapshot sharing a name with a non-Core card or with the wrong Core card, a TG #1–#40 card sharing any name. Two more name
  checks need the same pair exception: `crates/cards/tests/cross/chinese.rs:385` `r1303_names_are_unique` (the Chinese
  names in `crates/cards/chinese.json`; a snapshot copying its Core card's Chinese name fails it), and R279's `refs` proof
  (`crates/cards/tests/cross/references.rs:188` `named_by`), which finds a card by name: a text that names a Core card by
  name names its snapshot too. Today that hits S #88 Siva 2 ("Fill your hand with Bigots", refs `core-002`) **at once**
  (a held card names every set's cards), and, once the set ships, C+ #61 Bauble Bubble (Stockpile) and Core #76 Field of
  Dreams (Reminisce). Default: a text naming a Core card names the Core card alone (`named_by` skips a snapshot unless the
  card's `refs` lists it), as R381's rules words do; each of those three scripts makes the Core id and stays so.
- **TG-P2-5 `loc` and code lines.** `loc` (§5, `crates/tools/src/catalog.rs:2767` `loc_failure`) is frozen for an entry the
  newest shipped snapshot holds and must equal `cargo jackioh catalog loc` for a new one. Core's `loc` values were frozen in
  v0.3.0 and are far below what the Rust files count now (Bigot 10 vs ≈23; estimates in each entry, by a Python
  re-implementation of `count_loc`: comments, blanks and `use` lines out, above `#[cfg(test)]`). A snapshot's `loc` is
  therefore its own count, so C #48 Hired Shrimp, C+ #44 Simplicity Audit and C+ #45 Complexity Audit, which compare `loc`,
  treat a snapshot unlike its Core card. Either accept that (default: "plays exactly as" excludes those three readers), or
  P2 gives `loc_failure` a snapshot exception that takes the Core card's frozen `loc` (⚠ owner). R1846000's code lines
  (`crates/cards/build.rs` on every build, `use` lines in, `BUSTED_CODE_LINES = 100` and `COMPLICATED_CODE_LINES = 60`,
  `crates/engine/src/config.rs:78`, `:82`) follow the copied file; only #49 (≈77 now, with its kept script) and #57 (≈84)
  are over 60, so both snapshots are "complicated" like their Core cards; dropping #49's kept script keeps it over 60.
- **Other shared facts.** The AI's per-card rows (`crates/ai/src/shadow_ban.rs`: `SHADOW_WATCH` at 369, `DEALT_Q` at 767,
  keyed by id) have none for a snapshot, which deals at `DEALT_Q_PRIOR` until a sweep. `apps/web/src/audio/card-audio.json5`,
  `crates/cards/flavour.json` and `crates/cards/chinese.json` need a row per snapshot (copy the Core card's; the Chinese name
  under TG-P2-4). `apps/web/src/audio/music-cards.json` gives core-023 a `"station": "lofi"`; #49 gets one only if the owner
  wants (no rule reads it). Tags: no new tag; the snapshots add Human ×4, Felinor ×2, Jlockeed ×2, Jelly Bean ×1, Fruit ×1 to
  the census once the set ships, and two pins over **all** entries fail as soon as the entries land: the Jlockeed pin
  (`catalog.rs:8262`, R278: add `landed(&["traininggrounds-045","traininggrounds-048"])`) and the Jelly Bean pin
  (`catalog.rs:8561`, R1837000: `tagged(Tag::JellyBean)` == core-026…029 exactly; #50 and TG #27 Hopeful Jelly Bean join it).
  A TG pool named by set (`set: "Training Grounds"`, TG #20, #36, #38, #39's entries) holds the snapshots too (⚠ designer:
  "a random Training-Grounds card" may mean #1–#40 only).
- **As built (#2195, [[R2195000]]–[[R2195003]]).** The pair is `snapshotOf` on the snapshot's entry, read through
  `catalog::pair_key` by `validate_deck_with`, the shared validator's L3, `build_ai_deck` (so every server and
  practice deal) and the fuzz deal; the draft rules (`check_deck_draft`, `validator.rs:681`) are not, since a draft
  is judged by L3 when it is played, and the deck builder shows the other of a pair unavailable ("in this deck, as
  its Core card"). Each snapshot's `loc` is its own count (TG6 #27's default). Besides `loc`, a snapshot's element
  ([[R980]]) and Meditative #14 Prime Time read its own number, so "plays exactly as" holds for the card's own
  behaviour, not for what other cards read off its number.

#### Training Grounds #41 · Legacy: Bigot
`traininggrounds-041` · (2) Unit, Human · Common (Core Common) · 4/1 → 8/2 · built by P2 (#2195)

> **Designer:** 1 Bigot

- **Text:** Cry: Destroy an enemy non-Human Unit.
- **Radiant:** Cry: Destroy all enemy non-Human Units.
- **Engine:** Copy `core/c002_bigot.rs` (`ID` "core-002") → `training_grounds/c041_bigot.rs`. Base: `TargetDecl::target(1, 1, { side: enemy, of: [unit], notTags: [Human] })`, Cry `destroy { target: chosen }`; Radiant: no target, `destroy_all { side: enemy, rows: [units], notTags: [Human] }` (`effects/destroy.rs`). Reads the dead/target's tags, not its own; no self-reference. Named by S #88 Siva 2 (`c088_siva_2.rs:15` `BIGOT = "core-002"`, refs core-002): TG-P2-4. No kept script. `loc` 10 (Core, frozen) vs ≈23 (copy).
- **Rulings:** none new (R81, R90, R59; TG-P2-1…5).
- **Numbers:** none (Core declares none).
- **Check:** (1) plays exactly as core-002 at the foundation's commit; (2) destroys a chosen enemy non-Human; (3) a Human is not targetable; (4) with no legal target it enters anyway; (5) radiant destroys every enemy non-Human, Humans survive, no target declared. R275: Meets (radiant-audit, stats doubled, one target → all). Typos: none. ⚠ designer: none.
- **Class:** A.

#### Training Grounds #42 · Legacy: Stockpile
`traininggrounds-042` · (1) Spell · Common (Core Common) · built by P2 (#2195)

> **Designer:** 2 Stockpile

- **Text:** Draw {draw}. Heal your hero {heal}.
- **Radiant:** Draw {draw}. Heal your hero {heal}.
- **Engine:** Copy `core/c005_stockpile.rs` → `c042_stockpile.rs`. Cry `draw { count: param("draw") }`, `heal { target: selfHero, amount: param("heal") }`; `radiant = base.clone()`. Named by C+ #61 Bauble Bubble (`c061_bauble_bubble.rs:21` `STOCKPILE = "core-005"`): TG-P2-4 once the set ships. Core has a `SHADOW_WATCH` row (`shadow_ban.rs:535`). `loc` 7 vs ≈14.
- **Rulings:** none new (§2.4, R4, R386).
- **Numbers:** draw 2 ↑ (Radiant 4, step 1, min 1); heal 2 ↑ (Radiant 4, step 1, min 1).
- **Check:** (1) plays exactly as core-005 at the foundation's commit; (2) draws 2; (3) heals its hero 2, past 30; (4) a full hand burns the overflow; (5) radiant draws 4, heals 4. R275: Meets (2×). Typos: none.
- **Class:** A.

#### Training Grounds #43 · Legacy: Mr. Vanilla
`traininggrounds-043` · (1) Unit, Human · Common (Core Common) · 4/4 → 12/12 · built by P2 (#2195)

> **Designer:** 3 Mr vanilla

- **Text:** (none)
- **Radiant:** (none)
- **Engine:** Copy `core/c008_mr_vanilla.rs` → `c043_mr_vanilla.rs`: two `Script::default()`. Nothing to change but `ID`. `VOICE_PREVIEW_DEF_ID = "core-008"` (`apps/web/src/audio/constants.ts:49`) stays Core's. `loc` 3 vs ≈6.
- **Rulings:** none new.
- **Numbers:** none.
- **Check:** (1) plays exactly as core-008 at the foundation's commit; (2) a 4/4 with no text that Sheepish transforms like any Unit; (3) radiant a 12/12 with no text, made Radiant on the field it takes 12/12 at once keeping its damage. R275: the designer's patch v0.1.1 verdict (stats tripled, no text). Typos: "Mr vanilla" → "Mr. Vanilla".
- **Class:** A.

#### Training Grounds #44 · Legacy: Duplicating Felinors
`traininggrounds-044` · (2) Unit, Felinor · Rare (Core Rare) · 3/4 → 6/8 · built by P2 (#2195)

> **Designer:** 4 Duplicating felinors

- **Text:** Cry: Summon a copy of this.
- **Radiant:** Cry: Summon a copy of this.
- **Engine:** Copy `core/c012_duplicating_felinors.rs` → `c044_duplicating_felinors.rs`. Cry `summon_copy { of: { of: self }, player: self }` (`effects/summon.rs`). `{ of: self }` is the snapshot's own instance, so the copy is a `traininggrounds-044` (not a Core card): correct and the point of the copy. `loc` 4 vs ≈15.
- **Rulings:** none new (R57, R64, R1/§6.2).
- **Numbers:** none.
- **Check:** (1) plays exactly as core-012 at the foundation's commit; (2) the copy is a snapshot (`traininggrounds-044`) in the leftmost free zone; (3) its Cry does not fire; (4) buffs copied, damage not; (5) full board → no copy; (6) radiant 6/8 and its copy a Radiant 6/8. R275: Meets (raised in v0.3.10). Typos: "felinors" → "Felinors".
- **Class:** A.

#### Training Grounds #45 · Legacy: Jlockeed's Weapons
`traininggrounds-045` · (4) Field Spell, Jlockeed · Common (Core Common) · built by P2 (#2195)

> **Designer:** 5 Jlockheed weapons

- **Text:** Aura: Your Units have +{attack} Attack, Rush and First Strike.
- **Radiant:** Aura: Your Units have +{attack} Attack, Rush and First Strike.
- **Engine:** Copy `core/c014_jlockeeds_weapons.rs` → `c045_jlockeeds_weapons.rs`. `aura_hook` → `AuraEntry { applies: controller == self's && in_units_row_or_standing, mod_: StatMod { attack: param("attack"), keywords: [Rush, FirstStrike] } }`; `radiant = base.clone()`. Jlockeed tag: R278's pin over all entries (`catalog.rs:8262`) needs this id (TG-P2 shared facts). `loc` 12 vs ≈26.
- **Rulings:** none new (§10.4 layer 5, R1081000, R386).
- **Numbers:** attack 3 ↑ (Radiant 6, step 1, min 1).
- **Check:** (1) plays exactly as core-014 at the foundation's commit; (2) your Units +3 Attack, Rush, First Strike while it is on the field; (3) Units summoned later get it; (4) gone when it is destroyed; (5) radiant +6. R275: Meets (2×). Typos: "Jlockheed weapons" → "Jlockeed's Weapons" (R381: the faction is spelt Jlockeed).
- **Class:** A.

#### Training Grounds #46 · Legacy: Hit Job
`traininggrounds-046` · (3) Spell · Common (Core Common) · built by P2 (#2195)

> **Designer:** 6 Hit job

- **Text:** Destroy a Unit.
- **Radiant:** Destroy a Unit and the Units adjacent to it.
- **Engine:** Copy `core/c016_hit_job.rs` → `c046_hit_job.rs`. Target `{ side: any, of: [unit] }`; `destroy { target: chosen }`; radiant adds `destroy_adjacent_to { target: chosen }`. `loc` 12 vs ≈22.
- **Rulings:** none new (R46, R59, R81, R90, §3.1).
- **Numbers:** none.
- **Check:** (1) plays exactly as core-016 at the foundation's commit; (2) the target is destroyed; (3) an Indestructible target survives; (4) radiant also destroys its same-side neighbours, never across the centre line. R275: Meets. Typos: "Hit job" → "Hit Job".
- **Class:** A.

#### Training Grounds #47 · Legacy: Midrange Menace
`traininggrounds-047` · (3) Unit · Common (Core Common) · 9/9 → 18/18 · built by P2 (#2195)

> **Designer:** 7 Midranges menace

- **Text:** Taunt / End of turn: Heal this to full.
- **Radiant:** Taunt, Immutable / End of turn: Heal this to full.
- **Engine:** Copy `core/c019_midrange_menace.rs` → `c047_midrange_menace.rs`. `end_of_turn` → `heal { target: self, toFull: true }`; keywords printed (base [Taunt], radiant [Taunt, Immutable]), granted by no script. `loc` 8 vs ≈15.
- **Rulings:** none new (§6.3 Heal, R23).
- **Numbers:** none.
- **Check:** (1) plays exactly as core-019 at the foundation's commit; (2) Taunt enforced; (3) heals to full at its controller's end of turn, not the opponent's; (4) radiant Immutable refuses Sheepish. R275: Meets. Typos: "Midranges menace" → "Midrange Menace". ⚠ designer: TG #5 is "Midlane Menace" — distinct name, one letter-group apart; both stay.
- **Class:** A.

#### Training Grounds #48 · Legacy: Jlockeed Shredder-10
`traininggrounds-048` · (3) Unit, Jlockeed · Common (Core Common) · 6/10 → 12/20 · built by P2 (#2195)

> **Designer:** 8 Jlockheed shredder

- **Text:** End of turn: Deal {damage} damage to each enemy.
- **Radiant:** End of turn: Deal {damage} damage to each enemy.
- **Engine:** Copy `core/c013_jlockeed_shredder_10.rs` → `c048_jlockeed_shredder_10.rs`. `end_of_turn` → `damage_all { side: enemy, amount: param("damage"), heroes: true }` (`effects/damage.rs`); `radiant = base.clone()`. Jlockeed pin as #45. `loc` 10 vs ≈12.
- **Rulings:** none new (R51, R59, R386).
- **Numbers:** damage 2 ↑ (Radiant 4, step 1, min 1).
- **Check:** (1) plays exactly as core-013 at the foundation's commit; (2) at its controller's end of turn 2 to each enemy Unit and the enemy hero, one instance each; (3) Units it kills die after every hit lands; (4) not at the opponent's end of turn; (5) radiant 4. R275: Meets. Typos: "Jlockheed shredder" → "Jlockeed Shredder-10" (the Core name, R381's spelling).
- **Class:** A.

#### Training Grounds #49 · Legacy: Reoccurring Dream
`traininggrounds-049` · (1) Spell · Rare (Core Rare) · built by P2 (#2195)

> **Designer:** 9 Reoccuring dream

- **Text:** {chance}% chance a random card in your hand becomes Radiant. / End of turn: Return this to hand.
- **Radiant:** Lucky 1 / {chance}% chance a random card in your hand becomes Radiant. / End of turn: Return this to hand.
- **Engine:** Copy `core/c023_reoccurring_dream.rs` → `c049_reoccurring_dream.rs`, **without** `script_v0_4_4b` (kept for core-023 in `KEPT_SCRIPTS`, `crates/cards/src/history.rs:187`, R767000) and so without `BASE_CHANCE`/`RADIANT_CHANCE` and the `declared` switch: `script()` is `dream(true)`, odds = `param("chance") / 100`. Cry: `ctx.rng.lucky(printed Lucky (0 / RADIANT_LUCKY 1) + given_lucky_on + luck_of(controller), rng.chance(odds))` → `set_radiant_random { zones: hand, count: 1 }`; no roll with an empty hand. `end_of_turn` → `bounce { target: self }` when `return_to_hand_at_end_of_turn` or `was_played_this_turn` (reads its own instance, not its id). Radiant keyword `Lucky 1` printed. Code lines ≈77 now (complicated, > 60); without the kept script still > 60. `loc` 33 vs ≈76 now (a few fewer once the kept script and the printed-odds switch are dropped). Music station: TG-P2 shared facts.
- **Rulings:** none new (R60, R68, R129, R177, R987, R1438, §5.1).
- **Numbers:** chance 30 ↑ (Radiant 40, step 10, min 1, max 100).
- **Check:** (1) plays exactly as core-023 at the foundation's commit (its current script, not v0.4.4b's); (2) a seeded 30% roll makes one non-Radiant hand card Radiant; (3) returns from the graveyard to hand at the end of the turn it was played; (4) a full hand burns it; (5) radiant two rolls at 40% keeping a success. R275: Meets. Typos: "Reoccuring dream" → "Reoccurring Dream".
- **Class:** B.

#### Training Grounds #50 · Legacy: Glowy Jelly Bean
`traininggrounds-050` · (3) Spell, Jelly Bean · Rare (Core Rare) · built by P2 (#2195)

> **Designer:** 10 Glowy Jelly Bean

- **Text:** Make a card in your hand Radiant.
- **Radiant:** Make 2 cards in your hand Radiant.
- **Engine:** Copy `core/c026_glowy_jelly_bean.rs` → `c050_glowy_jelly_bean.rs`. `TargetDecl::hand(picks, picks, { of: [hand] })`, `set_radiant { target: chosen, index }` per pick (1, radiant 2). Jelly Bean tag: R1837000's pin (`catalog.rs:8561`) fails on landing; R739's "___ Glowy Jelly Bean" family (`catalog.rs` ~7834, Rare) lists ids, so the snapshot stays Rare. Joins S #7 The Greatest Heist's `anyTags: [Acclaimed, Jelly Bean]` Discover once TG ships. `loc` 12 vs ≈18.
- **Rulings:** none new (R74, R81, R90).
- **Numbers:** none (the pick count is the face, as Core).
- **Check:** (1) plays exactly as core-026 at the foundation's commit; (2) the chosen hand card becomes Radiant, picked with the play, never a prompt; (3) never itself; (4) radiant chooses 2, or the one card there is. R275: Meets (1 → 2). Typos: none.
- **Class:** A.

#### Training Grounds #51 · Legacy: Big Felinor
`traininggrounds-051` · (4) Unit, Felinor · Rare (Core Rare) · 3/10 → 6/20 · built by P2 (#2195)

> **Designer:** 11 Big felinor

- **Text:** Cry: Destroy all non-Felinor Units.
- **Radiant:** Cry: Destroy all enemy non-Felinor Units.
- **Engine:** Copy `core/c043_big_felinor.rs` → `c051_big_felinor.rs` (its header says "cost 3"; the catalog's 4 wins). Cry `destroy_all { side: any | enemy, notTags: [Felinor] }`. Spares itself through its own Felinor tag, which the snapshot keeps. `loc` 8 vs ≈15.
- **Rulings:** none new (R59, R46, R13).
- **Numbers:** none.
- **Check:** (1) plays exactly as core-043 at the foundation's commit; (2) every non-Felinor Unit on both sides destroyed in one state check; (3) Felinors and itself survive; (4) radiant the enemy side only. R275: Meets. Typos: "felinor" → "Felinor".
- **Class:** A.

#### Training Grounds #52 · Legacy: Snom Bunny Mind Control
`traininggrounds-052` · (4) Spell · Rare (Core Rare) · built by P2 (#2195)

> **Designer:** 12 Snom bunny mind control

- **Text:** Steal an enemy Unit.
- **Radiant:** Steal an enemy Unit. Make it Radiant.
- **Engine:** Copy `core/c049_snom_bunny_mind_control.rs` → `c052_snom_bunny_mind_control.rs`, **without** `script_v0_3_7b` and `enemy_permanent` (kept for core-049 in `KEPT_SCRIPTS`, `history.rs:100`): `script()` is `scripts_with(enemy_unit)`, or inline it. Target `{ side: enemy, of: [unit] }`; `steal { target: chosen }`; radiant then `set_radiant { target: chosen }`. `loc` 12 vs ≈32 (≈26 without the kept script).
- **Rulings:** none new (R12, R13, R15, R22, R74, R81, R90).
- **Numbers:** none.
- **Check:** (1) plays exactly as core-049 at the foundation's commit (its current script, Units only, not v0.3.7b's); (2) targets an enemy Unit only, never a Field Spell or Trap; (3) placement per R15 (same lane, else first free, else stays); (4) `owner` never moves; (5) radiant the stolen Unit becomes Radiant. R275: Meets. Typos: "Snom bunny mind control" → "Snom Bunny Mind Control".
- **Class:** A.

#### Training Grounds #53 · Legacy: Prejudiced Postdoc
`traininggrounds-053` · (2) Unit, Human · Rare (Core Rare) · 2/4 → 4/8 · built by P2 (#2195)

> **Designer:** 13 prejudice post doc

- **Text:** Cry: Choose a Human Unit on the field. Summon a Vanilla copy of it.
- **Radiant:** Cry: Choose any Unit on the field. Summon a Vanilla copy of it.
- **Engine:** Copy `core/c061_prejudiced_postdoc.rs` → `c053_prejudiced_postdoc.rs`. Target `{ side: any, of: [unit], tags: [Human] }` (radiant no tag); `summon_copy { of: chosen, vanilla: true, grantedKeywords: false }`. `loc` 27 vs ≈28.
- **Rulings:** none new (R23, R57, R64, R781001, R781003).
- **Numbers:** none.
- **Check:** (1) plays exactly as core-061 at the foundation's commit; (2) the Vanilla copy of a Human keeps its form at printed stats, no buffs, keywords, text or damage; (3) auras apply afresh; (4) an Immutable target is legal; (5) radiant any Unit. R275: Meets. Typos: "prejudice post doc" → "Prejudiced Postdoc".
- **Class:** A.

#### Training Grounds #54 · Legacy: The Rock
`traininggrounds-054` · (4) Unit, Human · Common (Core Common) · 10/10 → 20/20 · built by P2 (#2195)

> **Designer:** 14 the rock

- **Text:** Tribute 1, Indestructible
- **Radiant:** Tribute 1, Indestructible, Immutable
- **Engine:** Copy `core/c066_the_rock.rs` → `c054_the_rock.rs`. `static_flags.tribute = TRIBUTE_COST (1)`, read by `tribute_cost_of` / `refuse_tributes` (`play_choices.rs`); keywords printed (Indestructible; radiant + Immutable). Tribute is a static flag, not a catalog keyword. `loc` 4 vs ≈15.
- **Rulings:** none new (§6.3 Tribute, §3.2, R46, R69, R23).
- **Numbers:** none (Tribute 1 is the face's).
- **Check:** (1) plays exactly as core-066 at the foundation's commit; (2) play refused without a tribute; (3) a Sheep Token pays it; (4) Indestructible under damage and a destroy mark, dies at 0 max health; (5) radiant also Immutable. R275: Meets. Typos: "the rock" → "The Rock".
- **Class:** A.

#### Training Grounds #55 · Legacy: Reminisce
`traininggrounds-055` · (1) Spell · Rare (Core Rare) · built by P2 (#2195)

> **Designer:** 15 reminisce

- **Text:** Discover a card from your graveyard. It costs ({discount}) less. Exile this.
- **Radiant:** Discover a card from your graveyard. It costs ({setCost}). Exile this.
- **Engine:** Copy `core/c072_reminisce.rs` → `c055_reminisce.rs`. Cry `discover_from_graveyard { step: "chosen" }` then `exile { target: self }` (parked); `resume["chosen"]` → `add_to_hand { instance: chosen }` + `set_cost_mod -discount` (base) / `set_cost_override setCost` (radiant), `inHandOnly`. The resume is found by the stored def id (R127), so it is the snapshot's own table. Named by Core #76 Field of Dreams (`c076_field_of_dreams.rs:20` `REMINISCE = "core-072"`), which keeps making core-072: TG-P2-4. `loc` 17 vs ≈32.
- **Rulings:** none new (R50, R65, R77, R4).
- **Numbers:** discount 1 ↑ (tunedOn base); setCost 0 ↓ (tunedOn radiant, min 0).
- **Check:** (1) plays exactly as core-072 at the foundation's commit; (2) Discovers from the actual graveyard, spell tokens included; (3) the pick costs 1 less; (4) Reminisce is exiled; (5) an empty graveyard opens nothing and it is still exiled; (6) radiant the pick costs 0. R275: Meets. Typos: "reminisce" → "Reminisce".
- **Class:** B.

#### Training Grounds #56 · Legacy: Corpse Eater
`traininggrounds-056` · (4) Unit · Epic (Core Epic) · 2/2 → 4/4 · built by P2 (#2195)

> **Designer:** 16 corpse eater

- **Text:** Rush / While this is in your hand, it gains the attack and health of each Unit that dies.
- **Radiant:** Rush, Divine Shield / While this is in your hand, it gains twice the attack and health of each Unit that dies.
- **Engine:** Copy `core/c089_corpse_eater.rs` → `c056_corpse_eater.rs`. `hand_triggers: [TriggerDef::new("corpse-eater-feed", [Destroyed], …)]`: a non-token Unit's death (`def_of(def_id)`, `dead.token` skipped) → `buff { target: self, attack, health }` ×1 / ×2, clamped at 0 (R219). The trigger id is per definition (a queued entry names the card's def and the hook id), so the copy may keep "corpse-eater-feed". `loc` 23 vs ≈39.
- **Rulings:** none new (R11, R38, R68, R78, R89, R219).
- **Numbers:** none.
- **Check:** (1) plays exactly as core-089 at the foundation's commit; (2) in hand it gains each dying Unit's current attack and max health, either side; (3) tokens excluded; (4) stops once on the field; (5) radiant double. R275: Meets. Typos: "corpse eater" → "Corpse Eater".
- **Class:** B.

#### Training Grounds #57 · Legacy: Recycling Initiative
`traininggrounds-057` · (0) Spell · Epic (Core Epic) · built by P2 (#2195)

> **Designer:** 17 recycling initiative

- **Text:** Exile this. / End of turn: Add a copy of each other card you played this turn to your hand.
- **Radiant:** Exile this. / End of turn: Add a Radiant copy of each other card you played this turn to your hand. They cost ({discount}) less.
- **Engine:** Copy `core/c039_recycling_initiative.rs` → `c057_recycling_initiative.rs`. Cry `delay { at: { phase: end, player: self }, step: "copies", hook: RESUME_HOOK, data: { selfId } }` then `exile self`; `resume["copies"]` reads `played_ids_this_turn` (query.rs), skips its own instance id (`selfId`), dedupes, skips vanished instances (`find_instance`), `add_to_hand { defId, radiant, costMod: -discount, chinese }`. The delayed continuation names its script by the stored def id (R127), the snapshot's; it excludes itself by instance id, not def id, so a Core Recycling Initiative played the same turn is copied like any other card (and vice versa). Code lines ≈84 (complicated). `loc` 50 vs ≈78.
- **Rulings:** none new (R57, R62, R65, R71, R86, R126, R127, R133, R1300).
- **Numbers:** discount 1 ↑ (tunedOn radiant).
- **Check:** (1) plays exactly as core-039 at the foundation's commit; (2) exiled on play; (3) at end of turn a copy of every other card played this turn, later plays included, each id once; (4) vanished instances skipped; (5) radiant copies are Radiant and cost 1 less. R275: Meets (raised). Typos: "recycling initiative" → "Recycling Initiative".
- **Class:** B.

#### Training Grounds #58 · Legacy: True Strike
`traininggrounds-058` · (1) Spell · Common (Core Common) · built by P2 (#2195)

> **Designer:** 18 true strike

- **Text:** Pierce / Deal {damage} damage. Exile this.
- **Radiant:** Pierce / Deal {damage} damage. Exile this.
- **Engine:** Copy `core/c044_true_strike.rs` → `c058_true_strike.rs`. Target `{ side: any, of: [unit, hero] }`; `damage { to: chosen, amount: param("damage"), ignoreArmor: true }` + `exile self`. Catalog: `Pierce` keyword on both faces, `dealsDamage: true` (R1748001) copied. `loc` 12 vs ≈19.
- **Rulings:** none new (R346, R63, R81, R90).
- **Numbers:** damage 4 ↑ (Radiant 8, step 1, min 1).
- **Check:** (1) plays exactly as core-044 at the foundation's commit; (2) 4 damage through Armor; (3) Divine Shield still blocks; (4) exiled; (5) radiant 8. R275: Meets (2×). Typos: "true strike" → "True Strike".
- **Class:** A.

#### Training Grounds #59 · Legacy: Fig of Life
`traininggrounds-059` · (3) Spell, Fruit · Common (Core Common) · built by P2 (#2195)

> **Designer:** 19 fig of life

- **Text:** Heal a target {heal}.
- **Radiant:** Heal a target {heal}.
- **Engine:** Copy `core/c047_fig_of_life.rs` → `c059_fig_of_life.rs`. Target `{ kind: target, filter: { side: any, of: [unit, hero] }, aim: help }`; `heal { target: chosen, amount: param("heal") }`. Fruit tag: joins Fruit pools once TG ships. `loc` 9 vs ≈21.
- **Rulings:** none new (R19, R656, §6.3 Heal).
- **Numbers:** heal 20 ↑ (Radiant 40, step 5, min 1).
- **Check:** (1) plays exactly as core-047 at the foundation's commit; (2) heals a Unit up to its max; (3) heals a hero with no cap; (4) any Unit or hero, either side; (5) radiant 40. R275: Meets (2×). Typos: "fig of life" → "Fig of Life".
- **Class:** A.

#### Training Grounds #60 · Legacy: Rush Token Farm
`traininggrounds-060` · (2) Field Spell · Common (Core Common) · built by P2 (#2195)

> **Designer:** 20 Rush token farm

- **Text:** Start of turn: Summon a Rush Token.
- **Radiant:** Start of turn: Summon a Rush Token. / Aura: Your Rush Tokens have +{buff}/+{buff}.
- **Engine:** Copy `core/c058_rush_token_farm.rs` → `c060_rush_token_farm.rs`. `RUSH_TOKEN = "core-t-rush"` stays: the snapshot summons and buffs Core's shared Rush Token (§7), so no new token and `refs: ["core-t-rush"]`. `start_of_turn` → `summon { defId: core-t-rush }`; radiant also `aura_hook` on `unit.controller == self's && unit.def_id == RUSH_TOKEN`, +buff/+buff. `loc` 11 vs ≈33.
- **Rulings:** none new (R62, R64, §10.4 layer 5).
- **Numbers:** buff 3 ↑ (tunedOn radiant).
- **Check:** (1) plays exactly as core-058 at the foundation's commit; (2) a Rush Token each start of its controller's turn; (3) full row → nothing; (4) radiant also +3/+3 on its controller's Rush Tokens only, gone with the Field Spell. R275: Meets. Typos: "Rush token farm" → "Rush Token Farm".
- **Class:** A.

## TG6. Readings for the designer

Each row is the build's default until the designer (or the owner) says otherwise; while the set is held a change is a
correction to the rows, and after it ships a patch of its own.

| # | Card | The unclear words | The default |
| --- | --- | --- | --- |
| 1 | #2 Personal Trainer | Radiant "a friendly Human" | One of your other Humans, as the base face: targets are picked in hand ([[R81]]), so it can never pick itself (TG-P3-1) |
| 2 | #3 Group leader, #15 In a hurry, #34 Dino-size | lower-case names | As printed, as Core's "Right-house defender" |
| 3 | #4 Boulder Fist Ogre, #6 Chosen by the Sun | Radiant adds no keyword ([[R275]] asks one more or a stronger one) | As printed, the designer's word (the radiant-audit rows say so, as S #68 Stratos's); Charge for #4, one more keyword for #6 would meet it |
| 4 | #8 Clobberer | Radiant 42/14 triples only the attack | As printed; 42/21 would triple both, as C #86 Genn |
| 5 | #5 Midlane Menace | "the Middle lane"; the name one word from Core #19 Midrange Menace | Midlane, lane 3 of 5 ([[R685]]); both names stay |
| 6 | #9 Ravager | "On kill"; Radiant 17 health | Whenever this destroys a Unit, on any turn, a defending kill included (C+ #19.5 Bot Loser); 14/17 as printed |
| 7 | #10 Fan of Knives, #11 Thwomp, #18 Knife Juggler | Radiant numbers past R275's 2–2.5× band (2× and Echo; 10×; 3×) | As printed |
| 8 | #13 Explosive Runes, #14 Explosive Trap | the names swap Hearthstone's two cards' effects | As printed; the effects are read as written (#13 springs before damage, #14's Trample is the excess to the hero) |
| 9 | #13 Explosive Runes | "a unit attacks your hero", "all enemies" | An enemy Unit's declared attack (a forced attack opens no window, [[R121]]); the enemy hero and every enemy Unit, the attacker among them |
| 10 | #15 In a hurry | "die at the end of this turn" against "{give them brittle 1}" | The text: destroyed at the end of the turn they arrive, whoever's; they print no Brittle (Brittle 1 would keep them through the opponent's turn) |
| 11 | #16 Celestial Caller | "to the left / right of this" | The zone one lane lower / higher on its side ([[R1060]]); an edge lane or a taken zone makes that one summon fail |
| 12 | #17 Flaming Felinor | "all other units" | Both sides, itself excepted; no hero |
| 13 | #18 Knife Juggler | "when you summon a unit", "a random enemy" | Any Unit arriving on your side, a played one after it resolves, its own arrival excepted; the enemy hero or an enemy Unit |
| 14 | #22 Right-Hand Felinor | "to the right of a unit" | The Unit in lane N−1 on its own side, the owner's right (no client mirrors the board) |
| 15 | #23 Quadrupling Felinors | four bodies for (3) | As printed (balance) |
| 16 | #24 Goliath Situation | "On reveal", "its cost" | Every firing of the Field Trap ([[§5.1]]: it stays and fires again); the cost paid ([[R56]]); a thin Radiant face, as printed |
| 17 | #25 Trigor | "a Trigor with +3/+3" | Cumulative: each new Trigor is the dying one's base stats +3/+3 (3/3, 6/6, 9/9 …) (TG-P4-3), overridden as the terminator by R2197002 and R2197004; the Radiant face's successors are base Trigors from 12/12 |
| 18 | #28 Occidental, #29 Oriental | no type; "right most / left most lane"; the West hits the right | Units; lane 5 and lane 1, both sides, both rows, the card itself included on the base face; as written; #29 CN only |
| 19 | #31 Level Up! | "your units attack 3 times" | Each Unit counts its own attacks, across turns, from when this arrives; Radiant after the combat of its third |
| 20 | #32 Selfless Protector | the Radiant Cry's "your units" | Includes itself |
| 21 | #33 Guerilla Felinor | "repeat 7 times"; "Guerilla" | 7 hits in all; the name as printed |
| 22 | #35 Corpse Recycler | whose units; where the token goes | Both sides' non-token Units, each token for the dead Unit's controller, in [[R64]]'s leftmost open zone (not the zone it died in) |
| 23 | #36 Training Dummy | the Radiant face drops "Can't be in Defense Position" | As printed, as C+ #48 Jlockeed's Lobbyist's Radiant face |
| 24 | #37 Seance | "exact copy" | [[R57]]'s field copy plus its price; damage, exertion and counters reset; any permanent, either side |
| 25 | #39 Call to Chaos (TG Edition) | "every other turn"; "a Call to Chaos"; the hyphen | Every second turn of yours, one at once; a random edition at (0); the names keep the hyphen, the set's name has none |
| 26 | #12 Fortify | R279 reads M #22's "Fortify Mind" as naming it | The name stays; "Fortify Mind" joins `RULES_PHRASES` |
| 27 | #41–#60 | the snapshots' `loc` | Each its own script's count (frozen Core counts are older and smaller), so C #48 Hired Shrimp and C+ #44, #45 read a snapshot unlike its Core card (⚠ owner) |
| 28 | #41–#60 | a Core card's text that names a card the snapshot shares | Names the Core card alone (S #88 Siva 2's Bigots, C+ #61's Stockpile, Core #76's Reminisce keep making Core's) |

## TG7. Sound, visuals, the AI and the release

- **Sound.** Each card's `card-audio.json5` entry is written by the foundation ([[§10.11]]'s lines); #18 Knife Juggler's
  `play` line is the designer's "Put this apple on your head". No voice file is rendered or expected before the set
  ships ([[R1420]]); the files are owed with the release, rendered through the `voice` workflow (`[gen:voice]`, R1448).
  The intro snippets ([[R1352]], `gen:music`) for the Legendaries and the Mythic (#37 Seance, #38 Tutorial Master, #39
  Call to Chaos (Training-Grounds Edition), #40 Goetzmann Game Design) are P9's (#2202).
- **Visuals.** The set mark ([[R503]]) shows the plain diamond of a set the client does not know until P9 draws the set a
  glyph of its own; the client's new screens (a Level Up! count, the end-of-turn tokens, Call to Chaos's alternate-turn
  badge) come with the parts that build their systems, and P9 looks them over.
- **The AI** (P10, #2203): every new prompt and mode answered by `crates/ai` within its budget, and the sweep
  (`cargo jackioh sweep`) over the set with it previewed, at most half of it in `SHADOW_BAN` ([[R186]]).
- **The release** (P11, #2204) ships the set as the newest set: `SHIPPED_SETS` lists it last; one fragment claims its 61
  entries and every card that plays differently once it ships (the Call to Chaos editions and S #149 Wildar, whose
  recast pools gain #39; M #22 Mind Games' refs; R1370000); the server's seed takes the set (no new tag, so no
  `cards_tags_check` change); deck and trio codes take its offset after Stinkyscape's ([[R1410]]); the golden traces are
  re-blessed once for the bigger pools; the Almanac's and the deck builder's chips, the newest set's ribbon, the
  `RELEASES` line; and every stated total moves. The account's statistics cap (`PLAYER_STATS_BYTES_MAX`, 65,536 bytes,
  [[R1142000]]) no longer holds every entry at four-digit counters once the set's 61 entries ship (751 entries need
  more): the release raises it, with the server's body bound and `apps/web/src/stats/sync.test.ts`, which reads the
  shipped sets until then.
- **The tutorial** (P12, #2205) is remade from the shipped set: five lessons, the fifth the first to deal a Legendary or
  Mythic, the human's hero unlosable by P8's rule, and a coach that sets up each mechanic, asks rather than tells, names
  the rule once the player finds it and hints harder only when the player stalls. It replaces [[§9.10]]'s four lessons;
  a lesson deck cannot hold a held set's card (`state.rs:2267`), so it waits for P11.
- **The audit** (P13, #2206) runs REVIEW.md's paste-in prompt as a separate session (CLAUDE.md rule 8), with this
  brief, `JackiOh_TrainingGrounds_Cards.md` and §8.11 among Part A's inputs.

## TG8. The plan: parts, issues, order and ruling blocks

Every part is one issue and one pull request under the tracker #2193, built on `main` behind the gate (TG3). Each takes
its ruling numbers from its own issue's block, R(N×1000) to R(N×1000+999) (`docs/issues-and-patches.md`, Ruling
numbers), and names its proving tests after them (CLAUDE.md rule 3), its §11 paragraphs in `spec/pending/<N>.md`.

| Part | Issue | Cards | Systems | Blocked by | Rulings |
| --- | --- | --- | --- | --- | --- |
| P1 The foundation | #2194 | — (#1–#40, #39.1 stubbed) | the held set, the census, this brief, §8.11, §7, BUILD M13, the gate's tests | #1884 | R2194000… |
| P2 The snapshots | #2195 | 41–60 | the Core-or-snapshot pair rule; the name exceptions | #2194 | R2195000… |
| P3 Units on existing keywords and Cries | #2196 | 1–8, 16, 17, 22, 23, 28, 29, 33, 39.1 | none | #2194 | R2196000… |
| P4 Triggers, the adjacency aura, Trigor, the Death cards | #2197 | 9, 18, 21, 25, 30, 32, 36, 38 | the adjacency aura's ruling; Trigor's terminator | #2194 | R2197000… |
| P5 The spells | #2198 | 10, 11, 12, 15, 26, 27, 34, 37, 40 | die at end of turn; Cleave on a board pick; set stats on one Unit; an exact copy | #2194 | R2198000… |
| P6 The backrow | #2199 | 13, 14, 19, 20, 24, 31, 35 | a Trap's Pierce and Trample; Goliath Situation's firing; an after-attack hook for a non-attacker | #2194 | R2199000… |
| P7 Call to Chaos (TG Edition) | #2200 | 39 | its table; the every-other-turn add | #2194 | R2200000… |
| P8 The unlosable lesson | #2201 | — | a hero that cannot fall below 1; Legendary and Mythic held out before lesson 5 | #1884 | R2201000… |
| P9 Look and sound | #2202 | all | the set mark, the intros, the new screens | #2194 | R2202000… |
| P10 The AI | #2203 | all | every new prompt; the sweep | #2194 (and P2–P7 for its tests) | R2203000… |
| P11 The release | #2204 | all | the gate opens; the newest set | P1–P7, P9, P10 | R2204000… |
| P12 The tutorial | #2205 | — | five lessons, a discovery coach | P11, P8 | R2205000… |
| P13 The audit | #2206 | all | REVIEW.md, run separately | P11, P12 | R2206000… |

**Order.** The foundation runs first (P8 needs only Stinkyscape); P2 to P7, P9 and P10 then build side by side, since no
card part builds on another's system (P3's #39.1 is the Golem P7's eighth entry summons, its keywords only); the release
follows them, then the tutorial, then the audit.
