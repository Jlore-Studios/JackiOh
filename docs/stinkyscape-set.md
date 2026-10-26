# Stinkyscape — the plan of issues

2026-10-25 · written from the designer's list of the same day (`JackiOh_Stinkyscape_Cards.md`), with the
designer's answers of the same day (ST9)

> **Status: the plan of record for opening the issues.** SPEC is still the only source of rules
> (CLAUDE.md). This file reads the designer's list against the engine as it stands on `main`, names what
> the set reuses and what it must build, records what the designer decided, and splits the work the way
> the designer asked (ST6): the mechanics first, each merged on `main` without the set; then three
> fullsends — the foundation and the cards, shipped with the AI dealing itself none of the set; the AI
> to 80% of the set; the AI to all of it. The foundation turns this plan into the card-by-card brief, in
> `docs/meditative-plus-set.md`'s shape. A reading marked **default** is the build's where the designer
> said nothing.

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
- [ST9. The designer's answers](#st9-the-designers-answers)
- [ST10. What every issue body holds](#st10-what-every-issue-body-holds)
- [ST11. Reading the brief](#st11-reading-the-brief)
- [ST12. The set's data](#st12-the-sets-data)
- [ST15. The ten AI cards](#st15-the-ten-ai-cards)
- [ST13. Cards, card by card](#st13-cards-card-by-card)
- [ST14. Readings for the designer](#st14-readings-for-the-designer)

---

## ST0. What the designer asked for

- **A new set, Stinkyscape**: 173 numbered cards (two numbers used twice, so 175 cards), 30 tokens on
  sub-numbers (#4.1 to #150.1), about 13 tokens the list names without a number (the Dormant Starship,
  three drones and the Battleship's three layers, six Resources, Anomaly Found!), and 10 AI cards the
  Marketplace offers.
- **The mechanics** the set is built around: **Dormant**, the **Starship** (Starship Pieces fused into a
  Dormant Starship as they die, launched for (3)), **Tradable**, an in-match economy of **Resources**,
  **Credits** and the **Marketplace**, and **Anomalies** that swap your board for a preset one.
- **Patches to shipped cards** (ST4): ten gain a tag (four of them the new Jelly Bean tag), four lose one,
  Showdown and Prestige gain Tradable, and Prestige needs a target.

## ST1. The set in numbers

| | Count | Notes |
| --- | ---: | --- |
| Numbered cards | 175 | #1–#173, with #21 and #122 used twice; the second of each becomes #174 (Aerelret) and #175 (Frost Module) |
| Tokens on sub-numbers | 30 | #4.1–#4.6 Terminus, #8.1, #10.1–#12.3 the nine Other-Origins spells, #18.1–#18.3 the Brains, #84.1, #112.1, #135.1, #137.1–#137.4 War Doctrines, #147.1, #147.2, #148.1, #150.1 |
| Unnumbered tokens | 13 | Dormant Starship; Drone Interceptor, Drone Corvette; Drone Battleship's Shield, Propagation and Meltdown Layers; Reknite, Korrelite, Gellium, Axnit, Narcor, Red Narcor; Anomaly Found! |
| AI cards | 10 | "Create 10 AI Stinkyscape cards and always offer one of those" (as T-AI-1 to T-AI-10 and M #42's AI lot, R1000), designed by the build (ST9 Q8) |
| **Catalog entries** | **≈ 228** | Meditative was 132, Meditative+ 30 |

By type (numbered and sub-numbered): 96 Units, 77 Spells, 20 Field Spells (22 with ST8's two), 8 Traps,
2 Field Traps and 2 Swarm Units (#91, #92). By printed rarity: 44 Common, 51 Rare, 52 Epic, 39 Legendary,
19 Mythic.

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
| Cards the list names: Book of Buff (C+ #71), University (M #97.6), Fiery Waraxe (M #66), Wardrum (C+ #37), Bigot (Core #2), CN Peptides (M #36), Auspicious Rock (M #39.1), the Jelly Beans (Core #26–#29), Felinor Token, Rush Token, the three Calls to Chaos, Zephrys | the catalog |
| R1390's soft gate (`AI_DECK_GATED_SETS`) | the AI dealing itself none of a set, as Meditative shipped (#732) |

Two words clash with ones the engine uses. **Dormant** is the new keyword's, and the spec's "dormant"
for a card beneath a Stack becomes "beneath a Stack" (ST9 Q3); Jlarna's **credit** line is already a
field on a seat's view (R1223), so the Credits' identifiers are named apart from it (`market_credits`).
"Trade" is also the AI's word for a combat exchange, and "Duo" names the Duos mode; the foundation names
the identifiers around both.

**`MAX_MANA` is 4.** Drone Core VIII to X (6, 6 and 8) are played through "Float reduces cost", as the list
intends, and the Marketplace prices a card past (4) at 5 Credits a mana (ST3).

## ST3. The new systems

Each is built once, by one mechanics issue on `main` (ST6, phase 0), with its engine tests on fixture
scripts, its §6 row and its rulings; nothing reaches it until the set's cards do.

### Dormant (M01)

A Dormant card is on the field and can't attack, be attacked, be targeted, or be reached by any effect
(destroy, exile, damage, bounce, transform, steal, Allure, Freeze, buffs) until it awakens; its auras and
triggers are off. "Dormant for N turns" counts its controller's turns and it awakens at the start of the
Nth; "When this awakens" is a new trigger. **Default**: Hearthstone's Dormant — it fills its zone, it is
not summoning sick when it awakens, and it counts for board space but not as a Unit for "count your
Units".

### The Starship (M02)

- **Starship Piece** is a keyword with no rule of its own until a piece **dies**: then a copy of it fuses
  into its owner's **Dormant Starship** (Fuse, R77: summed stats, the union of keywords and tags, the
  scripts kept), summoning one first if they have none — into the leftmost open unit zone, else stacked on
  top of their lane-3 unit zone, Locked or not. The original goes to the graveyard as usual. A discard, a
  mill or any other way into the graveyard fuses nothing (ST9 Q1).
- Equip does not pass the keyword on: a piece equipped onto a Unit that is not a piece leaves it not a
  piece.
- The Dormant Starship starts **0/0**: its stats are only what is fused into it. It is Dormant, a card
  played onto it by Stack is exiled before it resolves, and it is not a token, so it reaches a graveyard.
  One per player at a time; after a launch, the next piece to die starts a new one.
- **Activate: (3)** awakens it, which **launches** it (the same thing): every fused Cry fires, then every
  fused "When launched"; it takes the **Starship** keyword and a random name from the list, a tagged name
  only when the Starship holds that tag (The Ninth Life ~ Felinor, Frontier's Promise ~ Human, The
  Celestial Archive ~ KY, The Lantern Ferry ~ CN, a Jlockeed name, Ember of Departure and The Silent Choir
  for any). The names are names only.
- The launch is an event: Spacerace Stimulus reveals on it, Prepare For Liftoff makes it cost (0), and the
  Starship remembers which pieces it holds (Space Doctrine). "Building a ship" (#137) means having a
  Dormant Starship.

### Tradable (L2, live)

Hearthstone's Tradeable, as the list writes it: in your main phase, put a Tradable card from your hand into
your deck at random and draw a card, at no mana. **One Trade per player per turn**, over all Tradable cards
(The Trade Union's Radiant face makes it two). The client borrows the Duos pass pop-up (§13). Showdown and
Prestige gain it, so the keyword ships as its own patch, live when it merges.

### Resources and Credits (M03)

- A **Resource** is a (0) Spell token that does not take a hand slot: each player has a Resource tray
  beside the hand, holding at most 8; one gained past 8 is sold at once for its current sell price.
  Resources are not hand cards: a discard, a steal or a "fill your hand" never reaches them, and "discard
  a Narcor" (#39) and "fill your hand with Resources" (#96) mean the tray. The opponent sees the tray's
  count only (ST9 Q5).
- **Credits** are a per-player count, public, starting at 0, gained by selling Resources (and Spice, #89),
  from Paycheck (#112.1) and Megabase (#116), and spent at the Marketplace.
- "Gain a random Stinkyscape Resource" rolls the rarity table (Common 55, Rare 25, Epic 14, Legendary 5,
  Mythic 1), then uniformly among the Resources of that rarity; a Radiant gain makes it Radiant. A card
  may carry a restricted pool ("only gives Narcor and Red Narcor").
- The asteroids (#169–#173) are your mining nodes: "Activate: Take N damage. Tribute this" is your hero
  taking N damage and the asteroid tributed, so its Death pays its Resources to its controller; the
  Radiant face drops the damage.

### The Marketplace (M04)

Built on the night market's shop prompt (M #42, R1000):

- It opens for both players the first time either gains a Resource or Credits, so a game with no
  Stinkyscape card never sees it. One offering for both players, a stock for each; you buy in your main
  phase from a button, for Credits and no mana; a Resource bought goes to the tray.
- The offering re-rolls every 6 turns, by a table of lots, each a pool and a count, with a stock range:
  four of the Resources (stock 4–8), three of the twelve v3–v5 Weapons, both Warp Disruptors, both
  Scanning Probes (stock 6 and 3), two Drone Cores, one Launcher, two Starship Pieces, two Stinkyscape
  cards and one of the 10 AI cards (stock 1–3 for the rest). The table is the set's data (S21); the
  mechanic reads any table.
- **Prices** (ST9 Q6): a card of cost c costs **10 × min(c, 4) + 5 × max(c − 4, 0)** Credits, and a
  (0) card 5, so (4) is 40 and (5) 45. A lot is Radiant 20% of the time, at twice the price. Prices move
  every turn, drawn around that price with a spread of a tenth of it; Resources' buy prices never move,
  and their sell prices move within their printed ranges.
- A lot can be added for the rest of the game at a fixed price (Call to Chaos (Scape Edition): a Call to
  Chaos at 20).

### The weapon kit (M05)

"{has an extra turn cooldown}" (the ability can't be used on the turn after it was); "{deals half damage
to heroes}" (rounded down); Equip straight into a Unit from a Discover, a random pick or a choice of two,
once or twice, without passing through a hand; and "Float reduces Tribute requirement by 1" (each Float
lowers it by 1, never below 0).

### Layered summons and the lethal guard (M06)

A summon of several cards stacked in one zone, in order (the Drone Battleship: Meltdown at the bottom,
then Propagation, Shield on top, so the Shield Layer is fought first, §3.2), and "Fatal damage a friendly
ally would take is redirected to this", the lethal guard (R1204) for Units.

### Staged Quests (M07)

The **Quest** tag (In Too Deep gains it, ST4) and a quest card of stages: each stage a goal and a reward,
the current stage public on the card. "Complete the current stage on all your Quests" reaches In Too
Deep's quests and the staged ones. The goals the set's stages need are watchers of their own: make an
Acclaimed Unit Radiant; have two of your Acclaimed Units die; steal an enemy card; play a card that costs
the same as another; play three cards of different costs in a turn; take or deal 12 or more in one
instance. With them: steal your opponent's mana crystals (they lose their maximum mana, you gain it up to
the cap), and "hear back in N turns".

### Anomalies (M08)

The board swap: exile one side's board and lay a preset (Units, backrow cards, positions, Radiant, and a
rider per card such as a restricted Resource pool). **Only the caster's side** is replaced, and every
placed card's **Cry fires, with random targets and choices** (ST9 Q7); a Dormant card on that side stays
where it is and the preset's card for its zone is skipped. With it, a per-player chance that rises on a
miss (the Scanning Probes). The ten boards are the set's data (S20).

### Two new endings (M09)

- **Voted out** (the Brains): each Brain votes at its controller's start of turn on its own condition, and
  all three in the same turn remove that player: **a loss for them and a win for the opponent** (ST9
  Q9), after an animation of the three agreeing.
- **Fled** (Edge of the Universe): **a win for the player who flees and a draw for the opponent** on the
  ladder; on the Radiant face the fleer chooses whether it is a draw or a loss for the opponent (ST9 Q10).
- Both are engine endings, with the server's migration (`results_reason_check`,
  `rated_games_reason_check`), `DECISIVE_REASONS` and the ladder's update for a result that is a win on one
  side and a draw on the other.

### The AI at the wheel (M10)

The autopilot (R1183002) as an aura: the first N actions of each of your turns (#46), or whole turns
(#47); the Pareto judge (ME-PARETO) shown to a player as a hint, the best action lit up, read from their
own view only (#147); and a free helpful action at the start of turn, a Zephrys-style pick cast at no cost
(#147.2).

### Hidden Nerfs and trade offers (M11)

A Nerf hidden from the card's owner until it is played, never touching its cost (#123), through
`view_for`, the AI's redaction and `determinize`; and a trade offer at the opponent's start of turn: you
give a card from your hand, they Discover one of theirs to give you (#77).

### Card metrics (M12)

`crates/cards/build.rs` counts each script's code lines outside its tests (comments and blank lines left
out) and compiles in two lists, so both grow with the catalog and nothing is a hand-kept table (ST9 Q12):

- **BUSTED**: over 100 lines (the designer's number; 16 cards today). xxFreshJack's pool is every BUSTED
  card the catalog holds, the sets made for single-player modes included once they exist (dungeon runs and
  bosses, the reserved Boss set): a catalog query option, "every set, single-player ones too".
- **Complicated**: over 60 lines (67 cards today). Shockwave447 fills both hands from it.

### The starscape map (M13)

Alteran Reach's map: a seeded map of many nodes, Reach the Summit's shape (M #8), travelled one node per
Activate ♾️ (1), the first each turn free on the Radiant face; rewards on the nodes; at least 20 travels to
the edge, which adds Edge of the Universe to your hand; the progress public on the view; the client's map.

### Spell, combat and card-on-card verbs (M14, M15, M16)

The rest of what the set's cards need, built as verbs on fixture scripts:

- **Spells (M14)**: Freeze forever; a Spell's Trample onto adjacent Units and the hero; Poisonous for a
  Spell ("this and your next Spell this turn"); copies with a fixed X; "lose N mana next turn"; the melee
  (every Unit attacks another until one survives, at most 20 attacks); Inflation (each card in hand and
  deck has a chance to cost (1) more, the chance growing).
- **Combat and replacement (M15)**: Immune to Spell Damage; Evolve, and "after this attacks, summon a
  copy of it"; destroy becomes exile (every card's, or the enemy's); damage this takes also dealt to the
  enemy hero (and the attacker); your own-turn damage redirected to the opponent with a per-turn cap; the
  Gauntlet's lanes (their Units target only each other, and no other Unit targets them); "when the enemy
  summons a Unit, a random Unit (every Unit) of yours attacks it"; "Units can't be in Defense Position";
  "when one of your Taunt Units is attacked"; a trap with a chance to reveal.
- **Card on card (M16)**: copy a Unit's text; Duo and Squad (a mark that makes a permanent Radiant at your
  next turn, the Allure animation recoloured); exile auras (cost (N) or less; not of given tags); hold a
  permanent beneath a card until it leaves (Ticketed, Sack); a beneficial Spell (one whose chosen targets
  are all its caster's own hero or permanents) cast for yourself instead (Wormhole); Radiant on reaching N
  keywords (Rod of Ages); Crater on a Field Spell, Locking its backrow zone.

## ST4. The patches to shipped cards

They change shipped games, so each is a patch of its own that goes live when it merges (with its pending
fragment), never held behind the set's gate:

| Issue | Cards | Change | Why the set needs it |
| --- | --- | --- | --- |
| L1 | Core #100 Ceaseless Void; C #90 In Too Deep; C+ #32 Otherworldly Removal; C+ #32.1 Execute, #32.2 Brawl, #32.3 Blade Storm; Core #26 Glowy Jelly Bean, #27 Blood Ridden Glowy Jelly Bean, #28 Knockoff Temu Glowy Jelly Bean, #29 GIGA Glowy Jelly Bean; C #32 Felinor Feelings; Core #62 Friend of Felinors; C+ #68 Organic Produce; C+ #63 Fruit Tree | Gain Terminus, Quest, Otherworldly, Other-Origins (×3) and Jelly Bean (×4); Felinor Feelings and Friend of Felinors lose Felinor; Organic Produce and Fruit Tree lose Fruit | Olivian Expanse discovers Terminus cards; Tylannt completes Quests; Moonberry adds Other-Origins cards; The Greatest Heist discovers a Jelly Bean; Fruit Launcher casts a random Fruit, so the two Field Spells that make fruit must not be one. The five new tags enter the engine's `Tag` union and the server's `cards_tags_check` here |
| L2 | M #98 Showdown, M #53 Prestige | Gain Tradable, with the keyword itself (ST3) | The set's Tradable cards use the same rule |
| L3 | M #53 Prestige | Playable only while a card to De-Radiant exists | — (after L2: the same card) |

## ST5. Decisions across the set

- **D1 Built behind the gate.** Stinkyscape is held by R1420 while fullsend 1 builds it, as Meditative+
  was (R1178000): no card part adds a pending fragment or moves a golden trace; fullsend 1's reconcile
  ships it, claiming every entry in one fragment.
- **D2 R275's stat half** holds: every Radiant Unit at twice its stats, so Falcon's Radiant face is 8/2 and
  Prototype-X2's 16/16 (ST9).
- **D3 New state stays out of shipped games' hashes.** The Resource tray, Credits, the Marketplace, a
  Starship's memory, quest stages, votes and the map are optional fields skipped when absent
  (`docs/meditative-set.md` D14).
- **D4 Purity** (CLAUDE.md rule 4). Price moves, rolls and the map draw from the match's `Rng`; the card
  metrics are computed at build time; nothing in the engine sends an email (ST9 Q11).
- **D5 Numbers** (CLAUDE.md rule 9). The tray's 8, the rarity table, the 6-turn cycle, every stock range,
  the price rule, its spread, the 20% Radiant chance and its 2× premium, the (3) launch, BUSTED's 100 and
  complicated's 60, Wild System's 20 attacks, Masochism Unmasked's 20 and 40, Alteran Reach's 20 travels:
  named constants in `crates/engine/src/config.rs` (the metrics' in `crates/cards`).
- **D6 Words.** "Stinkyscape" everywhere ("Sinkyscape", "Stinkscape" are typos); "Starpiece" is Starship
  Piece; "Indestructable" is Indestructible; "Equipt" is Equip; "Book of Buffs" is Book of Buff (C+ #71);
  house style as R366, R373 and R432 set it.
- **D7 Rulings.** Each issue takes its R-numbers from its own block, R(N×1000) up (`docs/issues-and-patches.md`,
  Ruling numbers), and names its proving tests after them (CLAUDE.md rule 3).
- **D8 The AI deals itself none of the set until fullsend 2.** The shadow ban (R186) is written only by a
  sweep (`cargo jackioh sweep --write`, "never tuned by hand"), so "every card shadow banned" is R1390's
  soft gate instead: `AI_DECK_GATED_SETS` holds Stinkyscape from fullsend 1's release until fullsend 2's
  sweep lifts it. The AI must still answer every new prompt and card from fullsend 1 on, since a player may
  play any of them against it (R186).
- **D9 A fullsend layout.** Fullsend's parts build blind on one integration branch and a part that
  conflicts is parked for the reconcile (`bot/README.md`, Fullsend). So the foundation writes every shared
  row for all the set's entries — `catalog.json`, `card-audio.json5`, `flavour.json`, `chinese.json`,
  §8.10, §7, BUILD M12, `docs/radiant-audit.md`, the census and the fixture, and the scripts' module list
  with a stub per card — and a card part changes only its own cards' script files (and a correction to its
  own rows where its build decides otherwise).

## ST6. The issues

**Opened on 2026-10-25** as #1836 (the tracker to pin) to #1886:

| Id | Issue | Id | Issue | Id | Issue | Id | Issue |
| --- | --- | --- | --- | --- | --- | --- | --- |
| L1 | #1837 | M05 | #1841 | M13 | #1847 | S07 | #1863 |
| L2 | #1838 | M06 | #1842 | M14 | #1848 | S08 | #1864 |
| L3 | #1851 | M07 | #1854 | M15 | #1849 | S09 | #1873 |
| M01 | #1839 | M08 | #1855 | M16 | #1850 | S10 | #1865 |
| M02 | #1852 | M09 | #1843 | fullsend 1 | #1856 | S11 | #1866 |
| M03 | #1840 | M10 | #1844 | F | #1857 | S12 | #1874 |
| M04 | #1853 | M11 | #1845 | S01–S06 | #1858, #1859, #1860, #1861, #1872, #1862 | S13–S18 | #1867, #1868, #1875, #1869, #1876, #1877 |
| M12 | #1846 | release | #1884 | S19–S22 | #1878, #1879, #1881, #1870 | S23–S26 | #1871, #1880, #1882, #1883 |
| fullsend 2 | #1885 | fullsend 3 | #1886 | | | | |

**How they are built** (the designer, 2026-10-25): not through Squishy's fullsend mode. An issue being built in
a Claude Code session with subagents is labelled `human`; every other buildable issue is `bot:build`, for the
night bot, and waits for its `Blocked by` lines. So fullsend 1's parts are ordinary issues built on `main`
behind the gate, as Meditative+'s were, and the release is a part of its own (#1884) rather than a
reconcile. The trackers (#1836, #1856) carry no queue label.

One tracker, `Patch: the Stinkyscape set` (labels `major version`, `patch`), holds a checklist of everything
below. **No issue is titled `(part n of m)`**: the night bot builds such parts strictly in order
(`bot/harness/queue.py`, `waits_for`), and these build in parallel, each waiting only for the issues its
`Blocked by` lines name. Difficulty follows CLAUDE.md rule 11.

### Phase 0: the mechanics, each merged on `main` without the set

Each is an ordinary issue and pull request, with CI. L1 to L3 go live as they merge; M01 to M16 are
reached by no card until fullsend 1 ships.

| Id | Title | Blocked by | Difficulty |
| --- | --- | --- | --- |
| L1 | `Patch: Terminus, Quest, Otherworldly, Other-Origins and Jelly Bean tags on shipped cards, and Felinor and Fruit off four` | — | medium |
| L2 | `Patch: the Tradable keyword, on Showdown and Prestige` | — | hard |
| L3 | `Patch: Prestige can be played only with a card to De-Radiant` | L2 | medium |
| M01 | `Patch: the Dormant keyword` | — | hard |
| M02 | `Patch: Starship Pieces and the Starship` | M01 | hard |
| M03 | `Patch: Resources, the Resource tray and Credits` | — | hard |
| M04 | `Patch: the Marketplace` | M03 | hard |
| M05 | `Patch: the weapon kit: cooldowns, half damage to heroes, Equip from a pick, Float lowers Tribute` | — | hard |
| M06 | `Patch: layered summons and the lethal guard for Units` | — | hard |
| M07 | `Patch: staged Quests` | L1 | hard |
| M08 | `Patch: Anomalies, the board swap` | M01, M03 | hard |
| M09 | `Patch: two new endings, voted out and fled` | — | hard |
| M10 | `Patch: the AI at the wheel: a bounded autopilot, the Pareto hint and a free helpful action` | — | hard |
| M11 | `Patch: hidden Nerfs and trade offers` | — | hard |
| M12 | `Patch: card metrics, BUSTED and complicated cards` | — | hard |
| M13 | `Patch: the starscape map` | — | hard |
| M14 | `Patch: spell verbs for Stinkyscape` | — | hard |
| M15 | `Patch: combat and replacement verbs for Stinkyscape` | — | hard |
| M16 | `Patch: card-on-card verbs for Stinkyscape` | — | hard |

### Fullsend 1: the foundation and the cards, shipped with the AI dealing itself none of them

The parent, `Patch: Stinkyscape fullsend 1: the foundation, the cards and the release`, is blocked by
every phase-0 issue and is labelled for fullsend **only once they have all merged**: a fullsend on an issue
split by hand makes its integration branch from `main`'s head the moment it is asked (`queue.adopt`), so
asked earlier its parts would build without the mechanics. Its sub-issues are F and S01 to S26; its
reconcile is the release (its Done when below). Every S part is blocked by F.

| Id | Title (`Patch: Stinkyscape …`) | Cards | Uses | Blocked by (besides F) | Difficulty |
| --- | --- | --- | --- | --- | --- |
| F | `the foundation` | — | the set held by R1420; the tags Drone Core, Weapon, War Doctrine and Swarm; the card-by-card brief from this plan; every shared row (D9) | — | hard |
| S01 | `S01, the Dormant cards` | #17 Imprisoned Ship Smith, #19 Imprisoned Cosmality | M01 | — | hard |
| S02 | `S02, the Starship cards` | Dormant Starship, #90 After Burner, #106 Warpfield Amplifier, #131 Spacerace Stimulus, #133 Ultra Capacitor, #136 Magic Module, #137 Firststrike Capability, #137.1–#137.4 Land, Sea, Air and Space Doctrine, #144 Prepare For Liftoff | M02, L2 | — | hard |
| S03 | `S03, Starship Pieces on existing keywords` | #38 Shield Amplifier, #40 Poison Capsule, #43 Healing Capsule, #48 Ultralight Chassis, #49 Spacemine, #68 Startos, #69 Falcon, #72 Paragon Mk1, #73 Paragon Mk2, #93 Patcher Nanobots, #98 Hull Booster, #100 AP Rounds, #101 Overdrive Module, #107 RR-5000, #108 Trila 3, #122 Vamp Rounds, #175 Frost Module, #132 Defensive Crystal, #134 Felinor Containment Chamber, #158 Stratos Swarm | M02 | — | medium |
| S04 | `S04, the Resources and the asteroids` | Reknite, Korrelite, Gellium, Axnit, Narcor, Red Narcor; #39 The Narcor Project, #67 Tractor Beam, #75 Badger, #76 Honey Badger, #84 Power Cell, #84.1 Metal Scraps, #89 Spice, #96 Ocean Tsunami, #97 Asteroid Field, #116 Megabase, #155 Comet Crash, #169 Tiny, #170 Medium, #171 Large, #172 Huge Asteroid, #173 Abandoned Spice Tower | M03, M16 | — | hard |
| S05 | `S05, the Miners` | #62–#66 Miner v1–v5, #78 Ozark, #79 Orca, #80 Galaxy | M03, M05 | S04 | medium |
| S06 | `S06, the weapon rack` | #50–#61 Beam, Repeater, Cannon and Flak v3–v5, #81 Radix, #82 Phalanx, #83 Halo, #85 Prototype-X1, #86 Prototype-X2, #140 Rush Token, #141 Fruit, #142 Jade and #143 Book Launcher | M05, L1 | — | hard |
| S07 | `S07, the drones` | Drone Interceptor, Drone Corvette, the Battleship's three layers; #27–#36 Drone Core I–X, #41 Drone Sympathizer, #42 Core Researcher, #44 Great Replacement Theory, #45 Drone Commander, #168 Drone Hanger | M06 | — | hard |
| S08 | `S08, the trade cards` | #87 Siva 1, #164 The Syndicate, #165 The Trade Union | L2 | — | hard |
| S09 | `S09, The Greatest Heist and Tylannt` | #6 Tylannt, #7 The Greatest Heist | M07, L1, L2 | S04 | hard |
| S10 | `S10, Job Application` | #112 Job Application, #112.1 Paycheck | M07, M03 | — | hard |
| S11 | `S11, the three Brains` | #18 Imitated Instrumentality, #18.1 MELCHIOR, #18.2 BALTHASAR, #18.3 CASPER | M09 | — | hard |
| S12 | `S12, the AI at the wheel` | #46 Circadian Halfcycler, #47 Circadian Fullcycler, #147 IN33DH3, #147.1 IN33DH3 Prime, #147.2 Wardrum Blob | M10 | S07 | hard |
| S13 | `S13, spells on existing primitives` | #1 Freeze Ray, #8 Conjure Rosemary Triscuit, #8.1 Rosemary Triscuit, #10–#12 the three Otherworldlies and #10.1–#12.3 their nine spells, #21 Moonberry, #145 Fortify Flesh, #146 Flash Freeze, #156 Catalyst Comet | L1 | — | medium |
| S14 | `S14, spells that bend a rule` | #2 Absolute Zero, #3 Meteor, #13 Asphyxiants Cloud, #99 No-No Words, #102 Book of Nono-Words, #103 Vxyl, #117 Core System, #118 Secure System, #119 Unsecure System, #120 Wild System, #121 System Schematic, #153 Stimmycheck | M14 | — | hard |
| S15 | `S15, traps and reactions` | #5 Masochism Unmasked, #9 Ticketed, #174 Aerelret, #22 Gatinsir, #23 Warp Disruptor, #24 Warp Disruptor II, #135 Sacked, #135.1 Sack, #138 Pocket Peptides, #139 Wormhole | M15, M16 | S03, S06 | hard |
| S16 | `S16, auras and rule benders` | #14 Arm of the Jempire, #15 Arg Arg Defender, #16 Rod of Ages, #70 Z'Zoth, #71 A'Zoth, #74 Paragon-X, #91 The Propagation's Wasp, #92 The Propagation's Hornet, #94 Tmack Trav, #104 Reflective Plating, #105 Disintegrex, #152 Sanctum, #157 The Gauntlet | M15, M16 | — | hard |
| S17 | `S17, the Acclaimed` | #20 Tnack Trav, #26 ZolarKeth, #37 Kwzka, #88 Siva 2, #110 iKwzki, #113 A Very Dandy Lad, #114 Arctic Citadel, #115 Scornboi | M16 | S03 | hard |
| S18 | `S18, outside data and hidden hands` | #25 KY's Tricknology, #77 Yuma, #109 xxFreshJack, #111 Shockwave447, #123 Gokugains | M11, M12 | S03 | hard |
| S19 | `S19, Jlockeed's space program` | #124–#130, the seven Jlockeed cards | M02 | S03 | medium |
| S20 | `S20, the Anomalies and the Scanning Probes` | Anomaly Found!, the ten boards, #166 Scanning Probe Mk1, #167 Scanning Probe Mk2 | M08 | S04, S06, S07, S08, S15, S16 | hard |
| S21 | `S21, the Marketplace's table and its 10 AI cards` | the 10 AI cards | M04 | S03, S04, S06, S07, S15, S20 | hard |
| S22 | `S22, Terminus` | #4 Olivian Expanse, #4.1 Farewell, #4.2 KY's Last Exam, #4.3 Event Horizon, #4.4 Last Word, #4.5 1v1 Final Destination, #4.6 The Big Crunch, #154 Dice of Doom | L1, M06 | — | hard |
| S23 | `S23, Alteran Reach` | #148 Alteran Reach, #148.1 Edge of the Universe | M13, M09 | — | hard |
| S24 | `S24, the bundles` | #95 Vanguard Coalition, #159 ETHICS, #160 Golden Accord, #161 Pegasus, #162 Star-jumper Core, #163 Swear Loyalities | — | S03, S04, S05, S08, S09, S16, S17, S18 | medium |
| S25 | `S25, the two Calls to Chaos` | #149 Wildar, #150 Call to Chaos (Stinky Edition), #150.1 Stinkyscape Golem, #151 Call to Chaos (Scape Edition) | M02, M04, M08 | S02, S07, S09, S10, S13, S14, S17, S20, S21, S23, S24 | hard |
| S26 | `S26, the set's look and sound` | all | — | every other S part | hard |

S26 is the set mark, the Almanac's and the deck builder's chips and `FILTER_TAGS`, the `RELEASES` line,
every card's intro-music spec for the Legendaries and Mythics (`gen:music`), and every face read in the
browser.

Every one of the 175 numbered cards and 30 sub-numbered tokens is in exactly one row, and every card a
card names (a bundle's contents, a pool a test needs, an anomaly's board) is in a row its part is blocked
by (both checked against the list).

**Fullsend 1 is done when** (its reconcile): every part's cards play by their rows; Stinkyscape is in
`SHIPPED_SETS` and in `AI_DECK_GATED_SETS` (D8); every new prompt and card is answered by the AI, the random
policy and the timeout; one pending fragment claims every entry; the server's migration admits the four
foundation tags and seeds the set; deck and trio codes take it at 5000 + n (R1410); the golden traces are
re-blessed once and the seed-pinned tests re-pinned for the bigger pools; the voice files are rendered
(`[gen:voice]`, R1448); every stated total moves; `cargo jackioh fuzz --seeds 200`, `catalog check`,
`patches check`, `spec check` and the AI's smoke gate are green; REVIEW.md's audit runs as a separate
session (CLAUDE.md rule 8); and every required check is green on the pull request into `main`.

### Fullsend 2: the AI deals itself 80% of the set

`Patch: Stinkyscape fullsend 2: the AI deals itself 80% of the set`, blocked by fullsend 1, with no parts
opened by hand: fullsend splits it when it is asked. **Done when**: a sweep of record over the set's 175
non-token cards (`cargo jackioh sweep`, R186, R390) bans at most 35 of them (`STINKYSCAPE_BAN_CEILING`,
20%); `cargo jackioh sweep --write` writes those bans; `AI_DECK_GATED_SETS` is empty again, so the AI
deals itself every Stinkyscape card the ban does not hold; and the AI's gates are green.

### Fullsend 3: the AI deals itself the whole set

`Patch: Stinkyscape fullsend 3: the AI deals itself the whole set`, blocked by fullsend 2, split by
fullsend in turn. **Done when**: a sweep of record bans none of the 175, so `SHADOW_BAN` holds no
Stinkyscape card, and the AI's gates are green.

## ST7. The order

1. **Phase 0, now:** L1, L2, M01, M03, M05, M06, M09 to M16 in parallel; then L3 (L2), M02 (M01), M04
   (M03), M07 (L1), M08 (M01, M03).
2. **Fullsend 1**, asked once every phase-0 issue has merged: F; then S01–S04, S06–S08, S10, S11, S13,
   S14, S16, S22, S23; then S05, S09, S12, S15, S17, S18, S19; then S20 and S24; then S21; then S25; then
   S26; then the reconcile and the release.
3. **Fullsend 2**, asked once fullsend 1's pull request has merged.
4. **Fullsend 3**, asked once fullsend 2's has.

## ST8. Fixes to the list

| # | Card | The list says | The build reads |
| --- | --- | --- | --- |
| 21, 122 | Aerelret; Frost Module | the same number as Moonberry; as Vamp Rounds | #174 and #175 |
| 25–47, 124, 125, 152, 153 | KY's Tricknology, ZolarKeth; the Drone Cores, #41, #42, #44–#47, #124, #125, #152, #153 | Sinkyscape, Stinkscape | Stinkyscape |
| 69 | Falcon | Radiant 8/1 | 8/2 |
| 86 | Prototype-X2 | Radiant 8/8 | 16/16 |
| 12 | Otherworldly Nightmare | Radiant adds Execute, Brawl, Blade Storm | Radiant Shadow Word Death, Shadow Word Pain, Psychic Scream |
| 4.5 | 1v1 Final Destination | a Trap with no reveal condition | Reveals when your opponent's turn ends (the end-of-turn trap window, R62), so you strike first at 1 health |
| 7 | The Greatest Heist | "or a Jellybean" | or a Jelly Bean: a card tagged Jelly Bean (Core #26–#29, ST4) |
| 79 | Orca | an Embiggen price (5) with no Embiggened text | Embiggened (5): the two Miners offered are Radiant |
| 125 | Jlockeed's Spacerace Propaganda | a Spell with Activate, no Jlockeed tag | a Field Spell, tagged Jlockeed |
| 164 | The Syndicate | a Spell with an Aura | a Field Spell (FO's backrow places it) |
| 124 | Jlockeed's Shipbuilding Grant | tag "Jlockeed's" | Jlockeed |
| 95 | Vanguard Coalition | Radiant face drops Vxyl | Radiant Vxyl too |
| 162 | Star-jumper Core | Radiant face the same as the base | Radiant Badger, Honey Badger, Ozark, Orca and Galaxy |
| 120 | Wild System | Radiant face the same as the base | one text on both faces |
| 116 | Megabase | a Field Spell with Health, attacked and dying | Animated |
| 155 | Comet Crash | Crater on a Field Spell (Crater does nothing there, §6.1) | Crater Locks its backrow zone (M16) |
| 45 | Drone Commander | no trigger word | Cry |
| 167 | Scanning Probe Mk2 | Radiant "Luck 3" | Lucky 3 (Luck X is a different, player-wide keyword) |
| 173, 75, 76 | Abandoned Spice Tower; Badger, Honey Badger | "Add a three Spice"; "add gain a random" | "Add three Spices"; "gain a random" |
| 163 | Swear Loyalities | "Golden" | Golden Accord; the name kept as written |
| 169–172 | the asteroids | "Can't be in Defense Positio n" | Position |
| anomalies | AT, MT, AN | "Small Asteroid" | Tiny Asteroid (#169) |
| 68, 158 | Startos; Stratos Swarm | two spellings | Startos, as the Unit prints it |
| 13 | Asphyxiants Cloud | "should deal 1 poisonous damage to all enemies, if it doesn't please reword" | "This and your next Spell this turn have **Poisonous**. Deal 1 damage to all enemies. Draw 1." |

## ST9. The designer's answers

Asked and answered on 2026-10-25; the S parts and mechanics are written in these terms.

| Q | Question | Answer |
| --- | --- | --- |
| 1 | Launch = awaken? Starting stats? What fuses? | Yes, the same; only a piece that **dies** fuses; it starts 0/0, since its stats are what is fused into it |
| 2 | The War Doctrines' Starship lines: once or every turn? | Once, as the doctrine is cast (the keyword half lasts the rest of the game) |
| 3 | Keep "Dormant" though the engine calls a card beneath a Stack dormant? | Keep it; the Stack wording becomes "beneath a Stack" |
| 4 | The Marketplace: in every game? shared? bought how? | As proposed: opens on the first Resource or Credits; one offering, a stock each; a button in your main phase, no mana |
| 5 | Does the opponent see your Resources? | The count only |
| 6 | The price formula | 10 × min(cost, 4) + 5 × max(cost − 4, 0); a (0) card 5 |
| 7 | Anomalies: whose side; do Cries fire? | Your side; every Cry fires, its targets and choices random |
| 8 | The 10 AI Marketplace cards | The build designs them; no approval needed |
| 9 | Voted out by the Brains | A loss for you and a win for your opponent |
| 10 | Edge of the Universe and the ladder | A win for you and a draw for your opponent |
| 11 | Job Application's email Easter egg | Dropped |
| 12 | xxFreshJack's single-player pools; complicated cards | Single-player sets come with dungeon runs and bosses; complicated is a computed criterion, so it grows (M12) |
| 14 | KY's Tricknology's coordinates | The build's choice (below) |
| — | Falcon's and Prototype-X2's Radiant stats | Doubled |
| — | Otherworldly Nightmare's Radiant face | Fixed by the build |
| — | 1v1 Final Destination's reveal | Inferred by the build (ST8) |
| — | "Jellybean" | A Jelly Bean tag on Core #26–#29 |
| — | Orca's Embiggened version | The offered Miners are Radiant |
| — | Spacerace Propaganda and The Syndicate | Field Spells, with the tags they lack |
| — | Great Replacement Theory's name | Kept: it is satire |
| — | Who builds | First Squishy; then (when the issues opened) a Claude Code session's subagents for what it takes (`human`) and the night bot for the rest (`bot:build`), ST6 |

**KY's Tricknology's coordinates** (the build's choice): a zone is (x, y) with x its lane, 1 to 5, and y
its row — 1 your unit row, 2 your backrow, −1 the enemy's unit row, −2 the enemy's backrow. The chosen
zone (x, y) maps the unit square through the Jacobian of F(x, y) = (x² − y, xy) at that point,
J = [[2x, −1], [y, x]], placed with its corner at the zone; every zone whose centre lies in that
parallelogram gets a Starship Piece, its controller's (base face), or only yours (Radiant face).

**The readings no one asked about** keep the defaults the build uses: Circadian Halfcycler's three actions
are every one of your turns while it stands; Masochism Unmasked's cap is per turn; Warp Disruptor's "Units"
are every Unit on both sides, and its attack response is your side's; Scornboi exiles Units; AMBUSH is a
label (the summoned Units arrive before the attack resolves, which goes on as declared); Swarm is a tag on
Units; a Rush Token Launcher's token attacks an enemy Unit; Tnack Trav (#20) and Tmack Trav (#94) are two
cards; Drone Hanger keeps its spelling.

## ST10. What every issue body holds

- **Scope**: the cards (index, name, the designer's text) or the mechanic, from ST3 and ST6.
- **Reading**: this plan's entries, and for a card part the brief's (after F), every default named.
- **Blocked by #n** lines, one per blocker, so the queue reads them.
- **Rulings**: the block it owns, R(N×1000) up.
- **Done when**: for a mechanic, its engine tests on fixture scripts, its §6 row and rulings, and the
  client's piece, with the repo's gates (CLAUDE.md, Commands); for a card part, each card's base and
  Radiant tests in its script file (CLAUDE.md rule 6), its BUILD M12 rows green, and `catalog check`,
  `patches check` (no fragment) and `spec check` green on the integration branch's reconcile.
- **Labels**: `patch` (with `major version` for the set's issues), one `difficulty:*`, and `human` while a
  Claude Code session builds it, else `bot:build` (ST6, How they are built).

## ST11. Reading the brief

ST11 to ST15 are the foundation's (issue #1857): the card-by-card brief ST0 to ST10 planned, in
`docs/meditative-plus-set.md` MP6's shape. SPEC is still the only source of rules (CLAUDE.md): the set-level
rules the foundation decides are in SPEC already ([[R1857000]] to [[R1857003]], [[§5]], [[§8.10]], [[§7]]),
every card's row is in [[§8.10]] (a token's also in [[§7]]), and each card's mechanics become SPEC as the
S part that builds them lands, with its rulings.

- **Numbers.** "§" is SPEC; "ST3" is this brief's section ST3; "S #N" is Stinkyscape card N (BUILD M12
  writes "S N"); "M+ #N" is a Meditative+ card, "M #N" a Meditative card, and "C #N", "C+ #N" and "Core #N"
  (or "#N" in SPEC) the other sets' cards. The set's shared entries go by their index: T-dormant-starship,
  T-drone-interceptor, T-anomaly-found, T-AI-1 to T-AI-10 and the rest (ST12). The parts go by their ids and
  issues (ST6): F #1857, S01 #1858 to S26 #1883, the release #1884.
- **A card entry** (ST13) has the heading (set, number, name), the header line (id · cost, type and tags ·
  rarity · stats base → Radiant · the S part that builds it, with its issue), the designer's text verbatim on
  one line (`~~~` between his base face and his Radiant face, his `{…}` notes kept), and then:
  - **Text** and **Radiant**: the catalog's `base.text` and `radiant.text`, house style, params as `{key}`,
    a line break written " / ";
  - **Engine**: what the card does in the engine's terms, the verbs of `crates/engine/src/effects/*.rs`,
    the hooks of `script.rs`, the rulings it stands on, and **NEW** for anything no phase-0 issue built,
    with the S part that builds it;
  - **Rulings**: the readings its builder needs, numbered ST-S13-1 and the like by the S part that proves
    them. They are proposals, not `R<n>` rows: that part takes an R number from its own issue's block
    (`docs/issues-and-patches.md`, Ruling numbers) and names its proving test after it (CLAUDE.md rule 3);
  - **Numbers**: the declared params, key, base value, ↑ when more is better for its controller, and the
    Radiant value and step where they differ;
  - **Check**: ST8's fixes, ST9's answers, R275's verdict, the list's typos and **⚠ designer** for a
    reading the designer should confirm (ST14 lists each with the default the build uses; the build does
    not wait for it);
  - **Class**: A keywords or one primitive, B composed primitives, C a phase-0 system, D a subsystem of its
    own.
- **Words.** House style as R366, R373, R432 and R1175000 set it: "deck" for the library, "Tribute" for
  Sacrifice, Cry and Death, labelled abilities on lines of their own (Cry:, Death:, Start of turn:, End of
  turn:, Aura:, Activate:, Activate 2: to Activate 6: ([[R1857003]]), Activate ♾️:, Cast on draw:, Quest:,
  Embiggened (N):), "(N) Cost" for a cost as a noun and "costs (N)" as a verb, Bounce for a permanent's
  return from the field, "End the turn", straight apostrophes, and never "start of your turn" or "end of
  your turn" in a sentence. The set's own words, as the phase-0 issues print them: "Dormant for 2 turns.
  When this awakens, …", "When launched, …", "Evolve: …", Starship Piece (never Starpiece), Equip (never
  Equipt), Indestructible, Jelly Bean, Tradable, and a Stinkyscape Resource that a card **gains** into its
  player's tray, never "adds to your hand" ([[R1840000]]).
- **Not yet on main.** The Marketplace (#1853, PR #1963) and Anomalies (#1855) are cited in words
  ("#1853's Marketplace", "#1855's board swap"), never by their R numbers, and no row depends on a name
  either adds: neither adds a keyword, tag, catalog field or label. S20, S21 and S25 correct their own rows
  if either changes in review.

## ST12. The set's data

**Ids.** A card is `stinkyscape-NNN` (three digits), a token a card defines `stinkyscape-NNN-k`, a shared
entry `stinkyscape-t-<name>`, and an AI card `stinkyscape-t-ai-0N` (`id_for_index`: T-AI-1 is
`stinkyscape-t-ai-01`); the index strings are "4", "4.1", "T-drone-interceptor" and "T-AI-1". The scripts
live in `crates/cards/src/scripts/stinkyscape/` (`c004_olivian_expanse.rs`, `c004_1_farewell.rs`,
`t_drone_interceptor.rs`, `t_ai_01_escort_protocol.rs`), each a stub with no behaviour until its S part
replaces it, but the Dormant Starship's, which is the engine's Dormant hull. The set is
`SetName::Stinkyscape`, written "Stinkyscape", sixth in `CATALOG_SETS` and held by [[R1420]]'s gate until
the release (#1884, [[R1857000]]).

**Numbers.** 228 entries ([[R1857001]]):

| Kind | Count | Indices |
| --- | ---: | --- |
| Numbered cards | 175 | "1" to "175": the designer's #1 to #173, his second #21 as #174 Aerelret and his second #122 as #175 Frost Module |
| Tokens on sub-numbers | 30 | 4.1–4.6 (Terminus), 8.1, 10.1–12.3 (the nine Other-Origins spells), 18.1–18.3 (the Brains), 84.1, 112.1, 135.1, 137.1–137.4 (the War Doctrines), 147.1, 147.2, 148.1, 150.1 |
| The Dormant Starship | 1 | T-dormant-starship, no token ([[R1852000]]): the engine's `DORMANT_STARSHIP_DEF_ID` |
| Shared tokens | 12 | T-drone-interceptor, T-drone-corvette, T-shield-layer, T-propagation-layer, T-meltdown-layer; T-reknite, T-korrelite, T-gellium, T-axnit, T-narcor, T-red-narcor; T-anomaly-found |
| AI cards | 10 | T-AI-1 to T-AI-10 (ST15) |

By type, the 228 are 107 Units, 86 Spells, 23 Field Spells, 10 Traps and 2 Field Traps; the 176 entries
that are no token are 96 Units (the Dormant Starship among them), 54 Spells, 18 Field Spells, 6 Traps and 2
Field Traps. Against ST1, which counts the numbered and sub-numbered entries by the list's types: 96 Units,
75 Spells and 22 Field Spells after ST8's two, 8 Traps, 2 Field Traps, the two Swarm entries among the Units.

**Names.** The final names are unique against the catalog and `chinese.json`, with straight apostrophes and
ST8's spellings: Startos, Star-jumper Core (the list's header writes "core"), Swear Loyalities as written,
Starship Piece for "Starpiece". The Drone Battleship's layers are **Drone Battleship (Shield Layer)**,
**(Propagation Layer)** and **(Meltdown Layer)**, so a text that says "the Drone Battleship" names all
three, as "Call to Chaos" names every edition ([[R279]]'s parenthesis rule). Anomaly Found!'s text names no
board: "Wormhole" or "Asteroid Field" would name a card. A name a text uses is the catalog's exactly, so the
list's "Paragon mk1" and "Paragon X" are S #72 Paragon Mk1 and S #74 Paragon-X, and its "Book of Buffs" C+
#71 Book of Buff.

**Types.** The designer's, with ST8's: #125 Jlockeed's Spacerace Propaganda and #164 The Syndicate are
Field Spells; #91 and #92 are Units tagged Swarm; #16 Rod of Ages, an Animated Field Spell that prints no
stats, prints 1/1 → 2/2, since an Animated face must; #120 Wild System, which ST8 gave one text on both
faces, gains 2 mana next turn on its Radiant face, not 1 ([[R276]]).

**Tags.** The designer's, and #125's Jlockeed (ST8) and #40 Poison Capsule's Plague ([[§5]]: every card that
uses Plague Counters carries it). Four new family tags, no tribes and read by no rule ([[R1857000]]):
**Drone Core** (#27–#36), **Weapon** (#50–#61, #140–#143), **War Doctrine** (#137.1–#137.4) and **Swarm**
(#91, #92). The census over the 228: Token 52; Human 20; Weapon 16; AI 10; Drone Core 10; Acclaimed 9;
Other-Origins 9; Jlockeed 7; Terminus 6; Quickdraw 5; War Doctrine 4; Otherworldly 3; Call to Chaos, KY,
Quest, Swarm and Wincon 2 each; Book, CN, Catalyst, Felinor, Fruit, Plague and Prime 1 each; Pancake,
Potion and Jelly Bean none.

**Rarity.** The designer's. The 176 entries that are no token are 42 Common, 50 Rare, 43 Epic, 29 Legendary
(the Dormant Starship one of them) and 12 Mythic, the ceilings `catalog check` holds the set to. Every
token's rarity is Token, the designer's rarity its `printedRarity` (12 Mythic, 12 Legendary, 10 Epic, 3
Rare and 5 Common), but the ten AI cards', which print none, as Classic+'s ten do. #137.1–#137.4, whose
headers print no Token, are tokens printed Legendary, as ST1 counts them. The Dormant Starship is no token
("NOT a token so it can be shuffled into your GY"): a (3) Legendary Unit, 0/0, its Radiant face [[R349]]'s
fallback (`radiantFallback`), in no pool and no deck unless a query names it ([[R1857001]]).

**Radiant stats.** [[R275]]'s stat half holds as written: Falcon's Radiant face is 8/2 and Prototype-X2's
16/16 (ST8, ST9), and xxFreshJack, which prints no Radiant stats, is 12/22.

**The census** (`crates/tools/src/catalog.rs`'s Stinkyscape expectation and tag counts, and the cards
crate's `STINKYSCAPE` fixture, generated from ST13's header lines): every entry the set holds is one of the
indices above, once, with its header line's name, cost, type, tags, rarity and stats, and a row with no
entry is a card still being built. None counts toward the shipped totals (482 entries, 396 cards and 86
tokens) or the tag census until the release ([[R1420]]).

**Who names the tokens** ([[R1857002]]): a Terminus token by "Terminus card" (#4, #154), a War Doctrine by
"War Doctrine" (#137), an AI card by the AI pools once the set ships ([[R1421]]), a Resource by the rule
that deals it ([[R1840002]]'s roll); every other token by its maker's text.

## ST15. The ten AI cards

The designer: "Create 10 AI Stinkyscape cards and always offer one of those", the Marketplace's last lot
(ST3); his answer to ST9 Q8: the build designs them, with no approval needed. They are the set's T-AI-1 to
T-AI-10, in Classic+'s shape ([[§8.7]]'s end): tokens tagged AI and Token, `rarity: "Token"` and no
`printedRarity`, `flavor: "Tech"` (`docs/polish/8-flavor.md` rule 1), costing (0) to (2), named for AI
tropes, declaring no numbers ([[R482]]; their printed numbers are `AI_PRINTED` in `params.rs`), each Radiant
face doubling a number or adding a rider ([[R275]]).

**The constraints.**

1. **Cost (0) to (2)**, so the Marketplace prices one at 5, 10 or 20 Credits (twice that Radiant, ST3).
2. **No Credits.** None makes Credits or sells a Resource, and none gains a Resource worth more than it
   costs: a card bought for 5 to 20 that returns more would be a money pump in the shared Marketplace. So
   T-AI-2 Mining Subroutine gains only a Common or Rare Resource, whose average sale is about 4 Credits
   (Radiant 8) against its price of 5 (Radiant 10).
3. **Verbs on main or in the S parts S21 waits for** (S03, S04, S06, S07, S15, S20): a drone summoned, a
   Resource gained, a Starship Piece Discovered or added, the launch price, Cancel an attack and Freeze, a
   Scanning Probe, a random Combat Keyword, a keyword given.
4. **Names** unique, none a §6 rules word, none naming another card by accident.
5. **The T-AI mix**: three Units, four Spells, two Traps and a Field Spell (Classic+'s ten are two, five,
   two and one; Salvage Protocol is a Unit because it is a Starship Piece).
6. **One Stinkyscape system each** (the drones, the Resources, the Starship, the Anomalies, the combat
   keywords), each still playable in a deck that has none.

**The ten.**

| Index | Name | Type (cost) | Base | Radiant | The idea |
| --- | --- | --- | --- | --- | --- |
| T-AI-1 | Escort Protocol | Unit (1), 1/3 Flying | Cry: Summon a Drone Interceptor. | 2/6; a Radiant Drone Interceptor | An escort: two flying bodies for (1) |
| T-AI-2 | Mining Subroutine | Spell (0) | Gain a random Common or Rare Stinkyscape Resource. | A Radiant one | A free roll into the tray that never pays for itself in Credits |
| T-AI-3 | Route Optimizer | Spell (1) | Discover a Starship Piece. It costs (1) less. | A Radiant one | The optimizer picks the part the ship needs |
| T-AI-4 | Predictive Shield | Trap (1) | Reveals when an enemy Unit attacks one of your Units: Cancel the attack and Freeze the attacker. | Also when it attacks you | It saw the attack coming, and the next one |
| T-AI-5 | Launch Sequencer | Spell (1) | Your Starship launch costs (1). Draw 1. | (0); draw 2 | A cantrip that makes the launch cheap |
| T-AI-6 | Probe Relay | Spell (1) | Add 2 Scanning Probe Mk1s to your hand. They cost (0). | 2 Scanning Probe Mk2s | Two free scans toward an Anomaly |
| T-AI-7 | Combat Subroutine | Unit (2), 2/2 | Cry: Give each of your other Units a random Combat Keyword. | 4/4; 2 each | A team upgrade from the combat keyword pool |
| T-AI-8 | Drone Uplink | Field Spell (2) | End of turn: Summon a Drone Interceptor. It gains Starship Piece. | A Radiant one | Drones that build the ship as they die |
| T-AI-9 | Honeypot | Trap (1) | Reveals when your opponent plays a Unit: Summon a Drone Interceptor. Give it Taunt. | 2 of them | A decoy that draws the attack |
| T-AI-10 | Salvage Protocol | Unit (1), 1/2 Starship Piece | Death: Add a random Starship Piece to your hand. | 2/4; a Radiant one | A piece that salvages another as it fuses |

**Where they differ from the planner's proposal** (the plan of #1857, section 5). Their names, types,
costs, stats and keywords are the proposal's; their texts are refined:

- T-AI-2 gains a Common or Rare Resource, not any: over the whole pool the average sale, about 5.8 Credits,
  beat its price of 5 (constraint 2).
- T-AI-3 Discovers its piece, an optimizer's choice, rather than drawing one at random.
- T-AI-4 cancels the attack it springs on, since a Freeze lands after the attack is declared and would not
  stop it; its Radiant face guards the hero too, rather than buffing the Unit attacked.
- T-AI-5 draws, so it is never a dead card without a Starship.
- T-AI-6 adds two Probes: one Mk1 at (0) for (1) would have gained nothing.
- T-AI-8's drones are Starship Pieces, so it is not #168 Drone Hanger at the other end of the turn.
- T-AI-9's drones have Taunt (a honeypot is a decoy), and its Radiant face summons two plain ones (R275's
  2×) rather than two Radiant ones (4×).

**How they are dealt.** #1853's Marketplace (pending PR #1963) offers one of the ten each cycle, S21's lot
over their ids (stock 1 to 3, a Radiant one 20% of the time at twice the price). Once the set ships they join
every AI pool ([[R1421]]): C+ #43 AI Slop (the (1) Cost ones), C+ #78 Claude's Datacenter, M #42's night
market, M #53, M #59 and M #60, which grow to 24 cards (the release re-pins R480's list).

## ST13. Cards, card by card

Entries in catalog order: each card with its tokens after it (N.k), then the set's shared entries (the
Dormant Starship, the drones, the Battleship's layers, the Resources and Anomaly Found!) and the ten AI
cards. Engine names are the Rust ones (`crates/engine/src/effects/*.rs` verbs, `script.rs` hooks). The
foundation wrote every entry from the designer's list and this plan; the S part that builds a card
corrects its entry, and its rows in [[§8.10]], [[§7]], BUILD M12 and `docs/radiant-audit.md`, where its
build decides otherwise, and says so. Where a decision in ST5 differs from an entry, the decision wins.

#### Stinkyscape #1 · Freeze Ray
`stinkyscape-001` · (0) Spell · Common · built by S13 (#1867)

> **Designer:** (0) Freeze Ray, Spell, Common, Stinkyscape, #1 · Freeze a Unit and its adjacent Units. ~~~ Freeze all enemy Units.

- **Text:** Freeze a Unit and the Units adjacent to it.
- **Radiant:** Freeze all enemy Units.
- **Engine:** A declared target, any Unit on the field ([[R81]]); `freeze { target }` on it and on each Unit acting in a zone beside it, in its row and on its side ([[§3.1]], `adjacent_to`), each [[R1174000]]'s Freeze: it loses its next attack or switch and thaws at the end of the turn it could have made one. A neighbour Immune to Spells is passed by ([[R81]]); Immutable stops none. Radiant: no target; `freeze_all` over the enemy's Units, a hero passed by ([[§6.3]] Freeze).
- **Rulings:** ST-S13-1: the adjacent Units are the target's neighbours in its row on its side, Cleave's adjacency ([[§3.1]]), whichever side the target stands on; a Unit with no neighbour is Frozen alone.
- **Numbers:** none (the card prints no number).
- **Check:** R275: up to three Units Frozen against every enemy Unit, with nothing to aim (one target to all). No typos.
- **Class:** A (Freeze).

#### Stinkyscape #2 · Absolute Zero
`stinkyscape-002` · (2) Spell · Rare · built by S14 (#1868)

> **Designer:** (2) Absolute Zero, Spell, Rare, Stinkyscape, #2 · Freeze a Unit forever. ~~~ Freeze a Unit forever. You may give it Indestructible.

- **Text:** Freeze a Unit forever.
- **Radiant:** Freeze a Unit forever. You may give it Indestructible.
- **Engine:** A declared target, any Unit ([[R81]]); `freeze { target, forever: true }` ([[R1848000]]): Frozen with its thaw stamped `FROZEN_FOREVER`, which no cleanup reaches, so only leaving the field ([[R78]]) or a Vanilla ends it; a change of control keeps it, and both views show `frozenForever`. Radiant: a choice of two modes made with the play ([[R81]]), give it Indestructible or not; given, `grant_keyword { Indestructible }` on it, a granted keyword it keeps until it leaves the field, a Vanilla included ([[§10.4]]).
- **Rulings:** ST-S14-1: "You may" is a choice of two modes made with the play ([[R81]]), not a prompt after it; given, the Indestructible stays until the Unit leaves the field. An ordinary Freeze on it later changes nothing ([[R1848000]]: the later thaw is kept).
- **Numbers:** none (the card prints no number).
- **Check:** R275: a rider: Indestructible keeps an enemy Unit Frozen in its zone for good, never destroyed to free it, or makes one of yours a wall. As written.
- **Class:** C (M14's forever Freeze).

#### Stinkyscape #3 · Meteor
`stinkyscape-003` · (3) Spell · Epic · built by S14 (#1868)

> **Designer:** (3) Meteor, Spell, Epic, Stinkyscape, #3 · Deal 15 damage to a Unit. Lock its zone. Tramples onto adjacent Units. ~~~ Deal 20 damage to a Unit. Lock its zone. Tramples onto adjacent Units and the enemy hero.

- **Text:** Deal {damage} damage to a Unit, trampling onto the Units adjacent to it. Lock its zone.
- **Radiant:** Deal {damage} damage to a Unit, trampling onto the Units adjacent to it and then its hero. Lock its zone.
- **Engine:** A declared target, any Unit ([[R81]]); `damage { to, amount, trampleOnto: adjacent }` ([[R1848001]]): the hit tramples, the target counting it only up to its Health ([[§4.4]] step 5), and its excess goes to one neighbour in its row on its side, drawn from the match rng when there are two, as a new instance of the same hit counted up to that Unit's Health, then what passes it to the other; a neighbour Immune to Spells is passed by and one that takes the hit whole stops it. Spell Damage raises the first hit alone. Then `lock` on the target's zone, whether the Unit lived or not (a Lock evicts nothing, [[§6.3]] Lock). Radiant: `trampleOnto: adjacentAndHero`, what passes both neighbours (all of the excess with none) going to the target's controller's hero.
- **Rulings:** ST-S14-2: "the enemy hero" is the target's controller's hero ([[R1848001]]), the enemy's whenever the Meteor is aimed at an enemy Unit; "Lock its zone" Locks the target's zone after the hit, emptied or not.
- **Numbers:** damage 15 ↑ (Radiant 20, step 4).
- **Check:** R275: 20 against 15 (1.33×) and the excess on to the hero, a rider. "Tramples onto adjacent Units" is M14's verb ([[§6.3]] Trample onto adjacent Units).
- **Class:** C (M14's Trample onto adjacent Units; Lock).

#### Stinkyscape #4 · Olivian Expanse
`stinkyscape-004` · (4) Spell · Mythic · built by S22 (#1870)

> **Designer:** (4) Olivian Expanse, Spell, Mythic, Stinkyscape, #4 · Discover a Terminus card. If your Deck is empty, it costs (0). ~~~ Discover a Terminus card. If your Deck is empty, it costs (0). If this is also the last card in your hand, it gains Win the Game.

- **Text:** Discover a Terminus card. If your deck is empty, it costs (0).
- **Radiant:** Discover a Terminus card. If your deck is empty, it costs (0). If this was also the last card in your hand, you win the game when you play it.
- **Engine:** A Discover ([[§6.3]], [[§10.8]]): three different options from the Terminus cards, the set's six Terminus tokens S #4.1–#4.6 ([[R1857002]]) and Core #100 Ceaseless Void (tagged Terminus since [[R1837000]]). Terminus is no `POOL_TOKEN_TAGS` tag ([[R382]]), so S22's query asks for the tokens, naming the set or the ids while the set is held ([[R1420]]). If its caster's library is empty as it resolves, the pick takes a `costOverride` of 0; a full hand burns it ([[§2.4]]). Radiant: if the caster's hand also holds no other card as it resolves (this one is out of the hand by then), NEW (S22): the pick carries a win, so its play, a cast included ([[R70]]), sets its player's won flag (`winGame`, held at the state check that closes that play, a hero at 0 losing first, [[R850]]).
- **Rulings:** ST-S22-1: "it gains Win the Game" is the discovered card's: the win rides on that one instance until it is played or leaves the game, and a copy or a Transform of it carries none. ST-S22-2: "the last card in your hand" is read as the Spell resolves: the hand holds no other card. ST-S22-3: the Terminus pool is the six tokens and Ceaseless Void, one of each, Discover's three without repeats.
- **Numbers:** none (the (0) is the price an empty deck sets).
- **Check:** R275: the Radiant face adds an outright win to the same Discover (a rider). Ceaseless Void at (0) is its strongest pick, as the designer's empty-deck line intends. ⚠ designer: whose win it is (E1).
- **Class:** C (a Terminus pool with tokens; a win carried by a card).

#### Stinkyscape #4.1 · Farewell
`stinkyscape-004-1` · (4) Spell, Terminus, Token · Token (printed Mythic) · built by S22 (#1870)

> **Designer:** (4) Farewell, Spell, Mythic, Terminus, Token, Stinkyscape, #4.1 · Exile all permanents and graveyards. ~~~ Exile all permanents, graveyards, hands, and decks.

- **Text:** Exile all permanents and every card in both graveyards.
- **Radiant:** Exile all permanents and every card in both graveyards, both hands and both decks.
- **Engine:** Exile ([[§6.3]]): every permanent on both sides, the unit zones and then the backrow, a Stack pile whole and face-down cards included, in [[R68]]'s order, then every card in both graveyards; the Radiant face then both hands and both libraries. A Dormant card stays, since no effect reaches it ([[R1839000]]); an exile fires no Death ([[§6.2]]), and a unit token ceases to exist ([[R11]]). Farewell itself goes to its owner's graveyard once it resolves ([[§3.2]]). An emptied library is fatigue at the next draw.
- **Rulings:** ST-S22-4: the field goes first, then the graveyards, so nothing the field loses lands in a graveyard already emptied; the Radiant face's hands include its caster's; a Dormant Starship stays.
- **Numbers:** none.
- **Check:** R275: two more zones of both players (a broader scope).
- **Class:** A (Exile over scopes).

#### Stinkyscape #4.2 · KY's Last Exam
`stinkyscape-004-2` · (4) Spell, Terminus, KY, Token · Token (printed Mythic) · built by S22 (#1870)

> **Designer:** (4) KY's Last Exam, Spell, Mythic, Terminus, KY, Token, Stinkyscape, #4.2 · Solve an impossible problem. If you get it right, deal 30 damage to the enemy hero and Echo this. ~~~ Solve an impossible problem. If you get it right, deal 30 damage to the enemy hero and Echo this. You snuck in a cheat sheet.

- **Text:** Solve an impossible problem. If you get it right, deal {damage} damage to the enemy hero, and this gains Echo +1.
- **Radiant:** Solve an impossible problem. You snuck in a cheat sheet: the right answer is marked. If you get it right, deal {damage} damage to the enemy hero, and this gains Echo +1.
- **Engine:** C+ #42's `answer` prompt ([[R420]]): one problem from a bank of impossible ones (NEW, S22: the bank, data in `crates/cards` as `ky_test_bank.rs` is), four options under letters in an rng order, the key kept in the engine ([[R465]]). No answerer can work it out, so each guesses: the player, the AI, the random policy and a timeout ([[R79]]). Right: one hit of 30 on the enemy hero ([[§4.4]], its caster's Spell Damage raising it), then NEW: the resolving card gains Echo +1 (Core #79 Twinspell's grant), one more repeat on `state.echoQueue` ([[§6.2]] Echo), each asking a new problem, the chain capped by a constant S22 names (CLAUDE.md rule 9). Wrong: nothing more, the Spell still played. Radiant: NEW, the cheat sheet: the prompt marks the right option for its chooser alone; the other seat's view and the AI's redaction of it stay keyless ([[R465]], [[R185]]).
- **Rulings:** ST-S22-5: "impossible" means no statement gives its answer away, so the base face is a one-in-four guess for every answerer, the AI included. ST-S22-6: "Echo this" is one more resolution per right answer, so a streak goes on until a miss, the end of the game ([[R216]]) or the cap. ST-S22-7: the cheat sheet shows the key to its answerer only; [[R465]] holds for every other seat.
- **Numbers:** damage 30 ↑ (step 5).
- **Check:** R275: a one-in-four guess → a sure answer, about 4× the damage, every repeat sure too. ⚠ designer: what the problem is (E4).
- **Class:** C (the answer prompt; a conditional Echo; a marked answer).

#### Stinkyscape #4.3 · Event Horizon
`stinkyscape-004-3` · (4) Spell, Terminus, Token · Token (printed Mythic) · built by S22 (#1870)

> **Designer:** (4) Event Horizon, Spell, Mythic, Terminus, Token, Stinkyscape, #4.3 · Take an extra turn. ~~~ Echo. Take an extra turn.

- **Text:** Take an extra turn after this one.
- **Radiant:** Echo 1 / Take an extra turn after this one.
- **Engine:** Extra turn ([[§6.3]], `takeExtraTurn`): one more turn owed to its caster, taken as this one ends, a whole turn ([[R845]], [[R846]]). No per-game limit: M #19.1 Temporal Rift's flag is that card's alone ([[R847]]). Radiant: Echo 1 (`StaticFlags.echo`, [[§6.2]]), so it resolves twice and two turns are owed.
- **Rulings:** ST-S22-8: no per-game cap, unlike Temporal Rift; two Event Horizons owe two turns.
- **Numbers:** none (Echo 1 is a keyword's count).
- **Check:** R275: Echo 1, two extra turns against one (2×).
- **Class:** A.

#### Stinkyscape #4.4 · Last Word
`stinkyscape-004-4` · (4) Trap, Terminus, Token · Token (printed Mythic) · built by S22 (#1870)

> **Designer:** (4) Last Word, Trap, Mythic, Terminus, Token, Mythic, Stinkyscape, #4.4 · Reveal when you take lethal damage: End the Turn. You then take an extra turn where your Units have Charge. Lose the game at the end of that turn. ~~~ Reveal when you take lethal damage: End the Turn. You then take an extra turn where you have 100 mana and your Units have Charge. Lose the game at the end of that turn.

- **Text:** Reveals when your hero would take lethal damage: Prevent it and end the turn. The next turn is yours: your Units have Charge during it, and you lose the game at its end.
- **Radiant:** Reveals when your hero would take lethal damage: Prevent it and end the turn. The next turn is yours: you have {mana} mana and your Units have Charge during it, and you lose the game at its end.
- **Engine:** A face-down Trap on the *would take lethal damage* replacement of its controller's hero ([[§6.2]] Replacement, [[§4.4]] step 4a, [[R1204]]'s judge), on either player's turn, in combat or by an effect; it fires there and goes to the graveyard ([[§3.2]]). The hit is prevented: it deals nothing. Then End the turn ([[§6.3]]: the rest of the list resolves first). NEW (S22): the next turn is its controller's, an extra turn ([[R845]]) when the hit came on their own turn and their own next turn when it came on the opponent's; during it their Units have Charge (a player modifier read where [[§6.1]] Charge is, gone at its cleanup), and as it ends, after its end-of-turn steps, the opponent wins the game (`winGame`, [[R850]]). Radiant: their mana is set to 100 as that turn starts, past `MAX_MANA` ([[§6.3]] Mana).
- **Rulings:** ST-S22-9: "lethal damage" is the lethal guard's judge, after Armor and shields, on its controller's hero only. ST-S22-10: "You then take an extra turn" is always the very next turn: on the opponent's turn it is the controller's normal next turn, never a second one after it. ST-S22-11: "Lose the game" is the other player's win (`winGame`), so it needs no new ending; a hero at 0 still loses first.
- **Numbers:** mana 100 ↑ (the Radiant face only, step 25).
- **Check:** Designer's "End the Turn" is End the turn (R366). R275: the last turn with 100 mana against a normal refresh (a rider). ⚠ designer: E5.
- **Class:** C (a last turn: Charge for a turn and a loss at its end).

#### Stinkyscape #4.5 · 1v1 Final Destination
`stinkyscape-004-5` · (4) Trap, Terminus, Token · Token (printed Mythic) · built by S22 (#1870)

> **Designer:** (4) 1v1 Final Destination, Trap, Mythic, Terminus, Token, Mythic, Stinkyscape, #4.5 · Lock all but one random lane. Set each hero's health to 1\. Both players discard all non-permanents. ~~~ Lock all but one random lane. Set each hero's health to 1\. Both players discard all non-permanents. Fill your hand with (0) cost Book of Buffs.

- **Text:** Reveals when your opponent's turn ends: Lock every lane but one, picked at random. Set each hero's health to 1. Both players discard every Spell in their hand.
- **Radiant:** Reveals when your opponent's turn ends: Lock every lane but one, picked at random. Set each hero's health to 1. Both players discard every Spell in their hand. Then fill your hand with Book of Buffs. They cost (0).
- **Engine:** A face-down Trap in the end-of-turn trap window of its controller's opponent's turn ([[R62]], ST8), going to the graveyard as it fires. One lane drawn from the match rng stays open and the other four are Locked whole ([[§6.3]] Lock, C #71's lane; a Lock evicts nothing, [[R688]]). Then Set health ([[§6.3]]): each hero's health is 1, no pipeline ([[R18]]). Then each player discards every Spell in their hand, the non-permanents ([[§5.1]]): a Discard ([[§6.3]]), in hand order, no choice. Radiant: then its controller's hand is filled (their hand size less the cards held, [[§2.4]]) with C+ #71 Book of Buff, each at `costOverride` 0. Its controller's turn comes next, so they strike first.
- **Rulings:** ST-S22-12: "non-permanents" are Spells ([[§5.1]]'s permanents are Units, Field Spells, Traps and Field Traps); a Resource is in the tray, not the hand, and stays ([[R1840000]]). ST-S22-13: the open lane is drawn uniformly from the five; Units already in a Locked lane stay and fight.
- **Numbers:** none.
- **Check:** ST8: it reveals as the opponent's turn ends. "Book of Buffs" is C+ #71 Book of Buff (ST5 D6). R275: a hand of free Buffs on top (a rider).
- **Class:** B (Lock, Set health, Discard; a lane drawn).

#### Stinkyscape #4.6 · The Big Crunch
`stinkyscape-004-6` · (4) Spell, Terminus, Token · Token (printed Mythic) · built by S22 (#1870)

> **Designer:** (4) The Big Crunch, Spell, Mythic, Terminus, Token, Mythic, Stinkyscape, #4.6 · Cast every Call to Chaos. ~~~ Cast every Radiant Call to Chaos.

- **Text:** Cast one of each Call to Chaos.
- **Radiant:** Cast a Radiant one of each Call to Chaos.
- **Engine:** Cast ([[§6.3]], `cast_new`): one new card of each Call to Chaos definition, in catalog order (Core #95, C+ #73, M #95, S #150 and S #151, the set's own two named by id while it is held, [[R1420]]), each a play for every rule, its choices its caster's ([[R70]]), each rolling and naming its own effects ([[R436]]). Radiant: each cast Radiant, so three effects each. Their nested random casts count toward `RANDOM_CAST_CHAIN_CAP` ([[R452]]).
- **Rulings:** ST-S22-14: "every Call to Chaos" is one of each edition, never every copy in the game.
- **Numbers:** none.
- **Check:** R275: every edition Radiant, three effects each against one (3×).
- **Class:** B (five casts).

#### Stinkyscape #5 · Masochism Unmasked
`stinkyscape-005` · (1) Field Trap · Rare · built by S15 (#1875)

> **Designer:** (1) Masochism Unmasked, Field Trap, Rare, Stinkyscape, #5 · Reveals when you take damage on your turn: Redirect all damage you take on your turn to your opponent (up to 20). ~~~ Reveals when you take damage on your turn: Redirect all damage you take on your turn to your opponent and double it (up to 40).

- **Text:** Reveals when you take damage on your turn: Redirect all damage you take on your turn to your opponent, up to {cap} each turn.
- **Radiant:** Reveals when you take damage on your turn: Redirect all damage you take on your turn to your opponent, up to {cap} each turn, and double it.
- **Engine:** A Field Trap: set face-down, it reveals on the `damage` of a hit on its controller's hero during their own turn, after the hit, so that hit is taken ([[R1849004]]). Face-up it stays and acts: `staticFlags.ownTurnRedirect { cap }` (Radiant with `multiplier: 2`): each later hit on its controller's hero on their own turn moves at [[§4.4]] step 0a, after Spell Damage and before Divine Shield, the Armor and the caps; as much of it as the cap has left this turn is taken off and dealt to the opponent's hero as a new instance from the same source (doubled on the Radiant face, so at most 40 a turn), `redirected` reporting it, and the rest goes on through its controller's own pipeline. The cap counts what is taken off, per card and per turn (`memory.ownTurnRedirect`, cleared as it leaves the field, [[R78]]); on the opponent's turn nothing moves; several such cards take a hit in [[R68]]'s order, each to its own cap.
- **Rulings:** ST-S15-1: "on your turn" is its controller's own turn (an extra turn is one); the cap is per turn (ST9's default) and counts what is taken; the Radiant face's "double it (up to 40)" doubles what is dealt on the same cap of 20 ([[R1849004]]).
- **Numbers:** cap 20 ↑ (step 5); the Radiant doubling is the card's word (`multiplier` 2 in its flags), no param.
- **Check:** ST9's default (the cap per turn) and [[R1849004]]'s Radiant reading. R275: what moves is doubled (2×). Self-damage lines it up, such as the asteroids' "Take N damage" (S #169–#172).
- **Class:** C (M15's own-turn redirect).

#### Stinkyscape #6 · Tylannt
`stinkyscape-006` · (2) Unit, Acclaimed, Human · Mythic · 6/4 → 12/8 · built by S09 (#1873)

> **Designer:** (2) Tylannt, Unit, Mythic, Acclaimed, Human, Mythic, Stinkyscape, #6 · 6/4 / Tradable / Cry: Complete the current stage on all your Quests. ~~~ 12/8 / Tradable / Cry: Summon The Greatest Heist. Complete the current stage on all your Quests.

- **Text:** Tradable / Cry: Complete the current stage on all your Quests.
- **Radiant:** Tradable / Cry: Summon The Greatest Heist. Complete the current stage on all your Quests.
- **Engine:** Tradable ([[R1838000]]). Cry ([[R1]]): `complete_quests` ([[R1854001]]): every Quest-tagged card its controller has acting on the field, in lane order, has the quests open on it completed, a staged quest's current stage and C #90 In Too Deep's open quests alike, each reward given at the next state check; a line hearing back or done has nothing open. Radiant: `summon { defId: stinkyscape-007 }` into the leftmost open backrow zone first ([[R64]]); a summon that names its card fires its Cry ([[R1]], [[R411]]), so S #7's Discover is asked, and its line, not yet open, opens its first stage and has it completed ([[R1854001]]).
- **Rulings:** ST-S09-1: the Radiant summon names The Greatest Heist, so its Cry runs before the completion; a full backrow summons nothing and the completion reaches the other Quests alone. ST-S09-2: the opponent's Quests and an untagged card with a quest line are passed by (R1854001). These are proposals for S09, which takes R-numbers from its own block. No R-number here.
- **Numbers:** none (a verb and a summon).
- **Check:** R275: stats doubled, and the Radiant face brings its own Quest and completes its first stage at once. Tradable as printed (L2).
- **Class:** C (staged Quests, M07).

#### Stinkyscape #7 · The Greatest Heist
`stinkyscape-007` · (3) Field Spell, Quest · Legendary · built by S09 (#1873)

> **Designer:** (3) The Greatest Heist, Field Spell, Quest, Legendary, Stinkyscape, #7 · Cry: Discover an [Radiant] Acclaimed Unit or a Jellybean. / Empty the VC vault! /  / * Stage 1: Initiation /   * Quest: Make an Acclaimed Unit Radiant. /   * Reward: Summon a random [Radiant] Acclaimed Unit. / * Stage 2: Comradery /   * Quest: Have 2 Acclaimed Units you control die. /   * Reward: Add 3 random [Radiant] Anomalies or Resources to your hand. / * Stage 3: Empty the Vaults /   * Quest: Steal an enemy card. /   * Reward: Steal your enemy's mana crystals.

- **Text:** Cry: Discover an Acclaimed Unit or a Jelly Bean. / Quest: Stage 1, Initiation. Make an Acclaimed Unit Radiant. Reward: Summon a random Acclaimed Unit. / Quest: Stage 2, Comradery. Have 2 Acclaimed Units you control die. Reward: Do this {finds|time|times}: at random, add an Anomaly Found! to your hand or gain a random Stinkyscape Resource. / Quest: Stage 3, Empty the Vaults. Steal an enemy card. Reward: Steal your opponent's mana crystals.
- **Radiant:** Cry: Discover a Radiant Acclaimed Unit or a Radiant Jelly Bean. / Quest: Stage 1, Initiation. Make an Acclaimed Unit Radiant. Reward: Summon a random Radiant Acclaimed Unit. / Quest: Stage 2, Comradery. Have 2 Acclaimed Units you control die. Reward: Do this {finds|time|times}: at random, add a Radiant Anomaly Found! to your hand or gain a random Radiant Stinkyscape Resource. / Quest: Stage 3, Empty the Vaults. Steal an enemy card. Reward: Steal your opponent's mana crystals.
- **Engine:** A staged Quest ([[R1854000]]): `Script.stages`, three stages of one goal each, the first opened as the card enters the field, each reward's last effect `next_stage`, the line done after Stage 3. Cry ([[R1]]): one Discover over one pool, the non-token Acclaimed Units and the Jelly Bean cards (Core #26–#29, [[R1837000]]) of the sets that ship ([[R380]]); Radiant: the options Radiant. Stage 1: `unitsMadeRadiant { count: 1, tag: Acclaimed }` ([[R1854002]]), reward a random non-token Acclaimed Unit summoned ([[R64]], no Cry, [[R1]]). Stage 2: `unitsDied { count: 2, tag: Acclaimed }`, reward finds rolls from the match rng, each at even odds an Anomaly Found! added to the hand ([[§2.4]]'s cap; #1855's board swap, pending) or a Stinkyscape Resource gained into the tray by [[R1840002]]'s roll ([[R1840000]]: a Resource never enters a hand). Stage 3: `cardsStolen { count: 1 }`, reward `steal_mana_crystals` ([[R1854003]]). Radiant: the summoned Unit, the Anomaly Found! and the Resource Radiant.
- **Rulings:** ST-S09-3: "an Acclaimed Unit or a Jelly Bean" is one Discover over the union, so a Jelly Bean (a hand card, Core #26's pick) can meet Stage 1 (R1854002). ST-S09-4: "3 random Anomalies or Resources" is three rolls, each an Anomaly Found! or a Resource at even odds. ST-S09-5: a stage line reads `Quest: Stage N, <name>. <goal>. Reward: <reward>.`, since `Quest:` starts its own line (R366). These are proposals for S09, which takes R-numbers from its own block. No R-number here.
- **Numbers:** finds 3 ↑ (Radiant 3). The stage ordinals and Stage 2's count of 2 are the line's structure (`FIXED` STRUCTURE, R1748003).
- **Check:** ST8: "Jellybean" is a Jelly Bean (L1's tag). The list prints one face with [Radiant] marks; the Radiant face is built from them. "Empty the VC vault!" is flavour. Resources are gained, never added to a hand (R1840000). R275: every reward and the Discover Radiant.
- **Class:** C (staged Quests, M07; Resources, M03).

#### Stinkyscape #8 · Conjure Rosemary Triscuit
`stinkyscape-008` · (2) Spell · Rare · built by S13 (#1867)

> **Designer:** (2) Conjure Rosemary Triscuit, Spell, Rare, Stinkyscape, #8 · Add a Rosemary Triscuit to your hand. Shuffle a Wardrum into your deck. ~~~ Add a Radiant Rosemary Triscuit to your hand. Shuffle a Radiant Wardrum into your deck.

- **Text:** Add a Rosemary Triscuit to your hand. Shuffle a Wardrum into your deck.
- **Radiant:** Add a Radiant Rosemary Triscuit to your hand. Shuffle a Radiant Wardrum into your deck.
- **Engine:** `add_to_hand { defId: stinkyscape-008-1 }`, S #8.1, a full hand burning it ([[§2.4]]), then `shuffle_into { defId: classicplus-037 }`, a new C+ #37 Wardrum at a random position ([[R311]]), a full deck refusing it ([[R80]]). Radiant: both `radiant: true`. The Triscuit is a (0) non-Unit play, which counts toward the Wardrum's own summon from the deck (C+ #37).
- **Rulings:** none new.
- **Numbers:** none (one of each).
- **Check:** R275: both cards Radiant (Refresh 4 against 2; a Radiant Wardrum). No typos.
- **Class:** B.

#### Stinkyscape #8.1 · Rosemary Triscuit
`stinkyscape-008-1` · (0) Spell, Token · Token (printed Rare) · built by S13 (#1867)

> **Designer:** (0) Rosemary Triscuit, Spell, Token, Rare, Stinkyscape, #8.1 · Refresh (2) mana. ~~~ Refresh (4) mana.

- **Text:** Refresh {refresh} mana.
- **Radiant:** Refresh {refresh} mana.
- **Engine:** `refresh_mana { amount }` ([[§6.3]] Refresh X, [[R364]]): its caster's current mana rises by up to 2, never above their max mana; at or above max, nothing happens. Radiant: 4.
- **Rulings:** none new ([[R364]]).
- **Numbers:** refresh 2 ↑ (Radiant 4).
- **Check:** "Refresh (2) mana" is Refresh X ([[§6.3]]). R275: 4 against 2 (2×).
- **Class:** A.

#### Stinkyscape #9 · Ticketed
`stinkyscape-009` · (2) Field Spell · Common · built by S15 (#1875)

> **Designer:** (2) Ticketed, Field Spell, Common, Stinkyscape, #9 · Brittle 3 / Cry: Stack a permanent under this. / Death: return that permanent to its controller. ~~~ Cry: Stack a permanent under this. / Death: return that permanent to its controller.

- **Text:** Brittle 3 / Cry: Hold another permanent beneath this. / When this leaves the field, the permanent beneath it returns to its controller.
- **Radiant:** Cry: Hold another permanent beneath this. / When this leaves the field, the permanent beneath it returns to its controller.
- **Engine:** A Field Spell whose Cry runs as it is played, its target declared with the play ([[R81]]): another face-up permanent acting on the field, either side. `hold_beneath { target }` ([[R1850003]]): it leaves the field as an exiled card does, no Death, reset ([[R78]]), a card beneath it in a pile resuming ([[§3.2]]), and `held` reports it; it waits in `state.held` beneath Ticketed. At the first state check after Ticketed leaves the field, any way (Brittle 3 crumbling it, [[R638]]; a destroy, a Bounce, an exile), it is summoned for its controller as it was taken: [[R64]]'s zone, summoning sick, no Cry ([[R1]]), or exiled with no zone open. A unit token ceases to exist as it is taken ([[R11]]); a face-down card is never held. Radiant: no Brittle, so it holds until Ticketed is removed.
- **Rulings:** ST-S15-2: "Stack a permanent under this" is Hold beneath ([[§6.3]]), not the Stack keyword, so the face prints that verb; "Death: return that permanent to its controller" is Ticketed leaving the field any way, as [[R1850003]] releases it (release at the holder's Death alone is its rejected reading); the target is another face-up permanent of either side.
- **Numbers:** none (Brittle's 3 is a numbered keyword, [[R482]]).
- **Check:** The designer's "Stack … under this" and "Death: return" are written in [[R1850003]]'s terms. R275: the Radiant face drops Brittle 3, a lighter drawback (R275's drawback case), so it holds until removed.
- **Class:** C (M16's hold beneath).

#### Stinkyscape #10 · Otherworldly Endurance
`stinkyscape-010` · (2) Spell, Otherworldly · Epic · built by S13 (#1867)

> **Designer:** (2) Otherworldly Endurance, Stinkyscape, Spell, Otherworldly, Epic, #10 · Add an Inner Fire, a Divine Spirit, and a Power Word Shield to your hand. ~~~ Add a Radiant Inner Fire, a Divine Spirit, and a Power Word Shield to your hand.

- **Text:** Add an Inner Fire, a Divine Spirit and a Power Word Shield to your hand.
- **Radiant:** Add a Radiant Inner Fire, a Radiant Divine Spirit and a Radiant Power Word Shield to your hand.
- **Engine:** Three `add_to_hand`, S #10.1 Inner Fire, S #10.2 Divine Spirit and S #10.3 Power Word Shield, in that order (C+ #32's pattern), a full hand burning what does not fit ([[§2.4]]). Radiant: each `radiant: true`.
- **Rulings:** ST-S13-2: the Radiant face adds all three Radiant, as #11's, #12's and C+ #32's do (the designer's "a Radiant Inner Fire, a Divine Spirit, and a Power Word Shield" read as the Radiant reaching the list).
- **Numbers:** none (one of each).
- **Check:** C+ #32 Otherworldly Removal's shape: an Otherworldly Spell adding three Other-Origins tokens ([[R1837000]]). R275: the three Radiant.
- **Class:** B.

#### Stinkyscape #10.1 · Inner Fire
`stinkyscape-010-1` · (1) Spell, Other-Origins, Token · Token (printed Epic) · built by S13 (#1867)

> **Designer:** (1) Inner Fire, Stinkyscape, Spell, Token, Other-Origins, Epic, #10.1 · Set a units attack equal to its health ~~~ Set a units attack equal to its [health + 4 (if ally)] or [health - 4 (if enemy)]

- **Text:** Set a Unit's Attack equal to its Health.
- **Radiant:** Set a Unit's Attack equal to its Health, plus {shift} if it's yours or minus {shift} if it's an enemy.
- **Engine:** A declared target, any Unit ([[R81]]). Hearthstone's Inner Fire on existing primitives: `buff { attack: Health − Attack }` on it, both read through the layers as it resolves ([[§10.4]]), so it shows its Health as its Attack; a Unit whose Attack is higher loses the difference, never below 0. Radiant: its Health +4 when its caster controls it, −4 when it is an enemy's, Attack never below 0.
- **Rulings:** ST-S13-3: "Set … equal to" reads the Unit's stats now, auras included, and is done as a Buff of the difference, an existing primitive, so an Attack aura that leaves later takes its part with it; a set of Attack alone at layer 1 (as `set_stats_all` sets both, [[§6.3]] Set stats) would be NEW, S13's call. "If ally" is a Unit its caster controls.
- **Numbers:** shift 4 ↑ (the Radiant face alone, `tunedOn: radiant`, [[R482]]).
- **Check:** Hearthstone's Inner Fire. R275: the Radiant face's ±4, a rider either way.
- **Class:** B.

#### Stinkyscape #10.2 · Divine Spirit
`stinkyscape-010-2` · (2) Spell, Other-Origins, Token · Token (printed Epic) · built by S13 (#1867)

> **Designer:** (2) Divine Spirit, Stinkyscape, Spell, Token, Other-Origins, Epic, #10.2 · Set a units health to double its health ~~~ Set a units health to triple its health

- **Text:** Double a Unit's Health.
- **Radiant:** Triple a Unit's Health.
- **Engine:** A declared target, any Unit ([[R81]]). Hearthstone's Divine Spirit: `buff { health: its Health now }` (Radiant twice that), its current and max Health rising together ([[§10.4]]), so a damaged Unit doubles what it has left.
- **Rulings:** ST-S13-4: "Set a units health to double its health" is Hearthstone's Divine Spirit, a Buff of its Health now, not a set: a 3/4 with 2 Health left becomes a 3/6 with 4 left.
- **Numbers:** none (the doubling and the tripling are the card's words, as Hearthstone prints them).
- **Check:** R275: three times against twice, so the gain doubles (+its Health → +twice it).
- **Class:** B.

#### Stinkyscape #10.3 · Power Word Shield
`stinkyscape-010-3` · (0) Spell, Other-Origins, Token · Token (printed Epic) · built by S13 (#1867)

> **Designer:** (0) Power Word Shield Stinkyscape, Spell, Token, Other-Origins, Epic, #10.3 · Give a unit +2 health; Draw 1 ~~~ Give a unit +4 health; Draw 2

- **Text:** Give a Unit +{health} Health. Draw {draw|card|cards}.
- **Radiant:** Give a Unit +{health} Health. Draw {draw|card|cards}.
- **Engine:** A declared target, any Unit ([[R81]]); `buff { health }` (+0/+2), then `draw { count }` ([[§2.4]]). Radiant: +4 and 2 cards.
- **Rulings:** none new.
- **Numbers:** health 2 ↑ (Radiant 4); draw 1 ↑ (Radiant 2).
- **Check:** The designer's "Give a unit +2 health; Draw 1" in house style. R275: 4 and 2 against 2 and 1 (2×).
- **Class:** B.

#### Stinkyscape #11 · Otherworldly Magic
`stinkyscape-011` · (1) Spell, Otherworldly · Epic · built by S13 (#1867)

> **Designer:** (1) Otherworldly Magic, Stinkyscape, Spell, Otherworldly, Epic, #11 · Add a Frostbolt, Arcane Missiles, and Flame Strike to your hand. ~~~ Add a Radiant Frostbolt, Arcane Missiles, and Flame Strike to your hand.

- **Text:** Add a Frostbolt, an Arcane Missiles and a Flame Strike to your hand.
- **Radiant:** Add a Radiant Frostbolt, a Radiant Arcane Missiles and a Radiant Flame Strike to your hand.
- **Engine:** Three `add_to_hand`, S #11.1 Frostbolt, S #11.2 Arcane Missiles and S #11.3 Flame Strike, in that order (C+ #32's pattern), a full hand burning what does not fit ([[§2.4]]). Radiant: each `radiant: true`.
- **Rulings:** none new (S #10's: all three Radiant on the Radiant face, as the designer wrote it here).
- **Numbers:** none (one of each).
- **Check:** C+ #32's shape ([[R1837000]]). R275: the three Radiant.
- **Class:** B.

#### Stinkyscape #11.1 · Frostbolt
`stinkyscape-011-1` · (1) Spell, Other-Origins, Token · Token (printed Epic) · built by S13 (#1867)

> **Designer:** (1) Frostbolt, Stinkyscape, Spell, Token, Other-Origins, Epic, #11.1 · Deal 3 damage to a target; Freeze it ~~~ Deal 5 damage to a target; Freeze it; Cleave

- **Text:** Deal {damage} damage and Freeze it.
- **Radiant:** Deal {damage} damage and Freeze it. Deal {damage} damage to the Units adjacent to it.
- **Engine:** A declared target, a Unit or a hero ([[R81]]); `damage { to, amount }`, then `freeze` on it, a hero passed by (heroes are never Frozen, [[§6.1]] Frozen) and so is a Unit the hit destroyed. Radiant: one hit on the target and one on each Unit beside it in its row on its side ([[§3.1]]; none beside a hero), all before one state check ([[R59]]), then the Freeze on the target alone. Spell Damage raises each hit ([[§4.4]] step 0).
- **Rulings:** ST-S13-5: "a target" is any Unit or hero; the Radiant face's "Cleave" is the Spell's hit on the target's neighbours, Cleave's adjacency ([[§3.1]]), not the keyword (a Unit's, in combat alone, [[§6.1]]); "Freeze it" Freezes the target alone.
- **Numbers:** damage 3 ↑ (Radiant 5).
- **Check:** Hearthstone's Frostbolt, its target unnamed as [[R751]] writes a face. R275: 5 against 3 (1.67×) and its neighbours hit too.
- **Class:** B.

#### Stinkyscape #11.2 · Arcane Missiles
`stinkyscape-011-2` · (0) Spell, Other-Origins, Token · Token (printed Epic) · built by S13 (#1867)

> **Designer:** (0) Arcane Missiles, Stinkyscape, Spell, Token, Other-Origins, Epic, #11.2 · Deal 1 damage to a random enemy; echo 2 ~~~ Deal 1 damage to a random enemy; echo 5

- **Text:** Echo 2 / Deal {damage} damage to a random enemy.
- **Radiant:** Echo 5 / Deal {damage} damage to a random enemy.
- **Engine:** `StaticFlags.echo` 2 (Radiant 5, [[§6.2]] Echo X): it resolves, then re-resolves twice (five times on the Radiant face), each time `damage` of 1 on one enemy, the enemy hero or an enemy Unit, drawn from the match rng as that resolution runs ([[R60]]), so a Unit the last one killed is not drawn; Spell Damage raises each hit ([[§4.4]] step 0).
- **Rulings:** ST-S13-6: "echo 2" is Echo 2 ([[§6.2]]), three resolutions in all (Radiant six), each drawing its own enemy among those still standing.
- **Numbers:** damage 1 ↑; the Echo count is a numbered keyword's ([[R482]]).
- **Check:** Hearthstone's Arcane Missiles, three hits of 1. R275: six hits against three (2×).
- **Class:** A.

#### Stinkyscape #11.3 · Flame Strike
`stinkyscape-011-3` · (2) Spell, Other-Origins, Token · Token (printed Epic) · built by S13 (#1867)

> **Designer:** (2) Flame Strike Stinkyscape, Spell, Token, Other-Origins, Epic, #11.3 · Deal 5 damage to all enemy units ~~~ Deal 10 damage to all enemy units

- **Text:** Deal {damage} damage to all enemy Units.
- **Radiant:** Deal {damage} damage to all enemy Units.
- **Engine:** `damage_all` over the enemy's Units, one hit each, before one state check ([[R59]]); no hero; Spell Damage raises each hit ([[§4.4]] step 0).
- **Rulings:** none new.
- **Numbers:** damage 5 ↑ (Radiant 10).
- **Check:** Hearthstone's Flamestrike. R275: 10 against 5 (2×).
- **Class:** A.

#### Stinkyscape #12 · Otherworldly Nightmare
`stinkyscape-012` · (1) Spell, Otherworldly · Epic · built by S13 (#1867)

> **Designer:** (1) Otherworldly Nightmare, Stinkyscape, Spell, Otherworldly, Epic, #12 · Add an Shadow Word Death, a Shadow Word Pain, and a Psychic Scream to your hand. ~~~ Add a Radiant Execute, Brawl, and Blade Storm to your hand.

- **Text:** Add a Shadow Word Death, a Shadow Word Pain and a Psychic Scream to your hand.
- **Radiant:** Add a Radiant Shadow Word Death, a Radiant Shadow Word Pain and a Radiant Psychic Scream to your hand.
- **Engine:** Three `add_to_hand`, S #12.1 Shadow Word Death, S #12.2 Shadow Word Pain and S #12.3 Psychic Scream, in that order (C+ #32's pattern), a full hand burning what does not fit ([[§2.4]]). Radiant: each `radiant: true`.
- **Rulings:** none new.
- **Numbers:** none (one of each).
- **Check:** ST8: the Radiant face adds the three Radiant, not C+ #32's Execute, Brawl and Blade Storm; the designer's "an Shadow Word Death" is "a". R275: the three Radiant.
- **Class:** B.

#### Stinkyscape #12.1 · Shadow Word Death
`stinkyscape-012-1` · (1) Spell, Other-Origins, Token · Token (printed Epic) · built by S13 (#1867)

> **Designer:** (1) Shadow Word Death, Stinkyscape, Spell, Token, Other-Origins, Epic, #12.1 · Destroy a unit with (5) or more attack ~~~ Destroy a unit with (5) or more attack; Cleave

- **Text:** Destroy a Unit with {attack} or more Attack.
- **Radiant:** Destroy a Unit with {attack} or more Attack, and the Units adjacent to it.
- **Engine:** A declared target, a Unit whose Attack, read through the layers ([[§10.4]]), is 5 or more as it is chosen and as it resolves ([[R81]]); `destroy` ([[§6.3]] Destroy: Indestructible ignores it). Radiant: `destroy_adjacent_to` as well (Core #16 Hit Job's Radiant), the Units beside it in its row on its side whatever their Attack, all marked before one state check ([[R59]]).
- **Rulings:** ST-S13-7: the Radiant face's "Cleave" destroys the target's neighbours too, whatever their Attack (S #11.1's adjacency); with no Unit to choose, the Spell can't be played.
- **Numbers:** attack 5 ↓ (a threshold: a lower one reaches more Units).
- **Check:** Hearthstone's Shadow Word: Death; the designer's "(5)" is an Attack, not a cost. R275: its neighbours too (one Unit to three).
- **Class:** B.

#### Stinkyscape #12.2 · Shadow Word Pain
`stinkyscape-012-2` · (0) Spell, Other-Origins, Token · Token (printed Epic) · built by S13 (#1867)

> **Designer:** (0) Shadow Word Pain, Stinkyscape, Spell, Token, Other-Origins, Epic, #12.2 · Destroy a unit with (3) or less attack ~~~ Destroy a unit with (3) or less attack; Cleave

- **Text:** Destroy a Unit with {attack} or less Attack.
- **Radiant:** Destroy a Unit with {attack} or less Attack, and the Units adjacent to it.
- **Engine:** A declared target, a Unit whose Attack, read through the layers ([[§10.4]]), is 3 or less as it is chosen and as it resolves ([[R81]]); `destroy` ([[§6.3]] Destroy). Radiant: `destroy_adjacent_to` as well, the Units beside it whatever their Attack ([[R59]]).
- **Rulings:** S #12.1's (ST-S13-7).
- **Numbers:** attack 3 ↑ (a ceiling: a higher one reaches more Units).
- **Check:** Hearthstone's Shadow Word: Pain; the designer's "(3)" is an Attack, not a cost. R275: its neighbours too.
- **Class:** B.

#### Stinkyscape #12.3 · Psychic Scream
`stinkyscape-012-3` · (3) Spell, Other-Origins, Token · Token (printed Epic) · built by S13 (#1867)

> **Designer:** (3) Psychic Scream Stinkyscape, Spell, Token, Other-Origins, Epic, #12.3 · Shuffle all permanents into your opponents deck ~~~ Shuffle all permanents into your opponents deck; they cost (2) more *{max (4)}*

- **Text:** Shuffle all permanents into your opponent's deck.
- **Radiant:** Shuffle all permanents into your opponent's deck. Each costs ({surcharge}) more (up to (4)).
- **Engine:** Every permanent on the field of both sides, the top of each pile and every backrow card, face-down ones included ([[R13]]), leaves the field, reset and with no Death ([[R78]]), and is shuffled into the opponent's deck at a random position ([[R311]]) as their card; a unit token ceases to exist ([[R11]]); a Dormant card is out of reach ([[R1839000]]); a full deck refuses the rest ([[R80]], [[R316]]). NEW: `shuffle_card_into` puts a card into its owner's library alone, so a card of its caster's must first become the opponent's (owner and controller, as a Steal off the field makes a card the taker's, [[R12]]); no verb on main moves a field card into another player's deck. Radiant: each card shuffled in gains `costMod` +2, never past (4) ([[R65]]'s cap, M #37's), a price a graveyard takes off ([[R766]]).
- **Rulings:** ST-S13-8: "all permanents" is every card on the field of both sides, face-down ones included, its caster's own too (Hearthstone's Psychic Scream takes every minion), each becoming the opponent's card in their deck; the Radiant face's "(2) more {max (4)}" is +2, up to (4).
- **Numbers:** surcharge 2 ↑ (the Radiant face alone, `tunedOn: radiant`); the (4) is a price ([[R1748003]]).
- **Check:** Hearthstone's Psychic Scream, read for every permanent. R275: a rider (each card shuffled in dearer).
- **Class:** C (NEW: a move into the other player's deck).

#### Stinkyscape #13 · Asphyxiants Cloud
`stinkyscape-013` · (2) Spell · Rare · built by S14 (#1868)

> **Designer:** (2) Asphyxiants Cloud, Stinkyscape, Spell, Rare, #13 · Deal 1 damage to all enemies, draw 1, your next spell you play this turn gains poisonous ~~~ Deal 1 damage to all enemies, draw 1, your next spell you play this turn gains poisonous / Echo 1 / *{should deal 1 poisonous damage to all enemies, if it doesn't please reword so it does}*

- **Text:** This and your next Spell this turn have Poisonous. Deal {damage} damage to all enemies. Draw {draw|card|cards}.
- **Radiant:** Echo 1 / This and your next Spell this turn have Poisonous. Deal {damage} damage to all enemies. Draw {draw|card|cards}.
- **Engine:** `give_next_spell_keyword { keyword: Poisonous }` ([[R1848002]]): a `keywordNextSpell` modifier on its caster, expiry this turn, which the next Spell they play face-up takes at [[§10.5]] step 4 for that resolution alone. The Cloud itself has Poisonous as its hits land: the keyword on its face, as [[R1848002]] writes it, or `grant_keyword` on the resolving card while the face prints none. Then `damage_all` of 1 on every enemy, the hero and each Unit, before one state check ([[R59]]): Poisonous destroys each Unit it deals 1 or more to, a Divine Shield stopping it. Then `draw`. Radiant: Echo 1 ([[§6.2]]): the whole resolves twice, the second modifier spent by the same next Spell, which has the keyword once.
- **Rulings:** ST-S14-3: ST8's text (the 1 damage is Poisonous, the designer's request); "all enemies" is the enemy hero and every enemy Unit; the modifier waits through a Unit, a Field Spell or a Trap played meanwhile, and ends with the turn ([[R1848002]]).
- **Numbers:** damage 1 ↑; draw 1 ↑; the Radiant face's Echo 1 is a numbered keyword's ([[R482]]).
- **Check:** ST8's fix (Poisonous on this Spell's own hit). R275: Echo 1, the whole twice (2×). ⚠ The face prints no Poisonous keyword, while [[R1848002]] puts the Cloud's on its face: S14 settles which (a correction is proposed with the foundation).
- **Class:** C (M14's Poisonous for a Spell).

#### Stinkyscape #14 · Arm of the Jempire
`stinkyscape-014` · (2) Unit, Human · Common · 4/5 → 8/10 · built by S16 (#1869)

> **Designer:** (2) Arm of the Jempire, Unit, Human, Common, Stinkyscape, #14 · 4/5 / When one of your Taunt Units is attacked, give it +2 attack. ~~~ 8/10 / When one of your Taunt Units is attacked, give it +4 attack.

- **Text:** When one of your Taunt Units is attacked, give it +{buff} Attack.
- **Radiant:** When one of your Taunt Units is attacked, give it +{buff} Attack.
- **Engine:** `ally_attacked` ([[R1849008]]): once [[§4.2]] step 4's trap window closes on a declared attack on a Unit of its controller's that has Taunt (read through the layers, [[§10.4]]), a plain +2 Attack buff on that Unit (`buff`, kept until it leaves the field, [[R78]]), before the combat, which reads it in the strike back ([[R94]]). Never for an attack on the hero or a forced attack ([[R121]]); this Unit answers for itself when it has Taunt.
- **Rulings:** ST-S16-1: "give it +2 attack" is a buff that stays, not one until the end of the turn; two attacks on one Taunt Unit stack it. These are proposals for S16, which takes R-numbers from its own block. No R-number here.
- **Numbers:** buff 2 ↑ (Radiant 4).
- **Check:** R275: stats doubled and the buff doubled (+2 → +4).
- **Class:** C (M15's `ally_attacked`).

#### Stinkyscape #15 · Arg Arg Defender
`stinkyscape-015` · (1) Unit, Human · Common · 2/3 → 4/6 · built by S16 (#1869)

> **Designer:** (1) Arg Arg Defender, Unit, Human, Common, Stinkyscape, #15 · 2 /3 / Cry: Give adjacent Units +1/+1 and Taunt. ~~~ 4/6 / Cry: Give adjacent Units +2/+2, Divine Shield, and Taunt.

- **Text:** Cry: Give adjacent Units +{buff}/+{buff} and Taunt.
- **Radiant:** Cry: Give adjacent Units +{buff}/+{buff}, Divine Shield and Taunt.
- **Engine:** Cry ([[R1]]): each Unit in the unit zones beside it on its own side (its lane's neighbours, [[§3.1]]) gets a +1/+1 buff and Taunt (`grant_keyword`); an empty neighbour, nothing. Radiant: +2/+2, Divine Shield and Taunt. The buff is its Cry's number ([[R1748000]]).
- **Rulings:** ST-S16-2: "adjacent Units" are the Units beside it on its side, never the enemy's across the lane. These are proposals for S16, which takes R-numbers from its own block. No R-number here.
- **Numbers:** buff 1 ↑ (Radiant 2), its Cry's (`cry: true`).
- **Check:** Typo "2 /3" read as 2/3. R275: stats doubled, the buff doubled and Divine Shield added.
- **Class:** B (a buff and a granted keyword).

#### Stinkyscape #16 · Rod of Ages
`stinkyscape-016` · (1) Field Spell, Quickdraw · Mythic · 1/1 → 2/2 · built by S16 (#1869)

> **Designer:** (1) Rod of Ages, Field Spell, Quickdraw, Mythic, Stinkyscape, #16 · Animated, Flying / The first unit played on this is Equipt / End of turn: Gain a random Combat Keyword / Upon reaching 10 Keywords make this card Radiant ~~~ Animated, Flying, Indestructable / The first unit played on this is Equipt / End of turn: Gain two random Combat Keyword

- **Text:** Animated, Flying / The first Unit you play onto this is Equipped into it. / End of turn: Gain {keywords|random combat keyword|random combat keywords}. / Upon reaching 10 keywords, make this Radiant.
- **Radiant:** Animated, Flying, Indestructible / The first Unit you play onto this is Equipped into it. / End of turn: Gain {keywords|random combat keyword|random combat keywords}.
- **Engine:** Animated ([[§6.1]]): it enters the field as a 1/1 Flying Unit, its own lane's unit zone or [[R64]]'s leftmost. NEW: while it has hosted none, the first Unit its controller plays may be played onto it as an Equip play ([[R1086]]), this card the kept host and the Unit fused into it ([[R77]], [[R102]]), a Starship Piece fused in leaving it no piece ([[R1852001]]). End of turn: `grant_random_keywords` from [[R21]]'s pool ([[R781005]]). Base: `Script.radiant_at_keywords` 10 ([[R1850005]]): at a state check where its keyword kinds through the layers reach 10, each kind once, it is made Radiant, once. Radiant: Indestructible and two keywords a turn. Quickdraw ([[R640]]).
- **Rulings:** ST-S16-3: "the first unit played on this is Equipt" lends the Equip play onto this card to its controller's first Unit played after it arrives; once a Unit has fused into it, later Units play as usual. ST-S16-4: Animated and Flying count toward the 10. These are proposals for S16, which takes R-numbers from its own block. No R-number here.
- **Numbers:** keywords 1 ↑ (Radiant 2). The 10 is `radiant_at_keywords`, printed data (`FIXED` STRUCTURE, R1748003).
- **Check:** D6: "Equipt" is Equip, "Indestructable" Indestructible. ⚠ designer: the list prints no stats, but an Animated Field Spell must (B3.1): 1/1 → 2/2 (R1857001, open question 1). R275: Indestructible and twice the keywords a turn.
- **Class:** C (M16's Radiant at N keywords), with a NEW Equip permission.

#### Stinkyscape #17 · Imprisoned Ship Smith
`stinkyscape-017` · (1) Unit, Human · Common · 2/2 → 4/4 · built by S01 (#1858)

> **Designer:** (1) Imprisoned Ship Smith, Unit, Human, Common, Stinkyscape, #17 · [2/2] / Dormant for 2 turns. When this awakens summon a Fiery Waraxe ~~~ [4/4] / Dormant for 2 turns. When this awakens summon a Radiant Fiery Waraxe / *{voice line is: They can not stop my work}*

- **Text:** Dormant for 2 turns. When this awakens, summon a Fiery Waraxe.
- **Radiant:** Dormant for 2 turns. When this awakens, summon a Radiant Fiery Waraxe.
- **Engine:** Dormant ([[R1839000]]): `StaticFlags.slumber { turns: 2 }`, counted at the start of its controller's turns and awake at the start of the second, before the start-of-turn triggers, not summoning sick ([[R1839001]]). `when_this_awakens` ([[R1839002]]): `summon { defId: meditative-066 }`, M #66 Fiery Waraxe, into the leftmost open backrow zone ([[R64]]), no Cry ([[R1]]); a full backrow summons nothing. Radiant: a Radiant one.
- **Rulings:** ST-S01-1: the Waraxe summoned at the awakening animates that same turn, the awakening coming before the "Animated on your turn" step (R1839001). These are proposals for S01, which takes R-numbers from its own block. No R-number here.
- **Numbers:** none (the 2 turns are the Dormant status's printed count, `slumber.turns`, no param).
- **Check:** Designer voice line "They can not stop my work" kept as its play line. R275: stats doubled and a Radiant Waraxe (6/4 Pierce).
- **Class:** C (Dormant, M01).

#### Stinkyscape #18 · Imitated Instrumentality
`stinkyscape-018` · (3) Spell · Legendary · built by S11 (#1866)

> **Designer:** (3) Imitated Instrumentality, Spell, Legendary, Stinkyscape, #18 · Summon the 3 Brains. They offer great strategic value but they can vote to remove you. {For AI: if all 3 Brains vote to remove you from the game, an animation plays where they all agree to remove you and you disconnect from the game after. This is treated as a loss} ~~~ Summon the 3 Radiant Brains. They offer great strategic value but they can vote to remove you.

- **Text:** Summon MELCHIOR, BALTHASAR and CASPER. They offer great strategic value, but they can vote to remove you.
- **Radiant:** Summon a Radiant MELCHIOR, a Radiant BALTHASAR and a Radiant CASPER. They offer great strategic value, but they can vote to remove you.
- **Engine:** Three summons in order, S #18.1, #18.2 and #18.3, into its caster's leftmost open backrow zones ([[R64]]), one with no zone not summoned; no Cry ([[R1]]). Radiant: each Radiant. The ending is the Brains' ([[R1843000]]): each votes at its controller's start of turn on its own condition, and `VOTE_TALLY_SIZE` (3) votes in one turn remove that player, a loss for them and a win for the opponent, read after the hero check ([[R1843001]]).
- **Rulings:** ST-S11-1: "the 3 Brains" are named, so each is a ref (R279). ST-S11-2: voted out is a loss for you and a win for your opponent (ST9 Q9); the client plays the three agreeing on `votedOut` before `gameOver`. These are proposals for S11, which takes R-numbers from its own block. No R-number here.
- **Numbers:** none (three named summons).
- **Check:** ST9 Q9 (the loss). The designer's "For AI" note (the animation, the disconnect) is R1843000's `votedOut`. R275: the three Brains Radiant.
- **Class:** C (the endings, M09).

#### Stinkyscape #18.1 · MELCHIOR
`stinkyscape-018-1` · (1) Field Spell, Token · Token (printed Legendary) · built by S11 (#1866)

> **Designer:** (1) MELCHIOR, Field Spell, Token, Legendary, Stinkyscape, #18.1 · Indestructible / Start of Turn: Votes to remove you if the enemy has more Units than you. / Activate: Discover a Combat Keyword. Give it to all your Units. ~~~ Indestructible / Start of Turn: Votes to remove you if the enemy has more Units than you. / Activate: Give your Units 3 random Combat Keywords.

- **Text:** Indestructible / Start of turn: Vote to remove you if your opponent has more Units than you. / Activate: Discover a combat keyword. Give it to each of your Units.
- **Radiant:** Indestructible / Start of turn: Vote to remove you if your opponent has more Units than you. / Activate: Give each of your Units {keywords|random combat keyword|random combat keywords}.
- **Engine:** Indestructible ([[§6.1]]). Start of turn ([[R62]]): `vote_out` when its controller's opponent has more Units than they do (`active_units_of`, a Dormant card no Unit, [[R1839000]]) ([[R1843000]]). Activate ([[R384]]): NEW: a Discover of three different combat keywords drawn from [[R21]]'s pool ([[R781005]]), offered as a `mode` prompt, the pick granted to each of its controller's Units (`grant_keyword`). Radiant: `grant_random_keywords` on each Unit, each drawing its own ([[R21]]).
- **Rulings:** ST-S11-3: "more Units than you" counts the Units on the field (`active_units_of`), at its controller's start of turn. ST-S11-4: the Radiant face's 3 keywords are each Unit's own draws. These are proposals for S11, which takes R-numbers from its own block. No R-number here.
- **Numbers:** keywords 3 ↑, on the Radiant face only (`tunedOn: radiant`, R824000).
- **Check:** House style: "Start of turn:". R275: one chosen keyword → three random ones on every Unit.
- **Class:** C (the endings, M09).

#### Stinkyscape #18.2 · BALTHASAR
`stinkyscape-018-2` · (2) Field Spell, Token · Token (printed Legendary) · built by S11 (#1866)

> **Designer:** (2) BALTHASAR, Field Spell, Token, Legendary, Stinkyscape, #18.2 · Indestructible / Start of Turn: Votes to remove you if you have less than half your starting life. / Activate: Add a copy of the last card you played to your hand. It costs (1) more. ~~~ Indestructible / Start of Turn: Votes to remove you if you have less than half your starting life. / Activate: Add a Radiant copy of the last card you played to your hand. It costs (1) more.

- **Text:** Indestructible / Start of turn: Vote to remove you if your hero has less than half its starting health. / Activate: Add a copy of the last card you played to your hand. It costs ({surcharge}) more.
- **Radiant:** Indestructible / Start of turn: Vote to remove you if your hero has less than half its starting health. / Activate: Add a Radiant copy of the last card you played to your hand. It costs ({surcharge}) more.
- **Engine:** Indestructible. Start of turn ([[R62]]): `vote_out` when its controller's hero has less health than half its starting health ([[R1843000]]). Activate ([[R384]]): a copy ([[R57]]) of the last card its controller played this game added to their hand ([[§2.4]]'s cap), with a +1 cost modifier (`set_cost_mod`, not capped); nothing when they have played none. Radiant: the copy Radiant.
- **Rulings:** ST-S11-5: "half your starting life" is half the hero's starting health, the seat's own (a handicap's included, R180); 30 votes at 14 or less. ST-S11-6: "the last card you played" is the last face-up play of the game, a cast included (R70). These are proposals for S11, which takes R-numbers from its own block. No R-number here.
- **Numbers:** surcharge 1 ↓ (Radiant 1).
- **Check:** House style: "Start of turn:", "health" for life. R275: the copy Radiant.
- **Class:** C (the endings, M09).

#### Stinkyscape #18.3 · CASPER
`stinkyscape-018-3` · (3) Field Spell, Token · Token (printed Legendary) · built by S11 (#1866)

> **Designer:** (3) CASPER, Field Spell, Token, Legendary, Stinkyscape, #18.3 · Indestructible / Start of Turn: Votes to remove you if you Floated last turn. / Activate: Allure a random enemy Unit. ~~~ Indestructible / Start of Turn: Votes to remove you if you Floated last turn. / Activate: Allure a random enemy Unit on the field and hand.

- **Text:** Indestructible / Start of turn: Vote to remove you if you Floated on your last turn. / Activate: Allure a random enemy Unit.
- **Radiant:** Indestructible / Start of turn: Vote to remove you if you Floated on your last turn. / Activate: Allure a random enemy Unit and a random Unit in your opponent's hand.
- **Engine:** Indestructible. Start of turn ([[R62]]): `vote_out` when its controller Floated on their last turn ([[§6.2]] Float) ([[R1843000]]). Activate ([[R384]]): an Allure of one random enemy Unit on the field ([[R963]], [[R1045000]]): marked, stolen at the start of its controller's next turn, destroyed with no room. Radiant: NEW: also one random Unit card of the opponent's hand, marked by a hand watch ([[R1140]], [[R1141]]) and taken into its controller's hand at the start of their next turn ([[R12]]), not taken if it has left that hand.
- **Rulings:** ST-S11-7: "Floated last turn" is a Float on its controller's previous turn. ST-S11-8: "a random enemy Unit on the field and hand" is two Allures, one on the field and one in the hand. These are proposals for S11, which takes R-numbers from its own block. No R-number here.
- **Numbers:** none (an Allure and a vote).
- **Check:** R275: a second Allure, of a hand Unit. ⚠ designer: Allure has no hand form on `main`; the Radiant one is NEW (S11).
- **Class:** C (the endings, M09), with a NEW hand Allure.

#### Stinkyscape #19 · Imprisoned Cosmality
`stinkyscape-019` · (2) Unit · Common · 10/6 → 20/12 · built by S01 (#1858)

> **Designer:** (2) Imprisoned Cosmality, Unit, Common, Stinkyscape, #19 · 10/6 / Pierce / Dormant for 2 turns. When this awakens, deal 10 damage split among enemies. ~~~ 20/12 / Pierce / Dormant for 2 turns. When this awakens, deal 20 damage split among enemies.

- **Text:** Pierce / Dormant for 2 turns. When this awakens, deal {damage} damage randomly split among all enemies.
- **Radiant:** Pierce / Dormant for 2 turns. When this awakens, deal {damage} damage randomly split among all enemies.
- **Engine:** Pierce ([[R346]]). Dormant ([[R1839000]]): `slumber { turns: 2 }`, awake at the start of its controller's second turn after it arrived ([[R1839001]]). `when_this_awakens` ([[R1839002]]): C+ #3's `damage_split`, the damage in hits of 1, each on an enemy drawn from the match rng among the enemy hero and the enemy Units still standing ([[R60]]), with no state check between the hits ([[R59]]); each hit's source is this Unit, so each pierces Armor ([[R346]]).
- **Rulings:** ST-S01-2: "split among enemies" is the enemy hero and every enemy Unit, 1 damage a hit; a Unit at 0 is not hit again. These are proposals for S01, which takes R-numbers from its own block. No R-number here.
- **Numbers:** damage 10 ↑ (Radiant 20). The 2 turns are the status's printed count (no param).
- **Check:** R275: stats doubled and the damage doubled (10 → 20).
- **Class:** C (Dormant, M01).

#### Stinkyscape #20 · Tnack Trav
`stinkyscape-020` · (2) Unit · Epic · 6/6 → 12/12 · built by S17 (#1876)

> **Designer:** (2) Tnack Trav, Unit, Epic, Stinkyscape, #20 · 6/6 / Cry: Copy the text of a Legendary or Mythic Unit on the Field. ~~~ 12/12 / Cry: Copy the Radiant text of a Legendary or Mythic Unit on the Field.

- **Text:** Cry: Copy the text of a Legendary or Mythic Unit on the field.
- **Radiant:** Cry: Copy the Radiant text of a Legendary or Mythic Unit on the field.
- **Engine:** Cry ([[R1]]): a declared target ([[R81]]), a Legendary or Mythic Unit on the field, either side, not itself; `copy_unit_text { target }` ([[R1850000]]): its keywords, aura, triggers, hooks, Death, Activates and numbers on this body until it leaves the field ([[R78]]), its own name, cost, type, tags, rarity and stats kept; the copied Cry does not run. Radiant: `radiant: true`, the target's Radiant text. No target, nothing.
- **Rulings:** ST-S17-1: "Legendary or Mythic" is the target's rarity; a token's rarity is Token, whatever it printed. ST-S17-2: an Immutable target may be copied; an Immutable Tnack Trav copies nothing (R1850000). These are proposals for S17, which takes R-numbers from its own block. No R-number here.
- **Numbers:** none.
- **Check:** ST9: Tnack Trav (#20) and Tmack Trav (#94) are two cards. "on the Field" is the field. R275: stats doubled and the Radiant text copied.
- **Class:** C (M16's copy a Unit's text).

#### Stinkyscape #21 · Moonberry
`stinkyscape-021` · (1) Spell, Fruit · Common · built by S13 (#1867)

> **Designer:** (1) Moonberry, Spell, Fruit, Common, Stinkyscape, #21 · Flashback / Add a random Other-Origins card to your hand. ~~~ Flashback / Add a random Radiant Other-Origins card to your hand. It costs (1) less.

- **Text:** Flashback / Add a random Other-Origins card to your hand.
- **Radiant:** Flashback / Add a random Radiant Other-Origins card to your hand. It costs ({discount}) less.
- **Engine:** Flashback ([[§6.1]], [[R783012]]): it may be played once more from its owner's graveyard at its cost, losing the keyword as it is. One pick from the match rng ([[R60]]) among the Other-Origins tokens of the sets that ship (`add_random_from_catalog { tags: [Other-Origins] }` with tokens let in: C+ #32.1–#32.3 now, and S #10.1–#12.3 once the set ships, 12 in all), added to the hand, a full hand burning it ([[§2.4]]). NEW (S13): the pool takes the tag's tokens, which [[R1837000]] leaves to the first card that names it, since a tag pool holds non-token cards and Other-Origins is in no `POOL_TOKEN_TAGS` ([[R382]]). Radiant: the card Radiant, with `costMod` −1, never below (0) ([[R65]]).
- **Rulings:** ST-S13-9: "a random Other-Origins card" is any of the tag's tokens, its only cards ([[R1837000]]), the shipped sets' alone ([[R1420]]); Moonberry is a Fruit, in no Other-Origins pool.
- **Numbers:** discount 1 ↑ (the Radiant face alone, `tunedOn: radiant`).
- **Check:** The first card to name an Other-Origins pool ([[R1837000]]). R275: the card Radiant and (1) cheaper.
- **Class:** B (and the Other-Origins pool's tokens, NEW).

#### Stinkyscape #22 · Gatinsir
`stinkyscape-022` · (4) Trap · Epic · built by S15 (#1875)

> **Designer:** (4) Gatinsir, Trap, Epic, Stinkyscape, #22 · 30% chance to Reveal when an opponent attacks / On reveal: AMBUSH summon two Warp Disruptor then 2 Phalanx ~~~ On reveal: AMBUSH summon two Radiant Warp Disruptor then 2 Radiant Phalanx

- **Text:** {chance}% chance to reveal when an enemy Unit attacks: Summon {disruptors|Warp Disruptor|Warp Disruptors}, then {phalanxes|Phalanx|Phalanxes}. The attack goes on.
- **Radiant:** {chance}% chance to reveal when an enemy Unit attacks: Summon {disruptors|Radiant Warp Disruptor|Radiant Warp Disruptors}, then {phalanxes|Radiant Phalanx|Radiant Phalanxes}. The attack goes on.
- **Engine:** A Trap with a chance to reveal ([[§6.2]], [[R1849009]]): set, each enemy attack declared rolls the match rng once against its declared `chance` (its controller's Luck rolling again, [[R987]]), a miss leaving it set and silent. A hit fires it in [[§4.2]] step 4's trap window: `summon` two Warp Disruptors (S #23, Field Spells) into its controller's open backrow zones, then two Phalanxes (S #82) into open unit zones ([[R64]]), no Cry ([[R1]]), one that finds no zone failing; then the attack goes on as declared (ST9's AMBUSH). The Warp Disruptors' aura acts at once: no Unit in Defense Position, +2 Attack. Radiant: all four Radiant.
- **Rulings:** ST-S15-3: "when an opponent attacks" is an enemy Unit's declared attack; AMBUSH is ST9's label (the summons arrive before the combat, the attack going on); the Radiant face, which the designer printed without its reveal line, keeps the trigger and the 30% chance.
- **Numbers:** chance 30 ↑ (step 10, max 100); disruptors 2 ↑; phalanxes 2 ↑.
- **Check:** ST9's AMBUSH; [[R1849009]]'s roll. R275: all four summons Radiant (Phalanx 8/8; the Disruptors' Radiant First Strike).
- **Class:** C (M15's chance to reveal).

#### Stinkyscape #23 · Warp Disruptor
`stinkyscape-023` · (1) Field Spell · Rare · built by S15 (#1875)

> **Designer:** (1) Warp Disruptor, Field Spell, Rare, Stinkyscape, #23 · Stack / Brittle 3 / Aura: Units cannot be in defense position, Units have +2 attack, when the enemy summons a unit a random unit from your side attacks it ~~~ Stack / Brittle 3 / Aura: Units cannot be in defense position, Units have +2 attack, your units have First strike, when the enemy summons a unit a random unit from your side attacks it

- **Text:** Stack, Brittle 3 / Aura: Units can't be in Defense Position and have +{attack} Attack. / After your opponent summons a Unit, a random Unit of yours attacks it.
- **Radiant:** Stack, Brittle 3 / Aura: Units can't be in Defense Position and have +{attack} Attack. Your Units have First Strike. / After your opponent summons a Unit, a random Unit of yours attacks it.
- **Engine:** Stack ([[§6.1]]: played onto an occupied backrow zone, the card beneath it out of play while it stands) and Brittle 3 ([[R638]]). While it acts face-up: `staticFlags.unitsNeverDefense` ([[R1849006]]), every Unit of both sides kept out of Defense Position, one already in it switched to Attack at the state check, spending nothing; and +2 Attack to every Unit of both sides (an aura, [[§10.4]]). After a Unit arrives on the opponent's side, however summoned (played, after it resolves; cast; a token; Recruit; Reborn: C #5 Tesla's reading), `forced_attacks { attackers: { side: self, random: true } }` ([[R1849007]], [[R53]]): one of its controller's Units, drawn from the match rng among those that may attack it (none able, no draw), makes a forced attack on it, struck back as any is. Radiant: First Strike for its controller's Units.
- **Rulings:** ST-S15-4: "Units" are every Unit of both sides and the attack is its controller's side's (ST9's default, [[R1849006]]); "when the enemy summons a unit" is C #5 Tesla's arrival, however summoned, a played Unit answered after it resolves.
- **Numbers:** attack 2 ↑; Brittle's 3 is a numbered keyword ([[R482]]).
- **Check:** ST9's defaults. R275: First Strike for your Units, a rider.
- **Class:** C (M15's aura and forced attacks).

#### Stinkyscape #24 · Warp Disruptor II
`stinkyscape-024` · (2) Field Spell · Rare · built by S15 (#1875)

> **Designer:** (2) Warp Disruptor II, Field Spell, Rare, Stinkyscape, #24 · Stack / Brittle 5 / Aura: Units cannot be in defense position, Units have +3 attack, when the enemy summons a unit all unit from your side attacks it ~~~ Stack / Brittle 5 / Aura: Units cannot be in defense position, Units have +3 attack, your units have First strike, when the enemy summons a unit all unit from your side attacks it

- **Text:** Stack, Brittle 5 / Aura: Units can't be in Defense Position and have +{attack} Attack. / After your opponent summons a Unit, each of your Units attacks it.
- **Radiant:** Stack, Brittle 5 / Aura: Units can't be in Defense Position and have +{attack} Attack. Your Units have First Strike. / After your opponent summons a Unit, each of your Units attacks it.
- **Engine:** S #23's, with +3 Attack, Brittle 5 ([[R638]]), and `forced_attacks` naming every Unit of its controller's that may attack the arrival, in lane order, each its own combat and state check, the run stopping once the target has left the field ([[R53]]; [[R1849007]] without `random`). Radiant: First Strike for its controller's Units.
- **Rulings:** S #23's (ST-S15-4); "all unit from your side" is each of your Units, in lane order.
- **Numbers:** attack 3 ↑; Brittle's 5 is a numbered keyword ([[R482]]).
- **Check:** ST9's defaults; the name's "II" prints no digit. R275: First Strike for your Units, a rider.
- **Class:** C (M15's aura and forced attacks).

#### Stinkyscape #25 · KY's Tricknology
`stinkyscape-025` · (2) Spell, KY · Epic · built by S18 (#1877)

> **Designer:** (2) KY's Tricknology, Spell, KY, Epic, Sinkyscape, #25 · Choose a zone. Its Jacobian stretches a unit square. A Starship Piece lands on each zone it covers. *F(x, y) \= (x² − y, xy)* ~~~ Choose a zone. Its Jacobian stretches a unit square. A Radiant Starship Piece lands on each of your zones it covers. *F(x, y) \= (x² − y, xy)*

- **Text:** Choose a zone. Its Jacobian stretches a unit square. Summon a random Starship Piece in each zone it covers, for the player whose zone it is. F(x, y) = (x² − y, xy).
- **Radiant:** Choose a zone. Its Jacobian stretches a unit square. Summon a random Radiant Starship Piece in each of your zones it covers. F(x, y) = (x² − y, xy).
- **Engine:** The build's coordinates (ST9 Q14): a zone is (x, y), x its lane 1 to 5, y 1 your unit row, 2 your backrow, −1 the enemy's unit row, −2 their backrow. A zone chosen (`choose_cell`, B5 E18) maps the unit square through the Jacobian J, rows (2x, −1) and (y, x), with its corner at the zone; NEW: each zone whose centre lies in that parallelogram, open and unlocked, gets a random non-token Starship Piece Unit summoned (a pool by keyword, of the sets that ship, [[R380]]) for the side it is on ([[R64]]'s placement, no Cry, [[R1]]), a backrow zone taking it as a Unit standing there ([[R1081000]]). Radiant: only the caster's zones, each piece Radiant.
- **Rulings:** ST-S18-1: an occupied or Locked zone gets none; the chosen zone itself gets one when it lies in the parallelogram. ST-S18-2: a covered backrow zone takes the piece as a Unit standing in the backrow; S18 may keep pieces to unit zones instead. These are proposals for S18, which takes R-numbers from its own block. No R-number here.
- **Numbers:** none (the formula is the card's text, not a number; it prints no ASCII digit).
- **Check:** D6: "Sinkyscape" is Stinkyscape. ST9 Q14 (the coordinates are the build's). R275: Radiant pieces, and only yours.
- **Class:** D (the Jacobian cover).

#### Stinkyscape #26 · ZolarKeth
`stinkyscape-026` · (4) Unit, Human, Acclaimed · Legendary · 2/9 → 4/18 · built by S17 (#1876)

> **Designer:** (4) ZolarKeth, Unit, Human, Acclaimed, Legendary, Sinkyscape, #26 · 2/9 / Cry: Summon a Radiant University. Its Activate is locked. / Activate: Summon a random Starship Piece or Activate your Universities. ~~~ 4/18 / Cry: Summon a Radiant University. Its Activate is locked. / Activate: Summon a random Radiant Starship Piece then Activate your Universities.

- **Text:** Cry: Summon a Radiant University. It can't use its own Activate. / Activate: Choose one: summon a random Starship Piece, or Activate your Universities.
- **Radiant:** Cry: Summon a Radiant University. It can't use its own Activate. / Activate: Summon a random Radiant Starship Piece, then Activate your Universities.
- **Engine:** Cry ([[R1]]): `summon { defId: meditative-097-6, radiant }`, a Radiant M #97.6 University ([[R64]]), whose own Activate is refused (a mark on it, NEW). Activate ([[R384]]): base, two modes ([[R81]]): a random non-token Starship Piece Unit summoned (a pool by keyword, [[R380]]), or each University of its controller's runs its Activate's effect (Buff each of its other permanents N times) as an ability of this card, spending none of its uses (NEW); Radiant: both, the piece Radiant.
- **Rulings:** ST-S17-3: "Its Activate is locked" refuses the summoned University's own use of its Activate (no zone is Locked); this card's Activate still runs it. ST-S17-4: "Activate your Universities" runs every University of yours, the locked one included, and spends none of their uses. These are proposals for S17, which takes R-numbers from its own block. No R-number here.
- **Numbers:** none (a summon and two modes).
- **Check:** D6: "Sinkyscape" is Stinkyscape. The base face's "or" is a choice of modes, the Radiant face's "then" both. R275: stats doubled, both modes each turn and a Radiant piece.
- **Class:** B (a summon and modes), with a NEW Activate of another card.

#### Stinkyscape #27 · Drone Core I
`stinkyscape-027` · (0) Spell, Drone Core · Common · built by S07 (#1863)

> **Designer:** (0) Drone Core I, Spell, Drone Core, Common, Stinkscape #27 · Discard a card. Summon a Drone Interceptor ~~~ Discard a card. Summon a Radiant Drone Interceptor

- **Text:** Discard a random card. Summon a Drone Interceptor.
- **Radiant:** Discard a random card. Summon a Radiant Drone Interceptor.
- **Engine:** A random discard of its caster's hand ([[R682]]), nothing with an empty hand, then `summon` of the Drone Interceptor (S #T-drone-interceptor) into the caster's leftmost open unit zone ([[R64]]); with no open zone nothing is summoned, the discard still made. Radiant: the Interceptor is summoned Radiant. Drone Core is a family tag no rule reads (#1857).
- **Rulings:** ST-S07-1: "Discard a card" is a random card ([[R682]]); with an empty hand nothing is discarded and the Interceptor still comes.
- **Numbers:** none (the discard and the drone are one each, written "a").
- **Check:** "Stinkscape" is Stinkyscape (ST8). "Discard a card" is house style's random discard. R275: a Radiant Interceptor, twice the stats and the keywords.
- **Class:** B.

#### Stinkyscape #28 · Drone Core II
`stinkyscape-028` · (1) Spell, Drone Core · Common · built by S07 (#1863)

> **Designer:** (1) Drone Core II, Spell, Drone Core, Common, Stinkscape #28 · Summon a Drone Interceptor ~~~ Summon a Radiant Drone Interceptor

- **Text:** Summon a Drone Interceptor.
- **Radiant:** Summon a Radiant Drone Interceptor.
- **Engine:** `summon` of a Drone Interceptor into its caster's leftmost open unit zone ([[R64]]), no Cry ([[R1]]); no open zone, nothing. Radiant: summoned Radiant.
- **Rulings:** none new: every reading is a ruling on `main` or the card's text.
- **Numbers:** none (one drone, written "a").
- **Check:** "Stinkscape" is Stinkyscape (ST8). R275: a Radiant Interceptor.
- **Class:** A.

#### Stinkyscape #29 · Drone Core III
`stinkyscape-029` · (2) Spell, Drone Core · Common · built by S07 (#1863)

> **Designer:** (2) Drone Core III, Spell, Drone Core, Common, Stinkscape #29 · Summon 2 Drone Interceptor ~~~ Summon 2 Radiant Drone Interceptor

- **Text:** Summon {interceptors|Drone Interceptor|Drone Interceptors}.
- **Radiant:** Summon {interceptors|Radiant Drone Interceptor|Radiant Drone Interceptors}.
- **Engine:** `summon` with a count: each Drone Interceptor into the leftmost open unit zone in turn ([[R64]]), as many as fit. Radiant: summoned Radiant.
- **Rulings:** none new: every reading is a ruling on `main` or the card's text.
- **Numbers:** interceptors 2 ↑.
- **Check:** "Stinkscape" is Stinkyscape (ST8). R275: Radiant Interceptors.
- **Class:** A.

#### Stinkyscape #30 · Drone Core IV
`stinkyscape-030` · (3) Spell, Drone Core · Rare · built by S07 (#1863)

> **Designer:** (3) Drone Core IV, Spell, Drone Core, Rare, Stinkscape #30 · Summon 3 Drone Interceptor ~~~ Summon 3 Radiant Drone Interceptor

- **Text:** Summon {interceptors|Drone Interceptor|Drone Interceptors}.
- **Radiant:** Summon {interceptors|Radiant Drone Interceptor|Radiant Drone Interceptors}.
- **Engine:** As S #29, three Interceptors ([[R64]]), as many as fit. Radiant: summoned Radiant.
- **Rulings:** none new: every reading is a ruling on `main` or the card's text.
- **Numbers:** interceptors 3 ↑.
- **Check:** "Stinkscape" is Stinkyscape (ST8). R275: Radiant Interceptors.
- **Class:** A.

#### Stinkyscape #31 · Drone Core V
`stinkyscape-031` · (3) Spell, Drone Core · Rare · built by S07 (#1863)

> **Designer:** (3) Drone Core V, Spell, Drone Core, Rare, Stinkscape #31 · Summon a Drone Corvette ~~~ Summon a Radiant Drone Corvette

- **Text:** Summon a Drone Corvette.
- **Radiant:** Summon a Radiant Drone Corvette.
- **Engine:** `summon` of a Drone Corvette (S #T-drone-corvette) ([[R64]]); no open zone, nothing. Radiant: summoned Radiant.
- **Rulings:** none new: every reading is a ruling on `main` or the card's text.
- **Numbers:** none (one Corvette, written "a").
- **Check:** "Stinkscape" is Stinkyscape (ST8). R275: a Radiant Corvette (16/16, four keywords).
- **Class:** A.

#### Stinkyscape #32 · Drone Core VI
`stinkyscape-032` · (4) Spell, Drone Core · Epic · built by S07 (#1863)

> **Designer:** (4) Drone Core VI, Spell, Drone Core, Epic, Stinkscape #32 · Summon 5 Drone Interceptor ~~~ Summon 5 Radiant Drone Interceptor

- **Text:** Summon {interceptors|Drone Interceptor|Drone Interceptors}.
- **Radiant:** Summon {interceptors|Radiant Drone Interceptor|Radiant Drone Interceptors}.
- **Engine:** As S #29, five Interceptors, a whole unit row when it is empty ([[R64]]). Radiant: summoned Radiant.
- **Rulings:** none new: every reading is a ruling on `main` or the card's text.
- **Numbers:** interceptors 5 ↑.
- **Check:** "Stinkscape" is Stinkyscape (ST8). R275: Radiant Interceptors.
- **Class:** A.

#### Stinkyscape #33 · Drone Core VII
`stinkyscape-033` · (4) Spell, Drone Core · Epic · built by S07 (#1863)

> **Designer:** (4) Drone Core VII, Spell, Drone Core, Epic, Stinkscape #33 · Summon a Drone Corvette & Summon 2 Drone Interceptors ~~~ Summon a Radiant Drone Corvette & Summon 2 Radiant Drone Interceptors

- **Text:** Summon a Drone Corvette and {interceptors|Drone Interceptor|Drone Interceptors}.
- **Radiant:** Summon a Radiant Drone Corvette and {interceptors|Radiant Drone Interceptor|Radiant Drone Interceptors}.
- **Engine:** Two `summon`s, the Corvette first, then the Interceptors, each into the leftmost open unit zone ([[R64]]), as many as fit. Radiant: all Radiant.
- **Rulings:** none new: every reading is a ruling on `main` or the card's text.
- **Numbers:** interceptors 2 ↑.
- **Check:** "Stinkscape" is Stinkyscape (ST8); the list's "&" is "and". R275: Radiant drones.
- **Class:** A.

#### Stinkyscape #34 · Drone Core VIII
`stinkyscape-034` · (6) Spell, Drone Core · Legendary · built by S07 (#1863)

> **Designer:** (6) Drone Core VIII, Spell, Drone Core, Legendary, Stinkscape #34 · Float reduces cost / Fill your board with Drone Interceptor ~~~ Float reduces cost / Fill your board with Radiant Drone Interceptor

- **Text:** Float reduces cost. / Fill your board with Drone Interceptors.
- **Radiant:** Float reduces cost. / Fill your board with Radiant Drone Interceptors.
- **Engine:** Float reduces cost ([[§6.2]], [[R1563000]]): a hand and deck trigger on its owner's Float, a `costMod` of minus the Float, kept until a graveyard or exile ([[R766]]), so at (6), past `MAX_MANA` (4), it is played only so. `fill_board` with the Drone Interceptor: one into each of the caster's empty unit zones ([[R64]]). Radiant: Radiant ones.
- **Rulings:** ST-S07-2: "Fill your board" is every empty unit zone of the caster's, as C+ #62's fill reads, never the backrow, though a Flying Unit may be played there.
- **Numbers:** none (a fill has no count).
- **Check:** "Stinkscape" is Stinkyscape (ST8). Its (6) is the designer's, paid down by Floats (ST2). R275: Radiant Interceptors.
- **Class:** B.

#### Stinkyscape #35 · Drone Core IX
`stinkyscape-035` · (6) Spell, Drone Core · Legendary · built by S07 (#1863)

> **Designer:** (6) Drone Core IX, Spell, Drone Core, Legendary, Stinkscape #35 · Float reduces cost / Summon 3 Drone Corvette ~~~ Float reduces cost / Summon 3 Radiant Drone Corvette

- **Text:** Float reduces cost. / Summon {corvettes|Drone Corvette|Drone Corvettes}.
- **Radiant:** Float reduces cost. / Summon {corvettes|Radiant Drone Corvette|Radiant Drone Corvettes}.
- **Engine:** Float reduces cost as S #34 ([[R1563000]]). `summon` of 3 Drone Corvettes, left to right ([[R64]]), as many as fit. Radiant: Radiant ones.
- **Rulings:** none new: every reading is a ruling on `main` or the card's text.
- **Numbers:** corvettes 3 ↑.
- **Check:** "Stinkscape" is Stinkyscape (ST8). Its (6) is paid down by Floats (ST2). R275: Radiant Corvettes.
- **Class:** B.

#### Stinkyscape #36 · Drone Core X
`stinkyscape-036` · (8) Spell, Drone Core · Mythic · built by S07 (#1863)

> **Designer:** (8) Drone Core X, Spell, Drone Core, Mythic, Stinkscape #36 · Float reduces cost / Summon the Drone Battleship ~~~ Float reduces cost / Summon the Radiant Drone Battleship

- **Text:** Float reduces cost. / Summon the Drone Battleship.
- **Radiant:** Float reduces cost. / Summon a Radiant Drone Battleship.
- **Engine:** Float reduces cost as S #34 ([[R1563000]]). A layered summon, `summon_pile` ([[R1842000]]), of the three layer tokens into one unit zone, bottom first: the Meltdown Layer, then the Propagation Layer, then the Shield Layer on top, so the Shield Layer is fought first ([[§3.2]], [[R13]]); each its own summon, summoning sick, no Cry; with no free zone nothing. "The Drone Battleship" names the three layers by [[R279]]'s parenthesis rule. Radiant: all three Radiant (the designer's note).
- **Rulings:** ST-S07-3: the Drone Battleship is the three layers, summoned as one pile bottom first (M06), and a Radiant one makes every layer Radiant; a zone that cannot take the pile takes none of it.
- **Numbers:** none (one Battleship).
- **Check:** "Stinkscape" is Stinkyscape (ST8). Its (8) is paid down by Floats (ST2). R275: three Radiant layers (twice the stats, Armor 8, Brittle 6, Radiant drones).
- **Class:** C.

#### Stinkyscape #37 · Kwzka
`stinkyscape-037` · (3) Unit, Human, Acclaimed · Legendary · 6/9 → 12/18 · built by S17 (#1876)

> **Designer:** (3) Kwzka, Unit, Human, Acclaimed, Legendary, Stinkyscape, #37 · 6/9 / Aura: Allure all enemy Units played that cost less than this. / Death: Remove all Allures on the field. ~~~ 12/18 / Divine Shield / Aura: Allure all enemy Units played that cost less than this. / Death: Remove all Allures on the field.

- **Text:** Aura: Each enemy Unit played that costs less than this is Allured. / Death: Remove every Allure on the field.
- **Radiant:** Divine Shield / Aura: Each enemy Unit played that costs less than this is Allured. / Death: Remove every Allure on the field.
- **Engine:** While it acts on the field, each Unit its controller's opponent plays whose cost paid ([[R56]]) is less than this card's cost now is Allured as it arrives ([[R963]], [[R1045000]]: the `allure` mark, stolen at the start of its controller's next turn, destroyed with no room). Death ([[§6.2]]): NEW: every Allure mark on the field, both sides', comes off, its steal cancelled ([[R437]]'s mark withdrawn).
- **Rulings:** ST-S17-5: "played" is played, not summoned; the cost compared is the cost paid against this card's cost now. ST-S17-6: "Remove all Allures" takes off its own Allures and the opponent's, so it steals nothing once it dies. These are proposals for S17, which takes R-numbers from its own block. No R-number here.
- **Numbers:** none.
- **Check:** R275: stats doubled and Divine Shield added.
- **Class:** B (an Allure watcher), with a NEW Allure removal.

#### Stinkyscape #38 · Shield Amplifier
`stinkyscape-038` · (1) Unit · Common · 1/1 → 2/2 · built by S03 (#1860)

> **Designer:** (1) Shield Amplifier, Unit, Common, Stinkyscape, #38 · 1/1 / Starship Piece / Equip / Divine Shield / When this survives an attack, give it +3/+3. ~~~ 2/2 / Starship Piece / Equip / Divine Shield / When this survives an attack, give it +6/+6.

- **Text:** Starship Piece, Equip, Divine Shield / Whenever this survives an attack, it gains +{buff}/+{buff}.
- **Radiant:** Starship Piece, Equip, Divine Shield / Whenever this survives an attack, it gains +{buff}/+{buff}.
- **Engine:** Equip ([[R1086]]): played onto one of your Units it fuses into it, the host keeping its zone and taking its stats, keywords and text ([[R77]], [[R102]]), Starship Piece only into a piece ([[R1852001]]); played alone its death fuses a copy into its owner's Dormant Starship ([[R1852000]]); an equipped host gains the text and the Divine Shield. The survival is read by two hooks on main: `afterAttack`, after an attack it made, when its facts say `survived`, and `afterAttacked` ([[R1026]]), after an attack made on it, declared or forced, when it still stands after that combat's state check; a Cleave splash is no attack on it. Then `buff` +N/+N on itself, lasting.
- **Rulings:** ST-S03-1: "survives an attack" is any combat it fights, as the attacker or the defender, declared or forced, after which it is still on the field; a Cleave splash is no attack on it, and a Divine Shield that took the hit counts. These are proposals for the S03 part, which takes R-numbers from its own block.
- **Numbers:** buff 3 ↑ (Radiant 6).
- **Check:** R275: stats doubled and the gain doubled. The designer's "give it" is "it gains".
- **Class:** B

#### Stinkyscape #39 · The Narcor Project
`stinkyscape-039` · (4) Field Spell · Legendary · built by S04 (#1861)

> **Designer:** (4) The Narcor Project, Field Spell, Legendary, Stinkyscape, #39 · Brittle 4 / End of Turn: Add a Narcor to your hand. / Activate: Discard a Narcor to reduce Brittle by 1\. / Death: If this died to Brittle, its End of Turn effect doubles and becomes permanent. ~~~ Brittle 4 / End of Turn: Add a Red Narcor to your hand. / Activate: Discard a Narcor or Red Narcor to reduce Brittle by 1\. / Death: If this died to Brittle, its End of Turn effect doubles and becomes permanent.

- **Text:** Brittle 4 / End of turn: Gain a Narcor. / Activate: Discard a Narcor to lower this card's Brittle by 1. / Death: If Brittle destroyed this, for the rest of the game, gain two Narcors at the end of each of your turns.
- **Radiant:** Brittle 4 / End of turn: Gain a Red Narcor. / Activate: Discard a Narcor or a Red Narcor to lower this card's Brittle by 1. / Death: If Brittle destroyed this, for the rest of the game, gain two Red Narcors at the end of each of your turns.
- **Engine:** Brittle 4 ([[§6.1]], [[R385]], [[R638]]). End of turn ([[R62]]): `gain_resources` of one Narcor (S #T-narcor, a one-id pool, [[R1840002]]) into its controller's tray, never a hand ([[R1840000]]); a ninth is sold at once ([[R1840001]]). Activate ([[R384]]), offered only while the tray holds a Narcor (a `canActivate` read of the tray): `discard_resource` of the oldest Narcor ([[R1840004]]), which ceases to exist, then `gain_brittle` of −1 on this card; a count driven to 0 so crumbles it at the state check, a Brittle death. Death: when its destroy was Brittle's (`crumbled`), a rest-of-game player effect on its controller, C+ #52's `never`-expiry modifier shape ([[§10.1]]): at the end of each of their turns, two Narcors, whatever became of the card. Another destroy, or an exile, gives nothing more. Radiant: Red Narcors (S #T-red-narcor), and the Activate takes the oldest Narcor or Red Narcor.
- **Rulings:** ST-S04-1: "Add a Narcor to your hand" gains it into the tray ([[R1840000]]); "Discard a Narcor" is the oldest one there ([[R1840004]]), the Activate's cost, so it is offered only with a Narcor in the tray. ST-S04-2: lowering Brittle to 0 by the Activate crumbles it at the state check, so its Death reads it as destroyed by Brittle; "died to Brittle" is a `crumbled` destroy, never another destroy. ST-S04-3: "its End of Turn effect doubles and becomes permanent" is a rest-of-game effect of its controller's (C+ #52's shape): two Narcors (Radiant: two Red Narcors) at the end of each of their turns, outliving the card; a second Project that crumbles adds a second such effect.
- **Numbers:** none (Brittle 4 is a numbered keyword's count ([[R482]]); the Activate's 1 is its step and the Death's two the doubling the designer names).
- **Check:** The list's "End of Turn" is the label End of turn ([[R366]]); "Add … to your hand" is a gain into the tray ([[R1840000]]). R275: Red Narcor (Mythic) for Narcor (Legendary), about twice the value.
- **Class:** C.

#### Stinkyscape #40 · Poison Capsule
`stinkyscape-040` · (3) Unit, Plague · Rare · 2/2 → 4/4 · built by S03 (#1860)

> **Designer:** (3) Poison Capsule, Unit, Rare, Stinkyscape, #40 · 2/2 / Starship Piece / Cry: Place 2 Plague Counters. / End of Turn: Deal 1 damage to all enemies per Plague Counter on this. ~~~ 4/4 / Starship Piece / Cry: Place 4 Plague Counters. / End of Turn: Deal 1 damage to all enemies per Plague Counter on this.

- **Text:** Starship Piece / Cry: Place {counters|Plague Counter|Plague Counters}. / End of turn: Deal {damage} damage to all enemies for each Plague Counter on this.
- **Radiant:** Starship Piece / Cry: Place {counters|Plague Counter|Plague Counters}. / End of turn: Deal {damage} damage to all enemies for each Plague Counter on this.
- **Engine:** Starship Piece: its death fuses a copy into its owner's Dormant Starship, summoned first if they have none ([[R1852000]]). Cry ([[R1]]): [[R689]]'s placement, `place_plague`: the counters all on the one permanent a single prompt names, either side, face-down cards included, this one among them; on another permanent they do nothing for it. End of turn ([[R62]]): one hit of damage × the counters on it to the enemy hero and to every enemy Unit (`damage_all`, [[R59]]), none with no counter on it. Tagged Plague by §5's rule for a card that uses Plague Counters ([[R1857001]]).
- **Rulings:** ST-S03-2: "Place 2 Plague Counters" names no card, so [[R689]]'s single prompt places them, this card a choice among the rest. ST-S03-3: "per Plague Counter on this" is one hit of damage times the counters, to each enemy, not that many hits. These are proposals for the S03 part, which takes R-numbers from its own block.
- **Numbers:** counters 2 ↑ (Radiant 4, `cry`); damage 1 ↑.
- **Check:** Tagged Plague (§5, R1857001's data decision). R275: stats and counters doubled.
- **Class:** B

#### Stinkyscape #41 · Drone Sympathizer
`stinkyscape-041` · (1) Spell · Common · built by S07 (#1863)

> **Designer:** (1) Drone Sympathizer, Spell, Common, Stinkscape #41 · Add 2 random Drone Cores to your hand ~~~ Add 2 Radiant random Drone Cores to your hand

- **Text:** Add {cards|random Drone Core card|random Drone Core cards} to your hand.
- **Radiant:** Add {cards|random Radiant Drone Core card|random Radiant Drone Core cards} to your hand.
- **Engine:** `add_random_from_catalog` over the Drone Core tag (S #27–#36, non-token, [[R380]]), two picks, repeats allowed ([[R60]]), each on its base face at its printed cost; a full hand burns the overflow ([[§2.4]]). The pool names no set, so while the set is held it reaches no card ([[R1420]]) and a test opens it with `preview_sets`. Radiant: added Radiant.
- **Rulings:** ST-S07-4: "random Drone Cores" are the cards tagged Drone Core (#27–#36), every tier as likely, repeats allowed ([[R60]]); the pool names no set, so it opens with the release.
- **Numbers:** cards 2 ↑.
- **Check:** "Stinkscape" is Stinkyscape (ST8). R275: Radiant Drone Cores (Radiant drones).
- **Class:** A.

#### Stinkyscape #42 · Core Researcher
`stinkyscape-042` · (2) Unit, Human · Rare · 4/5 → 8/10 · built by S07 (#1863)

> **Designer:** (2) Core Researcher, Unit, Human, Rare, Stinkscape #42 · [4/5] / Cry: add a Radiant Drone Core to your hand ~~~ [8/10] / Cry: add 2 Radiant Drone Core to your hand

- **Text:** Cry: Add {cards|random Radiant Drone Core card|random Radiant Drone Core cards} to your hand.
- **Radiant:** Cry: Add {cards|random Radiant Drone Core card|random Radiant Drone Core cards} to your hand.
- **Engine:** Cry ([[R1]]). Picks from S #41's Drone Core pool ([[R380]], [[R60]]), added Radiant ([[§5.2]]); a full hand burns it ([[§2.4]]). The base face prints the one card as 1.
- **Rulings:** none new: every reading is a ruling on `main` or the card's text.
- **Numbers:** cards 1 ↑ (Radiant 2; Cry only, [[R1748000]]).
- **Check:** "Stinkscape" is Stinkyscape (ST8). R275: stats doubled and 2 cards against 1.
- **Class:** A.

#### Stinkyscape #43 · Healing Capsule
`stinkyscape-043` · (1) Unit · Rare · 2/2 → 4/4 · built by S03 (#1860)

> **Designer:** (1) Healing Capsule, Unit, Rare, Stinkyscape, #43 · 2/2 / Starship Piece / End of Turn: Heal all allies 2\. ~~~ 4/4 / Starship Piece / End of Turn: Heal all allies 4\.

- **Text:** Starship Piece / End of turn: Heal your hero and each of your Units {heal}.
- **Radiant:** Starship Piece / End of turn: Heal your hero and each of your Units {heal}.
- **Engine:** Starship Piece: its death fuses a copy into its owner's Dormant Starship, summoned first if they have none ([[R1852000]]). End of turn ([[R62]]): `heal` on its controller's hero, no cap on a hero ([[§6.3]] Heal), and on each of their Units, itself included, in lane order ([[R68]]), C+ #19.4 Support Loser's line.
- **Rulings:** ST-S03-4: "all allies" is your hero and every Unit of yours, this one included (as Meditative+ #13's reading). These are proposals for the S03 part, which takes R-numbers from its own block.
- **Numbers:** heal 2 ↑ (Radiant 4).
- **Check:** The designer's "Heal all allies 2\." escape dropped. R275: stats and heal doubled.
- **Class:** A

#### Stinkyscape #44 · Great Replacement Theory
`stinkyscape-044` · (1) Spell, Quickdraw · Mythic · built by S07 (#1863)

> **Designer:** (1) Great Replacement Theory, Spell, Quickdraw, Mythic, Stinkscape, #44 · Add a Drone Core I to your hand. The first time you play each Drone Core tier add the subsequent tier Drone Core to your hand. ~~~ Add a Radiant Drone Core I to your hand. The first time you play each Drone Core tier add the Radiant Drone Core to your hand.

- **Text:** Add a Drone Core I to your hand. For the rest of the game, the first time you play each tier of Drone Core, add the next tier to your hand.
- **Radiant:** Add a Radiant Drone Core I to your hand. For the rest of the game, the first time you play each tier of Drone Core, add a Radiant one of the next tier to your hand.
- **Engine:** Quickdraw ([[R640]]). `add_to_hand` of Drone Core I (S #27), then NEW: a rest-of-game player effect on its caster, C+ #52's `never`-expiry modifier shape ([[§10.1]]), watching its owner's plays, a cast included ([[R70]]): the first play of each of Drone Core I to IX after it resolves adds the next tier (S #28–#36) to the hand, once per tier, Drone Core X adding none; a full hand burns it ([[§2.4]]). A Drone Core played before it counts for nothing, and each cast keeps its own record, so two stack. Radiant: every card it adds is Radiant.
- **Rulings:** ST-S07-5: "the first time you play each Drone Core tier" counts from this cast on, each tier once (a cast counts, [[R70]]), and "the subsequent tier" is the next numeral, I to X; X adds nothing. The effect is its caster's for the rest of the game, whatever becomes of the card, and each cast keeps its own.
- **Numbers:** none (the ten tiers are the frame).
- **Check:** "Stinkscape" is Stinkyscape (ST8). Its name is the designer's satire, kept (ST9). R275: every card it adds Radiant.
- **Class:** C.

#### Stinkyscape #45 · Drone Commander
`stinkyscape-045` · (4) Unit, Human · Epic · 8/12 → 16/24 · built by S07 (#1863)

> **Designer:** (4) Drone Commander, Unit, Human, Epic, Stinkscape, #45 · [8/12] / Add 6 random Drone Cores to your hand, they cost (1) less ~~~ [16/24] / Add 6 Radiant random Drone Cores to your hand, they cost (1) less

- **Text:** Cry: Add {cards|random Drone Core card|random Drone Core cards} to your hand. Each costs ({discount}) less.
- **Radiant:** Cry: Add {cards|random Radiant Drone Core card|random Radiant Drone Core cards} to your hand. Each costs ({discount}) less.
- **Engine:** Cry ([[R1]]; the list prints no trigger, ST8). Six picks from S #41's pool ([[R380]], [[R60]]), each with a `costMod` of −1 ([[R65]]), kept until a graveyard or exile ([[R766]]); a full hand burns the overflow ([[§2.4]]). Radiant: added Radiant.
- **Rulings:** none new: every reading is a ruling on `main` or the card's text.
- **Numbers:** cards 6 ↑ (step 2; Cry only, [[R1748000]]); discount 1 ↑ (Cry only, [[R1748000]]).
- **Check:** "Stinkscape" is Stinkyscape (ST8); the trigger is Cry (ST8); "they cost (1) less" is "Each costs (1) less" ([[R482]]). R275: stats doubled and Radiant Drone Cores.
- **Class:** B.

#### Stinkyscape #46 · Circadian Halfcycler
`stinkyscape-046` · (2) Field Spell · Epic · built by S12 (#1874)

> **Designer:** (2) Circadian Halfcycler, Field Spell, Epic, Stinkscape, #46 · Aura: the first 3 actions of your turn are played by an AI / End of turn: Summon a Drone Interceptor ~~~ Aura: the first 3 actions of your turn are played by an AI / End of turn: Summon a Radiant Drone Interceptor

- **Text:** Aura: The AI plays the first 3 actions of each of your turns. / End of turn: Summon a Drone Interceptor.
- **Radiant:** Aura: The AI plays the first 3 actions of each of your turns. / End of turn: Summon a Radiant Drone Interceptor.
- **Engine:** The autopilot as an aura ([[R1844000]]): `pilotsActions: 3`, so each of its controller's turns engages [[R1183002]]'s autopilot as it begins (`autopilotEngaged`), the start's prompts included, and [[R456]]'s first three actions, plays and attacks, are the engine's (an answer belongs to the action that asked), then the turn is the player's for good; ending the turn spends the rest. End of turn: `summon` of the Drone Interceptor token (`stinkyscape-t-drone-interceptor`, [[R64]]); Radiant: a Radiant one.
- **Rulings:** ST-S12-1: the three actions are every one of your turns while it stands (ST9); a card that arrives mid-turn waits for the next (R1844000). These are proposals for S12, which takes R-numbers from its own block. No R-number here.
- **Numbers:** none: the 3 is the aura's printed count, which no Nerf or Buff moves (R1844000, R386; `FIXED` STRUCTURE).
- **Check:** D6: "Stinkscape" is Stinkyscape. ST9 (every turn). R275: a Radiant Drone Interceptor each turn.
- **Class:** C (the AI at the wheel, M10).

#### Stinkyscape #47 · Circadian Fullcycler
`stinkyscape-047` · (2) Field Spell · Epic · built by S12 (#1874)

> **Designer:** (2) Circadian Fullcycler, Field Spell, Epic, Stinkscape, #47 · Brittle 5 / Aura: your turn is played by an AI / End of turn: Summon a Drone Corvette ~~~ Brittle 5 / Aura: your turn is played by an AI / End of turn: Summon a Radiant Drone Corvette

- **Text:** Brittle 5 / Aura: The AI plays each of your turns. / End of turn: Summon a Drone Corvette.
- **Radiant:** Brittle 5 / Aura: The AI plays each of your turns. / End of turn: Summon a Radiant Drone Corvette.
- **Engine:** Brittle 5 ([[§6.1]], a count started as it enters). The autopilot as an aura ([[R1844000]]): `pilotsTurn`, each of its controller's turns played by [[R1183002]]'s autopilot from its start, `AUTOPILOT_MAX_ACTIONS` and the timeout included ([[R1183003]]). End of turn: `summon` of the Drone Corvette token (`stinkyscape-t-drone-corvette`, [[R64]]); Radiant: a Radiant one.
- **Rulings:** ST-S12-2: a whole turn beats any count (#46's included) and the turn is the autopilot's from its start, its first prompts too (R1844000). These are proposals for S12, which takes R-numbers from its own block. No R-number here.
- **Numbers:** none (Brittle 5 is a numbered keyword, `FIXED` KEYWORD).
- **Check:** D6: "Stinkscape" is Stinkyscape. R275: a Radiant Drone Corvette each turn.
- **Class:** C (the AI at the wheel, M10).

#### Stinkyscape #48 · Ultralight Chassis
`stinkyscape-048` · (1) Unit · Common · 4/1 → 8/2 · built by S03 (#1860)

> **Designer:** (1) Ultralight Chassis, Unit, Common, Stinkyscape, #48 · 4/1 / Starship Piece / Equip / Rush ~~~ 8/2 / Starship Piece / Equip / Rush

- **Text:** Starship Piece, Equip, Rush
- **Radiant:** Starship Piece, Equip, Rush
- **Engine:** Keywords only ([[§6.1]]): its script is empty. Equip ([[R1086]]): played onto one of your Units it fuses into it, the host keeping its zone and taking its stats, keywords and text ([[R77]], [[R102]]), Starship Piece only into a piece ([[R1852001]]); played alone its death fuses a copy into its owner's Dormant Starship ([[R1852000]]).
- **Rulings:** none new.
- **Numbers:** none (keywords and stats).
- **Check:** R275: as an Equip its stats are its effect, doubled.
- **Class:** A

#### Stinkyscape #49 · Spacemine
`stinkyscape-049` · (2) Unit · Common · 5/1 → 10/2 · built by S03 (#1860)

> **Designer:** (2) Spacemine, Unit, Common, Stinkyscape, #49 · 5/1 / Starship Piece / Death: Deal attack as damage to a random enemy. ~~~ 10/2 / Starship Piece / Death: Deal attack as damage to 2 random enemies.

- **Text:** Starship Piece / Death: Deal damage equal to its Attack to {targets|random enemy|random enemies}.
- **Radiant:** Starship Piece / Death: Deal damage equal to its Attack to {targets|random enemy|random enemies}.
- **Engine:** Death ([[§6.2]]), on the snapshot it died with ([[R89]]): its Attack then, one hit to each of N different random enemies, the hero or a Unit, drawn from the match rng ([[R60]]), fewer when fewer stand; at 0 Attack, nothing ([[R63]]). Then its copy fuses into the Dormant Starship ([[R1852000]]).
- **Rulings:** ST-S03-5: "attack" is the Attack it died with, buffs included; "2 random enemies" are two different enemies, one when one stands. These are proposals for the S03 part, which takes R-numbers from its own block.
- **Numbers:** targets 1 ↑ (Radiant 2).
- **Check:** R275: stats doubled, which doubles the hit, and a second target.
- **Class:** A

#### Stinkyscape #50 · Beam v3
`stinkyscape-050` · (2) Unit, Weapon · Rare · 0/4 → 0/8 · built by S06 (#1862)

> **Designer:** (2) Beam v3, Unit, Rare, Weapon, Stinkyscape, #50 · [0/4] / Starship Piece / Equip / Activate 1: Deal 6 damage, Cleave, Pierce *{has an extra turn cooldown}* / *{deals half damage to heroes}* ~~~ [0/8] / Starship Piece / Equip / Activate 1: Deal 12 damage, Cleave, Pierce *{has an extra turn cooldown}* / *{deals half damage to heroes}*

- **Text:** Starship Piece, Equip / Activate: Deal {damage} damage to an enemy, with Cleave and Pierce. Has an extra turn cooldown. Deals half damage to heroes.
- **Radiant:** Starship Piece, Equip / Activate: Deal {damage} damage to an enemy, with Cleave and Pierce. Has an extra turn cooldown. Deals half damage to heroes.
- **Engine:** Equip ([[R1086]]): played onto one of your Units it fuses into it, the host keeping its zone and taking its stats, keywords and text ([[R77]], [[R102]]), Starship Piece only into a piece ([[R1852001]]); played alone its death fuses a copy into its owner's Dormant Starship ([[R1852000]]); an equipped host takes the Activate, its uses counted per ability ([[R1751000]]). Activate, once a turn ([[R384]]): a declared target, an enemy Unit or the enemy hero, as the weapon kit's fixture reads "Deal N damage" ([[R81]]); `damage`; the same hit on each Unit beside a Unit target, Cleave's splash ([[§6.1]], `zones::adjacent`) composed in the script, since no effect rider for it is on main; Pierce as `ignoreArmor` ([[R346]]); `extraTurnCooldown` ([[R1841000]]), so it is refused on its user's next turn; `halfToHeroes` ([[R1841001]]): a hit on a hero halved, rounded down, before Armor, a Unit taking it whole.
- **Rulings:** ST-S06-1: "Deal N damage" with no target named is a declared enemy, Unit or hero, as the weapon kit's fixture; "Cleave" on an Activate's hit is the same hit on each Unit beside a Unit target (Cleave's splash, which never strikes back), and nothing more on a hero target. These are proposals for the S06 part, which takes R-numbers from its own block.
- **Numbers:** damage 6 ↑ (Radiant 12).
- **Check:** The designer's "Activate 1" is Activate; his notes are the M05 riders. R275: damage doubled.
- **Class:** C

#### Stinkyscape #51 · Repeater v3
`stinkyscape-051` · (2) Unit, Weapon · Rare · 0/4 → 0/8 · built by S06 (#1862)

> **Designer:** (2) Repeater v3, Unit, Rare, Weapon, Stinkyscape, #51 · [0/4] / Starship Piece / Equip / Activate 4: Deal 1 damage, Pierce / *{deals half damage to heroes}* ~~~ [0/8] / Starship Piece / Equip / Activate 4: Deal 2 damage, Pierce / *{deals half damage to heroes}*

- **Text:** Starship Piece, Equip / Activate 4: Deal {damage} damage to an enemy, with Pierce. Deals half damage to heroes.
- **Radiant:** Starship Piece, Equip / Activate 4: Deal {damage} damage to an enemy, with Pierce. Deals half damage to heroes.
- **Engine:** Equip ([[R1086]]): played onto one of your Units it fuses into it, the host keeping its zone and taking its stats, keywords and text ([[R77]], [[R102]]), Starship Piece only into a piece ([[R1852001]]); played alone its death fuses a copy into its owner's Dormant Starship ([[R1852000]]); an equipped host takes the Activate, its uses counted per ability ([[R1751000]]). Activate 4, four uses a turn ([[R384]], [[R1751000]]): a declared target, an enemy Unit or the enemy hero, as the weapon kit's fixture reads "Deal N damage" ([[R81]]); `damage`; Pierce as `ignoreArmor` ([[R346]]), the hits one at a time, each its own state check; `halfToHeroes` ([[R1841001]]): a hit on a hero halved, rounded down, before Armor, a Unit taking it whole.
- **Rulings:** ST-S06-1 (its target); its hits are one at a time, so a Divine Shield takes one. These are proposals for the S06 part, which takes R-numbers from its own block.
- **Numbers:** damage 1 ↑ (Radiant 2).
- **Check:** Activate 4 as printed, a label F adds where it is 3 to 6 (R1857003). R275: damage doubled.
- **Class:** C

#### Stinkyscape #52 · Cannon v3
`stinkyscape-052` · (2) Unit, Weapon · Rare · 0/4 → 0/8 · built by S06 (#1862)

> **Designer:** (2) Cannon v3, Unit, Rare, Weapon, Stinkyscape, #52 · [0/4] / Starship Piece / Equip / Activate 1: Deal 5 damage / *{deals half damage to heroes}* ~~~ [0/8] / Starship Piece / Equip / Activate 1: Deal 10 damage / *{deals half damage to heroes}*

- **Text:** Starship Piece, Equip / Activate: Deal {damage} damage to an enemy. Deals half damage to heroes.
- **Radiant:** Starship Piece, Equip / Activate: Deal {damage} damage to an enemy. Deals half damage to heroes.
- **Engine:** Equip ([[R1086]]): played onto one of your Units it fuses into it, the host keeping its zone and taking its stats, keywords and text ([[R77]], [[R102]]), Starship Piece only into a piece ([[R1852001]]); played alone its death fuses a copy into its owner's Dormant Starship ([[R1852000]]); an equipped host takes the Activate, its uses counted per ability ([[R1751000]]). Activate, once a turn ([[R384]]): a declared target, an enemy Unit or the enemy hero, as the weapon kit's fixture reads "Deal N damage" ([[R81]]); `damage`; `halfToHeroes` ([[R1841001]]): a hit on a hero halved, rounded down, before Armor, a Unit taking it whole.
- **Rulings:** ST-S06-1 (its target). These are proposals for the S06 part, which takes R-numbers from its own block.
- **Numbers:** damage 5 ↑ (Radiant 10).
- **Check:** The designer's "Activate 1" is Activate. R275: damage doubled.
- **Class:** C

#### Stinkyscape #53 · Flak v3
`stinkyscape-053` · (2) Unit, Weapon · Rare · 0/4 → 0/8 · built by S06 (#1862)

> **Designer:** (2) Flak v3, Unit, Rare, Weapon, Stinkyscape, #53 · [0/4] / Starship Piece / Equip / Activate 1: Deal 2 damage to all enemies ~~~ [0/8] / Starship Piece / Equip / Activate 1: Deal 4 damage to all enemies

- **Text:** Starship Piece, Equip / Activate: Deal {damage} damage to all enemies.
- **Radiant:** Starship Piece, Equip / Activate: Deal {damage} damage to all enemies.
- **Engine:** Equip ([[R1086]]): played onto one of your Units it fuses into it, the host keeping its zone and taking its stats, keywords and text ([[R77]], [[R102]]), Starship Piece only into a piece ([[R1852001]]); played alone its death fuses a copy into its owner's Dormant Starship ([[R1852000]]); an equipped host takes the Activate, its uses counted per ability ([[R1751000]]). Activate, once a turn ([[R384]]): `damage_all` over the enemy hero and every enemy Unit, one hit each before one state check ([[R59]]).
- **Rulings:** ST-S06-2: "all enemies" is the enemy hero and every enemy Unit; "twice" and "three times" are whole rounds, each hitting every enemy that still stands, each its own state check. No half damage: the designer gives Flak none. These are proposals for the S06 part, which takes R-numbers from its own block.
- **Numbers:** damage 2 ↑ (Radiant 4).
- **Check:** The designer's "Activate 1" is Activate; "thrice" is "three times". R275: damage doubled.
- **Class:** A

#### Stinkyscape #54 · Beam v4
`stinkyscape-054` · (3) Unit, Weapon · Epic · 0/6 → 0/12 · built by S06 (#1862)

> **Designer:** (3) Beam v4, Unit, Epic, Weapon, Stinkyscape, #54 · [0/6] / Starship Piece / Equip / Activate 1: Deal 9 damage, Cleave, Pierce *{has an extra turn cooldown}* / *{deals half damage to heroes}* ~~~ [0/12] / Starship Piece / Equip / Activate 1: Deal 18 damage, Cleave, Pierce *{has an extra turn cooldown}* / *{deals half damage to heroes}*

- **Text:** Starship Piece, Equip / Activate: Deal {damage} damage to an enemy, with Cleave and Pierce. Has an extra turn cooldown. Deals half damage to heroes.
- **Radiant:** Starship Piece, Equip / Activate: Deal {damage} damage to an enemy, with Cleave and Pierce. Has an extra turn cooldown. Deals half damage to heroes.
- **Engine:** Equip ([[R1086]]): played onto one of your Units it fuses into it, the host keeping its zone and taking its stats, keywords and text ([[R77]], [[R102]]), Starship Piece only into a piece ([[R1852001]]); played alone its death fuses a copy into its owner's Dormant Starship ([[R1852000]]); an equipped host takes the Activate, its uses counted per ability ([[R1751000]]). Activate, once a turn ([[R384]]): a declared target, an enemy Unit or the enemy hero, as the weapon kit's fixture reads "Deal N damage" ([[R81]]); `damage`; the same hit on each Unit beside a Unit target, Cleave's splash ([[§6.1]], `zones::adjacent`) composed in the script, since no effect rider for it is on main; Pierce as `ignoreArmor` ([[R346]]); `extraTurnCooldown` ([[R1841000]]), so it is refused on its user's next turn; `halfToHeroes` ([[R1841001]]): a hit on a hero halved, rounded down, before Armor, a Unit taking it whole.
- **Rulings:** ST-S06-1 (as at S #50). These are proposals for the S06 part, which takes R-numbers from its own block.
- **Numbers:** damage 9 ↑ (Radiant 18).
- **Check:** The designer's "Activate 1" is Activate; his notes are the M05 riders. R275: damage doubled.
- **Class:** C

#### Stinkyscape #55 · Repeater v4
`stinkyscape-055` · (3) Unit, Weapon · Epic · 0/6 → 0/12 · built by S06 (#1862)

> **Designer:** (3) Repeater v4, Unit, Epic, Weapon, Stinkyscape, #55 · [0/6] / Starship Piece / Equip / Activate 6: Deal 1 damage, Pierce / *{deals half damage to heroes}* ~~~ [0/12] / Starship Piece / Equip / Activate 6: Deal 2 damage, Pierce / *{deals half damage to heroes}*

- **Text:** Starship Piece, Equip / Activate 6: Deal {damage} damage to an enemy, with Pierce. Deals half damage to heroes.
- **Radiant:** Starship Piece, Equip / Activate 6: Deal {damage} damage to an enemy, with Pierce. Deals half damage to heroes.
- **Engine:** Equip ([[R1086]]): played onto one of your Units it fuses into it, the host keeping its zone and taking its stats, keywords and text ([[R77]], [[R102]]), Starship Piece only into a piece ([[R1852001]]); played alone its death fuses a copy into its owner's Dormant Starship ([[R1852000]]); an equipped host takes the Activate, its uses counted per ability ([[R1751000]]). Activate 6, six uses a turn ([[R384]], [[R1751000]]): a declared target, an enemy Unit or the enemy hero, as the weapon kit's fixture reads "Deal N damage" ([[R81]]); `damage`; Pierce as `ignoreArmor` ([[R346]]), the hits one at a time, each its own state check; `halfToHeroes` ([[R1841001]]): a hit on a hero halved, rounded down, before Armor, a Unit taking it whole.
- **Rulings:** ST-S06-1 (its target); its hits are one at a time, so a Divine Shield takes one. These are proposals for the S06 part, which takes R-numbers from its own block.
- **Numbers:** damage 1 ↑ (Radiant 2).
- **Check:** Activate 6 as printed, a label F adds where it is 3 to 6 (R1857003). R275: damage doubled.
- **Class:** C

#### Stinkyscape #56 · Cannon v4
`stinkyscape-056` · (3) Unit, Weapon · Epic · 0/6 → 0/12 · built by S06 (#1862)

> **Designer:** (3) Cannon v4, Unit, Epic, Weapon, Stinkyscape, #56 · [0/6] / Starship Piece / Equip / Activate 1: Deal 7 damage / *{deals half damage to heroes}* ~~~ [0/12] / Starship Piece / Equip / Activate 1: Deal 14 damage / *{deals half damage to heroes}*

- **Text:** Starship Piece, Equip / Activate: Deal {damage} damage to an enemy. Deals half damage to heroes.
- **Radiant:** Starship Piece, Equip / Activate: Deal {damage} damage to an enemy. Deals half damage to heroes.
- **Engine:** Equip ([[R1086]]): played onto one of your Units it fuses into it, the host keeping its zone and taking its stats, keywords and text ([[R77]], [[R102]]), Starship Piece only into a piece ([[R1852001]]); played alone its death fuses a copy into its owner's Dormant Starship ([[R1852000]]); an equipped host takes the Activate, its uses counted per ability ([[R1751000]]). Activate, once a turn ([[R384]]): a declared target, an enemy Unit or the enemy hero, as the weapon kit's fixture reads "Deal N damage" ([[R81]]); `damage`; `halfToHeroes` ([[R1841001]]): a hit on a hero halved, rounded down, before Armor, a Unit taking it whole.
- **Rulings:** ST-S06-1 (its target). These are proposals for the S06 part, which takes R-numbers from its own block.
- **Numbers:** damage 7 ↑ (Radiant 14).
- **Check:** The designer's "Activate 1" is Activate. R275: damage doubled.
- **Class:** C

#### Stinkyscape #57 · Flak v4
`stinkyscape-057` · (3) Unit, Weapon · Epic · 0/6 → 0/12 · built by S06 (#1862)

> **Designer:** (3) Flak v4, Unit, Epic, Weapon, Stinkyscape, #57 · [0/6] / Starship Piece / Equip / Activate 1: Deal 2 damage to all enemies, twice ~~~ [0/12] / Starship Piece / Equip / Activate 1: Deal 4 damage to all enemies, twice

- **Text:** Starship Piece, Equip / Activate: Deal {damage} damage to all enemies, twice.
- **Radiant:** Starship Piece, Equip / Activate: Deal {damage} damage to all enemies, twice.
- **Engine:** Equip ([[R1086]]): played onto one of your Units it fuses into it, the host keeping its zone and taking its stats, keywords and text ([[R77]], [[R102]]), Starship Piece only into a piece ([[R1852001]]); played alone its death fuses a copy into its owner's Dormant Starship ([[R1852000]]); an equipped host takes the Activate, its uses counted per ability ([[R1751000]]). Activate, once a turn ([[R384]]): `damage_all` over the enemy hero and every enemy Unit, one hit each before one state check ([[R59]]), twice: two such rounds, each its own.
- **Rulings:** ST-S06-2 (as at S #53). These are proposals for the S06 part, which takes R-numbers from its own block.
- **Numbers:** damage 2 ↑ (Radiant 4).
- **Check:** The designer's "Activate 1" is Activate; "thrice" is "three times". R275: damage doubled.
- **Class:** A

#### Stinkyscape #58 · Beam v5
`stinkyscape-058` · (4) Unit, Weapon · Legendary · 0/8 → 0/16 · built by S06 (#1862)

> **Designer:** (4) Beam v5, Unit, Legendary, Weapon, Stinkyscape, #58 · [0/8] / Starship Piece / Equip / Activate 1: Deal 15 damage, Cleave, Pierce *{has an extra turn cooldown}* / *{deals half damage to heroes}* ~~~ [0/16] / Starship Piece / Equip / Activate 1: Deal 30 damage, Cleave, Pierce *{has an extra turn cooldown}* / *{deals half damage to heroes}*

- **Text:** Starship Piece, Equip / Activate: Deal {damage} damage to an enemy, with Cleave and Pierce. Has an extra turn cooldown. Deals half damage to heroes.
- **Radiant:** Starship Piece, Equip / Activate: Deal {damage} damage to an enemy, with Cleave and Pierce. Has an extra turn cooldown. Deals half damage to heroes.
- **Engine:** Equip ([[R1086]]): played onto one of your Units it fuses into it, the host keeping its zone and taking its stats, keywords and text ([[R77]], [[R102]]), Starship Piece only into a piece ([[R1852001]]); played alone its death fuses a copy into its owner's Dormant Starship ([[R1852000]]); an equipped host takes the Activate, its uses counted per ability ([[R1751000]]). Activate, once a turn ([[R384]]): a declared target, an enemy Unit or the enemy hero, as the weapon kit's fixture reads "Deal N damage" ([[R81]]); `damage`; the same hit on each Unit beside a Unit target, Cleave's splash ([[§6.1]], `zones::adjacent`) composed in the script, since no effect rider for it is on main; Pierce as `ignoreArmor` ([[R346]]); `extraTurnCooldown` ([[R1841000]]), so it is refused on its user's next turn; `halfToHeroes` ([[R1841001]]): a hit on a hero halved, rounded down, before Armor, a Unit taking it whole.
- **Rulings:** ST-S06-1 (as at S #50). These are proposals for the S06 part, which takes R-numbers from its own block.
- **Numbers:** damage 15 ↑ (Radiant 30).
- **Check:** The designer's "Activate 1" is Activate; his notes are the M05 riders. R275: damage doubled.
- **Class:** C

#### Stinkyscape #59 · Repeater v5
`stinkyscape-059` · (4) Unit, Weapon · Legendary · 0/8 → 0/16 · built by S06 (#1862)

> **Designer:** (4) Repeater v5, Unit, Legendary, Weapon, Stinkyscape, #59 · [0/8] / Starship Piece / Equip / Activate 6: Deal 2 damage, Pierce / *{deals half damage to heroes}* ~~~ [0/16] / Starship Piece / Equip / Activate 6: Deal 4 damage, Pierce / *{deals half damage to heroes}*

- **Text:** Starship Piece, Equip / Activate 6: Deal {damage} damage to an enemy, with Pierce. Deals half damage to heroes.
- **Radiant:** Starship Piece, Equip / Activate 6: Deal {damage} damage to an enemy, with Pierce. Deals half damage to heroes.
- **Engine:** Equip ([[R1086]]): played onto one of your Units it fuses into it, the host keeping its zone and taking its stats, keywords and text ([[R77]], [[R102]]), Starship Piece only into a piece ([[R1852001]]); played alone its death fuses a copy into its owner's Dormant Starship ([[R1852000]]); an equipped host takes the Activate, its uses counted per ability ([[R1751000]]). Activate 6, six uses a turn ([[R384]], [[R1751000]]): a declared target, an enemy Unit or the enemy hero, as the weapon kit's fixture reads "Deal N damage" ([[R81]]); `damage`; Pierce as `ignoreArmor` ([[R346]]), the hits one at a time, each its own state check; `halfToHeroes` ([[R1841001]]): a hit on a hero halved, rounded down, before Armor, a Unit taking it whole.
- **Rulings:** ST-S06-1 (its target); its hits are one at a time, so a Divine Shield takes one. These are proposals for the S06 part, which takes R-numbers from its own block.
- **Numbers:** damage 2 ↑ (Radiant 4).
- **Check:** Activate 6 as printed, a label F adds where it is 3 to 6 (R1857003). R275: damage doubled.
- **Class:** C

#### Stinkyscape #60 · Cannon v5
`stinkyscape-060` · (4) Unit, Weapon · Legendary · 0/8 → 0/16 · built by S06 (#1862)

> **Designer:** (4) Cannon v5, Unit, Legendary, Weapon, Stinkyscape, #60 · [0/8] / Starship Piece / Equip / Activate 1: Deal 10 damage / *{deals half damage to heroes}* ~~~ [0/16] / Starship Piece / Equip / Activate 1: Deal 20 damage / *{deals half damage to heroes}*

- **Text:** Starship Piece, Equip / Activate: Deal {damage} damage to an enemy. Deals half damage to heroes.
- **Radiant:** Starship Piece, Equip / Activate: Deal {damage} damage to an enemy. Deals half damage to heroes.
- **Engine:** Equip ([[R1086]]): played onto one of your Units it fuses into it, the host keeping its zone and taking its stats, keywords and text ([[R77]], [[R102]]), Starship Piece only into a piece ([[R1852001]]); played alone its death fuses a copy into its owner's Dormant Starship ([[R1852000]]); an equipped host takes the Activate, its uses counted per ability ([[R1751000]]). Activate, once a turn ([[R384]]): a declared target, an enemy Unit or the enemy hero, as the weapon kit's fixture reads "Deal N damage" ([[R81]]); `damage`; `halfToHeroes` ([[R1841001]]): a hit on a hero halved, rounded down, before Armor, a Unit taking it whole.
- **Rulings:** ST-S06-1 (its target). These are proposals for the S06 part, which takes R-numbers from its own block.
- **Numbers:** damage 10 ↑ (Radiant 20).
- **Check:** The designer's "Activate 1" is Activate. R275: damage doubled.
- **Class:** C

#### Stinkyscape #61 · Flak v5
`stinkyscape-061` · (4) Unit, Weapon · Legendary · 0/8 → 0/16 · built by S06 (#1862)

> **Designer:** (4) Flak v5, Unit, Legendary, Weapon, Stinkyscape, #61 · [0/8] / Starship Piece / Equip / Activate 1: Deal 2 damage to all enemies, thrice ~~~ [0/16] / Starship Piece / Equip / Activate 3: Deal 4 damage to all enemies, thrice

- **Text:** Starship Piece, Equip / Activate: Deal {damage} damage to all enemies, three times.
- **Radiant:** Starship Piece, Equip / Activate 3: Deal {damage} damage to all enemies, three times.
- **Engine:** Equip ([[R1086]]): played onto one of your Units it fuses into it, the host keeping its zone and taking its stats, keywords and text ([[R77]], [[R102]]), Starship Piece only into a piece ([[R1852001]]); played alone its death fuses a copy into its owner's Dormant Starship ([[R1852000]]); an equipped host takes the Activate, its uses counted per ability ([[R1751000]]). Activate, once a turn ([[R384]]); the Radiant face's Activate 3, three uses a turn ([[R384]], [[R1751000]]): `damage_all` over the enemy hero and every enemy Unit, one hit each before one state check ([[R59]]), three times: three such rounds, each its own.
- **Rulings:** ST-S06-2 (as at S #53). These are proposals for the S06 part, which takes R-numbers from its own block.
- **Numbers:** damage 2 ↑ (Radiant 4).
- **Check:** The designer's "Activate 1" is Activate; "thrice" is "three times". R275: damage doubled, and Activate 3 on the Radiant face (the designer's).
- **Class:** A

#### Stinkyscape #62 · Miner v1
`stinkyscape-062` · (0) Unit · Common · 0/1 → 0/2 · built by S05 (#1872)

> **Designer:** (0) Miner v1, Unit, Common, Stinkyscape, #62 · [0/1] / Starship Piece / Equip / Activate 1: Gain a random Stinkyscape resource. *{has an extra turn cooldown}* ~~~ [0/2] / Starship Piece / Equip / Activate 1: Gain a random Radiant Stinkyscape resource. *{has an extra turn cooldown}*

- **Text:** Starship Piece, Equip / Activate: Gain a random Stinkyscape Resource. Has an extra turn cooldown.
- **Radiant:** Starship Piece, Equip / Activate: Gain a random Radiant Stinkyscape Resource. Has an extra turn cooldown.
- **Engine:** Equip ([[R1086]]): played onto one of your Units it fuses into it, the host keeping its zone and taking its stats, keywords and text ([[R77]], [[R102]]), Starship Piece only into a piece ([[R1852001]]); played alone its death fuses a copy into its owner's Dormant Starship ([[R1852000]]); an equipped host takes the Activate. Activate, once a turn ([[R384]], [[R1751000]]), with `extraTurnCooldown` ([[R1841000]]), refused on its user's next turn: `gain_resources { count: 1 }`, the rarity roll then one Resource of the rolled rarity over every Resource in the catalog ([[R1840002]]), into its controller's tray and never the hand ([[R1840000]]), a ninth sold at once ([[R1840001]]); the Radiant face's `radiant: true`.
- **Rulings:** ST-S05-1: a Miner's Resource is a Stinkyscape Resource, every Resource the catalog holds, by R1840002's roll; it goes to the controller of the Unit the Activate is on, an equipped host's controller included. These are proposals for the S05 part, which takes R-numbers from its own block.
- **Numbers:** none (one Resource a use; the Activate count is no param, R482).
- **Check:** The designer's "Activate 1" is Activate; "has an extra turn cooldown" is the M05 rider. R275: health doubled and a Radiant Resource.
- **Class:** C

#### Stinkyscape #63 · Miner v2
`stinkyscape-063` · (1) Unit · Common · 0/2 → 0/4 · built by S05 (#1872)

> **Designer:** (1) Miner v2, Unit, Common, Stinkyscape, #63 · [0/2] / Starship Piece / Equip / Activate 1: Gain a random Stinkyscape resource. ~~~ [0/4] / Starship Piece / Equip / Activate 1: Gain a random Radiant Stinkyscape resource.

- **Text:** Starship Piece, Equip / Activate: Gain a random Stinkyscape Resource.
- **Radiant:** Starship Piece, Equip / Activate: Gain a random Radiant Stinkyscape Resource.
- **Engine:** Equip ([[R1086]]): played onto one of your Units it fuses into it, the host keeping its zone and taking its stats, keywords and text ([[R77]], [[R102]]), Starship Piece only into a piece ([[R1852001]]); played alone its death fuses a copy into its owner's Dormant Starship ([[R1852000]]); an equipped host takes the Activate. Activate, once a turn ([[R384]], [[R1751000]]): `gain_resources { count: 1 }`, the rarity roll then one Resource of the rolled rarity over every Resource in the catalog ([[R1840002]]), into its controller's tray and never the hand ([[R1840000]]), a ninth sold at once ([[R1840001]]); the Radiant face's `radiant: true`.
- **Rulings:** ST-S05-1 (as at S #62). These are proposals for the S05 part, which takes R-numbers from its own block.
- **Numbers:** none (one Resource a use; the Activate count is no param, R482).
- **Check:** The designer's "Activate 1" is Activate. R275: health doubled and a Radiant Resource.
- **Class:** C

#### Stinkyscape #64 · Miner v3
`stinkyscape-064` · (2) Unit · Rare · 0/4 → 0/8 · built by S05 (#1872)

> **Designer:** (2) Miner v3, Unit, Rare, Stinkyscape, #64 · [0/4] / Starship Piece / Equip / Activate 2: Gain a random Stinkyscape resource. ~~~ [0/8] / Starship Piece / Equip / Activate 2: Gain a random Radiant Stinkyscape resource.

- **Text:** Starship Piece, Equip / Activate 2: Gain a random Stinkyscape Resource.
- **Radiant:** Starship Piece, Equip / Activate 2: Gain a random Radiant Stinkyscape Resource.
- **Engine:** Equip ([[R1086]]): played onto one of your Units it fuses into it, the host keeping its zone and taking its stats, keywords and text ([[R77]], [[R102]]), Starship Piece only into a piece ([[R1852001]]); played alone its death fuses a copy into its owner's Dormant Starship ([[R1852000]]); an equipped host takes the Activate. Activate 2, two uses a turn ([[R384]], [[R1751000]]): `gain_resources { count: 1 }`, the rarity roll then one Resource of the rolled rarity over every Resource in the catalog ([[R1840002]]), into its controller's tray and never the hand ([[R1840000]]), a ninth sold at once ([[R1840001]]); the Radiant face's `radiant: true`.
- **Rulings:** ST-S05-1 (as at S #62). These are proposals for the S05 part, which takes R-numbers from its own block.
- **Numbers:** none (one Resource a use; the Activate count is no param, R482).
- **Check:** The designer's "Activate 1" is Activate. R275: health doubled and a Radiant Resource.
- **Class:** C

#### Stinkyscape #65 · Miner v4
`stinkyscape-065` · (3) Unit · Epic · 0/6 → 0/12 · built by S05 (#1872)

> **Designer:** (3) Miner v4, Unit, Epic, Stinkyscape, #65 · [0/6] / Starship Piece / Equip / Activate 3: Gain a random Stinkyscape resource. ~~~ [0/12] / Starship Piece / Equip / Activate 3: Gain a random Radiant Stinkyscape resource.

- **Text:** Starship Piece, Equip / Activate 3: Gain a random Stinkyscape Resource.
- **Radiant:** Starship Piece, Equip / Activate 3: Gain a random Radiant Stinkyscape Resource.
- **Engine:** Equip ([[R1086]]): played onto one of your Units it fuses into it, the host keeping its zone and taking its stats, keywords and text ([[R77]], [[R102]]), Starship Piece only into a piece ([[R1852001]]); played alone its death fuses a copy into its owner's Dormant Starship ([[R1852000]]); an equipped host takes the Activate. Activate 3, three uses a turn ([[R384]], [[R1751000]]): `gain_resources { count: 1 }`, the rarity roll then one Resource of the rolled rarity over every Resource in the catalog ([[R1840002]]), into its controller's tray and never the hand ([[R1840000]]), a ninth sold at once ([[R1840001]]); the Radiant face's `radiant: true`.
- **Rulings:** ST-S05-1 (as at S #62). These are proposals for the S05 part, which takes R-numbers from its own block.
- **Numbers:** none (one Resource a use; the Activate count is no param, R482).
- **Check:** The designer's "Activate 1" is Activate. R275: health doubled and a Radiant Resource.
- **Class:** C

#### Stinkyscape #66 · Miner v5
`stinkyscape-066` · (4) Unit · Legendary · 0/8 → 0/16 · built by S05 (#1872)

> **Designer:** (4) Miner v5, Unit, Legendary, Stinkyscape, #66 · [0/8] / Starship Piece / Equip / Activate 5: Gain a random Stinkyscape resource. ~~~ [0/16] / Starship Piece / Equip / Activate 5: Gain a random Radiant Stinkyscape resource.

- **Text:** Starship Piece, Equip / Activate 5: Gain a random Stinkyscape Resource.
- **Radiant:** Starship Piece, Equip / Activate 5: Gain a random Radiant Stinkyscape Resource.
- **Engine:** Equip ([[R1086]]): played onto one of your Units it fuses into it, the host keeping its zone and taking its stats, keywords and text ([[R77]], [[R102]]), Starship Piece only into a piece ([[R1852001]]); played alone its death fuses a copy into its owner's Dormant Starship ([[R1852000]]); an equipped host takes the Activate. Activate 5, five uses a turn ([[R384]], [[R1751000]]): `gain_resources { count: 1 }`, the rarity roll then one Resource of the rolled rarity over every Resource in the catalog ([[R1840002]]), into its controller's tray and never the hand ([[R1840000]]), a ninth sold at once ([[R1840001]]); the Radiant face's `radiant: true`.
- **Rulings:** ST-S05-1 (as at S #62). These are proposals for the S05 part, which takes R-numbers from its own block.
- **Numbers:** none (one Resource a use; the Activate count is no param, R482).
- **Check:** The designer's "Activate 1" is Activate. R275: health doubled and a Radiant Resource.
- **Class:** C

#### Stinkyscape #67 · Tractor Beam
`stinkyscape-067` · (1) Unit · Rare · 0/2 → 0/4 · built by S04 (#1861)

> **Designer:** (1) Tractor Beam, Unit, Rare, Stinkyscape, #67 · [0/2] / Starship Piece / Equip / Start of your turn: Gain a random Stinkyscape resource. ~~~ [0/4] / Starship Piece / Equip / Start of your turn: Gain a random Radiant Stinkyscape resource.

- **Text:** Starship Piece, Equip / Start of turn: Gain a random Stinkyscape Resource.
- **Radiant:** Starship Piece, Equip / Start of turn: Gain a random Radiant Stinkyscape Resource.
- **Engine:** Starship Piece ([[§6.1]], [[R1852000]]): its death fuses a copy into its owner's Dormant Starship. Equip ([[R1086]]): played onto one of your Units it fuses into it, the host gaining its Start of turn line and staying no piece ([[R1852001]]). Start of turn ([[R62]]): `gain_resources` of one random Resource into its controller's tray ([[R1840000]], [[R1840002]]'s roll), a ninth sold at once ([[R1840001]]); the first gain opens #1853's Marketplace (pending PR #1963). Radiant: a Radiant Resource.
- **Rulings:** ST-S04-4: equipped, its host gains the Resource at its controller's start of turn, the fused text's ([[R1086]]), and is no Starship Piece ([[R1852001]]).
- **Numbers:** none (one Resource, written "a").
- **Check:** "Start of your turn" is the label Start of turn ([[R366]]); "resource" is a Resource. R275: stats doubled; Radiant Resources.
- **Class:** C.

#### Stinkyscape #68 · Startos
`stinkyscape-068` · (1) Unit · Common · 3/3 → 6/6 · built by S03 (#1860)

> **Designer:** (1) Startos, Unit, Common, Stinkyscape, #68 · Starship Piece / Flying / [3/3] ~~~ Starship Piece / Flying / [6/6]

- **Text:** Starship Piece, Flying
- **Radiant:** Starship Piece, Flying
- **Engine:** Keywords only ([[§6.1]]): its script is empty. Starship Piece: its death fuses a copy into its owner's Dormant Starship, summoned first if they have none ([[R1852000]]).
- **Rulings:** none new.
- **Numbers:** none (keywords and stats).
- **Check:** R275's effect half: stats only, as the designer printed it (⚠ designer: no keyword more).
- **Class:** A

#### Stinkyscape #69 · Falcon
`stinkyscape-069` · (2) Unit · Rare · 4/1 → 8/2 · built by S03 (#1860)

> **Designer:** (2) Falcon, Unit, Rare, Stinkyscape, #69 · [4/1] / Starship Piece / Charge, Flying ~~~ [8/1] / Starship Piece / Charge, Flying, Pierce

- **Text:** Starship Piece, Charge, Flying
- **Radiant:** Starship Piece, Charge, Flying, Pierce
- **Engine:** Keywords only ([[§6.1]]): its script is empty. Starship Piece: its death fuses a copy into its owner's Dormant Starship, summoned first if they have none ([[R1852000]]).
- **Rulings:** none new.
- **Numbers:** none (keywords and stats).
- **Check:** ST8: Radiant 8/2, not 8/1 (R275's stat half). Pierce is the keyword more.
- **Class:** A

#### Stinkyscape #70 · Z'Zoth
`stinkyscape-070` · (4) Unit · Legendary · 5/7 → 10/14 · built by S16 (#1869)

> **Designer:** (4) Z'Zoth, Unit, Legendary, Stinkyscape, #70 · 5/7 / Cry: Resummon all Death Units from your graveyard. ~~~ 10/14 / Cry: Resummon all Death Units from your graveyard. Give them Reborn.

- **Text:** Cry: Summon every Unit with a Death from your graveyard.
- **Radiant:** Cry: Summon every Unit with a Death from your graveyard. Give them Reborn.
- **Engine:** Cry ([[R1]]): C #44 Back from the GY's summon from the graveyard, filtered to the Unit cards whose printed text has a Death (`Script.death`), each summoned into [[R64]]'s zone while its row has one, no Cry ([[R1]]), the rest left there. Radiant: each summoned one is given Reborn (`grant_keyword`).
- **Rulings:** ST-S16-5: "Death Units" are Unit cards whose own text prints a Death; a Death granted on the field is gone in the graveyard (R78, R1107). These are proposals for S16, which takes R-numbers from its own block. No R-number here.
- **Numbers:** none.
- **Check:** R275: stats doubled and every summoned Unit Reborn.
- **Class:** B (a graveyard summon with a filter).

#### Stinkyscape #71 · A'Zoth
`stinkyscape-071` · (4) Unit · Legendary · 7/5 → 14/10 · built by S16 (#1869)

> **Designer:** (4) A'Zoth, Unit, Legendary, Stinkyscape, #71 · 7/5 / Cry: Resummon all Cry Units from your graveyard. Activate each Cry with random targets. ~~~ 14/10 / Cry: Resummon all Cry Units from your graveyard. Activate each Cry twice with random targets.

- **Text:** Cry: Summon every Unit with a Cry from your graveyard. Trigger each of their Cries with random targets.
- **Radiant:** Cry: Summon every Unit with a Cry from your graveyard. Trigger each of their Cries {repeats|time|times}, with random targets.
- **Engine:** Cry ([[R1]]): C #44's summon from the graveyard, filtered to the Unit cards with a Cry (`has_triggerable_cry`), no Cry on the summon ([[R1]]); then `trigger_cry` on each summoned one in lane order ([[R467]]), NEW: its targets and choices drawn from the match rng, as a random cast's are ([[§6.3]] Cast). Radiant: each one's Cry triggered twice.
- **Rulings:** ST-S16-6: "Activate each Cry" is Trigger a Cry (§6.3), never an Activate; the summon itself fires none (R1). ST-S16-7: a Cry with no legal target does nothing. These are proposals for S16, which takes R-numbers from its own block. No R-number here.
- **Numbers:** repeats 2 ↑, on the Radiant face only (`tunedOn: radiant`, its Cry's).
- **Check:** House style: "Trigger" for the list's "Activate each Cry". R275: stats doubled and every Cry twice.
- **Class:** B (a graveyard summon and triggered Cries), with NEW random answers.

#### Stinkyscape #72 · Paragon Mk1
`stinkyscape-072` · (3) Unit · Epic · 6/1 → 12/2 · built by S03 (#1860)

> **Designer:** (3) Paragon Mk1, Unit, Epic, Stinkyscape, #72 · [6/1] / Starship Piece / Charge, Flying, Ranged ~~~ [12/2] / Starship Piece / Charge, Flying, Ranged, Pierce

- **Text:** Starship Piece, Charge, Flying, Ranged
- **Radiant:** Starship Piece, Charge, Flying, Ranged, Pierce
- **Engine:** Keywords only ([[§6.1]]): its script is empty. Starship Piece: its death fuses a copy into its owner's Dormant Starship, summoned first if they have none ([[R1852000]]).
- **Rulings:** none new.
- **Numbers:** none (keywords and stats).
- **Check:** R275: stats doubled, Pierce the keyword more.
- **Class:** A

#### Stinkyscape #73 · Paragon Mk2
`stinkyscape-073` · (4) Unit · Legendary · 7/1 → 14/2 · built by S03 (#1860)

> **Designer:** (4) Paragon Mk2, Unit, Legendary, Stinkyscape, #73 · [7/1] / Starship Piece / Charge, Flying, Ranged, Divine Shield ~~~ [14/2] / Starship Piece / Charge, Flying, Ranged, Divine Shield, Pierce

- **Text:** Starship Piece, Charge, Flying, Ranged, Divine Shield
- **Radiant:** Starship Piece, Charge, Flying, Ranged, Divine Shield, Pierce
- **Engine:** Keywords only ([[§6.1]]): its script is empty. Starship Piece: its death fuses a copy into its owner's Dormant Starship, summoned first if they have none ([[R1852000]]).
- **Rulings:** none new.
- **Numbers:** none (keywords and stats).
- **Check:** R275: stats doubled, Pierce the keyword more.
- **Class:** A

#### Stinkyscape #74 · Paragon-X
`stinkyscape-074` · (4) Unit · Mythic · 10/1 → 20/2 · built by S16 (#1869)

> **Designer:** (4) Paragon-X, Unit, Mythic, Stinkyscape, #74 · [10/1] / Starship Piece / Tribute 2 / Charge, Flying, Ranged, Divine Shield, Immutable, Immune to Spell Damage ~~~ [20/2] / Starship Piece / Tribute 2 / Charge, Flying, Ranged, Divine Shield, Immutable, Immune to Spell Damage, Pierce

- **Text:** Tribute 2, Starship Piece, Charge, Flying, Ranged, Divine Shield, Immutable / Immune to Spell Damage.
- **Radiant:** Tribute 2, Starship Piece, Charge, Flying, Ranged, Divine Shield, Immutable, Pierce / Immune to Spell Damage.
- **Engine:** Keywords ([[§6.1]]): Starship Piece ([[R1852000]]: dying, a copy fuses into its owner's Dormant Starship), Charge, Flying, Ranged, Divine Shield, Immutable ([[R23]], [[R781001]]). Tribute 2, its play's cost ([[R101]]). Immune to Spell Damage (`staticFlags.immuneToSpellDamage`, [[R1849000]]): a Spell's hit on it is no damage instance, its Divine Shield kept. Radiant: Pierce.
- **Rulings:** ST-S16-8: Immune to Spell Damage is a static flag, printed as a sentence of its own, never in the keyword line (R366). These are proposals for S16, which takes R-numbers from its own block. No R-number here.
- **Numbers:** none (keywords; an Immutable card's numbers are untouchable, R386).
- **Check:** R275: stats doubled and Pierce added. R347 holds (no Taunt).
- **Class:** A (keywords and M15's flag).

#### Stinkyscape #75 · Badger
`stinkyscape-075` · (1) Unit · Common · 2/3 → 4/6 · built by S04 (#1861)

> **Designer:** (1) Badger, Unit, Common, Stinkyscape, #75 · [2/3] / Starship Piece / When this attacks add gain a random Stinkyscape resource. ~~~ [4/6] / Starship Piece / When this attacks add gain a random Radiant Stinkyscape resource.

- **Text:** Starship Piece / Whenever this attacks, gain a random Stinkyscape Resource.
- **Radiant:** Starship Piece / Whenever this attacks, gain a random Radiant Stinkyscape Resource.
- **Engine:** Starship Piece ([[R1852000]]). A trigger on its own `attackDeclared`, forced attacks included ([[R53]]), C #45 Nature Titan's: `gain_resources` of one random Resource into its controller's tray ([[R1840000]], [[R1840002]]), before the combat, so a Badger that dies in it keeps the Resource; an attack called off before it fought ([[R44]]) gains nothing. Radiant: a Radiant Resource.
- **Rulings:** ST-S04-5: "When this attacks" gains as the attack is declared and still stands, before the combat (Hearthstone's "whenever this attacks"), once per attack, forced ones included ([[R53]]); C #45 Nature Titan answers after the combat, which S04 may follow instead.
- **Numbers:** none (one Resource, written "a").
- **Check:** "add gain a random" is "gain a random" (ST8). R275: stats doubled; Radiant Resources.
- **Class:** C.

#### Stinkyscape #76 · Honey Badger
`stinkyscape-076` · (2) Unit · Rare · 4/6 → 8/12 · built by S04 (#1861)

> **Designer:** (2) Honey Badger, Unit, Rare, Stinkyscape, #76 · [4/6] / Starship Piece / Rush / When this attacks add gain a random Stinkyscape resource. ~~~ [8/12] / Starship Piece / Rush / When this attacks add gain a random Radiant Stinkyscape resource.

- **Text:** Starship Piece, Rush / Whenever this attacks, gain a random Stinkyscape Resource.
- **Radiant:** Starship Piece, Rush / Whenever this attacks, gain a random Radiant Stinkyscape Resource.
- **Engine:** As S #75 ([[R1852000]], [[R53]], [[R1840000]]), with Rush ([[§6.1]]), so it may attack a Unit and gain on the turn it lands. Radiant: a Radiant Resource.
- **Rulings:** none new: every reading is a ruling on `main` or the card's text.
- **Numbers:** none (one Resource, written "a").
- **Check:** "add gain a random" is "gain a random" (ST8). R275: stats doubled; Radiant Resources.
- **Class:** C.

#### Stinkyscape #77 · Yuma
`stinkyscape-077` · (2) Unit · Epic · 1/8 → 2/16 · built by S18 (#1877)

> **Designer:** (2) Yuma, Unit, Epic, Stinkyscape, #77 · 1/8 / Starship Piece / Flying / Cry: Trade offer your opponent at the start of their turn. You give them any card in your hand and they have to Discover a card in their own to give to you. ~~~ 2/16 / Starship Piece / Flying / Lucky 1 / Cry: Trade offer your opponent at the start of their turn. You give them any card in your hand and they have to Discover a card in their own to give to you.

- **Text:** Starship Piece, Flying / Cry: At the start of your opponent's next turn, make them a trade offer: you give them a card from your hand, then they Discover one of theirs to give you.
- **Radiant:** Starship Piece, Flying, Lucky 1 / Cry: At the start of your opponent's next turn, make them a trade offer: you give them a card from your hand, then they Discover one of theirs to give you.
- **Engine:** Starship Piece ([[R1852000]]), Flying. Cry ([[R1]]): `trade_offer` ([[R1845002]]): a delayed effect due at the start of the opponent's next turn, a countdown badge on its side; with a card in each hand its controller picks one of theirs (a `hand` prompt), it goes to the opponent's hand as theirs ([[R12]]), then the opponent Discovers one of their other hand cards and it comes to its controller's; an empty hand asks nothing. Radiant: Lucky 1 ([[§6.1]], [[R1438]]).
- **Rulings:** ST-S18-3: Lucky 1 reaches the rolls its own text makes, the Discover's three options drawn twice; S18 names the comparator (default: the set whose cheapest card costs most, since the opponent gives the worst). These are proposals for S18, which takes R-numbers from its own block. No R-number here.
- **Numbers:** none.
- **Check:** R275: stats doubled and Lucky 1.
- **Class:** C (hidden Nerfs and trade offers, M11).

#### Stinkyscape #78 · Ozark
`stinkyscape-078` · (3) Unit · Epic · 4/4 → 8/8 · built by S05 (#1872)

> **Designer:** (3) Ozark, Unit, Epic, Stinkyscape, #78 · [4/4] / Starship Piece / Cry: A random [Radiant] Miner vXs is Equip this. ~~~ [8/8] / Starship Piece / Cry: A random [Radiant] Miner vXs is Equip this, twice.

- **Text:** Starship Piece / Cry: Equip a random Miner into this.
- **Radiant:** Starship Piece / Cry: Equip a random Radiant Miner into this, twice.
- **Engine:** Starship Piece: its death fuses a copy into its owner's Dormant Starship, summoned first if they have none ([[R1852000]]). Cry ([[R1]]): `equip_picks` ([[R1841002]]), a random pick from the five Miners, S #62 Miner v1 to S #66 Miner v5, by id, a fresh copy fused straight into it as an Equip play fuses its card ([[R1086]], [[R77]]): no hand, no play, no Cry, its Activate with it; the Radiant face makes two picks, each Radiant ([[R469]]), the second from the Miners it does not hold. Off the field, Immutable, or holding every Miner, nothing ([[R129]]). It stays a piece, the kept card being one ([[R1852001]]).
- **Rulings:** ST-S05-2: the designer's "[Radiant]" on both faces marks what the Radiant face changes, as his brackets do on one-face entries (the M05 fixture's chest reads it so): the base face's Miner is plain, the Radiant face's two are Radiant. "twice" is two picks, the second from the Miners it does not hold yet. "Miner vXs" are S #62 to S #66. These are proposals for the S05 part, which takes R-numbers from its own block.
- **Numbers:** none (one pick, two on the Radiant face: the frame).
- **Check:** The designer's "is Equip this" is "Equip into this". R275: stats doubled, the pick doubled and Radiant.
- **Class:** C

#### Stinkyscape #79 · Orca
`stinkyscape-079` · (4, Embiggened 5) Unit · Epic · 8/8 → 16/16 · built by S05 (#1872)

> **Designer:** (4) [Embiggen 5] Orca, Unit, Epic, Stinkyscape, #79 · 8/8 / Starship Piece / Flying / Cry: Choose between 2 [Radiant] Miner vXs to Equip this. ~~~ 16/16 / Starship Piece / Flying / Cry: Choose between 2 [Radiant] Miner vXs to Equip this, twice

- **Text:** Starship Piece, Flying / Cry: Choose one of 2 Miners and Equip it into this. / Embiggened (5): The Miners offered are Radiant.
- **Radiant:** Starship Piece, Flying / Cry: Choose one of 2 Miners and Equip it into this, twice. / Embiggened (5): The Miners offered are Radiant.
- **Engine:** Starship Piece: its death fuses a copy into its owner's Dormant Starship, summoned first if they have none ([[R1852000]]). Cry: `equip_picks` with a Discover of `options` 2 ([[R1841002]], `EQUIP_PICK_STEP`) from the five Miners, S #62 Miner v1 to S #66 Miner v5, by id; the Radiant face twice, the second offer once the first pick is in and without it. Embiggened ([[R1175000]]): the offer is Radiant, the Cry reading `instance.embiggened` ([[R81]]). Its Embiggened price, (5), is past `MAX_MANA` (4), so it is played so only with mana an effect gave ([[R1857001]]).
- **Rulings:** ST-S05-3: the designer's "[Radiant]" is the Embiggened version's (ST8, ST9: the offered Miners are Radiant); the Radiant face's change is "twice" alone, and Embiggened it offers Radiant Miners too. Its (5) is past `MAX_MANA` (4), so it is played Embiggened only with mana an effect gave (R1857001). These are proposals for the S05 part, which takes R-numbers from its own block.
- **Numbers:** none (the offer of 2 and the picks are the frame).
- **Check:** ST8 and ST9: Embiggened (5), the offered Miners Radiant; the cost {4, embiggen 5} (R1857003's label). R275: stats doubled and the picks doubled.
- **Class:** C

#### Stinkyscape #80 · Galaxy
`stinkyscape-080` · (4) Unit · Legendary · 10/10 → 20/20 · built by S05 (#1872)

> **Designer:** (4) Galaxy, Unit, Legendary, Stinkyscape, #80 · 10/10 / Starship Piece / Flying / Tribute 2 / Cry: Discover 2 Radiant Miner vXs to Equip this. ~~~ 20/20 / Starship Piece / Flying / Tribute 2 / Cry: Equip this with each Radiant Miner vX.

- **Text:** Tribute 2, Starship Piece, Flying / Cry: Discover 2 Radiant Miners and Equip them into this.
- **Radiant:** Tribute 2, Starship Piece, Flying / Cry: Equip a Radiant copy of every Miner into this.
- **Engine:** Tribute 2 ([[§6.3]] Tribute, [[R101]]), printed in the keyword line ([[R366]]), the script's `tribute`. Starship Piece: its death fuses a copy into its owner's Dormant Starship, summoned first if they have none ([[R1852000]]). Cry: `equip_picks`, two Discovers of three in turn ([[R1841002]]) from the five Miners, S #62 Miner v1 to S #66 Miner v5, by id, Radiant, the second without the first; the Radiant face fuses all five, Radiant, Miner v1 first, asking nothing.
- **Rulings:** ST-S05-4: "Equip this with each Radiant Miner vX" is all five Miners, Radiant, fused one by one, v1 to v5. These are proposals for the S05 part, which takes R-numbers from its own block.
- **Numbers:** none (the picks are the frame; Tribute is no param, R482).
- **Check:** R275: stats doubled; 2 of 5 → 5 of 5.
- **Class:** C

#### Stinkyscape #81 · Radix
`stinkyscape-081` · (3) Unit · Rare · 2/2 → 4/4 · built by S06 (#1862)

> **Designer:** (3) Radix, Unit, Rare, Stinkyscape, #81 · [2/2] / Starship Piece / Cry: Discover 2 Rare Weapons; Equip them into this ~~~ [4/4] / Starship Piece / Cry: Discover 2 Epic Weapons; Equip them into this

- **Text:** Starship Piece / Cry: Discover 2 Rare Weapons and Equip them into this.
- **Radiant:** Starship Piece / Cry: Discover 2 Epic Weapons and Equip them into this.
- **Engine:** Starship Piece: its death fuses a copy into its owner's Dormant Starship, summoned first if they have none ([[R1852000]]). Cry ([[R1]]): `equip_picks`, 2 Discovers of three in turn ([[R1841002]]) from the Weapon-tagged cards of that rarity (Rare: S #50 to S #53, S #140 and S #141; Epic: S #54 to S #57, S #142 and S #143), the pool naming the set so it reaches them while the set is held ([[R1420]]), each pick a fresh copy fused into it with its Activate ([[R1751000]]), no Cry, no hand, never a Weapon it holds.
- **Rulings:** ST-S06-3: "Rare Weapons" are the Weapon-tagged cards of that rarity (the tag is F's, R1857000); "Equip them into this" is a fusion per pick, each Discover asked once the last pick is in. These are proposals for the S06 part, which takes R-numbers from its own block.
- **Numbers:** none (the Discovers are the frame).
- **Check:** The designer's "Equip them into this" kept. R275: stats doubled and a rarity higher.
- **Class:** C

#### Stinkyscape #82 · Phalanx
`stinkyscape-082` · (4) Unit · Epic · 4/4 → 8/8 · built by S06 (#1862)

> **Designer:** (4) Phalanx, Unit, Epic, Stinkyscape, #82 · [4/4] / Starship Piece / Armor 3 / Cry: Discover 2 Rare Weapons; Equip them into this ~~~ [8/8] / Starship Piece / Armor 6 / Cry: Discover 2 Epic Weapons; Equip them into this

- **Text:** Starship Piece, Armor 3 / Cry: Discover 2 Rare Weapons and Equip them into this.
- **Radiant:** Starship Piece, Armor 6 / Cry: Discover 2 Epic Weapons and Equip them into this.
- **Engine:** Starship Piece: its death fuses a copy into its owner's Dormant Starship, summoned first if they have none ([[R1852000]]). Cry ([[R1]]): `equip_picks`, 2 Discovers of three in turn ([[R1841002]]) from the Weapon-tagged cards of that rarity (Rare: S #50 to S #53, S #140 and S #141; Epic: S #54 to S #57, S #142 and S #143), the pool naming the set so it reaches them while the set is held ([[R1420]]), each pick a fresh copy fused into it with its Activate ([[R1751000]]), no Cry, no hand, never a Weapon it holds.
- **Rulings:** ST-S06-3 (as at S #81). These are proposals for the S06 part, which takes R-numbers from its own block.
- **Numbers:** none (the Discovers are the frame).
- **Check:** The designer's "Equip them into this" kept. R275: stats doubled and a rarity higher.
- **Class:** C

#### Stinkyscape #83 · Halo
`stinkyscape-083` · (4) Unit · Epic · 2/2 → 4/4 · built by S06 (#1862)

> **Designer:** (4) Halo, Unit, Epic, Stinkyscape, #83 · [2/2] / Starship Piece / Cry: Discover 3 Rare Weapons; Equip them into this ~~~ [4/4] / Starship Piece / Cry: Discover 3 Epic Weapons; Equip them into this

- **Text:** Starship Piece / Cry: Discover 3 Rare Weapons and Equip them into this.
- **Radiant:** Starship Piece / Cry: Discover 3 Epic Weapons and Equip them into this.
- **Engine:** Starship Piece: its death fuses a copy into its owner's Dormant Starship, summoned first if they have none ([[R1852000]]). Cry ([[R1]]): `equip_picks`, 3 Discovers of three in turn ([[R1841002]]) from the Weapon-tagged cards of that rarity (Rare: S #50 to S #53, S #140 and S #141; Epic: S #54 to S #57, S #142 and S #143), the pool naming the set so it reaches them while the set is held ([[R1420]]), each pick a fresh copy fused into it with its Activate ([[R1751000]]), no Cry, no hand, never a Weapon it holds.
- **Rulings:** ST-S06-3 (as at S #81). These are proposals for the S06 part, which takes R-numbers from its own block.
- **Numbers:** none (the Discovers are the frame).
- **Check:** The designer's "Equip them into this" kept. R275: stats doubled and a rarity higher.
- **Class:** C

#### Stinkyscape #84 · Power Cell
`stinkyscape-084` · (1) Spell · Common · built by S04 (#1861)

> **Designer:** (1) Power Cell, Spell, Common, Stinkyscape, #84 · Shuffle 4 Metal Scraps into your deck. ~~~ Shuffle 8 Metal Scraps into your deck.

- **Text:** Shuffle {scraps} Metal Scraps into your deck.
- **Radiant:** Shuffle {scraps} Metal Scraps into your deck.
- **Engine:** `shuffle_into` of Metal Scraps (S #84.1) into its caster's deck, each at a random position ([[R311]]), a full deck refusing the rest ([[R80]]).
- **Rulings:** none new: every reading is a ruling on `main` or the card's text.
- **Numbers:** scraps 4 ↑ (Radiant 8).
- **Check:** R275: 8 Metal Scraps against 4 (2×).
- **Class:** A.

#### Stinkyscape #84.1 · Metal Scraps
`stinkyscape-084-1` · (1) Spell, Token · Token (printed Common) · built by S04 (#1861)

> **Designer:** (1) Metal Scraps, Spell, Token, Common, Stinkyscape, #84.1 · Cast on Draw: Gain a Stinkyscape resource. ~~~ Cast on Draw: Gain a Radiant Stinkyscape resource.

- **Text:** Cast on draw: Gain a random Stinkyscape Resource.
- **Radiant:** Cast on draw: Gain a random Radiant Stinkyscape Resource.
- **Engine:** Cast on draw ([[§6.2]]): cast as it is drawn, free and counted as played ([[R70]]), its owner then drawing again under [[R58]]'s chain cap; set aside through setup ([[R635]]). Its cast is `gain_resources` of one random Resource into the tray ([[R1840000]], [[R1840002]]); a Spell token, it then goes to the graveyard. S #84 Power Cell shuffles it in. Radiant: a Radiant Resource.
- **Rulings:** none new: every reading is a ruling on `main` or the card's text.
- **Numbers:** none (one Resource; its (1) is never paid, a cast on draw).
- **Check:** "Cast on Draw" is the label Cast on draw ([[R366]]); "Gain a Stinkyscape resource" is a random one ([[R1840002]]). R275: a Radiant Resource.
- **Class:** C.

#### Stinkyscape #85 · Prototype-X1
`stinkyscape-085` · (4) Unit · Legendary · 6/6 → 12/12 · built by S06 (#1862)

> **Designer:** (4) Prototype-X1, Unit, Legendary, Stinkyscape, #85 · Tribute 2 / Starship Piece / [6/6] / Cry: Discover 2 Epic Weapons; Equip them into this ~~~ Tribute 2 / Starship Piece / [12/12] / Cry: Discover 2 Legendary Weapons; Equip them into this

- **Text:** Tribute 2, Starship Piece / Cry: Discover 2 Epic Weapons and Equip them into this.
- **Radiant:** Tribute 2, Starship Piece / Cry: Discover 2 Legendary Weapons and Equip them into this.
- **Engine:** Tribute 2 ([[§6.3]] Tribute, [[R101]]), printed in the keyword line ([[R366]]), the script's `tribute`. Starship Piece: its death fuses a copy into its owner's Dormant Starship, summoned first if they have none ([[R1852000]]). Cry ([[R1]]): `equip_picks`, 2 Discovers of three in turn ([[R1841002]]) from the Weapon-tagged cards of that rarity (Epic: S #54 to S #57, S #142 and S #143; Legendary: S #58 to S #61), the pool naming the set so it reaches them while the set is held ([[R1420]]), each pick a fresh copy fused into it with its Activate ([[R1751000]]), no Cry, no hand, never a Weapon it holds.
- **Rulings:** ST-S06-3 (as at S #81). These are proposals for the S06 part, which takes R-numbers from its own block.
- **Numbers:** none (the Discovers are the frame; Tribute is no param, R482).
- **Check:** The designer's "Equip them into this" kept. R275: stats doubled and a rarity higher.
- **Class:** C

#### Stinkyscape #86 · Prototype-X2
`stinkyscape-086` · (4) Unit · Mythic · 8/8 → 16/16 · built by S06 (#1862)

> **Designer:** (4) Prototype-X2, Unit, Mythic, Stinkyscape, #86 · Float reduces Tribute requirement by 1 / Tribute 5 / Starship Piece / [8/8] / Cry: Discover 3 Epic Weapons; Equip them into this ~~~ Float reduces Tribute requirement by 1 / Tribute 5 / Starship Piece / [8/8] / Cry: Discover 3 Legendary Weapons; Equip them into this

- **Text:** Tribute 5, Starship Piece / Float reduces Tribute requirement by 1. / Cry: Discover 3 Epic Weapons and Equip them into this.
- **Radiant:** Tribute 5, Starship Piece / Float reduces Tribute requirement by 1. / Cry: Discover 3 Legendary Weapons and Equip them into this.
- **Engine:** Tribute 5 ([[§6.3]] Tribute, [[R101]]), printed in the keyword line ([[R366]]), the script's `tribute`, lowered 1 by each of its owner's Floats while it waits in hand or deck (`float_lowers_tribute`, [[R1841003]]), never below 0, kept in `memory.tributeLowered`. Starship Piece: its death fuses a copy into its owner's Dormant Starship, summoned first if they have none ([[R1852000]]). Cry ([[R1]]): `equip_picks`, 3 Discovers of three in turn ([[R1841002]]) from the Weapon-tagged cards of that rarity (Epic: S #54 to S #57, S #142 and S #143; Legendary: S #58 to S #61), the pool naming the set so it reaches them while the set is held ([[R1420]]), each pick a fresh copy fused into it with its Activate ([[R1751000]]), no Cry, no hand, never a Weapon it holds.
- **Rulings:** ST-S06-3 (as at S #81). ST-S06-4: Float lowers the Tribute 1 per Float whatever the Float's size (R1841003). These are proposals for the S06 part, which takes R-numbers from its own block.
- **Numbers:** none (the Discovers are the frame; Tribute is no param, R482).
- **Check:** ST8 and ST9: Radiant 16/16, not 8/8 (R275). The designer's "Equip them into this" kept. R275: stats doubled and a rarity higher.
- **Class:** C

#### Stinkyscape #87 · Siva 1
`stinkyscape-087` · (2) Unit, Felinor, Acclaimed · Legendary · 2/2 → 4/4 · built by S08 (#1864)

> **Designer:** (2) Siva 1, Unit, Felinor, Acclaimed, Legendary, Stinkyscape, #87 · 2/2 / Tradable / Cry: Allure a random enemy Acclaimed Unit. ~~~ 4/4 / Tradable / Cry: Allure a random enemy Acclaimed Unit. If there isn't one, summon a Frozen one to Allure.

- **Text:** Tradable / Cry: Allure a random enemy Acclaimed Unit.
- **Radiant:** Tradable / Cry: Allure a random enemy Acclaimed Unit. If there isn't one, summon a random Acclaimed Unit for your opponent, Frozen, and Allure it.
- **Engine:** Tradable ([[§6.1]], [[R1838000]]): one Trade a turn from the hand. Cry ([[R1]]): one random pick ([[R60]]) among the enemy's acting Units whose tags, granted ones included, hold Acclaimed, then K-Pop Fanatic's Allure ([[R1045000]], [[R963]]): the public `allure` mark ([[R437]]) and, at the start of its controller's next turn, a steal into the same row ([[R15]]), destroyed with no room ([[R46]]); with none, nothing ([[R129]]). Radiant: with none, a random non-token Acclaimed Unit of the shipped sets ([[R380]]), never Siva 1 ([[R387]]), is summoned onto the opponent's side ([[R64]]), Frozen ([[§6.3]] Freeze, [[R1174000]]), and Allured; a full enemy unit row summons nothing.
- **Rulings:** ST-S08-1: "a random enemy Acclaimed Unit" is one of the enemy's acting Units with the tag, a granted tag included; an Allure lands as K-Pop Fanatic's does ([[R1045000]]). ST-S08-2: "summon a Frozen one to Allure" summons a random Acclaimed Unit (not Siva 1, [[R387]]) on the opponent's side, Frozen so it cannot attack before the steal, then Allures it; nothing when their unit row is full.
- **Numbers:** none (none printed).
- **Check:** R275: stats doubled; the Radiant face Allures even when the opponent has no Acclaimed Unit. ⚠ designer: the Frozen summon lands on the opponent's side, the only side an Allure reaches.
- **Class:** B.

#### Stinkyscape #88 · Siva 2
`stinkyscape-088` · (3) Unit, Human, Acclaimed · Legendary · 4/4 → 8/8 · built by S17 (#1876)

> **Designer:** (3) Siva 2, Unit, Human, Acclaimed, Legendary, Stinkyscape, #88 · 4/4 / Aura: Your Humans have Poisonous. ~~~ 8/8 / Aura: Your Humans have Poisonous. / Cry: Fill your hand with Bigots.

- **Text:** Aura: Your Humans have Poisonous.
- **Radiant:** Aura: Your Humans have Poisonous. / Cry: Fill your hand with Bigots.
- **Engine:** Aura ([[§10.4]]): every Human Unit its controller has on the field (granted tags read, [[R923]]), this one included, has Poisonous. Radiant: Cry ([[R1]]): Core #2 Bigots added while the hand has room ([[§2.4]], M+ #24's fill, [[R1185004]]).
- **Rulings:** ST-S17-7: "Your Humans" are your Human Units on the field, this one included. ST-S17-8: "Fill your hand" adds Bigots until the hand is full, none burned. These are proposals for S17, which takes R-numbers from its own block. No R-number here.
- **Numbers:** none (the fill is the hand's room).
- **Check:** R275: stats doubled and a hand of Bigots.
- **Class:** B (an aura and a fill).

#### Stinkyscape #89 · Spice
`stinkyscape-089` · (1) Spell · Rare · built by S04 (#1861)

> **Designer:** (1) Spice, Spell, Rare, Stinkyscape, #89 · Can be sold on the Resource marketplace. {For AI: Its price is determined like all other Resources; only appearing once a Spice is potentially relevant} / Discover a Stinkyscape card. ~~~ Can be sold on the Resource marketplace. / Discover a Radiant Stinkyscape card.

- **Text:** Can be sold from your hand at the Marketplace, like a Resource. / Discover a Stinkyscape card.
- **Radiant:** Can be sold from your hand at the Marketplace, like a Resource. / Discover a Radiant Stinkyscape card.
- **Engine:** NEW (S04): a sale from the hand. While #1853's Marketplace (pending PR #1963) is open, its owner may sell it from their hand in their main phase for Credits ([[R1840005]]), a sale as public as a Resource's; #1853's sale takes tray Resources only, and the list prints no sell range ("its price is determined like all other Resources"), so S04 sets it, with a catalog field only if it must (a shared surface). Discover ([[§6.3]]) a non-token Stinkyscape card: the pool names the set, so the held gate does not empty it ([[R380]], [[R1420]]), and it is never Spice ([[R387]]). Radiant: the pick is Radiant.
- **Rulings:** ST-S04-6: "Can be sold on the Resource marketplace" is a sale from its owner's hand while the Marketplace is open, in their main phase, for Credits, public as a Resource's sale is; Spice stays a hand card, never a Resource, so no tray rule reaches it. ST-S04-7: the designer's "only appearing once a Spice is potentially relevant" (for the AI) is S04's: its sale is offered only while the Marketplace is open.
- **Numbers:** none (none printed).
- **Check:** ⚠ designer: no sell range is printed (QUESTION in the part's notes). R275: a Radiant Discover.
- **Class:** C.

#### Stinkyscape #90 · After Burner
`stinkyscape-090` · (2) Unit · Common · 4/8 → 8/16 · built by S02 (#1859)

> **Designer:** (2) After Burner, Unit, Common, Stinkyscape, #90 · 4/8 / Starship Piece / When launched, gain 2 random Combat Keywords. ~~~ 8/16 / Starship Piece / When launched, gain 4 random Combat Keywords.

- **Text:** Starship Piece / When launched, your Starship gains {keywords|random combat keyword|random combat keywords}.
- **Radiant:** Starship Piece / When launched, your Starship gains {keywords|random combat keyword|random combat keywords}.
- **Engine:** Starship Piece: its death fuses a copy into its owner's Dormant Starship, summoned first if they have none ([[R1852000]]); on the field its text does nothing. When launched ([[§6.2]], `when_launched`, [[R1852003]]): only fused into a Starship, after its fused Cries, in the order the pieces fused; `grant_random_keywords` of the combat keyword pool ([[R21]], [[R781005]]) onto that Starship, keywords it lacks.
- **Rulings:** ST-S02-1: "gain" is the Starship it is fused into, the only card its "When launched" runs on; the keywords are different ones it lacks, fewer when fewer are left. These are proposals for the S02 part, which takes R-numbers from its own block.
- **Numbers:** keywords 2 ↑ (Radiant 4).
- **Check:** "Combat Keywords" is the combat keyword pool's (§6.1). R275: stats and keywords doubled.
- **Class:** C

#### Stinkyscape #91 · The Propagation's Wasp
`stinkyscape-091` · (1) Unit, Swarm · Common · 3/3 → 6/6 · built by S16 (#1869)

> **Designer:** (1) The Propagation's Wasp, Swarm, Common, Stinkyscape, #91 · [3/3] / After these attacks, summon a copy of this, then evolve. / Evolve: gain Flying ~~~ [6/6] / After these attacks, summon a copy of this, then evolve. / Evolve: gain Flying

- **Text:** After this attacks, summon a copy of this, then evolve. / Evolve: Gain Flying.
- **Radiant:** After this attacks, summon a copy of this, then evolve. / Evolve: Gain Flying.
- **Engine:** `after_attack` ([[R1849001]]): once the state check closing each of its combats has run, `summon_copy` of it ([[R57]], [[R64]], no Cry, [[R1]]), made from it before it evolves, then `evolve` on it: its `Script.evolve` hook, Gain Flying (`grant_keyword`). The copy has no Flying and the original has it; one that died attacking leaves its copy and evolves into nothing. Forced attacks count too.
- **Rulings:** ST-S16-9: "After these attacks" is "After this attacks" (R1849001); a forced attack is an attack, so it copies too. These are proposals for S16, which takes R-numbers from its own block. No R-number here.
- **Numbers:** none.
- **Check:** Typo "these" read as this. ST1/ST9: Swarm is a tag on Units. ⚠ designer: both faces print one text; R275 by the doubled stats (each copy twice as big).
- **Class:** C (M15's Evolve and attack summon).

#### Stinkyscape #92 · The Propagation's Hornet
`stinkyscape-092` · (2) Unit, Swarm · Rare · 4/4 → 8/8 · built by S16 (#1869)

> **Designer:** (2) The Propagation's Hornet, Swarm, Rare, Stinkyscape, #92 · [4/4] / Rush / After these attacks, summon a copy of this, then evolve. / Evolve: gain Flying ~~~ [8/8] / Rush / After these attacks, summon a copy of this, then evolve. / Evolve: gain Flying

- **Text:** Rush / After this attacks, summon a copy of this, then evolve. / Evolve: Gain Flying.
- **Radiant:** Rush / After this attacks, summon a copy of this, then evolve. / Evolve: Gain Flying.
- **Engine:** Rush. `after_attack` ([[R1849001]]): once the state check closing each of its combats has run, `summon_copy` of it ([[R57]], [[R64]], no Cry, [[R1]]), made from it before it evolves, then `evolve` on it: its `Script.evolve` hook, Gain Flying (`grant_keyword`). The copy has no Flying and the original has it; one that died attacking leaves its copy and evolves into nothing. Forced attacks count too.
- **Rulings:** ST-S16-10: as #91; the copy has Rush too (a copy of the card), so it may attack that turn and copy again. These are proposals for S16, which takes R-numbers from its own block. No R-number here.
- **Numbers:** none.
- **Check:** Typo "these" read as this. ST1/ST9: Swarm is a tag on Units. ⚠ designer: both faces print one text; R275 by the doubled stats (each copy twice as big). Rush as printed.
- **Class:** C (M15's Evolve and attack summon).

#### Stinkyscape #93 · Patcher Nanobots
`stinkyscape-093` · (2) Unit · Common · 3/4 → 6/8 · built by S03 (#1860)

> **Designer:** (2) Patcher Nanobots, Unit, Common, Stinkyscape, #93 · 3/4 / Starship Piece / Taunt / Death: Heal your hero 6\. ~~~ 6/8 / Starship Piece / Taunt / Death: Heal your hero 12\.

- **Text:** Starship Piece, Taunt / Death: Heal your hero {heal}.
- **Radiant:** Starship Piece, Taunt / Death: Heal your hero {heal}.
- **Engine:** Starship Piece: its death fuses a copy into its owner's Dormant Starship, summoned first if they have none ([[R1852000]]). Death ([[§6.2]]): `heal` on its controller's hero as it died ([[R89]]), no cap on a hero ([[§6.3]] Heal).
- **Rulings:** none new.
- **Numbers:** heal 6 ↑ (Radiant 12, step 2).
- **Check:** The designer's "\." escape dropped. R275: stats and heal doubled.
- **Class:** A

#### Stinkyscape #94 · Tmack Trav
`stinkyscape-094` · (3) Unit, Human · Epic · 9/9 → 18/18 · built by S16 (#1869)

> **Designer:** (3) Tmack Trav, Unit, Epic, Human, Stinkyscape, #94 · 9/9 / Rush / After this kills a unit, this may attack again. ~~~ 18/18 / Rush, Pierce / After this kills a unit, this may attack again.

- **Text:** Rush / After this destroys a Unit, it may attack again.
- **Radiant:** Rush, Pierce / After this destroys a Unit, it may attack again.
- **Engine:** Rush. `after_attack` ([[§4.2]]): when the combat's `destroyedIds` holds the Unit it attacked, `may_attack_again` (B5 E35, C+ #73.1 Classic Golem's): a fresh exertion this turn, the rest of [[§4.2]]'s restrictions still applying. Radiant: Pierce ([[R346]]).
- **Rulings:** ST-S16-11: "kills a unit" is the Unit it attacked destroyed in that combat; a kill by its strike back as a defender does not count. Each kill grants one more attack, with no cap. These are proposals for S16, which takes R-numbers from its own block. No R-number here.
- **Numbers:** none.
- **Check:** ST9: Tmack Trav (#94) and Tnack Trav (#20) are two cards. R275: stats doubled and Pierce.
- **Class:** A (one verb).

#### Stinkyscape #95 · Vanguard Coalition
`stinkyscape-095` · (3) Spell · Rare · built by S24 (#1880)

> **Designer:** (3) Vanguard Coalition, Spell, Rare, Stinkyscape, #95 · Add a Scornboi, Ocean Tsunami, Tylannt, Vxyl, Siva 1, and Siva 2 to your hand ~~~ Add a Radiant Scornboi, Ocean Tsunami, Tylannt, Siva 1, and Siva 2 to your hand

- **Text:** Add a Scornboi, Ocean Tsunami, Tylannt, Vxyl, Siva 1 and Siva 2 to your hand.
- **Radiant:** Add a Radiant Scornboi, Ocean Tsunami, Tylannt, Vxyl, Siva 1 and Siva 2 to your hand.
- **Engine:** Add to hand ([[§6.3]]): a new card of each, in the order printed, owned by its caster; the Radiant face adds each Radiant ([[§5.2]]). A full hand burns the rest, in that order ([[§2.4]]). The cards are named, so they reach the hand while the set is held ([[R1420]]).
- **Rulings:** ST-S24-1: "a Radiant X, Y and Z" makes every card listed Radiant, as C+ #42.1 KY's Gift's list does; each card is added on its own normal face and price.
- **Numbers:** none (the digits are card names).
- **Check:** ST8: the Radiant face keeps Vxyl. R275: every card Radiant (each about 2×).
- **Class:** A.

#### Stinkyscape #96 · Ocean Tsunami
`stinkyscape-096` · (3) Unit, Human · Rare · 6/6 → 12/12 · built by S04 (#1861)

> **Designer:** (3) Ocean Tsunami, Unit, Human, Rare, Stinkyscape, #96 · [6/6] / Cry: discard your hand then fill your hand with random Stinkyscape resources ~~~ [12/12] / Cry: discard your hand then fill your hand with random Radiant Stinkyscape resources

- **Text:** Cry: Discard your hand. Fill your Resource tray with random Stinkyscape Resources.
- **Radiant:** Cry: Discard your hand. Fill your Resource tray with random Radiant Stinkyscape Resources.
- **Engine:** Cry ([[R1]]): a discard of its controller's whole hand ([[§6.3]] Discard), then `gain_resources { fill }` ([[R1840004]]): as many random Resources as the tray has room for, so none is sold; the tray is no hand, so the discard never reaches it ([[R1840000]]). Radiant: Radiant Resources.
- **Rulings:** ST-S04-8: "fill your hand with random Stinkyscape resources" fills the Resource tray ([[R1840004]]); "discard your hand" discards every hand card and no Resource ([[R1840000]]).
- **Numbers:** none (a fill has no count).
- **Check:** "fill your hand" is the tray (ST3, [[R1840004]]). R275: stats doubled; Radiant Resources.
- **Class:** C.

#### Stinkyscape #97 · Asteroid Field
`stinkyscape-097` · (2) Spell · Common · built by S04 (#1861)

> **Designer:** (2) Asteroid Field, Spell, Common, Stinkyscape, #97 · Cast on draw: deal 2 damage to all heroes and units, add two random Stinkyscape resources to your hand ~~~ Cast on draw: deal 2 damage to all enemy heroes and units, add two random Radiant Stinkyscape resources to your hand

- **Text:** Cast on draw: Deal {damage} damage to both heroes and all Units. Gain {resources|random Stinkyscape Resource|random Stinkyscape Resources}.
- **Radiant:** Cast on draw: Deal {damage} damage to all enemies. Gain {resources|random Radiant Stinkyscape Resource|random Radiant Stinkyscape Resources}.
- **Engine:** Cast on draw ([[§6.2]], [[R635]], [[R70]]). One hit on each hero and every Unit of both sides, one state check ([[R59]]), its caster's Spell Damage raising each ([[§4.4]] step 0); then `gain_resources` of two random Resources into the tray ([[R1840000]]). Radiant: the enemy hero and every enemy Unit only, and Radiant Resources.
- **Rulings:** ST-S04-9: "all heroes and units" is both heroes and every Unit, one hit each; the Radiant "all enemy heroes and units" is the enemy hero and every enemy Unit; "add … to your hand" is a gain into the tray ([[R1840000]]).
- **Numbers:** damage 2 ↑; resources 2 ↑.
- **Check:** "add two random … to your hand" is a gain into the tray ([[R1840000]]). R275: a one-sided hit and Radiant Resources.
- **Class:** C.

#### Stinkyscape #98 · Hull Booster
`stinkyscape-098` · (1) Unit · Common · 1/1 → 2/2 · built by S03 (#1860)

> **Designer:** (1) Hull Booster, Unit, Common, Stinkyscape, #98 · 1/1 / Starship Piece / Equip / Armor 2 ~~~ 2/2 / Starship Piece / Equip / Armor 4

- **Text:** Starship Piece, Equip, Armor 2
- **Radiant:** Starship Piece, Equip, Armor 4
- **Engine:** Keywords only ([[§6.1]]): its script is empty. Equip ([[R1086]]): played onto one of your Units it fuses into it, the host keeping its zone and taking its stats, keywords and text ([[R77]], [[R102]]), Starship Piece only into a piece ([[R1852001]]); played alone its death fuses a copy into its owner's Dormant Starship ([[R1852000]]).
- **Rulings:** none new.
- **Numbers:** none (keywords and stats).
- **Check:** R275: stats and Armor doubled.
- **Class:** A

#### Stinkyscape #99 · No-No Words
`stinkyscape-099` · (X) Spell · Common · built by S14 (#1868)

> **Designer:** (X) No-No Words, Spell, Common, Stinkyscape, #99 · Destroy a unit with less than [4X] attack ~~~ Destroy a unit with less than [4X] attack, Cleave

- **Text:** Destroy a Unit with less than {per}X Attack.
- **Radiant:** Destroy a Unit with less than {per}X Attack, and the Units adjacent to it.
- **Engine:** An X Spell ([[R65]], [[R348]]): its player chooses X as they play it, or it has its set X ([[R1848003]], S #102's copies). A declared target, a Unit whose Attack, read through the layers ([[§10.4]]), is below 4 × X as it is chosen and as it resolves ([[R81]]); `destroy`, Indestructible ignoring it. Radiant: `destroy_adjacent_to` as well, the Units beside it whatever their Attack ([[R59]]). A random cast takes its caster's mana as X, at least 1 ([[R348]]).
- **Rulings:** ST-S14-4: "less than [4X]" is strictly below 4 × X, so an X of 1 destroys a Unit of 3 Attack or less; a Nerf's or a Buff's X step moves X, never below 1 ([[R386]]).
- **Numbers:** per 4 ↑ (the threshold's multiple of X).
- **Check:** R275: its neighbours too, as S #12.1's Radiant.
- **Class:** B.

#### Stinkyscape #100 · AP Rounds
`stinkyscape-100` · (1) Unit · Common · 3/1 → 6/2 · built by S03 (#1860)

> **Designer:** (1) AP Rounds, Unit, Common, Stinkyscape, #100 · 3/1 / Starship Piece / Equip / Pierce ~~~ 6/2 / Starship Piece / Equip / Pierce

- **Text:** Starship Piece, Equip, Pierce
- **Radiant:** Starship Piece, Equip, Pierce
- **Engine:** Keywords only ([[§6.1]]): its script is empty. Equip ([[R1086]]): played onto one of your Units it fuses into it, the host keeping its zone and taking its stats, keywords and text ([[R77]], [[R102]]), Starship Piece only into a piece ([[R1852001]]); played alone its death fuses a copy into its owner's Dormant Starship ([[R1852000]]).
- **Rulings:** none new.
- **Numbers:** none (keywords and stats).
- **Check:** R275: as an Equip its stats are its effect, doubled.
- **Class:** A

#### Stinkyscape #101 · Overdrive Module
`stinkyscape-101` · (2) Unit · Rare · 5/5 → 10/10 · built by S03 (#1860)

> **Designer:** (2) Overdrive Module, Unit, Rare, Stinkyscape, #101 · 5/5 / Starship Piece / Equip / Brittle 2 / Reborn ~~~ 10/10 / Starship Piece / Equip / Brittle 4 / Reborn

- **Text:** Starship Piece, Equip, Brittle 2, Reborn
- **Radiant:** Starship Piece, Equip, Brittle 4, Reborn
- **Engine:** Keywords only ([[§6.1]]): its script is empty. Equip ([[R1086]]): played onto one of your Units it fuses into it, the host keeping its zone and taking its stats, keywords and text ([[R77]], [[R102]]), Starship Piece only into a piece ([[R1852001]]); played alone its death fuses a copy into its owner's Dormant Starship ([[R1852000]]).
- **Rulings:** none new.
- **Numbers:** none (keywords and stats).
- **Check:** R275: stats doubled; a longer Brittle is the lighter drawback.
- **Class:** A

#### Stinkyscape #102 · Book of Nono-Words
`stinkyscape-102` · (1) Spell, Book · Epic · built by S14 (#1868)

> **Designer:** (1) Book of Nono-Words, Spell, Epic, Book, Stinkyscape, #102 · Add 4 copies of No-No Words to your hand, their prices are set at 1,2,3,4 ~~~ Add 4 copies of Radiant No-No Words to your hand, their prices are set at 1,2,3,4

- **Text:** Add 4 No-No Words to your hand, their X set at 1, 2, 3 and 4.
- **Radiant:** Add 4 Radiant No-No Words to your hand, their X set at 1, 2, 3 and 4.
- **Engine:** Four `add_to_hand { defId: stinkyscape-099, fixedX }`, set at 1, 2, 3 and 4 in that order ([[R1848003]]): each is played for its set X, with no X to choose, and costs exactly it, no modifier reaching it ([[R65]]), its face showing the number; a full hand burns the rest, without it ([[§2.4]]); a Bounce keeps it and a graveyard takes it off ([[R766]]). Radiant: each `radiant: true`.
- **Rulings:** none new ([[R1848003]]).
- **Numbers:** none (the four set X are the card's list, [[R1748003]]).
- **Check:** "their prices are set at 1,2,3,4" is Set X ([[§6.3]]). R275: four Radiant copies, each with its Cleave.
- **Class:** C (M14's Set X).

#### Stinkyscape #103 · Vxyl
`stinkyscape-103` · (2) Unit, Human · Rare · 1/4 → 2/8 · built by S14 (#1868)

> **Designer:** (2) Vxyl, Unit, Human, Rare, Stinkyscape, #103 · Cloaked / [1/4] / Poisonous / On reveal: Add a No-No Words to your hand ~~~ Cloaked / [2/8] / Poisonous, Divine Shield / On reveal: Add a No-No Words to your hand

- **Text:** Cloaked, Poisonous / Cry: Add a No-No Words to your hand.
- **Radiant:** Cloaked, Poisonous, Divine Shield / Cry: Add a No-No Words to your hand.
- **Engine:** Cloaked ([[§6.1]], [[R783004]]–[[R783006]]): it may be set face-down at its own (2) as a Trap and revealed by its Activate: Reveal in its controller's main phase, which takes it into a unit zone and runs its Cry. Its Cry: `add_to_hand { defId: stinkyscape-099 }`, an X card whose X its player chooses as they play it ([[R348]]). Poisonous ([[§4.4]] step 7). Radiant: Divine Shield too.
- **Rulings:** ST-S14-5: "On reveal" is its Cry, which a Cloaked Unit's reveal runs ([[R783006]]), so a face-up play runs it too; a reveal-only trigger would be NEW, S14's call.
- **Numbers:** none (the card prints no number).
- **Check:** R275: 2/8 and Divine Shield, a keyword more.
- **Class:** A.

#### Stinkyscape #104 · Reflective Plating
`stinkyscape-104` · (2) Unit · Rare · 0/6 → 0/12 · built by S16 (#1869)

> **Designer:** (2) Reflective Plating, Unit, Rare, Stinkyscape, #104 · 0/6 / Starship Piece / Equip / Any damage this takes is dealt to the enemy hero. ~~~ 0/12 / Starship Piece / Equip / Any damage this takes is dealt to the enemy hero and the attacker.

- **Text:** Starship Piece, Equip / Any damage this takes is dealt to the enemy hero.
- **Radiant:** Starship Piece, Equip / Any damage this takes is dealt to the enemy hero and the attacker.
- **Engine:** Starship Piece ([[R1852000]]), Equip ([[R1086]]; it passes no Starship Piece to its host, [[R1852001]]). `staticFlags.reflectsDamage` ([[R1849003]]): what a hit dealt it is dealt again from it at [[§4.4]] step 8a to its controller's opponent's hero, a hit that dealt nothing reflecting nothing. Radiant: `reflectsToAttacker`, also to the attacker whose strike it was, while that attacker is on the field.
- **Rulings:** ST-S16-12: the Plating still takes the damage; the hero takes it again (R1849003). Equipped, the host reflects, its scripts kept by the Fuse (R77). These are proposals for S16, which takes R-numbers from its own block. No R-number here.
- **Numbers:** none.
- **Check:** R275: health doubled (0 Attack stays 0) and the attacker hit too.
- **Class:** C (M15's reflected damage).

#### Stinkyscape #105 · Disintegrex
`stinkyscape-105` · (2) Unit · Epic · 6/3 → 12/6 · built by S16 (#1869)

> **Designer:** (2) Disintegrex, Unit, Epic, Stinkyscape, #105 · 6/3 / Starship Piece / Aura: Anytime something would be Destroyed, it is Exiled instead. ~~~ 12/6 / Starship Piece / Aura: Anytime an enemy permanent would be Destroyed, it is Exiled instead.

- **Text:** Starship Piece / Aura: Anything that would be destroyed is exiled instead.
- **Radiant:** Starship Piece / Aura: Any enemy permanent that would be destroyed is exiled instead.
- **Engine:** Starship Piece ([[R1852000]]). `staticFlags.destroyedExiled` ([[R1849002]]): while it acts on the field, every card [[§4.5]] step 1 collects that no would-die replacement took is exiled instead: no Death, no Reborn, no `destroyed` ([[R461]]), this card too when it dies in the same pass, so it fuses into no Starship. Radiant: `enemyDestroyedExiled`, its controller's opponent's cards only, its own side destroyed as usual.
- **Rulings:** ST-S16-13: "Destroyed" means collected by the state check; a Tribute, a discard and a spent Trap still reach the graveyard (R1849002). An exiled Starship Piece fuses into nothing (R1852000: only a death fuses). These are proposals for S16, which takes R-numbers from its own block. No R-number here.
- **Numbers:** none.
- **Check:** R275: stats doubled and the aura spares your own side.
- **Class:** C (M15's destroy becomes exile).

#### Stinkyscape #106 · Warpfield Amplifier
`stinkyscape-106` · (1) Unit · Rare · 2/3 → 4/6 · built by S02 (#1859)

> **Designer:** (1) Warpfield Amplifier, Unit, Rare, Stinkyscape, #106 · 2/3 / Starship Piece / When launched, set all of your Units to attack position and give them +2 attack. ~~~ 4/6 / Starship Piece / When launched, set all of your Units to attack position and give them +4 attack.

- **Text:** Starship Piece / When launched, switch each of your Units to Attack Position and give it +{attackGain} Attack.
- **Radiant:** Starship Piece / When launched, switch each of your Units to Attack Position and give it +{attackGain} Attack.
- **Engine:** Starship Piece: its death fuses a copy into its owner's Dormant Starship, summoned first if they have none ([[R1852000]]). When launched ([[R1852003]]): `switch_all_positions` to Attack for its controller's Units in Defense Position, an effect's switch spending no exertion ([[R20]]), then `buff` +N Attack on each of their Units, the Starship included, lasting.
- **Rulings:** ST-S02-2: "all of your Units" are every Unit its controller has as the Starship launches, the Starship included, and the +2 Attack lasts (it is no this-turn buff). These are proposals for the S02 part, which takes R-numbers from its own block.
- **Numbers:** attackGain 2 ↑ (Radiant 4).
- **Check:** "attack position" is Attack Position (§4.1). R275: stats and the gain doubled.
- **Class:** C

#### Stinkyscape #107 · RR-5000
`stinkyscape-107` · (2) Unit · Rare · 4/4 → 8/8 · built by S03 (#1860)

> **Designer:** (2) RR-5000, Unit, Rare, Stinkyscape, #107 · 4/4 / Starship Piece / Ranged / Start of Turn: Gain +4 attack. ~~~ 8/8 / Starship Piece / Ranged / Start of Turn: Gain +8 attack.

- **Text:** Starship Piece, Ranged / Start of turn: Gain +{attackGain} Attack.
- **Radiant:** Starship Piece, Ranged / Start of turn: Gain +{attackGain} Attack.
- **Engine:** Starship Piece: its death fuses a copy into its owner's Dormant Starship, summoned first if they have none ([[R1852000]]). Ranged ([[R783000]]). Start of turn ([[R62]]): `buff` +N Attack on itself, lasting, so it grows every one of its controller's turns.
- **Rulings:** ST-S03-6: "Gain +4 attack" lasts and adds up, a turn at a time; it is no this-turn buff. These are proposals for the S03 part, which takes R-numbers from its own block.
- **Numbers:** attackGain 4 ↑ (Radiant 8).
- **Check:** R275: stats and the gain doubled.
- **Class:** A

#### Stinkyscape #108 · Trila 3
`stinkyscape-108` · (3) Unit · Common · 7/7 → 14/14 · built by S03 (#1860)

> **Designer:** (3) Trila 3, Unit, Common, Stinkyscape, #108 · 7/7 / Starship Piece / Ranged / Windfury ~~~ 14/14 / Starship Piece / Ranged / Rush / Windfury

- **Text:** Starship Piece, Ranged, Windfury
- **Radiant:** Starship Piece, Ranged, Rush, Windfury
- **Engine:** Keywords only ([[§6.1]]): its script is empty. Starship Piece: its death fuses a copy into its owner's Dormant Starship, summoned first if they have none ([[R1852000]]).
- **Rulings:** none new.
- **Numbers:** none (keywords and stats).
- **Check:** R275: stats doubled, Rush the keyword more.
- **Class:** A

#### Stinkyscape #109 · xxFreshJack
`stinkyscape-109` · (4) Unit, Human, Acclaimed · Mythic · 6/11 → 12/22 · built by S18 (#1877)

> **Designer:** (4) xxFreshJack, Unit, Human, Acclaimed, Mythic, Stinkyscape, #109 · 6/11 / Cry: Play a random BUSTED card that exists anywhere in the game's code. {For AI: A busted card is any card that required more than 100 lines of code to implement. This card is also allowed to pull from pools of cards that are normally restricted to single-player only game modes.} ~~~ Cry: Discover and play a Radiant BUSTED card that exists anywhere in the game's code.

- **Text:** Cry: Cast a random BUSTED card from anywhere in the game's code.
- **Radiant:** Cry: Discover a Radiant BUSTED card from anywhere in the game's code and cast it.
- **Engine:** Cry ([[R1]]): one pick from `{ busted: true, everySet: true }` ([[R1846000]], [[R1846001]]): every BUSTED catalog card of the sets that ship and the single-player sets, never this card ([[R387]]), cast as a new card (`cast_new`, [[R70]]), a random cast's choices all random ([[§6.3]] Cast). Radiant: a Discover of three from the same pool, Radiant, the pick cast with its caster's choices.
- **Rulings:** ST-S18-4: "Play" is a Cast (free, a play for every rule, R70); "anywhere in the game's code" is `everySet` (R1846001), single-player sets included once they exist. ST-S18-5: the BUSTED list is the build's (R1846000), so the pool grows with the catalog. These are proposals for S18, which takes R-numbers from its own block. No R-number here.
- **Numbers:** none.
- **Check:** ⚠ designer: no Radiant stats printed: 12/22 (R275 doubled, R1857001). The designer's "For AI" note is M12's (R1846000, R1846001). R275: stats doubled and a chosen, Radiant cast.
- **Class:** C (card metrics, M12).

#### Stinkyscape #110 · iKwzki
`stinkyscape-110` · (2) Unit, Human, Acclaimed · Mythic · 6/6 → 12/12 · built by S17 (#1876)

> **Designer:** (2) iKwzki, Unit, Human, Acclaimed, Mythic, Stinkyscape, #110 · 6/6 / Cry: Duo with a permanent and make it Radiant next turn. {For AI: Reuse the Allure animation but recolor it to be fitting for making something Radiant} ~~~ 12/12 / Cry: Squad with all your permanents and make them Radiant next turn.

- **Text:** Cry: Duo with a permanent and make it Radiant next turn.
- **Radiant:** Cry: Squad with all your permanents and make them Radiant next turn.
- **Engine:** Cry ([[R1]]): `duo_radiant { target }` ([[R1850001]]): a declared permanent on either side, face-down or not, marked with the public gold `duo` mark ([[R437]]) and made Radiant at the start of its marker's next turn (`radiantSet`), not if it has left the field meanwhile ([[R174]]), even after this card has left ([[R76]]). Radiant: `squad_radiant`, every permanent its controller has on the field, unit tops in lane order then the backrow, a Radiant face-up card or a Dormant one passed by.
- **Rulings:** ST-S17-9: "next turn" is the start of your next turn (R1850001). ST-S17-10: Squad does not mark iKwzki itself, which is Radiant already on that face. These are proposals for S17, which takes R-numbers from its own block. No R-number here.
- **Numbers:** none.
- **Check:** The designer's "For AI" note (the Allure animation recoloured) is R1850001's gold mark. R275: stats doubled and every permanent of yours instead of one.
- **Class:** C (M16's Duo and Squad).

#### Stinkyscape #111 · Shockwave447
`stinkyscape-111` · (4) Unit, Human, Acclaimed · Mythic · 4/7 → 8/14 · built by S18 (#1877)

> **Designer:** (4) Shockwave447, Unit, Human, Acclaimed, Mythic, Stinkyscape, #111 · 4/7 / Aura: Has 3 random Combat Keywords. / Cry: Replace both player's hands with complicated cards. ~~~ 8/14 / Aura: Has 3 random Combat Keywords. / Cry: Replace both player's hands with complicated cards. Make yours Radiant.

- **Text:** Aura: Has {keywords|random combat keyword|random combat keywords}. / Cry: Replace both players' hands with random complicated cards.
- **Radiant:** Aura: Has {keywords|random combat keyword|random combat keywords}. / Cry: Replace both players' hands with random complicated cards. Make yours Radiant.
- **Engine:** M #65 Keymaster Keenus's aura (`draw_combat_keywords`, [[R781004]]): three different combat keywords drawn as it arrives and kept on it. Cry ([[R1]]): C+ #65.5's `replace_hand_with_random` for each player, its controller's first: each hand moved to its owner's graveyard (no discard) and as many random cards from `{ complicated: true }` ([[R1846000]]) added, repeats allowed ([[R60]]), never this card ([[R387]]). Radiant: its controller's new cards Radiant.
- **Rulings:** ST-S18-6: each hand is replaced by as many complicated cards as it held; an empty hand gets none (R129). ST-S18-7: the pool is the sets that ship, as no `everySet` is asked (R1846001). These are proposals for S18, which takes R-numbers from its own block. No R-number here.
- **Numbers:** keywords 3 ↑ (Radiant 3).
- **Check:** Typo "both player's" read as both players'. R275: stats doubled and your new hand Radiant.
- **Class:** C (card metrics, M12).

#### Stinkyscape #112 · Job Application
`stinkyscape-112` · (1) Field Spell, Quest · Legendary · built by S10 (#1865)

> **Designer:** (1) Job Application, Field Spell, Quest, Legendary, Stinkyscape, #112 · [Lucky 1] / Get employed! / Death: Resummon this at Stage 1\. /  / * Stage 1: Upload Resume /   * Quest: Send a card from your hand in as your resume. /   * Reward: Hear back in 2-6 turns. /     * Create some evaluation algorithm based on the card uploaded. /       * Should have a little bit of RNG in it. /       * {Easter egg: if it's not super dumb and resource intensive, have a small % chance to send an email to the player thanking them for their application with the analysis results in it} /       * {Easter egg: an iKwzki upload always hears back in 6} / * Stage 2: The Interview (Quest chosen at random) /   * Quest: Prove your resume. Play a card that costs the same as your resume. /   * Quest: Behavioral round. Play 3 cards of different costs in a single turn. /   * Quest: Hardball question. Take or deal 12+ damage in a single instance. /   * Reward: Get your resume back, buffed 5 [10] times and it costs (0). / * Stage 3: The Offer /   * Quest: Activate to claim next turn or to send in another application (return to Stage 1) and triple rewards. /   * Reward: At the start of every turn get a Paycheck

- **Text:** Get employed! Its quest runs in three stages, one at a time. / Death: Resummon this at Stage 1. / Stage 1, Upload Resume. / Quest: Activate this to send a card from your hand in as your resume. Reward: Hear back in 2 to 6 turns. / Stage 2, The Interview: one of three quests, drawn at random. / Quest: Play a card that costs the same as your resume; play 3 cards of different costs in one turn; or take or deal 12 or more damage in one hit. Reward: Add your resume to your hand, Buffed {buffs|time|times}. It costs (0). / Stage 3, The Offer. / Quest: Activate this to accept the offer, or to send in another application, back to Stage 1, with every reward tripled. Reward: From your next turn on, add a Paycheck to your hand at the start of each of your turns.
- **Radiant:** Lucky 1 / Get employed! Its quest runs in three stages, one at a time. / Death: Resummon this at Stage 1. / Stage 1, Upload Resume. / Quest: Activate this to send a card from your hand in as your resume. Reward: Hear back in 2 to 6 turns. / Stage 2, The Interview: one of three quests, drawn at random. / Quest: Play a card that costs the same as your resume; play 3 cards of different costs in one turn; or take or deal 12 or more damage in one hit. Reward: Add your resume to your hand, Buffed {buffs|time|times}. It costs (0). / Stage 3, The Offer. / Quest: Activate this to accept the offer, or to send in another application, back to Stage 1, with every reward tripled. Reward: From your next turn on, add a Paycheck to your hand at the start of each of your turns.
- **Engine:** Staged Quests ([[§6.3]] Quest stages, [[R1854000]]): `Script.stages`, three stages, the first opening as it enters, each public on the card. Stage 1's goal is `byEffect` ([[R1854001]]): an Activate ([[R384]]) sends a hand card in as the resume, set aside and kept by the card (`remember`, its cost noted), and completes the stage (`complete_this_quest`); its reward is `hear_back` ([[R1854004]]) for 2–6 of its controller's turns, S10's evaluation of the resume drawing the number from the match rng (an iKwzki, S #110, always 6: the designer's note). Stage 2 draws one of three goals as it opens ([[R1854002]]): `playCosting { remembered }`, `differentCostsInTurn { count: 3 }` or `damageInstance { amount: 12 }`; its reward adds the resume to the hand Buffed 5 times ([[§6.3]] Buff) at a `costOverride` of 0, a full hand burning it ([[§2.4]]). Stage 3's goal is an Activate of two modes ([[R81]]): accept, after which the card's own start-of-turn effect adds a Paycheck (S #112.1) at each of its controller's starts of turn from the next on, while it stays; or re-apply, `open_stage { 1 }` with every reward tripled again. Death ([[§6.2]]): it is summoned again from the graveyard into its zone, its line reset to Stage 1 ([[R78]]); an exile ends it. Radiant: Lucky 1 ([[§6.1]], [[R1438]]) on the hear-back roll, keeping the shorter wait, and Buffed 10 times. NEW (S10): the resume's evaluation and the tripling. The designer's email Easter egg is dropped (ST9 Q11).
- **Rulings:** ST-S10-1: "Send a card from your hand in as your resume" is an Activate of the card's, a `byEffect` goal ([[R1854001]]): the card leaves the hand, set aside with the Job Application, and comes back as Stage 2's reward; if the Job Application leaves the field first, the resume is exiled. ST-S10-2: "Hear back in 2-6 turns" is S10's evaluation of the resume, a draw from the match rng between 2 and 6 weighted by the card (its cost and rarity, S10's table), an iKwzki always 6; on the Radiant face Lucky 1 draws twice and keeps the shorter wait. ST-S10-3: "triple rewards" multiplies Stage 2's Buffs and Stage 3's Paychecks by 3 for each application sent in again, cumulatively (15 Buffs and 3 Paychecks a turn after one, 45 and 9 after two); S10 may cap the factor with a named constant. ST-S10-4: "At the start of every turn get a Paycheck" is each of its controller's starts of turn from the one after the Activate, the card's own effect while it stays on the field: a Job Application that dies is resummoned at Stage 1 and the Paychecks stop. ST-S10-5: "Death: Resummon this at Stage 1" summons the same card from its owner's graveyard into its own backrow zone, else the leftmost open one; with none it stays in the graveyard.
- **Numbers:** buffs 5 ↑ (Radiant 10).
- **Check:** One face in the list, with [Lucky 1] and "5 [10]" marks (the plan's discrepancy 7): the Radiant face prints Lucky 1 and Buffs 10 times. "Get employed!" ends with a stage count so the line ends in a full stop ([[R366]]); each "Quest:" starts its own line. The email Easter egg is dropped (ST9 Q11); "an iKwzki upload always hears back in 6" is kept as a reading. R275: Lucky 1 and twice the Buffs.
- **Class:** D.

#### Stinkyscape #112.1 · Paycheck
`stinkyscape-112-1` · (0) Spell, Token · Token (printed Legendary) · built by S10 (#1865)

> **Designer:** (0) Paycheck, Spell, Legendary, Token, Stinkyscape, #112.1 · Choose one: Gain 2 mana, draw 1 card, and gain 20 credits or go all in on red with a 18/39 chance to gain double the reward ~~~ Lucky 1 / Gain 2 mana, draw 1 card, and gain 20 credits or go all in on red with a 18/39 chance to gain double the reward

- **Text:** Choose one: Gain {mana} mana, draw {draw|card|cards} and gain {credits} Credits; or go all in on red, with an 18/39 chance to gain twice as much.
- **Radiant:** Lucky 1 / Choose one: Gain {mana} mana, draw {draw|card|cards} and gain {credits} Credits; or go all in on red, with an 18/39 chance to gain twice as much.
- **Engine:** Two declared modes ([[R81]]). Payday: 2 mana this turn (The Coin's, [[§6.3]] Mana), a draw ([[§2.4]]) and `gain_market_credits` of 20 ([[R1840005]]), the Credits opening #1853's Marketplace (pending PR #1963). All in on red: one luck-based roll at 18 in 39 ([[§6.1]] Lucky), a win giving twice each (4 mana, 2 draws, 40 Credits), a loss nothing. Radiant: Lucky 1 rolls it again and keeps the win ([[R1438]], [[R1440]]); the face is otherwise the same.
- **Rulings:** ST-S10-6: "gain double the reward" doubles each of the three (4 mana, draw 2, 40 Credits), and a loss gives nothing; the 18/39 is the designer's roulette, a luck-based roll Lucky improves.
- **Numbers:** mana 2 ↑; draw 1 ↑; credits 20 ↑ (step 5).
- **Check:** The Radiant face keeps "Choose one" (the list drops it) and adds Lucky 1 (the plan's discrepancy 7). R275: Lucky 1 lifts the gamble from 46% to about 71%.
- **Class:** C.

#### Stinkyscape #113 · A Very Dandy Lad
`stinkyscape-113` · (4) Unit, Human · Epic · 9/15 → 18/30 · built by S17 (#1876)

> **Designer:** (4) A Very Dandy Lad, Unit, Human, Epic, Stinkyscape, #113 · [9/15] / Tribute 2 / Aura: all Permanents that cost (2) or less are Exiled ~~~ [18/30] / Tribute 2 / Aura: all enemy Permanents that cost (2) or less are Exiled

- **Text:** Tribute 2 / Aura: All permanents that cost (2) or less are exiled.
- **Radiant:** Tribute 2 / Aura: All enemy permanents that cost (2) or less are exiled.
- **Engine:** Tribute 2, its play's cost ([[R101]]). An exile aura (`Script.exile_auras`, [[R1850002]]): both rows, `maxCost` 2, the cost each card has now ([[R66]]): at every state check each face-up card on the field it reaches is exiled, never this card, a face-down or a Dormant one; an arrival's Cry resolves first. Radiant: the enemy side only.
- **Rulings:** ST-S17-11: "all Permanents" are both sides' Units and face-up backrow cards; the (2) is the aura's printed filter, no param. These are proposals for S17, which takes R-numbers from its own block. No R-number here.
- **Numbers:** none (the (2) is the aura's printed filter, `maxCost`, static data).
- **Check:** R275: stats doubled and your own side spared.
- **Class:** C (M16's exile aura).

#### Stinkyscape #114 · Arctic Citadel
`stinkyscape-114` · (4) Unit, Human · Epic · 4/4 → 8/8 · built by S17 (#1876)

> **Designer:** (4) Arctic Citadel, Unit, Human, Epic, Stinkyscape, #114 · [4/4] / Cry: Fuse all cards from your Exile into this ~~~ [8/8] / Cry: Fuse all cards from BOTH Exile into this

- **Text:** Cry: Fuse every card in your exile into this.
- **Radiant:** Cry: Fuse every card in both exiles into this.
- **Engine:** Cry ([[R1]]): `fuse_cards` ([[R77]], [[R102]]): every card in its controller's exile pile, oldest first, fused into this card in one Fuse, this card kept: summed stats, the union of keywords, the scripts concatenated, cost min(sum, 4); the exiled cards cease to exist. An empty exile, nothing. Radiant: both players' exiles.
- **Rulings:** ST-S17-12: "your Exile" is your exile pile; the fused cards leave it (they cease to exist, R77), the exile counter unchanged (R55). These are proposals for S17, which takes R-numbers from its own block. No R-number here.
- **Numbers:** none.
- **Check:** R275: stats doubled and both exiles instead of one.
- **Class:** B (a Fuse from a pile).

#### Stinkyscape #115 · Scornboi
`stinkyscape-115` · (4) Unit, Human, Acclaimed · Legendary · 12/9 → 24/18 · built by S17 (#1876)

> **Designer:** (4) Scornboi, Unit, Human, Legendary, Acclaimed, Stinkyscape, #115 · [12/9] / Tribute 3 / Aura: all non-human, non-starship piece, non-starship, are exiled ~~~ [24/18] / Tribute 3 / Aura: all enemy non-human, non-starship piece, non-starship, are exiled

- **Text:** Tribute 3 / Aura: Every Unit that is neither Human, a Starship Piece nor a Starship is exiled.
- **Radiant:** Tribute 3 / Aura: Every enemy Unit that is neither Human, a Starship Piece nor a Starship is exiled.
- **Engine:** Tribute 3, its play's cost ([[R101]]). An exile aura ([[R1850002]]): the Units on both sides, `notTags` Human and `notKeywords` Starship Piece and Starship, read through the layers ([[§10.4]]), so a granted tag or keyword spares a Unit; exiled at each state check, never this card, a face-down or a Dormant one. Radiant: the enemy side only.
- **Rulings:** ST-S17-13: Scornboi exiles Units only (ST9); a Dormant Starship is no Unit until it launches and is passed by (R1839000). These are proposals for S17, which takes R-numbers from its own block. No R-number here.
- **Numbers:** none.
- **Check:** ST9: Units. R275: stats doubled and your own side spared.
- **Class:** C (M16's exile aura).

#### Stinkyscape #116 · Megabase
`stinkyscape-116` · (1) Field Spell, Quickdraw · Legendary · 0/20 → 0/40 · built by S04 (#1861)

> **Designer:** (1) Megabase, Field Spell, Quickdraw, Legendary, Stinkyscape, #116 · Can't be in defense position / Can't attack, Immune to spells, Immutable, Can't be tributed / Armor 15 / [0/20] / Activate: choose to give both players 5, 10, or 15 credits / Death: Give 100 credits to both players ~~~ Can't be in defense position / Can't attack, Immune to spells, Immutable, Can't be tributed / Armor 30 / [0/40] / Activate: choose to give players 5, 10, or 15 credits, your opponent gets 5 less / Death: Give 200 credits to both players

- **Text:** Animated, Can't attack, Immune to Spells, Immutable, Untributable, Armor 15 / Can't be in Defense Position. / Activate: Choose one: Both players gain 5, 10 or 15 Credits. / Death: Both players gain 100 Credits.
- **Radiant:** Animated, Can't attack, Immune to Spells, Immutable, Untributable, Armor 30 / Can't be in Defense Position. / Activate: Choose one: You gain 5, 10 or 15 Credits, and your opponent gains 5 fewer. / Death: Both players gain 200 Credits.
- **Engine:** Quickdraw ([[R640]]). An Animated Field Spell ([[§6.1]]): it animates as it enters into an open unit zone, a Unit that can't attack and is never in Defense Position ([[§4.1]]), Immune to Spells, Immutable, so its numbers are printed ([[R386]]), and Untributable ([[R1220]]), with Armor 15. Activate ([[R384]]), three declared modes ([[R81]]): each player `gain_market_credits` 5, 10 or 15 ([[R1840005]]), the first gain opening #1853's Marketplace (pending PR #1963). Death ([[§6.2]]): both gain 100. Radiant: 0/40, Armor 30; its Activate gives its controller N and the opponent N − 5; its Death 200 each.
- **Rulings:** ST-S04-10: "choose to give both players 5, 10, or 15 credits" is three declared modes; the Radiant "your opponent gets 5 less" gives the opponent 0, 5 or 10; "Can't be tributed" is Untributable ([[R1220]]).
- **Numbers:** none (Immutable: nothing tunes them ([[R386]])).
- **Check:** A Field Spell with Health, attacked and dying, is Animated (ST8). R275: 0/40 and Armor 30, the Activate one-sided by 5, and a Death of 200.
- **Class:** C.

#### Stinkyscape #117 · Core System
`stinkyscape-117` · (3) Spell · Epic · built by S14 (#1868)

> **Designer:** (3) Core System, Spell, Epic, Stinkyscape, #117 · Bounce all permanents, lose 2 mana next turn ~~~ Bounce all enemy permanents, lose 2 mana next turn

- **Text:** Bounce all permanents. Lose {loss} mana next turn.
- **Radiant:** Bounce all enemy permanents. Lose {loss} mana next turn.
- **Engine:** `bounce_all` over every permanent on the field of both sides (`rows: [units, backrow]`): the top of each pile and every backrow card, face-down ones included ([[R13]]), each back to its controller's hand, reset ([[R78]]), a token vanishing and a full hand burning the rest ([[§2.4]]); a Dormant card is out of reach ([[R1839000]]). Then `next_turn_mana { amount: −2 }` on its caster ([[R1848004]]): their next refresh gives their max mana less 2, never below 0, a badge "Next refresh −2 mana" on both seats. Radiant: the enemy's permanents alone.
- **Rulings:** ST-S14-6: "all permanents" is every card on the field of both sides, face-down ones included (each pile's top, [[R13]]); "next turn" is its caster's next refresh ([[R1848004]]).
- **Numbers:** loss 2 ↓.
- **Check:** R275: its caster's side kept (both sides to one). Hearthstone's Overload is the same rider ([[R1848004]]).
- **Class:** C (M14's next-turn mana).

#### Stinkyscape #118 · Secure System
`stinkyscape-118` · (3) Spell · Epic · built by S14 (#1868)

> **Designer:** (3) Secure System, Spell, Epic, Stinkyscape, #118 · Set all units' attack to 0, lose 1 mana next turn ~~~ Set all enemy unit's attack to 0, lose 1 mana next turn

- **Text:** Set all Units' Attack to 0. Lose {loss} mana next turn.
- **Radiant:** Set all enemy Units' Attack to 0. Lose {loss} mana next turn.
- **Engine:** For each Unit on the field of both sides (`for_each_card`), `buff { attack: −its Attack }`, read through the layers as it resolves ([[§10.4]]), so each shows 0; a later Buff or aura adds on top, as after Hearthstone's set. Then `next_turn_mana { amount: −1 }` on its caster ([[R1848004]]). Radiant: the enemy's Units alone.
- **Rulings:** ST-S14-7: "Set … attack to 0" is a Buff of minus each Unit's Attack now (S #10.1's reading), existing primitives, so an Attack aura that leaves later takes its part with it, never below 0; a set of Attack alone at layer 1 would be NEW.
- **Numbers:** loss 1 ↓; the 0 is what the card is ([[R1748003]]).
- **Check:** R275: its caster's Units keep their Attack (both sides to one).
- **Class:** C (M14's next-turn mana).

#### Stinkyscape #119 · Unsecure System
`stinkyscape-119` · (3) Spell · Epic · built by S14 (#1868)

> **Designer:** (3) Unsecure System, Spell, Epic, Stinkyscape, #119 · Double the attack of all units, all units gain first strike ~~~ Double the attack of all units, your units gain first strike

- **Text:** Double the Attack of all Units. All Units gain First Strike.
- **Radiant:** Double the Attack of all Units. Your Units gain First Strike.
- **Engine:** For each Unit on the field of both sides (`for_each_card`), `buff { attack: its Attack now }`, read through the layers ([[§10.4]]); then `grant_keyword { First Strike }` on every Unit of both sides, kept until it leaves the field ([[R78]]). Radiant: the First Strike for its caster's Units alone.
- **Rulings:** ST-S14-8: "Double" is a Buff of each Unit's Attack now, as Hearthstone doubles; the First Strike is for good, not for the turn.
- **Numbers:** none (the doubling is the card's word).
- **Check:** R275: First Strike on your side alone (both sides to one).
- **Class:** B.

#### Stinkyscape #120 · Wild System
`stinkyscape-120` · (3) Spell · Epic · built by S14 (#1868)

> **Designer:** (3) Wild System, Spell, Epic, Stinkyscape, #120 · All units attack each other until only one survives, gain 1 mana next turn / *{allies can attack each other, max 20 attacks}* ~~~ All units attack each other until only one survives, gain 1 mana next turn / *{allies can attack each other, max 20 attacks}*

- **Text:** All Units attack each other until only one survives. Gain {mana} mana next turn.
- **Radiant:** All Units attack each other until only one survives. Gain {mana} mana next turn.
- **Engine:** `melee {}` ([[R1848005]]): forced attacks ([[R53]]) between pairs drawn from the match rng, two draws an attack, an ally as good a target as an enemy, each its own combat and state check, until one Unit or none is left, none may attack another, or `MELEE_MAX_ATTACKS` (20) attacks are made; a hero is never a target; a Frozen or a Can't attack Unit fights. Then `next_turn_mana { amount: 1 }` on its caster ([[R1848004]]). Radiant: 2.
- **Rulings:** ST-S14-9: the designer's "{allies can attack each other, max 20 attacks}" is [[R1848005]]'s melee and its cap; a question a Death asks parks the rest (`@melee`).
- **Numbers:** mana 1 ↑ (Radiant 2).
- **Check:** ST8 asks for one text on both faces; [[R276]] refuses identical faces, so the Radiant face gains 2 mana next turn, not 1 ([[R1857001]]). R275: the rider doubled, the melee alike (⚠ designer).
- **Class:** C (M14's melee).

#### Stinkyscape #121 · System Schematic
`stinkyscape-121` · (1) Spell · Epic · built by S14 (#1868)

> **Designer:** (1) System Schematic, Spell, Epic, Stinkyscape, #121 · Add a Core System, Secure System, Unsecure System, or Wild System to your hand ~~~ Add a Radiant Core System, Secure System, Unsecure System, or Wild System to your hand

- **Text:** Add your choice of a Core System, a Secure System, an Unsecure System or a Wild System to your hand.
- **Radiant:** Add your choice of a Radiant Core System, Secure System, Unsecure System or Wild System to your hand.
- **Engine:** Four modes, one per System, chosen with the play ([[R81]]); then `add_to_hand { defId }` of the chosen one, S #117 to #120 (Radiant `radiant: true`), a full hand burning it ([[§2.4]]).
- **Rulings:** ST-S14-10: "Add a Core System, Secure System, Unsecure System, or Wild System" is its player's choice of one, a mode ([[R81]]), not a random pick.
- **Numbers:** none (one card).
- **Check:** R275: the chosen System Radiant.
- **Class:** B.

#### Stinkyscape #122 · Vamp Rounds
`stinkyscape-122` · (1) Unit · Common · 2/1 → 4/2 · built by S03 (#1860)

> **Designer:** (1) Vamp Rounds, Unit, Common, Stinkyscape, #122 · 2/1 / Starship Piece / Equip / Lifesteal ~~~ 4/2 / Starship Piece / Equip / Lifesteal

- **Text:** Starship Piece, Equip, Lifesteal
- **Radiant:** Starship Piece, Equip, Lifesteal
- **Engine:** Keywords only ([[§6.1]]): its script is empty. Equip ([[R1086]]): played onto one of your Units it fuses into it, the host keeping its zone and taking its stats, keywords and text ([[R77]], [[R102]]), Starship Piece only into a piece ([[R1852001]]); played alone its death fuses a copy into its owner's Dormant Starship ([[R1852000]]).
- **Rulings:** none new.
- **Numbers:** none (keywords and stats).
- **Check:** R275: as an Equip its stats are its effect, doubled.
- **Class:** A

#### Stinkyscape #123 · Gokugains
`stinkyscape-123` · (1) Unit · Rare · 2/1 → 4/2 · built by S18 (#1877)

> **Designer:** (1) Gokugains, Unit, Rare, Stinkyscape, #123 · [2/1] / Cloaked / Cry: a random unit in your opponents hand is nerfed 3 times secretly *{nerfs avoid cost, opponent does not see the nerfs until they play the card}* ~~~ [4/2] / Cloaked / Cry: a random unit in your opponents hand is nerfed 30 times secretly *{nerfs avoid cost, opponent does not see the nerfs until they play the card}*

- **Text:** Cloaked / Cry: Secretly Nerf a random Unit in your opponent's hand {times|time|times}, never its cost. No one sees the Nerfs until it is played.
- **Radiant:** Cloaked / Cry: Secretly Nerf a random Unit in your opponent's hand {times|time|times}, never its cost. No one sees the Nerfs until it is played.
- **Engine:** Cloaked ([[R783004]]; set face-down, its Cry runs as it is revealed, [[R783006]]). Cry ([[R1]]): `degrade_secretly { random: 1, times }` ([[R1845000]]) over the Unit cards of the opponent's hand that can take Nerfs (not Immutable): a hidden count on the card, nothing drawn or reported, no view showing it, the AI's redaction dropping it. It lands as the card is next played, at [[§10.5]] step 3, each Nerf drawn from every row but the cost's ([[R1845001]]).
- **Rulings:** ST-S18-8: "nerfs avoid cost" and "the opponent does not see the nerfs until they play the card" are R1845001's landing; Gokugains' controller does not see them either (R1845000). An empty hand or one with no Unit, nothing. These are proposals for S18, which takes R-numbers from its own block. No R-number here.
- **Numbers:** times 3 ↑ (Radiant 30), its Cry's (`cry: true`).
- **Check:** The designer's {…} notes are M11's (R1845000, R1845001). R275: stats doubled and 30 Nerfs against 3 (10×).
- **Class:** C (hidden Nerfs, M11).

#### Stinkyscape #124 · Jlockeed's Shipbuilding Grant
`stinkyscape-124` · (2) Spell, Jlockeed · Common · built by S19 (#1878)

> **Designer:** (2) Jlockeed's Shipbuilding Grant, Spell, Common, Jlockeed's, Stinkscape, #124 · Add 3 random Starship Pieces to your hand ~~~ Add 3 random Radiant Starship Pieces to your hand

- **Text:** Add {cards|random Starship Piece|random Starship Pieces} to your hand.
- **Radiant:** Add {cards|random Radiant Starship Piece|random Radiant Starship Pieces} to your hand.
- **Engine:** `add_random_from_catalog`, one draw of the match rng per card ([[R60]]): NEW: a pool of the cards that print Starship Piece ([[§6.1]]), non-tokens, the Dormant Starship never one ([[R1852001]], [[R1857001]]): `CatalogQuery` has no keyword field on main, so S19 adds one or names the pieces by id; a pool that names no set draws from the sets that ship ([[R380]]), so it names Stinkyscape's while the set is held ([[R1420]]); a full hand burns the rest ([[§2.4]]). Radiant: `radiant: true`.
- **Rulings:** ST-S19-1: "a random Starship Piece" is a random non-token card that prints the keyword on its face, from the set's pieces (about forty, S03's and S06's); the Dormant Starship, no piece, is never one. These are proposals for the S19 part, which takes R-numbers from its own block.
- **Numbers:** cards 3 ↑.
- **Check:** ST8: the tag "Jlockeed's" is Jlockeed (R278); "Stinkscape" is Stinkyscape. R275: each piece Radiant.
- **Class:** B

#### Stinkyscape #125 · Jlockeed's Spacerace Propaganda
`stinkyscape-125` · (2) Field Spell, Jlockeed · Rare · built by S19 (#1878)

> **Designer:** (2) Jlockeed's Spacerace Propaganda, Spell, Rare, Stinkscape, #125 · Activate: add a random Starship Piece to your hand ~~~ Activate: add a random Radiant Starship Piece to your hand

- **Text:** Activate: Add a random Starship Piece to your hand.
- **Radiant:** Activate: Add a random Radiant Starship Piece to your hand.
- **Engine:** A Field Spell (ST8, [[R1857001]]) whose Activate runs while it is face-up in the backrow, once a turn ([[R384]]): one card from S #124's Starship Piece pool (NEW). Radiant: `radiant: true`.
- **Rulings:** ST-S19-1 (the pool). These are proposals for the S19 part, which takes R-numbers from its own block.
- **Numbers:** none (one card a use; the Activate count is no param, R482).
- **Check:** ST8: a Field Spell, tagged Jlockeed. The designer's "add" is "Add". R275: a Radiant piece.
- **Class:** B

#### Stinkyscape #126 · Jlockeed's Permanent Procurement Lobbying
`stinkyscape-126` · (2) Spell, Jlockeed · Epic · built by S19 (#1878)

> **Designer:** (2) Jlockeed's Permanent Procurement Lobbying, Epic, Jlockeed, Spell, Stinkyscape, #126 · For the rest of the game, at the start of your turn add a random Starship Piece card to your hand. ~~~ For the rest of the game, at the start of your turn add a random Radiant  Starship Piece card to your hand, it costs (1) less.

- **Text:** For the rest of the game. / Start of turn: Add {cards|random Starship Piece|random Starship Pieces} to your hand.
- **Radiant:** For the rest of the game. / Start of turn: Add {cards|random Radiant Starship Piece|random Radiant Starship Pieces} to your hand. Each costs ({discount}) less.
- **Engine:** C+ #52 Jlockeed's Permanent Defense Contract's shape: a rest-of-game start-of-turn modifier on its caster ([[R458]]), kept whatever becomes of the card, adding N cards from S #124's Starship Piece pool (NEW) at the start of each of their turns, before the draw ([[R62]]); a full hand burns them. Radiant: `radiant: true` and a `costMod` of −discount on each ([[R65]]).
- **Rulings:** ST-S19-2: "at the start of your turn" is each of your turns for the rest of the game, starting with your next, C+ #52's modifier; "costs (1) less" is a cost modifier that never takes it below (0). These are proposals for the S19 part, which takes R-numbers from its own block.
- **Numbers:** cards 1 ↑; discount 1 ↑ (Radiant only, `tunedOn: radiant`).
- **Check:** Written in C+ #52's words (R366: "Start of turn:", never "at the start of your turn"). R275: Radiant pieces at (1) less.
- **Class:** B

#### Stinkyscape #127 · Jlockeed's Siege Tank
`stinkyscape-127` · (3, Embiggened 4) Unit, Jlockeed · Rare · 8/5 → 16/10 · built by S19 (#1878)

> **Designer:** (3 embiggen 4) Jlockeed's Siege Tank, Rare, Jlockeed, Unit, Stinkyscape, #127 · [8/5] / End of turn: attacks a random enemy / Embiggen: gain Starship Piece ~~~ [16/10] / End of turn: attacks a random enemy / Embiggen: gain Starship Piece

- **Text:** End of turn: This attacks a random enemy. / Embiggened (4): Starship Piece.
- **Radiant:** End of turn: This attacks a random enemy. / Embiggened (4): Starship Piece.
- **Engine:** End of turn ([[R62]]): a forced attack ([[R53]]) on one enemy, the hero or a Unit, drawn from the match rng ([[R60]]) among those it may attack under [[§4.2]] step 2 (Flying's restriction holds, [[R783002]]); position, summoning sickness and Taunt are not read and no exertion is spent; with none, nothing. Embiggened ([[R1175000]]): Starship Piece, a keyword the Embiggened version has as the play resolves (`instance.embiggened`, [[R81]]), as Core #56 Jilliax's Equip comes with its price ([[R831013]]); NEW if the Embiggened version cannot lend a keyword on main, Core #56's Equip being a play option and no keyword.
- **Rulings:** ST-S19-3: "attacks a random enemy" is a forced attack on an enemy drawn among those it may attack (Flying's rule holding), Taunt not binding; "Embiggen: gain Starship Piece" is the Embiggened version having the keyword for good. These are proposals for the S19 part, which takes R-numbers from its own block.
- **Numbers:** none (no number is printed).
- **Check:** The designer's "Embiggen:" is "Embiggened (4):" (R1175000). R275: stats doubled, the attack with them.
- **Class:** B

#### Stinkyscape #128 · Jlockeed's Yamato Cannon
`stinkyscape-128` · (2, Embiggened 3) Unit, Jlockeed · Rare · 4/1 → 8/2 · built by S19 (#1878)

> **Designer:** (2 embiggen 3) Jlockeed's Yamato Cannon, Rare, Jlockeed, Unit, Stinkyscape, #128 · [4/1] / Ranged, Cleave / Embiggen: gain Starship Piece ~~~ [8/2] / Ranged, Cleave / Embiggen: gain Starship Piece

- **Text:** Ranged, Cleave / Embiggened (3): Starship Piece.
- **Radiant:** Ranged, Cleave / Embiggened (3): Starship Piece.
- **Engine:** Ranged ([[R783000]]) and Cleave ([[§6.1]], [[R63]]): it takes no strike back from a defender without Ranged, and its attack splashes the Units beside its target, which never strike back. Embiggened ([[R1175000]]): Starship Piece, as S #127's.
- **Rulings:** ST-S19-3 (the Embiggened keyword). These are proposals for the S19 part, which takes R-numbers from its own block.
- **Numbers:** none (keywords and stats).
- **Check:** R275's effect half: the same keywords on both faces (⚠ designer).
- **Class:** A

#### Stinkyscape #129 · Jlockeed's Missile Pod
`stinkyscape-129` · (1, Embiggened 2) Unit, Jlockeed · Epic · 2/1 → 4/2 · built by S19 (#1878)

> **Designer:** (1 embiggen 2) Jlockeed's Missile Pod, Epic, Jlockeed, Unit, Stinkyscape, #129 · [2/1] / End of turn: deal floor(attack / 4) to all enemies / Embiggen: gain Starship Piece and Equip ~~~ [4/2] / End of turn: deal floor(attack / 3) to all enemies / Embiggen: gain Starship Piece and Equip

- **Text:** End of turn: Deal damage equal to its Attack divided by {divisor}, rounded down, to all enemies. / Embiggened (2): Starship Piece and Equip.
- **Radiant:** End of turn: Deal damage equal to its Attack divided by {divisor}, rounded down, to all enemies. / Embiggened (2): Starship Piece and Equip.
- **Engine:** End of turn ([[R62]]): one hit of ⌊Attack ÷ divisor⌋, its Attack as it stands, to the enemy hero and every enemy Unit (`damage_all`, [[R59]]); a 0 is no hit ([[R63]]). Embiggened ([[R1175000]]): Starship Piece, and Equip as a play option at that price (Core #56 Jilliax's, [[R831013]], [[R1086]]), so it can be fused into a Unit whose Attack it then divides.
- **Rulings:** ST-S19-4: "floor(attack / 4)" is its Attack as it stands at the end of turn, buffs included; equipped, its text runs on the host and reads the host's Attack. These are proposals for the S19 part, which takes R-numbers from its own block.
- **Numbers:** divisor 4 ↓ (Radiant 3).
- **Check:** The designer's "Embiggen:" is "Embiggened (2):". R275: stats doubled, divisor 4 → 3.
- **Class:** B

#### Stinkyscape #130 · Jlockeed's Starship Retrofitting Program
`stinkyscape-130` · (1) Spell, Jlockeed · Epic · built by S19 (#1878)

> **Designer:** (1) Jlockeed's Starship Retrofitting Program, Epic, Jlockeed, Spell, Stinkyscape, #130 · Add a random Jlockeed card to your hand, it gains Starship Piece ~~~ Add a random Radiant Jlockeed card to your hand, it gains Starship Piece, and costs (1) less

- **Text:** Add a random Jlockeed card to your hand. It gains Starship Piece.
- **Radiant:** Add a random Radiant Jlockeed card to your hand. It gains Starship Piece and costs ({discount}) less.
- **Engine:** `add_random_from_catalog { tags: [Jlockeed] }`, the non-token cards with the tag, itself left out ([[R278]], [[R387]]), then `grant_keyword` Starship Piece on the card in the hand, read on its death as a granted piece is ([[R1852000]]); a full hand burns it ([[§2.4]]). Radiant: `radiant: true` and a `costMod` of −discount ([[R65]]). A card given it fuses only when it dies: a Spell that resolves never does; NEW if a keyword granted in the hand does not ride onto the field (S19 checks).
- **Rulings:** ST-S19-5: the granted Starship Piece rides the card from the hand onto the field and is read on its death (R1852000); a Spell or Field Spell given it fuses nothing unless it dies; "Radiant" makes the added card Radiant. These are proposals for the S19 part, which takes R-numbers from its own block.
- **Numbers:** discount 1 ↑ (Radiant only, `tunedOn: radiant`).
- **Check:** R275: a Radiant card and (1) less.
- **Class:** B

#### Stinkyscape #131 · Spacerace Stimulus
`stinkyscape-131` · (1) Field Trap · Common · 6/6 → 12/12 · built by S02 (#1859)

> **Designer:** (1) Spacerace Stimulus, Field Trap, Common, Stinkyscape, #131 · 6/6 / Animated / Starship Piece / Reveals when either player launches a Starship. ~~~ 12/12 / Animated / Starship Piece / Reveals when either player launches a Starship or plays a Starship Piece.

- **Text:** Animated, Starship Piece / Reveals when either player launches a Starship.
- **Radiant:** Animated, Starship Piece / Reveals when either player launches a Starship or plays a Starship Piece.
- **Engine:** A Field Trap ([[§5]]), set face-down: it answers `starshipLaunched` of either player's Starship ([[R1852003]]) and, on the Radiant face, `cardPlayed` of a card with Starship Piece, either player's, its own play left out; it fires as any Trap does ([[R17]]) and animates as its last step into its own lane's unit zone or the leftmost open one ([[§6.1]] Animated, [[R64]]), a 6/6 Unit with Starship Piece whose death fuses a copy into its owner's Dormant Starship ([[R1852000]]). With no open unit zone it stays face-up.
- **Rulings:** ST-S02-3: "plays a Starship Piece" is a card played that prints or has the keyword, either player's; its reveal is the animation, nothing more. These are proposals for the S02 part, which takes R-numbers from its own block.
- **Numbers:** none (no number is printed).
- **Check:** R275: the Animated face's stats doubled and a wider trigger.
- **Class:** C

#### Stinkyscape #132 · Defensive Crystal
`stinkyscape-132` · (2) Unit · Common · 3/6 → 6/12 · built by S03 (#1860)

> **Designer:** (2) Defensive Crystal, Common, Unit, Stinkyscape, #132 · [3/6] / Starship Piece / Death: Give your hero 1 armor. ~~~ [6/12] / Starship Piece / Death: Give your hero 2 armor.

- **Text:** Starship Piece / Death: Your hero gains {heroArmor} Armor.
- **Radiant:** Starship Piece / Death: Your hero gains {heroArmor} Armor.
- **Engine:** Starship Piece: its death fuses a copy into its owner's Dormant Starship, summoned first if they have none ([[R1852000]]). Death ([[§6.2]]): `gain_hero_armor`, §6.1's Armor on the hero, kept for the rest of the game and taken off each hit on the hero, as M #95's "your hero gains 6 Armor" ([[R124]]).
- **Rulings:** ST-S03-7: "armor" is §6.1's Armor on the hero, a reduction of each hit kept for the rest of the game (R124), not a pool. These are proposals for the S03 part, which takes R-numbers from its own block.
- **Numbers:** heroArmor 1 ↑ (Radiant 2).
- **Check:** R275: stats and Armor doubled.
- **Class:** A

#### Stinkyscape #133 · Ultra Capacitor
`stinkyscape-133` · (2) Unit · Common · 2/2 → 4/4 · built by S02 (#1859)

> **Designer:** (2) Ultra Capacitor, Common, Unit, Stinkyscape, #133 · [2/2] / Starship Piece / When launched: gain +1/+1 per starship piece for each other starship piece launched ~~~ [4/4] / Starship Piece / When launched: gain +2/+2 per starship piece for each starship piece launched

- **Text:** Starship Piece / When launched, your Starship gains +{per}/+{per} for each other Starship Piece fused into it.
- **Radiant:** Starship Piece / When launched, your Starship gains +{per}/+{per} for each Starship Piece fused into it.
- **Engine:** Starship Piece: its death fuses a copy into its owner's Dormant Starship, summoned first if they have none ([[R1852000]]). When launched ([[R1852003]]): the Starship's memory (`starship_pieces_of`, [[R1852005]]) counts its pieces, this one's copy left out on the base face and counted on the Radiant face, each copy of the same card counted; `buff` +N/+N per piece on the Starship.
- **Rulings:** ST-S02-4: "each other starship piece launched" is each other piece fused into the Starship it launches with, by its memory; the Radiant face drops "other", so it counts itself. "+1/+1 per starship piece for each" is one +1/+1 a piece. These are proposals for the S02 part, which takes R-numbers from its own block.
- **Numbers:** per 1 ↑ (Radiant 2).
- **Check:** The designer's "When launched:" is the trigger word "When launched,". R275: stats doubled, the gain doubled and one more counted.
- **Class:** C

#### Stinkyscape #134 · Felinor Containment Chamber
`stinkyscape-134` · (1) Unit · Common · 2/1 → 4/2 · built by S03 (#1860)

> **Designer:** (1) Felinor Containment Chamber, Common, Unit, Stinkyscape, #134 · [2/1] / Starship Piece / Death: Summon a Felinor token with stats equal to this units ~~~ [4/2] / Starship Piece / Death: Summon a Felinor token with stats equal to twice this units, it gains Divine Shield

- **Text:** Starship Piece / Death: Summon a Felinor Token with this Unit's Attack and Health.
- **Radiant:** Starship Piece / Death: Summon a Felinor Token with twice this Unit's Attack and Health. It gains Divine Shield.
- **Engine:** Death ([[§6.2]]), on the snapshot it died with ([[R89]]): `summon` the Felinor Token ([[§7]]) with a `statsOverride` of its Attack and its maximum Health then, into the leftmost open unit zone ([[R64]]), before its own copy fuses into the Dormant Starship ([[R1852000]]), which may then stack on lane 3 if the token took the last zone; the Radiant face doubles both and grants Divine Shield (`grant_keyword`). The token is the base Felinor Token, summoned X/X.
- **Rulings:** ST-S03-8: "stats equal to this unit's" are its Attack and its maximum Health as it died (its current Health is 0 or less then), buffs included. These are proposals for the S03 part, which takes R-numbers from its own block.
- **Numbers:** none (the stats are the card's; "twice" is the Radiant face's frame).
- **Check:** The designer's "Felinor token" is the Felinor Token (Core). R275: stats doubled and the token doubled with Divine Shield.
- **Class:** B

#### Stinkyscape #135 · Sacked
`stinkyscape-135` · (1) Trap · Common · built by S15 (#1875)

> **Designer:** (1) Sacked, Trap, Common, Stinkyscape, #135 · Reveals when your opponent plays a Unit. Stuff it in a 0/4 Sack. ~~~ Reveals when your opponent plays a Unit. Stuff it in a 0/4 Sack for yourself.

- **Text:** Reveals when your opponent plays a Unit: After it resolves, stuff it into a Sack they control.
- **Radiant:** Reveals when your opponent plays a Unit: After it resolves, stuff it into a Sack you control.
- **Engine:** A Trap answering an enemy Unit's play after it resolves, its Cry first, as Core #41 Sheepish Trap does ([[R17]]). `hold_beneath { target, holder: { defId: stinkyscape-135-1, player } }` ([[R1850003]]): the Sack (S #135.1) is summoned first, for the Unit's controller (Radiant: for the Trap's), into [[R64]]'s zone, then the Unit leaves the field beneath it as an exiled card does, no Death ([[R78]]), `summoned` coming before `held`. When the Sack leaves the field, the Unit is summoned for the Sack's controller (S #135.1). A Sack that finds no zone holds nothing, so the Unit comes back at the next check; a unit token ceases to exist as it is taken ([[R11]]).
- **Rulings:** ST-S15-5: "Reveals when your opponent plays a Unit. Stuff it …" answers the play after it resolves (Sheepish's timing); "for yourself" is the Sack summoned for the Trap's controller, so the Unit comes back to them ([[R1850003]]).
- **Numbers:** none (the card prints no number).
- **Check:** The designer's "a 0/4 Sack" is S #135.1, whose own row prints its stats. R275: the Sack, and the Unit it releases, become yours.
- **Class:** C (M16's hold beneath).

#### Stinkyscape #135.1 · Sack
`stinkyscape-135-1` · (1) Unit, Token · Token (printed Common) · 0/4 → 0/8 · built by S15 (#1875)

> **Designer:** (1) Sack, Unit, Token, Common, Stinkyscape, #135.1 · 0/4 / Death: Summon the Unit stuffed into this. ~~~ 0/8 / Brittle 1 / Death: Summon the Unit stuffed into this.

- **Text:** When this leaves the field, summon the Unit stuffed into it.
- **Radiant:** Brittle 1 / When this leaves the field, summon the Unit stuffed into it.
- **Engine:** A holder ([[R1850003]]): the Unit S #135 stuffs into it waits in `state.held`; at the first state check after the Sack leaves the field, any way (it dies, is Bounced, exiled, transformed or held itself), that Unit is summoned for the Sack's controller then, [[R64]]'s zone, summoning sick, no Cry ([[R1]]), or exiled with no zone open. A Sack holding nothing summons nothing. Radiant: Brittle 1 ([[R638]]): it crumbles at a start of its controller's turn after a full cycle, releasing its Unit.
- **Rulings:** ST-S15-6: "Death:" is the Sack leaving the field any way ([[R1850003]]); the Unit comes back to the Sack's controller.
- **Numbers:** none (Brittle's 1 is a numbered keyword, [[R482]]).
- **Check:** R275: 0/8 against 0/4 (a 0 stays 0), and Brittle 1 frees what it holds after a turn cycle.
- **Class:** C (M16's hold beneath).

#### Stinkyscape #136 · Magic Module
`stinkyscape-136` · (2) Unit · Common · 4/3 → 8/6 · built by S02 (#1859)

> **Designer:** (2) Magic Module, Common, Unit, Stinkyscape, #136 · [4/3] / Starship Piece / Equip / Spell damage +2 / When launched: Add a random spell to your hand that deals damage ~~~ [8/6] / Starship Piece / Equip / Spell damage +4 / When launched: Add two random spells to your hand that deals damage

- **Text:** Starship Piece, Equip, Spell Damage +2 / When launched, add {spells|random Spell that deals damage|random Spells that deal damage} to your hand.
- **Radiant:** Starship Piece, Equip, Spell Damage +4 / When launched, add {spells|random Spell that deals damage|random Spells that deal damage} to your hand.
- **Engine:** Equip ([[R1086]]): played onto one of your Units it fuses into it, the host keeping its zone and taking its stats, keywords and text ([[R77]], [[R102]]), Starship Piece only into a piece ([[R1852001]]); played alone its death fuses a copy into its owner's Dormant Starship ([[R1852000]]); Spell Damage +N ([[§6.1]]) goes with it into a host and into the Starship. When launched ([[R1852003]]): `add_random_from_catalog` over the non-token Spells marked `dealsDamage` ([[R1748001]]): NEW, `CatalogQuery` has no such field on main (S02 adds it or lists them); a pool that names no set reads the sets that ship ([[R380]]); a full hand burns the rest.
- **Rulings:** ST-S02-5: "a random spell that deals damage" is a non-token Spell marked `dealsDamage` (R1748001), Field Spells out; the designer's "spells … that deals" is "Spells that deal". These are proposals for the S02 part, which takes R-numbers from its own block.
- **Numbers:** spells 1 ↑ (Radiant 2).
- **Check:** The designer's "Spell damage +2" is the keyword Spell Damage +2. R275: stats, Spell Damage and Spells doubled.
- **Class:** C

#### Stinkyscape #137 · Firststrike Capability
`stinkyscape-137` · (4) Unit · Common · 9/9 → 18/18 · built by S02 (#1859)

> **Designer:** (4) Firststrike Capability, Common, Unit, Stinkyscape, #137 · [9/9] / First strike / Cry: If you are building a ship Launch it and activate *{discover}* a War doctrine ~~~ [18/18] / First strike / Cry: If you are building a ship Launch it and activate *{discover}* a Radiant War doctrine

- **Text:** First Strike / Cry: If you are building a ship, launch it, then Discover a War Doctrine and cast it.
- **Radiant:** First Strike / Cry: If you are building a ship, launch it, then Discover a Radiant War Doctrine and cast it.
- **Engine:** Cry ([[R1]]): if `building_a_ship` ([[R1852005]]), `awaken` its controller's Dormant Starship, which launches it for nothing, its fused Cries and When launched included, spending no launch price ([[R1852003]], [[R1852006]]); then a Discover ([[§6.3]]) of the four War Doctrines, S #137.1 to S #137.4 (by id, or `{ tags: [War Doctrine], withTokens: true }`, the pool [[R1857002]] names), Radiant on the Radiant face, and a `cast` of the pick, free and counted as played ([[R70]]), its Starship line reaching the Starship just launched. With no Dormant Starship, nothing.
- **Rulings:** ST-S02-6: "activate {discover} a War doctrine" is a Discover of one of the four (three offered) and a cast of it, free; both halves only while you are building a ship, and the launch first, so the Doctrine's Starship line finds the ship. "Launch it" spends no "launch costs (N)" price. These are proposals for the S02 part, which takes R-numbers from its own block.
- **Numbers:** none (no number is printed).
- **Check:** The designer's "First strike" is First Strike; "War doctrine" is the tag War Doctrine (F's). R275: stats doubled and a Radiant Doctrine.
- **Class:** C

#### Stinkyscape #137.1 · Land Doctrine
`stinkyscape-137-1` · (4) Spell, War Doctrine, Token · Token (printed Legendary) · built by S02 (#1859)

> **Designer:** (4) Land Doctrine, Legendary, War Doctrine, Spell, Stinkyscape, #137.1 · For the rest of the game your units have trample, your Starship attacks a random enemy unit ~~~ For the rest of the game your units have trample, your Starship attacks all enemy units

- **Text:** For the rest of the game, your Units have Trample. Your Starship attacks a random enemy Unit.
- **Radiant:** For the rest of the game, your Units have Trample. Your Starship attacks each enemy Unit.
- **Engine:** NEW: a rest-of-game player modifier lending Trample to every Unit its caster controls, now and later, a Vanilla not taking it ([[§10.4]]): no such modifier kind is on main ([[R458]]'s rest-of-game modifiers run a hook; none lends a keyword), so S02 builds it. Then, once, as it is cast (ST9 Q2): a forced attack ([[R53]]) by its caster's launched Starship (`launched_starship_of`, [[R1852005]]) on an enemy Unit drawn from the match rng among those it may attack ([[§4.2]] step 2, [[R60]]), the target striking back; with no launched Starship or no such Unit, nothing. The Radiant face attacks every enemy Unit in lane order, each its own combat and state check, stopping once the Starship leaves the field ([[R53]]).
- **Rulings:** ST-S02-7: "your units have X" is every Unit you control for the rest of the game, those that arrive later too, and it stays when the Doctrine is in the graveyard; the Starship line runs once, as it is cast (ST9 Q2); "your Starship" is your launched Starship on the field, the last launched (`launched_starship_of`), and with none the line does nothing. ST-S02-8: "attacks a random enemy unit" draws among the Units it may attack (Flying's rule holding); "all enemy units" is each in lane order, one combat each. These are proposals for the S02 part, which takes R-numbers from its own block.
- **Numbers:** none (no number is printed).
- **Check:** A token: the header prints no Token, but ST1 counts it among the 30 and only #137 makes it (R1857001); the designer's lower-case "trample" is Trample. R275: the Starship line raised.
- **Class:** C

#### Stinkyscape #137.2 · Sea Doctrine
`stinkyscape-137-2` · (4) Spell, War Doctrine, Token · Token (printed Legendary) · built by S02 (#1859)

> **Designer:** (4) Sea Doctrine, Legendary, War Doctrine, Spell, Stinkyscape, #137.2 · For the rest of the game your units have deft, your heal your hero for twice the amount of health as your Starship ~~~ For the rest of the game your units have deft, your heal your hero for five times the amount of health as your Starship

- **Text:** For the rest of the game, your Units have Deft. Heal your hero {multiplier}× your Starship's Health.
- **Radiant:** For the rest of the game, your Units have Deft. Heal your hero {multiplier}× your Starship's Health.
- **Engine:** NEW: a rest-of-game player modifier lending Deft to every Unit its caster controls, now and later, a Vanilla not taking it ([[§10.4]]): no such modifier kind is on main ([[R458]]'s rest-of-game modifiers run a hook; none lends a keyword), so S02 builds it. Then, once, as it is cast (ST9 Q2): `heal` on its caster's hero by multiplier × the Health of their launched Starship ([[R1852005]]) as it stands, damage counted, with no cap on a hero ([[§6.3]] Heal); no launched Starship, no heal.
- **Rulings:** ST-S02-7 (as at S #137.1). ST-S02-9: "twice the amount of health as your Starship" is 2 × its current Health, damage counted. These are proposals for the S02 part, which takes R-numbers from its own block.
- **Numbers:** multiplier 2 ↑ (Radiant 5).
- **Check:** A token: the header prints no Token, but ST1 counts it among the 30 and only #137 makes it (R1857001); the designer's lower-case "deft" is Deft. R275: the Starship line raised.
- **Class:** C

#### Stinkyscape #137.3 · Air Doctrine
`stinkyscape-137-3` · (4) Spell, War Doctrine, Token · Token (printed Legendary) · built by S02 (#1859)

> **Designer:** (4) Air Doctrine, Legendary, War Doctrine, Spell, Stinkyscape, #137.3 · For the rest of the game your units have rush, you deal damage split randomly among all enemies equal to your Starship's attack ~~~ For the rest of the game your units have rush, you deal damage split randomly among all enemies equal to twice your Starship's attack

- **Text:** For the rest of the game, your Units have Rush. Deal damage equal to {multiplier}× your Starship's Attack, split randomly among all enemies.
- **Radiant:** For the rest of the game, your Units have Rush. Deal damage equal to {multiplier}× your Starship's Attack, split randomly among all enemies.
- **Engine:** NEW: a rest-of-game player modifier lending Rush to every Unit its caster controls, now and later, a Vanilla not taking it ([[§10.4]]): no such modifier kind is on main ([[R458]]'s rest-of-game modifiers run a hook; none lends a keyword), so S02 builds it. Then, once, as it is cast (ST9 Q2): [[§6.3]] Damage's split: N hits of 1, N = multiplier × the Attack of its caster's launched Starship ([[R1852005]]), each to a random enemy still standing, hero or Unit, the caster's Spell Damage raising each hit ([[§4.4]] step 0); no launched Starship, nothing. `dealsDamage` ([[R1748001]]).
- **Rulings:** ST-S02-7 (as at S #137.1). ST-S02-10: "damage split randomly among all enemies" is [[§6.3]] Damage's split, that many hits of 1, each to a random enemy still standing. These are proposals for the S02 part, which takes R-numbers from its own block.
- **Numbers:** multiplier 1 ↑ (Radiant 2).
- **Check:** A token: the header prints no Token, but ST1 counts it among the 30 and only #137 makes it (R1857001); the designer's lower-case "rush" is Rush. R275: the Starship line raised.
- **Class:** C

#### Stinkyscape #137.4 · Space Doctrine
`stinkyscape-137-4` · (4) Spell, War Doctrine, Token · Token (printed Legendary) · built by S02 (#1859)

> **Designer:** (4) Space Doctrine, Legendary, War Doctrine, Spell, Stinkyscape, #137.4 · For the rest of the game your units have flying, fill your hand with the Starship Pieces that are part of the launched Starship ~~~ For the rest of the game your units have flying, fill your hand with the Starship Pieces that are part of the launched Starship, they cost (1)

- **Text:** For the rest of the game, your Units have Flying. Fill your hand with copies of the Starship Pieces fused into your Starship.
- **Radiant:** For the rest of the game, your Units have Flying. Fill your hand with copies of the Starship Pieces fused into your Starship. They cost ({setCost}).
- **Engine:** NEW: a rest-of-game player modifier lending Flying to every Unit its caster controls, now and later, a Vanilla not taking it ([[§10.4]]): no such modifier kind is on main ([[R458]]'s rest-of-game modifiers run a hook; none lends a keyword), so S02 builds it. Then, once, as it is cast (ST9 Q2): copies of the pieces its caster's launched Starship remembers (`starship_pieces_of`, [[R1852005]]), each on the face it fused on, oldest first, added until the hand is full, none burned ([[§2.4]]); no launched Starship, nothing. Radiant: each copy's `costOverride` setCost ([[R65]]).
- **Rulings:** ST-S02-7 (as at S #137.1). ST-S02-11: "fill your hand with the Starship Pieces that are part of the launched Starship" is one copy of each piece it remembers, oldest first, until the hand is full; a short list leaves the rest of the hand empty (no repeats). These are proposals for the S02 part, which takes R-numbers from its own block.
- **Numbers:** setCost 1 ↓ (Radiant only, `tunedOn: radiant`, min 0).
- **Check:** A token: the header prints no Token, but ST1 counts it among the 30 and only #137 makes it (R1857001); the designer's lower-case "flying" is Flying. R275: the Starship line raised.
- **Class:** C

#### Stinkyscape #138 · Pocket Peptides
`stinkyscape-138` · (1) Trap · Common · built by S15 (#1875)

> **Designer:** (1) Pocket Peptides, Common, Trap, Stinkyscape, #138 · Reveals when one of your Units is about to die by combat. Cast CN Peptides on it. ~~~ Reveals when one of your Units is about to die by combat. Cast Radiant CN Peptides on all your Units.

- **Text:** Reveals when one of your Units would die in combat: Cast CN Peptides on it.
- **Radiant:** Reveals when one of your Units would die in combat: Cast a Radiant CN Peptides on each of your Units.
- **Engine:** A Trap at [[§4.5]] step 1's would-die, for its controller's Unit marked by combat damage ([[§6.3]] Replacement, [[R460]]), firing there before cards move: `cast_new { defId: meditative-036 }` ([[R70]]) aimed at that Unit ([[R946]]): 50% (luck-based, [[R1440]]) it is Buffed its times, which may lift its Health past its damage so the re-check spares it ([[R460]]), otherwise it is destroyed. Radiant: a Radiant CN Peptides (Lucky 1) cast once at each of its controller's Units, in lane order, the dying one included.
- **Rulings:** ST-S15-7: "about to die by combat" is the would-die point of a Unit combat damage marked ([[§4.5]] step 1), not a Spell's hit or a destroy; a Buff that leaves it alive spares it at the re-check; "on all your Units" is one cast at each, the CN Peptides choosing that Unit.
- **Numbers:** none (CN Peptides' 50% and its count are its own).
- **Check:** The cast is M #36's own text and numbers ([[R70]]). R275: one cast → a Radiant cast at each of your Units (Lucky 1 each).
- **Class:** B.

#### Stinkyscape #139 · Wormhole
`stinkyscape-139` · (2) Trap · Rare · built by S15 (#1875)

> **Designer:** (2) Wormhole, Rare, Trap, Stinkyscape, #139 · Reveals when your opponent casts a beneficial Spell on themselves or a permanent they control. Cast it beneficially for yourself instead. ~~~ Reveals when your opponent casts a beneficial Spell on themselves or a permanent they control. Cast its Radiant version beneficially for yourself instead.

- **Text:** Reveals when your opponent plays a Spell aimed only at their hero or permanents they control: Counter it and cast it for yourself, aimed only at your hero or permanents you control.
- **Radiant:** Reveals when your opponent plays a Spell aimed only at their hero or permanents they control: Counter it, make it Radiant and cast it for yourself, aimed only at your hero or permanents you control.
- **Engine:** A Trap in [[§10.5]] step 3a's announce window ([[R1850004]]): a Spell whose chosen targets, one at least, are all its player's own hero or cards they control on the field (`query::beneficial_on_own_side`) is countered into the Trap's owner's hand as theirs ([[R448]]) and cast by them ([[R70]]) with `friendsOnly`: every target pick of the cast, and every target its text asks for while it is cast, is its caster's own hero or a card they control on the field, a declaration with none passed ([[R90]]). A Spell that chose no target is not beneficial. Radiant: `set_radiant` on the countered card before the cast, so it resolves Radiant and lands Radiant in its new owner's graveyard.
- **Rulings:** ST-S15-8: "a beneficial Spell on themselves or a permanent they control" is [[R1850004]]'s: a Spell whose chosen targets, one at least, are all its player's own hero or cards on the field; one that chose no target is not one.
- **Numbers:** none (the card prints no number).
- **Check:** "beneficial" written as [[R1850004]] reads it. R275: the Spell cast Radiant.
- **Class:** C (M16's cast it for yourself).

#### Stinkyscape #140 · Rush Token Launcher
`stinkyscape-140` · (2) Unit, Weapon · Rare · 0/4 → 0/8 · built by S06 (#1862)

> **Designer:** (2) Rush Token Launcher, Unit, Rare, Weapon, Stinkyscape, #140 · [0/4] / Starship Piece / Equip / Activate 1: Target an enemy, summon a Rush token to attack it *{obey's taunt}* / *{deals half damage to heroes}* ~~~ [0/8] / Starship Piece / Equip / Activate 1: Target an enemy, summon a Radiant Rush token to attack it *{obey's taunt}* / *{deals half damage to heroes}*

- **Text:** Starship Piece, Equip / Activate: Summon a Rush Token that attacks an enemy Unit, Taunt Units first.
- **Radiant:** Starship Piece, Equip / Activate: Summon a Radiant Rush Token that attacks an enemy Unit, Taunt Units first.
- **Engine:** Equip ([[R1086]]): played onto one of your Units it fuses into it, the host keeping its zone and taking its stats, keywords and text ([[R77]], [[R102]]), Starship Piece only into a piece ([[R1852001]]); played alone its death fuses a copy into its owner's Dormant Starship ([[R1852000]]); an equipped host takes the Activate. Activate ([[R384]]): a declared enemy Unit, a Taunt one while the enemy has one ([[§4.2]] step 3 read for the choice), then `summon` the Rush Token ([[§7]]) into the leftmost open unit zone ([[R64]]) and a forced attack by it on that Unit ([[R53]]), as Core #60 Bear Honeypot's tokens attack; with no open zone, nothing. Radiant: a Radiant Rush Token (6/6, Cleave). The designer's "deals half damage to heroes" has nothing to halve, its target being a Unit (ST9).
- **Rulings:** ST-S06-5: "Target an enemy" is an enemy Unit (ST9's default: the token attacks a Unit); "obeys taunt" makes a Taunt Unit the only choice while the enemy has one; "deals half damage to heroes" has nothing to halve. These are proposals for the S06 part, which takes R-numbers from its own block.
- **Numbers:** none (no number is printed).
- **Check:** The designer's notes "{obey's taunt}" and "{deals half damage to heroes}" read as above. R275: the token Radiant.
- **Class:** B

#### Stinkyscape #141 · Fruit Launcher
`stinkyscape-141` · (2) Unit, Weapon · Rare · 0/4 → 0/8 · built by S06 (#1862)

> **Designer:** (2) Fruit Launcher, Unit, Rare, Weapon, Stinkyscape, #141 · [0/4] / Starship Piece / Equip / Activate 1: Target a permanent, cast a random fruit ~~~ [0/8] / Starship Piece / Equip / Activate 1: Target a permanent, cast a random Radiant fruit

- **Text:** Starship Piece, Equip / Activate: Choose a permanent. Cast a random Fruit on it.
- **Radiant:** Starship Piece, Equip / Activate: Choose a permanent. Cast a random Radiant Fruit on it.
- **Engine:** Equip ([[R1086]]): played onto one of your Units it fuses into it, the host keeping its zone and taking its stats, keywords and text ([[R77]], [[R102]]), Starship Piece only into a piece ([[R1852001]]); played alone its death fuses a copy into its owner's Dormant Starship ([[R1852000]]); an equipped host takes the Activate. Activate ([[R384]]): a declared permanent, either side ([[R81]]); `cast_random` from the Fruit pool ([[R382]], the Grapes among it; Organic Produce and Fruit Tree left it, [[R1837000]]), aimed at that permanent where the Fruit takes a target it may be, its other choices random, free and counted as played ([[R70]]). Radiant: the Fruit Radiant.
- **Rulings:** ST-S06-6: "cast a random fruit" on the target is the Fruit aimed at that permanent where its target may be one, its other choices random; a Fruit with no target is simply cast. These are proposals for the S06 part, which takes R-numbers from its own block.
- **Numbers:** none (no number is printed).
- **Check:** "fruit" is the Fruit tag's pool (R382, R1837000). R275: the Fruit Radiant.
- **Class:** B

#### Stinkyscape #142 · Jade Launcher
`stinkyscape-142` · (3) Unit, CN, Weapon · Epic · 0/6 → 0/12 · built by S06 (#1862)

> **Designer:** (3) Jade Launcher, Unit, Epic, CN, Weapon, Stinkyscape, #142 · [0/6] / Starship Piece / Equip / Activate 1: Target an enemy, shoot a Auspicious Rock at it *{deal 2 damage, you gain the reward}* ~~~ [0/12] / Starship Piece / Equip / Activate 2: Target an enemy, shoot a Auspicious Rock at it *{deal 2 damage, you gain the reward}*

- **Text:** Starship Piece, Equip / Activate: Deal {damage} damage to an enemy, then gain the reward an Auspicious Rock gives.
- **Radiant:** Starship Piece, Equip / Activate 2: Deal {damage} damage to an enemy, then gain the reward an Auspicious Rock gives.
- **Engine:** Equip ([[R1086]]): played onto one of your Units it fuses into it, the host keeping its zone and taking its stats, keywords and text ([[R77]], [[R102]]), Starship Piece only into a piece ([[R1852001]]); played alone its death fuses a copy into its owner's Dormant Starship ([[R1852000]]); an equipped host takes the Activate. Activate (Radiant: Activate 2, [[R384]], [[R1751000]]): a declared enemy, Unit or hero, takes the hit the Auspicious Rock (M #39.1) would cost its caster in health (`damage`), then the Rock's reward: one of Dud (20%), Jade (75%) or Red Jade (5%) added to its controller's hand by the Rock's own table and one roll ([[R60]]); no Rock is added or cast.
- **Rulings:** ST-S06-7: "shoot an Auspicious Rock at it" with "{deal 2 damage, you gain the reward}" is the Rock's 2 dealt to the target instead of lost by you, and its reward (Dud 20%, Jade 75%, Red Jade 5%) added to your hand; no Rock is made. These are proposals for the S06 part, which takes R-numbers from its own block.
- **Numbers:** damage 2 ↑.
- **Check:** Tagged CN and Weapon (the designer's). The designer's "a Auspicious Rock" is "an". R275: health doubled and Activate 2 (2×).
- **Class:** B

#### Stinkyscape #143 · Book Launcher
`stinkyscape-143` · (3) Unit, Weapon · Epic · 0/6 → 0/12 · built by S06 (#1862)

> **Designer:** (3) Book Launcher, Unit, Epic, Weapon, Stinkyscape, #143 · [0/6] / Starship Piece / Equip / Activate 1: Target an enemy, cast a random harmful book on it / Activate 1: Target an ally, cast a random helpful book on it ~~~ [0/12] / Starship Piece / Equip / Activate 1: Target an enemy, cast a random Radiant harmful book on it / Activate 1: Target an ally, cast a random Radiant helpful book on it

- **Text:** Starship Piece, Equip / Activate: Cast a random harmful Book on an enemy. / Activate: Cast a random helpful Book on an ally.
- **Radiant:** Starship Piece, Equip / Activate: Cast a random Radiant harmful Book on an enemy. / Activate: Cast a random Radiant helpful Book on an ally.
- **Engine:** Equip ([[R1086]]): played onto one of your Units it fuses into it, the host keeping its zone and taking its stats, keywords and text ([[R77]], [[R102]]), Starship Piece only into a piece ([[R1852001]]); played alone its death fuses a copy into its owner's Dormant Starship ([[R1852000]]); an equipped host takes both. Two Activate abilities, each once a turn and counted apart ([[R1751000]]): a declared enemy (Unit or hero), or a declared ally (your hero or one of your Units), then `cast_random` from the Book pool (the Book tag, [[§5]]) the script splits into harmful Books (a target of the enemy's: damage, Nerf, destroy) and helpful ones (a target of your own: heal, Buff, stats), aimed at the target, free and counted as played ([[R70]]). Radiant: the Book Radiant.
- **Rulings:** ST-S06-8: "harmful" and "helpful" Books are two lists in the script, by whose card a Book's target is (the enemy's or your own); a Book in neither, or with no target, is in neither pool; "an enemy" and "an ally" are a Unit or a hero. These are proposals for the S06 part, which takes R-numbers from its own block.
- **Numbers:** none (no number is printed).
- **Check:** The designer's two "Activate 1" are two Activate abilities (R1751000). R275: the Books Radiant.
- **Class:** B

#### Stinkyscape #144 · Prepare For Liftoff
`stinkyscape-144` · (1) Trap · Common · built by S02 (#1859)

> **Designer:** (1) Prepare For Liftoff, Common, Trap, Stinkyscape, #144 · Tradable / Flashback / Reveals when one of your Starship Pieces dies. Your Starship launch costs (0). ~~~ Tradable / Flashback / Reveals when one of your Starship Pieces dies. Resummon it. Your Starship launch costs (0).

- **Text:** Tradable, Flashback / Reveals when one of your Starship Pieces dies: Your Starship launch costs (0).
- **Radiant:** Tradable, Flashback / Reveals when one of your Starship Pieces dies: Summon it from your graveyard. Your Starship launch costs (0).
- **Engine:** Tradable in the hand ([[R1838000]]); Flashback ([[§6.1]]): set again from its owner's graveyard once. Set face-down, it reveals on the death of a Starship Piece its controller controlled ([[R89]], [[R17]]), then `starship_launch_costs { cost: 0 }` ([[R1852006]]): their next launch by the Activate is free, a badge on both seats. Radiant: first `summon` that piece from its owner's graveyard into the leftmost open unit zone ([[R64]]), a summon with no Cry ([[R1]]), its copy already fused in the Starship staying there.
- **Rulings:** ST-S02-12: "one of your Starship Pieces" is a piece its controller controlled as it died, a stolen one included; "Resummon it" is a summon of that card from its owner's graveyard, no Cry, after its copy has fused, so its next death fuses again; "(0)" is R1852006's price. These are proposals for the S02 part, which takes R-numbers from its own block.
- **Numbers:** none ((0) is the verb's price, as R1852006 quotes it).
- **Check:** R275: the piece back as well.
- **Class:** C

#### Stinkyscape #145 · Fortify Flesh
`stinkyscape-145` · (4) Spell · Rare · built by S13 (#1867)

> **Designer:** (4) Fortify Flesh, Spell, Rare, Stinkyscape, #145 · Gain 3~5 Armor ~~~ Gain 3~7 Armor / Lucky 1

- **Text:** Your hero gains a random {least} to {most} Armor.
- **Radiant:** Lucky 1 / Your hero gains a random {least} to {most} Armor.
- **Engine:** One luck-based roll from the match rng, uniform from 3 to 5, both ends included (Radiant 3 to 7, its Lucky 1 rolling once more and keeping the higher, [[§6.1]] Lucky X, [[R1438]]), then `gain_hero_armor { amount }`: the hero's Armor, [[§4.4]] step 2's reduction of every hit on it by that much, kept for the rest of the game and stacking with any other Armor (C+ #46's, [[R124]]).
- **Rulings:** ST-S13-10: "Gain 3~5 Armor" is the hero's Armor, a reduction of each hit kept for the game, as all Armor here is ([[§6.1]] Armor X), not Hearthstone's pool; one roll from 3 to 5, both ends included; a Buff that lifts the low end past the high one rolls between the two.
- **Numbers:** least 3 ↑; most 5 ↑ (Radiant 7).
- **Check:** ⚠ designer: Armor here reduces every hit for the game (Hearthstone's is a pool), so 3 to 5 is strong at (4). R275: below, the mean 4 against about 5.8 with Lucky 1 (1.45×) and the top 5 against 7; the designer's numbers kept until S13 or the designer settles it.
- **Class:** A.

#### Stinkyscape #146 · Flash Freeze
`stinkyscape-146` · (0) Spell · Rare · built by S13 (#1867)

> **Designer:** (0) Flash Freeze, Spell, Rare, Stinkyscape, #146 · Cast on draw: Freeze all permanents ~~~ Cast on draw: Freeze all enemy permanents

- **Text:** Cast on draw: Freeze all Units.
- **Radiant:** Cast on draw: Freeze all enemy Units.
- **Engine:** Cast on draw ([[§6.2]]): drawn, it is cast at no cost and counted as played, and its player draws again ([[R40]], [[R70]], [[R58]]'s chain cap); set aside through the setup ([[R635]]). Played from the hand at (0) it does the same. `freeze_all` over every Unit of both sides ([[R1174000]]; Radiant: the enemy's), a hero passed by and a Unit Immune to Spells too ([[R81]]).
- **Rulings:** ST-S13-11: "Freeze all permanents" Freezes every Unit, the only permanents a Freeze reaches ([[§6.1]] Frozen); drawn at the start of its player's turn, their own Units lose that turn's attack.
- **Numbers:** none (the card prints no number).
- **Check:** R275: your side spared (both sides to one).
- **Class:** A.

#### Stinkyscape #147 · IN33DH3
`stinkyscape-147` · (0) Unit, Catalyst, Human · Legendary · 1/2 → 2/4 · built by S12 (#1874)

> **Designer:** (0) IN33DH3, Unit, Legendary, Catalyst, Human, Stinkyscape, #147 · [1/2] / Cry: For the rest of your turn an AI will help you make the Pareto Optimal decisions / Death: Shuffle IN33DH3 Prime into your deck ~~~ [2/4] / Cry: For the rest of your turn an AI will help you make the Pareto Optimal decisions / Death: Shuffle a Radiant IN33DH3 Prime into your deck

- **Text:** Cry: For the rest of your turn, an AI helps you make the Pareto Optimal decisions. / Death: Shuffle an IN33DH3 Prime into your deck.
- **Radiant:** Cry: For the rest of your turn, an AI helps you make the Pareto Optimal decisions. / Death: Shuffle a Radiant IN33DH3 Prime into your deck.
- **Engine:** Cry ([[R1]]): `pareto_hint` ([[R1844001]]): on its caster's own turn, until that turn's cleanup, their own view marks each hand card ME-PARETO's judge ([[R1125]]) would call optimal to play now (`paretoOptimal: true`), plays only, scored on their concealed copy ([[R222]]); on the opponent's turn nothing. Catalyst: Death shuffles S #147.1 IN33DH3 Prime into its owner's deck (C+ #38's shape, [[R1421]]), a Radiant one on its Radiant face.
- **Rulings:** ST-S12-3: "For the rest of your turn" is the caster's own turn only (R1844001); a hint never lights an attack, an activation or End turn. These are proposals for S12, which takes R-numbers from its own block. No R-number here.
- **Numbers:** none.
- **Check:** R275: stats doubled and the Prime Radiant.
- **Class:** C (the Pareto hint, M10).

#### Stinkyscape #147.1 · IN33DH3 Prime
`stinkyscape-147-1` · (4) Unit, Prime, Human, Token · Token (printed Legendary) · 9/5 → 18/10 · built by S12 (#1874)

> **Designer:** (4) IN33DH3 Prime, Unit, Legendary, Prime, Human, Token, Stinkyscape, #147.1 · [9/5] / Cry: Summon a Wardrum Blob ~~~ [18/10] / Cry: Summon a Radiant Wardrum Blob

- **Text:** Cry: Summon a Wardrum Blob.
- **Radiant:** Cry: Summon a Radiant Wardrum Blob.
- **Engine:** Cry ([[R1]]): `summon { defId: stinkyscape-147-2 }`, S #147.2 Wardrum Blob, into its controller's leftmost open backrow zone ([[R64]]); none with no zone. Radiant: a Radiant one. A Prime: S #147's Death puts it in the deck; once the set ships ([[R1420]]), any Prime pool ([[R1421]]).
- **Rulings:** ST-S12-4: the Blob is a Field Spell, summoned into the backrow, as M #71's The Cane is. These are proposals for S12, which takes R-numbers from its own block. No R-number here.
- **Numbers:** none.
- **Check:** R275: stats doubled and the Blob Radiant.
- **Class:** A (one summon).

#### Stinkyscape #147.2 · Wardrum Blob
`stinkyscape-147-2` · (2) Field Spell, Token · Token (printed Legendary) · built by S12 (#1874)

> **Designer:** (2) Wardrum Blob, Field Spell, Legendary, Token, Stinkyscape, #147.2 · Aura: An AI will always help you make the Pareto Optimal decisions / Start of Turn: Do something helpful *{should read board state and be a quasi zeph and do actions for no additional cost}* ~~~ Aura: An AI will always help you make the Pareto Optimal decisions / Start of Turn: Do something REALLY helpful *{should read board state and be a quasi zeph and do actions for no additional cost}*

- **Text:** Aura: An AI always helps you make the Pareto Optimal decisions. / Start of turn: Do something helpful.
- **Radiant:** Aura: An AI always helps you make the Pareto Optimal decisions. / Start of turn: Do something REALLY helpful.
- **Engine:** `hintsPlays` ([[R1844001]]): during each of its controller's turns while it acts on their field, their own view marks the hand cards the judge calls optimal. Start of turn ([[R62]]): `helpful_cast` ([[R1844002]]): the perfect pool, every non-token card of every set that ships but its own ids ([[R1446]], [[R387]]), ranked for its caster as C+ #27 ranks a hand ([[R416]]), its top card cast free as a new card ([[R70]]), its choices its caster's. Radiant: `radiant: true`, the pool ranked by its Radiant faces and the pick cast Radiant.
- **Rulings:** ST-S12-5: "REALLY helpful" is `radiant: true` (M10's open question 3); one cast, not two (R1844002). These are proposals for S12, which takes R-numbers from its own block. No R-number here.
- **Numbers:** none.
- **Check:** The designer's {quasi zeph} note is R1844002's `helpful_cast`. R275: the free cast Radiant.
- **Class:** C (the AI at the wheel, M10).

#### Stinkyscape #148 · Alteran Reach
`stinkyscape-148` · (1) Field Spell, Wincon, Quickdraw · Mythic · built by S23 (#1871)

> **Designer:** (1) Alteran Reach, Field Spell, Mythic, Wincon, Quickdraw, Stinkyscape, #148 · Indestructable / Activate Infinite (cost 1 mana): Traverse the endless expanse to get closer to the edge of the universe / *{Open up a Pseudo Starscape Map wherein the player can traverse, there should be many nodes and to reach the edge of the universe it should take at least 20 travels, there should be rewards along the way, think Reach to the Summit}* ~~~ Indestructable / Your first Traversal per turn is Free / Activate Infinite (cost 1 mana): Traverse the endless expanse to get closer to the edge of the universe

- **Text:** Indestructible / Activate ♾️: Pay (1) to travel one node along your starscape map and gain its reward. / The 20th node is the edge of the universe: reaching it adds an Edge of the Universe to your hand.
- **Radiant:** Indestructible / Your first travel each turn is free. / Activate ♾️: Pay (1) to travel one node along your starscape map and gain its reward. / The 20th node is the edge of the universe: reaching it adds an Edge of the Universe to your hand.
- **Engine:** Indestructible ([[§6.1]]); Quickdraw ([[R640]]). The starscape map ([[§6.3]], [[R1847000]]): the card's `starscape_abilities`, an Activate ♾️ for `STARSCAPE_TRAVEL_MANA` (1) whose `travel_starscape` moves its controller one node, reports `starscapeTravelled` and gives the node's reward ([[R1847001]]): Heal 3, Draw 1, Mana 1 or Damage 2 (`STARSCAPE_REWARDS`), drawn as the player's map is charted at their first travel and public on both seats. The map is the player's, so a second Alteran Reach goes on from where they stand. The 20th travel (`STARSCAPE_EDGE_TRAVELS`) adds S #148.1 Edge of the Universe on its base face, a full hand burning it, and nothing travels past the edge. Radiant: the player's first travel each turn is free, the free ability and the paid one switching ([[R1847002]]).
- **Rulings:** ST-S23-1: "Traverse the endless expanse" is one travel of [[R1847001]]; the designer's "rewards along the way, think Reach to the Summit" are the map's nodes ([[R1847000]]).
- **Numbers:** none (the engine's price and map).
- **Check:** "Indestructable" is Indestructible (ST5 D6). R275: the first travel each turn free (a tangential rider).
- **Class:** C (M13, the starscape map).

#### Stinkyscape #148.1 · Edge of the Universe
`stinkyscape-148-1` · (0) Spell, Wincon, Token · Token (printed Mythic) · built by S23 (#1871)

> **Designer:** (0) Edge of the Universe, Spell, Mythic, Token, Wincon, Stinkyscape, #148.1 · Escape the Universe and Win the Game / {Your opponent does not lose any elo since you fled} ~~~ Escape the Universe and Win the Game / {You may choose if your opponent loses elo since you fled}

- **Text:** Escape the Universe: you win the game, and it is a draw for your opponent.
- **Radiant:** Escape the Universe: you win the game, and you choose whether it is a draw or a loss for your opponent.
- **Engine:** Escape the Universe ([[§6.3]], `flee`, [[R1843002]]): its caster's `fled`, taking hold at the state check that closes its list, a hero at 0 losing first ([[R850]]); the game ends `fled`, a win for the fleer and a draw for the other player on the ladder, the record and every seat's result ([[R1843004]]). Radiant: a `mode` prompt as it resolves, a draw or a loss for the opponent (`fled-decisive`, [[R1843003]]). S #148's map adds it at its edge ([[R1847001]]).
- **Rulings:** none new: ST9 Q10 is [[R1843002]] to [[R1843004]].
- **Numbers:** none.
- **Check:** ST9 Q10: fled. R276: the faces differ by the Radiant choice. R275: a loss for the opponent on the ladder (a rider).
- **Class:** C (M09, fled).

#### Stinkyscape #149 · Wildar
`stinkyscape-149` · (3) Spell · Legendary · built by S25 (#1882)

> **Designer:** (3) Wildar, Spell, Legendary, Stinkyscape, #149 · Add a Call to Chaos that costs (0) to your opponents hand then add a Radiant Call to Chaos to your hand ~~~ Add a Radiant Call to Chaos to your hand

- **Text:** Add a random Call to Chaos to your opponent's hand. It costs (0). Then add a random Radiant Call to Chaos to your hand.
- **Radiant:** Add a random Radiant Call to Chaos to your hand.
- **Engine:** Two picks from the Call to Chaos pool (the cards tagged Call to Chaos; a pool that names no set holds the editions that ship, [[R380]], and S #150 and S #151 join it with their set), each drawn from the match rng ([[R60]]): the first goes to the opponent's hand as theirs at `costOverride` 0, the edition hidden from its caster ([[R97]]); then the second to its caster's hand, Radiant ([[§5.2]]), at its own cost. Radiant: only the caster's. A full hand burns ([[§2.4]]).
- **Rulings:** ST-S25-1: "a Call to Chaos" is a random edition, the two picks independent; the caster's costs its printed (4).
- **Numbers:** none.
- **Check:** The list's "opponents hand" is your opponent's hand. R275: the Radiant face drops the free Call to Chaos the base face gives the opponent.
- **Class:** B.

#### Stinkyscape #150 · Call to Chaos (Stinky Edition)
`stinkyscape-150` · (4) Spell, Call to Chaos · Legendary · built by S25 (#1882)

> **Designer:** (4) Call to Chaos (Stinky Edition) Spell, Call to Chaos, Stinkyscape, Legendary, #150 · ??? / *(One of the following random effects)* /  / - Add Radiant ETHICS and Vanguard Coalition to your hand / - Add a Very Dandy Lad and an Arctic Citadel to your hand, they cost (0) / - Summon 3 random Quests or Wincons. / - Add an Acclaimed Stinkyscape Unit to your hand. Refresh mana so you can play it. / - Place all enemies in attack position and Freeze them. / - Fill your board with random Human Stinkyscape Units. / - Allure all enemy Units. / - For the rest of the game, all Acclaimed cards have “Death: Cast a random Call to Chaos”. / - Summon a Stinkyscape Golem / - Cast a random Call to Chaos ~~~ !!! / *(three effects)*

- **Text:** One random effect: Add a Radiant ETHICS and a Radiant Vanguard Coalition to your hand; add A Very Dandy Lad and an Arctic Citadel to your hand, which cost (0); summon 3 random Quest or Wincon Field Spells into your backrow; add a random Acclaimed Stinkyscape Unit to your hand and refresh as much mana as it costs; switch every enemy Unit to Attack Position and Freeze them; fill your board with random Human Stinkyscape Units; Allure every enemy Unit; for the rest of the game, whenever an Acclaimed card dies, its controller casts a random Call to Chaos; summon a Stinkyscape Golem; cast a random Call to Chaos.
- **Radiant:** Three different random effects, resolved in the order listed: Add a Radiant ETHICS and a Radiant Vanguard Coalition to your hand; add A Very Dandy Lad and an Arctic Citadel to your hand, which cost (0); summon 3 random Quest or Wincon Field Spells into your backrow; add a random Acclaimed Stinkyscape Unit to your hand and refresh as much mana as it costs; switch every enemy Unit to Attack Position and Freeze them; fill your board with random Human Stinkyscape Units; Allure every enemy Unit; for the rest of the game, whenever an Acclaimed card dies, its controller casts a random Call to Chaos; summon a Stinkyscape Golem; cast a random Call to Chaos.
- **Engine:** Call to Chaos ([[§8]]'s family, Legendary): one entry drawn from the match rng and named to both players before it resolves ([[R436]]); Radiant: three different entries in list order ([[R1023]]'s draw). The entries: (1) S #159 ETHICS and S #95 Vanguard Coalition added Radiant; (2) S #113 A Very Dandy Lad and S #114 Arctic Citadel added at `costOverride` 0; (3) three picks, repeats allowed, from the Field Spells tagged Quest or Wincon (`anyTags`, [[R1422]]: C #90 In Too Deep, S #7, S #112 and S #148, the set's by name while it is held), summoned into its caster's backrow where a zone is open ([[R64]]); (4) a random Acclaimed Unit of the set (a pool that names Stinkyscape, [[R1420]]) added, then Refresh by its cost ([[§6.3]], never past max mana, [[R364]]); (5) every enemy Unit switched to Attack Position (an effect's switch, [[R20]]), then Frozen ([[§6.3]] Freeze); (6) `fill_board_random` with the set's Human Units ([[R1080]]); (7) `allure_enemy_units` ([[R963]]); (8) NEW (S25): a rest-of-game rule from this cast: whenever an Acclaimed card dies, either player's, its controller casts a random Call to Chaos, as a granted Death would ([[R1107]]); (9) a Stinkyscape Golem summoned; (10) a random Call to Chaos cast, the chain bounded by `RANDOM_CAST_CHAIN_CAP` ([[R452]]).
- **Rulings:** ST-S25-2: "Summon 3 random Quests or Wincons": only the Field Spells among them can be summoned (Reach the Summit and Aestheticize the Game are Spells). ST-S25-3: "Refresh mana so you can play it" refreshes as much as the card costs. ST-S25-4: "all enemies" are the enemy Units (Freeze passes a hero by). ST-S25-5: "all Acclaimed cards have Death: Cast a random Call to Chaos" covers both players' Acclaimed cards for the rest of the game, the cast its controller's; it prints no quoted label ([[R1445]]). ST-S25-6: "!!! (three effects)" is three different entries in list order, as every Call to Chaos's Radiant face.
- **Numbers:** none (the engine's table).
- **Check:** The list's "???" and "!!!" are "One random effect" and "Three different random effects". R275: three effects against one (3×).
- **Class:** C (ten entries; a rest-of-game Death rule).

#### Stinkyscape #150.1 · Stinkyscape Golem
`stinkyscape-150-1` · (4) Unit, Token · Token (printed Legendary) · 10/10 → 20/20 · built by S25 (#1882)

> **Designer:** (4) Stinkyscape Golem Unit, Token, Stinkyscape, Legendary, #150.1 · 10/10 / Starpiece, Flying, Ranged, Rush ~~~ 20/20 / Starpiece, Flying, Ranged, Rush

- **Text:** Starship Piece, Flying, Ranged, Rush
- **Radiant:** Starship Piece, Flying, Ranged, Rush
- **Engine:** Keywords only: Starship Piece ([[§6.1]], [[R1852000]]: its death fuses a copy into its owner's Dormant Starship), Flying, Ranged ([[R783000]]) and Rush. S #150's golem, as Core #95.1 Chaos Golem is #95's.
- **Rulings:** none.
- **Numbers:** none.
- **Check:** "Starpiece" is Starship Piece (ST5 D6). R275: 20/20 against 10/10.
- **Class:** A.

#### Stinkyscape #151 · Call to Chaos (Scape Edition)
`stinkyscape-151` · (4) Spell, Call to Chaos · Legendary · built by S25 (#1882)

> **Designer:** (4) Call to Chaos (Scape Edition) Spell, Call to Chaos, Stinkyscape, Legendary, #151 · ??? / *(One of the following random effects)* /  / - Trigger any Anomaly / - Summon 3 Drone Corvettes / - Summon the Drone Battleship / - Cast two of the following: Core System, Secure System, Unsecure System, Wild System / - Add 4 random Starship Pieces to your hand, they cost 0 / - Fill your board with Drone Interceptors, they have stack / - Launch a ship with 4 Random Starpieces fused into it / - Add 2 Radiant Weapons to your hand they cost (0) / - For the rest of the game the Marketplace offers you a Call to Chaos at a very cheap rate *(always 20 Credits)* / - Cast a random Call to Chaos ~~~ !!! / *(three effects)*

- **Text:** One random effect: Replace your side of the field with a random Anomaly, any of the ten; summon 3 Drone Corvettes; summon the Drone Battleship; cast 2 different cards of Core System, Secure System, Unsecure System and Wild System, picked at random; add 4 random Starship Pieces to your hand, which cost (0); summon a Drone Interceptor into each of your unit zones, on top of any Unit there; fuse 4 random Starship Pieces into your Dormant Starship, then launch it; add 2 random Radiant Weapons to your hand, which cost (0); for the rest of the game, the Marketplace offers you a Call to Chaos for 20 Credits each cycle; cast a random Call to Chaos.
- **Radiant:** Three different random effects, resolved in the order listed: Replace your side of the field with a random Anomaly, any of the ten; summon 3 Drone Corvettes; summon the Drone Battleship; cast 2 different cards of Core System, Secure System, Unsecure System and Wild System, picked at random; add 4 random Starship Pieces to your hand, which cost (0); summon a Drone Interceptor into each of your unit zones, on top of any Unit there; fuse 4 random Starship Pieces into your Dormant Starship, then launch it; add 2 random Radiant Weapons to your hand, which cost (0); for the rest of the game, the Marketplace offers you a Call to Chaos for 20 Credits each cycle; cast a random Call to Chaos.
- **Engine:** Call to Chaos ([[§8]]'s family, Legendary): one entry drawn from the match rng and named to both players before it resolves ([[R436]]); Radiant: three different entries in list order ([[R1023]]'s draw). The entries: (1) #1855's board swap (pending): its caster's side replaced by one of the ten Anomaly boards (S20's data), every laid Cry fired with random choices, as Anomaly Found! does (ST9 Q7); (2) three Drone Corvettes summoned ([[R64]]); (3) the Drone Battleship, a layered summon (`summon_pile`, [[R1842000]]: Meltdown at the bottom, then Propagation, Shield on top); (4) two different Systems of S #117 to #120 drawn from the rng ([[R60]]) and cast, their choices the caster's ([[R70]]); (5) four random Starship Pieces (the non-token cards printing the keyword; NEW unless S03 or S19 builds that pool by keyword kind) added at `costOverride` 0; (6) NEW (S25): a Drone Interceptor summoned into each of its caster's five unit zones, an occupied one taking it on top as a Stack play would ([[§3.2]]'s pile), a Locked one too ([[R688]]); (7) four random Starship Pieces fused into its caster's Dormant Starship, one summoned first if they have none ([[R1852000]]), then awakened, which launches it for nothing ([[R1852003]], [[R1852006]]): every fused Cry, then every When launched; (8) two random Radiant Weapons (the Weapon tag, [[R1857000]]) at `costOverride` 0; (9) #1853's Marketplace (pending PR #1963): a lot of one Call to Chaos drawn as it is added, its caster's alone, at a fixed 20 Credits, one copy each cycle for the rest of the game, opening the Marketplace if it is closed; (10) a random Call to Chaos cast ([[R452]]).
- **Rulings:** ST-S25-7: "Trigger any Anomaly" is the board swap with a board from all ten, rich ones included; it makes no Anomaly Found! card. ST-S25-8: "they have stack" is read as stacking: a Drone Interceptor tops each of its caster's unit zones, any Unit there beneath it (the Stack keyword alone would do nothing on the field). ST-S25-9: "Launch a ship with 4 Random Starpieces fused into it" fuses into its caster's Dormant Starship, a new one when they have none, and launches it for free. ST-S25-10: the Marketplace lot is its caster's alone, never Radiant, its 20 fixed.
- **Numbers:** none (the engine's table).
- **Check:** "Starpieces" are Starship Pieces (ST5 D6); "stack" read as stacking (⚠ designer, E13). R275: three effects against one (3×).
- **Class:** C (M02, M04, M06, M08).

#### Stinkyscape #152 · Sanctum
`stinkyscape-152` · (4) Field Spell · Legendary · built by S16 (#1869)

> **Designer:** (4) Sanctum, Field Spell, Stinkscape, Legendary, #152 · Indestructable / Aura: all cards in your Deck and GY Flashback ~~~ Indestructable / Cry: gain (4) mana / Aura: all cards in your Deck and GY Flashback

- **Text:** Indestructible / Aura: Every card in your deck and graveyard has Flashback.
- **Radiant:** Indestructible / Cry: Gain {mana} mana. / Aura: Every card in your deck and graveyard has Flashback.
- **Engine:** Indestructible. NEW: an aura lending Flashback ([[§6.1]], [[R783012]]) to every card in its controller's deck and graveyard while it acts on the field, read in those zones as a hand aura is ([[R1194000]]), so each card in the graveyard is offered as a play from there at its cost; a played one's `flashbackSpent` keeps the aura off it ([[R783013]]). Radiant: Cry ([[R1]]): gain 4 mana this turn ([[§6.3]] Mana).
- **Rulings:** ST-S16-14: a card plays from the graveyard by it once, its spent Flashback riding the instance (R783013); the deck's cards have it only for when they get there. ST-S16-15: "gain (4) mana" is a gain of 4 this turn. These are proposals for S16, which takes R-numbers from its own block. No R-number here.
- **Numbers:** mana 4 ↑, on the Radiant face only (`tunedOn: radiant`, its Cry's).
- **Check:** D6: "Indestructable" is Indestructible; "Stinkscape" Stinkyscape. R275: 4 mana on arrival repays its own cost.
- **Class:** B (an aura granting a keyword), with NEW reach into the deck and graveyard.

#### Stinkyscape #153 · Stimmycheck
`stinkyscape-153` · (0) Spell · Legendary · built by S14 (#1868)

> **Designer:** (0) Stimmycheck, Spell, Stinkscape, Legendary, #153 · Gain 1 mana, cards in your hand and deck have a 15% chance for Inflation to hit which causes them to cost (1) more. At the end of your turn, add a Stimmycheck to your hand with a +10% chance at Inflation ~~~ Lucky 1 / Gain 1 mana, cards in your hand and deck have a 15% chance for Inflation to hit which causes them to cost (1) more. At the end of your turn, add a Radiant Stimmycheck to your hand with a +10% chance at Inflation

- **Text:** Gain {mana} mana. Inflation: each card in your hand and deck has a {chance}% chance to cost (1) more. / At the end of this turn, add a Stimmycheck to your hand. Its Inflation chance goes up by 10%.
- **Radiant:** Lucky 1 / Gain {mana} mana. Inflation: each card in your hand and deck has a {chance}% chance to cost (1) more. / At the end of this turn, add a Radiant Stimmycheck to your hand. Its Inflation chance goes up by 10%.
- **Engine:** `gain_mana { amount: 1 }` (this turn's mana, [[§6.3]] Mana), then `inflate { chance, player: self }` ([[R1848006]]): one luck-based roll per card, hand in order and then the deck from the top, out of `INFLATION_ROLL_SIDES` (100), a hit adding `INFLATION_COST` (1) to its `costMod`; an X card and the resolving Stimmycheck are passed by; silent ([[R440]]). A card carrying its own chance (`inflationChance`) rolls at it. At this turn's end a delayed add (`delay`): `add_to_hand { defId: stinkyscape-153, memory: { inflationChance: next_inflation_chance(chance) } }`, 10 higher and never past 100. Radiant: Lucky 1 rolls each card once more and keeps the better, a miss on its caster's own cards ([[R1440]]), and the copy is Radiant.
- **Rulings:** ST-S14-11: "At the end of your turn" is the end of this turn, a delayed add, not an End of turn trigger (the card is in the graveyard by then); "+10% chance" is 10 points higher ([[R1848006]]), never past 100, carried on the copy as its own chance.
- **Numbers:** mana 1 ↑; chance 15 ↓ (step 4, max 100: a lower one spares more of your cards); the copy's +10 and the (1) are the engine's (`INFLATION_CHANCE_STEP`, `INFLATION_COST`).
- **Check:** The designer's "Stinkscape" is Stinkyscape (ST5 D6). R275: Lucky 1 on the rolls (fewer of your cards hit, [[R1440]]) and a Radiant copy, a rider.
- **Class:** C (M14's Inflation).

#### Stinkyscape #154 · Dice of Doom
`stinkyscape-154` · (0, Embiggened 1) Spell, Quickdraw · Mythic · built by S22 (#1870)

> **Designer:** (0 embiggen 1) Dice of Doom, Spell, Stinkyscape, Quickdraw, Mythic, #154 · Roll a 100 sided die, if it lands on 67 add every Terminus card to your hand. They cost (0). End of your turn add this back to your hand. / Embiggen permanently gain Lucky +1 ~~~ Roll a 80 sided die, if it lands on 67 add every Terminus card to your hand. They cost (0). End of your turn add this back to your hand. / Embiggen permanently gain Lucky +1

- **Text:** Roll a 100-sided die. If it lands on 67, add every Terminus card to your hand. They cost (0). / End of turn: Add this back to your hand. / Embiggened (1): This also gains Lucky +1 for good.
- **Radiant:** Roll an 80-sided die twice. If either lands on 67, add every Terminus card to your hand. They cost (0). / End of turn: Add this back to your hand. / Embiggened (1): This also gains Lucky +1 for good.
- **Engine:** An embiggen card ([[§6.3]] Embiggen, [[R1178001]]): (0), Embiggened (1). A luck-based roll ([[§6.1]] Lucky) of 1 to 100 from the match rng, a hit on 67, the card's Lucky rolling again and any 67 a hit ([[R1440]]); Radiant: two rolls of 1 to 80, either a hit. A hit adds one new card of each Terminus definition (S #4.1–#4.6 and Core #100 Ceaseless Void, [[R1857002]]) at `costOverride` 0, a full hand burning the rest ([[§2.4]]). End of turn: the card goes from its owner's graveyard back to their hand, an "add this back to hand" Spell ([[R62]]) as M #8 Reach the Summit is, a full hand burning it. Embiggened: the instance first gains Lucky +1, kept in every zone (Lucky X, given, [[R1438]]), so every Embiggened play makes it luckier for good. Quickdraw ([[R640]]).
- **Rulings:** ST-S22-15: the Embiggened play gains its Lucky before it rolls, so it rolls with it. ST-S22-16: "End of your turn add this back to your hand" is its controller's end of turn after a play, from the graveyard: the move [[R746]] calls Return to hand, printed "Add this back" as the list says it. ST-S22-17: "every Terminus card" is one of each of the seven, never more.
- **Numbers:** none (the die and its face are the frame).
- **Check:** "End of your turn" is the End of turn label (R366). R275: the designer's 80-sided die alone is 1.25× the chance; the Radiant face rolls it twice, about 2.5× (raised, E14).
- **Class:** B (a luck-based roll, a back-to-hand Spell, a given Lucky).

#### Stinkyscape #155 · Comet Crash
`stinkyscape-155` · (2) Field Spell · Rare · built by S04 (#1861)

> **Designer:** (2) Comet Crash, Field Spell, Rare, Stinkyscape, #155 · Crater / Brittle 1 / Death: Add 3 Stinkyscape resources to your hand. ~~~ Crater / Brittle 1 / Death: Add 3 Radiant Stinkyscape resources to your hand.

- **Text:** Crater, Brittle 1 / Death: Gain {resources|random Stinkyscape Resource|random Stinkyscape Resources}.
- **Radiant:** Crater, Brittle 1 / Death: Gain {resources|random Radiant Stinkyscape Resource|random Radiant Stinkyscape Resources}.
- **Engine:** Crater on a Field Spell ([[R1850006]]): it Locks its backrow zone as it enters, any way it enters ([[R688]], [[§6.3]] Lock). Brittle 1 ([[§6.1]], [[R385]], [[R638]]): it crumbles at its controller's first start of turn two or more turns after it arrived. Death, a backrow card's ([[§6.2]]), on any destroy, Brittle's included: `gain_resources` of three random Resources into its controller's tray ([[R1840000]]). Radiant: Radiant Resources.
- **Rulings:** none new: every reading is a ruling on `main` or the card's text.
- **Numbers:** resources 3 ↑.
- **Check:** Crater on a Field Spell Locks its backrow zone (ST8, [[R1850006]]); "Add 3 … to your hand" is a gain into the tray ([[R1840000]]). R275: Radiant Resources.
- **Class:** C.

#### Stinkyscape #156 · Catalyst Comet
`stinkyscape-156` · (1) Spell · Epic · built by S13 (#1867)

> **Designer:** (1) Catalyst Comet, Epic, Spell, Stinkyscape, #156 · Add a Catalyst card to your hand. Shuffle this back into your deck. ~~~ Add a Radiant Catalyst card to your hand. Shuffle back into your deck.

- **Text:** Add a random Catalyst card to your hand. Shuffle this into your deck.
- **Radiant:** Add a random Radiant Catalyst card to your hand. Shuffle this into your deck.
- **Engine:** One pick from the match rng ([[R60]]) among the Catalyst cards of the sets that ship (`add_random_from_catalog { tags: [Catalyst] }`, non-token cards, [[R380]]: C+ #38, C+ #46, M #45, M #49, M #91 and M+ #16, and S #147 once the set ships), a full hand burning it ([[§2.4]]). Then the Comet itself, once it has resolved, goes into its owner's deck at a random position instead of staying in the graveyard (`shuffle_card_into`, C #30 Recycle's verb, [[R80]], [[R316]]: a full deck leaves it in the graveyard). Radiant: the card Radiant; the Comet goes back as it is, Radiant.
- **Rulings:** ST-S13-12: "a Catalyst card" is a random one, no choice offered; "Shuffle this back into your deck" is the Comet itself, not a copy, moved after it resolves, so it keeps its Radiant flag ([[R78]]).
- **Numbers:** none (one card).
- **Check:** The designer's Radiant "Shuffle back into your deck" keeps "this". R275: the card Radiant.
- **Class:** B.

#### Stinkyscape #157 · The Gauntlet
`stinkyscape-157` · (1) Field Spell · Epic · built by S16 (#1869)

> **Designer:** (1) The Gauntlet, Field Spell, Epic, Stinkyscape, #157 · Brittle 3 / The middle 3 unit lanes are trapped in a Gauntlet. No other unit can target them and they can only target each other. ~~~ Brittle 3 / The middle 3 unit lanes are trapped in a Gauntlet. No other unit can target them and they can only target each other. / Activate: Choose to either to add Brittle +2 to this or Tribute this

- **Text:** Brittle 3 / The unit zones of the middle 3 lanes are trapped in a Gauntlet. Units there can attack only each other, and no other Unit can attack them.
- **Radiant:** Brittle 3 / The unit zones of the middle 3 lanes are trapped in a Gauntlet. Units there can attack only each other, and no other Unit can attack them. / Activate: Choose one: give this +{brittleGain} Brittle, or Tribute this.
- **Engine:** Brittle 3 ([[§6.1]]). `staticFlags.gauntlet` ([[R1849005]]): the unit zones of lanes 2 to 4 on both sides (`GAUNTLET_LANES`); at [[§4.2]] step 2 a Unit in them may attack only an enemy Unit in them, never a hero, and a Unit outside attacks none of them, forced attacks included; a Taunt binds only the attackers that may attack it. Attacks only: a Unit's text still targets across it. Radiant: Activate ([[R384]]), two modes ([[R81]]): +2 Brittle (`gain_brittle`), or Tribute this ([[R101]]).
- **Rulings:** ST-S16-16: "target" is an attack's target (R1849005). ST-S16-17: "add Brittle +2" raises the count by 2. These are proposals for S16, which takes R-numbers from its own block. No R-number here.
- **Numbers:** brittleGain 2 ↑, on the Radiant face only (`tunedOn: radiant`). Brittle 3 is a numbered keyword and the 3 lanes are the engine's table (`FIXED` KEYWORD; ENGINE too).
- **Check:** Typo "either to add" read as either add. R275: an Activate to keep it or end it at will.
- **Class:** C (M15's Gauntlet lanes).

#### Stinkyscape #158 · Stratos Swarm
`stinkyscape-158` · (3) Spell · Rare · built by S03 (#1860)

> **Designer:** (3) Stratos Swarm, Spell, Rare, Stinkyscape, #158 · Fill your board with Startos ~~~ Fill your board with Radiant Startos

- **Text:** Fill your board with Startos.
- **Radiant:** Fill your board with Radiant Startos.
- **Engine:** `fill_board` with S #68 Startos (Core #62 Friend of Felinors' verb): every open unit zone of its caster's, a Dormant card's zone not one ([[R1839000]]), each a summoned card of its own definition, no Cry ([[R1]]), no token; Radiant `radiant: true`. Each is a Starship Piece, so each that dies fuses ([[R1852000]]).
- **Rulings:** ST-S03-9: "Startos" is S #68 (ST8's spelling); a full board summons none. These are proposals for the S03 part, which takes R-numbers from its own block.
- **Numbers:** none (no number is printed).
- **Check:** ST8: Startos, as the Unit prints it. R275: Radiant Startos.
- **Class:** A

#### Stinkyscape #159 · ETHICS
`stinkyscape-159` · (3) Spell · Rare · built by S24 (#1880)

> **Designer:** (3) ETHICS, Spell, Rare, Stinkyscape, #159 · Add iKwzki, Shockwave447, xxFreshJack, Tnack Trav, Kwzka, and Tylannt to your hand ~~~ Add Radiant iKwzki, Shockwave447, xxFreshJack, Tnack Trav, Kwzka and Tylannt to your hand

- **Text:** Add an iKwzki, Shockwave447, xxFreshJack, Tnack Trav, Kwzka and Tylannt to your hand.
- **Radiant:** Add a Radiant iKwzki, Shockwave447, xxFreshJack, Tnack Trav, Kwzka and Tylannt to your hand.
- **Engine:** Add to hand ([[§6.3]]): a new card of each, in the order printed, owned by its caster; the Radiant face adds each Radiant ([[§5.2]]). A full hand burns the rest, in that order ([[§2.4]]). The cards are named, so they reach the hand while the set is held ([[R1420]]).
- **Rulings:** ST-S24-1: "a Radiant X, Y and Z" makes every card listed Radiant, as C+ #42.1 KY's Gift's list does; each card is added on its own normal face and price.
- **Numbers:** none (the digits are card names).
- **Check:** R275: every card Radiant (each about 2×).
- **Class:** A.

#### Stinkyscape #160 · Golden Accord
`stinkyscape-160` · (3) Spell · Rare · built by S24 (#1880)

> **Designer:** (3) Golden Accord, Spell, Rare, Stinkyscape, #160 · Add a Paragon mk1, Paragon mk2, Paragon X, and Falcon to your hand ~~~ Add a Radiant Paragon mk1, Paragon mk2, Paragon X, and Falcon to your hand

- **Text:** Add a Paragon Mk1, Paragon Mk2, Paragon-X and Falcon to your hand.
- **Radiant:** Add a Radiant Paragon Mk1, Paragon Mk2, Paragon-X and Falcon to your hand.
- **Engine:** Add to hand ([[§6.3]]): a new card of each, in the order printed, owned by its caster; the Radiant face adds each Radiant ([[§5.2]]). A full hand burns the rest, in that order ([[§2.4]]). The cards are named, so they reach the hand while the set is held ([[R1420]]).
- **Rulings:** ST-S24-1: "a Radiant X, Y and Z" makes every card listed Radiant, as C+ #42.1 KY's Gift's list does; each card is added on its own normal face and price.
- **Numbers:** none (the digits are card names).
- **Check:** The list's "Paragon mk1", "mk2" and "Paragon X" are S #72 Paragon Mk1, #73 Paragon Mk2 and #74 Paragon-X. R275: every card Radiant (each about 2×).
- **Class:** A.

#### Stinkyscape #161 · Pegasus
`stinkyscape-161` · (3) Spell · Rare · built by S24 (#1880)

> **Designer:** (3) Pegasus, Spell, Rare, Stinkyscape, #161 · Add A Very Dandy Lad, Tmack Trav, Gokugains and Arctic Citadel to your hand ~~~ Add Radiant A Very Dandy Lad, Tmack Trav, Gokugains and Arctic Citadel

- **Text:** Add A Very Dandy Lad, Tmack Trav, Gokugains and Arctic Citadel to your hand.
- **Radiant:** Add a Radiant A Very Dandy Lad, Tmack Trav, Gokugains and Arctic Citadel to your hand.
- **Engine:** Add to hand ([[§6.3]]): a new card of each, in the order printed, owned by its caster; the Radiant face adds each Radiant ([[§5.2]]). A full hand burns the rest, in that order ([[§2.4]]). The cards are named, so they reach the hand while the set is held ([[R1420]]).
- **Rulings:** ST-S24-1: "a Radiant X, Y and Z" makes every card listed Radiant, as C+ #42.1 KY's Gift's list does; each card is added on its own normal face and price.
- **Numbers:** none (a list of names).
- **Check:** The list's Radiant face drops "to your hand"; it is the same move. R275: every card Radiant (each about 2×).
- **Class:** A.

#### Stinkyscape #162 · Star-jumper Core
`stinkyscape-162` · (3) Spell · Rare · built by S24 (#1880)

> **Designer:** (3) Star-jumper core, Spell, Rare, Stinkyscape, #162 · Add a Badger, Honey Badger, Ozark, Orca, and Galaxy to your hand ~~~ Add a Badger, Honey Badger, Ozark, Orca, and Galaxy to your hand

- **Text:** Add a Badger, Honey Badger, Ozark, Orca and Galaxy to your hand.
- **Radiant:** Add a Radiant Badger, Honey Badger, Ozark, Orca and Galaxy to your hand.
- **Engine:** Add to hand ([[§6.3]]): a new card of each, in the order printed, owned by its caster; the Radiant face adds each Radiant ([[§5.2]]). A full hand burns the rest, in that order ([[§2.4]]). The cards are named, so they reach the hand while the set is held ([[R1420]]).
- **Rulings:** ST-S24-1: "a Radiant X, Y and Z" makes every card listed Radiant, as C+ #42.1 KY's Gift's list does; each card is added on its own normal face and price.
- **Numbers:** none (a list of names).
- **Check:** ST8: the Radiant face adds the five Radiant, and the name is Star-jumper Core, as #163 writes it. R275: every card Radiant (each about 2×).
- **Class:** A.

#### Stinkyscape #163 · Swear Loyalities
`stinkyscape-163` · (1) Spell · Rare · built by S24 (#1880)

> **Designer:** (1) Swear Loyalities, Spell, Rare, Stinkyscape, #163 · Add either a Vanguard Coalition, ETHICS, Golden, Pegasus, or Star-jumper Core to your hand. Exile the other options you didn't choose. *{wherever they are}* ~~~ Add either a Radiant Vanguard Coalition, ETHICS, Golden, Pegasus, or Star-jumper Core to your hand. Exile the other options you didn't choose. *{wherever they are}*

- **Text:** Choose a Vanguard Coalition, ETHICS, Golden Accord, Pegasus or Star-jumper Core and add it to your hand. Exile every card you own of the four you didn't choose, wherever it is.
- **Radiant:** Choose a Radiant Vanguard Coalition, ETHICS, Golden Accord, Pegasus or Star-jumper Core and add it to your hand. Exile every card you own of the four you didn't choose, wherever it is.
- **Engine:** A choice of five ([[R81]]'s declared modes, or a `pick` prompt its caster alone sees): the chosen bundle added to the hand (Radiant on the Radiant face, [[§5.2]]), a full hand burning it ([[§2.4]]). Then every card its caster owns of the four other definitions is exiled ([[§6.3]] Exile) from their hand, deck and graveyard, as Core #94 Genn's Greed's exile reaches those zones; the opponent's are untouched.
- **Rulings:** ST-S24-2: "wherever they are" is every zone its caster's own cards can be in, hand, deck and graveyard (a Spell never stands on the field); the opponent's copies stay. ST-S24-3: the choice shows all five, unlike Discover's three.
- **Numbers:** none.
- **Check:** ST8: "Golden" is Golden Accord; the name Swear Loyalities kept as written. R275: the chosen bundle Radiant (about 2×). ⚠ designer: E9.
- **Class:** B (a choice and an exile across zones).

#### Stinkyscape #164 · The Syndicate
`stinkyscape-164` · (2) Field Spell · Epic · built by S08 (#1864)

> **Designer:** (2) The Syndicate, Spell, Epic, Stinkyscape, #164 · Tradable / Aura: Your cards have Tradable, whenever you a Trade a card it has a 30% chance to be exiled and a 30% chance to become Radiant ~~~ Tradable / Aura: Your cards have Tradable, whenever you a Trade a card it has a 30% chance to be exiled and a 70% chance to become Radiant

- **Text:** Tradable / Aura: Cards in your hand have Tradable. / Whenever you Trade a card, the card you put into your deck has a {exileChance}% chance to be exiled and a {radiantChance}% chance to become Radiant.
- **Radiant:** Tradable / Aura: Cards in your hand have Tradable. / Whenever you Trade a card, the card you put into your deck has a {exileChance}% chance to be exiled and a {radiantChance}% chance to become Radiant.
- **Engine:** Tradable ([[R1838000]]). An aura over its controller's hand: every card there has Tradable, a card keyword read in the hand, lent as M #45's and C+ #33's auras lend to a hand ([[R1040]], [[R1194000]]). NEW (S08): a trigger on its controller's `traded`, one roll of the match rng on the card the Trade put into the deck: the first 30 exile it from the deck, the next 30 (Radiant 70) make it Radiant where it lies ([[§5.2]]), the rest nothing; the draw stands either way. The exile names the card, as every exile does; a card made Radiant in a deck stays hidden ([[R97]], [[R1838001]]).
- **Rulings:** ST-S08-3: "Your cards have Tradable" is the cards in your hand, where alone Tradable does anything ([[R1838000]]). ST-S08-4: "a 30% chance to be exiled and a 30% chance to become Radiant" is one roll on the card put into the deck (exiled, else Radiant, else nothing), as the Radiant face's 30 and 70 make 100; if a tune takes their sum past 100, Radiant takes what is left.
- **Numbers:** exileChance 30 ↓ (step 8; max 100); radiantChance 30 ↑ (Radiant 70; step 8; max 100).
- **Check:** A Spell with an Aura is a Field Spell (ST8; Frontier Outpost's backrow places it). "whenever you a Trade" is "whenever you Trade". R275: Radiant 70% against 30% (2.3×).
- **Class:** C.

#### Stinkyscape #165 · The Trade Union
`stinkyscape-165` · (1) Field Spell · Rare · built by S08 (#1864)

> **Designer:** (1) The Trade Union, Field Spell, Rare, Stinkyscape, #165 · Tradable / Aura: Your cards have Tradable ~~~ Tradable / Aura: Your cards have Tradable, you can Trade up to two cards a turn

- **Text:** Tradable / Aura: Cards in your hand have Tradable.
- **Radiant:** Tradable / Aura: Cards in your hand have Tradable. You may Trade up to {trades|card|cards} each turn.
- **Engine:** Tradable ([[R1838000]]); S #164's aura lends Tradable to its controller's hand ([[R1040]], [[R1194000]]). Radiant: while it acts its controller's Trades a turn are 2, the figure `trades_per_turn_of` reads ([[R1838000]]), the highest of several holding, never their sum.
- **Rulings:** ST-S08-5: "you can Trade up to two cards a turn" sets its controller's Trades a turn to 2 while it acts, not 1 + 2; two Unions still make 2.
- **Numbers:** trades 2 ↑ (the Radiant face only, [[R824000]]).
- **Check:** R275: the base face has no number, so the Radiant face adds a rider, a second Trade a turn.
- **Class:** C.

#### Stinkyscape #166 · Scanning Probe Mk1
`stinkyscape-166` · (1) Spell · Common · built by S20 (#1879)

> **Designer:** (1) Scanning Probe Mk1, Spell, Common, Stinkyscape, #166 · 5% chance to add a Radiant Anomaly Found! to your hand / 20% chance to add a Anomaly Found! to your hand / Else: future Scanning Probes have +1% & +4% chance to find an Anomaly ~~~ Lucky 1 / 5% chance to add a Radiant Anomaly Found! to your hand / 20% chance to add a Anomaly Found! to your hand / Else: future Scanning Probes have +1% & +4% chance to find an Anomaly

- **Text:** 5% chance to add a Radiant Anomaly Found! to your hand. / 20% chance to add an Anomaly Found! to your hand instead. / Otherwise, your future Scanning Probes have +1% and +4% to these chances.
- **Radiant:** Lucky 1 / 5% chance to add a Radiant Anomaly Found! to your hand. / 20% chance to add an Anomaly Found! to your hand instead. / Otherwise, your future Scanning Probes have +1% and +4% to these chances.
- **Engine:** #1855's Anomaly scan (pending): one roll of 100 against its controller's chances, 5 for a Radiant Anomaly Found! and 20 more for a plain one, each earlier miss of that player's adding 1 and 4 (a count per player, kept across both Probes); a find is added to the hand, a full hand burning it ([[§2.4]]); a miss is silent. Its Lucky, and its controller's Luck, roll again and keep the best: Radiant over plain over a miss ([[R1438]], [[R987]]). Radiant: Lucky 1.
- **Rulings:** ST-S20-1: the two chances are bands of one roll, 5% Radiant and 20% plain (25% in all); "future Scanning Probes" are its controller's later scans with either Probe, and a find keeps the count.
- **Numbers:** none (the engine's chances).
- **Check:** R275: Lucky 1, a second roll, about 1.75× the find (a rider).
- **Class:** C (M08, the scan).

#### Stinkyscape #167 · Scanning Probe Mk2
`stinkyscape-167` · (2) Spell · Rare · built by S20 (#1879)

> **Designer:** (2) Scanning Probe Mk2, Spell, Rare, Stinkyscape, #167 · Lucky 1 / 5% chance to add a Radiant Anomaly Found! to your hand / 20% chance to add a Anomaly Found! to your hand / Else: future Scanning Probes have +1% & +4% chance to find an Anomaly ~~~ Luck 3 / 5% chance to add a Radiant Anomaly Found! to your hand / 20% chance to add a Anomaly Found! to your hand / Else: future Scanning Probes have +1% & +4% chance to find an Anomaly

- **Text:** Lucky 1 / 5% chance to add a Radiant Anomaly Found! to your hand. / 20% chance to add an Anomaly Found! to your hand instead. / Otherwise, your future Scanning Probes have +1% and +4% to these chances.
- **Radiant:** Lucky 3 / 5% chance to add a Radiant Anomaly Found! to your hand. / 20% chance to add an Anomaly Found! to your hand instead. / Otherwise, your future Scanning Probes have +1% and +4% to these chances.
- **Engine:** #1855's Anomaly scan (pending): one roll of 100 against its controller's chances, 5 for a Radiant Anomaly Found! and 20 more for a plain one, each earlier miss of that player's adding 1 and 4 (a count per player, kept across both Probes); a find is added to the hand, a full hand burning it ([[§2.4]]); a miss is silent. Its Lucky, and its controller's Luck, roll again and keep the best: Radiant over plain over a miss ([[R1438]], [[R987]]). Lucky 1, Radiant Lucky 3.
- **Rulings:** ST-S20-1, as S #166.
- **Numbers:** none (Lucky is a keyword's count, the chances the engine's).
- **Check:** ST8: Radiant "Luck 3" is Lucky 3 (Luck X is a player's). R275: Lucky 1 → 3 (3× the extra rolls).
- **Class:** C (M08, the scan).

#### Stinkyscape #168 · Drone Hanger
`stinkyscape-168` · (2) Field Spell · Rare · built by S07 (#1863)

> **Designer:** (2) Drone Hanger, Field Spell, Rare, Stinkyscape, #168 · Start of your turn: Summon a Drone Interceptor ~~~ Start of your turn: Summon a Radiant Drone Interceptor

- **Text:** Start of turn: Summon a Drone Interceptor.
- **Radiant:** Start of turn: Summon a Radiant Drone Interceptor.
- **Engine:** Start of turn ([[R62]]): `summon` of a Drone Interceptor into its controller's leftmost open unit zone ([[R64]]); no zone, nothing. Radiant: a Radiant one.
- **Rulings:** none new: every reading is a ruling on `main` or the card's text.
- **Numbers:** none (one drone, written "a").
- **Check:** "Start of your turn" is the label Start of turn ([[R366]]); "Hanger" keeps its spelling (ST9). R275: a Radiant Interceptor.
- **Class:** A.

#### Stinkyscape #169 · Tiny Asteroid
`stinkyscape-169` · (1) Unit · Common · 0/4 → 0/8 · built by S04 (#1861)

> **Designer:** (1) Tiny Asteroid, Unit, Common, Stinkyscape, #169 · 0/4 / Can't Attack / Can't be in Defense Position / Death: Gain 1 Stinkyscape Resource / Activate: Take 2 damage. Tribute this. ~~~ 0/8 / Can't Attack / Can't be in Defense Position / Death: Gain 1 Radiant Stinkyscape Resource / Activate: Tribute this.

- **Text:** Can't attack / Can't be in Defense Position. / Death: Gain {resources|random Stinkyscape Resource|random Stinkyscape Resources}. / Activate: Take {selfDamage} damage. Tribute this.
- **Radiant:** Can't attack / Can't be in Defense Position. / Death: Gain {resources|random Radiant Stinkyscape Resource|random Radiant Stinkyscape Resources}. / Activate: Tribute this.
- **Engine:** A mining node ([[R1840006]]): Can't attack, and never in Defense Position ([[§4.1]]). Activate ([[R384]]): its controller's hero takes 2 damage ([[§4.4]]), then a Sacrifice of it ([[§6.3]]), a death, so its Death runs: `gain_resources` of one random Resource into its controller's tray ([[R1840000]], [[R1840002]]); any other death pays too, an exile or a Bounce nothing. An anomaly board's asteroid may carry a restricted pool ("only gives Narcor and Red Narcor", [[R1840002]]; #1855's rider, pending). Radiant: the Activate is the Sacrifice alone, and the Resources are Radiant.
- **Rulings:** ST-S04-11: "Activate: Take N damage. Tribute this" is its controller's hero taking N, then the asteroid's Sacrifice, whose Death pays ([[R1840006]]); a Tribute of it for a play, or any other death, pays as well; an Untributable grant would stop the Activate ([[R1220]]).
- **Numbers:** resources 1 ↑; selfDamage 2 ↓ (the base face only, [[R1431]]).
- **Check:** "Defense Positio n" is Position (ST8); the anomalies' "Small Asteroid" is Tiny Asteroid (ST8). R275: stats doubled; Radiant Resources, the damage gone.
- **Class:** C.

#### Stinkyscape #170 · Medium Asteroid
`stinkyscape-170` · (1) Unit · Common · 0/6 → 0/12 · built by S04 (#1861)

> **Designer:** (1) Medium Asteroid, Unit, Common, Stinkyscape, #170 · 0/6 / Can't Attack / Can't be in Defense Position / Death: Gain 2 Stinkyscape Resource / Activate: Take 3 damage. Tribute this. ~~~ 0/12 / Can't Attack / Can't be in Defense Position / Death: Gain 2 Radiant Stinkyscape Resources. / Activate: Tribute this.

- **Text:** Can't attack / Can't be in Defense Position. / Death: Gain {resources|random Stinkyscape Resource|random Stinkyscape Resources}. / Activate: Take {selfDamage} damage. Tribute this.
- **Radiant:** Can't attack / Can't be in Defense Position. / Death: Gain {resources|random Radiant Stinkyscape Resource|random Radiant Stinkyscape Resources}. / Activate: Tribute this.
- **Engine:** A mining node ([[R1840006]]): Can't attack, and never in Defense Position ([[§4.1]]). Activate ([[R384]]): its controller's hero takes 3 damage ([[§4.4]]), then a Sacrifice of it ([[§6.3]]), a death, so its Death runs: `gain_resources` of 2 random Resources into its controller's tray ([[R1840000]], [[R1840002]]); any other death pays too, an exile or a Bounce nothing. An anomaly board's asteroid may carry a restricted pool ("only gives Narcor and Red Narcor", [[R1840002]]; #1855's rider, pending). Radiant: the Activate is the Sacrifice alone, and the Resources are Radiant.
- **Rulings:** none new: every reading is a ruling on `main` or the card's text.
- **Numbers:** resources 2 ↑; selfDamage 3 ↓ (the base face only, [[R1431]]).
- **Check:** "Defense Positio n" is Position (ST8); the anomalies' "Small Asteroid" is Tiny Asteroid (ST8). R275: stats doubled; Radiant Resources, the damage gone. The list's base "Gain 2 Stinkyscape Resource" is plural.
- **Class:** C.

#### Stinkyscape #171 · Large Asteroid
`stinkyscape-171` · (3) Unit · Rare · 0/8 → 0/16 · built by S04 (#1861)

> **Designer:** (3) Large Asteroid, Unit, Rare, Stinkyscape, #171 · 0/8 / Can't Attack / Can't be in Defense Position / Death: Gain 3 Stinkyscape Resources / Activate: Take 4 damage. Tribute this. ~~~ 0/16 / Can't Attack / Can't be in Defense Position / Death: Gain 3 Radiant Stinkyscape Resources. / Activate: Tribute this.

- **Text:** Can't attack / Can't be in Defense Position. / Death: Gain {resources|random Stinkyscape Resource|random Stinkyscape Resources}. / Activate: Take {selfDamage} damage. Tribute this.
- **Radiant:** Can't attack / Can't be in Defense Position. / Death: Gain {resources|random Radiant Stinkyscape Resource|random Radiant Stinkyscape Resources}. / Activate: Tribute this.
- **Engine:** A mining node ([[R1840006]]): Can't attack, and never in Defense Position ([[§4.1]]). Activate ([[R384]]): its controller's hero takes 4 damage ([[§4.4]]), then a Sacrifice of it ([[§6.3]]), a death, so its Death runs: `gain_resources` of 3 random Resources into its controller's tray ([[R1840000]], [[R1840002]]); any other death pays too, an exile or a Bounce nothing. An anomaly board's asteroid may carry a restricted pool ("only gives Narcor and Red Narcor", [[R1840002]]; #1855's rider, pending). Radiant: the Activate is the Sacrifice alone, and the Resources are Radiant.
- **Rulings:** none new: every reading is a ruling on `main` or the card's text.
- **Numbers:** resources 3 ↑; selfDamage 4 ↓ (the base face only, [[R1431]]).
- **Check:** "Defense Positio n" is Position (ST8); the anomalies' "Small Asteroid" is Tiny Asteroid (ST8). R275: stats doubled; Radiant Resources, the damage gone.
- **Class:** C.

#### Stinkyscape #172 · Huge Asteroid
`stinkyscape-172` · (4) Unit · Epic · 0/10 → 0/20 · built by S04 (#1861)

> **Designer:** (4) Huge Asteroid, Unit, Epic, Stinkyscape, #172 · 0/10 / Can't Attack / Can't be in Defense Position / Death: Gain 5 Stinkyscape Resources / Activate: Take 5 damage. Tribute this. ~~~ 0/20 / Can't Attack / Can't be in Defense Position / Death: Gain 5 Radiant Stinkyscape Resources. / Activate: Tribute this.

- **Text:** Can't attack / Can't be in Defense Position. / Death: Gain {resources|random Stinkyscape Resource|random Stinkyscape Resources}. / Activate: Take {selfDamage} damage. Tribute this.
- **Radiant:** Can't attack / Can't be in Defense Position. / Death: Gain {resources|random Radiant Stinkyscape Resource|random Radiant Stinkyscape Resources}. / Activate: Tribute this.
- **Engine:** A mining node ([[R1840006]]): Can't attack, and never in Defense Position ([[§4.1]]). Activate ([[R384]]): its controller's hero takes 5 damage ([[§4.4]]), then a Sacrifice of it ([[§6.3]]), a death, so its Death runs: `gain_resources` of 5 random Resources into its controller's tray ([[R1840000]], [[R1840002]]); any other death pays too, an exile or a Bounce nothing. An anomaly board's asteroid may carry a restricted pool ("only gives Narcor and Red Narcor", [[R1840002]]; #1855's rider, pending). Radiant: the Activate is the Sacrifice alone, and the Resources are Radiant.
- **Rulings:** none new: every reading is a ruling on `main` or the card's text.
- **Numbers:** resources 5 ↑; selfDamage 5 ↓ (the base face only, [[R1431]]).
- **Check:** "Defense Positio n" is Position (ST8); the anomalies' "Small Asteroid" is Tiny Asteroid (ST8). R275: stats doubled; Radiant Resources, the damage gone.
- **Class:** C.

#### Stinkyscape #173 · Abandoned Spice Tower
`stinkyscape-173` · (3) Unit · Rare · 0/8 → 0/16 · built by S04 (#1861)

> **Designer:** (3) Abandoned Spice Tower, Unit, Rare, Stinkyscape, #173 · 0/8 / Can't Attack / Can't be in Defense Position / Death: Add a three Spice to your hand / Activate: Take 4 damage, Tribute this ~~~ 0/16 / Can't Attack / Can't be in Defense Position / Death: Add a three Radiant Spice to your hand / Activate: Take 4 damage, Tribute this /  /  / ```

- **Text:** Can't attack / Can't be in Defense Position. / Death: Add {spices|Spice|Spices} to your hand. / Activate: Take {selfDamage} damage. Tribute this.
- **Radiant:** Can't attack / Can't be in Defense Position. / Death: Add {spices|Radiant Spice|Radiant Spices} to your hand. / Activate: Take {selfDamage} damage. Tribute this.
- **Engine:** [[R1840006]]'s node, paying Spice: Can't attack, never in Defense Position ([[§4.1]]); Activate ([[R384]]): its controller's hero takes 4 ([[§4.4]]), then a Sacrifice of it ([[§6.3]]); Death: `add_to_hand` of three Spices (S #89), hand cards, never Resources, a full hand burning the rest ([[§2.4]]). Radiant: Radiant Spices; the Activate keeps its damage, as the list prints it.
- **Rulings:** ST-S04-12: "Add a three Spice" is three Spices into the hand: Spice is a card, not a Resource, so the hand cap applies.
- **Numbers:** spices 3 ↑; selfDamage 4 ↓.
- **Check:** "Add a three Spice" is "Add three Spices" (ST8). The Radiant face keeps the damage, as the list prints it. R275: stats doubled; Radiant Spices.
- **Class:** C.

#### Stinkyscape #174 · Aerelret
`stinkyscape-174` · (4) Trap · Epic · built by S15 (#1875)

> **Designer:** (4) Aerelret, Trap, Epic, Stinkyscape, #21 · 30% chance to Reveal when an opponent attacks / On reveal: AMBUSH summon a Warp Disruptor then 5 Paragon Mk1 ~~~ On reveal: AMBUSH summon a Radiant Warp Disruptor then 5 Radiant Paragon Mk1

- **Text:** {chance}% chance to reveal when an enemy Unit attacks: Summon a Warp Disruptor, then {paragons|Paragon Mk1|Paragon Mk1s}. The attack goes on.
- **Radiant:** {chance}% chance to reveal when an enemy Unit attacks: Summon a Radiant Warp Disruptor, then {paragons|Radiant Paragon Mk1|Radiant Paragon Mk1s}. The attack goes on.
- **Engine:** S #22 Gatinsir's chance to reveal ([[R1849009]]): a hit fires it in [[§4.2]] step 4's trap window: `summon` a Warp Disruptor (S #23) into its controller's open backrow zone, then five Paragon Mk1 (S #72, Starship Pieces with Charge, Flying and Ranged) into open unit zones ([[R64]]), as many as fit, no Cry ([[R1]]); then the attack goes on as declared (ST9's AMBUSH). Radiant: all six Radiant.
- **Rulings:** S #22's (ST-S15-3).
- **Numbers:** chance 30 ↑ (step 10, max 100); paragons 5 ↑.
- **Check:** ST8: the designer's second #21 is #174. ST9's AMBUSH; [[R1849009]]'s roll. R275: all six summons Radiant (Paragon Mk1 12/2).
- **Class:** C (M15's chance to reveal).

#### Stinkyscape #175 · Frost Module
`stinkyscape-175` · (1) Unit · Common · 2/1 → 4/2 · built by S03 (#1860)

> **Designer:** (1) Frost Module, Unit, Common, Stinkyscape, #122 · 2/1 / Starship Piece / Equip / Activate: Freeze an enemy ~~~ 4/2 / Starship Piece / Equip / Activate 2: Freeze an enemy

- **Text:** Starship Piece, Equip / Activate: Freeze an enemy Unit.
- **Radiant:** Starship Piece, Equip / Activate 2: Freeze an enemy Unit.
- **Engine:** Equip ([[R1086]]): played onto one of your Units it fuses into it, the host keeping its zone and taking its stats, keywords and text ([[R77]], [[R102]]), Starship Piece only into a piece ([[R1852001]]); played alone its death fuses a copy into its owner's Dormant Starship ([[R1852000]]); an equipped host takes the Activate. Activate ([[R384]]; Radiant Activate 2, [[R1751000]]): `freeze { target }` on a declared enemy Unit ([[§6.3]] Freeze, [[R1174000]]), its thaw stamped as it lands; a hero is never Frozen.
- **Rulings:** ST-S03-10: "Freeze an enemy" is an enemy Unit: a hero is never Frozen (§6.3 Freeze). These are proposals for the S03 part, which takes R-numbers from its own block.
- **Numbers:** none (the Activate count is no param, R482).
- **Check:** ST8: the designer's second #122 is #175 (R1857001). R275: stats doubled and Activate 2.
- **Class:** A

#### Stinkyscape #T-dormant-starship · Dormant Starship
`stinkyscape-t-dormant-starship` · (3) Unit · Legendary · 0/0 → 0/0 · built by S02 (#1859)

> **Designer:** (3) Dormant Starship, Unit, Legendary, Dormant Starship, Stinkyscape · Dormant · While Dormant cards cannot be stacked on top of this, if they do the stacked card is exiled before its effect resolves · Activate: Pay (3) to Awaken and trigger all Cries · When awoken gain one of these randomly (names with a tag gain preference ...) · NOT a token so it can be shuffled into your GY

- **Text:** Dormant until it launches. / Each of your Starship Pieces that dies fuses a copy into this. / A card played onto this by Stack while it is Dormant is exiled before it resolves. / Activate: Pay (3) to launch this. It takes a random name, then every Cry fused into it fires, then every When launched.
- **Radiant:** Dormant until it launches. / Each of your Starship Pieces that dies fuses a copy into this. / A card played onto this by Stack while it is Dormant is exiled before it resolves. / Activate: Pay (3) to launch this. It takes a random name, then every Cry fused into it fires, then every When launched.
- **Engine:** The engine's hull, `DORMANT_STARSHIP_DEF_ID` ([[R1852000]]): summoned by a piece's death into its owner's leftmost open unit zone, else on top of their lane-3 unit zone, never played, in no pool and no deck ([[R1857001]]). Its script is the hull: `StaticFlags { slumber, starshipHull }` on both faces, Dormant with no countdown ([[R1839000]]), its stats only what fuses into it ([[R77]]); the Stack exile ([[R1852007]]); the launch is the engine's Activate, `STARSHIP_LAUNCH_COST` (3), moved by "Your Starship launch costs (N)" ([[R1852006]]), which awakens it ([[R1839002]]): the Starship keyword, a name from `STARSHIP_NAMES`, a tagged one only for a ship holding the tag ([[R1852004]]), its memory ([[R1852005]]), its fused Cries, then its fused When launched ([[R1852003]]). No token, so it reaches a graveyard; its Radiant face is [[R349]]'s fallback (`radiantFallback`, [[R1857001]]).
- **Rulings:** ST-S02-13: its text is the engine's rules written out, so the script adds nothing; it is no token, Legendary, at `T-dormant-starship`, its Radiant face R349's fallback. The designer's "trigger all Cries" is its fused Cries, then its fused When launched (R1852003); the names list is `STARSHIP_NAMES` (R1852004). These are proposals for the S02 part, which takes R-numbers from its own block.
- **Numbers:** none (the launch price is `STARSHIP_LAUNCH_COST`, an engine constant, CLAUDE.md rule 9).
- **Check:** Not a token (the list's "NOT a token", R1852000); no Radiant face in the list (R349). ⚠ designer: R1857001 gives it R349's fallback, no rule making a Dormant hull Radiant.
- **Class:** C

#### Stinkyscape #T-drone-interceptor · Drone Interceptor
`stinkyscape-t-drone-interceptor` · (1) Unit, Token · Token (printed Common) · 2/2 → 4/4 · built by S07 (#1863)

> **Designer:** (1) Drone Interceptor, Unit, Common, Token, Stinkyscape · [2/2] · Ranged, Flying · <has 1 random combat keyword> ~~~ [4/4] · Ranged, Flying · <has 2 random combat keyword>

- **Text:** Ranged, Flying / Aura: Has {keywords|random combat keyword|random combat keywords}.
- **Radiant:** Ranged, Flying / Aura: Has {keywords|random combat keyword|random combat keywords}.
- **Engine:** Ranged and Flying ([[§6.1]]). Keymaster Keenus's Aura ([[R781004]]): random combat keywords from [[R21]]'s pool, drawn with the match rng as it arrives on the field and kept until it leaves ([[R78]]), never one it has (so neither Ranged nor Flying), no Taunt beside Indestructible ([[R347]]). A unit token: it ceases to exist off the field ([[R11]]).
- **Rulings:** ST-S07-6: "<has 1 random combat keyword>" is Keymaster Keenus's Aura ([[R781004]]): drawn as the drone arrives, kept until it leaves, never one it has.
- **Numbers:** keywords 1 ↑ (Radiant 2).
- **Check:** R275: 4/4 and two keywords against 2/2 and one.
- **Class:** A.

#### Stinkyscape #T-drone-corvette · Drone Corvette
`stinkyscape-t-drone-corvette` · (3) Unit, Token · Token (printed Rare) · 8/8 → 16/16 · built by S07 (#1863)

> **Designer:** (3) Drone Corvette, Unit, Rare, Token, Stinkyscape · [8/8] · Ranged, Flying · <has 2 random combat keyword> ~~~ [16/16] · Ranged, Flying · <has 4 random combat keyword>

- **Text:** Ranged, Flying / Aura: Has {keywords|random combat keyword|random combat keywords}.
- **Radiant:** Ranged, Flying / Aura: Has {keywords|random combat keyword|random combat keywords}.
- **Engine:** As the Drone Interceptor ([[R781004]], [[R21]], [[R347]]), two keywords drawn as it arrives. A unit token ([[R11]]).
- **Rulings:** none new: every reading is a ruling on `main` or the card's text.
- **Numbers:** keywords 2 ↑ (Radiant 4).
- **Check:** R275: 16/16 and four keywords against 8/8 and two.
- **Class:** A.

#### Stinkyscape #T-shield-layer · Drone Battleship (Shield Layer)
`stinkyscape-t-shield-layer` · (5) Unit, Token · Token (printed Mythic) · 12/12 → 24/24 · built by S07 (#1863)

> **Designer:** (5) Drone Battleship - Shield Layer, Unit, Mythic, Starship, Token, Stinkyscape · [12/12] · Starship · Can't attack · Taunt, Armor 4 · Fatal damage a friendly ally would take is redirected to this ~~~ [24/24] ... Armor 8

- **Text:** Starship, Can't attack, Taunt, Armor 4 / Lethal damage your hero or your other Units would take is redirected to this.
- **Radiant:** Starship, Can't attack, Taunt, Armor 8 / Lethal damage your hero or your other Units would take is redirected to this.
- **Engine:** The top of the Drone Battleship's pile ([[R1842000]]). Starship ([[§6.1]], a keyword with no rule of its own), Can't attack, Taunt, Armor 4. Unan's lethal guard ([[R1204]], [[R1842001]]): a lethal hit on its controller's hero or another of their Units is redirected to it, through its own Armor, while it tops its pile; a lethal hit on itself it never catches ([[R460]]); when it dies the Propagation Layer resumes ([[R13]]). A unit token ([[R11]]).
- **Rulings:** ST-S07-7: "Fatal damage a friendly ally would take is redirected to this" is Unan's lethal guard ([[R1204]]), answering only while the layer tops its pile ([[R1842001]]).
- **Numbers:** none (Armor 4 is a numbered keyword's ([[R482]])).
- **Check:** The list's "Drone Battleship - Shield Layer" is "Drone Battleship (Shield Layer)" (the plan's discrepancy 10); "Fatal" is Unan's "Lethal". R275: 24/24 and Armor 8.
- **Class:** C.

#### Stinkyscape #T-propagation-layer · Drone Battleship (Propagation Layer)
`stinkyscape-t-propagation-layer` · (5) Unit, Token · Token (printed Mythic) · 12/12 → 24/24 · built by S07 (#1863)

> **Designer:** (5) Drone Battleship - Propagation Layer, Unit, Mythic, Starship, Token, Stinkyscape · [12/12] · Starship · Can't attack · Activate 2: summon a Drone Interceptor, Activate 1: summon a Drone Corvette ~~~ [24/24] ... Radiant

- **Text:** Starship, Can't attack / Activate 2: Summon a Drone Interceptor. / Activate: Summon a Drone Corvette.
- **Radiant:** Starship, Can't attack / Activate 2: Summon a Radiant Drone Interceptor. / Activate: Summon a Radiant Drone Corvette.
- **Engine:** The middle of the pile ([[R1842000]]): beneath the Shield Layer it acts for nothing ([[R13]]); on top, two Activate abilities, counted apart ([[R1751000]], [[R384]]): Activate 2 summons a Drone Interceptor, Activate a Drone Corvette, each into the leftmost open unit zone ([[R64]]). Starship, Can't attack. A unit token ([[R11]]). Radiant: Radiant drones.
- **Rulings:** ST-S07-8: the list's "Activate 2" and "Activate 1" are two abilities, counted apart ([[R1751000]]): two Interceptors and one Corvette a turn at most, offered only while the layer tops its pile.
- **Numbers:** none (the Activate counts are no params ([[R482]])).
- **Check:** The list's "Activate 1" is Activate ([[R366]]). R275: 24/24 and Radiant drones.
- **Class:** C.

#### Stinkyscape #T-meltdown-layer · Drone Battleship (Meltdown Layer)
`stinkyscape-t-meltdown-layer` · (5) Unit, Token · Token (printed Mythic) · 12/12 → 24/24 · built by S07 (#1863)

> **Designer:** (5) Drone Battleship - Meltdown Layer, Unit, Mythic, Starship, Token, Stinkyscape · [12/12] · Starship · Charge, Windfury, Firststrike, Brittle 3 · End of turn: Take 6 damage ~~~ [24/24] ... Brittle 6 · End of turn: Take 6 damage

- **Text:** Starship, Charge, Windfury, First Strike, Brittle 3 / End of turn: This takes {selfDamage} damage.
- **Radiant:** Starship, Charge, Windfury, First Strike, Brittle 6 / End of turn: This takes {selfDamage} damage.
- **Engine:** The bottom of the pile ([[R1842000]]): it acts once both layers above are gone, and with Charge may attack at once, twice (Windfury, [[§6.1]]), First Strike. Brittle 3 ([[R385]]): its count starts as it is summoned and waits while it is buried, ticking once it tops the pile (ST-S07-9). End of turn ([[R62]]): one hit of 6 on itself ([[§4.4]]), its own hook, which a buried card does not run ([[R13]]). A unit token ([[R11]]). Radiant: 24/24, Brittle 6.
- **Rulings:** ST-S07-9: "End of turn: Take 6 damage" is the layer hitting itself, never its hero; beneath a Stack it runs no hook ([[R13]]), and its Brittle waits while buried as a Dormant countdown does ([[R1839003]]'s shape), ticking once it tops the pile.
- **Numbers:** selfDamage 6 ↓ (step 2).
- **Check:** "Firststrike" is First Strike. R275: 24/24 and Brittle 6, the self-hit unchanged.
- **Class:** B.

#### Stinkyscape #T-reknite · Reknite
`stinkyscape-t-reknite` · (0) Spell, Token · Token (printed Common) · built by S04 (#1861)

> **Designer:** (0) Reknite, Spell, Common, Token, Stinkyscape · Sells for 1~3 credits | buys for 3 · Your hero gains +1 Armor until next turn ~~~ Sells for 2~6 credits | buys for 6 · Your hero gains +2 Armor until next turn

- **Text:** Your hero has +{armor} Armor until the start of your next turn.
- **Radiant:** Your hero has +{armor} Armor until the start of your next turn.
- **Engine:** A Resource ([[R1840000]]): a (0) Spell token gained into its player's tray, never a hand, and played from there as from a hand ([[R1840003]]), in [[R1840002]]'s roll at its rarity; its `resource` prices it: sells 1–3, buys at 3 (Radiant sells 2–6, buys at 6), a ninth sold at once ([[R1840001]]) and #1853's Marketplace (pending PR #1963) quoting it. Its effect is Core #98's Armor Up: Armor on its caster's hero until their next start of turn, a player modifier (M+ #12).
- **Rulings:** ST-S04-13: Reknite's "+1 Armor until next turn" is its caster's hero, until the start of their next turn (M+ #12's wording).
- **Numbers:** armor 1 ↑ (Radiant 2).
- **Check:** R275: twice the Armor.
- **Class:** C.

#### Stinkyscape #T-korrelite · Korrelite
`stinkyscape-t-korrelite` · (0) Spell, Token · Token (printed Common) · built by S04 (#1861)

> **Designer:** (0) Korrelite, Spell, Common, Token, Stinkyscape · Sells for 2~5 credits | buys for 5 · Deal 1 damage to a random enemy ~~~ Sells for 4~10 credits | buys for 10 · Deal 1 damage to a random enemy. Echo 1

- **Text:** Deal {damage} damage to a random enemy.
- **Radiant:** Echo 1 / Deal {damage} damage to a random enemy.
- **Engine:** A Resource ([[R1840000]]): a (0) Spell token gained into its player's tray, never a hand, and played from there as from a hand ([[R1840003]]), in [[R1840002]]'s roll at its rarity; its `resource` prices it: sells 2–5, buys at 5 (Radiant sells 4–10, buys at 10), a ninth sold at once ([[R1840001]]) and #1853's Marketplace (pending PR #1963) quoting it. One hit of 1 on a random enemy, the hero or a Unit, drawn as it resolves ([[R60]]). Radiant: Echo 1 ([[§6.2]]), a second resolution with a fresh pick.
- **Rulings:** none new: every reading is a ruling on `main` or the card's text.
- **Numbers:** damage 1 ↑.
- **Check:** R275: Echo 1, a second resolution.
- **Class:** C.

#### Stinkyscape #T-gellium · Gellium
`stinkyscape-t-gellium` · (0) Spell, Token · Token (printed Rare) · built by S04 (#1861)

> **Designer:** (0) Gellium, Spell, Rare, Token, Stinkyscape · Sold for 3~10 credits | buys for 10 · Heal 4 hp ~~~ Sold for 6~20 credits | buys for 20 · Heal 8 hp

- **Text:** Heal your hero {heal}.
- **Radiant:** Heal your hero {heal}.
- **Engine:** A Resource ([[R1840000]]): a (0) Spell token gained into its player's tray, never a hand, and played from there as from a hand ([[R1840003]]), in [[R1840002]]'s roll at its rarity; its `resource` prices it: sells 3–10, buys at 10 (Radiant sells 6–20, buys at 20), a ninth sold at once ([[R1840001]]) and #1853's Marketplace (pending PR #1963) quoting it. A Heal of its caster's hero ([[§6.3]] Heal X, no cap on a hero).
- **Rulings:** ST-S04-14: Gellium's "Heal 4 hp" heals its caster's hero.
- **Numbers:** heal 4 ↑ (Radiant 8).
- **Check:** R275: twice the heal.
- **Class:** C.

#### Stinkyscape #T-axnit · Axnit
`stinkyscape-t-axnit` · (0) Spell, Token · Token (printed Epic) · built by S04 (#1861)

> **Designer:** (0) Axnit, Spell, Epic, Token, Stinkyscape · Sold for 5~15 credits | buys for 15 · Deal 3 damage, lifesteal ~~~ Sold for 10~30 credits | buys for 30 · Deal 3 damage, lifesteal, Echo 1

- **Text:** Deal {damage} damage to an enemy, with Lifesteal.
- **Radiant:** Echo 1 / Deal {damage} damage to an enemy, with Lifesteal.
- **Engine:** A Resource ([[R1840000]]): a (0) Spell token gained into its player's tray, never a hand, and played from there as from a hand ([[R1840003]]), in [[R1840002]]'s roll at its rarity; its `resource` prices it: sells 5–15, buys at 15 (Radiant sells 10–30, buys at 30), a ninth sold at once ([[R1840001]]) and #1853's Marketplace (pending PR #1963) quoting it. A declared target, the enemy hero or an enemy Unit ([[R81]]); one hit of 3 with Lifesteal ([[R85]]), healing its caster's hero by what got through. Radiant: Echo 1 ([[§6.2]]), the repeat asking its target again.
- **Rulings:** ST-S04-15: Axnit's "Deal 3 damage, lifesteal" is a declared enemy target, the hero or a Unit ([[R81]]), its hit with Lifesteal ([[R85]]).
- **Numbers:** damage 3 ↑.
- **Check:** R275: Echo 1, a second resolution.
- **Class:** C.

#### Stinkyscape #T-narcor · Narcor
`stinkyscape-t-narcor` · (0) Spell, Token · Token (printed Legendary) · built by S04 (#1861)

> **Designer:** (0) Narcor, Spell, Legendary, Token, Stinkyscape · Sold for 12~25 credits | buys for 25 · Add a random Rare card to your hand ~~~ Sold for 24~50 credits | buys for 50 · Add a Radiant random Rare card to your hand

- **Text:** Add a random Rare card to your hand.
- **Radiant:** Add a random Radiant Rare card to your hand.
- **Engine:** A Resource ([[R1840000]]): a (0) Spell token gained into its player's tray, never a hand, and played from there as from a hand ([[R1840003]]), in [[R1840002]]'s roll at its rarity; its `resource` prices it: sells 12–25, buys at 25 (Radiant sells 24–50, buys at 50), a ninth sold at once ([[R1840001]]) and #1853's Marketplace (pending PR #1963) quoting it. `add_random_from_catalog` of one non-token Rare card of the sets that ship ([[R380]]), on its base face at its printed cost; a full hand burns it ([[§2.4]]). S #39 The Narcor Project gains it. Radiant: added Radiant.
- **Rulings:** none new: every reading is a ruling on `main` or the card's text.
- **Numbers:** none (one card, written "a").
- **Check:** R275: a Radiant card.
- **Class:** C.

#### Stinkyscape #T-red-narcor · Red Narcor
`stinkyscape-t-red-narcor` · (0) Spell, Token · Token (printed Mythic) · built by S04 (#1861)

> **Designer:** (0) Red Narcor, Spell, Mythic, Token, Stinkyscape · Sold for 20~40 credits | buys for 40 · Add a random Legendary or Mythic card to your hand, it costs (2) less ~~~ Sold for 40~80 credits | buys for 80 · Add a random Radiant Legendary or Mythic card to your hand, it costs (0)

- **Text:** Add a random Legendary or Mythic card to your hand. It costs ({discount}) less.
- **Radiant:** Add a random Radiant Legendary or Mythic card to your hand. It costs ({setCost}).
- **Engine:** A Resource ([[R1840000]]): a (0) Spell token gained into its player's tray, never a hand, and played from there as from a hand ([[R1840003]]), in [[R1840002]]'s roll at its rarity; its `resource` prices it: sells 20–40, buys at 40 (Radiant sells 40–80, buys at 80), a ninth sold at once ([[R1840001]]) and #1853's Marketplace (pending PR #1963) quoting it. One random non-token Legendary or Mythic card (C+ #55 Book of Greed's pool, [[R380]]) with a `costMod` of −2 ([[R65]]); a full hand burns it ([[§2.4]]). S #39's Radiant face gains it. Radiant: Radiant, at a `costOverride` of 0.
- **Rulings:** none new: every reading is a ruling on `main` or the card's text.
- **Numbers:** discount 2 ↑ (the base face only, [[R1431]]); setCost 0 ↓ (the Radiant face only, [[R824000]]).
- **Check:** R275: a Radiant card at (0).
- **Class:** C.

#### Stinkyscape #T-anomaly-found · Anomaly Found!
`stinkyscape-t-anomaly-found` · (4) Spell, Token · Token (printed Mythic) · built by S20 (#1879)

> **Designer:** (4) Anomaly Found!, Spell, Mythic, Token, Stinkyscape · {Name of Anomaly - everything except AN, AA, FO} ~~~ {Name of Anomaly - AN, AA, FO}

- **Text:** Replace your side of the field with a random Anomaly: exile every card on it, then lay that Anomaly's board in its place. Every Cry on the new board fires, with random targets and choices.
- **Radiant:** Replace your side of the field with a random rich Anomaly: exile every card on it, then lay that Anomaly's board in its place. Every Cry on the new board fires, with random targets and choices.
- **Engine:** #1855's board swap (pending): its caster's side only (ST9 Q7); every card there the Spell can reach is exiled, a Dormant card staying ([[R1839000]]), then the board's preset is laid zone by zone, a summon each, and the Cry of every Unit it laid fires in list order, its caster's choices random ([[R452]]'s random mode). The board is drawn from the match rng: on the base face one of the seven (WH, BF, AL, AT, SP, MT, DH), on the Radiant face one of the three rich ones (AN, AA, FO), all S20's data; an asteroid's restricted Resource pool rides on its card (#1855's rider).
- **Rulings:** ST-S20-2: the base face's boards are the seven the list leaves out of the Radiant three, each equally likely; the text names no board, since "Wormhole" or "Asteroid Field" would name a card ([[R279]]). ST-S20-3: the boards' "Small Asteroid" is S #169 Tiny Asteroid (ST8).
- **Numbers:** none.
- **Check:** ST8: "Small Asteroid" is Tiny Asteroid. R275: a rich board (two Narcor or Axnit fields, the Frontier Outpost) against an ordinary one (designer's).
- **Class:** C (M08, the board swap).

#### Stinkyscape #T-AI-1 · Escort Protocol
`stinkyscape-t-ai-01` · (1) Unit, AI, Token · Token (no printed rarity) · 1/3 → 2/6 · built by S21 (#1881)

> **Designer:** (the designer's "Create 10 AI Stinkyscape cards and always offer one of those"; the build designs them, ST9 Q8: defined by this brief, ST15)

- **Text:** Flying / Cry: Summon a Drone Interceptor.
- **Radiant:** Flying / Cry: Summon a Radiant Drone Interceptor.
- **Engine:** Flying ([[§6.1]]). Cry ([[R1]]): a Drone Interceptor (the set's token, [[§7]]) summoned into the leftmost open unit zone ([[R64]]), nothing with none; Radiant: a Radiant one (two random Combat Keywords, [[R21]]). An AI generated card (no printed rarity, [[R482]]'s ten), offered by #1853's Marketplace's AI lot (S21) and, once the set ships, by every AI pool ([[R1421]]).
- **Rulings:** ST-S21-1: the ten are this brief's (ST15); none declares a number ([[R482]]).
- **Numbers:** none ([[R482]]: an AI card declares none).
- **Check:** Designed by the build (ST9 Q8, ST15). R275: 2/6 and a Radiant drone.
- **Class:** A.

#### Stinkyscape #T-AI-2 · Mining Subroutine
`stinkyscape-t-ai-02` · (0) Spell, AI, Token · Token (no printed rarity) · built by S21 (#1881)

> **Designer:** (the designer's "Create 10 AI Stinkyscape cards and always offer one of those"; the build designs them, ST9 Q8: defined by this brief, ST15)

- **Text:** Gain a random Common or Rare Stinkyscape Resource.
- **Radiant:** Gain a random Radiant Common or Rare Stinkyscape Resource.
- **Engine:** Gain a Resource ([[§6.3]], `gain_resources`, [[R1840002]]) from a pool of the Common and Rare Resources, named by id (Reknite, Korrelite, Gellium): the rarity roll over those two (55 against 25), then one of the rolled rarity; into the tray, never the hand, one past 8 sold at once ([[R1840001]]); Radiant: a Radiant one. It makes no Credits: its average sale, about 4 Credits (Radiant 8), is below its Marketplace price, 5 (Radiant 10). An AI generated card (no printed rarity, [[R482]]'s ten), offered by #1853's Marketplace's AI lot (S21) and, once the set ships, by every AI pool ([[R1421]]).
- **Rulings:** ST-S21-2: the pool leaves Axnit, Narcor and Red Narcor out, so buying it to sell its Resource loses Credits.
- **Numbers:** none ([[R482]]).
- **Check:** Designed by the build (ST15). R275: a Radiant Resource (2× its effect and its price).
- **Class:** C (M03, the tray).

#### Stinkyscape #T-AI-3 · Route Optimizer
`stinkyscape-t-ai-03` · (1) Spell, AI, Token · Token (no printed rarity) · built by S21 (#1881)

> **Designer:** (the designer's "Create 10 AI Stinkyscape cards and always offer one of those"; the build designs them, ST9 Q8: defined by this brief, ST15)

- **Text:** Discover a Starship Piece. It costs (1) less.
- **Radiant:** Discover a Radiant Starship Piece. It costs (1) less.
- **Engine:** A Discover ([[§6.3]]) over the non-token cards that print Starship Piece (NEW unless S03 or S19 builds it: a pool by keyword kind), three different, the pick at `costMod` −1; Radiant: the pick added Radiant ([[§5.2]]). An AI generated card (no printed rarity, [[R482]]'s ten), offered by #1853's Marketplace's AI lot (S21) and, once the set ships, by every AI pool ([[R1421]]).
- **Rulings:** ST-S21-3: "a Starship Piece" is a card that prints the keyword on its face, never a token.
- **Numbers:** none ([[R482]]).
- **Check:** Designed by the build (ST15). R275: the piece Radiant (2× its stats).
- **Class:** B.

#### Stinkyscape #T-AI-4 · Predictive Shield
`stinkyscape-t-ai-04` · (1) Trap, AI, Token · Token (no printed rarity) · built by S21 (#1881)

> **Designer:** (the designer's "Create 10 AI Stinkyscape cards and always offer one of those"; the build designs them, ST9 Q8: defined by this brief, ST15)

- **Text:** Reveals when an enemy Unit attacks one of your Units: Cancel the attack and Freeze the attacker.
- **Radiant:** Reveals when an enemy Unit attacks you or one of your Units: Cancel the attack and Freeze the attacker.
- **Engine:** A face-down Trap in [[§4.2]] step 4's window, on a declared attack by an enemy Unit on a Unit of its controller's (Radiant: or on their hero): Cancel an attack ([[§6.3]], `cancelAttack`, [[R44]]), then Freeze the attacker ([[§6.3]]): its exertion spent on the declaration, it thaws at the cleanup of its controller's next turn, so it cannot attack then either ([[R1174000]]). An AI generated card (no printed rarity, [[R482]]'s ten), offered by #1853's Marketplace's AI lot (S21) and, once the set ships, by every AI pool ([[R1421]]).
- **Rulings:** ST-S21-4: a forced attack declares nothing, so it springs nothing.
- **Numbers:** none ([[R482]]).
- **Check:** Designed by the build (ST15). R275: its controller's hero guarded too (a broader scope).
- **Class:** B (a cancel and a Freeze).

#### Stinkyscape #T-AI-5 · Launch Sequencer
`stinkyscape-t-ai-05` · (1) Spell, AI, Token · Token (no printed rarity) · built by S21 (#1881)

> **Designer:** (the designer's "Create 10 AI Stinkyscape cards and always offer one of those"; the build designs them, ST9 Q8: defined by this brief, ST15)

- **Text:** Your Starship launch costs (1). Draw 1.
- **Radiant:** Your Starship launch costs (0). Draw 2.
- **Engine:** `starship_launch_costs { cost: 1 }` ([[R1852006]]): its player's next launch by the Activate costs (1) instead of (3), a public badge, spent by that launch; then a draw ([[§2.4]]). Radiant: (0), and draw 2. An AI generated card (no printed rarity, [[R482]]'s ten), offered by #1853's Marketplace's AI lot (S21) and, once the set ships, by every AI pool ([[R1421]]).
- **Rulings:** ST-S21-5: with no Starship yet, the price waits for the next launch.
- **Numbers:** none ([[R482]]).
- **Check:** Designed by the build (ST15). R275: (0) and two cards against (1) and one (2×).
- **Class:** C (M02, the launch price).

#### Stinkyscape #T-AI-6 · Probe Relay
`stinkyscape-t-ai-06` · (1) Spell, AI, Token · Token (no printed rarity) · built by S21 (#1881)

> **Designer:** (the designer's "Create 10 AI Stinkyscape cards and always offer one of those"; the build designs them, ST9 Q8: defined by this brief, ST15)

- **Text:** Add 2 Scanning Probe Mk1s to your hand. They cost (0).
- **Radiant:** Add 2 Scanning Probe Mk2s to your hand. They cost (0).
- **Engine:** Add to hand ([[§6.3]]): two S #166 Scanning Probe Mk1 (Radiant: two S #167 Scanning Probe Mk2) at `costOverride` 0, named so they reach the hand while the set is held; a full hand burns ([[§2.4]]). An AI generated card (no printed rarity, [[R482]]'s ten), offered by #1853's Marketplace's AI lot (S21) and, once the set ships, by every AI pool ([[R1421]]).
- **Rulings:** none.
- **Numbers:** none ([[R482]]).
- **Check:** Designed by the build (ST15). R275: two Mk2s (Lucky 1, (2) each) against two Mk1s.
- **Class:** A.

#### Stinkyscape #T-AI-7 · Combat Subroutine
`stinkyscape-t-ai-07` · (2) Unit, AI, Token · Token (no printed rarity) · 2/2 → 4/4 · built by S21 (#1881)

> **Designer:** (the designer's "Create 10 AI Stinkyscape cards and always offer one of those"; the build designs them, ST9 Q8: defined by this brief, ST15)

- **Text:** Cry: Give each of your other Units a random Combat Keyword.
- **Radiant:** Cry: Give each of your other Units 2 random Combat Keywords.
- **Engine:** Cry ([[R1]]): each other Unit its controller has on the field gains one random Combat Keyword (Radiant: two), each drawn on its own from [[R21]]'s pool ([[§6.1]] Combat keyword), never one it has, an Indestructible one never offered Taunt ([[R347]]); granted, so gone with [[R78]]'s reset. An AI generated card (no printed rarity, [[R482]]'s ten), offered by #1853's Marketplace's AI lot (S21) and, once the set ships, by every AI pool ([[R1421]]).
- **Rulings:** ST-S21-6: each Unit draws its own keyword; a Dormant card is passed by ([[R1839000]]).
- **Numbers:** none ([[R482]]).
- **Check:** Designed by the build (ST15). R275: 4/4 and two keywords each against one.
- **Class:** A.

#### Stinkyscape #T-AI-8 · Drone Uplink
`stinkyscape-t-ai-08` · (2) Field Spell, AI, Token · Token (no printed rarity) · built by S21 (#1881)

> **Designer:** (the designer's "Create 10 AI Stinkyscape cards and always offer one of those"; the build designs them, ST9 Q8: defined by this brief, ST15)

- **Text:** End of turn: Summon a Drone Interceptor. It gains Starship Piece.
- **Radiant:** End of turn: Summon a Radiant Drone Interceptor. It gains Starship Piece.
- **Engine:** End of turn ([[R62]]): a Drone Interceptor (Radiant: a Radiant one) summoned into the leftmost open unit zone ([[R64]]), then given Starship Piece (a granted keyword, kept until it leaves the field, [[R78]]): when it dies, a copy of it fuses into its owner's Dormant Starship, a granted piece fusing as a printed one does ([[R1852000]]). Nothing with no open zone. An AI generated card (no printed rarity, [[R482]]'s ten), offered by #1853's Marketplace's AI lot (S21) and, once the set ships, by every AI pool ([[R1421]]).
- **Rulings:** ST-S21-7: the copy that fuses is a plain Drone Interceptor, the granted keyword not carried ([[R1852000]]).
- **Numbers:** none ([[R482]]).
- **Check:** Designed by the build (ST15): unlike S #168 Drone Hanger, its drones build the ship. R275: a Radiant drone each turn.
- **Class:** B (a summon and a granted Starship Piece).

#### Stinkyscape #T-AI-9 · Honeypot
`stinkyscape-t-ai-09` · (1) Trap, AI, Token · Token (no printed rarity) · built by S21 (#1881)

> **Designer:** (the designer's "Create 10 AI Stinkyscape cards and always offer one of those"; the build designs them, ST9 Q8: defined by this brief, ST15)

- **Text:** Reveals when your opponent plays a Unit: Summon a Drone Interceptor. Give it Taunt.
- **Radiant:** Reveals when your opponent plays a Unit: Summon 2 Drone Interceptors. Give them Taunt.
- **Engine:** A face-down Trap on the opponent's Unit play (`cardPlayed`, [[§10.5]] step 4), going to the graveyard as it fires: a Drone Interceptor (Radiant: two) summoned into the leftmost open unit zones ([[R64]]), each given Taunt ([[§6.1]]), a decoy; one that rolled Taunt keeps it. Nothing with no open zone. An AI generated card (no printed rarity, [[R482]]'s ten), offered by #1853's Marketplace's AI lot (S21) and, once the set ships, by every AI pool ([[R1421]]).
- **Rulings:** none.
- **Numbers:** none ([[R482]]).
- **Check:** Designed by the build (ST15). R275: two decoys against one (2×).
- **Class:** B.

#### Stinkyscape #T-AI-10 · Salvage Protocol
`stinkyscape-t-ai-10` · (1) Unit, AI, Token · Token (no printed rarity) · 1/2 → 2/4 · built by S21 (#1881)

> **Designer:** (the designer's "Create 10 AI Stinkyscape cards and always offer one of those"; the build designs them, ST9 Q8: defined by this brief, ST15)

- **Text:** Starship Piece / Death: Add a random Starship Piece to your hand.
- **Radiant:** Starship Piece / Death: Add a random Radiant Starship Piece to your hand.
- **Engine:** Starship Piece ([[§6.1]], [[R1852000]]): its death also fuses a copy of it into its owner's Dormant Starship. Death ([[§6.2]]): a random Starship Piece card (T-AI-3's pool) added to its controller's hand (Radiant: Radiant), a full hand burning it. An AI generated card (no printed rarity, [[R482]]'s ten), offered by #1853's Marketplace's AI lot (S21) and, once the set ships, by every AI pool ([[R1421]]).
- **Rulings:** none.
- **Numbers:** none ([[R482]]).
- **Check:** Designed by the build (ST15). R275: 2/4 and a Radiant piece.
- **Class:** C (M02, a piece).

## ST14. Readings for the designer

Each row is the build's default until the designer says otherwise: while the set is held a change is a
correction to the rows, and once it ships a patch of its own. The rows are numbered by the foundation part
that wrote them (A1, B1, … E1); each names its card, the words the list leaves open and the reading the
build uses, and the entry in ST13 says the rest.

| # | Card | The unclear words | The default |
| --- | --- | --- | --- |

| A1 | #38 Shield Amplifier | "When this survives an attack" | Any combat it fights, attacking or attacked, declared or forced, after which it is still on the field; a Cleave splash is no attack on it; a Divine Shield that took the hit counts |
| A2 | #40 Poison Capsule | "Place 2 Plague Counters" with no card named | R689's one prompt: every counter on the one permanent it names, either side, this card among the choices |
| A3 | #40 Poison Capsule | "Deal 1 damage to all enemies per Plague Counter on this" | One hit of 1 × its counters to the enemy hero and each enemy Unit, not that many hits |
| A4 | #43 Healing Capsule | "Heal all allies" | Your hero and each of your Units, itself included |
| A5 | #49 Spacemine | "Deal attack as damage"; "2 random enemies" | The Attack it died with, buffs included (R89); two different enemies, one when one stands |
| A6 | #50–#61 the v3–v5 Weapons | "Deal N damage" with no target | A declared enemy, Unit or hero, as M05's fixture; Flak's "all enemies" is the enemy hero and every enemy Unit |
| A7 | #50, #54, #58 the Beams | "Cleave" on an Activate's hit | The same hit on each Unit beside a Unit target, composed by S06 (no effect Cleave on main); nothing more on a hero target |
| A8 | #57, #61 Flak v4, v5 | "twice", "thrice" | Whole rounds, each hitting every enemy still standing, each its own state check; written "twice" and "three times" |
| A9 | #62–#66 the Miners | "Activate 1"; "a random Stinkyscape resource" | Activate (once a turn); one Resource by R1840002's roll over every Resource, into the controller's tray, an equipped host's controller included |
| A10 | #78 Ozark | "A random [Radiant] Miner vXs is Equip this" on both faces | The bracket marks the Radiant face's change: the base face's Miner plain, the Radiant face's two Radiant; "twice" is two different Miners |
| A11 | #79 Orca | "[Radiant]"; "Embiggen 5" | The Embiggened version's: its two offered Miners Radiant (ST8, ST9); the Radiant face's change is "twice"; (5) past MAX_MANA, played only with extra mana |
| A12 | #80 Galaxy | "Equip this with each Radiant Miner vX" | All five Miners, Radiant, fused v1 to v5, no prompt |
| A13 | #81–#86 the Weapon carriers | "Discover 2 Rare Weapons; Equip them into this" | Discovers in turn, each three of the Weapon-tagged cards of that rarity it does not hold, each pick fused into it (R1841002) |
| A14 | #86 Prototype-X2 | "Float reduces Tribute requirement by 1" | 1 off its Tribute per Float of its owner's, whatever its size, in hand and deck, never below 0 (R1841003) |
| A15 | #90 After Burner | "gain 2 random Combat Keywords" | The Starship it is fused into gains them, from the combat keyword pool, ones it lacks |
| A16 | #106 Warpfield Amplifier | "set all of your Units to attack position and give them +2 attack" | Every Unit of yours as the Starship launches, the Starship included: those in Defense switch, spending nothing (R20); the +2 Attack lasts |
| A17 | #107 RR-5000 | "Start of Turn: Gain +4 attack" | A lasting +4 Attack each of your turns, adding up |
| A18 | #124–#126 Jlockeed's | "a random Starship Piece" | A random non-token card printing Starship Piece; the Dormant Starship never one; the pool names the set while it is held (needs a keyword query field: NEW) |
| A19 | #126 Permanent Procurement Lobbying | "at the start of your turn"; "it costs (1) less" | Each of your turns for the rest of the game, C+ #52's modifier; a cost modifier that never takes it below (0) |
| A20 | #127 Siege Tank | "attacks a random enemy" | A forced attack on an enemy drawn among those it may attack (Flying holding), Taunt not binding |
| A21 | #127–#129 | "Embiggen: gain Starship Piece (and Equip)" | "Embiggened (N):": the Embiggened version is a Starship Piece (and offers Equip as a play), for good |
| A22 | #129 Missile Pod | "floor(attack / 4)" | Its Attack as it stands at the end of turn, divided, rounded down; equipped, the host's Attack |
| A23 | #130 Retrofitting Program | "it gains Starship Piece" | Granted in the hand and kept onto the field, read on its death; a Spell given it fuses nothing |
| A24 | #131 Spacerace Stimulus | "plays a Starship Piece" | Either player plays a card that prints or has the keyword; its reveal is its animation |
| A25 | #132 Defensive Crystal | "Give your hero 1 armor" | §6.1 Armor on the hero, a per-hit reduction kept for the rest of the game (R124) |
| A26 | #133 Ultra Capacitor | "for each other starship piece launched" | Each other piece fused into the Starship it launches with, by its memory (R1852005); the Radiant face counts itself |
| A27 | #134 Felinor Containment Chamber | "stats equal to this units" | Its Attack and maximum Health as it died, buffs included; the base Felinor Token, summoned X/X |
| A28 | #136 Magic Module | "a random spell … that deals damage" | A non-token Spell marked dealsDamage (R1748001), Field Spells out (needs a query field: NEW) |
| A29 | #137 Firststrike Capability | "activate {discover} a War doctrine"; "Launch it" | Discover one of the four War Doctrines and cast it free; both only while building a ship, the launch first and free |
| A30 | #137.1–#137.4 the War Doctrines | "For the rest of the game your units have X, your Starship …" | Every Unit you control, later ones too, for the rest of the game (a NEW modifier); the Starship line once, as cast (ST9 Q2); "your Starship" is your launched one, none doing nothing |
| A31 | #137.1 Land Doctrine | "attacks all enemy units" | Each enemy Unit in lane order, one combat each, stopping if the Starship leaves |
| A32 | #137.3 Air Doctrine | "damage split randomly among all enemies" | §6.3 Damage's split: that many hits of 1, each to a random enemy still standing |
| A33 | #137.4 Space Doctrine | "fill your hand with the Starship Pieces that are part of the launched Starship" | One copy of each piece it remembers, oldest first, until the hand is full; no repeats |
| A34 | #140 Rush Token Launcher | "Target an enemy"; "obey's taunt"; "deals half damage to heroes" | An enemy Unit (ST9), a Taunt one while the enemy has one; nothing to halve |
| A35 | #141 Fruit Launcher | "Target a permanent, cast a random fruit" | The Fruit aimed at that permanent where its target may be one, its other choices random |
| A36 | #142 Jade Launcher | "shoot a Auspicious Rock at it {deal 2 damage, you gain the reward}" | 2 damage to the target and the Rock's reward (Dud 20%, Jade 75%, Red Jade 5%) to your hand; no Rock made, no health lost |
| A37 | #143 Book Launcher | "harmful book", "helpful book" | Two lists in S06's script, by whose card a Book targets; "an enemy"/"an ally" a Unit or a hero |
| A38 | #144 Prepare For Liftoff | "one of your Starship Pieces dies"; "Resummon it" | A piece you controlled as it died; a summon of that card from its owner's graveyard, no Cry, after its copy fused |
| A39 | #175 Frost Module | "Freeze an enemy" | An enemy Unit: a hero is never Frozen |
| A40 | Dormant Starship | "Activate: Pay (3) to Awaken and trigger all Cries"; no Radiant face | The engine's launch: its fused Cries, then its fused When launched (R1852003); R349's fallback for its Radiant face |
| B1 | Drone Core I (S #27, ST-S07-1) | "Discard a card" | A random card of the caster's hand ([[R682]]); with an empty hand nothing is discarded and the Interceptor is still summoned |
| B2 | Drone Core VIII (S #34, ST-S07-2) | "Fill your board with Drone Interceptor" | Every empty unit zone of the caster's, as a fill reads (C+ #62), never the backrow, though a Flying Unit may be played there |
| B3 | Drone Core X (S #36, ST-S07-3) | "Summon the Drone Battleship"; "Radiant Drone Battleships make all layers Radiant" | One layered summon ([[R1842000]]) of the Meltdown, Propagation and Shield Layers, bottom first, into one unit zone; a zone that cannot take the pile takes none of it; the Radiant face makes all three Radiant |
| B4 | Drone Sympathizer, Core Researcher, Drone Commander (S #41, #42, #45, ST-S07-4) | "random Drone Cores" | The cards tagged Drone Core (#27–#36), each tier as likely, repeats allowed ([[R60]]); the pool names no set, so it is empty while the set is held ([[R1420]]) and opens with the release |
| B5 | Drone Commander (S #45) | "they cost (1) less" | Each added card carries a `costMod` of −1 ([[R65]]), kept until a graveyard or exile ([[R766]]) |
| B6 | Great Replacement Theory (S #44, ST-S07-5) | "The first time you play each Drone Core tier add the subsequent tier" | From this cast on, for the rest of the game: the first play (a cast counts, [[R70]]) of each of Drone Core I to IX adds the next numeral; X adds nothing; a Drone Core played before the cast counts for nothing; each cast keeps its own record, so two stack; the Radiant face adds Radiant ones |
| B7 | Drone Interceptor, Drone Corvette (ST-S07-6) | "<has 1 random combat keyword>" | Keymaster Keenus's Aura ([[R781004]]): drawn from [[R21]]'s pool as the drone arrives, kept until it leaves, never one it has (so not Ranged or Flying again) |
| B8 | Drone Battleship (Shield Layer) (ST-S07-7) | "Fatal damage a friendly ally would take is redirected to this" | Unan's lethal guard ([[R1204]]): the hero and the controller's other Units, through the layer's own Armor, only while it tops its pile ([[R1842001]]) |
| B9 | Drone Battleship (Propagation Layer) (ST-S07-8) | "Activate 2: summon a Drone Interceptor, Activate 1: summon a Drone Corvette" | Two abilities counted apart ([[R1751000]]): up to two Interceptors and one Corvette a turn, offered only while the layer tops its pile ([[R13]]) |
| B10 | Drone Battleship (Meltdown Layer) (ST-S07-9) | "End of turn: Take 6 damage"; Brittle 3 beneath the pile | The layer hits itself, never its hero; beneath a Stack it runs no hook ([[R13]]), and its Brittle waits while it is buried, as a Dormant countdown waits ([[R1839003]]), ticking once it tops the pile |
| B11 | The Narcor Project (S #39, ST-S04-1) | "Add a Narcor to your hand"; "Discard a Narcor" | A gain into the tray ([[R1840000]]); the discard is the oldest Narcor there ([[R1840004]]), the Activate's cost, so the Activate is offered only with one in the tray; the Radiant face takes the oldest Narcor or Red Narcor |
| B12 | The Narcor Project (S #39, ST-S04-2) | "reduce Brittle by 1"; "If this died to Brittle" | The Activate lowers the count by 1; driven to 0 it crumbles at the state check, a Brittle death; "died to Brittle" is a `crumbled` destroy, never another destroy or an exile |
| B13 | The Narcor Project (S #39, ST-S04-3) | "its End of Turn effect doubles and becomes permanent" | A rest-of-game effect of its controller's (C+ #52's shape): two Narcors (Radiant: two Red Narcors) at the end of each of their turns, outliving the card; a second Project that crumbles adds a second such effect |
| B14 | Tractor Beam (S #67, ST-S04-4) | "Equip" with "Start of your turn: Gain …" | Equipped, its host gains the Resource at its controller's start of turn (the fused text, [[R1086]]) and is no Starship Piece ([[R1852001]]) |
| B15 | Badger, Honey Badger (S #75, #76, ST-S04-5) | "When this attacks" | As the attack is declared and still stands, before the combat (Hearthstone's "whenever this attacks"), once per attack, forced ones included ([[R53]]); one that dies in the combat keeps its Resource; C #45 Nature Titan answers after the combat, which S04 may follow instead |
| B16 | Siva 1 (S #87, ST-S08-1) | "Allure a random enemy Acclaimed Unit" | One of the enemy's acting Units whose tags, granted ones included, hold Acclaimed; the Allure lands as K-Pop Fanatic's ([[R1045000]], [[R963]]); none, nothing |
| B17 | Siva 1 (S #87, ST-S08-2) | "If there isn't one, summon a Frozen one to Allure" | A random non-token Acclaimed Unit (never Siva 1, [[R387]]) summoned on the opponent's side, Frozen, then Allured, so it is stolen at the next start of turn; nothing when their unit row is full |
| B18 | Spice (S #89, ST-S04-6, ST-S04-7) | "Can be sold on the Resource marketplace"; "{For AI: … only appearing once a Spice is potentially relevant}" | A sale from its owner's hand, in their main phase, while #1853's Marketplace is open, for Credits, as public as a Resource's sale; Spice stays a hand card, never a Resource; the sale is offered only while the Marketplace is open. Its price: QUESTION (the part's notes) |
| B19 | Ocean Tsunami (S #96, ST-S04-8) | "discard your hand then fill your hand with random Stinkyscape resources" | Every hand card discarded, no Resource; then the Resource tray filled ([[R1840004]]), so nothing is sold |
| B20 | Asteroid Field (S #97, ST-S04-9) | "all heroes and units"; "all enemy heroes and units"; "add two … to your hand" | Both heroes and every Unit, one hit each; the Radiant face the enemy hero and every enemy Unit; the Resources gained into the tray ([[R1840000]]) |
| B21 | Megabase (S #116, ST-S04-10) | "choose to give both players 5, 10, or 15 credits"; "your opponent gets 5 less" | Three declared modes, each player gaining that many; the Radiant face gives the opponent 0, 5 or 10; "Can't be tributed" is Untributable ([[R1220]]) |
| B22 | Comet Crash (S #155) | "Death: Add 3 Stinkyscape resources to your hand" | Three random Resources gained into the tray ([[R1840000]]) on any destroy, Brittle's included |
| B23 | Tiny, Medium, Large and Huge Asteroid (S #169–#172, ST-S04-11) | "Activate: Take N damage. Tribute this"; "Gain N Stinkyscape Resource" | Its controller's hero takes N, then the asteroid's Sacrifice, whose Death gains N random Resources ([[R1840006]]); a Tribute for a play, or any other death, pays as well; an anomaly board's pool rider restricts the roll ([[R1840002]]) |
| B24 | Abandoned Spice Tower (S #173, ST-S04-12) | "Add a three Spice to your hand" | Three Spice cards (S #89) into the hand, a full hand burning the rest: Spice is a card, not a Resource |
| B25 | Reknite, Gellium, Axnit (ST-S04-13, ST-S04-14, ST-S04-15) | "+1 Armor until next turn"; "Heal 4 hp"; "Deal 3 damage, lifesteal" | The caster's hero until the start of their next turn (M+ #12's wording); a heal of the caster's hero; a declared enemy target, the hero or a Unit ([[R81]]), its hit with Lifesteal ([[R85]]) |
| B26 | The Syndicate, The Trade Union (S #164, #165, ST-S08-3) | "Your cards have Tradable" | The cards in your hand, where alone Tradable does anything ([[R1838000]]), lent by the aura while the Field Spell acts |
| B27 | The Syndicate (S #164, ST-S08-4) | "a 30% chance to be exiled and a 30% [70%] chance to become Radiant" | One roll on the card the Trade put into the deck: exiled from the deck in the first 30, made Radiant there in the next 30 (Radiant 70), else nothing, as the Radiant face's 30 and 70 make 100; the draw stands; past 100 after a tune, Radiant takes what is left |
| B28 | The Trade Union (S #165, ST-S08-5) | "you can Trade up to two cards a turn" | Its controller's Trades a turn are 2 while it acts (`tradesPerTurn`), not 1 + 2; two Unions still make 2 |
| B29 | Job Application (S #112, ST-S10-1) | "Send a card from your hand in as your resume" | An Activate of the card's, a `byEffect` goal ([[R1854001]]); the resume is set aside with the card and comes back as Stage 2's reward; exiled if the Job Application leaves the field first |
| B30 | Job Application (S #112, ST-S10-2) | "Hear back in 2-6 turns"; "Create some evaluation algorithm … a little bit of RNG"; "an iKwzki upload always hears back in 6" | S10's evaluation: a match-rng draw between 2 and 6 weighted by the resume (S10's table of its cost and rarity), an iKwzki (S #110) always 6; on the Radiant face Lucky 1 draws twice and keeps the shorter wait |
| B31 | Job Application (S #112, ST-S10-3) | "send in another application (return to Stage 1) and triple rewards" | Each re-application triples Stage 2's Buffs and Stage 3's Paychecks again, cumulatively (×3, ×9, …); S10 may cap the factor with a named constant. QUESTION (the part's notes) |
| B32 | Job Application (S #112, ST-S10-4) | "At the start of every turn get a Paycheck" | Each of its controller's starts of turn from the one after the Activate, the card's own effect while it stays on the field; resummoned at Stage 1 after a death, the Paychecks stop |
| B33 | Job Application (S #112, ST-S10-5) | "Death: Resummon this at Stage 1" | The same card summoned from its owner's graveyard into its own backrow zone, else the leftmost open one, its line at Stage 1 ([[R78]]); with none it stays in the graveyard; an exile ends it |
| B34 | Paycheck (S #112.1, ST-S10-6) | "go all in on red with a 18/39 chance to gain double the reward" | One luck-based roll at 18 in 39: a win doubles each part (4 mana, draw 2, 40 Credits), a loss gives nothing; the Radiant face's Lucky 1 rolls again and keeps the win |
| C1 | #1 Freeze Ray | "Freeze a Unit and its adjacent Units" | The target's neighbours in its row on its side, Cleave's adjacency (§3.1), whichever side the target stands on; a Unit with no neighbour is Frozen alone (ST-S13-1) |
| C2 | #2 Absolute Zero | "You may give it Indestructible" | A choice of two modes made with the play (R81); given, the Indestructible stays until the Unit leaves the field, a Vanilla included (ST-S14-1) |
| C3 | #3 Meteor | "Lock its zone"; "Tramples onto adjacent Units and the enemy hero" | The target's zone is Locked after the hit, emptied or not; "the enemy hero" is the target's controller's hero (R1848001), the enemy's whenever the Meteor is aimed at an enemy Unit (ST-S14-2) |
| C4 | #5 Masochism Unmasked | "(up to 20)"; "double it (up to 40)" | The cap is per turn (ST9) and counts what is taken; the Radiant face doubles what is dealt on the same 20 (R1849004); the hit that reveals it is taken (ST-S15-1) |
| C5 | #9 Ticketed | "Stack a permanent under this"; "Death: return that permanent to its controller" | Hold beneath (§6.3), not the Stack keyword: another face-up permanent of either side; released when Ticketed leaves the field any way (R1850003), summoned for its controller as it was taken (ST-S15-2) |
| C6 | #10 Otherworldly Endurance | Radiant "a Radiant Inner Fire, a Divine Spirit, and a Power Word Shield" | All three Radiant, as #11, #12 and C+ #32 (ST-S13-2) |
| C7 | #10.1 Inner Fire | "Set a units attack equal to its health"; "if ally" / "if enemy" | A Buff of the difference, its stats read now through the layers (an existing primitive), so an Attack aura that leaves later takes its part; an Attack-only set at layer 1 would be new. "Ally" is a Unit its caster controls (ST-S13-3) |
| C8 | #10.2 Divine Spirit | "Set a units health to double its health" | Hearthstone's Divine Spirit: +Health equal to its Health now, current and max together, not a set (ST-S13-4) |
| C9 | #11.1 Frostbolt | "a target"; Radiant "Freeze it; Cleave" | Any Unit or hero (a hero is never Frozen); Cleave is the Spell's hit on the target's neighbours, and only the target is Frozen (ST-S13-5) |
| C10 | #11.2 Arcane Missiles | "echo 2", "echo 5" | Echo 2 and Echo 5 printed: three resolutions (Radiant six), each drawing its own enemy among those still standing (ST-S13-6) |
| C11 | #12.1, #12.2 Shadow Word Death, Shadow Word Pain | "(5) or more attack", "(3) or less attack"; Radiant "Cleave" | The numbers are Attack thresholds, not costs; the Radiant face destroys the target's neighbours too, whatever their Attack (ST-S13-7) |
| C12 | #12.3 Psychic Scream | "all permanents"; "your opponents deck"; "they cost (2) more {max (4)}" | Every card on the field of both sides, face-down ones included, its caster's own too (Hearthstone's Psychic Scream), each becoming the opponent's card in their deck; a unit token is gone; Radiant +2 to each, up to (4) (ST-S13-8) |
| C13 | #13 Asphyxiants Cloud | "should deal 1 poisonous damage to all enemies" | ST8's text; "all enemies" is the enemy hero and every enemy Unit; the next Spell's Poisonous waits through other plays and ends with the turn (R1848002) (ST-S14-3) |
| C14 | #21 Moonberry | "a random Other-Origins card" | Any token tagged Other-Origins (C+ #32.1–#32.3, S #10.1–#12.3), the tag's only cards, of the sets that ship; the pool takes tokens, as R1837000 leaves to the first card naming it (ST-S13-9) |
| C15 | #22 Gatinsir, #174 Aerelret | "when an opponent attacks"; "AMBUSH"; the Radiant faces printed without the reveal line | An enemy Unit's declared attack; AMBUSH is ST9's label (the summons arrive before the combat, which goes on as declared); the Radiant face keeps the trigger and its 30% chance (ST-S15-3) |
| C16 | #23, #24 Warp Disruptor, Warp Disruptor II | "Units"; "when the enemy summons a unit"; "all unit from your side" | Every Unit of both sides (ST9, R1849006); C #5 Tesla's arrival, however summoned, a played Unit answered after it resolves; II's is each of your Units in lane order (ST-S15-4) |
| C17 | #99 No-No Words | "less than [4X] attack" | Strictly below 4 × X; X of 1 reaches 3 Attack or less (ST-S14-4) |
| C18 | #102 Book of Nono-Words | "their prices are set at 1,2,3,4" | Set X (R1848003), in that order, each costing exactly its X |
| C19 | #103 Vxyl | "On reveal: Add a No-No Words to your hand" | Its Cry, which a Cloaked Unit's reveal runs (R783006), so a face-up play runs it too; a reveal-only trigger would be new (ST-S14-5) |
| C20 | #117 Core System | "Bounce all permanents"; "next turn" | Every card on the field of both sides, face-down ones included (each pile's top, R13); its caster's next refresh (R1848004) (ST-S14-6) |
| C21 | #118 Secure System, #119 Unsecure System | "Set all units' attack to 0"; "Double the attack of all units"; "gain first strike" | A Buff of minus (plus) each Unit's Attack now, existing primitives, a later Buff adding on top; the First Strike is for good, not for the turn (ST-S14-7, ST-S14-8) |
| C22 | #120 Wild System | "{allies can attack each other, max 20 attacks}"; Radiant the same text | R1848005's melee and its 20; the Radiant face gains 2 mana next turn, not 1, since R276 refuses one text (R1857001) (ST-S14-9) |
| C23 | #121 System Schematic | "Add a Core System, Secure System, Unsecure System, or Wild System" | Its player's choice of one, a mode (R81), not a random pick (ST-S14-10) |
| C24 | #135 Sacked | "Reveals when your opponent plays a Unit. Stuff it in a 0/4 Sack"; "for yourself" | After the Unit resolves, its Cry first (Core #41 Sheepish's timing); the Sack for its controller, or on the Radiant face for the Trap's controller, who then gets the Unit back from it (R1850003) (ST-S15-5) |
| C25 | #135.1 Sack | "Death: Summon the Unit stuffed into this" | The Sack leaving the field any way (R1850003), the Unit summoned for the Sack's controller (ST-S15-6) |
| C26 | #138 Pocket Peptides | "about to die by combat"; "on all your Units" | The would-die point of a Unit combat damage marked (§4.5 step 1), not a Spell's hit or a destroy; one Radiant cast at each of your Units (ST-S15-7) |
| C27 | #139 Wormhole | "a beneficial Spell on themselves or a permanent they control" | R1850004's: chosen targets, one at least, all its player's own hero or cards on the field; a Spell with no target is not one (ST-S15-8) |
| C28 | #145 Fortify Flesh | "Gain 3~5 Armor" | The hero's Armor, a reduction of each hit kept for the game (§6.1 Armor X), not Hearthstone's pool; one roll from 3 to 5, both ends included (ST-S13-10) |
| C29 | #146 Flash Freeze | "Freeze all permanents" | Every Unit, the only permanents a Freeze reaches (§6.1 Frozen) (ST-S13-11) |
| C30 | #153 Stimmycheck | "At the end of your turn"; "+10% chance at Inflation" | The end of this turn, a delayed add; 10 points higher (R1848006), never past 100, carried on the copy (ST-S14-11) |
| C31 | #156 Catalyst Comet | "Add a Catalyst card"; "Shuffle this back into your deck" | A random non-token Catalyst card; the Comet itself after it resolves, not a copy, keeping its Radiant flag (ST-S13-12) |
| D1 | #6 Tylannt | "Summon The Greatest Heist" (Radiant) | The summon names its card, so #7's Cry (its Discover) runs ([[R1]], [[R411]]); then its line, not yet open, opens Stage 1 and has it completed with the rest ([[R1854001]]). A full backrow summons nothing and the completion reaches your other Quests alone |
| D2 | #7 The Greatest Heist | "Discover an Acclaimed Unit or a Jellybean" | One Discover over one pool: the non-token Acclaimed Units and the Jelly Bean cards (Core #26–#29) of the sets that ship; a Jelly Bean (a hand card) can then meet Stage 1 ([[R1854002]]) |
| D3 | #7 The Greatest Heist | "Add 3 random Anomalies or Resources to your hand" | Three rolls, each at even odds an Anomaly Found! added to your hand or a random Stinkyscape Resource gained into your tray by [[R1840002]]'s roll (a Resource never enters a hand, [[R1840000]]); Radiant: each Radiant. The 3 is a declared number (finds) |
| D4 | #7 The Greatest Heist | "Stage 1: Initiation / Quest: … / Reward: …" | One line per stage, "Quest: Stage N, name. Goal. Reward: reward.", since "Quest:" starts a line of its own ([[R366]]). S10's Job Application should print its stages the same way |
| D5 | #7 The Greatest Heist | "Steal your enemy's mana crystals" | Your opponent's: their crystals drop to 0 and yours rise by as many, up to the cap ([[R1854003]]) |
| D6 | #14 Arm of the Jempire | "give it +2 attack" | A buff that stays (`buff`), not until the end of the turn; each declared attack on a Taunt Unit of yours gives it again, the Arm's own included when it has Taunt ([[R1849008]]) |
| D7 | #15 Arg Arg Defender | "adjacent Units" | The Units in the unit zones beside it on its own side; never the enemy's across the lane |
| D8 | #16 Rod of Ages | "The first unit played on this is Equipt" | While it has hosted none, your first Unit played may be played onto it as an Equip play, the Rod the kept host and the Unit fused into it ([[R1086]], [[R77]]); later Units play as usual. NEW (S16) |
| D9 | #16 Rod of Ages | "Upon reaching 10 Keywords" | Keyword kinds through the layers, each counted once ([[R1850005]]); Animated and Flying count. The Radiant face has nothing left to reach |
| D10 | #16 Rod of Ages | stats; "two random Combat Keyword" | 1/1 → 2/2 (the planner's, open question 1); 2 random combat keywords a turn |
| D11 | #17 Imprisoned Ship Smith | "summon a Fiery Waraxe" | Into your leftmost open backrow zone; it animates that same turn, the awakening coming before the "Animated on your turn" step ([[R1839001]]); a full backrow, nothing |
| D12 | #18 Imitated Instrumentality | "Summon the 3 Brains" | MELCHIOR, BALTHASAR and CASPER, named so each is a ref ([[R279]]), summoned in that order into your leftmost open backrow zones; one with no zone is not summoned |
| D13 | #18.1 MELCHIOR | "the enemy has more Units than you" | Your opponent's Units on the field against yours, counted as `active_units_of` counts (a Dormant card no Unit), at your start of turn |
| D14 | #18.1 MELCHIOR | "Discover a Combat Keyword" / "3 random Combat Keywords" | Three different keywords of [[R21]]'s pool offered, the pick given to each of your Units (NEW: a Discover over keywords); Radiant: each Unit draws its own three |
| D15 | #18.2 BALTHASAR | "less than half your starting life" | Your hero's health below half its starting health, the seat's own (a handicap's included, [[R180]]): 30 votes at 14 or less |
| D16 | #18.2 BALTHASAR | "the last card you played"; "(1) more" | Your last face-up play of the game, a cast included ([[R70]]); none before a play. A copy ([[R57]]) at +1 cost, not capped at (4) |
| D17 | #18.3 CASPER | "you Floated last turn" | You Floated at least once on your previous turn |
| D18 | #18.3 CASPER | "a random enemy Unit on the field and hand" (Radiant) | Two Allures: a random enemy Unit on the field ([[R963]]) and a random Unit card in your opponent's hand, taken into yours at the start of your next turn if it is still there ([[R1140]]'s hand watch, [[R12]]). NEW (S11): Allure has no hand form on `main` |
| D19 | #19 Imprisoned Cosmality | "deal 10 damage split among enemies" | Ten hits of 1 on random enemies, the hero included, a Unit at 0 passed by (C+ #3's split); each hit's source is this Unit, so each pierces Armor ([[R346]]) |
| D20 | #20 Tnack Trav | "a Legendary or Mythic Unit on the Field" | A declared target on either side, not itself, by its rarity: a token's rarity is Token, whatever it printed |
| D21 | #25 KY's Tricknology | "Choose a zone … lands on each zone it covers" | ST9's coordinates; each covered zone that is open gets a random Starship Piece for the side it is on (none for an occupied or Locked one), a backrow zone taking it as a Unit standing there ([[R1081000]]); Radiant: only your zones. NEW (S18), which may keep pieces to unit zones |
| D22 | #26 ZolarKeth | "Its Activate is locked" | The summoned University cannot use its own Activate; no zone is Locked, and ZolarKeth's Activate still runs it |
| D23 | #26 ZolarKeth | "or Activate your Universities"; "then" | The base face's "or" is a choice of modes, the Radiant face's "then" both in order; every University of yours runs its Activate's effect, spending none of its uses. NEW (S17): an Activate run by another card |
| D24 | #37 Kwzka | "Allure all enemy Units played that cost less than this" | Each Unit your opponent plays (not summons) while Kwzka acts on the field, whose cost paid is less than Kwzka's cost now, is Allured as it arrives ([[R963]]) |
| D25 | #37 Kwzka | "Remove all Allures on the field" | Every Allure mark on both sides comes off and its steal is cancelled, Kwzka's own included. NEW (S17) |
| D26 | #46 Circadian Halfcycler | "the first 3 actions of your turn" | The first 3 plays or attacks of every one of your turns while it stands (ST9, [[R1844000]]), answers belonging to the action that asked |
| D27 | #70 Z'Zoth | "Death Units" | Unit cards whose own text prints a Death; a Death granted on the field is gone in the graveyard ([[R78]]) |
| D28 | #71 A'Zoth | "Activate each Cry with random targets" | Trigger a Cry ([[R467]]) on each Unit it summoned, its targets and choices drawn from the match rng, no prompt asked. NEW (S16); the summon itself fires none ([[R1]]) |
| D29 | #77 Yuma | Lucky 1 (Radiant) | The trade's Discover options are drawn twice, keeping the set better for Yuma's controller (default: the set whose cheapest card costs most, the opponent giving the worst); S18 names the comparator |
| D30 | #88 Siva 2 | "Your Humans"; "Fill your hand with Bigots" | Your Human Units on the field, this one included; Bigots (Core #2) until your hand is full, none burned |
| D31 | #91, #92 The Propagation's Wasp and Hornet | "After these attacks" | After this attacks, a forced attack included ([[R1849001]]): the copy first, made before it evolves, so the copy lacks Flying |
| D32 | #94 Tmack Trav | "After this kills a unit" | The Unit it attacked destroyed in that combat (C+ #73.1's reading); a kill while defending does not count; no cap on the attacks it chains |
| D33 | #104 Reflective Plating | "dealt to the enemy hero" | Dealt again: the Plating still takes it ([[R1849003]]) |
| D34 | #109 xxFreshJack | "Play a random BUSTED card that exists anywhere in the game's code" | Cast (free, a play for every rule, [[R70]]) one random card of `{ busted: true, everySet: true }` ([[R1846001]]), its choices random; Radiant: Discover one, Radiant, and cast it with your choices |
| D35 | #110 iKwzki | "next turn" | At the start of your next turn ([[R1850001]]) |
| D36 | #111 Shockwave447 | "Replace both player's hands with complicated cards"; "Make yours Radiant" | Each hand moved to its graveyard and refilled with as many random complicated cards as it held ([[R1846000]]); your new cards arrive Radiant |
| D37 | #113 A Very Dandy Lad | "all Permanents that cost (2) or less" | Both sides' face-up Units and backrow cards at their current cost ([[R1850002]]); never itself |
| D38 | #114 Arctic Citadel | "Fuse all cards from your Exile into this" | Every card of your exile pile fused into it in one Fuse, oldest first ([[R77]]); they cease to exist, the exile counter unchanged |
| D39 | #115 Scornboi | "all non-human, non-starship piece, non-starship" | Units only (ST9): every Unit with neither the Human tag nor the Starship Piece or Starship keyword ([[R1850002]]) |
| D40 | #123 Gokugains | "nerfed 3 times secretly"; "nerfs avoid cost" | Hidden Nerfs on a random Unit of their hand ([[R1845000]]), landing as it is played and never on its cost ([[R1845001]]); you do not see them either |
| D41 | #147 IN33DH3 | "For the rest of your turn" | Your own turn only, until its cleanup ([[R1844001]]) |
| D42 | #147.2 Wardrum Blob | "Do something REALLY helpful" | `helpful_cast` with `radiant: true`, one cast ([[R1844002]]) |
| D43 | #152 Sanctum | "all cards in your Deck and GY Flashback"; "gain (4) mana" | An aura lending Flashback to each card in your deck and graveyard; a card that spent one keeps it spent ([[R783013]]), so each plays from the graveyard once. NEW (S16). The Radiant Cry gains 4 mana this turn |
| D44 | #157 The Gauntlet | "No other unit can target them"; "add Brittle +2 to this or Tribute this" | Attacks only ([[R1849005]]); the Radiant Activate is a choice of two modes: +2 to its Brittle count, or Tribute it |
| E1 | #4 Olivian Expanse | "it gains Win the Game"; "If this is also the last card in your hand" | The discovered card carries the win: its controller wins as it is played (a cast too), at the state check after that play. Read as Olivian Expanse resolves, its caster's hand holding no other card |
| E2 | #4 Olivian Expanse, #154 Dice of Doom | "a Terminus card"; "every Terminus card" | The Terminus pool: the six tokens #4.1–#4.6 and Core #100 Ceaseless Void ([[R1857002]]); Discover's three are different, and Dice of Doom adds one of each of the seven |
| E3 | #4.1 Farewell | "all permanents and graveyards"; Radiant "hands, and decks" | Every permanent of both sides, then every card in both graveyards; the Radiant face also both hands and both decks. A Dormant card stays |
| E4 | #4.2 KY's Last Exam | "Solve an impossible problem"; "Echo this"; "You snuck in a cheat sheet" | A four-option `answer` problem no one can work out, so a one-in-four guess for every answerer, the AI included; each right answer gives the card one more resolution (Echo +1, a new problem each time), up to a cap; the cheat sheet marks the right option for its answerer alone |
| E5 | #4.4 Last Word | "Reveal when you take lethal damage"; "End the Turn. You then take an extra turn"; "Lose the game" | The lethal guard's moment on your hero: the hit is prevented. The next turn is yours (an extra turn on your own turn, your normal next turn on the opponent's), your Units with Charge during it; at its end the opponent wins |
| E6 | #4.5 1v1 Final Destination | "Lock all but one random lane"; "all non-permanents" | One lane drawn at random stays open and the other four are Locked whole; the non-permanents are the Spells in each hand |
| E7 | #4.6 The Big Crunch | "Cast every Call to Chaos" | One of each edition, in catalog order, each its own cast (Radiant: each Radiant) |
| E8 | #95, #159, #160, #161, #162 (the bundles) | "Add a Radiant X, Y and Z"; "Paragon mk1, Paragon mk2, Paragon X" | Every card listed is Radiant, each on its own price; the names are #72 Paragon Mk1, #73 Paragon Mk2 and #74 Paragon-X |
| E9 | #163 Swear Loyalities | "Exile the other options you didn't choose {wherever they are}" | Every card its caster owns of the four bundles not chosen, in hand, deck and graveyard; the opponent's untouched |
| E10 | #148 Alteran Reach | "Traverse the endless expanse"; "rewards along the way, think Reach to the Summit" | One travel of the starscape map ([[R1847001]]), its nodes' rewards the map's (Heal, Draw, Mana, Damage) |
| E11 | #149 Wildar | "a Call to Chaos that costs (0)"; "a Radiant Call to Chaos" | Two random editions, picked apart; the opponent's at (0), yours at its printed (4) |
| E12 | #150 Call to Chaos (Stinky Edition) | "Summon 3 random Quests or Wincons"; "Refresh mana so you can play it"; "Place all enemies in attack position"; "all Acclaimed cards have Death" | The Field Spells tagged Quest or Wincon (the Spells among them cannot be summoned); a refresh of as much as the card costs; the enemy Units; both players' Acclaimed cards for the rest of the game, each death's cast its controller's |
| E13 | #151 Call to Chaos (Scape Edition) | "Trigger any Anomaly"; "they have stack"; "Launch a ship with 4 Random Starpieces fused into it"; "a very cheap rate (always 20 Credits)" | A board swap from all ten boards; a Drone Interceptor stacked on top of each of your unit zones (the Stack keyword alone would do nothing on the field); four pieces fused into your Dormant Starship (a new one if none), launched for free; a Marketplace lot of yours alone at a fixed 20, one copy each cycle |
| E14 | #154 Dice of Doom | "End of your turn add this back to your hand"; "Embiggen permanently gain Lucky +1"; Radiant "Roll a 80 sided die" | From its graveyard at your end of turn; the Embiggened play gains Lucky +1 for good, before it rolls; the Radiant face rolls its 80-sided die twice, since one roll is 1.25× the base face's chance and [[R275]] asks about 2× (raised) |
| E15 | #166 Scanning Probe Mk1, #167 Scanning Probe Mk2 | "5% … 20% … Else: future Scanning Probes have +1% & +4% chance" | One roll in two bands, 5% Radiant then 20% plain (25% in all); each miss of yours adds 1 and 4 to your later scans with either Probe, and a find keeps the count |
| E16 | Anomaly Found! | "{Name of Anomaly - everything except AN, AA, FO}"; Radiant "{AN, AA, FO}" | The base face lays one of the seven boards, the Radiant face one of AN, AA and FO, each equally likely; only its caster's side (ST9 Q7) |
| E17 | #148.1 Edge of the Universe | "Your opponent does not lose any elo since you fled"; "You may choose if your opponent loses elo" | Fled: a win for you and a draw for your opponent ([[R1843004]]); the Radiant face's choice is a draw or a loss for them ([[R1843003]]) |
| E18 | #150.1 Stinkyscape Golem | "Starpiece" | Starship Piece (ST5 D6) |
| E19 | T-AI-1 to T-AI-10 | "Create 10 AI Stinkyscape cards and always offer one of those" | ST15's ten, designed by the build (ST9 Q8): no Credits made, costs (0) to (2), no numbers declared |
