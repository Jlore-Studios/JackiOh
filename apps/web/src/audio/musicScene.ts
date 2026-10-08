// Which music the page plays (SPEC §10.11 "Music", R631): the main menu theme on every screen
// without a board, and the board's own music while a board is mounted.
//
// `appAudio.ts` turns the menu on for as long as the page-wide audio is held. Each mounted Game
// enters the scene (`useGameAudio`); the newest board's director then decides, and when the last
// board leaves, the menu theme comes back. A card's intro (R1350) is the newest board's alone too:
// an older board asks for none, and a board that leaves cuts the one it started (R1351).

import { getMusicPlayer, type MusicRequest } from "./music.ts";
import type { MusicSink } from "./musicDirector.ts";
import { MENU_TRACK } from "./musicData.ts";

type Claim = { request: MusicRequest | null; intro: boolean };

let menuHolders = 0;
const claims: Claim[] = [];

function apply(): void {
  try {
    const player = getMusicPlayer();
    const game = claims[claims.length - 1];
    if (game !== undefined) {
      // A board that has not asked for anything yet leaves the music as it is.
      if (game.request !== null) player.request(game.request);
      return;
    }
    player.request(menuHolders > 0 ? { track: MENU_TRACK, opponentTurn: false } : { track: null });
  } catch {
    // Music is never worth an error.
  }
}

/** The menu theme plays while anything holds this and no board is mounted. Returns the release. */
export function holdMenuMusic(): () => void {
  menuHolders += 1;
  apply();
  let released = false;
  return () => {
    if (released) return;
    released = true;
    menuHolders -= 1;
    apply();
  };
}

/** A mounted board's way to the music; `leave()` hands it back. */
export function enterGameMusic(): MusicSink & { leave(): void } {
  const claim: Claim = { request: null, intro: false };
  claims.push(claim);
  const holds = (): boolean => claims[claims.length - 1] === claim;
  return {
    request(request) {
      claim.request = request;
      if (holds()) apply();
    },
    playIntro(id) {
      if (!holds()) return;
      claim.intro = true;
      getMusicPlayer().playIntro(id);
    },
    stopIntro() {
      if (!holds() || !claim.intro) return;
      claim.intro = false;
      getMusicPlayer().stopIntro();
    },
    preloadIntros(ids) {
      if (holds()) getMusicPlayer().preloadIntros(ids);
    },
    resetResume() {
      getMusicPlayer().resetResume();
    },
    preload(ids) {
      getMusicPlayer().preload(ids);
    },
    leave() {
      const at = claims.indexOf(claim);
      if (at < 0) return;
      if (holds() && claim.intro) getMusicPlayer().stopIntro();
      claims.splice(at, 1);
      apply();
    },
  };
}

/** How many boards hold the music (tests). */
export function gameMusicClaims(): number {
  return claims.length;
}
