# Classic+'s card animations (#2795)

Classic+'s pass over its animation table, `classicplus.json5` (`docs/polish/9-card-animations.md`, "A
set's pass"; R2795000): one row per Classic+ card, tokens included, in catalog order. `classicplus.test.ts`
holds every row's name, type, entry and scale to the catalog and the table, and the table to the pass's
floor.

- **Entry**: the card's entry in `classicplus.json5`: `none: <reason>`, `bespoke <recipe>`, `setPiece <key>`,
  or each hook's `<hook> <motif>[ <palette>]`, then `aura <look> <scope>[ <palette>]`. R3399000: a signature's
  reads `signature; entrance <key>` first, where it has an entrance, and `takeover <look>` last.
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
| `classicplus-001` | Doom Shroom | Trap | setPiece vortex | 1.65 | vanish | kept from #2800: its board clear's set piece |
| `classicplus-002` | Groom Shroom | Trap | cast conjure Beast/felinor | 1.55 | volley | the census's pick is its window's turn, draw or stray events; the hook's own event decides, which conjure reads; a Trap's cast draws nothing in a game yet |
| `classicplus-003` | Second Amendment Snake | Unit | death barrage; trigger tally | 1.28 | barrage | death: the census agrees (barrage); trigger: its end-of-turn Plague Counter is a counterChanged, which tally reads |
| `classicplus-004` | Juhan Biggest Bat | Unit | cry transmute | 1.24 | none: no motif reads transformed | the census predates the motifs of #2786 and #2793; transmute reads the hook's commonest own event (R2803000: mirror folded into transmute) |
| `classicplus-005` | Guy Att | Unit | cry bolt | 1.16 | bolt | the census agrees |
| `classicplus-006` | Wrong-House Attacker | Unit | none: vanilla | — | none: vanilla | the census agrees |
| `classicplus-007` | The House | Field Spell | cast tally; activate conjure; aura toxic zone | 1.36 | bolt | cast: its Cry's Plague Counters (counterChanged), which tally reads; activate: the Unit it summons, which conjure reads; its activate plans nothing on the board until the marker of #2783 reaches it, so the gallery plays it |
| `classicplus-008` | Withering Storm | Spell | cast flight Dark/curse | 1.19 | bolt | the census's pick is its window's turn, draw or stray events; the hook's own event decides, which flight reads |
| `classicplus-009` | Silence | Spell | cast transmute | 1.00 | none: no motif reads transformed | the census predates the motifs of #2786 and #2793; transmute reads the hook's commonest own event |
| `classicplus-010` | New Wraps | Spell | cast bless | 1.00 | none: no motif reads keywordGranted | the census predates the motifs of #2786 and #2793; bless reads the hook's commonest own event |
| `classicplus-011` | Anime Armor | Unit | none: no hook (its Aura is static; the damage rows show the cap) | — | bolt | no hook: its Aura is static, and a Unit wears no aura on the board |
| `classicplus-012` | The Mother Pancake | Unit | signature; entrance pancakeStack; bespoke pancakeFlip; aura syrup tile | 1.74 | bolt | R3399000: its signature (#3399), its beats and verdict in the Signatures table below |
| `classicplus-012-1` | Devour | Spell | signature; bespoke pancakeMouth | 0.90 | lance | R3399000: its signature (#3399), its beats and verdict in the Signatures table below |
| `classicplus-012-2` | Death Boil | Spell | signature; bespoke batterBubble | 0.97 | bolt | R3399000: its signature (#3399), its beats and verdict in the Signatures table below |
| `classicplus-012-3` | Fluffy Grip | Spell | signature; bespoke doughArm | 0.97 | none: no motif reads addedToHand | R3399000: its signature (#3399), its beats and verdict in the Signatures table below |
| `classicplus-012-4` | Powder Spray | Spell | signature; bespoke flourCloud | 0.97 | bolt | R3399000: its signature (#3399), its beats and verdict in the Signatures table below |
| `classicplus-012-5` | Anti-Waffle Shell | Field Spell | signature; entrance waffleIron; bespoke waffleGrid; aura waffleDome side Spirit | 0.97 | none: no motif reads keywordGranted | R3399000: its signature (#3399), its beats and verdict in the Signatures table below |
| `classicplus-012-6` | Frozen Wastes | Spell | signature; bespoke whiteout | 1.04 | not in census | R3399000: its signature (#3399), its beats and verdict in the Signatures table below |
| `classicplus-012-7` | Legion of the Hungry | Spell | signature; bespoke hungryPot | 1.04 | vanish | R3399000: its signature (#3399), its beats and verdict in the Signatures table below |
| `classicplus-012-8` | Frostspatula | Field Spell | signature; entrance griddleFreeze; bespoke spatulaFlip; aura frostMist zone Spirit | 1.04 | vanish | R3399000: its signature (#3399), its beats and verdict in the Signatures table below |
| `classicplus-013` | Mommy Barker | Unit | signature; entrance kitchenDoor; bespoke pancakeTray; aura ovenSteam tile | 1.51 | none: vanilla | R3399000: its signature (#3399), its beats and verdict in the Signatures table below |
| `classicplus-014` | Forever& | Spell | cast seal | 1.35 | none: no motif reads modifierChanged | its play lays a player modifier (modifierChanged), which seal (#2796) reads |
| `classicplus-015` | Conjure Rush Token | Spell | cast conjure | 1.08 | none: no motif reads keywordGranted | the census predates the motifs of #2786 and #2793; conjure reads the hook's commonest own event |
| `classicplus-016` | Conjure Rush Token+ | Spell | cast conjure | 1.28 | none: no motif reads keywordGranted | the census predates the motifs of #2786 and #2793; conjure reads the hook's commonest own event |
| `classicplus-017` | Conjure Rush Token++ | Spell | cast conjure | 1.65 | vanish | the census's pick is its window's turn, draw or stray events; the hook's own event decides, which conjure reads |
| `classicplus-018` | Gullible Treatler | Unit | trigger offering | 1.16 | bolt | the census's pick is its window's turn, draw or stray events; the hook's own event decides, which offering reads; its trigger plans nothing on the board until the marker of #2783 reaches it, so the gallery plays it |
| `classicplus-019` | League of Losers | Spell | signature; bespoke lineup | 1.85 | none: no motif reads summoned | R3399000: its signature (#3399), its beats and verdict in the Signatures table below |
| `classicplus-019-1` | Top Loser | Unit | signature; entrance shieldPlant; aura laneWall tile Physical/steel | 1.04 | not in census | R3399000: its signature (#3399), its beats and verdict in the Signatures table below |
| `classicplus-019-2` | Jungle Loser | Unit | signature; entrance vineNap; aura snoreLeaves tile Nature | 1.04 | bolt | R3399000: its signature (#3399), its beats and verdict in the Signatures table below |
| `classicplus-019-3` | Mid Loser | Unit | signature; entrance spotlightPose; bespoke coinCall; aura coinGlint tile Hustle | 1.04 | vanish | R3399000: its signature (#3399), its beats and verdict in the Signatures table below |
| `classicplus-019-4` | Support Loser | Unit | signature; entrance staffShuffle; bespoke sighMend; aura staffBreath tile | 1.04 | none: no motif reads healed | R3399000: its signature (#3399), its beats and verdict in the Signatures table below |
| `classicplus-019-5` | Bot Loser | Unit | signature; entrance flexBurst; bespoke killNotch; aura bowNotch tile Physical/blade | 1.04 | bolt | R3399000: its signature (#3399), its beats and verdict in the Signatures table below |
| `classicplus-020` | Mushroom Power | Unit | cry bless | 1.08 | none: no motif reads buffed | the census predates the motifs of #2786 and #2793; bless reads the hook's commonest own event |
| `classicplus-021` | Whirlwind | Spell | cast nova | 1.00 | nova | the census agrees |
| `classicplus-022` | Blood Moon | Trap | cast bolt Dark/blood | 1.28 | bolt | the census agrees; a Trap's cast draws nothing in a game yet |
| `classicplus-023` | Dropshipping | Spell | cast send Hustle/cash | 1.35 | bolt | no census family reads addedToHand and shuffledIn, which send (#2793) reads |
| `classicplus-024` | Crushing Walls | Spell | bespoke crushingWalls | 1.55 | bolt | kept from #2784: its bespoke recipe |
| `classicplus-025` | Soul Shot | Spell | cast bolt | 1.16 | bolt | the census agrees |
| `classicplus-026` | Tommy Tempo | Unit | cry mana | 1.24 | none: no motif reads manaChanged | the census predates the motifs of #2786 and #2793; mana reads the hook's commonest own event |
| `classicplus-027` | Zephrys Zealotism | Spell | signature; bespoke zealotCyclone; takeover zealot | 2.00 | none: no motif reads addedToHand | R3399000: its signature (#3399), its beats and verdict in the Signatures table below |
| `classicplus-028` | Nuestro hogar, nuestras tumbas | Unit | death mend | 1.16 | vanish | death: its heal, which mend (#2793) reads; its death plans nothing on the board until the marker of #2783 reaches it, so the gallery plays it |
| `classicplus-029` | Portal to the Past | Spell | signature; bespoke sepiaPortal; takeover portal | 1.98 | none: vanilla | R3399000: its signature (#3399), its beats and verdict in the Signatures table below |
| `classicplus-030` | Felinor Fuser | Unit | cry transmute Beast/felinor | 1.65 | none: no motif reads promptAnswered | cry: no census family reads fused, which transmute reads (#2802) |
| `classicplus-031` | Fusion Lab | Field Spell | cast transmute; activate transmute; aura arcane zone | 1.45 | none: vanilla | cast and activate: no census family reads fused, which transmute reads (#2802); its activate plans nothing on the board until the marker of #2783 reaches it, so the gallery plays it |
| `classicplus-032` | Otherworldly Removal | Spell | cast send | 1.55 | none: no motif reads addedToHand | no census family reads addedToHand and shuffledIn, which send (#2793) reads |
| `classicplus-032-1` | Execute | Spell | cast bolt | 0.97 | bolt | the census agrees |
| `classicplus-032-2` | Brawl | Spell | cast wipe | 1.12 | bolt | kept from #2800: a clear of one side takes wipe |
| `classicplus-032-3` | Blade Storm | Spell | none: token (each Whirlwind it casts draws its nova) | — | none: vanilla | each Whirlwind it casts draws its own nova |
| `classicplus-033` | Ivory Tower | Field Spell | cast bless; aura holy side | 1.28 | bolt | its Aura grants Flying and Stack (keywordGranted), which bless reads |
| `classicplus-034` | Memory Leak | Field Spell | cast bind; trigger bind; aura frost lane Tech | 1.45 | bolt | the census predates the motifs of #2786 and #2793; bind reads the hook's commonest own event; its aura is R2787000's example |
| `classicplus-035` | Rollback | Spell | signature; bespoke rollbackSweep | 1.85 | vanish | R3399000: its signature (#3399), its beats and verdict in the Signatures table below |
| `classicplus-036` | Conjure Bones | Spell | cast send | 1.28 | none: no motif reads shuffledIn | no census family reads addedToHand and shuffledIn, which send (#2793) reads |
| `classicplus-036-1` | Bone Storm | Spell | cast lance | 0.97 | lance | the census agrees |
| `classicplus-037` | Wardrum | Unit | signature; entrance drumLeap; bespoke drumBeat; aura warHeartbeat tile Fire | 1.85 | bolt | R3399000: its signature (#3399), its beats and verdict in the Signatures table below |
| `classicplus-038` | Solarius | Unit | death send | 1.45 | none: no motif reads damageAbsorbed | death: no census family reads addedToHand and shuffledIn, which send (#2793) reads; its death plans nothing on the board until the marker of #2783 reaches it, so the gallery plays it |
| `classicplus-038-1` | Solarius Prime | Unit | none: token (each Spell it casts draws its own) | — | bolt | each Spell it casts draws its own |
| `classicplus-039` | Book Worm | Unit | death send; trigger tally | 1.08 | bolt | death: no census family reads addedToHand and shuffledIn, which send (#2793) reads; trigger: its start-of-turn Plague Counter is a counterChanged, which tally reads; its death and trigger plan nothing on the board until the marker of #2783 reaches them, so the gallery plays them |
| `classicplus-040` | Appropriations | Spell | cast bless Hustle/cash | 1.55 | none: no motif reads buffed | the census predates the motifs of #2786 and #2793; bless reads the hook's commonest own event |
| `classicplus-041` | KY's Constant | Spell | bespoke constantStamp | 1.19 | none: no motif reads numberChanged | Flourish: no motif reads its one event, numberChanged |
| `classicplus-042` | KY's Test | Spell | signature; bespoke slateTest | 1.51 | none: no motif reads promptAnswered | R3399000: its signature (#3399), its beats and verdict in the Signatures table below |
| `classicplus-042-1` | KY's Gift | Field Spell | signature; entrance giftDesk; bespoke giftBoxes; aura appleRibbon zone Nerd/chalk | 1.19 | none: vanilla | R3399000: its signature (#3399), its beats and verdict in the Signatures table below |
| `classicplus-043` | AI Slop | Spell | signature; bespoke holoLoom | 1.74 | none: no motif reads addedToHand | R3399000: its signature (#3399), its beats and verdict in the Signatures table below |
| `classicplus-044` | Simplicity Audit | Spell | cast vanish Tech/glitch | 1.36 | vanish | the census agrees |
| `classicplus-045` | Complexity Audit | Spell | cast vanish Tech/glitch | 1.28 | vanish | the census agrees |
| `classicplus-046` | Felinor Flagbearer | Unit | signature; entrance bannerPlant; bespoke bannerFold; aura bannerBreeze tile Beast/felinor | 1.62 | vanish | R3399000: its signature (#3399), its beats and verdict in the Signatures table below |
| `classicplus-046-1` | Felinor Flagbearer Prime | Unit | signature; entrance greatBanner; aura greatBannerShade tile Beast/felinor | 1.04 | none: no motif reads summoned | R3399000: its signature (#3399), its beats and verdict in the Signatures table below |
| `classicplus-047` | Jogg's Box | Spell | signature; bespoke puzzleBox | 1.85 | none: vanilla | R3399000: its signature (#3399), its beats and verdict in the Signatures table below |
| `classicplus-048` | Jlockeed's Lobbyist | Unit | signature; entrance briefcaseStride; bespoke sealedEnvelope; aura laserDot tile Tech/laser | 1.51 | none: vanilla | R3399000: its signature (#3399), its beats and verdict in the Signatures table below |
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
| `classicplus-065-4` | Golden Grape | Spell | signature; bespoke grapeBounce | 0.97 | bolt | R3399000: its signature (#3399), its beats and verdict in the Signatures table below |
| `classicplus-065-5` | Mythic Grape | Spell | signature; bespoke vineRipen; takeover vineyard | 0.90 | none: no motif reads addedToHand | R3399000: its signature (#3399), its beats and verdict in the Signatures table below |
| `classicplus-066` | Vine of Grapes | Spell | cast send | 1.36 | none: no motif reads addedToHand | no census family reads addedToHand and shuffledIn, which send (#2793) reads |
| `classicplus-067` | Pear | Spell | cast conjure | 1.28 | none: no motif reads summoned | the census predates the motifs of #2786 and #2793; conjure reads the hook's commonest own event |
| `classicplus-068` | Organic Produce | Field Spell | cast send; aura holy side | 1.65 | none: no motif reads addedToHand | no census family reads addedToHand and shuffledIn, which send (#2793) reads |
| `classicplus-069` | Buff Billy | Unit | cry bless | 1.36 | none: vanilla | the census's pick is its window's turn, draw or stray events; the hook's own event decides, which bless reads |
| `classicplus-070` | Chaos Machine | Field Spell | cast bless; trigger bless; aura storm board | 1.28 | bolt | the census's pick is its window's turn, draw or stray events; the hook's own event decides, which bless reads; its trigger plans nothing on the board until the marker of #2783 reaches it, so the gallery plays it |
| `classicplus-071` | Book of Buff | Spell | cast bless Nerd/ink | 1.35 | none: no motif reads upgraded | the census predates the motifs of #2786 and #2793; bless reads the hook's commonest own event |
| `classicplus-072` | Book of Nerf | Spell | cast curse Nerd/ink | 1.35 | bolt | the census predates the motifs of #2786 and #2793; curse reads the hook's commonest own event |
| `classicplus-073` | Call to Chaos (Classic+ Edition) | Spell | signature; bespoke chaosLever | 1.85 | bolt | R3399000: its signature (#3399), its beats and verdict in the Signatures table below |
| `classicplus-073-1` | Classic Golem | Unit | signature; entrance templeStack; aura stoneFaces tile | 1.19 | not in census | R3399000: its signature (#3399), its beats and verdict in the Signatures table below |
| `classicplus-074` | Twice Forward One Step Backwards | Field Trap | signature; bespoke stairGlue; aura stairCracks zone; takeover stair | 1.86 | none: vanilla | R3399000: its signature (#3399), its beats and verdict in the Signatures table below |
| `classicplus-075` | J-lease J-Jungle EX-plorer | Unit | signature; entrance vineSwing; bespoke packLob; aura jungleLeaves tile Nature | 1.74 | none: no motif reads shuffledIn | R3399000: its signature (#3399), its beats and verdict in the Signatures table below |
| `classicplus-075-1` | J-lease J-Jungle EX-plorer Pack | Spell | signature; bespoke relicSpill | 1.04 | none: no motif reads addedToHand | R3399000: its signature (#3399), its beats and verdict in the Signatures table below |
| `classicplus-076` | Brother Lar | Unit | death conjure | 1.19 | none: vanilla | the census predates the motifs of #2786 and #2793; conjure reads the hook's commonest own event; its death plans nothing on the board until the marker of #2783 reaches it, so the gallery plays it |
| `classicplus-076-1` | Brother Ping | Unit | activate bolt | 1.04 | bolt | the census agrees; its activate plans nothing on the board until the marker of #2783 reaches it, so the gallery plays it |
| `classicplus-077` | Anti-Softlock | Spell | cast bless Nerd/pixel | 1.45 | none: no motif reads keywordGranted | the census predates the motifs of #2786 and #2793; bless reads the hook's commonest own event |
| `classicplus-078` | Claude's Datacenter | Field Spell | signature; entrance obeliskRise; bespoke printSlot; aura serverLights zone Tech/hologram | 1.74 | none: vanilla | R3399000: its signature (#3399), its beats and verdict in the Signatures table below |
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

## Signatures (#3399)

R3399000: every Classic+ Legendary and Mythic, tokens by the rarity they print, is its signature, its recipe
in `classicplus/` and each concept the issue's. `classicplus.test.ts` holds each row to the tables.

- **First beat**: a Unit's or a Field Spell's `entrance <key>`, or a Spell's or a Trap's `cast <recipe>` (its
  recipe claims its own `cardPlayed`, a Trap's its firing).
- **Text beat**: the recipe that draws what its text does, on the events the view carries; `—` for a card whose
  text emits nothing a recipe could key (its beat stands in its entrance or its tile).
- **Tile**: the aura of its own its tile wears (R3399001), `—` for a Spell.
- **Sound**: the effect its play or cast names in `card-audio.json5` (R3399004).
- **Verdict**: `as concept`, or `moved: <why>` where a beat moved to fit the budgets or the events the view carries.

| Card | Name | Type | First beat | Text beat | Tile | Sound | Verdict |
| --- | --- | --- | --- | --- | --- | --- | --- |
| `classicplus-012` | The Mother Pancake | Unit | entrance pancakeStack | pancakeFlip | aura syrup | motherPancakeSound | as concept |
| `classicplus-012-1` | Devour | Spell | cast pancakeMouth | pancakeMouth | — | devourSound | moved: the mouth rises over the board's centre, since its `cardPlayed` names no target, and snaps shut on the target at its `destroyed` |
| `classicplus-012-2` | Death Boil | Spell | cast batterBubble | batterBubble | — | deathBoilSound | moved: the bubble's arc is short, flown inside a Legendary Spell's burst window |
| `classicplus-012-3` | Fluffy Grip | Spell | cast doughArm | doughArm | — | fluffyGripSound | moved: the dough arm reels the card home at its `stolen`; its `addedToHand` and `costChanged` rows play as they are |
| `classicplus-012-4` | Powder Spray | Spell | cast flourCloud | flourCloud | — | powderSpraySound | as concept |
| `classicplus-012-5` | Anti-Waffle Shell | Field Spell | entrance waffleIron | waffleGrid | aura waffleDome | antiWaffleShellSound | moved: the dome's lift plays only on a `buffed`, which its aura (a buff read on demand) never emits |
| `classicplus-012-6` | Frozen Wastes | Spell | cast whiteout | whiteout | — | frozenWastesSound | moved: no board-clear hold (it leaves the `vortex` set piece to fit a Legendary Spell's ceiling), and the rime marks each side's middle zone, since its `cardPlayed` lists no Units |
| `classicplus-012-7` | Legion of the Hungry | Spell | cast hungryPot | hungryPot | — | legionOfTheHungrySound | moved: in a long run of exiles the crumbs bunch at the end of the Spell's burst window |
| `classicplus-012-8` | Frostspatula | Field Spell | entrance griddleFreeze | spatulaFlip | aura frostMist | frostspatulaSound | moved: its swing on its owner's turn is combat, which plays the shared attack rows; its Death's flip back is drawn |
| `classicplus-013` | Mommy Barker | Unit | entrance kitchenDoor | pancakeTray | aura ovenSteam | mommyBarkerSound | as concept |
| `classicplus-019` | League of Losers | Spell | cast lineup | lineup | — | leagueOfLosersSound | moved: each Loser's drop is its own signature entrance, which plays on its `summoned` before any recipe |
| `classicplus-019-1` | Top Loser | Unit | entrance shieldPlant | — | aura laneWall | topLoserSound | moved: his lane wall and chipping Armor stand in his tile's aura, since his text emits no event of its own |
| `classicplus-019-2` | Jungle Loser | Unit | entrance vineNap | — | aura snoreLeaves | jungleLoserSound | moved: his pounce follows an `attackDeclared`, which ends a hook's beat, so his leaves and snore stand in his tile's aura |
| `classicplus-019-3` | Mid Loser | Unit | entrance spotlightPose | coinCall | aura coinGlint | midLoserSound | as concept |
| `classicplus-019-4` | Support Loser | Unit | entrance staffShuffle | sighMend | aura staffBreath | supportLoserSound | as concept |
| `classicplus-019-5` | Bot Loser | Unit | entrance flexBurst | killNotch | aura bowNotch | botLoserSound | as concept |
| `classicplus-027` | Zephrys Zealotism | Spell | cast zealotCyclone | zealotCyclone | — | zephrysZealotismSound | as concept |
| `classicplus-029` | Portal to the Past | Spell | cast sepiaPortal | sepiaPortal | — | portalToThePastSound | moved: the view carries no last game's board, so the owner's ghost of it is a sepia haze over the board's unit zones |
| `classicplus-035` | Rollback | Spell | cast rollbackSweep | rollbackSweep | — | rollbackSound | moved: the clock's hand swings its one to three ticks at its `rolledBack`, which alone carries how many turns, and the sweep starts with its entry to fit a Legendary Spell's ceiling |
| `classicplus-037` | Wardrum | Unit | entrance drumLeap | drumBeat | aura warHeartbeat | wardrumSound | moved: his tile's heartbeat beats steadily, since an aura cannot count the non-Units played this turn |
| `classicplus-042` | KY's Test | Spell | cast slateTest | slateTest | — | kysTestSound | as concept |
| `classicplus-042-1` | KY's Gift | Field Spell | entrance giftDesk | giftBoxes | aura appleRibbon | kysGiftSound | as concept |
| `classicplus-043` | AI Slop | Spell | cast holoLoom | holoLoom | — | aiSlopSound | as concept |
| `classicplus-046` | Felinor Flagbearer | Unit | entrance bannerPlant | bannerFold | aura bannerBreeze | felinorFlagbearerSound | moved: the ribbons to the other Felinors play in his entrance, since his aura buffs on read and emits no `buffed` |
| `classicplus-046-1` | Felinor Flagbearer Prime | Unit | entrance greatBanner | — | aura greatBannerShade | felinorFlagbearerPrimeSound | moved: his Cry's copies land with a smaller variant of his entrance, which every copy's `summoned` reaches before any recipe, and the other Felinors' glow has no event (his aura buffs on read) |
| `classicplus-047` | Jogg's Box | Spell | cast puzzleBox | puzzleBox | — | joggsBoxSound | moved: the lid's snap and the dimming board are dropped, since the turn's end falls outside its play |
| `classicplus-048` | Jlockeed's Lobbyist | Unit | entrance briefcaseStride | sealedEnvelope | aura laserDot | jlockeedsLobbyistSound | as concept |
| `classicplus-065-4` | Golden Grape | Spell | cast grapeBounce | grapeBounce | — | goldenGrapeSound | as concept |
| `classicplus-065-5` | Mythic Grape | Spell | cast vineRipen | vineRipen | — | mythicGrapeSound | moved: the old hand withers at each `enteredGraveyard`, since the engine replaces the hand with no `discarded` |
| `classicplus-073` | Call to Chaos (Classic+ Edition) | Spell | cast chaosLever | chaosLever | — | callToChaosSound | as concept |
| `classicplus-073-1` | Classic Golem | Unit | entrance templeStack | — | aura stoneFaces | classicGolemSound | moved: its attack's reshape is unmarked in the event stream, so its rows play as they are; its carved faces and dust stand in its tile's aura |
| `classicplus-074` | Twice Forward One Step Backwards | Field Trap | cast stairGlue | stairGlue | aura stairCracks | twiceForwardSound | as concept |
| `classicplus-075` | J-lease J-Jungle EX-plorer | Unit | entrance vineSwing | packLob | aura jungleLeaves | jleaseExplorerSound | as concept |
| `classicplus-075-1` | J-lease J-Jungle EX-plorer Pack | Spell | cast relicSpill | relicSpill | — | jleasePackSound | as concept |
| `classicplus-078` | Claude's Datacenter | Field Spell | entrance obeliskRise | printSlot | aura serverLights | claudesDatacenterSound | as concept |
