// The music's priority stack (SPEC §10.11 "Music", R631): a pure choice of what plays, from the
// viewer's own situation. Highest wins: the result, low health, then the station's in-game track
// (`chooseMusic`'s order). No card plays a theme (R822000): a Legendary's or Mythic's intro plays over
// the stack (R1350).

import { MENU_TRACK, RESULT_TRACKS, dangerTrack, stationTracks } from "./musicData.ts";
import type { MusicManifest, MusicStation } from "./types.ts";

export type MusicMoment = {
  /** The viewer's station now: one a card switched to this match, else the setting. */
  station: MusicStation;
  /** The match's place in the rotation of each station's in-game tracks. */
  rotation: number;
  dynamic: boolean;
  result: keyof typeof RESULT_TRACKS | null;
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
  if (m.lowHealth) return { track: dangerTrack(m.station), opponentTurn: false };
  return { track: base, opponentTurn: m.opponentTurn };
}
