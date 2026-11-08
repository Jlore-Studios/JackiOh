// R3399000: Classic+ Zephrys Zealotism (`classicplus-027`, takeover "zealot", bespoke "zealotCyclone"), its signature (#3399):
// Takeover (wired, fx/takeover.ts): the robed wind-zealot over his cloud lectern as the indigo cyclone spins.
// Text: the cyclone sweeps the caster's hand to the graveyard: each card's `enteredGraveyard` flies as a back
// from the hand pile to the graveyard pile on an indigo gust (the first with the cyclone's ring and swirl
// over the hand pile, each settling in dust on the graveyard); a new hand drops in, a back from the board's
// top edge to the hand pile for each `addedToHand` (an indigo glint on the pile for its owner alone); and the
// mana refills one crystal at a time in blue (`manaChanged`, one glint per crystal up to its new current).
// Nothing moves from the concept: the engine moves the hand with `enteredGraveyard` (a replace, not a
// discard, R416). R202: every card in flight is a back, no cue names a card, and the opponent sees the backs
// and the piles only.

import type { GameEvent, PlayerId, PlayerView } from "@jackioh/shared";

import { animTestid } from "../../../game/animations.ts";
import { sideOf } from "../../../game/contract.ts";
import { frac, ringCue, tid, tunedBurst, type BurstTuning } from "../../build.ts";
import { FX_MANA_MAX_SPARKS } from "../../constants.ts";
import { timesOf } from "../../motifs/build.ts";
import type { FxAnchor, FxCue, FxPreset, FxSpread } from "../../types.ts";
import type { CardRecipe } from "../recipe.ts";

/** Every particle count the cyclone throws, at intensity "normal" (rule 9). */
const TUNING = {
  cyclone: { count: 14, power: 1.2 },
  gust: { count: 6, power: 0.9 },
  settle: { count: 5, power: 0.5 },
  glint: { count: 6, power: 0.6 },
  crystal: { count: 3, power: 0.5 },
} as const;

/** The cyclone's indigo (its takeover's tint, hot first) and the refill's blue. */
const INDIGO = ["#e0e7ff", "#818cf8", "#4f46e5"] as const;
const BLUE = ["#e0f2fe", "#38bdf8", "#2563eb"] as const;

/** Where the new hand drops in from: the board's top edge. */
const SKY: FxAnchor = { kind: "viewport", at: { x: 0.5, y: 0 } };

/** Each card of a run leaves this share of D after the one before, at most STAGGER_MAX steps in (R200). */
const STEP = 0.08;
const STAGGER_MAX = 5;

/** The `k`-th of an entry's events of one type, and whether it is the play's first of that type. */
function ordinalOf(event: GameEvent, events: readonly GameEvent[], seen: number | undefined): { k: number; first: boolean } {
  const same = events.filter((e) => e.type === event.type);
  const k = Math.max(0, same.indexOf(event));
  return { k, first: (seen ?? same.length) - same.length + k === 0 };
}

/** Classic+ #27 Zephrys Zealotism: the old hand swept away, the new one dropped in and the mana refilled. */
export const zealotCyclone: CardRecipe = (event, p, play) => {
  if (play.hook !== undefined) return null;
  const i = p.env.intensity;
  const t = timesOf(p.D);
  const burst = (preset: FxPreset, at: FxAnchor, spread: FxSpread, delay: number, tuning: BurstTuning, colors: readonly string[] = INDIGO): FxCue => ({
    ...tunedBurst(i, preset, at, spread, delay, tuning),
    colors,
  });
  const mine = sideOf(p.view, play.player);
  const hand = tid(animTestid.hand(mine));
  if (event.type === "enteredGraveyard" && event.owner === play.player) {
    const { k, first } = ordinalOf(event, p.entry.events, play.seen.enteredGraveyard);
    const at = frac(Math.min(k, STAGGER_MAX) * STEP, p.D);
    const grave = tid(animTestid.graveyard(mine));
    const cues: FxCue[] = [
      { kind: "ghost", from: hand, to: grave, delayMs: at, durationMs: p.D - at },
      burst("dust", hand, "point", at, TUNING.gust),
      burst("smoke", grave, "point", t.L, TUNING.settle),
    ];
    if (first) cues.push(ringCue(p.D, "arcane", hand, t.G), burst("arcane", hand, "ring", t.G, TUNING.cyclone));
    return { cues, row: "keep" };
  }
  if (event.type === "addedToHand" && event.player === play.player) {
    const { k } = ordinalOf(event, p.entry.events, play.seen.addedToHand);
    const at = frac(Math.min(k, STAGGER_MAX) * STEP, p.D);
    const cues: FxCue[] = [{ kind: "ghost", from: SKY, to: hand, delayMs: at, durationMs: p.D - at }];
    if (play.player === p.view.viewer) cues.push(burst("sparkle", hand, "point", t.L, TUNING.glint));
    return { cues, row: "keep" };
  }
  if (event.type === "manaChanged" && event.player === play.player) {
    const n = Math.min(Math.max(0, event.current), FX_MANA_MAX_SPARKS);
    if (n === 0) return null;
    // One crystal after another inside Travel to Land, so every speed lands the same beats (R201).
    const step = (t.L - t.T) / n;
    const cues: FxCue[] = Array.from({ length: n }, (_, k) => burst("sparkle", { kind: "crystal", side: mine, index: k }, "point", t.T + Math.round(k * step), TUNING.crystal, BLUE));
    return { cues, row: "keep" };
  }
  return null;
};

/** A card of the other seat's hand in the gallery, which shows it only as a count. */
const GALLERY_OLD_HAND = "core-004";

/** The gallery's Zealotism: its player's hand goes to their graveyard, as many new cards arrive, and the mana refills. */
export function zealotCycloneFixture(view: PlayerView, _instanceId: string, player: PlayerId): GameEvent[] {
  const own = view[sideOf(view, player)];
  // The other seat's hand is a count (§10.8): its cards are named as they reach the graveyard, face up.
  const hand = Array.isArray(own.hand) ? own.hand : Array.from({ length: own.hand.count }, (_, k) => ({ instanceId: `fx-old-${String(k)}`, defId: GALLERY_OLD_HAND }));
  return [
    ...hand.map((card): GameEvent => ({ type: "enteredGraveyard", instanceId: card.instanceId, defId: card.defId, owner: player })),
    ...hand.map((_, k): GameEvent => ({ type: "addedToHand", player, instanceId: `fx-zealot-${String(k)}`, defId: "core-097" })),
    { type: "manaChanged", player, current: own.mana.max, max: own.mana.max },
  ];
}
