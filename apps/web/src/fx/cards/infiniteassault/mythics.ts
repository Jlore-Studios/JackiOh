// R4596000: the cast recipes of the Infinite Assault's eight doctrine Mythics (#48 to #216, each doctrine's
// last card), each its card's signature (R3332000): a Spell's cast of its own, drawn after its takeover's
// hold (R2722001). Each keys on its own `cardPlayed` alone, so it draws whatever its effect turns out to do,
// and leaves what that effect reports to its own rows: a Mythic Spell's signature ends within
// SIGNATURE_MYTHIC_MAX_MS × SIGNATURE_SPELL_SHARE of its play, its hold included, before those rows come.
// - #48 Ego Death ("egoSwap"): two card backs cross between the hand piles in glitch light, which bursts on both.
// - #72 Overclock to Infinity ("overclockSurge"): a reactor over its caster's hero in laser red, fire and a ring of flame.
// - #96 Exponential Growth ("growthFork"): a blooming mirror over the board as its copies come.
// - #120 Eternal Return ("returnLoop"): three rings of circuits close in turn over the board, and a card
//   back flies to its caster's hand pile.
// - #144 Total Compromise ("compromiseBreach"): a virus zaps over the board.
// - #168 Impenetrable ("impenetrableWall"): a fortress over its caster's hero in hologram light, a full
//   shield closing over it.
// - #192 Become the Data ("dataSwap"): a wireframe over the board, two card backs crossing
//   between the hand piles.
// - #216 The Singularity ("singularityCollapse"): the board's centre collapses in void, rings
//   closing on it, before the Singularity lands.
// It anchors on the board, the heroes, the hand piles and the zones the view shows, never on a card's face,
// and flies only card backs (R202).

import type { GameEvent, PlayerId } from "@jackioh/shared";

import { animTestid } from "../../../game/animations.ts";
import { sideOf, testid } from "../../../game/contract.ts";
import { frac, ringCue, shakeCues, tid, tunedBurst } from "../../build.ts";
import { FX_CENTER } from "../../constants.ts";
import { figureCue, figureTint, type FxFigure } from "../../figures.ts";
import type { FxPalette } from "../../flavors.ts";
import type { FxAnchor, FxCue, FxPreset } from "../../types.ts";
import type { CardFxPlan, CardRecipe, RecipeFixture } from "../recipe.ts";
import { lookOf, replace } from "../stinkyscape/signatureKit.ts";

/** Every particle count the eight throw, at intensity "normal" (rule 9). */
export const MYTHIC_TUNING = {
  cast: { count: 30, power: 1.2 },
  beat: { count: 14, power: 0.9 },
} as const;

/** R4596000: the shake each Mythic's cast lands with (within SIGNATURE_SPELL_MAX_TRAUMA, rule 9). */
export const MYTHIC_TRAUMA = {
  egoSwap: 0.6,
  overclockSurge: 0.7,
  growthFork: 0.6,
  returnLoop: 0.5,
  compromiseBreach: 0.7,
  impenetrableWall: 0.5,
  dataSwap: 0.6,
  singularityCollapse: 0.9,
} as const;

/** Where Eternal Return's three rings close, as shares of what is left of its entry after the hold (rule 9). */
const RETURN_RINGS = [0, 0.2, 0.4] as const;

/** When a card back leaves for a hand pile, as a share of what is left of its entry after the hold (rule 9). */
const GHOST_AT = 0.3;

type Key = keyof typeof MYTHIC_TRAUMA;

const centre = (): FxAnchor => tid(testid.board, FX_CENTER);
const heroOf = (p: CardFxPlan, player: PlayerId): FxAnchor => tid(testid.hero(sideOf(p.view, player)));
const handOf = (p: CardFxPlan, player: PlayerId): FxAnchor => tid(animTestid.hand(sideOf(p.view, player)));
const otherOf = (player: PlayerId): PlayerId => (player === "p1" ? "p2" : "p1");

/** R2722001: where in its entry a Mythic's cast starts: after its takeover's hold. */
const holdOf = (p: CardFxPlan): number => p.entry.setPiece?.holdMs ?? 0;

/** A burst of `preset` in `palette`'s colors. */
function burst(p: CardFxPlan, preset: FxPreset, palette: FxPalette, at: FxAnchor, delayMs: number, tuning: { count: number; power: number }, spread: "point" | "area" | "ring" = "area"): FxCue {
  return { ...tunedBurst(p.env.intensity, preset, at, spread, delayMs, tuning), colors: lookOf(palette).colors };
}

/** The cast's first beat: `figure` over `at` in `palette` from the hold to the entry's end, a burst there, and its shake. */
function cast(p: CardFxPlan, key: Key, figure: FxFigure, palette: FxPalette, preset: FxPreset, at: FxAnchor): FxCue[] {
  const hold = holdOf(p);
  return [
    figureCue(figure, at, figureTint(lookOf(palette)), hold, p.D - hold),
    burst(p, preset, palette, at, hold, MYTHIC_TUNING.cast),
    ...shakeCues(p.env.intensity, MYTHIC_TRAUMA[key], hold),
  ];
}

/** Two card backs crossing between the hand piles, leaving at GHOST_AT of what is left after the hold. */
function crossing(p: CardFxPlan, player: PlayerId): FxCue[] {
  const hold = holdOf(p);
  const leave = hold + frac(GHOST_AT, p.D - hold);
  const mine = handOf(p, player);
  const theirs = handOf(p, otherOf(player));
  return [
    { kind: "ghost", from: mine, to: theirs, delayMs: leave, durationMs: p.D - leave },
    { kind: "ghost", from: theirs, to: mine, delayMs: leave, durationMs: p.D - leave },
  ];
}

/** Whether `event` is the play's own `cardPlayed`. */
const ownPlay = (event: GameEvent, play: { instanceId: string }): boolean => event.type === "cardPlayed" && event.instanceId === play.instanceId;

export const egoSwap: CardRecipe = (event, p, play) => {
  if (!ownPlay(event, play)) return null;
  const hold = holdOf(p);
  const piles = [handOf(p, play.player), handOf(p, otherOf(play.player))];
  return replace([
    ...piles.map((at) => burst(p, "circuit", "Tech/glitch", at, hold, MYTHIC_TUNING.beat)),
    ...crossing(p, play.player),
    ...shakeCues(p.env.intensity, MYTHIC_TRAUMA.egoSwap, hold),
  ]);
};

export const overclockSurge: CardRecipe = (event, p, play) => {
  if (!ownPlay(event, play)) return null;
  const hero = heroOf(p, play.player);
  return replace([...cast(p, "overclockSurge", "reactor", "Tech/laser", "fire", hero), ringCue(p.D, "fire", hero, holdOf(p))]);
};

export const growthFork: CardRecipe = (event, p, play) => (ownPlay(event, play) ? replace(cast(p, "growthFork", "mirrorBloom", "Tech/glitch", "circuit", centre())) : null);

export const returnLoop: CardRecipe = (event, p, play) => {
  if (!ownPlay(event, play)) return null;
  const hold = holdOf(p);
  const leave = hold + frac(GHOST_AT, p.D - hold);
  return replace([
    ...RETURN_RINGS.map((at) => ringCue(p.D, "circuit", centre(), hold + frac(at, p.D - hold))),
    burst(p, "circuit", "Tech/glitch", centre(), hold, MYTHIC_TUNING.cast, "ring"),
    { kind: "ghost", from: centre(), to: handOf(p, play.player), delayMs: leave, durationMs: p.D - leave },
    ...shakeCues(p.env.intensity, MYTHIC_TRAUMA.returnLoop, hold),
  ]);
};

export const compromiseBreach: CardRecipe = (event, p, play) => (ownPlay(event, play) ? replace(cast(p, "compromiseBreach", "virusZap", "Tech/glitch", "circuit", centre())) : null);

export const impenetrableWall: CardRecipe = (event, p, play) => {
  if (!ownPlay(event, play)) return null;
  const hero = heroOf(p, play.player);
  const hold = holdOf(p);
  return replace([...cast(p, "impenetrableWall", "fortress", "Tech/hologram", "sparkle", hero), { kind: "shield", size: "full", at: hero, delayMs: hold, durationMs: p.D - hold }]);
};

export const dataSwap: CardRecipe = (event, p, play) => (ownPlay(event, play) ? replace([...cast(p, "dataSwap", "wireframe", "Tech/glitch", "circuit", centre()), ...crossing(p, play.player)]) : null);

export const singularityCollapse: CardRecipe = (event, p, play) => {
  if (!ownPlay(event, play)) return null;
  const hold = holdOf(p);
  return replace([
    burst(p, "void", "Tech/starfield", centre(), hold, MYTHIC_TUNING.cast, "ring"),
    ringCue(p.D, "void", centre(), hold),
    ringCue(p.D, "arcane", centre(), hold),
    ...shakeCues(p.env.intensity, MYTHIC_TRAUMA.singularityCollapse, hold),
  ]);
};

/**
 * R4596000: the gallery's fixture of each (motifs/gallery.ts `RECIPE_FIXTURES`): none past its own play, since
 * each draws on its `cardPlayed` alone.
 */
export const ASSAULT_MYTHIC_FIXTURES: { readonly [K in Key]: RecipeFixture } = {
  egoSwap: () => [],
  overclockSurge: () => [],
  growthFork: () => [],
  returnLoop: () => [],
  compromiseBreach: () => [],
  impenetrableWall: () => [],
  dataSwap: () => [],
  singularityCollapse: () => [],
};

/** The recipes, for bespoke.ts. */
export const ASSAULT_MYTHIC_RECIPES = { egoSwap, overclockSurge, growthFork, returnLoop, compromiseBreach, impenetrableWall, dataSwap, singularityCollapse } as const satisfies Readonly<
  Record<Key, CardRecipe>
>;
