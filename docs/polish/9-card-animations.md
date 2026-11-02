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

Every catalog card starts as `"none: pending"` and moves up a tier in its set's pass. A bespoke
recipe is kept for the few cards no motif can say (the seven of the former `CARD_FX` table: #21
Hinder, #27 Blood Ridden Glowy Jelly Bean, Classic+ #24 Crushing Walls and the marquee entrances of
#100 Ceaseless Void, BOOM! Big Max, Nature Titan and Spell Tyrant).

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

Named and not drawn yet: `aura` (#2787), and A2's motifs (#2786). A table may name one already: it
parses, and until its part lands the card plays its event rows only (`planCardMotif` returns
nothing).

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
  instance) by default, or a flavor the entry names (`palette: "Fire"`). #2792 adds
  `"<Flavor>/<sub-look>"`.
- **scale:** `FX_MOTIF_RARITY_SCALE[rarity] × (1 + min(cost, FX_MOTIF_COST_CAP) × FX_MOTIF_SCALE_PER_COST)`,
  at most `FX_MOTIF_SCALE_MAX`, times the effects intensity (`FX_INTENSITY_SCALE`). An X cost counts
  as `FX_MOTIF_X_COST`, an Embiggen cost as its base. It scales every burst's count, every shot's
  density and the shake (`FX_MOTIF_TRAUMA[motif] × scale`, at most 1).
- **anchors:** the events' own targets (`targetFor`, as the event rows find them), their lanes and
  sides (`placeOf`), and the source: a damage's source card on the board, else the caster's hero.

## The tables

`apps/web/src/fx/cards/<set>.json5`, one per set: `core`, `classic`, `classicplus`, `meditative`,
`meditativeplus`, `stinkyscape-1` (`stinkyscape-001` to `-090`, with the tokens listed among them),
`stinkyscape-2` (`-091` to the end, its `stinkyscape-t-*` tokens included) and `traininggrounds`.
Stinkyscape is split so its two set passes merge without conflicts.

```json5
{
  "core-034": "none: pending", // Collateral Damage
  "core-035": { cast: { motif: "bolt" } }, // Lunar Eclipse
  "classic-016": { cast: { motif: "scorch", palette: "Fire" } }, // Book of Flame
  "classic-071": { cry: { motif: "lance" } }, // Lane Eater
  "core-021": { bespoke: "manaCrack" }, // Hinder
}
```

- One line per catalog card, tokens included, in catalog order, its name in a comment after it.
  `tables.test.ts` holds each file to the catalog: every card present, in order, each comment its
  name, no card twice, no unknown hook, motif, palette, field or recipe.
- **Hooks by kind:** `cast` is a non-Unit's play (a Spell's, a Field Spell's, a Trap's), `cry` a
  Unit's; a `cry` on a non-Unit or a `cast` on a Unit fails the loader. `death`, `trigger` and
  `activate` are accepted and plan nothing until #2783's `hookResolving` marker lands. A Trap's
  `cast` is inert too: a firing opens no play.
- **row:** `"replace"` (the default) draws the numbers (the damage splat, an Armor flash) in place of
  the events the motif replaces; `"keep"` plays over the event's own recipe, as a bespoke recipe's
  `keep` does.
- A bad table fails at load, naming its file and card (`fx/cards/<file>: <id>: <problem>`).

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
(`fx/motifs/beats.ts`, ruling R2789000), planned by `planFx` after the cast cues and built from the
helpers of `fx/motifs/build.ts` (`CueSpec` is the part of a motif's spec they read). The beats key on the
`via` of #2783's events (R2783001), and on the keywords the view shows on an attacker for the three that
act when it attacks.

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

**The callout** (`fx/motifs/callout.ts`) is the shared cue every keyword part of #2782 uses: the keyword's
icon (`KEYWORD_CALLOUT_ICONS`, `cards/keywordVisuals.ts`) pops over a unit for `FX_CALLOUT_MS`
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
at any of `FX_GALLERY_SCALES`, through a director of its own, and any card whose entry names a motif,
as that card's play through the real board's effects layer.

## Examples

Twelve cards across the sets wear the first motifs, as the pattern for the passes:

| Card | Entry |
|---|---|
| Core #35 Lunar Eclipse | `cast: bolt` |
| Core #88 Twisting Nether | `cast: wipe` |
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

No `negate` example: every counter in the catalog is a Trap, whose firing opens no play.

## Open

- #2782's full text, with its 27 motif names and section 4's table layout, is not in the
  repository. A person pastes it here, and the names not listed above join `FX_MOTIF_NAMES`
  (`fx/motifs/names.ts`), so a table may name them before their parts draw them.
- `cargo jackioh fx census` (`crates/tools/README.md`) suggests a motif per card and hook from recorded games. It also suggests `rise` (a unit back from a graveyard) and `offering` (own units paid as a cost), #2782's names, which are not yet in this doc's table or `FX_MOTIF_NAMES`.
