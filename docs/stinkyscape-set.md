# Stinkyscape — the plan of issues (first pass)

2026-10-25 · written from the designer's list of the same day (`JackiOh_Stinkyscape_Cards.md`)

> **Status: a draft plan, waiting on the designer's answers (ST9).** SPEC is still the only source of rules
> (CLAUDE.md). This file reads the designer's list against the engine as it stands on `main`, names what
> the set reuses and what it must build, and splits the work into GitHub issues the way the Meditative
> (#496) and Meditative+ (#1173) sets were split (`docs/meditative-plus-set.md` MP10). It is not yet the
> card-by-card brief: the foundation issue (ST6, F) turns it into one, in `docs/meditative-plus-set.md`'s
> shape, once the questions in ST9 are answered. Every reading marked **default** is what the build uses if
> the designer says nothing.

## Contents

- [ST0. What the designer asked for](#st0-what-the-designer-asked-for)
- [ST1. The set in numbers](#st1-the-set-in-numbers)
- [ST2. What the engine already has](#st2-what-the-engine-already-has)
- [ST3. The new systems](#st3-the-new-systems)
- [ST4. The patches to shipped cards](#st4-the-patches-to-shipped-cards)
- [ST5. Decisions across the set](#st5-decisions-across-the-set)
- [ST6. The issues](#st6-the-issues)
- [ST7. The order](#st7-the-order)
- [ST8. Fixes to the list](#st8-fixes-to-the-list)
- [ST9. Questions for the designer](#st9-questions-for-the-designer)
- [ST10. What every issue body holds](#st10-what-every-issue-body-holds)

---

## ST0. What the designer asked for

- **A new set, Stinkyscape**: 173 numbered cards (two numbers used twice, so 175 cards), 30 tokens on
  sub-numbers (#4.1 to #150.1), and about 13 tokens the list names without a number (the Dormant
  Starship, three drones and the Battleship's three layers, six Resources, Anomaly Found!), plus 10 AI
  cards the Marketplace offers.
- **Four new mechanics** the set is built around: **Dormant**, the **Starship** (Starship Pieces fused
  into a Dormant Starship as they die, launched for (3)), **Tradable**, and an in-match economy of
  **Resources**, **Credits** and the **Marketplace**, with **Anomalies** that swap your board for a preset
  one.
- **Twelve patches to shipped cards** (ST4): six gain a tag, four lose one, Showdown and Prestige gain
  Tradable, and Prestige needs a target.

## ST1. The set in numbers

| | Count | Notes |
| --- | ---: | --- |
| Numbered cards | 175 | #1–#173, with #21 (Moonberry, Aerelret) and #122 (Vamp Rounds, Frost Module) used twice |
| Tokens on sub-numbers | 30 | #4.1–#4.6 Terminus, #8.1, #10.1–#12.3 the nine Other-Origins spells, #18.1–#18.3 the Brains, #84.1, #112.1, #135.1, #137.1–#137.4 War Doctrines, #147.1, #147.2, #148.1, #150.1 |
| Unnumbered tokens | 13 | Dormant Starship; Drone Interceptor, Drone Corvette; Drone Battleship's Shield, Propagation and Meltdown Layers; Reknite, Korrelite, Gellium, Axnit, Narcor, Red Narcor; Anomaly Found! |
| AI cards | 10 | "Create 10 AI Stinkyscape cards and always offer one of those" (as T-AI-1 to T-AI-10 and M #42's AI lot, R1000) |
| **Catalog entries** | **≈ 228** | Meditative was 132, Meditative+ 30 |

By type (numbered and sub-numbered): 96 Units, 77 Spells, 20 Field Spells, 8 Traps, 2 Field Traps and 2
"Swarm" (#91, #92; ST9 Q26). By printed rarity: 44 Common, 51 Rare, 52 Epic, 39 Legendary, 19 Mythic.

**Ids.** `SetName::Stinkyscape`, written `"Stinkyscape"`; cards `stinkyscape-NNN`, a token a card defines
`stinkyscape-NNN-k`, index strings `"4"`, `"4.1"`; scripts in
`crates/cards/src/scripts/stinkyscape/cNNN_slug.rs`. The set takes its place in `CATALOG_SETS` after
Meditative+, its catalog section is §8.10 and its milestone BUILD M12.

## ST2. What the engine already has

Most of the list's words are already engine primitives, and the set's cards are written in their terms:

| The list says | Already in the engine as |
| --- | --- |
| Taunt, Rush, Charge, First Strike, Poisonous, Lifesteal, Reborn, Divine Shield, Trample, Cleave, Pierce, Windfury, Deft, Ranged, Flying, Armor, Indestructible, Immutable, Immune to Spells, Can't attack, Can't be in Defense Position, Untributable ("can't be tributed"), Spell Damage | §6.1 keywords |
| Stack, Equip ("Equipt"), Brittle, Animated, Cloaked, Crater, Flashback, Lucky, Luck | §6.1 keywords |
| Cry, Death, Start/End of turn, Float and "Float reduces cost", Activate / Activate X / Activate ♾️ (X is uses per turn), Aura, Echo X, Cast on draw, Quickdraw, Reveal | §6.2 triggers |
| Discover, Fuse, Embiggen, Lock, Allure, Freeze, Plague Counter, Refresh, Redirect, Steal, Transform, Set stats, Make Radiant, Buff and Nerf, Extra turn, Win the game, Autopilot, Answer a question (C+ #42), lose mana next turn | §6.3 verbs |
| The night market's shop prompt (M #42 CN Flea Market, R1000) | the base for the Marketplace |
| The engine's judge of the optimal play (ME-PARETO, M #71) and the autopilot (M+ #18, R1183002) | the base for "an AI plays / helps you" |
| The lethal guard (M #92 Unan, R1204) | the Shield Layer's and Last Word's "fatal damage" |
| Cards the list names: Book of Buff (C+ #71), University (M #97.6), Fiery Waraxe (M #66), Wardrum (C+ #37), Bigot (Core #2), CN Peptides (M #36), Auspicious Rock (M #39.1), Felinor Token, Rush Token, the three Calls to Chaos, Zephrys | the catalog |

Two words clash with ones the engine already uses, and need a decision (ST9 Q3): the engine calls a card
beneath a Stack **dormant** (§3.2), and Jlarna's **credit** line is already a field on a seat's view
(R1223). "Trade" is also the AI's word for a combat exchange, and "Duo" names the Duos mode; those two
are internal and the foundation can name around them.

**`MAX_MANA` is 4.** Drone Core VIII to X (6, 6 and 8) are played through "Float reduces cost", as the list
intends; the Marketplace's price for a card is read off its cost, so a 5-mana price only arises for those.

## ST3. The new systems

Each is built once, by one issue, with its engine tests on fixture scripts, its §6 row and its rulings; the
issues that need it wait for it. **Default** marks the reading used unless the designer answers otherwise.

### Dormant (S01)

A Dormant card is on the field and can't attack, be attacked, be targeted, or be reached by any effect
(destroy, exile, damage, bounce, transform, steal, Allure, Freeze, buffs) until it awakens; its auras and
triggers are off. "Dormant for N turns" counts its controller's turns and it awakens at the start of the
Nth; "When this awakens" is a new trigger. **Default**: Hearthstone's Dormant — it fills its zone, it is
not summoning sick when it awakens, and it counts for board space but not as a Unit for "count your
Units". Cards: #17 Imprisoned Ship Smith, #19 Imprisoned Cosmality, and the Dormant Starship.

### The Starship (S02)

- **Starship Piece** is a keyword with no rule of its own until a piece enters its owner's graveyard: then a
  copy of it fuses into its owner's **Dormant Starship** (Fuse, R77: summed stats, the union of keywords,
  the scripts kept), summoning one first if they have none — into the leftmost open unit zone, else stacked
  on top of their lane-3 unit zone, Locked or not. The original stays in the graveyard.
- Equip does not pass the keyword on: a piece equipped onto a Unit that is not a piece leaves it not a
  piece.
- The Dormant Starship is Dormant (above), and a card played onto it by Stack is exiled before it resolves.
  It is not a token, so it reaches a graveyard.
- **Activate: (3)** awakens it, which **launches** it: every fused Cry fires, then every fused "When
  launched"; it takes the **Starship** keyword and a random name from the list, a tagged name only when the
  Starship holds that tag (The Ninth Life ~ Felinor, Frontier's Promise ~ Human, The Celestial Archive ~ KY,
  The Lantern Ferry ~ CN, a Jlockeed name, Ember of Departure and The Silent Choir for any).
- The launch is an event: Spacerace Stimulus reveals on it, Prepare For Liftoff makes it cost (0), and the
  Starship remembers which pieces it holds (Space Doctrine's "the Starship Pieces that are part of the
  launched Starship"). "Building a ship" (#137) means having a Dormant Starship.
- **Default**: one Dormant Starship per player at a time; after a launch the next piece to die starts a new
  one; it starts as a 0/0 holding nothing.

Cards: the Dormant Starship, #90, #106, #131, #133, #136, #137 and its four War Doctrines, #144. The
Drone Battleship's three layers carry the Starship keyword too (S07).

### Tradable (L2, live)

Hearthstone's Tradeable, as the list writes it: in your main phase, put a Tradable card from your hand into
your deck at random and draw a card. **One Trade per player per turn**, over all Tradable cards (The Trade
Union's Radiant face makes it two). **Default**: a Trade costs no mana (the list names none). The client
borrows the Duos pass pop-up (§13). Because Showdown and Prestige gain it, the keyword ships as its own
patch, live when it merges, as Freeze did for Meditative+ (#1174).

### Resources and Credits (S04)

- A **Resource** is a (0) Spell token that does not take a hand slot: each player has a Resource tray
  beside the hand, holding at most 8; one gained past 8 is sold at once for its current sell price.
- **Credits** are a per-player count, public, starting at 0, gained by selling Resources (and Spice, #89),
  from Paycheck (#112.1) and Megabase (#116), spent at the Marketplace.
- "Gain a random Stinkyscape Resource" rolls the rarity table (Common 55, Rare 25, Epic 14, Legendary 5,
  Mythic 1), then uniformly among the Resources of that rarity; a Radiant gain makes it Radiant.
- The six Resources: Reknite, Korrelite (Common), Gellium (Rare), Axnit (Epic), Narcor (Legendary),
  Red Narcor (Mythic), each with a fixed buy price and a sell range per face.
- The asteroids (#169–#173) are your mining nodes: **default** — "Activate: Take N damage. Tribute this" is
  your hero taking N damage and the asteroid tributed, so its Death pays out its Resources to its
  controller; the Radiant face drops the damage.

### The weapon rack (S06)

The **Weapon** tag and pools by rarity ("Discover 2 Rare Weapons"); Discover-and-Equip, where the picks
equip straight into the Unit without passing through a hand; "{has an extra turn cooldown}" (the ability
can't be used on the turn after it was); "{deals half damage to heroes}" (**default**: rounded down); and
Prototype-X2's "Float reduces Tribute requirement by 1" (**default**: each Float lowers it by 1, never below
0).

### The drones (S07)

Drone Interceptor and Drone Corvette (tokens rolled with random combat keywords as they are made), and the
**Drone Battleship**: a summon of three layers stacked in one zone, Meltdown at the bottom, then
Propagation, Shield on top, so the Shield Layer is fought first (Stack, §3.2). The Shield Layer's "Fatal
damage a friendly ally would take is redirected to this" is the lethal guard (R1204) for Units. The Drone
Core tag and the ten Drone Cores, with Great Replacement Theory's chain.

### Staged Quests (S09, S10)

The **Quest** tag (In Too Deep gains it, ST4) and a quest card of stages: each stage a goal and a reward,
the current stage public on the card. Tylannt's "Complete the current stage on all your Quests" reaches In
Too Deep's quests and the staged ones. The Greatest Heist (S09) and Job Application (S10, with its resume,
interview and Paycheck) are the two staged quests.

### Anomalies (S20)

**Anomaly Found!** exiles your board and lays one of ten preset boards in its place: the base face rolls
among seven (WH, BF, AL, AT, SP, MT, DH), the Radiant face among three (AN, AA, FO). The boards are data;
"Lane 1 backrow Drone Interceptor" is legal, since a Flying Unit may stand in the backrow (R1081000); an
asteroid of AN or AA carries a restricted Resource pool. The Scanning Probes find Anomalies, each miss
raising that player's chance on later probes. **Default**: only your side is replaced; nothing placed fires
a Cry; a Dormant card on your board stays and the preset card for its zone is skipped.

### The Marketplace (S21)

Reuses the night market's shop prompt (M #42, R1000). **Default**: it opens for both players from the
first time either gains a Resource or Credits (so a game without the set never sees it); one offering for
both players, a stock per player; buying costs Credits, never mana, in your main phase; a Resource bought
goes to the tray. Offerings re-roll every 6 turns: four of the Resources (stock 4–8), three of the twelve
v3–v5 Weapons, both Warp Disruptors and both Scanning Probes (stock 6 and 3), two Drone Cores, one Launcher,
two Starship Pieces, two Stinkyscape cards and one of the 10 AI cards (stock 1–3 for the rest). A card is
Radiant 20% of the time at twice the price. Prices move every turn around the formula ST9 Q6 settles,
Resources' buy prices excepted. Call to Chaos (Scape Edition) adds a rest-of-game Call to Chaos lot at 20.

### The rest (each in its own issue)

| System | Cards | Issue |
| --- | --- | --- |
| Votes: each Brain votes at your start of turn on its condition; all three in one turn removes you, a loss with its own ending (a server migration at the release) | #18, #18.1–#18.3 | S11 |
| The AI at the wheel: the autopilot bounded to N actions as an aura (#46), for a whole turn (#47); the Pareto judge shown as a hint to you (#147); a free helpful action at your start of turn (#147.2) | #46, #47, #147, #147.1, #147.2 | S12 |
| Outside data: a build-time "BUSTED" list (scripts over 100 lines, read by `build.rs`, never at runtime); "complicated cards"; a nerf hidden from the owner of the card until it is played; a trade offer at the opponent's start of turn; zone coordinates for a Jacobian | #109, #111, #123, #77, #25 | S18 |
| The Terminus pool and its six endings: exile everything, an exam question with Echo, extra turns, a trap on lethal damage, one lane left, every Call to Chaos | #4, #4.1–#4.6, #154 | S22 |
| The starscape map: a seeded map of nodes with rewards, at least 20 travels to its edge; Edge of the Universe wins by fleeing, and the loser keeps their rating (ladder work on the server) | #148, #148.1 | S23 |
| Two new Call to Chaos tables, each with ten effects, three at once on the Radiant face | #149, #150, #150.1, #151 | S24 |

## ST4. The patches to shipped cards

They change shipped games, so each is a patch of its own that goes live when it merges (with its pending
fragment), never held behind the set's gate:

| Issue | Cards | Change | Why the set needs it |
| --- | --- | --- | --- |
| L1 | Core #100 Ceaseless Void; C #90 In Too Deep; C+ #32 Otherworldly Removal; C+ #32.1 Execute, #32.2 Brawl, #32.3 Blade Storm; C #32 Felinor Feelings; Core #62 Friend of Felinors; C+ #68 Organic Produce; C+ #63 Fruit Tree | Gain Terminus, Quest, Otherworldly and Other-Origins (×3); Felinor Feelings and Friend of Felinors lose Felinor; Organic Produce and Fruit Tree lose Fruit | Olivian Expanse discovers Terminus cards; Tylannt completes Quests; Moonberry adds Other-Origins cards; Fruit Launcher casts a random Fruit, so the two Field Spells that make fruit must not be one. The four new tags enter the engine's `Tag` union and the server's `cards_tags_check` here |
| L2 | M #98 Showdown, M #53 Prestige | Gain Tradable, with the keyword itself (ST3) | The set's Tradable cards use the same rule |
| L3 | M #53 Prestige | Playable only while a card to De-Radiant exists | — (after L2: the same card) |

## ST5. Decisions across the set

- **D1 The gate.** Stinkyscape is held by R1420 until the release (R) ships it, as Meditative+ was
  (R1178000): no set part adds a pending fragment or moves a golden trace; the release claims every entry in
  one fragment.
- **D2 R275's stat half** holds: every Radiant Unit at twice its stats, so Falcon's Radiant face is 8/2 and
  Prototype-X2's 16/16 (ST8).
- **D3 New state stays out of shipped games' hashes.** The Resource tray, Credits, the Marketplace, a
  Starship's memory, quest stages and votes are optional fields skipped when absent
  (`docs/meditative-set.md` D14).
- **D4 Purity** (CLAUDE.md rule 4). Price moves, rolls and the map draw from the match's `Rng`; "BUSTED"
  is computed at build time; nothing in the engine sends an email (ST9 Q11).
- **D5 Numbers** (CLAUDE.md rule 9). The tray's 8, the rarity table, the 6-turn cycle, every stock range,
  the price formula and its spread, the 20% Radiant chance and its 2× premium, the (3) launch, Wild System's
  20 attacks, Masochism Unmasked's 20 and 40, Alteran Reach's 20 travels: named constants in
  `crates/engine/src/config.rs`.
- **D6 Words.** "Stinkyscape" everywhere ("Sinkyscape", "Stinkscape" are typos); "Starpiece" is Starship
  Piece; "Indestructable" is Indestructible; "Equipt" is Equip; "Book of Buffs" is Book of Buff (C+ #71);
  house style as R366, R373 and R432 set it.
- **D7 Rulings.** Each issue takes its R-numbers from its own block, R(N×1000) up (`docs/issues-and-patches.md`,
  Ruling numbers), and names its proving tests after them (CLAUDE.md rule 3).

## ST6. The issues

One tracker, `Patch: the Stinkyscape set` (labels `major version`, `patch`), holds the checklist below.
Each part is a sub-issue of it for the record, but **no part is titled `(part n of m)`**: the night bot
builds such parts strictly in order (`bot/harness/queue.py`, `waits_for`), and these build in parallel,
each waiting only for the issues its `Blocked by` line names. Difficulty follows CLAUDE.md rule 11.

**Live patches** (each goes live when it merges):

| Id | Title | Blocked by | Difficulty |
| --- | --- | --- | --- |
| L1 | `Patch: Terminus, Quest, Otherworldly and Other-Origins tags on shipped cards, and Felinor and Fruit off four` | — | medium |
| L2 | `Patch: the Tradable keyword, on Showdown and Prestige` | — | hard |
| L3 | `Patch: Prestige can be played only with a card to De-Radiant` | L2 | medium |

**The set** (behind the gate; no fragment):

| Id | Title (`Patch: Stinkyscape …`) | Cards | Systems it builds | Blocked by | Difficulty |
| --- | --- | --- | --- | --- | --- |
| F | `the foundation` | — | `SetName::Stinkyscape` held; the tags Drone Core, Weapon, War Doctrine (and Swarm, Q26); Starship Piece as an inert keyword pools can read; the card-by-card brief from this plan; §8.10, §7, BUILD M12 and `docs/radiant-audit.md` rows; the census and the fixture; a dry run on one card | — | hard (the orchestrating session) |
| S01 | `S01, Dormant` | #17 Imprisoned Ship Smith, #19 Imprisoned Cosmality | Dormant, "for N turns", "When this awakens" | F | hard |
| S02 | `S02, the Starship` | Dormant Starship, #90 After Burner, #106 Warpfield Amplifier, #131 Spacerace Stimulus, #133 Ultra Capacitor, #136 Magic Module, #137 Firststrike Capability, #137.1–#137.4 Land, Sea, Air and Space Doctrine, #144 Prepare For Liftoff | fusion on the graveyard, the launch, its event and memory, the names | F, S01, L2 | hard |
| S03 | `S03, Starship Pieces on existing keywords` | #38 Shield Amplifier, #40 Poison Capsule, #43 Healing Capsule, #48 Ultralight Chassis, #49 Spacemine, #68 Startos, #69 Falcon, #72 Paragon Mk1, #73 Paragon Mk2, #93 Patcher Nanobots, #98 Hull Booster, #100 AP Rounds, #101 Overdrive Module, #107 RR-5000, #108 Trila 3, #122 Vamp Rounds, #122 Frost Module, #132 Defensive Crystal, #134 Felinor Containment Chamber, #158 Stratos Swarm | none | F | medium |
| S04 | `S04, Resources and Credits` | Reknite, Korrelite, Gellium, Axnit, Narcor, Red Narcor; #39 The Narcor Project, #67 Tractor Beam, #75 Badger, #76 Honey Badger, #84 Power Cell, #84.1 Metal Scraps, #89 Spice, #96 Ocean Tsunami, #97 Asteroid Field, #116 Megabase, #155 Comet Crash, #169 Tiny, #170 Medium, #171 Large, #172 Huge Asteroid, #173 Abandoned Spice Tower | the tray, Credits, selling, the rarity roll; the client's tray and Credits | F | hard |
| S05 | `S05, the Miners` | #62–#66 Miner v1–v5, #78 Ozark, #79 Orca, #80 Galaxy | none beyond S04's and S06's | S04, S06 | medium |
| S06 | `S06, the weapon rack` | #50–#61 Beam, Repeater, Cannon and Flak v3–v5, #81 Radix, #82 Phalanx, #83 Halo, #85 Prototype-X1, #86 Prototype-X2, #140 Rush Token, #141 Fruit, #142 Jade and #143 Book Launcher | Weapon pools, Discover-and-Equip, the cooldown, half damage to heroes, Float lowers Tribute | F, L1 (#141's Fruit pool) | hard |
| S07 | `S07, the drones` | Drone Interceptor, Drone Corvette, the Battleship's three layers; #27–#36 Drone Core I–X, #41 Drone Sympathizer, #42 Core Researcher, #44 Great Replacement Theory, #45 Drone Commander, #168 Drone Hanger | the layered summon, the lethal guard for Units, the Drone Core chain | F | hard |
| S08 | `S08, the trade cards` | #87 Siva 1, #164 The Syndicate, #165 The Trade Union | Tradable given by an aura, two Trades a turn, a Trade's outcome rolls | F, L2 | hard |
| S09 | `S09, staged Quests` | #6 Tylannt, #7 The Greatest Heist | staged quests, "complete the current stage", stealing mana crystals | L1, L2, S04 (the Anomaly half of Stage 2's pool fills when S20 lands) | hard |
| S10 | `S10, Job Application` | #112 Job Application, #112.1 Paycheck | the resume's evaluation, the interview, the offer loop | S04, S09 | hard |
| S11 | `S11, the three Brains` | #18 Imitated Instrumentality, #18.1 MELCHIOR, #18.2 BALTHASAR, #18.3 CASPER | votes and the vote-out ending, its animation | F | hard |
| S12 | `S12, the AI at the wheel` | #46 Circadian Halfcycler, #47 Circadian Fullcycler, #147 IN33DH3, #147.1 IN33DH3 Prime, #147.2 Wardrum Blob | the bounded autopilot as an aura, the Pareto hint, the free helpful action | S07 | hard |
| S13 | `S13, spells on existing primitives` | #1 Freeze Ray, #8 Conjure Rosemary Triscuit, #8.1 Rosemary Triscuit, #10–#12 the three Otherworldlies and #10.1–#12.3 their nine spells, #21 Moonberry, #145 Fortify Flesh, #146 Flash Freeze, #156 Catalyst Comet | none | F, L1 | medium |
| S14 | `S14, spells that bend a rule` | #2 Absolute Zero, #3 Meteor, #13 Asphyxiants Cloud, #99 No-No Words, #102 Book of Nono-Words, #103 Vxyl, #117 Core System, #118 Secure System, #119 Unsecure System, #120 Wild System, #121 System Schematic, #153 Stimmycheck | Freeze forever, a Spell's Trample, Poisonous Spells, a fixed X, "lose N mana next turn", the melee, Inflation | F | hard |
| S15 | `S15, traps and reactions` | #5 Masochism Unmasked, #9 Ticketed, #21 Aerelret, #22 Gatinsir, #23 Warp Disruptor, #24 Warp Disruptor II, #135 Sacked, #135.1 Sack, #138 Pocket Peptides, #139 Wormhole | chance reveals, AMBUSH, a "beneficial Spell", a Unit held in a Sack | S03, S06 | hard |
| S16 | `S16, auras and rule benders` | #14 Arm of the Jempire, #15 Arg Arg Defender, #16 Rod of Ages, #70 Z'Zoth, #71 A'Zoth, #74 Paragon-X, #91 The Propagation's Wasp, #92 The Propagation's Hornet, #94 Tmack Trav, #104 Reflective Plating, #105 Disintegrex, #152 Sanctum, #157 The Gauntlet | Immune to Spell Damage, Evolve, destroy-becomes-exile, the Gauntlet's lanes, a Radiant at 10 keywords | F | hard |
| S17 | `S17, the Acclaimed` | #20 Tnack Trav, #26 ZolarKeth, #37 Kwzka, #88 Siva 2, #110 iKwzki, #113 A Very Dandy Lad, #114 Arctic Citadel, #115 Scornboi | copy a Unit's text, Duo and Squad (a Radiant mark), exile auras | S03 (#26's piece pool) | hard |
| S18 | `S18, outside data and hidden hands` | #25 KY's Tricknology, #77 Yuma, #109 xxFreshJack, #111 Shockwave447, #123 Gokugains | see ST3, The rest | F, S03 | hard |
| S19 | `S19, Jlockeed's space program` | #124–#130, the seven Jlockeed cards | none | S03 (the piece pool) | medium |
| S20 | `S20, Anomalies and the Scanning Probes` | Anomaly Found!, the ten boards, #166 Scanning Probe Mk1, #167 Scanning Probe Mk2 | the board swap, the probes' rising chance | S04, S06, S07, S08, S15, S16 | hard |
| S21 | `S21, the Marketplace` | the 10 AI cards | the market, its cycle, stock and prices; the client's shop; the AI's buying | S03, S04, S06, S07, S15, S20, and Q8's approval | hard |
| S22 | `S22, Terminus` | #4 Olivian Expanse, #4.1 Farewell, #4.2 KY's Last Exam, #4.3 Event Horizon, #4.4 Last Word, #4.5 1v1 Final Destination, #4.6 The Big Crunch, #154 Dice of Doom | the Terminus pool, the lethal trap, the exam | F, L1 | hard |
| S23 | `S23, Alteran Reach` | #148 Alteran Reach, #148.1 Edge of the Universe | the starscape map, the fled ending and its rating rule | F | hard |
| S24 | `S24, the bundles` | #95 Vanguard Coalition, #159 ETHICS, #160 Golden Accord, #161 Pegasus, #162 Star-jumper Core, #163 Swear Loyalities | "exile the other options wherever they are" | S03, S04, S05, S08, S09, S16, S17, S18 | medium |
| S25 | `S25, the two Calls to Chaos` | #149 Wildar, #150 Call to Chaos (Stinky Edition), #150.1 Stinkyscape Golem, #151 Call to Chaos (Scape Edition) | two Chaos tables | S02, S07, S09, S10, S13, S14, S17, S20, S21, S23, S24 | hard |
| AI | `the AI with the new cards` | all | every new prompt and action answered (Trade, launch, buy, travel, vote, exam); the sweep, at most half banned (R186) | every S part | hard |
| AV | `sound, visuals and polish` | all | intro music for every Legendary and Mythic, voice files (#17's line), the set mark, the Almanac's and deck builder's chips, every face read in the browser | every S part | hard |
| R | `the release` | all | the gate opens; one fragment; the migrations (the tags, the vote-out and fled endings); deck codes at 5000 + n (R1410); the golden traces re-blessed once; totals; the `RELEASES` line; REVIEW.md's audit | every other part | hard (the orchestrating session) |

Every one of the 175 numbered cards and 30 sub-numbered tokens is in exactly one row, and every card a
card names (a bundle's contents, a pool a test needs, an anomaly's board) is in a row its issue is blocked
by (both checked against the list).

## ST7. The order

1. **Now, in parallel:** L1, L2 and F. L3 after L2.
2. **After F:** S01, S03, S04, S06 (and L1), S07, S11, S13 (and L1), S14, S16, S22 (and L1), S23.
3. **Then:** S02 (S01, L2), S05 (S04, S06), S08 (L2), S09 (L1, L2, S04), S12 (S07), S15 (S03, S06), S17,
   S18 and S19 (S03).
4. **Then:** S10 (S09), S20 (S04, S06, S07, S08, S15, S16).
5. **Then:** S21 (S20, and the AI cards approved), S24 (S05, S08, S09, S17, S18).
6. **Then:** S25.
7. **Last:** AI and AV, then R.

## ST8. Fixes to the list

What the build reads, unless the designer says otherwise:

| # | Card | The list says | The build reads |
| --- | --- | --- | --- |
| 21, 122 | Aerelret; Frost Module | the same number as Moonberry; as Vamp Rounds | #174 and #175 (Q19) |
| 25–47, 124, 125, 152, 153 | KY's Tricknology, ZolarKeth; the Drone Cores, #41, #42, #44–#47, #124, #125, #152, #153 | Sinkyscape, Stinkscape | Stinkyscape |
| 69 | Falcon | Radiant 8/1 | 8/2 (D2) |
| 86 | Prototype-X2 | Radiant 8/8 | 16/16 (D2) |
| 12 | Otherworldly Nightmare | Radiant adds Execute, Brawl, Blade Storm | Radiant Shadow Word Death, Shadow Word Pain, Psychic Scream (Q20) |
| 95 | Vanguard Coalition | Radiant face drops Vxyl | Radiant Vxyl too (Q20) |
| 120, 162 | Wild System, Star-jumper Core | Radiant face the same as the base | Star-jumper Core's Radiant adds Radiant cards; Wild System keeps one text (Q20) |
| 124 | Jlockeed's Shipbuilding Grant | tag "Jlockeed's" | Jlockeed |
| 125 | Jlockeed's Spacerace Propaganda | a Spell with Activate, no Jlockeed tag | a Field Spell, tagged Jlockeed (Q21) |
| 164 | The Syndicate | a Spell with an Aura | a Field Spell (FO's backrow places it) (Q21) |
| 116 | Megabase | a Field Spell with Health, attacked and dying | Animated (Q21) |
| 155 | Comet Crash | Crater on a Field Spell (Crater does nothing there, §6.1) | Crater extended: it Locks its backrow zone (Q21) |
| 45 | Drone Commander | no trigger word | Cry |
| 167 | Scanning Probe Mk2 | Radiant "Luck 3" | Lucky 3 (Luck X is a different, player-wide keyword) |
| 173, 75, 76 | Abandoned Spice Tower; Badger, Honey Badger | "Add a three Spice"; "add gain a random" | "Add three Spices"; "gain a random" |
| 163 | Swear Loyalities | "Golden"; the name | Golden Accord; the name kept as written unless Q19 says Loyalties |
| 169–172 | the asteroids | "Can't be in Defense Positio n" | Position |
| anomalies | AT, MT, AN | "Small Asteroid" | Tiny Asteroid (#169) (Q19) |
| 13 | Asphyxiants Cloud | "should deal 1 poisonous damage to all enemies, if it doesn't please reword" | "This and your next Spell this turn have **Poisonous**. Deal 1 damage to all enemies. Draw 1." |

## ST9. Questions for the designer

Each has the default the build uses; the ones marked **blocking** change how an issue is built, so their
issue waits for the answer.

**The systems (blocking)**

1. **Launch = Awaken?** Is launching a Starship the same as paying (3) to awaken the Dormant Starship? Does
   it start 0/0, holding only what is fused into it? One at a time, a new one started by the next piece
   after a launch? Does "enters the GY" mean dying only, or discards and mills too? Do the names (The
   Ninth Life …) do anything, and what is the Jlockeed one called? *Default: yes; 0/0; one at a time; any
   arrival in the graveyard; names only; "Jlockeed's Flagship".*
2. **The War Doctrines' Starship lines** ("your Starship attacks a random enemy unit", "heal your hero for
   twice … your Starship", …): once, as the doctrine is cast, or every turn for the rest of the game? And
   with no launched Starship? *Default: once, on cast, from the Starship just launched; the keyword half is
   for the rest of the game.*
3. **"Dormant" clashes** with the engine's word for a card beneath a Stack. Keep Dormant for the new keyword
   (the Stack wording moves to "beneath a Stack"), or name the keyword something else? *Default: keep
   Dormant.*
4. **Is the Marketplace in every game?** *Default: it opens the first time either player gains a Resource
   or Credits, so a game with no Stinkyscape card never sees it.* One offering shared by both players with a
   stock each? *Default: yes.* Opened from a button in your main phase, at no mana? *Default: yes.*
5. **Resources and the hand.** Does the opponent see your Resources or only their count? Are they "in your
   hand" for discards, steals and "fill your hand" (Ocean Tsunami #96 fills the tray to 8?)? Does an
   over-cap Resource sell at its current price? *Default: count only; not hand cards (Narcor Project's
   "Discard a Narcor" discards from the tray); Ocean Tsunami fills the tray; yes.*
6. **The price formula.** "10 Credits ~ 1 mana", "0 mana around 5", "every mana above 1 … 5 Credits" and
   "a 5 mana card ~45" don't agree. Which is it: (a) 5 + 10 × cost (0 → 5, 1 → 15, 4 → 45); (b) 5 + 10 ×
   (cost − 1), at least 5 (1 → 5, 5 → 45); (c) 10 × cost, 0 → 5 (5 → 50)? And is the spread "SD ≈ a tenth of
   the price"? *Default: (a), spread a tenth of the price.*
7. **Anomalies.** Do they replace only your side? Does anything placed fire its Cry (Halo and Phalanx
   discover Weapons)? *Default: your side only; no Cry.*
8. **The 10 AI Marketplace cards.** Should the foundation draft them for your approval? *Default: yes, drafted
   in F, approved by you before S21 builds them.*
9. **Removed by the Brains.** Does a vote-out count as a ranked loss like a concession? Does it take all
   three in the same turn? *Default: a loss; all three at one start of turn.*
10. **Edge of the Universe and the ladder.** "Your opponent does not lose elo" — does the winner still gain?
    *Default: the winner gains as usual; the loser's rating is unchanged; on the Radiant face the winner
    chooses.*
11. **Job Application's email Easter egg.** The engine can't send mail and the server would need each
    player's address and a mail service: drop it? *Default: dropped.*
12. **xxFreshJack's pools.** "Pools that are normally restricted to single-player only game modes": which
    are they (the Duos AI team's cards, the tutorial's, the AI cards)? And are Shockwave447's "complicated
    cards" the same list as BUSTED (scripts over 100 lines)? *Default: the AI cards and the Duos AI team's
    cards; yes, the same list.*
13. **IN33DH3 and Wardrum Blob's "AI help".** Is it a hint (the best action lit up, from your own view
    only), and "do something helpful" a free Zephrys-style pick cast for you? *Default: yes to both.*
14. **KY's Tricknology's Jacobian.** Which coordinates do zones have, and do enemy zones count on the base
    face (the pieces landing there being the enemy's)? *Default: your lanes x = 1–5, your unit row y = 1 and
    backrow y = 2, the enemy rows y = −1 and −2; the parallelogram J·[0,1]² at the chosen zone, every zone
    whose centre it covers; enemy zones count on the base face and their pieces are the enemy's.*

**Cards (the build goes ahead on the default)**

15. **Circadian Halfcycler** (an Aura): the first 3 actions of *every* turn of yours while it stands?
    *Default: yes.*
16. **Masochism Unmasked**: up to 20 per turn, or 20 over the trap's life? *Default: per turn.*
17. **1v1 Final Destination** (#4.5) is a Trap with no reveal condition. *Default: a Spell.*
18. **Orca** (#79) has an Embiggen price (5) but no Embiggened text. *Default: the Embiggened version equips
    both offered Miners.*
19. **Names and numbers.** #21 and #122 are used twice (default: the second of each becomes #174 and #175);
    Tnack Trav (#20) and Tmack Trav (#94) are two cards (default: yes); Startos (#68) vs Stratos Swarm
    (#158) (default: the Unit is Stratos); the anomalies' Small Asteroid is Tiny Asteroid (#169) (default:
    yes); Drone Hanger or Hangar (default: Hanger as written); Swear Loyalities or Loyalties.
20. **Radiant faces** ST8 fills in: Otherworldly Nightmare, Vanguard Coalition, Star-jumper Core, Wild
    System.
21. **Card types** ST8 changes: Spacerace Propaganda and The Syndicate to Field Spells, Megabase Animated,
    Crater on Comet Crash.
22. **The Greatest Heist**: "Steal your enemy's mana crystals" (default: they lose their maximum mana and you
    gain it, up to the cap); "or a Jellybean" — no card is called Jellybean (default: a Rosemary Triscuit?
    tell us which card).
23. **Warp Disruptor**: "Units cannot be in defense position, Units have +2 attack": every Unit on both
    sides? *Default: yes; the attack response is your side's.*
24. **Scornboi's** exile aura: Units, or every permanent? *Default: Units.*
25. **Aerelret and Gatinsir's AMBUSH**: a label, or a rule (the summoned Units join the defence)? *Default:
    a label; they are summoned before the attack resolves, which goes on as declared.*
26. **Swarm** (#91, #92): a card type, or a Unit tagged Swarm? *Default: a Unit tagged Swarm.*
27. **Wormhole**'s "beneficial Spell": *default — a Spell whose chosen targets are all its caster's own
    hero or permanents.*
28. **Rush Token Launcher**: a Rush Token can't hit a hero the turn it is summoned. *Default: it targets an
    enemy Unit only.*
29. **Great Replacement Theory** (#44) shares its name with a white-nationalist conspiracy theory. Keep it,
    or rename it?

**The issues**

30. Shall the issues be opened now with these defaults (the blocking ones waiting for answers), and who
    builds them: the night bot (`bot:build`), Squishy, or people (`human`)? *Default: L1–L3 and the S parts
    for the night bot, F and R for the orchestrating session.*

## ST10. What every issue body holds

- **Scope**: the cards (index, name, the designer's text) and the systems, from ST6.
- **Reading**: the brief's entries for those cards (after F), with every ⚠ default named.
- **Blocked by #n** lines, one per blocker, so the queue reads them.
- **Rulings**: the block it owns, R(N×1000) up.
- **Done when**: each card's base and Radiant tests in its script file (CLAUDE.md rule 6); its BUILD M12
  rows green; `cargo jackioh catalog check`, `patches check` (no fragment for a set part), `spec check` and
  `fuzz --seeds 200` green; a new event's `ANIMATIONS` and `SOUND_CUES` rows; a new prompt answered by the
  AI, the random policy and the timeout; the client's screen for the system, if it has one.
- **Labels**: `patch` (with `major version` for the tracker and the set's parts), one `difficulty:*`, and who
  builds it.
