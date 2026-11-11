// How the arena shows a change, from the viewer's own settings, and the client's own random source.
// Nothing here reads the game (CLAUDE.md rule 7): it is the viewer's screen, and the opponent never
// hears of it (R3348001).

import { getFxSettings } from "../../fx/settings.ts";
import { reducedMotionNow } from "../../settings/motion.ts";

/** "move": animate; "still": show the change without motion; "none": show nothing (the sound stays). */
export type ArenaPlay = "move" | "still" | "none";

/** The effects intensity `off` quiets the arena to a sound; reduced motion keeps its changes still. */
export function arenaPlay(): ArenaPlay {
  if (getFxSettings().intensity === "off") return "none";
  return reducedMotionNow() ? "still" : "move";
}

let source: () => number = Math.random;

/** The client's own random draw in [0, 1): the rare outcomes and escalation goals, never the game's RNG. */
export function arenaRandom(): number {
  return source();
}

/** A test's seeded source in place of `Math.random`; `null` puts the real one back. */
export function setArenaRandomForTests(draw: (() => number) | null): void {
  source = draw ?? Math.random;
}
