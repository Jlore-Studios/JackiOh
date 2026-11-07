// R2800000: the set pieces of the cards that clear the board, #2782's loudest tier. A card's entry in its
// set's effects table names one (`{ setPiece: "<key>" }`, fx/cards/tables.ts); the runner plays its hook's
// run of removals as one wipe entry (`planEntries`, game/runs.ts `wipeAt`), the queue claims that entry as
// the action's one set piece and holds it (`wipeClaim`, R2785000), and this file draws it inside the
// entry: the board twists into a vortex, or folds in from its edges, every card the wipe takes is drawn
// in and swallowed at the end of the hold, and the board snaps back. It keys on the hook's readable
// `defId` alone (R202): no cue names a card's definition, and a card R97 hides is a card of its side,
// never a guess at which. Every cue starts inside the entry and ends within FX_MAX_TAIL_MS of it (R200),
// a `pull` and a `conceal` excepted, which the board's next view ends; the effects off plan nothing.

import type { Flavor, GameEvent, PlayerId, PlayerView } from "@jackioh/shared";

import { FLAVOR_FALLBACK } from "../cards/flavor.ts";
import { HIDDEN_ID, SET_PIECE_HOLD_MS, locateInstance, type AnimationEntry, type SetPieceClaim } from "../game/animations.ts";
import { sideOf, testid } from "../game/contract.ts";
import { frac, raysCue, ringCue, shakeCues, tid, tunedBurst } from "./build.ts";
import { ENTRANCE_TRAUMA } from "./cards/recipe.ts";
import { FX_CARD_TABLE } from "./cards/tables.ts";
import { FX_HOLD_MAX_MS, FX_MOTIF_CENTER_LANE } from "./constants.ts";
import { FLAVOR_LOOKS, type FlavorLook } from "./flavors.ts";
import { flavorLookOf } from "./flavorSource.ts";
import { isSetPieceKey, type SetPieceKey } from "./setPieceKeys.ts";
import type { FxAnchor, FxCue, FxPlanEnv, FxSetPieceCue, FxTint, SetPieceShape } from "./types.ts";

export { SET_PIECE_KEYS, isSetPieceKey, type SetPieceKey } from "./setPieceKeys.ts";

/** The beat Ceaseless Void's fold asks for at speed 1, the takeover's and the collapse's together (R2785000 caps it). */
export const VOID_FOLD_HOLD_MS = 2_600;
/** Where in its entry a set piece with no hold (Big moments "off") swallows the cards it takes. */
export const SET_PIECE_SWALLOW_AT = 0.6;
/** A set piece's cue lasts its entry and this much more, inside FX_MAX_TAIL_MS (R200). */
export const SET_PIECE_TAIL_MS = 600;

/** Every burst a set piece throws, at intensity "normal" (rule 9). R2803000: eased for the phone frame budget. */
export const SET_PIECE_TUNING = {
  implode: { count: 40, power: 1.6 },
  pull: { count: 6, power: 0.7 },
  light: { count: 40, power: 1.2 },
} as const;

/**
 * One set piece: its shape, the beat it asks for at speed 1, which way its spiral turns, how hard the
 * board shakes at the swallow and the flavor whose colors it wears (absent: the card's own). R3332000:
 * the entrance a Unit whose set piece follows its landing lands with is its table entry's `entrance`.
 */
export type SetPieceSpec = {
  shape: SetPieceShape;
  holdMs: number;
  spin: 1 | -1;
  trauma: number;
  palette?: Flavor;
};

/** R2800000: every set piece by key; a missing one fails `typecheck`. */
export const SET_PIECES: { readonly [K in SetPieceKey]: SetPieceSpec } = {
  // #88 Twisting Nether: the board twists into the Nether and swallows what it destroys.
  // R2803000: eased for the phone frame budget (was 1).
  nether: { shape: "vortex", holdMs: SET_PIECE_HOLD_MS, spin: 1, trauma: 0.7, palette: "Dark" },
  // #100 Ceaseless Void: the board's edges fold into black around it, and it stands alone in the light.
  voidFold: { shape: "fold", holdMs: VOID_FOLD_HOLD_MS, spin: 1, trauma: ENTRANCE_TRAUMA.voidCollapse, palette: "Dark" },
  // Every other board clear: the shared vortex, turning the other way, in the card's own colors.
  vortex: { shape: "vortex", holdMs: SET_PIECE_HOLD_MS, spin: -1, trauma: 0.8 },
};

/** The set piece a readable card's table entry names; none for R97's sentinel (R202). */
export function setPieceKeyOf(defId: string): SetPieceKey | undefined {
  if (defId === HIDDEN_ID) return undefined;
  const entry = FX_CARD_TABLE.get(defId);
  return entry?.kind === "setPiece" ? entry.key : undefined;
}

/** Whether a card's hook plays its removals as a set piece's wipe (the queue's `wipeOf`). */
export function hasSetPiece(defId: string): boolean {
  return setPieceKeyOf(defId) !== undefined;
}

/** R2800000, R2785000: a wipe claims its entry as its card's set piece, asking for that set piece's beat. */
export function wipeClaim(entry: AnimationEntry): SetPieceClaim | null {
  const key = entry.wipe === undefined ? undefined : setPieceKeyOf(entry.wipe.by);
  return key === undefined ? null : { key, holdMs: SET_PIECES[key].holdMs };
}

/** The shape of a held set piece, for the board's `data-set-piece-shape`; none for another (the takeover). */
export function setPieceShapeOf(key: string): SetPieceShape | undefined {
  return isSetPieceKey(key) ? SET_PIECES[key].shape : undefined;
}

type Removal = Extract<GameEvent, { type: "destroyed" | "exiled" | "bounced" }>;

function isRemoval(event: GameEvent): event is Removal {
  return event.type === "destroyed" || event.type === "exiled" || event.type === "bounced";
}

/** The cards of `player`'s side the board shows: its units in lane order, then its face-up backrow. */
function fieldCards(view: PlayerView, player: PlayerId): string[] {
  const sv = view[sideOf(view, player)];
  const units = sv.units.flatMap((unit) => (unit === null ? [] : [testid.card(unit.instanceId)]));
  const backrow = sv.backrow.flatMap((slot) => (slot === null || slot.faceDown ? [] : [testid.card(slot.instanceId)]));
  return [...units, ...backrow];
}

/**
 * R2800000: the cards on the board the wipe takes, as their testids: each card a removal names, then,
 * for each removal R97 hides (a bounce into a hidden hand), the next card of that side not yet taken.
 * How many cards a side lost is public (R202); which hidden one is none of the planner's business.
 */
export function takenOf(entry: AnimationEntry, view: PlayerView): string[] {
  const taken: string[] = [];
  const hidden: PlayerId[] = [];
  for (const event of entry.events) {
    if (!isRemoval(event)) continue;
    if (event.instanceId === HIDDEN_ID) {
      hidden.push(event.type === "destroyed" ? event.controller : event.owner);
      continue;
    }
    const at = locateInstance(view, event.instanceId);
    if (at !== null && at.startsWith("card-") && !taken.includes(at)) taken.push(at);
  }
  for (const player of hidden) {
    const next = fieldCards(view, player).find((id) => !taken.includes(id));
    if (next !== undefined) taken.push(next);
  }
  return taken;
}

/** A look's three colors as a DOM cue's tint (the Starship's launch wears one too, starship.ts). */
export function tintOf(look: FlavorLook): FxTint {
  const [glow, rim, core] = look.colors;
  return { glow, rim, core: core ?? glow };
}

/** A set piece's cues, and the removals it draws in place of their own row recipe. */
export type SetPiecePlan = { cues: FxCue[]; replaces: ReadonlySet<GameEvent> };

/**
 * R2800000: the set piece of a wipe entry, or null: no wipe, a card with no set piece, the effects off
 * or an entry of no duration (Reduce motion). Held (`entry.setPiece`), it swallows at the end of its
 * hold, as the entry's motion starts; unheld (Big moments "off", or a second claim in the action), it
 * plays inside the entry and swallows at SET_PIECE_SWALLOW_AT of it.
 */
export function planSetPiece(entry: AnimationEntry, view: PlayerView, env: Pick<FxPlanEnv, "intensity">): SetPiecePlan | null {
  const wipe = entry.wipe;
  const key = wipe === undefined ? undefined : setPieceKeyOf(wipe.by);
  const i = env.intensity;
  const D = entry.durationMs;
  if (wipe === undefined || key === undefined || !(i > 0) || !(D > 0)) return null;
  const spec = SET_PIECES[key];
  const H = entry.setPiece?.key === key ? entry.setPiece.holdMs : 0;
  const swallow = H > 0 ? H : frac(SET_PIECE_SWALLOW_AT, D);
  const one = wipe.sides.length === 1 ? wipe.sides[0] : undefined;
  // A Trap's or a Unit's own zone (R154); else the middle of the one side it takes from; else the board.
  const centre: FxAnchor =
    wipe.anchor !== null ? tid(wipe.anchor) : one !== undefined ? tid(testid.zone(one, "units", FX_MOTIF_CENTER_LANE)) : tid(testid.board);
  const look = spec.palette !== undefined ? FLAVOR_LOOKS[spec.palette] : (flavorLookOf(wipe.by, wipe.sourceId, view) ?? FLAVOR_LOOKS[FLAVOR_FALLBACK]);
  const fold = spec.shape === "fold";
  const piece: FxSetPieceCue = {
    kind: "setPiece",
    shape: spec.shape,
    tint: tintOf(look),
    spin: spec.spin,
    side: one ?? null,
    at: centre,
    holdMs: swallow,
    delayMs: 0,
    durationMs: D + SET_PIECE_TAIL_MS,
  };
  const cues: FxCue[] = [piece];
  for (const card of takenOf(entry, view)) {
    cues.push(
      { kind: "pull", testid: card, to: fold ? tid(card) : centre, delayMs: 0, durationMs: FX_HOLD_MAX_MS },
      { kind: "conceal", testid: card, mode: "after", delayMs: swallow, durationMs: FX_HOLD_MAX_MS },
      { ...tunedBurst(i, look.particle, tid(card), "point", 0, SET_PIECE_TUNING.pull), colors: look.colors },
    );
  }
  cues.push(
    { ...tunedBurst(i, look.particle, centre, "ring", swallow, SET_PIECE_TUNING.implode), colors: look.colors },
    ringCue(D, look.particle, centre, swallow),
    ...shakeCues(i, spec.trauma, swallow),
  );
  if (fold) cues.push(raysCue(D, "mythic", centre, swallow), tunedBurst(i, "prismatic", centre, "area", swallow, SET_PIECE_TUNING.light));
  return { cues, replaces: new Set(entry.events.filter(isRemoval)) };
}
