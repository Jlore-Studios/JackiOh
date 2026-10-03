// The music's priority stack (SPEC §10.11 "Music", R631): a pure choice of what plays, from the
// viewer's own situation. Highest wins:
//
//   1. the result: the victory, defeat or draw sting and its loop
//   2. a Mythic card's theme
//   3. low health: the station's urgency track
//   4. the station's in-game track, through the opponent's-turn mix while it is not the viewer's turn
//
// With dynamic music off only the station's in-game track plays, and nothing in the game moves it.

import { MENU_TRACK, RESULT_TRACKS, dangerTrack, stationTracks } from "./musicData.ts";
import type { MusicManifest, MusicStation } from "./types.ts";

export type MusicMoment = {
  /** The viewer's station now: one a card switched to this match, else the setting. */
  station: MusicStation;
  /** The match's place in the rotation of each station's in-game tracks. */
  rotation: number;
  dynamic: boolean;
  result: keyof typeof RESULT_TRACKS | null;
  /** The track of the Mythic theme holding the music, or null. */
  theme: string | null;
  lowHealth: boolean;
  /** True while the opponent has the turn in play (the mulligan is nobody's turn). */
  opponentTurn: boolean;
};

export type MusicChoice = { track: string; opponentTurn: boolean };

/** The station's in-game track for this match's rotation, or the menu theme if it has none. */
export function defaultTrack(station: MusicStation, rotation: number, manifest?: MusicManifest): string {
  const tracks = stationTracks(station, manifest);
  return tracks.length === 0 ? MENU_TRACK : (tracks[rotation % tracks.length] ?? MENU_TRACK);
}

export function chooseMusic(m: MusicMoment, manifest?: MusicManifest): MusicChoice {
  const base = defaultTrack(m.station, m.rotation, manifest);
  if (!m.dynamic) return { track: base, opponentTurn: false };
  if (m.result !== null) return { track: RESULT_TRACKS[m.result], opponentTurn: false };
  if (m.theme !== null) return { track: m.theme, opponentTurn: false };
  if (m.lowHealth) return { track: dangerTrack(m.station), opponentTurn: false };
  return { track: base, opponentTurn: m.opponentTurn };
}
