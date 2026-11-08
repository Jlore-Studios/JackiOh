# Stinkyscape's card animations, part two (#2798)

Stinkyscape's pass over the second half of its animation table, `stinkyscape-2.json5`
(`stinkyscape-091` to the end, the set's `stinkyscape-t-*` tokens included;
`docs/polish/9-card-animations.md`, "A set's pass"): one row per card of the file, in its order.
`stinkyscape-2.test.ts` holds every row's name, type, entry and scale to the catalog and the table,
and the table to the pass's floor. Part one, `001` to `090`, is its own sub-issue of #2782.

- **Entry**: the card's entry in `stinkyscape-2.json5`: `none: <reason>`, `bespoke <recipe>`,
  `setPiece <key>`, or each hook's `<hook> <motif>[ <palette>]`, then `aura <look> <scope>[ <palette>]`,
  led by `signature` and `entrance <key>` where the entry names them (R3332000).
- **Scale**: the card's motif scale at intensity 1 (`motifScale`), from its cost and rarity alone; `—`
  for a `none`.
- **Census**: the "Suggested" cell of `cargo jackioh fx census` (#2788) for the card, run at
  `53e62cbe2` (200 seeded games and 19 forced ones), or `not in census` for the five tokens it never
  listed. Its families read damage, removals, counters, costs and returns only, so it knows neither
  #2786's motifs nor #2793's: where it says `none: no motif reads <event>`, the pick follows that
  event through the motif table's Reads column.
- **Why**: where the entry follows the census, and why it overrides it where it does.

`cast` and `cry` draw on the board as the card resolves. `death`, `trigger` and `activate`, and a
Trap's `cast`, plan nothing on the board until their hooks open a window (#2783); the `/dev/fx`
gallery plays every entry. Every Spell in the range has a `cast`, but two draw nothing at play yet, as
their play shows no event their motif reads: #126 Jlockeed's Permanent Procurement Lobbying (its play
adds a rider) and #145 Fortify Flesh (a hero's Armor gain sends no event).

| Card | Name | Type | Entry | Scale | Census | Why |
| --- | --- | --- | --- | --- | --- | --- |
| `stinkyscape-091` | The Propagation's Wasp | Unit | trigger conjure Beast/swarm | 1.08 | none: vanilla | trigger: the census never saw its after-attack hook fire; conjure reads the copy it summons; Beast/swarm for its wasps |
| `stinkyscape-092` | The Propagation's Hornet | Unit | trigger conjure Beast/swarm | 1.28 | none: vanilla | trigger: the census never saw its after-attack hook fire; conjure reads the copy it summons; Beast/swarm for its hornets |
| `stinkyscape-093` | Patcher Nanobots | Unit | death mend | 1.16 | vanish | death: the census's vanish is noise (fused pieces, other cards' exiles); its own Death is the healed, which mend (#2793) reads; its fuse is the Starship part's |
| `stinkyscape-094` | Tmack Trav | Unit | none: its second attack is combat, which the attack rows and keyword beats draw | — | none: vanilla | the census agrees (none: vanilla): Rush and its attack again are combat |
| `stinkyscape-095` | Faction: Vanguard Coalition | Spell | cast send | 1.28 | none: no motif reads addedToHand | cast: no census family reads addedToHand and shuffledIn, which send (#2793) reads |
| `stinkyscape-096` | Ocean Tsunami | Unit | cry send Hustle/ore | 1.45 | vanish | cry: the census's vanish is one exile in its window; its own effect fills the hand (addedToHand), which send reads; Hustle/ore for its Resources |
| `stinkyscape-097` | Asteroid Field | Spell | cast barrage | 1.16 | nova | kept: #2784's example; the census's nova reads the same hits, which barrage rains one by one |
| `stinkyscape-098` | Hull Booster | Unit | none: vanilla | — | none: vanilla | the census agrees: keywords only (Equip, Armor), which the keyword beats draw |
| `stinkyscape-099` | No-No Words | Spell | cast bolt Dark/curse | 1.24 | bolt | cast: the census agrees (bolt); Dark/curse for its hex |
| `stinkyscape-100` | AP Rounds | Unit | none: vanilla | — | none: no motif reads fused | keywords only (Equip, Pierce); the census's fused is other pieces' fuse, the Starship part's |
| `stinkyscape-101` | Overdrive Module | Unit | none: vanilla | — | none: vanilla | the census agrees: keywords only (Equip, Brittle, Reborn) |
| `stinkyscape-102` | Book of Nono-Words | Spell | cast send Nerd/ink | 1.35 | bolt | cast: the census's bolt is one damage in its window; its own effect is addedToHand, which send reads; Nerd/ink for its book |
| `stinkyscape-103` | Vxyl | Unit | cry send | 1.28 | none: no motif reads addedToHand | cry: no census family reads addedToHand and shuffledIn, which send (#2793) reads |
| `stinkyscape-104` | Reflective Plating | Unit | none: its reflection is standing, with no moment of its own; the damage rows draw the hits | — | none: no motif reads questProgressed | the census's questProgressed is another card's; its reflected damage is another hit's row |
| `stinkyscape-105` | Disintegrex | Unit | none: its Aura is standing, with no moment to draw; each exile it causes is its own row | — | none: vanilla | the census agrees (none: vanilla) |
| `stinkyscape-106` | Warpfield Amplifier | Unit | none: Starship part | — | none: vanilla | its only hook is When launched, which the Starship part draws (#2782) |
| `stinkyscape-107` | RR-5000 | Unit | trigger bless Tech/laser | 1.28 | bolt | trigger: the census's bolt is noise in its start-of-turn window (draws, fatigue); its own effect is buffed, which bless reads; Tech/laser for its gun |
| `stinkyscape-108` | Trila 3 | Unit | none: vanilla | — | none: no motif reads questProgressed | keywords only (Ranged, Windfury); the census's questProgressed is another card's |
| `stinkyscape-109` | xxFreshJack | Unit | bespoke freshJackComet; aura pixelcrawl tile Tech/glitch | 2.00 | none: no motif reads summoned | signature (R3407000), Mythic |
| `stinkyscape-110` | iKwzki | Unit | bespoke duoBond; aura duohalo tile Spirit | 1.86 | none: no motif reads marked | signature (R3407000), Mythic |
| `stinkyscape-111` | Shockwave447 | Unit | bespoke shockwaveLecture; aura arcflick tile Tech | 2.00 | none: no motif reads addedToHand | signature (R3407000), Mythic |
| `stinkyscape-112` | Job Application | Field Spell | bespoke jobInterview; aura steplights zone Hustle/cash | 1.51 | bolt | signature (R3407000) |
| `stinkyscape-112-1` | Paycheck | Spell | bespoke payday | 0.90 | bolt | signature (R3407000) |
| `stinkyscape-113` | A Very Dandy Lad | Unit | cry wipe | 1.65 | offering | cry: the census's offering reads its Tribute 2; its Aura exiles every cheap permanent as it lands (exiled 51), which wipe draws as one wave, a set-piece candidate |
| `stinkyscape-114` | Arctic Citadel | Unit | none: no motif reads fused | — | none: no motif reads fused | the census agrees: its Cry's fuse is `fused`, which no motif reads |
| `stinkyscape-115` | Scornboi | Unit | bespoke scornSweep; aura searchlight tile Dark/blood | 1.85 | offering | signature (R3407000) |
| `stinkyscape-116` | Megabase | Unit | bespoke giftShop; aura stationrings tile Tech/starfield | 1.51 | bolt | signature (R3407000) |
| `stinkyscape-117` | Core System | Spell | setPiece vortex | 1.55 | vanish | kept: #2800's `vortex` set piece (R2800000); the census's vanish reads its bounces, which the set piece draws as one beat |
| `stinkyscape-118` | Secure System | Spell | cast curse Tech/hologram | 1.55 | none: no motif reads buffed | cast: no census family reads buffed, its Attack set to 0, which curse reads; Tech/hologram for its security screen |
| `stinkyscape-119` | Unsecure System | Spell | cast bless Tech/glitch | 1.55 | none: no motif reads keywordGranted | cast: no census family reads buffed and keywordGranted, which bless reads; Tech/glitch for its hack |
| `stinkyscape-120` | Wild System | Spell | cast nova Tech/glitch | 1.55 | nova | cast: the census agrees (nova); Tech/glitch for its hack; a set-piece candidate |
| `stinkyscape-121` | System Schematic | Spell | cast send Tech/hologram | 1.35 | none: no motif reads addedToHand | cast: no census family reads addedToHand and shuffledIn, which send (#2793) reads; Tech/hologram for its schematic |
| `stinkyscape-122` | Vamp Rounds | Unit | none: vanilla | — | none: vanilla | the census agrees: keywords only (Equip, Lifesteal) |
| `stinkyscape-123` | Gokugains | Unit | none: its Nerf is secret (R202) | — | none: no motif reads turnAutoEnded | its Cry's Nerf is hidden from everyone until the card is played (R202), so nothing may show it |
| `stinkyscape-124` | Jlockeed's Shipbuilding Grant | Spell | cast send Tech/rocket | 1.16 | none: no motif reads addedToHand | cast: no census family reads addedToHand and shuffledIn, which send (#2793) reads; Tech/rocket for its ships |
| `stinkyscape-125` | Jlockeed's Spacerace Propaganda | Field Spell | cast conjure Tech/rocket; activate send Tech/rocket; aura ember zone Tech/rocket | 1.28 | none: vanilla | cast: a Field Spell's play is its own landing in the backrow (`summoned`), which the census leaves out; conjure draws it; activate: no census family reads addedToHand and shuffledIn, which send (#2793) reads; Tech/rocket for its ships |
| `stinkyscape-126` | Jlockeed's Permanent Procurement Lobbying | Spell | cast send Tech/rocket; trigger send Tech/rocket; aura arcane side Tech/rocket | 1.45 | none: no motif reads modifierChanged | cast: its play adds its start-of-turn rider (modifierChanged), which no motif reads, so send names what the rider does and draws nothing at play; trigger: the census never saw the rider fire; send reads its addedToHand; its aura stands over its rider (R2787000) |
| `stinkyscape-127` | Jlockeed's Siege Tank | Unit | trigger bolt Tech/laser | 1.36 | bolt | trigger: the census agrees (bolt); Tech/laser for its cannon |
| `stinkyscape-128` | Jlockeed's Yamato Cannon | Unit | none: vanilla | — | none: vanilla | the census agrees: keywords only (Ranged, Cleave) |
| `stinkyscape-129` | Jlockeed's Missile Pod | Unit | trigger volley Tech/rocket | 1.35 | bolt | trigger: the census's bolt saw its hits one by one, but its End of turn hits every enemy at once, a volley of missiles; Tech/rocket |
| `stinkyscape-130` | Jlockeed's Starship Retrofitting Program | Spell | cast send Tech/rocket | 1.35 | none: no motif reads addedToHand | cast: the Starship part's (#2782, R2801000): the Jlockeed card it adds, a Starship Piece from then on, is the census's addedToHand, which send reads; Tech/rocket for the ship |
| `stinkyscape-131` | Spacerace Stimulus | Field Trap | cast snap Hustle/cash | 1.08 | none: vanilla | cast: a Trap's cast is its firing, which plans nothing until firings open a window; snap reads its trapFired then; Hustle/cash for its stimulus |
| `stinkyscape-132` | Defensive Crystal | Unit | none: its Death's Armor shows no event; its fuse is the Starship part's | — | none: vanilla | the census's fused is its fuse, the Starship part's; a hero's Armor gain sends no event |
| `stinkyscape-133` | Ultra Capacitor | Unit | none: Starship part | — | none: vanilla | its only hook is When launched, which the Starship part draws (#2782) |
| `stinkyscape-134` | Felinor Containment Chamber | Unit | death conjure Beast/felinor | 1.08 | bolt | death: the census's lance is noise in its window; its own Death is summoned, which conjure reads; Beast/felinor for its Felinor |
| `stinkyscape-135` | Sacked | Trap | cast tether | 1.08 | none: vanilla | cast: a Trap's cast is its firing, which plans nothing until firings open a window; no census family reads held, the Unit stuffed into the Sack, which tether reads then |
| `stinkyscape-135-1` | Sack | Unit | none: token | — | not in census | a token the census never played; its release is the stuffed Unit's own summon row |
| `stinkyscape-136` | Magic Module | Unit | none: Starship part | — | none: vanilla | its only hook is When launched, which the Starship part draws (#2782) |
| `stinkyscape-137` | Starfleet Commander | Unit | signature; entrance fleetCommand | 1.85 | none: no motif reads awakened | signature (R3334004): it lands and orders its side's Dormant Starship up, a rocket bolt to the ship and a ring, a crown of gold and a shake there (its own with no ship); the launch its Cry makes is the Starship part's (#2782), and the Doctrine it casts plays its own entry |
| `stinkyscape-137-1` | Land Doctrine | Spell | cast nova Tech/rocket | 1.19 | nova | cast: the census agrees (nova): the Starship's hit; Tech/rocket for the ship |
| `stinkyscape-137-2` | Sea Doctrine | Spell | cast mend Tech/rocket | 1.19 | not in census | cast: a War Doctrine the census never played; its heal is healed, which mend reads; Tech/rocket for the ship |
| `stinkyscape-137-3` | Air Doctrine | Spell | cast barrage Tech/rocket | 1.19 | barrage | cast: the census agrees (barrage): its split damage; Tech/rocket for the ship |
| `stinkyscape-137-4` | Space Doctrine | Spell | cast send Tech/rocket | 1.19 | none: no motif reads modifierChanged | cast: its modifierChanged is its Flying rider; its hand of pieces is addedToHand, which send reads; Tech/rocket for the ship |
| `stinkyscape-138` | Pocket Peptides | Trap | cast snap | 1.08 | none: vanilla | cast: a Trap's cast is its firing, which plans nothing until firings open a window; snap reads its trapFired then |
| `stinkyscape-139` | Wormhole | Trap | cast negate Tech/starfield | 1.28 | negate | cast: the census agrees (negate): its firing counters, which negate reads once firings open a window; Tech/starfield for its wormhole |
| `stinkyscape-140` | Rush Token Launcher | Unit | cry conjure Tech/rocket | 1.28 | bolt | cry: the Starship part's (#2782, R2801000): each Launcher lands in a ring of rocket exhaust; its Activate, which a launched Starship inherits, waits for #2783 |
| `stinkyscape-141` | Fruit Launcher | Unit | cry conjure Tech/rocket | 1.28 | none: no motif reads locked | cry: the Starship part's (#2782, R2801000): each Launcher lands in a ring of rocket exhaust; its Activate, which a launched Starship inherits, waits for #2783 |
| `stinkyscape-142` | Jade Launcher | Unit | cry conjure Tech/rocket | 1.55 | bolt | cry: the Starship part's (#2782, R2801000): each Launcher lands in a ring of rocket exhaust; its Activate, which a launched Starship inherits, waits for #2783 |
| `stinkyscape-143` | Book Launcher | Unit | cry conjure Tech/rocket | 1.55 | bolt | cry: the Starship part's (#2782, R2801000): each Launcher lands in a ring of rocket exhaust; its Activate, which a launched Starship inherits, waits for #2783 |
| `stinkyscape-144` | Prepare For Liftoff | Trap | cast snap | 1.08 | none: vanilla | cast: a Trap's cast is its firing, which plans nothing until firings open a window; snap reads its trapFired then |
| `stinkyscape-145` | Fortify Flesh | Spell | cast bless | 1.45 | none: vanilla | cast: a hero's Armor gain sends no event, so the census sees nothing; ward folded into the rows (R2803000), so cast names bless, its nearest motif |
| `stinkyscape-146` | Flash Freeze | Spell | bespoke flashFreeze | 1.10 | none: no motif reads marked | bespoke: it freezes the whole board in one breath, and no motif reads `marked` |
| `stinkyscape-147` | IN33DH3 | Unit | bespoke dataPacket; aura aiwisp tile Tech/hologram | 1.40 | bolt | signature (R3407000) |
| `stinkyscape-147-1` | IN33DH3 Prime | Unit | bespoke primeConduct; aura crownpulse tile Tech/hologram | 1.19 | none: no motif reads summoned | signature (R3407000) |
| `stinkyscape-147-2` | Wardrum Blob | Field Spell | bespoke blobDrum; aura drumbeat zone Tech/hologram | 1.04 | vanish | signature (R3407000) |
| `stinkyscape-148` | Alteran Reach | Field Spell | bespoke starscapeJump; aura starpath zone Tech/starfield | 1.60 | bolt | signature (R3407000), Mythic |
| `stinkyscape-148-1` | Edge of the Universe | Spell | bespoke edgeFlee | 0.90 | not in census | signature (R3407000), Mythic |
| `stinkyscape-149` | Wildar | Spell | bespoke chaosOrb | 1.74 | none: no motif reads addedToHand | signature (R3407000) |
| `stinkyscape-150` | Call to Chaos (Stinky Edition) | Spell | bespoke stinkCloud | 1.85 | none: no motif reads addedToHand | signature (R3407000) |
| `stinkyscape-150-1` | Stinkyscape Golem | Unit | signature; entrance golemCrash; aura stinkvapour tile Toxic | 1.19 | not in census | signature (R3407000) |
| `stinkyscape-151` | Call to Chaos (Scape Edition) | Spell | bespoke tabletopTilt | 1.85 | vanish | signature (R3407000) |
| `stinkyscape-152` | Sanctum | Field Spell | bespoke sanctumEcho; aura pages zone Holy | 1.85 | none: vanilla | signature (R3407000) |
| `stinkyscape-153` | Stimmycheck | Spell | bespoke stimmyDrop | 1.40 | none: no motif reads manaChanged | signature (R3407000) |
| `stinkyscape-154` | Dice of Doom | Spell | setPiece diceOfDoom | 1.60 | bolt | set piece (R3338000): its d100 tumbles across the board onto its `rolled` result; on a 67 the board goes dark and every Terminus card's star rains into the hand, on a miss the die crumbles, and a Lucky reroll keeps a glint; its End of turn return plays its own `addedToHand` row |
| `stinkyscape-155` | Comet Crash | Field Spell | cast bind; death send Hustle/ore; aura ember zone | 1.28 | none: no motif reads locked | cast: no census family reads locked, its Crater, which bind reads; death: no census family reads addedToHand and shuffledIn, which send (#2793) reads; Hustle/ore for its Resources; its aura is #2787's example |
| `stinkyscape-156` | Catalyst Comet | Spell | cast send | 1.35 | none: no motif reads addedToHand | cast: no census family reads addedToHand and shuffledIn, which send (#2793) reads: it adds a card and shuffles itself in |
| `stinkyscape-157` | The Gauntlet | Field Spell | cast conjure; aura storm board | 1.35 | none: vanilla | cast: a Field Spell's play is its own landing in the backrow (`summoned`), which the census leaves out; conjure draws it; its Gauntlet spans the middle three lanes of both sides, so its aura covers the board |
| `stinkyscape-158` | Stratos Swarm | Spell | bespoke stratosSwarm | 1.45 | none: no motif reads summoned | bespoke: conjure's circle lands one Unit, where the Swarm fills the board |
| `stinkyscape-159` | Faction: ETHICS | Spell | cast send | 1.19 | none: no motif reads addedToHand | cast: no census family reads addedToHand and shuffledIn, which send (#2793) reads |
| `stinkyscape-160` | Faction: Golden Accord | Spell | cast send | 1.28 | none: no motif reads addedToHand | cast: no census family reads addedToHand and shuffledIn, which send (#2793) reads |
| `stinkyscape-161` | Faction: Pegasus | Spell | cast send | 1.28 | bolt | cast: the census's bolt is a split hit in its window; its own effect is addedToHand, which send reads |
| `stinkyscape-162` | Faction: Star-Jumper Core | Spell | cast send | 1.28 | none: no motif reads addedToHand | cast: no census family reads addedToHand and shuffledIn, which send (#2793) reads |
| `stinkyscape-163` | Swear Loyalities | Spell | cast vanish | 1.10 | vanish | cast: the census agrees (vanish): every card of another faction exiled |
| `stinkyscape-164` | The Syndicate | Field Spell | cast conjure Hustle/cash; trigger vanish Hustle/cash; aura arcane side Hustle/cash | 1.25 | vanish | cast: a Field Spell's play is its own landing in the backrow (`summoned`), which the census leaves out; conjure draws it; trigger: the census agrees (vanish): its trades' exiles; Hustle/cash for its trades |
| `stinkyscape-165` | The Trade Union | Field Spell | cast conjure Hustle/cash; aura bloom side Hustle/cash | 1.10 | none: vanilla | cast: a Field Spell's play is its own landing in the backrow (`summoned`), which the census leaves out; conjure draws it; Hustle/cash for its trades |
| `stinkyscape-166` | Scanning Probe Mk1 | Spell | bespoke probeScan | 1.08 | none: no motif reads addedToHand | bespoke: its scan is a radar sweep from its caster's hand, and a find pings on the board and flies into its owner's hand, a flash for a Radiant one (`probeScan`, R3337002); the other seat sees the sweep alone (R202) |
| `stinkyscape-167` | Scanning Probe Mk2 | Spell | bespoke probeScan | 1.28 | bolt | bespoke: Mk1's `probeScan` (R3337002); the census's bolt is one damage in its window, where its own effect is the scan's `addedToHand` |
| `stinkyscape-168` | Drone Hanger | Field Spell | cast conjure Tech/rocket; trigger conjure Tech/rocket; aura storm zone Tech/rocket | 1.36 | bolt | cast: a Field Spell's play is its own landing in the backrow (`summoned`), which the census leaves out; conjure draws it; trigger: the census's bolt is noise in its start-of-turn window; its own effect is summoned, which conjure reads; Tech/rocket for its drones |
| `stinkyscape-169` | Tiny Asteroid | Unit | death send Hustle/ore; activate bolt Hustle/ore | 1.08 | bolt | death: no census family reads addedToHand and shuffledIn, which send (#2793) reads; activate: the census agrees (bolt): the damage it costs; Hustle/ore for its Resources |
| `stinkyscape-170` | Medium Asteroid | Unit | death send Hustle/ore; activate bolt Hustle/ore | 1.08 | bolt | death: no census family reads addedToHand and shuffledIn, which send (#2793) reads; activate: the census agrees (bolt): the damage it costs; Hustle/ore for its Resources |
| `stinkyscape-171` | Large Asteroid | Unit | death send Hustle/ore; activate bolt Hustle/ore | 1.36 | bolt | death: no census family reads addedToHand and shuffledIn, which send (#2793) reads; activate: the census agrees (bolt): the damage it costs; Hustle/ore for its Resources |
| `stinkyscape-172` | Huge Asteroid | Unit | death send Hustle/ore; activate bolt Hustle/ore | 1.65 | bolt | death: no census family reads addedToHand and shuffledIn, which send (#2793) reads; activate: the census agrees (bolt): the damage it costs; Hustle/ore for its Resources |
| `stinkyscape-173` | Abandoned Spice Tower | Unit | death send Snack; activate bolt | 1.28 | bolt | death: no census family reads addedToHand and shuffledIn, which send (#2793) reads; Snack for its Spices; activate: the census agrees (bolt): the damage it costs |
| `stinkyscape-174` | Aerelret | Trap | cast conjure Tech/rocket | 1.65 | nova | cast: a Trap's cast is its firing, which plans nothing until firings open a window; the census's nova is the attack that goes on, while its own effect summons, which conjure reads; Tech/rocket for its ships |
| `stinkyscape-175` | Frost Module | Unit | none: no motif reads marked | — | none: vanilla | the census agrees: its Activate's freeze is `marked`, whose own row brands the card |
| `stinkyscape-176` | Preemptive Strike | Spell | bespoke preemptiveStrike | 1.62 | bolt | signature (R3334004): rings of fire and a red spark strike on its player's hero, then a red brand on the Dormant Starship it wakes; the launch is the Starship part's (#2782), and the Doctrine it casts plays its own entry |
| `stinkyscape-177` | Cloaking Module | Unit | none: vanilla | — | none: vanilla | the census agrees: keywords only (Equip, Cloaked, Immutable) |
| `stinkyscape-t-dormant-starship` | Dormant Starship | Unit | signature; entrance shipyardRollout | 1.74 | none: unseen | signature (R3334004): it rolls out of its shipyard in weld sparks and settles in a ring and dust at its foot, in Tech/rocket; its fuse and launch are the Starship part's (#2782) |
| `stinkyscape-t-drone-interceptor` | Drone Interceptor | Unit | none: token | — | bolt | a token never played: keywords only; the census's window is other cards' noise |
| `stinkyscape-t-drone-corvette` | Drone Corvette | Unit | none: token | — | bolt | a token never played: keywords only; the census's window is other cards' noise |
| `stinkyscape-t-shield-layer` | Drone Battleship (Shield Layer) | Unit | none: Battleship planner | — | not in census | a token the census never saw: its redirect, the dome's flare and plates and its break are `fx/battleship.ts`'s (R3336000), not a hook's |
| `stinkyscape-t-propagation-layer` | Drone Battleship (Propagation Layer) | Unit | none: Battleship planner | — | vanish | its Activates open a bay and launch the drone from it in Tech/rocket, which `fx/battleship.ts` plans (R3336000) from the public `activated` and `summoned`, so the conjure it had would draw a second summon over it |
| `stinkyscape-t-meltdown-layer` | Drone Battleship (Meltdown Layer) | Unit | none: Battleship planner | — | none: no motif reads addedToHand | a token: its End of turn self-hit pulses the core, its attacks fire from the reactor and its death is the blast, all `fx/battleship.ts`'s (R3336000); the census's addedToHand is the turn's draw |
| `stinkyscape-t-reknite` | Reknite | Spell | none: token | — | bolt | a token: its Armor is a modifier (modifierChanged), which its badge shows |
| `stinkyscape-t-korrelite` | Korrelite | Spell | cast bolt Hustle/ore | 0.90 | bolt | cast: the census agrees (bolt); Hustle/ore for its Resource |
| `stinkyscape-t-gellium` | Gellium | Spell | cast mend Hustle/ore | 0.90 | bolt | cast: the census's bolt is noise in its window; its own effect is healed, which mend (#2793) reads; Hustle/ore for its Resource |
| `stinkyscape-t-axnit` | Axnit | Spell | cast nova Hustle/ore | 0.90 | nova | cast: the census agrees (nova); Hustle/ore for its Resource |
| `stinkyscape-t-narcor` | Narcor | Spell | bespoke narcorDust | 0.90 | none: no motif reads addedToHand | signature (R3407000) |
| `stinkyscape-t-red-narcor` | Red Narcor | Spell | bespoke redNarcorHum | 0.90 | bolt | signature (R3407000), Mythic |
| `stinkyscape-t-anomaly-found` | Anomaly Found! | Spell | bespoke anomalyCall | 1.19 | vanish | bespoke: its lay is an Anomaly's set piece (`fx/anomaly.ts`, R3337003), which draws the census's exiles; `anomalyCall` gathers a void ring and stars where the rift tears |
| `stinkyscape-t-ai-01` | Escort Protocol | Unit | cry conjure Tech/hologram | 0.97 | none: no motif reads summoned | cry: no census family reads summoned, which conjure reads; Tech/hologram for its AI |
| `stinkyscape-t-ai-02` | Mining Subroutine | Spell | cast send Hustle/ore | 0.90 | none: no motif reads addedToHand | cast: no census family reads addedToHand and shuffledIn, which send (#2793) reads; Hustle/ore for its Resource |
| `stinkyscape-t-ai-03` | Route Optimizer | Spell | cast send Tech/hologram | 0.97 | none: no motif reads addedToHand | cast: no census family reads addedToHand and shuffledIn, which send (#2793) reads; Tech/hologram for its AI |
| `stinkyscape-t-ai-04` | Predictive Shield | Trap | none: token | — | none: vanilla | a token: no motif reads attackCancelled or marked, whose own rows play |
| `stinkyscape-t-ai-05` | Launch Sequencer | Spell | cast tithe Tech/rocket | 0.97 | none: no motif reads addedToHand | cast: the Starship part's (#2782, R2801000): its "Your Starship launch costs (1)" glints on its player's modifier badges and their Dormant Starship, which tithe reads from modifierChanged; Tech/rocket for the ship |
| `stinkyscape-t-ai-06` | Probe Relay | Spell | cast send Tech/hologram | 0.97 | none: no motif reads addedToHand | cast: no census family reads addedToHand and shuffledIn, which send (#2793) reads; Tech/hologram for its AI |
| `stinkyscape-t-ai-07` | Combat Subroutine | Unit | cry bless Tech/hologram | 1.04 | none: no motif reads keywordGranted | cry: no census family reads keywordGranted, which bless reads; Tech/hologram for its AI |
| `stinkyscape-t-ai-08` | Drone Uplink | Field Spell | cast conjure Tech/hologram; trigger conjure Tech/hologram; aura storm zone Tech/hologram | 1.04 | none: vanilla | cast: a Field Spell's play is its own landing in the backrow (`summoned`), which the census leaves out; conjure draws it; trigger: no census family reads summoned, which conjure reads; Tech/hologram for its AI |
| `stinkyscape-t-ai-09` | Honeypot | Trap | cast conjure Tech/hologram | 0.97 | none: vanilla | cast: a Trap's cast is its firing, which plans nothing until firings open a window; no census family reads summoned, which conjure reads then; Tech/hologram for its AI |
| `stinkyscape-t-ai-10` | Salvage Protocol | Unit | death send Tech/hologram | 0.97 | none: vanilla | death: no census family reads addedToHand and shuffledIn, which send (#2793) reads; Tech/hologram for its AI; its fuse is the Starship part's |

## Bespoke

Each built from the motif primitives (`fx/motifs/build.ts`) at the card's motif scale, with no new
rendering (`fx/cards/stinkyscape/`), drawn over the event rows (`keep`). The signatures' recipes (R3407000)
are in Signatures, at the end.

| Card | Recipe | Why bespoke | What plays |
|---|---|---|---|
| #110 iKwzki | `duoBond` | its Duo is a bond between two cards, and no motif reads `marked` | a gather on iKwzki (its caster's hero when it is off the board), a beam to the permanent it Duos with, and there Radiant rays, a ring and a burst, in the Duo mark's gold |
| #146 Flash Freeze | `flashFreeze` | it freezes the whole board in one breath, and no motif reads `marked` | at its first freeze a frost wave across both sides lane by lane into one ring over the board, and a sheen and a burst on each Unit it freezes, in the freeze mark's cyan |
| #158 Stratos Swarm | `stratosSwarm` | conjure's circle lands one Unit, where the Swarm fills the board | each Stratos streaking in on a rocket trail (`Tech/rocket`) from its caster's hero to the zone it fills, a burst, a ring and exhaust smoke there, one shake a play |
| #166 Scanning Probe Mk1, #167 Mk2 | `probeScan` | no motif reads a scan; a find is the start of an Anomaly (R3337002) | a radar sweep across the board from its caster's hand in Tech's starfield, on both seats; on its owner's a find's star ring and ping on the board, the card flying from it into the hand and, Radiant, rays, a prismatic burst and a shake there; the other seat's find plays its own row (R202) |
| Anomaly Found! | `anomalyCall` | its lay is a set piece of its own (R3337003), which draws the exiles and the landing | as its `boardReplaced` opens the rift, a void ring and a burst of stars over the middle of its caster's side |

R3407000 (#3407) gives the range's Legendaries and Mythics left to it a signature each: #109 xxFreshJack,
#110 iKwzki, #111 Shockwave447, #148 Alteran Reach, Edge of the Universe and Red Narcor a takeover look of their
own that draws the card's own picture filling the screen (`takeoverArtOf`), and every one an entrance or a
cast of its own, its text's beats and, a permanent, its own standing look on its tile or zone (the
Signatures section at the end). #154 Dice of Doom's yields to its own set piece (R3338000); the Dormant
Starship, #137 Starfleet Commander and #176 Preemptive Strike have theirs (R3334004, "The Starship parts"),
and the cards #3336 owns keep their planner's moments. Anomaly Found!, a token printed Mythic, yields its
takeover to the rift its play lays, as a Mythic yields to its own wipe (R3337003, R2800000), and none of
the three cards above is a signature (R3332000): the probes are Common and Rare, and the rift's hold is
longer than a Mythic Spell's signature budget.

## The Starship parts

The Starship's own moments are the Starship part's of #2782: the fuse, the launch, Launch Sequencer
`stinkyscape-t-ai-05`, #130 Jlockeed's Starship Retrofitting Program and the Launchers #140 to #143.
That part (R2801000) gives Launch Sequencer `cast tithe`, the Retrofitting Program `cast send` and the
Launchers `cry conjure`, all in Tech/rocket. Each other card whose only hook is one of them is
`none: Starship part` here: the three "When launched" pieces (#106 Warpfield Amplifier, #133 Ultra
Capacitor, #136 Magic Module). A Starship Piece's other hooks have entries here (#93 Patcher Nanobots' `death`,
#134 Felinor Containment Chamber's `death`, Salvage Protocol's `death`), and the War Doctrines a
launch casts (#137.1 to #137.4) each play their own `cast`.

The Starship's three Legendaries have signatures of their own (R3332000, R3334004;
`fx/cards/stinkyscape/starshipSignatures.ts` and `preemptiveStrike.ts`), their first beat beside the
Starship's moment, which still draws the launch that follows:

| Card | Entry | What plays |
|---|---|---|
| Dormant Starship | `entrance: "shipyardRollout"` | as it lands, weld sparks flash at three points of its hull, then it settles in a ring, dust at its foot and a shake, in Tech/rocket |
| #137 Starfleet Commander | `entrance: "fleetCommand"` | as it lands, a rocket bolt from its zone to its side's Dormant Starship, and there a ring, a crown of gold and a shake; with no Dormant Starship (its Cry does nothing), the ring, the gold and the shake are its own |
| #176 Preemptive Strike | `{ bespoke: "preemptiveStrike" }` (`keep`) | at its play, rings of fire on its player's hero and a red spark strike (Tech/laser) with a shake; at the Dormant Starship's awakening that starts its launch, a red brand (the red mark's) on the ship, its player's hero when the board cannot place it, with a ring and a softer shake |

## Set-piece candidates

Each takes a motif or a signature for now; a later part may give it a set piece.

| Card | Why | Its entry now |
|---|---|---|
| #113 A Very Dandy Lad | its Aura exiles every permanent that costs (2) or less as it lands | `cry: wipe` |
| #115 Scornboi | its Aura exiles every Unit but Humans and the Starship's as it lands | its signature `scornSweep` (R3407000) |
| #120 Wild System | every Unit fights every other until one is left | `cast: nova` |
| #150 Call to Chaos (Stinky Edition) | some of its ten random effects fill or freeze a whole side | its signature `stinkCloud` (R3407000) |

#117 Core System is #2800's `vortex` already, #151 Call to Chaos (Scape Edition) took its signature
`tabletopTilt` in place of `wipe` (R3407000), and the Anomaly Found! token's lay is the rift's set piece
(R3337003).

## Tokens given entries

Of the file's 32 tokens, 24 have an entry other than `none`: Paycheck, the four War Doctrines,
IN33DH3 Prime, Wardrum Blob, Edge of the Universe, the Stinkyscape Golem, Korrelite, Gellium, Axnit,
Narcor, Red Narcor, Anomaly Found! and the AI tokens Escort Protocol, Mining Subroutine, Route
Optimizer, Launch Sequencer, Probe Relay, Combat Subroutine, Drone Uplink, Honeypot and Salvage Protocol. The rest are keywords only, or show nothing a motif reads; the
Drone Battleship's three layers are `none: Battleship planner` (R3336000): their moments are
`fx/battleship.ts`'s, planned from public events and the layers the view draws, and no hook opens for
the Shield's guard, a break or the blast.

## New motifs

None: every event the range shows on three or more of its cards is read by a motif already (`send`,
`mend` and `radiance` came with Core's pass, #2793). `marked` comes up on #110, #146, #175 and Predictive
Shield, but each mark's own row brands its card, and the two cards whose mark is the moment take a
recipe of their own.
