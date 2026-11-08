// R502: named-card flourishes and Cast on draw are planned only from the redacted stream (R202).
// A card recipe may claim its own resolving events; hidden plays never key recipes.
// R670: marquee Legendary and Mythic Units replace a row entrance when their `summoned` has a `defId`.
// R2784000: a card's entry in its set's effects table (fx/cards/tables.ts) names its bespoke recipe
// (fx/cards/<set>/) or a shared motif per hook (fx/motifs/); `cast` and `cry` play by the readable
// play resolving now, and R2797000: `death`, `trigger` and `activate` by the hook entry its
// `hookResolving` opened, a Trap's `cast` by the one its `trapFired` opened (memory.ts). R3332000: the
// entrance a card lands with is its entry's `entrance`, whatever the entry's kind (`entranceKeyOf`): a
// set piece's (R2800000, setPieces.ts) and a Field Spell's landing face up in the backrow among them.
// R200: every delay lands inside the entry and trails off within FX_MAX_TAIL_MS.
// R3389000: a signature's `{ bespoke }` recipe also claims what its card does outside its own play's
// window, when the play resolving draws nothing of it: first a cast its play makes (Spell Tyrant's, the
// innermost readable play around the one resolving), then an event that names its card as the source
// (a redirect's or a counter's card, an Activate's, a quest's: `sourceIdOf`), found face up on the board
// or in a readable event remembered (`signaturePlayOf`). Neither reads a hidden card (R202). It draws
// beside a whole pile's one wave too (`planSignatureZone`).

import type { GameEvent, PlayerId, PlayerView } from "@jackioh/shared";

import { FLAVOR_FALLBACK } from "../cards/flavor.ts";
import { HIDDEN_ID, animTestid, leadMs, targetFor, type AnimationEntry } from "../game/animations.ts";
import { sideOf } from "../game/contract.ts";
import { CATALOG } from "../wire/cards.ts";
import { frac, raysCue, ringCue, shakeCues, tid, tunedBurst } from "./build.ts";
import { CARD_RECIPES, type CardRecipeKey } from "./cards/bespoke.ts";
import type { EntranceKey } from "./cards/recipe.ts";
import type { CardFxPlan, CardFxResult } from "./cards/recipe.ts";
import { FX_CARD_TABLE, isSignature, type FxCardEntry } from "./cards/tables.ts";
import { FX_CAST_ON_DRAW_TRAUMA, FX_SLAM_AT } from "./constants.ts";
import { ENTRANCES, entranceEvent } from "./entrances.ts";
import { FLAVOR_LOOKS, paletteLook } from "./flavors.ts";
import { flavorLookOf } from "./flavorSource.ts";
import { motifScale, type MotifSpec } from "./motifs/build.ts";
import { motifReplaces, planMotif } from "./motifs/index.ts";
import { isDrawnMotif } from "./motifs/names.ts";
import type { FxCue, FxMemory, FxPlanEnv, FxPlay } from "./types.ts";

export type { CardFxPlan, CardFxResult } from "./cards/recipe.ts";

/** Every particle count the cast on draw throws, at intensity "normal" (rule 9). */
const CARD_TUNING = {
  pileGold: { count: 34, power: 1.3 },
  pileArcane: { count: 20, power: 1 },
} as const;

/** R502: cast on draw bursts from the drawer's Deck pile. */
export function castOnDrawCues(event: GameEvent, p: CardFxPlan): FxCue[] {
  if (event.type !== "cardPlayed" || !p.env.memory.castOnDraw(event)) return [];
  const i = p.env.intensity;
  const pile = tid(animTestid.library(sideOf(p.view, event.player)));
  return [
    ringCue(p.D, "gold", pile, 0),
    raysCue(p.D, "radiant", pile, 0),
    tunedBurst(i, "gold", pile, "ring", 0, CARD_TUNING.pileGold),
    tunedBurst(i, "arcane", pile, "area", 0, CARD_TUNING.pileArcane),
    ...shakeCues(i, FX_CAST_ON_DRAW_TRAUMA, 0),
  ];
}

/** The bespoke recipe a card's table entry names, if any. */
function bespokeOf(defId: string, table: ReadonlyMap<string, FxCardEntry>): CardRecipeKey | undefined {
  const entry = table.get(defId);
  return entry?.kind === "bespoke" ? entry.key : undefined;
}

/**
 * R670, R3332000: the bespoke entrance a card lands with: its table entry's `entrance`, whatever the
 * entry's kind (Ceaseless Void's beside its set piece, so it lands with its collapse and its fold
 * follows, R2800000; Nature Titan's beside its Cry, R2794000).
 */
export function entranceKeyOf(defId: string, table: ReadonlyMap<string, FxCardEntry> = FX_CARD_TABLE): EntranceKey | undefined {
  const entry = table.get(defId);
  return entry === undefined || entry.kind === "none" ? undefined : entry.entrance;
}

function planEntrance(event: GameEvent, p: CardFxPlan, table: ReadonlyMap<string, FxCardEntry>): CardFxResult | null {
  if (event.type !== "summoned" || event.defId === HIDDEN_ID) return null;
  const type = (CATALOG[event.defId] ?? p.view.defs?.[event.defId])?.type;
  const landing = entranceEvent(event, HIDDEN_ID, type);
  if (landing === null) return null;
  const key = entranceKeyOf(landing.defId, table);
  if (key === undefined) return null;
  const tgt = targetFor(landing, p.view);
  if (tgt === null) return null;
  const lead = leadMs(p.entry);
  return { cues: ENTRANCES[key]({ D: p.D, tgt, intensity: p.env.intensity, view: p.view, landing, landMs: lead + frac(FX_SLAM_AT, p.D - lead) }), row: "replace" };
}

/** R3389000: the recipe of `defId`'s signature, a `{ bespoke }` entry with `signature: true`, if any. */
function signatureRecipeOf(defId: string, table: ReadonlyMap<string, FxCardEntry>): CardRecipeKey | undefined {
  if (defId === HIDDEN_ID) return undefined;
  const entry = table.get(defId);
  return isSignature(entry) && entry?.kind === "bespoke" ? entry.key : undefined;
}

/** R3389000: the card an event names as its source: a redirect's or a counter's, an Activate's, a quest's own card. */
export function sourceIdOf(event: GameEvent): string | null {
  switch (event.type) {
    case "redirected":
    case "countered":
      return event.byInstanceId;
    case "activated":
    case "questProgressed":
    case "questCompleted":
      return event.instanceId;
    default:
      return null;
  }
}

/**
 * R3389000, R202: the card `id` names, as a play of its own (a `trigger` entry: no play of it is open):
 * a unit or a backrow card the view shows face up with a definition it reads, else the newest readable
 * `summoned`, `cardPlayed`, `hookResolving` or `trapFired` of it the memory still holds; else undefined.
 */
export function signaturePlayOf(id: string, view: PlayerView, memory: FxMemory): FxPlay | undefined {
  if (id === HIDDEN_ID) return undefined;
  const play = (player: PlayerId, defId: string): FxPlay | undefined =>
    defId === HIDDEN_ID ? undefined : { player, instanceId: id, defId, castOnDraw: false, step: 0, seen: {}, hook: "trigger" };
  for (const seat of [view.you, view.opponent]) {
    const unit = seat.units.find((candidate) => candidate !== null && candidate.instanceId === id);
    if (unit !== undefined && unit !== null) return play(unit.controller, unit.defId);
    for (const card of seat.backrow) {
      if (card !== null && !card.faceDown && card.instanceId === id) return play(card.controller, card.defId);
    }
  }
  const recent = memory.recent();
  for (let k = recent.length - 1; k >= 0; k -= 1) {
    const event = recent[k];
    if (event === undefined) continue;
    if ((event.type === "summoned" || event.type === "cardPlayed") && event.instanceId === id) return play(event.player, event.defId);
    if (event.type === "hookResolving" && event.sourceId === id) return play(event.controller, event.defId);
    if (event.type === "trapFired" && event.instanceId === id) return play(event.controller, event.defId);
  }
  return undefined;
}

/**
 * R3389000: a signature recipe's claim on `event` outside its own play's window: the innermost readable
 * play still open around the one resolving whose card has a signature recipe (a cast it makes), else the
 * card `sourceIdOf(event)` names. A hook entry and a hidden card claim nothing here (R202).
 */
function signatureFallback(event: GameEvent, p: CardFxPlan, table: ReadonlyMap<string, FxCardEntry>): CardFxResult | null {
  const plays = p.env.memory.plays();
  for (let k = plays.length - 2; k >= 0; k -= 1) {
    const play = plays[k];
    if (play === undefined || play.hook !== undefined || play.instanceId === HIDDEN_ID) continue;
    const key = signatureRecipeOf(play.defId, table);
    const result = key === undefined ? null : CARD_RECIPES[key](event, p, play);
    if (result !== null) return result;
  }
  const source = sourceIdOf(event);
  const play = source === null ? undefined : signaturePlayOf(source, p.view, p.env.memory);
  const key = play === undefined ? undefined : signatureRecipeOf(play.defId, table);
  return play === undefined || key === undefined ? null : CARD_RECIPES[key](event, p, play);
}

/**
 * R3389000: a signature's recipe over a whole pile's one entry (a zone entry, which plays its one wave
 * and no event's row): the readable play resolving now, if its card's signature is a `{ bespoke }`, is
 * asked about the run's first event, and its cues play beside the wave (Second Wind's deck blown away).
 */
export function planSignatureZone(entry: AnimationEntry, view: PlayerView, env: FxPlanEnv, table: ReadonlyMap<string, FxCardEntry> = FX_CARD_TABLE): FxCue[] {
  const play = env.memory.resolving();
  const first = entry.events[0];
  if (entry.zone === undefined || play === undefined || play.instanceId === HIDDEN_ID || first === undefined) return [];
  const key = signatureRecipeOf(play.defId, table);
  return key === undefined ? [] : (CARD_RECIPES[key](first, { entry, view, env, D: entry.durationMs }, play)?.cues ?? []);
}

export function planCardFx(event: GameEvent, p: CardFxPlan, table: ReadonlyMap<string, FxCardEntry> = FX_CARD_TABLE): CardFxResult | null {
  const entrance = planEntrance(event, p, table);
  if (entrance !== null) return entrance;
  const play = p.env.memory.resolving();
  const key = play === undefined || play.defId === HIDDEN_ID ? undefined : bespokeOf(play.defId, table);
  const own = play === undefined || key === undefined ? null : CARD_RECIPES[key](event, p, play);
  return own ?? signatureFallback(event, p, table);
}

/** A motif's cues for one entry, and the entry's events it drew in place of their own row recipe. */
export type MotifResult = { cues: FxCue[]; replaces: ReadonlySet<GameEvent> };

/**
 * R2784000: the motif of the readable play resolving now, on its `cry` (a Unit) or `cast` (any other
 * card); R2797000: of the readable hook running now, on the hook its entry is (`FxPlay.hook`: a
 * `death`, a `trigger`, an `activate`, or a Trap's firing's `cast`). A hidden play or hook, a card with
 * no hooks, a motif not drawn yet and a run over a whole pile plan nothing here, so the event rows
 * play alone; nor does an Anomaly's rift, which its set piece draws (R3337003).
 */
export function planCardMotif(entry: AnimationEntry, view: PlayerView, env: FxPlanEnv, table: ReadonlyMap<string, FxCardEntry> = FX_CARD_TABLE): MotifResult | null {
  // R3337003: an Anomaly's rift is its own set piece (anomaly.ts), whichever card laid it (#151's entry too).
  if (entry.zone !== undefined || entry.rift !== undefined) return null;
  const play = env.memory.resolving();
  if (play === undefined || play.defId === HIDDEN_ID) return null; // R202
  const card = table.get(play.defId);
  if (card?.kind !== "hooks") return null;
  const type = (CATALOG[play.defId] ?? view.defs?.[play.defId])?.type;
  const hook = card.hooks[play.hook ?? (type === "Unit" ? "cry" : "cast")];
  if (hook === undefined || !isDrawnMotif(hook.motif)) return null;
  const look = (hook.palette !== undefined ? paletteLook(hook.palette) : undefined) ?? flavorLookOf(play.defId, play.instanceId, view) ?? FLAVOR_LOOKS[FLAVOR_FALLBACK];
  const spec: MotifSpec = { motif: hook.motif, look, scale: motifScale(play.defId, view, env.intensity), row: hook.row ?? "replace" };
  const cues = planMotif(spec, entry, view, env);
  if (cues.length === 0) return null;
  return { cues, replaces: motifReplaces(spec, entry, view, env) };
}
