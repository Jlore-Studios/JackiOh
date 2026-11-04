# Card animations: the motif language

Most cards should have a signature animation, and almost none should need code of their own to get
one. This doc is the shared vocabulary every card-animation pass (#2782 and its parts) uses: a card
names a **motif** in its set's table, and the motif draws it in the card's palette at the card's
scale. It is presentation only (CLAUDE.md rule 7) and its ruling is R2784000.

#2782's own "design language" section is the source of this doc. That text is not in the
repository yet, so this file is written from #2784's terms; its full motif list (27 names) is to be
pasted in by a person, and each name then joins `FX_MOTIF_NAMES` (see "Open" at the end).

## Tiers

| Tier | Table entry | What plays | Who draws it |
|---|---|---|---|
| none | `"none: <reason>"` | the card's event rows alone (`ANIMATIONS`' `fx` recipes, the flavor's cast, impact, entrance and exit) | nothing to draw |
| motif | `{ <hook>: { motif, palette?, row? } }` | a shared motif from `fx/motifs/`, on top of or in place of the event rows | the motif's planner, shared by every card that names it |
| bespoke | `{ bespoke: "<recipe>" }` | a recipe of the card's own, in `fx/cards/<set>/` | the card's own file |
| set piece | `{ setPiece: "<key>" }` | a board clear's set piece: its hook's removals as one wipe the queue holds for one capped, skippable beat (R2785000, R2800000) | `fx/setPieces.ts`, its board motion `game/board.css` and its sound bed `audio/setPieceBeds.ts` |

Every catalog card starts as `"none: pending"` and moves up a tier in its set's pass. A bespoke
recipe is kept for the few cards no motif can say (the seven of the former `CARD_FX` table: #21
Hinder, #27 Blood Ridden Glowy Jelly Bean, Classic+ #24 Crushing Walls and the marquee entrances of
BOOM! Big Max, Nature Titan and Spell Tyrant, and #100 Ceaseless Void's, which its set piece now
names, below), and the Flourishes of the set passes, each built from the motifs' cue helpers
(`fx/motifs/build.ts`): Meditative #39.1 Auspicious Rock's `jadeCrack`, #32 Spiritually 中国's
`cnSweep` and Meditative+ #21 Harmony Spirit's `harmonyWave` (#2796, `fx/cards/meditative/`,
`fx/cards/meditativeplus/`; their per-card table is `fx/cards/meditative.md`), and Classic's (#2794,
R2794000, `fx/cards/classic/`, over its events' own rows; its per-card table is `fx/cards/classic.md`):
#3 Book of Heal's `pageMend` (pages open over the caster's hero, a sparkle flies to what it heals, a
Holy mend lands there), #30 Recycle's `recycleLoop` (the graveyard loops into the deck in Nature's
leaves, once a play) and #85 King Wagtoggle's `crownSwap` (the two decks cross to each other's pile
under a gold crown).

Core's pass (#2793) adds three flourishes, each built from the motif primitives at the card's motif
scale (`fx/cards/core/flourish.ts`) and drawn over the event rows (`keep`):

| Card | Recipe | Why bespoke |
|---|---|---|
| #83 Transmogulate | `legendaryTide` | its transforms land in a hand and a deck the board does not show: Legendary rays over the caster's Deck and hand piles at the first, a sheen on each hand card the viewer reads |
| #97 Zephrys | `perfectWish` | its "perfect" card blows in from the far side: a star from the other hero into the caster's hand pile in Mythic rays |
| #99 Craft a Card | `fusionForge` | its Discovered Units fuse off the board and no motif reads `fused`: two halves forged at the board's centre, the card flying on into the hand |

Classic+'s pass (#2795, R2795000) adds three Flourishes beside Crushing Walls, each for a
card whose one event no motif reads: #35 Rollback (`rollbackSweep`: an arcane ring runs back from lane 5
to lane 1 on the sides it rolls back, then the heroes glint), #41 KY's Constant (`constantStamp`: the new
number chalked onto the card the seat reads) and #56 Book of Pain (`inkTear`: each discarded card torn
out in ink, the first with a ring and a shake). A Flourish is built from the motif primitives (the
phases, the look, the card's scale), keys on its readable play alone and keeps its events' rows.

Stinkyscape's first pass (#2797, R2797000) adds three more, in `fx/cards/stinkyscape/`, the same way:

| Card | Recipe | Why bespoke |
|---|---|---|
| #4 Olivian Expanse | `terminusRift` | a Mythic whose whole play is one Discover: a void rift tears open over the board's centre at its first card added, and the card flies out of it into the caster's hand |
| #18 Imitated Instrumentality | `magiBoot` | its three Field Spells come online one after another: a hologram beam from the caster's hero into each zone, booting with a ring and a burst, `FX_MAGI_STAGGER` of the entry apart within one |
| #36 Drone Core X | `battleshipWarp` | the Mythic top of the Drone Core line: stars gather where the Drone Battleship will stand and it streaks in from the board's top edge (`FX_WARP_FROM`), landing in exhaust |

Stinkyscape's pass, part two (#2798), adds three more, in `fx/cards/stinkyscape/` on the same shared
spec (`fx/cards/core/flourish.ts`); the two mark flourishes draw in the colors of the mark the engine
sends (`stinkyscape/markLook.ts`), as its brand row does:

| Card | Recipe | Why bespoke |
|---|---|---|
| #110 iKwzki | `duoBond` | its Duo is a bond between two cards and no motif reads `marked`: a beam from iKwzki (its caster's hero when it is off the board) to the permanent it Duos with, Radiant rays there, in the Duo mark's gold |
| #146 Flash Freeze | `flashFreeze` | it freezes the whole board in one breath and no motif reads `marked`: at its first freeze a frost wave across both sides lane by lane into one ring over the board, a sheen and a burst on each Unit frozen, in the freeze mark's cyan |
| #158 Stratos Swarm | `stratosSwarm` | conjure's circle lands one Unit, where the Swarm fills the board: each Stratos streaking in on a `Tech/rocket` trail from its caster's hero to the zone it fills, one shake a play |

Training Grounds' pass (#2799) adds three more on the same terms (`fx/cards/traininggrounds/`):

| Card | Recipe | Why bespoke |
|---|---|---|
| #16 Celestial Caller | `celestialCall` | its Cry calls a Sun and a Moon at once, in two looks one motif's palette cannot hold: a beam from the Caller into each, landing in Holy's radiant sunburst for the Sun and Spirit's shaft of moonlight for the Moon |
| #34 Dino-size | `dinoStomp` | no motif reads `numberChanged`, its one event, and no other card of the set shows one: rising arrows over the unit set to 10/10, then a stomp of dust at its foot and a heavy shake |
| #40 Goetzmann Game Design | `gameDesign` | the set's Mythic, its designer's card, which `send` undersells: a card drafted at the board's centre in Mythic rays and Nerd's pixel sub-look, dealt into the caster's hand pile |

## The motifs

A motif is a pure planner, `(spec, entry, view, env) => FxCue[]`, built from the cue kinds the effects
layer already draws (`fx/types.ts`: bursts, shots, rings, splats, cracks, sheens, shakes) and the
builders in `fx/build.ts` and `fx/motifs/build.ts`. It adds no rendering.

Drawn so far, the damage and removal family (#2784):

| Motif | Reads | Replaces | What it looks like |
|---|---|---|---|
| `bolt` | damage, destroyed | damage | a gather at the source, one glowing bolt into the first target, a hit on every target, a ring lingering on the first |
| `volley` | damage, destroyed | damage | one gather, then a fan of bolts that leave together, one per target up to `FX_MOTIF_VOLLEY_MAX` a play |
| `barrage` | damage, destroyed | damage | bolts raining in one after another, `FX_MOTIF_BARRAGE_STEP` of D apart, at most `FX_MOTIF_BARRAGE_MAX` one by one, never past linger |
| `chain` | damage, destroyed | damage | one bolt hopping target to target, each hop an equal share of travel to linger, on from the play's last hit when the queue split the run |
| `lance` | damage, destroyed, exiled | damage | **JackiOh's lane:** a beam down the target's whole lane, from the far side's backrow through both unit rows into its own side's backrow, every zone of the lane flaring as it lands, one beam a lane a play (a later hit in that lane flares it again); a target in no lane (a hero) takes bolt's cues |
| `nova` | damage, destroyed | damage | a blast gathering at one centre (its one target, the middle of the one side all targets are on, else the board) and bursting over every target at once |
| `scorch` | damage, destroyed | damage | each target heats, flares and burns down to embers and smoke where it stood |
| `wipe` | destroyed, exiled | nothing | at the play's first removal a wave rolls across both sides lane by lane into one ring over the board; every card removed flares where it stood |
| `vanish` | exiled, bounced | exiled | each card drawn in, glinting, and gone in a soft flare; no shake |
| `negate` | countered | countered | a ring closes on the countered card, which shatters and fizzles out in smoke |

Drawn in #2786, life and stats, board, cards and mana, and traps, on the same four phases
and three parameters:

| Motif | Reads | Replaces | What it looks like |
|---|---|---|---|
| `drain` | healthLost, healthSet | healthLost, healthSet | a liquid stream from the victim to the gainer (the entry's healed target, else the resolving play's hero), pooling where it lands; the loss's number only in place of the row |
| `bless` | buffed, upgraded, keywordGranted | all three | arrows up, a sheen and a burst over a card made better |
| `curse` | degraded, buffed, keywordGranted | all three | arrows down and a void burst over a card made worse |
| `ward` | damageAbsorbed, divineShieldLost | both | Armor's full shield over a hit it took whole (under `keep` a burst, the armor row drawing the flash); gold and shards where a Divine Shield broke |
| `conjure` | summoned | nothing | a summoning circle where the card arrives: a gather, a landing and an afterglow |
| `rise` | summoned, animated, awakened | nothing | the card rising out of its zone, from below to where it stands |
| `offering` | destroyed | nothing | the unit sinking under a glyph in its lane: a cost paid, never a kill |
| `shift` | swapped, rotated, positionSwitched, flickered | nothing | the path of every unit that moved; a board swap or a rotation walks both rows, a rotation along R14's ring from the resolving play's seat |
| `bind` | locked, unlocked | both | frost and chains over a Locked unit; gold shards when it releases |
| `tether` | stolen, controlChanged, held | nothing | an arcane line from where the card was to whose it is now |
| `mirror` | transformed | nothing | a translucent double peeling off the source toward the copy |
| `transmute` | transformed, fused, upgraded, degraded | all four | smoke and sparks as one card becomes another, landing on the card a fusion makes (an Equip fusing into its host, #2797), with a ring and rays for an upgrade and a crack for a degrade (#2908) |
| `flight` | drawn | drawn | a card back flying from the library into the hand, landing with its sparkle |
| `glimpse` | predicted, secretRevealed | nothing | a sheen and a small flare on the hero's badge |
| `mana` | manaChanged, manaSpent | manaChanged | one sparkle per crystal gained spread across Travel to Land, landing together; a soft pulse where a spend drained |
| `tithe` | costChanged, modifierChanged | costChanged | a glint on the cost gem of each changed hand card; by count on backs for cards the viewer may not read (R202); a price added for a player glints on their modifier badges and on their Dormant Starship, whose launch it may price (R2801000) |
| `tally` | counterChanged, questProgressed | nothing | a sparkle tick where the card stands |
| `snap` | trapFired | trapFired | a tight ring on the trap's own zone, then its reveal: in the card's flavor when the view names it, else the entry palette (R154, R202); a firing again is a short flare |

Drawn in #2793, Core's pass, for three events no motif read that come up on three or more Core cards
each:

| Motif | Reads | Replaces | What it looks like |
|---|---|---|---|
| `send` | addedToHand, shuffledIn | nothing | a gather at the caster's hero (the receiver's with no play), a card back flying to the hand or the library, and its sparkle there; no shake |
| `radiance` | radiantSet | nothing | a sheen and a burst over the card made Radiant (its hand pile for a hand card the viewer may not read), and once a play the Radiant rays |
| `mend` | healed | nothing | a sheen over the healed hero or unit, a ring closing as it lands and a soft afterglow; the heal's number stays on its row; no shake |

Drawn in #2796, Meditative's pass, built beside #2793's: `seal` for an event no motif read that comes up
on three or more Meditative cards, and `deal`, written at the same time as `send`, which it widens (it
also reads a fuse, gathers once an entry and caps its cards; a later pass may fold the two into one):

| Motif | Reads | Replaces | What it looks like |
|---|---|---|---|
| `deal` | addedToHand, shuffledIn, fused | nothing | a gather at the caster's hero (the receiver's with no play), then a card back flying to each hand, deck or card dealt, at most `FX_MOTIF_DEAL_MAX` an entry, landing with a sparkle; no shake |
| `seal` | modifierChanged (an add) | nothing | a bolt from the resolving play's hero to the seat's modifiers, a ring closing on them and a slow afterglow |

Drawn in #2794 (R2794000), for the cards that return one to the hand:

| Motif | Reads | Replaces | What it looks like |
|---|---|---|---|
| `recall` | addedToHand | nothing | a card back flying from its owner's graveyard pile into the hand: a gather on the pile, the flight, the hand's sparkle as it lands; no shake. The event names no zone, so a card back from exile flies from the graveyard pile too |

Drawn in #2797, Stinkyscape's first pass (R2797000), for a mark, which five of its first half's
cards put on or lift (Freeze Ray, Absolute Zero, CASPER, Kwzka, Siva 1):

| Motif | Reads | Replaces | What it looks like |
|---|---|---|---|
| `hex` | marked | nothing | a mark put on (a Freeze, an Allure) closes a ring on the card, a sheen crosses it and it flares as the mark takes; a mark lifted flares in a ring of sparkles; the mark's row keeps its color |

Every one-shot motif is drawn. `aura` on a hook parses and plays nothing (`planCardMotif` returns
nothing), so the card plays its event rows only: a card's standing look is its entry's `aura` field
(Auras, below).

### Phases

Each motif plays in four beats, each a fixed fraction of the entry's duration D, as `FX_SLAM_AT` is
(`FX_MOTIF_PHASES`):

| Phase | Starts at | What happens |
|---|---|---|
| Gather | 0 | energy collects at the source (or the centre, or the lane's far end) |
| Travel | 0.2 D | the shot, beam or wave crosses the board |
| Land | 0.6 D | every target is hit: a burst, the numbers when the motif replaces the row, a crack under a card destroyed, the shake |
| Linger | 0.75 D | the afterglow: a ring, embers, smoke |

### Parameters

- **palette:** the card's flavor's look (`FLAVOR_LOOKS`, `flavorLookOf`, a Chaos card's pick per
  instance) by default, or a flavor the entry names (`palette: "Fire"`). A palette may name a
  sub-look, `palette: "Tech/rocket"`, which changes no card's `flavor` (#2792).
- **scale:** `FX_MOTIF_RARITY_SCALE[rarity] × (1 + min(cost, FX_MOTIF_COST_CAP) × FX_MOTIF_SCALE_PER_COST)`,
  at most `FX_MOTIF_SCALE_MAX`, times the effects intensity (`FX_INTENSITY_SCALE`). An X cost counts
  as `FX_MOTIF_X_COST`, an Embiggen cost as its base. It scales every burst's count, every shot's
  density and the shake (`FX_MOTIF_TRAUMA[motif] × scale`, at most 1).
- **anchors:** the events' own targets (`targetFor`, as the event rows find them), their lanes and
  sides (`placeOf`), and the source: a damage's source card on the board, else the caster's hero.

### Sub-looks

A sub-look is a presentation-only variant of its flavor's look (#2792, `FLAVOR_SUB_LOOKS`): a
card's entry picks one with `palette: "<Flavor>/<sub-look>"`, and any other card keeps its
flavor's base look, which the event rows always draw.

| Flavor | Sub-look | Particle | For |
|---|---|---|---|
| Physical | steel | shard | armor and iron |
| Physical | wind | dust | speed and gusts |
| Physical | sport | confetti | athletes |
| Physical | blade | frost | swords and knives |
| Beast | felinor | sparkle | Felinors and cats |
| Beast | swarm | ember | wasps, hornets, scarabs, moths |
| Beast | wool | smoke | sheep |
| Dark | blood | blood | life stolen and blood prices |
| Dark | shadow | smoke | stealth |
| Dark | curse | arcane | hexes, doom, Locks |
| Nerd | ink | blood | writing and archives |
| Nerd | chalk | dust | classrooms |
| Nerd | pixel | circuit | games and screens |
| Tech | laser | spark | guns |
| Tech | rocket | exhaust | Stinkyscape's ships |
| Tech | starfield | star | its space and stations |
| Tech | hologram | sparkle | AI and displays |
| Tech | glitch | circuit | viruses and hacks |
| Hustle | jade | shard | the CN jade line |
| Hustle | cash | confetti | paychecks and taxes |
| Hustle | ore | crumb | Stinkyscape's resources |

A set pass picks a sub-look only for a card whose name or art sits squarely in that line; any
other card keeps its flavor's base look, which the event rows always draw.

The new canvas presets (#2792, drawn in code in `fx/presets.ts` and `fx/sprites.ts`) are `leaf`
(Nature's base particle), `circuit` (Tech's base particle, reused by Nerd/pixel), `crumb`
(Snack's base particle, reused by Hustle/ore), `star` (Tech/starfield) and `exhaust`
(Tech/rocket).

## Auras

`aura` (#2787, R2787000) is the one persistent motif: while a card's effect holds, the zone, lane, side
or board it covers wears a quiet looping look in the card's palette, a tint, a slow drift of motes and
an edge glow. An entry names it beside its hooks or alone: `{ aura: { look, palette?, scope } }`.

- **Looks** (`fx/aura/looks.ts`, `AURA_LOOKS`): `ember`, `frost`, `bloom`, `void`, `storm`, `holy`,
  `toxic` and `arcane`, each a mote velocity, size and life range, a mote, tint and edge opacity, a
  glow spread and a blend. `palette` is a flavor's look or a sub-look (`"Tech/rocket"`), the card's
  own by default, as a motif's.
- **Scopes**, nearest first: `zone` (the card's own backrow zone), `lane` (the lane's four zones,
  both seats), `side` (its seat's ten zones) and `board` (all twenty). Only a Field Spell or a Field
  Trap stands in a zone, so only it may name `zone` or `lane`; the loader fails any other card's.
- **What wears one** (`fx/aura/plan.ts`, read off the shown view alone): every face-up Field Spell or
  Field Trap whose entry has an aura, and every modifier the view lists that a readable play added
  (the play resolving when its `modifierChanged` came, with no trap fired inside it), on its seat's
  side. A face-down card, the viewer's own unrevealed Field Trap and a Unit in a backrow zone wear
  none (R202). The view's modifiers carry no source (R169), so a modifier whose play left the event
  window before the layer saw it (a reload, a hand-over) wears none.
- **Begin and end:** an aura the board starts to show fades in over `AURA_FADE_IN_MS` with a burst
  of motes from its box's centre; one it stops showing fades out over `AURA_FADE_OUT_MS`. Both follow
  the shown view, so the first view's auras stand without a begin.
- **Stacking:** at most `AURA_MAX_PER_ZONE` cover a zone: holding before fading, then the nearest
  scope, then the newest, the nearest drawn on top.
- **Cost:** one canvas and one frame loop (`fx/loop.ts`) for every aura, running only while one
  shows and never while the page is hidden; each draws at most `AURA_MOTES_PER_AURA` × the
  intensity, all of them at most `AURA_PARTICLE_SHARE` of the particle cap. Reduced motion leaves a
  still tint and edge; the effects intensity `off` leaves none.
- **The layer** (`fx/aura/AuraLayer.tsx`, `aura.css`): fixed over the board under the effects layer,
  no pointer events, a `.aura` per aura (`data-aura-look`, `-scope`, `-phase`) and no definition on
  the page.

Ten Field Spells wear one as the pattern for the set passes:

| Card | Aura |
|---|---|
| Core #14 Jlockeed's Weapons | `storm`, side |
| Core #46 Suppressive Aura | `void`, board |
| Classic #42 Transmutable Toxins | `toxic`, board |
| Classic #75 Argusland | `holy`, side |
| Classic+ #34 Memory Leak | `frost`, lane, palette Tech |
| Meditative #40 Feng Shui | `bloom`, board |
| Meditative+ #9 Magic Amplifier | `arcane`, board |
| Stinkyscape #46 Circadian Halfcycler | `storm`, lane |
| Stinkyscape #155 Comet Crash | `ember`, zone |
| Training Grounds #19 Extra Rations | `bloom`, zone |

No modifier example is wired yet, and a bespoke entry (Hinder's rider) cannot take an aura.

## The tables

`apps/web/src/fx/cards/<set>.json5`, one per set: `core`, `classic`, `classicplus`, `meditative`,
`meditativeplus`, `stinkyscape-1` (`stinkyscape-001` to `-090`, with the tokens listed among them),
`stinkyscape-2` (`-091` to the end, its `stinkyscape-t-*` tokens included) and `traininggrounds`.
Stinkyscape is split so its two set passes merge without conflicts.

```json5
{
  "classicplus-002": "none: pending", // Groom Shroom
  "core-035": { cast: { motif: "bolt" } }, // Lunar Eclipse
  "classic-016": { cast: { motif: "scorch", palette: "Fire" } }, // Book of Flame
  "classic-071": { cry: { motif: "lance" } }, // Lane Eater
  "core-021": { bespoke: "manaCrack" }, // Hinder
  "core-046": { cast: { motif: "bolt" }, aura: { look: "void", scope: "board" } }, // Suppressive Aura
}
```

- One line per catalog card, tokens included, in catalog order, its name in a comment after it.
  `tables.test.ts` holds each file to the catalog: every card present, in order, each comment its
  name, no card twice, no unknown hook, motif, look, scope, palette, field or recipe.
- **Hooks by kind:** `cast` is a non-Unit's play (a Spell's, a Field Spell's, a Trap's), `cry` a
  Unit's; a `cry` on a non-Unit or a `cast` on a Unit fails the loader. `death`, `trigger` (a
  start-of-turn or end-of-turn hook or a Whenever trigger) and `activate` play from the hook's own
  `hookResolving` marker (R2783000), and a Trap's `cast` from its `trapFired` (R2797000, Dispatch).
  A hook no marker reports (an attack's own, a Starship's launch) plays nothing.
- **entrance** (R2794000): a marquee Unit's entrance (`fx/entrances.ts`, R670) beside its hooks,
  `{ entrance: "titanBloom", cry: { motif: "flight" } }` (Classic #45 Nature Titan). Only a Unit may
  name one, and an entrance alone stays `{ bespoke: "<key>" }`.
- **row:** `"replace"` (the default) draws the numbers (the damage splat, an Armor flash) in place of
  the events the motif replaces; `"keep"` plays over the event's own recipe, as a bespoke recipe's
  `keep` does.
- A bad table fails at load, naming its file and card (`fx/cards/<file>: <id>: <problem>`).

### A set's pass

A set's pass (#2782's parts, Core's #2793 the first, then Training Grounds' #2799) fills its table from
`cargo jackioh fx census` (#2788), overriding the census's suggestion only with a reason, and leaves two
files beside it, the test built from the passes' shared kit (`src/test/setPass.ts`):

- `fx/cards/<set>.test.ts` holds the table to the pass: at least `FX_SET_PASS_FLOOR` of the set's
  non-token cards wear an entry other than `none`, every Spell, Field Spell, Trap and Field Trap has a
  `cast` (or a recipe or set piece of its own), every Field Spell an `aura`, every hook names a drawn
  motif, every play's motif draws on its gallery fixture as that card's play, and a hidden play keys
  none (R202).
- `fx/cards/<set>.md` is its per-card table, one row per card, tokens included, in catalog order:
  its name, type, entry, motif scale, the census's suggestion and why. The test holds each row's
  name, type, entry and scale to the catalog and the table.

A motif joins the vocabulary when three or more of a set's cards show an event no motif reads; a
card no motif can say takes a recipe of its own (Tiers, above).

Stinkyscape's two parts each hold their own file to the pass: part two's are `stinkyscape-2.test.ts`
and `stinkyscape-2.md` (#2798), over `stinkyscape-091` to the end with the set's tokens. The
Starship's own moments (the fuse, the launch, Launch Sequencer, #130 Jlockeed's Starship Retrofitting
Program and the Launchers #140 to #143) are #2782's Starship part: a card whose only hook is one of
them is `"none: Starship part"`, the one `none` a Spell of the pass may wear, and a Starship Piece's
other hooks take their entries in the pass. Part two adds no motif: every event it shows on three or
more of its cards is read already (`send`, `mend`), and the two cards whose `marked` is the moment
take recipes.

### Dispatch

`planFx` asks `planCardMotif` first, for every entry. It reads the play resolving now
(`FxMemory.resolving()`, from its `cardPlayed` to its `cardResolved` or `countered`); a hidden play,
whose `defId` is the sentinel, keys nothing (R202). The card's `cry` (a Unit) or `cast` (anything
else) names the motif; the motif plans over this entry's events it reads, and those it replaces skip
their row recipe. It never keys on an event's own `defId` (a destroyed card's is the victim's, not
the caster's), draws nothing on a damage another card dealt while the play resolved (a Trap
answering it keeps its own row) and leaves a run over a whole pile to its one wave. The runner gives
each hit of a run that sweeps no whole side its own entry, so the play remembers the ids its hits
landed on (`FxPlay.hits`): a volley gathers once, a chain hops on from its last hit, a lance beams a lane once,
and a card its hits already reached dies under its own row, so a lethal bolt flies once.

A card's hook is an entry too (R2797000): a Death's, a start-of-turn or end-of-turn hook's, a
Whenever trigger's or an Activate's `hookResolving` marker, and a Trap's `trapFired`, opens one for
its card in the memory, innermost (`FxPlay.hook`), so the events after it are the hook's and its
card's `death`, `trigger`, `activate` or `cast` names the motif. A Cry's marker opens none: its
play already keys it. The hook's entry ends at the next marker or firing, a `cardResolved` or
`countered`, an attack, a turn's end or start and the game's end; a hidden marker keys nothing
(R202), and a modifier a hook adds wears no aura (R2787000).

## Constraints

- **R200:** every cue starts inside its entry and is gone within `FX_MAX_TAIL_MS` of its end, through
  the existing tails (`FX_SPLAT_HOLD_MS`, `FX_CRACK_TAIL_MS`, `FX_RING_MS`); a shot lands inside the
  entry. Effects pace nothing.
- **R201:** the phases are fractions of D, and D already carries the effects speed.
- **R202:** only the redacted stream: the entry, its view and the readable resolving play. No cue
  carries a definition or a name.
- **Reduced motion and the effects off:** intensity 0, or an entry of no duration, plans nothing, so
  no shake and no flash.
- **CLAUDE.md rule 9:** every count is `MOTIF_TUNING`'s (`fx/motifs/build.ts`), every fraction, cap
  and shake a constant in `fx/constants.ts`, pinned by `constants.test.ts`.
- **No new rendering tech:** a motif builds the cue kinds the layer already draws.

## Keyword beats

A **beat** is #2782's smallest tier: a short decoration inside the entry of the event it keys on (at most
about 400 ms at speed 1), holding nothing and reading no card. The nine combat keywords each have one
(`fx/motifs/beats.ts`, ruling R2789000), and the ten tempo and card keywords one more each (ruling
R2791000), planned by `planFx` after the cast cues and built from the
helpers of `fx/motifs/build.ts` (`CueSpec` is the part of a motif's spec they read). The beats key on the
`via` of #2783's events (R2783001), and on the keywords the view shows on an attacker for the three that
act when it attacks; the tempo beats key on the existing events plus the keywords and ids the view
shows, a landing on the played card's hand keywords, never a hidden card.

| Keyword | Keyed on | Beat |
|---|---|---|
| Cleave | `damage` or `damageAbsorbed`, `combat`, `via: Cleave` | a wide `swipe` arc from the attacked target to the neighbour, a smaller landing and splat there at the land phase; the row is replaced |
| First Strike | `damage`, `combat`, `via: First Strike`; the death with `via: First Strike` | a sheen and glint on the attacker. The dying defender's counter-blow starts toward the attacker (a spark) and breaks off in smoke at `FX_FIRST_STRIKE_BREAK_AT`. A survivor's counter-blow, the next combat `damage` back, lands at the land phase, not at 0 |
| Trample | `damage`, `combat`, `via: Trample` | a bolt from the dead unit to the hero (or the unit beyond), a second splat there; the row is replaced |
| Pierce | `damage`, `via: Pierce` | a `beam` spear line `FX_PIERCE_REACH` past the defender's card on each side; the row is kept |
| Lifesteal | `healed`, `via: Lifesteal` | red `blood` motes from the damaged unit to the hero, landing with the heal's splat at the land phase; the heal row is replaced |
| Windfury | `attackDeclared`, second of the turn (`FxMemory.attacksThisTurn`) | a card-back `ghost` of the first attack's flight and a ring on the attacker |
| Poisonous | `destroyed`, `via: Poisonous` | the callout over the poisoner; the poison burst is the death recipe's (`sourceIsPoisonous`'s guess from the view is gone) |
| Ranged | `attackDeclared` by a Ranged unit | the callout; the flavor shot stays `stage.ts`'s |
| Flying | `attackDeclared` by a Flying unit | the callout; the lunge carries `arc`, and the director sets `--fx-lunge-lift` (`FX_FLYING_LUNGE_LIFT`) so `jk-lunge` lifts toward the viewer |
| Charge | `summoned`, the played card's hand keywords showing Charge | speed lines (`FX_CHARGE_LINE_SPREAD`) and a forward lean at the enemy hero as it lands, then a dash on its first attack that turn |
| Rush | `summoned`, the played card's hand keywords showing Rush | as Charge, but the lean points at the first enemy unit, never the hero (skipped when none shows); its own first-attack dash |
| Equip | `fused`, the kept result on the field with no ingredient there | the equipment flies from the hand region onto its host and snaps on, with a gold ring; the stat tick-up stays the `bless` row's |
| Tradable | `traded`, then the trade's `shuffledIn` and `drawn` | the card swirls hand region to library; the drawn card answers with a sparkle over the hand region, no second ghost (`flight` owns the flight) |
| Stack | `summoned` into an occupied slot | a gold burst at `FX_STACK_LAND` and a `zone` count pulse of the new pile total at `FX_ZONE_COUNT_AT` |
| Spell Damage | non-combat `damage`/`damageAbsorbed` while a Spell resolves, first entry | the hit burns hotter, with a `+N` sigil (the view's total) over the source |
| Lucky | `rolled` with extra draws | a leaf burst and a gold ring over the roll panel at each extra roll's `rollLandAt`, the kept face the panel's own highlight |
| Deft | `attackDeclared` after a same-turn `positionSwitched` | two Spirit bolts feinting out `FX_DEFT_FEINT_STEP` and back, under a sheen |
| Flashback | `cardPlayed` with `from: "graveyard"`, then its `exiled` | the card rises from the graveyard pile in a ghost frame, then dissolves at the exile pile once its graveyard play is in the history |
| Temporary | `discarded`, the hand card's Temporary count | the hourglass count over the hand card, dissolving in smoke and embers |

**The callout** (`fx/motifs/callout.ts`) is the shared cue every keyword part of #2782 uses: the keyword's
icon (`KEYWORD_CALLOUT_ICONS`, `cards/keywordVisuals.ts`: the nine combat icons plus R2791000's ten
tempo ones) pops over a unit for `FX_CALLOUT_MS`
(`.fx-callout`, a badge on the unit's top edge). A repeat of one keyword over a unit takes the first's place.

**Several keywords on one hit:** each fires its beat, `FX_BEAT_STAGGER_MS` apart (scaled with the speed,
and at most `FX_BEAT_STAGGER_MAX` of the entry in); at most `FX_CALLOUTS_PER_HIT` of them pop a callout.
Each beat is planned in the entry's D less its stagger, so it ends inside the entry and its tail (R200).

**Speed and motion:** D carries the speed, and so the stagger (R201). Under reduced motion a beat keeps
its callout as a static icon (`still`, `data-still`) and drops its motion: the runner signals `still`
for an entry that plays with no duration, and the layer draws `planStillCallouts` for it. With the
effects off, nothing. **R202:** a beat reads `via`, public keywords and shown ids only.

## The gallery

`/dev/fx` (dev builds only, as `/dev/hotseat`; a plain `build` leaves it out) is the review surface of
every pass. Over the fixture board (`fullBoardView`) it plays any drawn motif in any flavor's palette
or sub-look at any of `FX_GALLERY_SCALES`, through a director of its own, and any card whose entry is
not `none` (`galleryPlayOf`, `fx/motifs/gallery.ts`), as that card's play, in one of six ways:

| Play | Entry | How it plays |
|---|---|---|
| board | a drawn motif on `cast` or `cry` | the card resolves over the motif's fixture through the real board's effects layer |
| hook | a drawn motif on `death`, `trigger` or `activate` only | the hook's marker under the card's play, then the motif's fixture, through the real board's effects layer (R2797000) |
| recipe | a card recipe (`bespoke`) | the card resolves over the recipe's fixture (`RECIPE_FIXTURES`; Classic's Flourishes' fixtures sit beside their recipes in `fx/cards/classic/`) |
| entrance | a marquee entrance (`bespoke`, R2794000) | the card lands in the viewer's unit row as it plays, so its entrance plays |
| set piece | `setPiece` | the card's hook marker, then wipe's fixture, which its set piece holds |
| aura | an `aura` alone | the card's play alone, the card standing face up in the viewer's first backrow zone, where its aura shows |

A Field Spell or Field Trap of any play stands in that zone as it plays (`galleryFieldView`), so its
aura shows too. A marquee Unit whose entrance stands beside its hooks (`entrance`, R2794000) lands the
same way before its play's motif, so Nature Titan's entrance plays before its Cry. "Play as a
set piece" plays the same card as the fixture set piece (R2785000): its play's entry holds the board for
`SET_PIECE_HOLD_MS`, scaled and capped as every set piece, until a tap, a click or Space skips it.

## Set pieces

The cards that clear the board are #2782's loudest tier (R2800000). A card's set piece keys on its
hook's marker (`hookResolving`, R2783000, or a Trap's `trapFired`) when the seat reads the card
(R202): the runner plays the hook's first run of `destroyed`, `exiled` and `bounced` events (each
bounce with its `addedToHand`) as one pinned `WIPE_MS` entry (`game/runs.ts` `wipeAt`), the queue
claims it as the action's one set piece (`wipeClaim`, its key's beat at speed 1) and holds it as
R2785000 holds every set piece, and the effects layer draws it in place of the removals' own rows
(`planSetPiece`): its cue over the whole layer (`setPiece`), a `pull` and an `"after"` `conceal` on
each card it takes, and at the swallow, the end of the hold, an implosion, a ring and the shake. The
board twists the sides it takes from (`data-set-piece-shape`, `data-set-piece-sides`, board.css). It
centres on a Trap's zone (R154) or a Unit's own, else on the middle of the one side it takes from,
else on the board. Each has a sound bed of `sfx.ts` recipes across its hold (`SET_PIECE_BEDS`), and
the removals sound once, condensed, as it ends.

| Key | Shape | Beat | Palette | Cards |
|---|---|---|---|---|
| `nether` | vortex, the board swallowed into the Nether | `SET_PIECE_HOLD_MS` | Dark | Core #88 Twisting Nether (Radiant's "enemy permanents" over that side alone) |
| `voidFold` | fold: the edges close in to black, the cards sink, the light comes back at the Void | `VOID_FOLD_HOLD_MS` | Dark | Core #100 Ceaseless Void, which lands with its `voidCollapse` entrance first and whose takeover yields to the fold |
| `vortex` | vortex, turning the other way | `SET_PIECE_HOLD_MS` | the card's own | Core #17 Flood, Classic #43 Plague Nuke, Classic+ #1 Doom Shroom (at its zone), Classic+ #12.6 Frozen Wastes, Stinkyscape #4.1 Farewell, Stinkyscape #117 Core System |

The clears of one side, or too small to hold the game, take `wipe` on their hook: Classic+ #32.2
Brawl (it spares a Unit), both Calls to Chaos (Classic+ #73, Stinkyscape #151: one random effect of
theirs clears one side), the Anomaly Found! token (its caster's side) and Training Grounds #30
Floodgates (`death: wipe`, its own side, from its Death's marker). Stinkyscape
#12.3 Psychic Scream takes `wipe` too and draws nothing yet: its cards leave by `shuffledIn`, whose
ids R97 hides on both seats.

## The Starship

Stinkyscape's signature system (R1852000, R1852003) has two moments of its own (R2801000), drawn by
`fx/starship.ts` and keyed on nothing but the public Starship.

- **The fuse (a Signature, no hold).** Pieces fusing one after another into one Starship, each
  `fused` with the `starshipFused` after it, are one entry (`game/runs.ts` `fuseRunAt`) of
  `starshipFuseMs(n)`: `STARSHIP_FUSE_MS` and `FX_STARSHIP_FUSE_STAGGER_MS` for each later piece, up
  to `STARSHIP_FUSE_STAGGERED_MAX`. Each dead piece's card back flies from where it stood (else its
  graveyard) to the Starship, a stagger apart, and bolts on with a spark, a clank ring and a short
  shake. Unpinned, the burst budget may squeeze it; every beat keeps its share. A Dormant Starship's
  face bolts on a hull plate a piece (`cards/StarshipHull.tsx`, up to `STARSHIP_HULL_PLATES_MAX`),
  read from the view. R2791000's Equip beat stays off it: a piece's copy comes from the graveyard,
  never the hand.
- **The launch (a set piece, held).** `starshipLaunched` is claimed as the `starship` set piece
  (`starshipClaim`, `STARSHIP_LAUNCH_HOLD_MS` at speed 1), held, capped, scaled, shortened and
  skipped as R2785000 holds every set piece. Through the hold a 3-2-1 countdown pops over the ship
  (`FX_STARSHIP_COUNT_AT`), the board shaking harder at each count, and its engines ignite in Tech's
  rocket exhaust (`FX_STARSHIP_IGNITE_AT`); as the hold ends it lifts off (`jk-starship-launch`) with
  a contrail, its Dormant badge burning away, and lands as its launched self. Its exhaust, contrail
  and shake are `STARSHIP_TUNING`'s step for its pieces (small from 0, medium from 3, large from 6).
  Its bed (`STARSHIP_LAUNCH_BED`) beeps each count, higher each time, burns at the ignition and
  thuds at the landing; its own row's roar sounds at the lift-off. Under Big moments' "off" it plays
  unheld, with no countdown; under Reduce Motion it is one still frame of the launch, which holds
  nothing, as a Mythic's still card does.

| Card | Entry | Ties to the ship |
|---|---|---|
| Stinkyscape #130 Jlockeed's Starship Retrofitting Program | `cast: send`, Tech/rocket | the Jlockeed card it adds, a Starship Piece from then on, flies to the hand in the rocket's exhaust |
| Stinkyscape #140 to #143, the Launchers | `cry: conjure`, Tech/rocket | each lands in a ring of exhaust, the Activate a launched Starship takes from it |
| Stinkyscape T-AI-5 Launch Sequencer | `cast: tithe`, Tech/rocket | "Your Starship launch costs (1)": a glint on the badge and on the Dormant Starship whose launch it prices |
| the Dormant Starship | `none` | its fuse and launch are the Starship's own |

## Examples

Twelve cards across the sets wear the first motifs, as the pattern for the passes:

| Card | Entry |
|---|---|
| Core #35 Lunar Eclipse | `cast: bolt` |
| Classic+ #32.2 Brawl | `cast: wipe` |
| Classic #16 Book of Flame | `cast: scorch`, palette Fire |
| Classic #71 Lane Eater | `cry: lance` |
| Classic+ #21 Whirlwind | `cast: nova` |
| Classic+ #25 Soul Shot | `cast: bolt` |
| Meditative #78 Occidentless Mandate | `cast: vanish` |
| Meditative+ #5 Spirit Lash | `cast: chain` |
| Stinkyscape #3 Meteor | `cast: nova` |
| Stinkyscape #97 Asteroid Field | `cast: barrage` |
| Training Grounds #10 Fan of Knives | `cast: volley` |
| Training Grounds #26 Path of Exile | `cast: vanish` |

No `negate` example: every counter in the catalog is a Trap, whose `cast` plays from its firing
(R2797000).

Stinkyscape 001 to 090 (#2797, `stinkyscape-1.md`) is the first half of Stinkyscape's pass. It takes
`send` (#2793) for the cards it makes into a hand or a deck, `mend` for Healing Capsule's heals, `hex`
for its marks and `transmute` for its eight Equip Cries, and it keeps each Tech card apart by its
sub-look (#2792): `Tech/rocket` for the Drone Cores, Cannons and Flaks, `Tech/laser` for the Beams,
Repeaters and Weapon Equips, `Tech/hologram` for the MAGI and the Prototypes and `Tech/starfield` for
the Warp Disruptors, Imprisoned Cosmality and the Battleship. The second half (#2798) uses the same
names.

## Audit

Issue #2803's audit (R2803000) closes the work above and holds it:

- **Coverage:** 551 of the catalog's 634 non-token cards wear an entry other than `none`, past the
  audit's floor of half (`FX_COVERAGE_FLOOR`, `fx/cards/coverage.test.ts`). By set: Core 87/100,
  Classic 79/90, Classic+ 74/78, Meditative 94/102, Meditative+ 24/26, Stinkyscape 142/178,
  Training Grounds 51/60. By type: Unit 195/277, Spell 235/236, Field Spell 87/87, Trap 24/24,
  Field Trap 10/10. By rarity: Common 138/181, Rare 181/198, Epic 132/144, Legendary 72/81,
  Mythic 28/30. Every Spell, Field Spell, Trap and Field Trap wears a `cast`, a recipe, a set piece
  or the one `none` a Starship part may wear (Stinkyscape #176 Preemptive Strike, "Starship part"),
  and every Field Spell an `aura`.
- **Tiers:** every table entry's gallery play is planned through the board's effects layer, and every
  cue it plans ends within its entry plus `FX_MAX_TAIL_MS` (`fx/cards/tiers.test.ts`); every set
  piece's hold stays within `SET_PIECE_HOLD_CAP_MS`, one hold per action.
- **Motif use:** every drawn motif is used, and every sub-look. Four motifs sit below the audit's
  floor of three cards (`FX_MOTIF_MIN_CARDS`), each with its set pass's reason: `chain` (Spirit
  Lash, the Examples table's citation), `ward` (Argusland, Fortify Flesh), `mirror` (Rewind, Juhan
  Biggest Bat), `glimpse` (Fortify Mind, Feng Shui).
- **Frame rate:** at the phone particle cap with a phone viewport, Twisting Nether, the Starship
  launch, a Mythic takeover and a board with 4 auras hold their mean frame time within
  `FX_PHONE_FRAME_BUDGET_MS` (`e2e/cypress/component/fx-phone-frame.cy.tsx`).
- **Entrances:** the shared Mythic takeover, the Legendary entrance and the reduced-motion still
  land in the motif language's two-wave landing: a summoned or cast Legendary's ring carries its
  echo, the shared prismatic sweep has its own keyframes, the board recoils in two steps, and the
  still echoes the takeover at rest. The music, the entrances' timing and the one capped hold are
  unchanged (R2722000, R2722001, R2722002).
- **Captures:** the gallery spec plays every drawn motif mid-flight and every set piece mid-flight
  and at rest (`e2e/cypress/component/fx-gallery-captures.cy.tsx`).

## Open

- #2782's full text, with its 27 motif names and section 4's table layout, is not in the
  repository. A person pastes it here, and the names not listed above join `FX_MOTIF_NAMES`
  (`fx/motifs/names.ts`), so a table may name them before their parts draw them.
- `cargo jackioh fx census` suggests only #2784's damage and removal motifs, `rise` and `offering`:
  its `FAMILY_EVENTS` and `FAMILY_MOTIFS` (`crates/tools/src/fx_census.rs`) know none of #2786's
  other motifs nor #2793's, so a pass maps the events it counts through the Reads column above, with
  a reason per override (`core.md`, `stinkyscape-2.md`).
- `cargo jackioh fx census` (`crates/tools/README.md`) suggests a motif per card and hook from recorded games. It also suggests `rise` (a unit back from a graveyard) and `offering` (own units paid as a cost), #2782's names, which are not yet in this doc's table or `FX_MOTIF_NAMES`.
- The census's `FAMILY_MOTIFS` knows only damage, removal, return and cost, so for a draw, a buff, a
  summon, a card dealt or a modifier it says `none: no motif reads …`; a set pass maps that event to
  the motif whose Reads column names it (#2796's `meditative.md` does, card by card, as Classic+'s
  `classicplus.md` does, R2795000). It also takes a hook's window's turn and draw events (`turnEnded`,
  `manaChanged`, `drawn`) as the hook's own, which Classic+'s pass overrides with the hook's own event.
