// R3399000: Classic+ Mythic Grape (`classicplus-065-5`, takeover "vineyard", bespoke "vineRipen"), its signature (#3399):
// Takeover (wired, fx/takeover.ts): the luminous violet grape swelling over its ancient vineyard.
// Text: over the caster's hand pile the old hand withers like raisins and drops away: each card's
// `enteredGraveyard` puffs a raisin-dark wither on the hand pile and falls as a back to the graveyard pile
// (the first with a ring of withering); then the new hand ripens on a vine, one card back at a time: each
// `addedToHand` sprouts vine leaves on the pile and a back swings in along the vine from the board's edge,
// a violet glint on the pile for its owner alone.
// Moved: the engine moves the old hand with `enteredGraveyard` (a replace, not a discard; no `discarded` is
// ever emitted), so the wither plays there. R202: every card in flight is a back, no cue names a card, and
// the opponent sees the backs and the piles only.

import type { GameEvent, PlayerId, PlayerView } from "@jackioh/shared";

import { animTestid } from "../../../game/animations.ts";
import { sideOf } from "../../../game/contract.ts";
import { frac, ringCue, tid, tunedBurst, type BurstTuning } from "../../build.ts";
import { timesOf } from "../../motifs/build.ts";
import type { FxAnchor, FxCue, FxPreset, FxSpread } from "../../types.ts";
import type { CardRecipe } from "../recipe.ts";

/** Every particle count the vine throws, at intensity "normal" (rule 9). */
const TUNING = {
  wither: { count: 6, power: 0.4 },
  witherRing: { count: 10, power: 0.6 },
  sprout: { count: 6, power: 0.6 },
  glint: { count: 6, power: 0.6 },
} as const;

/** The withered raisins, the vine's green and the new grapes' violet (its takeover's tint), hot first. */
const RAISIN = ["#d6c1a8", "#6b3a4f", "#2e1020"] as const;
const VINE = ["#ecfccb", "#65a30d", "#3f6212"] as const;
const VIOLET = ["#f3e8ff", "#c084fc", "#9333ea"] as const;

/** Where the vine reaches in from: the board's left edge, at the middle. */
const VINE_ROOT: FxAnchor = { kind: "viewport", at: { x: 0, y: 0.5 } };

/** Each card of a run follows the one before by this share of D, at most STAGGER_MAX steps in (R200). */
const STEP = 0.08;
const STAGGER_MAX = 5;

/** The `k`-th of an entry's events of one type, and whether it is the play's first of that type. */
function ordinalOf(event: GameEvent, events: readonly GameEvent[], seen: number | undefined): { k: number; first: boolean } {
  const same = events.filter((e) => e.type === event.type);
  const k = Math.max(0, same.indexOf(event));
  return { k, first: (seen ?? same.length) - same.length + k === 0 };
}

/** Classic+ #65.5 Mythic Grape: the old hand withering away, the new one ripening on its vine. */
export const vineRipen: CardRecipe = (event, p, play) => {
  if (play.hook !== undefined) return null;
  const i = p.env.intensity;
  const t = timesOf(p.D);
  const burst = (preset: FxPreset, at: FxAnchor, spread: FxSpread, delay: number, tuning: BurstTuning, colors: readonly string[]): FxCue => ({
    ...tunedBurst(i, preset, at, spread, delay, tuning),
    colors,
  });
  const mine = sideOf(p.view, play.player);
  const hand = tid(animTestid.hand(mine));
  if (event.type === "enteredGraveyard" && event.owner === play.player) {
    const { k, first } = ordinalOf(event, p.entry.events, play.seen.enteredGraveyard);
    const at = frac(Math.min(k, STAGGER_MAX) * STEP, p.D);
    const cues: FxCue[] = [
      burst("smoke", hand, "point", at, TUNING.wither, RAISIN),
      { kind: "ghost", from: hand, to: tid(animTestid.graveyard(mine)), delayMs: at, durationMs: p.D - at },
    ];
    if (first) cues.push(ringCue(p.D, "smoke", hand, t.G), burst("crumb", hand, "ring", t.G, TUNING.witherRing, RAISIN));
    return { cues, row: "keep" };
  }
  if (event.type === "addedToHand" && event.player === play.player) {
    const { k } = ordinalOf(event, p.entry.events, play.seen.addedToHand);
    const at = frac(Math.min(k, STAGGER_MAX) * STEP, p.D);
    const cues: FxCue[] = [burst("leaf", hand, "point", at, TUNING.sprout, VINE), { kind: "ghost", from: VINE_ROOT, to: hand, delayMs: at, durationMs: p.D - at }];
    if (play.player === p.view.viewer) cues.push(burst("sparkle", hand, "point", t.L, TUNING.glint, VIOLET));
    return { cues, row: "keep" };
  }
  return null;
};

/** A card of the other seat's hand in the gallery, which shows it only as a count. */
const GALLERY_OLD_HAND = "core-004";

/** The gallery's Mythic Grape: its player's hand goes to their graveyard and as many random Mythics arrive. */
export function vineRipenFixture(view: PlayerView, _instanceId: string, player: PlayerId): GameEvent[] {
  const own = view[sideOf(view, player)];
  // The other seat's hand is a count (§10.8): its cards are named as they reach the graveyard, face up.
  const hand = Array.isArray(own.hand) ? own.hand : Array.from({ length: own.hand.count }, (_, k) => ({ instanceId: `fx-old-${String(k)}`, defId: GALLERY_OLD_HAND }));
  const mythics = ["core-096", "core-097", "core-100"] as const;
  return [
    ...hand.map((card): GameEvent => ({ type: "enteredGraveyard", instanceId: card.instanceId, defId: card.defId, owner: player })),
    ...hand.map((_, k): GameEvent => ({ type: "addedToHand", player, instanceId: `fx-ripe-${String(k)}`, defId: mythics[k % mythics.length] ?? "core-097" })),
  ];
}
