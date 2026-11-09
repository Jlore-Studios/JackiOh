// R3405001: Meditative #65 Keymaster Keenus (`meditative-065`, entrance "keyRing", bespoke "keenusKeys", steel
// keys orbiting on his tile), a Legendary Unit's signature. A ring of iron keys spins into his zone and
// unwinds into the old gatekeeper, every key chiming as he bows at the slam; then, as he arrives with his
// Aura's combat keywords, his keys fly out and turn in unseen locks over him, a click of sparks for each.
// His landing (`summoned`) is his entrance's, so the keys of his arrival turn there. As an Equip play fuses
// him into a Unit (`fused`), the same keys fly out over that Unit and turn. One key per keyword his own unit
// shows on the board's view, his Equip aside (one Flickered shows his last draw's; at most KEENUS_KEYS_MAX),
// else KEENUS_KEYS: no event names the keywords he draws, and the view is the board before them, so the
// Unit he fuses into shows only its own. It keys on his play and the event's type, never on a card's
// definition, and names no card (R202).

import type { GameEvent, PlayerId, PlayerView } from "@jackioh/shared";

import { locateInstance } from "../../../game/animations.ts";
import { sideOf, testid } from "../../../game/contract.ts";
import { frac, ringCue, shakeCues, tid, tunedBurst } from "../../build.ts";
import { FX_SLAM_AT } from "../../constants.ts";
import { figureCue, figureTint } from "../../figures.ts";
import { FLAVOR_LOOKS, paletteLook, type FlavorLook } from "../../flavors.ts";
import type { FxCue } from "../../types.ts";
import { ENTRANCE_TRAUMA, type CardRecipe, type Entrance, type RecipeFixture } from "../recipe.ts";

/** The look his keys wear: Physical's steel sub-look, his aura's. */
const KEENUS_PALETTE = "Physical/steel";

/** The keys he turns when his unit shows no keyword on the board's view (or is not on it): his base face's draw (catalog `keywords`). */
export const KEENUS_KEYS = 5;

/** The most keys one turn draws: his Radiant face's draw. */
export const KEENUS_KEYS_MAX = 10;

/** Where his keys' locks click, an arc over him: from `left` to `right` of his box, `top` at its middle and `dip` lower at its ends. */
const LOCKS = { left: 0.15, right: 0.85, top: 0.12, dip: 0.12 } as const;

/** Every particle count his signature throws, at intensity "normal" (rule 9). */
const TUNING = {
  jangle: { count: 16, power: 0.9 },
  click: { count: 4, power: 0.5 },
} as const;

/**
 * As fractions of the motion after the lead: the ring lasts the entry and `tailMs` more; after his bow his
 * keys fly out at the slam and click from `keysAt` past it, one `keysSpan` share for all of them. Over a
 * fused Unit, as fractions of the entry: the keys fly out at once and click from `fuseAt`, one `fuseSpan`
 * share for all of them; the keys last until `tailMs` past the entry.
 */
const TIMING = { tailMs: 300, keysAt: 0.05, keysSpan: 0.3, fuseAt: 0.15, fuseSpan: 0.4 } as const;

/** One key per keyword his unit `instanceId` shows on `view` (his Equip aside), at most KEENUS_KEYS_MAX; KEENUS_KEYS when it shows none or is not on it. */
function keysOf(view: PlayerView | undefined, instanceId: string): number {
  const unit = view === undefined ? undefined : [...view.you.units, ...view.opponent.units].find((shown) => shown !== null && shown.instanceId === instanceId);
  const keywords = unit?.keywords.filter((keyword) => keyword.kind !== "Equip").length ?? 0;
  return keywords === 0 ? KEENUS_KEYS : Math.min(keywords, KEENUS_KEYS_MAX);
}

/** `n` keys clicking open over the box `tgt`, left to right along the locks' arc, from `from` over `span`. */
function clicks(i: number, look: FlavorLook, tgt: string, n: number, from: number, span: number): FxCue[] {
  return Array.from({ length: n }, (_, k) => {
    const x = LOCKS.left + ((LOCKS.right - LOCKS.left) * (k + 0.5)) / n;
    const y = LOCKS.top + LOCKS.dip * (2 * x - 1) ** 2;
    return { ...tunedBurst(i, "spark", tid(tgt, { x, y }), "point", from + Math.round((k * span) / n), TUNING.click), colors: look.colors };
  });
}

export const keyRing: Entrance = ({ D, tgt, intensity: i, view, landing, leadMs }) => {
  const look = paletteLook(KEENUS_PALETTE) ?? FLAVOR_LOOKS.Physical;
  const tint = figureTint(look);
  const at = tid(tgt);
  const lead = leadMs ?? 0;
  const span = D - lead;
  const slam = lead + frac(FX_SLAM_AT, span);
  const side = view === undefined || landing === undefined ? {} : { side: sideOf(view, landing.player) };
  const n = keysOf(view, landing?.instanceId ?? "");
  return [
    figureCue("keyRing", at, tint, lead, D - lead + TIMING.tailMs, side),
    ringCue(D, look.particle, at, slam),
    { ...tunedBurst(i, look.particle, at, "ring", slam, TUNING.jangle), colors: look.colors },
    ...shakeCues(i, ENTRANCE_TRAUMA.keyRing, slam),
    figureCue("keyTurn", at, tint, slam, D - slam + TIMING.tailMs, side),
    ...clicks(i, look, tgt, n, slam + frac(TIMING.keysAt, span), frac(TIMING.keysSpan, span)),
  ];
};

export const keenusKeys: CardRecipe = (event, p, play) => {
  // His Equip play, fusing him into a Unit: the Unit is the result.
  if (event.type !== "fused" || play.hook !== undefined || !event.instanceIds.includes(play.instanceId)) return null;
  const look = paletteLook(KEENUS_PALETTE) ?? FLAVOR_LOOKS.Physical;
  const i = p.env.intensity;
  const side = sideOf(p.view, play.player);
  // Over the Unit while the board shows it, else at his hero.
  const box = locateInstance(p.view, event.resultInstanceId) ?? testid.hero(side);
  const cues: FxCue[] = [
    figureCue("keyTurn", tid(box), figureTint(look), 0, p.D + TIMING.tailMs, { side }),
    ...clicks(i, look, box, keysOf(p.view, play.instanceId), frac(TIMING.fuseAt, p.D), frac(TIMING.fuseSpan, p.D)),
  ];
  return { cues, row: "replace" };
};

/** R3405001: his Equip play as `player` sees it: fused into `player`'s first Unit. */
export const keenusKeysFixture: RecipeFixture = (view, instanceId, player: PlayerId): GameEvent[] => {
  const host = (player === view.viewer ? view.you : view.opponent).units.find((unit) => unit !== null);
  return host === undefined || host === null ? [] : [{ type: "fused", instanceIds: [host.instanceId, instanceId], resultInstanceId: host.instanceId, defId: host.defId }];
};
