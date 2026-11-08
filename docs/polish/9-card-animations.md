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
| set piece | `{ setPiece: "<key>" }` | a board clear's set piece: its hook's removals as one wipe the queue holds for one capped, skippable beat (R2785000, R2800000); or a Terminus card's, its signature run (R3338000) | `fx/setPieces.ts` (a Terminus card's planner in `fx/terminus/`), its board motion `game/board.css` and its sound bed `audio/setPieceBeds.ts` |
| signature | `signature: true` beside any of the last three, on a Legendary's or a Mythic's (R3332000) | the card's own animation, as in Hearthstone: its entrance (a Unit's, a Field Spell's) or a cast of its own (any other card's `bespoke` or `setPiece`), then the beat its text plays, and optionally an aura of its own, within its rarity's and type's budget ([Signatures](#signatures)) | the card's own files in `fx/cards/<set>/` |

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

Stinkyscape's first pass (#2797, R2797000) adds two more, in `fx/cards/stinkyscape/`, the same way
(its third, Olivian Expanse's `terminusRift`, became its set piece, R3338000):

| Card | Recipe | Why bespoke |
|---|---|---|
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
| `conjure` | summoned | nothing | a summoning circle where the card arrives: a gather, a landing and an afterglow |
| `rise` | summoned, animated, awakened | nothing | the card rising out of its zone, from below to where it stands |
| `offering` | destroyed | nothing | the unit sinking under a glyph in its lane: a cost paid, never a kill |
| `shift` | swapped, rotated, positionSwitched, flickered | nothing | the path of every unit that moved; a board swap or a rotation walks both rows, a rotation along R14's ring from the resolving play's seat |
| `bind` | locked, unlocked | both | frost and chains over a Locked unit; gold shards when it releases |
| `tether` | stolen, controlChanged, held | nothing | an arcane line from where the card was to whose it is now |
| `transmute` | transformed, fused, upgraded, degraded | all four | smoke and sparks as one card becomes another, landing on the card a fusion makes (an Equip fusing into its host, #2797), with a ring and rays for an upgrade and a crack for a degrade (#2908) |
| `flight` | drawn | drawn | a card back flying from the library into the hand, landing with its sparkle |
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
  `toxic`, `arcane`, (R3337003) `drift` (an asteroid field's), (R3400000) `flurry` (snow), `chalk`
  (chalk dust) and `timber` (sawdust), and (R3407000) fifteen signature looks, each one Stinkyscape card's
  own (`AURA_SIGNATURE_LOOK_NAMES`): `scanline`, `heartbeat`, `glint`, `pixelcrawl`, `duohalo`, `arcflick`,
  `steplights`, `searchlight`, `stationrings`, `aiwisp`, `crownpulse`, `drumbeat`, `starpath`,
  `stinkvapour` and `pages`; the twenty-seven each a mote velocity, size and life range, a mote, tint
  and edge opacity, a glow spread and a blend. `palette` is a flavor's look or a sub-look
  (`"Tech/rocket"`), the card's own by default, as a motif's.
- **Scopes**, nearest first: `tile` (R3400000: a Unit's own unit zone), `zone` (the card's own backrow
  zone), `lane` (the lane's four zones, both seats), `side` (its seat's ten zones) and `board` (all
  twenty). Only a Field Spell or a Field Trap stands in a zone, so only it may name `zone` or `lane`, and
  only a Unit may name `tile`; the loader fails any other card's.
- **What wears one** (`fx/aura/plan.ts`, read off the shown view alone): every face-up Field Spell or
  Field Trap whose entry has an aura, every face-up Unit whose entry has a `tile` aura (R3400000), and every modifier the view lists that a readable play added
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

No modifier example is wired yet. A bespoke entry may wear an aura beside its recipe (R3400000), as
Bob's and Tutorial Master's signatures do on their tiles; a modifier a Unit's play installs wears no
tile aura, which stands with the Unit.

### Tile auras

Tile auras (#3342, R3342000) mark the tile of every face-up Legendary and Mythic permanent on
the board (Units and backrow alike). A tile aura is a DOM element inside `.zone` that paints
under the card (`fx/aura/TileAura.tsx`, `tile.css`).

- **Colors:** derived from the card's picture (its painter palette), shifted within
  `±TILE_HUE_SPREAD_DEG` by its id hash so distinct Legendaries differ.
- **Tiers:** Legendary gets a soft edge glow (`TILE_GLOW_PX.Legendary`) and motes
  (`TILE_MOTES_LEGENDARY`). Mythic gets a richer pattern chosen by its set's look (circuitry for
  Core, runes for Classic, ink for Meditative, starfield for Stinkyscape, roots for Training Grounds),
  a wider edge glow and motes (`TILE_MOTES_MYTHIC`).
- **Signature replacement:** a card whose entry carries an `aura` (such as Feng Shui, or R3400000 a
  bespoke entry's, Bob's and Tutorial Master's) uses that standing aura in `AuraLayer` instead.
- **Interactions:** hover, press or tap sends a ripple, plays a bell in its hue (`auraTouch`) and
  buzzes on a phone (`Haptics.touch()`). Being picked as a target brightens the glow; damage or
  divine shield loss flickers it; Dormant, Frozen or Silenced dims it; destroyed or exiled fades
  it out over `TILE_FADE_OUT_MS`.
- **Budgets:** motes are capped at `TILE_PARTICLE_SHARE` of `FX_PARTICLE_CAP_MOBILE`. Under
  reduced motion, animations stop and motes/ripples omit; at intensity `off`, tile auras do not render.

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
- **The frame** (R3332000): any entry but a `none` may carry three fields beside its kind.
  - **entrance:** the entrance its card lands with in place of its rarity's (`fx/entrances.ts`'s
    `ENTRANCES`, R670, R2794000), beside any entry of a Unit or a Field Spell, or alone:
    `{ entrance: "titanBloom", cry: { motif: "flight" } }` (Classic #45 Nature Titan),
    `{ entrance: "bigBoom" }` (Classic #80), `{ setPiece: "voidFold", entrance: "voidCollapse", takeover: "void" }`
    (#100). A Field Spell's plays as it lands face up in the backrow, in place of the shared Legendary
    or Mythic rays. Any other card fails the loader: a Trap's entrance is its firing, its `cast`.
  - **takeover:** a Mythic's own takeover look (`void`, `pawn`, `genie`, `ai`: `takeoverLookOf`,
    `fx/takeover.ts`); every other Mythic takes the shared `prismatic`.
  - **signature:** `true` on a Legendary's or a Mythic's entry (a token's by the rarity it prints) that
    is its signature, which must name its first beat: a Unit's `entrance`, a Field Spell's `entrance`
    or its own cast, any other card's own cast (`bespoke` or `setPiece`).
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
other hooks take their entries in the pass. The Dormant Starship, #137 Starfleet Commander and #176
Preemptive Strike wear signatures of their own (R3334004). Part two adds no motif: every event it shows on three or
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
landed on (`FxPlay.hits`): a volley gathers once, a lance beams a lane once,
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
| entrance | an `entrance` alone (R2794000, R3332000) | the card lands in the viewer's unit row as it plays (a Field Spell face up in the first backrow zone), so its entrance plays, over the board its entrance reads (`galleryBoardOf`: Felinor Fiender's three Felinors) |
| set piece | `setPiece` | the card's hook marker, then wipe's fixture, which its set piece holds; a Terminus card's own run (`SET_PIECE_FIXTURES`, R3338000) |
| aura | an `aura` alone | the card's play alone, the card standing face up in the viewer's first backrow zone, where its aura shows |

A Field Spell or Field Trap of any play stands in that zone as it plays (`galleryFieldView`), so its
aura shows too. A marquee Unit whose entrance stands beside its hooks (`entrance`, R2794000) lands the
same way before its play's motif, so Nature Titan's entrance plays before its Cry. "Play as a
set piece" plays the same card as the fixture set piece (R2785000): its play's entry holds the board for
`SET_PIECE_HOLD_MS`, scaled and capped as every set piece, until a tap, a click or Space skips it.
"Signature" lists every card whose entry is its signature, grouped by set in the tables' order
(`gallerySignatures`, R3332000), so a person can review each one; "Play signature" plays it as "Play
card" does.

## Signatures

Issue #3330 asks for an animation of its own for every Legendary and Mythic, in place of the shared
motifs most of them played; #3332 laid the frame (R3332000) and each set's pass writes its cards'.

- **Budgets** (`fx/constants.ts`, `tiers.test.ts`): over its gallery play, held as the queue holds it
  at speed 1 with Big moments full, every cue of a Legendary's signature ends within
  `SIGNATURE_LEGENDARY_MAX_MS` (1.6 s) of the play's start, a Mythic's, its takeover's hold included,
  within `SIGNATURE_MYTHIC_MAX_MS` (2.6 s), and a Spell's, seen only briefly, within
  `SIGNATURE_SPELL_SHARE` of its rarity's. A Spell's shakes may reach `SIGNATURE_SPELL_MAX_TRAUMA` (1),
  a permanent's `SIGNATURE_PERMANENT_MAX_TRAUMA` (0.8), and a signature aura's alpha, tint and edge
  stay within `AURA_SIGNATURE_MAX_INTENSITY`. All of it sits inside R200's tail, one set piece per
  action and `SET_PIECE_HOLD_CAP_MS`, and a tap, the speed, the intensity, Big moments and Reduce
  Motion act on it as on every cue.
- **The ratchet** (`fx/cards/signature.test.ts`): `SIGNATURE_PENDING` lists every Legendary and Mythic,
  tokens by their printed rarity, whose entry is no signature, in catalog order. The test fails on one
  missing from it and on a listed card that has its signature; a pass takes its cards off, the list
  only shrinks, and the last pass empties it.
- **Felinor Fiender** (#92, R3332001), the first: `{ signature: true, entrance: "felinorAssembly" }`.
  It has the stats of every Felinor its controller has, so it assembles from them: each Felinor its
  side shows face up streaks cat light into its zone (`FX_FELINOR_STREAK_AT`, `FX_FELINOR_STREAK_STEP`,
  at most `FX_FELINOR_STREAKS_MAX`), all landing at the slam in a ring, a gathering burst and a gold
  crown. A Felinor buried under a Stack is face down and sends nothing (R202).
- **Meditative+ and Training Grounds** (#3400, R3400000, R3400001): every Legendary and Mythic of both,
  each built from its concept, drawn with figures (`fx/figures.ts`: shapes in a tint over their
  anchor's box, never a card face) and sounding its own effect (`audio/signatureCues.ts`):
  - **M+ #1 Bob's Plan** (`bobsPlan`, takeover `summit`): Bob's ink-wash range, a red brushstroke climbing
    to the summit flag; ink at the hold's end; a scroll and a card back for each card it adds, the
    plan's smaller. Sound `inkSummit`.
  - **M+ #24 Bob, Wisconsin's Strongest Soldier** (entrance `snowplowHeave`, `bobSoldier`, `flurry` on his
    tile): a snowplow heaved and slammed, frost into the enemy's zone of his lane; frost over every enemy
    Unit at the first freeze, a mitten for a stolen Unit, Coins from his hero, a Discovered card's sheen
    for its owner alone, a copied card's backs fanning in. Sound `plowHeave`.
  - **M+ #26 Scratch Pad** (`scratchPad`, takeover `canvas`): blank paper, faint peaks, a cyan cursor and
    ink rows; pixels at the hold's end; the scaffold folding into a card back; the paper tearing as it
    exiles itself. Sound `cursorTear`.
  - **TG #37 Seance** (`seance`): candles ring the announced tile (the centre for a face-down card) as the
    board dims to plum, a spirit rises; its copy drifts from there as a card back. Sound `seanceChant`.
  - **TG #38 Tutorial Master** (entrance `chalkboardLesson`, `tutorialMaster`, `chalk` on his tile): a
    chalkboard swings down, two raps, a chalk outline; each card his Cry or Death adds slides in as a
    card back, the first along a chalk arrow, his outline erasing first at his Death. Sound `slateRap`.
  - **TG #39 Call to Chaos (Training-Grounds Edition)** (`chaosDice`): two dice tumble in and stop in
    sparks; at its roll a face lights while R436's panel names the clause, capped at
    `FX_CHAOS_DICE_PANEL_MS`; a Call it casts rolls smaller dice. Sound `diceGong`.
  - **TG #39.1 Training-Grounds Golem** (entrance `golemLashing`, `timber` on its tile): posts fly in from
    the yard's four edges and lash together at the slam, then its keywords lock on (plank, shield,
    palisade, Rush, splinter). Sound `timberLash`.
  - **TG #40 Goetzmann Game Design** (`gameDesign`, takeover `napkin`): a napkin under lamplight, a pencil
    sketching, card silhouettes spiralling up; pixels at the hold's end; the sketch dealt as a card back
    in place of the hand's row. Sound `pencilArpeggio`.
- **The frame they need** (R3400000): a bespoke entry may wear an aura; a Unit's may stand on its `tile`;
  `summit`, `canvas` and `napkin` are takeover looks with art (`fx/takeoverArt.ts`), the same parts
  standing still behind Reduce Motion's card; and a signature Spell's own cast (`ownCast`) draws no
  shared rays, burst, rings or shake, its Mythic takeover ending `FX_SIGNATURE_TAKEOVER_TAIL_MS` after its
  entry.
- **Budget rules the recipes keep:** a Mythic Spell bursts only in its cast's entry, at most at its hold's
  end (its 1000 ms hold, then its 400 ms cast: a burst there ends at 1900 ms, inside 1950); a Legendary
  Spell bursts only in its cast's entry, at most 300 ms in; a Unit bursts in its entrance and at the start
  of its first text beat; every later entry draws only figures, card backs and sheens, each ending with
  its entry; every beat replaces its event's row; a Spell shakes at most 1, a Unit at most 0.8.
- **Sound** (R3332002): a Legendary's or Mythic's play sting is its own intro, played whole with
  dynamic music on, its opening cut to `MUSIC_STING_S` on the effects bus otherwise; the shared
  `entrance` sting plays only for one with no intro.

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
| `voidFold` | fold: the edges close in to black, each card is drawn into the Void's chest in turn, and its 10/10 burns in the dark (R3338000) | `VOID_FOLD_HOLD_MS` | Dark | Core #100 Ceaseless Void, which lands with its `voidCollapse` entrance first and whose takeover yields to the fold |
| `vortex` | vortex, turning the other way | `SET_PIECE_HOLD_MS` | the card's own | Core #17 Flood, Classic #43 Plague Nuke, Classic+ #1 Doom Shroom (at its zone), Classic+ #12.6 Frozen Wastes, Stinkyscape #117 Core System |
| `farewell` | lamp: a lamp swings over the table, every card it exiles lifts into its light and fades in turn, then a slow exhale and no shake (R3338000) | 2.8 s | Dark | Stinkyscape #4.1 Farewell |

The clears of one side, or too small to hold the game, take `wipe` on their hook: Classic+ #32.2
Brawl (it spares a Unit), Classic+ #73 Call to Chaos (one random effect of it clears one side; Stinkyscape
#151's took `wipe` too until its signature, R3407000), the Anomaly Found! token (its caster's side) and Training Grounds #30
Floodgates (`death: wipe`, its own side, from its Death's marker). Stinkyscape
#12.3 Psychic Scream takes `wipe` too and draws nothing yet: its cards leave by `shuffledIn`, whose
ids R97 hides on both seats.

### The Terminus cards (R3338000)

The end-of-the-world cards (#3338) each have a set piece of their own, its planner in `fx/terminus/`.
Besides Ceaseless Void's fold and Farewell's lamp, above, each is a **signature**: the runner plays its
hook's run of the event types its set piece names (`moment`, `signatureRunOf`) as one pinned
`SIGNATURE_RUN_MS` entry (`game/runs.ts` `signatureRunAt`; The Big Crunch's is its first cast), which
the queue claims (`signatureClaim`) and holds as every set piece. A Mythic's takeover yields to its own
signature. Under Reduce Motion each is one still frame (`planTerminusStill`). Each bed opens on its own
`organ` chord and closes on the Mythic choir. No cue names a card: a card a seat cannot read is a card
back flying to its hand (R202).

| Key | Shape | Beat | Its run | What it draws |
|---|---|---|---|---|
| `terminusRift` | rift | 2.4 s | the Discover's `addedToHand` | Stinkyscape #4 Olivian Expanse: a violet rift, a star circling in it for each Terminus card, one falling into the hand |
| `lastExam` | chalk | 2.4 s | the answer's `damage` | #4.2 KY's Last Exam: the problem chalked across the board, the chalk bolt into the enemy hero, a crack and its Echo stamped |
| `eventHorizon` | bend | 2.4 s | the extra turn's `modifierChanged` | #4.3 Event Horizon: the board smears to its centre under a stretched clock, the turn banner twice; the extra turn opens snapping back |
| `lastWord` | freeze | 2.2 s | its firing, its way to the graveyard, its modifiers | #4.4 Last Word: everything frozen in mid-strike, the hit on its hero shattering; the next turn opens with Charge on each Unit and a 3, 2, 1 |
| `finalDestination` | abyss | 2.8 s | its firing, `locked`, `healthSet`, its discards | #4.5 1v1 Final Destination: every lane but one chained shut over the abyss, the heroes spun down to 1, the Spells burning off the hands |
| `bigCrunch` | crunch | 2 s | its first cast | #4.6 The Big Crunch: the backdrop collapsing to a bright point, which holds over each entry of its play |
| `diceOfDoom` | doom | 2.8 s (a miss 1.2 s) | `rolled`, its Lucky gain, the cards it adds or burns | Stinkyscape #154 Dice of Doom: a big d100 tumbling in onto its roll; on a 67 the board dark and the Terminus cards raining in, on a miss the die crumbling, a Lucky reroll's glint |

## The Starship

Stinkyscape's signature system (R1852000, R1852003) has moments of its own (R2801000, R3334000 to
R3334004), drawn by `fx/starship.ts` from the ship's own picture (`cards/art/stinkyscape.ts`,
R3333000) and keyed on nothing but the public Starship: `starshipFused`, `starshipLaunched`,
`attackDeclared`, `destroyed` and a readable War Doctrine's `cardPlayed`. A point on the picture is an
anchor's `art: "picture"` (`fx/anchors.ts` `artBoxOf`), so a module, an engine or the prow is found on
the card wherever the board draws it. A phone (narrower than `FX_MOBILE_WIDTH`) throws
`STARSHIP_CINEMATIC.phoneShare` of every particle, with the same beats.

- **The fuse (a Signature, no hold).** Pieces fusing one after another into one Starship, each
  `fused` with the `starshipFused` after it, are one entry (`game/runs.ts` `fuseRunAt`) of
  `starshipFuseMs(n)`: `STARSHIP_FUSE_MS` and `FX_STARSHIP_FUSE_STAGGER_MS` for each later piece, up
  to `STARSHIP_FUSE_STAGGERED_MAX`. Each dead piece's module (R3333001) flies from where it stood
  (else its graveyard) to its own hardpoint on the ship (`starshipMountPoints` of the list the ship
  holds after the run), a stagger apart, and docks (R3334000): two clamps close on it, a weld flashes
  and it hums higher for each piece the ship then holds (`STARSHIP_DOCK`), with a spark, a clank ring
  and a short shake. Each docking sounds `starshipDock`, its pitch rising with the count
  (`audio/starshipSounds.ts`; the engine's retrigger guard reads a dock when it sounds,
  `SCHEDULED_SFX`). Unpinned, the burst budget may squeeze it; every beat keeps its share. A Dormant
  Starship's face bolts on a hull plate a piece (`cards/StarshipHull.tsx`, up to
  `STARSHIP_HULL_PLATES_MAX`), read from the view. R2791000's Equip beat stays off it: a piece's copy
  comes from the graveyard, never the hand.
- **The shipyard idle.** A Dormant Starship holding pieces idles on its tile (`cards/ShipyardIdle.tsx`,
  R3334000): its dock light sweeps slowly across the hull and welding sparks flicker where its newest
  piece went in (`SHIPYARD_IDLE_*` in `cards/constants.ts`). It takes one of the board's moving-art
  slots (`useArtMotion`, `COSMETIC_FOIL_ANIMATED_MAX`) and holds still under Reduce Motion or with
  Animated foil off.
- **The launch (a set piece, held).** `starshipLaunched` is claimed as the `starship` set piece
  (`starshipClaim`, `STARSHIP_LAUNCH_HOLD_MS` at speed 1), held, capped, scaled, shortened and
  skipped as R2785000 holds every set piece. At Big moments "full" the hold is a cinematic
  (R3334001, `STARSHIP_CINEMATIC`): the board dims and the ship's picture fills the screen, the camera
  moves along the hull from stern to bow past a stop at each module, the engines light one after
  another, the name it launched under stamps across the hull in its design's lettering
  (`STARSHIP_LETTERING`), and it climbs out of frame. As the hold ends it lifts off its tile
  (`jk-starship-launch`) with a contrail, its Dormant badge burning away, and lands as its launched
  self. Its zoom, sparks, exhaust, contrail and shake are `STARSHIP_TUNING`'s step for its pieces
  (small from 0, medium from 3, large from 6). Its bed (`STARSHIP_LAUNCH_BED`) ignites as the engines
  light, roars as it climbs and thuds at the landing; its own row's sting sounds at the lift-off.
  Big moments "short" and "off", an unheld launch and Reduce Motion show a still title card instead,
  the ship's picture and its name; under Reduce Motion it holds nothing, as a Mythic's still card
  does.
- **The War Doctrines.** A War Doctrine cast (Stinkyscape #137.1 to #137.4, which #137 Starfleet
  Commander and #176 Preemptive Strike cast after their launch) is announced over its caster's
  launched Starship, else the board's middle, in its own look for Land, Sea, Air or Space
  (`WAR_DOCTRINE_LOOKS`), with a burst, a ring and a shake (R3334001).
- **The broadside.** A launched Starship's attack fires its mounted weapons where the picture mounts
  them, one after another inside the attack's row, every shot landing as it ends (R3334002,
  `STARSHIP_BROADSIDE`, `BROADSIDE_WEAPONS`): beams sweep, repeaters stutter, cannons kick back, flak
  bursts in clouds, launchers lob what they carry (a token, fruit, jade, a book; the Missile Pod
  arrows). A ship with no weapons rams prow first, its lunge turned `FX_STARSHIP_RAM_TURN_DEG`
  toward its target (`stage.ts`). Each kind it fires sounds once, as its first gun fires; a ram sounds
  at the contact; the swing's own sound is dropped for them, so under Reduce Motion, which plays no
  broadside, it swings with the swing's sound.
- **The wreck.** A Starship's `destroyed`, outside a wipe (R3334003, `STARSHIP_WRECK`): its newest
  modules blow off one by one, the hull breaks in two as its card is hidden, debris drifts across its
  backrow and a scorch settles on its tile. It groans as its hull breaks, in the death sting's place;
  under Reduce Motion or inside a wipe, where no groan plays, the sting stays.

| Card | Entry | Ties to the ship |
|---|---|---|
| Stinkyscape #130 Jlockeed's Starship Retrofitting Program | `cast: send`, Tech/rocket | the Jlockeed card it adds, a Starship Piece from then on, flies to the hand in the rocket's exhaust |
| Stinkyscape #137 Starfleet Commander | signature, entrance `fleetCommand` | it lands and orders the Dormant Starship up: a rocket bolt to the ship, a ring, a crown of gold and a shake (R3334004) |
| Stinkyscape #140 to #143, the Launchers | `cry: conjure`, Tech/rocket | each lands in a ring of exhaust, the Activate a launched Starship takes from it |
| Stinkyscape #176 Preemptive Strike | signature, bespoke `preemptiveStrike` | an alarm of fire rings on its hero and a red strike; the Dormant Starship it wakes branded red (R3334004) |
| Stinkyscape T-AI-5 Launch Sequencer | `cast: tithe`, Tech/rocket | "Your Starship launch costs (1)": a glint on the badge and on the Dormant Starship whose launch it prices |
| the Dormant Starship | signature, entrance `shipyardRollout` | it rolls out of its shipyard: weld sparks over its hull, then a ring, dust and a slam (R3334004) |

## The Drone Battleship

Stinkyscape #36 Drone Core X summons the Drone Battleship as three tokens in one zone, bottom first (R1842000),
and R3336000 makes them one great ship in three decks: the reactor keel (Meltdown), the hangar decks
(Propagation) and the command superstructure (Shield). Its art is `cards/art/stinkyscape.ts`'s (a layer on the
board draws its own deck and every deck beneath it, so the top card shows the ship as far as it stands); its
arrival is Drone Core X's `battleshipWarp`, one beat per layer summoned; every other beat is
`fx/battleship.ts`'s (`planBattleship`), keyed on public events and the layers the view draws (R202), and each has
its sound in `audio/battleshipCues.ts`. The three layers' table entries are `none: Battleship planner`, since the
Shield's guard, a break and the blast belong to no hook; signature entries wait for #3332.

| Beat | Event | What it draws | Sound |
|---|---|---|---|
| Arrival, the keel | Drone Core X's 1st `summoned` | the warp's starlight streak from the board's top edge, exhaust, rings, a shake | `horn` |
| Arrival, the hangar | its 2nd | two bolts from above the zone's corners lock onto it, a ring, sparks, a shake | `lock` |
| Arrival, the superstructure | its 3rd | a bolt from above and the dome's full shield, a shake | `armorRing` |
| The shield takes the hit | `redirected` onto the Shield | a bolt from the protected target to the dome, which flares and ripples; a shake | `domeFlare` |
| The plates | the `damageAbsorbed` or `damage` the redirect sent | the dome flares again; `ship` cue `plates`: one per point of Armor (at most `FX_SHIP_PLATES_MAX`), as many lit as Armor took | `domeFlare` |
| An ordinary hit | any other `damage` or `damageAbsorbed` on the Shield | a sheen, a small shield on the dome, a light shake | `armorClank` |
| A bay opens | `activated` on the Propagation layer | `ship` cue `bay`: two doors slide apart and a light stays on for each bay with a use left | `bayDoor` |
| A drone launches | a drone `summoned` inside that Activate | an Interceptor shot fast from the bay to its tile; a Corvette rolled out slow in exhaust, landing with a shake | `whoosh`, `summon` large |
| The core pulses | the Meltdown's `damage` to itself | a fire ring and an ember burst, a shake: all grow with the share of its health gone | `reactorPulse` 1 to 10 |
| The reactor fires | the Meltdown's `attackDeclared` | a beam from the core to the target | `burn` |
| The blast | the Meltdown's `destroyed` | two rings, fire, embers, shards, rays and the hardest shake | `reactorPulse` 10 |
| A layer breaks | a Shield's or Propagation layer's `destroyed` with `buried` above 0 | a crack and a burst of plates, a shake, then a sheen and a ring over the zone as the next deck's lights come up | `deckShear` |

The arrival is three entries of the runner's summon time (about 1.65 s with their tails), tested to end inside
`SET_PIECE_HOLD_CAP_MS`, and not one held set piece. A Battleship that Call to Chaos (Scape Edition) or Anomaly
Found! summons gets the layer beats and no arrival. Under Reduce Motion the ship cue draws nothing. `e2e/cypress/component/battleship.cy.tsx` plays every
beat at 1280x720 and at 390x844, and `fx-phone-frame.cy.tsx` holds the arrival to `FX_PHONE_FRAME_BUDGET_MS`.

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
| Meditative+ #5 Spirit Lash | `cast: nova` |
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

- **Coverage:** 554 of the catalog's 634 non-token cards wear an entry other than `none`, past the
  audit's floor of half (`FX_COVERAGE_FLOOR`, `fx/cards/coverage.test.ts`). By set: Core 87/100,
  Classic 79/90, Classic+ 74/78, Meditative 94/102, Meditative+ 24/26, Stinkyscape 145/178,
  Training Grounds 51/60. By type: Unit 197/277, Spell 236/236, Field Spell 87/87, Trap 24/24,
  Field Trap 10/10. By rarity: Common 138/181, Rare 181/198, Epic 132/144, Legendary 75/81,
  Mythic 28/30. Every Spell, Field Spell, Trap and Field Trap wears a `cast`, a recipe or a set
  piece; the one `none` a Starship part may wear is worn by no Spell since #176 Preemptive Strike
  took its signature (R3334004). Every Field Spell wears an `aura`.
- **Tiers:** every table entry's gallery play is planned through the board's effects layer, and every
  cue it plans ends within its entry plus `FX_MAX_TAIL_MS` (`fx/cards/tiers.test.ts`); every set
  piece's hold stays within `SET_PIECE_HOLD_CAP_MS`, one hold per action.
- **Motif use:** 31 drawn motifs remain, every one used by at least `FX_MOTIF_MIN_CARDS` (3)
  cards, and every sub-look is used. The audit folds the four below the floor: `chain` into
  `nova` (Spirit Lash hits every Unit at once; amends R2784000's Examples-table citation),
  `mirror` into `transmute` (Rewind, Juhan Biggest Bat), `ward` into the rows (Armor's shield
  and the Divine Shield break draw in `cues.ts` in Holy, shaking `FX_ARMOR_TRAUMA`; Argusland
  and Fortify Flesh name `bless`), `glimpse` into bespoke `mindJudge` (Fortify Mind keeps its
  judgment's sheen and flare on the hero's badge, shaking `FX_MIND_JUDGE_TRAUMA`) and `bless`
  (Feng Shui).
- **Frame rate:** at the phone particle cap with a phone viewport, all five scenes hold
  their mean frame time within `FX_PHONE_FRAME_BUDGET_MS`
  (`e2e/cypress/component/fx-phone-frame.cy.tsx`): the cap check, Twisting Nether, the
  Starship launch, a Mythic takeover and a board with 4 auras. The two misses were scaled
  back with their composition untouched: Nether throws fewer pull/implode particles and
  shakes softer with a smaller-same-arms vortex spiral, the takeover fewer Mythic-burst
  particles and a softer shake with same-disc takeover rays; a follow-up issue tracks
  measuring them on a real phone.
- **Entrances:** the shared Mythic takeover, the Legendary entrance and the reduced-motion still
  land in the motif language's two-wave landing: a summoned or cast Legendary's ring carries its
  echo, the shared prismatic sweep has its own keyframes, the board recoils in two steps, and the
  still echoes the takeover at rest. The music, the entrances' timing and the one capped hold are
  unchanged (R2722000, R2722001, R2722002).
- **Captures:** the gallery spec plays every drawn motif mid-flight — `shift` proving its
  DOM ghosts at 0.25× speed (its canvas bursts age out in software rendering; the tier
  test proves their timing) — and every set piece mid-flight and at rest
  (`e2e/cypress/component/fx-gallery-captures.cy.tsx`).

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
