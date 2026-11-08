// R3400001: the figures a signature draws (`FxFigureCue`): each a shape fx.css paints over its anchor's
// box in a tint (`.fx-figure[data-figure="…"]`), with no image and no text. A figure is a prop of a
// card's moment (a scroll, a snowplow, a chalkboard, a die), never a card face, so it names no card
// (R202). Meditative+'s and Training Grounds' signatures draw them (fx/cards/meditativeplus/,
// fx/cards/traininggrounds/), R3405001 Meditative's (fx/cards/meditative/) and R3395001 Stinkyscape 001
// to 090's (fx/cards/stinkyscape/).

import type { FlavorLook } from "./flavors.ts";
import type { FxAnchor, FxFigureCue, FxTint } from "./types.ts";

/** Every figure, as fx.css draws it. */
export const FX_FIGURES = [
  "scroll",
  "plow",
  "mitten",
  "scaffold",
  "tear",
  "candles",
  "spirit",
  "dusk",
  "chalkboard",
  "chalkOutline",
  "chalkArrow",
  "erase",
  "die",
  "dieFace",
  "plank",
  "palisade",
  "splinter",
  "sketch",
  // R3405001: Meditative's signatures'.
  "stillPool",
  "breathRing",
  "ledge",
  "peakStar",
  "floodlights",
  "ticketStub",
  "pixelThreads",
  "pixelCrumble",
  "inkStroke",
  "brushSeal",
  "crackedEarth",
  "stoneWell",
  "jadeShard",
  "jadeBoulder",
  "silkFan",
  "baguaCompass",
  "fengBlossom",
  "lanterns",
  "awning",
  "porcelainMask",
  "chalkTendril",
  "maskSplit",
  "chalkWave",
  "cauldron",
  "taotieMaw",
  "inkScholar",
  "inkDrop",
  "brushRing",
  "virusFace",
  "virusZap",
  "wireDojo",
  "hexShield",
  "cushion",
  "wireframe",
  "hornedSkull",
  "eyeFire",
  "keyRing",
  "keyTurn",
  "paretoCurve",
  "caneStreak",
  "bambooCane",
  "sapling",
  "branch",
  "chalkHand",
  "handOutline",
  "lantern",
  "inkBird",
  "bunting",
  "silverCoin",
  "totem",
  "jadeDie",
  "golemBlocks",
  "blueprint",
  "prisonBars",
  "chains",
  "fortress",
  "convoy",
  // R3395001: Stinkyscape 001 to 090's signatures (fx/cards/stinkyscape/), each a prop of its card's concept.
  "vaultDoor",
  "tumbler",
  "lockRings",
  "rod",
  "magiCores",
  "lectern",
  "campus",
  "reactor",
  "press",
  "seedPod",
  "spotlight",
  "labCases",
  "paperclip",
  "mirrorBloom",
  "barrels",
  "cannon",
  "turret",
  "maw",
  "wraith",
  "horn",
  "fighter",
  "blade",
  "flagship",
  "telemetry",
  "forgeShell",
  "carpet",
  "vial",
] as const;

export type FxFigure = (typeof FX_FIGURES)[number];

/** A look's three colors as a figure's tint, the first its glow, as setPieces.ts's `tintOf` tints a set piece. */
export function figureTint(look: FlavorLook): FxTint {
  const [glow, rim, core] = look.colors;
  return { glow, rim, core: core ?? glow };
}

/** R3400001: `figure` over `at` in `tint`, from `delayMs` for `durationMs`; `side` mirrors it for the other seat, `small` draws it smaller. */
export function figureCue(
  figure: FxFigure,
  at: FxAnchor,
  tint: FxTint,
  delayMs: number,
  durationMs: number,
  opts: { side?: "you" | "opponent"; size?: "small" } = {},
): FxFigureCue {
  return {
    kind: "figure",
    figure,
    at,
    tint,
    delayMs,
    durationMs,
    ...(opts.side === undefined ? {} : { side: opts.side }),
    ...(opts.size === undefined ? {} : { size: opts.size }),
  };
}
