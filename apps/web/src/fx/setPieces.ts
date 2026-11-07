// R2800000: the set pieces of the cards that clear the board, #2782's loudest tier. A card's entry in its
// set's effects table names one (`{ setPiece: "<key>" }`, fx/cards/tables.ts); the runner plays its hook's
// run of removals as one wipe entry (`planEntries`, game/runs.ts `wipeAt`), the queue claims that entry as
// the action's one set piece and holds it (`wipeClaim`, R2785000), and this file draws it inside the
// entry: the board twists into a vortex, or folds in from its edges, every card the wipe takes is drawn
// in and swallowed at the end of the hold, and the board snaps back. It keys on the hook's readable
// `defId` alone (R202): no cue names a card's definition, and a card R97 hides is a card of its side,
// never a guess at which. Every cue starts inside the entry and ends within FX_MAX_TAIL_MS of it (R200),
// a `pull` and a `conceal` excepted, which the board's next view ends; the effects off plan nothing.
// R3338000: the Terminus cards' set pieces join them, each its card's own `moment`: a board clear's
// (Ceaseless Void's fold, Farewell's lamp) is a wipe as ever, drawn here; every other is a signature, one
// run of its hook's events (`signatureRunOf`, game/runs.ts `signatureRunAt`) or its first cast, which
// `signatureClaim` holds and fx/terminus/ draws. The fold now draws each card into the Void's chest in
// turn, as its removals say, and settles on its 10/10 burning in the dark.

import type { Flavor, GameEvent, GameEventType, PlayerId, PlayerView } from "@jackioh/shared";

import { FLAVOR_FALLBACK } from "../cards/flavor.ts";
import { HIDDEN_ID, SET_PIECE_HOLD_MS, locateInstance, pileTestid, type AnimationEntry, type SetPieceClaim } from "../game/animations.ts";
import { sideOf, testid } from "../game/contract.ts";
import { pileOf } from "../game/runs.ts";
import { frac, raysCue, ringCue, shakeCues, tid, tunedBurst } from "./build.ts";
import { ENTRANCE_TRAUMA, type EntranceKey } from "./cards/recipe.ts";
import { FX_CARD_TABLE } from "./cards/tables.ts";
import {
  FX_HOLD_MAX_MS,
  FX_MOTIF_CENTER_LANE,
  FX_TERMINUS_BEATS_FROM,
  FX_TERMINUS_BEATS_TO,
  FX_TERMINUS_LAMP_AT,
  FX_TERMINUS_LIFT_MAX,
  FX_TERMINUS_STATS_AT,
  FX_TERMINUS_TRAUMA,
} from "./constants.ts";
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
  // R3338000: the Void's 10/10 burning once the fold settles, and Farewell's slow exhale.
  stats: { count: 14, power: 0.6 },
  exhale: { count: 24, power: 0.4 },
} as const;

/**
 * R3338000: the beat each Terminus card's set piece asks for at speed 1 (`setPieceHoldMs` scales and caps
 * it at SET_PIECE_HOLD_CAP_MS); Ceaseless Void's is VOID_FOLD_HOLD_MS.
 */
export const TERMINUS_HOLD_MS = {
  terminusRift: 2_400,
  farewell: 2_800,
  lastExam: 2_400,
  eventHorizon: 2_400,
  lastWord: 2_200,
  finalDestination: 2_800,
  bigCrunch: 2_000,
  diceOfDoom: 2_800,
} as const;
/** R3338000: the shorter beat Dice of Doom's roll asks for when it misses (no Terminus card rains). */
export const DOOM_MISS_HOLD_MS = 1_200;

/**
 * R3338000: what a set piece plays over: a board clear's run of removals (`"wipe"`, R2800000), its card's
 * first cast (`"cast"`), or the run of its hook's events of these types, opened by the first type listed.
 */
export type SetPieceMoment = "wipe" | "cast" | readonly GameEventType[];

/**
 * One set piece: its shape, the beat it asks for at speed 1, which way its spiral turns, how hard the
 * board shakes at the swallow, the flavor whose colors it wears (absent: the card's own) and the
 * bespoke entrance its card lands with (R670), for a Unit whose set piece follows its landing.
 */
export type SetPieceSpec = {
  shape: SetPieceShape;
  moment: SetPieceMoment;
  holdMs: number;
  spin: 1 | -1;
  trauma: number;
  palette?: Flavor;
  entrance?: EntranceKey;
};

/** R2800000: every set piece by key; a missing one fails `typecheck`. */
export const SET_PIECES: { readonly [K in SetPieceKey]: SetPieceSpec } = {
  // #88 Twisting Nether: the board twists into the Nether and swallows what it destroys.
  // R2803000: eased for the phone frame budget (was 1).
  nether: { shape: "vortex", moment: "wipe", holdMs: SET_PIECE_HOLD_MS, spin: 1, trauma: 0.7, palette: "Dark" },
  // #100 Ceaseless Void: the board's edges fold into black around it, every other permanent is drawn
  // into its chest in turn, and it stands alone with its 10/10 burning in the dark (R3338000).
  voidFold: { shape: "fold", moment: "wipe", holdMs: VOID_FOLD_HOLD_MS, spin: 1, trauma: ENTRANCE_TRAUMA.voidCollapse, palette: "Dark", entrance: "voidCollapse" },
  // Every other board clear: the shared vortex, turning the other way, in the card's own colors.
  vortex: { shape: "vortex", moment: "wipe", holdMs: SET_PIECE_HOLD_MS, spin: -1, trauma: 0.8 },
  // R3338000, S #4 Olivian Expanse: the rift opens, the Terminus cards circle in it as stars, one falls to the hand.
  terminusRift: { shape: "rift", moment: ["addedToHand"], holdMs: TERMINUS_HOLD_MS.terminusRift, spin: 1, trauma: FX_TERMINUS_TRAUMA, palette: "Dark" },
  // S #4.1 Farewell: a lamp swings over the table, and every card it exiles lifts into its light and fades.
  farewell: { shape: "lamp", moment: "wipe", holdMs: TERMINUS_HOLD_MS.farewell, spin: 1, trauma: FX_TERMINUS_TRAUMA, palette: "Dark" },
  // S #4.2 KY's Last Exam: the impossible problem chalks itself across the board; the answer is the bolt.
  lastExam: { shape: "chalk", moment: ["damage"], holdMs: TERMINUS_HOLD_MS.lastExam, spin: 1, trauma: FX_TERMINUS_TRAUMA, palette: "Dark" },
  // S #4.3 Event Horizon: time bends, the board smearing to its centre under a stretched clock.
  eventHorizon: { shape: "bend", moment: ["modifierChanged"], holdMs: TERMINUS_HOLD_MS.eventHorizon, spin: 1, trauma: FX_TERMINUS_TRAUMA, palette: "Dark" },
  // S #4.4 Last Word: everything freezes in mid-strike as the lethal hit is prevented, and the turn ends
  // (the run is its firing, its own way to the graveyard and the turns it rearranges; the cut turn's banner follows).
  lastWord: { shape: "freeze", moment: ["trapFired", "enteredGraveyard", "modifierChanged"], holdMs: TERMINUS_HOLD_MS.lastWord, spin: 1, trauma: FX_TERMINUS_TRAUMA, palette: "Dark" },
  // S #4.5 1v1 Final Destination: every lane but one chains shut over the abyss, both heroes spin to 1.
  finalDestination: {
    shape: "abyss",
    moment: ["trapFired", "locked", "healthSet", "discarded", "enteredGraveyard", "discardPrevented"],
    holdMs: TERMINUS_HOLD_MS.finalDestination,
    spin: 1,
    trauma: FX_TERMINUS_TRAUMA,
    palette: "Dark",
  },
  // S #4.6 The Big Crunch: the board collapses to a bright point, which casts each Call to Chaos.
  bigCrunch: { shape: "crunch", moment: "cast", holdMs: TERMINUS_HOLD_MS.bigCrunch, spin: 1, trauma: FX_TERMINUS_TRAUMA, palette: "Dark" },
  // S #154 Dice of Doom: the d100 tumbles across the board onto its roll; on a 67 the Terminus cards rain.
  diceOfDoom: {
    shape: "doom",
    // A burned card lands in the graveyard (`enteredGraveyard`), inside the run (crates/engine draw.rs).
    moment: ["rolled", "keywordGranted", "addedToHand", "burned", "enteredGraveyard"],
    holdMs: TERMINUS_HOLD_MS.diceOfDoom,
    spin: 1,
    trauma: FX_TERMINUS_TRAUMA,
    palette: "Dark",
  },
};

/** The set piece a readable card's table entry names; none for R97's sentinel (R202). */
export function setPieceKeyOf(defId: string): SetPieceKey | undefined {
  if (defId === HIDDEN_ID) return undefined;
  const entry = FX_CARD_TABLE.get(defId);
  return entry?.kind === "setPiece" ? entry.key : undefined;
}

/** Whether a card's hook plays its removals as a set piece's wipe (the queue's `wipeOf`): a board clear's alone (R3338000). */
export function hasSetPiece(defId: string): boolean {
  const key = setPieceKeyOf(defId);
  return key !== undefined && SET_PIECES[key].moment === "wipe";
}

/** R2800000, R2785000: a wipe claims its entry as its card's set piece, asking for that set piece's beat. */
export function wipeClaim(entry: AnimationEntry): SetPieceClaim | null {
  const key = entry.wipe === undefined ? undefined : setPieceKeyOf(entry.wipe.by);
  return key === undefined ? null : { key, holdMs: SET_PIECES[key].holdMs };
}

/**
 * R3338000: what a readable card's signature plays over (the queue's `signatureOf`): its first cast, or
 * the event types of its hook's run; null for a card with none, a board clear's and R97's sentinel.
 */
export function signatureRunOf(defId: string): readonly GameEventType[] | "cast" | null {
  const key = setPieceKeyOf(defId);
  const moment = key === undefined ? "wipe" : SET_PIECES[key].moment;
  return moment === "wipe" ? null : moment;
}

/** R3338000: whether a Dice of Doom run hit its 67, by what the run did (the Terminus cards it added or burned), never by the face. */
export function doomHit(entry: Pick<AnimationEntry, "events">): boolean {
  return entry.events.some((event) => event.type === "addedToHand" || event.type === "burned");
}

/**
 * R3338000, R2785000: a Terminus card's signature claims its entry as its set piece, asking for that set
 * piece's beat; Dice of Doom asks DOOM_MISS_HOLD_MS for a miss.
 */
export function signatureClaim(entry: AnimationEntry): SetPieceClaim | null {
  const key = entry.signature === undefined ? undefined : setPieceKeyOf(entry.signature.by);
  if (key === undefined) return null;
  const holdMs = key === "diceOfDoom" && !doomHit(entry) ? DOOM_MISS_HOLD_MS : SET_PIECES[key].holdMs;
  return { key, holdMs };
}

/** R3338000: when beat `k` of `n` falls inside a set piece's span `S`, evenly from FX_TERMINUS_BEATS_FROM to FX_TERMINUS_BEATS_TO. */
export function beatAt(k: number, n: number, S: number): number {
  const step = (FX_TERMINUS_BEATS_TO - FX_TERMINUS_BEATS_FROM) / Math.max(1, n - 1);
  return frac(FX_TERMINUS_BEATS_FROM + k * step, S);
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
 * plays inside the entry and swallows at SET_PIECE_SWALLOW_AT of it. R3338000: the fold draws each card
 * into the Void's chest in turn, as its removals come, and its 10/10 burns once it settles; Farewell's
 * lamp lifts each card it exiles into its light in turn, a graveyard's as a card back rising from its
 * pile (at most FX_TERMINUS_LIFT_MAX), and ends in a slow exhale with no shake.
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
  const lamp = spec.shape === "lamp";
  const taken = takenOf(entry, view);
  // R3338000: the fold and the lamp take their cards one after another, each its share of the span.
  const staggered = fold || lamp;
  const lampAt = tid(testid.board, FX_TERMINUS_LAMP_AT);
  const lifts = lamp ? graveyardLifts(entry, view) : [];
  const beats = staggered ? [...taken.map((_, k) => Math.round((k * swallow) / taken.length)), ...lifts.map((_, k) => beatAt(k, lifts.length, swallow))] : [];
  const piece: FxSetPieceCue = {
    kind: "setPiece",
    shape: spec.shape,
    tint: tintOf(look),
    spin: spec.spin,
    side: one ?? null,
    at: lamp ? lampAt : centre,
    holdMs: swallow,
    delayMs: 0,
    durationMs: D + SET_PIECE_TAIL_MS,
    ...(beats.length > 0 ? { beats } : {}),
  };
  const cues: FxCue[] = [piece];
  taken.forEach((card, k) => {
    const n = taken.length;
    const startMs = staggered ? Math.round((k * swallow) / n) : 0;
    const goneMs = staggered ? Math.round(((k + 1) * swallow) / n) : swallow;
    const to = lamp ? lampAt : centre;
    cues.push(
      {
        kind: "pull",
        testid: card,
        to,
        delayMs: 0,
        durationMs: FX_HOLD_MAX_MS,
        ...(staggered ? { staggerMs: startMs, travelMs: goneMs - startMs } : {}),
      },
      { kind: "conceal", testid: card, mode: "after", delayMs: goneMs, durationMs: FX_HOLD_MAX_MS },
      { ...tunedBurst(i, look.particle, tid(card), "point", startMs, SET_PIECE_TUNING.pull), colors: look.colors },
    );
  });
  lifts.forEach((pile, k) => {
    const at = beatAt(k, lifts.length, swallow);
    cues.push({ kind: "ghost", from: tid(pile), to: lampAt, delayMs: at, durationMs: Math.max(1, swallow - at) });
  });
  if (lamp) {
    // Farewell: silence, and a slow exhale of smoke from under the lamp; nothing shakes.
    cues.push(
      { ...tunedBurst(i, "smoke", lampAt, "area", swallow, SET_PIECE_TUNING.exhale), colors: look.colors },
      ringCue(D, look.particle, lampAt, swallow),
    );
    return { cues, replaces: new Set(entry.events.filter(isRemoval)) };
  }
  cues.push(
    { ...tunedBurst(i, look.particle, centre, "ring", swallow, SET_PIECE_TUNING.implode), colors: look.colors },
    ringCue(D, look.particle, centre, swallow),
    ...shakeCues(i, spec.trauma, swallow),
  );
  if (fold) {
    // The Void settles alone, its 10/10 burning in the dark.
    const stats = tid(wipe.anchor ?? testid.board, FX_TERMINUS_STATS_AT);
    cues.push(
      raysCue(D, "mythic", centre, swallow),
      tunedBurst(i, "prismatic", centre, "area", swallow, SET_PIECE_TUNING.light),
      tunedBurst(i, "ember", stats, "point", swallow, SET_PIECE_TUNING.stats),
      ringCue(D, "ember", stats, swallow),
    );
  }
  return { cues, replaces: new Set(entry.events.filter(isRemoval)) };
}

/**
 * R3338000: the piles Farewell lifts a card from, one per card its wipe exiles out of a graveyard (a
 * public pile, R202), in the run's order, at most FX_TERMINUS_LIFT_MAX.
 */
function graveyardLifts(entry: AnimationEntry, view: PlayerView): string[] {
  const lifts: string[] = [];
  for (const event of entry.events) {
    if (event.type !== "exiled" || lifts.length >= FX_TERMINUS_LIFT_MAX) continue;
    const pile = pileOf(event, view, undefined);
    if (pile !== null && pile.pile === "graveyard") lifts.push(pileTestid(pile));
  }
  return lifts;
}
