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
