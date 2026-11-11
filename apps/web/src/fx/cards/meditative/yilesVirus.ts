// R3405001: Meditative #49.2 Yile's Virus (`meditative-049-2`, a token printed Mythic, entrance "virusBoot",
// bespoke "yileVirus", takeover "virus", a glitch on its tile), a Mythic Unit's signature in Tech's glitch.
// After its takeover's hold (once an action: a flood of them takes over once, R3405000) pixel snow falls on
// its zone and a boxy virus face, blinking, tumbles into it, booting up at the slam in a ring of pixels. At
// the start of its turn it sparks: a zap lands on its hero and on each adjacent Unit it hits (its `damage`),
// carrying the hit's number and, where Armor took half of it, the Armor's flash, as the row it replaces
// would (R1363). Its Activate pops it in a burst of pixels (its own `destroyed`), and as it dies a hand card
// tears away in a zap at its controller's hand pile (`discarded`), a card back flying from that pile to its
// Graveyard pile: the pile, never the card, so a hand card the viewer cannot read stays unnamed. It keys on
// its own id as a hit's source or the card destroyed and on the discard's owner, never on a card's
// definition, and names no card (R202).

import { animTestid, targetFor } from "../../../game/animations.ts";
import { sideOf } from "../../../game/contract.ts";
import { armorTookHalf } from "../../../game/damageFeel.ts";
import { frac, ringCue, shakeCues, tid, tunedBurst } from "../../build.ts";
import { FX_SLAM_AT, FX_SPLAT_HOLD_MS } from "../../constants.ts";
import { figureCue, figureTint } from "../../figures.ts";
import { FLAVOR_LOOKS, paletteLook } from "../../flavors.ts";
import { shieldCues } from "../../shield.ts";
import type { FxCue } from "../../types.ts";
import { ENTRANCE_TRAUMA, type CardRecipe, type Entrance, type RecipeFixture } from "../recipe.ts";

/** The look it glitches in: Tech's glitch, magenta and teal, its aura's. */
const VIRUS_PALETTE = "Tech/glitch";

/** Every particle count it throws, at intensity "normal" (rule 9): the pixel snow, its boot, a zap's sparks, its pop, a torn card. */
const TUNING = {
  snow: { count: 18, power: 0.5 },
  boot: { count: 20, power: 1.1 },
  zap: { count: 10, power: 1.2 },
  pop: { count: 24, power: 1.3 },
  tear: { count: 10, power: 0.8 },
} as const;

/** The face tumbles from the lead to `faceTailMs` past its entry; a zap flashes its entry and `zapTailMs` more. */
const TIMING = { faceTailMs: 350, zapTailMs: 200 } as const;

/** The gallery's start of turn: one of its zaps, two damage to its own hero. */
const FIXTURE_ZAP_AMOUNT = 2;

export const virusBoot: Entrance = ({ D, tgt, intensity: i, view, landing, leadMs }) => {
  const look = paletteLook(VIRUS_PALETTE) ?? FLAVOR_LOOKS.Tech;
  const at = tid(tgt);
  const lead = leadMs ?? 0;
  const slam = lead + frac(FX_SLAM_AT, D - lead);
  return [
    { ...tunedBurst(i, look.particle, at, "area", lead, TUNING.snow), colors: look.colors },
    figureCue("virusFace", at, figureTint(look), lead, D - lead + TIMING.faceTailMs, view === undefined || landing === undefined ? {} : { side: sideOf(view, landing.player) }),
    ringCue(D, look.particle, at, slam),
    { ...tunedBurst(i, look.particle, at, "ring", slam, TUNING.boot), colors: look.colors },
    ...shakeCues(i, ENTRANCE_TRAUMA.virusBoot, slam),
  ];
};

export const yileVirus: CardRecipe = (event, p, play) => {
  const look = paletteLook(VIRUS_PALETTE) ?? FLAVOR_LOOKS.Tech;
  const tint = figureTint(look);
  const i = p.env.intensity;
  switch (event.type) {
    case "damage": {
      // Its start of turn's zaps, each with the number its row would pop.
      if (event.sourceId !== play.instanceId) return null;
      const tgt = targetFor(event, p.view);
      if (tgt === null) return null;
      const at = tid(tgt);
      const cues: FxCue[] = [figureCue("virusZap", at, tint, 0, p.D + TIMING.zapTailMs), { ...tunedBurst(i, "spark", at, "point", 0, TUNING.zap), colors: look.colors }];
      if (event.amount > 0) cues.push({ kind: "splat", tone: "damage", amount: event.amount, at, delayMs: 0, durationMs: p.D + FX_SPLAT_HOLD_MS });
      if (armorTookHalf(event.absorbed, event.amount)) cues.push(...shieldCues("small", at, p.D, 0, i));
      return { cues, row: "replace" };
    }
    case "destroyed": {
      // Its Activate pops it.
      if (event.instanceId !== play.instanceId) return null;
      const tgt = targetFor(event, p.view);
      if (tgt === null) return null;
      const at = tid(tgt);
      return { cues: [{ ...tunedBurst(i, look.particle, at, "area", 0, TUNING.pop), colors: look.colors }, ringCue(p.D, look.particle, at, 0)], row: "replace" };
    }
    case "discarded": {
      // Its Death's discard: a zap at its controller's hand pile, and a card back from there to the Graveyard.
      if (event.owner !== play.player) return null;
      const side = sideOf(p.view, event.owner);
      const hand = tid(animTestid.hand(side));
      return {
        cues: [
          figureCue("virusZap", hand, tint, 0, p.D + TIMING.zapTailMs, { size: "small" }),
          { ...tunedBurst(i, look.particle, hand, "area", 0, TUNING.tear), colors: look.colors },
          { kind: "ghost", from: hand, to: tid(animTestid.graveyard(side)), delayMs: 0, durationMs: p.D },
        ],
        row: "replace",
      };
    }
    default:
      return null;
  }
};

/** The gallery's Yile's Virus: one zap of its start of turn, on its own hero. */
export const yileVirusFixture: RecipeFixture = (_view, instanceId, player) => [{ type: "damage", sourceId: instanceId, targetId: `hero-${player}`, amount: FIXTURE_ZAP_AMOUNT, combat: false }];
