# Classic+'s card animations (#2795)

Classic+'s pass over its animation table, `classicplus.json5` (`docs/polish/9-card-animations.md`, "A
set's pass"; R2795000): one row per Classic+ card, tokens included, in catalog order. `classicplus.test.ts`
holds every row's name, type, entry and scale to the catalog and the table, and the table to the pass's
floor.

- **Entry**: the card's entry in `classicplus.json5`: `none: <reason>`, `bespoke <recipe>`, `setPiece <key>`,
  or each hook's `<hook> <motif>[ <palette>]`, then `aura <look> <scope>[ <palette>]`.
- **Scale**: the card's motif scale at intensity 1 (`motifScale`), from its cost and rarity alone; `—`
  for a `none`.
- **Census**: the "Suggested" cell of `cargo jackioh fx census` (#2788) for the card, run on the engine of
  `0312164cd` (200 seeded games and 19 forced ones), or `not in census` for the tokens it never played.
  Its families predate the motifs of #2786, #2793 and #2796 and read damage, removals, counters, costs and
  returns only.
- **Why**: where the entry follows the census, and why it overrides it where it does.

Sub-looks follow each card's line: `Beast/felinor` for the Felinors, `Nerd/ink` for the Books, `Nerd/chalk`
for KY's cards, `Tech/hologram` for the AI cards, `Tech/laser` for Jlockeed's, `Tech/glitch` for the Audits
and `Hustle/cash` for the commerce cards. `cast` and `cry` draw on the board as the card resolves.
`death`, `trigger` and `activate`, and a Trap's `cast`, plan nothing on the board until their hooks open a
window (#2783); the `/dev/fx` gallery plays every entry.

| Card | Name | Type | Entry | Scale | Census | Why |
| --- | --- | --- | --- | --- | --- | --- |
| `classicplus-001` | Doom Shroom | Trap | setPiece vortex | 1.55 | vanish | kept from #2800: its board clear's set piece |
| `classicplus-002` | Groom Shroom | Trap | cast conjure Beast/felinor | 1.55 | volley | the census's pick is its window's turn, draw or stray events; the hook's own event decides, which conjure reads; a Trap's cast draws nothing in a game yet |
| `classicplus-003` | Second Amendment Snake | Unit | death barrage; trigger tally | 1.28 | barrage | death: the census agrees (barrage); trigger: its end-of-turn Plague Counter is a counterChanged, which tally reads |
| `classicplus-004` | Juhan Biggest Bat | Unit | cry mirror | 1.24 | none: no motif reads transformed | the census predates the motifs of #2786 and #2793; mirror reads the hook's commonest own event |
| `classicplus-005` | Guy Att | Unit | cry bolt | 1.16 | bolt | the census agrees |
| `classicplus-006` | Wrong-House Attacker | Unit | none: vanilla | — | none: vanilla | the census agrees |
| `classicplus-007` | The House | Field Spell | cast tally; activate conjure; aura toxic zone | 1.36 | bolt | cast: its Cry's Plague Counters (counterChanged), which tally reads; activate: the Unit it summons, which conjure reads; its activate plans nothing on the board until the marker of #2783 reaches it, so the gallery plays it |
| `classicplus-008` | Withering Storm | Spell | cast flight Dark/curse | 1.28 | bolt | the census's pick is its window's turn, draw or stray events; the hook's own event decides, which flight reads |
| `classicplus-009` | Silence | Spell | cast transmute | 1.00 | none: no motif reads transformed | the census predates the motifs of #2786 and #2793; transmute reads the hook's commonest own event |
| `classicplus-010` | New Wraps | Spell | cast bless | 1.00 | none: no motif reads keywordGranted | the census predates the motifs of #2786 and #2793; bless reads the hook's commonest own event |
| `classicplus-011` | Anime Armor | Unit | none: no hook (its Aura is static; the damage rows show the cap) | — | bolt | no hook: its Aura is static, and a Unit wears no aura on the board |
| `classicplus-012` | The Mother Pancake | Unit | trigger send | 1.74 | bolt | trigger: no census family reads addedToHand and shuffledIn, which send (#2793) reads; its trigger plans nothing on the board until the marker of #2783 reaches it, so the gallery plays it |
| `classicplus-012-1` | Devour | Spell | cast lance | 0.90 | lance | the census agrees |
| `classicplus-012-2` | Death Boil | Spell | cast bolt | 0.97 | bolt | the census agrees |
| `classicplus-012-3` | Fluffy Grip | Spell | cast send | 0.97 | none: no motif reads addedToHand | no census family reads addedToHand and shuffledIn, which send (#2793) reads |
| `classicplus-012-4` | Powder Spray | Spell | cast bolt | 0.97 | bolt | the census agrees |
| `classicplus-012-5` | Anti-Waffle Shell | Field Spell | cast bless; aura holy side | 0.97 | none: no motif reads keywordGranted | the census predates the motifs of #2786 and #2793; bless reads the hook's commonest own event |
| `classicplus-012-6` | Frozen Wastes | Spell | setPiece vortex | 1.04 | not in census | kept from #2800: its board clear's set piece (the census never saw it) |
| `classicplus-012-7` | Legion of the Hungry | Spell | none: token (one census play) | — | vanish | one census play, too few to earn a token an entry |
| `classicplus-012-8` | Frostspatula | Field Spell | none: token (two census plays) | — | vanish | two census plays, too few to earn a token an entry |
| `classicplus-013` | Mommy Barker | Unit | death send | 1.51 | none: vanilla | death: no census family reads addedToHand and shuffledIn, which send (#2793) reads; its death plans nothing on the board until the marker of #2783 reaches it, so the gallery plays it |
| `classicplus-014` | Forever& | Spell | cast seal | 1.35 | none: no motif reads modifierChanged | its play lays a player modifier (modifierChanged), which seal (#2796) reads |
| `classicplus-015` | Conjure Rush Token | Spell | cast conjure | 1.08 | none: no motif reads keywordGranted | the census predates the motifs of #2786 and #2793; conjure reads the hook's commonest own event |
| `classicplus-016` | Conjure Rush Token+ | Spell | cast conjure | 1.28 | none: no motif reads keywordGranted | the census predates the motifs of #2786 and #2793; conjure reads the hook's commonest own event |
| `classicplus-017` | Conjure Rush Token++ | Spell | cast conjure | 1.65 | vanish | the census's pick is its window's turn, draw or stray events; the hook's own event decides, which conjure reads |
| `classicplus-018` | Gullible Treatler | Unit | trigger offering | 1.16 | bolt | the census's pick is its window's turn, draw or stray events; the hook's own event decides, which offering reads; its trigger plans nothing on the board until the marker of #2783 reaches it, so the gallery plays it |
| `classicplus-019` | League of Losers | Spell | cast conjure Physical/sport | 1.85 | none: no motif reads summoned | the census predates the motifs of #2786 and #2793; conjure reads the hook's commonest own event |
| `classicplus-019-1` | Top Loser | Unit | none: token (no hook) | — | not in census | no hook |
| `classicplus-019-2` | Jungle Loser | Unit | none: token (its attack is combat) | — | bolt | its end-of-turn attack is combat, which the attack rows and keyword beats draw |
| `classicplus-019-3` | Mid Loser | Unit | cry bless | 1.04 | vanish | the census's pick is its window's turn, draw or stray events; the hook's own event decides, which bless reads |
| `classicplus-019-4` | Support Loser | Unit | trigger mend | 1.04 | none: no motif reads healed | trigger: its end-of-turn heals, which mend (#2793) reads; its trigger plans nothing on the board until the marker of #2783 reaches it, so the gallery plays it |
| `classicplus-019-5` | Bot Loser | Unit | none: token (turn events only) | — | bolt | its windows hold turn and draw events only |
| `classicplus-020` | Mushroom Power | Unit | cry bless | 1.08 | none: no motif reads buffed | the census predates the motifs of #2786 and #2793; bless reads the hook's commonest own event |
| `classicplus-021` | Whirlwind | Spell | cast nova | 1.00 | nova | the census agrees |
| `classicplus-022` | Blood Moon | Trap | cast bolt Dark/blood | 1.28 | bolt | the census agrees; a Trap's cast draws nothing in a game yet |
| `classicplus-023` | Dropshipping | Spell | cast send Hustle/cash | 1.35 | bolt | no census family reads addedToHand and shuffledIn, which send (#2793) reads |
| `classicplus-024` | Crushing Walls | Spell | bespoke crushingWalls | 1.55 | bolt | kept from #2784: its bespoke recipe |
| `classicplus-025` | Soul Shot | Spell | cast bolt | 1.16 | bolt | the census agrees |
| `classicplus-026` | Tommy Tempo | Unit | cry mana | 1.24 | none: no motif reads manaChanged | the census predates the motifs of #2786 and #2793; mana reads the hook's commonest own event |
| `classicplus-027` | Zephrys Zealotism | Spell | cast send | 2.00 | none: no motif reads addedToHand | no census family reads addedToHand and shuffledIn, which send (#2793) reads |
| `classicplus-028` | Nuestro hogar, nuestras tumbas | Unit | death mend | 1.16 | vanish | death: its heal, which mend (#2793) reads; its death plans nothing on the board until the marker of #2783 reaches it, so the gallery plays it |
| `classicplus-029` | Portal to the Past | Spell | cast send | 1.98 | none: vanilla | its Discover's card reaches the hand, which send (#2793) reads; the census saw only empty boards |
| `classicplus-030` | Felinor Fuser | Unit | cry transmute Beast/felinor | 1.65 | none: no motif reads promptAnswered | cry: no census family reads fused, which transmute reads (#2802) |
| `classicplus-031` | Fusion Lab | Field Spell | cast transmute; activate transmute; aura arcane zone | 1.45 | none: vanilla | cast and activate: no census family reads fused, which transmute reads (#2802); its activate plans nothing on the board until the marker of #2783 reaches it, so the gallery plays it |
| `classicplus-032` | Otherworldly Removal | Spell | cast send | 1.55 | none: no motif reads addedToHand | no census family reads addedToHand and shuffledIn, which send (#2793) reads |
| `classicplus-032-1` | Execute | Spell | cast bolt | 0.97 | bolt | the census agrees |
| `classicplus-032-2` | Brawl | Spell | cast wipe | 1.12 | bolt | kept from #2800: a clear of one side takes wipe |
| `classicplus-032-3` | Blade Storm | Spell | none: token (each Whirlwind it casts draws its nova) | — | none: vanilla | each Whirlwind it casts draws its own nova |
| `classicplus-033` | Ivory Tower | Field Spell | cast bless; aura holy side | 1.28 | bolt | its Aura grants Flying and Stack (keywordGranted), which bless reads |
| `classicplus-034` | Memory Leak | Field Spell | cast bind; trigger bind; aura frost lane Tech | 1.45 | bolt | the census predates the motifs of #2786 and #2793; bind reads the hook's commonest own event; its aura is R2787000's example |
| `classicplus-035` | Rollback | Spell | bespoke rollbackSweep | 1.85 | vanish | Flourish: no motif reads its one event, rolledBack |
| `classicplus-036` | Conjure Bones | Spell | cast send | 1.28 | none: no motif reads shuffledIn | no census family reads addedToHand and shuffledIn, which send (#2793) reads |
| `classicplus-036-1` | Bone Storm | Spell | cast lance | 0.97 | lance | the census agrees |
| `classicplus-037` | Wardrum | Unit | trigger conjure | 1.96 | bolt | the census's pick is its window's turn, draw or stray events; the hook's own event decides, which conjure reads; its trigger plans nothing on the board until the marker of #2783 reaches it, so the gallery plays it |
| `classicplus-038` | Solarius | Unit | death send | 1.45 | none: no motif reads damageAbsorbed | death: no census family reads addedToHand and shuffledIn, which send (#2793) reads; its death plans nothing on the board until the marker of #2783 reaches it, so the gallery plays it |
| `classicplus-038-1` | Solarius Prime | Unit | none: token (each Spell it casts draws its own) | — | bolt | each Spell it casts draws its own |
| `classicplus-039` | Book Worm | Unit | death send; trigger tally | 1.08 | bolt | death: no census family reads addedToHand and shuffledIn, which send (#2793) reads; trigger: its start-of-turn Plague Counter is a counterChanged, which tally reads; its death and trigger plan nothing on the board until the marker of #2783 reaches them, so the gallery plays them |
| `classicplus-040` | Appropriations | Spell | cast bless Hustle/cash | 1.55 | none: no motif reads buffed | the census predates the motifs of #2786 and #2793; bless reads the hook's commonest own event |
| `classicplus-041` | KY's Constant | Spell | bespoke constantStamp | 1.19 | none: no motif reads numberChanged | Flourish: no motif reads its one event, numberChanged |
| `classicplus-042` | KY's Test | Spell | cast send Nerd/chalk | 1.51 | none: no motif reads promptAnswered | no census family reads addedToHand and shuffledIn, which send (#2793) reads |
| `classicplus-042-1` | KY's Gift | Field Spell | none: token (no motif reads burned) | — | none: vanilla | its activate's commonest event is burned, which no motif reads |
| `classicplus-043` | AI Slop | Spell | cast send Tech/hologram | 1.74 | none: no motif reads addedToHand | no census family reads addedToHand and shuffledIn, which send (#2793) reads |
| `classicplus-044` | Simplicity Audit | Spell | cast vanish Tech/glitch | 1.36 | vanish | the census agrees |
| `classicplus-045` | Complexity Audit | Spell | cast vanish Tech/glitch | 1.28 | vanish | the census agrees |
| `classicplus-046` | Felinor Flagbearer | Unit | death send Beast/felinor | 1.62 | vanish | death: no census family reads addedToHand and shuffledIn, which send (#2793) reads; its death plans nothing on the board until the marker of #2783 reaches it, so the gallery plays it |
| `classicplus-046-1` | Felinor Flagbearer Prime | Unit | cry conjure Beast/felinor | 1.04 | none: no motif reads summoned | the census predates the motifs of #2786 and #2793; conjure reads the hook's commonest own event |
| `classicplus-047` | Jogg's Box | Spell | cast send | 1.85 | none: vanilla | send for the card its play adds to the hand (the census's one addedToHand); each Spell it casts draws its own motif |
| `classicplus-048` | Jlockeed's Lobbyist | Unit | death send Tech/laser | 1.51 | none: vanilla | death: no census family reads addedToHand and shuffledIn, which send (#2793) reads; its death plans nothing on the board until the marker of #2783 reaches it, so the gallery plays it |
| `classicplus-049` | Jay Fungus | Unit | trigger tithe | 1.28 | bolt | the census's pick is its window's turn, draw or stray events; the hook's own event decides, which tithe reads; its trigger plans nothing on the board until the marker of #2783 reaches it, so the gallery plays it |
| `classicplus-050` | Adaptive Growth | Spell | cast bless | 1.35 | wipe | the census predates the motifs of #2786 and #2793; bless reads the hook's commonest own event |
| `classicplus-051` | Jlockeed's J15 Fighter | Unit | none: vanilla | — | none: vanilla | the census agrees |
| `classicplus-052` | Jlockeed's Permanent Defense Contract | Spell | cast seal Tech/laser | 1.45 | none: no motif reads modifierChanged | its play lays a player modifier (modifierChanged), which seal (#2796) reads |
| `classicplus-053` | Book of Tokens | Spell | cast conjure Nerd/ink | 1.35 | none: no motif reads summoned | the census predates the motifs of #2786 and #2793; conjure reads the hook's commonest own event |
| `classicplus-054` | Book of Books | Spell | cast send Nerd/ink | 1.35 | bolt | no census family reads addedToHand and shuffledIn, which send (#2793) reads |
| `classicplus-055` | Book of Greed | Spell | cast send Nerd/ink | 1.35 | none: no motif reads addedToHand | no census family reads addedToHand and shuffledIn, which send (#2793) reads |
| `classicplus-056` | Book of Pain | Spell | bespoke inkTear | 1.35 | none: no motif reads discarded | Flourish: no motif reads its one event, discarded |
| `classicplus-057` | Book of Stats | Spell | cast bless Nerd/ink | 1.35 | none: no motif reads buffed | the census predates the motifs of #2786 and #2793; bless reads the hook's commonest own event |
| `classicplus-058` | Fruit Basket | Spell | cast send | 1.19 | none: no motif reads addedToHand | no census family reads addedToHand and shuffledIn, which send (#2793) reads |
| `classicplus-059` | All Purpose Apple | Spell | cast lance | 1.19 | lance | the census agrees |
| `classicplus-060` | Doctors Orders | Field Spell | cast send; activate send; aura bloom zone | 1.28 | bolt | cast and activate: no census family reads addedToHand and shuffledIn, which send (#2793) reads; its activate plans nothing on the board until the marker of #2783 reaches it, so the gallery plays it |
| `classicplus-061` | Bauble Bubble | Field Spell | cast send; death send; aura frost zone | 1.19 | bolt | cast and death: no census family reads addedToHand and shuffledIn, which send (#2793) reads; its death plans nothing on the board until the marker of #2783 reaches it, so the gallery plays it |
| `classicplus-062` | KY's Papaya | Spell | cast wipe Nerd/chalk | 1.45 | wipe | the census agrees |
| `classicplus-063` | Fruit Tree | Field Spell | cast send; trigger send; aura bloom zone | 1.36 | bolt | cast and trigger: no census family reads addedToHand and shuffledIn, which send (#2793) reads; its trigger plans nothing on the board until the marker of #2783 reaches it, so the gallery plays it |
| `classicplus-064` | Mulch Muncher | Unit | none: vanilla | — | none: vanilla | the census agrees |
| `classicplus-065` | Two Grapes | Spell | cast send | 1.19 | bolt | no census family reads addedToHand and shuffledIn, which send (#2793) reads |
| `classicplus-065-1` | Rotten Grape | Spell | cast drain | 0.97 | none: no motif reads healthLost | the census predates the motifs of #2786 and #2793; drain reads the hook's commonest own event |
| `classicplus-065-2` | Normal Grape | Spell | cast bolt | 0.97 | bolt | the census agrees |
| `classicplus-065-3` | Large Grape | Spell | cast bolt | 1.12 | bolt | the census agrees |
| `classicplus-065-4` | Golden Grape | Spell | cast radiance | 0.97 | bolt | its play makes a card Radiant (radiantSet), which radiance (#2793) reads |
| `classicplus-065-5` | Mythic Grape | Spell | none: token (two census plays) | — | none: no motif reads addedToHand | two census plays, too few to earn a token an entry |
| `classicplus-066` | Vine of Grapes | Spell | cast send | 1.36 | none: no motif reads addedToHand | no census family reads addedToHand and shuffledIn, which send (#2793) reads |
| `classicplus-067` | Pear | Spell | cast conjure | 1.28 | none: no motif reads summoned | the census predates the motifs of #2786 and #2793; conjure reads the hook's commonest own event |
| `classicplus-068` | Organic Produce | Field Spell | cast send; aura holy side | 1.65 | none: no motif reads addedToHand | no census family reads addedToHand and shuffledIn, which send (#2793) reads |
| `classicplus-069` | Buff Billy | Unit | cry bless | 1.36 | none: vanilla | the census's pick is its window's turn, draw or stray events; the hook's own event decides, which bless reads |
| `classicplus-070` | Chaos Machine | Field Spell | cast bless; trigger bless; aura storm board | 1.28 | bolt | the census's pick is its window's turn, draw or stray events; the hook's own event decides, which bless reads; its trigger plans nothing on the board until the marker of #2783 reaches it, so the gallery plays it |
| `classicplus-071` | Book of Buff | Spell | cast bless Nerd/ink | 1.35 | none: no motif reads upgraded | the census predates the motifs of #2786 and #2793; bless reads the hook's commonest own event |
| `classicplus-072` | Book of Nerf | Spell | cast curse Nerd/ink | 1.35 | bolt | the census predates the motifs of #2786 and #2793; curse reads the hook's commonest own event |
| `classicplus-073` | Call to Chaos (Classic+ Edition) | Spell | cast wipe | 1.85 | bolt | kept from #2800: a random effect takes wipe |
| `classicplus-073-1` | Classic Golem | Unit | none: token (its attack trigger is unmarked) | — | not in census | its attack trigger is unmarked |
| `classicplus-074` | Twice Forward One Step Backwards | Field Trap | cast transmute; aura void zone | 1.86 | none: vanilla | cast: its firing's fused, which transmute reads (#2802); a Field Trap's firing opens no play, so it draws nothing in a game yet |
| `classicplus-075` | J-lease J-Jungle EX-plorer | Unit | cry send | 1.74 | none: no motif reads shuffledIn | no census family reads addedToHand and shuffledIn, which send (#2793) reads |
| `classicplus-075-1` | J-lease J-Jungle EX-plorer Pack | Spell | cast send | 1.04 | none: no motif reads addedToHand | no census family reads addedToHand and shuffledIn, which send (#2793) reads |
| `classicplus-076` | Brother Lar | Unit | death conjure | 1.19 | none: vanilla | the census predates the motifs of #2786 and #2793; conjure reads the hook's commonest own event; its death plans nothing on the board until the marker of #2783 reaches it, so the gallery plays it |
| `classicplus-076-1` | Brother Ping | Unit | activate bolt | 1.04 | bolt | the census agrees; its activate plans nothing on the board until the marker of #2783 reaches it, so the gallery plays it |
| `classicplus-077` | Anti-Softlock | Spell | cast bless Nerd/pixel | 1.45 | none: no motif reads keywordGranted | the census predates the motifs of #2786 and #2793; bless reads the hook's commonest own event |
| `classicplus-078` | Claude's Datacenter | Field Spell | cast send Tech/hologram; activate send Tech/hologram; aura arcane zone Tech/hologram | 1.74 | none: vanilla | cast and activate: no census family reads addedToHand and shuffledIn, which send (#2793) reads; its activate plans nothing on the board until the marker of #2783 reaches it, so the gallery plays it |
| `classicplus-t-ai-01` | Helpful Assistant | Unit | none: token (not in the census) | — | not in census | not in the census |
| `classicplus-t-ai-02` | Scaling Law | Unit | none: token (no hook) | — | not in census | no hook |
| `classicplus-t-ai-03` | Hallucination | Spell | none: token (one census play) | — | none: no motif reads addedToHand | one census play, too few to earn a token an entry |
| `classicplus-t-ai-04` | Chain of Thought | Spell | none: token (one census play) | — | none: no motif reads addedToHand | one census play, too few to earn a token an entry |
| `classicplus-t-ai-05` | Autocomplete | Spell | none: token (two census plays) | — | none: no motif reads addedToHand | two census plays, too few to earn a token an entry |
| `classicplus-t-ai-06` | Datacenter Fire | Spell | cast nova | 1.04 | nova | the census agrees |
| `classicplus-t-ai-07` | Alignment Tax | Spell | cast seal | 0.97 | none: no motif reads modifierChanged | its play lays a player modifier (modifierChanged), which seal (#2796) reads, in 3 census windows |
| `classicplus-t-ai-08` | Rate Limit | Trap | none: token (one census play) | — | none: vanilla | one census play, too few to earn a token an entry |
| `classicplus-t-ai-09` | Refusal | Trap | cast negate | 0.97 | negate | the census agrees; a Trap's cast draws nothing in a game yet |
| `classicplus-t-ai-10` | Fine-Tuning | Field Spell | cast bless Tech/hologram; trigger bless Tech/hologram; aura arcane zone Tech/hologram | 1.04 | bolt | the census predates the motifs of #2786 and #2793; bless reads the hook's commonest own event; its trigger plans nothing on the board until the marker of #2783 reaches it, so the gallery plays it |
