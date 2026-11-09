# Stinkyscape+: the plan of issues

2026-11-09 · written from the designer's list of the same day (`JackiOh_StinkyscapePlus_Cards.md`)

> **Status: the plan of record for the issues.** SPEC is still the only source of rules (CLAUDE.md).
> This file reads the designer's list against the engine on `main`. It names what the set reuses and
> what it must build, gives a default reading wherever the list is silent or ambiguous, and splits the
> work into the three fullsends the owner asked for:
>
> 1. **Fullsend 1:** the engine's new mechanics and every card. The set ships, and players can play it
>    against each other, while the AI deals itself none of it (R1390's soft gate).
> 2. **Fullsend 2:** the AI deals itself at least 80% of the set.
> 3. **Fullsend 3:** the AI deals itself the whole set: no Stinkyscape+ card is in `SHADOW_BAN`.
>
> Each fullsend lands on `main`, reaches `staging` and then `production`. A reading marked
> **default** is the build's own wherever the designer said nothing. The part that builds a card or
> mechanic may change a default, and records its choice as a ruling note (CLAUDE.md rule 3).
> `docs/stinkyscape-set.md` is the model this plan follows (its ST3 to ST15), and Stinkyscape's
> parts are the precedent for every system named here.

## Contents

- [SP0. What the designer asked for](#sp0-what-the-designer-asked-for)
- [SP1. The set in numbers](#sp1-the-set-in-numbers)
- [SP2. What the engine already has](#sp2-what-the-engine-already-has)
- [SP3. The new systems](#sp3-the-new-systems)
- [SP4. The patches to shipped cards](#sp4-the-patches-to-shipped-cards)
- [SP5. Fixes to the list](#sp5-fixes-to-the-list)
- [SP6. Readings, card by card](#sp6-readings-card-by-card)
- [SP7. The seven AI cards](#sp7-the-seven-ai-cards)
- [SP8. The issues](#sp8-the-issues)
- [SP9. The order and the road to production](#sp9-the-order-and-the-road-to-production)
- [SP10. What every part's body holds](#sp10-what-every-parts-body-holds)

---

## SP0. What the designer asked for

- **A new set, Stinkyscape+**: 160 numbered cards. Six numbers are used twice, so there are 166 numbered
  cards. There are also 16 tokens on sub-numbers and 4 tokens marked `#Token` (186 entries in the
  list). On top of those come the tokens the list names without defining them (SP1), and 7 AI cards
  for the Marketplace (SP7).
- **New mechanics**: Evolve and Evolves, Refresh for Activates (with the old "Refresh mana" renamed
  Replenish), Tipsy X, GIGA, the Warhero Token, Flexible, Frenzy, Evasive, Burn and Toxic. There is
  also a long tail of verbs only one or two cards use (SP3, E6 to E8). One system change reaches every
  set: random cards are unique within an effect (E9).
- **Patches to shipped cards** (SP4): Jay Fungus, Selective Breeding, Rotten Grape, Money Machine,
  xxFreshJack, Heroic Power and Great Paperclip Theory. Every unit that can't be attacked gains Evasive,
  and every "after this kills a Unit it may attack again" gains Frenzy.
- **The release**: v0.6.0. A set that reaches `SHIPPED_SETS` is a major (`docs/issues-and-patches.md`,
  Version numbers; R1420). Nothing names the version by hand: `patches ship` names it once the set ships.

## SP1. The set in numbers

| | Count | Notes |
| --- | ---: | --- |
| Numbered cards | 166 | #1 to #160, where #74, #83, #92, #93, #102 and #140 are each used twice. The second card on each becomes #161 to #166 (SP5) |
| Tokens on sub-numbers | 16 | #6.1, #6.2, #26.1, #27.1, #61.1, #69.1, #75.1, #76.1, #121.1, #133.1 to #133.5, #142.1, #142.2 |
| `#Token` cards | 4 | Tears of the Kwzki Tree, Supernova, Alnilam (Supernova's), The Enigmata: Terminus tokens, unnumbered (SP5) |
| Tokens named, not defined | 11 | the Warhero Token (a Resource, SP3 E5); the ten Terminus Starships of #80 The Last Voyage and #155 The Far Vagabond (SP6) |
| AI cards | 7 | T-AI-1 to T-AI-7: 2 Fruit, 2 Jlockeed, 2 CN, 1 KY (SP7) |
| **Catalog entries** | **≈ 204** | Stinkyscape was ≈ 228, Meditative+ 30 |

Reused tokens are not counted: Drone Interceptor, Drone Corvette, Jlarna, Tiny Asteroid (the list's "Small
Asteroid"), The Coin, Jogg's Box, the six Stinkyscape Resources, Dormant Starship, Felinor Token.

**Ids.** `SetName::StinkyscapePlus`, written `"Stinkyscape+"`, segment `stinkyscapeplus`, abbreviation
`S+`, `shadow_ban_share` `(1, 5)` as Stinkyscape's. Cards are `stinkyscapeplus-NNN`. A token a card defines
is `stinkyscapeplus-NNN-k`, and an unnumbered token is `stinkyscapeplus-t-<slug>`. Index strings are
`"6"` and `"6.1"`. Scripts live in `crates/cards/src/scripts/stinkyscape_plus/cNNN_slug.rs`. The set's row goes at
the end of `set_table!` (`crates/engine/src/wire/catalog_types.rs`) as `stage: Held`, and ships when the
release turns it to `Shipped` (R1420, R3114000). Its §8 section is the next free one after Training
Grounds, and its BUILD milestone is M14 (BUILD's must-pass tables stop at M13, CLAUDE.md rule 6).

## SP2. What the engine already has

| The list says | Already in the engine as |
| --- | --- |
| Cleave, Pierce, Lifesteal, Rush, Charge, Windfury, Ranged, Flying, Taunt, Divine Shield, Poisonous, Reborn, Armor, Indestructible, Immutable, Immune to Spells, Untributable, Can't attack, Can't defend, Can't be in Defense Position, Deft, First Strike, Trample, Brittle, Lucky, Cloaked, Crater, Animated, Flashback, Stack, Equip, Temporary, Tradable | §6.1 keywords |
| Cry, Death, Start/End of turn, Activate / Activate X, Aura, Echo, Cast on draw, Quickdraw, Combo, Float reduces cost, Tribute X | §6.2 triggers |
| Discover, Fuse, Embiggen, Lock, Freeze, Plague Counter, Steal, Transform, Set stats, Make Radiant, Buff and Nerf, Vanilla, Recruit, Dormant, lose mana next turn, Win the game, Snapshot (C+ #35 Rollback, R419) | §6.3 verbs |
| Starship Pieces, the Dormant Starship and launching (`STARSHIP_LAUNCH_COST`, R1852003), the weapon kit (S M05), Drone Interceptor and Corvette | Stinkyscape's M02, M05, S07 |
| Resources, Credits, the Marketplace and its AI lot (R1000, R1853000) | Stinkyscape's M03, M04, ST15 |
| Jade, Credits and other player counters | §3 player zones |
| `evolve(target)`: runs a Unit's own Evolve text when an effect evolves it (R1849001, S #91 and #92) | §6.3 Evolve; Stinkyscape+ makes it a keyword (E1) |
| Rest-of-game first-play watchers (`watch_first_plays`, R1863000, S #44 Great Paperclip Theory) | §6.3; GPT II and III reuse it |
| BUSTED and complicated cards, card metrics (S #109 xxFreshJack, R1846003, `crates/cards/src/history.rs`) | Stinkyscape's M12 |
| Older shipped catalogs a replay folds under (R767000) and `patches.json` | the past versions True Conservative reads |
| Terminus (7 cards), Fruit, Jelly Bean, Jlockeed, Acclaimed, Catalyst, Felinor, Human, CN, KY, AI, Weapon, Swarm, Quickdraw, Call to Chaos tags | the catalog |

## SP3. The new systems

Each system is built in fullsend 1 by one engine part (E1 to E8, SP8) and proved on fixture scripts
before any card uses it. The text quoted is the designer's. Defaults follow.

### E1. Evolve and Evolves

> **Evolve:** An additional effect/stat change that happens when it is **Evolved**. **Evolves:** A
> condition that causes a unit to **Evolve**. You can only **Evolve** once.

- A permanent's `Evolve:` text runs once, the first time it evolves. Later evolves of that permanent do
  nothing (the designer). `evolve(target)` (R1849001) stays the verb, and an `evolved` flag on the
  instance holds the "once". **Default:** S #91 and #92 (The Propagation's Wasp and Hornet) keep their
  printed text and evolve once too, since the keyword now says once. Their copies are new instances
  and may evolve.
- `Evolves when …` is a condition the engine checks. The conditions the set names: your hand holds at
  least N cards that cost (0); all your unit zones are full; this falls below half health; this is
  discarded to Temporary; you launch a Starship; this is Stacked on something; your opponent plays a
  Unit; this moves from Defense to Attack Position; this survives damage; a friendly Unit dies; this
  destroys a Unit; Start of Turn. **Default:** a condition is checked at the state check after the
  event that can make it true, like every "when" trigger (§10).
- "Evolve all cards in your deck" (Selective Breeding) marks each card in the deck as evolved, so it
  enters play already evolved. Its Evolve text then runs as it enters. "Summon a Swarm card that is
  Evolved" (Swarm Golem) and "Add 3 random Evolved Swarm cards" (Call to Chaos) use the same mark.
- "Your evolutions trigger twice / thrice" (#3 High Evolutionary's Aura): each Evolve text of yours
  runs 2 or 3 times while the Aura is up. The once-only rule counts evolving, not the repeats.
- "For every card that Evolved this game" (#163 Accelerationist) is a per-player game counter.
- The view shows an Evolved mark on the card. The client shows a glow and a badge (E1 owns the
  client's mark; S26 polishes it).

### E2. Burn and Toxic

> **Burns [int]**: Burned units/heroes take [int] *unpierced* damage at the start of their owner's turn,
> then the burn is decremented by 3 (burn 8 → burn 5 → burn 2 → no burn). Can also get *Damage dealt
> Burns*, which converts all damage to **Burn**. **Toxic [int]**: Toxic units/heroes take [int] *pierce*
> damage at the start of their owner's turn in perpetuity. Can also get *Damage dealt is Toxic*, which
> converts all damage to **Toxic**.

- Both are counters on a Unit or a hero, shown on the card and the hero. **Default:** a new Burn or Toxic
  adds to the counter already there. Burn ticks first, then Toxic, at the start of the owner's turn,
  before the draw. Burn's tick is unpierced, so Armor and Divine Shield take it; Toxic's tick pierces.
- "Damage also Burns N / applies Toxic N" (Mace, Reign, Zenith, Antumbra, Eclipse, the Rounds) adds N
  after each damage instance this deals to a Unit or hero that survives it.
- "Damage dealt Burns / is Toxic" (Volcanic Rounds, Noxious Rounds) replaces the damage: the target gains
  Burn or Toxic equal to the damage it would have taken, and takes none now.
- "Burn N an enemy unit" (Fire Beam) and "Toxic N an enemy unit / all enemies" (Scourge Mortar, Toxopid,
  Call to Chaos) are verbs that add N.
- Burn and Toxic are not keywords that Nerf or Vanilla remove. **Default:** Vanilla and Silence-like
  effects clear both counters, and a Nerf does not.

### E3. Flexible, Evasive and Frenzy

- **Flexible**: "Can be played in the unit row or the back row." The player picks the row as they play it.
  **The owner's call (2026-11-09):** a Unit in the backrow **can attack and acts as a normal Unit**. It
  works as Flying's backrow placement already does: a Flying Unit may be played into an empty backrow
  zone and is a Unit there for every rule (R1081000, R2248000). **Flying Units innately have Flexible.**
  Flexible is that placement made a keyword of its own, which any card may have, and Flying now grants
  it rather than carrying the rule itself (same behavior for Flying Units, so no shipped Flying card
  plays differently). **Default:** a Flexible Field Spell may be played into a unit zone and keeps
  working there as a Field Spell; one with printed stats (#59 Mass Driver) is also a Unit there, which
  can be attacked and can attack. The part writes the ruling.
- **Evasive**: "Same thing as Can't be attacked … Units with Evasive implicitly have Can't Defend …
  A nerf can remove Evasive." It is a keyword. Every shipped card that says "Can't be attacked" gets
  Evasive instead (a catalog migration plus a fragment, since those cards gain Can't Defend).
  **Default:** a card that already has both shows Evasive alone.
- **Frenzy [effect]**: "triggers the effect after the unit attacks and destroys a unit." "Frenzy: this
  may attack again" means "After this kills a unit it may attack again". Every shipped card with that
  effect gains the keyword. A Nerf can remove Frenzy, and the random-keyword pools gain "Frenzy: this
  may attack again" as a combat keyword.

### E4. GIGA

> **GIGA** applies to permanents. It makes them 2x2 (visually and actually), taking up two backrow and
> two unit lanes. It can be applied to traps, field spells and units. It is technically part of both
> lanes … twice as susceptible to cleave … supposed to be BIG.

- A GIGA permanent takes two adjacent lanes in both the unit row and the backrow, four zones. The player
  picks its left lane as they play it. **Default:** it can be played or summoned only where all four zones
  are free (or are made free by its own placement rules, as Crater does). When it can't fit, a summon
  fails and a play is not offered.
- It is in both lanes for every rule that reads a lane: attacks across, Cleave, lane effects, "this lane
  and adjacent", the Black Hole and the slashed board. An effect that hits each lane once hits it once
  per lane. A Unit's own attack is made once.
- Making a card GIGA is a mark that carries from the hand onto the field ("Make a permanent in your
  hand GIGA", "Draw … and make them GIGA"). "Embiggen: GIGA" means the card becomes GIGA when embiggened.
- The client draws it across the four zones (E4 owns the board's layout for it).

### E5. Tipsy, the Warhero Token, Refresh for Activates and infinite mana

- **Tipsy X**: a player counter like Jade or Credits, shown as a stein. Playing a card with Tipsy X adds X
  to it. It decays by 1 at the start of its owner's turn. While it is above 0, a card the player newly
  plays, summons or draws may have one number, keyword or word of its name blurred, with a chance that
  grows with the counter. At 0 every blur clears. Going from 2 to 1 clears nothing; it only lowers the
  odds for new cards (the designer). **Default:** only the Tipsy player's own view blurs. Which word blurs
  is decided by the engine in `view_for` from a hash of the game's seed and the card's instance, so it
  draws nothing from the game's `Rng` and a reload or a replay shows the same blur. The client renders
  the blur and never changes a rule (CLAUDE.md rule 7). The chance per card is `TIPSY_BLUR_CHANCE_PER_STACK`,
  capped by `TIPSY_BLUR_CHANCE_CAP` (config.rs).
- **Warhero Token** (a Stinkyscape Resource, Rare, (0), "Yeah, I'm a war hero."): every Warhero Token is
  worth one game-wide value in Credits. It starts at 0. Once any Warhero Token has been printed in the
  game, the value grows by a random 0 to 10 at the end of every turn (`WARHERO_GROWTH_MIN`/`MAX`). A
  Radiant one is worth double. "They do nothing when played other than give credits": playing one gives
  its value in Credits, and selling one at the Marketplace quotes the same value. Money Machine, #29
  Grinder Veteran and #30 Yeah I'm a War Hero print them.
- **Refresh** (for Activates): "Gives you back your non-Refresh giving activates again … if you Refresh
  an Activate 4 you would get another 4 Activates." **Default:** Refresh resets this turn's uses of every
  Activate on your permanents, except Activates whose effect is a Refresh. #11 Energy Booster grants
  it when equipped.
- **Replenish**: every existing "Refresh N mana" / "Refresh your mana" wording becomes "Replenish …",
  with no change in behavior (catalog text, card-text.yaml, the spec's §2.3 and §6.3 terms, the
  glossary). Call to Chaos's "Refresh 2 mana" / "Refresh 4 mana" read as Replenish.
- **Infinite mana** (#56 Impact Reactor, "When awoken: Gain infinite mana"). **Default:** while the
  awakened Reactor is on its controller's field, that player's mana is not spent by paying costs, and the
  view shows ∞. It ends when the Reactor leaves the field.

### E6. Combat and reaction verbs

Each verb is small, and only a card or two use it, so they share one part.

- **Attack an enemy as it is summoned** (#72 Foreshadow, #73 Ripple): when an enemy Unit is summoned,
  this attacks it at once, even if it can't attack otherwise (Foreshadow's base face can't attack and
  still does). Foreshadow also queues "lose 1 mana next turn" each time. **Default:** the attack exerts
  nothing, and needs no target choice.
- **N attacks a turn, only across** (#37 Scorch: "Can attack 10 times per turn. Can only attack the unit
  directly across from this").
- **Attack riders**: "When this is Frozen, fully heal it" (#90, a new trigger) and "Freeze any Unit damaged
  by this" (#91). "After this attacks, Freeze it" (#27) and "When this attacks, gain +2 attack and take 2
  damage" (#90) are the existing `after_attack` hook (R1849001) and need nothing new.
- **Activate cooldowns** (#125 Omura: "has a three turn cooldown, e.g., normal activates have a 1 turn
  cooldown"). This is an Activate usable once, then not again for N of its controller's turns.
- **A per-turn damage pool** (#42 Force Projector, #123 Oct): "The first 5 damage allies would take per turn
  is nullified {5 left}". The pool refills at the start of its controller's turn, and the view shows what
  is left. **Default:** it applies after Armor ("accounts for armor"), to Units and the hero.
- **Counter a Ranged attack or an Activate** (#43 Segment: "The first enemy Ranged attack or Activate per
  turn is countered … they deal no damage, doesn't go through"; the Radiant face counters all of them).
  The Activate's use is spent.
- **A kill deadline** (#94 PIP): the target must destroy a Unit before the end of its controller's next
  turn, or it is destroyed. The Radiant face adds 3 The Coin when it fails.
- **A board snapshot and restore** (#84 Bogos Binted): a Cry stores your other permanents (card, stats,
  keywords, zone). Its Death clears your board, then rebuilds it from the snapshot (Radiant: made
  Radiant). The snapshot of C+ #35 Rollback (R419) is the model.
- **A secret pick from the opponent's hand** (#31 Soul Read): the trap's Cry shows its owner the
  opponent's hand (only to that player's view, CLAUDE.md rule 7) and they secretly choose one card. The
  trap reveals when the opponent plays the chosen card before any other card seen (Radiant: when they
  play it at all), and its owner takes control of that card instead. Otherwise the trap is tributed.
- **Take control** (#25 Snom Bunny's Death: "A random enemy Permanent comes under your control";
  Soul Read). This reuses the existing Steal and Mind Control.

### E7. Board geometry and damage routing

- **The Black Hole** (#77): it occupies the middle lane (lane 3 of 5) on both sides with a lasting
  animation. Any permanent that enters it is exiled. At the start of each player's turn it pulls that
  player's permanents one lane toward it, and those that reach it are exiled. It closes after swallowing
  10 permanents. The Radiant face pulls only the opponent's permanents, at the start of the opponent's turn.
- **The slashed board** (#78): the middle lane is exiled with every permanent in it, and the board becomes
  two halves for the rest of the game. A Unit or Field Spell affects only its own half. A Spell is played
  on one half, chosen as it is cast. The hand and the heroes' Health stay shared. The Radiant face picks
  the slash at lane 2, 3 or 4.
- **Line damage** (#109 Lancer, #126 Vela, #127 Corvus): choose (Lancer: Discover from three) an enemy tile,
  a row and a lane, and deal damage to every Unit on the line between this and that tile, as KY's papaya
  draws it (Pierce; Vela's and Corvus's Radiant faces add Cleave). The client draws the line, as a laser
  for Corvus.
- **Chain lightning** (#38 Arc): deal 2 damage to a Unit. It then jumps at random to an adjacent Unit (the
  next lane or the backrow behind it), never the same Unit twice in one Activate, until no Unit is left to
  jump to.
- **Damage chains** (#7 Churnwalker): chained enemies all take any damage one of them takes. Chained
  allies split any damage one of them takes among all of them, rounded down. **Default:** a chain lasts
  for the rest of the game, and only the Units on the field when the Cry resolves are in it.
- **Lane-weighted damage** (#107 Fuse): "Deal 5x damage to units in this lane, deals 3x damage to units in
  adjacent lanes". **Default:** its attack hits every enemy Unit in its lane for 5 times its attack and
  every enemy Unit in the adjacent lanes for 3 times, as one damage instance each.
- **Summon onto an occupied zone** (#121 Toxopid's Legs: "Summon 3 Toxopid Legs ANYWHERE on the field,
  exiling the card below it, should trigger leg's crater").

### E8. Hand, deck and fuse verbs

- **Hand position**: the leftmost card, the rightmost card, the card to the right or left of this
  (#69.1 Sweet Essence), and adding a card at either end of the hand (#98 Doomed Ship). "When they meet"
  (Doomed Ship) means the moment the two cards become adjacent in the hand. The hand's order is already
  state, and the player may not reorder it.
- **A merged fused Spell** (#17 Smoothie, #18 Large Smoothie): "when added to your hand, select two
  (three) random fruits to be added to the smoothie, their targets are merged together but essentially
  works like a fused card". The Fruit pool is every Fruit Spell in the shipped sets, excluding the
  Smoothies.
- **Fuse until a cost** (#5 Dingleberry Droat: "Fuse Fruits together until you make one that costs at
  least (5). Add it to your hand. It costs (2)"), and **fuse a different card of a kind into this at End
  of Turn while it is in hand** (#6.1 Fine Fermented Grape: "End of Turn: Fuse a different Grape into
  this"; a Grape is a card whose name holds "Grape").
- **Deck placement**: "Always starts on the bottom of your deck (if multiple cards have this, order is
  random)" and "When your exile has 5 cards cast this (wherever in your deck this is)" (#89 Animosity).
  **The owner's call (2026-11-09):** after the opening shuffle, every card with this keyword goes to the
  bottom of the deck, and when several have it they are shuffled among themselves there, in random order
  (from the game's `Rng`), below every other card.
- **Cast repeatedly** (#165 Shadiris: "Cast Shadiris Visions twice").

### E9. Random cards are unique

**The owner's call (2026-11-09), a system change:** every effect that generates random cards (adds,
summons, Discovers, fills, shuffles, casts) draws them **unique**: no card twice within the one effect,
unless the pool has too few cards left, and only then may it repeat. No card needs to say "unique" any
more: #96 Open Enrollment and #97 Selective Enrollment lose the word (SP5). The engine's random-card
pools (`catalog`'s pools, `with_pool_hold`, R2201001) draw without replacement within one effect, from the
game's `Rng`. The change reaches every shipped card that makes random cards, so the release's fragment
claims it as a rule (`--rule`), the golden traces are re-blessed and the seed-pinned tests re-pinned.

## SP4. The patches to shipped cards

All go live with the set, in fullsend 1's one pull request, claimed by its fragment.

| Card | Change |
| --- | --- |
| Jay Fungus | Gains the Swarm tag. Base gains "**Evolve**: Poisonous. **Evolves** when your hand has at least (3) cards that cost (0)". Radiant gains the same at at least (1) card (**default**: the list's second "Normal gains" line is the Radiant face) |
| Selective Breeding | Gains "**Evolve** all cards in your deck" (E1) |
| Rotten Grape | Gains **Tipsy 1** (E5) |
| Money Machine | New text: "**Cry**: Print either **Coins** or **Warhero Tokens**. **End of Turn**: Shuffle the printed choice into both decks. Flip yours. If heads, put it in your hand instead." The Radiant face gains **Lucky 1** and prints Radiant versions. When the Cry doesn't go off (the AI, the timeout), it defaults to The Coin |
| xxFreshJack | BUSTED cards include cards from past patches too, judged by winrate, through one general function that needs no upkeep each patch (S17) |
| Heroic Power | Every existing power costs (1) less. New power **(10) End Times**: "Discover a Terminus card. End of Turn: Reduce the cost of the next use of this by (1)" (Radiant: by (2)) |
| Great Paperclip Theory | Renamed **Great Paperclip Theory I**, and gains the Paperclip tag |
| Every "Can't be attacked" Unit | Gains Evasive (E3) |
| Every "after this kills a Unit it may attack again" | Gains Frenzy (E3) |
| Every "Refresh N mana" | Reads "Replenish N mana" (E5; wording only) |

## SP5. Fixes to the list

- **`{For AI: …}`** notes in the list are the designer's notes to the builder (the agent building the card),
  not rules for the game's AI. "{For AI: Voiceline is …}" is a voice line, and "{For AI: look in the patch
  notes …}" says where the data lives. Each one is binding on the card's part.
- **Duplicate numbers.** The second card on each reused number moves to the end: #161 Space Invader (the
  second #74), #162 Edwin Van'Spacechief (the second #83), #163 Accelerationist (the second #92), #164 KPIs
  (the second #93), #165 Shadiris (the second #102), #166 Mindustrial Revolution (the second #140).
- **The `#Token` cards** are unnumbered tokens (`stinkyscapeplus-t-<slug>`): Tears of the Kwzki Tree,
  Supernova, Alnilam and The Enigmata. They are Terminus cards that random-Terminus effects can make, and
  they are not collectible. Alnilam is a Unit: it has stats, though the list says Spell.
- **#70 Spaceberry** says "Stinkyscape". It is Stinkyscape+.
- **#28 Zombie-Box** lists "Rare, Common". **Default:** Rare, the first.
- **#22 Cryofluid** lists Mindustrial twice. It gets one tag.
- **#104 Tireless Cultivation** has no type. It is a Spell.
- **#4 Profe Yates**'s Radiant "hen" means "then".
- **#18 Large Smoothie**'s Radiant "Radiant X, Radiant X, & Radiant X" means X, Y and Z.
- **#50 Graphite**'s "Piece" means Pierce.
- **#53 Laser Drill**'s Radiant face lists GIGA twice. It is GIGA once.
- **#69.1** is "Sweet Essence" (the list spells it "Essense").
- **#133.5 Reignclipse**: "Regin" means Reign. **Tribute 3**, not the list's 4 (the owner, 2026-11-09): each
  Automa-ζ fusion takes its two cards' shared Tribute (Fortnith 1, Sceptumbra 2, Reignclipse 3, as Reign and
  Eclipse both have Tribute 3).
- **#142 Call to Chaos (Mindustrial-Swarm Edition)**: "Jlockheed" means Jlockeed, and "Refresh N mana"
  reads Replenish (E5).
- **#142.1, #142.2**: "Stinkscape+" means Stinkyscape+. #102b Shadiris's "Stinkscape Resources" means the
  Stinkyscape Resources.
- **"Small Asteroids"** (#76 Ayys LMAO) are Stinkyscape's Tiny Asteroid (`stinkyscape` #169).
- **"Kwzki"** (#98 Doomed Ship) is iKwzki (S #110), the one beside Shockwave447 (S #111) and xxFreshJack
  (S #109).
- **#7 Churnwalker**'s Radiant "Cry:" lost its "Choose one". It is the same choice.
- **#96 Open Enrollment, #97 Selective Enrollment** lose "unique" from their text: every random card is
  unique now (E9, the owner's call of 2026-11-09).

## SP6. Readings, card by card

These are the defaults where the list is silent. Each card's part may change one, and records it as a
ruling note.

- **#4 Profe Yates**: the exile hits every card in both players' hands, decks, graveyards and fields whose
  name holds any of M, O, U, S, E, case-insensitive, except CN cards and Yates itself. "Summon two random
  traps" summons them face down into its owner's backrow. Then its owner chooses one of the three
  Activates, which Yates keeps. Iron Mandate fuses Yates into a friendly Unit without running its Cry
  again. The voice lines go to S26.
- **#6 Wines McCheese**: the three are equally likely.
- **#6.2 Stinky Cheese**: Temporary 2 means it is discarded at the end of your second turn holding it.
  Evolving there gives it Flashback, Animated and +5/+5, so it can be flashed back as a Unit.
- **#20 B-Patch**: one 50/50 roll for the whole effect, not one per card. A card that can't be Buffed or
  Nerfed is skipped.
- **#24 Forced Propagation**: the copies are fresh copies of the card (no cost changes, no buffs). The
  Radiant face then Buffs each copy once.
- **#30 Yeah I'm a War Hero**: only the first card you draw each turn, by any draw, is replaced.
- **#33 True Conservative**: "a version from their Past" means the card's entry in an older shipped
  catalog (R767000, `patches.json`). A card with no older entry is untouched. A Unit with several past
  versions takes a random one; on the Radiant face, your own take the strongest (**default**: the
  highest stat total, then the highest cost).
- **#34 Neoconservative**: a Jlockeed card from a past version, taken from the older catalogs as #33 does.
- **#56 Impact Reactor**: the Cry's mana loss stacks with other "lose mana next turn" effects. For
  infinite mana, see E5.
- **#61.1 Shawarma Law**: each Unit counts the Shawarmas it has eaten. On the Radiant face, your Units stop
  at one.
- **#70 Spaceberry**: "Shuffle 3 Cast on draw into your deck" shuffles three random cards with Cast on draw
  from the shipped sets, never Spaceberry itself, so it can't chain forever.
- **#75.1 J-lease Pack**: "Stinkyscape or Stinkyscape+ cards" draws from both sets' collectible cards.
- **#79 A Glimpse into the Endless Sea**: the countdown is a player counter, ticking down at the start of
  its owner's turn. At 0 its owner wins. Bob's Plan (the AI's scripted plan) gets a line for it in S22.
- **#80 The Last Voyage** and **#155 The Far Vagabond**: there are ten Terminus Starships, each with a name
  and art of its own. Each is a Dormant Starship already holding Starship Pieces worth 5 to 12 mana. They
  can't be launched by hand; The Far Vagabond's can be, for (3) (Radiant (0)). The Last Voyage launches a
  random one now and another at the start of each of your turns.
- **#82 Endless Expedition**: "lose 1 mana" lowers the max mana by 1 for the rest of the game.
- **#93 KPIs** (#164): "a lower winrate than this" reads the card metrics' winrate of record (M12). A card
  with no recorded winrate counts as 50% (the owner, 2026-11-09), and so does KPIs itself when it has none.
- **#95, #96, #97** (the tiers): S, A, D and F are the card metrics' tiers of record. The cards are unique
  by E9's rule, which every random card follows.
- **#98 Doomed Ship**: one of the three pairs at random. The first card goes leftmost and the second
  rightmost.
- **#121 Toxopid**: "Float reduces mana" is the existing "Float reduces cost". "Friendly Crawlers dying
  reduces mana cost by (1)" is per Crawler, for the rest of the game, while the Toxopid is in your hand
  or deck.
- **#133 Great Paperclip Theory III**: the Automa-ζ ladder is #133.1 to #133.5 by cost.
- **#137 The Perfect Pair**: the pairs are the list's (Scepter & Reign, Antumbra & Eclipse, Quad & Oct,
  Sei & Omura, Vela & Corvus).
- **#138, #139** "embiggen they have Tribute 1 (2)": once embiggened, the added cards have that Tribute.
  Before that they keep their own.
- **#142 Call to Chaos**: "Cast 5 Jlockeed cards" draws five random Jlockeed cards, Units included (the
  owner, 2026-11-09): each Spell is cast with random targets and each Unit (or other permanent) is
  summoned. Its text reads "Summon or cast 5 random Jlockeed cards". The Radiant "!!!" does three distinct
  effects.
- **#157 Phantom Shipyard**: Dormant until launched. Its Activate 3 exiles it and launches a Starship
  holding every piece it fused in.
- **The Enigmata**: from then on, at the start of each player's turn, that player casts a Jogg's Box with
  random targets. The game otherwise goes on. (An automatic loop of casts until someone dies could never
  end within the match's action cap.)
- **Alnilam**: when it dies, its controller loses the game. It is Indestructible, so in practice only
  Brittle's count, an exile or a transform removes it.

## SP7. The seven AI cards

The designer: "Create 2 New AI cards that are also fruits, 2 that Jlockeed, 2 that are CN, 1 that is KY
(should have tags)." They are T-AI-1 to T-AI-7 of Stinkyscape+, made the way ST15 made Stinkyscape's ten:
tokens tagged AI and Token, plus Fruit, Jlockeed, CN or KY; `rarity: "Token"`; costing (0) to (2); making
no Credits (ST15's constraint 2); one Stinkyscape+ system each. Each Radiant face doubles a number or adds
a rider (R275). The foundation designs them and writes their table here. S25 scripts them and adds them to
the Marketplace's AI lot.

## SP8. The issues

One tracker, `Patch: the Stinkyscape+ set` (labels `tracking`, `major version`, `patch`), holds the three
fullsends as sub-issues, along with the road to production (SP9). Each fullsend is built by the night bot
in its fullsend mode (`bot/README.md`, Fullsend):

- **Fullsend 1** is split by hand. Its parts below are its sub-issues. **The owner's change of 2026-11-09:**
  F and E1–E9 are built in a Claude session by parallel agents and land on `main` first, in one pull
  request with the set still `Held`. In that pull request, E3's shipped-card migration (Evasive, Frenzy) and
  E5's Replenish rewording move to S01, and E9's unique draws ship behind `UNIQUE_RANDOM_DRAWS = false`,
  which R turns on, so no shipped card plays differently before the release. Then the parent is labelled
  `bot:fullsend`, and the bot adopts the open parts (S01–S26 and R, `queue.Adopt`). They land on
  `bot/issue-<fullsend 1>`, cut from a `main` that already holds the foundation and the mechanics, and the
  reconcile opens one pull request into `main`.
- **Fullsends 2 and 3** have no parts opened by hand. Labelled `bot:fullsend`, each waits on its `Blocked by`
  line, and the bot's split cuts it into parts once the fullsend before it has merged. Which cards the AI
  fails is only known then.

No part is titled `(part n of m)`, because the night bot builds those strictly in order. Each part waits
only for the issues its `Blocked by` line names. Difficulty follows CLAUDE.md rule 11.

**Opened on 2026-11-09:** the tracker #3533; fullsend 1 #3534 with its parts #3535 to #3573; fullsend 2 #3574;
fullsend 3 #3575.

| Id | Issue | Id | Issue | Id | Issue | Id | Issue |
| --- | --- | --- | --- | --- | --- | --- | --- |
| F | #3535 | E6 | #3550 | S05 | #3544 | S16 | #3553 |
| E1 | #3536 | E7 | #3554 | S06 | #3537 | S17 | #3538 |
| E2 | #3540 | E8 | #3558 | S07 | #3559 | S18 | #3557 |
| E3 | #3543 | E9 | #3563 | S08 | #3555 | S19 | #3567 |
| E4 | #3546 | S01 | #3551 | S09 | #3547 | S20 | #3568 |
| E5 | #3548 | S02 | #3541 | S10 | #3556 | S21 | #3561 |
| R | #3573 | S03 | #3542 | S11 | #3549 | S22 | #3569 |
| S26 | #3572 | S04 | #3552 | S12 | #3545 | S23 | #3539 |
| S24 | #3570 | S25 | #3571 | S13 | #3560 | S14 | #3565 |
| S15 | #3566 | | | | | | |

### Fullsend 1: the mechanics, every card and the release

| Id | Title (`Patch: Stinkyscape+ …`) | Cards and scope | Blocked by | Difficulty |
| --- | --- | --- | --- | --- |
| F | `F, the foundation` | the set row (`Held`), the new keyword kinds and tags declared, every catalog entry with both faces, a stub script per card, the §8 rows, BUILD M14, the seven AI cards designed (SP7), the server's migration for the new tags, this plan and the designer's list on the branch | — | hard |
| E1 | `E1, Evolve and Evolves` | SP3 E1 | F | hard |
| E2 | `E2, Burn and Toxic` | SP3 E2 | E1 | hard |
| E3 | `E3, Flexible, Evasive and Frenzy` | SP3 E3, with the shipped-card migration | E2 | hard |
| E4 | `E4, GIGA` | SP3 E4 | E3 | hard |
| E5 | `E5, Tipsy, the Warhero Token, Refresh for Activates and Replenish` | SP3 E5 | E4 | hard |
| E6 | `E6, combat and reaction verbs` | SP3 E6 | E5 | hard |
| E7 | `E7, board geometry and damage routing` | SP3 E7 | E6 | hard |
| E8 | `E8, hand, deck and fuse verbs` | SP3 E8 | E7 | hard |
| E9 | `E9, random cards are unique` | SP3 E9 (the owner's system change) | E8 | hard |
| S01 | `S01, the patches to shipped cards` | SP4: Jay Fungus, Selective Breeding, Rotten Grape, Money Machine, Heroic Power, Great Paperclip Theory I | E1, E5 | hard |
| S02 | `S02, the Swarm evolvers` | #2, #27, #27.1, #28, #83, #86, #88, #92, #101, #161 | E1 | medium |
| S03 | `S03, Swarm rule benders` | #3, #75, #75.1, #87, #93, #163 | E1 | hard |
| S04 | `S04, Starship Pieces on existing keywords` | #11–#16, #19, #154, #156 | E5 | medium |
| S05 | `S05, the Burn and Toxic arsenal` | #143–#153 | E2 | medium |
| S06 | `S06, the weapon rack` | #8, #9, #10, #60, #159, #160 | F | medium |
| S07 | `S07, the Mindustrial turrets` | #35, #36, #37, #38, #90, #91, #107, #108, #109 | E3, E6, E7 | hard |
| S08 | `S08, the GIGA cards` | #23, #64–#67, #72, #73, #99, #100, #106 | E4, E6 | hard |
| S09 | `S09, the Minerals` | #22, #44–#50 | E3 | medium |
| S10 | `S10, the Mindustrial field spells` | #40–#43, #55, #56, #57, #59, #62, #63 | E4, E6, S09 | hard |
| S11 | `S11, the drills and Mindustrial suppliers` | #39, #52, #53, #54, #58, #71, #166 | E4, S09 | medium |
| S12 | `S12, Automa α and β` | #110–#119 | E2 | medium |
| S13 | `S13, Automa γ, δ and Ω` | #120, #121, #121.1, #122–#127 | E6, E7 | hard |
| S14 | `S14, the Automa makers` | #128–#133, #133.1–#133.5, #134–#140 | S12, S13 | hard |
| S15 | `S15, fruits, grapes and food` | #1, #5, #6, #6.1, #6.2, #17, #18, #61, #61.1, #69, #69.1, #70 | E8 | hard |
| S16 | `S16, Jlockeed and the war heroes` | #21, #26, #26.1, #29, #30, #141 | E5 | medium |
| S17 | `S17, outside data: winrates, tiers and the past` | #33, #34, #95, #96, #97, #164, and xxFreshJack's BUSTED | F | hard |
| S18 | `S18, spells on existing primitives` | #20, #24, #32, #51, #68, #76, #76.1, #85, #94, #103, #104, #105, #158 | E6 | medium |
| S19 | `S19, the legends and rule benders` | #4, #7, #25, #31, #84, #98 | E8 | hard |
| S20 | `S20, units on existing primitives` | #74, #102, #162, #165 | E8, S18 | medium |
| S21 | `S21, Terminus: the board` | #77, #78 | E7 | hard |
| S22 | `S22, Terminus: endings and the rest of the game` | Tears of the Kwzki Tree, Supernova, Alnilam, The Enigmata, #79, #81, #82, #89 | E8 | hard |
| S23 | `S23, the Terminus Starships` | #80, #155, #157, the ten Terminus Starships | F | hard |
| S24 | `S24, Call to Chaos (Mindustrial-Swarm Edition)` | #142, #142.1, #142.2 | S01, S02, S03, S08, S13, S16, S21, S22, S23 | hard |
| S25 | `S25, the seven AI cards` | T-AI-1 to T-AI-7 | S15, S16 | hard |
| S26 | `S26, the set's look and sound` | all: set mark, chips, `FILTER_TAGS`, the `RELEASES` line, keyword icons (stein, Burn, Toxic, Evolved, GIGA), intro music for every Legendary and Mythic, the voice lines, every face read in the browser | every other S part | hard |
| R | `R, the release` | the set ships, the AI dealing itself none of it | every other part (E9 included) | hard |

The engine parts form one chain (F → E1 → … → E9). Each one touches the engine's shared surfaces
(`state.rs`, `effects/mod.rs`, `script.rs`, `wire/events.rs`, the keyword tables), and two of them landing
side by side would conflict on the integration branch, where a parked engine part would leave every card
part after it without its verbs. The card parts touch their own script files. Each may edit only its own
cards' catalog entries and §8 rows, and a collision between two card parts is the reconcile's to resolve.

Every one of the 186 entries is in exactly one row. Every card a card names (a pool a test needs, a token
it makes) is in a part its part is blocked by, or in its own part.

**Fullsend 1 is done when** (its reconcile, after R): every part's cards play by their text and these
readings. Stinkyscape+ is in `SHIPPED_SETS` (its row `Shipped`) and in `AI_DECK_GATED_SETS`. Every new
prompt and card is answered by the AI, the random policy and the timeout. One pending fragment claims
every entry and every shipped card that plays differently. The server's migration admits the new tags
and seeds the set. Deck and trio codes take it (R1410). The golden traces are re-blessed once, and the
seed-pinned tests are re-pinned for the bigger pools. The voice files are rendered (`[gen:voice]`). Every
stated total moves. `cargo jackioh fuzz --seeds 200`, `catalog check`, `patches check`, `spec check`,
`classify --base origin/main --claims` and the AI's smoke gate are green. REVIEW.md's audit runs as a
separate session (CLAUDE.md rule 8). The pull request into `main` merges with `rust`, `web`, `e2e smoke`,
`db` and `bot selftest` green.

### Fullsend 2: the AI deals itself 80% of the set

`Patch: Stinkyscape+ fullsend 2: the AI deals itself 80% of the set`, blocked by fullsend 1 and split by the
bot. **Done when**: a sweep of record over the set's 166 numbered cards (`cargo jackioh sweep`, R186, R390;
`.github/workflows/sweep.yml` runs one once the set ships, R1678000) bans at most 33 of them (20%, the
`shadow_ban_share` of `(1, 5)`). `cargo jackioh sweep --write` writes those bans from the report of record.
`AI_DECK_GATED_SETS` is empty again, so the AI deals itself every Stinkyscape+ card the ban does not hold.
The AI's gates are green (`cargo jackioh gate`, and `--full` on the daily run). The pull request merges
into `main` with every required check green.

### Fullsend 3: the AI deals itself the whole set

`Patch: Stinkyscape+ fullsend 3: the AI deals itself the whole set`, blocked by fullsend 2 and split by the
bot. **Done when**: a sweep of record bans none of the 166, so `SHADOW_BAN` holds no Stinkyscape+ card.
No Stinkyscape+ card is in `SHADOW_BAN` or any other AI exclusion (no shadow-banned cards). The AI's gates
are green, and the pull request merges into `main` with every required check green.

## SP9. The order and the road to production

1. **Fullsend 1**, asked once its parts are open: F; then E1 and S06, S17 and S23 (which need only F);
   then each E part in its turn, and each S part as soon as the E parts it names have landed; then S14,
   S20, S24 and S25; then S26; then R; then the reconcile and its pull request into `main`.
2. **Staging**: within two hours of fullsend 1's merge, `promote-staging.yml` moves `staging` to `main`'s
   newest green commit, and `deploy-staging.yml` deploys it. Check that staging.jackioh.com's
   `/api/catalog` lists Stinkyscape+ and the catalog version the release named.
3. **Fullsend 2** starts by itself once fullsend 1's issue closes (its pull request merged). It reaches
   staging the same way.
4. **Fullsend 3** starts once fullsend 2's issue closes, and reaches staging the same way.
5. **Production**: `promote-production.yml` promotes the commit of `staging` that the daily super run
   passed, once a day, or sooner on `/promote` on the open `production merge` issue
   (`docs/deploy-cloudflare.md` §2.1). A fullsend that reaches staging reaches production with the next
   daily promotion that passes. The tracker closes once fullsend 3's merge commit is live on
   jackioh.com (`/api/catalog` there) with every required check and the super run green.

A red check is fixed, never skipped (CLAUDE.md rule 12). A promotion that fails opens its own issue
for the night bot (`docs/deploy-cloudflare.md`), and the tracker links it.

## SP10. What every part's body holds

Every part's body holds the designer's text for each of its cards, verbatim (or, for an engine part, the
designer's words for its mechanic). It also holds the readings from SP5 and SP6 that touch it, the
engine parts it builds on, its `Blocked by` line, its "Done when", and `Part of #<fullsend 1>`.

Each card part's "Done when":

- Every card's script (`crates/cards/src/scripts/stinkyscape_plus/cNNN_slug.rs`) plays its base and
  Radiant faces by its text, with tests in the same file for both faces (CLAUDE.md rule 6).
- Each decision the spec doesn't make has a ruling note in the part's own issue block, a test named
  after it, and a paragraph in `spec/pending/<issue>.md` (CLAUDE.md rule 3).
- Every new prompt is answered by the AI, the random policy and the timeout.
- `cargo test -p jackioh-cards` passes for its cards, and `cargo jackioh spec check` and `catalog check`
  pass.
- Every number is a named constant or a card `param` (CLAUDE.md rule 9).
- `docs/ADDING_CARDS.md` was read first (CLAUDE.md rule 10).

Each engine part's "Done when": the mechanic is in the spec's sections (§3, §4, §6 or §10, whichever
it changes), with its rulings, and proved on fixture scripts in `crates/engine/tests/rules*` (the
`testkit` feature). The view and the wire carry it (`docs/v0.3.0/SURFACE.md` for any boundary it adds).
The client renders what it shows, and the AI, the random policy and the timeout answer any prompt it
opens. No card reaches it until its card part, so it changes nothing for players before the release.
