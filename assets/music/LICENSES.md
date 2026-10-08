# Music sources and licences

Every music track the client plays, where it came from, and what may be done with it (SPEC §10.11
"Music", R631; issue #51). `apps/web/scripts/gen-music.mjs` renders them, and `gen:music --check`
holds the files to their scores and to the size cap.

## How the music was made

#51 asked for free AI-generated music with a licence that permits use in the game, and failing
that, royalty-free music fetched from a source that permits it. The music here is the first kind,
and needs no outside licence:

- **Composition.** The implementing agent (Claude, Anthropic's model, working as this repository's
  builder) composed every track as code, in `apps/web/scripts/music/tracks.mjs`, on the toolkit in
  `compose.mjs` and `theory.mjs`: keys, tempos, chord progressions, arrangements, and melodies built
  on one shared motif. No melody, recording or sample was taken from another work. A melody's
  ornaments and fills come from a seeded generator, so the same score always renders the same
  music.
- **Instruments.** FluidSynth renders the scores with the FluidR3 General MIDI SoundFont (licence
  below). The SoundFont supplies the instrument samples; the rendered audio holds no copy of the
  SoundFont itself.
- **Encoding.** ffmpeg encodes each track to AAC-LC in an M4A file, at 44.1 kHz stereo and 80 kbps.
  Loop points live in `apps/web/src/audio/music-manifest.json`, not at the file's end.
- **Card intros** (R1352, issue #547). Each Legendary and Mythic card, and each token printed
  Legendary or Mythic, has a few bars of its own (`intro-<card id>`), composed the same way: the
  motif per card, derived from its id, set and tags, and hand-tuned for the Mythics (from their
  themes) and the best-known Legendaries. Each is a sting that plays once and ends on a faded tail.

Rejected sources:

- **Open-source music models.** MusicGen's weights are CC-BY-NC 4.0, which does not permit
  commercial use. They also cannot share one motif across tracks.
- **Stock royalty-free tracks.** The issue asks for one recurring motif and instrument palette
  across all of them, which separate stock pieces cannot give.

**Licence of the tracks:** they belong to the project, like the rest of this repository. No
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
| `mythic-my-pawn` | `apps/web/public/audio/music/mythic-my-pawn.m4a` | Theme of #96 My Pawn: harpsichord, pizzicato, bassoon | 24.3 s |
| `mythic-zephyrs` | `apps/web/public/audio/music/mythic-zephyrs.m4a` | Theme of #97 Zephyrs: flutes, string tremolo, harp | 27.0 s |
| `mythic-heroic-power` | `apps/web/public/audio/music/mythic-heroic-power.m4a` | Theme of #98 Heroic Power: the motif as a brass fanfare | 26.0 s |
| `mythic-craft-a-card` | `apps/web/public/audio/music/mythic-craft-a-card.m4a` | Theme of #99 Craft a Card: marimba, kalimba, woodblocks | 23.5 s |
| `mythic-ceaseless-void` | `apps/web/public/audio/music/mythic-ceaseless-void.m4a` | Theme of #100 Ceaseless Void: choir, drone, celesta | 34.4 s |
| `mythic-in-too-deep` | `apps/web/public/audio/music/mythic-in-too-deep.m4a` | Theme of Classic #90 In Too Deep: halo pad, harp, vibraphone | 31.9 s |
| `mythic-zephrys-zealotism` | `apps/web/public/audio/music/mythic-zephrys-zealotism.m4a` | Theme of Classic+ #27 Zephrys Zealotism: choir, organ, timpani | 29.2 s |
| `mythic-twice-forward` | `apps/web/public/audio/music/mythic-twice-forward.m4a` | Theme of Classic+ #74 Twice Forward One Step Backwards: a waltz for clarinet and bassoon | 28.0 s |
| `mythic-portal-to-the-past` | `apps/web/public/audio/music/mythic-portal-to-the-past.m4a` | Theme of Classic+ #29 Portal to the Past: music box, viola, strings | 30.5 s |
| `legendary-1` | `apps/web/public/audio/music/legendary-1.m4a` | Shared entrance theme of the Core and Classic Legendaries: a brass fanfare over timpani, celesta glint | 29.0 s |
| `legendary-2` | `apps/web/public/audio/music/legendary-2.m4a` | Shared entrance theme of the Classic+ Legendaries: a brass fanfare over timpani, piccolo glint | 29.0 s |
| `intro-core-052` | `apps/web/public/audio/music/intro-core-052.m4a` | Intro of #52 Silly Silas (Legendary): bassoon, pizzicato, tuba, F major, 4/4 at 126 | 4.8 s |
| `intro-core-083` | `apps/web/public/audio/music/intro-core-083.m4a` | Intro of #83 Transmogulate (Legendary): celesta, strings, cello, A-flat lydian, 4/4 at 104 | 5.6 s |
| `intro-core-085` | `apps/web/public/audio/music/intro-core-085.m4a` | Intro of #85 Unlicensed Experimentation (Legendary): whistle, string tremolo, contrabass, G-flat phrygian, 4/4 at 100 | 5.8 s |
| `intro-core-087` | `apps/web/public/audio/music/intro-core-087.m4a` | Intro of #87 Pocket Chaos (Legendary): xylophone, calliope, tuba, B harmonic minor, 4/4 at 138 | 4.5 s |
| `intro-core-092` | `apps/web/public/audio/music/intro-core-092.m4a` | Intro of #92 Felinor Fiender (Legendary): pizzicato, strings, D dorian, 4/4 at 116 | 5.1 s |
| `intro-core-093` | `apps/web/public/audio/music/intro-core-093.m4a` | Intro of #93 Combo-Index (Legendary): vibraphone, choir, cello, E major, 4/4 at 112 | 5.3 s |
| `intro-core-095` | `apps/web/public/audio/music/intro-core-095.m4a` | Intro of #95 Call to Chaos (Core Edition) (Legendary): calliope, xylophone, tuba, C mixolydian, 4/4 at 138 | 4.5 s |
| `intro-core-096` | `apps/web/public/audio/music/intro-core-096.m4a` | Intro of #96 My Pawn (Mythic): pizzicato, harpsichord, bassoon, E harmonic minor, 4/4 at 104 | 5.6 s |
| `intro-core-097` | `apps/web/public/audio/music/intro-core-097.m4a` | Intro of #97 Zephyrs (Mythic): flute, string tremolo, fretless bass, F lydian, 4/4 at 96 | 6.0 s |
| `intro-core-098` | `apps/web/public/audio/music/intro-core-098.m4a` | Intro of #98 Heroic Power (Mythic): trumpet, horn, contrabass, C major, 4/4 at 96 | 6.0 s |
| `intro-core-099` | `apps/web/public/audio/music/intro-core-099.m4a` | Intro of #99 Craft a Card (Mythic): kalimba, marimba, pizzicato, G major, 4/4 at 108 | 5.4 s |
| `intro-core-100` | `apps/web/public/audio/music/intro-core-100.m4a` | Intro of #100 Ceaseless Void (Mythic): celesta, choir, contrabass, C phrygian, 4/4 at 112 | 5.3 s |
| `intro-classic-004` | `apps/web/public/audio/music/intro-classic-004.m4a` | Intro of Classic #4 Palantir (Legendary): synth brass, strings, contrabass, B dorian, 4/4 at 107 | 5.5 s |
| `intro-classic-007` | `apps/web/public/audio/music/intro-classic-007.m4a` | Intro of Classic #7 InfiniScepter (Legendary): oboe, choir, cello, B lydian, 3/4 at 118 | 5.6 s |
| `intro-classic-009` | `apps/web/public/audio/music/intro-classic-009.m4a` | Intro of Classic #9 Income Tax (Legendary): muted trumpet, strings, tuba, G harmonic minor, 4/4 at 116 | 5.1 s |
| `intro-classic-028` | `apps/web/public/audio/music/intro-classic-028.m4a` | Intro of Classic #28 Second Wind (Legendary): clarinet, choir, cello, G lydian, 4/4 at 105 | 5.6 s |
| `intro-classic-033` | `apps/web/public/audio/music/intro-classic-033.m4a` | Intro of Classic #33 Joro (Legendary): harpsichord, pizzicato, cello, D-flat harmonic minor, 4/4 at 108 | 5.4 s |
| `intro-classic-044` | `apps/web/public/audio/music/intro-classic-044.m4a` | Intro of Classic #44 Back from the GY (Legendary): flute, strings, cello, D major, 3/4 at 142 | 4.8 s |
| `intro-classic-045` | `apps/web/public/audio/music/intro-classic-045.m4a` | Intro of Classic #45 Nature Titan (Legendary): horn, strings, contrabass, E-flat mixolydian, 4/4 at 96 | 6.0 s |
| `intro-classic-056` | `apps/web/public/audio/music/intro-classic-056.m4a` | Intro of Classic #56 Spell Tyrant (Legendary): trombone, organ, contrabass, B-flat minor, 4/4 at 104 | 5.6 s |
| `intro-classic-080` | `apps/web/public/audio/music/intro-classic-080.m4a` | Intro of Classic #80 BOOM! Big Max (Legendary): trumpet, brass, contrabass, D harmonic minor, 4/4 at 120 | 5.0 s |
| `intro-classic-085` | `apps/web/public/audio/music/intro-classic-085.m4a` | Intro of Classic #85 King Wagtoggle (Legendary): trumpet, strings, contrabass, D major, 4/4 at 112 | 5.3 s |
| `intro-classic-090` | `apps/web/public/audio/music/intro-classic-090.m4a` | Intro of Classic #90 In Too Deep (Mythic): vibraphone, halo pad, fretless bass, E-flat lydian, 4/4 at 100 | 5.8 s |
| `intro-classicplus-012` | `apps/web/public/audio/music/intro-classicplus-012.m4a` | Intro of Classic+ #12 The Mother Pancake (Legendary): trumpet, brass, tuba, B-flat major, 4/4 at 108 | 5.4 s |
| `intro-classicplus-012-1` | `apps/web/public/audio/music/intro-classicplus-012-1.m4a` | Intro of Classic+ #12.1 Devour (token printed Legendary): trumpet, horn, tuba, B-flat minor, 4/4 at 129 | 4.7 s |
| `intro-classicplus-012-2` | `apps/web/public/audio/music/intro-classicplus-012-2.m4a` | Intro of Classic+ #12.2 Death Boil (token printed Legendary): trumpet, trombone, tuba, A harmonic minor, 4/4 at 126 | 4.8 s |
| `intro-classicplus-012-3` | `apps/web/public/audio/music/intro-classicplus-012-3.m4a` | Intro of Classic+ #12.3 Fluffy Grip (token printed Legendary): trumpet, horn, tuba, A-flat mixolydian, 4/4 at 106 | 5.5 s |
| `intro-classicplus-012-4` | `apps/web/public/audio/music/intro-classicplus-012-4.m4a` | Intro of Classic+ #12.4 Powder Spray (token printed Legendary): trumpet, brass, tuba, E mixolydian, 4/4 at 128 | 4.8 s |
| `intro-classicplus-012-5` | `apps/web/public/audio/music/intro-classicplus-012-5.m4a` | Intro of Classic+ #12.5 Anti-Waffle Shell (token printed Legendary): trumpet, brass, tuba, F lydian, 4/4 at 115 | 5.2 s |
| `intro-classicplus-012-6` | `apps/web/public/audio/music/intro-classicplus-012-6.m4a` | Intro of Classic+ #12.6 Frozen Wastes (token printed Legendary): trumpet, brass, tuba, B-flat minor, 4/4 at 117 | 5.1 s |
| `intro-classicplus-012-7` | `apps/web/public/audio/music/intro-classicplus-012-7.m4a` | Intro of Classic+ #12.7 Legion of the Hungry (token printed Legendary): trumpet, brass, tuba, A mixolydian, 4/4 at 126 | 4.8 s |
| `intro-classicplus-012-8` | `apps/web/public/audio/music/intro-classicplus-012-8.m4a` | Intro of Classic+ #12.8 Frostspatula (token printed Legendary): trumpet, trombone, tuba, G-flat mixolydian, 4/4 at 119 | 5.0 s |
| `intro-classicplus-013` | `apps/web/public/audio/music/intro-classicplus-013.m4a` | Intro of Classic+ #13 Mommy Barker (Legendary): trumpet, brass, tuba, D-flat mixolydian, 4/4 at 122 | 4.9 s |
| `intro-classicplus-019` | `apps/web/public/audio/music/intro-classicplus-019.m4a` | Intro of Classic+ #19 League of Losers (Legendary): synth brass, strings, synth bass, A minor, 4/4 at 104 | 5.6 s |
| `intro-classicplus-019-1` | `apps/web/public/audio/music/intro-classicplus-019-1.m4a` | Intro of Classic+ #19.1 Top Loser (token printed Legendary): trombone, strings, tuba, A minor, 4/4 at 100 | 5.8 s |
| `intro-classicplus-019-2` | `apps/web/public/audio/music/intro-classicplus-019-2.m4a` | Intro of Classic+ #19.2 Jungle Loser (token printed Legendary): pan flute, marimba, double bass, A dorian, 4/4 at 120 | 5.0 s |
| `intro-classicplus-019-3` | `apps/web/public/audio/music/intro-classicplus-019-3.m4a` | Intro of Classic+ #19.3 Mid Loser (token printed Legendary): square lead, polysynth, synth bass, A minor, 4/4 at 128 | 4.8 s |
| `intro-classicplus-019-4` | `apps/web/public/audio/music/intro-classicplus-019-4.m4a` | Intro of Classic+ #19.4 Support Loser (token printed Legendary): flute, choir, cello, A dorian, 4/4 at 100 | 5.8 s |
| `intro-classicplus-019-5` | `apps/web/public/audio/music/intro-classicplus-019-5.m4a` | Intro of Classic+ #19.5 Bot Loser (token printed Legendary): saw lead, square lead, synth bass, A minor, 4/4 at 140 | 4.4 s |
| `intro-classicplus-027` | `apps/web/public/audio/music/intro-classicplus-027.m4a` | Intro of Classic+ #27 Zephrys Zealotism (Mythic): pan flute, organ, contrabass, D dorian, 4/4 at 104 | 5.6 s |
| `intro-classicplus-029` | `apps/web/public/audio/music/intro-classicplus-029.m4a` | Intro of Classic+ #29 Portal to the Past (Mythic): music box, strings, cello, B-flat major, 4/4 at 100 | 5.8 s |
| `intro-classicplus-035` | `apps/web/public/audio/music/intro-classicplus-035.m4a` | Intro of Classic+ #35 Rollback (Legendary): flute, strings, cello, D dorian, 4/4 at 107 | 5.5 s |
| `intro-classicplus-037` | `apps/web/public/audio/music/intro-classicplus-037.m4a` | Intro of Classic+ #37 Wardrum (Legendary): horn, strings, contrabass, D minor, 4/4 at 108 | 5.4 s |
| `intro-classicplus-042` | `apps/web/public/audio/music/intro-classicplus-042.m4a` | Intro of Classic+ #42 KY's Test (Legendary): tubular bells, celesta, fretless bass, A-flat major, 4/4 at 108 | 5.4 s |
| `intro-classicplus-042-1` | `apps/web/public/audio/music/intro-classicplus-042-1.m4a` | Intro of Classic+ #42.1 KY's Gift (token printed Legendary): vibraphone, celesta, fretless bass, E-flat lydian, 4/4 at 110 | 5.4 s |
| `intro-classicplus-043` | `apps/web/public/audio/music/intro-classicplus-043.m4a` | Intro of Classic+ #43 AI Slop (Legendary): square lead, polysynth, synth bass, E minor, 4/4 at 128 | 4.8 s |
| `intro-classicplus-046` | `apps/web/public/audio/music/intro-classicplus-046.m4a` | Intro of Classic+ #46 Felinor Flagbearer (Legendary): pizzicato, strings, E harmonic minor, 4/4 at 129 | 4.7 s |
| `intro-classicplus-046-1` | `apps/web/public/audio/music/intro-classicplus-046-1.m4a` | Intro of Classic+ #46.1 Felinor Flagbearer Prime (token printed Legendary): pizzicato, strings, A major, 4/4 at 149 | 4.2 s |
| `intro-classicplus-047` | `apps/web/public/audio/music/intro-classicplus-047.m4a` | Intro of Classic+ #47 Jogg's Box (Legendary): music box, strings, cello, G major, 4/4 at 112 | 5.3 s |
| `intro-classicplus-048` | `apps/web/public/audio/music/intro-classicplus-048.m4a` | Intro of Classic+ #48 Jlockheed's Lobbyist (Legendary): synth brass, strings, contrabass, E minor, 4/4 at 107 | 5.5 s |
| `intro-classicplus-065-4` | `apps/web/public/audio/music/intro-classicplus-065-4.m4a` | Intro of Classic+ #65.4 Golden Grape (token printed Legendary): marimba, kalimba, double bass, G-flat major, 4/4 at 141 | 4.4 s |
| `intro-classicplus-065-5` | `apps/web/public/audio/music/intro-classicplus-065-5.m4a` | Intro of Classic+ #65.5 Mythic Grape (token printed Mythic): marimba, kalimba, double bass, D lydian, 4/4 at 112 | 5.3 s |
| `intro-classicplus-073` | `apps/web/public/audio/music/intro-classicplus-073.m4a` | Intro of Classic+ #73 Call to Chaos (Classic+ Edition) (Legendary): square lead, xylophone, tuba, E harmonic minor, 4/4 at 144 | 4.3 s |
| `intro-classicplus-073-1` | `apps/web/public/audio/music/intro-classicplus-073-1.m4a` | Intro of Classic+ #73.1 Classic Golem (token printed Legendary): tuba, trombone, contrabass, B-flat minor, 4/4 at 129 | 4.7 s |
| `intro-classicplus-074` | `apps/web/public/audio/music/intro-classicplus-074.m4a` | Intro of Classic+ #74 Twice Forward One Step Backwards (Mythic): clarinet, pizzicato, bassoon, A minor, 3/4 at 132 | 5.1 s |
| `intro-classicplus-075` | `apps/web/public/audio/music/intro-classicplus-075.m4a` | Intro of Classic+ #75 J-lease J-Jungle EX-plorer (Legendary): pan flute, marimba, double bass, F mixolydian, 4/4 at 120 | 5.0 s |
| `intro-classicplus-075-1` | `apps/web/public/audio/music/intro-classicplus-075-1.m4a` | Intro of Classic+ #75.1 J-lease J-Jungle EX-plorer Pack (token printed Legendary): celesta, strings, cello, G dorian, 4/4 at 110 | 5.4 s |
| `intro-classicplus-078` | `apps/web/public/audio/music/intro-classicplus-078.m4a` | Intro of Classic+ #78 Claude's Datacenter (Legendary): FM piano, warm pad, synth bass, C dorian, 4/4 at 116 | 5.1 s |

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
