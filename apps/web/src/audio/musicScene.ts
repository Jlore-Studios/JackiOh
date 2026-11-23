// Which music the page plays (SPEC §10.11 "Music", R631): the menu theme on every screen without a
// board (`appAudio.ts` holds it), and the newest mounted board's music (`useGameAudio`) otherwise. A
// card's intro (R1350) is the newest board's alone too: an older board asks for none, and a board that
// leaves cuts the one it started (R1351). So is a card's sting cut from it (R3332002).

import { getMusicPlayer, type MusicRequest } from "./music.ts";
import type { MusicSink } from "./musicDirector.ts";
import { MENU_TRACK } from "./musicData.ts";

type Claim = { request: MusicRequest | null; intro: boolean };

let menuHolders = 0;
const claims: Claim[] = [];
let applyQueued = false;

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

/**
 * R4606000: a board leaving defers the hand-back to a microtask, so an unmount followed by a
 * mount in the same commit (a replay step, a portal flip) passes the music on without asking
 * for the menu theme in between. A real exit asks for it on the microtask.
 */
function scheduleApply(): void {
  if (applyQueued) return;
  applyQueued = true;
  queueMicrotask(() => {
    applyQueued = false;
    apply();
  });
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
    playSting(id) {
      // R3332002: an older board stings no more than it plays intros.
      if (holds()) getMusicPlayer().playSting(id);
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
      scheduleApply();
    },
  };
}

/** How many boards hold the music (tests). */
export function gameMusicClaims(): number {
  return claims.length;
}
