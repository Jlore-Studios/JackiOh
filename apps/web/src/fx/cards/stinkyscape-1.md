# Stinkyscape's card animations, part one (#2797)

The first half of Stinkyscape's pass over its animation table, `stinkyscape-1.json5`
(`docs/polish/9-card-animations.md`, "A set's pass"): one row per card of the file, `stinkyscape-001` to
`stinkyscape-090` and the tokens listed among them, in catalog order. `stinkyscape-1.test.ts` holds every
row's name, type, entry and scale to the catalog and the table, and the table to the pass's floor.
`stinkyscape-2.json5`, from `stinkyscape-091` on, is part two's (#2798).

- **Entry**: the card's entry in `stinkyscape-1.json5`: `none: <reason>`, `bespoke <recipe>`,
  `setPiece <key>`, or each hook's `<hook> <motif>[ <palette>]`, then `aura <look> <scope>[ <palette>]`.
- **Scale**: the card's motif scale at intensity 1 (`motifScale`), from its cost and rarity alone; `—`
  for a `none`.
- **Census**: the "Suggested" cell of `cargo jackioh fx census` (#2788) for the card, run on this pass's
  branch after its merge of `53e62cbe2` (200 seeded games and 19 forced ones), or `not in census` for
  the one token it never played. Its families read damage, removals, counters, costs and returns only,
  and its vocabulary predates `send`, `mend` and `radiance` (#2793) and `hex` (#2797).
- **Why**: where the entry follows the census, and why it overrides it where it does.

Every hook draws on the board (R2797000): `cast` and `cry` as the card resolves, `death`, `trigger` and
`activate` from the hook's own `hookResolving` marker (R2783000), and a Trap's `cast` from its
`trapFired`. A hook no marker reports (an attack's, a Starship's launch) is `none`. Set-piece
candidates, each on a motif for now: `stinkyscape-004-5` 1v1 Final Destination, `stinkyscape-034` Drone
Core VIII and `stinkyscape-036` Drone Core X.

| Card | Name | Type | Entry | Scale | Census | Why |
| --- | --- | --- | --- | --- | --- | --- |
| `stinkyscape-001` | Freeze Ray | Spell | cast hex | 1.00 | none: no motif reads marked | cast: its Freezes are marks (`marked`), which hex (#2797) reads; the census's `none: no motif reads marked` predates hex |
| `stinkyscape-002` | Absolute Zero | Spell | cast hex Spirit | 1.28 | vanish | cast: its Freeze forever is a mark, which hex reads, in Spirit's cold blue; the census's families read damage, removals, counters, costs and returns only, so its vanish is noise in the window, not this card's effect |
| `stinkyscape-003` | Meteor | Spell | cast nova | 1.55 | volley | cast: kept as #2784's nova example; its damage tramples onto the adjacent Units at once, one blast rather than the census's fan of volley shots |
| `stinkyscape-004` | Olivian Expanse | Spell | bespoke terminusRift | 2.00 | none: no motif reads addedToHand | bespoke: a Mythic whose whole play is its Discover; `terminusRift` tears a void rift over the board that the Terminus card flies out of into the hand (R2797000); the census predates send |
| `stinkyscape-004-1` | Farewell | Spell | setPiece vortex | 1.19 | vanish | unchanged: its vortex set piece (R2800000) holds the board as it exiles every permanent, the census's vanish |
| `stinkyscape-004-2` | KY's Last Exam | Spell | cast bolt Nerd/chalk | 1.19 | bolt | cast: the census agrees (bolt), its damage at the enemy hero, in Nerd's chalk for the exam |
| `stinkyscape-004-3` | Event Horizon | Spell | none: its extra turn moves nothing on the board; no motif reads modifierChanged | — | none: no motif reads modifierChanged | the census agrees: its extra turn is a modifier, which no motif reads |
| `stinkyscape-004-4` | Last Word | Trap | cast snap | 1.19 | none: vanilla | cast: a Trap's cast plays from its `trapFired` (R2797000), which snap reads; the census's families read damage, removals, counters, costs and returns only |
| `stinkyscape-004-5` | 1v1 Final Destination | Trap | cast bind | 1.19 | none: vanilla | cast: its firing Locks every lane but one, which bind reads; the census's families read damage, removals, counters, costs and returns only; a set-piece candidate |
| `stinkyscape-004-6` | The Big Crunch | Spell | none: its effects resolve under each Call to Chaos it casts, whose own entry plays | — | not in census | not in census (never played): each Call to Chaos it casts resolves as its own play, under its own entry |
| `stinkyscape-005` | Masochism Unmasked | Field Trap | cast snap; aura ember zone | 1.19 | none: vanilla | cast: its reveal is a `trapFired`, which snap reads (R2797000); aura: an ember glow on its zone while it stands; the census's families read damage, removals, counters, costs and returns only |
| `stinkyscape-006` | Tylannt | Unit | none: its Cry completes Quest stages (questCompleted), which no motif reads | — | none: no motif reads questCompleted | the census agrees: no motif reads questCompleted, and at one card in this half it is under the three a new motif needs |
| `stinkyscape-007` | The Greatest Heist | Field Spell | cast send Hustle/cash; aura arcane zone Hustle/cash | 1.74 | none: no motif reads addedToHand | cast: its Cry Discovers a card (`addedToHand`), which send (#2793) reads, in Hustle's cash; aura: arcane in the same palette; the census predates send |
| `stinkyscape-008` | Conjure Rosemary Triscuit | Spell | cast send | 1.28 | none: no motif reads addedToHand | cast: it adds one card and shuffles another in, both of which send reads; the census predates send |
| `stinkyscape-008-1` | Rosemary Triscuit | Spell | cast mana | 0.90 | bolt | cast: a mana refresh (`manaChanged`), which mana reads; the census's families read damage, removals, counters, costs and returns only |
| `stinkyscape-009` | Ticketed | Field Spell | cast tether; aura void zone | 1.16 | none: no motif reads held | cast: its Cry holds a permanent beneath it (`held`), which tether (#2786) reads; aura: void on its zone; the census's vocabulary has no motif for held |
| `stinkyscape-010` | Otherworldly Endurance | Spell | cast send | 1.45 | none: no motif reads addedToHand | cast: it adds three cards to the hand, which send reads; the census predates send |
| `stinkyscape-010-1` | Inner Fire | Spell | cast bless | 0.97 | none: no motif reads buffed | cast: a Unit's Attack set up (`buffed`), which bless reads; the census's vocabulary has no motif for buffed |
| `stinkyscape-010-2` | Divine Spirit | Spell | cast bless | 1.04 | none: no motif reads buffed | cast: a Unit's Health doubled (`buffed`), which bless reads; the census's vocabulary has no motif for buffed |
| `stinkyscape-010-3` | Power Word Shield | Spell | cast bless | 0.90 | bolt | cast: +Health is a `buffed`, which bless reads; its draw plays its own row; the census's bolt is noise in the window |
| `stinkyscape-011` | Otherworldly Magic | Spell | cast send | 1.45 | none: no motif reads addedToHand | cast: it adds three cards to the hand, which send reads; the census predates send |
| `stinkyscape-011-1` | Frostbolt | Spell | cast bolt Spirit | 0.97 | bolt | cast: the census agrees (bolt), in Spirit's cold for the Freeze |
| `stinkyscape-011-2` | Arcane Missiles | Spell | cast barrage Nerd | 0.90 | bolt | cast: its Echo fires a random enemy again and again, hits landing one after another, so barrage over the census's bolt, in Nerd's arcane |
| `stinkyscape-011-3` | Flame Strike | Spell | cast nova | 1.04 | volley | cast: it hits every enemy Unit at once, one blast over their side, so nova over the census's volley |
| `stinkyscape-012` | Otherworldly Nightmare | Spell | cast send | 1.45 | none: no motif reads addedToHand | cast: it adds three cards to the hand, which send reads; the census predates send |
| `stinkyscape-012-1` | Shadow Word Death | Spell | cast bolt Dark/shadow | 0.97 | bolt | cast: the census agrees (bolt), in Dark's shadow |
| `stinkyscape-012-2` | Shadow Word Pain | Spell | cast scorch Dark/shadow | 0.90 | bolt | cast: the census says bolt; scorch keeps it apart from its pair Shadow Word Death, its target burning down where it stood, in Dark's shadow |
| `stinkyscape-012-3` | Psychic Scream | Spell | cast wipe | 1.12 | none: no motif reads shuffledIn | unchanged: #2784 gave it wipe, every permanent leaving the board at once; the census's families read damage, removals, counters, costs and returns only |
| `stinkyscape-013` | Asphyxiants Cloud | Spell | cast nova | 1.28 | lance | cast: it hits every enemy at once, so nova over the census's lance |
| `stinkyscape-014` | Arm of the Jempire | Unit | none: its trigger (a Taunt Unit of yours attacked) is an ally hook, which no hookResolving marks | — | none: vanilla | its trigger (one of your Taunt Units is attacked) runs on an attack's hooks, which no `hookResolving` marks (R2783000); the census's none agrees |
| `stinkyscape-015` | Arg Arg Defender | Unit | cry bless Physical/steel | 1.08 | none: no motif reads keywordGranted | cry: its +buff and Taunt to adjacent Units are `buffed` and `keywordGranted`, which bless reads, in Physical's steel; the census's vocabulary has no motif for keywordGranted |
| `stinkyscape-016` | Rod of Ages | Field Spell | cast conjure; trigger bless Physical/blade; aura holy zone | 1.73 | bolt | cast: a Field Spell's play is its own landing in the backrow (`summoned`), which the census leaves out; conjure draws it; trigger: its end-of-turn keyword grant (`keywordGranted`) is bless's, in Physical's blade; aura: holy on its zone |
| `stinkyscape-017` | Imprisoned Ship Smith | Unit | trigger conjure Physical/steel | 1.08 | bolt | trigger: its awakening's Whenever summons a Fiery Waraxe, which conjure reads, in Physical's steel; the census's bolt is noise in the window |
| `stinkyscape-018` | Imitated Instrumentality | Spell | bespoke magiBoot | 1.85 | none: no motif reads summoned | bespoke: a Legendary that boots three Field Spells; `magiBoot` beams each from the hero into its zone, staggered, in Tech's hologram (R2797000); the census predates conjure reading summoned |
| `stinkyscape-018-1` | MELCHIOR | Field Spell | activate bless Tech/hologram; aura arcane zone Tech/hologram | 0.97 | volley | activate: its Discovered keyword given to each Unit is bless's, in Tech's hologram; aura: arcane in the same palette; the census's volley is noise in the window |
| `stinkyscape-018-2` | BALTHASAR | Field Spell | activate send Tech/hologram; aura arcane zone Tech/hologram | 1.04 | vanish | activate: it adds a copy to the hand, which send reads, in Tech's hologram; aura: arcane in the same palette; the census's vanish is noise in the window |
| `stinkyscape-018-3` | CASPER | Field Spell | activate hex Tech/hologram; aura arcane zone Tech/hologram | 1.12 | bolt | activate: its Allure is a mark (`marked`), which hex reads, in Tech's hologram; aura: arcane in the same palette; the census's bolt is noise in the window |
| `stinkyscape-019` | Imprisoned Cosmality | Unit | trigger barrage Tech/starfield | 1.16 | barrage | trigger: the census agrees (barrage), its awakening's split damage raining one hit after another, in Tech's starfield |
| `stinkyscape-020` | Tnack Trav | Unit | none: its Cry copies a card's text, which moves nothing on the board | — | none: vanilla | the census agrees: its Cry copies a card's text, which moves nothing on the board |
| `stinkyscape-021` | Moonberry | Spell | cast send | 1.08 | bolt | cast: it adds a card to the hand, which send reads; the census's bolt is noise in the window |
| `stinkyscape-022` | Gatinsir | Trap | cast conjure Tech/rocket | 1.65 | none: vanilla | cast: its firing summons Warp Disruptors and Phalanxes, which conjure reads, in Tech's rocket; the census's families read damage, removals, counters, costs and returns only |
| `stinkyscape-023` | Warp Disruptor | Field Spell | cast shift Tech/starfield; aura storm board Tech/starfield | 1.19 | bolt | cast: its play forces Units out of Defense Position (`positionSwitched`, in the census's play window), which shift reads, in Tech's starfield; aura: storm over the board, as its Aura holds every Unit; the census's bolt is its Whenever's attacks |
| `stinkyscape-024` | Warp Disruptor II | Field Spell | cast shift Tech/starfield; aura storm board Tech/starfield | 1.28 | bolt | cast: as Warp Disruptor, shift for the `positionSwitched` of its play, in Tech's starfield; aura: storm over the board; the census's bolt is its Whenever's attacks |
| `stinkyscape-025` | KY's Tricknology | Spell | cast conjure Tech/hologram | 1.45 | none: no motif reads summoned | cast: it summons Starship Pieces, which conjure reads, in Tech's hologram; the census predates conjure reading summoned |
| `stinkyscape-026` | ZolarKeth | Unit | cry conjure; activate conjure Tech/rocket | 1.85 | none: no motif reads summoned | cry: its University or Activate choice summons, which conjure reads; activate: the Starship Piece it summons, in Tech's rocket; the census predates conjure reading summoned |
| `stinkyscape-027` | Drone Core I | Spell | cast conjure Tech/rocket | 1.00 | vanish | cast: it summons a Drone Interceptor, which conjure reads, in Tech's rocket; the census's vanish is noise in the window |
| `stinkyscape-028` | Drone Core II | Spell | cast conjure Tech/rocket | 1.08 | vanish | cast: it summons a Drone Interceptor, which conjure reads, in Tech's rocket; the census's vanish is noise in the window |
| `stinkyscape-029` | Drone Core III | Spell | cast conjure Tech/rocket | 1.16 | none: no motif reads summoned | cast: it summons Drone Interceptors, which conjure reads, in Tech's rocket; the census predates conjure reading summoned |
| `stinkyscape-030` | Drone Core IV | Spell | cast conjure Tech/rocket | 1.36 | vanish | cast: it summons Drone Interceptors, which conjure reads, in Tech's rocket; the census's vanish is noise in the window |
| `stinkyscape-031` | Drone Core V | Spell | cast conjure Tech/rocket | 1.36 | none: no motif reads summoned | cast: it summons a Drone Corvette, which conjure reads, in Tech's rocket; the census predates conjure reading summoned |
| `stinkyscape-032` | Drone Core VI | Spell | cast conjure Tech/rocket | 1.65 | none: no motif reads summoned | cast: it summons Drone Interceptors, which conjure reads, in Tech's rocket; the census predates conjure reading summoned |
| `stinkyscape-033` | Drone Core VII | Spell | cast conjure Tech/rocket | 1.65 | vanish | cast: it summons a Corvette and Interceptors, which conjure reads, in Tech's rocket; the census's vanish is noise in the window |
| `stinkyscape-034` | Drone Core VIII | Spell | cast conjure Tech/rocket | 2.00 | bolt | cast: it fills the board with Drone Interceptors, which conjure reads, in Tech's rocket; the census's bolt is noise in the window; a set-piece candidate |
| `stinkyscape-035` | Drone Core IX | Spell | cast conjure Tech/rocket | 2.00 | bolt | cast: it summons Drone Corvettes, which conjure reads, in Tech's rocket; the census's bolt is noise in the window |
| `stinkyscape-036` | Drone Core X | Spell | bespoke battleshipWarp | 2.00 | lance | bespoke: the Mythic top of the Drone Core line; `battleshipWarp` warps the Drone Battleship in from the board's top edge in Tech's starfield (R2797000); the census's lance reads other events in its window; a set-piece candidate |
| `stinkyscape-037` | Kwzka | Unit | death hex; trigger hex | 1.74 | vanish | death: its Death lifts every Allure (`marked`, removed), which hex reads; trigger: the Whenever that Allures a cheaper enemy Unit marks it, which hex reads; the census's vanish is noise in the window |
| `stinkyscape-038` | Shield Amplifier | Unit | none: its trigger (it survives an attack) runs on afterAttack, which no hookResolving marks | — | none: vanilla | its trigger (it survives an attack) runs on an attack's hooks, which no `hookResolving` marks (R2783000); the census's none agrees |
| `stinkyscape-039` | The Narcor Project | Field Spell | cast conjure; trigger send Hustle/ore; aura toxic zone | 1.74 | vanish | cast: a Field Spell's play is its own landing in the backrow (`summoned`), which the census leaves out; conjure draws it; trigger: its end-of-turn Narcor (`addedToHand`) is send's, in Hustle's ore; aura: toxic on its zone; the census's vanish is noise in the window |
| `stinkyscape-040` | Poison Capsule | Unit | cry tally; trigger nova | 1.36 | bolt | cry: its Plague Counters are `counterChanged`, which tally reads; trigger: its end-of-turn damage hits every enemy at once, so nova over the census's bolt |
| `stinkyscape-041` | Drone Sympathizer | Spell | cast send Tech/rocket | 1.08 | vanish | cast: it adds Drone Core cards to the hand, which send reads, in Tech's rocket; the census's vanish is noise in the window |
| `stinkyscape-042` | Core Researcher | Unit | cry send | 1.28 | none: no motif reads addedToHand | cry: it adds Radiant Drone Core cards to the hand, which send reads; the census predates send |
| `stinkyscape-043` | Healing Capsule | Unit | trigger mend | 1.19 | bolt | trigger: its end-of-turn heals (`healed`) are mend's (#2793); the census's bolt is noise in the window |
| `stinkyscape-044` | Great Replacement Theory | Spell | cast send | 1.86 | bolt | cast: it adds a Drone Core I to the hand, which send reads; the census's bolt is noise in the window |
| `stinkyscape-045` | Drone Commander | Unit | cry send | 1.65 | none: no motif reads addedToHand | cry: it adds discounted Drone Core cards to the hand, which send reads; the census predates send |
| `stinkyscape-046` | Circadian Halfcycler | Field Spell | cast conjure; trigger conjure Tech/rocket; aura storm lane | 1.45 | bolt | cast: a Field Spell's play is its own landing in the backrow (`summoned`), which the census leaves out; conjure draws it; trigger: its end-of-turn Drone Interceptor is conjure's, in Tech's rocket; aura: storm on its lane; the census's bolt is noise in the window |
| `stinkyscape-047` | Circadian Fullcycler | Field Spell | cast conjure; trigger conjure Tech/rocket; aura storm side | 1.55 | none: vanilla | cast: a Field Spell's play is its own landing in the backrow (`summoned`), which the census leaves out; conjure draws it; trigger: its end-of-turn Drone Corvette is conjure's, in Tech's rocket; aura: storm over its side, as the AI plays each of its turns; the census's none is its play window |
| `stinkyscape-048` | Ultralight Chassis | Unit | none: vanilla | — | none: vanilla | the census agrees: keywords only, which the keyword beats draw |
| `stinkyscape-049` | Spacemine | Unit | death volley | 1.16 | bolt | death: its Death hits several random enemies at once, a fan of shots, so volley over the census's bolt |
| `stinkyscape-050` | Beam v3 | Unit | activate lance Tech/laser | 1.28 | bolt | activate: its Cleave and Pierce run the length of the target's lane, so lance over the census's bolt, in Tech's laser |
| `stinkyscape-051` | Repeater v3 | Unit | activate bolt Tech/laser | 1.28 | bolt | activate: the census agrees (bolt), in Tech's laser |
| `stinkyscape-052` | Cannon v3 | Unit | activate bolt Tech/rocket | 1.28 | bolt | activate: the census agrees (bolt), in Tech's rocket |
| `stinkyscape-053` | Flak v3 | Unit | activate volley Tech/rocket | 1.28 | bolt | activate: it hits each enemy Unit at once, so volley over the census's bolt, in Tech's rocket |
| `stinkyscape-054` | Beam v4 | Unit | activate lance Tech/laser | 1.55 | bolt | activate: as Beam v3, lance over the census's bolt, in Tech's laser |
| `stinkyscape-055` | Repeater v4 | Unit | activate bolt Tech/laser | 1.55 | bolt | activate: the census agrees (bolt), in Tech's laser |
| `stinkyscape-056` | Cannon v4 | Unit | activate bolt Tech/rocket | 1.55 | bolt | activate: the census agrees (bolt), in Tech's rocket |
| `stinkyscape-057` | Flak v4 | Unit | activate barrage Tech/rocket | 1.55 | barrage | activate: the census agrees (barrage), its two waves landing one by one, in Tech's rocket |
| `stinkyscape-058` | Beam v5 | Unit | activate lance Tech/laser | 1.85 | bolt | activate: as Beam v3, lance over the census's bolt, in Tech's laser |
| `stinkyscape-059` | Repeater v5 | Unit | activate bolt Tech/laser | 1.85 | bolt | activate: the census agrees (bolt), in Tech's laser |
| `stinkyscape-060` | Cannon v5 | Unit | activate bolt Tech/rocket | 1.85 | bolt | activate: the census agrees (bolt), in Tech's rocket |
| `stinkyscape-061` | Flak v5 | Unit | activate barrage Tech/rocket | 1.85 | barrage | activate: the census agrees (barrage), its three waves landing one by one, in Tech's rocket |
| `stinkyscape-062` | Miner v1 | Unit | activate send Hustle/ore | 1.00 | none: no motif reads fused | activate: the Resource it gains is added to the hand, which send reads, in Hustle's ore; the census's `fused` is its own Equip as it plays |
| `stinkyscape-063` | Miner v2 | Unit | activate send Hustle/ore | 1.08 | none: vanilla | activate: the Resource it gains is send's, in Hustle's ore; the census saw only its play |
| `stinkyscape-064` | Miner v3 | Unit | activate send Hustle/ore | 1.28 | none: vanilla | activate: the Resource it gains is send's, in Hustle's ore; the census saw only its play |
| `stinkyscape-065` | Miner v4 | Unit | activate send Hustle/ore | 1.55 | none: vanilla | activate: the Resource it gains is send's, in Hustle's ore; the census saw only its play |
| `stinkyscape-066` | Miner v5 | Unit | activate send Hustle/ore | 1.85 | none: vanilla | activate: the Resource it gains is send's, in Hustle's ore; the census saw only its play |
| `stinkyscape-067` | Tractor Beam | Unit | trigger send Hustle/ore | 1.19 | bolt | trigger: its start-of-turn Resource is send's, in Hustle's ore; the census's bolt is noise in the window |
| `stinkyscape-068` | Stratos | Unit | none: vanilla | — | none: vanilla | the census agrees: keywords only |
| `stinkyscape-069` | Falcon | Unit | none: vanilla | — | none: vanilla | the census agrees: keywords only |
| `stinkyscape-070` | Z'Zoth | Unit | cry rise | 1.85 | rise | cry: the census agrees (rise), Units summoned back out of the graveyard |
| `stinkyscape-071` | A'Zoth | Unit | cry rise | 1.85 | rise | cry: the census agrees (rise), Units summoned back out of the graveyard |
| `stinkyscape-072` | Paragon Mk1 | Unit | none: vanilla | — | none: vanilla | the census agrees: keywords only |
| `stinkyscape-073` | Paragon Mk2 | Unit | none: vanilla | — | none: vanilla | the census agrees: keywords only |
| `stinkyscape-074` | Paragon-X | Unit | cry offering | 2.00 | offering | cry: the census agrees (offering), its Tribute's Units sinking as a cost |
| `stinkyscape-075` | Badger | Unit | none: its trigger (When this attacks) is a whenAttacks hook, which no hookResolving marks | — | none: vanilla | its trigger (When this attacks) runs on an attack's hooks, which no `hookResolving` marks (R2783000); the census's none agrees |
| `stinkyscape-076` | Honey Badger | Unit | none: its trigger (When this attacks) is a whenAttacks hook, which no hookResolving marks | — | none: vanilla | its trigger (When this attacks) runs on an attack's hooks, which no `hookResolving` marks (R2783000); the census's none agrees |
| `stinkyscape-077` | Yuma | Unit | none: its Cry schedules a trade offer (a modifier), which no motif reads | — | bolt | its Cry schedules a trade offer, a modifier no motif reads; the census's bolt is noise in the window |
| `stinkyscape-078` | Ozark | Unit | cry transmute Tech/rocket | 1.55 | none: no motif reads fused | cry: its Equip fuses a Miner into it (`fused`), which transmute reads from #2797, in Tech's rocket; the census predates that |
| `stinkyscape-079` | Orca | Unit | cry transmute Tech/rocket | 1.65 | none: no motif reads fused | cry: its chosen Miner's Equip is a `fused`, which transmute reads, in Tech's rocket; the census predates that |
| `stinkyscape-080` | Galaxy | Unit | cry transmute Tech/rocket | 1.85 | offering | cry: its Equips are `fused`, which transmute reads, in Tech's rocket; the census's offering is its Tribute, whose own rows play |
| `stinkyscape-081` | Radix | Unit | cry transmute Tech/laser | 1.36 | none: no motif reads fused | cry: its Weapons' Equip is a `fused`, which transmute reads, in Tech's laser; the census predates that |
| `stinkyscape-082` | Phalanx | Unit | cry transmute Tech/laser | 1.65 | bolt | cry: its Weapons' Equip is a `fused`, which transmute reads, in Tech's laser; the census's bolt is noise in the window |
| `stinkyscape-083` | Halo | Unit | cry transmute Tech/laser | 1.65 | none: no motif reads fused | cry: its Weapons' Equip is a `fused`, which transmute reads, in Tech's laser; the census predates that |
| `stinkyscape-084` | Power Cell | Spell | cast send | 1.08 | none: no motif reads shuffledIn | cast: it shuffles Metal Scraps into the deck (`shuffledIn`), which send reads; the census predates send |
| `stinkyscape-084-1` | Metal Scraps | Spell | cast send Hustle/ore | 0.97 | none: no motif reads addedToHand | cast: its cast-on-draw Resource is added to the hand, which send reads, in Hustle's ore; the census predates send |
| `stinkyscape-085` | Prototype-X1 | Unit | cry transmute Tech/hologram | 1.85 | offering | cry: its Weapons' Equip is a `fused`, which transmute reads, in Tech's hologram; the census's offering is its Tribute, whose own rows play |
| `stinkyscape-086` | Prototype-X2 | Unit | cry transmute Tech/hologram | 2.00 | offering | cry: its Weapons' Equip is a `fused`, which transmute reads, in Tech's hologram; the census's offering is its Tribute, whose own rows play |
| `stinkyscape-087` | Siva 1 | Unit | cry hex Beast/felinor | 1.62 | none: vanilla | cry: its Allure is a mark (`marked`), which hex reads, in Beast's felinor; the census's none is its play window |
| `stinkyscape-088` | Siva 2 | Unit | none: an Aura with no hook | — | bolt | an Aura with no hook: nothing to draw as it plays; the census's bolt is noise in the window |
| `stinkyscape-089` | Spice | Spell | cast send Hustle/ore | 1.19 | none: no motif reads addedToHand | cast: its Discover adds a card to the hand, which send reads, in Hustle's ore; the census predates send |
| `stinkyscape-090` | After Burner | Unit | none: its one hook, When launched, is the Starship's own moment (#2782's Starship part) | — | none: vanilla | its one hook, When launched, is the Starship's own moment, left to #2782's Starship part; the census's none agrees |
