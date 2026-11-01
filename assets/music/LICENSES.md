# Music sources and licenses

Every music track the client plays, where it came from, and what may be done with it (SPEC §10.11
"Music", R631; issue #51). `apps/web/scripts/gen-music.mjs` renders them, and `gen:music --check`
holds the files to their scores and to the size cap.

## How the music was made

#51 asked for free AI-generated music with a license that permits use in the game, and failing
that, royalty-free music fetched from a source that permits it. The music here is the first kind,
and needs no outside license:

- **Composition.** The implementing agent (Claude, Anthropic's model, working as this repository's
  builder) composed every track as code, in `apps/web/scripts/music/tracks.mjs`, on the toolkit in
  `compose.mjs` and `theory.mjs`: keys, tempos, chord progressions, arrangements, and melodies built
  on one shared motif. No melody, recording or sample was taken from another work. A melody's
  ornaments and fills come from a seeded generator, so the same score always renders the same
  music.
- **Instruments.** FluidSynth renders the scores with the FluidR3 General MIDI SoundFont (license
  below). The SoundFont supplies the instrument samples; the rendered audio holds no copy of the
  SoundFont itself.
- **Encoding.** ffmpeg encodes each track to AAC-LC in an M4A file, at 44.1 kHz stereo and 80 kbps.
  Loop points live in `apps/web/src/audio/music-manifest.json`, not at the file's end.
- **Card intros** (R1352, issue #547). Each Legendary and Mythic card, and each token printed
  Legendary or Mythic, has a few bars of its own (`intro-<card id>`), composed the same way: the
  motif per card, derived from its id, set and tags, and hand-tuned for the Mythics (from their
  themes, retired since R822000) and the best-known Legendaries. Each is a sting that plays once and ends on a faded tail.
  R2722000 (issue #2722) makes each an orchestral fanfare around that motif: a riser (timpani, cymbal and tremolo
  strings swelling into the first downbeat), the full ensemble under the melody (brass doubling it, tremolo strings,
  a trombone on the bass, an orchestra hit and a crash), and an ending, the tonic struck by everything and held. A
  Mythic adds its own motif, picked by its id and stated by the brass and a choir, so it runs longer than every
  Legendary's. Every instrument is FluidR3's: no new SoundFont or sample set.

Rejected sources:

- **Open-source music models.** MusicGen's weights are CC-BY-NC 4.0, which does not permit
  commercial use. They also cannot share one motif across tracks.
- **Stock royalty-free tracks.** The issue asks for one recurring motif and instrument palette
  across all of them, which separate stock pieces cannot give.

**License of the tracks:** they belong to the project, like the rest of this repository. No
attribution is required when they play in the game.

## Tracks

| Track | File | What it is | Length |
| --- | --- | --- | --- |
| `menu` | `apps/web/public/audio/music/menu.m4a` | Main menu theme: the JackiOh motif in full, flute and oboe over lute, harp, strings and horns | 160.0 s |
| `tavern-1` | `apps/web/public/audio/music/tavern-1.m4a` | Tavern in-game track 1 (D major): flute and fiddle, lute, harp, hand percussion | 143.5 s |
| `tavern-2` | `apps/web/public/audio/music/tavern-2.m4a` | Tavern in-game track 2 (G mixolydian): fiddle and recorder, lute, accordion | 132.7 s |
| `tavern-danger` | `apps/web/public/audio/music/tavern-danger.m4a` | Tavern low-health track (D minor): faster, cello pulse, busier percussion | 123.5 s |
| `tavern-start` | `apps/web/public/audio/music/tavern-start.m4a` | Tavern match-start sting | 10.0 s |
| `edm-1` | `apps/web/public/audio/music/edm-1.m4a` | EDM in-game track 1 (A minor): saw lead, square plucks, pads, side-chained bass | 123.5 s |
| `edm-2` | `apps/web/public/audio/music/edm-2.m4a` | EDM in-game track 2 (D minor) | 127.9 s |
| `edm-danger` | `apps/web/public/audio/music/edm-danger.m4a` | EDM low-health track (A minor): faster, rolling bass, busier hats | 127.4 s |
| `edm-start` | `apps/web/public/audio/music/edm-start.m4a` | EDM match-start sting | 8.9 s |
| `lofi-1` | `apps/web/public/audio/music/lofi-1.m4a` | Lo-fi in-game track 1 (F major): electric piano, vibraphone, brushed drums, tape wow and crackle | 132.3 s |
| `lofi-2` | `apps/web/public/audio/music/lofi-2.m4a` | Lo-fi in-game track 2 (B-flat major) | 126.3 s |
| `lofi-danger` | `apps/web/public/audio/music/lofi-danger.m4a` | Lo-fi low-health track (F minor): muted trumpet, a low pulse | 123.5 s |
| `lofi-start` | `apps/web/public/audio/music/lofi-start.m4a` | Lo-fi match-start sting | 11.3 s |
| `epic-1` | `apps/web/public/audio/music/epic-1.m4a` | Epic Orchestral in-game track 1 (D minor): string ostinato, horns, choir, timpani | 134.4 s |
| `epic-2` | `apps/web/public/audio/music/epic-2.m4a` | Epic Orchestral in-game track 2 (G minor) | 128.7 s |
| `epic-danger` | `apps/web/public/audio/music/epic-danger.m4a` | Epic Orchestral low-health track (D minor): driving ostinato, brass, timpani | 123.5 s |
| `epic-start` | `apps/web/public/audio/music/epic-start.m4a` | Epic Orchestral match-start sting | 10.7 s |
| `victory` | `apps/web/public/audio/music/victory.m4a` | Victory sting and results loop | 29.9 s |
| `defeat` | `apps/web/public/audio/music/defeat.m4a` | Defeat sting and results loop | 40.2 s |
| `draw` | `apps/web/public/audio/music/draw.m4a` | Draw sting and results loop | 34.9 s |
| `intro-core-052` | `apps/web/public/audio/music/intro-core-052.m4a` | Intro of #52 Silly Silas (Legendary): bassoon, pizzicato, tuba, F major, 4/4 at 126 | 6.9 s |
| `intro-core-083` | `apps/web/public/audio/music/intro-core-083.m4a` | Intro of #83 Transmogulate (Legendary): celesta, strings, cello, A-flat lydian, 4/4 at 104 | 8.1 s |
| `intro-core-085` | `apps/web/public/audio/music/intro-core-085.m4a` | Intro of #85 Unlicensed Experimentation (Legendary): whistle, string tremolo, contrabass, G-flat phrygian, 4/4 at 100 | 8.4 s |
| `intro-core-087` | `apps/web/public/audio/music/intro-core-087.m4a` | Intro of #87 Pocket Chaos (Legendary): xylophone, calliope, tuba, B harmonic minor, 4/4 at 138 | 6.4 s |
| `intro-core-092` | `apps/web/public/audio/music/intro-core-092.m4a` | Intro of #92 Felinor Fiender (Legendary): pizzicato, strings, D dorian, 4/4 at 116 | 7.4 s |
| `intro-core-093` | `apps/web/public/audio/music/intro-core-093.m4a` | Intro of #93 Combo-Index (Legendary): vibraphone, choir, cello, E major, 4/4 at 112 | 7.6 s |
| `intro-core-095` | `apps/web/public/audio/music/intro-core-095.m4a` | Intro of #95 Call to Chaos (Core Edition) (Legendary): calliope, xylophone, tuba, C mixolydian, 4/4 at 138 | 6.4 s |
| `intro-core-096` | `apps/web/public/audio/music/intro-core-096.m4a` | Intro of #96 My Pawn (Mythic): pizzicato, harpsichord, bassoon, E harmonic minor, 4/4 at 104 | 10.4 s |
| `intro-core-097` | `apps/web/public/audio/music/intro-core-097.m4a` | Intro of #97 Zephrys (Mythic): flute, string tremolo, fretless bass, F lydian, 4/4 at 96 | 11.2 s |
| `intro-core-098` | `apps/web/public/audio/music/intro-core-098.m4a` | Intro of #98 Heroic Power (Mythic): trumpet, horn, contrabass, C major, 4/4 at 96 | 11.2 s |
| `intro-core-099` | `apps/web/public/audio/music/intro-core-099.m4a` | Intro of #99 Craft a Card (Mythic): kalimba, marimba, pizzicato, G major, 4/4 at 108 | 10.1 s |
| `intro-core-100` | `apps/web/public/audio/music/intro-core-100.m4a` | Intro of #100 Ceaseless Void (Mythic): celesta, choir, contrabass, C phrygian, 4/4 at 112 | 9.8 s |
| `intro-classic-004` | `apps/web/public/audio/music/intro-classic-004.m4a` | Intro of Classic #4 Palantir (Legendary): synth brass, strings, contrabass, B dorian, 4/4 at 107 | 7.9 s |
| `intro-classic-007` | `apps/web/public/audio/music/intro-classic-007.m4a` | Intro of Classic #7 InfiniScepter (Legendary): oboe, choir, cello, B lydian, 3/4 at 118 | 7.3 s |
| `intro-classic-009` | `apps/web/public/audio/music/intro-classic-009.m4a` | Intro of Classic #9 Income Tax (Legendary): muted trumpet, strings, tuba, G harmonic minor, 4/4 at 116 | 7.4 s |
| `intro-classic-028` | `apps/web/public/audio/music/intro-classic-028.m4a` | Intro of Classic #28 Second Wind (Legendary): clarinet, choir, cello, G lydian, 4/4 at 105 | 8.1 s |
| `intro-classic-033` | `apps/web/public/audio/music/intro-classic-033.m4a` | Intro of Classic #33 Joro (Legendary): harpsichord, pizzicato, cello, D-flat harmonic minor, 4/4 at 108 | 7.9 s |
| `intro-classic-044` | `apps/web/public/audio/music/intro-classic-044.m4a` | Intro of Classic #44 Back from the GY (Legendary): flute, strings, cello, D major, 3/4 at 142 | 6.3 s |
| `intro-classic-045` | `apps/web/public/audio/music/intro-classic-045.m4a` | Intro of Classic #45 Nature Titan (Legendary): horn, strings, contrabass, E-flat mixolydian, 4/4 at 96 | 8.7 s |
| `intro-classic-056` | `apps/web/public/audio/music/intro-classic-056.m4a` | Intro of Classic #56 Spell Tyrant (Legendary): trombone, organ, contrabass, B-flat minor, 4/4 at 104 | 8.1 s |
| `intro-classic-080` | `apps/web/public/audio/music/intro-classic-080.m4a` | Intro of Classic #80 BOOM! Big Max (Legendary): trumpet, brass, contrabass, D harmonic minor, 4/4 at 120 | 7.2 s |
| `intro-classic-085` | `apps/web/public/audio/music/intro-classic-085.m4a` | Intro of Classic #85 King Wagtoggle (Legendary): trumpet, strings, contrabass, D major, 4/4 at 112 | 7.6 s |
| `intro-classic-090` | `apps/web/public/audio/music/intro-classic-090.m4a` | Intro of Classic #90 In Too Deep (Mythic): vibraphone, halo pad, fretless bass, E-flat lydian, 4/4 at 100 | 10.8 s |
| `intro-classicplus-012` | `apps/web/public/audio/music/intro-classicplus-012.m4a` | Intro of Classic+ #12 The Mother Pancake (Legendary): trumpet, brass, tuba, B-flat major, 4/4 at 108 | 7.9 s |
| `intro-classicplus-012-1` | `apps/web/public/audio/music/intro-classicplus-012-1.m4a` | Intro of Classic+ #12.1 Devour (token printed Legendary): trumpet, horn, tuba, B-flat minor, 4/4 at 129 | 6.8 s |
| `intro-classicplus-012-2` | `apps/web/public/audio/music/intro-classicplus-012-2.m4a` | Intro of Classic+ #12.2 Death Boil (token printed Legendary): trumpet, trombone, tuba, A harmonic minor, 4/4 at 126 | 6.9 s |
| `intro-classicplus-012-3` | `apps/web/public/audio/music/intro-classicplus-012-3.m4a` | Intro of Classic+ #12.3 Fluffy Grip (token printed Legendary): trumpet, horn, tuba, A-flat mixolydian, 4/4 at 106 | 8.0 s |
| `intro-classicplus-012-4` | `apps/web/public/audio/music/intro-classicplus-012-4.m4a` | Intro of Classic+ #12.4 Powder Spray (token printed Legendary): trumpet, brass, tuba, E mixolydian, 4/4 at 128 | 6.8 s |
| `intro-classicplus-012-5` | `apps/web/public/audio/music/intro-classicplus-012-5.m4a` | Intro of Classic+ #12.5 Anti-Waffle Shell (token printed Legendary): trumpet, brass, tuba, F lydian, 4/4 at 115 | 7.5 s |
| `intro-classicplus-012-6` | `apps/web/public/audio/music/intro-classicplus-012-6.m4a` | Intro of Classic+ #12.6 Frozen Wastes (token printed Legendary): trumpet, brass, tuba, B-flat minor, 4/4 at 117 | 7.4 s |
| `intro-classicplus-012-7` | `apps/web/public/audio/music/intro-classicplus-012-7.m4a` | Intro of Classic+ #12.7 Legion of the Hungry (token printed Legendary): trumpet, brass, tuba, A mixolydian, 4/4 at 126 | 6.9 s |
| `intro-classicplus-012-8` | `apps/web/public/audio/music/intro-classicplus-012-8.m4a` | Intro of Classic+ #12.8 Frostspatula (token printed Legendary): trumpet, trombone, tuba, G-flat mixolydian, 4/4 at 119 | 7.3 s |
| `intro-classicplus-013` | `apps/web/public/audio/music/intro-classicplus-013.m4a` | Intro of Classic+ #13 Mommy Barker (Legendary): trumpet, brass, tuba, D-flat mixolydian, 4/4 at 122 | 7.1 s |
| `intro-classicplus-019` | `apps/web/public/audio/music/intro-classicplus-019.m4a` | Intro of Classic+ #19 League of Losers (Legendary): synth brass, strings, synth bass, A minor, 4/4 at 104 | 8.1 s |
| `intro-classicplus-019-1` | `apps/web/public/audio/music/intro-classicplus-019-1.m4a` | Intro of Classic+ #19.1 Top Loser (token printed Legendary): trombone, strings, tuba, A minor, 4/4 at 100 | 8.4 s |
| `intro-classicplus-019-2` | `apps/web/public/audio/music/intro-classicplus-019-2.m4a` | Intro of Classic+ #19.2 Jungle Loser (token printed Legendary): pan flute, marimba, double bass, A dorian, 4/4 at 120 | 7.2 s |
| `intro-classicplus-019-3` | `apps/web/public/audio/music/intro-classicplus-019-3.m4a` | Intro of Classic+ #19.3 Mid Loser (token printed Legendary): square lead, polysynth, synth bass, A minor, 4/4 at 128 | 6.8 s |
| `intro-classicplus-019-4` | `apps/web/public/audio/music/intro-classicplus-019-4.m4a` | Intro of Classic+ #19.4 Support Loser (token printed Legendary): flute, choir, cello, A dorian, 4/4 at 100 | 8.4 s |
| `intro-classicplus-019-5` | `apps/web/public/audio/music/intro-classicplus-019-5.m4a` | Intro of Classic+ #19.5 Bot Loser (token printed Legendary): saw lead, square lead, synth bass, A minor, 4/4 at 140 | 6.3 s |
| `intro-classicplus-027` | `apps/web/public/audio/music/intro-classicplus-027.m4a` | Intro of Classic+ #27 Zephrys Zealotism (Mythic): pan flute, organ, contrabass, D dorian, 4/4 at 104 | 10.4 s |
| `intro-classicplus-029` | `apps/web/public/audio/music/intro-classicplus-029.m4a` | Intro of Classic+ #29 Portal to the Past (Mythic): music box, strings, cello, B-flat major, 4/4 at 100 | 10.8 s |
| `intro-classicplus-035` | `apps/web/public/audio/music/intro-classicplus-035.m4a` | Intro of Classic+ #35 Rollback (Legendary): flute, strings, cello, D dorian, 4/4 at 107 | 7.9 s |
| `intro-classicplus-037` | `apps/web/public/audio/music/intro-classicplus-037.m4a` | Intro of Classic+ #37 Wardrum (Legendary): horn, strings, contrabass, D minor, 4/4 at 108 | 7.9 s |
| `intro-classicplus-042` | `apps/web/public/audio/music/intro-classicplus-042.m4a` | Intro of Classic+ #42 KY's Test (Legendary): tubular bells, celesta, fretless bass, A-flat major, 4/4 at 108 | 7.9 s |
| `intro-classicplus-042-1` | `apps/web/public/audio/music/intro-classicplus-042-1.m4a` | Intro of Classic+ #42.1 KY's Gift (token printed Legendary): vibraphone, celesta, fretless bass, E-flat lydian, 4/4 at 110 | 7.7 s |
| `intro-classicplus-043` | `apps/web/public/audio/music/intro-classicplus-043.m4a` | Intro of Classic+ #43 AI Slop (Legendary): square lead, polysynth, synth bass, E minor, 4/4 at 128 | 6.8 s |
| `intro-classicplus-046` | `apps/web/public/audio/music/intro-classicplus-046.m4a` | Intro of Classic+ #46 Felinor Flagbearer (Legendary): pizzicato, strings, E harmonic minor, 4/4 at 129 | 6.8 s |
| `intro-classicplus-046-1` | `apps/web/public/audio/music/intro-classicplus-046-1.m4a` | Intro of Classic+ #46.1 Felinor Flagbearer Prime (token printed Legendary): pizzicato, strings, A major, 4/4 at 149 | 6.0 s |
| `intro-classicplus-047` | `apps/web/public/audio/music/intro-classicplus-047.m4a` | Intro of Classic+ #47 Jogg's Box (Legendary): music box, strings, cello, G major, 4/4 at 112 | 7.6 s |
| `intro-classicplus-048` | `apps/web/public/audio/music/intro-classicplus-048.m4a` | Intro of Classic+ #48 Jlockeed's Lobbyist (Legendary): synth brass, strings, contrabass, E minor, 4/4 at 107 | 7.9 s |
| `intro-classicplus-065-4` | `apps/web/public/audio/music/intro-classicplus-065-4.m4a` | Intro of Classic+ #65.4 Golden Grape (token printed Legendary): marimba, kalimba, double bass, G-flat major, 4/4 at 141 | 6.3 s |
| `intro-classicplus-065-5` | `apps/web/public/audio/music/intro-classicplus-065-5.m4a` | Intro of Classic+ #65.5 Mythic Grape (token printed Mythic): marimba, kalimba, double bass, D lydian, 4/4 at 112 | 9.8 s |
| `intro-classicplus-073` | `apps/web/public/audio/music/intro-classicplus-073.m4a` | Intro of Classic+ #73 Call to Chaos (Classic+ Edition) (Legendary): square lead, xylophone, tuba, E harmonic minor, 4/4 at 144 | 6.2 s |
| `intro-classicplus-073-1` | `apps/web/public/audio/music/intro-classicplus-073-1.m4a` | Intro of Classic+ #73.1 Classic Golem (token printed Legendary): tuba, trombone, contrabass, B-flat minor, 4/4 at 129 | 6.8 s |
| `intro-classicplus-074` | `apps/web/public/audio/music/intro-classicplus-074.m4a` | Intro of Classic+ #74 Twice Forward One Step Backwards (Mythic): clarinet, pizzicato, bassoon, A minor, 3/4 at 132 | 9.4 s |
| `intro-classicplus-075` | `apps/web/public/audio/music/intro-classicplus-075.m4a` | Intro of Classic+ #75 J-lease J-Jungle EX-plorer (Legendary): pan flute, marimba, double bass, F mixolydian, 4/4 at 120 | 7.2 s |
| `intro-classicplus-075-1` | `apps/web/public/audio/music/intro-classicplus-075-1.m4a` | Intro of Classic+ #75.1 J-lease J-Jungle EX-plorer Pack (token printed Legendary): celesta, strings, cello, G dorian, 4/4 at 110 | 7.7 s |
| `intro-classicplus-078` | `apps/web/public/audio/music/intro-classicplus-078.m4a` | Intro of Classic+ #78 Claude's Datacenter (Legendary): FM piano, warm pad, synth bass, C dorian, 4/4 at 116 | 7.4 s |
| `intro-meditative-007` | `apps/web/public/audio/music/intro-meditative-007.m4a` | Intro of Meditative #7 Introspection (Legendary): pan flute, choir, cello, F lydian, 4/4 at 100 | 8.4 s |
| `intro-meditative-008` | `apps/web/public/audio/music/intro-meditative-008.m4a` | Intro of Meditative #8 Reach the Summit (Mythic): pan flute, strings, contrabass, D major, 4/4 at 108 | 10.1 s |
| `intro-meditative-011` | `apps/web/public/audio/music/intro-meditative-011.m4a` | Intro of Meditative #11 Double Header (Legendary): oboe, choir, cello, E-flat major, 4/4 at 104 | 8.1 s |
| `intro-meditative-017` | `apps/web/public/audio/music/intro-meditative-017.m4a` | Intro of Meditative #17 True Craft a Card (Mythic): dulcimer, marimba, pizzicato, A major, 4/4 at 112 | 9.8 s |
| `intro-meditative-020` | `apps/web/public/audio/music/intro-meditative-020.m4a` | Intro of Meditative #20 Aestheticize the Game (Mythic): flute, harp, cello, E-flat major, 3/4 at 112 | 10.8 s |
| `intro-meditative-026` | `apps/web/public/audio/music/intro-meditative-026.m4a` | Intro of Meditative #26 Alternate Fate (Legendary): clarinet, choir, cello, E lydian, 3/4 at 116 | 7.4 s |
| `intro-meditative-039-4` | `apps/web/public/audio/music/intro-meditative-039-4.m4a` | Intro of Meditative #39.4 Red Jade (token printed Mythic): dulcimer, strings, double bass, D dorian, 4/4 at 124 | 10.9 s |
| `intro-meditative-039-5` | `apps/web/public/audio/music/intro-meditative-039-5.m4a` | Intro of Meditative #39.5 Jade Beauty (token printed Mythic): dulcimer, strings, double bass, G-flat dorian, 4/4 at 111 | 9.8 s |
| `intro-meditative-040` | `apps/web/public/audio/music/intro-meditative-040.m4a` | Intro of Meditative #40 Feng Shui (Legendary): dulcimer, strings, double bass, G major, 4/4 at 104 | 8.1 s |
| `intro-meditative-042` | `apps/web/public/audio/music/intro-meditative-042.m4a` | Intro of Meditative #42 CN Flea Market (Legendary): dulcimer, xylophone, double bass, C major, 4/4 at 124 | 7.0 s |
| `intro-meditative-045` | `apps/web/public/audio/music/intro-meditative-045.m4a` | Intro of Meditative #45 Knowledge Breaker (Legendary): dulcimer, strings, double bass, E dorian, 3/4 at 145 | 6.2 s |
| `intro-meditative-045-1` | `apps/web/public/audio/music/intro-meditative-045-1.m4a` | Intro of Meditative #45.1 Knowledge Breaker Prime (token printed Legendary): dulcimer, strings, double bass, C dorian, 3/4 at 118 | 7.3 s |
| `intro-meditative-047` | `apps/web/public/audio/music/intro-meditative-047.m4a` | Intro of Meditative #47 饕餮 (Legendary): tuba, trombone, contrabass, C phrygian, 4/4 at 100 | 8.4 s |
| `intro-meditative-049` | `apps/web/public/audio/music/intro-meditative-049.m4a` | Intro of Meditative #49 YileGPT (Mythic): square lead, polysynth, synth bass, E dorian, 4/4 at 120 | 11.2 s |
| `intro-meditative-049-1` | `apps/web/public/audio/music/intro-meditative-049-1.m4a` | Intro of Meditative #49.1 YileGPT Prime (token printed Mythic): saw lead, polysynth, synth bass, E minor, 4/4 at 140 | 9.8 s |
| `intro-meditative-049-2` | `apps/web/public/audio/music/intro-meditative-049-2.m4a` | Intro of Meditative #49.2 Yile's Virus (token printed Mythic): square lead, polysynth, synth bass, F phrygian, 4/4 at 132 | 10.3 s |
| `intro-meditative-049-3` | `apps/web/public/audio/music/intro-meditative-049-3.m4a` | Intro of Meditative #49.3 AI Girlfriend (token printed Mythic): electric piano, vibraphone, fingered bass, C major, 4/4 at 100 | 10.8 s |
| `intro-meditative-051` | `apps/web/public/audio/music/intro-meditative-051.m4a` | Intro of Meditative #51 Devin Bot (Legendary): square lead, polysynth, synth bass, F dorian, 4/4 at 132 | 6.7 s |
| `intro-meditative-063` | `apps/web/public/audio/music/intro-meditative-063.m4a` | Intro of Meditative #63 Skull of J'Nari (Legendary): oboe, choir, cello, B major, 3/4 at 127 | 6.9 s |
| `intro-meditative-065` | `apps/web/public/audio/music/intro-meditative-065.m4a` | Intro of Meditative #65 Keymaster Keenus (Legendary): trumpet, strings, contrabass, G major, 4/4 at 112 | 7.6 s |
| `intro-meditative-071` | `apps/web/public/audio/music/intro-meditative-071.m4a` | Intro of Meditative #71 Pareto Optimality (Mythic): piccolo, pizzicato, F harmonic minor, 4/4 at 120 | 11.2 s |
| `intro-meditative-071-1` | `apps/web/public/audio/music/intro-meditative-071-1.m4a` | Intro of Meditative #71.1 The Cane (token printed Mythic): dulcimer, strings, double bass, F dorian, 4/4 at 116 | 9.5 s |
| `intro-meditative-075` | `apps/web/public/audio/music/intro-meditative-075.m4a` | Intro of Meditative #75 Ever Growing Tree (Legendary): flute, choir, cello, A-flat major, 4/4 at 102 | 8.3 s |
| `intro-meditative-079` | `apps/web/public/audio/music/intro-meditative-079.m4a` | Intro of Meditative #79 Touched by KY (Legendary): vibraphone, celesta, fretless bass, D-flat lydian, 4/4 at 99 | 8.5 s |
| `intro-meditative-080` | `apps/web/public/audio/music/intro-meditative-080.m4a` | Intro of Meditative #80 Aluneth (Legendary): flute, choir, cello, A major, 4/4 at 100 | 8.4 s |
| `intro-meditative-086` | `apps/web/public/audio/music/intro-meditative-086.m4a` | Intro of Meditative #86 Mayor Medinamogger (Legendary): trumpet, strings, contrabass, D-flat dorian, 4/4 at 117 | 7.4 s |
| `intro-meditative-087` | `apps/web/public/audio/music/intro-meditative-087.m4a` | Intro of Meditative #87 Tatches the Totem (Legendary): dulcimer, strings, double bass, G-flat dorian, 3/4 at 134 | 6.6 s |
| `intro-meditative-095` | `apps/web/public/audio/music/intro-meditative-095.m4a` | Intro of Meditative #95 Call to Chaos (Meditative Edition) (Legendary): dulcimer, xylophone, tuba, D dorian, 4/4 at 140 | 6.3 s |
| `intro-meditative-095-1` | `apps/web/public/audio/music/intro-meditative-095-1.m4a` | Intro of Meditative #95.1 CN Golem (token printed Legendary): tuba, trombone, contrabass, D dorian, 4/4 at 128 | 6.8 s |
| `intro-meditative-097` | `apps/web/public/audio/music/intro-meditative-097.m4a` | Intro of Meditative #97 Jlockeed's Evil Blueprints (Mythic): synth brass, strings, contrabass, C minor, 4/4 at 112 | 9.8 s |
| `intro-meditative-097-8` | `apps/web/public/audio/music/intro-meditative-097-8.m4a` | Intro of Meditative #97.8 Prison (token printed Legendary): trombone, string tremolo, contrabass, B-flat harmonic minor, 4/4 at 105 | 8.1 s |
| `intro-meditative-097-9` | `apps/web/public/audio/music/intro-meditative-097-9.m4a` | Intro of Meditative #97.9 Jlockeed's Headquarters (token printed Mythic): synth brass, strings, contrabass, A-flat dorian, 4/4 at 109 | 10.0 s |
| `intro-meditativeplus-001` | `apps/web/public/audio/music/intro-meditativeplus-001.m4a` | Intro of Meditative+ #1 Bob's Plan (Mythic): pan flute, harp, double bass, D major, 4/4 at 120 | 11.2 s |
| `intro-meditativeplus-024` | `apps/web/public/audio/music/intro-meditativeplus-024.m4a` | Intro of Meditative+ #24 Bob, Wisconsin's Strongest Soldier (Legendary): trumpet, brass, tuba, B-flat major, 4/4 at 112 | 7.6 s |
| `intro-meditativeplus-026` | `apps/web/public/audio/music/intro-meditativeplus-026.m4a` | Intro of Meditative+ #26 Scratch Pad (Mythic): electric piano, marimba, fingered bass, E major, 4/4 at 112 | 9.8 s |
| `intro-stinkyscape-004` | `apps/web/public/audio/music/intro-stinkyscape-004.m4a` | Intro of Stinkyscape #4 Olivian Expanse (Mythic): glockenspiel, strings, cello, E-flat dorian, 4/4 at 120 | 11.2 s |
| `intro-stinkyscape-004-1` | `apps/web/public/audio/music/intro-stinkyscape-004-1.m4a` | Intro of Stinkyscape #4.1 Farewell (Mythic): organ, choir pad, contrabass, A-flat phrygian, 4/4 at 114 | 9.6 s |
| `intro-stinkyscape-004-2` | `apps/web/public/audio/music/intro-stinkyscape-004-2.m4a` | Intro of Stinkyscape #4.2 KY's Last Exam (Mythic): vibes, celesta, fretless bass, D-flat major, 4/4 at 103 | 10.5 s |
| `intro-stinkyscape-004-3` | `apps/web/public/audio/music/intro-stinkyscape-004-3.m4a` | Intro of Stinkyscape #4.3 Event Horizon (Mythic): organ, choir pad, contrabass, B-flat phrygian, 3/4 at 120 | 10.2 s |
| `intro-stinkyscape-004-4` | `apps/web/public/audio/music/intro-stinkyscape-004-4.m4a` | Intro of Stinkyscape #4.4 Last Word (Mythic): organ, choir pad, contrabass, G phrygian, 4/4 at 102 | 10.6 s |
| `intro-stinkyscape-004-5` | `apps/web/public/audio/music/intro-stinkyscape-004-5.m4a` | Intro of Stinkyscape #4.5 1v1 Final Destination (Mythic): organ, choir pad, contrabass, E-flat phrygian, 3/4 at 113 | 10.8 s |
| `intro-stinkyscape-004-6` | `apps/web/public/audio/music/intro-stinkyscape-004-6.m4a` | Intro of Stinkyscape #4.6 The Big Crunch (Mythic): organ, choir pad, contrabass, G-flat phrygian, 3/4 at 110 | 9.4 s |
| `intro-stinkyscape-006` | `apps/web/public/audio/music/intro-stinkyscape-006.m4a` | Intro of Stinkyscape #6 Tylannt (Mythic): horn, strings, contrabass, D dorian, 4/4 at 106 | 10.3 s |
| `intro-stinkyscape-007` | `apps/web/public/audio/music/intro-stinkyscape-007.m4a` | Intro of Stinkyscape #7 The Greatest Heist (Legendary): oboe, choir, cello, E mixolydian, 4/4 at 102 | 8.3 s |
| `intro-stinkyscape-016` | `apps/web/public/audio/music/intro-stinkyscape-016.m4a` | Intro of Stinkyscape #16 Rod of Ages (Mythic): whistle, steel guitar, acoustic bass, G-flat dorian, 4/4 at 141 | 9.7 s |
| `intro-stinkyscape-018` | `apps/web/public/audio/music/intro-stinkyscape-018.m4a` | Intro of Stinkyscape #18 Imitated Instrumentality (Legendary): glockenspiel, strings, cello, F dorian, 4/4 at 119 | 7.3 s |
| `intro-stinkyscape-018-1` | `apps/web/public/audio/music/intro-stinkyscape-018-1.m4a` | Intro of Stinkyscape #18.1 MELCHIOR (Legendary): clarinet, choir, cello, B mixolydian, 4/4 at 110 | 7.7 s |
| `intro-stinkyscape-018-2` | `apps/web/public/audio/music/intro-stinkyscape-018-2.m4a` | Intro of Stinkyscape #18.2 BALTHASAR (Legendary): oboe, choir, cello, C major, 3/4 at 112 | 7.6 s |
| `intro-stinkyscape-018-3` | `apps/web/public/audio/music/intro-stinkyscape-018-3.m4a` | Intro of Stinkyscape #18.3 CASPER (Legendary): oboe, choir, cello, E-flat mixolydian, 3/4 at 113 | 7.6 s |
| `intro-stinkyscape-026` | `apps/web/public/audio/music/intro-stinkyscape-026.m4a` | Intro of Stinkyscape #26 ZolarKeth (Legendary): horn, strings, contrabass, G-flat major, 4/4 at 126 | 6.9 s |
| `intro-stinkyscape-034` | `apps/web/public/audio/music/intro-stinkyscape-034.m4a` | Intro of Stinkyscape #34 Drone Core VIII (Legendary): saw lead, synth strings, synth bass, G-flat phrygian, 4/4 at 149 | 6.0 s |
| `intro-stinkyscape-035` | `apps/web/public/audio/music/intro-stinkyscape-035.m4a` | Intro of Stinkyscape #35 Drone Core IX (Legendary): saw lead, synth strings, synth bass, B minor, 4/4 at 126 | 6.9 s |
| `intro-stinkyscape-036` | `apps/web/public/audio/music/intro-stinkyscape-036.m4a` | Intro of Stinkyscape #36 Drone Core X (Mythic): saw lead, synth strings, synth bass, D-flat phrygian, 4/4 at 148 | 9.3 s |
| `intro-stinkyscape-037` | `apps/web/public/audio/music/intro-stinkyscape-037.m4a` | Intro of Stinkyscape #37 Kwzka (Legendary): horn, strings, contrabass, A dorian, 4/4 at 115 | 7.5 s |
| `intro-stinkyscape-039` | `apps/web/public/audio/music/intro-stinkyscape-039.m4a` | Intro of Stinkyscape #39 The Narcor Project (Legendary): clarinet, choir, cello, C major, 4/4 at 107 | 7.9 s |
| `intro-stinkyscape-044` | `apps/web/public/audio/music/intro-stinkyscape-044.m4a` | Intro of Stinkyscape #44 Great Replacement Theory (Mythic): whistle, steel guitar, acoustic bass, C dorian, 4/4 at 137 | 10.0 s |
| `intro-stinkyscape-058` | `apps/web/public/audio/music/intro-stinkyscape-058.m4a` | Intro of Stinkyscape #58 Beam v5 (Legendary): synth brass, brass, contrabass, B minor, 4/4 at 107 | 7.9 s |
| `intro-stinkyscape-059` | `apps/web/public/audio/music/intro-stinkyscape-059.m4a` | Intro of Stinkyscape #59 Repeater v5 (Legendary): synth brass, brass, contrabass, E-flat harmonic minor, 4/4 at 116 | 7.4 s |
| `intro-stinkyscape-060` | `apps/web/public/audio/music/intro-stinkyscape-060.m4a` | Intro of Stinkyscape #60 Cannon v5 (Legendary): synth brass, brass, contrabass, B harmonic minor, 4/4 at 108 | 7.9 s |
| `intro-stinkyscape-061` | `apps/web/public/audio/music/intro-stinkyscape-061.m4a` | Intro of Stinkyscape #61 Flak v5 (Legendary): synth brass, brass, contrabass, A minor, 4/4 at 116 | 7.4 s |
| `intro-stinkyscape-066` | `apps/web/public/audio/music/intro-stinkyscape-066.m4a` | Intro of Stinkyscape #66 Miner v5 (Legendary): trumpet, strings, contrabass, G dorian, 4/4 at 109 | 7.8 s |
| `intro-stinkyscape-070` | `apps/web/public/audio/music/intro-stinkyscape-070.m4a` | Intro of Stinkyscape #70 Z'Zoth (Legendary): horn, strings, contrabass, E minor, 4/4 at 125 | 7.0 s |
| `intro-stinkyscape-071` | `apps/web/public/audio/music/intro-stinkyscape-071.m4a` | Intro of Stinkyscape #71 A'Zoth (Legendary): trumpet, strings, contrabass, D minor, 4/4 at 127 | 6.9 s |
| `intro-stinkyscape-073` | `apps/web/public/audio/music/intro-stinkyscape-073.m4a` | Intro of Stinkyscape #73 Paragon Mk2 (Legendary): horn, horn, contrabass, B-flat minor, 4/4 at 122 | 7.1 s |
| `intro-stinkyscape-074` | `apps/web/public/audio/music/intro-stinkyscape-074.m4a` | Intro of Stinkyscape #74 Paragon-X (Mythic): trumpet, strings, contrabass, E minor, 4/4 at 114 | 9.6 s |
| `intro-stinkyscape-080` | `apps/web/public/audio/music/intro-stinkyscape-080.m4a` | Intro of Stinkyscape #80 Galaxy (Legendary): horn, horn, contrabass, F minor, 4/4 at 129 | 6.8 s |
| `intro-stinkyscape-085` | `apps/web/public/audio/music/intro-stinkyscape-085.m4a` | Intro of Stinkyscape #85 Prototype-X1 (Legendary): trumpet, horn, contrabass, G dorian, 4/4 at 106 | 8.0 s |
| `intro-stinkyscape-086` | `apps/web/public/audio/music/intro-stinkyscape-086.m4a` | Intro of Stinkyscape #86 Prototype-X2 (Mythic): trumpet, strings, contrabass, C minor, 4/4 at 116 | 9.5 s |
| `intro-stinkyscape-087` | `apps/web/public/audio/music/intro-stinkyscape-087.m4a` | Intro of Stinkyscape #87 Siva 1 (Legendary): pizzicato strings, strings, pizzicato strings, B harmonic minor, 4/4 at 140 | 6.3 s |
| `intro-stinkyscape-088` | `apps/web/public/audio/music/intro-stinkyscape-088.m4a` | Intro of Stinkyscape #88 Siva 2 (Legendary): horn, strings, contrabass, E mixolydian, 4/4 at 124 | 7.0 s |
| `intro-stinkyscape-109` | `apps/web/public/audio/music/intro-stinkyscape-109.m4a` | Intro of Stinkyscape #109 xxFreshJack (Mythic): horn, strings, contrabass, D mixolydian, 4/4 at 117 | 9.4 s |
| `intro-stinkyscape-110` | `apps/web/public/audio/music/intro-stinkyscape-110.m4a` | Intro of Stinkyscape #110 iKwzki (Mythic): horn, strings, contrabass, G-flat major, 4/4 at 105 | 10.3 s |
| `intro-stinkyscape-111` | `apps/web/public/audio/music/intro-stinkyscape-111.m4a` | Intro of Stinkyscape #111 Shockwave447 (Mythic): horn, strings, contrabass, B mixolydian, 4/4 at 112 | 9.8 s |
| `intro-stinkyscape-112` | `apps/web/public/audio/music/intro-stinkyscape-112.m4a` | Intro of Stinkyscape #112 Job Application (Legendary): clarinet, choir, cello, C mixolydian, 3/4 at 110 | 7.7 s |
| `intro-stinkyscape-112-1` | `apps/web/public/audio/music/intro-stinkyscape-112-1.m4a` | Intro of Stinkyscape #112.1 Paycheck (Legendary): flute, strings, cello, B-flat dorian, 4/4 at 109 | 7.8 s |
| `intro-stinkyscape-115` | `apps/web/public/audio/music/intro-stinkyscape-115.m4a` | Intro of Stinkyscape #115 Scornboi (Legendary): horn, strings, contrabass, A dorian, 4/4 at 117 | 7.4 s |
| `intro-stinkyscape-116` | `apps/web/public/audio/music/intro-stinkyscape-116.m4a` | Intro of Stinkyscape #116 Megabase (Legendary): whistle, steel guitar, acoustic bass, B dorian, 4/4 at 129 | 6.8 s |
| `intro-stinkyscape-137` | `apps/web/public/audio/music/intro-stinkyscape-137.m4a` | Intro of Stinkyscape #137 Starfleet Commander (Legendary): horn, strings, contrabass, A dorian, 4/4 at 124 | 7.0 s |
| `intro-stinkyscape-137-1` | `apps/web/public/audio/music/intro-stinkyscape-137-1.m4a` | Intro of Stinkyscape #137.1 Land Doctrine (Legendary): trumpet, horn, tuba, A-flat minor, 4/4 at 121 | 7.2 s |
| `intro-stinkyscape-137-2` | `apps/web/public/audio/music/intro-stinkyscape-137-2.m4a` | Intro of Stinkyscape #137.2 Sea Doctrine (Legendary): trumpet, horn, tuba, D-flat minor, 4/4 at 109 | 7.8 s |
| `intro-stinkyscape-137-3` | `apps/web/public/audio/music/intro-stinkyscape-137-3.m4a` | Intro of Stinkyscape #137.3 Air Doctrine (Legendary): trumpet, horn, tuba, G minor, 4/4 at 111 | 7.7 s |
| `intro-stinkyscape-137-4` | `apps/web/public/audio/music/intro-stinkyscape-137-4.m4a` | Intro of Stinkyscape #137.4 Space Doctrine (Legendary): trumpet, horn, tuba, E-flat minor, 4/4 at 122 | 7.1 s |
| `intro-stinkyscape-147` | `apps/web/public/audio/music/intro-stinkyscape-147.m4a` | Intro of Stinkyscape #147 IN33DH3 (Legendary): horn, strings, contrabass, E-flat dorian, 4/4 at 114 | 7.5 s |
| `intro-stinkyscape-147-1` | `apps/web/public/audio/music/intro-stinkyscape-147-1.m4a` | Intro of Stinkyscape #147.1 IN33DH3 Prime (Legendary): horn, strings, contrabass, B-flat dorian, 4/4 at 107 | 7.9 s |
| `intro-stinkyscape-147-2` | `apps/web/public/audio/music/intro-stinkyscape-147-2.m4a` | Intro of Stinkyscape #147.2 Wardrum Blob (Legendary): clarinet, choir, cello, E mixolydian, 4/4 at 110 | 7.7 s |
| `intro-stinkyscape-148` | `apps/web/public/audio/music/intro-stinkyscape-148.m4a` | Intro of Stinkyscape #148 Alteran Reach (Mythic): whistle, steel guitar, acoustic bass, A dorian, 4/4 at 136 | 10.0 s |
| `intro-stinkyscape-148-1` | `apps/web/public/audio/music/intro-stinkyscape-148-1.m4a` | Intro of Stinkyscape #148.1 Edge of the Universe (Mythic): glockenspiel, strings, cello, D-flat dorian, 4/4 at 116 | 9.5 s |
| `intro-stinkyscape-149` | `apps/web/public/audio/music/intro-stinkyscape-149.m4a` | Intro of Stinkyscape #149 Wildar (Legendary): celesta, strings, cello, A-flat dorian, 4/4 at 119 | 7.3 s |
| `intro-stinkyscape-150` | `apps/web/public/audio/music/intro-stinkyscape-150.m4a` | Intro of Stinkyscape #150 Call to Chaos (Stinky Edition) (Legendary): calliope, xylophone, tuba, A mixolydian, 4/4 at 118 | 7.3 s |
| `intro-stinkyscape-150-1` | `apps/web/public/audio/music/intro-stinkyscape-150-1.m4a` | Intro of Stinkyscape #150.1 Stinkyscape Golem (Legendary): horn, strings, contrabass, A minor, 4/4 at 122 | 7.1 s |
| `intro-stinkyscape-151` | `apps/web/public/audio/music/intro-stinkyscape-151.m4a` | Intro of Stinkyscape #151 Call to Chaos (Scape Edition) (Legendary): calliope, xylophone, tuba, E mixolydian, 4/4 at 131 | 6.7 s |
| `intro-stinkyscape-152` | `apps/web/public/audio/music/intro-stinkyscape-152.m4a` | Intro of Stinkyscape #152 Sanctum (Legendary): oboe, choir, cello, G mixolydian, 3/4 at 119 | 7.3 s |
| `intro-stinkyscape-153` | `apps/web/public/audio/music/intro-stinkyscape-153.m4a` | Intro of Stinkyscape #153 Stimmycheck (Legendary): flute, strings, cello, E dorian, 4/4 at 107 | 7.9 s |
| `intro-stinkyscape-154` | `apps/web/public/audio/music/intro-stinkyscape-154.m4a` | Intro of Stinkyscape #154 Dice of Doom (Mythic): whistle, steel guitar, acoustic bass, B mixolydian, 4/4 at 138 | 9.9 s |
| `intro-stinkyscape-176` | `apps/web/public/audio/music/intro-stinkyscape-176.m4a` | Intro of Stinkyscape #176 Preemptive Strike (Legendary): glockenspiel, strings, cello, B dorian, 4/4 at 129 | 6.8 s |
| `intro-stinkyscape-t-dormant-starship` | `apps/web/public/audio/music/intro-stinkyscape-t-dormant-starship.m4a` | Intro of Stinkyscape Dormant Starship (Legendary): trumpet, strings, contrabass, B-flat major, 4/4 at 113 | 7.6 s |
| `intro-stinkyscape-t-shield-layer` | `apps/web/public/audio/music/intro-stinkyscape-t-shield-layer.m4a` | Intro of Stinkyscape Drone Battleship (Shield Layer) (Mythic): trumpet, strings, contrabass, E-flat minor, 4/4 at 112 | 9.8 s |
| `intro-stinkyscape-t-propagation-layer` | `apps/web/public/audio/music/intro-stinkyscape-t-propagation-layer.m4a` | Intro of Stinkyscape Drone Battleship (Propagation Layer) (Mythic): horn, horn, contrabass, C minor, 4/4 at 125 | 10.8 s |
| `intro-stinkyscape-t-meltdown-layer` | `apps/web/public/audio/music/intro-stinkyscape-t-meltdown-layer.m4a` | Intro of Stinkyscape Drone Battleship (Meltdown Layer) (Mythic): trumpet, horn, contrabass, E-flat major, 4/4 at 109 | 10.0 s |
| `intro-stinkyscape-t-narcor` | `apps/web/public/audio/music/intro-stinkyscape-t-narcor.m4a` | Intro of Stinkyscape Narcor (Legendary): flute, strings, cello, F major, 3/4 at 123 | 7.1 s |
| `intro-stinkyscape-t-red-narcor` | `apps/web/public/audio/music/intro-stinkyscape-t-red-narcor.m4a` | Intro of Stinkyscape Red Narcor (Mythic): flute, strings, cello, D-flat dorian, 4/4 at 127 | 10.6 s |
| `intro-stinkyscape-t-anomaly-found` | `apps/web/public/audio/music/intro-stinkyscape-t-anomaly-found.m4a` | Intro of Stinkyscape Anomaly Found! (Mythic): glockenspiel, strings, cello, F dorian, 4/4 at 107 | 10.2 s |
| `intro-traininggrounds-037` | `apps/web/public/audio/music/intro-traininggrounds-037.m4a` | Intro of Training Grounds #37 Seance (Legendary): celesta, strings, cello, G lydian, 3/4 at 125 | 7.0 s |
| `intro-traininggrounds-038` | `apps/web/public/audio/music/intro-traininggrounds-038.m4a` | Intro of Training Grounds #38 Tutorial Master (Legendary): horn, strings, contrabass, E major, 4/4 at 118 | 7.3 s |
| `intro-traininggrounds-039` | `apps/web/public/audio/music/intro-traininggrounds-039.m4a` | Intro of Training Grounds #39 Call to Chaos (Training-Grounds Edition) (Legendary): calliope, xylophone, tuba, A-flat harmonic minor, 4/4 at 119 | 7.3 s |
| `intro-traininggrounds-039-1` | `apps/web/public/audio/music/intro-traininggrounds-039-1.m4a` | Intro of Training Grounds #39.1 Training-Grounds Golem (token printed Legendary): horn, horns, contrabass, A-flat major, 4/4 at 126 | 6.9 s |
| `intro-traininggrounds-040` | `apps/web/public/audio/music/intro-traininggrounds-040.m4a` | Intro of Training Grounds #40 Goetzmann Game Design (Mythic): whistle, steel guitar, acoustic bass, B mixolydian, 4/4 at 145 | 9.5 s |

## Third-party material

### FluidR3 GM SoundFont

The instrument samples used to render every track. Debian/Ubuntu package `fluid-soundfont-gm`
(`/usr/share/sounds/sf2/FluidR3_GM.sf2`), originally from
<http://www.musescore.org/download/fluid-soundfont.tar.gz>. Only rendered audio is in this
repository, not the SoundFont. Its notice is kept here all the same:

```
Copyright (c) 2000-2002, 2008 Frank Wen <getfrank@gmail.com>
Copyright (c) 2008 Toby Smithe

Permission is hereby granted, free of charge, to any person
obtaining a copy of this software and associated documentation
files (the "Software"), to deal in the Software without
restriction, including without limitation the rights to use,
copy, modify, merge, publish, distribute, sublicense, and/or sell
copies of the Software, and to permit persons to whom the
Software is furnished to do so, subject to the following
conditions:

The above copyright notice and this permission notice shall be
included in all copies or substantial portions of the Software.

THE SOFTWARE IS PROVIDED "AS IS", WITHOUT WARRANTY OF ANY KIND,
EXPRESS OR IMPLIED, INCLUDING BUT NOT LIMITED TO THE WARRANTIES
OF MERCHANTABILITY, FITNESS FOR A PARTICULAR PURPOSE AND
NONINFRINGEMENT. IN NO EVENT SHALL THE AUTHORS OR COPYRIGHT
HOLDERS BE LIABLE FOR ANY CLAIM, DAMAGES OR OTHER LIABILITY,
WHETHER IN AN ACTION OF CONTRACT, TORT OR OTHERWISE, ARISING
FROM, OUT OF OR IN CONNECTION WITH THE SOFTWARE OR THE USE OR
OTHER DEALINGS IN THE SOFTWARE.
```

### Tools

FluidSynth (LGPL-2.1) and ffmpeg (LGPL/GPL) only run on the machine that renders the music. Neither
ships with the game.
