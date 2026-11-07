// The arena's numbers (#3348, CLAUDE.md rule 9). The client names only its own: how a tile answers a
// press and how a playmat's corner props escalate, roll their rare outcome and time their motion.
// The times that `arena.css` also states (`CORNER_*_MS`, `TILE_FEEL.rippleMs`) are checked against it
// by `arenaCss.test.ts`, so a retune changes both in one change.

/** How a tile answers a press (`SandSurface.tsx`): the ripple's life, the most kept at once and the sand-build each press sounds at. */
export const TILE_FEEL = {
  rippleMs: 520,
  maximum: 12,
  baseBuild: 1,
  legalBuild: 4,
} as const;

/** A prop's repeated taps that make its bigger payoff: the goal is drawn from min to max, inclusive. */
export const CORNER_ESCALATE_TAPS = { min: 5, max: 7 } as const;

/** One tap's chance of the rare outcome, drawn from the client's own random source and never the game's RNG. */
export const CORNER_RARE_CHANCE = 1 / 1000;

/** A prop's taps count as one streak while each follows the last within this. */
export const CORNER_STREAK_GAP_MS = 1500;

/** A hidden sequence is lost when its next tap comes later than this. */
export const CORNER_SEQUENCE_GAP_MS = 4000;

/** How long each outcome's animation runs (`arena.css` states the same numbers). */
export const CORNER_TAP_MS = 380;
export const CORNER_BIG_MS = 900;
export const CORNER_RARE_MS = 1600;
export const CORNER_SEQUENCE_MS = 1400;
/** How long a still change (reduced motion, or the animation queue draining) stays on a prop. */
export const CORNER_STILL_MS = 600;

/** How many cues an escalation sounds: the prop's own, one step up the variants each. */
export const CORNER_ESCALATE_CUES = 3;

/** The gap between the cues of one outcome's sound. */
export const CORNER_ESCALATE_STEP_MS = 70;
export const CORNER_RARE_STEP_MS = 80;
export const CORNER_SEQUENCE_STEP_MS = 90;

/** The sand-build a tap, and a bigger outcome, sounds at (`audio/types.ts` `sandBuild`, up to SAND_FEEL.maximumBuild). */
export const CORNER_TAP_BUILD = 2;
export const CORNER_BIG_BUILD = 6;

/** The recipe's four texture variants. */
export const CORNER_SOUND_VARIANTS = 4;

/** What a prop may not cover: the lanes, the hands, the seats, the controls and the log. */
export const CORNER_OBSTACLES = ".zone, .hand, .seat, .control-bar, .log";
