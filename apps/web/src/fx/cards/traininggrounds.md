# Training Grounds' card animations (#2799)

Training Grounds' pass over its animation table, `traininggrounds.json5` (`docs/polish/9-card-animations.md`,
"A set's pass"): one row per Training Grounds card, its token included, in catalog order.
`traininggrounds.test.ts` holds every row's name, type, entry and scale to the catalog and the table, and
the table to the pass's floor.

- **Entry**: the card's entry in `traininggrounds.json5`: `none: <reason>`, `bespoke <recipe>` (R3400000: then
  `; aura <look> <scope>[ <palette>]`), or `signature`, `entrance <key>` and each hook's
  `<hook> <motif>[ <palette>]`, then `aura <look> <scope>[ <palette>]`.
- **Scale**: the card's motif scale at intensity 1 (`motifScale`), from its cost and rarity alone; `—` for
  a `none`.
- **Census**: the "Suggested" cell of `cargo jackioh fx census` (#2788) for the card, run at `53e62cbe2`
  (200 seeded games and 19 forced ones), or `not in census` for the token it never played. Its families
  read damage, removals, counters, costs and returns only, and leave out the events on the card itself.
- **Why**: where the entry follows the census, and why it overrides it where it does.

`cast` and `cry` draw on the board as the card resolves. `death`, `trigger` and `activate`, and a Trap's
`cast`, plan nothing on the board until their hooks open a window (#2783); the `/dev/fx` gallery plays
every entry.

The 20 Legacy cards (#41 to #60) take the motif and palette a Training Grounds card of the same effect
takes, in the yard's style (#2724), not their Core card's: Legacy: Duplicating Felinors and Legacy: Big
Felinor wear the set's felinor sub-look beside #22, #23 and #33, while Legacy: Hit Job, Legacy: Snom Bunny
Mind Control and Legacy: Prejudiced Postdoc wear their own flavor's look where Core's wear blade, curse and
chalk (the set keeps blade for #18's knives and chalk for #38's teacher).

| Card | Name | Type | Entry | Scale | Census | Why |
| --- | --- | --- | --- | --- | --- | --- |
| `traininggrounds-001` | Spikey Steven | Unit | none: vanilla | — | none: vanilla | the census agrees: no text |
| `traininggrounds-002` | Personal Trainer | Unit | cry bless Physical/sport | 1.08 | none: no motif reads buffed | cry: no census family reads buffed, upgraded and keywordGranted, which bless (#2786) reads; Physical's sport sub-look for a trainer |
| `traininggrounds-003` | Group leader | Unit | none: its Aura is a standing Attack bonus, with no moment to draw | — | none: vanilla | the census agrees: its Aura changes nothing as it plays |
| `traininggrounds-004` | Boulder Fist Ogre | Unit | none: vanilla | — | none: vanilla | keywords only (Rush), which the keyword beats and the attack rows draw |
| `traininggrounds-005` | Midlane Menace | Unit | cry bless | 1.16 | none: vanilla | cry: the census leaves out the events on the card itself, its buffed and its granted Rush in midlane, which bless reads |
| `traininggrounds-006` | Chosen by the Sun | Unit | none: vanilla | — | none: vanilla | keywords only (Divine Shield), which the keyword beats draw; Celestial Caller's flourish draws its summon when the Caller calls it |
| `traininggrounds-007` | Chosen by the Moon | Unit | none: vanilla | — | none: vanilla | keywords only (Taunt, Armor), which the keyword beats draw; Celestial Caller's flourish draws its summon when the Caller calls it |
| `traininggrounds-008` | Clobberer | Unit | none: vanilla | — | none: vanilla | the census agrees: no text |
| `traininggrounds-009` | Ravager | Unit | trigger bless | 1.32 | bolt | trigger: the census's bolt is noise in its window (fatigue, other cards' events); its own effect after a kill is buffed on itself, which bless reads |
| `traininggrounds-010` | Fan of Knives | Spell | cast volley | 1.08 | lance | kept: #2784's example; it hits every enemy at once, a fan of knives (the census's lance saw one lane's hits) |
| `traininggrounds-011` | Thwomp | Spell | cast bolt | 1.16 | bolt | cast: the census agrees (bolt) |
| `traininggrounds-012` | Fortify | Spell | cast bless Physical/steel | 1.08 | none: no motif reads keywordGranted | cast: no census family reads buffed, upgraded and keywordGranted, which bless (#2786) reads; Physical's steel sub-look for its Armor sets it apart from #32's Holy bless |
| `traininggrounds-013` | Explosive Runes | Trap | cast volley | 1.08 | volley | cast: the census agrees (volley), its firing's hits on every enemy; a Trap's cast plans nothing until firings open a window |
| `traininggrounds-014` | Explosive Trap | Trap | cast bolt | 1.08 | bolt | cast: the census agrees (bolt), its firing's hit on the Unit played; a Trap's cast plans nothing until firings open a window |
| `traininggrounds-015` | In a hurry | Spell | cast conjure Physical/wind | 1.24 | vanish | cast: the census's vanish is its tokens' end-of-turn exile (2), its play their summons (34), which conjure reads; Physical's wind sub-look for its haste |
| `traininggrounds-016` | Celestial Caller | Unit | bespoke celestialCall | 1.45 | none: no motif reads summoned | bespoke: its Cry calls a Sun and a Moon at once, in two looks one motif's palette cannot hold; conjure would read the summons in one |
| `traininggrounds-017` | Flaming Felinor | Unit | cry nova | 1.28 | lance | cry: the census's lance counted one lane's hits; its play window's nova is the card, every other Unit hit at once on both sides |
| `traininggrounds-018` | Knife Juggler | Unit | trigger bolt Physical/blade | 1.19 | bolt | trigger: the census agrees (bolt), a knife at a random enemy, in Physical's blade sub-look (knives) |
| `traininggrounds-019` | Extra Rations | Field Spell | cast conjure; trigger flight; aura bloom zone | 1.19 | bolt | cast: a Field Spell's play is its own landing in the backrow (`summoned`), which the census leaves out; conjure draws it; trigger: the census's bolt is the fatigue its draws deal, its own effect drawn, which flight reads; aura: kept, #2787's example |
| `traininggrounds-020` | Recruitment Posters | Field Spell | cast conjure; trigger send; aura holy zone | 1.28 | bolt | cast: a Field Spell's play is its own landing in the backrow (`summoned`), which the census leaves out; conjure draws it; trigger: the census's bolt is the fatigue in its window, its own effect addedToHand, which send (#2793) reads |
| `traininggrounds-021` | Flame Tongue Totem | Unit | none: its Aura is a standing Attack bonus, with no moment to draw | — | none: vanilla | the census agrees: its Aura changes nothing as it plays |
| `traininggrounds-022` | Right-Hand Felinor | Unit | cry bless Beast/felinor | 1.28 | bolt | cry: the census's bolt is one stray hit in its play window; its Cry's buffed, which bless reads, in Beast's felinor sub-look |
| `traininggrounds-023` | Quadrupling Felinors | Unit | cry conjure Beast/felinor | 1.36 | none: no motif reads summoned | cry: no census family reads summoned, which conjure (#2786) reads, in Beast's felinor sub-look |
| `traininggrounds-024` | Goliath Situation | Field Trap | cast tithe | 1.19 | bolt | cast: the census's bolt is one stray hit; its firing's 152 costChanged are its own, which tithe reads; a Trap's cast plans nothing until firings open a window |
| `traininggrounds-025` | Trigor | Unit | cry offering; death conjure | 1.36 | offering | cry: the census agrees (offering), its Tribute 3; death: the census's bolt is the next Trigor's combat, its own effect that Trigor's summon, which conjure reads |
| `traininggrounds-026` | Path of Exile | Spell | cast vanish | 1.45 | vanish | kept: #2784's example; the census agrees (vanish) |
| `traininggrounds-027` | Hopeful Jelly Bean | Spell | cast radiance | 1.45 | none: no motif reads radiantSet | cast: no census family reads radiantSet, which radiance (#2793) reads |
| `traininggrounds-028` | Occidental | Unit | cry lance | 1.55 | bolt | cry: the census's bolt saw about one card destroyed per Cry, the lane mostly holding one; its Cry clears the whole rightmost lane, which lance draws |
| `traininggrounds-029` | Easterner | Unit | cry lance Hustle/jade | 1.55 | lance | cry: the census agrees (lance); Hustle's jade sub-look (the CN jade line, its CN tag) sets it apart from #28 |
| `traininggrounds-030` | Floodgates | Unit | death wipe; trigger bless | 1.55 | bolt | death: kept, #2800's wipe over its own side (the census's bolt is its destroyed); trigger: the census's vanish is other cards' exiles in its window, its own effect Armor on itself (keywordGranted), which bless reads |
| `traininggrounds-031` | Level Up! | Field Spell | cast conjure; trigger radiance; aura holy side | 1.55 | none: vanilla | cast: a Field Spell's play is its own landing in the backrow (`summoned`), which the census leaves out; conjure draws it; trigger: the census never saw this hook fire; radiance reads its radiantSet |
| `traininggrounds-032` | Selfless Protector | Unit | cry bless; death bless | 1.35 | vanish | cry: its Radiant face's Cry gives its Units Divine Shield (keywordGranted), which bless reads; the base face has no Cry, so it draws nothing there; death: the census's vanish is one stray bounce, against its 16 Divine Shields granted (keywordGranted), which bless reads |
| `traininggrounds-033` | Guerilla Felinor | Unit | cry barrage Beast/felinor | 1.55 | bolt | cry: the census's bolt reads its split as one shot; damage split at random is bolts raining in one by one, which barrage draws, in Beast's felinor sub-look |
| `traininggrounds-034` | Dino-size | Spell | bespoke dinoStomp | 1.55 | none: no motif reads numberChanged | bespoke: no motif reads numberChanged, its one event, and no other card of the set shows one, so no motif joins for it |
| `traininggrounds-035` | Corpse Recycler | Field Spell | cast conjure; trigger conjure; aura void board | 1.55 | vanish | cast: a Field Spell's play is its own landing in the backrow (`summoned`), which the census leaves out; conjure draws it; trigger: the census's vanish is other cards' exiles in its window, its own effect the Rush Token's summon (63), which conjure reads |
| `traininggrounds-036` | Training Dummy | Unit | death send | 1.35 | vanish | death: the census's vanish is other cards' exiles in its window; its own effect adds Radiant cards to the hand (addedToHand), which send reads |
| `traininggrounds-037` | Seance | Spell | bespoke seance | 1.62 | none: no motif reads addedToHand | signature (R3400001): candle flames ring the chosen permanent's tile and lean in as the board dims to plum, a pale spirit rises from it, and its copy drifts to the caster's hand pile as a card back; a face-down target's candles stand at the board's centre (R202) |
| `traininggrounds-038` | Tutorial Master | Unit | bespoke tutorialMaster; aura chalk tile Nerd/chalk | 1.85 | bolt | signature (R3400001): a chalkboard swings down behind his zone, his pointer raps it twice and a chalk outline draws itself around him (entrance chalkboardLesson); each card his Cry or Death adds slides to the hand pile as a card back, the first along a chalk arrow, his outline erasing first at his Death; chalk dust drifts off his tile |
| `traininggrounds-039` | Call to Chaos (Training-Grounds Edition) | Spell | bespoke chaosDice | 1.85 | none: no motif reads keywordGranted | signature (R3400001): two carved wooden dice tumble to the board's centre in magenta and teal sparks; on its roll the top face lights as R436's panel names the clause, its reels and panel shortened to the Legendary Spell's ceiling; the outcome's own rows draw its beat, and a Call it casts rolls smaller dice |
| `traininggrounds-039-1` | Training-Grounds Golem | Unit | signature; entrance golemLashing; aura timber tile | 1.19 | not in census | signature (R3400001), a token printed Legendary: training posts fly in from the yard's edges and lash together in its zone, then its keywords lock on one by one (plank armor, a shield bubble, a palisade for Taunt, Rush, a Pierce splinter); timber dust sifts from its tile |
| `traininggrounds-040` | Goetzmann Game Design | Spell | bespoke gameDesign | 1.60 | none: no motif reads addedToHand | signature (R3400001), the set's Mythic, its designer's card: its takeover is a napkin under canteen lamplight with a card sketched on it and card silhouettes spiralling up; pixels spark as the hold ends, and the card it adds lifts off as a sketch and is dealt into the hand pile as a card back |
| `traininggrounds-041` | Legacy: Speciesist | Unit | cry bolt | 1.16 | bolt | cry: the census agrees (bolt) |
| `traininggrounds-042` | Legacy: Stockpile | Spell | cast flight | 1.08 | bolt | cast: the census's bolt is the fatigue its draws deal; its own effect is drawn, which flight reads |
| `traininggrounds-043` | Legacy: Mr. Vanilla | Unit | none: vanilla | — | none: vanilla | the census agrees: no text |
| `traininggrounds-044` | Legacy: Duplicating Felinors | Unit | cry conjure Beast/felinor | 1.28 | none: no motif reads summoned | cry: no census family reads summoned, which conjure (#2786) reads, in Beast's felinor sub-look |
| `traininggrounds-045` | Legacy: Jlockeed's Weapons | Field Spell | cast conjure; aura storm side | 1.32 | none: vanilla | cast: a Field Spell's play is its own landing in the backrow (`summoned`), which the census leaves out; conjure draws it; aura: its standing Attack, Rush and First Strike over its side; no other card of the set has that effect, so it keeps Core #14's storm, side, in its own palette |
| `traininggrounds-046` | Legacy: Hit Job | Spell | cast bolt | 1.24 | bolt | cast: the census agrees (bolt) |
| `traininggrounds-047` | Legacy: Midrange Menace | Unit | trigger mend | 1.24 | bolt | trigger: the census's bolt is noise in its end-of-turn window; its own effect, the heal on itself, the census leaves out and mend (#2793) reads |
| `traininggrounds-048` | Legacy: Jlockeed Shredder-10 | Unit | trigger volley | 1.24 | volley | trigger: the census agrees (volley), damage to each enemy |
| `traininggrounds-049` | Legacy: Reoccurring Dream | Spell | cast radiance; trigger send | 1.19 | bolt | cast: the census's bolt is its End of turn window's noise; its play's radiantSet, which radiance reads; trigger: its return to the hand (bounced, then addedToHand), which send reads |
| `traininggrounds-050` | Legacy: Glowy Jelly Bean | Spell | cast radiance | 1.36 | none: no motif reads radiantSet | cast: no census family reads radiantSet, which radiance (#2793) reads |
| `traininggrounds-051` | Legacy: Big Felinor | Unit | cry wipe Beast/felinor | 1.45 | wipe | cry: the census agrees (wipe), in Beast's felinor sub-look |
| `traininggrounds-052` | Legacy: Snom Bunny Mind Control | Spell | cast tether | 1.45 | none: no motif reads controlChanged | cast: no census family reads stolen, controlChanged and held, which tether (#2786) reads |
| `traininggrounds-053` | Legacy: Prejudiced Postdoc | Unit | cry conjure | 1.28 | none: no motif reads summoned | cry: no census family reads summoned, which conjure (#2786) reads |
| `traininggrounds-054` | Legacy: The Rock | Unit | cry offering | 1.32 | offering | cry: the census agrees (offering), its Tribute 1 |
| `traininggrounds-055` | Legacy: Reminisce | Spell | cast send | 1.19 | bolt | cast: the census's bolt is one stray hit; its Discover adds the card to the hand (addedToHand), which send reads |
| `traininggrounds-056` | Legacy: Corpse Eater | Unit | none: it grows in the hand, with no board change to draw | — | vanish | the census's vanish is other cards' exiles in its window; it grows in the hand, with no board change to draw |
| `traininggrounds-057` | Legacy: Recycling Initiative | Spell | cast vanish; trigger send | 1.25 | none: vanilla | cast: the census leaves out the events on the card itself, its exile, which vanish reads; trigger: the census never saw this hook fire; send reads its copies added |
| `traininggrounds-058` | Legacy: True Strike | Spell | cast bolt | 1.08 | bolt | cast: the census agrees (bolt) |
| `traininggrounds-059` | Legacy: Fig of Life | Spell | cast mend | 1.24 | none: no motif reads healed | cast: no census family reads healed, which mend (#2793) reads |
| `traininggrounds-060` | Legacy: Rush Token Farm | Field Spell | cast conjure; trigger conjure; aura ember zone | 1.16 | bolt | cast: a Field Spell's play is its own landing in the backrow (`summoned`), which the census leaves out; conjure draws it; trigger: the census's bolt is other cards' draws and fatigue, its own effect the Rush Token's summon, which conjure reads; aura ember zone, apart from #19's bloom |

51 of the 60 non-token cards wear an entry other than `none` (85%, over the 55% floor); all 28 Spells,
Field Spells, Traps and Field Traps have a `cast` or a recipe, and all 6 Field Spells an aura.

## Bespoke

Each built from the motif primitives (`fx/motifs/build.ts`) at the card's motif scale
(`fx/cards/core/flourish.ts`), with no new rendering (`fx/cards/traininggrounds/`), and drawn over the
event rows (`keep`).

| Card | Recipe | Why bespoke | What plays |
|---|---|---|---|
| #16 Celestial Caller | `celestialCall` | its Cry calls a Sun and a Moon at once, in two looks one motif's palette cannot hold | for each summon, a gather at the Caller (its hero when the board does not show it), a beam into the new card's zone, landing in its sky's look and light: Holy's radiant sunburst and a gold afterglow for the Sun, Spirit's shaft of moonlight and a star afterglow for the Moon |
| #34 Dino-size | `dinoStomp` | its one event, `numberChanged`, no motif reads, and no other card of the set shows one | rising arrows over the unit set to 10/10, then a stomp: a ring of dust at its foot, a burst and a ring of the look, and a heavy shake, the dust settling after |
| #40 Goetzmann Game Design | `gameDesign` | the set's Mythic, its designer's own card, which `send`'s small flight undersells | a card drafted at the board's centre in Mythic rays and Nerd's pixel sub-look, dealt into the caster's hand pile, landing in a prismatic burst and a sparkling afterglow |

No Legendary entrance or Mythic takeover is restyled: #37 Seance, #38 Tutorial Master and #39 Call to
Chaos keep the shared Legendary entrance, and #40 Goetzmann Game Design the shared takeover, its flourish
playing after it as the card is added. A takeover look of #40's own would need a field `fx/takeover.ts`
reads, which #2803 is reworking.

## Set-piece candidates

Each takes a motif for now; a later part may give it a set piece.

| Card | Why | Its motif now |
|---|---|---|
| #51 Legacy: Big Felinor | its Cry destroys every non-Felinor Unit, both sides | `cry: wipe` |
| #28 Occidental | its Cry destroys every permanent in the rightmost lane | `cry: lance` |
| #29 Easterner | its Cry destroys every permanent in the leftmost lane | `cry: lance` |
| #30 Floodgates | its Death destroys all its controller's Units | `death: wipe` |
| #39 Call to Chaos (Training-Grounds Edition) | some of its ten random effects take the whole board (six permanents, five Rush Tokens and a rule for the game) | `cast: conjure` |

## New motifs

None: every event three or more Training Grounds cards show has a motif. Core's pass (#2793) drew `send`
(addedToHand, shuffledIn; here #20, #36, #37, #38, #49, #55 and #57), `radiance` (radiantSet; #27, #31,
#49 and #50) and `mend` (healed; #47 and #59). The one event no motif reads, Dino-size's
`numberChanged`, is one card's, so it takes a recipe.
