// R502: named-card flourishes and Cast on draw are planned only from the redacted stream (R202).
// A card recipe may claim its own resolving events; hidden plays never key recipes.
// R670: marquee Legendary and Mythic Units replace a row entrance when their `summoned` has a `defId`.
// R2784000: a card's entry in its set's effects table (fx/cards/tables.ts) names its bespoke recipe
// (fx/cards/<set>/) or a shared motif per hook (fx/motifs/); `cast` and `cry` play by the readable
// play resolving now, and R2797000: `death`, `trigger` and `activate` by the hook entry its
// `hookResolving` opened, a Trap's `cast` by the one its `trapFired` opened (memory.ts). R3332000: the
// entrance a card lands with is its entry's `entrance`, whatever the entry's kind (`entranceKeyOf`): a
// set piece's (R2800000, setPieces.ts) and a Field Spell's landing face up in the backrow among them.
// R200: every delay lands inside the entry and trails off within FX_MAX_TAIL_MS. R3405000: an entrance is
// passed its entry's lead (`leadMs`), so one that reads it starts after a Mythic's takeover hold.

import type { GameEvent, PlayerView } from "@jackioh/shared";

import { FLAVOR_FALLBACK } from "../cards/flavor.ts";
import { HIDDEN_ID, animTestid, leadMs, targetFor, type AnimationEntry } from "../game/animations.ts";
import { sideOf } from "../game/contract.ts";
import { CATALOG } from "../wire/cards.ts";
import { raysCue, ringCue, shakeCues, tid, tunedBurst } from "./build.ts";
import { CARD_RECIPES, type CardRecipeKey } from "./cards/bespoke.ts";
import type { EntranceKey } from "./cards/recipe.ts";
import type { CardFxPlan, CardFxResult } from "./cards/recipe.ts";
import { FX_CARD_TABLE, type FxCardEntry } from "./cards/tables.ts";
import { FX_CAST_ON_DRAW_TRAUMA } from "./constants.ts";
import { ENTRANCES, entranceEvent } from "./entrances.ts";
import { FLAVOR_LOOKS, paletteLook } from "./flavors.ts";
import { flavorLookOf } from "./flavorSource.ts";
import { motifScale, type MotifSpec } from "./motifs/build.ts";
import { motifReplaces, planMotif } from "./motifs/index.ts";
import { isDrawnMotif } from "./motifs/names.ts";
import type { FxCue, FxPlanEnv } from "./types.ts";

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
  // R3405000: its entry's lead (a Mythic's takeover hold) too, which an entrance starts after.
  return { cues: ENTRANCES[key]({ D: p.D, tgt, intensity: p.env.intensity, view: p.view, landing, leadMs: leadMs(p.entry) }), row: "replace" };
}

export function planCardFx(event: GameEvent, p: CardFxPlan, table: ReadonlyMap<string, FxCardEntry> = FX_CARD_TABLE): CardFxResult | null {
  const entrance = planEntrance(event, p, table);
  if (entrance !== null) return entrance;
  const play = p.env.memory.resolving();
  if (play === undefined || play.defId === HIDDEN_ID) return null;
  const key = bespokeOf(play.defId, table);
  return key === undefined ? null : CARD_RECIPES[key](event, p, play);
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
