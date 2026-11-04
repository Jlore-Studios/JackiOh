# Core's card animations (#2793)

Core's pass over its animation table, `core.json5` (`docs/polish/9-card-animations.md`, "A set's
pass"): one row per Core card, tokens included, in catalog order. `core.test.ts` holds every row's
name, type, entry and scale to the catalog and the table, and the table to the pass's floor.

- **Entry**: the card's entry in `core.json5`: `none: <reason>`, `bespoke <recipe>`, `setPiece <key>`,
  or each hook's `<hook> <motif>[ <palette>]`, then `aura <look> <scope>[ <palette>]`.
- **Scale**: the card's motif scale at intensity 1 (`motifScale`), from its cost and rarity alone; `—`
  for a `none`.
- **Census**: the "Suggested" cell of `cargo jackioh fx census` (#2788) for the card, run at
  `2b156276a` (200 seeded games and 19 forced ones), or `not in census` for the seven tokens it never
  played. Its families read damage, removals, counters, costs and returns only.
- **Why**: where the entry follows the census, and why it overrides it where it does.

`cast` and `cry` draw on the board as the card resolves. `death`, `trigger` and `activate` draw
from their hook's `hookResolving` marker, and a Trap's `cast` from its `trapFired` (R2797000); the
`/dev/fx` gallery plays every entry.

| Card | Name | Type | Entry | Scale | Census | Why |
| --- | --- | --- | --- | --- | --- | --- |
| `core-001` | Big D-fender | Unit | none: its Aura is standing Armor, with no moment to draw | — | none: vanilla | the census agrees: its Aura changes nothing as it plays |
| `core-002` | Speciesist | Unit | cry bolt | 1.16 | bolt | cry: the census agrees (bolt) |
| `core-003` | Right-house defender | Unit | none: vanilla | — | none: vanilla | keywords only, which the keyword beats and the attack rows draw |
| `core-004` | Gary the Gambler | Unit | cry bless Hustle/cash | 1.08 | none: no motif reads questCompleted | cry: the census leaves out the events on the card itself, its buffed, upgraded and keywordGranted, which bless reads |
| `core-005` | Stockpile | Spell | cast flight | 1.08 | bolt | cast: the census's suggestion is noise in its window (fatigue, other cards' events); its own effect is drawn, which flight reads |
| `core-006` | Mana Well | Field Spell | cast conjure; trigger mana; aura arcane zone | 1.24 | bolt | cast: a Field Spell's play is its own landing in the backrow (`summoned`), which the census leaves out; conjure draws it; trigger: no census family reads manaChanged and manaSpent, which mana (#2786) reads |
| `core-007` | Jewelosco Scarab | Unit | cry send Beast/swarm | 1.19 | none: no motif reads addedToHand | cry: no census family reads addedToHand and shuffledIn, which send (#2793) reads |
| `core-008` | Mr. Vanilla | Unit | none: vanilla | — | none: vanilla | the census agrees |
| `core-009` | Moths to the Flame | Unit | none: its start-of-turn attacks are combat, which the attack rows and keyword beats draw | — | bolt | the census's bolt is its start-of-turn combat (Pierce, Cleave), which the attack rows and keyword beats draw |
| `core-010` | Rapid Replenish | Spell | cast flight | 1.00 | none: no motif reads drawn | cast: no census family reads drawn, which flight (#2786) reads |
| `core-011` | Tempo Timmy | Unit | none: vanilla | — | none: no motif reads questProgressed | keywords only (Rush, First Strike), which the keyword beats draw; the census's questProgressed is another card's |
| `core-012` | Duplicating Felinors | Unit | cry conjure Beast/felinor | 1.28 | none: no motif reads summoned | cry: no census family reads summoned, which conjure (#2786) reads |
| `core-013` | Jlockeed Shredder-10 | Unit | trigger volley Tech/laser | 1.24 | lance | trigger: the census's lance saw one lane's hits, but its End of turn hits each enemy at once, a volley of bolts from the Shredder |
| `core-014` | Jlockeed's Weapons | Field Spell | cast conjure Tech/laser; aura storm side | 1.32 | none: vanilla | cast: a Field Spell's play is its own landing in the backrow (`summoned`), which the census leaves out; conjure draws it |
| `core-015` | Me and Mr Token | Unit | cry conjure | 1.08 | none: no motif reads summoned | cry: no census family reads summoned, which conjure (#2786) reads |
| `core-016` | Hit Job | Spell | cast bolt Physical/blade | 1.24 | bolt | cast: the census agrees (bolt) |
| `core-017` | Flood | Spell | setPiece vortex | 1.45 | vanish | kept: #2800's `vortex` set piece (R2800000); the census's vanish reads its bounces, which the set piece draws as one beat |
| `core-018` | Bread and Butter | Field Trap | cast snap | 1.35 | bolt | cast: a Trap's cast is its firing, which plans nothing until firings open a window; snap reads its trapFired then |
| `core-019` | Midrange Menace | Unit | trigger mend | 1.24 | none: vanilla | trigger: the census leaves out the events on the card itself, its healed, which mend reads |
| `core-020` | Pointmaster | Unit | none: vanilla | — | none: vanilla | the census agrees |
| `core-021` | Hinder | Spell | bespoke manaCrack | 1.00 | none: no motif reads discarded | kept: #2784's bespoke `manaCrack` (R502), which cracks the crystals the census's `discarded` window does not show |
| `core-022` | Carnivorous Cube | Unit | cry offering; death conjure | 1.55 | bolt | cry: its Cry tributes your own unit, a cost, which offering draws (the census's bolt reads that destroyed as a kill); death: no census family reads summoned, which conjure (#2786) reads |
| `core-023` | Reoccurring Dream | Spell | cast radiance | 1.19 | vanish | cast: the census's vanish is its End of turn return to hand; its play's radiantSet, which no census family reads, radiance (#2793) reads |
| `core-024` | Efficiency Dividend | Spell | cast bolt Hustle/cash | 1.55 | bolt | cast: the census agrees (bolt) |
| `core-025` | 4-mana 7/7 | Unit | none: vanilla | — | none: vanilla | the census agrees |
| `core-026` | Glowy Jelly Bean | Spell | cast radiance | 1.36 | none: no motif reads radiantSet | cast: no census family reads radiantSet, which radiance (#2793) reads |
| `core-027` | Blood Ridden Glowy Jelly Bean | Spell | bespoke bloodDrain | 1.19 | none: no motif reads healthLost | kept: #2784's bespoke `bloodDrain` (R502), which reads the healthLost the census names |
| `core-028` | Knockoff Temu Glowy Jelly Bean | Spell | cast radiance | 1.19 | none: no motif reads radiantSet | cast: no census family reads radiantSet, which radiance (#2793) reads; Holy has no sub-look, so it and #29 differ by scale alone |
| `core-029` | GIGA Glowy Jelly Bean | Spell | cast radiance | 1.63 | none: no motif reads radiantSet | cast: no census family reads radiantSet, which radiance (#2793) reads; Holy has no sub-look, so it and #28 differ by scale alone |
| `core-030` | Archivist | Unit | cry flight Nerd/ink | 1.28 | none: no motif reads drawn | cry: no census family reads drawn, which flight (#2786) reads |
| `core-031` | KY's Math Equation | Spell | cast bolt Nerd/chalk | 1.19 | bolt | cast: the census agrees (bolt) |
| `core-032` | Prem Panther | Unit | trigger flight Beast/felinor | 1.28 | bolt | trigger: the census never saw this hook fire; flight reads its drawn |
| `core-033` | Unstable Clone Machine | Field Spell | cast conjure Tech/glitch; trigger send Tech/glitch; aura storm zone Tech/glitch | 1.28 | bolt | cast: a Field Spell's play is its own landing in the backrow (`summoned`), which the census leaves out; conjure draws it; trigger: the census's suggestion is noise in its window (fatigue, other cards' events); its own effect is addedToHand and shuffledIn, which send reads |
| `core-034` | Collateral Damage | Spell | cast vanish | 1.45 | vanish | cast: the census agrees (vanish) |
| `core-035` | Lunar Eclipse | Spell | cast bolt | 1.19 | bolt | kept: #2784's example; the census agrees |
| `core-036` | Magic Jammed | Spell | cast bolt Dark/curse | 1.19 | bolt | cast: the census agrees (bolt) |
| `core-037` | Gravedigger | Unit | trigger send | 1.28 | none: vanilla | trigger: no census family reads addedToHand and shuffledIn, which send (#2793) reads |
| `core-038` | Quickstriker | Field Spell | cast conjure Physical/wind; aura ember side Physical/wind | 1.55 | none: vanilla | cast: a Field Spell's play is its own landing in the backrow (`summoned`), which the census leaves out; conjure draws it |
| `core-039` | Recycling Initiative | Spell | cast vanish; trigger send | 1.25 | none: vanilla | cast: the census leaves out the events on the card itself, its exiled and bounced, which vanish reads; trigger: the census never saw this hook fire; send reads its addedToHand and shuffledIn |
| `core-040` | Echoes of the Forgotten | Field Spell | cast conjure; trigger bolt; aura void zone | 1.28 | vanish | cast: a Field Spell's play is its own landing in the backrow (`summoned`), which the census leaves out; conjure draws it; trigger: the census's vanish is its exile of a deck card, which the board does not show, while its hero damage shows, which bolt draws |
| `core-041` | Sheepish | Trap | cast snap Beast/wool | 1.35 | bolt | cast: a Trap's cast is its firing, which plans nothing until firings open a window; snap reads its trapFired then |
| `core-042` | Selective Breeding | Spell | cast vanish | 1.16 | vanish | cast: the census agrees (vanish) |
| `core-043` | Big Felinor | Unit | cry wipe Beast/felinor | 1.45 | wipe | cry: the census agrees (wipe) |
| `core-044` | True Strike | Spell | cast bolt | 1.08 | bolt | cast: the census agrees (bolt) |
| `core-045` | Deft Duelist | Unit | none: vanilla | — | none: vanilla | keywords only, which the keyword beats and the attack rows draw |
| `core-046` | Suppressive Aura | Field Spell | cast bolt; aura void board | 1.28 | bolt | cast: the census agrees (bolt) |
| `core-047` | Fig of Life | Spell | cast mend | 1.24 | none: no motif reads healed | cast: no census family reads healed, which mend (#2793) reads |
| `core-048` | 5pek Controller | Spell | cast shift | 1.00 | none: no motif reads positionSwitched | cast: no census family reads swapped, rotated, positionSwitched and flickered, which shift (#2786) reads |
| `core-049` | Snom Bunny Mind Control | Spell | cast tether Dark/curse | 1.45 | none: no motif reads controlChanged | cast: no census family reads stolen, controlChanged and held, which tether (#2786) reads |
| `core-050` | K-Pop Fanatic | Unit | none: no motif reads marked, its Allure; the mark's own row plays | — | bolt | the census's bolt is its Allured card's later kills; no motif reads `marked`, its Allure, and the mark's own row plays |
| `core-051` | KY's Private Tutor | Spell | cast send Nerd/chalk | 1.35 | none: no motif reads promptAnswered | cast: the census reads its prompt; its Discover adds the card to the hand (addedToHand), which send (#2793) reads |
| `core-051-1` | KY's Empty Notebook | Spell | cast send Nerd/ink | 0.97 | not in census | cast: a token the census never played; its cards shuffle in (shuffledIn), which send (#2793) reads, in Nerd's ink beside its Tutor's chalk |
| `core-052` | Silly Silas | Unit | cry shift | 1.85 | vanish | cry: the census's vanish is the cards its rotation took; its Cry is a rotation (rotated), which shift reads |
| `core-053` | Reno | Unit | cry mend | 1.24 | none: no motif reads healed | cry: no census family reads healed, which mend (#2793) reads |
| `core-054` | Straaza | Unit | cry send | 1.32 | none: no motif reads addedToHand | cry: no census family reads addedToHand and shuffledIn, which send (#2793) reads |
| `core-055` | Lava Golem | Unit | cry offering | 1.36 | offering | cry: the census agrees (offering) |
| `core-056` | Jilliax | Unit | none: vanilla | — | none: vanilla | keywords only, which the keyword beats and the attack rows draw |
| `core-057` | Conjure KY | Spell | cast send | 1.16 | none: no motif reads addedToHand | cast: no census family reads addedToHand and shuffledIn, which send (#2793) reads |
| `core-058` | Rush Token Farm | Field Spell | cast conjure; trigger conjure; aura bloom zone | 1.16 | none: vanilla | cast: a Field Spell's play is its own landing in the backrow (`summoned`), which the census leaves out; conjure draws it; trigger: no census family reads summoned, which conjure (#2786) reads |
| `core-059` | Unbiased Immigration | Field Spell | cast conjure; trigger send; aura arcane zone | 1.28 | bolt | cast: a Field Spell's play is its own landing in the backrow (`summoned`), which the census leaves out; conjure draws it; trigger: the census's suggestion is noise in its window (fatigue, other cards' events); its own effect is addedToHand and shuffledIn, which send reads |
| `core-060` | Bear Honeypot | Trap | cast snap | 1.35 | vanish | cast: a Trap's cast is its firing, which plans nothing until firings open a window; snap reads its trapFired then |
| `core-061` | Prejudiced Postdoc | Unit | cry conjure Nerd/chalk | 1.28 | bolt | cry: the census's suggestion is noise in its window (fatigue, other cards' events); its own effect is summoned, which conjure reads |
| `core-062` | Friend of Felinors | Spell | cast conjure Beast/felinor | 1.08 | vanish | cast: the census's suggestion is noise in its window (fatigue, other cards' events); its own effect is summoned, which conjure reads |
| `core-063` | Plastic Surgery | Spell | cast bless | 1.08 | none: no motif reads buffed | cast: no census family reads buffed, upgraded and keywordGranted, which bless (#2786) reads |
| `core-064` | Gifted Program | Field Spell | cast conjure; trigger radiance; aura holy side | 1.28 | none: vanilla | cast: a Field Spell's play is its own landing in the backrow (`summoned`), which the census leaves out; conjure draws it; trigger: the census never saw this hook fire; radiance reads its radiantSet |
| `core-065` | Masochism Mask | Field Spell | cast conjure Dark/blood; trigger drain Dark/blood; aura void zone Dark/blood | 1.35 | vanish | cast: a Field Spell's play is its own landing in the backrow (`summoned`), which the census leaves out; conjure draws it; trigger: its life price is healthLost, which drain reads (the census's vanish is its other choice, a deck exile the board does not show) |
| `core-065-1` | Spikey Pillow | Unit | none: its Aura is a standing Attack loss, with no moment to draw | — | not in census | a token the census never played; its Aura is a standing Attack loss |
| `core-066` | The Rock | Unit | cry offering | 1.32 | offering | cry: the census agrees (offering) |
| `core-067` | Zoomerbin Oomen | Unit | cry conjure | 1.19 | none: no motif reads summoned | cry: no census family reads summoned, which conjure (#2786) reads |
| `core-068` | Twisted Sorcerer | Unit | cry bolt | 1.16 | bolt | cry: the census agrees (bolt) |
| `core-069` | Call to Arms | Spell | cast conjure | 1.16 | none: no motif reads summoned | cast: no census family reads summoned, which conjure (#2786) reads |
| `core-070` | Spiteful Stab | Spell | cast bolt Physical/blade | 1.24 | bolt | cast: the census agrees (bolt) |
| `core-071` | Intern Stimmy | Field Trap | cast snap | 1.19 | bolt | cast: a Trap's cast is its firing, which plans nothing until firings open a window; snap reads its trapFired then |
| `core-072` | Reminisce | Spell | cast send | 1.19 | none: no motif reads addedToHand | cast: no census family reads addedToHand and shuffledIn, which send (#2793) reads |
| `core-073` | Anti-oneshot Armor | Field Spell | cast flight Physical/steel; aura frost side Physical/steel | 1.28 | bolt | cast: the census's suggestion is noise in its window (fatigue, other cards' events); its own effect is drawn, which flight reads |
| `core-074` | Adaptive UI | Spell | cast bolt Tech/hologram | 1.36 | lance | cast: the census's lance counted its own Ghoul Token and its hero heal as targets; it deals X to one target, a bolt |
| `core-075` | Infinite Reserves | Field Spell | cast flight Hustle/cash; aura arcane zone Hustle/cash | 1.10 | none: no motif reads drawn | cast: no census family reads drawn, which flight (#2786) reads |
| `core-076` | Field of Dreams | Spell | cast send | 1.28 | none: no motif reads addedToHand | cast: no census family reads addedToHand and shuffledIn, which send (#2793) reads |
| `core-077` | Professor Curvature | Unit | none: its Cry adds a next-turn discount, whose modifier badge glints on its own | — | none: no motif reads modifierChanged | the census finds no motif either: its Cry adds a next-turn discount (modifierChanged) |
| `core-078` | /fullsend | Spell | cast mana | 1.65 | none: no motif reads modifierChanged | cast: no census family reads manaChanged and manaSpent, which mana (#2786) reads |
| `core-079` | Twinspell | Field Spell | cast conjure; aura arcane zone | 1.28 | none: no motif reads modifierChanged | cast: a Field Spell's play is its own landing in the backrow (`summoned`), which the census leaves out; conjure draws it |
| `core-080` | Zao Gao | Spell | cast conjure | 1.28 | volley | cast: the census's suggestion is noise in its window (fatigue, other cards' events); its own effect is summoned, which conjure reads |
| `core-081` | Radiant Saintess | Unit | death radiance | 1.45 | none: vanilla | death: no census family reads radiantSet, which radiance (#2793) reads |
| `core-082` | KY's Trial | Spell | cast send | 1.19 | none: no motif reads promptAnswered | cast: no census family reads addedToHand and shuffledIn, which send (#2793) reads |
| `core-083` | Transmogulate | Spell | bespoke legendaryTide | 1.62 | none: no motif reads transformed | bespoke: its transforms land in a hand and a deck the board does not show, which no motif anchors on |
| `core-084` | Going Long | Field Spell | cast conjure; aura holy side | 1.28 | none: vanilla | cast: a Field Spell's play is its own landing in the backrow (`summoned`), which the census leaves out; conjure draws it |
| `core-085` | Unlicensed Experimentation | Trap | cast snap | 1.62 | none: vanilla | cast: a Trap's cast is its firing, which plans nothing until firings open a window; snap reads its trapFired then |
| `core-086` | "Miss" Mrow | Unit | death tether Beast/felinor | 1.35 | none: vanilla | death: no census family reads stolen, controlChanged and held, which tether (#2786) reads |
| `core-087` | Pocket Chaos | Spell | cast shift | 1.85 | vanish | cast: the census's vanish is the cards a board swap moved; its swap is `swapped`, which shift reads |
| `core-088` | Twisting Nether | Spell | setPiece nether | 1.65 | wipe | kept: #2800's `nether` set piece (R2800000), which part F builds; the census's wipe is its removals |
| `core-089` | Corpse Eater | Unit | none: it grows in hand, with no board change to draw | — | none: vanilla | the census agrees |
| `core-090` | Super-Virus Injection | Spell | cast send | 1.19 | none: no motif reads shuffledIn | cast: no census family reads addedToHand and shuffledIn, which send (#2793) reads |
| `core-090-1` | Super-Virus | Spell | cast bolt | 0.97 | bolt | cast: the census agrees (bolt) |
| `core-091` | Fed Fauci | Unit | trigger mana | 1.28 | none: vanilla | trigger: the census leaves out the events on the card itself; its start-of-turn mana (manaChanged), its one trigger's payoff, mana reads |
| `core-092` | Felinor Fiender | Unit | none: no hook; its Legendary entrance plays | — | none: vanilla | the census agrees: no hook, and its Legendary entrance plays (R2722001) |
| `core-093` | Combo-Index | Field Spell | cast conjure; trigger tally; aura storm zone | 1.74 | bolt | cast: a Field Spell's play is its own landing in the backrow (`summoned`), which the census leaves out; conjure draws it; trigger: the census leaves out the events on the card itself, its counterChanged and questProgressed, which tally reads |
| `core-093-1` | Combo-Fodder | Spell | cast bolt | 0.90 | lance | cast: the census's lance counted its Lifesteal heal as a second target; it deals damage to one, a bolt |
| `core-094` | Genn's Greed | Spell | cast vanish Hustle/cash | 1.65 | vanish | cast: the census agrees (vanish) |
| `core-095` | Call to Chaos (Core Edition) | Spell | cast conjure | 1.85 | vanish | cast: the census's vanish is one of its ten random effects; conjure reads the four of them that summon |
| `core-095-1` | Chaos Golem | Unit | none: vanilla | — | not in census | a token the census never played: keywords only, which the keyword beats draw |
| `core-096` | My Pawn | Trap | cast snap | 1.73 | lance | cast: a Trap's cast is its firing, which plans nothing until firings open a window; snap reads its trapFired then |
| `core-097` | Zephrys | Spell | bespoke perfectWish | 1.73 | none: no motif reads addedToHand | bespoke: its "perfect" card blows in from the far side, which no hand motif says |
| `core-098` | Heroic Power | Field Spell | cast conjure; activate bolt; aura holy zone | 1.60 | bolt | cast: a Field Spell's play is its own landing in the backrow (`summoned`), which the census leaves out; conjure draws it; activate: the census agrees (bolt): its powers' damage |
| `core-099` | Craft a Card | Spell | bespoke fusionForge | 2.00 | none: no motif reads promptAnswered | bespoke: its Discovered Units fuse off the board, and no motif reads `fused` |
| `core-100` | Ceaseless Void | Unit | setPiece voidFold | 2.00 | vanish | kept: #2800's `voidFold` set piece and `voidCollapse` entrance (R2800000, R670); its takeover is #2800's |
| `core-t-rush` | Rush Token | Unit | none: vanilla | — | none: vanilla | keywords only, which the keyword beats and the attack rows draw |
| `core-t-sheep` | Sheep Token | Unit | none: vanilla | — | not in census | a token the census never played: vanilla, its Worth a Tribute count |
| `core-t-felinor` | Felinor Token | Unit | none: vanilla | — | not in census | a token the census never played: vanilla |
| `core-t-bread` | Bread Token | Unit | none: vanilla | — | not in census | a token the census never played: vanilla |
| `core-t-coin` | The Coin | Spell | cast mana Hustle/cash | 0.90 | none: no motif reads manaChanged | cast: no census family reads manaChanged and manaSpent, which mana (#2786) reads |
| `core-t-ghoul` | Ghoul Token | Unit | none: vanilla | — | not in census | a token the census never played: keywords only (Pierce), which the keyword beats draw |

## Bespoke

Each built from the motif primitives (`fx/motifs/build.ts`) at the card's motif scale, with no new
rendering (`fx/cards/core/`).

| Card | Recipe | Why bespoke | What plays |
|---|---|---|---|
| #83 Transmogulate | `legendaryTide` | its transforms land in a hand and a deck the board does not show, which no motif anchors on | at its first transform, Legendary rays, a ring and a gold burst over the caster's Deck and hand piles; a sheen on each hand card the viewer reads as it turns |
| #97 Zephrys | `perfectWish` | its "perfect" card blows in from the far side, which no hand motif says | a star gathering at the other hero, a bolt into the caster's hand pile, Mythic rays, a ring and a gold afterglow there |
| #99 Craft a Card | `fusionForge` | its Discovered Units fuse off the board, and no motif reads `fused` | two halves flying in from the board's edges, forged in a nova at its centre, the crafted card flying on into the hand |

No Legendary entrance or Mythic takeover is restyled: #92 Felinor Fiender and #52 Silly Silas keep
the shared Legendary entrance, #96 My Pawn and #97 Zephrys the shared takeover, and #100 Ceaseless
Void's is #2800's.

## Set-piece candidates

Each takes a motif for now; a later part may give it a set piece.

| Card | Why | Its motif now |
|---|---|---|
| #43 Big Felinor | its Cry destroys every non-Felinor Unit | `cry: wipe` |
| #52 Silly Silas | its Cry rotates every card on the field | `cry: shift` |
| #83 Transmogulate | Embiggened, it replaces the board too | bespoke `legendaryTide` |
| #87 Pocket Chaos | it can swap the boards or the decks | `cast: shift` |
| #95 Call to Chaos (Core Edition) | some of its ten random effects take the whole board | `cast: conjure` |
| #96 My Pawn | the rest of a lethal turn is played at random | `cast: snap` |

#17 Flood, #88 Twisting Nether and #100 Ceaseless Void are #2800's set pieces already; the board-wipe
part (F) builds #88 and #100.

## New motifs

Three events no motif read come up on three or more Core cards each, so each got a shared motif
(`docs/polish/9-card-animations.md`, "Drawn in #2793") rather than a recipe per card.

| Motif | Reads | Core cards |
|---|---|---|
| `send` | addedToHand, shuffledIn | #7 Jewelosco Scarab (`cry`), #33 Unstable Clone Machine (`trigger`), #37 Gravedigger (`trigger`), #39 Recycling Initiative (`trigger`), #51 KY's Private Tutor, #51.1 KY's Empty Notebook, #54 Straaza (`cry`), #57 Conjure KY, #59 Unbiased Immigration (`trigger`), #72 Reminisce, #76 Field of Dreams, #82 KY's Trial, #90 Super-Virus Injection |
| `radiance` | radiantSet | #23 Reoccurring Dream, #26 Glowy Jelly Bean, #28 Knockoff Temu Glowy Jelly Bean, #29 GIGA Glowy Jelly Bean, #64 Gifted Program (`trigger`), #81 Radiant Saintess (`death`) |
| `mend` | healed | #19 Midrange Menace (`trigger`), #47 Fig of Life, #53 Reno (`cry`) |
