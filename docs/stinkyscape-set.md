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
