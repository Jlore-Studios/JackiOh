# Classic and Classic+ — design brief for the implementing agent

2026-09-30 · written from the designer's card list of the same day

> **Status: a proposal, not the spec.** SPEC.md is still the only source of rules (CLAUDE.md). Nothing
> in this file is in force until the change that implements it ports it into SPEC.md (§2, §5–§8, §10,
> §11), BUILD.md and the package READMEs. This brief records three things for every card and every new
> mechanic: what the designer wrote (quoted verbatim, typos included), what it most likely means in
> this engine, and every place the text is ambiguous, with a recommended reading.
>
> Proposed rulings are numbered **CL1, CL2, …**. They are not `R<n>` rows. The implementing change
> takes a fresh block of §11 numbers (the last row today is R374, so R380 upward is free), assigns
> them, and keeps a CL→R table in its PR. Each one still needs its `it("R<n> …")` test and its line
> in `packages/engine/test/rulings.test.ts` (CLAUDE.md rule 3). Items marked **⚠ designer** are
> guesses the designer should confirm before or during the build; everything else follows the spec's
> existing rules or, where the spec is silent, Hearthstone.

## Contents

- [B0. What the designer asked for](#b0-what-the-designer-asked-for)
- [B1. Reading this brief](#b1-reading-this-brief)
- [B2. Sets, ids and the catalog](#b2-sets-ids-and-the-catalog)
- [B3. New keywords](#b3-new-keywords): Animated, Activate, Brittle, Degrade and Upgrade
- [B4. New global mechanics](#b4-new-global-mechanics): no self-generation, patch history, turn cap, shadow ban, Tribute space
- [B5. Engine systems the cards need](#b5-engine-systems-the-cards-need) (E1–E40)
- [B6. Classic, card by card](#b6-classic-card-by-card) (#1–#90)
- [B7. Classic+, card by card](#b7-classic-card-by-card) (#1–#78 and tokens)
- [B8. The ten AI generated cards](#b8-the-ten-ai-generated-cards)
- [B9. Decisions for the designer](#b9-decisions-for-the-designer)
- [B10. Implementation plan](#b10-implementation-plan)

---

## B0. What the designer asked for

Verbatim, as the designer listed it before the cards:

```
* New Card Sets
   * Classic
   * Classic+
* New Keywords
   * Animated
      * For Field Spells and Traps
      * Becomes a Unit if there are open Unit spaces
      * Animated on your turn
         * Neat evasion tech
   * Degrade
      * Only affects non-immutable cards
      * Does one of the following effects:
         * Increase cost by (1) (up to 4)
         * Reduces the stats of the effected card by a total of 4 (not below 1 health)
         * Remove a keyword
         * Reduce a value of X by 1
         * Reduce a number on it by some small amount (you determine what it would be based on the card)
   * Upgrade
      * Only affects non-immutable cards
      * Does one of the following effects
   * Activate
      * Once per turn on your turn, click the card to do an effect
      * Variants
         * Activate X: Up to X times per turn
         * Activate ♾️: Can do unlimited times per turn
   * Brittle X
      * Brittle count decreases by 1 at Start of Turn after existing for a full turn cycle (yours and your opponents). When it reaches 0, destroy the card.
* New Mechanics
   * Cards cannot Discover or generate random copies of themselves (unless specifically specified)
   * Card patches are tracked from here on (and retroactively) so older versions of cards can still be accessed
   * Double turn limit
   * AI is less inclined to shadowban cards during its training. Its (pseudo)random decks are stacked to more frequently include cards that are currently on track to be shadowbanned.
   * If Tributing as a cost would open up enough board space for the permanent to be played, it can be played.
```

Then 90 Classic cards and 80 numbered Classic+ entries (two of them tokens), plus 26 Classic+ tokens
numbered N.k. The designer asked, inside the cards, for three things only an implementer can supply:
cleaner text for Classic #10 Exile's Radiant face, clean wording for Classic+ Forever&, and "10 AI
generated cards with any effect you want" shared by Claude's Datacenter and AI Slop (B8 has them).

**Where this fits.** This brief is the design half of **patch v0.2.0**. The patch's tracking issue
lists these sets, keywords and mechanics beside Core card patches (new costs for Core #65 Masochism
Mask, #43 Big Felinor, #88 Twisting Nether, #49 Snom Bunny Mind Control, #17 Flood, #34 Collateral
Damage and #16 Hit Job; new text for #32 Prem Panther, #41 Sheepish, #22 Carnivorous Cube, #31 KY's
Math Equation, #60 Bear Honeypot, #21 Hinder and #95 Call to Chaos) and cosmetic work, none of which
this brief specifies. Four of those items reach into it: #95's Radiant becomes three random effects,
the shape Classic+ #73 already has (CL44); Sheepish stops costing a Unit its Cry, which E1's announce
window never relied on; "(N) Cost" becomes the noun in every card text (B1); and each Core patch is a
line of B4.2's history. **Patch v0.2.1** follows and moves Core #98 Heroic Power onto this brief's
Activate (B3.2 rule 10), with thirteen powers.

How these relate to what exists: the source notes already reserve the set name Classic
(`JackiOh_Mechanics.md`, "Sets (Classic, Core, Boss, Boss-X)") and already state the self-generation
rule ("A discover card OR a random generation card cannot discover nor randomly generate a copy of
itself [unless specifically specified it can]"), which SPEC §5.1 implements for Core. Several verbs
the new cards use are in the source notes but unused by Core, so the engine has no primitive for
them yet: Counter, Flicker (SPEC §6.3 says so in as many words), "Cast" of a card from a zone other
than hand, and unlocking a Locked zone.

---

## B1. Reading this brief

**Section numbers.** "B2.3" is this brief's own section B2.3. A "§" always means SPEC.md (so "§4.5"
is SPEC's state check, and "B4.5" is this brief's Tribute rule). "C #N" and "C+ #N" are Classic and
Classic+ card N; "#N" alone inside a card entry is a card of that entry's set, and Core cards are
named as "Core #N".

**Words.** Players read "Deck" for the rules' library and "Tribute" for Sacrifice (R373). The card
text this brief proposes is written the way R366 says card text is written: "Cost (N)" for a
specific cost, "(N)" for a price, keywords first on a line of their own, each labelled ability
("Cry:", "Death:", "Start of turn:", "End of turn:", "Aura:", "Activate:", "Cast on draw:",
"Paid (4):") on a line of its own, and sentences that start with a capital and end with a full stop.
The engine keeps `library`, `sacrifice` and its other identifiers. Patch v0.2.0 turns the cost
style around, "(N) Cost" for the noun and "costs (N)" for the verb (B0); the text pass that makes
that change for Core rewrites the proposals here with it.

**A card entry** looks like this:

- The heading: set, number and name. The first line: proposed id, cost, type and tags, rarity, and
  for a Unit its stats base → Radiant.
- **Designer** — the designer's own text, verbatim, with `~~~` between the base and the Radiant face.
  Line breaks and bullets are flattened onto one line (a bullet becomes " · "); a Radiant face that
  repeats a long base text word for word is shortened to "(the same)" or "(…)" plus what it adds.
- **Text** and **Radiant** — the catalog text this brief proposes for `base.text` and `radiant.text`.
  The Radiant face is written out in full, as R277 requires.
- **Engine** — what the card does, in SPEC §6's primitives and the systems of B5 below (E1–E40).
- **Rulings** — the proposed rulings (CL numbers) and short readings that need no ruling.
- **Numbers** — the numbers Degrade, Upgrade and KY's Constant may change (B3.4). ↑ means more is
  better for the card's controller, ↓ means less is better. The step is B3.4's default (1 for a
  number up to 5, 2 for 6–12, a quarter above that) unless the entry states one.
- **Check** — R275 (the Radiant power standard) and R276 (every Radiant face differs), typos, and
  open questions. Absent when there is nothing to say.

"Target" means the player chooses at play time among every legal unit and hero on either side unless
the text narrows it (§8's conventions). "Deal N damage" with no target named is targeted, as Core's
Twisted Sorcerer is. "Random" picks go through `rng` and follow R60. "Discard N" with no "random" is
the discarding player's choice (§6.3, R16).

---

## B2. Sets, ids and the catalog

### B2.1 Two new sets

| Set | Cards | Tokens | Notes |
| --- | --- | --- | --- |
| Core | 100 | 11 | unchanged |
| Classic | 90 (#1–#90) | 0 | `SetName` already has "Classic" |
| Classic+ | 78 (#1–#78) | 38 | `SetName` gains "Classic+" |
| **Total** | **268** | **49** | 317 catalog entries |

Classic+'s 38 tokens: 8 Pancake cards (#12.1–#12.8), 5 Losers (#19.1–#19.5), Otherworldly Removal's 3
(#32.1–#32.3), Bone Storm (#36.1), Solarius-Prime (#38.1), KY's Gift (#42.1), Felinor Flagbearer Prime
(#46.1), 5 Grapes (#65.1–#65.5), Classic Golem (#73.1), the J-lease J-Jungle EX-plorer Pack (#75.1),
Brother Ping (#76.1) and the ten AI generated cards (T-AI-1 to T-AI-10). Classic generates only cards
that exist elsewhere (the Felinor Token, Book of Flame, Ancient Acquisition).

One format, all sets (**CL1**). A deck may mix Core, Classic and Classic+ under §2.6's rules (20
cards, no duplicate ids, no Tokens), and a trio still needs 60 distinct cards, now from 268. Nothing
in the designer's list asks for per-set formats, so none is proposed; B4.2's patch history keeps a
later "legacy format" possible.

### B2.2 Ids, indices and file names

| | Core (today) | Classic | Classic+ |
| --- | --- | --- | --- |
| Card id | `core-043` | `classic-043` | `classicplus-043` |
| Token a card defines | `core-051-1` | — | `classicplus-012-1` |
| Shared token | `core-t-rush` | — | `classicplus-t-ai-01` … `-10` |
| `index` | `"43"`, `"51.1"`, `"T-rush"` | `"43"` | `"43"`, `"12.1"`, `"T-AI-1"` |
| Script file | `src/scripts/043-big-felinor.ts` | `src/scripts/classic/043-….ts` | `src/scripts/classic-plus/043-….ts` |
| Test file | `test/043-big-felinor.test.ts` | `test/classic/043-….test.ts` | `test/classic-plus/043-….test.ts` |

`classicplus` has no hyphen inside the set part so an id still splits one way. An `index` is only
unique within its set, so everything that looks a card up by index alone must key on `(set, index)`
or on the id instead:

- Lookups by index alone, in both packages: `cardDefByIndex` (cards); the engine's `defByIndex`
  (`catalog.ts`), which `draw.ts` uses to find the Rush Token and `callToChaos.ts` and `heroPower.ts`
  use to turn a rolled number back into a card; the query filters `index`, `notIndex` and
  `excludeIndex`; and the self-exclusion helpers, the engine's `excludingIndex` (`addToHand.ts`,
  `summon.ts`, `choose.ts`) and the cards package's `pool(ownIndex)`. Switch the self-exclusion to the
  def id (`excludeDefId`, `pool(ownId)`), which is also what B4.1 needs, and give every remaining
  index lookup a set (`defByIndex(set, index)`), so `"43"` can never find two cards.
- Pools that are Core today because Core was all there was: `packages/ai/src/deck.ts`
  (`buildAiDeck`'s `query({ set: "Core" })`) and `packages/ai/src/determinize.ts` (R185's resample:
  its two `set: "Core"` queries, and `AI_DETERMINIZE.excludeIndexes`, whose `["98"]` becomes an id)
  reach every set (B2.6). `packages/engine/src/subsystems/scorer.ts` (`query({ set: "Core",
  excludeIndex: ZEPHYRS_INDEX })`) stays Core, because Core #97 Zephyrs says so, and keys its
  exclusion by id.
- Core #82 KY's Trial rolls Core numbers 1–100 and adds "the Radiant Core card" with that index: filter
  `set: "Core"` explicitly. Core #97 Zephyrs ("only from the core set") the same.
- Deck codes carry "catalog numbers" (R255, `deckCode.ts`): bump `DECK_CODE_VERSION` to 2 and write
  a set tag with each number (a number offset per set is enough: Core n, Classic 1000 + n,
  Classic+ 2000 + n, still LEB128). Keep reading version 1 codes as Core numbers, since every code
  minted so far is one: R255 refuses "another version", which becomes "a version other than 1 or 2".
  Trio codes (R339, `trioCode.ts`) write each deck as R255 does, so `TRIO_CODE_VERSION` goes to 2 in
  the same change and reads version 1 the same way. Both constants live in `apps/server/src/config.ts`.
- `scripts/naming.ts`, `gen-registry.ts` and `missing-tests.ts` learn the set folders;
  `validate-catalog.ts` and `catalog.test.ts` count per set (B2.5).
- BUILD's must-pass table (M4-T4) gains a Classic and a Classic+ table keyed by set and number.

### B2.3 Classic+ numbering: the collisions

The designer's Classic+ numbers collide five times and number one token as a card. Proposed fix, which
moves as few cards as possible: the first card listed under a number keeps it, the displaced cards
fill the hole at #47 and then take #76–#78 in list order, and the two tokens become N.1 of the card
that makes them.

| Designer's number | Card | Proposed |
| --- | --- | --- |
| #11 (second) | Jogg's Box | **#47** |
| #25 (second) | Brother Lar | **#76** |
| #25.1 | Brother Ping | **#76.1** |
| #28 (second) | Anti-Softlock | **#77** |
| #42 (second) | Claude's Datacenter | **#78** |
| #47 | Felinor Flagbearer Prime (a Token) | **#46.1** |
| #75 (second) | J-lease J-Jungle EX-plorer Pack (a Token) | **#75.1** |

Every other Classic+ card keeps the designer's number. Classic has no numbering collisions (see B2.8
for its name collisions). **⚠ designer:** confirm, or give the numbers you meant.

### B2.4 Tags

New tags: **Book** (every "Book of …" card, Classic and Classic+), **Pancake** (#12, #13 and the eight
Pancake tokens), **AI** (the ten AI generated cards). Reused: Human, Felinor, KY, CN, Fruit,
"Call to Chaos", Quickdraw, Token.

**Jlockheed vs Jlockeed (⚠ designer).** Core's tag and names are "Jlockeed" (#13 Jlockeed
Shredder-10, #14 Jlockeed's Weapons, R278); the three Classic+ cards (#48, #51, #52) spell it
"Jlockheed". They are one faction: #48 and #52's pools ("a random Jlockheed card") should find Core's
two as well.
Proposed: one tag, the existing `Jlockeed`, on all five, the Classic+ names kept as the designer
spelled them, and R278 rewritten, since the tag stops being "a filter and nothing else" once cards
count it. If the designer prefers "Jlockheed" everywhere, that is a patch (B4.2) renaming #13 and #14
and the tag in one go.

The designer's tag lists are taken as written, including the ones that look deliberate: Classic #67
Felinor Feeler is tagged Human, not Felinor; Classic #32 Felinor Feelings is a Felinor Spell; Book
Worm, Mulch Muncher and Doctors Orders carry no Book or Fruit tag.

### B2.5 Rarity

The designer's rarities stand. SPEC §8 re-rated Core by mechanical complexity because the source had
given rarity by index block (#1–20 Common, #21–50 Rare, …); the new lists give rarity card by card, so
they are choices, and rarity feeds pools ("a random Legendary card", Book of Greed). Counts to encode
in `catalog.test.ts`:

| Set | Common | Rare | Epic | Legendary | Mythic |
| --- | --- | --- | --- | --- | --- |
| Classic | 35 | 26 | 18 | 10 | 1 |
| Classic+ (with B2.3's numbers) | 13 | 24 | 25 | 13 | 3 |

Tokens keep `rarity: "Token"`, so no pool ever finds one by rarity. The designer printed a rarity on
every Classic+ token but the AI generated cards (B8): Legendary on the eight Pancakes, the five Losers,
KY's Gift, Felinor Flagbearer Prime, Classic Golem and the J-lease Pack; Epic on Otherworldly
Removal's three and Solarius-Prime; Rare on Bone Storm and Brother Ping; and on each Grape (Rotten and
Normal Common, Large Rare, Golden Legendary, Mythic Mythic). Keep it on each of them as a display
field (`printedRarity`) for the card frame and the summon sting, never for a pool.

### B2.6 Random pools across sets (CL1)

A pool whose card does not name a set draws from every set. That is Hearthstone's reading of "a random
card", and it is the only reading that lets Core's generators see Classic and Classic+ at all. It
changes Core cards' games, so it is part of the same patch (B4.2) and these SPEC rows are rewritten
with it:

- Core pools that grow: #7 Jewelosco Scarab, #54 Straaza, #59 Unbiased Immigration, #67 Zoomerbin Oomen
  (Classic+ #22 Blood Moon is the only new Cost (1) Trap), #57 Conjure KY (§8's Engine cell fixes its pool
  as "#31, #51, #82"; it gains Classic+ #41, #42 and #62), #83 Transmogulate (R35 lists Core's six
  Legendaries by number; it becomes "every non-token Legendary except #83"), #95 Call to Chaos (its
  random units, cards, Field Spells and Traps, and "cast a random Call to Chaos", which now finds the
  Classic+ Edition too, R28), #98 Heroic Power's Discover and Stitching, #99 Craft a Card.
- Core pools that stay Core because the card says so: #82 KY's Trial, #97 Zephyrs.
- The AI: R184 ("distinct, token-free Core cards"), R185 (determinization "resampled from non-token
  Core cards"), `buildAiDeck`'s `query({ set: "Core" })` and `determinize.ts`'s two Core queries all
  become "every set" (B2.2). The tutorial's fixed decks (R291) do not change.
- Pools a new card names by set: Classic+ #75.1's "Classic or Classic+ cards", #73's "Classic cards",
  #73.1's "a random Classic or Classic+ card", #27 Zephrys Zealotism's "only using cards from Classic &
  Classic+".

Fruit is special by the designer's own note: the five Grapes are Tokens "that can be generated by any
Fruit card", so **a Fruit pool is the non-token Fruit cards plus the five Grapes**, and the Grapes
appear in no other pool but the one that takes every token, Classic+ #23 Dropshipping's (CL3).
Dropshipping's "(including tokens)" is the designer's own exception to every "only through" in this
brief: it can hand out any token of any set, a Grape, a Loser (CL32) or an AI generated card (B8)
included.

### B2.7 Faces that change more than text

- **A face with its own type.** Classic+ #22 Blood Moon is a Trap whose Radiant face "Becomes a Field
  Trap". `CardFace` gains an optional `type`; the card's type is the running face's (§5.2). Making a
  Blood Moon in hand Radiant turns it into a Field Trap there, which pools and filters then read.
- **X in the stats.** Classic+ #69 Buff Billy prints [3X/3X] and [7X/7X]: a Unit summoned with
  `statsOverride` from its played X, as the Ghoul Token's X/X is (§7), X at least 1 (R348).
- **A Radiant face with different keywords and no other change** is already legal (Core #25).

### B2.8 Names that collide (CL2)

| Problem | Cards | Proposal |
| --- | --- | --- |
| Two cards named Book of Flame | Classic #16 (Common) and #55 (Epic), identical text | Keep #16 as *the* Book of Flame, the one Book of Vital Kill and Devil's Pact name. **⚠ designer:** #55 looks like a copy-paste slip; hold it out of the first wave until it has its own name and text, or cut it (Classic would then be 89 cards) |
| Two cards named Counterspell | Classic #17 (Common) and #72 (Rare), different text | **⚠ designer:** rename #72 (suggestion: "Grand Counterspell", since it answers Spells and Traps). No card names either, so nothing else changes |
| A card named like a rules word | Classic #10 **Exile**, #36 **Burn**, #57 **Echo** (also the keyword Echo X), #30 **Recycle** (also inside "Malzahar's Recycler") | Keep the names. R279's proof (`references.test.ts`) matches a card name as whole words, case-sensitively, with an optional plural "s". Today that catches two of the four: "Exile" in twelve Core texts (#34, #39, #42, #44, #65, #72, #76, #78, #87, #94, #97, #100) and "Echo" in Core #51 and #79's "Echo 1"; "burned" and "Recycler" don't match, though a new text that says "Burn" or "Recycles" would. Give the proof a named list of rules words it never treats as a card reference unless `refs` lists the card, and let a card's `refs` stay curated |

---
## B3. New keywords

Each keyword below gets a row in SPEC §6 (Rule and Engine columns), a glossary entry (the glossary
copies §6's Rule column, R373), and its events get BUILD M5-T4 animation rows and `SOUND_CUES` rows
(B10).

### B3.1 Animated (CL4)

> **Designer:** For Field Spells and Traps · Becomes a Unit if there are open Unit spaces · Animated on
> your turn · Neat evasion tech

Cards: Classic #5 Tesla (Field Trap, 1/4), Classic #38 Jackiestan Auctioneer (Field Trap, 4/4),
Classic+ #12.8 Frostspatula (Field Spell token, 10/3, "Animated on your turn").

The reading. An Animated backrow card is a Unit in waiting: it prints attack and health, which do
nothing in the backrow, and when it animates it steps into the unit row and fights. Tesla is Clash
Royale's hidden tower (it pops up when an enemy arrives); Frostspatula is a weapon that is a Unit on
your turn and hides in the backrow on the opponent's, where nothing that attacks can reach it — the
"evasion tech".

1. **Printing.** Animated is a keyword a Field Spell, Trap or Field Trap may print, together with
   attack and health (its unit face). "Animated on your turn" is the variant in rule 4.
2. **To animate** is to move the card from its backrow zone to a unit zone of its controller: the
   unit zone in the same lane when it is open, else the leftmost open, unlocked, unreserved one
   (R64's placement). It enters in Attack Position unless its text says otherwise (Tesla: Defense).
   With no open unit zone it does not animate and stays where it is; a Trap that has fired then
   stays face-up in its zone, as a Field Trap does.
3. **While animated** it is a Unit for every rule: it attacks and is attacked, takes damage, counts
   among "your Units", is hit by "all Units" effects, and when it dies it goes to its owner's
   graveyard and fires Death. It is face-up (public), and it keeps all of its text: a Field Trap's
   trigger still fires (Tesla keeps zapping arrivals), an aura still applies.
4. **When it animates.**
   - An Animated Trap or Field Trap animates as the last step of its firing. "Then summon this in
     Defense Position" (Tesla) and "Activates/summons" (Auctioneer) are that step. A card that is
     already a Unit when it fires again does not move or change position.
   - An Animated Field Spell animates as it enters the field.
   - **Animated on your turn**: it animates at the start of its controller's turn (a step after the
     mana refresh and the Brittle tick, B3.3, and before start-of-turn delayed effects and triggers,
     §2.2 and R62) and when it enters the field during its
     controller's turn; it returns to its backrow zone at its controller's cleanup, after every
     end-of-turn step (§2.2), so its own end-of-turn text runs while it is a Unit.
5. **Moving is not leaving the field.** Between its backrow zone and a unit zone it keeps damage,
   buffs, counters, memory and granted keywords (R78 does not apply). Entering the unit zone is
   entering it on that turn (R83, R171): it is summoning sick, so Frostspatula's Rush matters and a
   Tesla in Defense Position never needed to attack.
6. **Its home zone.** While an "on your turn" card is animated, its backrow zone is reserved for its
   return, as a dying Reborn unit's zone is (R64): nothing else may enter it. It does not return,
   and stays a Unit until its next cleanup, when that zone has been Locked since (a Lock stops a
   return, as R175 says of Reborn), when it has changed controller (a stolen one has no home on the
   new side: it returns to its new controller's leftmost open backrow zone, or stays a Unit), or
   when it lies dormant under a Stack.
7. **In the backrow it is not a Unit**: it cannot attack or be attacked, "all Units" effects skip it,
   and backrow effects (Back Breaker, Guy Att, Magic Jammed, Crushing Walls) reach it.
8. A face-down Animated Trap is hidden like any trap (R33) until it fires.

Engine: a new zone move `animate`/`deanimate` in `zones.ts`, a `homeZone` reservation beside
`state.reserved`, and two events, `animated` and `deanimated` (§10.3's list and BUILD M5-T4).

### B3.2 Activate (CL5)

> **Designer:** Once per turn on your turn, click the card to do an effect · Activate X: Up to X
> times per turn · Activate ♾️: Can do unlimited times per turn

Cards: Classic #7 InfiniScepter, #15 Nose Hunter (read as Activate, see its entry), #20 The Power to
Punish, #21 Turtinator (♾️), #23 Devil's Pact, #42 Transmutable Toxins, #78 Mutate Spell (♾️),
#81 The Power to Thrive, #84 Lockdown; Classic+ #76.1 Brother Ping (Activate, Radiant Activate 2).

The engine already has one activated ability, Heroic Power's once-a-turn power (Core #98, R43:
`activatePower`, `memory.usedTurn`, and an `activate` hook in `Script` that only #98's script
declares). Activate generalises it.

1. "Activate: …" may be used once per turn, "Activate X: …" up to X times, "Activate ♾️: …" any
   number of times.
2. **Who and when.** The card's controller, in their own main phase, with no prompt open and the
   game not over, while the card is on the field and acting: the top of its pile, or a face-up
   backrow card. A face-down card has no text anyone can use (none of these cards is a Trap).
   Summoning sickness and exertion do not apply: activating is not attacking.
3. **Counting.** Uses are counted per card per turn on the instance (`memory.activations =
   { turn, count }`). Leaving the field resets them (R78), so a card bounced and played again starts
   fresh, as does a copy.
4. **Costs.** A cost written into the ability is paid as it is activated, and an ability whose cost
   cannot be paid cannot be activated: "Tribute a Unit." (Turtinator), "Discard a random card"
   (Nose Hunter), and Heroic Power's "spend (X)". Everything after the cost is the effect.
5. **Choices.** Targets and modes the ability declares travel in the action, as a play's do (R81):
   `activate { instanceId, ability?, targets?, modes? }`; `ability` names which one when a card has
   several (none here does). Choices made during resolution are ordinary prompts.
6. **Not a play.** Activating counts for nothing that counts plays (Combo, Quickstriker, "whenever you
   play a card", Ceaseless Void). What the effect plays or casts counts as usual (R70).
7. **♾️ is bounded** by `ACTIVATE_UNLIMITED_CAP` (100 per card per turn, `config.ts`) so a fuzz game
   still ends. Every ♾️ card here is bounded anyway by a resource (units to tribute, Plague Counters).
8. **Legal actions.** `legalActions` lists `activate` exactly as it lists `activatePower` today:
   `heroPower.whyCannotActivate` is already both the refusal and the list for Heroic Power (R43,
   §10.2's pattern), and it generalises to any card's ability.
9. **X.** Degrade and Upgrade's X option moves Activate X by 1, never below 1; Upgrade turns
   "Activate" into "Activate 2". Neither touches ♾️.
10. **Heroic Power.** Patch v0.2.1 moves Core #98 onto Activate: Heroic Power costs (0) to play,
    playing it no longer uses the power, each use is an Activate that still pays its power's (X) in
    mana, a power can be dragged to its target, and thirteen powers replace the eight. Build Activate
    so that v0.2.1 is a card patch, not an engine change: an ability with a mana price
    (`Activate: Spend (X): …`), a target declared with the activation, and `activatePower` kept as an
    alias of `activate` so old logs replay.

Event: `activated { instanceId, ability }`.

### B3.3 Brittle X (CL6)

> **Designer:** Brittle count decreases by 1 at Start of Turn after existing for a full turn cycle
> (yours and your opponents). When it reaches 0, destroy the card.

Cards: Classic+ #23 Dropshipping (gives Brittle 2 to cards in hand), #74 Twice Forward One Step
Backwards (prints Brittle 4, and gains Brittle), and the AI card Hallucination (B8).

1. **Where it lives.** Brittle N is a count on a card instance, kept in every zone the card passes
   through, hand to field included (like `radiant` and `costMod`, R78), but it only ticks on the
   field (R638, since v0.2.X: a card in a hand or a deck holds its count). A count *given* by an
   effect is held until the card enters the field. A *printed* Brittle starts when the card enters
   the field (Twice Forward would otherwise crumble in the deck before it is drawn).
2. **The tick.** At the start of its controller's turn, as a step right after the mana refresh, the
   count of each card they control on the field drops by 1 — but only once the card has had a full turn
   cycle with it: a count started on turn t first ticks at the first start of its controller's turn
   numbered t + 2 or later, by which time the card has lived through a whole turn of the other
   player's and the rest of the turn it arrived on. After that it ticks at each start of its
   controller's turn. (So a card given Brittle 2 on your turn 5 ticks to 1 at the start of your turn
   7 and crumbles at the start of your turn 9: you can use it for the rest of turn 5 and during turn
   7.) **⚠ designer:** the stricter reading, a whole turn of each player's, puts the first tick at
   t + 4 and gives every Brittle card one more turn (B9).
3. **At 0** the card is destroyed: an ordinary destroy, so Indestructible ignores it (R46) and the
   count stays at 0, checked again at each tick. That is not a discard, so "whenever you discard"
   does not see it. Event: `crumbled { instanceId, zone }`, always `zone: "field"` since R638 (it
   used to crumble a card in a hand or a deck to its graveyard).
4. **"Give Brittle N"** sets the count to N; **"gain +N Brittle"** adds N.
5. Degrade lowers the count by 1 and Upgrade raises it by 1 (its X, B3.4). A Vanilla removes printed
   Brittle and, as §10.4 does for every granted keyword, keeps a given one.
6. **View.** The count is public on the field and visible to the owner in hand; `counterChanged`
   gains the counter kind `"brittle"`.

### B3.4 Degrade and Upgrade (CL7)

> **Designer (Degrade):** Only affects non-immutable cards · Does one of the following effects:
> Increase cost by (1) (up to 4) · Reduces the stats of the effected card by a total of 4 (not below 1
> health) · Remove a keyword · Reduce a value of X by 1 · Reduce a number on it by some small amount
> (you determine what it would be based on the card)
>
> **Designer (Upgrade):** Only affects non-immutable cards · Does one of the following effects

Upgrade's list was left blank; it is read as Degrade's mirror. Cards: Classic+ #8 Withering Storm,
#69 Buff Billy, #70 Chaos Machine, #71 Book of Buff, #72 Book of Nerf, #73 Call to Chaos (Classic+
Edition), and the AI card Fine-Tuning (B8).

1. **One change per application.** A Degrade applies one change to a card, drawn uniformly (match
   rng) from the changes below that can apply to that card now; an Upgrade the same from the mirror
   list. "Degrade a card 5 times" is five separate draws. A card none of them can change is left
   alone without a draw.
2. **Immutable cards are never changed** (no draw). Otherwise a Degrade or Upgrade reaches a card in
   any zone its text names: the field, a hand, a deck.
3. **The menu.**

   | Change | Degrade | Upgrade | Can apply to |
   | --- | --- | --- | --- |
   | Cost | +1, never above (4) | −1, never below (0) | a card whose own cost (printed or `costOverride`, plus `costMod`, R65) is below 4 (Degrade) or at least 1 (Upgrade); never an X-cost card (R65) |
   | Stats | −4 split: roll k in 0–4, −k attack and −(4−k) health; attack floors at 0 and current health never falls below 1, and what the floors refuse is lost | +4 split the same way | a Unit, or an Animated card |
   | Keyword | remove one keyword it has | add one keyword it lacks from R21's random-keyword pool | remove: any card with a removable keyword (not Immutable, which blocks the whole thing, and never a harmful one: Can't attack, Brittle, Cannot be in Defense Position); add: a Unit |
   | X | one X on the card gets 1 worse | 1 better | an X-cost card (its X counts 1 less or more when it resolves, never below 1) and a numbered keyword: Armor, Lucky, Echo, Activate, Brittle, Spell Damage (more is better), Tribute (less is better, never below 1) |
   | Number | one declared number gets one step worse | one step better | the numbers the card declares (rule 5) |

4. **It stays.** These changes are part of the card: stored on the instance as `tuning` (a stat delta
   beside `buffs`, keywords added and removed, X deltas, number deltas; the cost change is
   `costMod`), kept in every zone and through leaving the field — R78's reset list does not include
   `tuning`, exactly as it leaves out `costMod` and `radiant`. A copy keeps it, which changes R57: a
   copy on the field keeps `tuning` with the buffs and keywords it already keeps, and a copy shuffled
   into a library carries `tuning` beside the radiant flag and `statsOverride`. (A copy still resets
   counters, R57, so it never inherits a Brittle count.) A Transform makes a new card without it; a
   Fuse sums it (R102).
5. **Declared numbers.** Each card declares its tunable numbers in the catalog, per face:
   `params: [{ key, base, radiant, better: "up" | "down", step, min, max }]`, and writes them into
   its text as `{key}`; the view carries the instance's current values and the client fills them in,
   as it prints `preview` (R280). R277's diff reads each face's text with its own values filled in. A
   script reads `param(ctx, key)` instead of a literal. Defaults: step 1 for a number up to 5, 2 for
   6–12, a quarter (rounded) above that; an amount never drops below 1 (a Degrade weakens an effect
   and never deletes it); a threshold ("the 3rd card", "(4)+ Mana") moves toward harder for a
   Degrade. Every Classic and Classic+ card's numbers are listed in its entry below. Core cards can
   declare theirs in the same patch or later; until then their Degrade simply has fewer options.
6. **On the field** the stats change moves max health, and "not below 1 health" is current health. In
   a hand or deck it changes the face the card will enter with.
7. **View and hidden information.** A changed card shows its current cost, stats, keywords and numbers
   wherever its viewer may read it. The owner of a deck does not see a change made inside it until
   the card leaves the deck (R311). Events `degraded` and `upgraded { instanceId, change }` are hidden
   like any other hand or deck event: the opponent learns only that some card of that pile changed.

**KY's Constant** (Classic+ #41) uses the same notion of "a number on a card": its cost (not X), its
attack and health, a numbered keyword's value, or a declared number.

---
## B4. New global mechanics

### B4.1 A card never generates itself (CL8)

> **Designer:** Cards cannot Discover or generate random copies of themselves (unless specifically
> specified)

This is already the rule for Core: §5.1 ("never include the generating card's own definition, unless
the card names the pool itself") and `pool(ownIndex)` in `packages/cards/src/query.ts`. The new sets
make it matter far more (Book of Books makes Books, Fruit Basket makes Fruit, Jogg's Box casts Spells),
so it is restated for every path, and re-keyed:

1. **By definition id, not index**: `excludeDefId` and `pool(ownId)` (B2.2), since indices repeat
   across sets.
2. **Every way a card makes a card from a pool**: added to a hand, shuffled in, summoned, Discovered,
   cast (Jogg's Box, Solarius-Prime), transformed into (Classic Golem), fused in (Fusion Lab, AI Slop,
   Call to Chaos (Classic+ Edition)'s deck fusion), or replacing another card. A fused card excludes
   every one of its ingredients' definitions.
3. **"Unless specifically specified"** is a text that names a pool containing itself. Only the Call to
   Chaos family does ("cast a random Call to Chaos", R28; Classic+ #73 also "replace your deck with
   random Call to Chaos"). Everything else excludes itself: Book of Books never makes Book of Books,
   Fruit Basket never makes Fruit Basket, Dropshipping's "(including tokens)" never makes
   Dropshipping, Jogg's Box never casts Jogg's Box, Zephrys Zealotism never offers itself.
4. **Copies are not generation.** "Summon a copy of this unit" (Core #12), "Fill your board with copies
   of this" (Classic+ #46.1), Echo repeats and Forever& are unaffected.
5. A test sweeps every generating script with a seeded rng and asserts its own id never comes out.

### B4.2 Card patch history (CL9)

> **Designer:** Card patches are tracked from here on (and retroactively) so older versions of cards
> can still be accessed

Today `CATALOG_VERSION` is `"core-1"` (`packages/cards/src/catalog-data.ts`, the server's env) and
patch v0.1.1 changed 105 entries without bumping it, so nothing records what a card used to be except
git. Proposal:

1. **Patches are data.** `packages/cards/patches/patches.json` lists every patch in order:
   `{ version, date, title, source, notes }`. `packages/cards/patches/<version>.json` is the whole
   catalog as that patch left it (snapshots rather than diffs: simple, and small next to the client
   bundle). A generated index maps each card id to the versions in which it changed.
2. **The version is the patch.** `CATALOG_VERSION` becomes the latest patch's version and every patch
   bumps it everywhere the string lives: `catalog-data.ts`; the server's env (`apps/server/src/env.ts`
   reads it, `.env.example` and `render.yaml` set it, and `VITE_CATALOG_VERSION` is the client's
   copy); and the database, where `db:seed-catalog` stamps every `cards` row and `app.settings`'
   `catalog_version` (migration 0001 seeded `"core-1"`), so each patch reseeds. R105 is rewritten with
   it: it names `core-1` as the Core set's version and forbids parsing or ordering a version, so the
   order of patches comes from `patches.json`, never from comparing version strings. A test holds
   `catalog.json` equal to the latest snapshot and `CATALOG_VERSION` equal to its version.
3. **Making a patch** is one script, `pnpm --filter @jackioh/cards run patch <version> "<title>"` (in
   `scripts/`, where fs is allowed): it snapshots `catalog.json`, diffs it against the previous
   snapshot and writes the patch-notes entry, card by card.
4. **Retroactively**, from `git log --follow packages/cards/catalog.json` on a full clone (a shallow
   one stops at later commits that change no entry), checked on 2026-09-30:

   | Proposed version | Commit(s) | Date | Card data that changed |
   | --- | --- | --- | --- |
   | v0.1.0 | `4626690` | 2026-09-18 | The initial commit: Core as first built, 100 cards and 9 tokens |
   | v0.1.0-r1 | `1539fa7`, `cd780db` | 2026-09-22 | Core Set balance changes (issue #1): #3, #68's name, #81 (twice), the Rush, Sheep, Felinor and Bread Tokens' Radiant faces |
   | v0.1.0-r2 | `f5b94bc`, `a17a9e8` | 2026-09-24 | #95's text; The Coin added |
   | v0.1.0-r3 | `c219bb4` | 2026-09-25 | The Radiant pass (PR #18, R275–R279): Radiant faces, the Jlockeed tag and `refs` of 99 entries |
   | v0.1.1 | `1005c50` | 2026-09-27 | Patch v0.1.1 (issue #27, PR #28): the Ghoul Token added and 105 entries changed |
   | v0.2.0 | this work | | Classic, Classic+, the new keywords, Core's pool changes (B2.6) and the Core card patches in the v0.2.0 issue (B0) |
   | v0.2.1 | | | Heroic Power as an Activate, with thirteen powers (B3.2 rule 10) |

   The designer's balance notes behind v0.1.0-r1 landed first, as `ba45d13` (2026-09-20, merged
   in PR #2), in the source notes rather than the catalog. The "-rN" labels are proposals:
   **⚠ designer** may prefer other names for the pre-v0.1.1 changes.

   > **Renamed (#85).** This table is the brief's record and keeps the names it proposed. Revisions
   > now take a letter (`docs/issues-and-patches.md`), so the patch history ships v0.1.0-r1, -r2 and
   > -r3 as v0.1.0b, v0.1.0c and v0.1.0d (R375, R388).
5. **Reading old versions.**
   - The collection's card detail view gets a History section: each patch in which the card changed,
     its faces as they were (drawn from the snapshot), and the words and numbers that changed marked
     the way R277 marks a Radiant face (a word diff between versions). A public Patch notes page
     lists every patch and the cards it touched.
   - The server serves a snapshot (`GET /api/catalog/:version`).
   - The engine can register an older snapshot (`registerCatalog(defs, version)`). A match already
     stores the catalog version it started under, so a replay can fold under that version's data.
   - Data is not code. A patch that changes what a script *does* (not only a number, which B3.4's
     `params` make data) leaves old logs of that card unreplayable exactly. Proposal: such a patch
     keeps the old script as `src/scripts/legacy/<file>@<version>.ts`, registered under
     `<id>@<version>`, so a match pinned to an old version runs the old code.
6. **Later, not asked:** a Legacy format that pins a version, as Hearthstone's Classic format plays the
   2014 cards.

A card's lines of code (Classic #48, Classic+ #44 and #45, E36) are card data and so part of a patch.

### B4.3 Double the turn cap (CL10)

> **Designer:** Double turn limit

The limit read here is the turn cap: "There is a 30 turn cap, where an auto-draw happens"
(`JackiOh_Mechanics.md`), §2.5, and R2, which is still marked decide. `TURN_CAP_PLAYER_TURNS` goes
from 30 to 60: 30 turns each. **⚠ designer:** the other limit in the game is the 75-second turn clock
(R79, `TURN_CLOCK_SECONDS` in `apps/server/src/config.ts`); if that is what was meant, the clock goes
to 150 seconds instead and the cap stays 30.

Knock-on work:

- §2.5's reason for 30 ("lines up with a 20-card deck … keeps fatigue rare") no longer holds and is
  rewritten. One draw a turn empties a 20-card deck on the first player's 17th turn and the second
  player's 16th (3 or 4 opening cards, §2.4), and fatigue, N damage on the Nth empty draw, then kills a
  hero at 30 health on the eighth. So two decks that do nothing end by fatigue on the second player's
  24th turn, player-turn 48, and 60 is reached only by games that heal, gain Armor, refill a deck
  (Classic #30) or replace fatigue (Core #75 Infinite Reserves). The cap becomes a backstop, fatigue
  the usual end of a long game, and CN-Virus, Pile On, Last Hurrah, Rapid Draw and Second Wind bring
  it sooner. R2 loses "decide". **⚠ designer:** if "double" meant longer games that still end on the
  cap, the cap alone won't do it; the deck size or fatigue would have to change too.
- The hard match ceiling (`MATCH_CEILING_MINUTES`, 60, R79): 60 player-turns at a full 75-second
  clock is 75 minutes, and even a game that fatigues out at player-turn 48 can take 60, so a slow game
  would hit the ceiling first. Raise it to 120 minutes (proposal) or accept that such games end as
  ceiling draws.
- The AI weighs face damage "more as the turn cap nears" by reading the constant, so it adapts; the
  sweep's `maxActions` (600) and the gates' game lengths may need raising, and gate counts will move
  (fewer turn-cap draws).
- Fuzz still "always terminates within the cap"; long seeds take longer. The e2e spec
  `08-turn-cap-draw.cy.ts` can't reach the cap as written: its two do-nothing decks fatigue out at
  player-turn 48, and it asserts both heroes are still at `HERO_HEALTH` when the Draw appears. It
  needs decks that never fatigue (both seats play Core #75 Infinite Reserves) or a game seeded near
  the cap, and its R2 arithmetic (15 turns each) becomes 30. Any client text that prints the cap
  reads the constant.

### B4.4 The shadow-ban sweep leans toward cards at risk (CL11)

> **Designer:** AI is less inclined to shadowban cards during its training. Its (pseudo)random decks
> are stacked to more frequently include cards that are currently on track to be shadowbanned.

Today (R186, `packages/ai/src/sweep.ts`): each non-token card is forced into AI decks, 8 seeds at Easy
and 8 at Hard, against the greedy baseline; a card is flagged `error`, `timeout`, `neverPlayed`
(affordable on 3 or more turns, never played) or `selfHarm`, and any flag bans it at every tier. The
rest of each AI deck, 19 cards at Easy and 29 at Hard, is an ordinary `buildAiDeck` draw that leaves
out every banned card but the swept one, "so its errors are not charged to this one" (`sweep.ts`).
"Training" is this sweep: the AI has no learned weights.

1. **At risk.** A card is at risk when a pass's numbers meet a flag's condition at half strength
   (affordable on `minAffordableTurns` turns and played at most once; an average evaluation change
   below half of `selfHarmDelta`), or it is on the current ban or the watch list (rule 5).
2. **Two passes.** Pass 1 is today's sweep over every card (268 with the new sets). Pass 2 sweeps only
   the at-risk cards, with more games each (`seedsPerCardAtRisk`, 24), and in *every* pass-2 game the
   AI's filler draw multiplies at-risk cards' weights by `atRiskBoost` (4) — the same mechanism as
   `buildAiDeck`'s `themeBoost` — so an at-risk card is dealt far more often: as the forced card of
   its own games and as filler in everyone else's. A card's numbers add up over every game it was
   dealt in, forced or not.
3. **A ban needs pass-2 evidence.** `neverPlayed` needs 6 affordable turns (was 3) and no play in any
   pass-2 game at that tier; `selfHarm` needs 8 plays (was 4). `error` and `timeout` ban as today:
   they are bugs, not judgement.
4. **Still reproducible.** Pass 1's at-risk list is a pure function of pass 1's results and pass 2's
   seeds are named (`sweep2:<tier>:<id>:<n>`), so slices still run in parallel and a sweep of record
   replays.
5. **Memory between sweeps.** `shadowBan.ts` gains `SHADOW_WATCH`, the cards that were at risk and
   cleared, with their numbers, which the next sweep counts as at risk from the start: "currently on
   track" carries over.
6. **Filler and blame.** Pass 2's filler draw lifts the ban for at-risk cards banned for
   `neverPlayed` or `selfHarm`, the judgements pass 2 exists to revisit, and keeps out cards banned
   for `error` or `timeout`, so a known bug is never filler. `neverPlayed` and `selfHarm` are counted
   per card, so a card's filler games are its own evidence. An `error` or `timeout` still bans only
   the game's forced card; one in a game that also dealt at-risk filler is listed against that filler
   too (a `suspect` line in the sweep's output), and bans it only if its own forced games repeat it.

Rejected: rewarding the AI's search for playing at-risk cards during the sweep. That would measure a
different AI from the one that plays. The ban's scope is unchanged (R186): AI deck building only.

### B4.5 A Tribute can pay for its own zone (CL12)

> **Designer:** If Tributing as a cost would open up enough board space for the permanent to be
> played, it can be played.

Today `legalZonesFor` and `refuseZone` (`packages/engine/src/playChoices.ts`) judge the zone before the
Tribute is paid, so with five units down no Tribute card can be played at all, although its own
Tribute would empty a zone (§3.2: "requires an empty, unlocked zone").

1. A play may name a zone its own Tribute empties: a unit zone whose pile is exactly one tributed unit
   (a Stack pile's next card would resume, R13, so tributing the top of a pile frees nothing), whose
   tributed unit has no Reborn (its zone would be reserved for the return, R64), and which is not
   Locked.
2. `playActionsFor` stops crossing zones with tribute sets independently: it pairs each zone with the
   sets that leave it open, and `whyChoicesRefused` checks the pair.
3. §10.5 already pays at step 2 and places at step 4, so only the validator changes. R360's Lava Golem
   already reads the opponent's side "once the Tribute is paid"; the same holds on the player's own
   side now.
4. It applies to a backrow card with a Tribute cost the same way (none in these sets), and never to an
   activation cost, which needs no zone.

Cards: Classic #45 Nature Titan, #61 Plague Bringer Goliath, #80 BOOM! Big Max; Core #55 Lava Golem,
#66 The Rock.

---

## B5. Engine systems the cards need

The cards below cite these by number. Build them first (engine wave, B10), each with an engine test
through a fixture script, as CLAUDE.md asks of engine behaviour, before any card file uses it.

| # | System | What it is | Used by |
| --- | --- | --- | --- |
| E1 | **Counter** | Cancel a card being played or cast: it never resolves or enters the field and goes to its owner's graveyard, or exile when the text says so. It is treated as never played: no Cry, no Death, no `cardPlayed`, no `cardResolved`, not counted by `turnLog.cardsPlayed`, the game's `played` counter, Combo, Quickstriker or Ceaseless Void. Mana and Tributes paid stay spent (Hearthstone). **Timing:** §10.5 step 4 places the card before it emits `cardPlayed` (R119), so a counter can't answer that event. §10.5 gains a step between 3 and 4 instead: once the price is paid and before the card moves, the engine emits `cardAnnounced { instanceId, player, type, costPaid, targets }` and runs a window in which counters answer it, as traps answer any event (§10.3: traps first, then other triggers). The first counter to resolve cancels the play; the rest find no card and stay set. A cancelled play stops there, so step 4 never places the card or emits `cardPlayed`, Sheepish (which answers `cardPlayed`, R17) never sees it, and a countered card's Echo repeats never happen. The window can open a prompt (Palantir's base face asks), so steps 4–8 park on `state.work` (`work.ts`) while it is open. The announce shows what `cardPlayed` would: a card set face-down shows the opponent only its zone. Event `countered`. | C #4, #10, #17, #72, #87; AI Refusal |
| E2 | **Steal off the field** | A Spell as it is cast (countered, then moved), or a card in a hand or deck, moves to the thief's hand and **its owner becomes the thief**: the second exception to "a card always goes to its owner's piles" (§3.2), beside R73's library swap. The hand cap applies (a burned card goes to its new owner's graveyard). Event `stolen { instanceId, from, to, zone }`, hidden per zone. | C #4, #72 r, #9, #58; C+ #12.3 |
| E3 | **Draw limit** | "Limited to 1 Draw per turn": a player's draws beyond the limit in one turn (any player's turn; the start-of-turn draw counts) do not happen at all — no card moves, no fatigue, no cast on draw. With several limits the lowest holds. | C #4, #49 |
| E4 | **More counters** | Per player, per turn, on both players' turns: draws (C #9), plays by type (C+ #37). Per player, per game: plays by tag (`playedByTag`, C+ #64, AI Scaling Law). Game-wide: the last Spell played by anyone (C #57), the last face-up card each player played (AI Autocomplete). The per-turn counts reset and survive exactly as `turnLog` does; the per-game counts never reset, like the game `played` counter; each "last" record is overwritten by the next play and never cleared. | as listed |
| E5 | **Replacement windows** | Effects that change an event before it happens. The engine has one, My Pawn's attack cancel (R44). New points, each at a fixed place: *would take lethal damage* (§4.4 before step 5; lethal projected as My Pawn's is), *would be healed* (inside `heal`), *would die* (§4.5 step 1, before cards move), *would go to a graveyard* (every zone move into a GY), *a friendly unit is targeted* (§4.2 step 2 for attacks, §10.5 step 1 and every target prompt for effects). | C #14 r, #28, #33, #50, #52, #60, #89; C+ #22 |
| E6 | **Damage pipeline additions** | Spell Damage +X (before step 1: a Spell's hit is raised by the Spell Damage on its controller's side; Spells only, not Field Spells or Traps); hero damage multipliers after Armor (half or a quarter, rounded up); per-hit caps (the lowest cap wins: Anti-oneshot Armor 5 or 3, Anime Armor 1); Trample on a Spell (the excess over the target Unit's health hits that Unit's controller's hero as a new instance, as R346 put Pierce on a Spell). | C #75, #83; C+ #11, #38, #38.1 |
| E7 | **Set health** | "Set a Hero's health to 13": no pipeline, not damage, not a heal (like R18's lose health). Event `healthSet`. | C #29 |
| E8 | **Heal becomes damage** | A heal of X on a matching target becomes X Pierce damage from the converting card. | C+ #22 |
| E9 | **Redirect** | A damage instance moves to another hero (C #52); a chosen target or an attack moves to another unit (C #33). Event `redirected`. | C #33, #52 |
| E10 | **End the turn from an effect** | The rest of the effect list resolves, then the turn ends as if the player had pressed End turn (every end-of-turn step runs). "One more action, then your turn ends": a counter of actions left, the turn ending after the next action resolves. | C+ #26; AI Rate Limit |
| E11 | **Play from the graveyard** | While a permission is active, `legalActions` offers `play` for graveyard cards and §10.5 takes the card from the graveyard; cost, choices and counting as from hand; its Cry fires (R1's "played from hand" becomes "played"). R65's player discounts apply (they are prices for a play). | C #28, #74, #90 (reward L) |
| E12 | **Casts from anywhere; random casts** | Cast a card from a graveyard, a copy, a named card or a random catalog card (R70: free, counts as played). A *random* cast makes every choice at random — targets, modes, Discover picks — as §6.2's "Targets chosen randomly" does; its X is the caster's current mana, at least 1. "Target enemies when possible" narrows each random target pick to enemies when one is legal. Casting a Field Spell or Trap places it and needs a zone (no zone: it fizzles and goes to the graveyard). | C #7, #47, #56; C+ #37, #38.1, #47 |
| E13 | **Trigger a Cry** | Run a unit's `cry` hook with that unit as `self`, on the field or in a graveyard, under the triggering card's controller, who makes its choices (R70). A Cry that acts on "this" finds nothing when the unit is in a graveyard. | C #54 |
| E14 | **Copy the last Spell's text** | See Classic #57 Echo. | C #57 |
| E15 | **Cost rules** | Through R65's `effectiveCost`, which gains floors and "can't be played" last: next Trap or Field Spell discount; your Traps cost (0); Cost (3)+ cards cost (1) more, or can't be played; Spells cost (1) more; a cost floor (Forever&); set to (1); a `cost` hook counting Fruit played (like Ceaseless Void's). | C #2, #6, #35, #68, #77; C+ #14, #23, #49, #64; AI Alignment Tax |
| E16 | **Cards between players' piles** | Give cards from one hand to the other (C #9), draw from the opponent's deck (C #58), take a card from the opponent's deck (C+ #12.3) — all E2's ownership change; swap decks is R73's library swap. | C #9, #58, #85; C+ #12.3 |
| E17 | **Show the opponent's hand in a prompt** | Their hand cards are the options, seen by the chooser only, as KY's Private Tutor reveals library cards (§10.8, R310). | C #11 |
| E18 | **New prompt kinds** | `number` (pick a number from a fixed range: C #18), a mode prompt held by the *other* player (C #8), a multiple-choice answer (C+ #42), board cells (C+ #62), a quest reward (C #90), a budgeted multi-pick from a pile (C #44). The existing kinds stay. | as listed |
| E19 | **Plague Counters, extended** | "Place N Plague Counters" with no card named is N placements, each on a permanent (either side, face-down cards included) the placer chooses, repeats allowed, one prompt per token; "Place N Plague Counters on X" is one placement of N. Placement multipliers (×2, ×3); "when a Plague Counter is placed on this" (once per placement); stats per token (aura or self); tokens spent as mana; tokens consumed. `counterChanged` already carries `plague`. | C #27, #39, #42, #43, #53, #59, #61, #62, #63, #69, #70, #74, #76, #78, #87; C+ #3 |
| E20 | **Lock variants, Unlock** | Lock a whole lane, the zone a permanent was just played into, a random zone, the firing trap's own zone; unlock every zone (event `unlocked`). | C #71, #84; C+ #1, #34, #77 |
| E21 | **Backrow piles** | §3.2 already lets a Stack card onto an occupied zone "of the right row", but no backrow card has had Stack, so no backrow pile has ever existed and nothing handles one. Once something gives backrow cards Stack, backrow zones hold piles; only the top acts, so a face-down trap under a pile cannot fire and an aura under one is off. Ivory Tower is the one backrow pile a Unit may top (CL39). | C+ #33, #77 |
| E22 | **Flicker** | The card leaves the field and re-enters the same zone at once: R78's reset, summoning sick, no Cry, no Death. It counts as summoned. | C #14 r |
| E23 | **Fuse variants** | Fuse into a hand card keeping that card's cost; fuse a random card into every deck card keeping each one's cost; fuse the opponent's played card into your permanent; fuse three generated cards; Discover and fuse onto self; fuse an enemy card onto one of yours of its type. All R77/R102; "keeps its cost" is a `costOverride` of the cost it had. | C #78 r; C+ #30, #31, #43, #73, #74 |
| E24 | **Transform variants** | The cards beneath a Stack become copies of the top card; a unit becomes a random Classic or Classic+ Unit. | C+ #4, #73.1 |
| E25 | **Recruit extensions** | From the opponent's exile; N at once or "your entire deck"; with filters. Recruit stays permanents-only (R43's "recruits a permanent"). | C #1 r, #13 r, #31, #60, #65, #85 r; C #90 (reward I) |
| E26 | **Summon from hand or deck by trigger** | Hand triggers exist (Corpse Eater); add deck triggers and "summon this from your hand/deck". | C #33, #66; C+ #37 |
| E27 | **Delayed effects, more kinds** | Destroy at the start of your next turn; discard at the end of your *next* turn. `state.delayed` exists. | C #20, #37 r |
| E28 | **A player effect for the rest of the game** | A `never`-expiry player modifier that acts at its owner's start of turn. | C+ #52 |
| E29 | **Board snapshots** | See Classic+ #35 Rollback. | C+ #35 |
| E30 | **Match setup input from outside the match** | `createGame` takes each seat's last-game board; see Classic+ #29. | C+ #29 |
| E31 | **Question bank** | See Classic+ #42 KY's Test. | C+ #42 |
| E32 | **Curve targeting** | See Classic+ #62 KY's Papaya. | C+ #62 |
| E33 | **Quests** | See Classic #90 In Too Deep. | C #90 |
| E34 | **Perfect-hand scorer** | R29's Zephyrs scorer, extended to choose a hand. | C+ #27 |
| E35 | **Unit restrictions and statuses** | Can't be attacked; can be attacked only by units in its lane; can't attack or be attacked; immune to Spells (not targeted by one and unaffected by one); Berserk; may attack again after a kill; a keyword that holds only while a condition does; forced attacks on "a random enemy" and on the unit's *own* hero. Forced attacks skip §4.2's steps 1–3 (R53) but not these restrictions: a forced attack on a target its attacker may not attack does not happen, and "a random enemy" is drawn from the targets it may attack. | C #69, #78; C+ #19.1, #19.2, #19.5, #33, #51, #73.1 |
| E36 | **Lines of code** | Every card definition carries `loc`: the non-blank, non-comment lines of its script file (imports excluded), computed by a `scripts/` generator into the catalog and held current by a test, as `_generated.ts` is. A fused card's `loc` is its ingredients' sum. It is public (printed in the inspect overlay) and part of the card's patch history (B4.2), so a refactor that moves a card's `loc` is a balance change. | C #48; C+ #44, #45 |
| E37 | **Random split damage** | "Deal N damage split among enemies": N hits of 1, each to a random enemy (hero or unit) still standing. | C+ #3 |
| E38 | **Buffs and keywords in hand and deck** | §10.4's layer 4 already keeps a hand buff (R243, Corpse Eater); extend it to granted keywords and to the deck, carried onto the field when the card enters. | C+ #40, #77 |
| E39 | **Enchantments that ride a card** | Flags on the instance that persist in every zone: "after this resolves, return it to your hand; it costs at least (2)" (C+ #14), "Cast on draw; target enemies when possible" (C+ #40), Brittle (B3.3), `tuning` (B3.4). | as listed |
| E40 | **Catalog shape** | X in stats (C+ #69), a face's own type (C+ #22), a Token's printed rarity (C+ #65.x), `params` (B3.4), `loc` (E36). | as listed |

New events, each with a BUILD M5-T4 row and a `SOUND_CUES` row (the maps are total, so the build fails
until they exist): `cardAnnounced`, `countered`, `stolen`, `unlocked`, `activated`, `animated`, `deanimated`, `crumbled`,
`degraded`, `upgraded`, `redirected`, `healthSet`, `questProgressed`, `questCompleted`, `rolledBack`.

---
## B6. Classic, card by card

Ninety cards, `classic-001` to `classic-090`. Header line: id · cost, type and tags · rarity · stats
base → Radiant.

#### Classic #1 · Curse of the Forgotten Classic
`classic-001` · (1) Spell · Rare

> **Designer:** Deal 1 damage to the opposing Hero for each card in their Exile. Draw 1 ~~~ Deal 1
> damage to the opposing Hero for each card in their Exile. Also Recruit a card from their Exile. If
> it’s a Unit, it attacks them right away.

- **Text:** Deal 1 damage to the enemy hero for each card in their exile. Draw 1.
- **Radiant:** Deal 1 damage to the enemy hero for each card in their exile. Draw 1. Recruit a card
  from their exile. If it's a Unit, it attacks the enemy hero at once.
- **Engine:** one hit of N, N = the opponent's exile size as it resolves (N = 0 is no hit, R63), then a
  draw. Radiant: E25 — the most recently exiled permanent card in their exile is summoned under your
  control, its owner unchanged (it goes back to their piles when it leaves the field, §3.2); a Unit
  then makes one forced attack on the enemy hero (R53), summoning sickness ignored.
- **Rulings:** "for each card" in one sentence is one hit (Armor once), Hearthstone's reading. Exile is
  chronological (§3), so Recruit's "top down" is "newest first" there.
- **Numbers:** damage per card 1 ↑; draw 1 ↑.
- **Check:** the Radiant text omits "Draw 1"; "Also" reads as adding to the base face and R275 wants
  the Radiant stronger, so the draw is kept. **⚠ designer.**

#### Classic #2 · The Trickster
`classic-002` · (1) Unit, Human · Common · 2/1 → 4/2

> **Designer:** 2/1 Cry: Your next Trap or Field Spell costs (2) less. ~~~ 4/2 Cry: Your next Trap or
> Field Spell costs (0).

- **Text:** Cry: Your next Trap or Field Spell costs (2) less.
- **Radiant:** Cry: Your next Trap or Field Spell costs (0).
- **Engine:** a player modifier like Lunar Eclipse's next-Spell discount (`costDiscount`), for Trap,
  Field Trap and Field Spell, expiring when used (E15). The Radiant face sets the cost to 0 instead.
- **Rulings:** "next" has no "this turn", so it waits across turns until used. Field Trap counts as Trap.
- **Numbers:** discount 2 ↑.

#### Classic #3 · Book of Heal
`classic-003` · (1) Spell, Book · Common

> **Designer:** Heal 9 ~~~ Heal 18

- **Text:** Heal a target 9. **Radiant:** Heal a target 18.
- **Engine:** §6.3 Heal on any unit or hero, as Core #47 Fig of Life reads "Heal".
- **Numbers:** heal 9 ↑ (step 2).

#### Classic #4 · Palantir
`classic-004` · (1) Field Spell · Legendary

> **Designer:** Your opponent is limited to 1 Draw per turn. When your opponent casts a Book, Tribute
> this to Steal it. ~~~ Your opponent is limited to 1 Draw per turn. When your opponent casts a Spell,
> Steal it. Tribute this once it has stolen (2) or more cost worth of Spells.

- **Text:** Aura: Your opponent can't draw more than 1 card each turn.
  When your opponent plays a Book, you may Tribute this to steal it.
- **Radiant:** Aura: Your opponent can't draw more than 1 card each turn.
  When your opponent plays a Spell, steal it. Once this has stolen Spells costing (2) or more in
  total, Tribute this.
- **Engine:** E3 on the opponent, on every turn. The steal answers the opponent's announce of a
  Spell with the Book tag, in E1's window before §10.5 step 4: the base face asks its controller (a
  prompt during the opponent's turn, with R79's 30-second clock); yes sacrifices Palantir, counters
  the Book (E1) and moves it to your hand as yours (E2). The Radiant face steals every Spell with no question, adds
  each one's own cost (R65, out of play) to `memory.stolenCost`, and sacrifices itself once that
  reaches 2.
- **Rulings:** "casts" is the designer's word for plays; a cast by an effect (R70) counts too. "Tribute
  this to Steal it" is a price, so the base face asks. **⚠ designer** if it should be automatic.
- **Numbers:** draw limit 1 ↓ (never below 1); Radiant threshold 2 ↑.

#### Classic #5 · Tesla
`classic-005` · (2) Field Trap · Epic · 1/4 → 2/8

> **Designer:** 1/4 Animated Lifesteal Activates when an opponent summons a Unit. Deal 4 damage to it.
> Then summon this in Defense Position. ~~~ 2/8 Animated Lifesteal Activates when an opponent summons
> a Unit. Deal 8 damage to it. Then summon this in Defense Position.

- **Text:** Animated, Lifesteal
  Reveals when your opponent summons a Unit: Deal 4 damage to it. Then summon this as a Unit in
  Defense Position.
- **Radiant:** the same with 8 damage.
- **Engine:** a Field Trap answering each Unit that arrives on the opponent's side, however it
  arrives (§6.3's Summon: played, cast, token, Recruit, Reborn). A played Unit is answered after it
  resolves, as Bear Honeypot answers (R17, Hearthstone's Snipe), so its Cry happens first. The hit's
  source is Tesla, whose Lifesteal heals you (§4.4 step 8). Then it animates in Defense Position
  (B3.1); already a Unit, it stays put. It keeps firing while animated: a turret.
- **Numbers:** damage 4 ↑.
- **Check:** zapping every arrival for 4 and healing 4 each time is strong at (2). **⚠ designer** if it
  should fire only once (it would then be a Trap).

#### Classic #6 · Cloaked Toe Cracker
`classic-006` · (2) Unit, Human · Common · 3/4 → 6/8

> **Designer:** 3/4 Your Traps cost (0). ~~~ 6/8 Your Traps cost (0). Gain +1 Mana after playing one.

- **Text:** Aura: Your Traps cost (0).
- **Radiant:** Aura: Your Traps cost (0). After you play a Trap, gain 1 mana.
- **Engine:** E15 cost aura on its controller's Traps and Field Traps in hand. Radiant: a trigger on
  your `cardPlayed` of a Trap, +1 temporary mana.
- **Numbers:** Radiant mana 1 ↑.

#### Classic #7 · InfiniScepter
`classic-007` · (1) Field Spell · Legendary

> **Designer:** Cry: Exile a (0) or (1) cost Spell from your hand. Activate: Cast a copy of that Spell. ~~~
> Cry: Exile a (0)-(2) cost Spell. Activate: Cast a copy of that Spell.

- **Text:** Cry: Exile a Cost (1) or less Spell from your hand.
  Activate: Cast a copy of that Spell.
- **Radiant:** Cry: Exile a Cost (2) or less Spell from your hand.
  Activate: Cast a copy of that Spell.
- **Engine:** the Cry declares a hand pick (R81) filtered to Spells costing 1 (2) or less, exiles it
  and remembers it (`memory.scepter = { defId, radiant }`). Activate (B3.2, once per turn): cast a
  fresh copy (E12, R70; a card cast from no zone goes to your graveyard after, R87), your choices.
- **Rulings:** with nothing remembered the ability can't be activated. The exiled card stays in exile.
  The Radiant text drops "from your hand"; kept, since the Cry has nowhere else to look. **⚠ designer.**
- **Numbers:** cost limit 1 ↑.

#### Classic #8 · Pickle
`classic-008` · (1) Spell · Rare

> **Designer:** Your opponent chooses 3 times: They Discard 1 · They Exile the bottom card of their
> Library · You Draw 1 ~~~ Your opponent chooses 3 times: They Discard 2 · They Exile the bottom 2
> cards of their Library · You Draw 2

- **Text:** Your opponent chooses three times: they discard a card, they exile the bottom card of
  their deck, or you draw a card.
- **Radiant:** Your opponent chooses three times: they discard 2 cards, they exile the bottom 2 cards
  of their deck, or you draw 2 cards.
- **Engine:** E18: three mode prompts held by the opponent, one after another (repeats allowed);
  "discard" opens their own hand pick (R16). Only choices that would do something are offered, and
  "you draw" always is.
- **Rulings:** the opponent answers during your turn, a non-active player's prompt with its own clock
  (R79); a timeout answers with the AI policy.
- **Numbers:** choices 3 ↑; discard 1 ↑; exile 1 ↑; draw 1 ↑.

#### Classic #9 · Income Tax
`classic-009` · (2) Trap · Common

> **Designer:** Activates when your opponent draws the 2nd card on a turn. They give you all but one
> card of their choice from their hand. ~~~ Activates when your opponent draws the 2nd card on a turn.
> They give you all but one card of their choice from their hand and the cost of those cards are all
> reduced by (1).

- **Text:** Reveals when your opponent draws their 2nd card in a turn: They keep one card of their
  choice and give you the rest of their hand.
- **Radiant:** … The cards you get cost (1) less.
- **Engine:** E4's per-turn draw count on both players' turns (their start-of-turn draw counts, so on
  their turn any extra draw sets it off). The trap fires once the draw is complete (a cast-on-draw
  card cast first, R58). The opponent picks one hand card to keep (their prompt); the rest move to
  your hand and become yours (E2, E16); your hand cap burns the overflow into your graveyard.
  Radiant: `costMod −1` on each.
- **Numbers:** trigger draw 2 ↓ (never below 2); Radiant discount 1 ↑.

#### Classic #10 · Exile
`classic-010` · (2) Trap · Common

> **Designer:** Activates when your opponent plays a card that costs (1) or less. Counter and Exile
> it. ~~~ Activates when your opponent plays a card that costs (3) or less. Counter and Exile it. If
> it costs less than (3), Exile random Permanents from your opponent’s Field until the difference in
> cost is made up (but never exceeded). {AI please find a way to clean up this text}

- **Text:** Reveals when your opponent plays a card that costs (1) or less: Counter and exile it.
- **Radiant (the clean-up asked for):** Reveals when your opponent plays a card that costs (3) or
  less: Counter and exile it. Then exile random enemy permanents with a total cost of up to (3) minus
  its cost.
- **Engine:** E1 in the announce window, the card exiled rather than sent to the graveyard. Radiant:
  budget = 3 − the countered card's cost; repeat: pick a random enemy permanent whose cost (R65 as it
  stands on the field; an X card at the X it was played for, CL17) is at most the budget, exile it,
  subtract its cost; stop when the budget is 0 or nothing fits.
- **Rulings:** "costs" is the cost paid, as Core #60 Bear Honeypot reads it, so a card cast for free
  (R70) always qualifies. A 0-cost permanent always fits while the budget is above 0.
- **Numbers:** threshold 1 ↑ (Radiant 3).
- **Check:** the name is also a rules word (B2.8).

#### Classic #11 · Mind Melt
`classic-011` · (1) Spell · Common

> **Designer:** Look at your opponent’s hand. Exile a card in it. ~~~ Look at your opponent’s hand.
> Exile all cards of a single cost in it.

- **Text:** Look at your opponent's hand. Exile a card from it.
- **Radiant:** Look at your opponent's hand. Choose a cost. Exile every card of that cost from it.
- **Engine:** E17: their hand cards are the prompt's options, seen by you alone. Radiant: the options
  are their hand grouped by cost (the cost each would be played for now); every card of the chosen
  cost is exiled.
- **Rulings:** the opponent sees that a prompt is open, then which cards left their hand (exile is
  public).
- **Numbers:** cards exiled 1 ↑.

#### Classic #12 · Book of Blood
`classic-012` · (1) Spell, Book · Common

> **Designer:** Deal 5 damage to a Unit. Lifesteal. ~~~ Deal 10 damage to a Unit. Lifesteal.

- **Text:** Lifesteal
  Deal 5 damage to a Unit.
- **Radiant:** the same with 10.
- **Engine:** a Unit target; the effect states its own Lifesteal (R85).
- **Numbers:** damage 5 ↑.

#### Classic #13 · Boots on the Ground
`classic-013` · (1) Unit, Human · Common · 2/1 → 4/2

> **Designer:** 2/1 Charge After this attacks, Draw 1. ~~~ 4/2 Charge After this attacks, Recruit.

- **Text:** Charge
  After this attacks, draw 1.
- **Radiant:** Charge
  After this attacks, Recruit a card.
- **Engine:** a trigger after each combat it attacked in (forced attacks included), whether or not it
  survived (it reads its last-known state, R78). Radiant: E25, the first permanent from the top of your
  deck.
- **Numbers:** draw 1 ↑; Radiant recruits 1 ↑.

#### Classic #14 · Shadowstep
`classic-014` · (2) Trap · Common

> **Designer:** Activates when any number of your Units die. Return them all to your hand. They cost
> (0). ~~~ Activates when any number of your Units die. Flicker them so that they survive and add
> copies to your hand that cost (0).

- **Text:** Reveals when any of your Units die: Return them to your hand. They cost (0).
- **Radiant:** Reveals when any of your Units would die: Flicker them instead, so they survive. Add a
  copy of each to your hand. The copies cost (0).
- **Engine:** base: one firing for every Unit of yours one state-check pass collects; each card still
  in a graveyard afterwards goes to its owner's hand (§3.2) with `costOverride 0`. Tokens have ceased
  to exist (R11); a Reborn unit that came back is on the field, not in a graveyard, and is skipped.
  Radiant: E5's "would die" window at §4.5 step 1 — those units leave the collection and flicker
  (E22): back in their zones, reset, at full health, summoning sick; a fresh copy of each (Radiant
  flag kept, R57) goes to your hand costing 0.
- **Numbers:** cost 0 ↓.

#### Classic #15 · Nose Hunter
`classic-015` · (1) Unit, Human · Common · 3/1 → 6/2

> **Designer:** 3/1 Discard a random card: Exile the bottom card of each opponent's Deck. ~~~ 6/2
> Discard a random card: Exile the bottom card of each opponent's Deck and a random card in their
> hand.

- **Text:** Activate: Discard a random card. Exile the bottom card of your opponent's deck.
- **Radiant:** Activate: Discard a random card. Exile the bottom card of your opponent's deck and a
  random card from their hand.
- **Engine:** B3.2 Activate with a cost (a random discard; no card in hand, no activation).
  "Each opponent" is the multiplayer-proof phrasing Heroic Power uses; with two players it is the
  opponent.
- **Rulings (CL13):** "Cost: effect" is read as an activated ability, once per turn — the designer's
  new keyword is how a card is "clicked to do an effect". **⚠ designer:** once per turn, or ♾️?
- **Numbers:** exiled 1 ↑.

#### Classic #16 · Book of Flame
`classic-016` · (1) Spell, Book · Common

> **Designer:** Deal 4 damage ~~~ Deal 8 damage

- **Text:** Deal 4 damage. **Radiant:** Deal 8 damage.
- **Engine:** one targeted hit.
- **Numbers:** damage 4 ↑.
- **Check:** this is the Book of Flame that #23 Devil's Pact and #29 Book of Vital Kill name (B2.8).

#### Classic #17 · Counterspell
`classic-017` · (2) Trap · Common

> **Designer:** Activates when your opponent casts a Spell. Counter it. ~~~ Activates when your
> opponent casts a Spell. Counter it. Add a (0) cost copy to your hand.

- **Text:** Reveals when your opponent plays a Spell: Counter it.
- **Radiant:** Reveals when your opponent plays a Spell: Counter it. Add a copy of it to your hand.
  The copy costs (0).
- **Engine:** E1 on the announce of a Spell (the Spell type, not Field Spells). Radiant: a fresh copy
  (Radiant flag kept) in your hand, yours, `costOverride 0`.
- **Numbers:** Radiant cost 0 ↓.
- **Check:** shares its name with #72 (B2.8).

#### Classic #18 · Glitch in the System
`classic-018` · (3) Spell · Common

> **Designer:** Pick a number. Exile all cards that cost that much. ~~~ Pick a number. Exile all
> cards from your opponent’s Field, Hand, and Deck that cost that much.

- **Text:** Choose a number. Exile every card on the field, in hands and in decks that costs that much.
- **Radiant:** Choose a number. Exile every card on your opponent's field, in their hand and in their
  deck that costs that much.
- **Engine:** E18's `number` choice, declared with the play (R81) from a fixed list, 0 to 10
  (`GLITCH_NUMBERS`), so the options reveal nothing. Costs read per R65 at resolution, as R66 reads
  Genn's Greed's: a hand card at its hand cost, a deck or field card at its own; an X-cost card counts
  the X it was played for on the field and 0 anywhere else (CL17). The spell itself is resolving, in
  no pile, and is spared.
- **Rulings:** the base face reaches both players' field, hand and deck — the zones its Radiant face
  names; the Radiant narrows whose, not where. Graveyards and exile are untouched.
- **Numbers:** none (the number is chosen).

#### Classic #19 · Lizard’s Breath
`classic-019` · (1) Spell · Rare

> **Designer:** Deal 2 damage. Gains one of the additional effects based on which Pile has the most
> cards: Deck: Draw 1 · GY: +2 Mana · Exile: Deal +4 damage. {Note: Ties are broken from top to
> bottom} ~~~ Deal 4 damage. Gains two of the additional effects based on the two Piles with the most
> cards: Deck: Draw 1 · GY: +2 Mana · Exile: Deal +4 damage.

- **Text:** Deal 2 damage. Your largest pile adds its effect: Deck, draw 1; Graveyard, gain 2 mana;
  Exile, deal 4 more damage. Ties go to the pile listed first.
- **Radiant:** Deal 4 damage. Your two largest piles add their effects (same list, same ties).
- **Engine:** your own deck, graveyard and exile, counted as it resolves. The damage is one hit on the
  play's target: 2, or 6 with Exile (Radiant 4 or 8); then the draw and the temporary mana.
- **Numbers:** damage 2 ↑; draw 1 ↑; mana 2 ↑; extra damage 4 ↑.
- **Check:** a `preview` (R280) naming the pile or piles that would count now.

#### Classic #20 · The Power to Punish
`classic-020` · (2) Field Spell · Rare

> **Designer:** Activate: Choose one: Deal 2 damage · Opponent Discards 1 · Choose a Unit. It is
> Destroyed at the start of your next turn. ~~~ Activate: Choose one: Deal 4 damage · Opponent
> Discards 2 · All opposing Units are Destroyed at the start of your next turn.

- **Text:** Activate: Choose one: Deal 2 damage; your opponent discards a card; or choose a Unit,
  which is destroyed at the start of your next turn.
- **Radiant:** Activate: Choose one: Deal 4 damage; your opponent discards 2 cards; or all enemy Units
  are destroyed at the start of your next turn.
- **Engine:** B3.2 Activate, mode and target declared in the action. The delayed destroy is E27, keyed
  to the unit (R174: it fizzles if that unit left the field meanwhile). The Radiant's third mode
  destroys the enemy Units there *then*, not a list fixed now.
- **Numbers:** damage 2 ↑; discards 1 ↑.

#### Classic #21 · Turtinator
`classic-021` · (2) Unit · Common · 5/4 → 10/8

> **Designer:** 5/4 Activate ♾️: Tribute a Unit. Deal damage equal to its attack to any target. ~~~
> 10/8 Activate ♾️: Tribute a Unit. Deal damage equal to twice its attack to any target.

- **Text:** Activate ♾️: Tribute a Unit. Deal damage equal to its Attack to any target.
- **Radiant:** Activate ♾️: Tribute a Unit. Deal damage equal to twice its Attack to any target.
- **Engine:** B3.2 with the cost "sacrifice one of your units" (a declared pick, itself allowed), then
  one hit from Turtinator equal to that unit's attack as it stood (last-known, R78).
- **Rulings:** one unit is one Tribute here: a Sheep's "worth 2" counts only toward a Tribute X (§6.3).
- **Numbers:** multiplier 1 ↑ (Radiant 2).

#### Classic #22 · Mid Runner
`classic-022` · (1) Unit, Human · Common · 2/1 → 4/2

> **Designer:** 2/1 Cry: If this was Summoned in midlane, Tribute it. If you had (4)+ Mana when cast,
> Bounce 2 random enemy cards. ~~~ 4/2 Cry: If this was Summoned in midlane, Tribute it. If you had
> (4)+ Mana when cast, Bounce 2 random enemy cards.

- **Text:** Cry: If this is in lane 3, Tribute it. If you had 4 or more mana when you played this,
  bounce 2 random enemy permanents.
- **Radiant:** the same text.
- **Engine:** two independent checks. Midlane is lane 3. "When cast" is the mana before paying for it
  (recorded as the play begins, §10.5 step 1). Two different random enemy permanents (R60) go to their
  owners' hands.
- **Numbers:** mana threshold 4 ↓; bounces 2 ↑.
- **Check:** **R275**: the Radiant face only doubles the stats. Suggest "bounce 4", or record the
  designer's word in `docs/radiant-audit.md` as v0.1.1 did. **⚠ designer.**

#### Classic #23 · Devil’s Pact
`classic-023` · (2) Field Spell · Rare

> **Designer:** Cry: Discard 666 Activate: All cards you play are this turn are replaced by Book of
> Flame. ~~~ Cry: Discard 6 Activate: All cards you play this turn are replaced by Radiant Book of
> Flame.

- **Text:** Cry: Discard 666 cards.
  Activate: This turn, each card you play is replaced by a Book of Flame.
- **Radiant:** Cry: Discard 6 cards.
  Activate: This turn, each card you play is replaced by a Radiant Book of Flame.
- **Engine:** the Cry discards your choice of up to N (666 is the whole hand). Activate installs a
  this-turn player modifier: at §10.5 step 3 a card you play is replaced (R31, R35) by a new Book of
  Flame instance (#16; Radiant on the Radiant face), which resolves as that play and asks its target
  then; the old card ceases to exist; the price paid was the old card's. Casts are plays (R70) and are
  replaced too.
- **Numbers:** discards 666 ↓ (Radiant 6).
- **Check:** "are this turn" typo. The Radiant's smaller discard is the joke and the upgrade.

#### Classic #24 · Book of Knowledge
`classic-024` · (1) Spell, Book · Epic

> **Designer:** Draw 3 ~~~ Draw 6

- **Text:** Draw 3. **Radiant:** Draw 6.
- **Numbers:** draw 3 ↑.

#### Classic #25 · Lag in the System
`classic-025` · (0) Spell · Common

> **Designer:** Exile all (0) and (1) cost cards. ~~~ Exile all enemy (0) and (1) cost cards.

- **Text:** Exile every card on the field, in hands and in decks that costs (1) or less.
- **Radiant:** Exile every enemy card on the field, in their hand and in their deck that costs (1) or
  less.
- **Engine:** #18 with the numbers fixed at 0 and 1: the same zones, the same cost reading (CL17: an
  X card in a hand or deck costs 0, so it goes; on the field it costs its X), the spell itself spared.
- **Numbers:** threshold 1 ↑.

#### Classic #26 · Rapid Draw
`classic-026` · (0) Spell · Common

> **Designer:** Draw 4 Discard 4 ~~~ Draw 5 Discard 4

- **Text:** Draw 4. Then discard 4 cards. **Radiant:** Draw 5. Then discard 4 cards.
- **Engine:** four draws, then your choice of 4 to discard (R16; fewer in hand, all of them).
- **Numbers:** draw 4 ↑; discard 4 ↓.

#### Classic #27 · Pestilent Slime
`classic-027` · (0) Unit · Common · 1/1 → 2/2

> **Designer:** 1/1 Double Plague Tokens placed on this. ~~~ 2/2 Triple Plague Tokens placed on this.

- **Text:** Plague Counters placed on this are doubled. **Radiant:** … tripled.
- **Engine:** E19's placement multiplier on every placement onto it.
- **Numbers:** multiplier 2 ↑.

#### Classic #28 · Second Wind
`classic-028` · (0) Field Spell · Epic

> **Designer:** Cry: Exile your Deck. Discard your hand. Aura: You can play cards from your GY. When
> a card enters your GY, Exile it. ~~~ Cry: Exile your Deck. Discard your hand. Aura: You can play
> cards from your GY as long as (1)+ Mana is spent.

- **Text:** Cry: Exile your deck. Discard your hand.
  Aura: You may play cards from your graveyard. Cards that would go to your graveyard are exiled
  instead.
- **Radiant:** Cry: Exile your deck. Discard your hand.
  Aura: You may play cards from your graveyard that cost (1) or more.
- **Engine:** E11 (every card type), plus on the base face an E5 replacement for cards you own that
  would go to your graveyard. The Radiant's play from the graveyard needs a price of at least 1 as it
  would be paid, which stops a (0) loop.
- **Rulings (CL14):** the Cry's own discard lands in the graveyard before the Aura starts exiling, so
  the discarded hand is playable — the reading that gives the card its name. **⚠ designer.** With no
  deck left, every draw is fatigue (§2.4).
- **Numbers:** Radiant minimum price 1 ↓.

#### Classic #29 · Book of Vital Kill
`classic-029` · (1) Spell, Book · Epic

> **Designer:** Set a Hero’s health to 13. ~~~ Set a Hero’s health to 13. Add a Book of Flame to your
> hand.

- **Text:** Set a hero's health to 13.
- **Radiant:** Set a hero's health to 13. Add a Book of Flame to your hand.
- **Engine:** E7 on either hero (a declared target). Radiant: adds #16's base face (a card named without
  "Radiant" is its base face).
- **Numbers:** none (13 is good or bad depending on whose hero).

#### Classic #30 · Recycle
`classic-030` · (1) Spell · Rare

> **Designer:** Shuffle your GY into your Library. Draw 1. ~~~ Shuffle your GY into your Library.
> Reduce shuffled cards’ cost by (1). Draw 1.

- **Text:** Shuffle your graveyard into your deck. Draw 1.
- **Radiant:** Shuffle your graveyard into your deck. They cost (1) less. Draw 1.
- **Engine:** every card in your graveyard (this Spell is resolving, not in it) at random positions;
  R80's cap: cards that don't fit stay in the graveyard. Radiant: `costMod −1` on each. Then the draw.
- **Numbers:** draw 1 ↑; Radiant discount 1 ↑.
- **Check:** the name is inside "Malzahar's Recycler" (B2.8).

#### Classic #31 · Cookie Guild
`classic-031` · (2) Unit, Human · Common · 2/4 → 4/8

> **Designer:** 2/4 Cry: Recruit a Unit that costs (2) or less. ~~~ 4/8 Cry: Recruit 3 Units that
> cost (2) or less.

- **Text:** Cry: Recruit a Cost (2) or less Unit. **Radiant:** Cry: Recruit 3 Cost (2) or less Units.
- **Engine:** §6.3 Recruit filtered to Units whose deck cost (R65) is 2 or less; three scans on the
  Radiant face, as Core #69 Call to Arms does.
- **Numbers:** cost limit 2 ↑; units 1 ↑.

#### Classic #32 · Felinor Feelings
`classic-032` · (0) Spell, Felinor · Rare

> **Designer:** Take control of any card in a lane with a (1) cost Unit you control. ~~~ Summon a
> Felinor Token. Take control of any card in a lane with a (1) cost Unit you control.

- **Text:** Steal an enemy permanent in a lane where you control a Cost (1) Unit.
- **Radiant:** Summon a Felinor Token. Then steal an enemy permanent in a lane where you control a
  Cost (1) Unit.
- **Engine:** base: a declared target (R81), any enemy permanent (unit, backrow card, face-down card —
  you see a stolen trap from then on, R33) in a lane where a Unit you control has its own cost (R65;
  an X Unit its X, CL17) of exactly 1. Steal per §6.3 and R15. Radiant: the Felinor Token (Cost (1))
  lands first in your leftmost open zone, so the target is a prompt after it lands rather than a
  play-time pick.
- **Numbers:** none.

#### Classic #33 · Joro
`classic-033` · (0) Unit · Legendary · 1/1 → 1/1

> **Designer:** 1/1 After a friendly Unit is targeted by an opponent, this is instead summoned to be
> the new target. ~~~ 1/1 Indestructible After a friendly Unit is targeted by an opponent, this is
> instead summoned to be the new target.

- **Text:** While this is in your hand: When your opponent targets one of your Units, summon this and
  make it the new target.
- **Radiant:** Indestructible
  (the same).
- **Engine:** a hand trigger on E5's targeting point: an enemy attack declared at one of your units
  (§4.2 step 2, before the trap window) or an enemy play, cast, activation or prompt answer that picks
  one of your units. Joro is summoned (leftmost open zone; with none, nothing happens), no Cry,
  summoning sick, and the attack or pick is redirected to it (E9).
- **Rulings (CL15):** "targeted" means chosen — an attack's target or a declared or prompted pick;
  random picks and "all" effects target nothing. One Joro answers one targeting; a play that names
  several of your units redirects the first. Joro answers from the hand only (a Yu-Gi-Oh hand trap;
  the text has to summon it from somewhere). **⚠ designer:** hand only, or the deck too?
- **Numbers:** none.
- **Check:** **R275**: the Radiant face keeps 1/1 where the stat half asks for 2/2; it adds
  Indestructible, which makes it an endless decoy. Register a named exception or print 2/2.
  **⚠ designer.**

#### Classic #34 · Ancient Acquisition
`classic-034` · (1) Spell · Rare

> **Designer:** Return 2 cards from your GY to your hand. ~~~ Return 4 cards from your GY or Exile to
> your hand.

- **Text:** Return 2 cards from your graveyard to your hand.
- **Radiant:** Return 4 cards from your graveyard or exile to your hand.
- **Engine:** your choice of up to 2 (4) from the pile or piles (an E18 pick from a pile, without
  Discover's three-option limit).
- **Numbers:** cards 2 ↑.
- **Check:** #47 Recurring Felinor casts it.

#### Classic #35 · Prep
`classic-035` · (0) Spell · Common

> **Designer:** Your next Spell this turn costs (2) less. ~~~ Your next Spell this turn costs (4) less.

- **Text:** Your next Spell this turn costs (2) less. **Radiant:** … (4) less.
- **Engine:** Core #35 Lunar Eclipse's modifier (`costDiscount`, `onlyType: "Spell"`, this turn,
  consumed on use).
- **Numbers:** discount 2 ↑.

#### Classic #36 · Burn
`classic-036` · (0) Spell · Common

> **Designer:** Deal 2 damage. Draw 1 if you have at 4 or more remaining Mana. ~~~ Deal 4 damage. Draw
> 1 if you have at 4 or more Total Mana.

- **Text:** Deal 2 damage. If you have 4 or more mana left, draw 1.
- **Radiant:** Deal 4 damage. If your max mana is 4 or more, draw 1.
- **Engine:** "remaining" is current mana as it resolves; "Total Mana" is max mana (§2.3).
- **Numbers:** damage 2 ↑; threshold 4 ↓; draw 1 ↑.
- **Check:** "at 4" typo; the name is also a rules word (B2.8).

#### Classic #37 · Last Hurrah
`classic-037` · (0) Spell · Epic

> **Designer:** Draw your deck. At the end of this turn, Discard your hand. ~~~ Draw your deck. At the
> end of your next turn, Discard your hand.

- **Text:** Draw your deck. At the end of this turn, discard your hand.
- **Radiant:** Draw your deck. At the end of your next turn, discard your hand.
- **Engine:** "draw your whole library" (R58: its size as it starts; the hand cap burns everything past
  10, §2.4), then an E27 delayed discard of the whole hand at the end of this turn (Radiant: of your
  next turn). It is a discard, so Malzahar's Recycler sees it.
- **Numbers:** none.
- **Check:** by design a fatigue clock.

#### Classic #38 · Jackiestan Auctioneer
`classic-038` · (2) Field Trap, Human · Rare · 4/4 → 8/8

> **Designer:** 4/4 Animated Activates/summons when a player plays their 3rd card on a turn. Whenever
> a player plays a card, you Draw 1 and deal 2 damage to each enemy hero. ~~~ 8/8 Animated
> Activates/summons when a player plays their 2nd card on a turn. Whenever a player plays a card, you
> Draw 1 and deal 4 damage to each enemy hero.

- **Text:** Animated
  Reveals when a player plays their 3rd card in a turn: Summon this as a Unit.
  Once this has revealed: Whenever a player plays a card, draw 1 and deal 2 damage to the enemy hero.
- **Radiant:** the same with "2nd card" and 4 damage.
- **Engine:** a Field Trap answering any player's 3rd `cardPlayed` of a turn (E4 counts plays per
  player per turn; casts count, R70), then it animates (B3.1). From then on — animated, or stuck face-up
  in the backrow for want of a unit zone — a trigger on every `cardPlayed` by either player: draw 1 and
  one hit of 2 on the enemy hero ("each enemy hero" is the multiplayer phrasing).
- **Rulings (CL16):** while face-down only the activation condition is live; the "whenever" text starts
  with the play after the one that set it off (R119's rule about a permanent's own arrival).
- **Numbers:** trigger play 3 ↓ (never below 2); draw 1 ↑; damage 2 ↑.
- **Check:** a draw-and-burn engine on every card anyone plays, for (2); its 4/4 body is the answer.

#### Classic #39 · Outbreak
`classic-039` · (1) Spell · Epic

> **Designer:** Place a Plague Token. If the number of Plague Tokens is greater than or equal to its
> cost and it belongs to an enemy, Steal it. Otherwise, Draw equal to the number of Plague Tokens on
> it. ~~~ Place 2 Plague Tokens. If the number of Plague Tokens on the Permanent is greater than or
> equal to its cost and it belongs to an enemy, Steal it. Otherwise, Draw equal to the number of
> Plague Tokens on it.

- **Text:** Place a Plague Counter on a permanent. If it's an enemy permanent with at least as many
  Plague Counters as its cost, steal it. Otherwise, draw a card for each Plague Counter on it.
- **Radiant:** the same with 2 Plague Counters.
- **Engine:** one declared target, one placement of 1 (2) (E19; a Pestilent Slime doubles it). Its
  cost is R65's on the field (an X card at the X it was played for, CL17). Steal per §6.3 and R15;
  otherwise draw N (hand cap).
- **Rulings (CL17):** an X-cost card on the field costs the X it was played for (the instance's `x`,
  which §2.3 already stores) wherever a rule compares or counts costs; anywhere else, and on the field
  when it arrived without a chosen X (Recruit, a summon), it costs 0, as R65 says. This amends R65,
  whose "outside play … comparisons" would otherwise read a field card at 0, so Classic #25 Lag in the
  System would exile a Classic+ #69 Buff Billy played for 3. Classic #10, #18, #25 and #32 read costs
  the same way.
- **Numbers:** tokens 1 ↑.

#### Classic #40 · MC Tech
`classic-040` · (1) Unit · Rare · 3/3 → 6/6

> **Designer:** 3/3 Cry: If an opponent controls 4 or more Permanents, take control of one of them at
> random. ~~~ 6/6 Cry: If an opponent controls 4 or more Permanents, take control of one of them of
> your choice.

- **Text:** Cry: If your opponent controls 4 or more permanents, steal a random one.
- **Radiant:** Cry: If your opponent controls 4 or more permanents, steal one of your choice.
- **Engine:** the count and the pick at resolution (the Radiant's pick is a prompt, since the condition
  is read then); a random pick over their permanents (tops of piles and backrow). R15's placement; no
  free zone, it stays with them.
- **Numbers:** threshold 4 ↓.

#### Classic #41 · State of the Game
`classic-041` · (1) Unit · Common · 3/3 → 6/6

> **Designer:** 3/3 Indestructible ~~~ 6/6 Indestructible

- **Text:** Indestructible. **Radiant (proposed):** Indestructible, Lifesteal.
- **Engine:** keywords only; under R347 it never has Taunt.
- **Check:** **R275**: the designer's Radiant only doubles the stats, and a keyword-only Unit's
  Radiant owes one more keyword or a stronger one. Proposed Lifesteal (Taunt is out, R347).
  **⚠ designer.**

#### Classic #42 · Transmutable Toxins
`classic-042` · (2) Field Spell · Rare

> **Designer:** Activate: Place a 2 Plague Tokens at random on Units. Ally Units gain +1/+1 per Plague
> Token on them. Enemy Units gain -1/-1 per Plague Token on them. ~~~ Activate: Place 2 Plague Tokens
> at random on Units. Ally Units gain +2/+2 per Plague Token on them. Enemy Units gain -2/-2 per Plague
> Token on them.

- **Text:** Aura: Your Units have +1/+1 for each Plague Counter on them. Enemy Units have −1/−1 for each
  Plague Counter on them.
  Activate: Place a Plague Counter on each of 2 random Units.
- **Radiant:** +2/+2 and −2/−2.
- **Engine:** an aura (§10.4 layer 5) reading each unit's `counters.plague`; −1/−1 lowers max health, so
  an enemy can die of it at the state check (Core #46's rule). Activate (once per turn): one token
  on each of two different random units on the field, either side (R60: a random pick of N picks N
  different cards; with one unit on the field, it gets one token).
- **Numbers:** tokens 2 ↑; stats per token 1 ↑.
- **Check:** "a 2" typo.

#### Classic #43 · Plague Nuke
`classic-043` · (3) Spell · Epic

> **Designer:** Destroy all Units. +1 Mana for each Plague Token on them. ~~~ Destroy all Units. +1
> Mana for each Plague Token on them and those with Plague Tokens are resummoned from the GY under your
> control.

- **Text:** Destroy all Units. Gain 1 mana for each Plague Counter that was on them.
- **Radiant:** Destroy all Units. Gain 1 mana for each Plague Counter that was on them. Then summon, under
  your control, each of those Units that had a Plague Counter from its owner's graveyard.
- **Engine:** count the tokens on every unit first, destroy all (one state check, §4.5), then the
  temporary mana. Radiant: after that check, each non-token unit card that had a token and is now in a
  graveyard is summoned to your side (control yours, owner unchanged), leftmost open zones in lane
  order; a Reborn unit already back on the field is not summoned again; tokens are gone (R11).
- **Rulings:** an Indestructible unit survives, but its tokens count ("on them" is every Unit the spell
  hit).
- **Numbers:** mana per token 1 ↑.

#### Classic #44 · Back from the GY
`classic-044` · (4) Spell · Legendary

> **Designer:** Summon up to (5) mana worth of Units from your GY. Exile this. ~~~ Summon all Units
> from your GY. Exile this.

- **Text:** Summon Units from your graveyard with a total cost of (5) or less. Exile this.
- **Radiant:** Summon every Unit from your graveyard. Exile this.
- **Engine:** an E18 budgeted pick from your graveyard (costs per R65 out of play), then each is
  summoned (no Cry) into your leftmost open zones; a full board leaves the rest. Radiant: every Unit,
  oldest first, until the board is full. "Exile this" per §5.1.
- **Numbers:** budget 5 ↑.

#### Classic #45 · Nature Titan
`classic-045` · (2) Unit · Legendary · 6/6 → 12/12

> **Designer:** 6/6 Tribute 1 Cry and on Attack: Draw 1, your Hero Heals 3. ~~~ 12/12 Tribute 1 Cry
> and on Attack: Draw 2, your Hero Heals 6.

- **Text:** Tribute 1
  Cry and whenever this attacks: Draw 1 and heal your hero 3.
- **Radiant:** the same with draw 2 and heal 6.
- **Engine:** a Tribute cost (§6.3, R101; B4.5 lets it pay for its own zone). One script for the Cry and
  the attack trigger ("Cry and Death" share one script on Core #22; this is "Cry and on attack"), the
  trigger answering `attackDeclared` for this unit, forced attacks included.
- **Numbers:** draw 1 ↑; heal 3 ↑.

#### Classic #46 · Divine Favor
`classic-046` · (1) Spell · Rare

> **Designer:** Draw until you have as many cards as your opponent. ~~~ Draw until you have twice as
> many cards as your opponent.

- **Text:** Draw until you have as many cards in hand as your opponent.
- **Radiant:** Draw until you have twice as many cards in hand as your opponent.
- **Engine:** read as it resolves (this Spell has left your hand); one draw at a time until the target
  is met. A draw that adds no card — a fatigue hit, a burn at the hand cap — ends it, so it can't loop.
- **Numbers:** multiplier 1 ↑ (Radiant 2).

#### Classic #47 · Recurring Felinor
`classic-047` · (2) Unit, Felinor · Rare · 3/2 → 6/4

> **Designer:** 3/2 Cry: Cast Ancient Acquisition If this is in your GY and you activate a Trap, return
> this to your hand. ~~~ 6/4 Cry: Cast Ancient Acquisition If this is in your GY and you activate a
> Trap, return this to your hand. It costs (0).

- **Text:** Cry: Cast Ancient Acquisition.
  While this is in your graveyard: When one of your Traps activates, return this to your hand.
- **Radiant:** the same, and it costs (0) when it returns.
- **Engine:** E12: cast a generated #34 on its base face (goes to your graveyard afterwards, R87), your
  picks. A graveyard trigger (R68's graveyard triggers) on your `trapFired`; Radiant: `costOverride 0`.
- **Numbers:** Radiant cost 0 ↓.

#### Classic #48 · Hired Shrimp
`classic-048` · (2) Unit · Common · 4/3 → 8/6

> **Designer:** 4/3 Cry: Destroy a card that takes more lines of code to implement. ~~~ 8/6 Cry:
> Destroy a card that takes more lines of code to implement. Valid targets are highlighted.

- **Text:** Cry: Destroy a permanent whose card takes more lines of code to implement than this one.
- **Radiant:** the same. Valid targets are highlighted.
- **Engine:** E36's `loc`. Base: the Cry's target may be *any* permanent, and at resolution it is
  destroyed only if its `loc` is greater than Hired Shrimp's. Radiant: only qualifying permanents are
  offered — the highlight.
- **Rulings (CL18):** the joke is kept as a rule: the base face does not filter its targets, so a wrong
  guess fizzles. `loc` is public (printed in the inspect overlay), so the guess can be informed.
- **Numbers:** none.

#### Classic #49 · Anti-Greed Machine
`classic-049` · (3) Unit · Common · 9/9 → 18/18

> **Designer:** 9/9 Rush Draw is limited to 1 per turn. ~~~ 18/18 Rush Enemy Draw is limited to 1 per
> turn.

- **Text:** Rush
  Aura: Players can't draw more than 1 card each turn.
- **Radiant:** Rush
  Aura: Your opponent can't draw more than 1 card each turn.
- **Engine:** E3 on both players (Radiant: the opponent).
- **Numbers:** Radiant limit 1 ↓ (never below 1).

#### Classic #50 · Voidwalker
`classic-050` · (2) Unit · Rare · 6/3 → 12/6

> **Designer:** 6/3 Cry: Exile all GYs. Whenever a card is sent to a GY, Exile it. ~~~ 12/6 Cry: Exile
> the enemy GYs. Whenever an enemy card is sent to a GY, Exile it.

- **Text:** Cry: Exile every card in both graveyards.
  Aura: Cards that would go to a graveyard are exiled instead.
- **Radiant:** Cry: Exile every card in your opponent's graveyard.
  Aura: Cards your opponent owns that would go to a graveyard are exiled instead.
- **Engine:** E5's "would go to a graveyard" replacement while it is on the field.
- **Rulings (CL19):** Voidwalker's own card reaches the graveyard when it dies: it has left the field,
  and its aura with it, as its move begins.
- **Numbers:** none.

#### Classic #51 · Back Breaker
`classic-051` · (1) Unit · Common · 3/2 → 6/4

> **Designer:** 3/2 Stack Death: Destroy all backrow. ~~~ 6/4 Stack Death: Destroy all enemy backrow.

- **Text:** Stack
  Death: Destroy every backrow card.
- **Radiant:** Stack
  Death: Destroy every enemy backrow card.
- **Engine:** both backrows, face-down cards included; Indestructible ones stay (Heroic Power,
  Lockdown, In Too Deep). An animated card is in the unit row and is not backrow (B3.1).
- **Numbers:** none.

#### Classic #52 · Final Gambit
`classic-052` · (2) Trap · Epic

> **Designer:** Activate when you would take lethal damage. Redirect it to your opponent, heal 10, and
> Draw 3. ~~~ Activate when you would take lethal damage. Redirect it to your opponent, heal 20, and
> Draw your deck.

- **Text:** Reveals when a hit would bring your hero to 0 or less: Redirect the hit to the enemy hero.
  Then heal your hero 10 and draw 3.
- **Radiant:** the same, then heal your hero 20 and draw your deck.
- **Engine:** E5 at §4.4 after the hero caps and before step 5: when the amount would leave your hero at
  0 or less, the trap fires and the instance is re-aimed at the enemy hero (E9) as a new instance from
  the same source, through their Armor and caps; then the heal and the draws (the Radiant's per R58,
  most of it burning at the hand cap).
- **Rulings:** "lethal" is this hit alone, as My Pawn judges an attack (R44). Fatigue is damage and
  counts; losing health is not (R18). If the redirected hit kills the opponent, the state check ends the
  game (a draw if both heroes are at 0, §2.5).
- **Numbers:** heal 10 ↑; draw 3 ↑.

#### Classic #53 · Plague Crawler
`classic-053` · (1) Unit · Common · 2/2 → 4/4

> **Designer:** 2/2 Cry: Place a Plague Token on another card. When a Plague Token is placed on this,
> Draw 1. ~~~ 4/4 Cry: Place 2 Plague Tokens on another card. When a Plague Token is placed on this,
> Draw 2.

- **Text:** Cry: Place a Plague Counter on another permanent.
  Whenever Plague Counters are placed on this, draw 1.
- **Radiant:** 2 Plague Counters; draw 2.
- **Engine:** a declared target (another permanent); E19's placement trigger, once per placement however
  many tokens it places.
- **Numbers:** tokens 1 ↑; draw 1 ↑.

#### Classic #54 · Rewind
`classic-054` · (1) Spell · Common

> **Designer:** Trigger the Cry of a friendly Unit on the Field or in the GY. ~~~ Trigger the Cry of a
> Unit twice on the Field or in the GY.

- **Text:** Trigger the Cry of one of your Units on the field or in your graveyard.
- **Radiant:** Trigger the Cry of any Unit on the field or in a graveyard, twice.
- **Engine:** E13 on a declared target: a unit that has a Cry (top of a pile, or a unit card in a
  graveyard). Radiant: either side, two separate resolutions, each with its own choices.
- **Numbers:** repeats 1 ↑ (Radiant 2).

#### Classic #55 · Book of Flame (the Epic one)
`classic-055` · (1) Spell, Book · Epic

> **Designer:** Deal 4 damage. ~~~ Deal 8 damage.

- **Engine:** identical to #16.
- **Check:** **⚠ designer (B2.8):** the same name and text as #16 at another rarity. Hold it out of the
  first wave until it has its own name and text, or cut it.

#### Classic #56 · Spell Tyrant
`classic-056` · (4) Unit · Legendary · 5/5 → 10/10

> **Designer:** 5/5 Cry: Cast 3 Spells from your GY. Exile them afterward. ~~~ 10/10 Cry: Cast all
> Spells from your GY. Exile them afterward.

- **Text:** Cry: Choose up to 3 Spells in your graveyard. Cast them, then exile them.
- **Radiant:** Cry: Cast every Spell in your graveyard, oldest first, then exile them.
- **Engine:** E12 casts from the graveyard, one at a time, each with your choices (R70), each exiled
  after it resolves instead of going back. The Radiant face casts the Spells there as the Cry begins; a
  Spell a cast puts in the graveyard is not cast.
- **Rulings:** "Cast 3 Spells" has no "random", so they are your choice.
- **Numbers:** spells 3 ↑.

#### Classic #57 · Echo
`classic-057` · (1) Spell · Epic

> **Designer:** Has the text of the last played Spell (from either player). ~~~ Has the text of the
> last played Spell (from either player). Echo 1.

- **Text:** This has the text of the last Spell either player played.
- **Radiant:** Echo 1
  This has the text of the last Spell either player played.
- **Engine:** E4's game-wide `lastSpell = { defId, radiant }`. In hand, its view carries that Spell's
  face under Echo's name (as R243 carries a fused card's); played, it declares and resolves that Spell's
  choices and script. Radiant: plus Echo 1 (§6.2).
- **Rulings (CL20):** a played Echo records the Spell it copied, never "Echo", so it can't copy itself
  into a loop. With no Spell played yet it has no text and does nothing. "Spell" is the Spell type.
- **Numbers:** Radiant Echo 1 ↑.
- **Check:** the name is also a keyword (B2.8).

#### Classic #58 · Common Resources
`classic-058` · (2) Field Spell · Common

> **Designer:** Start of Turn: Draw the bottom card of the enemy Deck. ~~~ Start of Turn and End of
> Turn: Draw the bottom card of the enemy Deck.

- **Text:** Start of turn: Draw the bottom card of your opponent's deck.
- **Radiant:** Start of turn and end of turn: Draw the bottom card of your opponent's deck.
- **Engine:** E16: a draw of yours (hand cap, cast on draw for you, your E3 limit) taken from their
  deck's bottom; the card becomes yours (E2). An empty enemy deck gives nothing, and fatigue for no one.
- **Numbers:** cards 1 ↑.

#### Classic #59 · Plague Doctor
`classic-059` · (1) Unit, Human · Common · 2/3 → 4/6

> **Designer:** 2/3 Cry: Deal damage equal to the number of Plague Tokens on the field. ~~~ 4/6 Cry:
> Place 2 Plague Tokens on this. Deal damage equal to the number of Plague Tokens on the field.

- **Text:** Cry: Deal damage equal to the number of Plague Counters on the field.
- **Radiant:** Cry: Place 2 Plague Counters on this. Then deal damage equal to the number of Plague
  Tokens on the field.
- **Engine:** a declared target; one hit of N, every token on both sides counted as it resolves (after
  the Radiant's own placement). A `preview` (R280) shows N.
- **Numbers:** Radiant tokens 2 ↑.

#### Classic #60 · Pile On
`classic-060` · (5) Spell · Rare

> **Designer:** Recruit your entire Deck. If this is put in your GY, put it at the bottom of your
> Library instead. ~~~ Recruit your entire Deck.

- **Text:** Recruit every permanent in your deck.
  If this would go to your graveyard, put it on the bottom of your deck instead.
- **Radiant:** Recruit every permanent in your deck.
- **Engine:** E25: scan your deck top to bottom, summoning each permanent until its row is full (units
  into unit zones, the rest into backrow zones, traps face-down); Spells stay. The base face's clause is
  an E5 replacement on this card's own move to your graveyard, whatever sends it there.
- **Check:** the Radiant drops the base's return to the deck. Read as intended: a (5)-cost Spell coming
  back is a dead draw later, so losing the clause is the upgrade. Cost (5) needs mana gain (§2.3).
- **Numbers:** none.

#### Classic #61 · Plague Bringer Goliath
`classic-061` · (3) Unit · Legendary · 7/7 → 14/14

> **Designer:** 7/7 Tribute 1 Rush Trample Cry: Place 3 Plague Tokens. Draw 1. ~~~ 14/14 Tribute 1
> Rush Trample Cry: Place 3 Plague Tokens. Draw 3.

- **Text:** Tribute 1, Rush, Trample
  Cry: Place 3 Plague Counters. Draw 1.
- **Radiant:** the same with draw 3.
- **Engine:** Tribute (B4.5 applies); E19: three placements, each on a permanent you choose.
- **Numbers:** tokens 3 ↑; draw 1 ↑.

#### Classic #62 · Living Bomb
`classic-062` · (1) Field Spell · Rare

> **Designer:** At the start of each player’s turn, they destroy all cards with a Plague Counter on
> them. ~~~ At the start of each enemy’s turn, they destroy all cards with a Plague Counter on them.

- **Text:** At the start of each player's turn: Destroy every permanent that player controls with a
  Plague Counter on it.
- **Radiant:** At the start of your opponent's turn: Destroy every permanent they control with a Plague
  Token on it.
- **Engine:** a start-of-turn trigger on both players' turns (the text names each player's), in R68's
  order. "Plague Counter" is the Plague Counter.
- **Rulings (CL21):** "they destroy all cards" means *their own* cards — the only reading under which
  the Radiant face ("each enemy's turn") is the stronger one.
- **Numbers:** none.

#### Classic #63 · Crop Dusting
`classic-063` · (2) Trap · Common

> **Designer:** Start of Turn: activate. Put a Plague Token on each Permanent. Draw 1.
> *(no Radiant face written)*

- **Text:** Start of turn: Reveal. Place a Plague Counter on each permanent. Draw 1.
- **Radiant (designer, patch v0.2.2):** Start of turn: Reveal. Place 3 Plague Counters on each permanent.
  Draw 3.
- **Engine:** a Trap whose condition is its controller's start of turn (with the start-of-turn
  triggers, R62); it fires once and goes to the graveyard. Every permanent on the field, both sides,
  face-down ones included.
- **Numbers:** tokens 1 ↑; draw 1 ↑.
- **Check:** **R276**: every card needs a Radiant face; the one proposed doubled both numbers (R275)
  until designer patch v0.2.2 set them to 3 and 3.

#### Classic #64 · Malzahar’s Recycler
`classic-064` · (2) Field Spell · Rare

> **Designer:** End of Turn: Discard 2 Whenever you Discard, Draw the same number. ~~~ End of Turn:
> Discard 2 Whenever you Discard, Draw your entire deck.

- **Text:** End of turn: Discard 2 cards.
  Whenever you discard cards, draw that many.
- **Radiant:** End of turn: Discard 2 cards.
  Whenever you discard cards, draw your deck.
- **Engine:** the end-of-turn discard is your choice (R16). The draw answers your `discarded` events one
  effect at a time (an effect that discards 2 draws 2). Radiant: "draw your whole library" (R58) per
  discarding effect.
- **Rulings:** every discard of yours counts: your own, Nose Hunter's random one, Pickle's, Last
  Hurrah's. A card crumbling from Brittle is not a discard (B3.3).
- **Numbers:** none.

#### Classic #65 · Ace in the Hole
`classic-065` · (2) Trap · Common

> **Designer:** End of Turn: Flip a coin. If heads, activate. Recruit a card. ~~~ End of Turn: Flip a
> coin. If tails, Recruit a card. If heads, activate. Recruit 3 cards.

- **Text:** End of your turn: Flip a coin. On heads, this reveals: Recruit a card.
- **Radiant:** End of your turn: Flip a coin. On tails, Recruit a card. On heads, this reveals:
  Recruit 3 cards.
- **Engine:** a check in the end-of-turn trap window of its controller's turns (R62). Heads fires it
  (consumed); E25 recruits permanents from the top of your deck. The Radiant's tails recruits without
  firing, so the trap stays, face-down.
- **Numbers:** recruits 1 ↑ (Radiant 3).

#### Classic #66 · EU Striker
`classic-066` · (2) Unit, Human · Common · 5/4 → 10/8

> **Designer:** 5/4 Summon this from your hand if you play a Unit. If you play a card, Bounce this. ~~~
> 10/8 Rush Summon this from your hand if you play a Unit. If you play a card, Bounce this.

- **Text:** While this is in your hand: After you play a Unit, summon this.
  After you play a card, return this to your hand.
- **Radiant:** Rush
  (the same).
- **Engine:** E26: a hand trigger on your `cardPlayed` of a Unit, after it resolves, summoning this (no
  Cry, summoning sick) into your leftmost open zone; a field trigger on your `cardPlayed` of any card,
  bouncing this (R78's reset).
- **Rulings (CL22):** neither trigger answers the play that moved the card (R119): the Unit that summons
  it doesn't bounce it, and the card that bounces it doesn't summon it back.
- **Numbers:** none.

#### Classic #67 · Felinor Feeler
`classic-067` · (1) Unit, Human · Common · 2/4 → 4/8

> **Designer:** 2/4 Pierce Cry: Set all enemy Units to Defense Position. ~~~ 4/8 Pierce Rush Cry: Set
> all enemy Units to Defense Position.

- **Text:** Pierce
  Cry: Switch every enemy Unit to Defense Position.
- **Radiant:** Pierce, Rush
  (the same).
- **Engine:** an effect switch (no exertion, R20); units already in Defense stay; Spikey Pillow never
  enters Defense.
- **Numbers:** none.
- **Check:** tagged Human, not Felinor, by the designer; kept.

#### Classic #68 · Small Card Lobbyist
`classic-068` · (4) Unit · Common · 11/13 → 22/26

> **Designer:** 11/13 (3)+ cost cards cost (1) more. ~~~ 22/26 Enemy (3)+ cost cards cost can’t be
> played.

- **Text:** Aura: Cost (3)+ cards cost (1) more.
- **Radiant:** Aura: Your opponent can't play Cost (3)+ cards.
- **Engine:** E15 on both players' hands; "(3)+" read where R363 reads Curvature's "(4)+" (before this
  aura adds its 1). Radiant: `legalActions` offers the opponent no play of a card costing 3 or more at
  that moment; casts (R70) are not plays from hand and are unaffected.
- **Numbers:** surcharge 1; threshold 3 ↓.
- **Check:** "cost can't" typo.

#### Classic #69 · Plague Charger
`classic-069` · (2) Unit · Rare · 4/2 → 8/4

> **Designer:** 4/2 Charge If this has a Plague Token on it, First Strike +2 Attack for each Plague
> Token on this ~~ 8/4 Charge If this has a Plague Token on it, First Strike +4 Attack for each Plague
> Token on this

- **Text:** Charge
  Has First Strike while it has a Plague Counter.
  Has +2 Attack for each Plague Counter on it.
- **Radiant:** the same with +4.
- **Engine:** E35's conditional keyword and a self stat layer (§10.4) reading its own
  `counters.plague`.
- **Numbers:** attack per token 2 ↑.
- **Check:** the separator is `~~`, read as `~~~` (§5.3's precedent for Core #15 and #16).

#### Classic #70 · Book of Plague
`classic-070` · (1) Spell, Book · Epic

> **Designer:** Place 5 Plague Tokens. ~~~ Place 10 Plague Tokens.

- **Text:** Place 5 Plague Counters. **Radiant:** Place 10 Plague Counters.
- **Engine:** E19: five (ten) placements, each on a permanent you choose.
- **Numbers:** tokens 5 ↑.
- **Check:** the tag is written "book"; read as Book.

#### Classic #71 · Lane Eater
`classic-071` · (3) Unit · Common · 4/4 → 8/8

> **Designer:** 4/4 Destroy other cards in and Lock this Lane. ~~~ 8/8 Destroy enemy cards in and Lock
> the enemy Lane.

- **Text:** Cry: Destroy every other card in this lane. Lock this lane.
- **Radiant:** Cry: Destroy the enemy cards in this lane. Lock the enemy side of this lane.
- **Engine:** the lane's four zones (§3.1): your backrow zone and both of the opponent's. The top card of
  each is destroyed (a dormant card under a Stack is not on the field, R13, and resumes); then E20 locks
  all four — Lock evicts nothing, so Lane Eater stays and its zone stays Locked after it leaves. Radiant:
  the opponent's two zones only.
- **Rulings:** unlabelled one-time text on a Unit is its Cry.
- **Numbers:** none.

#### Classic #72 · Counterspell (the Rare one)
`classic-072` · (2) Trap · Rare

> **Designer:** Activate when enemy casts any kind of Spell or Trap. Counter that card. ~~~ Activate
> when enemy casts any kind of Spell or Trap. Steal that card.

- **Text:** Reveals when your opponent plays a Spell, Field Spell, Trap or Field Trap: Counter it.
- **Radiant:** Reveals when your opponent plays a Spell, Field Spell, Trap or Field Trap: Steal it.
- **Engine:** E1 on the opponent's announce of any non-Unit, so a Field Spell or a Trap is countered
  before it reaches the backrow (a face-down set is announced by its zone only, but the engine knows
  what it is). Radiant: E2 — countered and moved to your hand as yours.
- **Numbers:** none.
- **Check:** shares its name with #17 (B2.8).

#### Classic #73 · Nurse Cleaver
`classic-073` · (2) Unit · Common · 3/6 → 6/12

> **Designer:** 3/6 Rush Cleave Lifesteal ~~~ 6/12 Charge Cleave Lifesteal

- **Text:** Rush, Cleave, Lifesteal. **Radiant:** Charge, Cleave, Lifesteal.
- **Engine:** keywords only.

#### Classic #74 · Corpse Plantation
`classic-074` · (2) Field Spell · Epic

> **Designer:** Cry: Place 2 Plague Tokens on this. You may spend Plague Tokens on this as mana to cast
> Units from your GY. You must spend at least (1) Plague Token. ~~~ Cry: Place 4 Plague Tokens on this.
> You may spend Plague Tokens on this as mana to cast Units from your GY. You must spend at least (1)
> Plague Token.

- **Text:** Cry: Place 2 Plague Counters on this.
  You may play Units from your graveyard, paying with Plague Counters from this: each token pays (1), and
  each such play spends at least 1 token.
- **Radiant:** the same with 4 Plague Counters.
- **Engine:** E11 for Units, with a second way to pay: the `play` action carries how many tokens pay
  (at least 1, at most the tokens here and the price), the rest in mana. The designer's "cast" here is a
  play: the Unit's Cry fires and it counts as played (not R70's free cast).
- **Numbers:** tokens 2 ↑.

#### Classic #75 · Argusland
`classic-075` · (1) Field Spell · Rare

> **Designer:** Your Hero takes half damage rounded up. ~~~ Your Hero takes quarter damage rounded up.

- **Text:** Aura: Damage to your hero is halved, rounded up.
- **Radiant:** Aura: Damage to your hero is quartered, rounded up.
- **Engine:** E6, after Armor and before the hero caps; several multiply.
- **Numbers:** divisor 2 ↑.

#### Classic #76 · Plague Bringer
`classic-076` · (2) Unit · Common · 4/4 → 8/8

> **Designer:** 4/4 Rush Cry: Place 2 Plague Tokens. Draw 1. ~~~ 8/8 Rush Cry: Place 4 Plague Tokens.
> Draw 2.

- **Text:** Rush
  Cry: Place 2 Plague Counters. Draw 1.
- **Radiant:** the same with 4 Plague Counters and draw 2.
- **Engine:** E19: two (four) placements, your choice each.
- **Numbers:** tokens 2 ↑; draw 1 ↑.

#### Classic #77 · Anti-Magic Monkey
`classic-077` · (2) Unit · Common · 5/5 → 10/10

> **Designer:** 5/5 Stack Spells cost (1) more. ~~~ 10/10 Stack Spells cost (2) more.

- **Text:** Stack
  Aura: Spells cost (1) more.
- **Radiant:** the same with (2).
- **Engine:** E15 on both players' Spells (the Spell type).
- **Numbers:** surcharge 1.

#### Classic #78 · Mutate Spell
`classic-078` · (1) **Field Spell** (the designer wrote Spell) · Rare

> **Designer:** Active ♾️: Consume a Plague Token on a card for the following effect: Enemy card:
> Exile it. · Ally Backrow: Draw 2. · Ally Unit: Attack a random enemy. ~~~ Activate ♾️: Consume a
> Plague Token on a card for the following effect: Enemy card: Fuse it with a valid card in your
> Field, hand, or Library (Exile otherwise). · Ally Backrow: Draw 4. · Ally Unit: Attack a random
> enemy twice.

- **Text:** Activate ♾️: Remove a Plague Counter from a permanent. If it's an enemy permanent, exile it.
  If it's your backrow card, draw 2. If it's your Unit, it attacks a random enemy.
- **Radiant:** Activate ♾️: Remove a Plague Counter from a permanent. If it's an enemy permanent, fuse it
  onto a card of yours of its type on your field, in your hand or in your deck, or exile it if you
  have none. If it's your backrow card, draw 4. If it's your Unit, it attacks a random enemy twice.
- **Engine:** B3.2 Activate ♾️ with a declared target (a permanent with a token). A forced attack (R53)
  on a random enemy (hero or unit). The Radiant's fuse is E23 (R77; the enemy card ceases to exist)
  onto the card you choose: the designer's "a valid card" is a choice, a prompt over your cards of its
  type on the field, in your hand and in your deck, the deck's shown to you only (§10.8, as Core #51
  shows library cards).
- **Rulings (CL23):** an Activate ability needs a card that stays on the field, so the card is read as a
  Field Spell, as §5.3 corrected Core #68's type. **⚠ designer:** the other reading is a Spell whose one
  resolution lets you consume tokens again and again until you stop.
- **Numbers:** draw 2 ↑; attacks 1 ↑.
- **Check:** "Active" typo for Activate.

#### Classic #79 · Risky Die
`classic-079` · (1) Spell · Common

> **Designer:** Draw 3 Reduce their cost by (1). Exile all that cost more than (0). ~~~ Draw 3 Reduce
> their cost by (1). Exile all that cost more than (1).

- **Text:** Draw 3. They cost (1) less. Then exile each of them that costs (1) or more.
- **Radiant:** Draw 3. They cost (1) less. Then exile each of them that costs (2) or more.
- **Engine:** the cards the three draws put in your hand (a cast-on-draw card never gets there; a burned
  one isn't there): `costMod −1` each, then exile those whose hand cost is above 0 (Radiant: above 1).
- **Numbers:** draw 3 ↑; kept threshold 0 ↑.

#### Classic #80 · BOOM! Big Max
`classic-080` · (4) Unit · Legendary · 26/8 → 26/16

> **Designer:** 26/8 Tribute 2 Rush Trample Indestructible ~~~ 26/16 Tribute 2 Rush Trample
> Indestructible

- **Text:** Tribute 2, Rush, Trample, Indestructible. **Radiant:** the same.
- **Engine:** keywords only; Tribute 2 (B4.5 applies); no Taunt ever (R347).
- **Check:** **R275**: the Radiant attack is not doubled, and as a keyword-only Unit its Radiant also
  owes one more keyword or a stronger one. Register a named exception in the radiant-standard test
  (the designer's number stands, as in v0.1.1) or print 52/16, and add a rider either way (proposed:
  Charge for Rush, as R276 gave Core #95.1). **⚠ designer.**

#### Classic #81 · The Power to Thrive
`classic-081` · (2) Field Spell · Rare

> **Designer:** Active: Choose one: You heal 3 · Draw 1 · +1 Mana ~~~ Active: Choose one: You heal 6 ·
> Draw 2 · +2 Mana

- **Text:** Activate: Choose one: Heal your hero 3; draw 1; or gain 1 mana.
- **Radiant:** Activate: Choose one: Heal your hero 6; draw 2; or gain 2 mana.
- **Engine:** B3.2 Activate, the mode declared in the action; temporary mana.
- **Numbers:** heal 3 ↑; draw 1 ↑; mana 1 ↑.
- **Check:** "Active" typo.

#### Classic #82 · Sheeople
`classic-082` · (1) Unit · Common · 1/1 → 2/2

> **Designer:** 1/1 Death: Draw 2 Counts as 2 Tribute ~~~ 2/2 Death: Draw 3 Counts as 3 Tribute

- **Text:** Worth 2 Tributes.
  Death: Draw 2.
- **Radiant:** Worth 3 Tributes.
  Death: Draw 3.
- **Engine:** the Sheep Token's `tributeWorth` (§7), 2 and 3. A Tribute is a death (§6.2), so the draw
  happens when it is tributed.
- **Numbers:** draw 2 ↑; worth 2 ↑.

#### Classic #83 · Flame Lance
`classic-083` · (3) Spell · Common

> **Designer:** Trample Deal 11 damage to a Unit. ~~~ Trample Deal 22 damage to a Unit.

- **Text:** Trample
  Deal 11 damage to a Unit.
- **Radiant:** the same with 22.
- **Engine:** E6's Trample on a Spell.
- **Numbers:** damage 11 ↑ (step 2).

#### Classic #84 · Lockdown
`classic-084` · (2) Field Spell · Rare

> **Designer:** Indestructible Aura: When a Permanent is cast, Lock that space Active: Tribute this ~~~
> Indestructible Aura: When an enemy Permanent is cast, Lock that space Active: Tribute this

- **Text:** Indestructible
  After a permanent is played, Lock its zone.
  Activate: Tribute this.
- **Radiant:** Indestructible
  After your opponent plays a permanent, Lock its zone.
  Activate: Tribute this.
- **Engine:** a trigger on `cardPlayed` of a permanent (either player's; the Radiant's: the opponent's)
  that Locks the zone it entered (E20); the card stays. The Activate sacrifices this, which bypasses
  Indestructible (§6.3).
- **Rulings:** "cast" is the designer's "played" (a cast by an effect counts, R70); a summon that is no
  play (a token, a Recruit) does not.
- **Numbers:** none.

#### Classic #85 · King Wagtoggle
`classic-085` · (4) Unit · Legendary · 5/5 → 10/10

> **Designer:** 5/5 Cry: Swap Decks with the enemy. ~~~ 10/10 Cry: Swap Decks with the enemy. Recruit a
> card.

- **Text:** Cry: Swap decks with your opponent.
- **Radiant:** Cry: Swap decks with your opponent. Then Recruit a card.
- **Engine:** R73's library swap (owners change, fatigue counters stay); the Radiant recruits from your
  new deck (E25).
- **Numbers:** Radiant recruits 1 ↑.

#### Classic #86 · Genn
`classic-086` · (4) Unit · Common · 14/14 → 14/14 (proposed 42/42)

> **Designer:** 14/14 ~~~ 14/14

- **Text:** none. **Radiant:** none.
- **Engine:** a vanilla Unit, like Core #8 Mr. Vanilla.
- **Check:** **R276/R275**: identical faces, which R276's test refuses. Proposed Radiant 42/42: a
  vanilla Unit has no text to scale, so it follows Core #8 Mr. Vanilla, whose Radiant face triples its
  stats (4/4 → 12/12, `docs/radiant-audit.md`); 28/28 would meet only the stat half.
  **⚠ designer.**

#### Classic #87 · Plague Chalice
`classic-087` · (X) Field Spell · Epic

> **Designer:** Enters with X Plague Tokens on it. Counter all cards with cost equal to the number of
> Plague Tokens on this. ~~~ Enters with X Plague Tokens on it. Counter all enemy cards with cost equal
> to the number of Plague Tokens on this.

- **Text:** This enters with X Plague Counters on it.
  Aura: Counter every card played whose cost equals the number of Plague Counters on this.
- **Radiant:** This enters with X Plague Counters on it.
  Aura: Counter every card your opponent plays whose cost equals the number of Plague Counters on this.
- **Engine:** X is at least 1 (R348). E1 on every announce whose cost paid equals the current count
  (both players'; the Radiant's: the opponent's). The count moves: Mutate Spell consumes tokens, other
  cards add them. It is not on the field during its own announce, so it never counters itself.
- **Rulings:** "cost" is the cost paid, as Core #60 reads it, so a free cast (R70) is countered only at
  a count of 0.
- **Numbers:** none.
- **Check:** Magic's Chalice of the Void.

#### Classic #88 · Siphon Squad
`classic-088` · (2) Field Trap · Rare

> **Designer:** Enemy Units have -X attack where X is twice the number of Units they control. Tribute
> this if an enemy ever has 0 Units. ~~~ Enemy Units have 0 attack. Tribute this if an enemy ever has 0
> Units.

- **Text:** Aura: Enemy Units have −X Attack, where X is twice the number of Units your opponent
  controls.
  When your opponent controls no Units, Tribute this.
- **Radiant:** Aura: Enemy Units have 0 Attack.
  When your opponent controls no Units, Tribute this.
- **Engine:** a layer-5 aura (the Radiant's "0 Attack" sets attack last, after every other layer); the
  self-Tribute is a condition checked at every state check, the one right after it is set included.
- **Rulings (CL24):** a Trap or Field Trap whose text has no activation condition is live while
  face-down: its aura works from the moment it is set, and it stays face-down until something reveals
  it. The opponent sees their attack drop and can deduce the card; R33 hides identity, not
  consequences.
- **Numbers:** multiplier 2 ↑.

#### Classic #89 · Paul Allen’s Ghost
`classic-089` · (2) Unit · Rare · 5/6 → 10/12

> **Designer:** 5/6 Divine Shield As an additional cost to target this with anything but an attack,
> Discard 2. ~~~ 10/12 Divine Shield Reborn As an additional cost to target this with anything but an
> attack, Discard 2.

- **Text:** Divine Shield
  To target this with anything but an attack, a player must also discard 2 cards.
- **Radiant:** Divine Shield, Reborn
  (the same).
- **Engine:** E5's targeting point: a declared target (a play or an activation) naming it carries a
  2-card discard pick in the action, as a Tribute carries its paying set (R101); a prompt answer naming
  it asks for the 2 cards next. With fewer than 2 other cards in hand it is not a legal target. It binds
  both players, its controller included.
- **Numbers:** discard 2 ↑.

#### Classic #90 · In Too Deep
`classic-090` · (1) Field Spell, Quickdraw · Mythic

> **Designer:** Indestructible · Go on a quest! · Quests — Quest 1: Draw 2 cards · Quest 2: Destroy 2
> cards · Quest 3: Control 3 cards · Quest 4: Float 3 mana · Quest 5: Deal 12 damage · Quest 6: Control
> 10 attack and 10 health work of stats · Quest 7: Float 5 mana · Quest 8: Exile 3 cards · Quest 9:
> Draw your entire Deck · Quest 10: Have 6 Units in your GY · Rewards — Reward A: Heal 6 · Reward B:
> Deal 3 damage · Reward C: Return 2 random cards from your GY to your hand · Reward D: Place 3 Plague
> Tokens · Reward E: Give a random friendly Unit +3/+3 · Reward F: Bounce a card · Reward G: Enemy
> Discards 2 · Reward H: Draw 2 · Reward I: Recruit a card · Reward J: Gain 100 mana · Reward K: Exile
> the enemy Deck · Reward L: Aura: You may play cards from the GY · Reward M: Aura: Your Units have
> Indestructible · Progression paths — Quest 1 => Reward A or B · Quest 2 => Reward C or D · Quest 3
> => Reward D or E · Quest 4 => Reward F or G · Quest 5 => Reward G or H · Quest 6 => Reward H or I ·
> Quest 7 => Reward J · Quest 8 => Reward K · Quest 9 => Reward L · Quest 10 => Reward M · Reward A =>
> Quest 2 · Reward B => Quest 3 · Reward C => Quest 4 · Reward D => Quest 5 · Reward E => Quest 6 ·
> Reward F => Quest 7 · Reward G => Quest 8 · Reward H => Quest 9 · Reward I => Quest 10 ~~~ Radiant:
> You don’t have to choose a path, you get them all. {after completing a quest, choose a reward, which
> also sends you down another quest path. when actually in game, only shows the active quest and the
> current reward options}

- **Text:** Indestructible
  Quest: Draw 2 cards. Each quest you complete offers rewards; the reward you choose sets your next
  quest.
- **Radiant:** Indestructible
  Quest: Draw 2 cards. Each quest you complete gives every reward it offers, and you follow every path.
- **The tree** (data in the card file, so it is testable):

  | Quest | Done when, counted from the moment it opens | Rewards → next quest |
  | --- | --- | --- |
  | 1 | you have drawn 2 cards | A → 2, B → 3 |
  | 2 | 2 enemy permanents have been destroyed | C → 4, D → 5 |
  | 3 | you control 3 or more permanents at once | D → 5, E → 6 |
  | 4 | you end a turn with 3 or more unspent mana | F → 7, G → 8 |
  | 5 | your cards have dealt 12 damage to enemies in total | G → 8, H → 9 |
  | 6 | your Units have 10 or more total Attack and 10 or more total health at once | H → 9, I → 10 |
  | 7 | you end a turn with 5 or more unspent mana | J (end) |
  | 8 | 3 cards have been exiled (either exile pile) | K (end) |
  | 9 | a draw of yours takes the last card of your deck (a deck already empty when the quest opens completes it at once) | L (end) |
  | 10 | your graveyard holds 6 or more Units | M (end) |

  Rewards: **A** heal your hero 6 · **B** deal 3 damage (a target) · **C** return 2 random cards from
  your graveyard to your hand · **D** place 3 Plague Counters (E19) · **E** a random Unit of yours gets
  +3/+3 · **F** bounce a permanent (a target) · **G** your opponent discards 2 (their choice) · **H**
  draw 2 · **I** Recruit a card · **J** gain 100 mana this turn (temporary, §2.3) · **K** exile your
  opponent's deck · **L** Aura: you may play cards from your graveyard (E11) · **M** Aura: your Units
  have Indestructible.
- **Engine (E33):** `memory.quest = { active, progress, done, auras }`. A quest's progress counts events
  from the moment it opens; completion is noticed at the state check after the event that completes it,
  on either player's turn, and the reward choice is a prompt for the card's controller then (a
  non-active player's prompt if it is the opponent's turn). J to M end a line. The auras L and M hold
  while In Too Deep is on the field (Indestructible, so usually the rest of the game; a Tribute or an
  exile ends them). The view shows the open quest or quests with their progress ("1/2") and the rewards
  on offer, which is what the designer asked the board to show; the collection prints the whole tree.
  Radiant: each completed quest grants all of its rewards and opens all of their quests; a quest reached
  by two paths opens once, and a reward two completed quests offer (D, G, H) is granted by each.
- **Rulings (CL25):** the quest wording above makes each condition countable: "Destroy 2 cards" counts
  enemy permanents destroyed by anything, "Deal 12 damage" counts damage your cards deal to enemies,
  "Exile 3 cards" counts cards entering either exile, "Float" is unspent mana at the end of your turn,
  and "Draw your entire Deck" is done by the draw that empties it, never by cards leaving it any other
  way.
  **⚠ designer**, especially quest 2 (anything, or only your own effects?).
- **Numbers:** none.
- **Check:** "work of stats" is "worth of stats". Quickdraw: it starts in your opening hand (§2.1).

---
## B7. Classic+, card by card

Seventy-eight cards, `classicplus-001` to `classicplus-078`, numbered as B2.3 proposes, each token
after the card that defines it. The designer's number is given where B2.3 moved a card.

#### Classic+ #1 · Doom Shroom
`classicplus-001` · (3) Trap · Epic

> **Designer:** Activate when your Hero is attacked: Exile all Units. Lock this slot. ~~~ Activate when
> your Hero is attacked: Exile all enemy Units. Lock this slot.

- **Text:** Reveals when an enemy Unit attacks your hero: Exile all Units. Lock this zone.
- **Radiant:** Reveals when an enemy Unit attacks your hero: Exile all enemy Units. Lock this zone.
- **Engine:** §4.2 step 4's trap window (My Pawn's). Exile every Unit on the field (tops of piles; a
  card dormant beneath resumes). The attacker is gone, so no combat happens (R44's cancel). The trap goes
  to the graveyard as it fires, and E20 Locks the backrow zone it stood in.
- **Numbers:** none.

#### Classic+ #2 · Groom Shroom
`classicplus-002` · (3) Trap, Felinor · Epic

> **Designer:** Activate when your Hero is attacked: Fill your board with random Felinors. Give them
> Taunt. ~~~ Activate when your Hero is attacked: Fill your board with random Radiant Felinors. Give
> them Taunt.

- **Text:** Reveals when an enemy Unit attacks your hero: Fill your board with random Felinor Units.
  Give them Taunt.
- **Radiant:** … random Radiant Felinor Units …
- **Engine:** the same window. "Fill your board" (R64): one random non-token Felinor Unit of any set
  per empty, unlocked unit zone (repeats allowed, R60), summoned (no Cry), each granted Taunt.
- **Rulings (CL26):** "Felinors" are Felinor-tagged *Units* (a noun for the creatures; Spells can't fill
  a board), here and on Felinor Fuser. The declared attack still hits your hero: the Taunts arrived after
  §4.2 step 3 (Hearthstone's order). **⚠ designer:** redirect the attack to one of them?
- **Numbers:** none.

#### Classic+ #3 · Second Amendment Snake
`classicplus-003` · (2) Unit · Rare · 1/6 → 2/12

> **Designer:** 1/6 End of Turn: Gain 2 Plague Tokens Death: Deal 1 damage split among enemies for each
> Plague Token on this. ~~~ 2/12 End of Turn: Gain 3 Plague Tokens Death: Deal 1 damage split among
> enemies for each Plague Token on this.

- **Text:** End of turn: Place 2 Plague Counters on this.
  Death: Deal 1 damage to a random enemy for each Plague Counter on this.
- **Radiant:** the same with 3 Plague Counters.
- **Engine:** one placement of 2 (3) on itself (E19). Death reads its last-known tokens (R78) and makes
  E37's random split: that many 1-damage hits, each on a random enemy (the hero or a unit) still
  standing.
- **Numbers:** tokens per turn 2 ↑; damage per token 1 ↑.

#### Classic+ #4 · Juhan Biggest Bat
`classicplus-004` · (3) Unit, CN · Common · 9/6 → 18/12

> **Designer:** 9/6 Stack Cards under this are transformed into copies of this. ~~~ 18/12 Stack First
> Strike Cards under this are transformed into copies of this.

- **Text:** Stack
  When this is played onto a pile, the cards beneath it become copies of this.
- **Radiant:** Stack, First Strike
  (the same).
- **Engine:** E24 on arrival: every dormant card beneath it is transformed (§6.3; Immutable ones stay)
  into a copy of this card, with its face (a Radiant Juhan makes Radiant copies). The copies keep the old
  cards' owners and controllers (Replace, §6.3) and stay dormant; when the top leaves, the next Juhan
  resumes.
- **Numbers:** none.

#### Classic+ #5 · Guy Att
`classicplus-005` · (2) Unit, Human · Common · 6/8 → 12/16

> **Designer:** 6/8 Cry: Destroy all your backrow. ~~~ 12/16 Cry: Destroy ALL backrow.

- **Text:** Cry: Destroy every backrow card you control.
- **Radiant:** Cry: Destroy every backrow card.
- **Engine:** face-down cards included; Indestructible ones stay.
- **Numbers:** none.

#### Classic+ #6 · Wrong-House Attacker
`classicplus-006` · (1) Unit, Human · Common · 1/1 → 2/2

> **Designer:** 1/1 Rush, Lifesteal, Poisonous ~~~ 2/2 Rush, Lifesteal, Poisonous, Reborn

- **Text:** Rush, Lifesteal, Poisonous. **Radiant:** Rush, Lifesteal, Poisonous, Reborn.
- **Engine:** keywords only. The evil twin of Core #3 Right-house defender (#7 The House summons both).

#### Classic+ #7 · The House
`classicplus-007` · (3) Field Spell · Rare

> **Designer:** Cry and start of turn summon either a Right-House Protector (⅔) or Wrong House Attacker
> (⅓) ~~~ Radiant: both

- **Text:** Cry and start of turn: Summon a Right-house defender (2 in 3) or a Wrong-House Attacker
  (1 in 3).
- **Radiant:** Cry and start of turn: Summon a Right-house defender and a Wrong-House Attacker.
- **Engine:** `rng.chance(2/3)`; the Units are generated from the catalog (Core #3, Classic+ #6), yours,
  summoned (no Cry); being cards rather than tokens, they go to your graveyard when they die.
- **Rulings (CL27):** "Right-House Protector" is Core #3 Right-house defender, the Wrong-House
  Attacker's twin; ⅔ and ⅓ are the odds (they sum to 1). **⚠ designer** if a new token was meant.
- **Numbers:** none.

#### Classic+ #8 · Withering Storm
`classicplus-008` · (2) Spell · Rare

> **Designer:** Degrade 4 random cards in your opponents deck Draw 1 ~~~ Degrade all cards in your
> opponents deck Draw 1

- **Text:** Degrade 4 random cards in your opponent's deck. Draw 1.
- **Radiant:** Degrade every card in your opponent's deck. Draw 1.
- **Engine:** B3.4, four different cards (R60). Nobody sees the changes until the cards leave the deck
  (R311).
- **Numbers:** cards 4 ↑; draw 1 ↑.

#### Classic+ #9 · Silence
`classicplus-009` · (0) Spell · Common

> **Designer:** Vanilla a Unit ~~~ Vanilla a Permanent

- **Text:** Vanilla a Unit. **Radiant:** Vanilla a permanent.
- **Engine:** §6.3 Vanilla (blocked by Immutable, R23). The Radiant face reaches backrow cards.
- **Rulings (CL28):** a Vanilla backrow card has no text: a Field Spell's aura stops, a face-down trap
  can never fire and sits there as an inert card, an Animated card loses Animated where it stands. As on
  a unit, granted keywords stay (§10.4).
- **Numbers:** none.

#### Classic+ #10 · New Wraps
`classicplus-010` · (0) Spell · Common

> **Designer:** Give a Unit Reborn. ~~~ Give a Unit Reborn. Make it Radiant.

- **Text:** Give a Unit Reborn. **Radiant:** Give a Unit Reborn. Make it Radiant.
- **Engine:** a granted keyword (R21 already grants Reborn); Make Radiant (§5.2).
- **Numbers:** none.

#### Classic+ #11 · Anime Armor
`classicplus-011` · (2) Unit · Rare · 4/4 → 8/8

> **Designer:** 4/4 Your Hero can only take up to 1 damage at a time. ~~~ 8/8 Reborn Year Hero can
> only take up to 1 damage at a time.

- **Text:** Aura: Your hero can't take more than 1 damage at a time.
- **Radiant:** Reborn
  (the same).
- **Engine:** E6's per-hit cap (the lowest cap wins beside Anti-oneshot Armor's).
- **Numbers:** cap 1 ↓ (never below 1).
- **Check:** "Year" is "Your".

#### Classic+ #12 · The Mother Pancake
`classicplus-012` · (3) Unit, Pancake · Legendary · 8/8 → 16/16

> **Designer:** 8/8 Taunt End of turn: Add a Pancake card tokens to your hand ~~~ 16/16 Taunt End of
> turn: Add 2 Pancake card tokens to your hand

- **Text:** Taunt
  End of turn: Add a random Pancake token to your hand.
- **Radiant:** Taunt
  End of turn: Add 2 random Pancake tokens to your hand.
- **Engine:** the pool is the eight Pancake tokens below (named by the text); repeats allowed (R60).
- **Numbers:** tokens 1 ↑.
- **Check:** Hearthstone's The Lich King; the tokens mirror his Death Knight cards (Obliterate, Death
  Coil, Death Grip, Death and Decay, Anti-Magic Shell, Doom Pact, Army of the Dead, Frostmourne).

The eight Pancake tokens (`classicplus-012-1` … `-8`), all Legendary by the designer, `rarity: "Token"`:

- **#12.1 Devour** · (0) Spell, Pancake.
  > **Designer:** Destroy a Unit. Take damage equal to its remaining health. ~~~ Destroy a Unit. Gain
  > health equal to its remaining health.
  - **Text:** Destroy a Unit. Your hero takes damage equal to its health.
    **Radiant:** Destroy a Unit. Heal your hero by its health.
  - **Engine:** its health read before the destroy; the self-hit is one instance from Devour (your Armor
    applies); the Radiant's gain is a heal. An Indestructible target survives and the damage still
    happens (Hearthstone's Obliterate).
  - **Numbers:** none.
- **#12.2 Death Boil** · (1) Spell, Pancake.
  > **Designer:** Target a Unit or Hero. Deal 6 damage if an enemy and Heal 6 if an ally. ~~~ Target a
  > Unit or Hero. Deal 12 damage if an enemy and Heal 12 if an ally.
  - **Text:** Choose a Unit or hero. If it's an enemy, deal 6 damage to it. If it's yours, heal it 6.
    **Radiant:** 12 and 12.
  - **Numbers:** amount 6 ↑ (step 2).
- **#12.3 Fluffy Grip** · (1) Spell, Pancake.
  > **Designer:** Steal a Unit from your opponent's deck Deck add it to your hand. It costs (0). ~~~
  > Steal a Unit from your opponent's Deck and add it to your hand. It costs (0) and becomes Radiant.
  - **Text:** Steal a random Unit from your opponent's deck and put it in your hand. It costs (0).
    **Radiant:** … It costs (0) and becomes Radiant.
  - **Engine:** E2/E16: a random Unit card from their deck becomes yours, in your hand, `costOverride 0`.
  - **Check:** "deck Deck" typo.
- **#12.4 Powder Spray** · (1) Spell, Pancake.
  > **Designer:** Deal 3 damage to all enemies. ~~~ Deal 6 damage to all enemies.
  - **Text:** Deal 3 damage to each enemy. **Radiant:** 6.
  - **Engine:** one hit each on the enemy hero and every enemy unit, all landing before the check (R59).
  - **Numbers:** damage 3 ↑.
- **#12.5 Anti-Waffle Shell** · (1) Field Spell, Pancake.
  > **Designer:** Cry: Give your Units Divine Shield Aura: Your Units have +2/+2 ~~~ Cry: Give your Units
  > Divine Shield Aura: Your Units have +4/+4
  - **Text:** Cry: Give your Units Divine Shield.
    Aura: Your Units have +2/+2.
    **Radiant:** +4/+4.
  - **Numbers:** aura 2 ↑.
- **#12.6 Frozen Wastes** · (2) Field Spell, Pancake.
  > **Designer:** Destroy all Units. Exile a card from your Deck for each one. ~~~ Destroy all Units.
  > Your opponent Exiles a card from their Deck for each one.
  - **Text:** Cry: Destroy all Units. Exile the top card of your deck for each one destroyed.
    **Radiant:** Cry: Destroy all Units. Exile the top card of your opponent's deck for each one
    destroyed.
  - **Engine:** "a card" is the top card, as Hearthstone's Doom Pact removes it; "each one" counts the
    Units that died (Indestructible survivors don't count).
  - **Rulings (CL29):** a Field Spell's unlabelled one-time text is its Cry; the card then stays in the
    backrow with no further text (a body for backrow effects to hit). **⚠ designer:** Spell meant?
    (Frozen Wastes and Legion of the Hungry are Spells in Hearthstone.)
- **#12.7 Legion of the Hungry** · (2) Field Spell, Pancake.
  > **Designer:** Exile 5 random cards from your Deck. Summon any Units among them. ~~~ Exile 5 random
  > cards from your Deck. Summon any Units among them. Make them Radiant.
  - **Text:** Cry: Exile 5 random cards from your deck. Summon the Units among them.
    **Radiant:** … Summon the Units among them and make them Radiant.
  - **Engine:** five different cards (R60); the Units are summoned out of exile (no Cry) until the board
    is full; the rest stay exiled. CL29 applies.
  - **Numbers:** cards 5 ↑.
- **#12.8 Frostspatula** · (2) Field Spell, Pancake · 10/3 → 20/6.
  > **Designer:** 10/3 Animated on your turn Rush Death: Resummon all Units destroyed by this ~~~ 20/6
  > Animated on your turn Rush Death: Resummon all Units destroyed by this. Make them Radiant.
  - **Text:** Animated on your turn, Rush
    Death: Summon a copy of every Unit this destroyed.
    **Radiant:** … and make them Radiant.
  - **Engine:** B3.1 (the evasion card). It remembers each unit it killed (R42's killer) as
    `{ defId, radiant }`; its Death — as a Unit, or destroyed in the backrow (Death is leaving the field
    for a graveyard, §6.2) — summons a fresh copy of each for you, tokens included, until your board is
    full.
  - **Rulings (CL30):** "resummon" makes copies, as Hearthstone's Frostmourne does; the originals stay in
    their owners' graveyards.

#### Classic+ #13 · Mommy Barker
`classicplus-013` · (1) Unit, Human, Pancake · Legendary · 2/2 → 4/4

> **Designer:** 2/2 Death: Add a Pancake card token to your hand ~~~ 4/4 Reborn Death: Add a Pancake
> card token to your hand

- **Text:** Death: Add a random Pancake token to your hand.
- **Radiant:** Reborn
  (the same; Death fires on both deaths, §4.5).
- **Numbers:** tokens 1 ↑.

#### Classic+ #14 · Forever&
`classicplus-014` · (1) Spell · Epic

> **Designer:** Then next spell you play gains “When this leaves your hand, add it right back to your
> hand (it can’t cost less than (2) mana)” ~~~ Then next spell you play gains “When this leaves your
> hand, add it right back to your hand (it can’t cost less than (1) mana)” Draw 1 [fix wording]

- **Text (the fixed wording asked for):** The next Spell you play gains "After this resolves, return it
  to your hand. This can't cost less than (2)."
- **Radiant:** The next Spell you play gains "After this resolves, return it to your hand. This can't
  cost less than (1)." Draw 1.
- **Engine:** a player modifier, waiting until used (not turn-scoped), that stamps E39's enchantment on
  the next Spell as it is played. The enchantment rides the card in every zone, so it comes back every
  time; its floor applies after every discount (E15).
- **Rulings (CL31):** "leaves your hand" is read as "is played and resolves": a discarded or countered
  copy does not come back (that would make it undiscardable and uncounterable, and the price floor shows
  the aim was a repeatable Spell, not an indestructible one). **⚠ designer.**
- **Numbers:** floor 2 ↓ (never below 1); Radiant draw 1 ↑.
- **Check:** "Then next" is "The next".

#### Classic+ #15 · Conjure Rush Token
`classicplus-015` · (1) Spell · Common

> **Designer:** Summon a 3/3 rush token, it gains a random keyword ~~~ Summon 3 of them

- **Text:** Summon a Rush Token. It gains a random keyword.
- **Radiant:** Summon 3 Rush Tokens. Each gains a random keyword.
- **Engine:** Core's Rush Token (3/3 Rush); R21's pool, never a keyword it has.
- **Numbers:** keywords 1 ↑.

#### Classic+ #16 · Conjure Rush Token+
`classicplus-016` · (2) Spell · Rare

> **Designer:** Summon a 3/3 rush token, it gains 3 random keywords ~~~ Summon 3 of them

- **Text:** Summon a Rush Token. It gains 3 random keywords.
- **Radiant:** Summon 3 Rush Tokens. Each gains 3 random keywords.
- **Numbers:** keywords 3 ↑.

#### Classic+ #17 · Conjure Rush Token++
`classicplus-017` · (4) Spell · Epic

> **Designer:** (4) Conjure Tush Token++, spell, Epic · Summon a 3/3 rush token, it gains 3 random
> keywords ~~~ Summon 3 of them

- **Text (proposed):** Summon a Rush Token. It gains 6 random keywords.
- **Radiant (proposed):** Summon 3 Rush Tokens. Each gains 6 random keywords.
- **Check:** the text is #16's word for word at twice the cost and a rarity up; 6 keywords (twice #16's)
  is proposed. "Tush" is "Rush". **⚠ designer.**
- **Numbers:** keywords 6 ↑.

#### Classic+ #18 · Gullible Treatler
`classicplus-018` · (2) Unit, Human · Common · 8/9 → 16/18

> **Designer:** 8/9 Start of Turn: Tribute this if you don’t control a Field Spell or Trap ~~~ 16/18
> Start of Turn: Tribute this if no player controls a Field Spell or Trap

- **Text:** Start of turn: If you control no Field Spell or Trap, Tribute this.
- **Radiant:** Start of turn: If no player controls a Field Spell or Trap, Tribute this.
- **Engine:** at its controller's start of turn; Field Traps count as Traps, face-down cards count.
- **Numbers:** none.

#### Classic+ #19 · League of Losers
`classicplus-019` · (4) Spell · Legendary

> **Designer:** Summon the five stack. Summon Top Loser, Jungle Loser, Mid Loser, Support Loser, and
> Bot Loser into your Unit Zones 1–5, in that order. (Occupied zones are skipped.) ~~~ Summon the
> Radiant five stack. Summon Radiant Top Loser, Radiant Jungle Loser, Radiant Mid Loser, Radiant
> Support Loser, and Radiant Bot Loser into your Unit Zones 1–5, in that order. (Occupied zones are
> skipped.)

- **Text:** Summon the five-stack: Top Loser, Jungle Loser, Mid Loser, Support Loser and Bot Loser, into
  your unit zones 1 to 5 in that order. An occupied or Locked zone is skipped.
- **Radiant:** the same, all five Radiant.
- **Engine:** five summons aimed at named zones (§3.2: an aimed summon fails when its zone is occupied
  or Locked).
- **Rulings (CL32):** Mid Loser's Cry fires when League of Losers summons it. That is its only way onto
  the field but one (Dropshipping can hand a Loser out as a card, and a Loser played from hand fires its
  Cry under R1 like any card), so under R1 alone its Cry would be nearly dead text; the exception is a
  summon that names the card it summons. **⚠ designer:** or reword Mid Loser's Cry as "When this is
  summoned:".
- **Check:** League of Legends' five roles.

The five Losers (`classicplus-019-1` … `-5`), Units, Tokens, Legendary by the designer:

- **#19.1 Top Loser** · (2) · 5/5 → 10/10.
  > **Designer:** 5/5 Armor 3 Cannot be in Defense Position. Can only be attacked by Units in this lane. ~~~
  > 10/10 Armor 6 Cannot be in Defense Position. Can only be attacked by Units in this lane, Immune
  > to Spells.
  - **Text:** Armor 3
    Cannot be in Defense Position. Only Units in this lane can attack this.
    **Radiant:** Armor 6, Immune to Spells
    (the same).
  - **Engine:** E35: an attack on it is legal only from the enemy unit zone of the same lane. Immune to
    Spells: a Spell can't target it and doesn't affect it. A Taunt on it binds only attackers that could
    legally attack it (§4.2 step 3 counts the Taunts an attacker may reach).
  - **Numbers:** Armor 3 ↑.
- **#19.2 Jungle Loser** · (2) · 5/5 → 10/10.
  > **Designer:** 5/5 End of Turn: 25% chance to attack a random enemy Unit. If this Destroys the enemy
  > Unit in Bot Loser's lane, Bot Loser goes Berserk. ~~~ 10/10 End of Turn: 50% chance to attack a
  > random enemy Unit. If this Destroys the enemy Unit in Bot Loser's lane, Bot Loser gets the kill
  > instead (trigger its "When this destroys a Unit" effect).
  - **Text:** End of turn: 25% chance: This attacks a random enemy Unit. If it destroys the enemy Unit
    across from your Bot Loser, your Bot Loser goes Berserk.
    **Radiant:** End of turn: 50% chance: This attacks a random enemy Unit. If it destroys the enemy Unit
    across from your Bot Loser, your Bot Loser gets the kill instead (its "Whenever this destroys a Unit"
    triggers).
  - **Engine:** `rng.chance`; a forced attack (R53). "Destroys" is R42's killer. The Radiant's credit
    fires Bot Loser's kill trigger as if Bot Loser had killed it.
  - **Numbers:** chance 25% ↑ (step 10%).
- **#19.3 Mid Loser** · (2) · 5/5 → 10/10.
  > **Designer:** 5/5 Cry: Flip a coin. Heads (Fed): Gain +5/+5. Tails (Int): Gain -3/-3. Your opponent
  > gains 1 mana next turn. ~~~ 10/10 Lucky 1 Cry: Flip a coin. Heads (Fed): Gain +10/+10. Tails (Int):
  > Gain -3/-3. Your opponent gains 1 mana next turn.
  - **Text:** Cry: Flip a coin. Heads (Fed): This gets +5/+5. Tails (Int): This gets −3/−3 and your
    opponent gains 1 mana next turn.
    **Radiant:** Lucky 1
    Cry: … Heads (Fed): +10/+10 …
  - **Engine:** `rng.coin`, Lucky 1 keeping heads (§6.1); permanent buffs; the opponent's
    `nextTurnMod` +1. CL32 makes the Cry fire.
  - **Numbers:** heads 5 ↑; tails 3 ↓.
- **#19.4 Support Loser** · (2) · 0/5 → 0/10.
  > **Designer:** 0/5 Divine Shield End of Turn: Heal your Hero and all your Units by 3. ~~~ 0/10 Divine
  > Shield, Reborn End of Turn: Heal your Hero and all your Units by 6.
  - **Text:** Divine Shield
    End of turn: Heal your hero and each of your Units 3.
    **Radiant:** Divine Shield, Reborn
    … 6.
  - **Numbers:** heal 3 ↑.
- **#19.5 Bot Loser** · (2) · 5/5 → 10/10.
  > **Designer:** 5/5 Rush, First Strike When this destroys a Unit, gain +5 Attack. While Berserk: At the
  > Start and End of Turn, attack your Hero. ~~~ 10/10 Charge, First Strike When this destroys a Unit,
  > gain +10 Attack. Tranquility makes this unit not be able to go Berserk
  - **Text:** Rush, First Strike
    Whenever this destroys a Unit, it gets +5 Attack.
    While Berserk: At the start and end of your turn, this attacks your hero.
    **Radiant:** Charge, First Strike
    Whenever this destroys a Unit, it gets +10 Attack.
    This can't go Berserk.
  - **Engine:** a kill trigger (R42, like Prem Panther's) and a permanent buff. Berserk (E35) is a flag
    set by Jungle Loser, lost when it leaves the field; while it's set, at its controller's start and
    end of turn it makes a forced attack (R53) on its *own* controller's hero (a hero never strikes back).
  - **Rulings (CL33):** "Tranquility makes this unit not be able to go Berserk" is the designer's name for
    the immunity: "This can't go Berserk."
  - **Numbers:** attack gained 5 ↑.

#### Classic+ #20 · Mushroom Power
`classicplus-020` · (1) Unit · Common · 2/2 → 4/4

> **Designer:** 2/2 Cry: Give adjacent Units +2/+2 ~~~ 4/4 Cry: Give adjacent Units +4/+4

- **Text:** Cry: Give the Units next to this +2/+2. **Radiant:** +4/+4.
- **Engine:** §3.1's adjacency (lanes N−1 and N+1 on its side); permanent buffs.
- **Numbers:** buff 2 ↑.

#### Classic+ #21 · Whirlwind
`classicplus-021` · (0) Spell · Common

> **Designer:** Pierce Deal 1 damage to all Units ~~~ Pierce Deal 1 damage to all Units End of turn:
> Return this to your hand

- **Text:** Pierce
  Deal 1 damage to all Units.
- **Radiant:** Pierce
  Deal 1 damage to all Units.
  End of turn: Return this to your hand.
- **Engine:** a Spell's Pierce (R346). The Radiant is Core #23's `returnToHandAtEndOfTurn` (R68's
  graveyard triggers).
- **Numbers:** damage 1 ↑.

#### Classic+ #22 · Blood Moon
`classicplus-022` · (1) Trap · Rare; Radiant face: **Field Trap**

> **Designer:** Activates when an enemy is healed: All enemy healing this turn is converted to Pierce
> damage instead. ~~~ {Becomes a Field Trap} Activates when an enemy is healed: All enemy healing is
> converted to Pierce damage instead.

- **Text:** Reveals when an enemy would be healed: For the rest of this turn, healing on enemies deals
  that much Pierce damage to them instead.
- **Radiant (a Field Trap):** Reveals when an enemy would be healed: From now on, healing on enemies
  deals that much Pierce damage to them instead.
- **Engine:** E5's "would be healed" point and E8. "Enemy" is the enemy hero or an enemy unit; Lifesteal
  is healing; "heal up to" (Reno) and "heal to full" are healing; setting health (E7) is not. The damage
  comes from Blood Moon. The Radiant face's type is its own (B2.7): it stays and keeps converting while
  it is on the field.
- **Rulings (CL34):** the heal that sets it off is converted too ("is healed" read as "would be healed");
  otherwise the first heal always escapes it.
- **Numbers:** none.

#### Classic+ #23 · Dropshipping
`classicplus-023` · (1) Spell, CN · Epic

> **Designer:** Add 3 random cards (including tokens) to your hand. Give them Brittle 2. ~~~ Add 3
> random cards (including tokens) to your hand. Give them Brittle 2 and set their Cost to (1).

- **Text:** Add 3 random cards to your hand. They may be tokens. Give them Brittle 2.
- **Radiant:** Add 3 random cards to your hand. They may be tokens. Give them Brittle 2. They cost (1).
- **Engine:** the pool is every card and token of every set but Dropshipping (B4.1); repeats allowed
  (R60); a unit-token card may sit in a hand (R11's Infinite Reserves case). It is the one pool that
  reaches the Grapes outside Fruit (CL3), the Losers outside League of Losers (CL32) and the AI
  generated cards outside Claude's Datacenter and AI Slop (B8), at their printed cost (Radiant: (1)).
  Brittle per B3.3; `costOverride 1`.
- **Numbers:** cards 3 ↑; Brittle 2 ↑.

#### Classic+ #24 · Crushing Walls
`classicplus-024` · (3) Spell · Epic

> **Designer:** Destroy all cards in each player’s leftmost and rightmost lanes for each player. ~~~
> Destroy all cards in the enemy’s leftmost and rightmost lanes.

- **Text:** Destroy every card in lanes 1 and 5.
- **Radiant:** Destroy every enemy card in lanes 1 and 5.
- **Engine:** §3.1 numbers each side from its owner's seat and faces lane N with lane N, so each
  player's outermost lanes are 1 and 5 on both sides: the top card of those 8 zones (Radiant: the
  enemy's 4).
- **Numbers:** none.

#### Classic+ #25 · Soul Shot
`classicplus-025` · (2) Spell · Common

> **Designer:** Destroy a random enemy Unit ~~~ Lucky 1 Destroy a random enemy Unit

- **Text:** Destroy a random enemy Unit. **Radiant:** Lucky 1 · Destroy a random enemy Unit.
- **Engine:** Lucky 1 (§6.1): two picks, keep the better.
- **Rulings (CL35):** Lucky needs a "best" (§6.1: a per-effect comparator): the unit with the higher
  attack plus current health, then the higher cost, then the lower lane.
- **Numbers:** none.

#### Classic+ #26 · Tommy Tempo
`classicplus-026` · (3) Unit, Human · Common · 9/9 → 18/18

> **Designer:** 9/9 Taunt Cast on Draw: End your turn immediately ~~~ 18/18 Taunt Cast on Draw: You can
> take one more action, then your turn ends

- **Text:** Taunt
  Cast on draw: End your turn.
- **Radiant:** Taunt
  Cast on draw: You may take one more action. Then your turn ends.
- **Engine:** a Unit cast on draw is played for free into your leftmost open unit zone (R70); with no
  zone it goes to your hand uncast, as R58's cap sends one. Then E10 on your turn; drawn on the
  opponent's turn, there is no turn of yours to end and only the summon happens. Drawn at the start of
  your turn, it ends that turn before your main phase: the price of a free 9/9 Taunt.
- **Rulings (CL36):** "an action" is one main-phase action: a play, an attack, a position switch or an
  activation (ending the turn yourself also uses it).
- **Numbers:** Radiant actions 1 ↑.
- **Check:** named like Core #11 Tempo Timmy, a different card.

#### Classic+ #27 · Zephrys Zealotism
`classicplus-027` · (4) Spell · Mythic

> **Designer:** Replace your Hand with the perfect Hand. Refresh your mana. ~~~ Replace your Hand with
> the perfect Radiant Hand. Refresh your mana. (only using cards from Classic & Classic+)

- **Text:** Replace your hand with the perfect hand of Classic and Classic+ cards. Refresh your mana.
- **Radiant:** Replace your hand with the perfect Radiant hand of Classic and Classic+ cards. Refresh
  your mana.
- **Engine:** E34: R29's scorer ranks every non-token Classic and Classic+ card but this one for the
  current state. Each other card in your hand goes to your graveyard (Core #76's reading) and as many
  new cards arrive: the scorer's top distinct cards. Then a Refresh (R364) of your max mana, which gives
  back this card's 4.
- **Rulings (CL37):** the hand keeps its size ("replace"), and "only Classic & Classic+" covers both
  faces, as Zephyrs' "only from the core set" does. **⚠ designer:** a full hand of 10 instead? (With
  only this card in hand it gives nothing.)
- **Numbers:** none.

#### Classic+ #28 · Nuestro hogar, nuestras tumbas
`classicplus-028` · (2) Unit · Common · 3/4 → 6/8

> **Designer:** 3/4 Taunt, Reborn Death: Your hero heals 3 ~~~ 6/8 Taunt, Reborn, Divine Shield Death:
> Your hero heals 8

- **Text:** Taunt, Reborn
  Death: Heal your hero 3.
- **Radiant:** Taunt, Reborn, Divine Shield
  Death: Heal your hero 8.
- **Engine:** Death fires on both deaths of a Reborn unit (§4.5).
- **Numbers:** heal 3 ↑.

#### Classic+ #29 · Portal to the Past
`classicplus-029` · (3) Spell · Epic

> **Designer:** Discover a card from your last game’s board (as it was when that game ended). It costs
> (0). ~~~ Add 3 random cards from your last game’s board (as it was when that game ended) to your
> hand. They cost (0).

- **Text:** Discover a card from the board your last game ended with. It costs (0).
- **Radiant:** Add 3 random cards from the board your last game ended with to your hand. They cost (0).
- **Engine:** E30: each seat's last board is a setup input of `createGame` (beside decks and handicaps)
  and is frozen into the match, so `(seed, decks, handicaps, lastBoards, log)` still replays. It is a
  list of `{ defId, radiant }` — a fused card by its R179 id, which the engine can rebuild — for every
  permanent on the field when that player's previous game ended, both sides, except the opponent's
  face-down cards, which that player never saw. The server stores it per profile when a game ends and
  sends it at the match's start; practice keeps its own on the device; hotseat has none (empty, so the
  card fizzles). Discover offers 3 different entries; the Radiant takes 3 different random ones. A token
  can come back as a card in hand (R11).
- **Rulings (CL38):** "your last game" is your last finished game of the same kind (server matches for
  a server match, practice games for practice); stats, buffs and damage are not carried, only card and
  face. **⚠ designer:** which games count, and does the opponent's side count ("your last game's
  board" could mean yours only)?
- **Numbers:** Radiant cards 3 ↑.

#### Classic+ #30 · Felinor Fuser
`classicplus-030` · (3) Unit, Felinor · Epic · 3/3 → 6/6

> **Designer:** 3/3 Cry: Discover 2 Felinors. Fuse them into this. ~~~ 6/6 Cry: Discover 2 Radiant
> Felinors. Fuse them into this.

- **Text:** Cry: Discover a Felinor Unit, then another. Fuse both into this.
- **Radiant:** Cry: Discover a Radiant Felinor Unit, then another. Fuse both into this.
- **Engine:** two chained Discovers (Stitching's pattern, R352) from non-token Felinor Units of every
  set but this one (B4.1, CL26), then R77's Fuse with this unit as the target on the field: it keeps its
  instance, sums the stats, joins the texts.
- **Numbers:** none.

#### Classic+ #31 · Fusion Lab
`classicplus-031` · (2) Field Spell · Epic

> **Designer:** Cry and End of Turn: Fuse a random card into a card in your hand. Its cost stays the
> same. ~~~ Cry and End of Turn: Fuse a random Radiant card into a card in your hand. Its cost stays the
> same.

- **Text:** Cry and end of turn: Choose a card in your hand. Fuse a random card into it. Its cost doesn't
  change.
- **Radiant:** … Fuse a random Radiant card into it …
- **Engine:** the Cry's hand pick is declared at play (R81), the end of turn's is a prompt; a random
  non-token card of every set but Fusion Lab (B4.1); R77 with the hand card as the kept instance (its
  type wins); `costOverride` of the cost it had (E23).
- **Rulings:** "a card in your hand" has no "random", so you choose. An empty hand does nothing.
- **Numbers:** none.

#### Classic+ #32 · Otherworldly Removal
`classicplus-032` · (2) Spell · Epic

> **Designer:** Add an Execute, a Brawl, and a Blade Storm to your hand. ~~~ Add a Radiant Execute,
> Brawl, and Blade Storm to your hand.

- **Text:** Add an Execute, a Brawl and a Blade Storm to your hand.
- **Radiant:** Add a Radiant Execute, a Radiant Brawl and a Radiant Blade Storm to your hand.

Its three Spell tokens (`classicplus-032-1` … `-3`), Epic by the designer:

- **#32.1 Execute** · (1).
  > **Designer:** Destroy a damaged Unit. ~~~ Destroy all damaged enemy Units.
  - **Text:** Destroy a damaged Unit. **Radiant:** Destroy every damaged enemy Unit.
  - **Engine:** damaged = damage above 0.
- **#32.2 Brawl** · (2).
  > **Designer:** Destroy all Units except one chosen at random. ~~~ Destroy all Units except one of your
  > choice.
  - **Text:** Destroy all Units but one chosen at random. **Radiant:** Destroy all Units but one of your
    choice.
  - **Engine:** the Radiant's pick is declared at play.
- **#32.3 Blade Storm** · (1).
  > **Designer:** Deal 1 damage to all Units. Repeat until a Unit dies (up to 30 times). ~~~ Deal 1
  > damage to all enemy Units. Repeat until a Unit dies (up to 30 times).
  - **Text:** Deal 1 damage to all Units. Repeat until a Unit dies, up to 30 times.
    **Radiant:** … to all enemy Units …
  - **Engine:** each round is one effect list followed by its own state check (an exception to R59's
    one check per list, as R283 is); it stops after a round in which any unit died (a Reborn death
    counts), after 30 rounds (`BLADE_STORM_ROUNDS`), or when no target is left.
  - **Numbers:** rounds 30.

#### Classic+ #33 · Ivory Tower
`classicplus-033` · (2) Field Spell · Rare

> **Designer:** Your cards gain Stack. A Unit may be played on top of this. That Unit can't attack or
> be attacked. ~~~ Your cards gain Stack. A Unit may be played on top of this. That Unit can't attack or
> be attacked, and becomes Radiant.

- **Text:** Aura: Your cards have Stack.
  A Unit may be played on top of this. That Unit can't attack or be attacked.
- **Radiant:** … That Unit can't attack or be attacked, and it becomes Radiant.
- **Engine:** E21. The aura grants Stack to your cards in hand and on the field, so your Units stack on
  your units and your backrow cards on your backrow (backrow piles). A Unit you play may name the
  Tower's zone and top its pile: a Unit in the backrow row that acts (its own text works), is targeted
  and hit by "all Units" effects as usual, and can neither attack nor be attacked.
- **Rulings (CL39):** the Tower stays active beneath the Unit it carries — the one exception to "only the
  top of a pile acts", as R13 makes Felinor Fiender's count one — so its aura and its protection keep
  working, and "all Field Spells" effects still find it. It carries one Unit, and that Unit is a Unit
  for every rule, not a backrow card, so backrow effects (Back Breaker, Crushing Walls) pass it by. If
  the Tower leaves the field, the Unit moves to its controller's unit zone in that lane, or the
  leftmost open one (R64), without leaving the field (no Cry, no reset: R78 doesn't apply); with no
  open unit zone it is destroyed.
- **Numbers:** none.

#### Classic+ #34 · Memory Leak
`classicplus-034` · (3) Field Spell · Epic

> **Designer:** Choose one: End of Turn: Lock a random Zone on your opponent's side. After your opponent
> plays a Field Spell or Unit, Lock that Zone. ~~~ End of Turn: Lock a random Zone on your opponent's
> side. After your opponent plays a Field Spell or Unit, Lock that Zone.

- **Text:** Choose one: "End of turn: Lock a random zone on your opponent's side"; or "After your
  opponent plays a Unit or Field Spell, Lock its zone."
- **Radiant:** End of turn: Lock a random zone on your opponent's side.
  After your opponent plays a Unit or Field Spell, Lock its zone.
- **Engine:** the mode is declared at play and remembered (`memory.mode`). E20. "A random zone" is one
  of their ten zones not already Locked (an occupied one is fine: a Lock evicts nothing).
- **Numbers:** none.

#### Classic+ #35 · Rollback
`classicplus-035` · (4) Spell · Legendary

> **Designer:** Return the board to its state from 1, 2, or 3 turns ago. ~~~ Return the board to its
> state from 1, 2, or 3 turns ago. Choose whether this affects only your side, only your opponent's
> side, or both.

- **Text:** Choose 1, 2 or 3. Return the board to how it was that many turns ago.
- **Radiant:** Choose 1, 2 or 3, and your side, your opponent's side or both. Return that part of the
  board to how it was that many turns ago.
- **Engine (E29):** at the start of every turn, before anything happens, the state keeps a snapshot of
  the field — both sides' piles and backrow zones as full instance data, and their Locks — and holds
  the last four (`state.boardHistory`). "N turns ago" is the snapshot taken at the start of the
  player-turn N before this one. Restoring a side:
  1. each card on that side now that the snapshot doesn't hold leaves the field for its owner's hand
     (a token ceases to exist);
  2. each card the snapshot holds is put back in its zone as it was (damage, buffs, position, counters,
     controller), moved from wherever it is now, or recreated from the snapshot if it no longer exists
     (a token, a card that was transformed or fused);
  3. that side's Locks become the snapshot's.

  Nothing else changes: health, mana, decks and graveyards (bar the cards that moved). No Cry, no Death;
  one `rolledBack` event and the moves.
- **Rulings (CL40):** the three steps above. **⚠ designer**, particularly step 1 (bounced to hand,
  rather than destroyed or removed from the game).
- **Numbers:** none.

#### Classic+ #36 · Conjure Bones
`classicplus-036` · (2 embiggen 4) Spell · Rare

> **Designer:** Shuffle 7 embiggen 17 Bone Storm into your deck. ~~~ Shuffle 7 embiggen 17 Radiant Bone
> Storm into your deck.

- **Text:** Shuffle 7 Bone Storms into your deck.
  Paid (4): 17 instead.
- **Radiant:** Shuffle 7 Radiant Bone Storms into your deck.
  Paid (4): 17 instead.
- **Engine:** embiggen (§6.3, R81); each at a random position; R80's cap (60) turns the rest away.
- **Numbers:** count 7 ↑ (step 2); paid count 17 ↑ (step 4).

- **#36.1 Bone Storm** · (1) Spell · Token, Rare by the designer (`classicplus-036-1`).
  > **Designer:** Cast on Draw: Deal 1 damage to all enemies. ~~~ Echo Cast on Draw: Deal 1 damage to all
  > enemies.
  - **Text:** Cast on draw: Deal 1 damage to each enemy.
    **Radiant:** Echo
    Cast on draw: Deal 1 damage to each enemy.
  - **Engine:** cast-on-draw chains under R58's cap (20).
  - **Numbers:** damage 1 ↑.

#### Classic+ #37 · Wardrum
`classicplus-037` · (5) Unit, Quickdraw · Legendary · 5/5 → 10/10

> **Designer:** 5/5 After you play 3 or more Spells, Field Spells, or Traps in a turn: Summon this from
> your hand or deck. End of Turn: Cast a random Spell, Field Spell, or Trap you played this turn. ~~~
> 10/10 After you play 3 or more Spells, Field Spells, or Traps in a turn: Summon this from your hand or
> deck. End of Turn: Cast all Spells, Field Spells, and Traps you played this turn.

- **Text:** While this is in your hand or deck: After you play your 3rd Spell, Field Spell or Trap in a
  turn, summon this.
  End of turn: Cast a copy of a random Spell, Field Spell or Trap you played this turn.
- **Radiant:** … End of turn: Cast a copy of each Spell, Field Spell and Trap you played this turn.
- **Engine:** E26: a hand-and-deck trigger on your 3rd non-Unit play of the turn (E4), after it
  resolves; summoned (no Cry) into your leftmost open zone. The end of turn casts fresh copies (E12) of
  what `turnLog.playedIds` holds, by definition and face; a Trap cast is set face-down and needs a zone.
- **Rulings:** it fires on the 3rd such play only. Quickdraw puts it in your opening hand, so the deck
  clause matters after a mulligan returns it.
- **Numbers:** threshold 3 ↓ (never below 2).

#### Classic+ #38 · Solarius
`classicplus-038` · (2) Unit · Epic · 3/2 → 6/4

> **Designer:** 3/2 Spell Damage +2 Cry: Draw 1. Death: Shuffle a Solarius-Prime into your deck. ~~~ 6/4
> Spell Damage +5 Cry: Draw 2. Death: Shuffle a Radiant Solarius-Prime into your deck.

- **Text:** Spell Damage +2
  Cry: Draw 1.
  Death: Shuffle a Solarius-Prime into your deck.
- **Radiant:** Spell Damage +5
  Cry: Draw 2.
  Death: Shuffle a Radiant Solarius-Prime into your deck.
- **Engine:** Spell Damage is a new numbered keyword (E6); R80's cap.
- **Numbers:** Spell Damage 2 ↑; draw 1 ↑.
- **Check:** Hearthstone's Astromancer Solarian.

- **#38.1 Solarius-Prime** · (4) Unit · Token, Epic by the designer · 9/5 → 18/10
  (`classicplus-038-1`).
  > **Designer:** 9/5 Spell Damage +3 Cry: Cast 5 random Spells. They target enemies when possible. ~~~
  > 18/10 Spell Damage +7 Cry: Cast 5 random Radiant Spells. They target enemies when possible.
  - **Text:** Spell Damage +3
    Cry: Cast 5 random Spells. They target enemies when they can.
    **Radiant:** Spell Damage +7
    Cry: Cast 5 random Radiant Spells. They target enemies when they can.
  - **Engine:** E12's random casts, narrowed to enemy targets when one is legal, from the non-token
    Spells of every set; its own Spell Damage counts (it's on the field during its Cry).
  - **Numbers:** Spell Damage 3 ↑; casts 5 ↑.

#### Classic+ #39 · Book Worm
`classicplus-039` · (1) Unit · Common · 1/4 → 2/8

> **Designer:** 1/4 Death: Add 1 random Book to your hand. Start of Turn: Increase the number of Books
> this adds by 1. ~~~ 2/8 Death: Add 1 random Radiant Book to your hand. Start of Turn: Increase the
> number of Books this adds by 1.

- **Text:** Death: Add N random Books to your hand. N starts at 1.
  Start of turn: N increases by 1.
- **Radiant:** Death: Add N random Radiant Books to your hand. …
- **Engine:** `counters.books`, 1 on arrival, +1 at its controller's start of turn; Death reads it
  last-known (R78). Pool: non-token Books of every set (Book Worm is no Book). A `preview` (R280) shows N.
- **Numbers:** growth 1 ↑.

#### Classic+ #40 · Appropriations
`classicplus-040` · (X) Spell · Epic

> **Designer:** Choose one: Military: Units on your board, in your deck, and in your hand gain +2X
> Attack and Rush. Education: Shuffle 2X Radiant Books into your deck. They gain Cast on Draw and target
> enemies when possible. Culture: Cards on your board, in your deck, and in your hand have a 10X%
> chance to become Radiant. Healthcare: Units on your board, in your deck, and in your hand gain +2X
> Health and Armor X. ~~~ Choose one: Military: Units on your board, in your deck, and in your hand gain
> +5X Attack and Rush. Education: Shuffle 5X Radiant Books into your deck. They gain Cast on Draw and
> target enemies when possible. Culture: Cards on your board, in your deck, and in your hand have a 25X%
> chance to become Radiant. Healthcare: Units on your board, in your deck, and in your hand gain +7X
> Health and Armor 2X.

- **Text:** Choose one:
  Military: Your Units on the field, in your hand and in your deck get +2X Attack and Rush.
  Education: Shuffle 2X random Radiant Books into your deck. They have Cast on draw and target enemies
  when they can.
  Culture: Each card on your field, in your hand and in your deck has a 10X% chance to become Radiant.
  Healthcare: Your Units on the field, in your hand and in your deck get +2X Health and Armor X.
- **Radiant:** the same with 5X Attack, 5X Books, 25X% and +7X Health with Armor 2X.
- **Engine:** X at least 1 (R348), the mode declared at play. E38 carries the buffs and keywords from
  hand and deck onto the field. Education: random non-token Books of every set (repeats allowed), made
  Radiant, with E39's "cast on draw, targets preferring enemies" enchantment; R80's cap. Culture: one
  roll per non-Radiant card (R60); at X = 4 that is 40% (Radiant 100%).
- **Numbers:** none beyond X.

#### Classic+ #41 · KY’s Constant
`classicplus-041` · (1) Spell, KY · Rare

> **Designer:** Change a random number on a card in your hand to 3. ~~~ Discover a number number on a
> card in your hand to 3.

- **Text:** Choose a card in your hand. Change a random number on it to 3.
- **Radiant:** Choose a card in your hand. Discover a number on it and change that number to 3.
- **Engine:** "a number on a card" per B3.4: its cost (not X), attack, health, a numbered keyword's
  value, or a declared number. The designer wrote "a card in your hand", not a random one, so the card
  is a declared hand pick (R81, as Glowy Jelly Bean's), among cards with a number that isn't already 3.
  Base: a random such number on it. Radiant: a Discover of up to 3 different such numbers on it. The
  change is `tuning` (B3.4), so it stays with the card.
- **Numbers:** none.
- **Check:** "number number" typo. The KY pool gains this card (B2.6).

#### Classic+ #42 · KY’s Test
`classicplus-042` · (1) Spell, KY · Legendary

> **Designer:** Offer three multiple-choice problems: one Easy, one Medium, and one Hard. Each shows a
> random reward from its difficulty’s list. Choose one to answer. If you answer correctly, gain its
> reward. Easy (simple addition): 3 Coins, a random (2) cost KY card, a random Legendary card that
> costs (0), or 2 random Books. Medium (relatively simple double integrals): 2 random (4) cost cards
> that cost (1), 5 random Books, 5 random KY cards, or fill your hand with Books. Hard (rigorous proofs,
> complex Markov chains, statistics, PDEs, linear algebra): Add KY’s Gift to your hand. It costs (0). ~~~
> (the same) Rewards are Radiant.

- **Text:** Offer an Easy, a Medium and a Hard problem, each showing a random reward from its list.
  Choose one and answer it. If you're right, gain its reward.
  Easy (addition): 3 The Coins; a random Cost (2) KY card; a random Legendary card costing (0); or 2
  random Books.
  Medium (double integrals): 2 random Cost (4) cards costing (1); 5 random Books; 5 random KY cards; or
  fill your hand with random Books.
  Hard (proofs, Markov chains, statistics, PDEs, linear algebra): KY's Gift, costing (0).
- **Radiant:** the same, and the rewards are Radiant.
- **Engine (E31):** two prompts: the three difficulties, each labelled with the reward rolled for it (one
  random entry of its list, rolled as the Spell resolves); then the chosen problem, its statement and
  four options in an rng-shuffled order. The bank is data in `packages/cards` (public, like the catalog):
  `{ id, difficulty, statement, options[4], answer }`, at least 30 problems per difficulty, the Easy
  ones generated from the rng (a + b with near-miss wrong sums). The engine checks the answer, and **the
  answer never leaves the engine**: the options go to the chooser, the key stays in the prompt's resume
  data, which `viewFor` never sends (§10.8), and the opponent sees only that a prompt is open, then the
  reward's ordinary events. "3 Coins" are three Core The Coin tokens; "fill your hand" adds random Books
  until the hand holds 10.
- **Rulings (CL41):** a wrong answer gives nothing (the Spell was still played). Statements are plain
  text with Unicode maths (∫, ², √, subscripts), so the client needs no maths renderer. The AI will find
  the right answer by simulating each (every answer is a `reduce`), which is fine for a computer.
  **⚠ designer:** the turn clock (75 s, R79) keeps running while a player works on a Hard problem; pause
  it or give the prompt its own clock? Default: neither; the clock runs, as for every prompt. The bank
  ships with the engine, client bundle included, so a determined player can look answers up; keeping
  the keys server-only would cost hotseat and practice the card.
- **Numbers:** none.

- **#42.1 KY’s Gift** · (4) Field Spell, KY · Token, Legendary by the designer (`classicplus-042-1`).
  > **Designer:** Start of Turn: Gain 1 mana. Your opponent discards a card. Heal your Hero for 5. Add a
  > random Book, KY card, Legendary card, and (4) cost card to your hand. They cost (0). ~~~ Start of
  > Turn: Gain 2 mana. Your opponent discards 2 cards. Heal your Hero for 10. Add a random Radiant Book,
  > KY card, Legendary card, and (4) cost card to your hand. They cost (0).
  - **Text:** Start of turn: Gain 1 mana. Your opponent discards a card. Heal your hero 5. Add a random
    Book, a random KY card, a random Legendary card and a random Cost (4) card to your hand. They cost
    (0).
    **Radiant:** 2 mana, 2 discards, heal 10, and the four cards are Radiant.
  - **Engine:** the discard is the opponent's choice (R16), their prompt on your turn.
  - **Numbers:** mana 1 ↑; discards 1 ↑; heal 5 ↑.

#### Classic+ #43 · AI Slop
`classicplus-043` · (4) Spell · Legendary

> **Designer:** Add 3 AI generated cards to your hand, fuse them, it costs (0) (you [the ai programming
> this] create the AI generated cards, make 10 of them with any effect you want, they should be the
> same as Claude’s Datacenter) ~~~ Add 3 Radiant AI generated cards to your hand, fuse them, it costs
> (0) (…)

- **Text:** Fuse 3 random AI generated cards and add the result to your hand. It costs (0).
- **Radiant:** Fuse 3 random Radiant AI generated cards and add the result to your hand. It costs (0).
- **Engine:** E23: three random picks from B8's ten (repeats allowed, R60), fused with no target (R77,
  R102: the shared type, else the first's; a Token, since every ingredient is one), `costOverride 0`.
- **Numbers:** cards 3 ↑.

#### Classic+ #44 · Simplicity Audit
`classicplus-044` · (2) Spell · Rare

> **Designer:** Exile every card in the field with LESS lines of code than this ~~~ Choose if its all on
> only your opponents; Exile every card in the field with LESS lines of code than this (highlight
> targets)

- **Text:** Exile every permanent whose card has fewer lines of code than this one.
- **Radiant:** Choose all permanents or only your opponent's. Exile each of them whose card has fewer
  lines of code than this one.
- **Engine:** E36: each permanent's `loc` against this card's. "(highlight targets)" is the Radiant's
  alone: its view marks the permanents it would exile (a `preview` carrying a set of ids, R280's
  pattern). The base face marks nothing, the same joke as Hired Shrimp's.
- **Numbers:** none.

#### Classic+ #45 · Complexity Audit
`classicplus-045` · (2) Spell · Rare

> **Designer:** Exile every card in the field with MORE lines of code than this ~~~ Choose if its all on
> only your opponents; Exile every card in the field with MORE lines of code than this (highlight
> targets)

- **Text:** Exile every permanent whose card has more lines of code than this one.
- **Radiant:** Choose all permanents or only your opponent's. Exile each of them whose card has more
  lines of code than this one.
- **Engine:** #44 with the comparison reversed.
- **Numbers:** none.

#### Classic+ #46 · Felinor Flagbearer
`classicplus-046` · (2) Unit, Felinor · Legendary · 4/4 → 8/8

> **Designer:** 4/4 Rush, Cleave Cry: Permanently your hero gains +1 Armor Aura: Your other Felinors have
> +1/+1 Death: Shuffle Felinor Flagbearer Prime into your deck ~~~ 8/8 Rush, Cleave Cry: Permanently
> your hero gains +2 Armor Aura: Your Felinors have +2/+2 Death: Shuffle Felinor Flagbearer Prime into
> your deck

- **Text:** Rush, Cleave
  Cry: Your hero gains +1 Armor for the rest of the game.
  Aura: Your other Felinors have +1/+1.
  Death: Shuffle a Felinor Flagbearer Prime into your deck.
- **Radiant:** Rush, Cleave
  Cry: Your hero gains +2 Armor for the rest of the game.
  Aura: Your Felinors have +2/+2.
  Death: Shuffle a Felinor Flagbearer Prime into your deck.
- **Engine:** hero Armor is §4.4 step 2's per-hit reduction: `hero.armor` +1, and it stacks. The aura
  (layer 5) reaches your Felinor-tagged Units (the Radiant's includes itself). R80's cap.
- **Numbers:** Armor 1 ↑; aura 1 ↑.

- **#46.1 Felinor Flagbearer Prime** · (2) Unit, Felinor · Token, Legendary by the designer · 5/5 →
  10/10 (`classicplus-046-1`; the designer's #47).
  > **Designer:** 5/5 Rush Cry: Fill your board with copies of this Aura: Your other Felinors have
  > +1/+1 ~~~ 10/10 Rush Cry: Fill your board with copies of this Aura: Your other Felinors have +2/+2
  - **Text:** Rush
    Cry: Fill your board with copies of this.
    Aura: Your other Felinors have +1/+1.
    **Radiant:** … +2/+2.
  - **Engine:** copies per R57, without Cry (R1), so no loop; every copy's aura lifts the others.
  - **Numbers:** aura 1 ↑.

#### Classic+ #47 · Jogg’s Box *(the designer's second #11)*
`classicplus-047` · (4) Spell · Legendary

> **Designer:** Cast 10 random spells. ~~~ Echo 1 Cast 10 random spells.

- **Text:** Cast 10 random Spells. **Radiant:** Echo 1 · Cast 10 random Spells.
- **Engine:** E12's random casts, one after another, every choice random (Hearthstone's Yogg-Saron),
  from the non-token Spells of every set but Jogg's Box (B4.1). With every choice random nothing pauses
  for a prompt. Echo 1 runs the ten again.
- **Numbers:** casts 10 ↑ (step 2).

#### Classic+ #48 · Jlockheed’s Lobbyist
`classicplus-048` · (1) Unit, Jlockeed · Legendary · 0/3 → 0/6

> **Designer:** 0/3 Can’t be in defense position Death: Add a random Jlockheed card into your hand, it
> costs (0) ~~~ 0/6 Death: Add a random Radiant Jlockheed card into your hand, it costs (0)

- **Text:** Cannot be in Defense Position.
  Death: Add a random Jlockheed card to your hand. It costs (0).
- **Radiant:** Death: Add a random Radiant Jlockheed card to your hand. It costs (0).
- **Engine:** the pool is the non-token cards with the Jlockeed tag (B2.4: Core #13, #14; Classic+ #48,
  #51, #52), this one excluded (B4.1). The Radiant face drops the Defense restriction.
- **Numbers:** none.

#### Classic+ #49 · Jay Fungus
`classicplus-049` · (2) Unit · Rare · 3/6 → 6/12

> **Designer:** 3/6 Taunt End of Turn: Reduce a random card in your hands’ costs by (2) ~~~ 6/12 Taunt
> End of Turn: Reduce a random card in your hands’ costs by (20)

- **Text:** Taunt
  End of turn: A random card in your hand costs (2) less.
- **Radiant:** Taunt
  End of turn: A random card in your hand costs (20) less.
- **Engine:** `costMod` −2 (−20, which floors at 0, §2.3) on a random hand card that it can make
  cheaper: cost above 0 and not X-cost (R65: cost modifiers never reach X).
- **Numbers:** discount 2 ↑.

#### Classic+ #50 · Adaptive Growth
`classicplus-050` · (1) Spell · Epic

> **Designer:** Cast when Drawn: If you have less units than your opponent, give all units -3/-3, if
> not give all units +2/+2 ~~~ Cast when Drawn: If you have less units than your opponent, give all
> enemy units -4/-4, if not give all friendly units +3/+3

- **Text:** Cast on draw: If you control fewer Units than your opponent, give all Units −3/−3.
  Otherwise, give all Units +2/+2.
- **Radiant:** Cast on draw: If you control fewer Units than your opponent, give enemy Units −4/−4.
  Otherwise, give your Units +3/+3.
- **Engine:** Cast on draw (§6.2); permanent buffs; −3/−3 lowers max health, so units at 0 die at the
  state check.
- **Numbers:** buff 2 ↑; Radiant debuff 4 ↑.

#### Classic+ #51 · Jlockheed’s J15 Fighter
`classicplus-051` · (3) Unit, Jlockeed · Epic · 7/2 → 14/4

> **Designer:** 7/2 First Strike, Rush Cant be in a defense position. Can’t be attacked ~~~ 14/4 First
> Strike, Rush, Divine Shield Cant be in a defense position. Can’t be attacked

- **Text:** First Strike, Rush
  Cannot be in Defense Position. Can't be attacked.
- **Radiant:** First Strike, Rush, Divine Shield
  (the same).
- **Engine:** E35's "can't be attacked": still targeted by effects and hit by "all" effects.
- **Numbers:** none.

#### Classic+ #52 · Jlockheed’s Permanent Defense Contract
`classicplus-052` · (2) Spell, Jlockeed · Epic

> **Designer:** For the rest of the game, at the start of your turn add a random Jlockheed card to your
> hand. ~~~ For the rest of the game, at the start of your turn add a random Radiant Jlockheed card to
> your hand, it costs (1) less.

- **Text:** For the rest of the game: At the start of your turn, add a random Jlockheed card to your
  hand.
- **Radiant:** For the rest of the game: At the start of your turn, add a random Radiant Jlockheed card
  to your hand. It costs (1) less.
- **Engine:** E28; the pool excludes this card (B4.1); several contracts stack.
- **Numbers:** cards 1 ↑; Radiant discount 1 ↑.

#### Classic+ #53 · Book of Tokens
`classicplus-053` · (1) Spell, Book · Epic

> **Designer:** Summon 2 Rush Tokens ~~ Summon 2 Radiant Rush Tokens

- **Text:** Summon 2 Rush Tokens. **Radiant:** Summon 2 Radiant Rush Tokens.
- **Numbers:** tokens 2 ↑.
- **Check:** the `~~` separator, read as `~~~`.

#### Classic+ #54 · Book of Books
`classicplus-054` · (1) Spell, Book · Epic

> **Designer:** Add 2 Random Books to your hand, they cost (0) ~~ Add 2 Random Radiant Books to your
> hand, they cost (0)

- **Text:** Add 2 random Books to your hand. They cost (0).
- **Radiant:** Add 2 random Radiant Books to your hand. They cost (0).
- **Engine:** every set's non-token Books but this one (B4.1).
- **Numbers:** books 2 ↑.

#### Classic+ #55 · Book of Greed
`classicplus-055` · (1) Spell, Book · Epic

> **Designer:** Add 3 Random Legendary or Mythic cards to your hand ~~ Add 3 Random Radiant Legendary or
> Mythic cards to your hand

- **Text:** Add 3 random Legendary or Mythic cards to your hand.
- **Radiant:** Add 3 random Radiant Legendary or Mythic cards to your hand.
- **Engine:** non-token cards of those rarities, every set (tokens carry `rarity: "Token"`, B2.5).
- **Numbers:** cards 3 ↑.

#### Classic+ #56 · Book of Pain
`classicplus-056` · (1) Spell, Book · Epic

> **Designer:** Your opponent discards 2 cards ~~ Your opponent discards 4 cards

- **Text:** Your opponent discards 2 cards. **Radiant:** … 4 cards.
- **Engine:** their choice (R16).
- **Numbers:** discards 2 ↑.

#### Classic+ #57 · Book of Stats
`classicplus-057` · (1) Spell, Book · Epic

> **Designer:** Give a unit +5/+5 ~~ Give a unit +10/+10

- **Text:** Give a Unit +5/+5. **Radiant:** +10/+10.
- **Numbers:** buff 5 ↑.

#### Classic+ #58 · Fruit Basket
`classicplus-058` · (1) Spell, Fruit · Rare

> **Designer:** Add 3 Random Fruits to your hand ~~ Add 3 Random Radiant Fruits to your hand

- **Text:** Add 3 random Fruits to your hand. **Radiant:** … Radiant Fruits …
- **Engine:** the Fruit pool (CL3: the non-token Fruit cards and the five Grapes), Fruit Basket excluded.
- **Numbers:** fruits 3 ↑.

#### Classic+ #59 · All Purpose Apple
`classicplus-059` · (1) Spell, Fruit · Rare

> **Designer:** Summon a rush token, heal 2 to your hero, deal 1 damage ~~ Summon a Radiant rush token,
> heal 4 to your hero, deal 2 damage

- **Text:** Summon a Rush Token. Heal your hero 2. Deal 1 damage.
- **Radiant:** Summon a Radiant Rush Token. Heal your hero 4. Deal 2 damage.
- **Engine:** the damage's target is declared at play.
- **Numbers:** heal 2 ↑; damage 1 ↑.

#### Classic+ #60 · Doctors Orders
`classicplus-060` · (1) **Field Spell** (the designer wrote "Unit, Spell") · Rare

> **Designer:** Cry and Start of Turn: Add an All Purpose Apple to your hand ~~~ Cry and Start of Turn:
> Add a Radiant All Purpose Apple to your hand

- **Text:** Cry and start of turn: Add an All Purpose Apple to your hand.
- **Radiant:** Cry and start of turn: Add a Radiant All Purpose Apple to your hand.
- **Rulings (CL42):** no stats were given, and "Cry and start of turn" is how this set writes a Field
  Spell (The House, Fusion Lab), so it is read as one — as §5.3 settled Core #68's "Spell, Unit".
  **⚠ designer:** or a Unit, with stats to name.
- **Numbers:** apples 1 ↑.

#### Classic+ #61 · Bauble Bubble
`classicplus-061` · (1) Field Spell, Fruit · Rare

> **Designer:** Death: Add 2 (0) cost Stockpile to your hand. ~~ Death: Add 2 (0) cost Radiant Stockpile
> to your hand.

- **Text:** Death: Add 2 Stockpiles to your hand. They cost (0).
- **Radiant:** Death: Add 2 Radiant Stockpiles to your hand. They cost (0).
- **Engine:** a Death on a backrow card (it leaves the field for a graveyard: destroyed, tributed, eaten
  by Core #22 Carnivorous Cube). No Core backrow card has a Death, so §4.5 step 3 must fire Death for
  collected backrow cards as it does for units (Frostspatula needs the same). Stockpile is Core #5.
- **Numbers:** cards 2 ↑.
- **Check:** a bait card: nothing until it pops.

#### Classic+ #62 · KY’s Papaya
`classicplus-062` · (1) Spell, Fruit, KY · Epic

> **Designer:** Create a coordinate plane of the board from your hero wherein your lane 1, your backrow
> corresponds to <0,0> and lane 5 back their backrow corresponds to <4,3>. Create up to a 3rd degree
> polynomial equation. All cards that would pass through this line get exiled. ~~ (the same) All enemy
> cards that would pass through this line get exiled.

- **Text:** Draw a curve y = ax³ + bx² + cx + d across the board, where x is the lane (0 to 4 from your
  lane 1) and y the row (0 your backrow, 1 your units, 2 their units, 3 their backrow). Exile every card
  on the curve.
- **Radiant:** … Exile every enemy card on the curve.
- **Engine (E32):** the answer is 1 to 4 cells in different lanes (E18's `cells` prompt, at
  resolution); the curve is the lowest-degree polynomial through them (Lagrange, in exact rationals,
  degree at most 3). A card is on the curve when its cell satisfies y = p(x) exactly, so with four
  cells the curve is fixed and may also hit a cell in the fifth lane. There are about 2,100 answers,
  and `promptAnswers` stops listing at `MAX_PROMPT_ANSWERS` (256), so one prompt would leave most
  curves out of `legalActions`, the fuzz suite and the AI. Ask for the cells one prompt at a time
  instead: each offers the 4 cells of every lane not yet used and, after the first, "done", so at most
  21 answers, and every curve stays reachable.
- **Rulings (CL43):** the player picks points, not coefficients: up to four cells define the cubic,
  which is how a player "creates" one without typing it; a client may show the equation it makes. Cards
  are points at their cells; between lanes the curve touches nothing.
- **Numbers:** none.

#### Classic+ #63 · Fruit Tree
`classicplus-063` · (2) Field Spell, Fruit · Rare

> **Designer:** Start of your turn: Add a random Fruit to your hand it costs (0). ~~ Start of your turn:
> Add a random Fruit to your hand it costs (0).

- **Text:** Start of turn: Add a random Fruit to your hand. It costs (0).
- **Radiant (proposed):** Start of turn: Add a random Radiant Fruit to your hand. It costs (0).
- **Check:** **R276**: the designer's two faces are identical. Proposed as above (or "2 random
  Fruits"). **⚠ designer.**
- **Numbers:** fruits 1 ↑.

#### Classic+ #64 · Mulch Muncher
`classicplus-064` · (10) Unit · Rare · 9/9 → 18/18

> **Designer:** 9/9 Rush, Trample Costs (1) less per fruit you’ve played this game ~~~ 18/18 Rush,
> Trample, Divine Shield Costs (1) less per fruit you’ve played this game

- **Text:** Rush, Trample
  Costs (1) less for each Fruit you've played this game.
- **Radiant:** Rush, Trample, Divine Shield
  (the same).
- **Engine:** a `cost` hook (Core #100's pattern, R55) reading E4's `playedByTag.Fruit` for its
  controller; casts count (R70), and a Grape is a Fruit.
- **Numbers:** discount per Fruit 1 ↑.

#### Classic+ #65 · Two Grapes
`classicplus-065` · (1) Spell, Fruit · Rare

> **Designer:** Add 3 Graphes to your hand (12% rotten) (60% normal) (20% large) (7% golden) (1% mythic)
> ~~ Add 3 Radiant Graphes to your hand, roles are Lucky 1 (12% rotten) (60% normal) (20% large) (7%
> golden) (1% mythic)

- **Text:** Add 3 Grapes to your hand, each rolled: Rotten 12%, Normal 60%, Large 20%, Golden 7%,
  Mythic 1%.
- **Radiant:** Add 3 Radiant Grapes to your hand, each rolled with Lucky 1 (the same odds).
- **Engine:** independent weighted rolls (`GRAPE_ODDS` in `config.ts`); Lucky 1 rolls twice and keeps
  the better, in the order Rotten < Normal < Large < Golden < Mythic.
- **Numbers:** grapes 3 ↑.
- **Check:** "Graphes" is Grapes, "roles" is rolls. Three grapes from "Two Grapes" is kept as the joke
  it looks like.

The five Grapes (`classicplus-065-1` … `-5`), Fruit Spell Tokens that any Fruit pool also offers
(CL3), each with the designer's printed rarity (B2.5):

- **#65.1 Rotten Grape** · (1) · printed Common.
  > **Designer:** (Despite being a token can be generated by any Fruit card) Your hero loses 5 hp ~~~
  > Your hero loses 1 hp
  - **Text:** Your hero loses 5 health. **Radiant:** Your hero loses 1 health.
  - **Engine:** lose health (R18), no pipeline. The Radiant's smaller loss is its upgrade.
- **#65.2 Normal Grape** · (1) · printed Common.
  > **Designer:** (…) Deal 2 damage if played on an enemy, heal 2 hp if played on an ally Draw 1, the
  > card costs (1) less ~~~ Deal 4 damage if played on an enemy, heal 4 hp if played on an ally Draw 2,
  > the card costs (1) less
  - **Text:** Choose a Unit or hero. If it's an enemy, deal 2 damage to it; if it's yours, heal it 2.
    Draw 1. It costs (1) less.
    **Radiant:** 4 and 4; draw 2, and they cost (1) less.
  - **Numbers:** amount 2 ↑; draw 1 ↑.
- **#65.3 Large Grape** · (3) · printed Rare.
  > **Designer:** (…) Deal 5 damage if played on an enemy, heal 5 hp if played on an ally Draw 1, the
  > card costs (0) ~~~ Deal 10 damage if played on an enemy, heal 10 hp if played on an ally Draw 2, the
  > card costs (0)
  - **Text:** … 5 and 5. Draw 1. It costs (0). **Radiant:** 10 and 10; draw 2, and they cost (0).
  - **Numbers:** amount 5 ↑; draw 1 ↑.
- **#65.4 Golden Grape** · (1) · printed Legendary.
  > **Designer:** (…) Make a card in your board or hand Radiant ~~~ Make a card in your board or hand
  > Radiant, Cleave (Cleave targets adjacent cards on board OR in hand)
  - **Text:** Make a card on your side of the field or in your hand Radiant.
    **Radiant:** … Radiant, and the cards next to it (beside it in its row, or beside it in your hand).
  - **Engine:** a declared pick (R81: a hand pick or one of your permanents); "next to it" in a hand is
    the neighbours by hand index.
- **#65.5 Mythic Grape** · (0) · printed Mythic.
  > **Designer:** (…) Replace your hand with Random Mythic cards. They cost (0) ~~~ Replace your hand
  > with Random Radiant Mythic cards. They cost (0)
  - **Text:** Replace your hand with random Mythic cards. They cost (0).
    **Radiant:** … random Radiant Mythic cards …
  - **Engine:** each other card in hand goes to your graveyard and is replaced by a random non-token
    Mythic of every set (Core #76's reading).

#### Classic+ #66 · Vine of Grapes
`classicplus-066` · (3) Spell, Fruit · Rare

> **Designer:** Add 5 Graphes to your hand (12% rotten) (60% normal) (20% large) (7% golden) (1% mythic)
> ~~ Add 3 Radiant Graphes to your hand, roles are Lucky 1 (12% rotten) (60% normal) (20% large) (7%
> golden) (1% mythic) (uses graphs as above)

- **Text:** Add 5 Grapes to your hand (Two Grapes' odds).
- **Radiant (proposed):** Add 5 Radiant Grapes to your hand, each rolled with Lucky 1.
- **Check:** the Radiant text says 3, which reads as copied from Two Grapes; 5 is proposed so the Radiant
  doesn't lose two Grapes. **⚠ designer.**
- **Numbers:** grapes 5 ↑.

#### Classic+ #67 · Pear
`classicplus-067` · (2) Spell, Fruit · Rare

> **Designer:** Summon 2 random (1) cost Common units ~~~ Summon 2 random (1) cost Radiant Common units

- **Text:** Summon 2 random Cost (1) Common Units. **Radiant:** … Radiant …
- **Engine:** non-token Units printed at cost 1 and rarity Common, every set; summoned (no Cry).
- **Numbers:** units 2 ↑.

#### Classic+ #68 · Organic Produce
`classicplus-068` · (4) Field Spell, Fruit · Epic

> **Designer:** Cry: Add a random Fruit to your hand Aura: Fruit you play is Radiant ~~~ Cry: Add two
> random Fruit to your hand, they cost (0) Aura: Fruit you play is Radiant

- **Text:** Cry: Add a random Fruit to your hand.
  Aura: Fruits you play are Radiant.
- **Radiant:** Cry: Add 2 random Fruits to your hand. They cost (0).
  Aura: Fruits you play are Radiant.
- **Engine:** Core #64 Gifted Program's static flag at §10.5 step 3, for Fruit-tagged plays (R213,
  R214).
- **Numbers:** fruits 1 ↑.

#### Classic+ #69 · Buff Billy
`classicplus-069` · (X) Unit, Human · Rare · 3X/3X → 7X/7X

> **Designer:** [3X / 3X] Cry: Upgrade this X times ~~~ [7X / 7X] Cry: Upgrade this 2X times

- **Text:** This is a 3X/3X.
  Cry: Upgrade this X times.
- **Radiant:** This is a 7X/7X.
  Cry: Upgrade this 2X times.
- **Engine:** E40's X in the stats: summoned with a `statsOverride` from the X it was played for (at
  least 1, R348). Outside a play (a Recruit, a copy with no X) its X is 0, so it arrives 0/0 and dies,
  as a Ghoul Token with no X would.
- **Numbers:** none.

#### Classic+ #70 · Chaos Machine
`classicplus-070` · (2) Field Spell · Rare

> **Designer:** Start & End of turn: Upgrade a random card in your hand or you control, and Degrade a
> random card in your opponents hand or under their control ~~~ Start & End of turn: Upgrade two random
> cards in your hand or you control, and Degrade two random cards in your opponents hand or under their
> control.

- **Text:** Start of turn and end of turn: Upgrade a random card in your hand or on your side of the
  field. Degrade a random card in your opponent's hand or on their side of the field.
- **Radiant:** … two random cards … two random cards …
- **Engine:** B3.4. The Degrade's pick is over cards you can't see (their hand), so it is cued as R177's
  hidden picks are.
- **Numbers:** cards 1 ↑.

#### Classic+ #71 · Book of Buff
`classicplus-071` · (1) Spell, Book · Epic

> **Designer:** Upgrade a card 5 times (Can target cards in your hand) ~~ Upgrade a card 10 times (Can
> target cards in your hand)

- **Text:** Upgrade a card 5 times. It may be a card in your hand.
- **Radiant:** Upgrade a card 10 times. It may be a card in your hand.
- **Engine:** a declared target: a permanent (either side) or a card in your hand.
- **Numbers:** times 5 ↑.

#### Classic+ #72 · Book of Nerf
`classicplus-072` · (1) Spell, Book · Epic

> **Designer:** Degrade a card 5 times ~~ Degrade a card 10 times (Can target cards in your hand)

- **Text:** Degrade a permanent 5 times.
- **Radiant:** Degrade a card 10 times. It may be a card in your hand.
- **Check:** the hand clause is on the Radiant face only, which may be copied from Book of Buff; kept as
  written, a broader scope (R275). **⚠ designer.**
- **Numbers:** times 5 ↑.

#### Classic+ #73 · Call to Chaos (Classic+ Edition)
`classicplus-073` · (4) Spell, Call to Chaos · Legendary

> **Designer:** ??? (One of the following random effects) Add 5 fruit to your hand, they cost (0) · Add
> 3 books to your hand, they cost (0) · Destroy all enemy permanents · Add 3 Classic cards to your hand,
> they cost (0) · Upgrade all cards in your hand & deck twice · Fuse all cards in your deck with a
> random card, they maintain their original cost · Degrade all cards in your opponents board and hand
> three times · Summon a Classic Golem · Replace your deck with random Call to Chaos, they cost (0) ·
> Cast a random Call to Chaos ~~~ !!! (three effects)

- **Text:** One random effect: add 5 random Fruits to your hand, costing (0); add 3 random Books to your
  hand, costing (0); destroy all enemy permanents; add 3 random Classic cards to your hand, costing
  (0); Upgrade every card in your hand and deck twice; fuse a random card into each card in your deck,
  each keeping its cost; Degrade every card on your opponent's field and in their hand three times;
  summon a Classic Golem; replace your deck with random Call to Chaos cards costing (0); cast a random
  Call to Chaos.
- **Radiant:** Three different random effects from the same list.
- **Engine:** Core #95's subsystem (`subsystems/callToChaos.ts`) with a second table. The "Call to
  Chaos" pool now holds both editions (R28); `CALL_TO_CHAOS_CHAIN_CAP` (20) counts casts of either; R87
  carries over (a card cast from no zone goes to the graveyard; a recursion roll at the cap resolves into
  nothing). The deck replacement names its pool, so it may bring this card back (B4.1); the deck fusion
  is E23 with every set's non-token cards but this one; "Classic cards" is the Classic set.
- **Rulings (CL44):** the Radiant's three effects are three different entries, resolved in the list's
  order, the recursion where it falls. Patch v0.2.0 gives Core #95's Radiant the same shape (three
  random effects in place of the recursion plus one, R28 and R87 rewritten with it), so one rule
  serves both editions. **⚠ designer.**
- **Check:** in play the rules box reads "???" (Core's rule for the tag, §10.10), "!!!" on the Radiant.

- **#73.1 Classic Golem** · (4) Unit · Token, Legendary by the designer · 10/10 → 20/20
  (`classicplus-073-1`).
  > **Designer:** 10/10 Rush, Firststrike, Trample After this kills a unit this may attack again. When
  > this attacks a unit, it transforms into a random Classic or Classic+ card ~~~ 20/20 Rush,
  > Firststrike, Trample, Divine Shield (the same)
  - **Text:** Rush, First Strike, Trample
    After this attacks a Unit, it transforms into a random Classic or Classic+ Unit. If it destroyed
    that Unit, the new Unit may attack again this turn.
    **Radiant:** Rush, First Strike, Trample, Divine Shield
    (the same).
  - **Engine:** E24 and E35: after the combat of an attack it declared on a Unit, it is transformed
    (§6.3) into a random non-token Unit of Classic or Classic+; if the Golem was the defender's killer
    (R42), the new Unit has a fresh exertion and is not summoning sick this turn.
  - **Rulings (CL45):** the transform comes after the combat, so the Golem's own stats fight; it makes a
    Unit (a Transform on the field keeps the type, as R35 does); "may attack again" passes to the new
    Unit. It is the one reading in which both of its lines can matter. **⚠ designer.**
  - **Check:** "Firststrike" is First Strike.

#### Classic+ #74 · Twice Forward One Step Backwards
`classicplus-074` · (2) **Field Trap** (the designer wrote Trap) · Mythic

> **Designer:** Brittle 4 Every (2) cards your opponent plays, fuse it into this card and gain +1
> Brittle ~~~ Brittle 10 Every (2) cards your opponent plays, fuse a radiant version into this card and
> gain +1 Brittle

- **Text:** Brittle 4
  Every second card your opponent plays is fused into this after it resolves, and this gains +1 Brittle.
- **Radiant:** Brittle 10
  Every second card your opponent plays: a Radiant copy of it is fused into this, and this gains +1
  Brittle.
- **Engine:** printed Brittle starts when it is set (B3.3). It counts the opponent's plays since then
  (`memory.plays`); on each even count, after that card resolves, the card — if it still exists: a Unit
  on the field, a Spell in the graveyard, a trap in the backrow — is fused into this (E23, R77: this is
  the kept instance and stays a Field Trap; the opponent's card ceases to exist), then +1 Brittle. The
  texts fused in work for you where they can: a fused unit's end-of-turn line runs on your side, a
  fused aura covers your side; a fused Cry never runs, since this is already on the field. It turns
  face-up the first time it fuses (R33: a fired Field Trap is public).
- **Rulings (CL46):** a Trap is consumed when it fires, which would leave nothing to gain Brittle, so it
  is typed Field Trap. **⚠ designer.**
- **Numbers:** Brittle 4 ↑; every 2 ↓ (never below 2); Brittle gained 1 ↑.

#### Classic+ #75 · J-lease J-Jungle EX-plorer
`classicplus-075` · (2) Unit · Legendary · 5/5 → 10/10

> **Designer:** [5/5] Cry: Shuffle a J-lease J-Jungle EX-plorer Pack into your deck. ~~~ [10/10] Cry:
> Shuffle a J-lease J-Jungle EX-plorer Pack into your deck.

- **Text:** Cry: Shuffle a J-lease J-Jungle EX-plorer Pack into your deck.
- **Radiant (proposed):** Cry: Shuffle a Radiant J-lease J-Jungle EX-plorer Pack into your deck.
- **Check:** **R275**: the Radiant text repeats the base; shuffling the Radiant Pack (whose five cards
  cost (0)) makes the effect stronger as well as the stats. **⚠ designer.**
- **Numbers:** packs 1 ↑.

- **#75.1 J-lease J-Jungle EX-plorer Pack** · (2) Spell · Token, Legendary by the designer
  (`classicplus-075-1`; the designer's second #75).
  > **Designer:** Cast on Draw: Add 5 random Radiant Classic or Classic+ cards to your hand. ~~~ Cast on
  > Draw: Add 5 random Radiant Classic or Classic+ cards to your hand, they cost (0)
  - **Text:** Cast on draw: Add 5 random Radiant Classic or Classic+ cards to your hand.
    **Radiant:** … They cost (0).
  - **Engine:** non-token cards of the two sets; the hand cap burns what doesn't fit.
  - **Numbers:** cards 5 ↑.

#### Classic+ #76 · Brother Lar *(the designer's second #25)*
`classicplus-076` · (1) Unit, CN, Human · Rare · 1/1 → 2/2

> **Designer:** 1/1 Death: Summon a Brother Ping token ~~~ 2/2 Death: Summon a Radiant Brother Ping
> token

- **Text:** Death: Summon a Brother Ping. **Radiant:** Death: Summon a Radiant Brother Ping.
- **Engine:** into your leftmost open zone (its own is free again unless a Reborn reserves it).
- **Numbers:** none.

- **#76.1 Brother Ping** · (2) Unit, CN, Human · Token, Rare by the designer · 4/4 → 8/8
  (`classicplus-076-1`).
  > **Designer:** 4/4 Pierce Activate: Deal 1 damage ~~~ 8/8 Pierce Activate 2: Deal 1 damage
  - **Text:** Pierce
    Activate: Deal 1 damage.
    **Radiant:** Pierce
    Activate 2: Deal 1 damage.
  - **Engine:** B3.2; the hit's source is Brother Ping, so its Pierce applies (R346).
  - **Numbers:** Activate 1 ↑ (its X); damage 1 ↑.

#### Classic+ #77 · Anti-Softlock *(the designer's second #28)*
`classicplus-077` · (2) Spell · Rare

> **Designer:** Draw 1. All cards on both players' boards, hands, and decks gain Stack and Pierce.
> Unlock all slots. ~~~ Draw 2. Choose one: all cards on both players' boards, hands, and decks, or only
> yours, gain Stack and Pierce. Unlock all slots.

- **Text:** Draw 1. Every card on the field, in hands and in decks gains Stack and Pierce. Unlock every
  zone.
- **Radiant:** Draw 2. Choose all cards or only yours: each card on the field, in hands and in decks
  gains Stack and Pierce. Unlock every zone.
- **Engine:** E38 (keywords granted in hands and decks, carried onto the field), E21 (a backrow card
  with Stack may be played onto an occupied backrow zone), E20's Unlock. On a Spell, Pierce is R346's
  spell Pierce and Stack does nothing.
- **Rulings:** a "softlock" is a board too full or too Locked to play on; this undoes both.
- **Numbers:** draw 1 ↑.

#### Classic+ #78 · Claude’s Datacenter *(the designer's second #42)*
`classicplus-078` · (2) Field Spell · Legendary

> **Designer:** End of turn add a random AI generated card to your hand, it costs (0) (you [the ai
> programming this] create the AI generated cards, make 10 of them with any effect you want) ~~~ End of
> turn add a random Radiant AI generated card to your hand, it costs (0) (…)

- **Text:** End of turn: Add a random AI generated card to your hand. It costs (0).
- **Radiant:** End of turn: Add a random Radiant AI generated card to your hand. It costs (0).
- **Engine:** the pool is B8's ten, named by the text.
- **Numbers:** cards 1 ↑.

---
## B8. The ten AI generated cards

> **Designer (Claude's Datacenter and AI Slop):** you [the ai programming this] create the AI generated
> cards, make 10 of them with any effect you want, they should be the same as Claude’s Datacenter

Ten Tokens of the Classic+ set, tag **AI**, ids `classicplus-t-ai-01` … `-10`, indices `T-AI-1` …
`T-AI-10` (shared tokens: two cards make them). They reach play through #78 Claude's Datacenter and
#43 AI Slop, which hand them out costing (0), and through #23 Dropshipping's pool of every card and
token (CL3), which hands them out at their printed cost; their printed costs matter there and for
Degrade and Upgrade, KY's Constant and fusions. Each is built from systems the two sets need anyway, so none asks for
an engine feature of its own, and each Radiant face meets R275. The theme is the thing that made them:
a language model's habits, good and bad.

| # | Card | Cost, type | Base | Radiant |
| --- | --- | --- | --- | --- |
| T-AI-1 | **Helpful Assistant** | (1) Unit, 1/3 → 2/6 | Taunt. Cry: Discover a card from your deck. | Taunt, Divine Shield. Cry: Discover a card from your deck. It costs (1) less. |
| T-AI-2 | **Scaling Law** | (2) Unit, 2/2 → 4/4 | Has +1/+1 for each AI generated card you've played this game. | Has +2/+2 for each … |
| T-AI-3 | **Hallucination** | (0) Spell | Add a copy of a random card in your opponent's deck to your hand. Give it Brittle 2. | Add copies of 2 different random cards in your opponent's deck to your hand. Give them Brittle 2. |
| T-AI-4 | **Chain of Thought** | (1) Spell | Draw 1. If it costs (1) or less, repeat this, up to 4 more times. | … If it costs (2) or less … |
| T-AI-5 | **Autocomplete** | (0) Spell | Add a copy of the last Unit, Spell or Field Spell your opponent played to your hand. | … It costs (0). |
| T-AI-6 | **Datacenter Fire** | (2) Spell | Destroy all Field Spells. Deal 1 damage to each hero for each one destroyed. | Destroy all enemy Field Spells. Deal 2 damage to the enemy hero for each one destroyed. |
| T-AI-7 | **Alignment Tax** | (1) Spell | Your opponent's cards cost (1) more during their next turn. | … (2) more … |
| T-AI-8 | **Rate Limit** | (1) Trap | Reveals when your opponent plays their 3rd card in a turn: After it resolves, their turn ends. | … their 2nd card … |
| T-AI-9 | **Refusal** | (1) Trap | Reveals when your opponent plays a Spell that targets one of your Units: Counter it. | Reveals when your opponent plays a Spell that targets you or one of your cards: Counter it. Draw 1. |
| T-AI-10 | **Fine-Tuning** | (2) Field Spell | End of turn: Upgrade a random card in your hand. | End of turn: Upgrade 2 random cards in your hand. |

Engine notes:

- **Helpful Assistant**: a Discover whose pool is your own deck, as Core #51 KY's Private Tutor reveals
  library cards (the options are shown to you only, §10.8); the chosen card moves from deck to hand.
- **Scaling Law**: a layer-2 `setStat` (§10.4) reading E4's `playedByTag.AI` for its controller.
- **Hallucination**: you own the copy (it is a new card); only you see which card it copied. Brittle 2
  means it crumbles at the start of your turn after next (B3.3): a hallucination doesn't last.
- **Chain of Thought**: "it" is the card the draw put in your hand; a cast-on-draw card never gets there
  (R58) and a burned one isn't there, so either ends the chain.
- **Autocomplete**: E4's last face-up play of the opponent. Traps and Field Traps are set face-down, so
  they never count and nothing hidden is copied. AI generated cards are skipped, so two Autocompletes
  can't feed each other forever. Nothing played yet: nothing.
- **Datacenter Fire**: Indestructible Field Spells survive and don't count. The base face burns your
  own Claude's Datacenter too.
- **Alignment Tax**: a player modifier on the opponent, Professor Curvature's timing turned outward
  (R48, R363), through E15.
- **Rate Limit**: E10; the play that set it off resolves first. **Refusal**: E1 on a Spell whose
  declared targets (R81) include one of your Units (Radiant: you, or any of your cards).
- **Fine-Tuning**: B3.4's Upgrade.

---

## B9. Decisions for the designer

Everything below has a default in this brief, so the build need not wait; but each is a guess, and
the designer's answer should land in SPEC before the card it touches ships.

| # | Where | Question | Default here |
| --- | --- | --- | --- |
| 1 | B2.3 | Classic+ numbers collide five times; is the renumbering right? | Displaced cards to #47 and #76–#78; tokens to #46.1, #75.1 |
| 2 | B2.4 | "Jlockheed" (Classic+) or "Jlockeed" (Core)? | One tag, `Jlockeed`; Classic+ names as written |
| 3 | B2.5 | Keep the designer's rarities, not §8's complexity rubric? | Keep them |
| 4 | B2.6 | Should "random" pools reach every set (and so change Core cards' games)? | Yes, one format |
| 5 | B2.8 | Classic #55 is a second Book of Flame. Its real name and text? | Held back |
| 6 | B2.8 | Classic #72 shares Counterspell's name. Rename? | "Grand Counterspell" suggested |
| 7 | B3.4 | Upgrade's list was blank. Mirror of Degrade? | Yes |
| 8 | B4.2 | Labels for the pre-v0.1.1 history | v0.1.0, v0.1.0-r1 … r3 |
| 9 | B4.3 | "Double turn limit": the turn cap (30 → 60) or the turn clock (75 → 150 s)? At 60, two 20-card decks fatigue out at player-turn 48, so the cap is rarely reached | The cap; match ceiling 60 → 120 min |
| 10 | C #1 | Does the Radiant keep "Draw 1"? | Yes |
| 11 | C #4 | Is Palantir's base steal optional? | Yes ("Tribute this to …") |
| 12 | C #5 | Does Tesla keep firing while animated? | Yes (a Field Trap) |
| 13 | C #7 | Radiant InfiniScepter: still "from your hand"? | Yes |
| 14 | C #15 | Nose Hunter: once per turn or ♾️? | Once per turn |
| 15 | C #22 | Mid Runner's Radiant only doubles stats (R275) | Keep; record in the audit |
| 16 | C #28 | Second Wind: are the Cry's discards playable? | Yes |
| 17 | C #33 | Joro: from hand only? Radiant 1/1 (R275)? | Hand only; named exception |
| 18 | C #63 | Crop Dusting has no Radiant face | 2 Plague Counters each, draw 2; designer patch v0.2.2: 3 each, draw 3 |
| 19 | C #78 | Mutate Spell: a Field Spell? | Yes |
| 20 | C #80 | BOOM! Big Max Radiant 26/16 and no rider (R275) | Named exception for the stats; Charge for Rush |
| 21 | C #86 | Genn's faces are identical (R276) | Radiant 42/42, tripled as Mr. Vanilla's |
| 22 | C #90 | In Too Deep: what each quest counts | As its table says |
| 23 | C+ #2 | Groom Shroom: redirect the attack to a new Taunt? | No |
| 24 | C+ #7 | "Right-House Protector" is Core #3? | Yes |
| 25 | C+ #12.6–7 | Frozen Wastes and Legion of the Hungry: Field Spells? | Yes, the text as a Cry |
| 26 | C+ #14 | Forever&: does a discarded or countered copy come back? | No |
| 27 | C+ #17 | Conjure Rush Token++ copies #16's text | 6 keywords |
| 28 | C+ #19 | Mid Loser's Cry fires when League of Losers summons it? | Yes |
| 29 | C+ #27 | Zephrys Zealotism: same hand size, or a full hand? | Same size |
| 30 | C+ #29 | Portal to the Past: which games count, and whose side? | Same kind of game; both sides, minus hidden cards |
| 31 | C+ #35 | Rollback: where do cards that weren't there go? | Their owners' hands |
| 32 | C+ #42 | KY's Test: pause the turn clock while answering? | No; the clock runs, as for any prompt |
| 33 | C+ #60 | Doctors Orders: a Field Spell? | Yes |
| 34 | C+ #63 | Fruit Tree's faces are identical (R276) | Radiant: a Radiant Fruit |
| 35 | C+ #66 | Vine of Grapes Radiant says 3 Grapes | 5 |
| 36 | C+ #72 | Book of Nerf: hand clause on the Radiant only? | As written |
| 37 | C+ #73 | Radiant Call to Chaos: three different effects, in list order? | Yes |
| 38 | C+ #73.1 | Classic Golem: transform after the combat? | Yes |
| 39 | C+ #74 | Twice Forward One Step Backwards: a Field Trap? | Yes |
| 40 | C+ #75 | J-lease Explorer's Radiant repeats its base text (R275) | Shuffle a Radiant Pack |
| 41 | B3.3 | Brittle's first tick: at your next turn (t + 2) or after a whole turn of each player's (t + 4)? | t + 2 |
| 42 | C #41 | State of the Game's Radiant only doubles the stats (R275) | Add Lifesteal |

---

## B10. Implementation plan

The work order follows BUILD.md's habits: engine first, then cards in waves, then the client and the
server, each task done when its acceptance items are green tests (CLAUDE.md rule 2).

### B10.1 Documents first

1. **SPEC.md.** Port this brief: §1 (the pillars' "30-turn cap"), §2.2 and R62 (two new start-of-turn
   steps, so refresh → Brittle tick → "on your turn" cards animate → delayed effects → triggers →
   draw, in the diagram too), §2.5 (the cap, R2), §3.1 and R5 (attacks stay free of lanes and
   `LANE_RESTRICTED_ATTACKS` stays false, but a card may restrict who attacks it, E35), §3.2 (backrow
   piles, the Tribute zone rule), §4.2 (E35's restrictions in step 2, which forced attacks obey too),
   §4.4 (Spell Damage, multipliers, caps, spell Trample, the lethal window), §4.5 (backrow Death, the
   would-die window), §5 (sets, `SetName`, tags, the new catalog fields), §5.1 (pools across sets,
   self-exclusion by id), §6 (Animated, Activate, Brittle, Degrade, Upgrade, Spell Damage and the
   E-systems' verbs: Counter, Flicker, Unlock, steal off the field), §7 (the 38 tokens), §8 (two new
   tables, §8.6 Classic and §8.7 Classic+, in §8's column format), §9.4 (deck and trio codes v2),
   §9.9 (AI pools, the sweep), §10.1–§10.10 (state fields, the `activate` action, §10.5's announce
   step, new events, new prompt kinds, `viewFor` marks), and a block of §11 rows for every CL here
   plus the rulings each changes: R1 (Mid Loser), R2, R12 and R73 (a second ownership change), R13
   (Ivory Tower), R17 (counters answer before Sheepish), R28 and R87 (two editions), R35 (the
   Legendary pool), R43 (Heroic Power as Activate, patch v0.2.1), R57 (copies keep `tuning`), R59
   (Blade Storm), R65 (floors; an X card's cost on the field, CL17), R78 (`tuning` and Brittle
   persist), R79 (the ceiling), R105 (the catalog version is the patch), R184–R186 (AI pools and the
   sweep), R255 and R339 (codes v2), R278, R279 (rules-word names), R366 (the Activate labels, and
   v0.2.0's "(N) Cost").
2. **BUILD.md.** A milestone for the sets (call it M9): its tasks below, its constants in §2 (every
   number this brief names: `TURN_CAP_PLAYER_TURNS` 60, `ACTIVATE_UNLIMITED_CAP`, `GLITCH_NUMBERS`,
   `GRAPE_ODDS`, `BLADE_STORM_ROUNDS`, the sweep's new fields, `MATCH_CEILING_MINUTES`…; CLAUDE.md
   rule 9),
   two must-pass tables in M4-T4's format, and M5-T4 rows for the new events.
3. **A source note.** Add the designer's list verbatim as `JackiOh_Classic_Cards.md` beside
   `JackiOh_Core_Cards.md` (this brief's Designer quotes are that text, per card), so REVIEW.md Part A
   can check SPEC against it; add the new keywords to `JackiOh_Mechanics.md` if the designer wants the
   mechanics sheet to carry them.
4. **READMEs.** `packages/cards` (set folders, `params`, `loc`), `packages/ai` (the sweep),
   `apps/server` (catalog versions, last boards), `apps/web` (the new client layers).

### B10.2 Build order

| Task | Scope | Done when |
| --- | --- | --- |
| M9-T1 Catalog shape | `SetName` "Classic+", tags Book, Pancake, AI; `CardFace.type`; `params`; `loc` and its generator; `printedRarity`; ids and set folders in `naming.ts`, `gen-registry.ts`, `missing-tests.ts`; `excludeDefId` and every index lookup keyed by set (B2.2: `defByIndex`, `cardDefByIndex`, `excludingIndex`, the `index`/`notIndex`/`excludeIndex` filters, `deck.ts`, `determinize.ts`, `scorer.ts`); deck and trio codes v2 | catalog, query, deck-code, trio-code and registry tests pass with Core alone, unchanged |
| M9-T2 Patch history | `patches/`, snapshots for v0.1.0 … v0.1.1 rebuilt from git (B4.2), the `patch` script, `CATALOG_VERSION` from the latest patch everywhere it lives (`catalog-data.ts`, the server's env, `render.yaml`, `VITE_CATALOG_VERSION`, the reseeded `cards` rows and `app.settings`; R105 rewritten), and `GET /api/catalog/:version` | the snapshot test; the server serves each version |
| M9-T3 Global mechanics | turn cap 60 (B4.3), with `08-turn-cap-draw.cy.ts` reworked in the same change; Tribute zones (B4.5); self-exclusion by id (B4.1) | their R-tests; fuzz green at 1,000 seeds; e2e 08 green |
| M9-T4 Keywords | Animated, Activate, Brittle, Degrade/Upgrade (B3) with fixture scripts in `packages/engine/test/fixtures/` | an engine test per rule of B3 |
| M9-T5 Systems | E1–E40, each with fixture tests, in dependency order (E1, E2, E4, E5, E6, E15, E19, E11, E12 first: most cards need them) | an engine test per row of B5 |
| M9-T6 Cards, wave A | keyword-only and one-primitive cards (Classic #3, #12, #16, #24, #26, #41, #73, #82; Classic+ #6, #10, #15, #16, #20, #28, #53–#57, #59, #67, …) | their card tests (base and Radiant, CLAUDE.md rule 6) |
| M9-T7 Cards, wave B | cards on one or two B5 systems | the same |
| M9-T8 Cards, wave C | the subsystems: Classic #57, #90; Classic+ #19 and the Losers, #27, #29, #35, #42, #62, #73, #74, the Audits and Hired Shrimp | the same, plus each subsystem's own tests |
| M9-T9 AI | pools across sets (R184, R185), the two-pass sweep (B4.4), a sweep of record over 268 cards, the gates re-run and their counts recorded in §9.9 | `pnpm ai:gate`; the new `shadowBan.ts` |
| M9-T10 Client | the Activate control (as Heroic Power's), animations and cues for the new events, the Brittle badge, Degrade/Upgrade marks, the KY's Test dialog, the Papaya cell picker, the In Too Deep quest panel, the History tab and Patch notes page, `loc` in the inspect overlay, glossary rows, deck builder set filter | component tests; `ANIMATIONS` and `SOUND_CUES` stay total |
| M9-T11 Server | a migration re-adding `cards_tags_check` with Book, Pancake and AI, as `0010_jlockeed_tag.sql` did for Jlockeed (without it `db:seed-catalog` refuses the whole catalog); new cards seeded (`db:seed-catalog`) and granted to every account (the collection is a ledger: a grant migration through `collection_grants`), last boards stored per profile for Portal to the Past (with RLS, `test:sql`) | `test:sql`, `test:db`, the API tests |
| M9-T12 End to end | Cypress specs for an Animated trap springing, an Activate, a Counter on the opponent's turn and a Tribute onto a full board | the e2e job on Chrome and Electron |
| M9-T13 Review | REVIEW.md's paste-in prompt as a separate session (CLAUDE.md rule 8) | the report in `reviews/` |

### B10.3 Things that will bite

- **The sweep and the gates take longer.** 268 cards × 2 tiers × 8 seeds, plus pass 2; the gates'
  frozen seeds deal different decks once pools reach every set, so their counts move and §9.9's numbers
  are rewritten.
- **Voice lines.** §10.11 caps the pre-rendered voice at about 3 MB for 111 cards; 206 more cards and
  tokens blow it. Raise the cap, render shorter lines, or let the new sets fall back to the browser's
  speech synthesis (§10.11 already allows a missing file).
- **Hidden information.** Every new prompt that shows cards (Mind Melt, Helpful Assistant, Portal to the
  Past, KY's Test's answer key) must reach the chooser only, and the answer key never; `viewFor` tests
  in the R97/R177 style for each.
- **Termination.** ♾️ activations, Blade Storm, Chain of Thought, Call to Chaos across two editions,
  Echo copying Echo: each has a cap or a rule in this brief; the fuzz suite must reach them (add these
  cards to the random-policy decks it deals).
- **Replays.** Portal to the Past adds a setup input, Rollback adds state history, KY's Test adds a
  data bank: all three must survive `JSON.parse(JSON.stringify(state))` and fold exactly from the log.
- **Rulings coverage.** A CL number is not an R number: none of them may appear in `packages/` or `apps/`
  (CLAUDE.md rule 3's scan fails on an `R<n>` §11 lacks, and a stray "CL" reference would only confuse).
  Cite the R numbers the port assigns.
