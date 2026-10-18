# Meditative+ — design brief for the implementing agents

2026-10-18 · written from the designer's list of the same day (issue #1173, `JackiOh_MeditativePlus_Cards.md`)

> **Status: the brief, and the plan of record.** SPEC is still the only source of rules (CLAUDE.md). The
> set-level rules this brief decides are in SPEC already ([[R1178000]] to [[R1178002]], [[§5]], [[§8.9]],
> [[§7]]); every card's row is in [[§8.9]] (tokens also in [[§7]]); each card's mechanics become SPEC when
> the part that builds them lands, with its rulings. This file records, for every card and every new
> mechanic, what the designer wrote (verbatim in the source note), what it means in this engine, every
> place the text is unclear with the reading chosen, and the order the work is built in. It follows
> `docs/meditative-set.md`'s shape, much shorter: 26 cards, not 102.
>
> Proposed rulings are numbered **MP-B01-1, MP-B02-1, …**, by the part that builds them. They are not
> `R<n>` rows: the part that proves one takes an R-number from its own issue's block (MP10) and names
> its test after it. **⚠ designer** marks a reading the designer should confirm; the build does not wait
> for it (MP7 lists each with the default the build uses).

## Contents

- [MP0. What the designer asked for](#mp0-what-the-designer-asked-for)
- [MP1. Reading this brief](#mp1-reading-this-brief)
- [MP2. The set: ids, numbers, tags, rarity and the census](#mp2-the-set-ids-numbers-tags-rarity-and-the-census)
- [MP3. Built on main before it ships (R1420, R1178000)](#mp3-built-on-main-before-it-ships-r1420-r1178000)
- [MP4. The four patches beside the set](#mp4-the-four-patches-beside-the-set)
- [MP5. The systems, and the decisions](#mp5-the-systems-and-the-decisions)
- [MP6. Cards, card by card](#mp6-cards-card-by-card)
- [MP7. Readings for the designer](#mp7-readings-for-the-designer)
- [MP8. Sound, visuals, the AI and the release](#mp8-sound-visuals-the-ai-and-the-release)
- [MP9. Where this brief differs from the batch issues](#mp9-where-this-brief-differs-from-the-batch-issues)
- [MP10. The plan: parts, issues, order and ruling blocks](#mp10-the-plan-parts-issues-order-and-ruling-blocks)

---

## MP0. What the designer asked for

Issue #1173, verbatim in `JackiOh_MeditativePlus_Cards.md`:

- **A new set, Meditative+**: 26 numbered cards, two of them tokens the list names (#16.1 Jazakus Prime,
  #20.1 Lil' Kenny), and the AI cards #1 Bob's Plan may make: 26 cards and 4 tokens, 30 catalog
  entries (MP2).
- **Every card in Chinese gains CN** (the designer wrote "tribute"; CN is a tribe, [[R1424]]): #1176.
- **Every token labelled as a token**: #1177.
- **Embiggen cards standardised**: a card with two prices is an embiggen card; its cheaper price plays
  its normal version, its dearer price its **Embiggened** version: #1175.
- **Freeze**, Hearthstone-style: a Unit loses its next attack or position change, and thaws at the end of
  the turn it could have acted: #1174.

The four rule changes are patches of their own that go live when they merge (MP4); the set is built
behind [[R1420]]'s gate and ships with its last part (MP3, MP10).

## MP1. Reading this brief

- **Numbers.** "§" is SPEC; "MP3" is this brief's section MP3; "M+ #N" is Meditative+ card N, "M #N" a
  Meditative card, "C #N", "C+ #N" and "Core #N" (or "#N" in SPEC) the other sets' cards. The parts are
  named by their issues: B01 #1179 to B07 #1185, the AI #1186 and the release #1187.
- **A card entry** (MP6) has the heading (set, number, name), the header line (id · cost, type and tags ·
  rarity · stats base → Radiant · the part that builds it), the designer's text flattened onto one line,
  and then **Text** and **Radiant** (the proposed `base.text` and `radiant.text`, house style, params as
  `{key}`, a line break written " / "), **Engine** (what the card does in the engine's terms, the verbs
  and rulings it uses, and the systems of MP5 it needs, marked NEW), **Rulings** (the readings a builder
  needs), **Numbers** (the declared params: base → Radiant, ↑ when more is better for the controller),
  **Check** (R275, R276, typos, ⚠ designer items) and **Class** (A keywords or one primitive, B composed
  primitives, C a shared new system, D a subsystem of its own).
- **Words.** House style as R366, R432 and R373 set it: "Deck" for the library, "Tribute" for Sacrifice,
  Cry and Death, labelled abilities on lines of their own, "(N) Cost" for a cost as a noun and "costs
  (N)" as a verb, straight apostrophes. An embiggen card's dearer price is written **`Embiggened (N): …`**
  (#1175's label, which replaces R366's "Paid (N): …"); its params are `embiggenedX` where the
  Meditative cards wrote `paidX`.

## MP2. The set: ids, numbers, tags, rarity and the census

**Ids.** Card `meditativeplus-NNN` (three digits), a token a card defines `meditativeplus-NNN-k`; index
strings `"16"`, `"16.1"`. Scripts in `crates/cards/src/scripts/meditative_plus/cNNN_slug.rs`
(`c016_1_jazakus_prime.rs`). The set is `SetName::MeditativePlus`, written `"Meditative+"`.

**Numbers.** The designer's 1 to 26 stand, with no hole and no repeat. The tokens:

| Index | Token | Made by | Rarity |
| --- | --- | --- | --- |
| 1.1 | Base Camp | #1 Bob's Plan, after M #8 Reach the Summit | Token, AI, no printed rarity (as T-AI-1 to T-AI-10) |
| 1.2 | Spa Day | #1 Bob's Plan, after M #20 Aestheticize the Game | Token, AI, no printed rarity |
| 16.1 | Jazakus Prime | #16 Jazakus's Death | Token, printed Epic (the designer's) |
| 20.1 | Lil' Kenny | #20 KY's Competition | Token, printed Rare (the designer's) |

**Names.** One name names one card ([[R381]], [[R739]]): Classic #60 is Pile On, so the designer's #17
Pile On is **Dogpile** ([[R1178002]], ⚠ designer). The apostrophes are straight.

**Types.** #24 Bob, Wisconsin's Strongest Soldier, which the list calls "Unit, Spell", is a Unit (it
prints stats and a Cry). Every other type is the designer's.

**Tags.** The designer's, with "Jlockheed" read as the existing tag `Jlockeed` ([[R278]]). One new tag,
**Potion** (#11, #12, #13 and the potions #16 and #16.1 craft; the pool of #14 and #15), a family tag
that names no tribe ([[R1424]]) and that no rule reads ([[R1178000]]). The AI cards #1.1 and #1.2 carry
AI and Token, as the T-AI cards. Census of the 30 entries: Human 5 (#7, #8, #10, #16, #16.1), Token 4,
KY 3 (#20, #20.1, #25), Potion 3, AI 2, CN 1 (#21), Fruit 1 (#23), Quickdraw 1 (#1), Jlockeed 1 (#4),
Book 1 (#15), Catalyst 1 (#16), Prime 1 (#16.1).

**Rarity.** The designer's, and for #26 Scratch Pad, which has none, **Mythic**, as M #17 True Craft a
Card, whose crafter it opens ([[R1178002]], ⚠ designer). The 26 cards: **5 Common** (#7, #8, #9, #10,
#19), **9 Rare** (#5, #11, #12, #13, #14, #17, #20, #21, #25), **9 Epic** (#2, #3, #4, #6, #15, #16, #18,
#22, #23), **1 Legendary** (#24), **2 Mythic** (#1, #26). #15 Book of Potions is a Book of ___, Epic
as the family is ([[§8]]).

**Radiant stats.** [[R275]]'s stat half holds as written: every Unit's Radiant face doubles its stats,
Lil' Kenny's 2X/2X included. Lil' Kenny prints 0/0 with `xStats` 1/1 (Radiant 2/2), as C+ #69 Buff
Billy prints X multiples, and is always summoned with its X.

**The census** (`crates/tools/src/catalog.rs`'s `sets()` and tag counts, and
`crates/cards/tests/cross/catalog.rs`'s `MEDITATIVE_PLUS` fixture, written once by the foundation from
the table in [[§8.9]]): every entry the set holds must be one of these indices, with its row's name,
cost, type, tags, rarity and stats.

## MP3. Built on main before it ships (R1420, R1178000)

The set is built on `main`, card by card, behind [[R1420]]'s gate, as the Meditative set was
([[R1178000]]):

- `SetName::MeditativePlus` is in `CATALOG_SETS` (catalog order, after Meditative) and not in
  `SHIPPED_SETS`.
- A pool that names no set draws from the sets that ship; one that names Meditative+, or names a card by
  id, reaches it. So a Meditative+ card finds its own tokens and its Potion pool, and no other card, AI
  deck, determinization or random cast meets one until the set ships.
- [[§9.4]]'s L3 and `create_game` refuse a Meditative+ card in a deck; no lean names the set; the Almanac
  and the deck builder show no card, set chip or tag of it; the server neither seeds nor serves it; no
  voice file is owed for its lines.
- `patches check` and `patches ship` read the catalog without it: **a card batch adds no pending
  fragment**, and the release claims all 30 entries in one fragment.
- The testkit previews the set on one thread (`preview_sets(&[SetName::MeditativePlus])`): a card test
  that needs Meditative+ cards in a pool that names no set previews it (the Potion pool of #14 and #15,
  the embiggen pool of #23 reaching the set's own); the fuzz tool previews every set, so every card is
  fuzzed from the day it lands; the golden traces deal from the shipped sets, so no batch moves a
  recorded game.
- `catalog check` holds the set to its shape (MP2's indices, ids, rarity ceilings) and keeps it out of
  the totals and the tag census; the cards crate's fixture holds each entry to its row and lets a row
  wait for its card. The fixture's tag lists (Jlockeed, Catalyst, Prime) take #4, #16 and #16.1 as they
  land.
- The Potion tag is in the engine's `Tag` union and the catalog check now; the server's
  `cards_tags_check` admits it with the release's migration, and its test waits for that while the set is
  held ([[R1178000]]); the deck builder's `FILTER_TAGS` gains it with the release.

**What a card batch never touches**: the golden traces, the seed-pinned tests, the pending fragments,
the shipped totals and tag counts, and other sets' cards' behaviour, except where a part's system says
so (MP5). A batch whose change moves a golden trace changed something that ships: it says what and why,
or it is a bug.

**What a card batch adds**, per card (`docs/ADDING_CARDS.md`, a set that has not shipped): the
`catalog.json` entry (after the previous Meditative+ entry in index order, its `loc` from `cargo jackioh
catalog loc`; the census and fixture are already written), the script file with its tests, the
`card-audio.json5` entry, the `flavour.json` line and the `chinese.json` entry, and corrections to its
own [[§8.9]], [[§7]], BUILD M11 and `docs/radiant-audit.md` rows where its build decided a detail
differently. New verbs, statuses and hooks get their [[§6]] rows and ruling notes in the part that
builds them.

**The dry run.** The foundation added a temporary Meditative+ card and ran every suite on it, then
removed it, so the first batch meets no surprise; what it found is in its pull request.

## MP4. The four patches beside the set

Each changes every game when it merges, with its own ruling block; the set's cards are written in its
terms from the start.

| Issue | Change | What the set takes from it |
| --- | --- | --- |
| #1174 Freeze | A status, Frozen, and the verb Freeze, Hearthstone-style: a Frozen Unit loses its next attack or position switch and thaws at the cleanup of the turn it could have acted (`combat::has_exertion_in`) | #24 Bob's first mode, "Freeze all enemy Units"; B07 waits for it |
| #1175 Embiggened | An embiggen card's cheaper price plays its normal version, its dearer price its Embiggened version; R366's "Paid (N): …" becomes "Embiggened (N): …" on the eight shipped embiggen cards | #3, #4, #5, #19 and #23 print "Embiggened (N): …"; #22 and #23 name the class; B01 and B03 wait for it |
| #1176 CN | An instance in Chinese (`CardInstance.chinese`) has the CN tag wherever tags are read (`query::tags_of`) | #21 Harmony Spirit's Radiant Rush Tokens, summoned in Chinese, are CN; B07 waits for it |
| #1177 Tokens | Every token says Token on its type line and on the board | #1.1, #1.2, #16.1 and #20.1, which its catalog test holds to the four signals once they land |

## MP5. The systems, and the decisions

One part builds each system, with its engine tests through fixture scripts
(`crates/engine/tests/rules/fixtures/`), its [[§6]] rows and its rulings; a part that needs another's
system waits for it ("Blocked by").

| System | What it is | Cards | Built by |
| --- | --- | --- | --- |
| The gate's second set, Potion, `embiggen` | `SetName::MeditativePlus` held ([[R1178000]]); the Potion tag; the catalog query's `embiggen: true`, "a card with two prices" ([[R1178001]]) | all; #14, #15; #22, #23 | the foundation, #1178 |
| Freeze | Frozen status and the Freeze verb | #24 | #1174 |
| Embiggened wording | normal and Embiggened versions; the "Embiggened (N): …" label | #3, #4, #5, #19, #22, #23 | #1175 |
| CN for cards in Chinese | `tags_of` holds CN for a translated instance | #21 | #1176 |
| Token labels | Token on every token's type line and board face | #1.1, #1.2, #16.1, #20.1 | #1177 |
| none new | existing verbs and keywords | #3, #5, #8, #10, #12, #13, #14, #15, #19 | B01, #1179 |
| Spell Damage beyond Units | a player's Spell Damage as one sum: Units' keyword, Field Spell auras (one or both players), and a "+N this turn" player modifier | #9, #11 | B02, #1180 |
| A Spell-wide keyword aura | "your Spells have Lifesteal", an aura over the resolving Spell ([[R1044]]'s pattern) | #7 | B02 |
| A healing multiplier | doubled healing on a player's characters at the `healed` replacement ([[R413]], [[R462]], [[R1160]]) | #7 | B02 |
| Health as the price | the next Spell paid in health (3 × its cost), read by `price_of`, `legal_actions` and the payment ([[R18]]) | #6 | B02 |
| After a Spell targeting an enemy | a resolving Spell's enemy targets kept, and a hand-and-deck trigger that casts the card at them ([[R464]]) | #17 | B02 |
| In N turns | a `delay` counted in the caster's turns, with a public countdown mark ([[R437]]) | #4 | B03, #1181 |
| A price for one version | a cost override on the Embiggened price alone | #23 | B03 |
| Mountaintop's aura | the Embiggened effect at the normal price; the dearer price turned into a Radiant play | #22 | B03 |
| Potion and from-scratch crafting | ME-CRAFT's potion mode (a fixed Potion frame, cost (1), HUGE at (3) with five times the points) and from-scratch mode (an empty canvas, presets kept for the AI); the client's block editor for both | #16, #16.1, #26 | B04, #1182 |
| The lowest win rate, a two-pick fuse | `win_rate_leader`'s mirror ([[R1100]]); a fuse of two picks ([[R1108]]) | #2 | B05, #1183 |
| The autopilot | the rest of a player's turn played by the engine's scorer ([[§10.7]]), bounded (`AUTOPILOT_MAX_ACTIONS`) | #18 | B05 |
| Code blocks | each card's script excerpt compiled in by `build.rs`; a Discover shown as code alone | #25 | B05 |
| A question sequence | C+ #42's `answer` prompt asked again, with Stop, the streak in a `Resume` record ([[R420]]) | #20 | B06, #1184 |
| Keywords by X | conditional keywords on the summoned X (C #69's shape) | #20.1 | B06 |
| Set stats | a Unit's Attack and Health set, its damage and stat buffs cleared | #21 | B07, #1185 |
| An enemy Unit into your hand | the field to your hand as yours ([[R747]]'s ownership move) | #24 | B07 |
| Ascent levels, the plan table | raising M #8's play count ([[R429]]) on every copy you own; Bob's Plan's helper per Wincon card | #1, #1.1, #1.2 | B07 |
| The AI | every new prompt kind and mode answered; the sweep, at most 13 of the 26 banned ([[R186]]) | all | #1186 |

**Decisions across the set** (these win over any entry in MP6 and over the batch issues' tables):

- **D1 The gate.** Meditative+ is held by [[R1420]] until #1187 ([[R1178000]]); no batch adds a pending
  fragment or moves a golden trace.
- **D2 R275's stat half** holds as written, every Radiant Unit at twice its stats (MP2).
- **D3 Bob's Plan's help.** The Wincon pool is the shipped cards tagged Wincon: M #8 Reach the Summit
  and M #20 Aestheticize the Game. The easiest way to each, and the AI card that helps:
  - **Reach the Summit** wins at Ascent 10, its play count ([[R429]]), each level costing its number:
    the help is levels and mana. **M+ #1.1 Base Camp** (0) Spell, AI, Token: "Every Reach the Summit
    you own goes up 2 Ascent levels. Add 2 Coins to your hand." Radiant: 3 levels and 2 Radiant Coins.
  - **Aestheticize the Game**'s easiest line is the hero's health (30 to 100, Radiant 90): a heal of a
    hero has no cap ([[§6.3]]). **M+ #1.2 Spa Day** (2) Spell, AI, Token: "Heal your hero 25. Shuffle 2
    Love Bombs into your deck." (M #30.1, 7 each). Radiant: heal 50, Radiant Love Bombs (14 each). The
    player still chooses M #20's line; the help suits the health line.
  - A Wincon card the table does not name is added alone. Each plan is one AI card that carries its own
    support (the Coins, the Love Bombs), so Bob's Plan's text stays one line.
- **D4 Embiggen cards** use #1175's words: "embiggen card" is the class ([[R1178001]]), "normal
  version" and "Embiggened version" its two plays, `Embiggened (N): …` the label.
- **D5 Names** keep straight apostrophes; #17 is Dogpile (MP2).
- **D6 Rulings**: each MP proposal becomes an R-number from its part's block (MP10).
- **D7** The ⚠ readings are the build's defaults (MP7).
- **D8** The [[§8.9]], [[§7]], BUILD M11 and `docs/radiant-audit.md` rows are written by the foundation
  from this brief; a batch corrects its own rows when its build decides otherwise, and says so.
- **D9 New state stays out of shipped games' hashes.** A field the set adds to the state, an instance or
  a view is optional and skipped when absent, so a game that never uses it serializes, hashes and
  replays as before and no golden trace moves (`docs/meditative-set.md` D14).
- **D10 Crafted potions** are transient definitions of the Meditative+ set, tagged Potion, so they meet
  every rule a Potion card meets but are in no pool ([[R77]], [[R882]]).

## MP6. Cards, card by card

Entries in number order, each token after its card. Engine names are the Rust ones
(`crates/engine/src/effects/*.rs` verbs, `script.rs` hooks). Where a decision in MP5 (D1–D10) differs
from an entry, the decision wins.

#### Meditative+ #1 · Bob's Plan
`meditativeplus-001` · (1) Spell, Quickdraw · Mythic · built by #1185

> **Designer:** (1) Bob's Plan, Quickdraw, Spell, Mythic · Add a Wincon card to your hand & supporting cards to help you achieve it {for AI creating this: you determine the easiest way to achieve it and help out, you may make some AI cards to help if you deem necessary} ~~~ Add a Radiant Wincon card to your hand & supporting cards to help you achieve it

- **Text:** Add a random Wincon card to your hand, and its plan: a Base Camp for Reach the Summit, a Spa Day for Aestheticize the Game.
- **Radiant:** Add a random Radiant Wincon card to your hand, and its Radiant plan: a Radiant Base Camp for Reach the Summit, a Radiant Spa Day for Aestheticize the Game.
- **Engine:** One pick, by the match rng, from the Wincon pool (`{ tags: [Wincon] }`, the shipped sets', [[R380]]; today M #8 Reach the Summit and M #20 Aestheticize the Game), added to your hand (`add_to_hand { defId }`), then the helper token its plan names, from a table in the script keyed by catalog id (`WINCON_PLANS`: `meditative-008` → M+ #1.1 Base Camp, `meditative-020` → M+ #1.2 Spa Day), added after it. A Wincon card with no plan (a later set's) is added alone. The Radiant face adds both Radiant (`radiant: true`). A full hand burns what does not fit, in that order ([[§2.4]]). Quickdraw ([[R640]]).
- **Rulings:** MP-B07-1: the plan is the helper the brief defines for the card drawn, never a random one; a Wincon card with no plan is added alone. The pick is one `rng.pick` over the pool in catalog order, so it replays ([[R60]]).
- **Numbers:** none (the pool and the plans are the frame).
- **Check:** The designer's "{for AI creating this: …}" is answered by MP5's D3: the easiest way to each Wincon, and two AI cards (M+ #1.1, #1.2) as the help. R275: Radiant Wincon card and Radiant helpers, both stronger. Quickdraw as written.
- **Class:** C (the plan table; Ascent levels as a verb for #1.1).

#### Meditative+ #1.1 · Base Camp
`meditativeplus-001-1` · (0) Spell, AI, Token · Token (no printed rarity) · built by #1185

> **Designer:** (the designer: "you may make some AI cards to help"; defined by this brief, MP5 D3)

- **Text:** Every Reach the Summit you own goes up {levels} Ascent levels. Add {coins|Coin|Coins} to your hand.
- **Radiant:** Every Reach the Summit you own goes up {levels} Ascent levels. Add {coins|Radiant Coin|Radiant Coins} to your hand.
- **Engine:** NEW: Ascent levels as a verb (B07): raise M #8 Reach the Summit's play count ([[R429]], `counts_plays`) by N on every instance its caster owns, in every zone (hand, deck, graveyard, exile, the field's stack while it resolves), each a public `numberChanged` on a revealed card and hidden in a hand or deck ([[R97]]). Then `add_to_hand { defId: core-t-coin, count }` (Radiant: `radiant: true`). An instance at level 10 or above wins when it is next played (ME-WIN, [[R850]]); raising the count never wins by itself.
- **Rulings:** MP-B07-2: "you own" is the owner ([[§3.2]]), wherever the card is; a copy made later starts at 0 ([[R57]]); a Reach the Summit you do not own is untouched.
- **Numbers:** levels 2 ↑ (Radiant 3); coins 2 ↑ (the Radiant face's Coins are Radiant: 2 mana each).
- **Check:** Defined by this brief (⚠ designer, MP7). R275: 3 levels and Radiant Coins against 2 and Coins.
- **Class:** C (Ascent levels).

#### Meditative+ #1.2 · Spa Day
`meditativeplus-001-2` · (2) Spell, AI, Token · Token (no printed rarity) · built by #1185

> **Designer:** (the designer: "you may make some AI cards to help"; defined by this brief, MP5 D3)

- **Text:** Heal your hero {heal}. Shuffle {bombs|Love Bomb|Love Bombs} into your deck.
- **Radiant:** Heal your hero {heal}. Shuffle {bombs|Radiant Love Bomb|Radiant Love Bombs} into your deck.
- **Engine:** `heal` on the caster's hero (no cap, [[§6.3]] Heal), then `shuffle_into { defId: meditative-030-1, count }` (Radiant: `radiant: true`), each at a random position ([[R311]]), refused by a full deck ([[R80]]). Each Love Bomb heals on draw (M #30.1).
- **Rulings:** none new: Heal has no cap on a hero, so it climbs toward M #20's 100.
- **Numbers:** heal 25 ↑ (Radiant 50, step 5); bombs 2 ↑.
- **Check:** Defined by this brief (⚠ designer, MP7): M #20's easiest line is the hero's health (30 to 100, Radiant 90), which nothing caps. R275: heal 50 and Radiant Love Bombs (14 each) against 25 and 7.
- **Class:** B.

#### Meditative+ #2 · NA Tech
`meditativeplus-002` · (1) Spell · Epic · built by #1183

> **Designer:** (1) NA Tech, Spell, Epic · Fuse the lowest winrate card & highest winrate card, add it to your hand, it costs (2) ~~~ Fuse the Radiant lowest winrate card & Radiant highest winrate card, add it to your hand, it costs (0)

- **Text:** Fuse the card with the lowest win rate and the card with the highest win rate, and add it to your hand. It costs ({setCost}).
- **Radiant:** Fuse a Radiant card with the lowest win rate and a Radiant card with the highest win rate, and add it to your hand. It costs ({setCost}).
- **Engine:** NEW: the lowest win rate (B05), `win_rates::win_rate_trailer` beside M #50 CN Tech's `win_rate_leader` ([[R1100]]): the same compiled table (`crates/cards/data/win_rates.json`, which ships empty), the same eligibility (`CN_TECH_MIN_GAMES` games, non-token cards of the sets that ship) and ties broken the same way read from the other end. NEW: a two-ingredient fuse from two picks (`fuse_generated`'s shape, one ingredient per pick, [[R1108]]), added with `costOverride` ([[R77]], [[R102]]); the Radiant face makes both ingredients Radiant first. With no eligible card a random non-token card stands in for each missing ingredient, as M #50's fallback, and two different cards are fused (the trailer skips the leader).
- **Rulings:** MP-B05-1: the lowest is the leader's mirror over the same table and floor; with one eligible card it is the leader and a random card the other; the fused card is a transient definition ([[R77]]), never in a pool.
- **Numbers:** setCost 2 ↓ (Radiant 0).
- **Check:** "winrate" is R1100's win rate. The table ships empty, so until a stats run fills it the card fuses two random cards (⚠ designer). R275: Radiant ingredients and (0) against (2).
- **Class:** C (the trailer, the two-pick fuse).

#### Meditative+ #3 · Wondrous Wand
`meditativeplus-003` · (2, Embiggened 4) Spell · Epic · built by #1179

> **Designer:** (2 embiggen 4) Wondrous Wand, Spell, Epic · Draw 1 embiggen 2 cards, they cost (0) ~~~ Draw 2 embiggen 5 cards, they cost (0)

- **Text:** Draw {draw|card|cards}. They cost (0). / Embiggened (4): Draw {embiggenedDraw} instead.
- **Radiant:** Draw {draw|card|cards}. They cost (0). / Embiggened (4): Draw {embiggenedDraw} instead.
- **Engine:** C #79 Risky Die's pattern: note the caster's library on the resolving card (`remember`), `draw { count }` (the count read off `instance.embiggened`, [[R81]]), then `set_cost_override 0` on each card now in the hand that was in that library, so a card cast on draw, a burned card or a card created meanwhile is not one of them ([[R58]], [[§2.4]]).
- **Rulings:** "They" are the drawn cards that reached the hand, as C #79; the override is the card's cost in every zone until it leaves the hand ([[R766]]).
- **Numbers:** draw 1 ↑ (Radiant 2); embiggenedDraw 2 ↑ (Radiant 5).
- **Check:** The designer's "embiggen" is the Embiggened version (#1175). R275: 2 and 5 cards against 1 and 2.
- **Class:** B.

#### Meditative+ #4 · Supply Drop
`meditativeplus-004` · (2, Embiggened 4) Spell, Jlockeed · Epic · built by #1181

> **Designer:** (2 embiggen 4) Supply Drop, Spell, Jlockheed, Epic · Choose Combat or Supplies: Combat: Pierce, Lifesteal, In 3 turns add deal 4 damage to all enemies, embiggen 5 turns 9 damage · Supplies: In 3 turns add a Radiant Jlockheed card to your hand, it costs (2) embiggen 5 turns 5 Radiant Jlockheed cards ~~~ Choose Combat or Supplies: Combat: Pierce, Lifesteal, In 3 turns add deal 9 damage to all enemies, embiggen 5 turns 15 damage · Supplies: In 3 turns add 2 Radiant Jlockheed card to your hand, it costs (0) embiggen 5 turns fill your hand with Radiant Jlockheed cards

- **Text:** Choose one: / Combat: In {turns} turns, deal {damage} damage to each enemy, with Pierce and Lifesteal. / Supplies: In {turns} turns, add {cards|random Radiant Jlockeed card|random Radiant Jlockeed cards} to your hand. They cost ({suppliesCost}). / Embiggened (4): In {embiggenedTurns} turns instead; Combat deals {embiggenedDamage}, Supplies adds {embiggenedCards}.
- **Radiant:** Choose one: / Combat: In {turns} turns, deal {damage} damage to each enemy, with Pierce and Lifesteal. / Supplies: In {turns} turns, add {cards|random Radiant Jlockeed card|random Radiant Jlockeed cards} to your hand. They cost ({suppliesCost}). / Embiggened (4): In {embiggenedTurns} turns instead; Combat deals {embiggenedDamage}, Supplies fills your hand.
- **Engine:** Two declared modes ([[R81]]). NEW: "in N turns" (B03), a `delay` whose `not_before` counts the caster's turns (cast on your turn T, it resolves at the start of your turn N turns later, before the draw, [[R458]]'s timing), with a public countdown mark on the caster's side ([[R437]]) that a Spell leaves behind, since the card is in the graveyard by then. Combat: one hit on the enemy hero and on every enemy Unit, Pierce ([[R346]]) and Lifesteal ([[R85]]) carried by the delayed effect, all before one state check ([[R59]]). Supplies: `add_random_from_catalog { tags: [Jlockeed], radiant, costOverride }` excluding Supply Drop itself ([[R387]]); the Radiant Embiggened face fills the hand (M #77's count, `hand_cap_of` less the hand).
- **Rulings:** MP-B03-1: "in 3 turns" is the start of your third turn from now, counted in your own turns (an extra turn counts, [[R846]]); the delayed effect is the caster's, kept if the card leaves the graveyard; "all enemies" is the enemy hero and every enemy Unit; the Lifesteal heals the caster's hero once per hit.
- **Numbers:** turns 3 ↓; embiggenedTurns 5 ↓; damage 4 ↑ (Radiant 9); embiggenedDamage 9 ↑ (Radiant 15); cards 1 ↑ (Radiant 2); suppliesCost 2 ↓ (Radiant 0); embiggenedCards 5 ↑ (the base face only, `tunedOn: base`, the Radiant face filling the hand, [[R1431]]).
- **Check:** "Jlockheed" is the tag `Jlockeed` ([[R278]]); "add deal" is a typo for "deal"; "embiggen" is the Embiggened version (#1175). The countdown being public is the brief's (⚠ designer: Hearthstone shows a Dormant count). R275: 9 and 15 against 4 and 9; 2 cards at (0) and a full hand against 1 at (2) and 5.
- **Class:** C (in N turns).

#### Meditative+ #5 · Spirit Lash
`meditativeplus-005` · (0, Embiggened 1) Spell · Rare · built by #1179

> **Designer:** (0 embiggen 1) Spirit Lash, Spell, Rare · Deal 1 damage to all permanents embiggen Lifesteal, Pierce ~~~ Deal 1 damage to all permanents embiggen Lifesteal, Pierce · Echo 1

- **Text:** Deal {damage} damage to all Units. / Embiggened (1): With Lifesteal and Pierce.
- **Radiant:** Echo 1 / Deal {damage} damage to all Units. / Embiggened (1): With Lifesteal and Pierce.
- **Engine:** `damage_all` over every Unit on both sides, acting animated cards included, one hit each ([[R59]]); Embiggened, the hits carry `DamageFlags { lifesteal, pierce }` ([[R85]], [[R346]]). The Radiant face prints Echo 1 (`StaticFlags.echo`, [[§6.2]]): the repeat is Embiggened if the play was (the instance keeps the flag, [[R81]]).
- **Rulings:** MP-B01-1: "permanents" that take damage are the Units (a backrow card takes none unless it is an acting animated Unit); heroes are not permanents.
- **Numbers:** damage 1 ↑.
- **Check:** "embiggen" is the Embiggened version. The designer's Radiant face adds Echo 1 only: R275's effect half by a second resolution (2×).
- **Class:** A.

#### Meditative+ #6 · Bloodbloom
`meditativeplus-006` · (0) Spell · Epic · built by #1180

> **Designer:** (0) Bloodbloom, Spell, Epic · Your next spell costs [spell_cost * 3] health instead of mana ~~~ Your next spell costs [spell_cost * 3] health instead of mana, it also gains Echo

- **Text:** Your next Spell costs Health equal to {multiple} times its Cost instead of mana.
- **Radiant:** Your next Spell costs Health equal to {multiple} times its Cost instead of mana, and has Echo 1.
- **Engine:** NEW: health as the price (B02), a player modifier on the caster, until a Spell uses it: `mana::price_of` prices the next Spell (not a Field Spell or a Trap, [[§5.1]]) at 0 mana and `multiple ×` its cost after every modifier ([[R65]]; an X Spell its X, [[R348]]) in health, paid with the play as `lose_health` ([[R18]]); `legal_actions` offers a Spell that only the health price affords. A cast pays nothing and spends nothing ([[R70]]). The Radiant face also grants Echo 1 to that Spell (`EchoNextSpell`'s shape, [[R30]]).
- **Rulings:** MP-B02-1: the next Spell you play, from your hand, after this resolves; it may cost you the game, as Hearthstone's Bloodbloom (a hero at 0 loses at the state check); a (0) Spell costs 0 health and still uses it; several Bloodblooms do not stack, the newest replacing.
- **Numbers:** multiple 3 ↓.
- **Check:** "[spell_cost * 3]" is 3 × the Spell's cost; "gains Echo" is Echo 1. R275: Echo 1 on the discounted Spell, a whole second resolution.
- **Class:** C (health as the price).

#### Meditative+ #7 · Life Mage
`meditativeplus-007` · (1) Unit, Human · Common · 1/2 → 2/4 · built by #1180

> **Designer:** (1) Life Mage, Unit, Common, Human · Divine Shield · [1/2] · Aura: Your spells have lifesteal ~~~ Divine Shield · [2/4] · Aura: Your spells have lifesteal, and healing is twice as effective

- **Text:** Divine Shield / Aura: Your Spells have Lifesteal.
- **Radiant:** Divine Shield / Aura: Your Spells have Lifesteal, and healing your hero and Units is twice as effective.
- **Engine:** NEW: a Spell-wide keyword aura (B02): while it acts, a resolving Spell of its controller's (not a Field Spell or a Trap) has Lifesteal, read by damage step 8 ([[R85]]) as an `AuraEntry` matching the resolving card (`alt_play::cloak_hand_aura`'s pattern, [[R1044]]). NEW on the Radiant face: a healing multiplier at the `healed` replacement moment ([[R413]], [[R462]]; [[R1160]]'s "twice as effective"), doubling every heal of its controller's hero and Units, Lifesteal's included.
- **Rulings:** MP-B02-2: two Life Mages give Lifesteal once and double healing twice (×4), as two Prophet Velens do in Hearthstone; a Spell's Lifesteal heals once per hit; a heal the doubling turns into Blood Moon's damage ([[R413]]) is not doubled.
- **Numbers:** none (the doubling is the rule's 2, `HEAL_MULTIPLIER`).
- **Check:** "spells" are Spells ([[§5.1]]); "healing is twice as effective" read for its controller's characters (⚠ designer: or all healing). R275: 2/4 and the doubled healing.
- **Class:** C (the Spell aura, the heal multiplier).

#### Meditative+ #8 · Arch Mage
`meditativeplus-008` · (2) Unit, Human · Common · 4/7 → 8/14 · built by #1179

> **Designer:** (2) Arch Mage, Unit, Common, Human · [4/7] · Spell damage +3 ~~~ [8/14] · Spell damage +6

- **Text:** Spell Damage +3
- **Radiant:** Spell Damage +6
- **Engine:** The Spell Damage keyword (`Keyword::SpellDamage`, [[§6.1]]; C+ #38 Solarius): its controller's Spells deal 3 more per hit ([[§4.4]] step 0).
- **Rulings:** none new.
- **Numbers:** none: Spell Damage's 3 (Radiant 6) is a numbered keyword, which a Nerf or Buff moves as an X ([[R482]], as C+ #38).
- **Check:** "Spell damage +3" is the keyword. R275: 8/14 and +6.
- **Class:** A.

#### Meditative+ #9 · Magic Amplifier
`meditativeplus-009` · (1) Field Spell · Common · built by #1180

> **Designer:** (1) Magic Amplifier, Field Spell, Common · Spell damage +2 for both players ~~~ Spell damage +5 for both players

- **Text:** Aura: Both players have Spell Damage +{spellDamage}.
- **Radiant:** Aura: Both players have Spell Damage +{spellDamage}.
- **Engine:** NEW: Spell Damage beyond Units (B02): `damage::spell_damage_for` sums its Units' keyword, Field Spell auras that give a player Spell Damage (this one gives both), and the player modifier of M+ #11, in that one place ([[§4.4]] step 0).
- **Rulings:** MP-B02-3: a player's Spell Damage is one sum, whatever gives it; it is read as each hit of a Spell lands.
- **Numbers:** spellDamage 2 ↑ (Radiant 5).
- **Check:** R275: +5 against +2, for both players as the designer wrote (a symmetric card, as written).
- **Class:** C (Spell Damage from an aura).

#### Meditative+ #10 · Medina Grifter
`meditativeplus-010` · (1) Unit, Human · Common · 4/1 → 8/2 · built by #1179

> **Designer:** (1) Medina Grifter, Unit, Human, Common · [4/1] · Cry: Steal a card from your opponents hand ~~~ [8/2] · Cry: Steal a card from your opponents hand, it costs (0)

- **Text:** Cry: Steal a random card from your opponent's hand.
- **Radiant:** Cry: Steal a random card from your opponent's hand. It costs (0).
- **Engine:** `give_from_hand { from: enemy, cards: random, count: 1 }` (M #21, C #9): the card becomes yours (Steal off the field, [[R12]]), hidden per zone ([[R97]]), the hand cap burning it into your graveyard ([[§2.4]]); the Radiant face's `TakenRiders.cost_override` 0.
- **Rulings:** MP-B01-2: "a card" is a random one (the opponent's hand is hidden, [[§9.1]]); an empty hand steals nothing and draws no random number ([[R129]]).
- **Numbers:** cards 1 ↑.
- **Check:** R275: 8/2 and the stolen card at (0).
- **Class:** B.

#### Meditative+ #11 · Living Mana Potion
`meditativeplus-011` · (1) Unit, Potion · Rare · 5/1 → 10/2 · built by #1180

> **Designer:** (1) Living Mana Potion, Unit, Rare, Potion · [5/1] · Activate: tribute this and gain +5 Spell damage for the rest of the turn ~~~ [10/2] · Activate: gain +5 Spell damage for the rest of the turn

- **Text:** Activate: Tribute this. You have Spell Damage +{spellDamage} this turn.
- **Radiant:** Activate: You have Spell Damage +{spellDamage} this turn.
- **Engine:** An Activate ([[§6.2]], [[R384]]): `ActivationCost { tribute_self }` on the base face (C #84, M #54), none on the Radiant face (once a turn). NEW: a player modifier "Spell Damage +N this turn" (B02), expiring at the turn's cleanup, read in `spell_damage_for` with the rest (M+ #9).
- **Rulings:** MP-B02-4: the bonus is the activator's, for the rest of that turn, and stays when the potion leaves; two activations stack.
- **Numbers:** spellDamage 5 ↑.
- **Check:** R275: 10/2 and the Tribute gone, so the bonus comes every turn.
- **Class:** C (the Spell Damage modifier).

#### Meditative+ #12 · Living Armor Potion
`meditativeplus-012` · (1) Unit, Potion · Rare · 1/5 → 2/10 · built by #1179

> **Designer:** (1) Living Armor Potion, Unit, Rare, Potion · [1/5] · Activate: tribute this and your hero gains +5 Armor unit the start of your next turn ~~~ [2/10] · Activate: your hero gains +5 Armor unit the start of your next turn

- **Text:** Activate: Tribute this. Your hero has +{armor} Armor until the start of your next turn.
- **Radiant:** Activate: Your hero has +{armor} Armor until the start of your next turn.
- **Engine:** An Activate (`tribute_self` on the base face) adding the `heroArmor` player modifier (Core #98 Armor Up, [[R757]]) that expires at the start of its controller's next turn (`expiry nextTurnOf`).
- **Rulings:** none new: two activations stack, each expiring at that start of turn.
- **Numbers:** armor 5 ↑.
- **Check:** "unit" is "until". R275: 2/10 and no Tribute.
- **Class:** B.

#### Meditative+ #13 · Living Health Potion
`meditativeplus-013` · (1) Unit, Potion · Rare · 3/3 → 6/6 · built by #1179

> **Designer:** (1) Living Health Potion, Unit, Rare, Potion · [3/3] · Activate: tribute this and heal all allies 5 health ~~~ [6/6] · Activate: heal all allies 5 health

- **Text:** Activate: Tribute this. Heal your hero and each of your Units {heal}.
- **Radiant:** Activate: Heal your hero and each of your Units {heal}.
- **Engine:** An Activate (`tribute_self` on the base face, paid first, so the base potion heals the others), then `heal` on its controller's hero and `for_each_card` over its Units (M #7, M #22).
- **Rulings:** MP-B01-3: "all allies" is your hero and your Units (Hearthstone's "all friendly characters"); the Radiant potion heals itself too.
- **Numbers:** heal 5 ↑.
- **Check:** R275: 6/6 and no Tribute.
- **Class:** B.

#### Meditative+ #14 · Potion Pouch
`meditativeplus-014` · (1) Spell · Rare · built by #1179

> **Designer:** (1) Potion Pouch, Spell, Rare · Add 2 Potions to your hand ~~~ Add 2 Radiant Potions to your hand

- **Text:** Add {cards|random Potion card|random Potion cards} to your hand.
- **Radiant:** Add {cards|random Radiant Potion card|random Radiant Potion cards} to your hand.
- **Engine:** `add_random_from_catalog { query: { tags: [Potion] }, count }` (the Radiant face `radiant: true`): the non-token Potion cards of the sets that ship ([[R380]]), repeats allowed ([[R60]]); a crafted potion (M+ #16) is transient and in no pool.
- **Rulings:** MP-B01-4: "Potions" are Potion-tagged cards: M+ #11, #12 and #13 once the set ships; under a preview the same three ([[R1178000]]).
- **Numbers:** cards 2 ↑.
- **Check:** R275: the potions Radiant (twice the stats, no Tribute).
- **Class:** B.

#### Meditative+ #15 · Book of Potions
`meditativeplus-015` · (1) Spell, Book · Epic · built by #1179

> **Designer:** (1) Book of Potions, Spell, Epic, Book · Add 1 Radiant Potion to your hand ~~~ Add 2 Radiant Potion to your hand

- **Text:** Add {cards|random Radiant Potion card|random Radiant Potion cards} to your hand.
- **Radiant:** Add {cards|random Radiant Potion card|random Radiant Potion cards} to your hand.
- **Engine:** As M+ #14, Radiant on both faces.
- **Rulings:** as M+ #14. The Book family's rarity is Epic ([[§8]]), as the designer printed.
- **Numbers:** cards 1 ↑ (Radiant 2).
- **Check:** R275: 2 against 1.
- **Class:** B.

#### Meditative+ #16 · Jazakus
`meditativeplus-016` · (1) Unit, Human, Catalyst · Epic · 2/2 → 4/4 · built by #1182

> **Designer:** (1) Jazakus, Unit, Epic, Human, Catalyst · [2/2] · Cry: Craft a custom potion {costs (1)} · Death: Shuffle Jazakus Prime into your deck ~~~ [4/4] · Cry: Craft a Radiant custom potion {costs (1)} · Death: Shuffle Radiant Jazakus Prime into your deck

- **Text:** Cry: Craft a custom Potion. It costs (1). / Death: Shuffle a Jazakus Prime into your deck.
- **Radiant:** Cry: Craft a custom Radiant Potion. It costs (1). / Death: Shuffle a Radiant Jazakus Prime into your deck.
- **Engine:** NEW: the crafter's potion mode (B04), on M #17's ME-CRAFT ([[R880]] to [[R883]], `subsystems/craft.rs`): the recipe's frame is fixed, a Unit with the Potion tag whose one hook is "Activate: Tribute this. …" with the effects the player builds, at cost (1) and the crafter's budget for (1) (`CRAFT_POTION_COST`); the stats are in the recipe. The crafted potion is a transient definition named by its recipe's digest ([[R882]]), of the Meditative+ set, tagged Potion, added to the hand. Death: `shuffle_into { defId: meditativeplus-016-1 }` (C+ #38's Catalyst shape; Radiant face Radiant).
- **Rulings:** MP-B04-1: a custom potion is a Unit whose Activate tributes it, so it is a Potion like M+ #11 to #13; the AI, the random policy and a timeout answer the prompt with a preset ([[R881]]).
- **Numbers:** none on the card (the potion's cost and budget are the crafter's named constants).
- **Check:** "{costs (1)}" is the crafted potion's cost. R275: 4/4, the potion and the Prime Radiant.
- **Class:** D (the potion crafter).

#### Meditative+ #16.1 · Jazakus Prime
`meditativeplus-016-1` · (1) Unit, Human, Prime, Token · Token (printed Epic) · 8/8 → 16/16 · built by #1182

> **Designer:** (1) Jazakus Prime, Unit, Epic, Human, Prime, Token · [8/8] · Cry: Craft a HUGE custom potion {costs (3) ~ about 5 times better than the normal custom potion} ~~~ [16/16] · Cry: Craft a Radiant HUGE custom potion {costs (3) ~ about 5 times better than the Radiant normal custom potion}

- **Text:** Cry: Craft a HUGE custom Potion. It costs (3).
- **Radiant:** Cry: Craft a HUGE custom Radiant Potion. It costs (3).
- **Engine:** The potion mode at cost (3) with `CRAFT_HUGE_POTION_BUDGET_MULTIPLE` (5) times the normal potion's points (and lines, `CRAFT_HUGE_POTION_LINES`), so it is about five times a normal potion; otherwise M+ #16's crafter.
- **Rulings:** MP-B04-2: "about 5 times better" is five times the points budget; the frame is the same.
- **Numbers:** none on the card.
- **Check:** A token (printed Epic, [[R1178002]]): only M+ #16's Death makes one. R275: 16/16 and a Radiant HUGE potion.
- **Class:** D (the potion crafter).

#### Meditative+ #17 · Dogpile
`meditativeplus-017` · (1) Spell · Rare · built by #1180

> **Designer:** (1) Pile On, Spell, Rare · After a spell targeting an enemy resolves, this targets them dealing 2 damage (from anywhere in your hand or deck) ~~~ After a spell targeting an enemy resolves, this targets them dealing 5 damage (from anywhere in your hand or deck)

- **Text:** Deal {damage} damage to an enemy. / In your hand or deck: After another Spell of yours that targeted an enemy resolves, cast this at it.
- **Radiant:** Deal {damage} damage to an enemy. / In your hand or deck: After another Spell of yours that targeted an enemy resolves, cast this at it.
- **Engine:** A targeted Spell (an enemy Unit or the enemy hero, [[R81]]). NEW (B02): the targets a Spell declared or chose in a prompt, kept with the resolving card, and a hand-and-deck trigger (`TriggerZone::Hand`, `Library`, [[R464]]) on that Spell's resolution that casts this card ([[§6.3]] Cast: free, a play, [[R70]]) at the first such enemy still there. One Dogpile answers each Spell, the one in hand first (leftmost), else the topmost in the deck; its own cast is a Spell that targeted an enemy, so it can call the next.
- **Rulings:** MP-B02-5: "a spell" is a Spell of yours, other than this card; "them" is the enemy it targeted, a Unit or the hero; a target gone or no longer an enemy casts nothing; a Spell that targeted several enemies calls it at the first in declaration order.
- **Numbers:** damage 2 ↑ (Radiant 5).
- **Check:** Classic #60 is Pile On, so this is Dogpile (R381, [[R1178002]], ⚠ designer). R275: 5 against 2.
- **Class:** C (the targets kept, the hand-and-deck cast).

#### Meditative+ #18 · Cognitive Offloading
`meditativeplus-018` · (1) Spell · Epic · built by #1183

> **Designer:** (1) Cognitive Offloading, Spell, Epic · Cast when drawn: Gain 1 mana & draw 1 card; AI completes the rest of your turn ~~~ Cast when drawn: Gain 3 mana & draw 2 card; AI completes the rest of your turn

- **Text:** Cast on draw: Gain {mana} mana and draw {draw|card|cards}. The AI plays the rest of your turn.
- **Radiant:** Cast on draw: Gain {mana} mana and draw {draw|card|cards}. The AI plays the rest of your turn.
- **Engine:** Cast on draw (`StaticFlags.cast_on_draw`, [[§6.2]], [[R635]]): `gain_mana` (temporary), `draw`, then NEW: the autopilot (B05), a status on the caster for the rest of the turn under which the engine plays for them, one greedy legal action at a time by the §10.7 scorer (`subsystems/scorer.rs`, as M #71's judge, never `crates/ai`), up to `AUTOPILOT_MAX_ACTIONS`, then ends the turn; Core #96 My Pawn's `ai_plays_out_turn` is its random cousin. Played from the hand (past [[R58]]'s cap), it does the same.
- **Rulings:** MP-B05-2: only on its caster's own turn; drawn on the opponent's turn it gains and draws and the autopilot does nothing; a prompt the autopilot meets is answered by the same scorer; the client shows the turn played for you and takes no input but a concession.
- **Numbers:** mana 1 ↑ (Radiant 3); draw 1 ↑ (Radiant 2).
- **Check:** "the AI" is the engine's scorer, deterministic and seeded ([[§9.3]]). R275: 3 mana and 2 cards against 1 and 1.
- **Class:** D (the autopilot).

#### Meditative+ #19 · Grubby Gloves
`meditativeplus-019` · (1, Embiggened 2) Spell · Common · built by #1179

> **Designer:** (1 embiggen 2) Grubby Gloves, Spell, Common · Steal 2 cards from your opponents deck embiggen their hand instead ~~~ Steal 3 cards from your opponents deck embiggen their hand instead, they cost (0)

- **Text:** Steal the top {cards|card|cards} of your opponent's deck. / Embiggened (2): Steal {cards|random card|random cards} from their hand instead.
- **Radiant:** Steal the top {cards|card|cards} of your opponent's deck. They cost (0). / Embiggened (2): Steal {cards|random card|random cards} from their hand instead.
- **Engine:** Normal: `take_from_library` (C+ #12.3) of the opponent's top cards into your hand, yours from then on ([[R12]]); Embiggened: `give_from_hand { from: enemy, cards: random }` (as M+ #10). The Radiant face's `cost_override` 0 on each card taken, either version. The hand cap burns the overflow ([[§2.4]]).
- **Rulings:** MP-B01-5: "2 cards from your opponent's deck" is its top 2, the deck being shuffled (no random number, as [[R801]]); fewer cards, fewer; an empty deck gives nothing and deals no fatigue.
- **Numbers:** cards 2 ↑ (Radiant 3).
- **Check:** R275: 3 at (0) against 2.
- **Class:** B.

#### Meditative+ #20 · KY's Competition
`meditativeplus-020` · (2) Spell, KY · Rare · built by #1184

> **Designer:** (2) KY's Competition, Spell, Rare, KY · Solve questions as you can, summon a [X/X] Lil' Kenny when you are finished or fail to answer a question, where X=number of correct answers in a row ~~~ Solve easy questions as you can, summon a [2X/2X] Lil' Kenny when you are finished or fail to answer a question, where X=number of correct answers in a row · {normal questions triple digit addition / subtraction, easy single or double digit addition / subtraction}

- **Text:** Answer three-digit sums one after another until you stop or miss one. Then summon a Lil' Kenny with X/X, X your correct answers.
- **Radiant:** Answer one- and two-digit sums one after another until you stop or miss one. Then summon a Radiant Lil' Kenny with 2X/2X, X your correct answers.
- **Engine:** NEW: a question sequence (B06): C+ #42 KY's Test's `answer` prompt ([[R420]], `subsystems/ky_test.rs`) asked again after each right answer, with a Stop option, the streak in a plain-data `Resume` record (`work.rs`), each problem rolled from the match rng (three-digit addition or subtraction; the Radiant face one- or two-digit), the key never leaving the engine; at most `KY_COMPETITION_MAX` (30) questions. Then a summon of M+ #20.1 with `stats_override` X/X (Radiant face: Radiant, its own face doubling to 2X/2X, [[R349]]'s rule as written on its face), with X = 0 summoning nothing.
- **Rulings:** MP-B06-1: a wrong answer ends it and X counts the right ones before it; Stop ends it with X so far; a timeout ends it as a Stop; the AI, the random policy and a timeout answer it ([[R79]]).
- **Numbers:** none (the cap is `KY_COMPETITION_MAX`; the digits are the frame).
- **Check:** "in a row": the sequence stops at the first miss, so every answer counted was in a row. The Radiant face's easier questions are R275's effect half; 2X/2X is its Kenny's Radiant face.
- **Class:** D (the question sequence).

#### Meditative+ #20.1 · Lil' Kenny
`meditativeplus-020-1` · (1) Unit, KY, Token · Token (printed Rare) · X/X → 2X/2X · built by #1184

> **Designer:** (1) Lil' Kenny, Unit, Rare, KY, Meditative+, Token · [X/X] · Has the following keywords based on the value of X: 4: Rush / 8: Divine Shield / 12: First Strike / 16: Cleave / 20: Lifesteal / 30: Indestructible ~~~ [2X/2X] · Has the following keywords based on the value of X: 4: Rush / 8: Divine Shield / 12: First Strike / 16: Cleave / 20: Lifesteal / 30: Indestructible

- **Text:** With X at least 4, it has Rush; 8, Divine Shield; 12, First Strike; 16, Cleave; 20, Lifesteal; 30, Indestructible.
- **Radiant:** With X at least 4, it has Rush; 8, Divine Shield; 12, First Strike; 16, Cleave; 20, Lifesteal; 30, Indestructible.
- **Engine:** Printed 0/0 with `xStats` 1/1 (Radiant 2/2), always summoned as X/X (`stats_override`, as the Bread and Ghoul Tokens, [[§7]]). Keywords by X: `Script.conditional_keywords` reading the instance's X (C #69's shape, `layers::conditional_keywords_of`), cumulative.
- **Rulings:** MP-B06-2: the thresholds read X, not the current stats (a buff gives no keyword; the Radiant face's 2X/2X reads X too); a Vanilla removes them ([[R781003]]) and keeps X/X.
- **Numbers:** none (the thresholds are the frame, named constants).
- **Check:** A token (printed Rare, [[R1178002]]): only M+ #20 makes one. R275: 2X/2X, the same keywords.
- **Class:** B.

#### Meditative+ #21 · Harmony Spirit
`meditativeplus-021` · (2) Unit, CN · Rare · 3/7 → 6/14 · built by #1185

> **Designer:** (2) Harmony Spirit, Unit, Rare, CN · [3/7] · Taunt · Cry: Set the stats of all other units to 3/3 ~~~ [6/14] · Taunt · Cry: Set the stats of all other units to 3/3, fill your board with Rush Tokens {they are in Chinese and gain the CN keyword}

- **Text:** Taunt / Cry: Set every other Unit's Attack and Health to 3/3.
- **Radiant:** Taunt / Cry: Set every other Unit's Attack and Health to 3/3. Fill your board with Rush Tokens in Chinese.
- **Engine:** NEW: set stats (B07), a `stats_override` of 3/3 on every other Unit of both sides with its damage and stat buffs cleared (Hearthstone's "set"), an Immutable one untouched ([[R23]]). The Radiant face then fills its controller's board with Rush Tokens (`fill_board`, M #56, [[R942]], Locked zones included, [[R1136000]]) with the `chinese` rider, so each is CN (#1176).
- **Rulings:** MP-B07-3: "set" replaces the attack and the max health and heals to it: a 10/10 with 5 damage becomes 3/3 undamaged; auras still apply on top ([[§10.4]]); the Rush Tokens arrive after the set, at their printed 3/3.
- **Numbers:** none: the set's 3/3 is the frame (it is good for one side and bad for the other, so no `better` direction fits it).
- **Check:** "CN keyword" is the CN tag (#1176). R275: 6/14 and a full board of Rush Tokens.
- **Class:** C (set stats).

#### Meditative+ #22 · Mountaintop Garden
`meditativeplus-022` · (3) Field Spell · Epic · built by #1181

> **Designer:** (3) Mountaintop Garden, Field Spell, Epic · Aura: All embiggened innately gain their Embiggened effect (without paying the extra cost); cards instead of an embiggened (cost +2) effect to make them Radiant (if they are not Radiant already) ~~~ Aura: All embiggened innately gain their Embiggened effect (without paying the extra cost); cards instead of an embiggened (cost +1) effect to make them Radiant (if they are not Radiant already)

- **Text:** Aura: Embiggen cards play their Embiggened version at their normal price. For ({premium}) more, a card that isn't Radiant is played Radiant instead.
- **Radiant:** Aura: Embiggen cards play their Embiggened version at their normal price. For ({premium}) more, a card that isn't Radiant is played Radiant instead.
- **Engine:** NEW: Mountaintop's aura (B03), while it acts, for every embiggen card of both players ([[R1178001]]): a play at the normal price is stamped `embiggened` (read where `play_steps` stamps it; [[R545]]'s casts unchanged, a cast resolving at its base), and the dearer choice (`play_choices::legal_embiggen_choices`) is priced at the normal price + premium and plays the card Radiant and Embiggened; a Radiant card offers the normal price only.
- **Rulings:** MP-B03-2: "All" is both players' embiggen cards (⚠ designer); a cost modifier applies to both prices ([[R65]]); two Gardens are one aura, the cheaper premium applying.
- **Numbers:** premium 2 ↓ (Radiant 1).
- **Check:** "embiggened" is the embiggen card class (#1175); "(cost +2)" is the premium over the normal price. R275: a premium of 1 against 2.
- **Class:** C (the aura over embiggen prices).

#### Meditative+ #23 · Enormous Elderberry
`meditativeplus-023` · (2, Embiggened 4) Spell, Fruit · Epic · built by #1181

> **Designer:** (2 embiggen 4) Enormous Elderberry, Spell, Fruit, Epic · Add a random embiggened card to your hand, its embiggened form costs (0); embiggen add 3 instead of 1 ~~~ Add a random radiant embiggened card to your hand, its embiggened form costs (0); embiggen add 3 instead of 1

- **Text:** Add {cards|random embiggen card|random embiggen cards} to your hand. Their Embiggened version costs (0). / Embiggened (4): Add {embiggenedCards} instead.
- **Radiant:** Add {cards|random Radiant embiggen card|random Radiant embiggen cards} to your hand. Their Embiggened version costs (0). / Embiggened (4): Add {embiggenedCards} instead.
- **Engine:** `add_random_from_catalog { query: { embiggen: true } }` ([[R1178001]], the foundation's field), never itself ([[R387]]), then NEW: a price for one version (B03), a cost override on the Embiggened price alone (`mana::price_of`, `play_choices::legal_embiggen_choices`), shown on the face's second badge; the normal version keeps its price.
- **Rulings:** MP-B03-3: "its embiggened form costs (0)" is the Embiggened price alone, read before modifiers like any `costOverride`; a Radiant card stays an embiggen card.
- **Numbers:** cards 1 ↑; embiggenedCards 3 ↑.
- **Check:** "embiggened card" is an embiggen card (#1175). R275: the cards Radiant.
- **Class:** C (a price for one version).

#### Meditative+ #24 · Bob, Wisconsin's Strongest Soldier
`meditativeplus-024` · (3) Unit · Legendary · 5/5 → 10/10 · built by #1185

> **Designer:** (3) Bob, Wisconsin's Strongest Soldier, Unit, Spell, Legendary · [5/5] · Cry: Choose one · Freeze all enemy units · Add an enemy unit to your hand and give your opponent 3 coins · Discover a random unit, make it radiant · Draw 1, fill your hand with copies of it ~~~ [10/10] · Cry: Do all of the following in order · Freeze all enemy units · Add an enemy unit to your hand and give your opponent 3 coins · Discover a random unit, make it radiant · Draw 1, fill your hand with copies of it

- **Text:** Cry: Choose one: / Freeze all enemy Units. / Put an enemy Unit into your hand. Your opponent gets {coins|Coin|Coins}. / Discover a Unit. Make it Radiant. / Draw 1. Fill your hand with copies of it.
- **Radiant:** Cry: Do all of these, in order: / Freeze all enemy Units. / Put an enemy Unit into your hand. Your opponent gets {coins|Coin|Coins}. / Discover a Unit. Make it Radiant. / Draw 1. Fill your hand with copies of it.
- **Engine:** Four declared modes ([[R81]]; M #49.1's every-effect list on the Radiant face). 1: Freeze (#1174) on every enemy Unit. 2: NEW: an enemy Unit into your hand (B07), a declared enemy Unit, the top of its pile ([[R13]]), leaving the field to your hand as yours (`ownership::take_into_hand` extended to the field, [[R747]]'s ownership move; a token ceases to exist, [[R11]]), then `add_to_hand { defId: core-t-coin, count: 3, player: enemy }`. 3: `discover_from_catalog { type: Unit }` with the pick added Radiant (M #88). 4: `draw` 1 then `add_to_hand { copyOf }` until the hand is full ([[R596]], [[R1242]]).
- **Rulings:** MP-B07-4: mode 2 targets at play time and fizzles with no enemy Unit; a token taken ceases to exist and the Coins are still given; the Radiant face declares mode 2's target with the play and resolves the four in order, each its own step; with no draw (an empty deck: fatigue) mode 4 copies nothing.
- **Numbers:** coins 3 ↓.
- **Check:** The designer's "Unit, Spell" is a Unit ([[R1178002]]). "coins" are The Coin. R275: 10/10 and all four modes.
- **Class:** C (an enemy Unit into your hand; Freeze from #1174).

#### Meditative+ #25 · KY's Intuition
`meditativeplus-025` · (1) Spell, KY · Rare · built by #1183

> **Designer:** (1) KY's Intuition, Spell, Rare, KY · Discover one of 3 code blocks. Add the Radiant card with that code to your hand. {should come from an actual coded card} ~~~ Discover one of 3 code blocks. Add the Radiant card with that code to your hand. It costs (0) {should come from an actual coded card}

- **Text:** Discover one of 3 blocks of card code. Add the Radiant card with that code to your hand.
- **Radiant:** Discover one of 3 blocks of card code. Add the Radiant card with that code to your hand. It costs (0).
- **Engine:** NEW: code blocks (B05): each card's excerpt compiled in from its own script by `crates/cards/build.rs` (the lines above `#[cfg(test)]`, header comments and `use` lines dropped, at most `CODE_BLOCK_MAX_LINES`), held beside the catalog, and a Discover over 3 random non-token cards of the shipped sets ([[R380]]) whose options the view shows as code alone, never the card's id, name or face, until one is picked ([[§10.8]], CLAUDE.md rule 7); the pick is added Radiant (M #88), the Radiant face with `costOverride` 0. The WASM size cap (#442 L12) is checked.
- **Rulings:** MP-B05-3: the three are distinct cards; a card's code block is its own script's, the same on both faces; the AI chooses by the scorer over the hidden cards it would get (the engine knows them), as any Discover.
- **Numbers:** setCost 0, on the Radiant face only (`tunedOn: radiant`, [[R824000]]).
- **Check:** "should come from an actual coded card": the excerpt is the card's real script. R275: the card at (0).
- **Class:** D (code blocks).

#### Meditative+ #26 · Scratch Pad
`meditativeplus-026` · (0) Spell · Mythic · built by #1182

> **Designer:** (0) Scratch Pad, Spell, Meditative+, #26 · Code a card from scratch. Exile this. ~~~ Code a card from scratch, make it radiant. Exile this. {no tool tips or hints}

- **Text:** Code a card from scratch and add it to your hand. Exile this.
- **Radiant:** Code a Radiant card from scratch and add it to your hand. Exile this.
- **Engine:** NEW: the crafter's from-scratch mode (B04): M #17's `craft` prompt ([[R880]] to [[R883]]) with a flag the client reads to open an empty canvas with no presets, tooltips or hints; the engine still rolls its four presets so the AI, the random policy and a timeout can answer ([[R881]]). The crafted card is added to the hand (Radiant on the Radiant face); then `exile { target: { of: self } }` sends Scratch Pad to its owner's exile instead of the graveyard ([[R178]]).
- **Rulings:** MP-B04-3: the cost prompt and the budget are M #17's (0 to 4, 2 + 5 × the cost points, 24 lines); "from scratch" is the client's empty canvas, not a smaller budget.
- **Numbers:** none.
- **Check:** No rarity in the list: Mythic, as M #17 ([[R1178002]], ⚠ designer). R275: the crafted card Radiant.
- **Class:** D (the from-scratch crafter).


## MP7. Readings for the designer

Each row is the build's default until the designer says otherwise; a change is a patch of its own after
the set ships, or a correction to the rows before it.

| # | Card | The unclear words | The default |
| --- | --- | --- | --- |
| 1 | #1 Bob's Plan | "supporting cards … you may make some AI cards" | One AI card per Wincon card, carrying its own support: #1.1 Base Camp for M #8, #1.2 Spa Day for M #20 (MP5 D3); a Wincon card with no plan is added alone |
| 2 | #1.1, #1.2 | their names, costs and numbers | Base Camp (0): 2 Ascent levels and 2 Coins (Radiant 3 and Radiant Coins); Spa Day (2): heal 25 and 2 Love Bombs (Radiant 50 and Radiant Love Bombs) |
| 3 | #2 NA Tech | "lowest winrate" with an empty table | R1100's table read from the other end; until a stats run fills the table, two random non-token cards are fused |
| 4 | #4 Supply Drop | "In 3 turns", "all enemies" | At the start of your third turn from now, counted in your turns, before the draw, with a public countdown; the enemy hero and every enemy Unit |
| 5 | #5 Spirit Lash | "all permanents" | Every Unit on both sides (the permanents that take damage); no hero |
| 6 | #7 Life Mage | "healing is twice as effective" | Every heal of its controller's hero and Units, Lifesteal's included; two Radiant Life Mages quadruple it |
| 7 | #10 Medina Grifter | "a card" | A random card of the opponent's hand |
| 8 | #13 Living Health Potion | "all allies" | Your hero and your Units |
| 9 | #17 Dogpile | the name; "a spell", "them" | Dogpile, Classic #60 having the name Pile On; another Spell of yours; the enemy it targeted, Unit or hero |
| 10 | #18 Cognitive Offloading | "the rest of your turn" when drawn on the opponent's turn | Only on your own turn; on theirs it gains mana and draws, nothing more |
| 11 | #19 Grubby Gloves | "2 cards from your opponent's deck" | Its top 2 (the deck is shuffled: no random number) |
| 12 | #20 KY's Competition | "when you are finished", X = 0 | A Stop option ends it; a wrong answer or a timeout ends it; at most 30; X = 0 summons nothing |
| 13 | #21 Harmony Spirit | "all other units" | Both sides; damage and stat buffs cleared, auras still applying on top |
| 14 | #22 Mountaintop Garden | "All embiggened", a Radiant card | Both players' embiggen cards; a Radiant card offers its normal price only |
| 15 | #24 Bob | "Unit, Spell"; "an enemy unit" | A Unit; a declared enemy Unit, taken as yours |
| 16 | #26 Scratch Pad | no rarity | Mythic, as M #17 |
| 17 | #16.1, #20.1 | tokens with a rarity | `rarity: "Token"`, the designer's Epic and Rare as `printedRarity` |

## MP8. Sound, visuals, the AI and the release

- **Sound.** Each card's `card-audio.json5` entry comes with it ([[§10.11]]'s lines); no voice file is
  rendered or expected before the set ships ([[R1420]]): the release renders them through the `voice`
  workflow (`[gen:voice]`, R1448). The intro snippets ([[R1352]], `gen:music`) for the Legendary and the
  Mythics, #1 Bob's Plan, #24 Bob and #26 Scratch Pad, are the release's.
- **Visuals.** The set mark (R503) shows the plain diamond of a set the client does not know until the
  release draws Meditative+ a glyph of its own; the client's new screens (the potion and from-scratch
  crafter, the question sequence, the code-block Discover, the autopilot's turn, the health price, the
  countdown) come with the parts that build their systems.
- **The AI** (#1186): every new prompt kind and mode answered by `crates/ai` within its budget, the
  sweep (`cargo jackioh sweep`) over the 26 cards with every set previewed, at most 13 of them in
  `SHADOW_BAN` ([[R186]]); R1390's soft gate keeps the set out of the AI's random decks until then.
- **The release** (#1187): `SHIPPED_SETS` lists Meditative+; one fragment claims its 30 entries; the
  server's migration admits Potion and `seed-catalog` seeds the set; deck and trio codes take its offset
  after Meditative's; the golden traces are re-blessed once for the bigger pools; the Almanac's and the
  deck builder's chips, `FILTER_TAGS` with Potion, the newest set's ribbon, the `RELEASES` line; every
  stated total (§8's intro, CLAUDE.md's census, the READMEs) moves to 396 cards and 84 tokens; and
  REVIEW.md's audit is run as a separate session.

## MP9. Where this brief differs from the batch issues

The batch issues' tables were the defaults this brief formalises. Read against the engine, it keeps
them, with these changes (the brief wins, as each issue says):

- **#17 is Dogpile** (B02): Classic #60 holds the name Pile On ([[R381]], [[R739]]).
- **Bob's Plan's support rides on its AI cards** (B07): Base Camp adds the 2 Coins and Spa Day shuffles
  the 2 Love Bombs, rather than Bob's Plan adding them beside the helper, so each plan is one card and the
  Radiant face's "Radiant helpers" covers the Coins and Love Bombs too. Base Camp raises every Reach the
  Summit its caster owns, wherever it is, since the level rides the card through every zone ([[R429]]).
- **Lil' Kenny prints 0/0 with `xStats`** (B06), as C+ #69, so `catalog check`'s stat rules hold and the
  Radiant face's 2X/2X is data, not a script's doubling.
- **Harmony Spirit's 3/3 is not tuned** (B07): it is good for one side and bad for the other, so no
  `better` direction fits it; the frame is the rule.
- **Supply Drop's Embiggened Supplies count is tuned on the base face alone** (B03): the Radiant face
  fills the hand, so it prints no number there ([[R1431]]).
- **Mountaintop Garden and Life Mage** (B03, B02): two Gardens are one aura, the cheaper premium
  applying; two Radiant Life Mages multiply (×4), as two Prophet Velens do in Hearthstone.

## MP10. The plan: parts, issues, order and ruling blocks

Every part is one issue and one pull request under the tracker #1173, built on `main` behind the gate
(MP3). Each takes its ruling numbers from its own issue's block, R(N×1000) to R(N×1000+999)
(`docs/issues-and-patches.md`, Ruling numbers), and names its proving tests after them (CLAUDE.md rule 3).

| Part | Issue | Cards | Systems | Blocked by | Rulings |
| --- | --- | --- | --- | --- | --- |
| Freeze | #1174 | (#24) | Frozen, Freeze | — | R1174000… |
| Embiggened wording | #1175 | (#3, #4, #5, #19, #22, #23) | the vocabulary, the label | — | R1175000… |
| CN in Chinese | #1176 | (#21) | `tags_of` and the `chinese` flag | — | R1176000… |
| Token labels | #1177 | (#1.1, #1.2, #16.1, #20.1) | the token label | — | R1177000… |
| The foundation | #1178 | — | the gate's second set, Potion, `embiggen`; this brief and the spec rows | — | R1178000–R1178002 |
| B01, existing primitives | #1179 | 3, 5, 8, 10, 12, 13, 14, 15, 19 | none | #1178, #1175 | R1179000… |
| B02, spell power | #1180 | 6, 7, 9, 11, 17 | Spell Damage sum, Spell aura, heal multiplier, health price, targets kept | #1178 | R1180000… |
| B03, the embiggen benders | #1181 | 4, 22, 23 | in N turns, one version's price, Mountaintop's aura | #1178, #1175 | R1181000… |
| B04, potions and the crafter | #1182 | 16, 16.1, 26 | potion and from-scratch modes | #1178 | R1182000… |
| B05, outside data and the autopilot | #1183 | 2, 18, 25 | the trailer, two-pick fuse, autopilot, code blocks | #1178 | R1183000… |
| B06, KY's Competition | #1184 | 20, 20.1 | question sequence, keywords by X | #1178 | R1184000… |
| B07, Bob and the plan | #1185 | 1, 1.1, 1.2, 21, 24 | set stats, enemy Unit to hand, Ascent levels, the plan | #1178, #1174, #1176 | R1185000… |
| The AI | #1186 | all | the sweep and the new prompts | B01–B07 | R1186000… |
| The release | #1187 | all | the gate opens | every part | R1187000… |

**Order.** The four patches and the foundation run in parallel; the batches start once the foundation
has merged (B01 and B03 also after #1175, B07 after #1174 and #1176) and run in parallel with each other,
since no batch builds on another's system; the AI part follows them all, and the release closes the
tracker.
