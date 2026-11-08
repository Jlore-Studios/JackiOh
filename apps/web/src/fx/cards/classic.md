# Classic's card animations (#2794)

Classic's pass over its animation table, `classic.json5` (`docs/polish/9-card-animations.md`, "A set's
pass", R2794000): one row per Classic card, its token included, in catalog order. `classic.test.ts`
holds every row's name, type, entry and scale to the catalog and the table, and the table to the pass's
floor.

- **Entry**: the card's entry in `classic.json5`: `none: <reason>`, `bespoke <recipe>`, `setPiece <key>`,
  or `entrance <key>` (a marquee Unit's, R2794000), then each hook's `<hook> <motif>[ <palette>]`, then
  `aura <look> <scope>[ <palette>]`. A signature's recipe (R3389001) reads `signature; [entrance <key>; ]bespoke
  <recipe>[; aura <look> <scope>[ <palette>]][; takeover <look>]`.
- **Scale**: the card's motif scale at intensity 1 (`motifScale`), from its cost and rarity alone; `—`
  for a `none`.
- **Census**: the "Suggested" cell of `cargo jackioh fx census` (#2788) for the card (200 seeded games
  and 19 forced ones), or `not in census` for the token, which it does not list. Its families read
  damage, removals, counters, costs and returns only, so it predates #2786's motifs: a draw, a summon, a
  counter placed or a steal is `none` or noise to it.
- **Why**: where the entry follows the census, and why it overrides it where it does. "Fatigue" means
  the census's `bolt` came from fatigue damage in the play's window, not the card's effect.

`cast` and `cry` draw on the board as the card resolves. `death`, `trigger` and `activate`, and a
Trap's `cast`, plan nothing on the board until their hooks open a window (#2783); the `/dev/fx`
gallery plays every entry.

| Card | Name | Type | Entry | Scale | Census | Why |
| --- | --- | --- | --- | --- | --- | --- |
| `classic-001` | Curse of the Forgotten Classic | Spell | cast bolt Dark/curse | 1.19 | bolt | cast: the census agrees (bolt), in Dark's curse sub-look for a curse on the enemy hero |
| `classic-002` | The Trickster | Unit | none: its Cry only sets a cost modifier | — | none: no motif reads modifierChanged | its Cry changes nothing on the board, only a cost |
| `classic-003` | Book of Heal | Spell | bespoke pageMend | 1.35 | none: no motif reads healed | bespoke: its pages fly from the caster's hero to what it heals, where #2793's mend glows on the healed card alone |
| `classic-004` | Palantir | Field Spell | signature; entrance seeingOrb; bespoke palantirGaze; aura seeingIris zone Tech/hologram | 1.62 | negate | signature (#3389): a black seeing-orb drops and its iris opens, a scan sweeping the other side; its steal cracks the orb and an eye-beam reels the Spell across; a cyan iris watches from its zone |
| `classic-005` | Tesla | Field Trap | cast bolt | 1.45 | bolt | cast: the census agrees (bolt) |
| `classic-006` | Cloaked Toe Cracker | Unit | none: vanilla | — | none: vanilla | the census agrees: its reveal is its entrance, and its Aura shows nothing |
| `classic-007` | InfiniScepter | Field Spell | signature; entrance sceptrePlant; bespoke sceptreLocket; aura sceptreLoop zone | 1.51 | vanish | signature (#3389): a crystal sceptre spins down and plants, its locket opening; its Cry draws the Spell into the locket, each Activate sends a copy out; the held Spell turns in its loop |
| `classic-008` | Pickle | Spell | cast curse Dark/curse | 1.19 | none: vanilla | cast: its effect waits a turn, so its play shows nothing; a curse in Dark's curse sub-look says the threat |
| `classic-009` | Income Tax | Trap | signature; bespoke taxSeizure | 1.74 | none: vanilla | signature (#3389): as it fires, a strongbox lid flies open, a chain and a crimson seal close on the other hand, and each card it takes is yanked across, a card back; a face-down Trap wears no aura |
| `classic-010` | Exile | Trap | cast negate | 1.16 | negate | cast: the census agrees (negate) |
| `classic-011` | Mind Melt | Spell | cast vanish | 1.08 | vanish | cast: the census agrees (vanish) |
| `classic-012` | Book of Blood | Spell | cast bolt Dark/blood | 1.35 | lance | cast: it hits one Unit, a bolt; the census's lance counted its Lifesteal heal as a second target; blood sub-look |
| `classic-013` | Boots on the Ground | Unit | none: its draw comes as it attacks, which takes no hook marker | — | none: vanilla | the census agrees: its draw comes in combat, which opens no hook window |
| `classic-014` | Shadowstep | Trap | cast recall Dark/shadow | 1.08 | none: vanilla | cast: its firing bounces your dying Units to the hand, which recall (R2794000) reads; #2793's send flies a made card from the hero, not one back; shadow sub-look |
| `classic-015` | Nose Hunter | Unit | activate vanish | 1.08 | vanish | activate: the census agrees (vanish) |
| `classic-016` | Book of Flame | Spell | cast scorch Fire | 1.35 | bolt | kept: #2784's example; scorch burns where the bolt lands |
| `classic-017` | Counterspell | Trap | cast negate | 1.16 | negate | cast: the census agrees (negate) |
| `classic-018` | Glitch in the System | Spell | cast vanish Tech/glitch | 1.55 | vanish | cast: the census agrees (vanish), in Tech's glitch sub-look; a set-piece candidate |
| `classic-019` | Lizard's Breath | Spell | cast scorch | 1.19 | bolt | cast: a breath of fire burns where it hits, which scorch draws over the census's bolt |
| `classic-020` | The Power to Punish | Field Spell | cast bolt; activate bolt; aura void side | 1.36 | bolt | cast and activate: the census agrees (bolt), its Activate's damage |
| `classic-021` | Turtinator | Unit | activate offering | 1.16 | offering | activate: the census agrees (offering), its Tribute the cost |
| `classic-022` | Mid Runner | Unit | cry vanish Physical/wind | 1.08 | vanish | cry: the census agrees (vanish), in Physical's wind sub-look for a runner |
| `classic-023` | Devil's Pact | Field Spell | cast offering Dark/blood; aura ember side Dark/blood | 1.28 | none: no motif reads discarded | cast: its discards are a price paid, which offering draws; blood sub-look |
| `classic-024` | Book of Knowledge | Spell | cast flight Nerd/ink | 1.35 | bolt | cast: fatigue; it draws, which flight (#2786) reads; ink sub-look |
| `classic-025` | Lag in the System | Spell | cast vanish Tech/glitch | 1.25 | vanish | cast: the census agrees (vanish), in Tech's glitch sub-look; a set-piece candidate |
| `classic-026` | Rapid Draw | Spell | cast flight | 1.00 | bolt | cast: fatigue; it draws, which flight (#2786) reads |
| `classic-027` | Pestilent Slime | Unit | death tally | 1.00 | none: vanilla | death: it spreads its Plague Counters, which tally (#2786) reads; its play shows nothing, so the census saw none |
| `classic-028` | Second Wind | Field Spell | signature; entrance galeBurst; bespoke secondGust; aura graveBreeze zone Physical/wind | 1.40 | vanish | signature (#3389): a gale bursts from its zone and it lands in a whirl of wind; its Cry blows the deck into exile and the hand into the graveyard, back by back; a breeze stands on its zone |
| `classic-029` | Book of Vital Kill | Spell | cast drain Dark/blood | 1.35 | none: no motif reads healthSet | cast: no census family reads healthSet, which drain (#2786) reads; blood sub-look |
| `classic-030` | Recycle | Spell | bespoke recycleLoop | 1.19 | none: no motif reads shuffledIn | bespoke: its graveyard visibly loops into the deck, where #2793's send flies a made card from the caster's hero |
| `classic-031` | Cookie Guild | Unit | cry conjure | 1.16 | none: no motif reads summoned | cry: no census family reads summoned, which conjure (#2786) reads |
| `classic-032` | Felinor Feelings | Spell | cast tether Beast/felinor | 1.28 | none: vanilla | cast: its steal found no free lane in the census's games; tether draws a steal; felinor sub-look |
| `classic-033` | Joro | Unit | signature; entrance silkDrop; bespoke webRedirect; aura silkWeb zone | 1.40 | none: vanilla | signature (#3389): nothing shows in the hand; a thread drops and it swings down into its zone, and a web snaps from the Spell's aim to it as it takes the target; a faint web in its zone |
| `classic-034` | Ancient Acquisition | Spell | cast recall Nerd/ink | 1.19 | none: no motif reads addedToHand | cast: a card back from the graveyard, which recall (R2794000) reads; #2793's send flies a made card from the hero; ink sub-look |
| `classic-035` | Prep | Spell | cast flight | 1.00 | none: no motif reads modifierChanged | cast: its Embiggened draw, which flight (#2786) reads |
| `classic-036` | Burn | Spell | cast bolt | 1.00 | bolt | cast: the census agrees (bolt) |
| `classic-037` | Last Hurrah | Spell | cast flight | 1.25 | bolt | cast: fatigue; it draws your deck, which flight (#2786) reads |
| `classic-038` | Jackiestan Auctioneer | Field Trap | cast bolt Hustle/cash | 1.28 | bolt | cast: the census agrees (bolt), in Hustle's cash sub-look |
| `classic-039` | Outbreak | Spell | cast tally | 1.35 | bolt | cast: it places counters, which tally (#2786) reads |
| `classic-040` | MC Tech | Unit | cry tether | 1.19 | none: no motif reads controlChanged | cry: no census family reads controlChanged, which tether (#2786) reads |
| `classic-041` | State of the Game | Unit | none: vanilla | — | none: vanilla | the census agrees |
| `classic-042` | Transmutable Toxins | Field Spell | cast tally; activate tally; aura toxic board | 1.28 | wipe | cast and activate: it places counters, which tally (#2786) reads; the census's wipe read the deaths they cause |
| `classic-043` | Plague Nuke | Spell | setPiece vortex | 1.65 | wipe | kept: #2800's `vortex` set piece (R2800000), which draws its wipe as one beat |
| `classic-044` | Back from the GY | Spell | signature; bespoke graveRise | 1.85 | rise | signature (#3389): a lantern flare over the graveyard and cracks along the row, each Unit clawing up through its zone, the Spell crumbling to ash into exile; a Spell wears no aura |
| `classic-045` | Nature Titan | Unit | signature; entrance titanBloom; bespoke titanLeaf; aura mossCreep zone | 1.62 | bolt | signature (#3389): its `titanBloom` (R670) grown: roots crack, a moss boulder heaves up, wildflowers ring its foot; its Cry draws on a leaf and a seed heals its hero; moss creeps over its zone |
| `classic-046` | Divine Favor | Spell | cast flight | 1.19 | bolt | cast: fatigue; it draws, which flight (#2786) reads |
| `classic-047` | Recurring Felinor | Unit | trigger recall Beast/felinor | 1.28 | none: vanilla | trigger: from the graveyard to the hand, which recall (R2794000) reads; felinor sub-look |
| `classic-048` | Hired Shrimp | Unit | cry bolt | 1.16 | bolt | cry: the census agrees (bolt) |
| `classic-049` | Anti-Greed Machine | Unit | none: vanilla | — | none: vanilla | the census agrees: its Aura limits draws, with no moment to draw |
| `classic-050` | Voidwalker | Unit | cry vanish Dark/shadow | 1.28 | vanish | cry: the census agrees (vanish), in Dark's shadow sub-look |
| `classic-051` | Back Breaker | Unit | death wipe | 1.08 | bolt | death: it destroys every backrow card, which wipe draws; a set-piece candidate |
| `classic-052` | Final Gambit | Trap | cast flight | 1.45 | bolt | cast: fatigue; it heals and draws, and flight (#2786) reads the draw |
| `classic-053` | Plague Crawler | Unit | cry tally; trigger flight | 1.08 | none: no motif reads promptAnswered | cry: it places counters, which tally (#2786) reads; trigger: it draws, which flight reads |
| `classic-054` | Rewind | Spell | cast transmute | 1.08 | none: vanilla | cast: the repeated Cry plays as its own card's, which transmute draws (R2803000: mirror folded into transmute) |
| `classic-055` | Book of Wildfire | Spell | cast bolt Fire | 1.35 | bolt | cast: the census agrees (bolt); Fire sets it apart from Book of Flame's scorch |
| `classic-056` | Spell Tyrant | Unit | signature; entrance tyrantSigil; bespoke tyrantCommand; aura crownSigil zone | 1.85 | none: no motif reads promptAnswered | signature (#3389): its `tyrantSigil` (R670) grown: a crown-sigil slams, cracking the stone; each Spell its Cry casts rises to its crown, then burns into exile; a crown-sigil turns above it |
| `classic-057` | Echo | Spell | cast bolt | 1.35 | bolt | cast: the census agrees (bolt) |
| `classic-058` | Common Resources | Field Spell | cast flight; trigger flight; aura bloom side | 1.16 | none: vanilla | cast and trigger: its start-of-turn draw, which flight (#2786) reads; its play shows nothing, so its cast takes its hook's motif |
| `classic-059` | Plague Doctor | Unit | cry bolt | 1.08 | bolt | cry: the census agrees (bolt) |
| `classic-060` | Pile On | Spell | cast conjure Physical/sport | 1.75 | none: no motif reads summoned | cast: no census family reads summoned, which conjure (#2786) reads; sport sub-look |
| `classic-061` | Plague Bringer Goliath | Unit | cry tally | 1.36 | offering | cry: the census's offering read its Tribute, a cost; its Cry places counters, which tally (#2786) reads |
| `classic-062` | Living Bomb | Field Spell | cast nova; trigger nova; aura toxic board | 1.28 | bolt | cast and trigger: its kills of every countered permanent land at once, which nova draws; a set-piece candidate |
| `classic-063` | Crop Dusting | Trap | cast tally | 1.08 | nova | cast: the census's nova read fatigue; it places counters, which tally (#2786) reads |
| `classic-064` | Malzahar's Recycler | Field Spell | cast flight; trigger flight; aura arcane side | 1.19 | bolt | cast and trigger: fatigue; it draws, which flight (#2786) reads |
| `classic-065` | Ace in the Hole | Trap | cast conjure | 1.16 | none: vanilla | cast: its Recruit summons, which conjure (#2786) reads |
| `classic-066` | EU Striker | Unit | trigger conjure Physical/sport | 1.16 | bolt | trigger: the census's bolt is noise; it summons itself, which conjure (#2786) reads; sport sub-look |
| `classic-067` | Felinor Feeler | Unit | cry shift Beast/felinor | 1.08 | none: no motif reads positionSwitched | cry: no census family reads positionSwitched, which shift (#2786) reads; felinor sub-look |
| `classic-068` | Small Card Lobbyist | Unit | none: vanilla | — | none: vanilla | the census agrees: its Aura taxes costs, with no moment to draw |
| `classic-069` | Plague Charger | Unit | none: vanilla | — | none: vanilla | the census agrees |
| `classic-070` | Book of Plague | Spell | cast tally Toxic | 1.35 | none: no motif reads counterChanged | cast: no census family reads counterChanged, which tally (#2786) reads; Toxic over its Nerd flavor |
| `classic-071` | Lane Eater | Unit | cry lance | 1.24 | lance | kept: #2784's example; the census agrees |
| `classic-072` | Grand Counterspell | Trap | cast negate | 1.28 | negate | cast: the census agrees (negate) |
| `classic-073` | Nurse Cleaver | Unit | none: vanilla | — | none: vanilla | the census agrees: keywords only, which the keyword beats draw |
| `classic-074` | Corpse Plantation | Field Spell | cast tally; activate rise; aura bloom zone | 1.45 | bolt | cast: the census's bolt is noise; it places counters, which tally (#2786) reads; activate: it raises a Unit, which rise draws |
| `classic-075` | Argusland | Field Spell | cast bless; aura holy side | 1.36 | none: vanilla | cast: its standing guard draws in the rows; ward folded into the rows (R2803000), so cast names bless, its nearest motif |
| `classic-076` | Plague Bringer | Unit | cry tally | 1.28 | bolt | cry: it places counters, which tally (#2786) reads |
| `classic-077` | Anti-Magic Monkey | Unit | none: vanilla | — | bolt | the census's bolt is one stray damage; its Aura taxes Spells, with no moment to draw |
| `classic-078` | Mutate Spell | Field Spell | cast vanish; activate vanish; aura arcane zone | 1.19 | vanish | cast and activate: the census agrees (vanish) |
| `classic-079` | Risky Die | Spell | cast vanish | 1.08 | vanish | cast: the census agrees (vanish) |
| `classic-080` | BOOM! Big Max | Unit | signature; entrance bigBoom; bespoke craterLock; aura craterSmoke zone Fire | 1.85 | offering | signature (#3389): its `bigBoom` (R670) grown: a fuse along the row, the drop, the blast down its lane; its Crater cracks and clamps its zone; smoke curls from the crater |
| `classic-081` | The Power to Thrive | Field Spell | cast flight; activate flight; aura bloom side | 1.28 | bolt | cast and activate: fatigue; its Activate draws or heals, and flight (#2786) reads the draw |
| `classic-082` | Sheeople | Unit | death flight Beast/wool | 1.08 | bolt | death: fatigue; its Death draws, which flight (#2786) reads; wool sub-look |
| `classic-083` | Flame Lance | Spell | cast lance | 1.24 | lance | cast: the census agrees (lance) |
| `classic-084` | Lockdown | Field Spell | cast bind Dark/curse; trigger bind Dark/curse; aura frost board Dark/curse | 1.28 | none: vanilla | cast and trigger: no census family reads locked, which bind (#2786) reads; curse sub-look |
| `classic-085` | King Wagtoggle | Unit | signature; entrance royalCloak; bespoke crownSwap; aura cloakShimmer zone | 1.85 | none: no motif reads swapped | signature (#3389): a velvet cloak sweeps in and he tips his crown; the two decks cross under a crown, and its Radiant Recruit leaps out of its new deck; a crimson shimmer runs along its cloak |
| `classic-086` | Genn | Unit | none: vanilla | — | none: vanilla | the census agrees: no text |
| `classic-087` | Plague Chalice | Field Spell | cast negate; trigger negate; aura void zone | 1.55 | negate | cast and trigger: the census agrees (negate) |
| `classic-088` | Siphon Squad | Field Trap | cast curse | 1.36 | bolt | cast: the census's bolt is noise; its Aura weakens, which curse draws |
| `classic-089` | Paul Allen's Ghost | Unit | none: vanilla | — | none: vanilla | the census agrees: its Divine Shield has its own keyword beat |
| `classic-090` | In Too Deep | Field Spell | signature; entrance abyssSurface; bespoke shrineDescent; aura abyssWhirl zone Dark/shadow; takeover abyss | 1.73 | bolt | signature (#3389): its own takeover tips the board into a spiral shaft, water rushing down it, and it surfaces; each quest step lights a shrine lower down its spiral; a whirlpool turns on its zone |
| `classic-t-glitch` | Glitch | Spell | none: token with no text | — | not in census | a token with no text |

## Bespoke

Each built from the motif primitives (`fx/motifs/build.ts`) at the card's motif scale, over its
events' own rows (`keep`), with no new rendering (`fx/cards/classic/`).

| Card | Recipe | Why bespoke | What plays |
|---|---|---|---|
| #3 Book of Heal | `pageMend` | its heal travels from the caster, where #2793's `mend` glows on the healed card alone | pages lifting from the caster's hero, a sparkle flying to what it heals, a ring, a mend burst and a glow there |
| #30 Recycle | `recycleLoop` | its graveyard goes back into the deck, which no motif anchors on the two piles | once a play, on its first shuffle: a lift on the graveyard pile, a card back flying to the Deck pile, a ring and a leafy loop there |
| #85 King Wagtoggle | `crownSwap` | it swaps the two decks, and no motif reads `swapped` | a lift on both Deck piles, a gold crown ringed at the middle of the board, two card backs crossing under it, a ring on each pile, a small shake; its Radiant Recruit leaping from its new deck (#3389) |
| #4 Palantir | `palantirGaze` | its steal is a counter into its owner's hand, by it (#3389) | under its steal's marker, its own Tribute: a crack, shards and an eye-beam onto the other hand; the `countered` it makes: rings and a card back from hand to hand, in place of the fizzle |
| #7 InfiniScepter | `sceptreLocket` | it stores a Spell and casts copies of it (#3389) | its Cry's first exile: a card back from the hand into its zone, a ring and sparkle; each `activated` of its own: rings, a sheen and a card back out to the middle, in place of that row |
| #9 Income Tax | `taxSeizure` | it takes the other hand (#3389) | its own firing: a lid burst, coins, a chain beam and a crimson seal over the other hand; each card stolen from that hand: a card back across, in place of the steal's row |
| #28 Second Wind | `secondGust` | its Cry exiles the deck and discards the hand (#3389) | the first exile: a gust and card backs from the deck pile to exile, leaves there; the first discard: backs from the hand to the graveyard, ringed, in place of that row; beside a whole pile's one wave too |
| #33 Joro | `webRedirect` | it takes a Spell's target from the hand (#3389) | the `redirected` it makes: a snap at the first aim, a web beam to it and a wink of sparkle |
| #44 Back from the GY | `graveRise` | it raises the graveyard and exiles itself (#3389) | its own play: a lantern flare on the graveyard and cracks along the row; each Unit it summons: a ring and a crack, in place of the summon's row; its own exile: a card back into exile, in place of that row |
| #45 Nature Titan | `titanLeaf` | its Cry draws and heals (#3389) | its draw: a card back on a leaf from the deck to the hand, a seed flying from it to its hero; its hero's heal: two bloom rings there |
| #56 Spell Tyrant | `tyrantCommand` | its Cry casts the graveyard's Spells (#3389) | each cast its play makes: a card back rising from the graveyard to its crown, a bow of sparkle and a ring; that cast's exile: a card back into exile, in place of that row |
| #80 BOOM! Big Max | `craterLock` | its Crater Locks its zone as it lands (#3389) | the `locked` of its own row: a crack, crater smoke, a clamp ring and a shake |
| #90 In Too Deep | `shrineDescent` | its quests (#3389) | each step of its own quests: a shrine light and ring lower down a spiral on the card; a quest completed: a burst and two rings |

## Signatures (#3389)

Every Classic Legendary and Mythic has a signature of its own (R3389000, R3389001), held to its
budget by `tiers.test.ts` and played card by card by `classic/signatures.test.ts`, `/dev/fx` and
`e2e/cypress/component/fx-signature-classic.cy.tsx`. A Unit plays an entrance of its own, then its text's
beat; a Spell or a Trap a cast of its own, which stands in for the shared Legendary rays; a permanent
also wears a standing aura of its own on its zone (a signature look, `fx/aura/looks.ts`). Each lays a
sound bed of its own over its entrance's entry (`audio/signatureBeds.ts`). A beat the issue's concept
asked for and the view's events cannot carry, or that would break its budget, moved, as the issue allows.

| Card | Entrance or cast | Its text's beat | On its zone | Sound | What moved | Verdict |
|---|---|---|---|---|---|---|
| #4 Palantir | `seeingOrb`: an orb drops, its iris opens, a scan sweeps the other side | its Tribute cracks the orb and an eye-beam locks on the other hand; the stolen Spell reels across as a card back | `seeingIris` | `radiant`, `lock` | the draw limit's shutter (`drawLimited` names no source); the beam's lock to the crack (budget); the stolen Spell's cast plays its own entry | built |
| #7 InfiniScepter | `sceptrePlant`: a sceptre spins down, plants, its locket opens | its Cry draws the Spell into the locket; each Activate sends a spectral copy out | `sceptreLoop` | `radiant`, `upgrade` | the Activate's pulse is rings and a sheen, no burst (budget) | built |
| #9 Income Tax | `taxSeizure` at its firing: a strongbox lid, a chain, a crimson seal | each card it takes is yanked across, faces never shown | none (a face-down Trap shows nothing) | `lock`, `goldBurst`, `steal` | the chain to the firing (budget); the deferred keep on the drawer's next turn opens no hook of its own | built |
| #28 Second Wind | `galeBurst`: a gale bursts, it lands in a whirl of wind | its Cry blows the deck into exile and the hand into the graveyard, back by back | `graveBreeze` | `whoosh`, `sand`, `drain` | the breeze stands on its own zone, not on the graveyard pile; later graveyard plays open no play of its own | built |
| #33 Joro | `silkDrop`: a thread drops, it swings down into its zone | a web snaps from the Spell's aim to it as it takes the target | `silkWeb` | `whoosh`, `summon`, `emoteLaugh` | none | built |
| #44 Back from the GY | `graveRise` at its play: a lantern flare, cracks along the row | each Unit claws up through its zone; the Spell crumbles into exile | none (a Spell) | `impact`, `crumble`, `death` | the soil's burst is a ring and a crack on each zone (a Spell's shorter budget) | built |
| #45 Nature Titan | `titanBloom`, grown: roots, a moss boulder, wildflowers | its Cry draws on a leaf and a seed heals its hero | `mossCreep` | `tapWood`, `summon` huge, `heal` | the same beats as it attacks (no hook of its card opens) | built |
| #56 Spell Tyrant | `tyrantSigil`, grown: a crown-sigil slams, the stone cracks | each cast Spell rises to its crown, casts, then burns into exile | `crownSigil` | `horn`, `whoosh` | the ash's burst (a card back into exile, budget) | built |
| #80 BOOM! Big Max | `bigBoom`, grown: a fuse along the row, the drop, a blast down its lane | its Crater cracks and clamps its zone | `craterSmoke` | `fuse`, `overkill`, `crumble` | blows sliding off as cinders and Trample's blast (combat opens no hook of its card) | built |
| #85 King Wagtoggle | `royalCloak`: a cloak sweeps in, he tips his crown | the decks cross under a crown; its Radiant Recruit leaps from its new deck | `cloakShimmer` | `emoteLaugh`, `give`, `riffle` | his wink is the cloak's shimmer | built |
| #90 In Too Deep | its own takeover, `abyss`, then `abyssSurface`: water bursts, it surfaces | each quest step lights a shrine lower down its spiral; a quest completed bursts it open | `abyssWhirl` | `drain`, `whoosh`, `notify` | the rewards' glow (its owner's prompt, never drawn) | built |

## Set-piece candidates

Each takes a motif for now; a later part may give it a set piece.

| Card | Why | Its motif now |
|---|---|---|
| #18 Glitch in the System | it exiles every card on the field of the chosen cost | `cast: vanish` |
| #25 Lag in the System | it exiles every card on the field up to a cost | `cast: vanish` |
| #51 Back Breaker | its Death destroys every backrow card | `death: wipe` |
| #62 Living Bomb | each turn it destroys every permanent of that player's with a Plague Counter | `cast: nova` |

## New motifs

One event no motif drew as a card coming back comes up on three Classic cards, so it got a shared
motif (`docs/polish/9-card-animations.md`, "Drawn in #2794") rather than a recipe per card.

| Motif | Reads | Classic cards |
|---|---|---|
| `recall` | addedToHand | #14 Shadowstep, #34 Ancient Acquisition, #47 Recurring Felinor (`trigger`) |
