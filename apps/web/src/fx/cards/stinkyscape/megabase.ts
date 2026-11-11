// R3407000: Stinkyscape #116 Megabase (`stinkyscape-116`, a Legendary Unit: entrance "megabaseDock", recipe
// "giftShop", aura "stationrings" in Tech/starfield on its tile), a signature.
//
// - Its entrance: the board's sky darkens (the `dusk` figure) as the station comes down from far above its
//   zone in a streak of starlight, three rings closing on the zone one by one as it lowers; at the slam two
//   docking clamps thud on the zone's flanks (spark bursts) with a shake, and its lobby lights blink on (two
//   small star bursts).
// - Its Activate (`hookResolving` "activate"): each `marketCreditsChanged` fountains a credit chip (a gold
//   projectile) from its gift-shop hatch to the hero that gains, the hatch opening with a gold burst on the
//   first. Its Death (`hookResolving` "death"): its station rings burst where it stood (a starfield ring and
//   a ring of stars) on the first Credits, and every `marketCreditsChanged` showers gold over that hero.
//
// Moved from the issue's concept: the hits sparking off its armour rings (`damage`) and the burst on
// `destroyed` are dropped as their own beats: both happen in another card's play or an attack, which its
// recipe never keys (only its own play and hooks reach it, cardFx.ts); its rings burst on its Death hook's
// Credits instead, the entry that follows its `destroyed`. The shared Credits moment (systemMoments.ts) still
// draws its own gold at each hero beneath these beats.
//
// R202: Credits are public; no cue names a card. The hatch is its card while the view shows it face up, else
// the zone its `summoned` named, else its controller's hero.

import type { GameEvent, PlayerId, PlayerView } from "@jackioh/shared";

import { locateInstance } from "../../../game/animations.ts";
import { sideOf, testid } from "../../../game/contract.ts";
import { frac, shakeCues, tid } from "../../build.ts";
import { figureCue, figureTint } from "../../figures.ts";
import { motifBurst, motifRing, motifShot } from "../../motifs/build.ts";
import type { FxAnchor, FxCue, FxPlay, FxPoint } from "../../types.ts";
import { ENTRANCE_TRAUMA, projectile, type CardFxPlan, type CardRecipe, type Entrance } from "../recipe.ts";
import { entranceSpec, entranceTimes, sigSpec } from "./signature.ts";

/** Every particle count the station throws at scale 1 (rule 9). */
const TUNING = {
  clamp: { count: 12, power: 0.9 },
  lights: { count: 5, power: 0.4 },
  hatch: { count: 16, power: 1 },
  rings: { count: 20, power: 1.2 },
  shower: { count: 22, power: 1.1 },
} as const;

/** Each beat's moments as fractions of its entry's D (an entrance's of its motion after its hold), and its tails in ms. */
const TIMING = {
  megabaseDock: { descendAt: 0.05, ringAt: 0.15, ringStep: 0.15, rings: 3, lightsAt: 0.8, lightsStep: 0.08, duskTailMs: 300 },
  giftShop: { hatchAt: 0, chipsAt: 0.05, chipsFor: 0.5, showerAt: 0.1 },
} as const;

/** The station's starfield, and the Credits' cash. */
const STARFIELD = "Tech/starfield";
const CASH = "Hustle/cash";

/** Where the station comes down from, far above its zone, and its two docking clamps, on the zone's flanks (fractions of the zone). */
const DESCENT_FROM: FxPoint = { x: 0.5, y: -1.5 };
const CLAMPS: readonly FxPoint[] = [
  { x: 0, y: 0.5 },
  { x: 1, y: 0.5 },
];
/** Its lobby, low in its zone. */
const LOBBY: FxPoint = { x: 0.5, y: 0.7 };

/** R3407000: where Megabase stands: its card while the view shows it, else the zone its `summoned` named, else its controller's hero. */
function standing(p: CardFxPlan, play: FxPlay): FxAnchor {
  const at = locateInstance(p.view, play.instanceId);
  if (at !== null && at.startsWith("card-")) return tid(at);
  const landed = [...p.env.memory.recent()].reverse().find((event) => event.type === "summoned" && event.instanceId === play.instanceId);
  if (landed?.type === "summoned") return tid(testid.zone(sideOf(p.view, landed.player), landed.row, landed.lane));
  return tid(testid.hero(sideOf(p.view, play.player)));
}

/** Whether `event` is the first Credits of this hook's run (each `marketCreditsChanged` is its own entry, `seen` counting it). */
function firstCredits(event: GameEvent, p: CardFxPlan, play: FxPlay): boolean {
  const own: readonly GameEvent[] = p.entry.events.filter((candidate) => candidate.type === "marketCreditsChanged");
  return (play.seen.marketCreditsChanged ?? 0) - own.length + own.indexOf(event) === 0;
}

export const megabaseDock: Entrance = (p) => {
  const spec = entranceSpec(p, STARFIELD);
  const { wait, at, slam } = entranceTimes(p);
  const T = TIMING.megabaseDock;
  const zone = tid(p.tgt);
  const cues: FxCue[] = [
    figureCue("dusk", tid(testid.board), figureTint(spec.look), wait, p.D - wait + T.duskTailMs),
    motifShot(spec, "bolt", tid(p.tgt, DESCENT_FROM), zone, at(T.descendAt), slam - at(T.descendAt)),
  ];
  for (let k = 0; k < T.rings; k += 1) cues.push(motifRing(spec, p.D, zone, at(T.ringAt + k * T.ringStep)));
  for (const clamp of CLAMPS) cues.push(motifBurst(spec, tid(p.tgt, clamp), "point", slam, TUNING.clamp, "spark"));
  cues.push(
    motifBurst(spec, tid(p.tgt, LOBBY), "point", at(T.lightsAt), TUNING.lights),
    motifBurst(spec, tid(p.tgt, LOBBY), "point", at(T.lightsAt + T.lightsStep), TUNING.lights),
    ...shakeCues(p.intensity, ENTRANCE_TRAUMA.megabaseDock, slam),
  );
  return cues;
};

export const giftShop: CardRecipe = (event, p, play) => {
  if (event.type !== "marketCreditsChanged" || (play.hook !== "activate" && play.hook !== "death")) return null;
  const T = TIMING.giftShop;
  const spec = sigSpec(p, play, STARFIELD);
  const cash = sigSpec(p, play, CASH);
  const hero = tid(testid.hero(sideOf(p.view, event.player)));
  const first = firstCredits(event, p, play);
  const at = standing(p, play);
  if (play.hook === "activate") {
    const cues: FxCue[] = first ? [motifBurst(cash, at, "area", frac(T.hatchAt, p.D), TUNING.hatch, "gold")] : [];
    cues.push(projectile(spec.scale, "gold", at, hero, frac(T.chipsAt, p.D), frac(T.chipsFor, p.D)));
    return { cues, row: "keep" };
  }
  const shower = frac(T.showerAt, p.D);
  const cues: FxCue[] = first ? [motifRing(spec, p.D, at, 0), motifBurst(spec, at, "ring", 0, TUNING.rings, "star")] : [];
  cues.push(motifBurst(cash, hero, "area", shower, TUNING.shower, "gold"));
  return { cues, row: "keep" };
};

/** The gallery's Credits: the Activate's middle mode. */
const GIFT_CREDITS = 10;

/** R3407000: Megabase's Activate gives its controller its Credits (one keyed event: the other player's is the module test's). */
export function giftShopFixture(_view: PlayerView, instanceId: string, player: PlayerId, defId = "stinkyscape-116"): GameEvent[] {
  return [
    { type: "hookResolving", sourceId: instanceId, defId, controller: player, hook: "activate" },
    { type: "marketCreditsChanged", player, value: GIFT_CREDITS },
  ];
}
