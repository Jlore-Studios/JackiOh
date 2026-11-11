// R2797000, R3395000: Stinkyscape #18 Imitated Instrumentality (`stinkyscape-018`, bespoke "magiBoot"), a
// Legendary Spell's signature, which summons MELCHIOR, BALTHASAR and CASPER. As it is cast the board's
// light turns server-room blue around its caster's hero: three tall core silhouettes flicker up behind the
// hero, linked by a triangle of beams that hums awake, in a burst of hologram light. Each brain then boots
// in its zone in turn as its `summoned` lands, flashing in its own hue (a brand in its tint, a sheen, and
// for the first two a ring), the first shaking the board. Every beat replaces its summon's own row, so the
// whole cast fits the Legendary Spell's ceiling (R3332000). It keys on its own play and how many summons it
// has seen, never on a card's definition (R202). Moved: the three turning toward the caster's hero for one
// beat is dropped, as no event follows the third boot inside the ceiling.

import { targetFor } from "../../../game/animations.ts";
import { ringCue, shakeCues, tid } from "../../build.ts";
import { FX_MAGI_TRAUMA } from "../../constants.ts";
import { figureCue, figureTint } from "../../figures.ts";
import { motifBurst, motifScale, type CueSpec } from "../../motifs/build.ts";
import type { FxCue, FxPreset, FxTint } from "../../types.ts";
import { heroOf, type CardRecipe } from "../recipe.ts";
import { lookOf, replace } from "./signatureKit.ts";

/** The look the cores boot in: Tech's hologram. */
const MAGI_PALETTE = "Tech/hologram";

/** Every particle count the cast throws at scale 1 (rule 9). */
const TUNING = {
  boot: { count: 26, power: 1.1 },
} as const;

/** Each brain's own hue, in summon order: MELCHIOR's, BALTHASAR's and CASPER's (a fourth takes the first's again). */
const BRAIN_HUES: readonly { ring: FxPreset; tint: FxTint }[] = [
  { ring: "arcane", tint: { rim: "#a78bfa", core: "#ede9fe", glow: "#7c3aed" } },
  { ring: "frost", tint: { rim: "#38bdf8", core: "#e0f2fe", glow: "#0284c7" } },
  { ring: "fire", tint: { rim: "#fb923c", core: "#ffedd5", glow: "#ea580c" } },
];

/** How many boots ring their zone; later boots flash and sheen alone, inside the ceiling. */
const RINGED_BOOTS = 2;

/** R2797000, R3395000: Imitated Instrumentality's cast and its three boots. */
export const magiBoot: CardRecipe = (event, p, play) => {
  const look = lookOf(MAGI_PALETTE);
  const spec: CueSpec = { look, scale: motifScale(play.defId, p.view, p.env.intensity), row: "replace" };
  if (event.type === "cardPlayed" && event.instanceId === play.instanceId) {
    const hero = heroOf(p.view, play);
    return replace([figureCue("magiCores", hero, figureTint(look), 0, p.D), motifBurst(spec, hero, "area", 0, TUNING.boot), ringCue(p.D, "frost", hero, 0)]);
  }
  if (event.type !== "summoned" || event.player !== play.player) return null;
  const zone = targetFor(event, p.view);
  if (zone === null) return null;
  const k = Math.max(0, (play.seen.summoned ?? 1) - 1);
  const hue = BRAIN_HUES[k % BRAIN_HUES.length] ?? BRAIN_HUES[0];
  const at = tid(zone);
  const cues: FxCue[] = [];
  if (hue !== undefined) cues.push({ kind: "brand", at, tint: hue.tint, delayMs: 0, durationMs: p.D });
  cues.push({ kind: "sheen", at, delayMs: 0, durationMs: p.D });
  if (k < RINGED_BOOTS && hue !== undefined) cues.push(ringCue(p.D, hue.ring, at, 0));
  if (k === 0) cues.push(...shakeCues(spec.scale, FX_MAGI_TRAUMA, 0));
  return replace(cues);
};
