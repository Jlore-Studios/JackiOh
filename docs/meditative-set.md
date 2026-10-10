# Meditative — design brief for the implementing agents

2026-10-08 · written from the designer's list of the same day (issue #496, `JackiOh_Meditative_Cards.md`)

> **Status: the brief, and the plan of record.** SPEC is still the only source of rules (CLAUDE.md). The
> set-level rules this brief decides are in SPEC already ([[R1420]] to [[R1424]], [[§5]], [[§8.8]], [[§7]]);
> every card's row is in [[§8.8]] (tokens also in [[§7]]); each card's mechanics become SPEC when the
> part that builds them lands, with its rulings. This file records, for every card and every new
> mechanic, what the designer wrote (verbatim in the source note), what it most likely means in this
> engine, every place the text is ambiguous with the reading chosen, and the order the work is built in.
>
> Proposed rulings are numbered **MD-A1, MD-B1, …** (one series per drafting group, A to G). They are
> not `R<n>` rows. The part that proves one takes an R-number from its reserved block (M10) and names its
> test after it. **⚠ designer** marks a reading the designer should confirm; the build does not wait for
> it (M9 lists them, each with the default the build uses).

## Contents

- [M0. What the designer asked for](#m0-what-the-designer-asked-for)
- [M1. Reading this brief](#m1-reading-this-brief)
- [M2. The set: ids, numbers, tags, rarity and the census](#m2-the-set-ids-numbers-tags-rarity-and-the-census)
- [M3. Built on main before it ships (R1420)](#m3-built-on-main-before-it-ships-r1420)
- [M4. Nerf and Buff](#m4-nerf-and-buff)
- [M5. The systems](#m5-the-systems)
- [M6. Cards, card by card](#m6-cards-card-by-card)
- [M7. Proposed rulings](#m7-proposed-rulings)
- [M8. The visuals, sound, AI, decks and Almanac pass](#m8-the-visuals-sound-ai-decks-and-almanac-pass)
- [M9. Decisions for the designer](#m9-decisions-for-the-designer)
- [M10. The plan: parts, issues, order and ruling blocks](#m10-the-plan-parts-issues-order-and-ruling-blocks)

---

## M0. What the designer asked for

Issue #496, "v0.3.X: Large Patch: Meditative & Visuals Pass", verbatim in `JackiOh_Meditative_Cards.md`:

- **A new set, Meditative**: 97 numbered entries plus two unnumbered stubs, 99 cards in all once the
  numbering is fixed (M2), and three more the designer added after this brief (issue #571), 102 cards,
  and the 30 tokens they make: 132 catalog entries.
- **Keywords**: Degrade is renamed **Nerf** and Upgrade **Buff** (M4).
- **Cosmetics**: hero portraits can be clicked on and have more vivid art; more emotes, dealt at random
  each game.
- **Sound**: every Legendary and Mythic card plays an intro music snippet when played, as Hearthstone's
  legendaries do; more sound effects for niche interactions — a hit Armor halves or more, a hit Armor
  stops entirely, and any other case worth one.
- **AI**: the AI beats the previous AI in at least 40 of 100 games with the new cards, at least 80% of
  the new cards in its pool of allowed (not shadow-banned) cards.
- **Randomized decks**: a skewed option for random decks (against the AI and in All Random duels) with at
  least half of the deck from the new set.
- **Misc**: the Almanac updated with the new cards; any other small polish found on the way.

M8 reads each non-card item; M10 turns all of it into parts.

## M1. Reading this brief

- **Numbers.** "§" is SPEC; "M3" is this brief's section M3; "#N" alone is Meditative card N; "C #N",
  "C+ #N" and "Core #N" are the other sets' cards.
- **A card entry** (M6) has the heading (set, number, name), the header line (id · cost, type and tags ·
  rarity · stats base → Radiant), the designer's text flattened onto one line, and then:
  **Text** and **Radiant** (the proposed `base.text` and `radiant.text`, house style, params as `{key}`),
  **Engine** (what the card does in the engine's terms, the existing verbs and rulings it uses, and the
  systems of M5 it needs), **Rulings** (the readings a builder needs; MD-numbers for new rulings),
  **Numbers** (the declared params: base → Radiant, ↑ when more is better for the controller), **Check**
  (R275, R276, typos, ⚠ designer items) and **Class** (A keywords or one primitive, B composed
  primitives, C a shared new system, D a subsystem of its own).
- **The groups.** The card entries were drafted in seven groups (A: #1–#24, B: #25–#38 and #98, C: #39–#49,
  D: #50–#75, E: #76–#94 and #99, F: #95–#97.9, and G: #100–#102, the three cards the designer added
  after the brief, read with issue #571) and their MD rulings are numbered per group. Where the
  groups disagreed, M5's decisions (D1–D13) settled it, and those decisions win over any entry.
- **Words.** House style as R366, R432 and R373 set it: "Deck" for the library, "Tribute" for Sacrifice,
  "Bounce" for a return from the field to hand ([[R692]]), "(N) Cost" for a cost as a noun and "costs
  (N)" as a verb, labelled abilities on lines of their own, Nerf and Buff for the tuning verbs (M4).

## M2. The set: ids, numbers, tags, rarity and the census

**Ids.** Card `meditative-NNN` (three digits), a token a card defines `meditative-NNN-k`; index strings
`"39"`, `"39.1"`. Scripts in `crates/cards/src/scripts/meditative/cNNN_slug.rs` (`c039_1_auspicious_rock.rs`).

**Numbering fixes.** The designer's numbers stand except:

| Designer's entry | Problem | Becomes |
| --- | --- | --- |
| #27 Clip-Farming Lawyer | listed twice, identically | one card, #27 |
| #28 Showdown and #28 Shade-iris | one number twice | Shade-iris keeps #28 (its token Ancient Curse is #28.1); Showdown is **#98** |
| #93 Paranoia and #93 Growing Felinor | one number twice | Growing Felinor keeps #93 (its tokens are #93.1–#93.3); Paranoia is **#99** |
| (0) Small Time Recruits, unnumbered, "Something with 1 costs" | a stub with no number | **#62**, the one hole; read from Hearthstone's Small-Time Recruits (draw three (1) Cost Units from your deck) |
| (0) Jlarna #89, no text | a stub | #89; read as the J-pun on Klarna, "pay in 4" (⚠ designer) |
| Empty Plot #100.1 | the nine Blueprint units are #97's | **#97.1** |
| #98 Gachaholic, #99 Catboy Maid SSR+ and #100 Greaser, added after the brief (issue #571) | #98 and #99 are Showdown's and Paranoia's already | Greaser keeps **#100**; Gachaholic is **#101** and Catboy Maid SSR+ **#102**, the designer's order at the end; the designer accepted it, so M9 has no row |
| #45.1, #49.1–#49.3, #70.1, #71.1, #91.1, #93.1–#93.3, #95.1, #96.1, #97.1–#97.9 | generated, so Tokens | tokens, `rarity: "Token"`, the designer's rarity as `printedRarity` |

**Types the designer left out or wrote loosely**: #8 Reach the Summit is a Spell (it returns to hand,
R746); #44 CN Jade Well is a Field Spell (a "Start of turn" Spell is the Mana Well shape); #49.3 AI
Girlfriend is an Animated Field Spell (it prints stats and is attacked).

**Tags.** The designer's, with `Jlockheed` read as the existing tag `Jlockeed` ([[R278]]; the names keep
"Jlockheed"). One new tag, **Wincon** (#8, #20). #69 The Maestro carries Plague, as every card that uses
Plague Counters does ([[§5]]). #87 Tatches the Totem's "All Tribes" is the five tribal tags at once
([[R1424]]). Census of the 132 entries: CN 33, Token 30, Human 12, Felinor 11, KY 6, Quickdraw 5,
Acclaimed 4, Jlockeed 3, Wincon 2, Plague 2, Catalyst 2, Prime 2, Fruit 1, Call to Chaos 1.

**Rarity.** The designer's, and for the five with none, §8's rubric: #98 Showdown Rare, #55 Dragon Fruit
Rare (a Fruit card that generates, as Fruit Basket), #56 House Party Common (one fill, as Friend of
Felinors), #62 Small Time Recruits Rare, #89 Jlarna Rare. The 102 cards: **29 Common, 29 Rare, 22 Epic,
16 Legendary, 6 Mythic**.

**Radiant stats.** [[R275]]'s stat half (a Unit's Radiant attack and health each at least double) holds
with no exception, as patch v0.2.10 chose over keeping named ones; the six Units where the designer kept
the stats (#12, #59, #83, #93–#93.3, #49.2, #97.5) are doubled, and #69 The Maestro, which the designer
gave no Radiant face, gets 8/8 and two exiles per zone (⚠ designer, M9).

**The census** (`crates/tools/src/catalog.rs`'s `sets()` and `crates/cards/tests/cross/catalog.rs`'s
`MEDITATIVE` fixture, both written once by the foundation part from the table in [[§8.8]]): every entry
the set holds must be one of these indices, with its row's name, cost, type, tags, rarity and stats.

## M3. Built on main before it ships (R1420)

The night bot builds from `main` and merges into `main` (`bot/harness/work.py`'s `base_ref`), so the set
is built there, card by card, without reaching a player until its last part ([[R1420]]):

- The set is `SetName::Meditative`, in `CATALOG_SETS` (catalog order) and not in `SHIPPED_SETS`.
- A pool that names no set draws from the sets that ship; one that names Meditative, or names a card by
  id, reaches it. So a Meditative card finds its own tokens, and no Core, Classic or Classic+ card, AI
  deck, determinization or random cast meets a Meditative card until the set ships.
- §9.4's L3 and `create_game` refuse a Meditative card in a deck; the Almanac's shelf and the deck
  builder's pool leave them out; the server neither seeds them nor serves them.
- `patches check` and `patches ship` read the catalog without them: **a card batch adds no pending
  fragment**, and the patch that ships the set claims all 132 entries in one fragment.
- The testkit can preview the set on one thread (`preview_sets(&[SetName::Meditative])`): a card test
  that needs Meditative cards in a generic pool previews it; the fuzz tool previews every set, so every
  Meditative card is fuzzed from the day it lands; the golden traces deal from the shipped sets, so no
  batch moves a recorded game.
- `catalog check` holds the set to its shape (M2's indices, ids, rarity ceilings) and keeps it out of
  the totals and the tag census; the cards crate's fixture holds each entry to its row.

**What a card batch therefore never touches**: the golden traces, the seed-pinned tests, the pending
fragments, the shipped totals and tag counts, and other sets' cards' behaviour. A batch whose change
moves a golden trace changed something that ships: it says what and why, or it is a bug.

**What a card batch adds**, per card (`docs/ADDING_CARDS.md` §1, with this set's differences): the
`catalog.json` entry (after the previous Meditative entry in index order; the census and fixture are
already written), the script file with its tests, the `card-audio.json5` entry (its rendering owed to a
person with macOS, `docs/ADDING_CARDS.md` §7), the `flavour.json` line, its Chinese entry once
`crates/cards/chinese.json` exists (MS01), and corrections to its own [[§8.8]], [[§7]], BUILD M10 and
`docs/radiant-audit.md` rows where its build decided a detail differently. New keywords and verbs get
their [[§6]] rows and ruling notes in the batch that builds them.

## M4. Nerf and Buff

Players read **Nerf** for Degrade and **Buff** for Upgrade. The engine's identifiers stay (`degrade`,
`upgrade`, `tune_once`, the events `degraded` and `upgraded`, the `TuneDirection`), as [[R373]] kept
`library` and `Sacrifice`. "Buff a card" is one Upgrade application (one draw, [[R386]]); "buff it
twice" is two; "+2/+2" is a plain stat buff and stays one. Every Meditative text is written with Nerf and
Buff, and the designer's "degrade" in #45, #49.1, #49.3 and #95 is written Nerf. The rename of the shipped
cards' texts (C+ #8, #69, #70, #71, #72, #73, T-AI-10, Core #98's Radiant face), [[§6.3]]'s rows, the
glossary and every client label is its own part (MN01), which ships when it lands.

## M5. The systems

The groups named each mechanic the set lacks. One part builds each system (the owner below), with its
engine tests through fixture scripts (`crates/engine/tests/rules/fixtures/`), its [[§6]] rows and its
rulings; a part that needs another part's system waits for it ("Blocked by"). Each group's own Systems
list (in M6, after its cards) says more.

| System | What it is | Cards | Built by |
| --- | --- | --- | --- |
| R1420–R1424 | the release gate, Prime pools, `anyTags`, `give_control`, `TRIBAL_TAGS` | all; #6, #30, #31, #38, #55, #60, #95 | the foundation (MF) |
| discard guard | "you cannot be forced to discard on your opponent's turn" | #1 | MB02 |
| computed Echo X | Echo from max mana | #5 | MB02 |
| ME-TRIG | start/end-of-turn and Cry/Death trigger multipliers; "trigger your end of turn effects" | #9, #10, #12 (#11, #13) | MB03 |
| ME-TURN | lost refreshes ("lose all mana next N turns"), extra turns and the once-a-game Rift flag; the zeroed refresh aura (#26); a repayment schedule (#89's credit line) | #18, #19, #19.1; #26; #89; #22 | MB04 (the aura in MB09, the schedule in MB23) |
| ME-WIN | an effect that wins; chosen alternative win conditions held by the state check | #8, #20 | MB04 |
| ME-SECRET | a hidden choice kept until a later turn, the opponent's prediction | #22, #22.1 | MB05 |
| ME-CRAFT | the card crafter: a block editor over the engine's real keywords, hooks and verbs, priced in mana and lines of code, building a transient definition as Fuse does | #17 | MB06 |
| unlock_random_zone, highest_permanent_cost | small verbs | #27, #30 | MB07 |
| ME-CN | every card's Chinese name and text (a sidecar), the `chinese` instance flag, `translate`, the client's rendering | #32, #33, #35, #37 | MS01 (table and flag), MB08 (the cards' riders) |
| granted tags, `enters_hand`, transform riders | `tags_of`; a hand-arrival hook; Transform options | #34, #35, #37 | MB08 |
| ME-TRIBAL immunity, ME-CREATED, turn watcher, aimed cast | "immune to tribal tag based hate"; a `created` flag on every card made after the deal; Showdown's watcher and cast | #25, #31, #98 | MB09 |
| ME-JADE, ME-ALLURE, weighted roll | the Jade Counter, Jade Beauty at 5 and Radiant at 10; Allure; a general weighted roll | #39–#39.5, #41, #43, #44 | MB10 |
| ME-ELEMENT, ME-LUCK | Feng Shui's five elements and their cycles; a player-wide Lucky | #40 | MB11 |
| ME-MARKET | the night market's shop prompt, yuan and barter | #42 | MB12 |
| ME-PICK-N, after-attacked, hero immune, deck bottom, random fuse | small verbs and hooks | #46–#49.3 | MB13 |
| ME-ALTPLAY | a Unit played as an Animated Field Trap; a Spell played as a timed Trap | #45, #45.1, #99 | MB14 |
| R383 revised, armor_on_field | "Animated on your turn" not sick once it has been yours since the turn began (D7) | #66, #73 (#71.1) | MB15 |
| ME-MAGNETIC, query `stats`, random fills, voice trigger hook | Magnetic; printed-stats queries; small verbs | #51, #57, #60, #63, #65, #67, #75 | MB16 |
| ME-STATS, De-Radiant, either-player Activate, ME-GRANT, `manaSpent` | a compiled-in win-rate table; un-Radiant; Activate by either player; a granted Death ability | #50, #53, #54, #58, #59, #69 | MB17 |
| attack mods and `conditionTargets`, attack redirect, exile on damage, ME-PARETO, ME-EMOTE | combat-only auras and the yellow highlight; the attack half of Redirect; the engine-side judge of an optimal play; emotes as engine actions | #52, #64, #71, #71.1, #72 | MB18 |
| ME-HANDCAP, hand marks | a per-player hand size; marked hand cards | #76, #79 | MB19 |
| ME-TUNEMULT, tribal deck trigger | "Buffs and Nerfs are twice as effective"; Tatches' summon from deck | #84, #87 | MB20 |
| ME-RANDOMTARGETS, attack summons, lethal guard | "all targets are random"; Windfast's and Windfurious Prime's attacks; Unan's redirect of fatal damage | #86, #91, #91.1, #92 | MB22 |
| Untributable, tribute pool | a keyword; a pool of cards with a Tribute cost | #80, #88 | MB23 |
| Chaos (Meditative Edition), shuffle memory | the third Call to Chaos table; memory on a shuffled card | #95, #96, #96.1 | MB24 |
| stack base, wide tribute, lane strike, capture | the Blueprint buildings' mechanics | #97.1, #97.4, #97.5, #97.8 | MB26 |
| luck-based pool, given Lucky, lucky coins | the Luck-based pool (MD-G1); Lucky given to a card, read by every pool card's roll (MD-G2); coin flips that keep heads with Lucky (MD-G4) | #101, #102 (Core #4, C #65 and the other pool cards) | MB27 |

**Decisions across groups** (these win over any entry in M6):

- **D1 Tribes.** `TRIBAL_TAGS` = Human, Felinor, KY, CN, Jlockeed ([[R1424]]). "All Tribes" is all five
  (#87's tags list them; the frame prints "All Tribes" for any card with every tribal tag). Group B's
  four-tag list is superseded.
- **D2 R275's stat half** holds with no exception (M2).
- **D3–D6** Prime pools hold the Prime tokens ([[R1421]]); `anyTags` ([[R1422]]); `give_control`
  ([[R1423]]); `TRIBAL_TAGS` ([[R1424]]): built by the foundation, so no batch waits on another for them.
- **D7 R383 revised** (MD-D23): an "Animated on your turn" card that has been on its controller's side
  (either zone) since that turn began is not summoning sick when it animates. MB15 builds it.
- **D8 Names** use straight apostrophes, as the catalog does.
- **D9 Rulings**: each MD becomes an R-number from its part's block (M10).
- **D10** The ⚠ proposals the groups made are the build's defaults (M9).
- **D11** The five rarities above.
- **D12** The [[§8.8]], [[§7]], BUILD M10 and `docs/radiant-audit.md` rows are written by the
  foundation from this brief; a batch corrects its own rows when its build decides otherwise, and says so.
- **D13** `crates/cards/chinese.json` is built by MS01 for every entry the catalog then holds; every part
  after it adds its cards' Chinese entries with them.
- **D14 New state stays out of shipped games' hashes.** A field the set adds to the state, an instance or
  a view is optional and skipped when absent (as `berserk` is), so a game that never uses it serializes,
  hashes and replays as before and no golden trace moves. Where a shipped game must change (ME-CREATED's
  flag on The Coin, MN05's Armor field on the damage event), the part says so and re-blesses the golden
  traces after merging `main`, never by hand-merging `games.jsonl`.

## M6. Cards, card by card

Six drafting groups read the designer's list, one range each, and Group G the three cards the designer
added after it; their entries follow in number order, each
token after its card. Every entry gives the card's id, cost, type, tags, rarity and stats (base → Radiant),
then **Text** and **Radiant** (the faces as they go in `catalog.json`), **Engine** (how the script builds
it, with the verbs and hooks), **Rulings** (the MD proposals it relies on, M7), **Numbers** (its `params`),
and **Check** (each ambiguity in the designer's words and the reading chosen; ⚠ designer marks one M9
lists). Each group's Systems list, after its cards, details the mechanics M5's table names. Where a
decision in M5 (D1–D14) differs from an entry, the decision wins.

### Group A: Meditative #1 to #24, with #19.1 and #22.1

Entries in number order, each token after its card. "§" is SPEC (`spec/NN-*.md`), "R" a ruling note,
"C #N" / "C+ #N" Classic / Classic+ card N, "Core #N" a Core card, "#N" a Meditative card. Engine
names are the Rust ones (`crates/engine/src/effects/*.rs` verbs, `script.rs` hooks). Measured from
`crates/cards/catalog.json` for the crafter's calibration: a keyword-only card's `loc` is 3 (Core #8 Mr.
Vanilla, C #41 State of the Game), Core #5 Stockpile's is 7, the median card's 13.5, the 75th
percentile's 24.

---

#### Meditative #1 · Disruptive Disruptor
`meditative-001` · (2) Field Spell · Common

> **Designer:** You cannot be forced to discard cards on your opponents turn ~~~ You cannot be forced
> to discard cards on your opponents turn · Cry: Draw 1 and gain 2 mana

- **Text:** Aura: You can't be forced to discard cards during your opponent's turn.
- **Radiant:** Aura: You can't be forced to discard cards during your opponent's turn.
  Cry: Draw {draw|card|cards} and gain {mana} mana.
- **Engine:** `NEW: discard guard` — a pure-read hook beside `draw_limit` (R457), asked of the cards
  acting on the field (a face-up Field Spell here). The effect discards (`discard`, `discard_random`,
  `discard_hand`, all through `move_.rs`'s `discard_card`) skip each card of a guarded player's hand
  while `state.active` is not that player: no card moves, nothing answers it, no rng draw for it (R129).
  A public `discardPrevented { player, count }` reports it (a new event: a BUILD M5-T4 row and a
  `SOUND_CUES` row). Costs go through `discard_from_hand` (R450's targeting cost) and the Activate
  cost path (R384), which the guard never reads. Radiant Cry: `draw` then `gain_mana` (R1: fires
  when played or cast).
- **Rulings:** **MD-A1:** A forced discard is any discard an effect makes from your hand (random,
  of your choice or a whole hand). While it is not your turn the guard stops it outright, and a
  `discardPrevented` reports it. A discard you pay as a price (an Activate's cost, R384, or a
  targeting cost, R450) isn't forced, so it still happens. Temporary (R637) discards at the end of
  your own turn, so the guard never meets it. Several Disruptors do no more than one. C #8 Pickle's
  "they discard" option does nothing for a guarded player, so Pickle doesn't offer it (C #8's
  "only choices that would do something"). The guard stops #2's discard when your Rhino is hit on
  the opponent's turn, and #8's Ascent 3 against you.
- **Numbers:** Radiant draw 1 ↑, mana 2 ↑ (Radiant face only; the base face prints none, R749).
- **Class:** C (NEW: discard guard).

#### Meditative #2 · Rampaging Rhino
`meditative-002` · (2) Unit · Common · 5/9 → 11/20

> **Designer:** 5/9 · When this takes an instance of damage damage, discard a card ~~~ 11/20 · When this
> takes an instance of damage damage, discard a card

- **Text:** Whenever this takes damage, discard {discards|card|cards}.
- **Radiant:** Trample
  Whenever this takes damage, discard {discards|card|cards}.
- **Engine:** a `TriggerDef` on `damage` whose `target_id` is this card, as Core #91 Fed Fauci's
  (`is_hit_on_self`; R63: a hit Armor or a cap absorbed, or a Divine Shield took, emits no `damage`
  and costs nothing), returning `discard_random { count, player: self }` (R682: random by default).
  Every hit of a sweep is its own instance (R59).
- **Rulings:** **MD-A2:** The discard belongs to the Rhino's controller as the hit lands (if #6 gave it
  away, the new controller discards). It is one random card per damage instance, and an empty hand
  discards nothing. On the opponent's turn it counts as a forced discard, so #1 stops it (MD-A1).
- **Numbers:** discards 1 ↓.
- **Check:** "damage damage" typo. The Radiant has only stats (more than double: 11/20 against 10/18)
  and repeats the text, which R275 doesn't accept for a Unit with text. The smallest fix in the
  designer's direction, a bigger rhino, is one added keyword: **Trample**, which fits "Rampaging".
  **⚠ designer** (Armor 1 would also soften the drawback).
- **Class:** B.

#### Meditative #3 · Jlockwork Machine
`meditative-003` · (3) Unit · Rare · 10/10 → 20/20

> **Designer:** 10/10 · When your opponent plays a card exile the top 3 card of your library ~~~ 20/20 ·
> Rush · When your opponent plays a card exile the top 3 card of your library

- **Text:** Whenever your opponent plays a card, exile the top {exiles|card|cards} of your deck.
- **Radiant:** Rush
  Whenever your opponent plays a card, exile the top {exiles|card|cards} of your deck.
- **Engine:** a `TriggerDef` on `cardPlayed` with `player` the opponent (§10.5 step 4: casts are plays,
  R70; a countered card was never played, §6.3 Counter; a Trap set face-down is a play too). It runs
  C+ #12.6 Frozen Wastes' pattern: `for_each_card` over `zone_cards(…, Library)` (the top is index
  0), `.take(n)`, then `exile` each. With fewer cards left it exiles what is there. An empty deck
  exiles nothing and causes no fatigue (§6.3 Exile is no draw).
- **Rulings:** it answers each play once, at step 4, before the played card resolves.
- **Numbers:** exiles 3 ↓.
- **Check:** "3 card" typo. The tag stays as written: none. Its name puns on Jlockeed, but it isn't
  "Jlockeed ___", so the Common family doesn't apply. Rush meets R275.
- **Class:** B.

#### Meditative #4 · Juicy Kumquat Melon
`meditative-004` · (4) Spell · Epic

> **Designer:** Draw a (0), (1), (2), (3), (4), and (5) cost Meditative card ~~~ Draw a (0), (1), (2),
> (3), (4), and (5) cost Meditative card · They cost (2) less

- **Text:** Draw a (0), (1), (2), (3), (4) and (5) Cost Meditative card.
- **Radiant:** Draw a (0), (1), (2), (3), (4) and (5) Cost Meditative card. They cost ({discount}) less.
- **Engine:** six draws in cost order. For each cost N, the script finds the topmost deck card whose
  definition's `set` is Meditative and whose cost is N, using `effective_cost` the way
  `matches_library_filter` reads it (R65: an X card counts as 0, an embiggen card at its base price,
  `costMod` included). It then draws that card with `draw_from_library { instanceId }`, a real §2.4
  draw: cast on draw, R58's chain, the hand cap and R457's limit all apply. If no card matches, that
  cost draws nothing. Radiant: `set_cost_mod { amount: −discount, inHandOnly }` on each drawn card
  that reached the hand (R4).
- **Rulings:** **MD-A3:** "Draw a [kind of] card from your deck" takes the topmost matching card,
  as Recruit's top-down scan does (§6.3), and draws no rng. The deck is shuffled, so that is still
  a uniform pick. No match draws nothing and causes no fatigue. #14 reads the same way.
- **Numbers:** Radiant discount 2 ↑.
- **Check:** no Fruit tag, as written, despite the name.
- **Class:** B.

#### Meditative #5 · Death by 1000 cuts
`meditative-005` · (1) Spell · Rare

> **Designer:** Deal 1 damage to a unit, echo (2 * [current max mana]{ie turn 1=1, turn 4=4, turn 4< =
> 4} ~~~ Deal 1 damage to a unit, echo (4 * [current max mana]{ie turn 1=1, turn 4=4, turn 4< = 4} ·
> Lifesteal

- **Text:** Echo X
  Deal {damage} damage to a Unit. X is {echo} times your max mana.
- **Radiant:** Echo X, Lifesteal
  Deal {damage} damage to a Unit. X is {echo} times your max mana.
- **Engine:** `NEW: computed Echo X`. Today `StaticFlags.echo` is a fixed number. Add a pure-read hook
  `Script.echo_x(HookArgs) -> i32`, which `echo::printed_echo` reads at §10.5 step 4 as
  `queue_echo_repeats` queues the repeats. The script uses `param(echo) × max_mana_of(controller)`
  (§2.3: min(turns started, 4) plus persistent modifiers), and `tuned_echo` then adds the card's
  "Echo" tuning step (R386). Twinspell's grant adds its +1 (R30). The declared target is any Unit
  on either side (R81). Each repeat asks a fresh target prompt (§6.2 Echo, §10.6), and a repeat
  with no Unit to hit fizzles. Lifesteal is printed on the Radiant face, as on Core #93.1
  Combo-Fodder, and heals once per hit (§4.4 step 8).
- **Rulings:** **MD-A4:** X is fixed when the play is made (§10.5 step 4), from the caster's max mana
  at that moment. A random cast uses its caster's max mana. Mana gained during the resolution
  doesn't change X.
- **Numbers:** damage 1 ↑; echo 2 ↑ (Radiant 4).
- **Check:** the designer's brace note means max mana (1 on turn 1, 4 from turn 4). At 4 max mana
  that is 9 hits (17 on the Radiant), each its own target prompt. R275 is met: twice the repeats
  plus Lifesteal.
- **Class:** C (NEW: computed Echo X).

#### Meditative #6 · Me no Likey
`meditative-006` · (0) Spell · Epic

> **Designer:** Give a unit you control to an enemy; Gain mana equal to its cost ~~~ Give a unit you
> control to an enemy; Gain mana equal to twice its cost

- **Text:** Give one of your Units to your opponent. If you do, gain mana equal to its cost.
- **Radiant:** Give one of your Units to your opponent. If you do, gain mana equal to {multiple} times
  its cost.
- **Engine:** `NEW: give control`. `steal` gains a `to: PlayerSpec` (default "self"), so
  `take_control` can move the card to the opponent's side. Placement follows R15 (the same lane, else
  their first free zone). It is an entry for them (R171: summoning sick, fresh exertion) and emits
  `controlChanged`. The declared target is a Unit you control, the top of its pile (R13, R81). Its
  cost is read before the move: `cost_now` (R65, as it stands on the field; an X Unit costs its X,
  R396; a token its printed cost). The mana is `gain_mana`, temporary mana that may exceed 4.
- **Rulings:** **MD-A5:** If the opponent has no free unit zone, the Unit stays and you gain nothing,
  because the mana is the price of the gift. The owner doesn't change (§3.2), so a given Unit that
  dies goes to your graveyard.
- **Numbers:** Radiant multiple 2 ↑ (the base face prints none, R749).
- **Check:** with Radiant at (0) and a 4-cost Unit, that's +8 mana: an engine for #8's Ascent 10.
  Giving away #2 makes the opponent discard whenever it is hit.
- **Class:** C (NEW: give control, a small extension of `steal`).

#### Meditative #7 · Introspection
`meditative-007` · (4) Field Spell · Legendary

> **Designer:** End of turn: Heal all allies 3, draw 1, reduce the cost of cards in your hand by (1),
> deal 2 damage to all enemies ~~~ End of turn: Heal all allies 8, draw 1, reduce the cost of cards in
> your hand by (2), deal 4 damage to all enemies

- **Text:** End of turn: Heal your hero and each of your Units {heal}. Draw {draw|card|cards}. Cards in
  your hand cost ({discount}) less. Deal {damage} damage to each enemy.
- **Radiant:** the same text, with heal 8, discount 2 and damage 4.
- **Engine:** an `end_of_turn` hook returning, in the written order: `heal` on `selfHero`, then each
  of your Units (C+ #19.4 Support Loser's list); `draw`; `for_each_card` over your hand with
  `set_cost_mod { amount: −discount }`; and `damage_all { side: enemy, heroes: true }`, where every
  hit lands before the state check (R59). The draw comes before the discount, so the drawn card is
  cheaper too. The discount is a `costMod` (R78: kept until a graveyard takes it off, R766), and it
  never reaches an X card (R65).
- **Numbers:** heal 3 ↑ (Radiant 8, step 2); draw 1 ↑; discount 1 ↑ (Radiant 2); damage 2 ↑ (Radiant 4).
- **Check:** the Radiant keeps draw 1, and every other number is 2 to 2.7 times the base, so R275 is met.
- **Class:** B.

#### Meditative #8 · Reach the Summit
`meditative-008` · (0) Spell, Quickdraw, Wincon · Mythic

> **Designer:** Current ascent level: 0 · End of turn: Return this to your hand with cost equal to its
> ascent level · Gain the effects of your ascent level: · Ascent 0: heal 2 to your hero · Ascent 1: deal
> 2 damage · Ascent 2: draw 2 cards · Ascent 3: your opponent discards 2 cards randomly · Ascent 4: exile
> two random enemy Permian’s · Ascent 5: add two random Radiant cards to your hand, they cost (0) ·
> Ascent 6: reach for the stars · Ascent 7: and when they’re near… · Ascent 8: push even harder · Ascent
> 9: and you might just · Ascent 10: win the game ~~~ Current ascent level: 0 · End of turn: Return this
> to your hand with cost equal to its [ascent level - 1] · Gain the effects of your ascent level: ·
> Ascent 0: heal 3 to your hero · Ascent 1: deal 3 damage · Ascent 2: draw 3 cards · Ascent 3: your
> opponent discards 3 cards randomly · Ascent 4: exile three random enemy Permian’s · Ascent 5: add three
> random Radiant cards to your hand, they cost (0) · Ascent 6: reach for the stars · Ascent 7: and when
> they’re near… · Ascent 8: push even harder · Ascent 9: and you might just · Ascent 10: win the game

- **Type (inferred): Spell.** It prints no stats. "End of turn: Return this to your hand" is §5.1's
  return Spell (Core #23 Reoccurring Dream, Core #31 KY's Math Equation). Under R746 a Spell that
  comes back after it resolves says "Return … to hand", while a permanent's return would print
  "Bounce" (R692). Played as a Spell, it resolves its level and waits in the graveyard to come back.
- **Text:** Ascent level: starts at 0. This costs (Ascent level).
  Gain the effect of your Ascent level. Then your Ascent level goes up by 1.
  End of turn: Return this to your hand.
  Ascent 0: Heal your hero {heal}.
  Ascent 1: Deal {damage} damage.
  Ascent 2: Draw {draw} cards.
  Ascent 3: Your opponent discards {discards} random cards.
  Ascent 4: Exile {exiles} random enemy permanents.
  Ascent 5: Add {adds} random Radiant cards to your hand. They cost (0).
  Ascent 6: Reach for the stars.
  Ascent 7: And when they're near…
  Ascent 8: Push even harder.
  Ascent 9: And you might just…
  Ascent 10: Win the game.
- **Radiant:** the same text with "This costs (Ascent level − 1)." and heal, damage, draw, discards,
  exiles and adds at 3.
- **Engine:**
  - **The level** is R429's play count. `StaticFlags.counts_plays` makes `timesPlayed` count each play
    at §10.5 step 4, casts included and countered plays never, and the count rides the card through
    every zone. The level is `times_played_of(self) − 1` while it resolves and `times_played_of(self)`
    elsewhere. A `preview` (R280) labelled "Ascent level" shows it in play.
  - **The cost** is a `cost` hook (R55's computed cost). The base face costs the level, the Radiant
    face max(0, level − 1), and then R65 applies its `costMod`, discounts and floors on top.
  - **Resolution** (`cry`), by level:
    - 0: `heal` on `selfHero`.
    - 1: a `choose_target` prompt over any Unit or hero, then `damage`. "Deal N damage" with no
      target named is targeted (§8 Conventions), and only this level targets, so the choice is a
      resolution prompt (§10.6) rather than a declared one (R81).
    - 2: `draw`.
    - 3: `discard_random { player: enemy }`.
    - 4: `exile` on N distinct random enemy permanents (R60): tops of piles, face-down Traps
      included, and the exile shows them.
    - 5: `add_random_from_catalog { count, radiant: true, costOverride: 0 }`. The pool is non-token
      cards of every set (R380), never this card (R387).
    - 6–9: nothing.
    - 10 and above: ME-WIN's win effect.
  - **The return** is Core #31's `end_of_turn` hook: from the graveyard, only on the turn it was
    played (`was_played_this_turn`, R153, R155), a `bounce`. A full hand burns it back (§2.4).
- **Rulings:**
  - **MD-A6:** The Ascent level is the number of times this instance has been played before
    (R429). A copy starts at 0 (R57 resets the count). An Echo repeat isn't a play, so it resolves
    the same level again. Made Radiant mid-climb, the card keeps its level.
  - **MD-A7:** "Gain the effects of your ascent level" is the one line for the current level, not
    every line up to it.
  - **MD-A8:** "Return this to your hand with cost equal to its ascent level" is the cost hook,
    level (base) or max(0, level − 1) (Radiant). Discounts apply on top, as to any card (R65).
  - **MD-A9:** Ascent 1's target is a resolution prompt, and a random cast picks it at random
    (§6.3 Cast).
  - Lines 6–9 are the climb and do nothing.
  - Ascent 10 wins as ME-WIN's effect does (MD-A23).
  - #12 Fear Mongerer triggers the return mid-turn, so the card can climb twice in one turn.
  - #10 Double Counting doesn't repeat it, since a Spell isn't a Cry (MD-A12).
- **Numbers:** heal 2 ↑, damage 2 ↑, draw 2 ↑, discards 2 ↑, exiles 2 ↑, adds 2 ↑ (Radiant 3 each). The
  levels and their costs aren't params; Ascent 10 is the frame.
- **Check:** "Permian's" is "permanents". The Radiant's "[ascent level − 1]" floors at 0. Under R275
  each effect is 1.5×, and the whole climb is one mana cheaper at every step (Ascent 10 costs (9)):
  the rider makes up the rest. Ascent 10 costs (10) on the base face, beyond max mana, so it needs
  temporary mana (#6, #22's Greed, #19.1 Radiant, The Coin, Core #6 Mana Well), as Core #29's cost
  of 6 does. Mythic fits: it sets a rule for the whole match.
- **Class:** C (ME-WIN; its level, cost and return are existing pieces).

#### Meditative #9 · Joint Filing
`meditative-009` · (2) Field Spell · Epic

> **Designer:** Your start and end of turn effect trigger an additional time ~~~ Your start and end of
> turn effects trigger two additional times

- **Text:** Aura: Your Start of turn and End of turn effects trigger {extra|additional time|additional
  times}.
- **Radiant:** the same, extra 2.
- **Engine:** ME-TRIG (turn-hook multiplier). `triggers::queue_hooks_in_trigger_order` queues every
  `StartOfTurn` and `EndOfTurn` entry (§2.2's trigger steps, R62, R68), and when the queue's player
  has an acting Joint Filing it queues each entry 1 + N times in a row. Each copy is its own
  `QueuedTrigger`, and `run_queued_trigger` re-reads it as it pops: a card no longer in a zone that
  registers the hook fizzles (R153). There is a state check between copies (R59), and each copy gets
  fresh rng and prompts. This reaches field cards, the return-flagged Spells in your graveyard (R153,
  R155: #8, #21, Core #23, #31) and "Animated on your turn" units. It doesn't reach
  `start_of_opponent_turn` (C #62), delayed effects, or the end-of-turn trap window (R62).
- **Rulings:**
  - **MD-A10:** Turn-hook multipliers don't stack. The highest acting one holds, as with
    Hearthstone's Drakkari Enchanter and Brann. The same holds for #10.
  - **MD-A11:** Each extra trigger is its own queue entry right behind the original. It multiplies
    the End of turn effects #12 triggers too.
- **Numbers:** extra 1 ↑ (Radiant 2).
- **Check:** "effect" typo.
- **Class:** C (ME-TRIG).

#### Meditative #10 · Double Counting
`meditative-010` · (2) Field Spell · Epic

> **Designer:** Your cry and death effects trigger an additional time ~~~ Your cry and death effects
> trigger two additional times

- **Text:** Aura: Your Cry and Death effects trigger {extra|additional time|additional times}.
- **Radiant:** the same, extra 2.
- **Engine:** ME-TRIG (Cry and Death multiplier), Hearthstone's Brann Bronzebeard and Baron
  Rivendare in one card.
  - **Cry:** a Cry run at §10.5 step 5 for a card you play or cast (R1, R70), or one you trigger
    with C #54 Rewind (R467), runs N more times. Each run is a whole effect list closed by its own
    state check (R59), parked on `state.work` behind the first (R113), and the prompts inside it
    are asked again.
  - **Death:** the Death hook (§4.5 step 3) of a card that died under your control (R89) runs N
    more times, back to back, on the same snapshot, in the death pass's R68 order.
- **Rulings:**
  - **MD-A12:** A Cry is a permanent's Cry: a Unit's, a Field Spell's or an Animated card's labelled
    "Cry:", or a Field Spell's unlabelled one-time text (R408). A Spell's resolution isn't a Cry,
    even though the engine runs it from the same `cry` hook, so Spells aren't repeated.
  - **MD-A13:** An extra Cry reuses the play's declared targets and modes (Brann). A target that
    has left since fizzles that part (R174). A triggered Cry reuses the answers its prompts got
    (R467).
  - It doesn't stack (MD-A10).
- **Numbers:** extra 1 ↑ (Radiant 2).
- **Check:** with #12 it gives two (four on the Radiant) full rounds of End of turn effects.
- **Class:** C (ME-TRIG).

#### Meditative #11 · Double Header
`meditative-011` · (4) Field Spell · Legendary

> **Designer:** The first card you play each turn adds a (0) cost copy non-Radiant of it to your hand ~~~
> The first card you play each turn adds a (0) cost Radiant copy of it to your hand

- **Text:** The first card you play each turn adds a copy of it to your hand. The copy isn't Radiant and
  costs ({setCost}).
- **Radiant:** The first card you play each turn adds a Radiant copy of it to your hand. It costs
  ({setCost}).
- **Engine:** a `TriggerDef` on your `cardResolved` (§10.5 step 7, as Core #33 Unstable Clone
  Machine's, R17), when the event's instance heads `played_ids_this_turn` (R213: "first" counts the
  player's plays, not a flag). It returns `add_to_hand { defId, radiant: false | true, costOverride }`,
  a fresh copy of the played card's definition (Core #39's form, R71). A fused or crafted card's
  transient definition copies too (R77). `defId` comes from the event, so a card that has since
  ceased to exist (Core #41 Sheepish) still copies.
- **Rulings:** **MD-A24:** Double Header's own play is still the turn's first card, but Double
  Header doesn't answer it (R119), so that turn copies nothing. Casts are plays (R70), so a cast
  can be the first card. A countered card was never played, so the next card is the first. The copy
  is a second card and copies nothing.
- **Numbers:** setCost 0 ↓ (`min` 0).
- **Check:** a copy of #8 starts at Ascent 0 (MD-A6).
- **Class:** B.

#### Meditative #12 · Fear Mongerer
`meditative-012` · (2) Unit, Human · Epic · 6/8 → 12/16

> **Designer:** 6/8 · Cry: trigger your end of turn effects ~~~ 6/8 · Cry: trigger your end of turn
> effects twice

- **Text:** Cry: Trigger your End of turn effects.
- **Radiant:** Cry: Trigger your End of turn effects {times|time|times}.
- **Engine:** ME-TRIG (trigger now). A new verb, `trigger_turn_hooks { hook: EndOfTurn, times }`,
  calls `queue_hooks_in_trigger_order(EndOfTurn, Some(controller))`, the same entries `turn::end_turn`
  queues (field cards and return-flagged graveyard Spells, R153, R155; #9's multiplier included). The
  loop settles them after the Cry's list, in R68 order. `times` rounds run one after another, each
  settled before the next is queued.
- **Rulings:** **MD-A14:** "Your End of turn effects" are the `endOfTurn` hooks §2.2's end-of-turn
  trigger step would queue for you now. The turn doesn't end, and they run again at the real end of
  turn. The trap window and the delayed effects aren't included (R62). A return Spell (#8, Core #31)
  comes back now and can be played again this turn.
- **Numbers:** Radiant times 2 ↑ (base face untuned, R749).
- **Check:** the Radiant face's 6/8 equals the base's, below R275's double. The smallest fix is
  **12/16**. **⚠ designer.** It is the engine of the #9 to #13 combo (with #13 Gatling Pea).
- **Class:** C (ME-TRIG).

#### Meditative #13 · Gatling Pea
`meditative-013` · (2) Unit · Epic · 2/6 → 4/12

> **Designer:** 2/6 · Armor 2 · End of turn: deal 1 damage to your opponent and permanently increase this
> damage by 1 ~~~ 4/12 · Armor 4 · End of turn: deal 2 damage to your opponent and permanently increase
> this damage by 2

- **Text:** Armor 2
  End of turn: Deal {damage} damage to the enemy hero. Then this damage permanently goes up by {growth}.
- **Radiant:** Armor 4
  End of turn: Deal {damage} damage to the enemy hero. Then this damage permanently goes up by {growth}.
- **Engine:** an `end_of_turn` hook: `damage` on the enemy hero, `param(damage)`, as one hit through
  hero Armor and caps. It then runs `set_number { which: params.damage, value: damage + growth }` on
  itself, C+ #41 KY's Constant's verb (R386 `tuning.set`, event `numberChanged`). The text's
  `{damage}` then prints the current value through `fillParams`, with no `preview` needed.
- **Rulings:** **MD-A15:** "Permanently" is the card's own tuning (R386). It stays in every zone and
  through leaving the field (R78), and a copy keeps it (R57). A Nerf or Buff moves the same number.
  The growth comes after the hit, even when Armor took all of it. Every extra trigger (#9, #12)
  grows it.
- **Numbers:** damage 1 ↑ (Radiant 2), with no `max`; growth 1 ↑ (Radiant 2). Armor 2 is a numbered
  keyword.
- **Check:** "your opponent" means the enemy hero.
- **Class:** B.

#### Meditative #14 · Prime Time
`meditative-014` · (2) Spell · Epic

> **Designer:** Draw 3 prime indexed cards from your deck ~~~ Draw all prime indexed cards from your deck

- **Text:** Draw {draws|card|cards} with a prime index from your deck.
- **Radiant:** Draw every card with a prime index from your deck.
- **Engine:** the index is the definition's `index` (§5), read as a whole number. A token's "N.k" or
  T-name, and a transient card's (whose index is its id, `subsystems/fuse.rs`), are never prime. The
  index is printed in the inspect overlay ("#N · set · rarity …", `CardDetail.tsx`). Each draw is
  `draw_from_library { instanceId }`, topmost match first (MD-A3). The Radiant draws every match in
  the deck as it stood when the effect began (§2.4's "draw your whole library" reading). The hand
  cap burns the overflow.
- **Rulings:** **MD-A16:** "Prime indexed" means the card's catalog index is a prime (2, 3, 5 … 101),
  in any set: indices repeat across sets, so Core #2 and Meditative #2 both count. It doesn't mean a
  position in the deck, whose order is hidden.
- **Numbers:** draws 3 ↑ (base face only).
- **Check:** this is a deckbuilding payoff (Epic: "engines and payoffs a deck is built around"). It
  depends on 25 indices under 100.
- **Class:** B.

#### Meditative #15 · Smelly Steven
`meditative-015` · (2) Unit, Human · Common · 5/5 → 10/10

> **Designer:** 5/5 · Cry: your opponents spells cost (1) more next turn ~~~ 10/10 · Cry: your opponents
> spells cost (2) more next turn

- **Text:** Cry: Your opponent's Spells cost ({surcharge}) more during their next turn.
- **Radiant:** the same, surcharge 2.
- **Engine:** `add_cost_rule { player: enemy, rule: { types: ["Spell"], amount }, lasts: theirNextTurn }`,
  T-AI-7 Alignment Tax's shape (E15, R455, `ModifierExpiry::NextTurnOf`). It applies to the Spell
  type only, not Field Spells (C #77 Anti-Magic Monkey's reading), as a price for a play. A cast pays
  nothing (R70).
- **Rulings:** played on your turn, it lasts through their next turn (R458: "your next turn" skips
  the current one), however many extra turns of yours come between.
- **Numbers:** surcharge 1 ↑ (Radiant 2).
- **Check:** "opponents" typo.
- **Class:** A.

#### Meditative #16 · Trenful Trickster
`meditative-016` · (2) Unit · Rare · 1/5 → 2/10

> **Designer:** 1/5 · End of turn: Summon a random Trap ~~~ 2/10 · End of turn: Summon a random Traps

- **Text:** End of turn: Summon {traps|random Trap|random Traps}.
- **Radiant:** End of turn: Summon {traps|random Trap|random Traps}. (traps 2)
- **Engine:** `summon_random { query: { type: [Trap, Field Trap] } }` (Core #67 Zoomerbin Oomen's verb;
  a Field Trap counts as a Trap), drawing non-token cards of every set (R380). Each one goes
  face-down into your leftmost open backrow zone (R64, R688), and the opponent sees only a back and
  its cost (R33, R351). With no open zone nothing is summoned and no rng is drawn (R129). The
  Radiant makes two picks, and they may repeat (R60).
- **Numbers:** traps 1 ↑ (Radiant 2).
- **Check:** "a random Traps" is read as two Traps, the plural being the Radiant's change (R275: the
  stats alone wouldn't do). **⚠ designer** to confirm.
- **Class:** A.

#### Meditative #17 · True Craft a Card
`meditative-017` · (0) Spell · Mythic

> **Designer:** Craft a card · (Physically pull up a scratch esque gui with resources allocation and lines
> of code, should be representative of how the codebase actually is and let them create a custom card)
> ~~~ Craft a Radiant card · (Radiant here is arbitrary, it’s more so it will be about %100-150 stronger
> on average and have the Radiant tag)

- **Text:** Craft a card and add it to your hand.
- **Radiant:** Craft a Radiant card and add it to your hand.
- **Engine (ME-CRAFT).** In this codebase a card is a catalog `CardDef` (type, cost, stats, keywords)
  plus a `Script` of hooks (`cry`, `death`, `start_of_turn`, `end_of_turn`, a Spell's resolution in
  `cry`, a Trap's `TriggerDef`). Each hook returns `Vec<Effect>` built from `jackioh_engine::effects`,
  with its numbers as declared `params` (R386) and its `loc` counted line by line (§5). The crafter
  assembles exactly that, from blocks that *are* those pieces, and it is priced in two resources:
  points (mana's worth) and lines of code.
  - **Prompt sequence.**
    1. The Spell resolves and opens a `number` prompt (C #18's kind) for the crafted card's cost, 0
       to `CRAFT_MAX_COST` (4).
    2. A new `craft` prompt opens: the editor for that budget. Its `options` are `CRAFT_PRESETS` (4)
       complete recipes the engine generates within the budget, one Unit, one Spell, one Field Spell
       and one Trap, filled from a seeded block order by the match rng as the prompt opens. The
       answer is one new `Selection::Craft { recipe }`: a preset, or the player's own recipe.
    3. The engine validates the recipe (R90). If it is invalid, the action is an error, the state is
       unchanged and the prompt stays open. If valid, the engine compiles it, mints the definition
       and adds a fresh instance to your hand at its cost, Radiant on the Radiant face. A full hand
       burns it (§2.4).
  - **Budget.** Points ≤ `CRAFT_POINTS_BASE` (2) + `CRAFT_POINTS_PER_MANA` (5) × cost, so 2, 7, 12,
    17 or 22, which matches the catalog's stats per cost for Units. Lines of code must be at most
    `CRAFT_LOC_BUDGET` (24, the catalog's 75th percentile), counting `CRAFT_LOC_SKELETON` (3, the
    `pub const ID` / `pub fn script()` / `CardScripts` frame that Core #8's `loc` 3 is). There are
    at most `CRAFT_MAX_EFFECTS` (8) effect blocks, and each number runs 1 to `CRAFT_MAX_N` (10).
  - **Blocks** (a data table in `subsystems/craft.rs`; every price a named constant, CLAUDE.md rule 9).
    Lines are what the block becomes in a card file. Keywords and stats are catalog data (0 lines),
    a hook adds `x: Some(hook(|ctx| vec![` … `])),` (2 lines), a verb call is 1 line, and a
    declared target 1 more (`TargetDecl`).

    | Block | For | Points | Lines | Becomes |
    | --- | --- | --- | --- | --- |
    | Type: Unit, Spell, Field Spell, Trap (one) | all | 0 | 0 | `CardDef.type_` |
    | +1 Attack, +1 Health (a Unit starts 0/1) | Unit | 1 each | 0 | face stats |
    | Taunt, Rush, First Strike, Cleave, Trample, Pierce, Lifesteal / Deft / Divine Shield, Reborn / Charge, Poisonous, Windfury / Armor N | Unit (Lifesteal, Pierce: Spell too) | 2 / 1 / 3 / 4 / 2N | 0 | face keywords (R21's pool and Armor) |
    | Echo N | Spell | 4N | 0 | `staticFlags.echo` |
    | Hats: Cry, Death, Start of turn, End of turn, When cast | Unit, Field Spell; Death: Unit; When cast: Spell | 0 (Start and End of turn double the points under them) | 2 | `cry`, `death`, `start_of_turn`, `end_of_turn` |
    | Hat: Reveals when your opponent plays a Unit / plays a Spell / attacks | Trap (one, required) | 0 | 4 | a `TriggerDef` with `when` (Core #41's, #60's shapes) |
    | Deal N damage (a target / the enemy hero / a random enemy / each enemy) | effect | N / N / N / 3N | 2 / 1 / 1 / 1 | `damage`, `damage_all` |
    | Heal N (a target / your hero / your hero and Units) | effect | ⌈N/2⌉ / ⌈N/2⌉ / N | 2 / 1 / 2 | `heal`, `for_each_card` |
    | Draw N / Gain N mana / Your hero gains N Armor | effect | 3N / 3N / N | 1 | `draw`, `gain_mana`, `gain_hero_armor` |
    | Summon a Rush Token / a Felinor Token / a Sheep Token | effect | 5 / 2 / 2 | 1 | `summon` |
    | Give a Unit +N/+N / Give your Units +N/+N / Give a Unit a keyword | effect | 2N / 4N / the keyword's | 2 / 1 / 2 | `buff`, `buff_all_units`, `grant_keyword` |
    | Destroy a Unit / Bounce a Unit | effect | 9 / 4 | 2 | `destroy`, `bounce` |
    | Add a random (type) card / Discover a (type) card | effect | 4 / 5 | 1 | `add_random_from_catalog`, `discover_from_catalog` |
    | Your opponent discards N / Buff a random card of yours / Nerf a random enemy card / Lock a random enemy zone | effect | 3N / 3 / 3 / 2 | 1 | `discard_random`, `upgrade`, `degrade`, `lock_random_zone` |

    The calibration holds. Core #5 Stockpile's "Draw 2. Heal your hero 2." comes to 3 + 2 + 1 + 1
    = 7 lines, its catalog `loc`, and to 7 points, a (1). A Unit's stats cost one point each. A
    Spell needs at least one effect, a Trap needs its reveal hat with an effect under it, a Field
    Spell needs one hat with an effect, and a Unit may be stats alone.
  - **The crafted definition.** It is a transient `CardDef` in `state.transient_defs`, as Fuse's is
    (R77). Its id is `craft:` plus the recipe's canonical form, or a digest of it past
    `FUSED_ID_CAP` with the recipe kept on the def (R179, R468's form), so `scripts::script_of`
    rebuilds the script from the def in any process (§10.1).
    - **Name:** an adjective and a noun from two fixed lists (`CRAFT_NAME_WORDS`), chosen in the
      editor, never free text.
    - **Card data:** set Meditative; `index` = id (never prime, never in a pool); `token: false`;
      rarity Mythic, its maker's (§8: "Tokens take the rarity of the card that makes them");
      no tags; `refs` the tokens it names.
    - **Text and params:** each block's house-style line (keywords first, then Cry, Death, Start
      of turn, End of turn, "Reveals when …:"). Each block number is a declared param `{bN}`, so
      Nerf and Buff work on it (R386).
    - **`loc`:** the lines counted above, so C #48 and C+ #44 and #45 read it.
    - **Radiant face:** every param and stat doubled (`CRAFT_RADIANT_MULTIPLIER` 2), R275 by
      construction. A crafted Unit with no number beyond its stats also gains the cheapest
      palette keyword it lacks (R275's keyword-only rule).
  - **The AI.** `legal_actions` lists the `number` prompt's five costs, then the `craft` prompt's
    presets. It enumerates no free recipe, as `MAX_PROMPT_ANSWERS` already bounds enumeration (R90),
    while the reducer still accepts any valid recipe. `crates/ai` simulates each answer on its
    determinized world (R185) and keeps the best. §10.7's policy and a timeout (R79) pick a preset
    at random.
  - **The client.** The `craft` prompt opens a block editor in Scratch's manner:
    - A palette down the left, in coloured categories: Type and stats, Keywords, Hats, Effects.
    - A canvas where hats take effect blocks that snap beneath them, with steppers for N and a
      target chip where a block targets.
    - Two meters: Points `used/budget` and Lines of code `used/24`.
    - A live card face, which the engine itself builds through WASM (`craft_preview(recipe, budget)`
      returns the def, its validity and the reasons), so the client decides nothing (CLAUDE.md
      rule 7).
    - A Suggestions strip of the presets, and a Craft button enabled only while the engine calls
      the recipe valid.
    - On a phone, the palette is a bottom sheet and blocks are added with a tap.
    - The opponent sees "Your opponent is crafting a card…" (a prompt is open, §10.6), then an
      `addedToHand` naming only the sentinel (R97).
    - The definition reaches their view once a card of it is readable (R243's `match_defs_in`).
- **Rulings:**
  - **MD-A30:** True Craft a Card asks its cost (0 to 4), then a `craft` answer: a preset or any
    recipe the engine validates against the block table, the point budget, the line budget and the
    type's rules. An invalid recipe is refused and the prompt stays open.
  - **MD-A31:** The AI and the timeout answer only with the engine's presets. Players may answer
    with any valid recipe.
  - **MD-A32:** The crafted card is a transient definition. Its id names its recipe; it is not a
    token, it is Mythic, its set is Meditative and its `loc` is the counted lines. Every number is
    a param, and its Radiant face doubles every number and stat. "Craft a Radiant card" makes the
    card Radiant: Radiant is an instance flag, not a tag (§5.2).
- **Numbers:** none on the card. The crafter's prices are config constants.
- **Check:** distinct from Core #99 Craft a Card (R739). At (0), its price is the crafted card's own
  cost. **⚠ designer:** the 75-second turn clock (R79) runs while crafting. The default is that it
  runs, as for C+ #42 KY's Test (R420). A `craft` prompt could take its own `CRAFT_PROMPT_SECONDS`.
- **Class:** D (ME-CRAFT).

#### Meditative #18 · Expedition12
`meditative-018` · (1) Spell, Quickdraw · Epic

> **Designer:** Lose all mana next turn. Choose a card in your hand to become Radiant ~~~ Lose all mana
> next turn. Choose a card in your hand become Radiant. Echo

- **Text:** Lose all mana next turn. Choose a card in your hand. It becomes Radiant.
- **Radiant:** Echo 1
  Lose all mana next turn. Choose a card in your hand. It becomes Radiant.
- **Engine:** ME-TURN (lost refreshes) and a declared hand pick (R81, as C+ #41's): your other
  non-Radiant hand cards. It then runs `set_radiant` (§5.2). The Radiant's Echo repeat reopens the
  pick as a `hand` prompt (§10.6), and its mana loss is the same next turn, so it changes nothing.
- **Rulings:** **MD-A17:** "Lose all mana next N turns" means the refresh of each of your next N
  turns gives 0 mana (§2.3), and the next-turn rider (Hinder's or your own gain) is spent with it.
  Extra turns count among the N. Mana gained during such a turn (Core #6 Mana Well, The Coin, #22's
  Greed) still adds. Overlapping losses keep the latest end. With no other non-Radiant hand card,
  the pick has nothing and the Spell still resolves (§8 Conventions).
- **Numbers:** none (one turn; Echo 1 is a numbered keyword).
- **Check:** "become" typo. "QuickDraw" is the Quickdraw tag (R640).
- **Class:** C (ME-TURN).

#### Meditative #19 · Expedition1234
`meditative-019` · (1) Spell, Quickdraw · Epic

> **Designer:** Lose all mana next three turns. Add a Temporal Rift to your hand ~~~ Lose all mana next
> three turns. Add a Radiant Temporal Rift to your hand

- **Text:** Lose all mana for your next {turns} turns. Add a Temporal Rift to your hand.
- **Radiant:** Lose all mana for your next {turns} turns. Add a Radiant Temporal Rift to your hand.
- **Engine:** ME-TURN (lost refreshes through your next 3 turns), then
  `add_to_hand { defId: "meditative-019-1", radiant }`. `refs`: #19.1.
- **Rulings:** MD-A17. An extra turn from the Rift counts as one of the three (Hearthstone: Overload
  lands on Time Warp's extra turn). The extra turn's value is the attacks, the draw and the
  turn-hook triggers, not mana.
- **Numbers:** turns 3 ↓.
- **Class:** C (ME-TURN).

#### Meditative #19.1 · Temporal Rift
`meditative-019-1` · (2) Spell, Token · Token (printed Epic)

> **Designer:** Gain an Extra Turn · (Players can only gain at most one Extra Turn from the temporal rift
> card) ~~~ Gain an Extra Tura, 2 mana, 2 mana next turn, and draw 2 · (Players can only gain at most one
> Extra Turn from the temporal rift card)

- **Text:** Take an extra turn after this one.
  You can take only one extra turn from Temporal Rift each game.
- **Radiant:** Take an extra turn after this one. Gain {mana} mana and {nextMana} mana next turn. Draw
  {draw} cards.
  You can take only one extra turn from Temporal Rift each game.
- **Engine:** ME-TURN (extra turns). A new verb, `take_extra_turn`, adds 1 to the caster's
  `extraTurns` count unless their `gameLog.riftExtraTurn` is set, then sets the flag (public, on the
  view). `turn::end_of_turn_cleanup_settle` checks the turn cap, then calls `start_turn(player)` again
  in place of `start_turn(opponent_of(player))` while the player who just ended has an extra turn
  owed, and spends one. Radiant: `gain_mana`, `next_turn_mana` (Core #24's rider) and `draw`.
- **Rulings:**
  - **MD-A18:** An extra turn is a whole §2.2 turn. `state.turn` and `turnsStarted` go up (max mana
    grows, and the 60-turn cap counts it, R2), and it has its snapshot, refresh, Brittle tick,
    start-of-turn triggers and draw. Units are no longer summoning sick. The gainer's "next turn"
    effects land on it, while the opponent's "next turn" effects wait for their next turn.
  - **MD-A19:** An extra turn is owed to a player and taken at the end of that player's next turn.
    Gained on your own turn, it follows at once; gained during the opponent's turn (a cast), it
    follows your next turn. Several are taken one by one.
  - **MD-A20:** Each player may take one extra turn from Temporal Rift per game. The flag is set
    when a Rift grants one (either face, a copy or a stolen Rift). A later Rift resolves its other
    effects and grants no turn.
- **Numbers:** Radiant mana 2 ↑, nextMana 2 ↑, draw 2 ↑ (Radiant face only, R749).
- **Check:** "Tura" typo. R275 is met by the riders. The client needs an "Extra turn" banner.
- **Class:** C (ME-TURN).

#### Meditative #20 · Aestheticize the Game
`meditative-020` · (1) Spell, Quickdraw, Wincon · Mythic

> **Designer:** Choose an alternative Wincon: · Have 100 player health · Have 100 cards in your GY · Have
> 100 attack and 100 health combined on your board ~~~ Choose an alternative Wincon: · Have 90 player
> health · Have 90 cards in your GY · Have 90 attack and 90 health combined on your board

- **Text:** Choose one. For the rest of the game you also win while it is true:
  Your hero has {health} or more health.
  Your graveyard holds {graveyard} or more cards.
  Your Units have {board} or more Attack and {board} or more Health in total.
- **Radiant:** the same with 90s, then: Draw a card. (**⚠ designer**, see Check)
- **Engine:** ME-WIN (chosen condition). The three modes are declared (R81). The chosen one installs
  a rest-of-game player effect on the caster, `altWin { condition, threshold }` (E28's `never`
  expiry, R458), public on the caster's side of the view together with its progress (for example
  "Health 47/100"). The state check reads it at its game-end point, where `hero_check` now stands,
  each time the board settles (§4.5).
  - **Health:** the hero's health (§3: no cap).
  - **Graveyard:** `zone_count(graveyard)` of your own graveyard. Unit tokens never arrive there
    (R11). #21 adds one card a turn.
  - **Board:** the sums of attack and of current health (§10.4 layers) over your acting Units: tops
    of piles and animated cards, with dormant cards excluded (R13).
- **Rulings:**
  - **MD-A21:** A chosen condition lasts the rest of the game and can't be removed. Several kept
    conditions win on any one of them, each at its own threshold.
  - **MD-A22:** The board condition needs both totals at or above the threshold.
  - **MD-A23:** At the state check's game-end point, a player whose hero is at 0 or less loses even
    if a win of theirs holds (Magic's "a loss beats a win"). If no hero is at 0, a player meeting a
    win (an alternative condition or #8's Ascent 10) wins (`GameOverReason::AltWin`, `WonByEffect`),
    and two winners draw. A win effect takes hold at the check that closes its effect list (R59).
- **Numbers:** health 100 ↓, graveyard 100 ↓, board 100 ↓ (Radiant 90 each, step 10); Radiant draw 1 ↑.
- **Check:** the designer's Radiant is only 10% easier, which falls short of R275. The smallest fix
  keeps the 90s and adds R275's allowed Spell rider, **Draw a card**. **⚠ designer** (or let the
  Radiant choose two). The Wincon tag is new.
- **Class:** C (ME-WIN).

#### Meditative #21 · API Key Fishing
`meditative-021` · (0, embiggen 1) Spell · Rare

> **Designer:** You have a 2% embiggen 5% chance to steal your opponents hand. · End of turn: Add a API
> Key Fishing to your hand ~~~ You have a 5% embiggen 12% chance to steal your opponents hand. · End of
> turn: Add a API Key Fishing to your hand

- **Text:** {chance}% chance: Steal your opponent's hand.
  Paid (1): {paidChance}% instead.
  End of turn: Add an API Key Fishing to your hand.
- **Radiant:** the same, chance 5 and paidChance 12.
- **Engine:** `cost: { base: 0, embiggen: 1 }` (§2.3; the script reads `ctx.embiggened`). It draws one
  `ctx.rng.chance(p / 100)` at resolution. On success it runs `give_from_hand { from: enemy, cards:
  all }`: the cards become yours (E2/E16, R12), your hand cap burns the overflow into your graveyard,
  and `stolen` is hidden per zone (R97). The `end_of_turn` hook works from the graveyard on the turn
  it was played (the return flag, R153, R155): `add_to_hand { defId: "meditative-021" }`, a fresh
  card, while this one stays in the graveyard.
- **Rulings:** **MD-A25:** The card added is a new, base API Key Fishing, even from the Radiant face (a
  generated card isn't Radiant unless said, §5.2). A named card isn't a pool, so R387 doesn't stop it.
  #9 adds two, and #12 adds one now.
- **Numbers:** chance 2 ↑ (Radiant 5); paidChance 5 ↑ (Radiant 12, step 2).
- **Check:** "a API" is "an API". R275 is met (2.4–2.5×).
- **Class:** B.

#### Meditative #22 · Mind Games
`meditative-022` · (2) Spell · Epic

> **Designer:** Choose one secretly: Greed, Attack, Defend · Gain a reward on your next turn depending on
> what you played: · Greed: gain 2 mana and draw 2 · Attack: deal 8 damage to all enemies · Defend: all
> allies heal 8 and gain 2 armor · Add a Fortify Mind to your opponents hand ~~~ Choose one secretly:
> Greed, Attack, Defend · Gain a reward on your next turn depending on what you played: · Greed: gain 2
> mana and draw 2 · Attack: deal 8 damage to all enemies · Defend: all allies heal 8 and gain 2 armor ·
> Add a Fortify Mind to your opponents hand, it costs (2) · (Players can only gain at most one Extra Turn
> from the temporal rift card)

- **Text:** Choose one in secret: Greed, Attack or Defend. At the start of your next turn, gain its
  reward.
  Greed: Gain {mana} mana and draw {draw} cards.
  Attack: Deal {damage} damage to each enemy.
  Defend: Heal your hero and each of your Units {heal}. They gain +{armor} Armor.
  Add a Fortify Mind to your opponent's hand.
- **Radiant:** the same, ending: Add a Fortify Mind to your opponent's hand. It costs ({fortifyCost}).
- **Engine:** ME-SECRET.
  - **The choice:** the mode is a declared play choice (R81). `cardPlayed` carries no modes, and the
    engine files it as a secret record on the caster, `state.secrets { id, owner, choice, turn }`.
    `view_for` shows the opponent only that a secret is held. The AI's `redact` drops the choice
    (R185, as it drops C+ #42's `ANSWER_KEY`), and `determinize` samples it uniformly.
  - **The reward:** a start-of-turn delayed effect at the caster's next turn (`delay`, R62's delayed
    step, after the refresh), keyed to the secret. It reveals the secret (`secretRevealed`) and
    resolves:
    - Greed: `gain_mana`, `draw`.
    - Attack: `damage_all { side: enemy, heroes: true }`.
    - Defend: `heal` on your hero and Units, `gain_hero_armor`, and `grant_keyword` Armor N on each
      Unit.
  - **Fortify Mind:** `add_to_hand { defId: "meditative-022-1", player: enemy, costOverride }`, carrying
    the secret's id on the instance (ME-SECRET's link).
- **Rulings:**
  - **MD-A26:** The opponent never sees the secret choice in an event, view or log line until it is
    revealed: when its reward resolves, or when a Fortify Mind is judged against it.
  - **MD-A27:** "On your next turn" means at the start of the caster's next turn, an extra turn
    included (MD-A18), before the start-of-turn triggers and the draw.
- **Numbers:** mana 2 ↑, draw 2 ↑, damage 8 ↑ (step 2), heal 8 ↑ (step 2), armor 2 ↑; Radiant
  fortifyCost 2 ↑.
- **Check:** the last line of the designer's Radiant face is Temporal Rift's note, copied by mistake,
  and is dropped. **⚠ designer.** The Radiant changes only Fortify Mind's price, a tangential rider
  (R275). It is a real tax, since the opponent either pays (2) or eats every penalty. The designer's
  word stands. **⚠ designer** if the rewards should double instead.
- **Class:** C (ME-SECRET).

#### Meditative #22.1 · Fortify Mind
`meditative-022-1` · (0) Spell, Token · Token (printed Epic)

> **Designer:** Temporary (discards at the end of your turn) · Choose one: Greed, Attack, Defend · If you
> successfully predict your opponent e.g., Attack when they Greed, Greed when they Defend, Defend when
> they Attack; counter their gain · If you pick the same thing as your opponent, do nothing. · If you
> failed to predict your opponent e.g, Greed when they Attack, Defend when they Greed, Attack when they
> Defend; one of the negative effects happens depending on what you picked · Greed: lose 2 mana next
> turn and discard 2 random cards now · Attack: all allies take 8 damage · Defend: all enemies heal 8 and
> gain 2 armor · If you discard this trigger all of the negative effects happen ~~~ Temporary (discards
> at the end of your turn) · Choose one: Greed, Attack, Defend · If you successfully predict your
> opponent e.g., Attack when they Greed, Greed when they Defend, Defend when they Attack; counter their
> gain · If you pick the same thing as your opponent, do nothing. · If you failed to predict your
> opponent e.g, Greed when they Attack, Defend when they Greed, Attack when they Defend; one of the
> negative effects happens depending on what you picked · Greed: lose 2 mana next turn and discard 2
> random cards now · Attack: all allies take 8 damage · Defend: all enemies heal 8 and gain 2 armor

- **Text:** Temporary
  Choose one: Greed, Attack or Defend, to guess your opponent's Mind Games. If yours beats theirs
  (Attack beats Greed, Greed beats Defend, Defend beats Attack), cancel their reward. If you chose the
  same, nothing happens. If theirs beats yours, take your choice's penalty.
  Greed: You have {mana} less mana next turn. Discard {discards} random cards.
  Attack: Deal {damage} damage to your hero and each of your Units.
  Defend: Heal your opponent's hero and each of their Units {heal}. They gain +{armor} Armor.
  When you discard this, take all three penalties.
- **Radiant:** the same without the last line.
- **Engine:** ME-SECRET (prediction).
  - **Temporary** is printed (R637): discarded at its holder's own turn end.
  - **The guess** is a declared mode (R81), judged at resolution against the linked secret, with a
    public outcome event `predicted { outcome }`, plus `secretRevealed`. A win removes the reward's
    delayed effect.
  - **The penalties:**
    - Greed: `next_turn_mana { amount: −mana }` (Core #21 Hinder's rider, on yourself), then
      `discard_random`.
    - Attack: `damage` on your hero and each Unit, one hit each (R59).
    - Defend: `heal`, `gain_hero_armor` and `grant_keyword` Armor on the opponent's side.
  - **The discard clause** is a `graveyard_triggers` entry (B5 E26) on its own `discarded` (the
    Temporary discard included), running all three penalties.
- **Rulings:**
  - **MD-A28:** Fortify Mind judges the secret of the Mind Games that made it (the link rides the
    instance). Played after that secret has resolved, it does nothing. Its discard clause still
    applies on the base face, whatever the secret's state.
  - **MD-A29:** "Allies" and "enemies" are those of the player who plays it (or whose hand discards
    it), and its penalties fall on that player, even the secret's own owner if they stole it back
    (#21).
- **Numbers:** mana 2 ↓, discards 2 ↓, damage 8 ↓ (step 2), heal 8 ↓ (step 2), armor 2 ↓ (fewer is
  better for its holder).
- **Check:** #1 doesn't save it, because the discard is on its holder's own turn (MD-A1). The
  Radiant drops only the discard clause, a large rider for its holder, so the designer's word stands
  (R275). It is reached through Radiant-making effects such as #18.
- **Class:** C (ME-SECRET).

#### Meditative #23 · Golly Bob Howdy
`meditative-023` · (1) Unit, Human · Common · 3/5 → 6/10

> **Designer:** 3/5 · Cry: Transform adjacent Units into Sheep Tokens ~~~ 6/10 · Cry: Transform adjacent
> Units into Radiant Sheep Tokens

- **Text:** Cry: Transform the Units next to this into Sheep Tokens.
- **Radiant:** Cry: Transform the Units next to this into Radiant Sheep Tokens.
- **Engine:** `for_each_card` over `adjacent_to(self)`: §3.1's lanes N − 1 and N + 1 on your own side
  and row, tops of piles (R13), snapshotted first. Each gets `transform { defId: "core-t-sheep",
  radiant }`: no Cry, position kept (R691), Immutable untouched (R23), and a unit token transformed
  ceases to exist (R11). A Sheep is worth 2 Tributes, a Radiant one 3 (§3.2, §7). `refs`: Sheep Token.
- **Rulings:** adjacency is your own side (§3.1), so this sets up Tributes (#25's Tribute 2). #10 runs
  it again on the Sheep, which changes nothing.
- **Numbers:** none.
- **Class:** B.

#### Meditative #24 · Polymorph
`meditative-024` · (3) Spell · Common

> **Designer:** Transform a Unit into a Sheep Token ~~~ Transform a Unit and adjacent ones into Sheep
> Tokens

- **Text:** Transform a Unit into a Sheep Token.
- **Radiant:** Transform a Unit and the Units next to it into Sheep Tokens.
- **Engine:** a declared target, any Unit on either side (R81), then `transform { defId: "core-t-sheep" }`
  (Core #41 Sheepish's verb). The Radiant reads `adjacent_to(target)` on the target's side before the
  first transform, and the Sheep are base Sheep on both faces. An Immutable target is left alone (R23).
  `refs`: Sheep Token.
- **Numbers:** none.
- **Check:** R275 is met by the broader scope.
- **Class:** B.

---

#### Group A's systems

- **ME-TRIG** (#9, #10, #12; #13 is the combo's payoff, #8 and #21 are reached by it). It has three
  parts.
  - **(a) Turn-hook multiplier:** `queue_hooks_in_trigger_order` queues each `startOfTurn` and
    `endOfTurn` entry of a player under an acting multiplier 1 + N times. Each copy is its own
    queue entry, re-checked as it pops (R153).
  - **(b) Cry and Death multiplier:** a permanent's Cry (never a Spell's resolution) reruns N times
    with the play's own targets and modes, parked on `state.work` (R113). A Death hook reruns N
    times on its snapshot (R89).
  - **(c) `trigger_turn_hooks`:** queues a player's `endOfTurn` hooks now, as the end-of-turn step
    would.
  - Multipliers don't stack; the highest holds.
- **ME-TURN** (#18, #19, #19.1; #15, #22 and #22.1 read its "next turn").
  - **Lost refreshes:** `mana.lostThrough`, a `turnsStarted` index through which the refresh gives 0
    and spends the next-turn rider. Mana gained later in the turn still counts.
  - **Extra turns:** a per-player owed count, spent at the end of that player's turn by starting
    their turn again, so the whole §2.2 turn runs and the turn cap counts it. Next to it is a
    per-player once-a-game Rift flag. Both are public on the view, with an "Extra turn" banner in
    the client.
- **ME-WIN** (#8, #20). A `win_game` verb, and a rest-of-game `altWin { condition, threshold }`
  player effect (R458's kind) for the health, graveyard and board conditions, shown with progress
  on the view. The state check's game-end point decides: a hero at 0 loses first, a held win then
  wins, and two winners draw. It needs new `GameOverReason`s `AltWin` and `WonByEffect` and the new
  tag Wincon.
- **ME-SECRET** (#22, #22.1).
  - **The record:** `state.secrets` holds `{ id, owner, choice }`, made from a declared mode. The
    choice reaches only its owner's view, and the AI's `redact` drops it while `determinize`
    samples it (R185).
  - **The reward:** a start-of-turn delayed effect keyed to the secret.
  - **The prediction:** a linked card's declared mode, judged against the secret (rock-paper-
    scissors), which can cancel the reward's delayed effect.
  - **Events:** `secretChosen` (public that it exists), `secretRevealed` and `predicted`, each
    with BUILD M5-T4 and `SOUND_CUES` rows.
- **ME-CRAFT** (#17). The `subsystems/craft.rs` block table.
  - **Prompts:** a `number` prompt for the cost, then a new `craft` prompt kind whose options are
    seeded presets and whose answer is `Selection::Craft { recipe }`.
  - **Compiler:** a validator and compiler from a recipe to a transient `CardDef` plus a `Script`
    built from the existing effect verbs. The id names the recipe (R179/R468), so scripts rebuild
    from state, as Fuse's do. A WASM `craft_preview` lets the editor show the engine's own verdict.
  - **Client:** the block editor, with points and lines-of-code meters.
- **NEW: discard guard** (#1). A pure-read hook beside `draw_limit` (R457). It stops every effect
  discard from a guarded player's hand while it isn't their turn, reported by `discardPrevented`.
  Costs (R384, R450) bypass it.
- **NEW: computed Echo X** (#5). `Script.echo_x`, a pure read at §10.5 step 4 that `printed_echo`
  adds to (or uses in place of) `StaticFlags.echo`, then applies the "Echo" tuning step (R386).
- **NEW: give control** (#6). `steal`'s `to`, so control can move to the opponent with R15's
  placement and R171's entry.

### Group B: Meditative #25–#38 and #98

Script files are `crates/cards/src/scripts/meditative/cNNN_slug.rs`. A name written in Chinese characters
gets a pinyin slug: `c032_spiritually_zhongguo.rs`, `c033_first_day_of_xuexiao.rs`, `c034_gaokao.rs`.
The others are `c025_blue_eyes_white_felinor`, `c026_alternate_fate`, `c027_clip_farming_lawyer`,
`c028_shade_iris`, `c028_1_ancient_curse`, `c029_forbiddenous_factory`, `c030_fickle_e_kitten`,
`c030_1_love_bomb`, `c031_the_conductor`, `c035_rcta_cn`, `c036_cn_peptides`, `c037_cn_in_a_bottle`,
`c038_h1b_printer` and `c098_showdown`.

#### Meditative #25 · Blue-Eyes White Felinor
`meditative-025` · (1) Unit, Felinor · Rare · 12/9 → 24/18

> **Designer:** 12/9 · Tribute 2 \n Immune to tribal tag based hate. ~~~ 24/18 · Tribute 2 \n Immutable \n
> Immune to tribal tag based hate.

- **Text:** Tribute 2, Immune to tribal tag based hate
- **Radiant:** Tribute 2, Immutable, Immune to tribal tag based hate
- **Engine:** Tribute is `static_flags.tribute: 2`, paid at play time through the play validator (§6.3
  Tribute X). R391 lets the zone the Tribute empties take the card. "Immune to tribal tag based hate"
  is a NEW §6.1 keyword (ME-TRIBAL). It is read through the layers, as `Immune to Spells` is in
  `restrictions.rs` (E35), and it protects the card only while the card acts on the field. Under it, a
  harmful effect whose card filter names a tribal tag in `tags` or `notTags` passes the card by:
  - It is no legal pick for a declaration with `aim: harm` (R656). That covers the play-time pick in
    `play_choices::card_allowed` and every target prompt.
  - Every harmful verb over a board scope or a card scope skips it (`targets::matches_scope` and
    `card_scope::matches_card_scope`, given the verb's aim).

  Helpful tag-filtered effects still reach it. The Radiant face adds §6.1 Immutable, which blocks
  Transform, Vanilla, Fuse-onto, Nerf and Buff (R23, R386). The existing hate it skips is Core #2
  Bigot's base pick (`notTags: [Human]`) and its Radiant `destroy_all`. Core #43 Big Felinor's
  `notTags: [Felinor]` is moot while the card is a Felinor. Meditative #34, #45, #45.1, #52 and #78 are
  covered under ME-TRIBAL below.
- **Rulings:** MD-B1 (what the immunity covers). A Nerf may remove the keyword: it is the card's own
  and not a harmful one (§6.3 Degrade). A Vanilla removes it too. The Radiant face's Immutable stops
  both. Core #92 Felinor Fiender's stat sum and C+ #46's Felinor aura still read the card, because they
  help.
- **Numbers:** none. Tribute 2 is a numbered keyword that R386's X row tunes (less is better, never
  below 1).
- **Check:** R275 passes: the stats double and Immutable is added. R276 passes. The card is
  Yu-Gi-Oh's Blue-Eyes White Dragon, a two-Tribute monster.
- **Class:** C (ME-TRIBAL).

#### Meditative #26 · Alternate Fate
`meditative-026` · (4) Field Spell · Legendary

> **Designer:** Aura: Players don't generate mana naturally. · End of Turn: Summon a Mana Well in ALL open
> backrow. ~~~ Aura: Players don't generate mana naturally. · End of Turn: Summon a Mana Well in ALL open
> backrow. Make yours Radiant.

- **Text:** Aura: Players don't generate mana naturally.
  End of turn: Summon a Mana Well into every empty backrow zone, for the player whose zone it is.
- **Radiant:** Aura: Players don't generate mana naturally.
  End of turn: Summon a Mana Well into every empty backrow zone, for the player whose zone it is. Yours
  are Radiant.
- **Engine:**
  - **The aura** is ME-TURN's zeroed refresh, held by a card. It is a static flag (`noNaturalMana`)
    that `mana::refresh_mana` reads for both players while this card acts on the field (face-up, top
    of its pile). Today `refresh_mana` takes only a `PlayerState`, so it must also be given the state.
    `max_mana_for` still sets max mana (§2.3), but current mana becomes max(0, next-turn rider)
    instead of max + rider.
  - **End of turn** (the controller's, §6.2): summon a Core #6 Mana Well (`core-006`, "Start of turn:
    Gain {mana} mana.", 2 on its Radiant face) into every empty, unlocked backrow zone of both
    players. The active player's side goes first, lane 1 upward (R68). Each Well is owned and
    controlled by the player whose zone it is (`summon`'s `player`). This needs a NEW, small extension
    of `fill_board`: `row: backrow` and a side.
  - The Wells are summoned, so they fire no Cry (R1). The refresh comes before start-of-turn triggers
    (§2.2), so a Well's mana arrives after the empty refresh.
  - On the Radiant face, the Wells on the controller's side are Radiant.
  - The card's `refs` are `[core-006]`.
- **Rulings:** MD-B2 (the zeroed refresh) and MD-B3 ("ALL open backrow"). A summoned Well is a
  Created card (ME-CREATED) and a real Core card: it goes to its owner's graveyard when it leaves.
- **Numbers:** none. A Well's mana is Mana Well's own `mana`.
- **Check:** R275 passes: the Radiant Wells double the controller's income. The base face gives the
  opponent up to five Wells against your four, since this card takes one of your zones. That
  asymmetry is the designer's. The backrow it fills also shuts out Traps on both sides. Legendary
  fits the rubric's "breaks a core rule".
- **Class:** C (ME-TURN).

#### Meditative #27 · Clip-Farming Lawyer
`meditative-027` · (2) Unit, Human · Common · 6/5 → 12/10

> **Designer:** 6/5 · End of Turn: Unlock a random zone. If you do, add 1 Coin to your hand. ~~~ 12/10 ·
> End of Turn: Unlock a random zone. If you do, add 2 Coins to your hand. If there are no locked zones,
> lock a random enemy one.

- **Text:** End of turn: Unlock a random zone. If you do, add {coins|Coin|Coins} to your hand.
- **Radiant:** End of turn: Unlock a random zone. If you do, add {coins|Coin|Coins} to your hand. If
  no zone is Locked, Lock a random enemy zone.
- **Engine:** The end-of-turn hook first reads the Locked zones on both sides and in both rows, with
  `zones::is_locked` over `slots_of` (both in the prelude).
  - If at least one is Locked, it runs NEW `unlock_random_zone(ZoneScope)`. This mirrors
    `lock_random_zone` in `effects/locks.rs`: a uniform rng pick among the Locked zones in scope, with
    the event `unlocked`. It then runs `add_to_hand({ defId: "core-t-coin" })` `coins` times.
  - If none is Locked, the base face does nothing. The Radiant face runs
    `lock_random_zone({ side: enemy })`, which picks among the opponent's unlocked zones in both rows
    (R60).

  The card's `refs` are `[core-t-coin]`.
- **Rulings:** MD-B4.
- **Numbers:** coins 1 → 2 ↑.
- **Check:**
  - The designer listed this card twice, identically; it is one card.
  - "Coin" means The Coin (§7). R279's naming proof (`tests/cross/references.rs`) must accept "Coin"
    and "Coins" as naming it.
  - R275 passes: double the stats, double the Coins and an added rider.
- **Class:** B (plus the small NEW `unlock_random_zone`).

#### Meditative #28 · Shade-iris
`meditative-028` · (2) Unit · Common · 7/4 → 14/8

> **Designer:** 7/4 · Cry: Shuffle 1 Ancient Curse into the enemy Deck. ~~~ 14/8 · Cry: Shuffle 2 Ancient
> Curse into the enemy Deck.

- **Text:** Cry: Shuffle {curses|Ancient Curse|Ancient Curses} into your opponent's deck.
- **Radiant:** Cry: Shuffle {curses|Ancient Curse|Ancient Curses} into your opponent's deck.
- **Engine:** `shuffle_into({ defId: "meditative-028-1", count: param("curses"), player: opponent })`
  (§6.3 Shuffle into, R80's library cap, a public `shuffledIn`), the same shape as Core #90 CN-Viral
  Injection. The curses belong to the deck's owner and are Created cards (ME-CREATED). The card's
  `refs` are `[meditative-028-1]`.
- **Numbers:** curses 1 → 2 ↑.
- **Check:** R275 passes: the stats double and the curses double. R276 passes on stats and the param.
  This is the designer's first of two #28s (Showdown is the other, now #98).
- **Class:** A.

#### Meditative #28.1 · Ancient Curse
`meditative-028-1` · (2) Spell, Token · Token (printed Common)

> **Designer:** Cast on Draw: Take 7 damage. ~~~ Pierce · Cast on Draw: Take 7 damage.

- **Text:** Cast on draw: Take {damage} damage.
- **Radiant:** Pierce
  Cast on draw: Take {damage} damage.
- **Engine:** §6.2 Cast on draw (`static_flags.cast_on_draw`), which is a cast (R40, R70). It deals one
  `damage` instance from this card to its caster's own hero, through §4.4, with Armor applying.
  "Take" names the caster's hero, as §7 reads CN-Virus. The caster is whoever drew it (R549 covers a
  card drawn out of the other deck). The Radiant face prints Pierce: under R346 a Spell's printed
  Pierce skips the Armor step. After the cast the draw repeats (§2.4) under R58's chain cap. Setup
  never deals it (R635, R748).
- **Numbers:** damage 7 → 7 ↓ (step 2). Down is better for its caster, as with CN-Virus's numbers.
- **Check:** R275 passes on the added keyword. Its (2) cost matters only if it reaches a hand uncast
  (R58's cap, R748), and it lets Book of Buff and Book of Nerf move its cost.
- **Class:** A.

#### Meditative #29 · Forbiddenous Factory
`meditative-029` · (3) Field Spell, Plague · Rare

> **Designer:** Cry: Place 3 Plague Counters on this. · End of Turn: Spend a Plague Counter to shuffle 1
> Ancient Curse into the enemy Deck. ~~~ Cry: Place 3 Plague Counters on this. · End of Turn: Spend a
> Plague Counter to shuffle 2 Ancient Curse into the enemy Deck.

- **Text:** Cry: Place {counters|Plague Counter|Plague Counters} on this.
  End of turn: Spend a Plague Counter from this to shuffle {curses|Ancient Curse|Ancient Curses} into
  your opponent's deck.
- **Radiant:** Cry: Place {counters|Plague Counter|Plague Counters} on this.
  End of turn: Spend a Plague Counter from this to shuffle {curses|Ancient Curse|Ancient Curses} into
  your opponent's deck.
- **Engine:**
  - **Cry** (only when played, R1): `place_plague({ amount: counters })` on itself. That is one
    placement of 3 (§6.3 Plague Counter, R689); R471's multiplier applies to it.
  - **End of turn** (the controller's): when `ctx.self_`'s `counters.plague` is at least 1, run
    `consume_plague({ amount: 1 })` and then the same `shuffle_into` as #28. With no counter it does
    nothing.
  - Other Plague cards can refill it: C #53 Plague Crawler, C #61, C #76 and C #42's Activate.
  - The card's `refs` are `[meditative-028-1]`.
- **Rulings:** "Spend … to …" is a price. With no counter to spend, no curse is shuffled.
- **Numbers:** counters 3 → 3 ↑; curses 1 → 2 ↑.
- **Check:** R275 passes: the curses double. The name keeps the designer's spelling, "Forbiddenous".
- **Class:** B.

#### Meditative #30 · Fickle E-Kitten
`meditative-030` · (1) Unit, Felinor · Epic · 3/4 → 6/8

> **Designer:** 3/4 · Start of Turn: If your opponent has a more expensive permanent than you, they gain
> control of this. Otherwise, shuffle a Love Bomb into your deck. ~~~ 6/8 · Start of Turn: If your
> opponent has a more expensive permanent than you and a larger Deck, they gain control of this.
> Otherwise, shuffle a Radiant Love Bomb into your deck.

- **Text:** Start of turn: If your opponent has a more expensive permanent than you, they gain control
  of this. Otherwise, shuffle a Love Bomb into your deck.
- **Radiant:** Start of turn: If your opponent has a more expensive permanent than you and a larger
  deck, they gain control of this. Otherwise, shuffle a Radiant Love Bomb into your deck.
- **Engine:** This triggers at the start of its controller's turn (§2.2, R62's queue).
  - **The condition** compares each side's most expensive permanent with a NEW read helper,
    `highest_permanent_cost(state, player) -> Option<i32>`, in `query.rs`. It covers the top of each
    pile and every backrow card, each at R396's field cost: an X card at the X it was played for
    (0 without one), any other card at `numbers::own_cost`, which returns `None` for an X card. A
    face-down card counts at the cost both players see (R33, R351). The opponent's must be strictly higher. The
    Radiant face also needs `zone_count(opponent, library)` to be greater than yours.
  - **When it holds**, NEW verb `give_control({ target: self, to: opponent })`. This is §6.3 Steal
    toward the other player: `steal.rs`'s `take_control` with the receiver named. It uses R15's
    placement (same lane, else the first free zone) and R171's entry. With no free unit zone the
    card stays.
  - **Otherwise**, `shuffle_into({ defId: "meditative-030-1", count: 1, radiant: ctx.radiant })` into
    its controller's deck.
  - The card's `refs` are `[meditative-030-1]`.
- **Rulings:** MD-B5.
- **Numbers:** none.
- **Check:**
  - R275 passes: double the stats, a stricter condition for losing the card (both parts must hold),
    and a Radiant Love Bomb.
  - Its new controller's start of turn asks the question again from their side. So it settles with
    the player whose board is cheaper and feeds that player Love Bombs.
  - With #31 The Conductor it becomes a healing engine.
- **Class:** B. NEW `give_control` is shared with Meditative #6 Me no Likey ("Give a unit you control
  to an enemy").

#### Meditative #30.1 · Love Bomb
`meditative-030-1` · (1) Spell, Token · Token (printed Epic)

> **Designer:** Cast on Draw: Heal your Hero 7. ~~~ Cast on Draw: Heal your Hero 14.

- **Text:** Cast on draw: Heal your hero {heal}.
- **Radiant:** Cast on draw: Heal your hero {heal}.
- **Engine:** Cast on draw, as #28.1. It runs `heal` of `heal` on its caster's hero (§6.3 Heal). C+ #22
  Blood Moon's would-be-healed replacement applies when that card's controller's enemy casts it.
  #30 and Meditative #67 Sentient Cat Ears make this card.
- **Numbers:** heal 7 → 14 ↑ (step 2).
- **Class:** A.

#### Meditative #31 · The Conductor
`meditative-031` · (3) Unit, Human · Rare · 7/9 → 14/18

> **Designer:** 7/9 · Deft · Cry: Draw all Created cards from your Deck. ~~~ 14/18 · Deft · Cry: Shuffle a
> Prime card into your Deck then draw all Created cards from your Deck.

- **Text:** Deft
  Cry: Draw every Created card in your deck.
- **Radiant:** Deft
  Cry: Shuffle a random Prime card into your deck. Then draw every Created card in your deck.
- **Engine:**
  - **Deft** is the §6.1 keyword.
  - **Created** needs the NEW system ME-CREATED. Nothing in the engine marks a created card today.
    Every instance comes from `state::new_instance`, deck cards and generated cards alike, and only
    R673's Glitch roll knows that a card was generated.
  - **The Cry** reads, as it begins, the ids of the Created cards in its controller's library, top
    down. It returns one `draw_from_library({ instanceId })` per id (`effects/draw.rs`). Each is a
    real §2.4 draw:
    - a `drawn` event and R55's counter;
    - R4's hand cap, which burns the overflow;
    - a cast-on-draw card is cast, and its replacement draw comes from the top of the library;
    - R58's chain cap and R457's draw limit apply.

    An id that has left the library by its turn fizzles.
  - **The Radiant face** first runs `shuffle_random_from_catalog({ query: { tags: [Prime] } })`. That
    needs Prime in `POOL_TOKEN_TAGS` (`config.rs`, which today holds only Fruit), because every Prime
    card is a token: C+ #38.1 Solarius Prime, C+ #46.1 Felinor Flagbearer Prime, and Meditative #45.1
    Knowledge Breaker Prime and #91.1 Windfurious Prime. The card it shuffles in is Created, so the
    same Cry draws it.
  - **Hidden information:** the opponent sees hidden `drawn` events and each public cast-on-draw cast,
    never which cards were drawn (R97). The owner reads their Created cards off their library list
    (R310), which marks them.
- **Rulings:** MD-B6 (Created), MD-B7 (the draw) and MD-B8 (the Prime pool).
- **Numbers:** none.
- **Check:**
  - It draws the opponent's gifts too. Every Ancient Curse and Core #90.1 CN-Virus in your deck is
    cast at once (7 damage per Curse), and every Love Bomb heals.
  - R275 passes: double the stats and a Prime card.
- **Class:** C (ME-CREATED).

#### Meditative #32 · Spiritually 中国
`meditative-032` · (2) Spell, CN · Epic

> **Designer:** Randomly do one of three things: · Get ready to learn Chinese (Translate all cards in your
> and your opponents hand and deck into Chinese) · Convert all cards into your hand into the same CN card
> (they cost 0) · Summon 5 random CN units ~~~ Randomly do one of three things: · Get ready to learn
> Chinese (Translate all cards in your opponents hand and deck into Chinese) · Convert all cards into your
> hand into the same Radiant CN card (they cost 0) · Summon 5 random Radiant CN units · (For the AI
> building this card: This cards text is in Chinese)

- **Text:** One random effect: Get ready to learn Chinese; transform every card in your hand into the
  same random CN card, which cost (0); summon {units} random CN Units.
- **Radiant:** printed in Chinese. This is the catalog's own `radiant.text`, and no English form of it
  exists:
  随机一项效果：准备好学中文；将你手牌中的每张牌变形为同一张随机的光辉中国牌，它们的法力值消耗为(0)；召唤{units}个随机的光辉中国单位。
- **Engine:** The match rng makes one uniform pick of the three effects as the Spell resolves
  (`rng.int(3)`, R60), the single-effect shape of Call to Chaos (`subsystems::call_to_chaos`).
  1. **Get ready to learn Chinese** (ME-CN) sets `chinese` on every card in both hands and both
     libraries. On the Radiant face it reaches only the opponent's hand and library. The change is
     silent (R440). As the designer asked, the printed text never says what it does.
  2. **Transform the hand.** Draw one definition from `query({ tags: [CN] })`, never this card (R387).
     Then replace every card in your hand, in its place, with a new card of that definition (§6.3
     Replace off the field, R31; R671 keeps the hand order). Off the field a Replace is no Transform,
     so an Immutable hand card is replaced too (R35, `transform::transformable`). Each costs (0)
     (`costOverride`) and is Radiant on the Radiant face. This needs NEW `transform_random` over a
     hand scope with one shared draw and cost riders; today the verb takes one card and no price
     rider. Each new card is generated into a hand, so R673's Glitch roll applies to each.
  3. **Summon.** `summon_random({ query: { type: Unit, tags: [CN] } })` `units` times. Repeats are
     allowed (R60); each goes into the leftmost empty, unlocked unit zone (R64). They are Radiant on
     the Radiant face. A summon with no zone draws nothing (R129).
- **Rulings:** MD-B9 (the pick), MD-B10 (the one card for the whole hand) and MD-B13 (a face printed in
  Chinese).
- **Numbers:** units 5 → 5 ↑.
- **Check:**
  - The Radiant face's first effect reaches only the opponent's cards. That is narrower, which is
    better for its caster: they keep reading their own cards.
  - R275 passes on the Radiant cards and Units.
  - "Spiritually Chinese" (精神中国人) is a meme; the name stays as written.
  - The house-style proof (`crates/cards/tests/cross/card_text.rs`, R366, R432) must read a Chinese
    face by Chinese punctuation (MD-B13).
- **Class:** C (ME-CN).

#### Meditative #33 · First Day of 学校
`meditative-033` · (0) Spell, CN · Rare

> **Designer:** Add two random CN cards to your hand; they cost (1) less · (For the AI building this card:
> The cards added to your hand should have their text be in Chinese) ~~~ Add two random Radiant CN cards
> to your hand; they cost (1) less · (For the AI building this card: The cards added to your hand should
> have their text be in Chinese)

- **Text:** Add {cards|random CN card|random CN cards} to your hand. Each costs ({discount}) less.
- **Radiant:** Add {cards|random Radiant CN card|random Radiant CN cards} to your hand. Each costs
  ({discount}) less.
- **Engine:** `add_random_from_catalog({ query: { tags: [CN] }, count: cards, costMod: -discount,
  radiant })`. The pool never holds this card (R387), and R673 applies. It adds the NEW rider
  `chinese: true` (ME-CN), which goes on before the card moves, as the radiant rider does
  (`apply_radiant_rider`), so a card the hand cap burns is Chinese in its graveyard too. As the
  designer asked, the printed text never says Chinese.
- **Numbers:** cards 2 → 2 ↑; discount 1 → 1 ↑.
- **Check:** R275 passes on the Radiant cards, as with Core #57 Conjure KY. 学校 means "school"; the
  name stays as written.
- **Class:** C (ME-CN).

#### Meditative #34 · 高考
`meditative-034` · (4) Spell, CN, KY · Rare

> **Designer:** Destroy all non-CN or KY permanents. Buff all CN & KY permanents twice. ~~~ Destroy all
> enemy non-CN or KY permanents. Buff all friendly CN & KY permanents thrice.

- **Text:** Destroy every permanent that is neither CN nor KY. Buff each CN or KY permanent
  {times|time|times}.
- **Radiant:** Destroy every enemy permanent that is neither CN nor KY. Buff each of your CN or KY
  permanents {times|time|times}.
- **Engine:**
  - **Destroy:** `destroy_all({ side: any, rows: [units, backrow], notTags: [CN, KY] })` (§6.3 Destroy).
    Indestructible cards, cards Immune to Spells (E35) and #25's immunity (ME-TRIBAL) survive it.
  - **Buff:** `upgrade({ scope: { zones: [field], side: any, tags: [CN, KY] }, times })` (B3.4). A
    card scope's `tags` means any of them (`card_scope::matches_card_scope`), so the scope takes
    permanents with either tag.
  - The destroyed cards leave at the state check after the whole list (R59), and the two sets never
    overlap.
  - Tags are read with the tags effects have granted (#35, NEW `tags_of`).
  - On the Radiant face the destroy is `side: enemy` and the Buffs `side: self`.
- **Rulings:** MD-B14.
- **Numbers:** times 2 → 3 ↑.
- **Check:** R275 passes: a narrower destroy that favours the caster and a third Buff. 高考 (gāokǎo) is
  China's university entrance exam: only the CN and KY students pass.
- **Class:** B.

#### Meditative #35 · RCTA (CN)
`meditative-035` · (1) Spell, CN · Common

> **Designer:** Give a non-CN permanent the CN tag. Convert its text to Chinese. Give it +2/+2 and buff it.
> ~~~ Give a non-CN permanent the CN tag. Convert its text to Chinese. Give it +5/+5 and buff it twice.

- **Text:** Give a non-CN permanent the CN tag and +{stats}/+{stats}. Buff it {times|time|times}.
- **Radiant:** Give a non-CN permanent the CN tag and +{stats}/+{stats}. Buff it {times|time|times}.
- **Engine:**
  - **Play-time pick:** a declared target (`TargetDecl` of kind target, `of: [permanent]` in both rows,
    either side, `notTags: [CN]`, `aim: help`, R81).
  - **On resolution:**
    1. NEW `grant_tag` adds CN to a NEW `CardInstance.granted_tags`.
    2. ME-CN sets `chinese` on the card.
    3. `buff` gives +stats/+stats. This is layer 4, stored even on a backrow card, where it applies if
       the card animates.
    4. `upgrade({ target: chosen, times })`.
  - As the designer asked, the printed text never says Chinese.
- **Rulings:** MD-B15 (granted tags), MD-B16 (face-down picks) and MD-B11 (a translation on the field).
  A target that is CN by the time the Spell resolves fails its filter and fizzles (§10.5).
- **Numbers:** stats 2 → 5 ↑; times 1 → 2 ↑.
- **Check:** R275 passes: 2.5 times the stats and twice the Buffs.
- **Class:** C (ME-CN and granted tags).

#### Meditative #36 · CN Peptides
`meditative-036` · (1) Spell, CN · Common

> **Designer:** Choose a unit; it has a 50% chance to be buffed 7 times and a 50% chance to die. ~~~ Choose
> a unit; it has a LUCKY 1 50% chance to be buffed 10 times and a 50% chance to die. · (Luck depending on
> if its an ally or enemy)

- **Text:** Choose a Unit. 50% chance: Buff it {times|time|times}. Otherwise, destroy it.
- **Radiant:** Lucky 1
  Choose a Unit. 50% chance: Buff it {times|time|times}. Otherwise, destroy it.
- **Engine:**
  - **Play-time pick:** a declared Unit on either side (R81).
  - **The roll:** the Cry resolves the target first; with no target it draws nothing (R129, as in
    `coins.rs`). Then it flips one `ctx.rng.coin()`. With the face's Lucky X (§6.1 `rng.lucky`; the
    Spell's printed keyword as Nerf and Buff have moved it, read with `numbered_keywords_on` as
    `effects/fruit.rs`'s `lucky_of` reads a Spell's), it flips X more coins. The comparator keeps the outcome better for the caster: the Buffs on a Unit the caster
    controls, the destroy on an enemy one.
  - **Heads:** `upgrade({ target: chosen, times })`. **Tails:** `destroy`; an Indestructible Unit
    survives (R46).
- **Rulings:** MD-B17.
- **Numbers:** times 7 → 10 ↑ (step 2).
- **Check:**
  - The Radiant face wins for its caster 75% of the time and gives 10 Buffs.
  - An enemy Unit that wins the coin gets those Buffs: that risk is the designer's.
  - "LUCKY 1" is read as the keyword. The parenthetical is the builder's note behind MD-B17 and is not
    printed.
- **Class:** B.

#### Meditative #37 · CN in a bottle
`meditative-037` · (X) Spell, CN · Epic

> **Designer:** When this enters your hand: Replaced with a random Radiant card. Convert its text to
> Chinese. ~~~ When this enters your hand: Replaced with a random Radiant card; it costs(0). Convert its
> text to Chinese.

- **Text:** When this enters your hand: Replace it with a random Radiant card.
- **Radiant:** When this enters your hand: Replace it with a random Radiant card. It costs (0).
- **Engine:**
  - **The hook:** NEW Script hook `enters_hand`, run by `draw::run_arrival_hooks` on a hand arrival,
    beside R151's start-of-game clause. Every way into a hand passes through `draw::add_to_hand`:
    - a draw, an add or a Discover;
    - a bounce, a return, a gift, or a steal off the field;
    - the opening deal and the mulligan's replacements.
  - **The replacement:** the hook is `transform_random` of the card itself in the hand (§6.3 Replace,
    R31; R671 keeps its place). The pool is the non-token cards of every open set except this one
    (R380, R387). The new card is Radiant and carries NEW riders: `chinese` (ME-CN) and, on the
    Radiant face, `costOverride: 0`.
  - **What follows:** the replacement is a card generated into a hand, so R673's Glitch roll applies.
    It runs its own arrival hooks (R151), so a Heroic Power rolls its power.
  - **Hidden information:** only the owner sees any of it. The opponent sees a hidden card arrive (R97).
- **Rulings:** MD-B18 (the arrival) and MD-B19 (the X).
- **Numbers:** none.
- **Check:**
  - R275 passes: the Radiant face makes the new card cost (0).
  - The (X) is printed only. A full hand burns the card before it enters (§2.4, R4), so it reaches
    the graveyard unreplaced.
  - The typo "costs(0)" is fixed.
- **Class:** C (ME-CN and the NEW `enters_hand` hook).

#### Meditative #38 · H1B Printer
`meditative-038` · (2) Field Spell, CN, KY · Rare

> **Designer:** Start of turn: Add a random Radiant KY or CN card to your hand ~~~ Start of turn: Add a
> random Radiant KY or CN card to your hand; it costs (0)

- **Text:** Start of turn: Add a random Radiant KY or CN card to your hand.
- **Radiant:** Start of turn: Add a random Radiant KY or CN card to your hand. It costs (0).
- **Engine:** At the start of its controller's turn it runs `add_random_from_catalog({ query: { anyTags:
  [KY, CN] }, radiant: true })`, with `costOverride: 0` on the Radiant face. `anyTags` is a NEW
  `CatalogQueryArgs` field: a query's `tags` asks for all of them (`catalog::matches_query`). The
  Printer itself is excluded (R387), and R673 applies.
- **Rulings:** MD-B20.
- **Numbers:** none.
- **Check:**
  - R275 passes: the Radiant face makes the card cost (0).
  - There is no Chinese rider. The designer's remark covers the cards whose text says they convert
    (#32, #33, #35, #37). **⚠ designer:** confirm whether this card's cards should arrive in Chinese.
- **Class:** B (plus `anyTags`).

#### Meditative #98 · Showdown
`meditative-098` · (2) Spell · Rare (assigned)

> **Designer:** Choose a lane. Lock all other lanes until the start of your next turn. ~~~ Choose a lane.
> Lock all other lanes until the start of your next turn. Cast Book of Buff on any cards you place in it
> this turn.

- **Text:** Choose a lane. Lock every other lane until the start of your next turn.
- **Radiant:** Choose a lane. Lock every other lane until the start of your next turn. This turn, after
  you put a card into that lane, cast Book of Buff on it.
- **Engine:**
  - **The lane** is a play-time choice: five `modes`, "lane-1" to "lane-5" (R81). A random cast picks
    one at random.
  - **The Lock:** the hook locks every zone of the four other lanes, both sides and both rows, that is
    not already Locked (`lock` per zone). Under §6.3 Lock and R688 this refuses plays only; summons
    still land. It then runs `delay` (`at`: the start of the caster's next turn, R62's delayed stage)
    with an `unlock` of exactly those zones, carried in the delay's `data`, each only if still Locked.
    All of this exists today.
  - **The Radiant face** needs two NEW pieces:
    - A turn watcher: a player modifier (`ModifierKind`, expiry `thisTurn`). It answers the caster's
      `cardResolved` for a permanent played into one of their two zones in that lane, and their
      `summoned` there.
    - An aimed cast: `cast_new({ def: "classicplus-071" })` with a NEW `CastHow` option that names the
      cast card's target, since today a cast takes no preset pick. Each cast is Book of Buff's base
      face (Buff 5 times) under R70, so it counts as a play.
  - The card's `refs` are `[classicplus-071]`.
- **Rulings:** MD-B21 and MD-B22.
- **Numbers:** none.
- **Rarity:** the designer gave none. Rare by §8's rubric: one idea with a twist (a Lock for one turn
  cycle on both players, plus a choice). Lane locks run Common (C #71 Lane Eater), Rare (C #84
  Lockdown) and Epic (C+ #34 Memory Leak).
- **Check:** This was the designer's first #28, renumbered #98 by the set decision. R275 passes on the
  added rider.
- **Class:** C (the turn watcher and the aimed cast); the base face alone is B.

---

#### Group B's systems

- **ME-CN: Chinese text** (#32, #33, #35, #37; any later card that converts text).
  1. **Where the translations live.** A new sidecar, `crates/cards/chinese.json`, follows R660's
     `flavour.json` pattern. It is not card data: an edit is no patch and claims no fragment, and
     `catalog check` and `patches check` never read it.
     - It is keyed by catalog id and covers every card and token of every set. Each entry is
       `{ name, base, radiant }`: the name and both faces' texts in Simplified Chinese.
     - Each text carries exactly the `{key}`s its English face has. An English `{key|singular|plural}`
       becomes `{key}` plus a measure word ("抽{draw}张牌").
     - A card that declares `preview` (R280) also carries `previews: { "<English label>": "<Chinese
       label>" }`.
     - A companion file, `crates/cards/chinese-terms.json`, holds the frame's words: types, tags,
       rarities, keywords, labels ("Cry:" is "战吼："), "Radiant" and "Created", and the glossary
       entries.
     - Both files are bundled into the client as `@jackioh/cards/chinese.json`, the way
       `flavour.json` is. `crates/cards` exposes `chinese_json()` (`include_str!`) for a new
       `tests/cross/chinese.rs`, which proves four things: every key is a catalog entry and every
       entry has a key; each face has the same set of `{key}`s as its English; names are unique; and
       every term Hearthstone's zh-CN client has is used as it is.
     - Hearthstone's terms: 战吼 Cry, 亡语 Death, 嘲讽 Taunt, 突袭 Rush, 冲锋 Charge, 圣盾 Divine Shield,
       吸血 Lifesteal, 复生 Reborn, 剧毒 Poisonous, 风怒 Windfury, 发现 Discover, 抽到时施放 Cast on draw,
       法术伤害 Spell Damage, 回响 Echo, 幸运币 The Coin.
     - The set's own terms: 光辉 Radiant, 献祭 Tribute, 牌库 Deck, 强化 Buff, 削弱 Nerf, 锁定 Lock,
       瘟疫指示物 Plague Counter, 易碎 Brittle, 活化 Animated, 衍生物 Token, 中国 CN, 猫族 Felinor,
       人类 Human.
     - Writing the table for about 450 entries is most of this batch's work.
  2. **A face printed in Chinese** (#32's Radiant face). Its catalog text is the Chinese itself, and
     its table entry is the same text. No English form of it exists anywhere. The house-style proof
     reads a CJK face by Chinese punctuation (clauses end in "。" or "；", labels end in "："; a cost
     stays "(N)"). R277's gold diff compares it with the other face's Chinese translation, character
     by character (MD-B13).
  3. **The flag.** `CardInstance.chinese: Option<bool>` is only ever `Some(true)` and absent
     otherwise, like `berserk`, so an old state hashes and replays as before.
     - It is set by a NEW verb, `translate`, over a target or a card scope, and by a NEW `chinese`
       rider on `add_to_hand`, `add_random_from_catalog`, `transform`, `transform_random`, `summon`,
       `summon_random` and `shuffle_into`, the same way `radiant` is.
     - Nothing removes it. It persists in every zone and through R78's and R766's resets; it is added
       to their keep lists beside `radiant`.
     - The event is NEW `translated { instanceId }`, sent only where both players can read the card.
       A change in a hand or a deck is silent (R440), and the owner reads it off their own view.
  4. **Who can see it.** It is public, like `radiant`.
     - `CardView.chinese?: true` (and so `UnitView` and `BackrowView`) appears on every view of a card
       the viewer may read. It never appears on the hidden sentinel, a card in someone else's hand, a
       library card or a face-down card the viewer does not control.
     - The owner's library list keeps its records as they were shown (R311). So a deck translated in
       place shows each card in Chinese as it is drawn.
     - In a match the client draws a Chinese card's name, text, type line, tags, keywords and glossary
       from the tables, every face, with params filled as `fillParams` fills them. It marks R279's
       refs by their Chinese names and shows R280's preview labels in Chinese. It never shows the
       English, because the joke is "Get ready to learn Chinese".
     - Outside a match (the Almanac, the deck builder) every card shows its catalog face, so #32's
       Radiant face is Chinese there too.
     - A fused card's Chinese name and text are its ingredients' (`CardDef.ingredients`, R468), joined
       the way the engine joins their English. A copier's copied text (C #57) shows the copied card's
       Chinese.
     - The AI reads no text, so nothing changes for it.
  5. **Copy, transform, fuse** (MD-B11).
     - A copy of a Chinese card is Chinese, as a copy of a Radiant card is Radiant. That covers R57's
       `clone_of`, `add_library_copies`, CN-Virus's copies, T-AI-3 and T-AI-5.
     - A Transform's or an off-field Replace's new card is not Chinese, unless its own text makes it so
       (#37).
     - A Fuse that keeps an instance keeps its flag: R77's field target, R470's hand or deck card. A
       Fuse that makes a new instance is Chinese when any ingredient was.
     - Make Radiant, Vanilla, Nerf, Buff, steal, bounce and death all leave the flag on.
  6. **"Convert its text to Chinese" on a field card** (#35).
     - The flag goes on the permanent. From then on both players' clients show its name and both faces
       in Chinese, in every zone it reaches.
     - No rule changes: its keywords, scripts, tags, params, stats and Immutable are untouched.
     - Immutable does not stop it: the text is the same, only the language it is shown in changes.
     - On a face-down trap, only its controller sees it in Chinese until the trap is revealed.
     - On a Stack pile only the top card is reached (R13).
- **NEW ME-CREATED: Created cards** (#31 reads it; #26's Wells, #27's Coins, #28's and #29's curses and
  #30's Love Bombs are all Created).
  - `CardInstance.created: Option<bool>` is set on every instance minted after the decks are built:
    every `state::new_instance` call except `build_game`'s deck loop and a Glitch reset's new deal
    (R676).
  - That covers cards added, shuffled in or summoned by an effect, new cards cast or Discovered from
    the catalog, copies, a Transform's, Replace's or Fuse's new card, and The Coin (§2.1).
  - A card that keeps its instance keeps its value: drawn, bounced, stolen, given, mulliganed back,
    Recycled, or fused into as the kept card (R77, R470).
  - It persists through R78 and R766. It is public wherever the viewer may read the card
    (`CardView.created?: true`, a small "Created" mark), and the owner's library list (R310) marks
    the cards it knows.
  - New read helpers in `query.rs`: `is_created(card)` and `created_in_library(state, player)` (ids,
    top down).
- **ME-TRIBAL: tribal tags** (#25; #34 and #35 interact with it).
  - **Proposed tribes:** Human, Felinor, KY and CN, the tags that name a people. They are the four the
    catalog's prejudice cards name: Bigot's "non-Human", Big Felinor's "non-Felinor", 高考's "non-CN or
    KY" and Occidentless Mandate's "non-CN". A card tagged All Tribes (Meditative #87) counts as each.
    Book, Fruit, Pancake, AI, Plague, Catalyst, Prime, Acclaimed, Call to Chaos, Quickdraw, Jlockeed,
    Token and Wincon name card families and mechanics, so they are not tribes. The orchestrator
    reconciles this with the group that has #87.
  - **The immunity** is MD-B1. It is a keyword checked in `restrictions.rs` beside E35's
    `unaffected_by`.
  - **Every harmful verb passes its aim to the scope matcher.** Harmful: destroy, exile, damage, steal
    and `give_control`, bounce, transform and replace, vanilla, Nerf, discard, sacrifice, counter, set
    health. Helpful: buff, heal, grant keyword or tag, Buff, Make Radiant, translate, copy. Declared
    picks use `TargetDecl.aim` (R656).
  - Note: a declaration's `tags` means all of them (`play_choices::tags_allow`), while a board or card
    scope's `tags` means any of them. #34 relies on the second.
  - **Existing tag filters the builder must handle:**
    - **Harmful, must skip an immune card:** Core #2 Bigot (its base target filter `notTags: [Human]`
      in `c002_bigot.rs`; its Radiant `destroy_all`); Core #43 Big Felinor (`destroy_all notTags:
      [Felinor]`); and in Meditative, #34 高考 (destroy, `notTags: [CN, KY]`), #45 Knowledge Breaker
      (Nerf every other CN and KY card), #45.1 (destroy or exile every other CN and KY card), #52
      Economic Anxiety (its +3 against a Unit that shares no tag) and #78 Occidentless Mandate (exile
      non-CN).
    - **Harmful but not tribal, so not skipped:** Classic #4 Palantir's steal of a Book
      (`c004_palantir.rs`'s `Tag::Book` test) and Core #85's Token test.
    - **Helpful or neutral, still reach it:**
      - Core #61 Prejudiced Postdoc. Its "Human Unit" pick must declare `aim: help`, because `aim`
        defaults to harm (R656).
      - Core #92 Felinor Fiender (it sums your Felinors' stats).
      - C+ #46 and #46.1 (the Felinor aura), and C+ #68 Organic Produce.
      - Counts of plays by tag: T-AI-2 Scaling Law and C+ #64 Mulch Muncher (`played_this_game_with_tag`).
      - #34's Buffs and #35's pick.
    - **Pools, which read definitions and never a card in play:** Core #57, C+ #2 Groom Shroom, C+
      #12, #13, #30, #39, #40, #42.1, #43, #48, #52, #54, #58, #63, #68, #73 and #78, C #55, Core #95,
      and Core #98's powers.
- **ME-TURN, the zeroed refresh** (#26). This is an aura form of ME-TURN's "lose all mana next turn":
  a static flag `noNaturalMana` that `mana::refresh_mana` reads from any card acting on the field.
  Max mana is still computed; current mana becomes max(0, next-turn rider). Temporary mana, the
  rider, Hinder and Refresh X are unchanged (MD-B2). The timed form for the other group's cards and
  this aura should share one branch in `refresh_mana`.
- **NEW granted tags** (#35; #34 and ME-TRIBAL read them). `CardInstance.granted_tags`, set by a NEW
  `grant_tag` verb, and a NEW `tags_of(state, card)` that returns the definition's tags plus the
  granted ones. Every instance-level tag read switches to `tags_of`:
  - `targets::matches_scope`, `card_scope::matches_card_scope`, `play_choices::card_allowed` and
    `summon.rs`'s instance filter;
  - the scripts' own tag tests: C #4, Core #92, C+ #46 and #46.1;
  - the play records behind `played_this_game_with_tag`, which store the tags a card had as it was
    played.

  Catalog pools do not change. `CardView.tags` appears where a card's tags differ from its
  definition's (MD-B15).
- **NEW `enters_hand` hook** (#37). A Script hook run by `draw::run_arrival_hooks` on hand arrivals
  only, after R151's start-of-game clause. It can draw from the rng, because that function holds the
  sink. It may ask no question; #37 asks none.
- **NEW transform riders and a hand-wide transform** (#32, #37). `transform` and `transform_random`
  gain `costOverride`, `chinese` and `created`. `transform_random` also gains a card scope with
  `same: true`: one definition drawn for every card it replaces.
- **NEW `anyTags` in catalog queries** (#38; also Meditative #60 Eschews' "a random Human, Book, CN, or
  AI-Generated card"). A card matches when it has any of the listed tags; `tags` keeps its all-of
  meaning.
- **NEW `unlock_random_zone(ZoneScope)`** (#27). It mirrors `lock_random_zone`: a uniform pick among
  the Locked zones in scope, and nothing drawn when there is none (R129).
- **NEW `give_control`** (#30; Meditative #6 Me no Likey). It gives a permanent its controller holds
  to the other player. It reuses `steal.rs`'s `take_control` with the receiver named: R15's
  placement, R171's entry and the event `controlChanged`.
- **NEW read helper `highest_permanent_cost(state, player)`** (#30), in `query.rs`. It returns the
  highest R396 field cost among the permanents acting on that side: an X card at its played X, any
  other at `own_cost`, face-down cards at their public cost. It returns `None` when the side has no
  permanent.
- **NEW Prime pool** (#31; Meditative #55 Dragon Fruit and #95's "Add 2 Prime cards"). Add Prime to
  `POOL_TOKEN_TAGS` so a Prime pool holds the Prime tokens (MD-B8).
- **NEW `fill_board` row and side** (#26). `fill_board` gains `row: backrow` and `side: any`, which
  fills each side's empty, unlocked zones for that side's player.
- **NEW turn watcher** (#98). A player modifier with expiry `thisTurn`. It names an event filter (the
  player's `cardResolved` for a permanent, and their `summoned`, landing in lane L on their side) and
  a card step to re-enter, the way `StartOfTurnEffect` re-enters its step.
- **NEW aimed cast** (#98). A `CastHow` field that answers the cast card's first target declaration
  with a named instance. The caster still makes every other choice (R70).

### Group C: Meditative #39 to #49 and their tokens

Twenty entries: the Jade line (#39, its five tokens, #41, #43, #44), Feng Shui (#40), the night market
(#42), Knowledge Breaker and its Prime (#45, #45.1), Conjure Intellect (#46), 饕餮 (#47), Tranquility
(#48) and the YileGPT line (#49 and its three tokens). Every rarity is the designer's; a token carries
`rarity: "Token"` and the designer's rarity as `printedRarity`. Script files use ASCII slugs:
`c039_dushi_addict.rs` (赌石 is *dǔshí*, jade gambling: buying uncut stones hoping for jade),
`c039_1_auspicious_rock.rs` … `c039_5_jade_beauty.rs`, `c040_feng_shui.rs`, `c047_taotie.rs` (饕餮,
the glutton of Chinese myth), `c049_1_yilegpt_unleashed.rs`, `c049_2_yiles_virus.rs`,
`c049_3_ai_girlfriend.rs`. Card text uses Nerf and Buff, as the set-level decisions say. None of these
cards uses ME-CN: the two Chinese names are simply the cards' names.

---

#### Meditative #39 · 赌石 Addict
`meditative-039` · (2) Unit, CN · Rare · 4/4 → 8/8

> **Designer:** [4/4] · Cry: Add an Auspicious Rock to your hand ~~~ [8/8] · Cry: Add a Radiant
> Auspicious Rock to your hand

- **Text:** Cry: Add an Auspicious Rock to your hand.
- **Radiant:** Cry: Add a Radiant Auspicious Rock to your hand.
- **Engine:** `add_to_hand { defId: "meditative-039-1", radiant }` in `cry` (R1: played or cast only). The
  token is named, so no pool rule applies (R382). A full hand burns it (§2.4).
- **Numbers:** none.
- **Check:** R275: stats doubled and the Rock is Radiant (Lucky 2).
- **Class:** A.

#### Meditative #39.1 · Auspicious Rock
`meditative-039-1` · (0) Spell, CN, Token · Token (printed Rare)

> **Designer:** When played lose 2 health and randomly gain: · (20%) Dud · (70%) Jade · (10%) Red Jade
> ~~~ When played lose 2 health and Lucky 2 randomly gain: · (20%) Dud · (70%) Jade · (10%) Red Jade

- **Text:** Lose {life} health. Add one of these to your hand at random: Dud (20%), Jade (70%) or Red
  Jade (10%).
- **Radiant:** Lucky 2
  Lose {life} health. Add one of these to your hand at random: Dud (20%), Jade (70%) or Red Jade (10%).
- **Engine:** `lose_health` on your hero (R18: no pipeline, no Armor, so ME-HERO-IMMUNE and Armor never
  stop it), then one weighted roll, NEW: ME-WEIGHTED-ROLL (`add_rolled_grapes`' table-and-order roll
  made general, the table `AUSPICIOUS_ROCK_ODDS` in engine config). The token rolled lands in your hand
  on its base face. Lucky X is the Rock's printed Lucky (Radiant 2) plus its controller's Luck (NEW:
  ME-LUCK, from #40), each extra roll kept if it's better.
- **Rulings:** **MD-C1: Auspicious Rock makes one weighted roll (Dud 20, Jade 70, Red Jade 10) and adds
  the result to its player's hand, uncast. Its Lucky (its printed Lucky plus its player's Luck) rolls
  again that many times and keeps the best, ranked Dud < Jade < Red Jade.** "When played" on a Spell is
  its resolution. A burned roll is lost (§2.4).
- **Numbers:** `life` 2 → 2 ↓. The odds are a distribution that must add to 100, so they are a config
  constant and not params. A Nerf or Buff moves the Radiant face's Lucky (the X row) and `life`.
- **Check:** R275: the rider is Lucky 2. Red Jade goes from 10% to 27.1%, Dud from 20% to 0.8%.
- **Class:** C (NEW: ME-WEIGHTED-ROLL, NEW: ME-LUCK).

#### Meditative #39.2 · Jade
`meditative-039-2` · (0) Spell, CN, Token · Token (printed Rare)

> **Designer:** Add 1 to your Jade Counter · Gain 1 mana ~~~ Add 2 to your Jade Counter · Gain 2 mana

- **Text:** Add {jade} to your Jade Counter. Gain {mana} mana.
- **Radiant:** Add {jade} to your Jade Counter. Gain {mana} mana.
- **Engine:** ME-JADE's `add_jade { amount }`, whose crossings of 5 and 10 run inside the same effect
  (MD-C3), then `gain_mana` (§2.3 temporary mana; can go above 4).
- **Rulings:** **MD-C2: The Jade Counter is a public count each player has. It starts at 0 and only
  rises. The player who resolves a Jade or a Red Jade adds to their own counter, and both seats see it
  beside that player's hero.**
- **Numbers:** `jade` 1 → 2 ↑; `mana` 1 → 2 ↑.
- **Check:** R279 and R381: this card's name, "Jade", sits inside the rules word "Jade Counter" and
  inside the names Red Jade, Jade Beauty, CN Jade Market and CN Jade Well. Add "Jade Counter" to the
  proof's list of rules words. The longer names follow C+ #38's precedent (`refs` also list Jade where
  R279's naming rule reads it).
- **Class:** C (ME-JADE).

#### Meditative #39.3 · Dud
`meditative-039-3` · (0) Spell, CN, Token · Token (printed Common)

> **Designer:** Deal 2 damage ~~~ Deal 5 damage

- **Text:** Deal {damage} damage.
- **Radiant:** Deal {damage} damage.
- **Engine:** the target is declared with the play (any unit or hero, B1's convention for "Deal N
  damage"), and `damage` is one instance through §4.4.
- **Numbers:** `damage` 2 → 5 ↑.
- **Class:** A.

#### Meditative #39.4 · Red Jade
`meditative-039-4` · (0) Spell, CN, Token · Token (printed Mythic)

> **Designer:** Add 5 to your Jade Counter ~~~ Add 10 to your Jade Counter

- **Text:** Add {jade} to your Jade Counter.
- **Radiant:** Add {jade} to your Jade Counter.
- **Engine:** ME-JADE `add_jade`. From 0, the Radiant face's 10 crosses both thresholds in one add:
  the summon at 5 happens first, then the ascension at 10 (MD-C3).
- **Numbers:** `jade` 5 → 10 ↑.
- **Class:** C (ME-JADE).

#### Meditative #39.5 · Jade Beauty
`meditative-039-5` · (10) Unit, CN, Token · Token (printed Mythic) · 20/20 → 40/40

> **Designer:** (Summons when your Jade Counter reaches 5) · [20/20] · Can’t Attack; Indestructable;
> Immutable · End of turn: Allure all enemy units to join your side, at the start of your next turn
> they join your side (or die of heartbreak if there is not space) ~~~ (Ascends to radiant when your
> Jade Counter reaches 10) · [40/40] · Indestructable; Immutable · End of turn: Allure all enemy units
> to join your side, at the start of your next turn they join your side (or die of heartbreak if there
> is not space)

- **Text:** Can't attack, Indestructible, Immutable
  End of turn: Allure every enemy Unit. (At the start of your next turn they join your side, or die of
  heartbreak if you have no room.)
  You summon this when your Jade Counter reaches 5.
- **Radiant:** Indestructible, Immutable
  End of turn: Allure every enemy Unit. (At the start of your next turn they join your side, or die of
  heartbreak if you have no room.)
  Your Jade Beauties become Radiant when your Jade Counter reaches 10.
- **Engine:** a Unit token that ME-JADE summons. It has no Cry. Indestructible (R46). Immutable (R23):
  no Nerf, Buff, Transform or Fuse-onto, though Radiant still applies. Each end of its controller's
  turn (R62) it runs ME-ALLURE over every enemy Unit then on the field. The counter's 10 turns it
  Radiant on the field (§5.2): 40/40 with its damage kept, Can't attack gone, nothing fires again.
- **Rulings:** **MD-C3: When a player's Jade Counter goes from below 5 to 5 or more, a Jade Beauty is
  summoned for them (R64, no Cry). With a full row none is, and that threshold is spent. When it goes
  from below 10 to 10 or more, every Jade Beauty they control becomes Radiant, and if they control none,
  a Radiant one is summoned. Each threshold acts once a game; one add that crosses both does 5 first,
  then 10.** **MD-C4: Allure marks each enemy Unit on the field at that end of turn (the top of its pile,
  an animated card included) with a public `allure` mark (R437). At the start of the Allurer's next turn
  (R62's start-of-turn delayed effects, as K-Pop Fanatic's steal), each marked Unit still under the
  opponent's control is stolen, in the order marked, which is lane order (R15: the same lane, else the
  first free zone). If no unit zone is open, it is destroyed instead: an ordinary destroy, so Death and
  Reborn apply and an Indestructible Unit stays where it is (R46). A marked Unit that leaves the field
  loses its mark and is not stolen (R174). The Allure still lands if Jade Beauty has left the field (R76),
  and a stolen Unit is summoning sick (R171).** Handed out as a card by C+ #23 Dropshipping's
  every-token pool (R382), it is played at its printed (10), with no Cry to fire.
- **Numbers:** none. The thresholds 5 and 10 are ME-JADE's config constants, not this card's numbers.
- **Check:** "Indestructable" corrected to Indestructible; the header was missing a comma. R275: stats
  doubled and the Radiant can attack.
- **Class:** C (ME-JADE, ME-ALLURE).

#### Meditative #40 · Feng Shui
`meditative-040` · (2) Field Spell, CN · Legendary

> **Designer:** All cards gain the tag 水，木，火，金，土 · Auspicious behavior will be rewarded and
> inauspicious behavior will be punished · Last played tag opponent: · Last played tag you: · (hidden
> text: if a card reacts positively with your last played card it becomes Radiant; if it reacts
> negatively you take 10 damage and it gains Brittle 2) · Aura: gain Luck 1 ~~~ All cards gain the tag
> 水，木，火，金，土 · Auspicious behavior will be rewarded for you; and inauspicious behavior will be
> punished heavily for your opponent · Last played tag opponent: · Last played tag you: · (hidden text:
> if a card reacts positively with your last played card it becomes Radiant; if it reacts negatively you
> take 10 damage and it gains Brittle 2) · Aura: gain Luck 1

- **Text:** Aura: Every card has an element: 水, 木, 火, 金 or 土.
  Auspicious behavior will be rewarded and inauspicious behavior will be punished.
  Your last element:
  Your opponent's last element:
  Aura: You have Luck {luck}.
- **Radiant:** Aura: Every card has an element: 水, 木, 火, 金 or 土.
  Auspicious behavior will be rewarded for you, and inauspicious behavior will be punished heavily for
  your opponent.
  Your last element:
  Your opponent's last element:
  Aura: You have Luck {luck}.
- **Engine:** ME-ELEMENT, a subsystem (`subsystems/feng_shui.rs`).
  - Every definition has an element, `element_of(def)` (MD-C5). While any Feng Shui is on the field,
    each card that a seat can read carries its element in that seat's view. Elements are public data.
  - Each player's last face-up play is recorded at §10.5 step 4, beside R451's records (MD-C7). This
    happens whether or not a Feng Shui is on the field, so a Feng Shui that arrives judges at once. Its
    two "last element" lines show both records through `preview` (R280), with the glyph as `display`
    (R372) and "—" before any play.
  - At §10.5 step 3, where Gifted Program's static flag is read (R213, R214, R449), each Feng Shui on
    the field judges the play (MD-C6, MD-C8, MD-C9). A positive play is made Radiant and resolves on
    that face. A negative play is given Brittle 2 (R385, R638: the count starts as the card enters the
    field, so a Spell's never starts), and a Feng Shui trigger answering its `cardPlayed` deals 10
    damage (20 on the Radiant face) from Feng Shui to that player's hero through §4.4, after the play
    resolves.
  - A NEW public event, `fengShui { instanceId, player, outcome }`, lets the client draw 吉 or 凶. It
    needs an M5-T4 row and a SOUND_CUES row. The outcome is visible from its results anyway.
  - The Aura is NEW: ME-LUCK, Luck 1 for its controller.
- **Rulings:**
  - **MD-C5: A card's element is the Hetu (河图) element of the last digit of its index: 1 and 6 水
    (water), 2 and 7 火 (fire), 3 and 8 木 (wood), 4 and 9 金 (metal), 5 and 0 土 (earth). An index
    with no digit (T-rush, T-coin and the other named tokens) is 土, the centre. A fused or crafted card
    has its first ingredient's element (R77's order). So Feng Shui itself is 土 and Jade (#39.2) is 火.**
  - **MD-C6: The relation runs from your last played card to the card now played. It is positive when
    the last card's element generates the new one (木→火→土→金→水→木) and negative when it overcomes it
    (木→土→水→火→金→木). The same element, the two reverse relations and a player's first play are
    neutral, so a random play is positive one time in five and negative one time in five.**
  - **MD-C7: Every face-up play is judged and recorded, casts included (R70). A Trap, a Field Trap and
    any card set face-down (R448, ME-ALTPLAY) is neither judged nor recorded, so the hidden rule never
    acts on or reveals a hidden card. The record holds the element of the player's last face-up card,
    is never cleared, and ignores R451's AI-card skip. A countered play is neither judged nor recorded
    (R448).**
  - **MD-C8: The reward makes the card Radiant at §10.5 step 3 (already Radiant: nothing more). The
    punishment gives the card Brittle 2 there (a given count, which sets it to 2) and deals the damage
    once the play has resolved. Feng Shui does not judge its own play, since it is not on the field at
    that step.**
  - **MD-C9: The base face judges both players' plays. The Radiant face rewards only its controller's
    plays and punishes only the opponent's, for 20 damage. Each Feng Shui on the field judges each play
    on its own, so two punishments stack.**
  - **MD-C10: The hidden rule's numbers are engine config (`FENG_SHUI_DAMAGE` 10, Radiant 20;
    `FENG_SHUI_BRITTLE` 2) and are printed nowhere. Like `GRAPE_ODDS` they are no params, and Nerf and
    Buff never move them.**
  - **MD-C11: Elements are not `Tag`s: no pool, filter, `notTags` or "shares a tag" rule reads them
    (⚠ designer if Meditative #52 Economic Anxiety should).**
  - **MD-C12: "Luck X" is a player's Lucky X (ME-LUCK): while Feng Shui is on its controller's field,
    every luck-based roll that player's cards make rolls X more times and keeps the best. A roll with no
    "best" is unaffected (R32).**
  - The AI learns the hidden rule only as any simulation does, which is fine for a computer (C+ #42's
    precedent).
- **Numbers:** `luck` 1 → 1 ↑ (min 1).
- **Check:**
  - The designer's hidden text repeats "10 damage" on the Radiant face, but its visible text says
    "punished heavily". The smallest change that keeps "heavily" true is double damage, 20, with
    Brittle 2 kept. **⚠ designer.**
  - R275 is met by the asymmetry: the Radiant face never punishes you and never rewards the opponent.
  - The header lacked "Meditative".
  - The designer's full-width "，" becomes ", ".
- **Class:** D (ME-ELEMENT), with NEW: ME-LUCK.

#### Meditative #41 · CN Smuggler
`meditative-041` · (2) Unit, CN · Rare · 4/5 → 8/10

> **Designer:** [4/5] · Start of Turn: Add an Auspicious Rock to your hand and a random CN card ~~~
> [8/10] · Start of Turn: Add a Radiant Auspicious Rock to your hand and a random Radiant CN card

- **Text:** Start of turn: Add an Auspicious Rock and a random CN card to your hand.
- **Radiant:** Start of turn: Add a Radiant Auspicious Rock and a random Radiant CN card to your hand.
- **Engine:** `start_of_turn` (its controller's, R62): `add_to_hand` the Rock, then
  `add_random_from_catalog { tags: [CN] }` through `catalog.pool` (non-token CN cards of every set,
  R380 and R382, never this card, R387). Both cards are Radiant on the Radiant face. The hand cap burns
  extras.
- **Numbers:** none (the "an" and "a" are no printed numbers, as on C+ #78).
- **Check:** "Start of Turn" gets house-style case.
- **Class:** B.

#### Meditative #42 · CN Flea Market
`meditative-042` · (0) Spell, CN · Legendary

> **Designer:** Open up a night market; you gain 50 yuan to buy cards · (For AI offer primarily CN
> cards Auspicious Rocks and other AI generated cards that are related to CN) ~~~ Open up a night
> market; you gain 80 yuan to buy cards; you may barter · (For AI offer primarily CN cards Auspicious
> Rocks and other AI generated cards that are related to CN; Barter system is up for your
> interpretation)

- **Text:** Open a night market. You have {yuan} yuan to buy its cards.
- **Radiant:** Open a night market. You have {yuan} yuan to buy its cards, and you may barter cards
  from your hand.
- **Engine:** ME-MARKET, a subsystem (`subsystems/night_market.rs`).
  - **The stall** is rolled once, from the match rng, as the Spell resolves (MD-C13). It holds three
    random non-token CN cards (every set, R380 and R382, all different, R60, never this card, R387),
    two Auspicious Rocks (#39.1) and one random AI generated card (T-AI-1 to T-AI-10, the pool C+ #78
    names).
  - **Prices** are in yuan (¥), set by engine config: `YUAN_PER_COST` (10) × the card's printed cost
    out of play (R65: X counts 0, an embiggen card its base price), plus `YUAN_PER_RARITY` (5) × its
    rarity rank (Common 1 to Mythic 5). A token is ranked by its printed rarity, and the AI cards, which
    print none, count as Common. A Radiant card costs double. So a Rock is 10 ¥, a (1) AI card 15 ¥, a
    (2) Rare CN card 30 ¥ and a (4) Legendary 60 ¥.
  - **The prompt** is a NEW kind, `market`, held by the caster, with its `budget` set to the yuan left.
    It offers each lot the caster can afford (no lots while their hand is full), on the Radiant face
    one barter option per card in their hand, and Leave.
  - **Each answer is one deal.** A buy pays the price and creates the card in the caster's hand on its
    base face at its printed cost (`addedToHand`, hidden per zone). A barter exiles that hand card
    (public) and adds its price to the yuan. The prompt then reopens through `resume` (R113) until
    Leave, or until nothing can be bought or bartered. Unspent yuan is lost.
  - **Hidden information:** the options reach the chooser alone (§10.6); the opponent sees only that a
    prompt is open.
  - **The AI** answers a `market` prompt as it answers every prompt. `decide` enumerates its answers
    (at most 6 lots, 10 barters and Leave) and simulates each on determinized copies (R185). The stall
    is fixed when the market opens, so every simulation sees the same lots. A timeout (R79) and the
    random policy answer uniformly (§10.7).
- **Rulings:** **MD-C13: A night market's stall is three CN cards, two Auspicious Rocks and one AI
  generated card, rolled once as it opens and priced in yuan by printed cost and rarity. The caster
  makes one deal per answer until they leave. A bought card arrives on its base face at its printed
  cost, no lot is offered while the hand is full, and unspent yuan is lost when the market closes.**
  **MD-C14: To barter (Radiant face only) is to trade a card from your hand to the merchant: it is
  exiled and its price, by the same rule and doubled if it is Radiant, joins your yuan. Any number of
  hand cards may be bartered.**
- **Numbers:** `yuan` 50 → 80 ↑ (step 10).
- **Check:**
  - R275: 1.6× the yuan plus the barter rider.
  - "Other AI generated cards that are related to CN" reads as the ten AI generated cards (the
    set-level definition).
  - ⚠ designer: if a CN-themed AI line was meant, the CN-tagged AI-themed tokens #49.1 to #49.3 could
    stock that shelf instead.
- **Class:** D (ME-MARKET).

#### Meditative #43 · CN Jade Market
`meditative-043` · (X) Spell, CN · Rare

> **Designer:** Add X Auspicious Rocks to your hand ~~~ Add X Radiant Auspicious Rocks to your hand

- **Text:** Add X Auspicious Rocks to your hand.
- **Radiant:** Add X Radiant Auspicious Rocks to your hand.
- **Engine:** X is chosen with the play and is between 1 and the caster's current mana (R348); cost
  modifiers never apply (R65). Then `add_to_hand` the Rock X times, Radiant on the Radiant face; the
  hand cap burns extras (§2.4). A Nerf or Buff moves X by 1 at resolution (§6.3's X row).
- **Numbers:** none (X is chosen).
- **Class:** A.

#### Meditative #44 · CN Jade Well
`meditative-044` · (3) Field Spell, CN · Rare

> **Designer:** Start of turn: Add a Auspicious Rocks to your hand ~~~ Start of turn: Add a Radiant
> Auspicious Rocks to your hand

- **Text:** Start of turn: Add an Auspicious Rock to your hand.
- **Radiant:** Start of turn: Add a Radiant Auspicious Rock to your hand.
- **Engine:** a Field Spell's `start_of_turn` (R62), `add_to_hand` the Rock. Core #6 Mana Well's shape:
  the same cost, the same "Well", the same clock.
- **Rulings:** **MD-C15: CN Jade Well is a Field Spell; the designer typed it Spell. A start-of-turn text
  needs a card that stays on the field, as R402 reasoned for C #78. C+ #52's "For the rest of the game"
  Spell is the other reading, rejected: it would be an unanswerable Rock every turn at (3).**
- **Numbers:** none.
- **Check:** "a Auspicious Rocks" corrected. R275: the rider is the Rock's Lucky 2.
- **Class:** A.

#### Meditative #45 · Knowledge Breaker
`meditative-045` · (1) Unit, CN, KY, Catalyst · Legendary · 1/3 → 2/6

> **Designer:** [1/3] · Cry: Degrade all other CN & KY cards · Aura: your units may be played as field
> Animated Field Traps that activate at the Start of your turn · Death: Shuffle Knowledge Breaker Prime
> into your Deck ~~~ [2/6] · Cry: Degrade all other CN & KY cards twice · Divine Shield · Aura: your
> units may be played as field Animated Field Traps that activate at the Start of your turn · Death:
> Shuffle Radiant Knowledge Breaker Prime into your Deck

- **Text:** Cry: Nerf every other CN or KY card on the field, in each hand and in each deck.
  Aura: You may play your Units face-down into your backrow as Animated Field Traps that reveal at the
  start of your turn.
  Death: Shuffle a Knowledge Breaker Prime into your deck.
- **Radiant:** Divine Shield
  Cry: Nerf every other CN or KY card on the field, in each hand and in each deck {times|time|times}.
  Aura: You may play your Units face-down into your backrow as Animated Field Traps that reveal at the
  start of your turn.
  Death: Shuffle a Radiant Knowledge Breaker Prime into your deck.
- **Engine:**
  - Cry: `degrade { scope: { side: any, zones: [field, hand, library], tags: [CN, KY], excludeSelf },
    times }`. The scope's `tags` is any-of, so "CN & KY" is the union. Changes to cards a seat cannot
    read are reported per R440.
  - Death: `shuffle_into` Knowledge Breaker Prime (C+ #38's Catalyst shape), Radiant on the Radiant
    face.
  - Aura: ME-ALTPLAY's permission, live while this acts on its controller's field (MD-C16, MD-C17).
    `determinize` (R185, R762) must also sample a hidden backrow card from that seat's Units while a
    permission is on its side.
- **Rulings:**
  - **MD-C16: While a card with this Aura acts on its controller's field, `legalActions` also offers
    each Unit card in that player's hand as a face-down play into an empty, unlocked backrow zone (a new
    flag on the `play` action). The price is the Unit's: mana, X and Tribute, with R65's cost rules
    reading it as the Unit it is in hand. It declares no targets or modes. It is announced and placed as
    a Trap (R448, a fresh id per R227) and counted as a Trap play (B2.7). While face-down it is a Field
    Trap and none of its own text acts. The form ends when it leaves the field (R78), but not when the
    Aura leaves.**
  - **MD-C17: At its controller's start of turn (the start-of-turn triggers, traps first, §10.3) it
    fires: it turns face-up and animates (R383: its own lane, else R64; `animated`, never `summoned`,
    R445). Then its Cry runs, with its declared choices asked as prompts (R467's way). It keeps the turn
    it was set as its `summonedTurn`, so it may attack that turn, like a Yu-Gi-Oh set monster flipped up
    or a Hearthstone minion that was on the board as the turn began. This is the one exception to R83
    and R171. With no open unit zone it stays face-up in the backrow (a Field Trap, R383) and fires again
    at the next start of turn, and its Cry waits for the animation.**
  - **MD-C18: "All other CN & KY cards" is every card other than this one with the CN tag or the KY
    tag, in every zone its verb reaches. A Nerf reaches both fields, both hands and both decks; a
    Destroy reaches both fields.**
- **Numbers:** `times` 1 → 2 ↑ (on the Radiant face only, R749).
- **Check:**
  - "field Animated Field Traps" corrected (the first "field" dropped).
  - Divine Shield moved to the keyword line.
  - R279: this card's name sits inside its Prime's, so `refs` list both (C+ #38's precedent).
  - R275: stats doubled, Divine Shield added, Nerf twice, Radiant Prime.
- **Class:** C (ME-ALTPLAY).

#### Meditative #45.1 · Knowledge Breaker Prime
`meditative-045-1` · (4) Unit, CN, KY, Prime, Token · Token (printed Legendary) · 6/18 → 12/36

> **Designer:** [6/18] · Rush · Cry: Summon 5 Random Traps; Destroy all other CN & KY cards · Aura: your
> units may be played as field Animated Traps that activate at the Start of your turn ~~~ [12/36] · Rush,
> Poisonous · Cry: Summon 5 Random Radiant Traps; Exile all other CN & KY cards · Aura: your units may be
> played as field Animated Traps that activate at the Start of your turn

- **Text:** Rush
  Cry: Summon {traps|random Trap|random Traps}. Destroy every other CN or KY permanent.
  Aura: You may play your Units face-down into your backrow as Animated Field Traps that reveal at the
  start of your turn.
- **Radiant:** Rush, Poisonous
  Cry: Summon {traps|random Radiant Trap|random Radiant Traps}. Exile every other CN or KY permanent.
  Aura: You may play your Units face-down into your backrow as Animated Field Traps that reveal at the
  start of your turn.
- **Engine:**
  - Cry, in the text's order: first `summon_random { query: { type: TRAP_TYPES } }` five times. These
    are non-token Traps and Field Traps of every set (R380), repeats allowed (R60), placed face-down
    (R33) in the leftmost open backrow zones (R64); a full backrow takes fewer. Then
    `destroy_all { side: any, rows: both, tags: [CN, KY], excludeSelf }`, where a pile's top is the
    card on the field (R13), face-down cards are included, and Indestructible cards survive (R46). The
    Radiant face uses `exile_all` with the same scope.
  - Aura: ME-ALTPLAY, as #45.
- **Rulings:** **MD-C19: The Prime's "Animated Traps" is the same face-down play as Knowledge Breaker's
  "Animated Field Traps". R383 already keeps an Animated Trap that has no room face-up, as a Field Trap
  does, so the two wordings differ in nothing.** MD-C18 sets the scope. The Radiant face's Exile keeps
  the base face's reach (both fields): the designer changed the verb, not the scope. A CN or KY Trap
  among the five is destroyed with the rest.
- **Numbers:** `traps` 5 → 5 ↑ (max 5, the backrow's size).
- **Check:** R275: stats doubled, Poisonous added, Radiant Traps, and an exile that beats Indestructible,
  Reborn and Death.
- **Class:** C (ME-ALTPLAY).

#### Meditative #46 · Conjure Intellect
`meditative-046` · (2, embiggen 4) Spell · Epic

> **Designer:** Add a random a KY, CN, and Book to your hand; embiggen they cost (0) ~~~ Add a random a
> Radiant KY, Radiant CN, and Radiant Book to your hand; embiggen they cost (0)

- **Text:** Add a random KY card, a random CN card and a random Book to your hand.
  Paid (4): They cost ({setCost}).
- **Radiant:** Add a random Radiant KY card, a random Radiant CN card and a random Radiant Book to your
  hand.
  Paid (4): They cost ({setCost}).
- **Engine:** cost `{ base: 2, embiggen: 4 }`. Three independent `add_random_from_catalog` draws, with
  `tags: [KY]`, then `[CN]`, then `[Book]`: non-token cards of every set (R380, R382), never this card
  (R387), repeats allowed (R60). `cost_override` is `setCost` when `instance.embiggened`; `radiant` is
  set on the Radiant face. The hand cap burns extras.
- **Numbers:** `setCost` 0 → 0 ↓ (min 0).
- **Check:** "a random a" corrected. R275: three Radiant cards.
- **Class:** B.

#### Meditative #47 · 饕餮
`meditative-047` · (4) Unit, CN · Legendary · 8/8 → 16/16

> **Designer:** [8/8] · End of turn: Fuse a random enemy permanent into this ~~~ [16/16] · End of turn:
> Fuse a random enemy permanent into this; and one from their deck

- **Text:** End of turn: Fuse a random enemy permanent into this.
- **Radiant:** End of turn: Fuse a random enemy permanent and a random card from your opponent's deck
  into this.
- **Engine:** `end_of_turn` (R62), NEW: ME-FUSE-RANDOM: `fuse_cards` with this as the kept target and
  one ingredient drawn inside `apply` from the match rng. It fuses per R77 and R102: this keeps its
  instance, zone, damage and radiant flag; stats are summed; keywords, tags and text are joined; the
  cost is min(sum, 4), so it stays (4). The eaten card ceases to exist, with no Death and no
  `destroyed`. The Radiant face then runs a second fusion with a random card of the opponent's library.
- **Rulings:** **MD-C20: 饕餮's ingredient is drawn uniformly from the enemy permanents on the field (the
  top of each pile, R13, both rows, face-down cards included), never an Immutable one (R23: being eaten
  changes its text). With none, nothing is fused. The Radiant face's deck card is drawn uniformly from
  the opponent's library, Immutable cards excluded, and is public once it is part of 饕餮's face. What
  the fused texts do follows C+ #74: an aura covers your side, start- and end-of-turn lines run, a Death
  fires when 饕餮 dies, and an Activate is 饕餮's to use. A Cry never runs again, and a Trap's "Reveals
  when" never fires from a unit zone.** An Immutable 饕餮 eats nothing (R23). A long fused id becomes a
  digest (R468).
- **Numbers:** none.
- **Check:** the header was missing a comma ("Legendary Meditative"). R275: stats doubled, plus a second
  meal each turn.
- **Class:** B (plus NEW: ME-FUSE-RANDOM, one small verb).

#### Meditative #48 · Tranquility
`meditative-048` · (2) Spell · Epic

> **Designer:** Immediately end your turn; your hero is immune to damage until the start of your turn
> turn ~~~ Your hero is immune to damage until the start of your turn turn

- **Text:** End your turn. Your hero is immune to damage until your next turn begins.
- **Radiant:** Your hero is immune to damage until your next turn begins.
- **Engine:**
  - The end of the turn is E10's `end_turn` (§6.3, R456): the rest of the list resolves first, then
    every end-of-turn step runs. On the opponent's turn, a cast has no turn of theirs to end.
  - The immunity is NEW: ME-HERO-IMMUNE, a player modifier read by `hero_damage_cap` as a cap of 0
    (§4.4 step 3, beside C+ #11's `heroGuard`). Its expiry is `nextTurnOf` the opponent, R757's Armor Up
    expiry: it is taken off by the cleanup that ends the opponent's next turn, so it is gone as your
    next turn begins. `modifierChanged` shows a badge on the hero.
- **Rulings:** **MD-C21: "Immune to damage" makes every damage instance to that hero 0 (a cap of 0 at
  §4.4 step 3), fatigue included, so Lifesteal against it heals nothing. Lose health (R18) and Set health
  (C #29) still happen. It lasts until its player's next turn begins.**
- **Numbers:** none.
- **Check:**
  - "turn turn" corrected.
  - R275: the Radiant face drops the drawback, which is the designer's upgrade.
  - The name already appears in C+ #19.5's designer text ("Tranquility makes this unit not be able to go
    Berserk"). The catalog prints that line as "This can't go Berserk" (R412), so the name names no
    other card and R279's proof is unaffected.
- **Class:** B (plus NEW: ME-HERO-IMMUNE, one small modifier kind).

#### Meditative #49 · YileGPT Tamed
`meditative-049` · (2) Unit, CN, Acclaimed · Mythic · 2/6 → 4/12

> **Designer:** [2/6] · Cry: summon a Virus in the opposite lane of this · Death: place YileGPT
> Unleashed at the bottom of your deck ~~~ [4/12] · Cry: summon a Radiant Virus in the opposite lane of
> this · Death: place Radiant YileGPT Unleashed at the bottom of your deck

- **Text:** Cry: Summon a Yile's Virus for your opponent across from this.
  Death: Put a YileGPT Unleashed on the bottom of your deck.
- **Radiant:** Cry: Summon a Radiant Yile's Virus for your opponent across from this.
  Death: Put a Radiant YileGPT Unleashed on the bottom of your deck.
- **Engine:**
  - Cry: `summon { defId: "meditative-049-2", player: enemy, lane: this lane }`. The opponent owns and
    controls the token (R12), and an aimed summon fails if that zone is occupied (§3.2, R47).
  - Death: NEW: ME-DECK-BOTTOM, a fresh YileGPT Unleashed created at the bottom of the deck of the
    player who controlled this as it died. R80's cap applies, and its owner's own-library list shows the
    card (R310).
- **Rulings:** "Virus" is #49.2 Yile's Virus. The CN-Virus is a Spell token and can't be summoned, and
  one name names one card (§8), so the text prints the full name.
- **Numbers:** none.
- **Check:** R275: stats doubled, a Radiant Virus and a Radiant Unleashed.
- **Class:** B (plus NEW: ME-DECK-BOTTOM).

#### Meditative #49.1 · YileGPT Unleashed
`meditative-049-1` · (10) Unit, CN, Acclaimed, Token · Token (printed Mythic) · 8/20 → 16/40

> **Designer:** (Excess mana you end your turn with decrease the cost of this card) · [8/20] · Cry:
> Randomly picks 2 out of 5 · Summon 2 Claude’s Datacenter · Add 3 Radiant Glitch in the systems to your
> hand, they cost (0) · Summon an AI Girlfriend · Fill your opponents board with Virus · Degrade all
> cards in your opponents, board, hand, and deck twice. · Can’t Attack · Start of turn: make a random
> friendly permanent radiant ~~~ (Excess mana you end your turn with decrease the cost of this card) ·
> [16/40] · Cry: Do the following: · Summon 2 Claude’s Datacenter · Add 3 Radiant Glitch in the systems
> to your hand, they cost (0) · Summon an AI Girlfriend · Give your opponent two Viruses · Degrade all
> cards in your opponents, board, hand, and deck twice. · Can’t Attack; Armor 5 · Start of turn: make two
> random friendly permanent radiant

- **Text:** Can't attack
  Cry: Do {picks} of these at random: summon {datacenters|Claude's Datacenter|Claude's Datacenters};
  add {glitches|Radiant Glitch in the System card|Radiant Glitch in the System cards} to your
  hand, which cost ({setCost}); summon an AI Girlfriend; fill your opponent's board with Yile's
  Viruses; Nerf every card on your opponent's field, in their hand and in their deck
  {nerfs|time|times}.
  Start of turn: Make a random permanent of yours Radiant.
  Whenever you end your turn with unspent mana, this costs that much less.
- **Radiant:** Can't attack, Armor 5
  Cry: Summon {datacenters|Claude's Datacenter|Claude's Datacenters}. Add {glitches|Radiant Glitch in
  the System card|Radiant Glitch in the System cards} to your hand; they cost ({setCost}). Summon an AI
  Girlfriend. Summon {viruses|Radiant Yile's Virus|Radiant Yile's Viruses} for your opponent. Nerf every
  card on your opponent's field, in their hand and in their deck {nerfs|time|times}.
  Start of turn: Make {radiants|random permanent|random permanents} of yours Radiant.
  Whenever you end your turn with unspent mana, this costs that much less.
- **Engine:**
  - The discount is a hand trigger and a deck trigger (R464) on its owner's `turnEnded`. They read the
    event's `unspentMana`, the number Bread and Butter reads, and apply `set_cost_mod { amount:
    -unspent }`. The `costMod` keeps in hand and deck until the card reaches a graveyard or exile
    (R766), and the price floors at 0.
  - The Cry is NEW: ME-PICK-N over the five entries:
    - `summon` C+ #78 twice into your backrow (R64);
    - `add_to_hand` C #18 three times, Radiant, `cost_override` 0 (each play of one raises the Glitch
      odds, R673);
    - `summon` #49.3 into your backrow, where it animates at once (MD-C25);
    - `fill_board { defId: "meditative-049-2", player: enemy }`;
    - `degrade { scope: { side: enemy, zones: [field, hand, library] }, times: 2 }` (hidden per R440,
      as C+ #73's entry 7).
  - Start of turn: `set_radiant_random { zones: [field], count }`, among your non-Radiant permanents
    (R60, R242), this one included.
- **Rulings:**
  - **MD-C22: YileGPT Unleashed costs (1) less for each mana its owner has unspent as each of their
    turns ends, while it is in their hand or deck. It is a permanent `costMod`.**
  - **MD-C23: "Randomly picks 2 out of 5" draws two different entries (R60) from the match rng as the
    Cry resolves. They resolve in the list's order and are named to both players before they resolve
    (R423's and R436's way). The Radiant face does all five, in order.**
  - **MD-C24: "Give your opponent two Viruses" summons two Yile's Viruses for the opponent (R64), as the
    base face's "fill your opponent's board" does. A Virus in their hand would do nothing.**
- **Numbers:**
  - `picks` 2 → 2 ↑ (base face only, max 5; the Radiant face prints no number, R749);
  - `datacenters` 2 → 2 ↑, `glitches` 3 → 3 ↑, `setCost` 0 → 0 ↓ (min 0), `nerfs` 2 → 2 ↑;
  - `viruses` 2 ↑ and `radiants` 1 → 2 ↑, on the Radiant face only (R749).
  - Armor 5 is a numbered keyword.
- **Check:**
  - "Unleased", "Glitch in the systems" (C #18 is Glitch in the System) and "opponents, board, hand"
    corrected.
  - The Radiant face's two Viruses are weaker than the base face's "fill the board", even though the face
    as a whole is far stronger (five entries against two). The smallest fix in the designer's direction
    makes the two Viruses Radiant (twice the discard and damage). **⚠ designer.**
  - R279: `refs` list C #18 (whose name holds the hidden Glitch's, R674, as C #18's own `refs` already
    face), C+ #78, #49.2 and #49.3.
- **Class:** C (NEW: ME-PICK-N).

#### Meditative #49.2 · Yile's Virus
`meditative-049-2` · (2) Unit, CN, Acclaimed, Token · Token (printed Mythic) · 0/8 → 0/16 (designer 0/8; R275's stat half doubles it)
⚠ designer)

> **Designer:** [0/8] · Can’t attack; Can’t be in defense position · Activate: Destroy this · Death:
> Discard a random card · Start of turn: Deal 2 damage to adjacent non Virus units & to your hero ~~~
> [0/8] · Can’t attack; Can’t be in defense position · Activate: Destroy this · Death: Discard 2 random
> cards · Start of turn: Deal 4 damage to adjacent non Virus units & to your hero

- **Text:** Can't attack
  Can't be in Defense Position.
  Activate: Destroy this.
  Death: Discard {discards|random card|random cards}.
  Start of turn: Deal {damage} damage to each adjacent Unit that isn't a Yile's Virus, and to your hero.
- **Radiant:** the same text, with its own `discards` and `damage`.
- **Engine:**
  - Can't attack is a keyword. Can't be in Defense Position is the static flag Spikey Pillow uses
    (§4.1), so the token never has Defense's Taunt.
  - Activate (R384): once a turn, in its controller's main phase, `destroy` self.
  - Death: `discard_random` from its controller's hand (R682). It fires even though the token vanishes
    (R11, §6.2).
  - Start of turn (its controller's): one `damage` instance each to the Units beside it on its side and
    row (§3.1), skipping those whose definition is Yile's Virus, then one to its controller's hero.
- **Rulings:** "Your hero" and "your" are the Virus's controller, its victim, since #49 and #49.1 create
  it for the opponent (R12). Forced attacks still make it attack (R53; see #49.3).
- **Numbers:** `discards` 1 → 2 ↓; `damage` 2 → 4 ↓. Better is down because the Virus's controller is
  its victim, as CN-Virus's numbers run (§7).
- **Check:**
  - R275: the Radiant face prints 0/8 again. The proposal is 0/16, the doubled health. **⚠ designer.**
  - "non Virus" written out.
  - The designer's curly "Yile’s" is the catalog's straight apostrophe.
- **Class:** B.

#### Meditative #49.3 · AI Girlfriend
`meditative-049-3` · (4) Field Spell, CN, Acclaimed, Token · Token (printed Mythic) · 2/30 → 4/60

> **Designer:** [2/30] · Can’t attack · Start of turn: all enemy units attack this unit · After this is
> attacked: degrade the attacking unit ~~~ [4/60] · Can’t attack · Start of turn: all enemy units attack
> this unit · After this is attacked: degrade the attacking unit twice

- **Text:** Animated, Can't attack
  Start of turn: Every enemy Unit attacks this.
  After this is attacked: Nerf the attacker.
- **Radiant:** Animated, Can't attack
  Start of turn: Every enemy Unit attacks this.
  After this is attacked: Nerf the attacker {times|time|times}.
- **Engine:**
  - Animated (§6.1, R383): summoned into the backrow, it animates as it enters the field, into its
    lane's unit zone or else R64's, and is a Unit for every rule from then on.
  - Start of turn: Core #9 Moths to the Flame's `forced_attacks_on` self (R53). Every enemy Unit
    attacks in lane order, each in its own combat with its own state check, and the run stops once this
    leaves the field. Forced attacks skip the declare check, so Can't attack Units and 0-attack Units
    attack too (R53).
  - NEW: ME-AFTER-ATTACKED, `Script.after_attacked`: `degrade` the attacker, once (twice on the Radiant
    face).
- **Rulings:**
  - **MD-C25: AI Girlfriend is an Animated Field Spell. The designer printed a Field Spell with stats,
    and Animated is the keyword that puts a Field Spell's stats in a unit zone (R383; Frostspatula,
    C+ #12.8, is the printed precedent of a Field Spell with a unit face). If no unit zone is open when
    it enters, it stays a backrow card, and its forced attacks find no target.**
  - **MD-C26: "After this is attacked" runs once for each combat in which this was the attack's target,
    declared or forced (Cleave splash is no attack on it). It runs after that combat's state check, on
    its pre-check snapshot even if it died, as R426 runs `after_attack` for the attacker. It Nerfs the
    attacker only if the attacker is still on the field.**
- **Numbers:** `times` 1 → 2 ↑ (on the Radiant face only, R749).
- **Check:** R275: stats doubled and Nerf twice. "Animated" is a keyword the designer did not print; it
  is the reading the card needs (MD-C25).
- **Class:** C (NEW: ME-AFTER-ATTACKED).

---

#### Group C's systems

- **ME-JADE** (#39.2, #39.4, #39.5; through them #39, #39.1, #41, #42's Rock lots, #43 and #44; in other
  groups #54 Money Machine's hidden Jades and #95's "Summon a Jade Beauty"). A public count
  `PlayerState.jade`, 0 at setup, that only rises, written by a verb `add_jade(amount)` with a NEW public
  event `jadeChanged { player, value }` (BUILD M5-T4 and SOUND_CUES rows). Inside the same add, it crosses
  the config thresholds `JADE_BEAUTY_AT` 5 (summon a Jade Beauty, R64) and `JADE_ASCEND_AT` 10 (make your
  Jade Beauties Radiant, or summon a Radiant one when you have none), each once a game (MD-C2, MD-C3).
  The view shows it beside the hero, "Jade Counter" joins R381's rules words, and §6 gains a glossary row.
- **ME-ALLURE** (#39.5). A verb `allure(scope)`: for each enemy Unit in the scope, an `effects::delay`
  until the start of your next turn that watches the Unit (R174) and marks it `allure` (R437). On landing
  it steals the Unit (R15), and with no open unit zone destroys it, which needs a new `otherwise:
  destroy` on `steal` (MD-C4). §6.3 gains an "Allure" row.
- **ME-ELEMENT** (#40). A subsystem `subsystems/feng_shui.rs` that holds:
  - `element_of(def)`, the Hetu rule (MD-C5);
  - a per-player record of the last face-up play's element, written at §10.5 step 4 (MD-C7);
  - the step-3 judgement beside Gifted Program's flag (R213, R214, R449), which makes a positive play
    Radiant (MD-C6, MD-C8, MD-C9);
  - the punishment: a given Brittle 2 and a damage trigger on `cardPlayed`;
  - the hidden config numbers (MD-C10);
  - view fields (each readable card's element while a Feng Shui is on the field, and both records as
    previews);
  - a NEW event `fengShui` with M5-T4 and SOUND_CUES rows.
- **NEW: ME-LUCK** (#40; it also feeds #39.1's Lucky 2, and #36, #86, #97.2 and #97.8 in other groups).
  A player's Luck X is the sum of the "You have Luck X" auras acting on their side. It is added to the
  Lucky X of every roll that already has a "best" (`radiant_chance`, the Grapes' roll, Soul Shot's pick
  R414, Book of Tokens' count, Mid Loser's coin, Die Insect, ME-WEIGHTED-ROLL) made by an effect that
  player controls. A roll with no best (a pool pick, a random target) is untouched (R32,
  R130). Since MD-G4 (R1440) every coin flip has a best, heads, so Gary's coins and Ace in the Hole's
  take it too; whichever of #527 and #571 lands second proves that. The value is public, shown by the
  hero (MD-C12).
- **ME-MARKET** (#42). A subsystem `subsystems/night_market.rs` (MD-C13, MD-C14) with:
  - the stall roll (3 CN cards, 2 Auspicious Rocks, 1 AI generated card);
  - the yuan price function (`YUAN_PER_COST` 10, `YUAN_PER_RARITY` 5, Radiant ×2);
  - a NEW prompt kind `market` whose budget is the yuan left, offering lots, barters (Radiant face) and
    Leave, one deal per answer, reopened through `resume` (R113);
  - barter, which exiles a hand card for its price.

  The AI needs nothing new: the stall is fixed when the market opens, so `decide`'s enumeration and
  simulation answer it like any prompt.
- **ME-ALTPLAY** (#45, #45.1; #99 Paranoia in another group). It needs:
  - a flag on the `play` action, offered by `legalActions` while a permission card acts on the player's
    field;
  - the face-down placement as a Field Trap (R448, R227), with its own text dormant;
  - a trap trigger on its controller's `turnStarted` that turns it face-up, animates it (R383, R445)
    and runs its deferred Cry with prompts (R467), keeping its set turn as `summonedTurn` (MD-C16, MD-C17,
    MD-C19);
  - a change to `determinize` (R185, R762) so it samples Units for a hidden backrow card while a
    permission is on that seat's side.
- **NEW: ME-WEIGHTED-ROLL** (#39.1). `add_rolled_grapes` (`effects/fruit.rs`) made general: a weighted
  table with an order from config (`AUSPICIOUS_ROCK_ODDS`, Dud < Jade < Red Jade), Lucky-aware (the
  printed Lucky plus ME-LUCK), adding the result to a hand (MD-C1).
- **NEW: ME-PICK-N** (#49.1; #95's Radiant "three effects" in another group). Call to Chaos's Radiant
  draw (R423) made into a verb: N different entries of a list (R60), drawn from the match rng, resolved
  in list order and named to both players by a `chaosRolled`-style event (R436). When N reaches the
  list's length, it does every entry (MD-C23).
- **NEW: ME-AFTER-ATTACKED** (#49.3). `Script.after_attacked`, the defender's mirror of R426's
  `after_attack`. It runs for the attacked unit once per combat it was the target of, declared or forced,
  after that combat's state check, on its snapshot, with `{ attackerId, forced }` (MD-C26).
- **NEW: ME-HERO-IMMUNE** (#48). A player-modifier kind that `hero_damage_cap` reads as a cap of 0 at §4.4
  step 3, with expiry `nextTurnOf` the opponent (R757). Lose health and Set health are untouched
  (MD-C21).
- **NEW: ME-DECK-BOTTOM** (#49). `shuffle_into` gains `position: bottom`, the `LibraryPosition::Bottom`
  that `zones.rs` already has, so a fresh card is put on the bottom of a deck under R80's cap.
- **NEW: ME-FUSE-RANDOM** (#47). `fuse_cards` gains a random ingredient, drawn inside `apply` from a
  scope (enemy permanents on the field, tops only, Immutable excluded) or from the opponent's library,
  fused into a kept target per R77 and R102 (MD-C20).
- **Existing systems these cards compose (no change):**
  - E10 End the turn (R456): #48.
  - The aimed summon for the opponent (§3.2, R47, R12): #49.
  - `fill_board` for the enemy (§7): #49.1.
  - `degrade` with an any-of `CardScope` (R386, R440): #45, #49.1.
  - `set_radiant_random` (R60, R242): #49.1.
  - Hand and deck triggers (R464) with `set_cost_mod` (R766): #49.1.
  - `catalog.pool` (R380, R382, R387): #41, #42, #45.1, #46.
  - Activate (R384): #49.2.
  - `forced_attacks_on` (R53): #49.3.
  - Animated (R383, R445): #49.3, and ME-ALTPLAY.
  - Brittle given counts (R385, R638): #40.
  - Radiant at §10.5 step 3 (R213, R214, R449): #40.
  - Fuse (R77, R102, R468): #47.
  - `delay` with watch and mark (R76, R174, R437) and `steal` (R15, R171): #39.5.
  - Embiggen's Paid price (§2.3): #46.
  - X (R348): #43.

### Group D: Meditative #50–#75, #62, and their tokens

Researched against `spec/05`–`spec/10`, `crates/cards/README.md`, `crates/engine/src/effects/*`, `script.rs`,
`subsystems/` (`scorer.rs`, `ai_policy.rs`, `activate.rs`, `fuse.rs`), `animated.rs`, `catalog.rs`,
`wire/emotes.rs`, `wire/stats.rs`, `crates/server/src/actor/match_actor.rs` and `crates/server/src/api/stats.rs`.
"Buff" means one Upgrade application (`upgrade`, R386) and "Nerf" one Degrade application, as the set decided.

---

#### Meditative #50 · CN Tech
`meditative-050` · (0) Spell, CN · Rare

> **Designer:** Add the highest win rate card to your hand ~~~ Add the highest win rate Radiant card to
> your hand

- **Text:** Add the card with the highest win rate to your hand.
- **Radiant:** Add the card with the highest win rate to your hand. Make it Radiant.
- **Engine:** ME-STATS. The table is `crates/cards/data/win_rates.json`, compiled in by `crates/cards/build.rs`
  next to the catalog and handed to the engine inside the catalog's own `OnceLock`, so rule 4 still has
  two statics. It holds `{ patch, source: "live" | "provisional", cards: [{ id, wins, games }] }`.
  Its figures are R377's "in deck" breakdown for every mode and every pilot (`unified`), with tutorial
  games left out. That is the figure `GET /api/stats/cards` ranks by, taken for the newest *shipped*
  patch under R654's gate: live games only once the patch has `PUBLIC_STATS_MIN_LIVE_GAMES`, otherwise
  live plus dev. This way CN Tech agrees with `/stats`.
  **Refresh:** `cargo jackioh winrates <saved stats response | record files…>` (crates/tools, which may
  do I/O) rewrites the table. It is run by hand once per card patch, the way `sweep` feeds
  `shadow_ban.rs`. The diff is a balance change, so the patch's pending fragment claims it (R388, R646),
  `patches check` treats the table as catalog data, and the server and the WASM client always read the
  same compiled table.
  **Pick:** the cards with at least `CN_TECH_MIN_GAMES` (20, engine `config.rs`) games, excluding
  CN Tech itself (R387), Glitch and any id the catalog no longer holds. The order is the rate compared
  exactly (`wins_a × games_b` against `wins_b × games_a`), then more games, then catalog id. The top
  card is added as a fresh instance; nothing is random. If no card qualifies (a new patch, an empty
  table), it adds a random non-token card of every set instead (R380, match rng).
  **Hidden:** the added card is hidden from the opponent like any hand add, although the public table
  lets them infer it.
- **Rulings:** **MD-D1** (above: what the table holds, its threshold, tie order and fallback).
  **MD-D2:** the table changes only with a card patch, never mid-patch. So a game's `(seed, log)` replays
  on the build that played it, as for any card change. The client may name the table's leader in CN
  Tech's inspect overlay, read from the same compiled table through the WASM module and never from
  `/api/stats`.
- **Numbers:** none.
- **Class:** C (ME-STATS).

#### Meditative #51 · Devin Bot
`meditative-051` · (2) Unit · Legendary · 1/1 → 2/2

> **Designer:** 1/1 · Reborn · Cry: Fill your board with random 1/1s. {for AI: there are cards with a
> 1/1 stat line, including Tokens} ~~~ 2/2 · Reborn · Cry: Fill your board with random Radiant 1/1s.
> {for AI: there are cards with a 1/1 stat line, including Tokens}

- **Text:** Reborn
  Cry: Fill your board with random 1/1 Units.
- **Radiant:** Reborn
  Cry: Fill your board with random Radiant 1/1 Units.
- **Engine:** §7's "fill your board": every empty, unlocked unit zone, left to right (R64), each gets
  its own pick from the match rng, and repeats are allowed (R60). The pool is the Units whose printed
  *base* face is 1/1, tokens included as the designer says (Sheep Token, Felinor Token, Right-house
  defender, Joro, Wrong-House Attacker, Keymaster Keenus, Sentient Cat Ears and others; 13 cards today
  plus Meditative's). Devin Bot itself is left out (R387).
  Missing: **NEW: catalog query `stats`** (filter on printed base attack and health) and **NEW:
  `fill_board_random`** (fill_board where each zone draws from a pool; nothing is drawn when no zone is
  open, R129). The query is `{ type: Unit, stats: {1,1}, withTokens: true }`. Summons fire no Cry (R1).
  Reborn's return does not fire the Cry again. Radiant: each summon is Radiant.
- **Rulings:** **MD-D3:** "random 1/1s" are Units whose printed base face is 1/1, tokens included,
  with one independent pick per open zone.
- **Numbers:** none.
- **Class:** B (two small NEW pieces: the query field and the verb).

#### Meditative #52 · Economic Anxiety
`meditative-052` · (1) Field Spell · Rare

> **Designer:** Units gain +3 Attack when attacking a Unit that doesn’t share one of their tags. {for
> AI: highlight valid targets in yellow when dragging for the attack} ~~~ Your Units gain +3 Attack
> and Poisonous when attacking a Unit that doesn’t share one of their tags. {for AI: highlight valid
> targets in yellow when dragging for the attack}

- **Text:** Aura: Units have +{bonus} Attack while attacking a Unit that shares none of their tags.
- **Radiant:** Aura: Your Units have +{bonus} Attack and Poisonous while attacking a Unit that shares
  none of their tags.
- **Engine:** **NEW: conditional attack modifiers.** A new `Script.attack_mods` aura hook,
  `(source, attacker, defender) → { attack, keywords }`. It is read only inside a combat in which the
  attacker attacks a Unit (declared or forced). It adds to that attacker's strike in §4.3 step 1 or 2,
  in Cleave (step 10) and in Trample (step 9). The Radiant face's Poisonous counts at §4.4 step 7 for
  those hits.
  It is never a layer stat: the printed attack shown and §4.2 step 1's "attack above 0" do not see it.
  The defender's strike back never gets it, and neither does an attack on a hero. My Pawn's lethal
  projection (`lethal.rs`) and the scorer's dry run read it through the same function. The base face
  reaches both sides' Units; the Radiant face reaches yours.
  **The yellow highlight (R195's sibling):** a NEW view field `conditionTargets: [instanceId]` on each of
  the viewer's *own* units that may attack now. It lists the enemy units on which an `attack_mods` entry
  would apply, computed by the same predicate, and it is absent otherwise (never on the opponent's
  cards). While a unit is dragged or click-selected to attack, `apps/web/src/game/drag/targets.ts`
  puts `data-condition-active` on those targets beside the green legal glow (`glow.ts`). Tags are
  public, so the field reveals nothing.
- **Rulings:** **MD-D4:** "when attacking" means the attacker's own strikes in a combat it started
  against a Unit. "Shares none of their tags" compares every printed tag (Token included), so an untagged
  attacker always qualifies. **MD-D5:** `conditionTargets` is the engine's (CLAUDE.md rule 7). It sits
  only on the viewer's units, during the viewer's main phase with no prompt open, as R195's flag does.
- **Numbers:** bonus 3 → 3 ↑.
- **Check:** R275: the Radiant face narrows the aura to your side and adds Poisonous. That is a broader
  choice plus a rider, so it passes.
- **Class:** C (NEW: conditional attack modifiers and `conditionTargets`).

#### Meditative #53 · Prestige
`meditative-053` · (1) Spell · Common

> **Designer:** De-Radiant a card. Add an AI-Generated card to your hand. {for AI: if no valid targets,
> can be played for just the AI-Generated card} ~~~ De-Radiant all enemy cards. Add a Radiant
> AI-Generated card to your hand. {for AI: if no valid targets, can be played for just the AI-Generated
> card}

- **Text:** De-Radiant a card. Add {cards|random AI generated card|random AI generated cards} to your
  hand.
- **Radiant:** De-Radiant every enemy card. Add {cards|random Radiant AI generated card|random Radiant
  AI generated cards} to your hand.
- **Engine:** **NEW: De-Radiant.** It is the inverse of §5.2's Make Radiant. Today "nothing in any set
  un-sets it", and this verb is the first exception. It clears `radiant` and does nothing to a
  non-Radiant card.
  In a hand or deck, the face swaps back. On the field, the base-stat layer swaps back; damage, buffs,
  `tuning` and granted keywords stay; keywords that only the Radiant face printed are lost; ongoing
  triggers read the base text from then on; and no Cry fires. A fused card returns to its fused base
  form (R77). The event is NEW `deradianted { instanceId, defId, zone }`, hidden per zone like
  `radiantSet` (R440), and it needs an `ANIMATIONS` row and a `SOUND_CUES` row.
  Base: a declared target (R81) that is a Radiant Unit or face-up backrow card on either side, or a
  Radiant card in your hand. The target is optional when no card qualifies.
  Radiant: every Radiant card your opponent has on the field (face-down included), in their hand and in
  their deck.
  Then a random pick from the ten AI generated cards (`{ tags: [AI], withTokens: true }`), Radiant on the
  Radiant face. A full hand burns it (§2.4).
- **Rulings:** **MD-D6:** De-Radiant's semantics are as described above. **MD-D7:** Prestige can be
  played with no target when nothing qualifies, and still adds its card; with any target it must take one.
  **MD-D8:** "all enemy cards" means the opponent's field, hand and deck, not their graveyard or exile.
  **MD-D11** (the AI pool) applies.
- **Numbers:** cards 1 → 1 ↑.
- **Check:** De-Radiant reverses §5.2's "nothing un-sets it", so §5.2 needs the exception written in.
- **Class:** C (NEW: De-Radiant).

#### Meditative #54 · Money Machine
`meditative-054` · (1) Field Spell · Common

> **Designer:** End of Turn: Shuffle a Coin into each player’s deck. · Activate: Spend 2 mana to
> Tribute this. Either player may Activate this. {for AI: your “Coin” has a sneaky 10% chance to be
> Jade and a 1% chance to be Red Jade} ~~~ End of Turn: Shuffle a Coin into your opponent’s deck and a
> Radiant Coin into yours. · Activate: Spend 2 mana to Tribute this. Either player may Activate this.
> {for AI: your Radiant  “Coin” has a sneaky 10% chance to be Radiant Jade and a 1% chance to be
> Radiant Red Jade}

- **Text:** End of turn: Shuffle The Coin into each player's deck.
  Activate: Spend ({price}). Tribute this. Either player may activate this.
- **Radiant:** End of turn: Shuffle The Coin into your opponent's deck and a Radiant The Coin into
  yours.
  Activate: Spend ({price}). Tribute this. Either player may activate this.
- **Engine:** At its controller's end of turn, it makes two fresh shuffles (`core-t-coin`; R80's cap
  turns a full library away).
  **The sneaky roll** applies only to the card going into its *controller's* deck. One match-rng draw
  against `MONEY_MACHINE_ODDS` (89 Coin, 10 Jade `meditative-039-2`, 1 Red Jade `meditative-039-4`, in
  engine `config.rs`) decides it, Radiant on the Radiant face. It is unprinted, as R673's Glitch roll is.
  Both shuffles go in *unseen* (R311's unknown card in each owner's list), so nothing tells a Jade from a
  Coin before the draw.
  **NEW: either-player Activate**: `ActivationDecl.by: Either`. The opponent may activate it in their own
  main phase (R384's other conditions hold), paying the mana from their own pool. The `activated` event
  names the activator. The cost is `{ mana: price, tributeSelf: true }`, the card's own Sacrifice (a
  Death would fire). `legal_actions` lists it for both seats.
  Depends on ME-JADE for the two tokens.
- **Rulings:** **MD-D9:** "Either player may activate this": the non-controller activates in their own
  main phase and pays from their own mana. Uses are counted on the card as usual. **MD-D10:** the
  89/10/1 roll is hidden data: unprinted, documented in §8.8's Engine column and `config.rs`, and both
  Coins are shuffled in unseen.
- **Numbers:** price 2 → 2 ↑ (a higher price protects the Machine, which favours its controller,
  especially on the Radiant face).
- **Check:** R275 is met: the controller's Coin doubles (1 mana to 2) and the hidden roll goes Radiant.
  Typo: the designer's Radiant line has a double space before "Coin".
- **Class:** C (NEW: either-player Activate; ME-JADE).

#### Meditative #55 · Dragon Fruit
`meditative-055` · (2) Spell, CN, Fruit · Rare

> **Designer:** Add a random Prime card to your hand ~~~ Add a random Radiant Prime card to your hand

- **Text:** Add {cards|random Prime card|random Prime cards} to your hand.
- **Radiant:** Add {cards|random Radiant Prime card|random Radiant Prime cards} to your hand.
- **Engine:** `add_random_from_catalog({ tags: [Prime], withTokens: true })`. Every Prime card is a
  token: C+ #38.1, C+ #46.1, Meditative #45.1 and #91.1. Repeats are allowed (R60). A full hand burns
  the card. As a Fruit card, Dragon Fruit joins every Fruit pool (R382). It can never generate itself,
  because it is not Prime.
- **Rulings:** **MD-D11:** a pool named by the Prime tag, or by "AI generated card", holds those tokens,
  extending R382 beyond Fruit (add `Prime` and `AI` to `POOL_TOKEN_TAGS`, or pass `withTokens`).
- **Numbers:** cards 1 → 1 ↑.
- **Check:** Rarity was not given. It takes **Rare**: a single generation from a named pool, as its
  nearest Fruit sibling C+ #58 Fruit Basket is.
- **Class:** A.

#### Meditative #56 · House Party
`meditative-056` · (3) Spell · Common

> **Designer:** Fill your board with Right-House Defenders ~~~ Fill your board with Radiant Right-House
> Defenders

- **Text:** Fill your board with Right-house defenders.
- **Radiant:** Fill your board with Radiant Right-house defenders.
- **Engine:** `fill_board({ defId: "core-003", radiant? })`: every empty, unlocked unit zone, left to
  right (§7, R64). There is no Cry; Core #3 has none. The Radiant defenders carry "Death: summon a base
  Right-house defender".
- **Numbers:** none.
- **Check:** Rarity was not given. It takes **Common**: the same shape as Core #62 Friend of Felinors
  (Common), and the card it summons belongs to the Common Right-house defender family. The name is the
  catalog's spelling, "Right-house defender" (R279's refs).
- **Class:** A.

#### Meditative #57 · Clip-Farming Critikal
`meditative-057` · (1) Unit · Common · 5/1 → 10/2

> **Designer:** [5/1] · Combo: add the previous card you played back to your hand ~~~ [10/2] · Combo:
> add the previous card you played back to your hand, make it Radiant

- **Text:** Combo: Return the card you played before this to your hand.
- **Radiant:** Combo: Return the card you played before this to your hand. Make it Radiant.
- **Engine:** Combo 1 (§6.2) as its Cry. "The previous card" is `played_ids_this_turn` at the index
  just before this play (`played_earlier(self) - 1`). Casts count, since they are plays (R70).
  `find_instance` finds the card wherever it is (R98):
  - in your graveyard or exile, it moves to your hand ("Return … to hand", R746);
  - as a permanent you control, it is Bounced (R692, R747), and a unit token vanishes;
  - anywhere else (a hand, a deck, ceased to exist, controlled by the opponent), nothing happens.
  A full hand burns it. Radiant: then `set_radiant` on it, in hand. Hidden: nothing new.
- **Rulings:** **MD-D12:** "the previous card you played" is the play before this one in this turn's
  log. It comes back from your graveyard or exile, or is Bounced from your side of the field; otherwise
  nothing.
- **Numbers:** none.
- **Class:** B.

#### Meditative #58 · Permanent Underclassman
`meditative-058` · (2) Unit, Human · Common · 3/3 → 6/6

> **Designer:** 3/3 · Cry: Summon a Random (1) cost Unit with “Death: Add a Book to your hand.” ~~~ 6/6 ·
> Cry: Summon a Random (1) cost Radiant Unit with Death: Add a Radiant Book to your hand.

- **Text:** Cry: Summon a random (1) Cost Unit. It gains "Death: Add {books|random Book|random Books}
  to your hand."
- **Radiant:** Cry: Summon a random Radiant (1) Cost Unit. It gains "Death: Add {books|random Radiant
  Book|random Radiant Books} to your hand."
- **Engine:** First, `summon_random({ type: Unit, cost: 1 })`: every set (R380), cost read out of play
  (R65), match rng, no Cry (R1), and no zone means no draw (R129). The summoned id comes back in the memo.
  Then **NEW: granted abilities** (ME-GRANT) put a plain-data grant on that instance:
  `{ grant: "meditative-058#bookDeath", radiant }`. It names a hook the granting card registers beside
  its faces (`CardScripts.grants`), so state stays JSON and replays exactly (closures never enter state,
  as `fuse.rs` explains).
  §4.5 step 3 runs a dying unit's granted Deaths after its own Death, from the last-known state (R78),
  under the dying unit's *controller*. The Book is random over the non-token Books of every set. The
  client prints the grant's text on the unit (the catalog entry carries `grants: { bookDeath: "…" }`).
- **Rulings:** **MD-D13:** a granted ability behaves like a granted keyword (§10.4). A Vanilla and a
  copy (R57) keep it; leaving the field (R78) and a Transform lose it; it runs for whoever controls the
  unit when it dies.
- **Numbers:** books 1 → 1 ↑.
- **Class:** C (NEW: ME-GRANT).

#### Meditative #59 · Permanent Upperclassman
`meditative-059` · (3) Unit, Human · Rare · 3/3 → 6/6

> **Designer:** 3/3 · Cry: Summon a Random (4) cost Unit with “Death: Add a Book Fused with an
> AI-Generated card to your hand.” ~~~ 3/3 · Cry: Summon a Random (4) cost Radiant Unit with “Death: Add
> a Radiant Book Fused with a Radiant AI-Generated card to your hand.”

- **Text:** Cry: Summon a random (4) Cost Unit. It gains "Death: Fuse a random Book with a random AI
  generated card and add the result to your hand."
- **Radiant:** Cry: Summon a random Radiant (4) Cost Unit. It gains "Death: Fuse a random Radiant Book
  with a random Radiant AI generated card and add the result to your hand."
- **Engine:** It works as #58 does, with a (4) Cost pool and the grant `meditative-059#fusedBookDeath`.
  The Death fuses with no target (R77, R102). The ingredients are one random non-token Book and one of
  the ten AI generated cards (MD-D11). The type is the shared one, else the Book's (a Spell). The result
  is not a Token, since the Book isn't one, and it goes to the hand at R77's fused cost, min(sum, 4), as
  Stitching's result does (R352). A full hand burns it. On the Radiant face both ingredients are Radiant,
  so the result is Radiant (R469).
  Missing: **NEW: `fuse_generated` with one pick from each of several pools** (today it takes `count`
  picks from one pool).
- **Rulings:** **MD-D14:** "a Book fused with an AI generated card" is one pick from each pool, fused
  with no target, at the fused cost.
- **Numbers:** none.
- **Check:** R275: the designer's Radiant face keeps 3/3. The fix is **6/6**. **⚠ designer.**
- **Class:** C (ME-GRANT, NEW: multi-pool `fuse_generated`).

#### Meditative #60 · Eschews
`meditative-060` · (3) Field Spell · Epic

> **Designer:** End of Turn: Cast a random Human, Book, CN, or AI-Generated card. · Tribute this after
> 5 ally Units have died. ~~~ End of Turn: Cast a random Radiant Human, Book, CN, or AI-Generated card.
> · Tribute this after 50 ally Units have died.

- **Text:** End of turn: Cast {casts|random Human, Book, CN or AI generated card|random Human, Book, CN
  or AI generated cards}.
  Once {deaths} of your Units have died, Tribute this.
- **Radiant:** End of turn: Cast {casts|random Radiant Human, Book, CN or AI generated card|random
  Radiant Human, Book, CN or AI generated cards}.
  Once {deaths} of your Units have died, Tribute this.
- **Engine:** `cast_random` (E12, R70, R452). The pool is the non-token cards tagged Human, Book or CN,
  plus the ten AI generated cards (MD-D11), each definition once, excluding Eschews (R387).
  Missing: **NEW: catalog query `anyTags`** (`tags` today means "has every listed tag", and a script may
  not merge pools itself).
  In a random cast every choice is random and X is the caster's current mana, at least 1. A Unit is
  summoned and fires its Cry. A Field Spell or Trap needs a backrow zone, else it fizzles to the
  graveyard. The cast counts as your play.
  A trigger on `destroyed` of your Units (tokens included; each death of a Reborn unit counts) adds to
  `memory.deaths`, which R78 resets. `tribute_when` (C #88, R403) sacrifices it at the state check once
  the count reaches `{deaths}`.
- **Rulings:** **MD-D15:** the deaths are counted since this entered the field; it is Tributed at the
  next state check once they reach the number.
- **Numbers:** casts 1 → 1 ↑; deaths 5 → 50 ↑ (step 1 on the base face; a quarter, 12, on the Radiant).
- **Check:** R275: Radiant casts plus a tenfold life span, so it passes.
- **Class:** B (NEW: `anyTags`).

#### Meditative #61 · Joon Jorker
`meditative-061` · (3) Unit, Human · Common · 7/5 → 14/10

> **Designer:** 7/5 · End of Turn: Buff the Unit to the right 5 times. ~~~ 14/10 · End of Turn: Buff the
> Unit to the right 10 times.

- **Text:** End of turn: Buff the Unit to the right of this {times} times.
- **Radiant:** End of turn: Buff the Unit to the right of this {times} times.
- **Engine:** It runs at its controller's end of turn: `upgrade({ instanceId, times })` (R386, one draw
  per application) on the acting Unit in lane +1 on its own side, using §3's lanes (1 to 5 from the
  owner's seat; `zones::adjacent`). With none there, or an Immutable one (R23), nothing happens and
  nothing is drawn (R129). The events are `upgraded`.
- **Rulings:** **MD-D16:** "the Unit to the right" is the acting Unit in the next lane on its
  controller's side, as that player sees the board.
- **Numbers:** times 5 → 10 ↑.
- **Class:** B.

#### Meditative #62 · Small Time Recruits
`meditative-062` · (0) Spell · Rare

> **Designer:** (0) Small Time Recruits · Something with 1 costs

- **Text:** Draw {cards|(1) Cost Unit|(1) Cost Units} from your deck.
- **Radiant:** Draw {cards|(1) Cost Unit|(1) Cost Units} from your deck. Make them Radiant.
- **Engine:** This follows Hearthstone's Small-Time Recruits ("Draw three 1-Cost minions from your
  deck"). The cards are different random cards (R60, match rng) among your deck's Units whose cost *as
  it stands in the deck* is 1 (`effective_cost`, R65/R66; X-cost cards are excluded, as C #94 Genn's
  Greed does). It takes fewer if fewer exist. Each is drawn with `draw_from_library` (§6.3 Draw:
  hand cap, cast on draw, the draw counter). The set is read once (`for_each_card`, R113).
  Radiant: each drawn card still in your hand afterwards is made Radiant. Hidden: the draws are hidden
  from the opponent as any draw is.
- **Rulings:** **MD-D17:** three different random (1) Cost Units, the cost read as it stands in the
  deck, drawn as ordinary draws.
- **Numbers:** cards 3 → 3 ↑.
- **Check:** The designer gave only a stub; this is the full card. Rarity was not given. It takes
  **Rare**: a filtered draw is "one idea with a twist", as Core #30 Archivist (Rare) is. R275: three
  Radiant Units, so each card is doubled.
- **Class:** B.

#### Meditative #63 · Skull of J’Nari
`meditative-063` · (3) Field Spell · Legendary

> **Designer:** Start of Turn: Summon a random Unit from your hand. ~~~ Start of Turn: Summon a random
> Unit from your hand. Make it Radiant. · {for AI: multiple activation voicelines: “This one will prove
> very useful! ...Probably.”, “there IS a method to my madness!”, “Good units are hard to find.”}

- **Text:** Start of turn: Summon {units|random Unit|random Units} from your hand.
- **Radiant:** Start of turn: Summon {units|random Unit|random Units} from your hand. Make it Radiant.
- **Engine:** It runs at its controller's start of turn (R62). It picks a random Unit card in your hand
  (match rng, unit-token cards included) and summons it straight to the leftmost open unit zone (§6.2's
  "summon this from your hand": no Cry, R1; summoning sick). With no open zone, nothing is drawn
  (R129). The opponent sees nothing until it lands, so the hidden pick is cued per R177. An X-stat card
  (C+ #69) arrives with X 0 and dies.
  Missing: **NEW: `summon_random_from_hand`** (a small verb: today `summon` moves a named instance).
  Radiant: Make Radiant once it is on the field.
  **Voice:** a NEW `card-audio.json5` hook `trigger` holding a list of lines. The client's own cosmetic
  rng picks one each time, never `state.rng`, as R645's personas draw. It is spoken when the runner
  starts the `summoned` entry this card caused, which needs a NEW optional `sourceId` on `summoned`.
- **Rulings:** **MD-D18:** the pick is over your hand's Unit cards and is drawn only when a zone is open.
- **Numbers:** units 1 → 1 ↑.
- **Class:** B (NEW: the verb, the voice hook).

#### Meditative #64 · Traitorous Blood
`meditative-064` · (1) Trap · Common

> **Designer:** Reveals when an opposing Unit attacks and has a neighbor: The attack is redirected to
> the neighbor. ~~~ Reveals when an opposing Unit attacks and has a neighbor: The attack is redirected to
> the neighbor. Summon a copy of any of the Units destroyed.

- **Text:** Reveals when an enemy Unit with an adjacent Unit attacks: Redirect the attack to that
  adjacent Unit.
- **Radiant:** Reveals when an enemy Unit with an adjacent Unit attacks: Redirect the attack to that
  adjacent Unit. Then summon a copy of each Unit that combat destroyed.
- **Engine:** The trap fires in §4.2 step 4's window on a *declared* attack (a forced one opens no
  window, R121) by an enemy unit that has an adjacent acting Unit on its own side (§3.1), whatever its
  target.
  Missing: **NEW: attack redirect.** §6.3 Redirect already describes "a declared attack re-aimed at
  another unit keeps its source and its declaration", but only the Spell-target half exists
  (`targeting_point.rs`). The attack half sets `declared_attack.target_id` inside the window, like
  `cancel_attack` (R44), and emits `redirected { what: attack }`. §4.3 then resolves a combat between
  allies: the neighbour strikes back, and the death and kill rules hold as usual.
  The neighbour must be one the attacker may attack under §4.2 step 2's restrictions. With two, one is
  picked at random (match rng). Radiant: once that combat's state check has run, the trap's controller
  gets a fresh copy, by definition and face (R409's reading), of each Unit it destroyed. Tokens are
  included, until the board is full.
- **Rulings:** **MD-D19:** declared attacks only. The neighbour is an adjacent acting Unit that the
  attacker could legally attack; if there are two, one at random; Taunt is ignored. **MD-D20:** "any of
  the Units destroyed" means each Unit that combat destroyed, copied for you as Frostspatula copies.
- **Numbers:** none.
- **Check:** "neighbor" is written as "adjacent" (§3.1's word).
- **Class:** C (NEW: attack redirect).

#### Meditative #65 · Keymaster Keenus
`meditative-065` · (4) Unit · Legendary · 1/1 → 2/2

> **Designer:** 1/1 · Has every Keyword. ~~~ 2/2 · Has every Keyword. · Death: Give them all to another
> one of your Units.

- **Text:** Taunt, Armor 1, Rush, Charge, First Strike, Poisonous, Lifesteal, Reborn, Divine Shield,
  Trample, Cleave, Pierce, Indestructible, Immutable, Stack, Lucky 1, Spell Damage +1, Immune to Spells,
  Windfury, Deft, Magnetic
  This has every keyword.
- **Radiant:** (the same keywords)
  This has every keyword.
  Death: Give every keyword this has to another random Unit of yours.
- **Engine:** The 21 are printed `keywords`: every engine `KeywordKind` plus Meditative's Magnetic.
  Five are left out:
  - Can't attack, Brittle and Temporary, which only hurt;
  - Animated and Animated on your turn, which mean something only on a backrow card.
  Static restriction texts (Can't be attacked and the like) and the card keywords Cast on draw and
  Quickdraw are not keywords in this sense either; Cast on draw would also clash with Quickdraw (R635).
  R347 drops its Taunt while it is Indestructible. Its Death fires only through a Sacrifice or Tribute
  (Indestructible) or after an exile replacement fails. Numbered keywords move under Nerf and Buff's X
  row (R386). Magnetic and Stack are two distinct play options (ME-MAGNETIC, MD-D24). Radiant Death: the
  keywords it had as it died (LKI, R78) go to a random other Unit you control as granted keywords
  (`grant_keyword`), and Reborn's return fires it again on the second death.
- **Rulings:** **MD-D21:** the list above is the card's printed data, not "whatever the engine knows". A
  later keyword joins only by a patch. **MD-D22:** the Radiant Death's receiver is random (Hearthstone's
  Deathrattle), and the keywords are granted (§10.4).
- **Numbers:** none (Armor, Lucky and Spell Damage are numbered keywords, R386).
- **Class:** B (Magnetic comes from ME-MAGNETIC).

#### Meditative #66 · Fiery Waraxe
`meditative-066` · (2) Field Spell · Common · 3/2 → 6/4

> **Designer:** 3/2 · Animated on your turn ~~~ 6/4 · Animated on your turn · Pierce

- **Text:** Animated on your turn
- **Radiant:** Animated on your turn, Pierce
- **Engine:** Animated on your turn (§6.1, R383, as C+ #12.8 Frostspatula): it is a Unit from its
  controller's start of turn to their cleanup, otherwise in its reserved backrow zone. **Problem:**
  R383 makes the card summoning sick every time it animates, so with no Rush or Charge the Waraxe could
  never attack, and it is never on the field on the opponent's turn. MD-D23 fixes this.
- **Rulings:** **MD-D23 (revises R383):** an "Animated on your turn" card that was already on its
  controller's field when their turn began is not summoning sick when it animates at that start of
  turn. Only the animation as it enters the field (its first turn) is an entry for §4.1.
- **Numbers:** none.
- **Check:** MD-D23 also lets C+ #12.8 Frostspatula (Rush) attack heroes from its second turn, and #71.1
  The Cane attack on your turn. **⚠ designer** if instead the Waraxe should print Charge and R383 stay
  as it is.
- **Class:** A (plus the MD-D23 ruling).

#### Meditative #67 · Sentient Cat Ears
`meditative-067` · (1) Unit, Felinor · Epic · 1/1 → 2/2

> **Designer:** 1/1 · Magnetic {For AI: can optionally Stack on to one of your Units. If it does, Fuse
> with it} · End of Turn: Shuffle a Love Bomb into your Deck. ~~~ 2/2 · Magnetic · End of Turn: Shuffle 2
> Love Bombs into your Deck.

- **Text:** Magnetic
  End of turn: Shuffle {bombs|Love Bomb|Love Bombs} into your deck.
- **Radiant:** Magnetic
  End of turn: Shuffle {bombs|Love Bomb|Love Bombs} into your deck.
- **Engine:** **ME-MAGNETIC.** A Magnetic Unit may be played onto the zone of one of your acting,
  non-Immutable Units (R23), chosen in the `play` action (`magnetic: true` beside `zone`, R81; a card
  with Stack too has both options in `legal_actions`).
  It stands on top of that Unit while its own play resolves, its Cry included. This is R653's reading of
  Hearthstone's Magnetic, using Ivory Tower's machinery with a Unit host; the host lies dormant beneath
  it meanwhile (R13). On its `cardResolved`, the card on top is fused into the host (R77, R102, R653).
  The host is the kept instance (zone, damage, exertion, `summonedTurn`). The result sums stats and
  buffs, unions keywords and tags (the host becomes a Felinor), joins texts, and costs min(sum, 4). The
  Magnetic card ceases to exist with no Death.
  If either card left before the fusion, nothing is fused, and the top card, if still there, stays as
  the pile's top. The play counts as a play (Combo, R70 counters). With the host's text now holding it,
  "End of turn: Shuffle a Love Bomb" (`meditative-030-1`, a fresh copy, R80's cap) runs for the host.
- **Rulings:** **MD-D24:** Magnetic plays onto your own acting non-Immutable Unit, resolves in full on
  top of it, then fuses with the host kept. It is a separate play option from Stack.
- **Numbers:** bombs 1 → 2 ↑.
- **Class:** C (ME-MAGNETIC).

#### Meditative #68 · Catnip
`meditative-068` · (1) Field Trap, Felinor · Rare · 5/5 → 10/10

> **Designer:** 5/5 · Animated · Reveal after an enemy casts a Spell. ~~~ 10/10 · Animated · Taunt ·
> Reveal after an enemy casts a Spell.

- **Text:** Animated
  Reveals after your opponent plays a Spell.
- **Radiant:** Animated, Taunt
  Reveals after your opponent plays a Spell.
- **Engine:** An Animated Field Trap (§6.1, R383, as C #5 Tesla). It fires once a Spell (the Spell
  type) the opponent played or cast has resolved (R17), on either turn, and as its last step animates
  into its lane's unit zone, else the leftmost open one. A later firing while it is a Unit does
  nothing. With no open zone it stays face-up and can fire again. It is hidden until it fires (R33).
- **Rulings:** **MD-D25:** "casts a Spell" means plays or casts a Spell (C #4's reading), and Catnip
  answers after it resolves.
- **Numbers:** none.
- **Class:** A.

#### Meditative #69 · The Maestro
`meditative-069` · (4) Unit, Human, Plague · Rare · 4/4 → 8/8

> **Designer:** 4/4 · For every mana you spend, put a Plague Counter on this. · Activate: Spend 4 Plague
> Tokens. Exile a random card from each of the enemy Field, Hand, Deck, and GY.

- **Text:** Whenever you spend mana, place a Plague Counter on this for each mana spent.
  Activate: Remove {price} Plague Counters from this. Exile {exiles|random card|random cards} from each
  of your opponent's field, hand, deck and graveyard.
- **Radiant (proposed):** Whenever you spend mana, place a Plague Counter on this for each mana spent.
  Activate: Remove {price} Plague Counters from this. Exile {exiles|random card|random cards} from each
  of your opponent's field, hand, deck and graveyard. (`{exiles}` is 2 on this face.)
- **Engine:** **NEW: `manaSpent { player, amount, for: play | activate }` event.** It is emitted at the
  engine's two spend sites (`play_steps.rs` §10.5 step 2 and `activate.rs`'s cost) and answered by a
  trigger while the Maestro is on the field. It makes N placements of one counter each (E19, R689;
  Pestilent Slime's multiplier applies to each). Its own play's mana is spent before it lands, so it
  does not count.
  Activate (R384, once a turn): `can_activate` checks for at least `{price}` counters, and the run
  first removes them (`consume_plague`, as C #78 pays). Then, for each of four zones in this order, one
  match-rng pick: an acting enemy permanent (face-down included), a card in their hand, a card in their
  deck, a card in their graveyard. An empty zone is skipped. Exile is public, so the hand and deck cards
  are revealed as they land.
- **Rulings:** **MD-D26:** "mana you spend" is mana its controller pays for a play's price or an
  activation's price while it is on the field, one placement per mana. **MD-D27:** the four exiles are
  one random card per zone, skipping empty zones.
- **Numbers:** price 4 → 4 ↓ (min 1); exiles 1 → 2 ↑.
- **Check:** There is no Radiant face; the proposal is **8/8 and two exiles per zone**. **⚠ designer.**
  "Plague Tokens" are Plague Counters (R740). The tag **Plague** is added because §5 puts it on every
  card that uses Plague Counters; the designer gave Human only. **⚠ designer.**
- **Class:** C (NEW: `manaSpent`).

#### Meditative #70 · I’M WILL BE YOUR DOOM
`meditative-070` · (1) Unit · Common · 5/8 → 10/16

> **Designer:** 5/8 · Cry: Summon a “Ready… I’m” token for your enemy. ~~~ 10/16 · Cry: Summon a “Ready…
> I’m” token for your enemy and a Radiant “Ready… I’m” token for yourself. · {for AI: voice line is “I’M
> WILL BE YOUR DOOM”}

- **Text:** Cry: Summon a Ready… I'm for your opponent.
- **Radiant:** Cry: Summon a Ready… I'm for your opponent and a Radiant Ready… I'm for yourself.
- **Engine:** `summon({ defId: "meditative-070-1", player: opponent })`, the token summoned under the
  other seat as Lava Golem's base face is (R360): their leftmost open zone (R64), nothing if their board
  is full, and no Cry. Radiant: then the same summon on your own side, Radiant. Voice: the card's `play`
  line "I'M WILL BE YOUR DOOM" in `card-audio.json5`.
- **Rulings:** **MD-D35:** the enemy's token is placed and controlled as if they had summoned it, and
  the enemy owns it.
- **Numbers:** none.
- **Class:** B.

#### Meditative #70.1 · Ready… I’m
`meditative-070-1` · (1) Unit, Token · Token (printed Common) · 2/1 → 4/2

> **Designer:** 2/1 · Poisonous ~~~ 4/2 · Poisonous · Divine Shield · {for AI: voice line is “Ready… I’m”}

- **Text:** Poisonous
- **Radiant:** Poisonous, Divine Shield
- **Engine:** Keywords only. It is a unit token: it vanishes when it leaves the field (R11). Voice: a
  `play` line "Ready… I'm".
- **Numbers:** none.
- **Class:** A.

#### Meditative #71 · Pareto Optimality
`meditative-071` · (2) Field Spell · Mythic

> **Designer:** Cry: Summon The Cane. · Whenever the enemy plays a card that isn’t the AI optimal move,
> The Cane attacks a random enemy. ~~~ Cry: Summon Radiant The Cane. · Whenever the enemy emotes or
> plays a card that isn’t the AI optimal move, The Cane attacks a random enemy.

- **Text:** Cry: Summon The Cane.
  Whenever your opponent plays a card that isn't the AI's optimal move, The Cane attacks a random enemy.
- **Radiant:** Cry: Summon a Radiant The Cane.
  Whenever your opponent emotes or plays a card that isn't the AI's optimal move, The Cane attacks a
  random enemy.
- **Engine:** The Cry summons `meditative-071-1` into your leftmost open backrow zone (R64) and
  remembers its id (`memory.cane`).
  **ME-PARETO** is a NEW `subsystems/pareto.rs` and stays pure (rule 4): it never calls `crates/ai`.
  The judge is `scorer.rs`'s §10.7 scorer.
  - **Who:** while a card with the NEW static flag `judges_plays` acts on your field, `reduce` judges
    each `play` action the *opponent* makes. It judges the state *before* the play, ahead of §10.5
    step 1, and stores `state.play_judgement = { instanceId, optimal }`.
  - **Candidates:** every card the opponent could play now (hand, and graveyard under a permission).
  - **Scoring:** each candidate is scored from the opponent's own view. The base is
    `dry_run_base(state, opponent)`, so R222's stand-ins conceal your hand, both libraries and your
    face-down traps. A NEW `score_instance` dry-runs the real card at its own price, tuning and face,
    where `score_def` scores a fresh definition.
  - **Verdict:** the play is optimal when its card's score equals the top score, so ties share
    "optimal".
  - **Trigger:** Pareto's trigger on that play's `cardResolved` (R17) reads the judgement. A countered
    play was never played, so nothing happens. Casts (R70) are not decisions and are not judged.
  - **Cost:** at most `HAND_CAP × SCORER_DRY_RUN_PLAYS` dry runs, and only while a Pareto watches.
    Dry runs are not `reduce` calls, so `crates/ai`'s budgets don't count them, though its wall clock
    (`should_stop`) does.
  - **The attack:** see #71.1.
  **Emotes (Radiant): NEW: ME-EMOTE.** Today an emote is never an `ActionBody` (R643): the actor relays
  it outside `reduce` (`match_actor.rs`, `ClientMessage::Emote`). For a card to see one:
  - a NEW `ActionBody::Emote { emote: EmoteId }`, legal for either seat when no prompt or mulligan is
    open and the game is on, and *only* while the other seat controls an acting card with the NEW flag
    `hears_emotes`. `legal_actions` lists it then; otherwise `reduce` refuses it, so other games are
    unchanged;
  - its NEW public event `emoted { player, emote }`, which the Radiant face's trigger answers;
  - in the actor, an emote that passed R643's gate is relayed as now and, when `emote_heard(state,
    seat)`, also minted as that seat's `Emote` action through the normal reduce queue (log, replay
    hash). The actor refuses an `Emote` arriving on the action channel, so the wall-clock rate limit
    cannot be bypassed;
  - in hotseat and practice, the emote session sends the action when the emoter's view says
    `emotesHeard: true`, and the practice host submits the AI persona's emotes the same way (R645);
  - `AI_SKIPPED_ACTIONS` gains `Emote`, so the random policy and My Pawn never emote.
- **Rulings:** **MD-D28:** "the AI optimal move" is the engine scorer's best-scoring playable card
  for the opponent, judged from their own view on the pre-play state. Only `play` actions are judged,
  ties count as optimal, and the trigger fires after the play resolves. The verdict is public through
  the Cane's attack, which tells you only that a better-scored card was in their hand. The judge's
  choice is shown to no one. **MD-D29 (revises R643):** an emote reaches the engine, as the `Emote`
  action, exactly while a card hears it. Every other emote stays R643's cosmetic relay.
- **Numbers:** none.
- **Check:** The scorer rates Traps on printed data only (`may_act_now`), so setting a trap is usually
  "not optimal". That is inherent to the judge, and it is documented, not special-cased.
- **Class:** D (ME-PARETO, ME-EMOTE).

#### Meditative #71.1 · The Cane
`meditative-071-1` · (2) Field Spell, CN, Token · Token (printed Mythic) · 3/1 → 6/2

> **Designer:** 3/1 · Animated on your turn and when Pareto Optimal · Indestructible · Pierce ~~~ 6/2 ·
> Animated on your turn and when Pareto Optimal · Indestructible · Pierce · Trample

- **Text:** Animated on your turn and when Pareto Optimal, Indestructible, Pierce
- **Radiant:** Animated on your turn and when Pareto Optimal, Indestructible, Pierce, Trample
- **Engine:** "Animated on your turn" is R383's, with MD-D23 so it can attack from its second turn.
  Summoned on your turn, it animates as it enters.
  "And when Pareto Optimal" is part of ME-PARETO: when Pareto Optimality's trigger makes it attack, it
  goes through these steps:
  1. If it is in its backrow zone, it animates (R383's move, home reserved).
  2. It makes one forced attack (R53, `forced_attack_random`) on a random enemy it may attack, hero or
     unit. The target strikes back, but Indestructible takes nothing.
  3. Once that attack's state check has run, it returns to its home zone, unless it is now its
     controller's turn.
  With no open unit zone it cannot animate, and that attack does not happen. On your turn, while
  already a Unit, it just attacks. The Cane is the one this Pareto summoned (`memory.cane`); if that one
  is gone, nothing attacks. As a spell token, Bounced, it goes to hand like a real card (§7).
- **Rulings:** **MD-D30:** Pareto's attack animates the Cane for that one forced attack and returns it
  afterwards; no open zone means no attack, and "The Cane" is the one Pareto summoned.
- **Numbers:** none.
- **Class:** C (inside ME-PARETO).

#### Meditative #72 · The Banisher
`meditative-072` · (1) Unit · Common · 2/1 → 4/2

> **Designer:** 2/1 · When this damages a Unit, exile that Unit. ~~~ 4/2 · Cleave · When this damages a
> Unit, exile that Unit.

- **Text:** Whenever this damages a Unit, exile that Unit.
- **Radiant:** Cleave
  Whenever this damages a Unit, exile that Unit.
- **Engine:** **NEW: exile on damage.** It is a §4.4 step 7 rider beside Poisonous: when the source has
  it, the target is a Unit and 1 or more was dealt, the target is marked `marked_exiled`. §4.5 step 1
  exiles marked units before collecting deaths, so there is no Death, no Reborn and no `destroyed`
  (R461), even when the hit was lethal. Divine Shield, Indestructible and the zero rule stop it (no
  damage dealt). It applies to combat both ways and to any damage the unit deals. Radiant: each Cleave
  hit (step 10) is its own instance, so the adjacent units are exiled too. The Banisher itself dies in
  the trade as usual.
- **Rulings:** **MD-D31:** the exile is a mark collected by the next state check ahead of deaths, and a
  unit it exiles has not died.
- **Numbers:** none.
- **Class:** C (NEW: exile on damage).

#### Meditative #73 · Plate Packer
`meditative-073` · (2) Unit, Human · Common · 6/6 → 12/12

> **Designer:** 6/6 · Cry: Gain +1/+1 for each Armor on the Field, including Heroes. ~~~ 12/12 · Cry:
> Gain +2/+2 for each Armor on the Field, including Heroes. · {for AI: voice line is “305 is the new 225”}

- **Text:** Cry: Gain +{per}/+{per} for each Armor on the field, heroes' included.
- **Radiant:** Cry: Gain +{per}/+{per} for each Armor on the field, heroes' included.
- **Engine:** X is read as the Cry resolves: the sum of every acting Unit's Armor on both sides
  (through §10.4: printed, Defense +1 and auras; `armor_of(unit_view)`) plus both heroes' Armor
  (`hero_of().armor`). It then `buff`s itself +X×per/+X×per (permanent). Missing: **NEW read helper
  `armor_on_field(state)`** in `query.rs`. Voice: `play` line "305 is the new 225".
- **Rulings:** **MD-D32:** "each Armor" means each point of Armor on every Unit and both heroes.
- **Numbers:** per 1 → 2 ↑.
- **Class:** B.

#### Meditative #74 · Montaña Giant
`meditative-074` · (8) Unit · Epic · 8/8 → 16/16

> **Designer:** 8/8 · Costs (1) less for each card in your Hand. · End of Turn: Radiant this if your Hand
> is full at the end of your turn. ~~~ 16/16 · Costs (1) less for each card in your Hand. · End of Turn:
> +8/+8 this if your Hand is full at the end of your turn.

- **Text:** Costs ({discount}) less for each card in your hand.
  End of turn: If your hand is full, make this Radiant.
- **Radiant:** Costs ({discount}) less for each card in your hand.
  End of turn: If your hand is full, this gains +{buff}/+{buff}.
- **Engine:** A `cost` hook like C+ #64 Mulch Muncher's, R65's floor at 0. It counts every card in your
  hand, itself included. With `MAX_MANA` 4 it is playable from 4 cards in hand. On the field, at its
  controller's end of turn, when the hand holds `HAND_CAP`: base `set_radiant` on itself (§5.2: the
  16/16 layer, keeping damage and buffs, with the Radiant text from then on); Radiant `buff` +8/+8.
- **Rulings:** **MD-D33:** the count includes the Giant itself, as the designer wrote, unlike
  Hearthstone's "other". "Full" means `HAND_CAP` cards.
- **Numbers:** discount 1 → 1 ↑; buff 8 → 8 ↑ (Radiant text only, as Core #62 declares its `buff`).
- **Check:** Cost 8 is a new value for §5's cost list (0–6, 10, 100, X). Nerf's cost row never applies
  to it (above 4, R386). **⚠ designer** on "each card" against Hearthstone's "each other card".
- **Class:** B.

#### Meditative #75 · Ever Growing Tree
`meditative-075` · (4) Field Spell · Legendary

> **Designer:** Activate: Fuse a random Field Spell into this. ~~~ Activate: Fuse a random Radiant Field
> Spell into this.

- **Text:** Activate: Fuse a random Field Spell into this.
- **Radiant:** Activate: Fuse a random Radiant Field Spell into this.
- **Engine:** Activate (R384, once a turn). `fuse_random_into({ into: self, query: { type: Field Spell
  } })` (E23, R77, R102) with the Tree as the kept instance. The pool is the non-token Field Spells of
  every set except this card and every ingredient already in it (R387). Radiant: the pick goes in on
  its Radiant face (R469).
  The fused texts work as C+ #74's do. A Cry never runs, since the Tree is already on the field. Auras
  and start- and end-of-turn lines run. Fused Activates share the Tree's per-turn count (B3.2 rule 3),
  so a once-a-turn ability is spent by any use. The cost stays min(sum, 4) = 4.
- **Rulings:** **MD-D34:** a keyword fused in applies at once. A fused "Animated on your turn"
  animates the Tree at your next start of turn with the summed stats, while a fused plain Animated
  waits until the Tree enters the field again.
- **Numbers:** none.
- **Class:** B.

---

#### Group D's systems

- **ME-STATS** (#50). A compiled-in win-rate table, `crates/cards/data/win_rates.json`, built by a
  `crates/tools` command from the public statistics of the newest shipped patch (R377's in-deck rate,
  R654's gate). It is refreshed by hand once per card patch and claimed by the pending fragment
  (R388), and it travels inside the catalog's `OnceLock`. The engine picks the leader with
  `CN_TECH_MIN_GAMES`, an exact integer rate comparison, then games, then id, and falls back to a random
  card when nothing qualifies.
- **ME-PARETO** (#71, #71.1). `subsystems/pareto.rs`: the scorer judges each opponent `play` against
  their other playable cards, from their own concealed view on the pre-play state, plus a per-instance
  dry run in `scorer.rs`. It stores `state.play_judgement` for the watching card's trigger, and it
  animates the Cane for its forced attacks.
- **NEW: ME-EMOTE** (#71 Radiant). `ActionBody::Emote` and the `emoted` event, legal only while a card
  `hears_emotes`. The actor mints it from an emote that passed R643's gate and refuses it on the action
  channel; hotseat, practice and the AI persona send it by the same flag. It revises R643, and
  `AI_SKIPPED_ACTIONS` gains it.
- **ME-MAGNETIC** (#67, #65). The play option `magnetic: true` onto your acting non-Immutable Unit. The
  card resolves in full on top of it, then is fused into the host (R77, host kept, R653's reading). It
  is distinct from Stack.
- **ME-JADE** (#54). It provides Jade `meditative-039-2` and Red Jade `meditative-039-4` (and their
  Radiant faces), which Money Machine's hidden roll shuffles in.
- **NEW: ME-GRANT, granted abilities** (#58, #59). A plain-data grant on an instance naming a hook the
  granting card registers (`CardScripts.grants`, with its text in the catalog). §4.5 runs granted Deaths
  under the dying unit's controller. Grants behave like granted keywords (kept by Vanilla and copies,
  lost by R78 and Transform).
- **NEW: Conditional attack modifiers and `conditionTargets`** (#52). The `attack_mods` aura hook,
  read only for an attacker's strikes against a Unit (and in lethal projection and dry runs). It comes
  with a view field on the viewer's own attackers listing the targets on which it applies, painted
  yellow by the drag layer (R195's sibling).
- **NEW: De-Radiant** (#53). The inverse of Make Radiant, with the event `deradianted`. It amends
  §5.2's "nothing un-sets it".
- **NEW: Either-player Activate** (#54). `ActivationDecl.by: Either`: the opponent activates in their
  own main phase and pays from their own mana.
- **NEW: Attack redirect** (#64). The unbuilt attack half of §6.3 Redirect: it re-aims
  `declared_attack` inside §4.2 step 4's window and emits `redirected { what: attack }`. It allows a
  combat between allies.
- **NEW: Exile on damage** (#72). A §4.4 step 7 rider that marks the damaged unit `marked_exiled`;
  §4.5 step 1 exiles it ahead of deaths (R461).
- **NEW: `manaSpent` event** (#69). Emitted at the play-price and activation-price spend sites, with
  the amount.
- **NEW: Catalog query fields** (#51, #60). `stats` (printed base attack and health) and `anyTags`.
  Also MD-D11: the Prime and AI tag pools take their tokens (`POOL_TOKEN_TAGS`), for #53, #55, #59 and
  #60.
- **NEW: Small verbs** (#51, #59, #63, #73). `fill_board_random` (one pick per zone),
  multi-pool `fuse_generated`, `summon_random_from_hand`, and the `armor_on_field` read helper.
- **R383 revision (MD-D23)** (#66, #71.1). An "Animated on your turn" card is not sick when it animates
  at a start of turn after the one it entered on. It touches C+ #12.8.
- **Voice** (#63, #70, #70.1, #73). `card-audio.json5` play lines, plus a NEW `trigger` hook with a list
  of lines picked by the client's cosmetic rng, which needs a `sourceId` on `summoned`.
- **Existing, cited:**
  - Animated / Animated on your turn (R383);
  - Fuse (R77, R102, R469, R470) and `fuse_random_into`;
  - random casts (E12, R70, R452);
  - `tribute_when` (R403);
  - `cost` hook (C+ #64);
  - forced attacks (R53, `forced_attack_random`);
  - summon for the opponent (R360);
  - `fill_board`, `summon_random`, `add_random_from_catalog`;
  - Upgrade (R386);
  - `draw_from_library` (C #94);
  - Bounce (R692, R747);
  - `consume_plague` and Plague Counters (E19, R689);
  - Activate (R384).

### Group E: Meditative #76 to #94 with their tokens, and #99

Twenty-four entries: nineteen cards (#76–#94, #99 Paranoia, the designer's second #93) and five tokens
(#91.1, #93.1–#93.3). Every text uses Nerf and Buff for the engine's `degrade` and `upgrade` (R386),
"Deck" and "Tribute" (R373), "(N) Cost" and "costs (N)" (R432). Script files are
`crates/cards/src/scripts/meditative/cNNN_slug.rs`. Ruling proposals are MD-E1 to MD-E20.

---

#### Meditative #76 · Do or Die
`meditative-076` · (1) Spell · Common

> **Designer:** Mark 2 random cards in the enemy hand. If they aren’t played, steal them at the start
> or your turn. ~~~ Mark every card in the enemy hand. If they aren’t played, steal them at the start or
> your turn.

- **Text:** Mark {marks|random card|random cards} in your opponent's hand. At the start of your next
  turn, steal each marked card still in their hand.
- **Radiant:** Mark every card in your opponent's hand. At the start of your next turn, steal each
  marked card still in their hand.
- **Engine:** as it resolves, `marks` different cards of the opponent's hand are drawn uniformly (match
  rng, R60; all of them when fewer; a pick over a pile the caster can't read, R242). A delayed effect
  (§2.2's start-of-turn delayed point, R62, as #50 K-Pop Fanatic's R76) watches each picked card and
  marks it (R437). At the start of the caster's next turn it steals every watched card still in that
  hand off the field (§6.3 Steal, R12): into the caster's hand as theirs, in hand order, the hand cap
  burning the rest into the caster's graveyard; a unit-token card ceases to exist as it leaves (R11).
  Existing: `delay`, marks, `give_from_hand`. Missing: **NEW: ME-HANDMARK**, a delayed effect that
  watches several hand cards, drops each one as it leaves the hand, marks hand cards, and a
  `give_from_hand` selector by instance ids.
- **Rulings:** **MD-E1:** a hand mark lasts while that card stays in that hand (its stay, R174): played,
  cast, discarded, shuffled away or stolen by something else, it is unwatched and unmarked, and a card
  that comes back to the hand later is a new stay and unmarked. **MD-E2:** the hand's owner sees which
  of their cards carry the mark; the caster sees only how many cards in that hand are marked (the
  opponent's `HandView::Count` gains `marked`), and meets each card as it reaches their hand (R97).
  The Radiant face marks the cards in the hand as it resolves; cards drawn later are not marked. With
  an empty enemy hand nothing is scheduled.
- **Numbers:** `marks` 2 (base face only; the Radiant face marks every card) ↑.
- **Check:** "start or your turn" is "start of your turn" (read as the next one). R275: the Radiant
  widens one scope (two cards to all).
- **Class:** C (NEW: ME-HANDMARK).

#### Meditative #77 · Bulk Booster
`meditative-077` · (3) Spell · Common

> **Designer:** Fill your hand with random Common cards. ~~~ Fill your hand with random Radiant Common
> cards.

- **Text:** Fill your hand with random Common cards.
- **Radiant:** Fill your hand with random Radiant Common cards.
- **Engine:** `add_random_from_catalog` with `count` = the caster's hand cap less the cards in hand as
  it resolves (Bulk Booster itself is resolving, not in hand), the pool `{ rarity: Common }`: non-token
  Commons of every set (R380, R382) but this one (R387), repeats allowed (R60), each on its base face at
  its printed cost (Radiant: `radiant: true`). The cap is ME-HANDCAP's per-player cap (12 after #79),
  not the `HAND_CAP` literal; "fill" is C+ #42's reward wording, which reads the same.
- **Rulings:** with a full hand nothing is added and nothing is drawn from the rng (R129).
- **Numbers:** none ("fill" is no number).
- **Class:** B (reads ME-HANDCAP).

#### Meditative #78 · Occidentless Mandate
`meditative-078` · (4) Spell, CN · Epic

> **Designer:** Exile all non-CN permanents; add a random CN card for each friendly permanent you
> exiled. ~~~ Exile all non-CN permanents; add a random Radiant CN card for each friendly permanent you
> exiled, they cost (2) less.

- **Text:** Exile every non-CN permanent. Add a random CN card to your hand for each of yours exiled
  this way.
- **Radiant:** Exile every non-CN permanent. Add a random Radiant CN card to your hand for each of yours
  exiled this way. Each costs ({discount}) less.
- **Engine:** the hook counts first, then exiles (Frozen Wastes' pattern, C+ #12.6): N = the
  permanents its controller controls that lack the CN tag, the tops of unit piles and every backrow
  card, face-down ones included; then `exile_all({ side: "any", rows: ["units", "backrow"],
  notTags: ["CN"] })` (exile ignores Indestructible, no Death), then `add_random_from_catalog` N times
  from `{ tags: ["CN"] }` but this card (R387), repeats allowed (R60); Radiant `radiant: true` and
  `costMod −discount` (an X-cost card ignores it, R65). The hand cap burns the overflow.
- **Rulings:** **MD-E3:** "each friendly permanent you exiled" counts the permanents you controlled that
  this exile took off the field: a stolen enemy card you control counts, your card the opponent
  controls does not, a unit token counts although it ceases to exist instead of reaching exile (R11),
  and a card dormant under a Stack is not exiled (it resumes, R13) and does not count. A face-down CN
  trap stays where it is, which tells its opponent its tag, as R403 lets consequences show. This is
  "tribal tag based hate" under ME-TRIBAL (non-CN), so a card immune to it (Meditative #25) stays.
- **Numbers:** `discount` 2 (Radiant only, `tunedOn: "radiant"`) ↑.
- **Class:** B (reads ME-TRIBAL's "hate" definition).

#### Meditative #79 · Touched by KY
`meditative-079` · (3) Unit, KY · Legendary · 4/4 → 8/8

> **Designer:** Cry: Draw 4 cards, for the rest of the game your handsize is 12 · [4/4] ~~~ Cry: Draw 4
> cards, make them Radiant, for the rest of the game your handsize is 12 · [8/8] · {for ai implementing:
> voiceline should be “We must expand our minds”}

- **Text:** Cry: For the rest of the game, your hand size is {handSize}. Draw {draw|card|cards}.
- **Radiant:** Cry: For the rest of the game, your hand size is {handSize}. Draw {draw|card|cards}. Each
  becomes Radiant.
- **Engine:** ME-HANDCAP sets its controller's hand cap to `handSize` for the rest of the game, then
  `draw` separate draws (§2.4, each with its own cast-on-draw chain). Radiant: each draw is a
  `draw_priced`-style draw whose card, if the draw put it in the hand (`card_this_draw_put_in_hand`,
  R596), is made Radiant (§5.2); a card cast on draw, burned, or a fatigue or limited draw gets nothing.
  Missing: `draw_priced`'s rider gains `radiant: true` (small). Voice line: the play line in
  `apps/web/src/audio/card-audio.json5` is "We must expand our minds." (R204; flavour, not rules).
- **Rulings:** **MD-E4:** "your hand size is N" sets that player's cap to N for the rest of the game,
  whether or not the card that set it is still on the field; it is a setting, not an increase (a second
  Touched by KY changes nothing, and the latest setting wins), it is public in both views, and every
  rule that reads the hand cap reads it (draws, adds, steals, "fill your hand", "your hand is full").
  The Cry sets the cap before it draws, so the four draws use the new room.
- **Numbers:** `handSize` 12 → 12 ↑ (step 1, min 11, max 14 = `HAND_CAP_MAX`, the most cards the
  client lays out); `draw` 4 → 4 ↑.
- **Check:** the designer's order (draw, then the cap) is reversed so the draws are not burned at 10;
  "handsize" is "hand size".
- **Class:** C (ME-HANDCAP).

#### Meditative #80 · Aluneth
`meditative-080` · (3) Field Spell, Quickdraw · Legendary

> **Designer:** Indestructible, Untributable, Immutable · End of turn: Draw 3 ~~~ Indestructible,
> Immutable · Activate: Exile this · End of turn: Draw 3 · {for ai implementing: voiceline should be
> “Come, child. Let us wreak havoc.”}

- **Text:** Indestructible, Untributable, Immutable
  End of turn: Draw 3 cards.
- **Radiant:** Indestructible, Immutable
  Activate: Exile this.
  End of turn: Draw 3 cards.
- **Engine:** Quickdraw (§6.2, R640) deals it in the opening hand, as #98 Heroic Power is. An
  Indestructible Field Spell (#98, C #84 Lockdown) that `end_of_turn` draws 3 for its controller (each
  draw its own, burning past the cap, fatigue past the deck): Hearthstone's Aluneth, a card that
  draws you to death unless you can get rid of it. Immutable blocks Transform, Vanilla, Fuse-onto,
  Buff, Nerf and KY's Constant (R23, R386). Radiant: one Activate (R384), once per turn on its
  controller's main phase, that exiles it (no Death), the off-switch C #84's "Activate: Tribute this."
  is. Missing: **NEW: ME-UNTRIBUTABLE**, the keyword. Voice line: its cast line in
  `card-audio.json5` is "Come, child. Let us wreak havoc." (R204).
- **Rulings:** **MD-E5:** Untributable: this card can't be Tributed. No Tribute cost (a play's,
  R101, or an activation's) may take it, and every Sacrifice of it, by its own text or another card's,
  does nothing; it is still exiled, bounced and stolen as usual (a stolen Aluneth draws for its new
  controller). It is not in R21's random keyword pool, and a Nerf never removes it
  (`TUNE_HARMFUL_KEYWORDS`).
- **Numbers:** none: Immutable puts its numbers beyond every Buff, Nerf and KY's Constant, so "3" is
  printed.
- **Check:** R275: the Radiant face drops the drawback that keeps it on the field and adds an
  Activate, a rider; its draw is unchanged, since more cards would only kill you sooner.
- **Class:** C (NEW: ME-UNTRIBUTABLE).

#### Meditative #81 · Deadman's Hand
`meditative-081` · (0 embiggen 2) Spell · Rare

> **Designer:** Shuffle a copy of your hand {including this} into your deck, embiggen cards have a 25%
> chance of becoming Radiant ~~~ Shuffle a copy of your hand {including this} into your deck, embiggen
> cards have a 100% chance of becoming Radiant · Draw 1

- **Text:** Shuffle a copy of each card in your hand, and of this, into your deck.
  Paid (2): Each copy has a {chance}% chance to become Radiant.
- **Radiant:** Shuffle a copy of each card in your hand, and of this, into your deck. Then draw
  {draw|card|cards}.
  Paid (2): Each copy has a {chance}% chance to become Radiant.
- **Engine:** cost `{ base: 0, embiggen: 2 }` (§2.3, `instance.embiggened`). As it resolves: for each
  card in its controller's hand, in hand order, then for this card, a library copy (R57: radiant flag,
  `statsOverride` and `tuning`, as Unstable Clone Machine's; `shuffle_into { copyOf }`) shuffled in at a
  random position (§6.3), R80's cap turning the rest away; paid, each copy that is not already Radiant
  rolls `chance` (match rng) and becomes Radiant on a success. Then the Radiant face's draw. The copies
  are the owner's known cards in their library list (R311); the opponent sees how many went in.
- **Rulings:** **MD-E6:** "embiggen cards" is read as the embiggen price (the only text that names it):
  paid at (2), each copy rolls; at (0) none does. A copy that is already Radiant rolls nothing (R129).
  "Including this" is a copy of Deadman's Hand on the face it was played with; a copy is not generation
  (R387).
- **Numbers:** `chance` 25 → 100 ↑ (max 100); `draw` 1 (Radiant only, `tunedOn: "radiant"`) ↑.
- **Check:** R275: the Radiant quadruples the chance and adds a draw.
- **Class:** B.

#### Meditative #82 · Medina Outfitter
`meditative-082` · (1) Unit · Common · 1/1 → 2/2

> **Designer:** [1/1] · Cry: Buff every card in your hand ~~~ [2/2] · Cry: Buff every card in your
> hand, thrice

- **Text:** Cry: Buff every card in your hand.
- **Radiant:** Cry: Buff every card in your hand {times|time|times}.
- **Engine:** `upgrade({ scope: { side: "self", zones: ["hand"] }, times })`: each card in the hand
  takes `times` separate Buffs (R386), each one draw (R442), an Immutable card left alone; a hand is a
  pile the opponent can't read, so every application is cued and none is counted (R440).
- **Numbers:** `times` 1 → 3 (`tunedOn: "radiant"`) ↑.
- **Class:** A.

#### Meditative #83 · Medina Enforcer
`meditative-083` · (2) Unit · Rare · 4/4 → 8/8

> **Designer:** 4/4 · End of Turn: Buff every card in your hand ~~~ 4/4 · End of Turn: Buff every card
> in your hand, thrice

- **Text:** End of turn: Buff every card in your hand.
- **Radiant:** End of turn: Buff every card in your hand {times|time|times}.
- **Engine:** #82's effect on its controller's `end_of_turn` (R62's end-of-turn triggers).
- **Numbers:** `times` 1 → 3 (`tunedOn: "radiant"`) ↑.
- **Check:** the designer's Radiant face is 4/4, its base stats; R275 wants each doubled, so 8/8.
  **⚠ designer.**
- **Class:** A.

#### Meditative #84 · Volatility
`meditative-084` · (1) Unit · Common · 3/3 → 6/6

> **Designer:** 3/3 · Buffs & Nerfs are twice as effective on this. ~~~ 6/6 · Buffs are three times as
> effective on this.

- **Text:** Buffs and Nerfs are twice as effective on this.
- **Radiant:** Buffs are three times as effective on this.
- **Engine:** **NEW: ME-TUNEMULT**: a static flag `tuneMultiplier { upgrade: 2, degrade: 2 }` (Radiant
  `{ upgrade: 3 }`, Nerfs ×1), read off the card's running face wherever it is (field, hand, deck),
  which `tune_effect` hands to `tune_once` for each application on this card.
- **Rulings:** **MD-E7:** one Buff or Nerf on a card "N times as effective" is still one application
  and one draw (R442: the row, then the item, then the split), and the change it draws is N times as
  large: the cost row moves N steps (Nerf never above (4), Buff never below (0)); the stats row splits
  N × `TUNE_STAT_TOTAL` (k in 0 to 4N); the keyword row adds N different keywords it lacks (R21's pool)
  or removes N of its own (never a harmful one); an X moves N; a declared number moves N steps. What a
  bound refuses is lost, and one event reports the whole change, so R440's count of applications over a
  hidden pile is unchanged. "Buffs" and "Nerfs" are the verbs only: a stat buff ("+2/+2") and KY's
  Constant are not multiplied. A fused card's multipliers multiply.
- **Numbers:** none: "twice" and "three times" are kept out of `params`, so a Buff made twice as
  effective can never raise its own multiplier.
- **Check:** "Buffs & Nerfs" is "Buffs and Nerfs". R275: stats doubled, Buffs ×3, and the Nerf
  weakness gone.
- **Class:** C (NEW: ME-TUNEMULT).

#### Meditative #85 · Playtester
`meditative-085` · (2) Unit · Rare · 4/5 → 8/10

> **Designer:** [4/5] · Start of turn: Add either a Book of Buff or a Book of Nerf to your hand ~~~
> [8/10] · Start of turn: Add either a Radiant Book of Buff or a Radiant Book of Nerf to your hand

- **Text:** Start of turn: Choose one: add {books|Book of Buff|Books of Buff} or {books|Book of
  Nerf|Books of Nerf} to your hand.
- **Radiant:** Start of turn: Choose one: add {books|Radiant Book of Buff|Radiant Books of Buff} or
  {books|Radiant Book of Nerf|Radiant Books of Nerf} to your hand.
- **Engine:** at its controller's start of turn (R62), `choose_mode` with two modes, then `add_to_hand`
  of C+ #71 `classicplus-071` or C+ #72 `classicplus-072` (Radiant: `radiant: true`). Both are real
  cards, named, so no pool and no R387. The prompt is the controller's; a timeout answers it with the
  AI policy (R79); the opponent sees a prompt open and a hidden card arrive.
- **Rulings:** **MD-E8:** "add either A or B" is a choice of its controller's, a mode prompt, never a
  coin.
- **Numbers:** `books` 1 → 1 ↑.
- **Class:** B.

#### Meditative #86 · Mayor Medinamogger
`meditative-086` · (2) Unit · Legendary · 5/4 → 10/8

> **Designer:** 5/4 · All targets are random. ~~~ 10/8 · Lucky 1 · All targets are random.

- **Text:** Aura: All targets are chosen at random.
- **Radiant:** Lucky 1
  Aura: All targets are chosen at random.
- **Engine:** Hearthstone's Mayor Noggenfogger, for both players, while a Mayor acts on the field (the
  top of its pile). Builds on §6.2's "Targets chosen randomly" and the random cast machinery
  (`random_cast.rs`, R452: `random_picks`, a prompt answered at once). Missing: **ME-RANDOMTARGETS**
  (shared): (1) `legal_actions` offers each play, cast choice and activation once per set of its other
  choices with its targets left out, and each attack once per attacker with no target; (2) the reducer
  draws each declared target at §10.5 step 1, before anything is paid, so R450's targeting point,
  targeting costs (C #89) and "a friendly unit is targeted" replacements (C #33 Joro) meet the drawn
  target; (3) a `target` prompt is answered at once at random, for whichever player it opens for;
  (4) an attack's target is drawn from the targets that attacker may legally attack (§4.2 steps 2 and 3:
  Taunt, lane restrictions, Rush on its arrival turn); (5) the view says random targets are on, and the
  client asks for no target (CLAUDE.md rule 7). Every draw is the match rng.
- **Rulings:** **MD-E9:** "targets" are a play's or activation's `TargetDecl` of kind `target` (any
  cards it allows, a hand card included), every `target` prompt (Echo repeats and Trigger a Cry's
  re-asked targets included) and every attack's target; Discover, modes, `hand` picks, numbers, cells,
  zones and Tributes are choices, not targets, and stay the player's. A declaration takes as many picks
  as R452's random cast takes for it, different cards (R60); one with no legal pick plays and fizzles,
  unless it requires one (R703). **MD-E10:** Lucky 1 (Radiant) rolls each random target of its
  controller's twice and keeps the better: first the side R656's aim prefers (an enemy for harm, a
  friend for help), then, for an attack, a target whose strike back the attacker survives, then one
  the attacker destroys; otherwise the first roll stands. The opponent's targets are rolled once.
- **Numbers:** none (Lucky is a numbered keyword, which a Buff's X row moves).
- **Check:** "Medinamogger" is the designer's pun on Noggenfogger.
- **Class:** C (ME-RANDOMTARGETS).

#### Meditative #87 · Tatches the Totem
`meditative-087` · (1) Unit, All Tribes (Human, Felinor, KY, CN, Jlockeed) · Legendary · 0/3 → 0/6

> **Designer:** 0/3 · Summon this from your Deck if you play a card with a Tribal tag. · End of Turn:
> Buff a random card in your Hand or Deck. ~~~ 0/6 · Summon this from your Deck if you play a card with
> a Tribal tag. · End of Turn: Buff a random card in your Hand or Deck and adjacent Units.

- **Text:** While this is in your deck: After you play a card with a tribal tag, summon this.
  End of turn: Buff {cards|random card|random cards} in your hand or deck.
- **Radiant:** While this is in your deck: After you play a card with a tribal tag, summon this.
  End of turn: Buff {cards|random card|random cards} in your hand or deck. Then Buff each Unit adjacent
  to this.
- **Engine:** a deck trigger (R464, C+ #37 Wardrum's) on `cardResolved` of its owner's play, a cast
  included (R70), whose running face carries a tribal tag (ME-TRIBAL): `summon_this` into the leftmost
  open unit zone (R64), no Cry (R1), the first either player sees of it (R97); with no open zone it
  stays in the deck. End of turn: `upgrade({ scope: { side: "self", zones: ["hand", "library"] },
  random: cards })`, one uniform pick over both piles (R440: whole hidden piles reached); Radiant, then
  one Buff on each Unit `adjacent_to` this (§3.1). "All Tribes": **ME-TRIBAL**.
- **Rulings:** **MD-E11:** the tribal tags are Human, Felinor, KY, CN and Jlockeed (`TRIBAL_TAGS`):
  the tags that name a people, a creature or a faction, the ones "tribal tag based hate" reads (Core #2
  Bigot's non-Human, #78's non-CN, Meditative #34's non-CN or KY); Fruit, Book, Pancake and Call to
  Chaos name card families, and Quickdraw, Plague, AI, Catalyst, Prime, Acclaimed, Wincon and Token name
  mechanics or origins. An "All Tribes" card lists every tribal tag in its catalog `tags`, so every rule
  that reads a tag (pools, filters, auras, counts) sees each one, and the frame prints "All Tribes"
  (`allTribes: true`, a display field a cross test holds to the full list). A face-down play is a "Trap"
  with no tags until it is revealed (R448), so it summons nothing.
- **Numbers:** `cards` 1 → 1 ↑.
- **Check:** 0 attack doubled is 0; the health doubles and the Radiant adds the adjacent Buffs (R275).
- **Class:** C (ME-TRIBAL).

#### Meditative #88 · The True Sheep
`meditative-088` · (4) Unit · Epic · 1/1 → 2/2

> **Designer:** [1/1] · Cry: Discover a tribute card to replace this with · Worth 5 tributes ~~~ [2/2] ·
> Cry:  Discover a Radiant tribute card to replace this with · Worth 500 tributes

- **Text:** Worth {worth|Tribute|Tributes}.
  Cry: Discover a card with Tribute to replace this with.
- **Radiant:** Worth {worth|Tribute|Tributes}.
  Cry: Discover a Radiant card with Tribute to replace this with.
- **Engine:** `tributeWorth` (§6.3 Tribute, the Sheep Token's and C #82 Sheeople's static flag) at
  `worth`, counted toward a Tribute X only (R101). The Cry: `discover_from_catalog` over the non-token
  cards with a Tribute X play cost, every set (R380) — Lava Golem, The Rock, Nature Titan, Plague Bringer
  Goliath, BOOM! Big Max, Meditative #25 and #91 today — three different options shown to the chooser
  only (§6.3), Radiant on the Radiant face; the pick goes to your hand. Missing: **NEW:
  ME-TRIBUTEPOOL**, a `catalog.query` filter for "has a Tribute cost" (the cost is a script flag, not
  catalog data).
- **Rulings:** **MD-E12:** "to replace this with" says what the Discovered card is for: the card goes to
  your hand, and the Sheep, worth 5, pays any Tribute in the game alone (the largest printed is 3, and
  a set of one Unit is minimal, R101), so its Tribute replaces this. Reading it as a Transform would
  leave "Worth 5 tributes" with almost nothing to do (the Sheep would never stay on the field after its
  Cry), so this reading keeps both lines live.
- **Numbers:** `worth` 5 → 500 ↑ (step 1; Radiant step a quarter, 125).
- **Check:** "Cry:  Discover" has a double space. **⚠ designer:** if the Cry should transform the Sheep
  into the Discovered card on the spot (§6.3 Replace on the field), say so; the Tribute would then never
  be paid.
- **Class:** C (NEW: ME-TRIBUTEPOOL, small).

#### Meditative #89 · Jlarna
`meditative-089` · (3) Field Spell · Rare

> **Designer:** (2) Jlarna, Field Spell, Rare, Meditative, #89
> Combo 2: This costs (2) less.
> Aura: You can spend mana from next turn. {for AI: lock your next turn’s mana crystals as this happens}
> If either player doesn’t play a card on a turn, Tribute this.
> ~~~
> Combo 2: This costs (2) less.
> Aura: You can spend mana from next turn.

The designer then made four later changes, kept in the reading below: the brief's "pay in 4" stays
(M9 row 3, now answered); the Combo line is gone and the cost is (3); a missed instalment is
forgiven; and the Tribute condition is "If you don't use your credit line", checked on the turn
Jlarna is played too.

- **Text:** Aura: Credit line {credit}: you can spend mana you don't have, owing up to {credit} at
  once. Pay it back in {instalments|instalment|instalments}, one at the start of each of your next
  turns.
  End of turn: If you didn't use your credit line this turn, Tribute this.
- **Radiant:** the Aura line only.
- **Engine:** the Aura, a credit line paid in 4 (MD-E13, rewritten). While Jlarna acts on its
  controller's field, any mana they spend (a card's price, an X, an Activate's cost) can go past
  the mana they have: they spend their current mana first, and the shortfall is borrowed. What they
  owe at once (every instalment not yet taken) can never be more than 4. Everything borrowed during
  one turn is one debt, split into 4 instalments as evenly as possible, larger first. One
  instalment falls due at each of the borrower's next four refreshes (§2.3), taken off what the
  refresh gives; debts from different turns add up on one refresh. The instalments lock in as soon
  as the mana is borrowed, and the schedule is public. A missed instalment is forgiven: it takes
  what there is, and the rest is dropped, never carried. The debt stays if Jlarna leaves the field;
  two or more Jlarnas share one line. The base face is Tributed at the end of a turn its controller
  borrowed nothing on, the turn it is played included (R1225). One affordability function
  (`spendable_mana`) decides what a player can pay, read by `legal_actions`, the reducer's refusal
  and the AI. This reading replaces the brief's old card, MD-E13 and every row that described it.
- **Rulings:** **MD-E13 (rewritten):** the Aura is now a credit line paid in 4, above. **R1223** the
  borrowing and its limit; **R1224** the repayment schedule and its forgiveness; **R1225** use it or
  lose it.
- **Numbers:** `credit` 4 → 4 ↑; `instalments` 4 → 4 ↑ (more instalments, thinner shares).
- **Check:** R275 ✓: the Radiant face has no self-Tribute. Rare now by the designer's choice, not
  the brief's.
- **Class:** C (ME-TURN's repayment schedule).


#### Meditative #90 · Spell Basket
`meditative-090` · (1) Spell · Common

> **Designer:** Add 3 random spells to your hand ~~~ Add 3 random Radiant spells to your hand

- **Text:** Add {cards|random Spell|random Spells} to your hand.
- **Radiant:** Add {cards|random Radiant Spell|random Radiant Spells} to your hand.
- **Engine:** `add_random_from_catalog({ query: { type: "Spell" }, count: cards })`: non-token Spells
  of every set (R380, R382) but this one (R387), repeats allowed (R60), on their base faces at their
  printed costs (Radiant: `radiant: true`); the hand cap burns the rest.
- **Rulings:** "spells" is the Spell type, not Field Spells, as C+ #38.1's random Spells are.
- **Numbers:** `cards` 3 → 3 ↑.
- **Class:** A.

#### Meditative #91 · Windfast
`meditative-091` · (1) Unit, Catalyst · Epic · 1/1 → 2/2

> **Designer:** 1/1 · Tribute 1 · Windfury · Instead of attacking itself, this summons a Unit from your
> Hand to do the attack, then Bounces it if it survives. · Death: Shuffle Windfurious Prime into your
> Deck. ~~~ 2/2 · Tribute 1 · Windfury · Instead of attacking itself, this summons a Unit from your Hand
> to do the attack. · Death: Shuffle Radiant Windfurious Prime into your Deck.

- **Text:** Tribute 1, Windfury
  When this would attack, summon a Unit of your choice from your hand to make that attack instead.
  Bounce it afterwards if it survives.
  Death: Shuffle a Windfurious Prime into your deck.
- **Radiant:** Tribute 1, Windfury
  When this would attack, summon a Unit of your choice from your hand to make that attack instead.
  Death: Shuffle a Radiant Windfurious Prime into your deck.
- **Engine:** Tribute 1 (`tribute`, R101, R391) and Windfury (R636: two declared attacks a turn). Death:
  `shuffle_into` a fresh `meditative-091-1` (Radiant: `radiant: true`) at a random position, R80's cap
  turning it away, as C+ #38 Solarius's. The attack replacement is **NEW: ME-ATTACKSUMMON**: on every
  attack Windfast makes (declared, spending its exertion, or forced), before `attackDeclared`, its
  controller picks a Unit in their hand (a `hand` prompt, held on the opponent's turn under its own
  clock, R79; one Unit is no prompt, R129); that card is summoned into the leftmost open unit zone (R64,
  no Cry, R1; a summon, so traps that answer one fire, C #5 Tesla) and makes the attack on the same
  target as a forced attack (R53: no sickness, no exertion, §4.2 step 2's restrictions holding), with
  its own trap window and combat; Windfast neither strikes nor is struck. Base: after that combat's
  state check, the summoned card, if still on the field, is bounced to its controller's hand (R692,
  R78's reset; a unit token vanishes, R11).
- **Rulings:** **MD-E14:** with no Unit in hand or no open unit zone the replacement can't happen and
  Windfast attacks itself; a summoned Unit that dies or leaves before its combat makes no attack, and
  Windfast's attack is spent either way; each of Windfury's two attacks summons its own. "A Unit from
  your Hand" with no "random" is the controller's pick, the way a target is.
- **Numbers:** none (Tribute 1 is a numbered keyword a Buff's X row moves).
- **Check:** R275: stats doubled, the summoned Unit stays, and the Prime it shuffles is Radiant.
- **Class:** C (NEW: ME-ATTACKSUMMON).

#### Meditative #91.1 · Windfurious Prime
`meditative-091-1` · (3) Unit, Prime, Token · Token (printed Epic) · 5/10 → 10/20

> **Designer:** 5/10 · Rush · Windfury · In addition to attacking itself, this summons 2 Units from
> your Hand or Deck to join the attack as well. {for AI: these units attack first} ~~~ 10/20 · Rush ·
> Windfury · First Strike · In addition to attacking itself, this summons 2 Units from your Hand or Deck
> to join the attack as well.

- **Text:** Rush, Windfury
  Whenever this attacks: Summon {joiners|random Unit|random Units} from your hand or deck. They attack
  its target first.
- **Radiant:** Rush, Windfury, First Strike
  Whenever this attacks: Summon {joiners|random Unit|random Units} from your hand or deck. They attack
  its target first.
- **Engine:** a unit-token card that lives in the deck and hand until played (R11, as C+ #46.1). On
  each attack it makes (declared or forced, C #45 Nature Titan's reading), before its own combat:
  `joiners` different Units drawn uniformly from the Unit cards in its controller's hand and deck
  together (match rng, R60), each summoned into the leftmost open unit zone (R64, no Cry), a deck card
  seen first as it arrives (R97); then each joiner, in lane order, attacks the Prime's target as a
  forced attack (R53, each its own combat and state check, the run stopping once the target is gone);
  then the Prime's own combat. Joiners stay on the field. ME-ATTACKSUMMON's rider half.
- **Rulings:** **MD-E15:** a declared attack whose target has left the field before its combat is
  cancelled (`attackCancelled`, R44's shape; the exertion stays spent, so Windfury's second attack is
  still there); the designer's AI note is the rule that joiners attack first; fewer joiners arrive when
  zones or Units run out; a joiner that is itself a Windfurious Prime brings its own joiners, bounded by
  the five zones.
- **Numbers:** `joiners` 2 → 2 ↑.
- **Check:** "Medidative" is Meditative. R275: stats doubled and First Strike.
- **Class:** C (NEW: ME-ATTACKSUMMON).

#### Meditative #92 · Unan
`meditative-092` · (3) Unit · Rare · 3/13 → 6/26

> **Designer:** [3/13] · Armor 3 · Fatal damage a friendly ally would take is redirected to this ~~~
> [6/26] · Armor 7 · Fatal damage a friendly ally would take is redirected to this

- **Text:** Armor 3
  Lethal damage your hero or your other Units would take is redirected to this.
- **Radiant:** Armor 7
  Lethal damage your hero or your other Units would take is redirected to this.
- **Engine:** a replacement (§6.2, `replacements` on `lethalHit`, R460) at §4.4 step 4a, which today
  opens only for a hero (C #52 Final Gambit). Missing: **NEW: ME-LETHALGUARD**: step 4a also opens for
  a hit on a Unit, judged after steps 0 to 4 (a Divine Shield takes the hit first and an Indestructible
  unit takes nothing, so neither hit is lethal), and `instead.redirect: "self"` sends the hit to this
  card as a new instance from the same source through this card's own pipeline (its Armor, shield and
  caps), as Final Gambit's goes to the enemy hero. Event `redirected`. Answers only its controller's
  hero and units, on either player's turn, combat or effect.
- **Rulings:** **MD-E16:** "a friendly ally" is your hero and your other Units (the designer's "ally"
  takes heroes, as C+ #65.2's does); a hit is lethal when it alone would leave its target at 0 or less
  (R44's judge, fatigue included, losing health and Poisonous not); the redirected hit is never caught
  again by the same Unan (each card once per event, R460), so a hit lethal to Unan itself kills it, and
  `DAMAGE_REDIRECT_CAP` bounds a chain of Unans; each hit of an "all" effect is judged alone against the
  board before any of them lands (R59), and Unan's Armor takes each instance it catches. A Trample hit
  redirected to Unan tramples into its controller's hero.
- **Numbers:** none (Armor is a numbered keyword).
- **Check:** R275: stats doubled, Armor 3 → 7.
- **Class:** C (NEW: ME-LETHALGUARD).

#### Meditative #93 · Growing Felinor
`meditative-093` · (1) Unit, Felinor · Common · 1/1 → 2/2

> **Designer:** [1/1] · Cannot be in defense position · Death: Summon a Growing Felinor Sr ~~~ [1/1] ·
> Divine Shield, Rush · Cannot be in defense position · Death: Summon a Radiant Growing Felinor Sr

- **Text:** Can't be in Defense Position.
  Death: Summon a Growing Felinor Sr.
- **Radiant:** Divine Shield, Rush
  Can't be in Defense Position.
  Death: Summon a Radiant Growing Felinor Sr.
- **Engine:** `never_defense` (§6.1, as #65.1 Spikey Pillow). Death: `summon({ defId:
  "meditative-093-1" })` (Radiant: `radiant: true`) into the leftmost open unit zone (R64); a named
  token, so no pool and no R387.
- **Rulings:** **MD-E19:** the chain grows one size each death: #93 → #93.1 Sr → #93.2 Sr Sr → #93.3
  Super Senior, which ends it; the designer's base faces of #93.1 (summons a Sr, itself) and #93.2
  (summons a Sr, a size down) are read as copy slips, since a Sr that summons a Sr never ends and the
  Radiant faces climb one size each.
- **Numbers:** none.
- **Check:** the designer's Radiant stats repeat the base ones down the whole chain; R275 wants each
  doubled: 2/2, 4/4, 6/6, 8/8. **⚠ designer.**
- **Class:** A.

#### Meditative #93.1 · Growing Felinor Sr
`meditative-093-1` · (1) Unit, Felinor, Token · Token (printed Common) · 2/2 → 4/4

> **Designer:** [2/2] · Cannot be in defense position · Death: Summon a Growing Felinor Sr ~~~ [2/2] ·
> Divine Shield, Rush · Cannot be in defense position · Death: Summon a Radiant Growing Felinor Sr Sr

- **Text:** Can't be in Defense Position.
  Death: Summon a Growing Felinor Sr Sr.
- **Radiant:** Divine Shield, Rush
  Can't be in Defense Position.
  Death: Summon a Radiant Growing Felinor Sr Sr.
- **Engine:** #93's, summoning `meditative-093-2`. A unit token vanishes when it leaves the field (R11)
  and still fires its Death.
- **Numbers:** none.
- **Check:** the base Death fixed to the next size up (MD-E19); Radiant stats 4/4. **⚠ designer.**
- **Class:** A.

#### Meditative #93.2 · Growing Felinor Sr Sr
`meditative-093-2` · (1) Unit, Felinor, Token · Token (printed Common) · 3/3 → 6/6

> **Designer:** [3/3] · Cannot be in defense position · Death: Summon a Growing Felinor Sr ~~~ [3/3] ·
> Divine Shield, Rush · Cannot be in defense position · Death: Summon a Radiant Growing Felinor Super
> Senior

- **Text:** Can't be in Defense Position.
  Death: Summon a Growing Felinor Super Senior.
- **Radiant:** Divine Shield, Rush
  Can't be in Defense Position.
  Death: Summon a Radiant Growing Felinor Super Senior.
- **Engine:** #93's, summoning `meditative-093-3`.
- **Numbers:** none.
- **Check:** the base Death fixed to the next size up (MD-E19); Radiant stats 6/6. **⚠ designer.**
- **Class:** A.

#### Meditative #93.3 · Growing Felinor Super Senior
`meditative-093-3` · (1) Unit, Felinor, Token · Token (printed Common) · 4/4 → 8/8

> **Designer:** [4/4] · Cannot be in defense position ~~~ [4/4] · Divine Shield, Rush · Cannot be in
> defense position

- **Text:** Can't be in Defense Position.
- **Radiant:** Divine Shield, Rush
  Can't be in Defense Position.
- **Engine:** keywords only; the end of the chain.
- **Numbers:** none.
- **Check:** Radiant stats 8/8 (R275). **⚠ designer.**
- **Class:** A.

#### Meditative #94 · Shrinking Felinor
`meditative-094` · (4) Unit, Felinor · Rare · 9/11 → 18/22

> **Designer:** [9/11] · Death: Summon a Shrinking Felinor with base stats [-3/-3] ~~~ [18/22] · Death:
> Summon a Shrinking Felinor with base stats [-2/-3]

- **Text:** Death: Summon a Shrinking Felinor whose base stats are this one's
  −{shrinkAttack}/−{shrinkHealth}.
- **Radiant:** Death: Summon a Radiant Shrinking Felinor whose base stats are this one's
  −{shrinkAttack}/−{shrinkHealth}.
- **Engine:** the Death reads this card's §10.4 layer-1 stats off its last-known state (R78): its
  `statsOverride`, else its printed face. It summons a fresh `meditative-094` (not a copy: no buffs,
  keywords or `tuning` carry, R57) on the same face, with `statsOverride` (max(0, attack −
  shrinkAttack), health − shrinkHealth), into the leftmost open unit zone (R64). Missing: a read helper
  for layer-1 stats in `query.rs` (small).
- **Rulings:** **MD-E20:** a summon whose base health would be 0 or less does not happen, which ends the
  chain: base 9/11, 6/8, 3/5, 0/2 (four bodies); Radiant 18/22, 16/19, 14/16, 12/13, 10/10, 8/7, 6/4,
  4/1 (eight). The shrink can't be tuned below 1, so the chain always ends. A Shrinking Felinor made
  Radiant on the field keeps its `statsOverride` and shrinks by the Radiant numbers from then on.
- **Numbers:** `shrinkAttack` 3 → 2 ↓ (min 1); `shrinkHealth` 3 → 3 ↓ (min 1).
- **Check:** the designer's Radiant face summons "a Shrinking Felinor"; its −2/−3 means something only
  if the chain carries it, so the summoned card is Radiant too and the text says so. **⚠ designer.**
- **Class:** B.

#### Meditative #99 · Paranoia
`meditative-099` · (2) Field Spell · Epic

> **Designer:** Your Spells may be played as Traps that activate at the End of your turn, Start of your
> next turn, or End of your next turn · Draw 1 ~~~ Your Spells may be played as Traps that activate at
> the End of your turn, Start of your next turn, or End of your next turn · They also gain Echo +1 ·
> Draw 1

- **Text:** Aura: You may play your Spells face-down as Traps. Each reveals at the time you choose as you
  play it: the end of this turn, the start of your next turn, or the end of your next turn.
  Cry: Draw {draw|card|cards}.
- **Radiant:** Aura: You may play your Spells face-down as Traps. Each reveals at the time you choose as
  you play it: the end of this turn, the start of your next turn, or the end of your next turn. Each has
  Echo +{echo}.
  Cry: Draw {draw|card|cards}.
- **Engine:** **ME-ALTPLAY** (shared with Knowledge Breaker). A permission hook (as `graveyard_play`
  is) live while Paranoia acts on its controller's field: `legal_actions` also offers each Spell they
  could play, from hand or under a graveyard permission, as a face-down play into an empty, unlocked
  backrow zone at its price, the timing a mode in the `play` action (R81), with X or embiggen chosen
  then. It is announced as a Trap (R448), takes a fresh id (R227) and shows its cost face-down (R351);
  the timing is its controller's alone. It reveals at its point: the end-of-turn trap window (§2.2,
  R62, Bread and Butter's), or the start of the next turn as traps answer it, before the start-of-turn
  triggers (C #88 Siphon Squad's "Start of turn: Reveal"), or the next end-of-turn trap window. Then
  its Spell text resolves, its declared targets and modes asked as prompts in declaration order (R467's
  way), Echo repeats with fresh prompts (Radiant: +`echo`, as Twinspell's grant); then it goes to its
  owner's graveyard as its printed Spell (R766). The Cry is its unlabelled draw (R408).
- **Rulings:** **MD-E17:** a Spell played this way is a Trap play for every rule that reads plays and
  prices (C #6's "Your Traps cost (0)", C #2's next-Trap discount, play counts by type, Combo), and a
  Trap on the field until it reveals (backrow and Trap effects reach it; a Counter for Spells doesn't
  see it); its effects come from a Trap (no Spell Damage, and Immune to Spells doesn't stop them); it
  needs no legal target when set (R703 is read as it reveals, where it fizzles); "this turn" in its text
  is the turn it reveals in. **MD-E18:** a set Spell stays set and reveals on time after Paranoia leaves
  the field; bounced or destroyed first, it is its printed Spell again in hand or graveyard; one Radiant
  Paranoia acting as it is set gives it Echo +1, however many permit the play.
- **Numbers:** `draw` 1 → 1 ↑; `echo` 1 (Radiant only, `tunedOn: "radiant"`) ↑.
- **Check:** the designer's second #93 is #99. "Activate" is read as reveal, not R384's Activate.
- **Class:** C (ME-ALTPLAY).

---

#### Group E's systems

- **ME-HANDCAP** (#79 sets it; #77 and #80 read it, as does every draw and add). `PlayerState` gains
  a hand cap (absent = `HAND_CAP`, 10) that an effect sets for the rest of the game, the latest
  setting winning (MD-E4). Every reader of the constant asks `hand_cap_of(state, player)` instead:
  draws (`draw.rs`), adds (`add_to_hand.rs`), steals and counters into a hand (`ownership.rs`,
  `move_.rs`), `draw_while`'s bound and KY's Test's fill. "Fill your hand" and "your hand is full" read
  it too. Both players' caps are public in the view, and the client lays out up to `HAND_CAP_MAX` (14)
  cards, a phone included.
- **NEW: ME-HANDMARK** (#76). A delayed effect (R62, R76) may watch several cards in a hand, each
  dropped as its stay in that hand ends (R174), and mark each (R437 on hand cards): the owner sees
  the marks, the other player's hand count carries how many (MD-E2). At its point it runs with the
  cards still watched, and `give_from_hand` gains a selector by instance ids to steal them (R12).
- **ME-TURN** (#89). Besides "next turn" mana, a per-player repayment schedule, `owedInstalments`:
  the instalment due at each of that player's next refreshes, next first, each refresh taking its
  share off what it gives, mana floored at 0 (MD-E13, rewritten; R1224). Jlarna's credit line borrows
  past current mana up to its limit through the one affordability function (`spendable_mana`,
  R1223), its instalments go on that one schedule, and a missed one is forgiven. Extra turns count as
  turns.
- **NEW: ME-TUNEMULT** (#84). A static flag `tuneMultiplier { upgrade?, degrade? }` read off a card's
  running face in every zone, which `tune_effect` passes to `tune_once`. Each menu row then scales by
  it: N cost steps, N × `TUNE_STAT_TOTAL`, N keywords, N X steps, N number steps, still one draw and one
  event per application (MD-E7). The `upgraded` and `degraded` change payloads carry a list where a
  row moves several keywords. A fused card's multipliers multiply.
- **ME-TRIBAL** (#87; #78 is tribal hate). `TRIBAL_TAGS` in `config.rs` = Human, Felinor, KY, CN,
  Jlockeed (MD-E11). "All Tribes" is every tribal tag in the catalog's `tags`, plus an `allTribes`
  display flag that a cross test holds to the full list, so no tag reader changes. "A card with a
  tribal tag" is a running face whose tags meet `TRIBAL_TAGS`. "Tribal tag based hate" is an effect
  that picks cards by having or lacking a tribal tag (Bigot, #78, Meditative #34), which Meditative
  #25's immunity reads.
- **ME-RANDOMTARGETS** (#86). While a Mayor acts on the field, both players' targets are drawn by the
  engine (MD-E9): `legal_actions` offers plays, activations and attacks without targets, the reducer
  draws declared targets at §10.5 step 1 and an attack's from its legal targets, and `target` prompts
  are answered at once from the match rng (R452's machinery, `random_cast.rs`). The view flags it so
  the client asks for no target. Lucky narrows the Mayor controller's own rolls (MD-E10).
- **NEW: ME-ATTACKSUMMON** (#91, #91.1). Two hooks on the attack pipeline (§4.2). A replacement:
  when this would attack, a Unit summoned from hand makes that attack instead as a forced attack, and
  afterwards may be bounced (Windfast, MD-E14). A rider: when this attacks, summoned joiners attack its
  target first as forced attacks, then its own combat, and a declared attack whose target is gone is
  cancelled (Windfurious Prime, MD-E15). Both run on declared and forced attacks, and each combat has
  its own state check (R53).
- **NEW: ME-LETHALGUARD** (#92). §4.4 step 4a opens for Units as well as heroes: a hit that alone would
  leave a Unit at 0 or less after steps 0–4 meets `lethalHit` replacements. `ReplacedEvent::LethalHit`
  gains a unit target, and `instead.redirect` gains `"self"`, a new instance at the replacing card
  through its own pipeline. R460 keeps each card to once per event, and `DAMAGE_REDIRECT_CAP` bounds
  chains (MD-E16).
- **NEW: ME-UNTRIBUTABLE** (#80). Keyword `Untributable`: the play validator's and the activation's
  Tribute choices refuse it, and every Sacrifice of it, including `tribute_when`'s, does nothing. It
  is not in R21's pool, is in `TUNE_HARMFUL_KEYWORDS`, and has a glossary row in §6.1 (MD-E5).
- **NEW: ME-TRIBUTEPOOL** (#88). `catalog.query({ tribute: true })`: the non-token definitions whose
  registered script declares a Tribute X play cost (`static_flags.tribute`). The pool then keeps up
  with every new Tribute card (MD-E12).
- **ME-ALTPLAY** (#99). A permission hook that lets a Spell be played face-down as a Trap with a chosen
  reveal point (a play mode in the action). The instance plays as a Trap until it resolves, reveals at
  its point, and resolves its Spell text with prompts for its choices (MD-E17, MD-E18). It is shared
  with Knowledge Breaker's Unit played as an Animated Field Trap.
- **Existing systems these cards use:**
  - Buff and Nerf: #82–#85 and #87 (R386, R440, R442).
  - Deck triggers: #87 (R464).
  - Delayed effects and marks: #76 (R76, R437).
  - Steal off the field: #76 (R12).
  - Catalog pools: #77, #78, #88, #90 (R380, R382, R387, R60).
  - Library copies: #81 (R57, R311).
  - Embiggen: #81 (§2.3).
  - Named Death summons: #93–#94 (R64).
  - Discover: #88 (§6.3).
  - Activate: #80 (R384).
  - Quickdraw: #80 (R640).
  - Windfury: #91, #91.1 (R636).
  - Forced attacks: #91, #91.1 (R53).
  - Replacements: #92 (R460).
  - Lucky: #86.
  - Mode prompts: #85.
  - Voice lines: #79, #80 (R204, `apps/web/src/audio/card-audio.json5`).
- **Small engine additions:**
  - `draw_priced` gains a `radiant` rider (#79).
  - `give_from_hand` gains a selector by instance ids (#76).
  - `query.rs` gains a layer-1 stats read (#94).

### Group F: Call to Chaos (Meditative Edition), Meditative Journey, Jlockheed's Evil Blueprints

Fourteen entries: #95 and its token #95.1, #96 and its token #96.1, #97 and the nine Blueprint
buildings #97.1–#97.9 (the designer's "#100.1" Empty Plot is #97.1). Every token here has `rarity:
"Token"`, the tag `Token` and the designer's rarity as `printedRarity` (§7's Classic+ rule). Script
files: `crates/cards/src/scripts/meditative/c095_call_to_chaos_meditative_edition.rs`,
`c095_1_cn_golem.rs`, `c096_meditative_journey.rs`, `c096_1_journey_complete.rs`,
`c097_jlockheeds_evil_blueprints.rs`, `c097_1_empty_plot.rs` … `c097_9_jlockheeds_headquarters.rs`.

---

#### Meditative #95 · Call to Chaos (Meditative Edition)
`meditative-095` · (4) Spell, Call to Chaos · Legendary

> **Designer:** ??? · (One of the following random effects) · Fuse your entire hand into one card, add
> two more copies of it to your hand, they costs(0) · Add 3 CN cards to your hand, they cost (0) · Add 2
> Prime cards to your hand, they cost (0) · Your hero gains 8 Armor & heal your hero 8 health · Summon a
> Jade Beauty · Summon 3 random Acclaimed cards · Bounce your opponents field, then degrade all the cards
> · Summon a CN Golem · For the rest of the game: Start of turn cast a random Call to Chaos · Cast a
> random Call to Chaos ~~~ !!! · (three effects)

- **Text:** One random effect: fuse your hand into one card and add 2 copies of it to your hand, all
  three costing (0); add 3 random CN cards to your hand, which cost (0); add 2 random Prime cards to
  your hand, which cost (0); your hero gains 8 Armor and you heal it 8; summon a Jade Beauty; summon 3
  random Acclaimed cards; Bounce every enemy permanent, then Nerf each card bounced; summon a CN Golem;
  for the rest of the game, at the start of your turn, cast a random Call to Chaos; cast a random Call
  to Chaos.
- **Radiant:** Three different random effects, resolved in the order listed: fuse your hand into one
  card and add 2 copies of it to your hand, all three costing (0); add 3 random CN cards to your hand,
  which cost (0); add 2 random Prime cards to your hand, which cost (0); your hero gains 8 Armor and you
  heal it 8; summon a Jade Beauty; summon 3 random Acclaimed cards; Bounce every enemy permanent, then
  Nerf each card bounced; summon a CN Golem; for the rest of the game, at the start of your turn, cast a
  random Call to Chaos; cast a random Call to Chaos.
- **Engine:** a third table on Core's roll, as C+ #73 does it: `subsystems/call_to_chaos_meditative.rs`
  holds `CHAOS_MED_EFFECTS`, ten `ChaosEffectDef { name, label, build }`, each built as it resolves
  (`resolve::lazy_part`, so each reads the board after the ones before it, R87, and a prompt parks the
  rest, R113); the card file is C+ #73's one-liner, `call_to_chaos({ radiant, table:
  CHAOS_MED_EFFECTS })` on each face. The roll happens as the Spell resolves (match rng), and
  `chaosRolled` names the labels to both players first (R436). The entries, in list order:
  (1) **hand fuse** (MD-F3): `fuse_cards { instanceIds: <hand>, toHand: self, handPrice: free }`, then
  two copies of the result into your hand (NEW in CHAOS-MED: a copy-to-hand with R57's riders), each
  `costOverride` 0; the `fused` event is hidden as the hand is (R470). (2) `add_random_from_catalog
  { query: { tags: [CN] }, count: 3, costOverride: 0 }`, non-token CN cards of every set (R380, §5.1),
  repeats allowed (R60), hand cap burns (§2.4). (3) The same over the Prime pool, tokens in (MD-F4,
  NEW: PRIME-POOL). (4) `gain_hero_armor { amount: 8 }` (the hero's own per-hit Armor for the rest of
  the game, §4.4 step 2, R124) then `heal` your hero 8 (no cap). (5) `summon` the Jade Beauty token
  `meditative-039-5` (ME-JADE's token; its End of turn is ME-ALLURE), placed per R64, nothing on a full
  row. (6) Three `summon_random { query: { tags: [Acclaimed], type: [Unit, Field Spell, Trap, Field
  Trap] } }` (MD-F5). (7) Bounce and Nerf (MD-F6): snapshot the enemy permanents, `bounce_all { side:
  enemy, rows: [units, backrow] }`, then `degrade { instanceId, times: 1 }` on each snapshot id now in
  a hand.
  (8) `summon` the CN Golem `meditative-095-1`. (9) `for_rest_of_game` (R458) with an engine-owned
  delayed hook (`@chaosEternal`, run by `effects::delay::run_engine_delayed` as its destroy and discard
  hooks are) that runs `cast_random_call_to_chaos()`; capped (MD-F2). (10) `cast_random_call_to_chaos`
  (existing). Lookups of the two tokens go by `(SetName::Meditative, index)` (§5). Every count is a
  `config.rs` constant: `CHAOS_MED_FUSE_COPIES` 2, `CHAOS_MED_CN_CARDS` 3, `CHAOS_MED_PRIME_CARDS` 2,
  `CHAOS_MED_HERO_ARMOR` 8, `CHAOS_MED_HEAL` 8, `CHAOS_MED_ACCLAIMED` 3, `CHAOS_MED_NERFS` 1,
  `CHAOS_MED_COST` 0, `CALL_TO_CHAOS_ETERNAL_CAP` 3 (rule 9).
- **Rulings:** MD-F1, MD-F2, MD-F3, MD-F4, MD-F5, MD-F6. "Your hero gains 8 Armor" is §6.1's Armor on
  the hero (each hit on it is reduced by 8, Pierce excepted), kept for the rest of the game as C+ #46's
  hero Armor was, the heal adding 8 health with no cap. "Summon a Jade Beauty" summons the token's base
  face and does not touch the Jade Counter; ME-JADE's "ascends at 10" reaches it as it reaches any Jade
  Beauty you control. The cards the recursion and the rest-of-game effect cast are base Calls (R28);
  R87's details hold for this edition as for #95 and C+ #73. In play the rules box reads "???", and
  "!!!" on the Radiant (the tag's rule, §10.10).
- **Numbers:** none, as #95 and C+ #73: the counts are the `config.rs` constants above.
- **Check:** the family "Call to Chaos (___ Edition)" is Legendary, as printed. R275/R276: three
  different effects (R423) ✓. "they costs(0)" is "they cost (0)", and "they" is all three cards (C+ #43
  AI Slop's fused card costs (0) too, so the fused original is not left at (4)); "opponents" is
  "opponent's"; "degrade" is Nerf. **⚠ designer:** 8 Armor that stays on the hero stops every hit of 8
  or less for the rest of the game (the designer's own C+ hero Armor gave +1, Radiant +2); if a
  Hearthstone-style 8-point pool was meant, the reading becomes "heal your hero 16". Termination: the
  fuzz suite's random decks must include this card (B10.3 of `docs/classic-sets.md`).
- **Class:** D (subsystem: `call_to_chaos_meditative`, CHAOS-MED).

#### Meditative #95.1 · CN Golem
`meditative-095-1` · (4) Unit, CN, Token · Token (printed Legendary) · 10/10 → 20/20

> **Designer:** 10/10 · Rush, Poisonous, Cleave, Pierce · On kill shuffle a CN-Virus into your
> opponents deck ~~~ 20/20 · Rush, Poisonous, Cleave, Pierce, Windfury · On kill shuffle a Radiant
> CN-Virus into your opponents deck

- **Text:** Rush, Poisonous, Cleave, Pierce
  Whenever this destroys a Unit, shuffle {viruses|CN-Virus|CN-Viruses} into your opponent's deck.
- **Radiant:** Rush, Poisonous, Cleave, Pierce, Windfury
  Whenever this destroys a Unit, shuffle {viruses|Radiant CN-Virus|Radiant CN-Viruses} into your
  opponent's deck.
- **Engine:** keywords (§6.1: Rush; Poisonous, §4.4 step 7; Cleave, step 10; Pierce, step 2; Windfury,
  `WINDFURY_ATTACKS` 2, R636). A kill trigger, C+ #19.5 Bot Loser's: a `TriggerDef` on `destroyed`
  whose `killerId` is this (R42) returns `shuffle_into { defId: "core-090-1", count: viruses, player:
  enemy, radiant }` (Radiant face). The virus is the opponent's card in their deck (#90's ownership),
  shuffled in by a public trigger, so it is recorded in their list (R311); a full deck refuses it
  (R80).
- **Rulings:** MD-F7.
- **Numbers:** viruses 1 → 1 ↑.
- **Check:** R275 ✓ (20/20, Windfury, the virus Radiant). It is the first card to print Windfury, so
  §6.1's Windfury row ("No card prints it yet") names it. Distinct from Core #95.1 Chaos Golem and C+
  #73.1 Classic Golem (R739). "opponents" is "opponent's".
- **Class:** B.

#### Meditative #96 · Meditative Journey
`meditative-096` · (2) Spell · Rare

> **Designer:** Select up to two cards in your hand to go on a journey {exile them and a shuffle a
> Journey Complete into your deck} ~~~ Select up to five cards in your hand to go on a journey {exile
> them and a shuffle a Journey Complete into your deck} · Draw 1

- **Text:** Choose up to {cards|card|cards} in your hand to go on a journey: exile them and shuffle a
  Journey Complete into your deck.
- **Radiant:** Choose up to {cards|card|cards} in your hand to go on a journey: exile them and shuffle a
  Radiant Journey Complete into your deck. Draw {draw|card|cards}.
- **Engine:** as the Spell resolves, `choose_pick { from: [your hand], min: 0, max: param("cards") }`
  (B5 E18's `pick` prompt, the chooser's alone, §10.8; a resolution prompt rather than a declared hand
  pick, because a declaration's `max` is static and `cards` must tune, R386). Its resume step exiles
  each picked card (§6.3 Exile, counted by the game's exile counter; a unit-token card ceases to exist
  instead, R11), then shuffles one Journey Complete (`meditative-096-1`, Radiant on the Radiant face,
  MD-F9) into your deck carrying `memory.journey`: the exiled cards' instance ids in the order they were
  exiled (NEW: SHUFFLE-MEMORY). The shuffle-in is public and recorded in your list (R311). The Radiant
  face then draws.
- **Rulings:** MD-F8, MD-F9.
- **Numbers:** cards 2 → 5 ↑; draw (Radiant face only) 1 ↑.
- **Check:** "and a shuffle a" is "and shuffle a". R275 ✓ (2.5 times the cards, a draw, and MD-F9's
  discount on the way back).
- **Class:** C (SHUFFLE-MEMORY).

#### Meditative #96.1 · Journey Complete
`meditative-096-1` · (2) Spell, Token · Token (printed Rare)

> **Designer:** Cast of draw: add a ascended cards back to your hand [Radiant versions of the two cards
> that were exiled] ~~~ Cast of draw: add a ascended cards back to your hand, they cost (1) less
> [Radiant versions of the cards that were exiled]

- **Text:** Cast on draw: Return the cards that went on this journey from exile to your hand. They
  become Radiant.
- **Radiant:** Cast on draw: Return the cards that went on this journey from exile to your hand. They
  become Radiant and cost ({discount}) less.
- **Engine:** `staticFlags.castOnDraw` (§6.2; a Cast, R40, R70; chains capped by R58). The script reads
  `memory.journey` and, for each id in order whose card is still in its owner's exile, `add_to_hand {
  instance, radiant: true, costMod: -discount (Radiant face) }`: the move goes to the owner's hand
  (R746, R12), the card is made Radiant (§6.3 Make Radiant, R74) and keeps its `tuning`; the hand cap
  burns the rest (§2.4). The price lands after R766 has reset it in exile, so the Radiant face's card
  costs its printed cost less 1.
- **Rulings:** MD-F10.
- **Numbers:** discount (Radiant face only) 1 ↑.
- **Check:** "Cast of draw" is Cast on draw, "add a ascended cards" is "add the ascended cards". The
  base's "two" and the Radiant's "the cards" are read by MD-F10 (all that went, two or five) and MD-F9
  (the five go with the Radiant Journey Complete). R275 ✓ (a tangential rider: the discount).
- **Class:** B.

#### Meditative #97 · Jlockheed's Evil Blueprints
`meditative-097` · (0) Spell, Jlockeed · Mythic

> **Designer:** Piece together the blueprint ~~~ Piece together the Radiant blueprint · {for ai:
> discover one of the following 9 units}

- **Text:** Piece together the blueprint: Discover one of Empty Plot, Wishing Well, School, Mega
  Church, Bunker, University, The Great Wall, Prison or Jlockheed's Headquarters.
- **Radiant:** Piece together the Radiant blueprint: Discover a Radiant one of Empty Plot, Wishing
  Well, School, Mega Church, Bunker, University, The Great Wall, Prison or Jlockheed's Headquarters.
- **Engine:** `discover_from_catalog { query: { defId: [meditative-097-1 … -9], withTokens: true } }`,
  three different options drawn uniformly from the nine (§6.3 Discover, R60), shown to the chooser only
  (§10.8); the resume step adds the pick to your hand (`add_to_hand { defId, radiant }`, Radiant on the
  Radiant face), the hand cap burning it (§2.4). The nine are tokens, so they are in no other pool but
  C+ #23 Dropshipping's (R382).
- **Rulings:** MD-F11.
- **Numbers:** none (the Discover's 3 is the keyword's, not printed).
- **Check:** Mythic stands: the "Jlockeed ___" Common family is Core #13 and #14's naming and shape,
  which C+ #48, #51 and #52 also left. Tag `Jlockeed` (R278), so the Blueprints join the pools of C+ #48
  (Death) and C+ #52 (the contract). `refs` list the nine (R279); "School", "Prison", "Bunker" and
  "University" are plain words, so the builder checks R279's proof for false hits in other texts and
  adds rules words to R381's list if needed. R275 ✓ (Radiant buildings, as C+ #48's "Radiant Jlockheed
  card").
- **Class:** A.

#### Meditative #97.1 · Empty Plot
`meditative-097-1` · (0) Unit, Token · Token (printed Common) · 0/3 → 0/8

> **Designer:** [0/3] · Can't attack · Cards may be stacked on this {they have stack} ~~~ [0/8] · Can't
> attack · Cards may be stacked on this {they have stack}

- **Text:** Can't attack
  You may play a Unit on top of this, as if it had Stack.
- **Radiant:** Can't attack
  You may play a Unit on top of this, as if it had Stack. When you do, Buff that Unit
  {buffs|time|times}.
- **Engine:** NEW: STACK-BASE, a static flag `stackBase`. While the Plot acts (the top of its unit
  zone), `play_choices::legal_zones_for` also lists its zone for any Unit its controller plays, under
  `zones::accepts_stack_card`'s checks (not Locked, R688; not reserved, R64), and `refuse_zone` reads the
  same predicate. The Unit tops the pile (§3.2); the Plot lies dormant beneath it (R13) and resumes when
  the pile above it is gone. Radiant: as the Unit is placed (§10.5 step 4), before its Cry, it gets
  `upgrade { instanceId, times: buffs }` (R386), the Plot's flag read at placement since the Plot is dormant by the
  time any trigger could answer. "Can't attack" is the attack validator flag.
- **Rulings:** MD-F12.
- **Numbers:** buffs (Radiant face only) 1 ↑.
- **Check:** **⚠ designer:** the printed Radiant face changes only health (0/3 → 0/8), which R275 does
  not accept for a Unit with text; the Buff on the stacked Unit is the smallest rider in the card's
  direction. Numbered #100.1 by the designer; #97.1 here.
- **Class:** C (STACK-BASE).

#### Meditative #97.2 · Wishing Well
`meditative-097-2` · (1) Unit, Token · Token (printed Common) · 0/6 → 0/12

> **Designer:** [0/6] · Can't attack · Activate: 10% chance to add a random Radiant card to your hand
> ~~~ [0/12] · Can't attack · Activate: Lucky 1 12% chance to add a random Radiant card to your hand

- **Text:** Can't attack
  Activate: {chance}% chance to add a random Radiant card to your hand.
- **Radiant:** Can't attack, Lucky 1
  Activate: {chance}% chance to add a random Radiant card to your hand.
- **Engine:** Activate (§6.2, R384: once per turn, its controller's main phase, sickness irrelevant).
  The roll is `rng.chance(chance / 100)`, the Radiant face's `rng.lucky(1, …)` keeping a success (§6.1
  Lucky, #23's pattern); a success is `add_random_from_catalog { radiant: true }`, any non-token card of
  every set (R380, §5.1), hidden in your hand (R97), burned by a full hand (§2.4).
- **Numbers:** chance 10 → 12 ↑ (step 2). Lucky 1 is a numbered keyword (R386).
- **Check:** R275 ✓ (Lucky 1 at 12% is 22.6%, 2.26 times 10%; 0/12).
- **Class:** B.

#### Meditative #97.3 · School
`meditative-097-3` · (2) Unit, Token · Token (printed Rare) · 0/8 → 0/16

> **Designer:** [0/8] · Can't attack · Activate: summon a random (1) cost unit ~~~ [0/16] · Can't
> attack · Activate: summon a random Radiant (1) cost unit

- **Text:** Can't attack
  Activate: Summon a random ({cost}) Cost Unit.
- **Radiant:** Can't attack
  Activate: Summon a random Radiant ({cost}) Cost Unit.
- **Engine:** Activate (R384); `summon_random { query: { type: Unit, cost }, radiant }`: one uniform pick
  of the non-token Units of every set at that printed cost (R380, R60), placed per R64 (a Locked zone
  only when no open one is left, R688), no Cry (R1); no zone to land in draws nothing (R129).
- **Numbers:** cost 1 ↑.
- **Check:** R275 ✓ (0/16, the Unit Radiant).
- **Class:** B.

#### Meditative #97.4 · Mega Church
`meditative-097-4` · (3) Unit, Token · Token (printed Rare) · 0/5 → 0/10

> **Designer:** [0/5] · Can't attack · Tribute 5 (can tribute any card {owned by any player} that costs
> (1) or less) · Activate: Take control of an enemy Permanent ~~~ [0/10] · Can't attack, Divine Shield ·
> Tribute 5 (can tribute any card {owned by any player} that costs (1) or less) · Activate: Take control
> of an enemy Permanent, make it Radiant

- **Text:** Tribute 5, Can't attack
  This may Tribute any permanent on either side that costs ({cheap}) or less.
  Activate: Take control of an enemy permanent.
- **Radiant:** Tribute 5, Can't attack, Divine Shield
  This may Tribute any permanent on either side that costs ({cheap}) or less.
  Activate: Take control of an enemy permanent. It becomes Radiant.
- **Engine:** NEW: WIDE-TRIBUTE, a static flag (`tributeCheap: cheap`) that `legal_tribute_units`
  reads: besides your own Units, every permanent acting on the field on either side (unit tops,
  backrow cards, face-down ones included, never a dormant card, R13) whose cost is `cheap` or less
  (R65; an X card its X, R396) may pay, each worth 1 (a Sheep Token its 2 or 3); the set is minimal
  (R101) and travels in the play (R81), and the payment is §10.5 step 2's Sacrifice. Activate (R384):
  one declared target, an enemy permanent acting on the field, in the `activate` action; `steal`
  (§6.3; same lane if free, else the first free zone, else it stays, R15; R171's entry), then on the
  Radiant face `set_radiant` on it (Core #49's two faces).
- **Rulings:** MD-F13.
- **Numbers:** cheap 1 ↑. Tribute 5 is a numbered keyword (R386: less is better).
- **Check:** R101 lists every minimal set (R90): up to C(20,5) = 15,504 sets with twenty permanents
  out, so the builder measures `legal_actions` and the AI's budget on a full board. R275 ✓ (Divine
  Shield, the stolen card Radiant).
- **Class:** C (WIDE-TRIBUTE).

#### Meditative #97.5 · Bunker
`meditative-097-5` · (2) Unit, Token · Token (printed Epic) · 5/10 → 10/30 (designer 5/30; R275's stat half doubles the attack)

> **Designer:** Tribute 2 · [5/10] · Armor 3, Can't attack, First strike · Has triple the attack
> against units in the same lane ~~~ Tribute 2 · [5/30] · Armor 5, Can't attack, First strike · Has
> triple the attack against units in the same lane

- **Text:** Tribute 2, Armor 3, Can't attack, First Strike
  This strikes Units in its lane with {multiplier} times its Attack.
- **Radiant:** Tribute 2, Armor 5, Can't attack, First Strike
  This strikes Units in its lane with {multiplier} times its Attack.
- **Engine:** NEW: LANE-STRIKE. In `combat::resolve_combat`, where both attacks are read once (R94), a
  combatant whose face declares the lane multiplier has its attack multiplied by `multiplier` when the
  other combatant stands in the opposing unit zone of its own lane (§3.1); the hit then goes through
  §4.4 as usual (Divine Shield, the target's Armor). It strikes back when attacked, first (First
  Strike, §4.3), and in any forced attack (which skips "Can't attack", §4.2). Cleave hits, which land
  in other lanes, and every non-combat damage are not multiplied.
- **Rulings:** MD-F14.
- **Numbers:** multiplier 3 ↑ (step 1). Armor 3 → 5 and Tribute 2 are numbered keywords.
- **Check:** **⚠ designer:** the printed Radiant 5/30 does not double the attack, and R275's stat half is
  held by a catalog-wide test with no exceptions left; 10/30 is the smallest fix (30 in its lane). The
  view may show the in-lane number through a `preview` (R280). "First strike" is First Strike.
- **Class:** C (LANE-STRIKE).

#### Meditative #97.6 · University
`meditative-097-6` · (4) Unit, Token · Token (printed Epic) · 0/12 → 0/24

> **Designer:** [0/12] · Can't attack · Activate: Buff all friendly permanents twice ~~~ [0/24] · Can't
> attack · Activate: Buff all friendly permanents five times

- **Text:** Can't attack
  Activate: Buff each of your permanents {times|time|times}.
- **Radiant:** Can't attack
  Activate: Buff each of your permanents {times|time|times}.
- **Engine:** Activate (R384); `upgrade { scope: { side: self, zones: [field] }, times }` (R386): each
  permanent acting on your side (unit tops, backrow cards, face-down ones, this one included), each
  application its own draw; an Immutable card is left alone (R23); a face-down card's applications are
  reported as R440 reports hidden ones.
- **Numbers:** times 2 → 5 ↑.
- **Check:** R275 ✓ (2.5 times, 0/24).
- **Class:** B.

#### Meditative #97.7 · The Great Wall
`meditative-097-7` · (2) Unit, Token · Token (printed Epic) · 0/50 → 0/100

> **Designer:** Tribute 3 · Immutable · Can't attack · [0/50] · Cry: Lock all friendly unit tiles ~~~
> Tribute 3 · Immutable · Armor 2 · Can't attack · [0/100] · Cry: Lock all friendly unit tiles

- **Text:** Tribute 3, Immutable, Can't attack
  Cry: Lock every unit zone on your side.
- **Radiant:** Tribute 3, Immutable, Armor 2, Can't attack
  Cry: Lock every unit zone on your side.
- **Engine:** the Cry (played, R1) is five `lock { zone: { of: "lane", row: "units", lane: 1…5 } }`
  (`effects/counters.rs`; or a one-line `lock_all(ZoneScope)` beside `unlock_all`), each emitting
  `locked`, a zone Locked already skipped, occupied zones included since a Lock evicts nothing (§3.2).
  Immutable (R23). The Locks outlast the Wall.
- **Rulings:** MD-F15.
- **Numbers:** none (Armor 2 and Tribute 3 are keywords).
- **Check:** "tiles" are unit zones. R275 ✓ (0/100, Armor 2). The Lock is the Wall's price; M #27
  Clip-Farming Lawyer's Unlock and C+ #77 Anti-Softlock pay it back.
- **Class:** B.

#### Meditative #97.8 · Prison
`meditative-097-8` · (4) Unit, Token · Token (printed Legendary) · 0/16 → 0/32

> **Designer:** Tribute 2 · [0/16] · Every Unit your opponent plays has a 50% chance of being stacked
> under this {for the ai: giving you control of it} ~~~ [0/32] · Every Unit your opponent plays has a
> Lucky 50% chance of being stacked under this {for the ai: giving you control of it}

- **Text:** Tribute 2
  After your opponent plays a Unit: {chance}% chance to stack it under this. You control it.
- **Radiant:** Tribute 2, Lucky 1
  After your opponent plays a Unit: {chance}% chance to stack it under this. You control it.
- **Engine:** NEW: CAPTURE. A trigger on `cardResolved` of a Unit its controller's opponent played or
  cast (R70), in the window Sheepish uses (after the Unit resolves, its Cry included, §10.5 step 7,
  R427), while Prison acts on the field: `rng.chance(chance / 100)` (Radiant `rng.lucky(1, …)`). On a
  success the Unit, if still on the field on that stay (R174), moves from the top of its zone to directly
  beneath Prison in Prison's pile without leaving the field (no R78 reset, damage kept), dormant (R13),
  and its controller becomes Prison's (control only, its owner unchanged, R12; an entry, R171, which
  covers dormant cards); `controlChanged` reports it, publicly. A card it uncovered on the opponent's
  side resumes there (R212). When Prison leaves the field the card beneath it resumes (§3.2), under
  Prison's controller.
- **Rulings:** MD-F16.
- **Numbers:** chance 50 → 50 ↑. Lucky 1 is a numbered keyword.
- **Check:** the printed Radiant face omits "Tribute 2"; §8's Conventions (a Radiant cell that names no
  keywords keeps the base keywords) keep it. R275 ✓ (0/32, Lucky 1 makes 75%). A captured Unit counts
  for Felinor Fiender (R13's exception).
- **Class:** C (CAPTURE).

#### Meditative #97.9 · Jlockheed's Headquarters
`meditative-097-9` · (4) Unit, Jlockeed, Token · Token (printed Mythic) · 0/20 → 0/50

> **Designer:** Tribute 5 · Indestructable · Can't attack · [0/20] · Activate: Fill your board with
> random Jlockheed cards ~~~ Tribute 5 · Indestructable · Can't attack · [0/50] · Activate 2: Fill your
> board with random Radiant Jlockheed cards

- **Text:** Tribute 5, Indestructible, Can't attack
  Activate: Fill your board with random Jlockheed Units.
- **Radiant:** Tribute 5, Indestructible, Can't attack
  Activate 2: Fill your board with random Radiant Jlockheed Units.
- **Engine:** Activate (R384; Activate 2 on the Radiant face). C+ #2 Groom Shroom's fill: for each zone
  `fill_board_zones` returns (empty, unlocked, unreserved, left to right, R64), `summon_random { query:
  { tags: [Jlockeed], type: Unit }, lane, radiant }`: the non-token Jlockeed Units of every set, today
  Core #13, C+ #48 and C+ #51 (R380; this token is out, §5.1, R387), repeats allowed (R60), summoned, so
  no Cry (R1). Indestructible (R46; never Taunt, R347).
- **Rulings:** MD-F17.
- **Numbers:** none (Activate 1 → 2 and Tribute 5 are keywords).
- **Check:** "Indestructable" is Indestructible. "Jlockheed cards" are Units because "fill your board"
  fills unit zones (R64), as C+ #2 reads "random Felinor Units". R275 ✓ (0/50, Activate 2, Radiant
  Units).
- **Class:** B.

---

#### Group F's systems

- **Call to Chaos roll** (existing: `subsystems::call_to_chaos`, R28, R87, R423, R436) — #95. Rolls one
  entry of an edition's table (three different, list order, on the Radiant face), announces them and
  resolves each as a lazy part. It needs nothing new beyond the third table; the chain counter in the
  cast card's `memory` already counts any edition.
- **NEW: CHAOS-MED** — #95. `subsystems/call_to_chaos_meditative.rs`: `CHAOS_MED_EFFECTS`, ten entries
  (above), the engine-owned delayed hook `@chaosEternal` that casts a random Call to Chaos, and the
  cap `CALL_TO_CHAOS_ETERNAL_CAP` on how many of those a player holds. It also needs a copy-to-hand
  option (`add_to_hand { copyOf }` or a sibling verb) carrying R57's riders (radiant flag,
  `statsOverride`, `tuning`) for the hand-fuse entry's two copies.
- **NEW: PRIME-POOL** — #95 (and, outside this group, #31's Radiant face and #55 Dragon Fruit). Every
  Prime-tagged card is a token (C+ #38.1, C+ #46.1, M #45.1, M #91.1), so a pool that names Prime must
  take tokens: an exception to §5.1 written beside R382's, i.e. `catalog.query { tags: [Prime],
  withTokens: true }`, with R387's self-exclusion kept.
- **ME-JADE, ME-ALLURE** — #95 (entry 5 summons the Jade Beauty token #39.5, whose End of turn is an
  Allure). The token and its text must exist before #95 can be built.
- **Rest-of-game effects** (existing: `for_rest_of_game`, R458, R127) — #95. The player's own
  start-of-turn effect, first acting at their next turn start, stacking; the cap above is the only
  addition.
- **Fuse** (existing: `fuse_cards`, R77, R102, R469, R470) — #95. A no-target fusion of the hand's
  cards into a fresh hand card at `handPrice: free`.
- **Kill trigger** (existing: R42's `killerId` on `destroyed`, C+ #19.5) — #95.1.
- **Pick prompt** (existing: `choose_pick`, B5 E18) — #96. "Up to N" cards from your hand at resolution.
- **NEW: SHUFFLE-MEMORY** — #96, #96.1. A `memory` (plain JSON) option on `shuffle_into` written on each
  new instance, so a token shuffled in can remember what made it; Journey Complete's `memory.journey`
  is the first. It must survive a JSON round trip and fold from the log like any memory (§10.1).
- **Cast on draw** (existing: §6.2, R58) — #96.1.
- **Discover from a named pool** (existing: `discover_from_catalog` with `defId` and `withTokens`) — #97.
- **Activate** (existing: `subsystems::activate`, R384, "Activate 2" included) — #97.2, #97.3, #97.4,
  #97.6, #97.9.
- **Lucky** (existing: `rng.lucky`, §6.1) — #97.2, #97.8.
- **NEW: STACK-BASE** — #97.1. A static flag that makes a card a pile's base: while it tops its unit
  zone, its controller's played Units may name that zone as if they had Stack, through the same
  predicate in `legal_zones_for` and `refuse_zone`, honouring Locks (R688) and reservations (R64). A
  Radiant rider Buffs the Unit as it lands.
- **NEW: WIDE-TRIBUTE** — #97.4. A static flag widening `legal_tribute_units` from "your Units" to
  every acting permanent on either side at or under a cost; the payment stays a Sacrifice and the set
  minimal (R101). The enumeration's size (up to 15,504 sets) is the risk.
- **Steal** (existing: `steal`, R15, R171) and **Make Radiant** (`set_radiant`) — #97.4.
- **NEW: LANE-STRIKE** — #97.5. A combat-only multiplier on a unit's attack when the other combatant
  stands in the opposing zone of its lane, applied where `resolve_combat` reads both attacks (R94),
  before §4.4. Tunable through the card's `multiplier` param.
- **Upgrade (Buff) and Degrade (Nerf)** (existing: `tune::upgrade`, `tune::degrade`, R386, R440) — #97.6,
  #97.1 (Radiant), #95 (entry 7).
- **Lock** (existing: `lock`, R688; optional `lock_all` twin of `unlock_all`) — #97.7.
- **NEW: CAPTURE** — #97.8. A move of a Unit acting on the field into another zone's pile, directly
  beneath its top, without leaving the field, with a change of control (R171) and dormancy (R13); the
  card it uncovers resumes (R212). Needs a `controlChanged` for a card going dormant and the pile
  insertion at index 1.
- **Fill the board at random** (existing pattern: C+ #2's `fill_board_zones` plus `summon_random` per
  lane) — #97.9.

### Group G: Meditative #100 to #102

Three cards the designer added to the list after this brief and the census were written (issue #571,
part MB27), and a rule the designer added with them: a coin flip is luck-based by default, and heads is
the Lucky side (MD-G4). The designer numbered them #98, #99 and #100; M2's numbering table gives them
#100 to #102. No tokens. Script files: `crates/cards/src/scripts/meditative/c100_greaser.rs`,
`c101_gachaholic.rs`, `c102_catboy_maid_ssr.rs`. Ruling proposals are MD-G1 to MD-G4.

---

#### Meditative #100 · Greaser
`meditative-100` · (2) Unit · Common · 7/7 → 21/21

> **Designer:** 7/7 ~~~ 21/21

- **Text** and **Radiant:** none. It is a vanilla Unit, and the designer gave it no tags.
- **Engine:** a script file with no hooks on either face. Its tests check both faces' stats.
- **Rulings:** none.
- **Numbers:** none.
- **Check:** R275 ✓ (21/21 is three times 7/7, as Core #8 Mr. Vanilla's and C #86 Genn's Radiant
  faces are their tripled stats).
- **Class:** A.

#### Meditative #101 · Gachaholic
`meditative-101` · (1) Unit, CN, Human · Common · 1/1 → 2/2

> **Designer:** 1/1 · Cry: Add a random Luck-based card to your hand. Give it Lucky 1. ~~~ 2/2 ·
> Activate: Add a random Luck-based card to your hand. Give it Lucky 1.

- **Text:** Cry: Add a random Luck-based card to your hand. Give it Lucky {lucky}.
- **Radiant:** Activate: Add a random Luck-based card to your hand. Give it Lucky {lucky}.
- **Engine:**
  - **MD-G1, what counts as a Luck-based card:** a non-token card that prints Lucky on either face
    (§6.1) or flips a coin (MD-G4). Those are the cards with a roll that Lucky improves.
    - The pool names no set (R1420). Until the release it holds Core #4 Gary the Gambler, Core #23
      Reoccurring Dream, Core #42 Eugenics, C #65 Ace in the Hole, C+ #25 Soul Shot, C+ #53 Book of
      Tokens, C+ #65 Two Grapes and C+ #66 Vine of Grapes. From the release on, it also holds M #36 CN
      Peptides, M #86 Mayor Medinamogger and M #102 Catboy Maid SSR+.
    - It is a query in `crates/cards/src/query.rs` (`luck_based()`), like every other random pool,
      over the catalog query's `luckBased` (`CatalogQueryArgs.luck_based`, `is_luck_based` in
      `crates/engine/src/catalog.rs`): either face's printed keywords hold Lucky, or its text says
      "Flip a coin". A cross test proves the pool is exactly the non-token cards that print Lucky or
      flip a coin.
    - The card arrives on its base face at its printed cost, hidden from the opponent. A full hand
      burns it (§2.4).
  - **MD-G2, Lucky given to a card:**
    - The given Lucky becomes the card's own: it is written on the instance, kept in hand and when the
      card is played, and shown on its face. It is `add_random_from_catalog`'s `lucky` rider, a granted
      `Lucky X` put on once the card is in the hand, as R637's Temporary is, with no event, and none on
      a burned card.
    - It adds to any Lucky the card already has, so Lucky 1 given to a Lucky 1 card makes Lucky 2.
    - Every pool card's roll must read the Lucky on its instance (printed plus given,
      `query::lucky_on`), not a number fixed on its face. Where a script reads the face, this part
      changes it. For each pool card, a test checks that its base face, given Lucky 1, rolls twice and
      keeps the better result.
  - The Radiant face's Activate follows R384: once per turn, on your turn, at no cost. It works on the
    turn the card is played, since summoning sickness does not apply.
- **Rulings:** MD-G1, MD-G2; MD-G4 for the coin-flip cards it adds.
- **Numbers:** `lucky` 1 → 1 ↑ (step 1).
- **Check:**
  - #40 Feng Shui gives a player Luck but makes no roll of its own, so it is not in the pool.
  - R275 ✓: the stats double, and where the Cry gives one card, the Radiant face's Activate gives one
    every turn.
  - "Luck-based" joins R381's rules words and gets a §6 glossary row beside Lucky.
- **Class:** B (a new pool; Lucky given to a card).

#### Meditative #102 · Catboy Maid SSR+
`meditative-102` · (1) Unit, CN, Felinor · Common · 1/1 → 2/2

> **Designer:** 1/1 · Lucky 1 · Cry: Draw 1-2. +1-4 to your Jade Counter. ~~~ 2/2 · Lucky 2 · Cry: Draw
> 1-2. +1-10 to your Jade Counter.

- **Text:** Lucky 1
  Cry: Draw 1-2. Add 1-4 to your Jade Counter.
- **Radiant:** Lucky 2
  Cry: Draw 1-2. Add 1-10 to your Jade Counter.
- **Engine:**
  - **MD-G3:** the Cry makes two separate rolls, each through `rng.lucky` with the card's Lucky
    (printed plus given, MD-G2), and higher is better for both. First it rolls how many cards to draw
    (1 or 2), as C+ #53 Book of Tokens rolls its count. Then it rolls how much to add, uniformly from
    1–4 (1–10 on the Radiant face).
  - The draws follow the usual rules: the hand cap burns the extra card, and an empty deck deals
    fatigue.
  - The amount goes through #525's ME-JADE verb. So reaching 5 summons a Jade Beauty (MD-C3), and
    reaching 10 makes it Radiant.
  - ME-LUCK (#527) adds a player's Luck to every roll that has a best, so it reaches both of these
    rolls. Whichever of this part and #527 lands second proves that.
  - Its catalog entry, script and tests, and MD-G3's note, wait for ME-JADE (#525); its census line,
    fixture row, §8.8, BUILD M10 and audit rows are written now, ceilings until the set ships (R1420).
- **Rulings:** MD-G3.
- **Numbers:** none. The ranges are not declared numbers, just as Book of Tokens' count is not. Lucky
  is a numbered keyword (R386), which a Buff's X row moves.
- **Check:**
  - "SSR+" (gacha's top rarity) stays in the name, and the file name drops the "+".
  - The rules word is "Jade Counter" (R381, MD-C2).
  - R275 ✓: the stats double, Lucky goes to 2 and the Jade roll goes up to 10.
- **Class:** B (ME-JADE).

#### Group G's systems

- **NEW: the Luck-based pool** (MD-G1) — #101. `luck_based()` in `crates/cards/src/query.rs` over the
  catalog query's `luckBased`, read off the definition, so a given Lucky never puts a card in it.
- **NEW: Lucky given to a card** (MD-G2) — #101. The `lucky` rider of `add_to_hand` and
  `add_random_from_catalog` (`effects/add_to_hand.rs`), and `query::lucky_on` (the Lucky on an
  instance: printed as tuning leaves it, plus granted). Every pool card's roll reads it: Core #23's and
  #42's Radiant faces read it where they read a fixed Lucky 1, and their base faces read it too;
  C+ #53's and M #36's `lucky_of` and `effects/fruit.rs`'s `lucky_of` (C+ #65, C+ #66) read it in
  place of the face's Lucky; C+ #25's pick already reads the keywords on its instance. #86 Mayor
  Medinamogger's roll (MB22, #538) must read `lucky_on` too when it lands.
- **NEW: coin flips take Lucky** (MD-G4) — Core #4, C #65, C+ #19.3. The flip helper `Rng::lucky_coin`,
  below.
- **ME-JADE** (#525, MB10) — #102.
- **Activate** (existing: `subsystems::activate`, R384) — #101's Radiant face.
- **Lucky** (existing: `rng.lucky`, §6.1) — #102.

#### Coin flips take Lucky (MD-G4)

The designer's rule: any card that flips a coin can take Lucky, and heads is the Lucky side.

- **A coin flip is a luck-based roll by default.** For each flip, a card with Lucky X flips X more
  coins and keeps heads if any of them lands heads. Each flip is its own roll, so each of Gary the
  Gambler's coins rolls separately.
- **Heads is the better side unless the card's text names another.** #36 CN Peptides keeps its own
  rule (the result better for the caster, MD-B17).
- **The Lucky is the flipping card's own:** printed plus given (MD-G2). So Gachaholic's Lucky 1 makes a
  Gary the Gambler or an Ace in the Hole lucky. ME-LUCK's player Luck (#527) reaches flips as well,
  since a flip now has a best. Whichever of this part and #527 lands second proves that.
- **Built once.** A flip helper beside `Rng::coin`, `Rng::lucky_coin(x)`, that is
  `rng.lucky(x, coin, prefer heads)`, as Mid Loser's `lands_heads` already was. Then:
  - `effects/coins.rs` (Gary) and every card script whose text flips a coin (Ace in the Hole, Mid
    Loser) call it with their instance's Lucky;
  - Mid Loser's `lands_heads` folds into it.
- **No shipped game changes.** With Lucky 0, the helper draws exactly one number, as `coin` does
  today. No shipped coin-flip card has Lucky, except Radiant Mid Loser, whose flip already keeps heads.
- §6.1's Lucky row states the default ("a coin flip is luck-based; heads is its better side unless the
  card says otherwise"), with this ruling's note.

## M7. Proposed rulings

Each group's proposals, numbered MD-<group><n>. The part that builds a card the proposal settles writes it
as a note with an R-number from its block (M10) and a test named after it; a proposal the build reads
differently is corrected in its note, not here.

### Group A

- **MD-A1** (#1): A forced discard is every effect discard from your hand. The guard stops it outright
  while it isn't your turn (`discardPrevented`). Discards paid as a price (R384, R450) and Temporary's
  own-turn discard are untouched, and several guards do no more than one.
- **MD-A2** (#2): The Rhino's discard is its current controller's, one random card per damage instance
  (R63, R682), and an empty hand discards nothing.
- **MD-A3** (#4, #14): "Draw a [kind of] card from your deck" takes the topmost match (Recruit's scan),
  with no rng. No match draws nothing and causes no fatigue.
- **MD-A4** (#5): The computed Echo X is fixed when the play is made, from the caster's max mana.
- **MD-A5** (#6): The gift's mana is the Unit's cost on the field, read before it moves. With no free
  zone on the opponent's side, nothing moves and no mana is gained.
- **MD-A6** (#8): The Ascent level is the times this instance was played before (R429). A copy starts
  at 0, an Echo repeat resolves the same level, and making it Radiant keeps the level.
- **MD-A7** (#8): A play gains only the current level's line, not every line up to it.
- **MD-A8** (#8): The cost is a computed cost: the level, or max(0, level − 1) on the Radiant face,
  with R65's modifiers on top.
- **MD-A9** (#8): Ascent 1's target is a resolution prompt (random under a random cast).
- **MD-A10** (#9, #10): Trigger multipliers don't stack; the highest acting one holds.
- **MD-A11** (#9, #12): Each extra trigger is its own queue entry behind the original, re-checked as it
  pops, and triggered End of turn effects are multiplied too.
- **MD-A12** (#10): A Cry is a permanent's Cry (labelled, or a Field Spell's one-time text, R408). A
  Spell's resolution isn't a Cry.
- **MD-A13** (#10): An extra Cry reuses the play's declared targets and modes, a triggered Cry reuses
  its prompt answers, and an extra Death reruns on the same snapshot.
- **MD-A14** (#12): "Your End of turn effects" are the `endOfTurn` hooks the end-of-turn step would
  queue for you now. The turn doesn't end, and the trap window and delayed effects aren't included.
- **MD-A15** (#13): "Permanently increase" is a `set_number` on the card's own param (R386 tuning),
  kept everywhere and made after the hit.
- **MD-A16** (#14): "Prime indexed" means the catalog index is prime, in any set, not a deck position.
- **MD-A17** (#18, #19): "Lose all mana next N turns" means each of your next N refreshes gives 0 and
  spends the next-turn rider. Extra turns count, gains during the turn still add, and overlaps keep
  the latest end.
- **MD-A18** (#19.1): An extra turn is a whole §2.2 turn that the turn cap and max mana count. The
  gainer's "next turn" effects land on it.
- **MD-A19** (#19.1): An extra turn is taken at the end of its gainer's next turn: at once on their own
  turn, after their next turn when gained on the opponent's.
- **MD-A20** (#19.1): Each player may take one extra turn from Temporal Rift per game. The flag is set
  when a Rift grants one, and later Rifts still resolve their other effects.
- **MD-A21** (#20): A chosen alternative condition lasts the rest of the game and can't be removed.
  Several are kept, and any one wins.
- **MD-A22** (#20): The board condition needs both the attack total and the health total at or above
  the threshold.
- **MD-A23** (#8, #20): At the state check's game-end point, a hero at 0 loses first. Otherwise a
  player with a held win wins, two winners draw, and a win effect counts at the check closing its
  list.
- **MD-A24** (#11): The first card is the head of your turn's plays (R213). Double Header's own play
  counts and copies nothing (R119), and the copy is made after resolution, fresh from the definition.
- **MD-A25** (#21): The card the end-of-turn line adds is a new base API Key Fishing, even from the
  Radiant face.
- **MD-A26** (#22): The secret choice reaches no opponent view, event or log line until it is revealed
  by its reward or a judged Fortify Mind.
- **MD-A27** (#22): The reward resolves at the start of the caster's next turn (extra turns included),
  among the start-of-turn delayed effects.
- **MD-A28** (#22.1): Fortify Mind judges the secret that made it. After that secret resolves it does
  nothing, but its discard clause still applies.
- **MD-A29** (#22.1): Its allies, enemies and penalties are those of the player who plays or discards
  it.
- **MD-A30** (#17): The cost (0 to 4) comes first, then a `craft` answer: a preset or any recipe the
  engine validates against the blocks, points, lines and type rules. An invalid one is refused and the
  prompt stays open.
- **MD-A31** (#17): The AI and the timeout answer only with the engine's presets.
- **MD-A32** (#17): The crafted card is a transient definition whose id names its recipe. It is not a
  token, it is Mythic, its set is Meditative and its `loc` is the counted lines. Its numbers are
  params, its Radiant face doubles them, and "Radiant" means the flag.

### Group B

- **MD-B1** (#25, ME-TRIBAL): "Immune to tribal tag based hate" works only while the card acts on the
  field, as Immune to Spells does (E35); in a hand or a deck it protects nothing. A tribal tag is
  Human, Felinor, KY or CN, and a card tagged All Tribes counts as each.
  - A harmful effect whose card filter names a tribal tag in `tags` or `notTags` can neither pick the
    card nor reach it. That covers a declaration or target prompt with `aim: harm`, and every harmful
    verb over a board or card scope.
  - A combat bonus that reads the defending card's tags (Meditative #52) does not apply against it.
  - Helpful effects reach it as usual: `aim: help`, buffs, heals, auras, counts and pools.
- **MD-B2** (#26, ME-TURN): while a card that says "Players don't generate mana naturally" acts on the
  field, each player's start-of-turn refresh sets max mana as §2.3 does but sets current mana to
  max(0, next-turn rider). Mana from effects (Mana Well, The Coin, Fed Fauci), the rider, Hinder and
  Refresh X work as usual.
- **MD-B3** (#26): "Summon a Mana Well in ALL open backrow" summons one Core #6 Mana Well into every
  empty, unlocked backrow zone of both players, active player's side first, lane 1 upward (R68, §7's
  "fill" reading). Each Well belongs to and is controlled by the player whose zone it is. The Radiant
  face makes the controller's Wells Radiant.
- **MD-B4** (#27): "Unlock a random zone" picks uniformly (R60) among the Locked zones of both sides
  and both rows, and "If you do" holds when one was unlocked. The Radiant face's "If there are no
  locked zones" reads the same zones before the Unlock, and its Lock picks among the opponent's
  unlocked zones in both rows. With nothing to pick, nothing is drawn (R129).
- **MD-B5** (#30): "Your opponent has a more expensive permanent than you" holds when the highest cost
  among the opponent's permanents acting on the field beats the highest among yours, this card
  included. Tops of piles and backrow cards count, each at R396's field cost, a face-down card at the
  cost both players see. With no permanent on their side it fails. "They gain control of this" is a
  Steal toward the opponent placed by R15. With no free unit zone on their side the card stays, and
  the "Otherwise" clause does not run.
- **MD-B6** (ME-CREATED): a Created card is any card instance an effect or a rule made during the game,
  that is, any instance except the cards of the decks dealt at setup or at a Glitch reset (R676). That
  includes cards added, shuffled in or summoned by an effect, The Coin, copies, and the new card of a
  Transform, a Replace or a Fuse. A card that keeps its instance keeps whether it is Created, in every
  zone (R78, R766): drawn, bounced, stolen, given, mulliganed back, or fused into as the kept card
  (R77, R470).
- **MD-B7** (#31): "Draw all Created cards from your Deck" draws each Created card in its controller's
  library, top down, as the draws begin (on the Radiant face, after the Prime card is shuffled in).
  Each is one named §2.4 draw: R58's cast-on-draw cast and the top-of-deck draw after it, the hand cap,
  and R457's draw limit. A card that has left the library by its turn is skipped, and a card that
  enters the library during the draws is not drawn.
- **MD-B8** (#31; Meditative #55, #95): a pool named by the Prime tag holds the Prime tokens, since
  every Prime card is a token. This is an exception like R382's for Fruit: Prime joins
  `POOL_TOKEN_TAGS`.
- **MD-B9** (#32): "Randomly do one of three things" is one uniform pick among the card's three effects
  by the match rng as it resolves, whatever each would do on the board then.
- **MD-B10** (#32): "Convert all cards into your hand into the same CN card" draws one definition from
  the CN pool: non-token cards of every open set, never this card (R387). It then replaces every card
  in its caster's hand, in its place, with a new card of that definition (§6.3 Replace off the field,
  R31, R35; an Immutable hand card is replaced too). Each costs (0) and is Radiant on the Radiant face.
  Each is a card generated into a hand for R673.
- **MD-B11** (ME-CN): the `chinese` flag is presentation only. No rule reads it (CLAUDE.md rule 7), and
  Immutable does not stop it.
  - It rides the card through every zone and through R78's and R766's resets, and nothing removes it.
  - A copy of a Chinese card is Chinese.
  - A Transform's or Replace's new card is not, unless its text makes it so.
  - A Fuse that keeps an instance keeps its flag, and a newly made fused card is Chinese when any
    ingredient was.
- **MD-B12** (ME-CN): the flag appears on every view of a card the viewer may read and on no hidden
  card. In a match a Chinese card shows only its Chinese name, text, type line and glossary, every
  face, with params filled. Translating cards in a hand or a deck is silent (R440), and a library list
  keeps its records (R311).
- **MD-B13** (#32, ME-CN): a face printed in Chinese keeps its Chinese in the catalog, and that Chinese
  is also its translation. The house-style proof reads it by Chinese punctuation. R277's diff compares
  it, character by character, with the other face's Chinese translation.
- **MD-B14** (#34): "non-CN or KY permanents" are permanents with neither tag, and "CN & KY permanents"
  are those with either. The Buffs come after the destroy, on the cards still standing.
- **MD-B15** (#35, granted tags): a tag an effect gives is part of the card.
  - Every instance-level tag read sees it: targets, scopes, declarations, the scripts' tag tests and
    plays counted by tag. Catalog pools do not.
  - It persists in every zone (R78, R766). A copy keeps it, a Fuse unites it with its ingredients'
    tags, a Transform drops it and a Vanilla keeps it.
  - The view lists a card's tags where they differ from its definition's.
- **MD-B16** (#35): a face-down card its chooser cannot read is a legal pick for a declaration that
  filters by tag, whatever its tags, so the legal picks reveal nothing (R440's principle). If it fails
  the filter as the card resolves, the effect fizzles on it.
- **MD-B17** (#36): CN Peptides' Lucky roll keeps the result better for the caster: the Buffs when the
  target is a Unit they control, the destroy when it is an enemy's. The target's controller as the
  Spell resolves decides which.
- **MD-B18** (#37): "When this enters your hand" answers every arrival in a hand through §2.4's
  add-to-hand: a draw, an add or Discover, a bounce or return, a gift or a steal off the field, the
  opening deal and the mulligan's replacements.
  - A card a full hand burns never entered it.
  - The replacement is a Transform of the card in that hand into a random non-token card of every open
    set except this one: Radiant, Chinese, and costing (0) on the Radiant face.
  - The replacement counts as generated (R673) and as an arrival (R151).
- **MD-B19** (#37): CN in a bottle's (X) is printed only. It is never played from a hand, it costs 0
  wherever a rule reads it out of play (R65), and a cast of it (a random cast, a Call to Chaos)
  resolves with no effect.
- **MD-B20** (#38): "a KY or CN card" is any non-token card with either tag (`anyTags`), never the
  generating card (R387).
- **MD-B21** (#98): Showdown locks every zone of the four lanes not chosen, on both sides and in both
  rows, that is not already Locked. At the start of its caster's next turn it unlocks each of those
  zones still Locked. A zone Locked before Showdown resolved stays Locked. The Lock refuses plays only
  (R688).
- **MD-B22** (#98): "Cast Book of Buff on any cards you place in it this turn": for the rest of the turn,
  after a card of yours enters one of your zones in the chosen lane, by a play once it resolves or by a
  summon of yours, you cast a new C+ #71 Book of Buff (base face) aimed at it. The cast counts as a
  play (R70) and goes to your graveyard (R87). A move (an animate, a steal, a rotation) is not placing
  a card.

### Group C

- **MD-C1** (#39.1 Auspicious Rock): Auspicious Rock makes one weighted roll (Dud 20, Jade 70, Red Jade
  10) and adds the result to its player's hand, uncast. Its Lucky (its printed Lucky plus its player's
  Luck) rolls again that many times and keeps the best, ranked Dud < Jade < Red Jade.
- **MD-C2** (#39.2 Jade, #39.4 Red Jade): The Jade Counter is a public count each player has. It starts
  at 0 and only rises. The player who resolves a Jade or a Red Jade adds to their own counter, and both
  seats see it beside that player's hero.
- **MD-C3** (#39.5 Jade Beauty): When a player's Jade Counter goes from below 5 to 5 or more, a Jade
  Beauty is summoned for them (R64, no Cry). With a full row none is, and that threshold is spent. When it
  goes from below 10 to 10 or more, every Jade Beauty they control becomes Radiant, and if they control
  none, a Radiant one is summoned. Each threshold acts once a game; one add that crosses both does 5
  first, then 10.
- **MD-C4** (#39.5 Jade Beauty, Allure):
  - Allure marks each enemy Unit on the field at that end of turn with a public `allure` mark (R437).
  - At the start of the Allurer's next turn, in lane order, each marked Unit still under the opponent's
    control is stolen (R15). With no open unit zone it is destroyed instead: Death and Reborn apply,
    and an Indestructible Unit stays where it is.
  - A Unit that leaves the field loses its mark (R174). The Allure still lands if Jade Beauty has left
    (R76), and a stolen Unit is summoning sick (R171).
- **MD-C5** (#40 Feng Shui): A card's element is the Hetu element of the last digit of its index (1 and 6
  水, 2 and 7 火, 3 and 8 木, 4 and 9 金, 5 and 0 土). An index with no digit is 土, and a fused or crafted
  card has its first ingredient's element.
- **MD-C6** (#40): The relation runs from your last played card to the card now played. It is positive
  when the last card's element generates the new one (木→火→土→金→水→木) and negative when it overcomes it
  (木→土→水→火→金→木). The same element, the reverse relations and a first play are neutral.
- **MD-C7** (#40): Every face-up play is judged and recorded, casts included (R70). Traps, Field Traps
  and any card set face-down are neither judged nor recorded, and neither is a countered play (R448). The
  record holds the element of the player's last face-up card, is never cleared, and ignores R451's AI-card
  skip.
- **MD-C8** (#40): The reward makes the card Radiant at §10.5 step 3. The punishment gives the card Brittle
  2 there and deals its damage from Feng Shui to the playing player's hero through §4.4 once the play has
  resolved. Feng Shui does not judge its own play.
- **MD-C9** (#40): The base face judges both players' plays. The Radiant face rewards only its
  controller's plays and punishes only the opponent's, for 20 damage. Each Feng Shui on the field judges
  each play on its own.
- **MD-C10** (#40): The hidden rule's numbers (`FENG_SHUI_DAMAGE` 10 and 20, `FENG_SHUI_BRITTLE` 2) are
  engine config, printed nowhere and never tuned.
- **MD-C11** (#40): Elements are not `Tag`s, and no pool, filter or "shares a tag" rule reads them.
- **MD-C12** (#40, ME-LUCK): A player's Luck X adds X to the Lucky of every roll with a "best" that their
  cards make. A roll with no best is unaffected (R32).
- **MD-C13** (#42 CN Flea Market): A night market's stall is three CN cards, two Auspicious Rocks and one
  AI generated card, rolled once as it opens and priced in yuan by printed cost and rarity. The caster
  makes one deal per answer until they leave. A bought card arrives on its base face at its printed cost,
  no lot is offered while the hand is full, and unspent yuan is lost.
- **MD-C14** (#42): To barter (Radiant face only) is to trade a card from your hand to the merchant: it is
  exiled and its price, by the same rule and doubled if it is Radiant, joins your yuan. Any number of hand
  cards may be bartered.
- **MD-C15** (#44 CN Jade Well): CN Jade Well is a Field Spell; the designer typed it Spell (R402's
  reasoning).
- **MD-C16** (#45 Knowledge Breaker, ME-ALTPLAY):
  - While the Aura acts, a Unit card in hand may also be played face-down into an empty, unlocked
    backrow zone at the Unit's price, with no declared targets or modes.
  - It is announced, placed and counted as a Trap play (R448, R227, B2.7), and while face-down it is a
    Field Trap whose own text is dormant.
  - The form ends when it leaves the field (R78), but not when the Aura leaves.
- **MD-C17** (#45, ME-ALTPLAY):
  - At its controller's start of turn (traps first) it turns face-up, animates (R383, R445) and then runs
    its Cry, with choices asked as prompts (R467).
  - It keeps its set turn as `summonedTurn`, so it may attack that turn: the exception to R83 and R171.
  - With no open unit zone it stays face-up and fires again at the next start of turn, and its Cry waits
    for the animation.
- **MD-C18** (#45, #45.1): "All other CN & KY cards" is every other card with either tag, in every zone
  its verb reaches. A Nerf reaches both fields, hands and decks; a Destroy reaches both fields. The
  Radiant Prime's Exile keeps the base face's reach (both fields).
- **MD-C19** (#45.1 Knowledge Breaker Prime): The Prime's "Animated Traps" is the same face-down play as
  Knowledge Breaker's "Animated Field Traps"; R383 makes the two wordings differ in nothing.
- **MD-C20** (#47 饕餮):
  - The ingredient is drawn uniformly from the enemy permanents on the field (tops only, both rows,
    face-down included), never an Immutable one. The Radiant face's deck card is drawn uniformly from the
    opponent's library, Immutable cards excluded.
  - It fuses per R77 with 饕餮 kept. The fused texts act as on C+ #74 (auras, start- and end-of-turn
    lines, Death and Activate work; a Cry never runs again; a Trap trigger never fires from a unit zone).
- **MD-C21** (#48 Tranquility): "Immune to damage" makes every damage instance to that hero 0 (a cap of 0
  at §4.4 step 3), fatigue included. Lose health and Set health still happen. It lasts until its player's
  next turn begins.
- **MD-C22** (#49.1 YileGPT Unleashed): It costs (1) less for each mana its owner has unspent as each of
  their turns ends, while it is in their hand or deck. It is a permanent `costMod`.
- **MD-C23** (#49.1): "Randomly picks 2 out of 5" draws two different entries from the match rng as the
  Cry resolves. They resolve in the list's order and are named to both players (R423, R436). The Radiant
  face does all five.
- **MD-C24** (#49.1): "Give your opponent two Viruses" summons two Yile's Viruses for the opponent (R64).
- **MD-C25** (#49.3 AI Girlfriend): AI Girlfriend is an Animated Field Spell. It animates as it enters the
  field (R383); with no open unit zone it stays a backrow card, and its forced attacks find no target.
- **MD-C26** (#49.3): "After this is attacked" runs once for each combat in which this was the attack's
  target, declared or forced, after that combat's state check, on its snapshot even if it died (R426's
  mirror). It Nerfs the attacker only if the attacker is still on the field.

### Group D

- **MD-D1** (#50): the table holds per-card in-deck wins and games. The leader is the best exact rate
  among cards with at least `CN_TECH_MIN_GAMES` (20) games, then more games, then id, excluding CN
  Tech, tokens and cards no longer in the catalog. With none, it adds a random non-token card of every
  set.
- **MD-D2** (#50): the table changes only with a card patch, and its diff is claimed like a catalog
  change. Clients read the compiled table, never `/api/stats`.
- **MD-D3** (#51): "random 1/1s" are Units printed 1/1 on their base face, tokens included, one pick
  per open zone, repeats allowed.
- **MD-D4** (#52): "when attacking" covers the attacker's strikes in a combat it started against a
  Unit. Tags compared are every printed tag, so an untagged attacker always qualifies.
- **MD-D5** (#52): `conditionTargets` is computed by the engine on the viewer's own attackers only,
  during their main phase.
- **MD-D6** (#53): De-Radiant is Make Radiant reversed: base face back, damage and buffs kept,
  Radiant-only keywords lost, no Cry.
- **MD-D7** (#53): Prestige may be played with no target only when nothing qualifies, and it still adds
  its card.
- **MD-D8** (#53): "all enemy cards" means the opponent's field, hand and deck.
- **MD-D9** (#54): either player may activate it in their own main phase, paying from their own mana.
- **MD-D10** (#54): the 89/10/1 Coin, Jade or Red Jade roll is unprinted hidden data, and both Coins
  shuffle in unseen.
- **MD-D11** (#53, #55, #59, #60): a pool named by Prime or "AI generated card" holds those tokens.
- **MD-D12** (#57): the previous play this turn comes back from your graveyard or exile, or is Bounced
  from your field; otherwise nothing.
- **MD-D13** (#58, #59): a granted ability behaves like a granted keyword and runs for the dying unit's
  controller.
- **MD-D14** (#59): a Book fused with an AI generated card is one pick from each pool, fused with no
  target, at the fused cost.
- **MD-D15** (#60): deaths of your Units are counted since it entered, and it is Tributed at the next
  state check at the threshold.
- **MD-D16** (#61): "the Unit to the right" is the acting Unit in the next lane on its controller's
  side.
- **MD-D17** (#62): three different random (1) Cost Units, cost as it stands in the deck, drawn as
  ordinary draws.
- **MD-D18** (#63): a random Unit card from your hand is summoned with no Cry, and nothing is drawn
  when no zone is open.
- **MD-D19** (#64): declared attacks only. The redirect goes to an adjacent ally the attacker could
  legally attack, a random one of two.
- **MD-D20** (#64): the Radiant face copies each Unit that combat destroyed, for you (R409's reading).
- **MD-D21** (#65): Keenus's 21 keywords are printed data. A later keyword joins only by a patch.
- **MD-D22** (#65): the Radiant Death grants every keyword it had as it died to a random other Unit of
  yours.
- **MD-D23** (#66, #71.1; revises R383): an "Animated on your turn" card is not summoning sick when it
  animates at the start of a turn after the one it entered on.
- **MD-D24** (#67, #65): Magnetic resolves in full on top of your Unit, then fuses with the host kept.
  It is a separate play option from Stack.
- **MD-D25** (#68): "casts a Spell" means plays or casts a Spell, answered after it resolves.
- **MD-D26** (#69): mana spent is its controller's play and activation prices while it is on the field,
  one placement per mana.
- **MD-D27** (#69): the Maestro's exiles are one random card per enemy zone, skipping empty zones.
- **MD-D28** (#71): the AI optimal move is the scorer's best playable card for the opponent, from their
  view, before the play. Only plays are judged, ties count as optimal, and the trigger fires after the
  play resolves.
- **MD-D29** (#71; revises R643): an emote is an engine action exactly while a card hears it.
- **MD-D30** (#71.1): Pareto's attack animates the Cane for one forced attack and returns it
  afterwards; no open zone means no attack.
- **MD-D31** (#72): exile on damage is a mark the next state check collects ahead of deaths, so the
  unit has not died.
- **MD-D32** (#73): "each Armor" means each point of Armor on every Unit on the field and both heroes.
- **MD-D33** (#74): the Giant counts itself, and "full" means `HAND_CAP`.
- **MD-D34** (#75): a fused keyword applies at once. A fused "Animated on your turn" animates at your
  next start of turn.
- **MD-D35** (#70): the enemy's Ready… I'm is theirs, placed as if they had summoned it.

### Group E

- **MD-E1** (#76 Do or Die): a hand mark lasts while that card stays in that hand. Leaving the hand
  ends it, and a card that comes back is a new stay, unmarked.
- **MD-E2** (#76): the hand's owner sees which cards are marked; the caster sees only how many, and
  meets each card as it reaches their hand.
- **MD-E3** (#78 Occidentless Mandate): "each friendly permanent you exiled" counts the permanents you
  controlled that the exile took off the field. A unit token counts; a card dormant under a Stack does
  not.
- **MD-E4** (#79 Touched by KY): "your hand size is N" sets that player's hand cap to N for the rest of
  the game. It is a setting, not an increase, and it is public. The Cry sets it before its draws.
- **MD-E5** (#80 Aluneth): Untributable: no Tribute cost can take this card and no Sacrifice of it
  happens; exile, bounce and steal still work. It is not in R21's pool, and a Nerf never removes it.
- **MD-E6** (#81 Deadman's Hand): "embiggen cards" is the embiggen price: only a copy shuffled in by a
  play paid at (2) rolls its Radiant chance. A copy that is already Radiant rolls nothing, and
  "including this" is a copy of Deadman's Hand itself.
- **MD-E7** (#84 Volatility): a Buff or Nerf "N times as effective" is one application and one draw,
  whose change is multiplied by N row by row. The multiplier holds in every zone, and stat buffs and
  KY's Constant are not multiplied.
- **MD-E8** (#85 Playtester): "add either A or B" is its controller's choice, a mode prompt.
- **MD-E9** (#86 Mayor Medinamogger): "all targets are random" covers both players' declared targets
  (kind `target`), `target` prompts and attack targets while a Mayor acts on the field. Discover,
  modes, hand picks, numbers, cells, zones and Tributes stay choices. Declared targets are drawn at
  §10.5 step 1, and attack targets from the targets that attacker may legally attack.
- **MD-E10** (#86, Radiant): Lucky 1 rolls each of its controller's random targets twice and keeps
  the better. Side comes first by R656's aim, then for an attack a target whose strike back the
  attacker survives, then one it destroys. The opponent's rolls are rolled once.
- **MD-E11** (#87 Tatches the Totem): the tribal tags are Human, Felinor, KY, CN and Jlockeed. An All
  Tribes card carries every one in `tags`. A face-down play has no tags until revealed.
- **MD-E12** (#88 The True Sheep): the Discovered card with a Tribute cost goes to your hand. "To
  replace this with" names its use: the Sheep, worth 5, pays any Tribute alone.
- **MD-E13** (#89 Jlarna, rewritten): the Aura is a credit line paid in 4. While Jlarna acts on
  its controller's field, any mana they spend may go past what they have, owing up to 4 at once;
  one turn's debt is split into 4 instalments as evenly as possible, larger first, one falling due
  at each of the next four refreshes, taken off what the refresh gives. A missed instalment is
  forgiven, never carried; the debt is owed whatever becomes of the card; two or more Jlarnas share
  one line. The base face is Tributed at the end of a turn its controller borrowed nothing on.
- **MD-E14** (#91 Windfast): with no Unit in hand or no open unit zone, Windfast attacks itself. The
  summoned Unit makes the attack as a forced attack on the same target, and the base face bounces it
  after the combat if it is still on the field. Each Windfury attack does this.
- **MD-E15** (#91.1 Windfurious Prime): the joiners are drawn at random from the Units in hand and deck
  together and attack first, in lane order. A declared attack whose target has left the field before
  its combat is cancelled, its exertion spent.
- **MD-E16** (#92 Unan): a friendly ally is your hero or another of your Units. Lethal means this hit
  alone after §4.4 steps 0–4 (R44's judge). The redirected hit goes through Unan's own pipeline, and
  each Unan catches a hit once.
- **MD-E17** (#99 Paranoia): a Spell played as a Trap is a Trap play for every play and price rule, and
  a Trap until it reveals. Its effects come from a Trap. It needs no legal target when set, and "this
  turn" is its reveal turn.
- **MD-E18** (#99): a set Spell reveals on time after Paranoia leaves. Off the field it is its printed
  Spell again. A Radiant Paranoia's Echo +1 applies once per play.
- **MD-E19** (#93–#93.3 Growing Felinor): each base Death summons the next size up (Sr, Sr Sr, Super
  Senior), and Super Senior ends the chain.
- **MD-E20** (#94 Shrinking Felinor): the new Shrinking Felinor is a fresh card on the dying one's
  face, with its layer-1 stats less the shrink (attack floored at 0). A new base health of 0 or less
  summons nothing, which ends the chain.

### Group F

- **MD-F1** (#95): Call to Chaos (Meditative Edition) is the third card with the Call to Chaos tag, so
  "cast a random Call to Chaos" (R28), C+ #73's deck replacement (R387) and every Call to Chaos pool
  draw uniformly from Core #95, C+ #73 and M #95; `CALL_TO_CHAOS_CHAIN_CAP` counts casts of all three;
  R28, R87, R380, R387, R423 and R436 read "every edition" where they say "either edition".
- **MD-F2** (#95): The rest-of-game entry gives its caster a start-of-turn effect (R458) that, from
  their next turn on, casts a random base Call to Chaos as a new chain (cast depth 1, R28's cap per
  chain); a player holds at most `CALL_TO_CHAOS_ETERNAL_CAP` (3) of them, and a roll of the entry for a
  player at the cap resolves into nothing, with no re-roll, as R87's recursion at the cap does.
- **MD-F3** (#95): "Fuse your hand" fuses every card in your hand as the entry resolves, in hand order,
  except Immutable ones, which stay (R23), into one fresh hand card per R77, R102 and R469 (no kept
  card: the shared type, else the first's; a Token only if every ingredient is one), adds 2 copies of it
  (R57's riders), and all three cost (0); with one fusable card there is no fusion and that card takes
  the (0) and the copies; with none the entry does nothing.
- **MD-F4** (#95, also #31 Radiant and #55): A pool named by the Prime tag holds the Prime tokens, since
  only tokens carry it, an exception to §5.1 beside R382's; R387 still keeps a card out of its own pool.
- **MD-F5** (#95): "Summon 3 random Acclaimed cards" makes three independent picks (R60) of the
  non-token Acclaimed permanents of every set (C #80, C+ #37 and M #49 today), each summoned per R64
  with no Cry and no Tribute paid (R1); the Acclaimed tokens #49.1–#49.3 are not in the pool (§5.1).
- **MD-F6** (#95): "Bounce your opponent's field, then Nerf all the cards" bounces every enemy permanent
  acting on the field together (pile tops and face-down cards; a card beneath a pile resumes and stays,
  R13) to its controller's hand (R747; tokens cease to exist, R11; a full hand burns, R4), then Nerfs
  once (one Degrade application, R386) each bounced card that reached a hand, hidden there (R177, R440).
- **MD-F7** (#95.1): CN Golem shuffles one CN-Virus into its controller's opponent's deck for each Unit
  it is R42's killer of, Cleave and Poisonous kills included, while it still stands on the field when
  that `destroyed` is dispatched (a Golem that died in the same combat shuffles nothing, as for every
  field trigger); the virus is the opponent's, and R80's cap refuses it.
- **MD-F8** (#96): Meditative Journey's choice is a `pick` prompt of up to N cards of your hand as it
  resolves; the picks are exiled (a unit-token card ceases to exist, R11) and one Journey Complete
  remembering their ids is shuffled in; with no card picked nothing is exiled and no Journey Complete is
  shuffled in; a full deck refuses it (R80) and the cards stay in exile.
- **MD-F9** (#96): The Radiant Meditative Journey shuffles in a Radiant Journey Complete. **⚠ designer**:
  the printed face says "a Journey Complete", but Journey Complete's Radiant face speaks of "the cards"
  where the base says "the two", which pairs it with the Radiant Journey's five, and a Radiant face
  making its token Radiant is the house pattern (CN-Viral Injection, C+ #48).
- **MD-F10** (#96.1): Journey Complete returns, in exile order, every card its memory names that is still
  in its owner's exile, to its owner's hand (R746), Radiant (Radiant face: costing 1 less); it returns
  as many as went, two from a base Journey or five from a Radiant one, the base text's "two" being the
  base Journey's count and not a cap; a card that left that exile meanwhile does not come back; a
  Journey Complete that remembers nothing (made by any other card, a copy, R57, or one that reached a
  graveyard or exile, R766) does nothing.
- **MD-F11** (#97): "Piece together the blueprint" is a Discover (§6.3) of three different buildings
  out of the nine (#97.1–#97.9), uniform, the pick added to your hand (Radiant on the Radiant face).
  Discover is always one of three, and the designer used that word; offering all nine every time would
  make the pick Prison or Headquarters each play and flatten the printed rarities (Common to Mythic)
  into names.
- **MD-F12** (#97.1): While Empty Plot is the top of its unit zone, its controller may play any Unit onto
  that zone as if the Unit had Stack, unless the zone is Locked (R688) or reserved (R64); summons never
  stack on it and the opponent cannot; the Plot lies dormant under the pile and resumes when the cards
  above it are gone (R13); on the Radiant face the Unit is Buffed once as it lands, before its Cry.
- **MD-F13** (#97.4): Mega Church's Tribute may be paid, besides your own Units, by any permanent acting
  on the field on either side whose cost (R65, R396) is (1) or less, a face-down one included (its cost
  is public, R33), each worth 1 (a Sheep Token its 2 or 3); the set must be minimal (R101), the payment
  is a Sacrifice (Death fires, its owner's graveyard), cards in a hand never pay, and the Church stays
  with its player (no R360 hand-over).
- **MD-F14** (#97.5): Bunker's attack is multiplied (3 times) in a combat whose other unit stands in the
  opposing unit zone of Bunker's lane, read once per combat (R94) before §4.4; strikes into other lanes,
  its Cleave (none printed) and any non-combat damage are not multiplied; it applies on a forced attack
  too.
- **MD-F15** (#97.7): The Great Wall's Cry Locks each of its side's five unit zones, occupied ones
  included; until one is Unlocked, no Unit (a Stack play included) may be played there (R688), and
  "fill your board" effects (R64) fill nothing on that side, whoever's effect it is (#97.9's fill, M #49.1
  YileGPT Unleashed's Viruses), while ordinary summons and moves still enter (R688).
- **MD-F16** (#97.8): Prison answers each Unit its opponent plays or casts, after the Unit resolves, Cry
  included (R427's window), while Prison acts; on the roll's success the Unit, still on its stay
  (R174), moves directly beneath Prison without leaving the field (no R78 reset), dormant (R13), under
  Prison's controller (owner unchanged, R12; R171); several captures lie newest first beneath Prison,
  and when Prison leaves the field the newest resumes under Prison's controller.
- **MD-F17** (#97.9): Jlockheed's Headquarters fills each empty, unlocked, unreserved unit zone of its
  side, left to right, with a random non-token Jlockeed Unit of every set (R64, R380, R60), as C+ #2
  fills its board, summoned with no Cry (R1); the Radiant face's second activation in a turn fills what
  the first left empty.

### Group G

These came with MB27 (issue #571), after the blocks were reserved, so they take the next free numbers
(M10).

- **MD-G1** (#101 Gachaholic; R1437): A Luck-based card is a non-token card that prints Lucky on either
  face or whose text says "Flip a coin", read off the definition, so a given Lucky never makes a card
  Luck-based. The pool names no set (R1420); tokens are out (§5.1), and #40 Feng Shui, which makes no
  roll of its own, is not in it. The card arrives on its base face at its printed cost, hidden from the
  opponent, and a full hand burns it.
- **MD-G2** (#101; R1438): "Give it Lucky X" is a granted Lucky X, put on once the card is in the hand
  (none on a burned card), with no event. It is the card's own: kept in hand, ridden onto the field and
  through a Spell's resolution, gone when the card leaves the field (R78) or reaches a graveyard or exile
  (R215), and shown on its face. It adds to any Lucky the card has, and every pool card's roll reads the
  Lucky on its instance, printed plus given.
- **MD-G3** (#102 Catboy Maid SSR+; R1439, its note written with #102's script once #525 lands): The
  Cry makes two separate rolls, each through `rng.lucky` with the card's Lucky, higher better for both:
  the draws (1 or 2), then the Jade Counter's amount (1–4, Radiant 1–10), which goes through ME-JADE's
  verb (MD-C3 at 5 and 10).
- **MD-G4** (Core #4, C #65, C+ #19.3; R1440): A coin flip is a luck-based roll, its better side heads
  unless the card's text names another (#36 keeps MD-B17). For each flip, a card with Lucky X flips X
  more coins and keeps heads if any lands heads; with no Lucky it is exactly one draw, so no shipped
  game changes. It revises R32 and R130 for coin flips.

## M8. The visuals, sound, AI, decks and Almanac pass

Each item is a part of its own (M10). Readings, with what exists today:

- **MN01 Nerf and Buff** (M4). Shipped card texts, [[§6.3]]'s rows (renamed, the engine names kept), the
  glossary, the log, the animations' and cues' labels, the tutorial's words, `wording.test.ts`. A patch
  of its own with a pending fragment for the changed texts; it ships when it lands.
- **MN02 Hero portraits, clickable and vivid.** Today (`apps/web/src/game/Hero.tsx`, `emotes/Portrait.tsx`,
  [[R641]]–[[R643]]) a click on your portrait opens the emote menu, on the opponent's the mute menu, and a
  click aims when the hero is a legal target; the six portraits are procedural `CardArt` in an oval.
  Reading: a portrait is something to look at, like a card. A click (or long-press on a phone) on either
  portrait's art, outside targeting, opens the hero's inspect view — the portrait large, its name and
  title, a line of flavour, its health and Armor — with the emote menu (yours) or the mute toggle
  (theirs) inside it, so nothing reachable today is lost; a click also gives the portrait a short
  reaction (a squash and a glint, nothing under Reduce motion). "More vivid": each portrait's art gets
  richer procedural layers (a lit gradient background per portrait, rim light, saturated palette,
  a soft animated idle — breathing light, drifting motes — that Reduce motion stills), in the inspect
  view, on the board and in the deck builder's picker. Presentation only.
- **MN03 Emotes, more of them, dealt each game.** Today ten fixed emotes, five voice lines per portrait
  and five shared emoji ([[R643]]–[[R645]]). Reading: a pool of at least 24 — the ten, plus new emoji
  (SVG and procedural sound, as the five are) — of which each seat is dealt a hand of 8 at random each
  game from the match seed (as [[R642]] deals All Random portraits), the same on reconnect and replay,
  sent with the portraits message, never in the view; the menu shows that hand, the AI personas
  ([[R645]]) choose within it, and the rate limits stay. As amended on #545: no new voice lines, since a
  line renders on macOS alone and the voice-asset check expects a file for every one. Built as
  [[R1340]]–[[R1345]]: fourteen new emoji, 24 in all, and a hand of three voice lines and five emoji.
- **MN04 Intro music for Legendaries and Mythics.** Today every Mythic has a looping theme of its own and
  every Legendary shares `legendary-1` or `legendary-2` (`music-cards.json`, [[R631]]), and a played
  Legendary or Mythic gets the procedural `entrance` sting ([[R669]]). Reading, as Hearthstone's
  legendary music: each Legendary and Mythic card (and each token printed Legendary or Mythic, when it
  is played) gets its own short intro snippet, 3 to 6 seconds, non-looping, composed in
  `apps/web/scripts/music/tracks.mjs` (a motif per card, derived from its id, set and tags, hand-tuned
  where it matters), rendered by `gen:music` (FluidSynth on Linux) and played as the card's play line
  starts, ducking the music bus; the themes stay. The shipped sets' snippets come first; the Meditative
  set's 22 come with its release (MR), so no card waits on audio. Every rendered file is committed.
- **MN05 Armor and other niche sound effects.** Today `GameEvent::Damage { amount }` carries what got
  through Armor, a hit Armor stops entirely emits nothing ([[R63]]'s zero rule), and no cue knows about
  Armor. Reading: the damage event carries the Armor that absorbed (`absorbed`), a hit Armor stops
  entirely is reported (`damageAbsorbed`), and the cues: a dull clank when Armor takes half or more of a
  hit, a bright ring and shield flash when it takes all of it. Then a pass over every event for the other
  niche cases worth a sound (overkill, a card burned from a full hand, a Brittle crumble, a lock and an
  unlock, a steal, a counter, a transform into a Sheep, a fuse, a Nerf and a Buff, an extra turn, the
  Jade Counter, a translation, a purchase at the night market, …), each new event's cue added with its
  system. The damage event change moves the golden traces once (D14).
- **MN06 Random decks skewed toward the new set.** Today `build_ai_deck` (`crates/ai/src/deck.rs`) deals
  All Random's decks ([[R258]]) and practice's random deck from every shipped set, with `theme` and
  `boost` options. Reading: an option "More cards from the newest set" — the newest set in
  `SHIPPED_SETS`, Meditative once it ships — that deals at least half of the deck (10 of 20, rounded up
  for a handicap's larger deck) from that set and the rest as today; on practice's Random deck and on
  All Random (queue, rooms and rematch), chosen per player and sent as intent; the server deals it.
- **MN07 The Almanac and the set's mark.** The Almanac shows every card of the shipped sets
  automatically, so the Meditative cards appear when the set ships. The part adds what the set needs:
  its set mark (an ensō, beside Core's orb and the Classic temples, `cards/setMark.ts`), the set filter
  and an "All Tribes" frame label, the Created mark, Chinese faces and every new keyword's glossary row
  checked, and the newest set's ribbon on its cards for the first patch it is in.
- **MN08 The AI with the new cards.** The AI plays by search over `legal_actions` and needs no card
  knowledge, but every new prompt kind (the crafter, the market, the secret choice, the prediction, …)
  must have answers the AI and the random policy can give, built by the system's part. This part, once
  every card part has merged: the tools learn to preview a set (`arena`, `sweep`, `gate`); the sweep runs
  over the whole catalog with Meditative previewed, and at most 20% of the 102 Meditative cards may be
  shadow-banned (at most 20, so at least 82 stay in the pool); the AI with the new cards plays the previous AI
  (generation 0, built from `main` before the first card part) for 100 games in `arena` with alternating
  seats, and must win at least 40; where it does not, the part improves the AI until it does. The
  results are recorded in [[§9.9]] and `crates/ai/generation.json`. As amended on #551: that training is
  deferred, and until it is done the AI's random decks hold no Meditative card, a soft gate lifted by
  emptying `AI_DECK_GATED_SETS` ([[R1390]]).
- **MN09 Polish.** The small polish found while the set is built, collected on its issue and done
  together near the end.

## M9. Decisions for the designer

The build uses the default in each row; each is a guess, and an answer changes the card in a later patch.

| # | Where | Question | Default |
| --- | --- | --- | --- |
| 1 | M2 | Showdown and Paranoia renumbered #98 and #99 (the cards whose tokens did not carry the number move) | as M2 |
| 2 | #62 | Small Time Recruits' text ("Something with 1 costs") | Draw three (1) Cost Units from your deck; Radiant makes them Radiant |
| 3 | #89 | Jlarna's text (none given) | answered 2026-10-09: the designer gave the text (a (2) Field Spell Aura, "spend mana from next turn", tributing when a turn passes with no card played), then removed the Combo line, raised the cost to (3), forgave missed instalments, and made the Tribute "if you don't use your credit line", keeping "pay in 4" as the Aura's four instalments |
| 4 | #12, #59, #83, #93–#93.3, #49.2, #97.5 | Radiant stats the designer kept equal | doubled, R275's stat half |
| 5 | #69 | The Maestro has no Radiant face | 8/8; two exiles from each zone |
| 6 | #2 | Rampaging Rhino's Radiant only raises stats | adds Trample |
| 7 | #16 | "a random Traps" | two random Traps |
| 8 | #20 | Aestheticize the Game's Radiant is only 10% easier | adds "Draw a card." |
| 9 | #22 | Mind Games' Radiant only prices Fortify Mind at (2) | as written |
| 10 | #40 | "punished heavily" | 20 damage on the Radiant face |
| 11 | #49.1 | Radiant gives two Viruses where the base fills the board | the two are Radiant Viruses |
| 12 | #97.1 | Empty Plot's Radiant only raises health | adds "Buff the stacked Unit once." |
| 13 | #95 | "Your hero gains 8 Armor" is per-hit Armor here, very strong | as written |
| 14 | #38 | Do H1B Printer's cards arrive in Chinese? | no (only the cards whose text converts) |
| 15 | #42 | "AI generated cards related to CN" | one AI-tagged card lot |
| 16 | #66 | Fiery Waraxe could never attack under R383 | R383 revised (D7) |
| 17 | #17 | The turn clock while crafting | it runs, as for any prompt |
| 18 | M8 | MN02's reading of "clickable" portraits | an inspect view that holds the emote menu |
| 19 | M8 | MN03's hand of emotes | 8 of at least 24, dealt by the match seed |
| 20 | M6 | every other ⚠ designer item in M6's Check lines | the entry's reading |

## M10. The plan: parts, issues, order and ruling blocks

Every part is an issue under the tracker #496, titled `Patch v0.3.X: Meditative, <name>` (a card batch:
`Patch v0.3.X: Meditative batch NN, <name>`), labelled `patch` and `large patch`. No part is titled
`(part n of m)`: the night bot builds every such part strictly in order, and these build in parallel,
each waiting only for what its "Blocked by" line names. The night bot builds every part but the
foundation and the release, which the orchestrating session builds.

Two kinds of part. The card parts (MS01, MB01–MB27) and MN08 are parts of the Meditative patch: they
land behind the release gate ([[R1420]]), so players meet none of their cards before the release (MR),
and they need no fragment. The other MN parts (MN01–MN07, MN09) change what players see and hear in
every game, the shipped sets included. Each is a patch of its own (`docs/issues-and-patches.md`, A patch
that takes several pull requests), titled without the set's name where it is not about the set, and
goes live when it merges: MN01 with its own pending fragment for the shipped texts it renames, the
others with no card data change. MN06's option leans on the newest shipped set, Classic+ until the
release and Meditative from it; MN07's set mark and set filter have nothing to show until the release.

**Rulings.** Each part takes its R-numbers from its block, in order: MB01–MB26 twenty each, MB01
R780–R799 up to MB26 R1280–R1299; MS01 R1300–R1319; MN01–MN09 ten each, MN01 R1320–R1329 up to MN09
R1400–R1409; MR R1410–R1419; the foundation (MF) R1420–R1429, the top block, so that once it is on `main`
the next free number (`spec/INDEX.md`'s last row plus one) is above every block and work outside #496
never lands in one. A part that needs more than its block takes the next free number on `main`, and
so does MB27, which came after the blocks were reserved and has none: it took R1437–R1440. The
blocks are listed in docs/issues-and-patches.md, Ruling numbers, so other work can see they are held;
should a number of a block be on `main` anyway, the part renumbers per that section.

**Every card part** follows M3's list, keeps `cargo jackioh catalog check`, `patches check` (it adds no
fragment) and `spec check` green, and adds a test per behaviour of its BUILD M10 rows, base and Radiant
separately (CLAUDE.md rule 6). A part that adds a new event adds its `ANIMATIONS` and `SOUND_CUES` rows
(the maps are total) and its BUILD M5-T4 row; one that adds a prompt kind makes sure the AI, the random
policy and the timeout can answer it.

| Part | Issue | Cards or scope | Systems it builds | Blocked by | Difficulty |
| --- | --- | --- | --- | --- | --- |
| MF | #515 | the release gate and the shared pieces; the brief, §8.8, §7, BUILD M10, the census and the fixture | R1420–R1424: the gate, Prime and AI pools, `anyTags`, `give_control`, `TRIBAL_TAGS`; the tag Wincon | — | built by the orchestrating session |
| MB01 | #517 | the cards on existing primitives: #2 Rampaging Rhino, #3 Jlockwork Machine, #15 Smelly Steven, #16 Trenful Trickster, #23 Golly Bob Howdy, #24 Polymorph | none: existing verbs and keywords | #515 | medium |
| MB02 | #518 | composed spells: #1 Disruptive Disruptor, #4 Juicy Kumquat Melon, #5 Death by 1000 cuts, #6 Me no Likey, #7 Introspection, #14 Prime Time, #21 API Key Fishing | the discard guard (#1, Group A's NEW: discard guard); a computed Echo X from max mana (#5); it uses R1423's `give_control` (#6), already on `main` | #515 | hard |
| MB03 | #519 | the trigger multipliers: #9 Joint Filing, #10 Double Counting, #11 Double Header, #12 Fear Mongerer, #13 Gatling Pea | ME-TRIG: the start and end of turn multiplier, the Cry and Death multiplier, and `trigger_turn_hooks` ("trigger your end of turn effects") | #515 | hard |
| MB04 | #520 | extra turns and other ways to win: #8 Reach the Summit, #18 Expedition12, #19 Expedition1234, #19.1 Temporal Rift, #20 Aestheticize the Game | ME-TURN: lost refreshes ("lose all mana next N turns"), extra turns and the once-a-game Temporal Rift flag, built so MB09's zeroed-refresh aura and MB23's repayment schedule can share it; ME-WIN: an effect that wins the game and chosen alternative win conditions the state check holds (new GameOverReasons), with the client's banner and progress | #515 | hard |
| MB05 | #521 | Mind Games: #22 Mind Games, #22.1 Fortify Mind | ME-SECRET: a secret choice only its owner reads, the AI's `redact` and `determinize` hiding and sampling it, the reward on the caster's next turn, and Fortify Mind's prediction; the client's secret-choice and prediction UI | #515, #520 | hard |
| MB06 | #522 | True Craft a Card: #17 True Craft a Card | ME-CRAFT: `subsystems/craft.rs` (the block table of the engine's real keywords, hooks and verbs with their mana and lines-of-code prices, the recipe validator and compiler to a transient definition named by its recipe as a Fuse's is), the `craft` prompt kind with seeded presets the AI and a timeout answer, a WASM `craft_preview`, and the client's Scratch-style block editor with points and lines-of-code meters | #515 | hard |
| MB07 | #523 | curses and kittens: #27 Clip-Farming Lawyer, #28 Shade-iris, #28.1 Ancient Curse, #29 Forbiddenous Factory, #30 Fickle E-Kitten, #30.1 Love Bomb, #36 CN Peptides | `unlock_random_zone`; `highest_permanent_cost`; it uses R1423's `give_control` (#30), already on `main` | #515 | hard |
| MS01 | #516 | every card's Chinese name and text | ME-CN: `crates/cards/chinese.json` for every entry, the `chinese` instance flag, `translate`, the client's Chinese faces | #515 | hard |
| MB08 | #524 | the Chinese cards: #32 Spiritually 中国, #33 First Day of 学校, #34 高考, #35 RCTA (CN), #37 CN in a bottle, #38 H1B Printer | granted tags (`tags_of`), read wherever an instance's tags are read; the `enters_hand` hook (#37); transform riders (`costOverride`, `chinese`, `created`) and the hand-wide transform (#32); the `chinese` riders these cards set, on top of MS01's flag and `translate`; it uses R1422's `anyTags` (#38), already on `main` | #515, #516 | hard |
| MB09 | #526 | the rule benders: #25 Blue-Eyes White Felinor, #26 Alternate Fate, #31 The Conductor, #98 Showdown | ME-TRIBAL's immunity, "immune to tribal tag based hate" (MD-B1, over R1424's `TRIBAL_TAGS`); ME-CREATED: a `created` flag on every instance made after the deal (it moves the golden traces once: re-bless after merging `main` and say why, D14); ME-TURN's zeroed refresh as an aura (#26), sharing MB04's branch in `refresh_mana`; the turn watcher and the aimed cast (#98); `fill_board` for the backrow on both sides (#26) | #515, #520, #524 | hard |
| MB10 | #525 | the Jade line: #39 赌石 Addict, #39.1 Auspicious Rock, #39.2 Jade, #39.3 Dud, #39.4 Red Jade, #39.5 Jade Beauty, #41 CN Smuggler, #43 CN Jade Market, #44 CN Jade Well | ME-JADE: each player's public Jade Counter, Jade Beauty summoned at 5 and made Radiant at 10, the `jadeChanged` event and the counter on the view; ME-ALLURE: enemy Units marked now join you at the start of your next turn, or die with no room; a general weighted roll (`add_rolled_grapes` made general) with Lucky | #515 | hard |
| MB11 | #527 | Feng Shui: #40 Feng Shui | ME-ELEMENT: every card's element, the generating and overcoming cycles, the reward and the punishment at §10.5 step 3, the last-played elements on the view and the client; ME-LUCK: a player-wide Lucky added to every roll that has a best | #515 | hard |
| MB12 | #528 | the night market: #42 CN Flea Market | ME-MARKET: the stall rolled when the market opens, prices in yuan, the `market` prompt kind (buy, barter on the Radiant face, leave), the AI's and a timeout's answers, and the client's shop | #515, #525 | hard |
| MB13 | #529 | the YileGPT line: #46 Conjure Intellect, #47 饕餮, #48 Tranquility, #49 YileGPT Tamed, #49.1 YileGPT Unleashed, #49.2 Yile's Virus, #49.3 AI Girlfriend | pick N different entries (Call to Chaos's R423 draw as a verb); `after_attacked`, the defender's mirror of R426's hook; hero immune to damage until a turn starts; `shuffle_into` at the bottom of the deck; `fuse_cards` with a random ingredient | #515 | hard |
| MB14 | #530 | alternative plays: #45 Knowledge Breaker, #45.1 Knowledge Breaker Prime, #99 Paranoia | ME-ALTPLAY: a Unit played face down as an Animated Field Trap that fires at its controller's start of turn (#45, #45.1), a Spell played face down as a Trap that fires at a chosen timing (#99), `legal_actions` and the play action carrying the mode, `determinize` sampling them, and the client's control for it | #515 | hard |
| MB15 | #531 | keywords and existing primitives: #55 Dragon Fruit, #56 House Party, #61 Joon Jorker, #62 Small Time Recruits, #66 Fiery Waraxe, #68 Catnip, #70 I'M WILL BE YOUR DOOM, #70.1 Ready… I'm, #73 Plate Packer, #74 Montaña Giant | R383 revised (D7, MD-D23): an "Animated on your turn" card on its controller's side since the turn began is not summoning sick when it animates (it changes C+ #12.8 Frostspatula's play: say so); `armor_on_field` (#73); it uses R1421's Prime pool (#55), already on `main` | #515 | hard |
| MB16 | #532 | pools and fusion: #51 Devin Bot, #57 Clip-Farming Critikal, #60 Eschews, #63 Skull of J'Nari, #65 Keymaster Keenus, #67 Sentient Cat Ears, #75 Ever Growing Tree | ME-MAGNETIC (#67, and #65's keyword list): played onto one of your Units, it resolves and fuses into it, with the client's drop target; the catalog query's `stats` field (printed base attack and health); `fill_board_random`, `summon_random_from_hand`; a voice-line `trigger` hook (#63's lines); it uses R1422's `anyTags` (#60), already on `main` | #515, #523 | hard |
| MB17 | #533 | riders and outside data: #50 CN Tech, #53 Prestige, #54 Money Machine, #58 Permanent Underclassman, #59 Permanent Upperclassman, #69 The Maestro | ME-STATS: a compiled-in table of card win rates (`crates/cards/data/win_rates.json` with a `cargo jackioh winrates` command, `CN_TECH_MIN_GAMES`, ties and an empty table handled), never a runtime read (CLAUDE.md rule 4); De-Radiant; an Activate either player may use (#54); ME-GRANT: a granted Death ability as plain data (#58, #59); the `manaSpent` event (#69); multi-pool `fuse_generated` (#59) | #515, #525 | hard |
| MB18 | #535 | combat and the judge: #52 Economic Anxiety, #64 Traitorous Blood, #71 Pareto Optimality, #71.1 The Cane, #72 The Banisher | combat-only attack modifiers and the engine's `conditionTargets`, painted yellow while dragging an attack (#52); the attack half of Redirect (#64); exile on damage (#72); ME-PARETO: the engine-side judge of the AI optimal move, on the scorer, never `crates/ai` (CLAUDE.md rule 4); ME-EMOTE: an emote as an engine action and the `emoted` event while a card listens, minted by the match actor from an emote that passed the rate gate (R643 revised), hotseat and practice alike | #515, #531 | hard |
| MB19 | #534 | the hand: #76 Do or Die, #77 Bulk Booster, #78 Occidentless Mandate, #79 Touched by KY, #81 Deadman's Hand, #90 Spell Basket | ME-HANDCAP: a per-player hand size read everywhere `HAND_CAP` is read today; hand marks for delayed effects (#76) | #515 | hard |
| MB20 | #536 | buffs: #82 Medina Outfitter, #83 Medina Enforcer, #84 Volatility, #85 Playtester, #87 Tatches the Totem | ME-TUNEMULT: "Buffs and Nerfs are twice as effective on this" (one application, a doubled change, R440 kept); Tatches' deck trigger on a card with a tribal tag (R1424) | #515 | hard |
| MB21 | #537 | the Felinor chains: #93 Growing Felinor, #93.1 Growing Felinor Sr, #93.2 Growing Felinor Sr Sr, #93.3 Growing Felinor Super Senior, #94 Shrinking Felinor | nothing new: named Death summons and `statsOverride` | #515 | medium |
| MB22 | #538 | combat replacements: #86 Mayor Medinamogger, #91 Windfast, #91.1 Windfurious Prime, #92 Unan | ME-RANDOMTARGETS: "all targets are random", for both players' targets, attacks and target prompts; attack summons (#91, #91.1); the lethal guard (#92): §4.4 step 4a opening for Units as well as heroes | #515 | hard |
| MB23 | #539 | the play pipeline: #80 Aluneth, #88 The True Sheep, #89 Jlarna | the Untributable keyword (R1220); a catalog pool of cards with a Tribute cost (R1221); Jlarna's credit line: borrowing past current mana up to its limit through one affordability function (R1223), ME-TURN's repayment schedule taking an instalment off each refresh, a missed one forgiven (R1224), and the turn log's record of borrowing that the base face's Tribute reads (R1225), on MB04's refresh | #515, #520 | hard |
| MB24 | #540 | chaos and the journey: #95 Call to Chaos (Meditative Edition), #95.1 CN Golem, #96 Meditative Journey, #96.1 Journey Complete | the third Call to Chaos table, `subsystems/call_to_chaos_meditative.rs`, with "for the rest of the game" bounded by a cap and R28, R87, R380, R423 and R436 reworded for three editions (their pools find the Meditative edition once the set ships, R1420); memory written on a shuffled card (#96) | #515, #525 | hard |
| MB25 | #541 | the plain buildings: #97.2 Wishing Well, #97.3 School, #97.6 University, #97.7 The Great Wall, #97.9 Jlockheed's Headquarters | nothing new beyond an optional `lock_all` | #515 | medium |
| MB26 | #542 | the blueprint: #97 Jlockheed's Evil Blueprints, #97.1 Empty Plot, #97.4 Mega Church, #97.5 Bunker, #97.8 Prison | stack base (#97.1); wide tribute (#97.4); lane strike (#97.5); capture (#97.8) | #515, #541 | hard |
| MB27 | #571 | the gacha pulls: #100 Greaser, #101 Gachaholic, #102 Catboy Maid SSR+ | MD-G1's Luck-based pool; MD-G2's Lucky given to a card, read by every pool card's roll; MD-G4's coin flips that keep heads with Lucky (`Rng::lucky_coin`); the set's count to 102 cards and 132 entries | #515, #525 | hard |
| MN01 | #543 | Nerf and Buff: the keyword renames | Degrade and Upgrade renamed in every player-facing word; a pending fragment for the shipped texts | #515 | medium |
| MN02 | #544 | hero portraits you can click, with more vivid art | the inspect view of a hero; richer procedural portraits | #515 | medium |
| MN03 | #545 | more emotes, a hand of 8 dealt each game | a pool of at least 24; the seeded deal; the menu; the AI personas | #515 | hard |
| MN04 | #547 | intro music for every Legendary and Mythic | a snippet per card, composed and rendered by `gen:music`, played as it is played | #515 | hard |
| MN05 | #548 | Armor and the other niche sounds | `absorbed` and `damageAbsorbed`; the half and all cues; the niche cues | #515 | hard |
| MN06 | #549 | random decks that lean on the newest set | `build_ai_deck`'s newest-set floor; the option in practice and All Random | #515 | hard |
| MN07 | #550 | the Almanac and the set mark | the ensō mark, the filters, Wincon, All Tribes, the glossary, the New ribbon | #515 | medium |
| MN08 | #551 | the AI with the new cards | the tools' preview; the sweep (at most 20 of the 102 banned); 40 of 100 against the previous AI | #515, #516–#542, #549, #571 | hard |
| MN09 | #552 | polish | every face read and every card played in the browser; the items on its issue | #551, #543, #544, #545, #547, #548, #550 | medium |
| MR | #553 | the release | the set ships: the gate opens, one fragment, the migration, deck codes, the traces, the audit | every other part (#571 included) | built by the orchestrating session |

**The end state** (MR): every card of [[§8.8]] in the catalog with its script and tests; the set in
`SHIPPED_SETS`; one pending fragment claiming the 132 entries, shipped by `patches ship`; the migration
that lets the server seed the new tag; deck and trio codes that know the set's numbers; the golden traces
re-blessed once and every seed-pinned test re-pinned for the bigger pools; the fuzz tool at 1,000 seeds,
the AI's gates and the arena result of MN08 green; `cargo jackioh catalog check` holding Meditative to
every count; REVIEW.md's audit run (CLAUDE.md rule 8); every required check green on `main`.
