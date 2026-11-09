// R2784000: the system moments of issue #2802 (docs/polish/9-card-animations.md): each set's
// mechanics as a Signature in the motif language, reusing existing cue kinds only. `questCompleted`
// is the one set piece (R2785000, a system's climax); everything else holds nothing, with
// `questProgressed` as a `tally` Beat. `transformed`, `fused`, `upgraded` and `degraded` all plan
// through one shared `transmute` (board.ts). `chaosRolled` and `rolled` keep their overlays (R436,
// R2652001). R200: every delay and flight lies inside the entry, every tail is an existing one.
// R202: nothing reads a defId except through `env.card` with the sentinel guard, and no cue carries
// one or a name; hidden Market and vote cards plan the same cues whatever they hide.

import type { GameEvent, PlayerView } from "@jackioh/shared";

import { HIDDEN_ID, animTestid, targetFor, type AnimationEntry, type SetPieceClaim } from "../game/animations.ts";
import { sideOf, testid } from "../game/contract.ts";
import { bannerCue as bannerOf, frac, ghostCue as ghostOf, raysCue, ringCue, shakeCues, tid, tunedBurst, viewportCenter as center } from "./build.ts";
import {
  FX_CRACK_TAIL_MS,
  FX_JADE_SCALE_MAX,
  FX_JADE_SCALE_STEP,
  FX_PROJECTILE_FLIGHT_FRACTION,
  FX_QUEST_HOLD_MS,
  FX_QUEST_TRAUMA,
  FX_REWIND_TRAUMA,
  FX_SYSTEM_TRAUMA,
  FX_TEXT,
} from "./constants.ts";
import { FLAVOR_LOOKS, paletteLook } from "./flavors.ts";
import { flavorLookOf } from "./flavorSource.ts";
import { MOTIF_TUNING, hitAnchor, motifRing, motifShot, timesOf, type MotifSpec } from "./motifs/build.ts";
import { planMotif } from "./motifs/index.ts";
import type { FxAnchor, FxBannerTone, FxCue, FxPlanEnv } from "./types.ts";

/** R2785000: the key of the quest's set piece, the one set piece of this change. */
export const QUEST_SET_PIECE = "quest";

/** R2785000: the entry holding a `questCompleted` claims the quest's beat; anything else claims nothing. */
export function questClaim(entry: AnimationEntry): SetPieceClaim | null {
  return entry.events.some((event) => event.type === "questCompleted") ? { key: QUEST_SET_PIECE, holdMs: FX_QUEST_HOLD_MS } : null;
}

/**
 * Every event with a moment in this change: the table's rows except `chaosRolled` and `rolled`,
 * which keep their overlays (R436, R2652001). The transform family plans through `transmuteCues`;
 * the rest plan through `planSystemMoment`.
 */
export const SYSTEM_MOMENTS = [
  "questProgressed",
  "questCompleted",
  "jadeChanged",
  "marketCreditsChanged",
  "marketBought",
  "marketSold",
  "voteCast",
  "votedOut",
  "starscapeTravelled",
  "fled",
  "glitched",
  "awakened",
  "boardReplaced",
  "rolledBack",
  "fengShui",
  "transformed",
  "fused",
  "upgraded",
  "degraded",
] as const;

export type SystemMoment = (typeof SYSTEM_MOMENTS)[number];

/**
 * The shared transmute spec: the resolving play's flavor look, else the arcane (Nerd) look
 * (there is no `FLAVOR_LOOKS.Arcane`; Nerd's particle is arcane), at the intensity.
 */
export function transmuteSpec(_entry: AnimationEntry, view: PlayerView, env: FxPlanEnv): MotifSpec {
  const play = env.memory.resolving();
  const look =
    play === undefined || play.defId === HIDDEN_ID
      ? FLAVOR_LOOKS.Nerd
      : (flavorLookOf(play.defId, play.instanceId, view) ?? FLAVOR_LOOKS.Nerd);
  return { motif: "transmute", look, scale: env.intensity, row: "replace" };
}

/** The transform family through one shared transmute: no event draws its own copy. */
export function transmuteCues(entry: AnimationEntry, view: PlayerView, env: FxPlanEnv): FxCue[] {
  return planMotif(transmuteSpec(entry, view, env), entry, view, env);
}

function viewportAt(x: number, y: number): FxAnchor {
  return { kind: "viewport", at: { x, y } };
}

function heroSide(view: PlayerView, player: Parameters<typeof sideOf>[1]): FxAnchor {
  return tid(testid.hero(sideOf(view, player)));
}

/** A gold coin flight that lands inside the entry (R200). */
function coinFlight(intensity: number, from: FxAnchor, to: FxAnchor, D: number): FxCue {
  return { kind: "projectile", preset: "gold", from, to, delayMs: 0, flightMs: frac(FX_PROJECTILE_FLIGHT_FRACTION, D), density: intensity };
}

export type SystemPlan = { cues: FxCue[]; replaces: ReadonlySet<GameEvent> };

/**
 * Every system moment except the transform family's (which plans through `transmuteCues`): its
 * Signature in the motif language, holding nothing. Events in `skip` (already drawn by a card
 * motif, a set piece or a beat) are left alone. The row banner stays for `votedOut`, `fled`,
 * `glitched` and `fengShui`: this decorates it only.
 */
export function planSystemMoment(
  entry: AnimationEntry,
  view: PlayerView,
  env: FxPlanEnv,
  skip: ReadonlySet<GameEvent>,
): SystemPlan {
  const cues: FxCue[] = [];
  const replaces = new Set<GameEvent>();
  const D = entry.durationMs;
  const i = env.intensity;
  if (!(i > 0) || !(D > 0)) return { cues, replaces };
  const t = timesOf(D);
  const skipped = (event: GameEvent): boolean => skip.has(event);

  for (const event of entry.events) {
    if (skipped(event)) continue;
    switch (event.type) {
      case "questProgressed": {
        const spec: MotifSpec = { motif: "tally", look: FLAVOR_LOOKS.Holy, scale: i, row: "replace" };
        const sub: AnimationEntry = { ...entry, events: [event] };
        cues.push(...planMotif(spec, sub, view, env));
        replaces.add(event);
        break;
      }
      case "questCompleted": {
        const tracker = targetFor(event, view);
        const at = tracker === null ? tid(testid.board) : tid(tracker);
        const hero = heroSide(view, event.player);
        const tone: FxBannerTone = sideOf(view, event.player);
        cues.push(bannerOf(D, FX_TEXT.questComplete, tone));
        cues.push(raysCue(D, "radiant", at, 0));
        cues.push({ ...tunedBurst(i, "gold", at, "area", t.L, MOTIF_TUNING.land), colors: FLAVOR_LOOKS.Holy.colors });
        cues.push(coinFlight(i, at, hero, D));
        cues.push(...shakeCues(i, FX_QUEST_TRAUMA, t.L));
        replaces.add(event);
        break;
      }
      case "jadeChanged": {
        const jade = paletteLook("Hustle/jade") ?? FLAVOR_LOOKS.Hustle;
        const scale = i * Math.min(FX_JADE_SCALE_MAX, 1 + event.value * FX_JADE_SCALE_STEP);
        const spec: MotifSpec = { motif: "tally", look: jade, scale, row: "replace" };
        const sub: AnimationEntry = { ...entry, events: [event] };
        cues.push(...planMotif(spec, sub, view, env));
        replaces.add(event);
        break;
      }
      case "marketCreditsChanged": {
        const hero = heroSide(view, event.player);
        const hustle = FLAVOR_LOOKS.Hustle;
        cues.push({ ...tunedBurst(i, "gold", hero, "area", t.L, MOTIF_TUNING.land), colors: hustle.colors });
        cues.push(motifRing({ look: hustle, scale: i, row: "keep" }, D, hero, t.L));
        break;
      }
      case "marketBought": {
        const side = sideOf(view, event.player);
        const hero = tid(testid.hero(side));
        const market = tid(testid.marketplace);
        const hand = tid(animTestid.hand(side));
        const hustle = FLAVOR_LOOKS.Hustle;
        cues.push(coinFlight(i, hero, market, D));
        cues.push(ghostOf(D, market, hand));
        cues.push({ ...tunedBurst(i, "sparkle", hand, "point", D, MOTIF_TUNING.small), colors: hustle.colors });
        break;
      }
      case "marketSold": {
        const side = sideOf(view, event.player);
        const hero = tid(testid.hero(side));
        const market = tid(testid.marketplace);
        const hand = tid(animTestid.hand(side));
        cues.push(ghostOf(D, hand, market));
        cues.push(coinFlight(i, market, hero, D));
        break;
      }
      case "voteCast": {
        const from = targetFor(event, view);
        if (from === null) break;
        const hero = heroSide(view, event.player);
        cues.push(ghostOf(D, tid(from), hero));
        cues.push({ ...tunedBurst(i, "sparkle", hero, "point", D, MOTIF_TUNING.small), colors: FLAVOR_LOOKS.Holy.colors });
        break;
      }
      case "votedOut": {
        const hero = heroSide(view, event.player);
        cues.push({ kind: "crack", at: hero, delayMs: 0, durationMs: D + FX_CRACK_TAIL_MS });
        cues.push(...shakeCues(i, FX_SYSTEM_TRAUMA, 0));
        break;
      }
      case "fled": {
        const hero = heroSide(view, event.player);
        const out = viewportAt(0.5, 1);
        cues.push(ghostOf(D, hero, out));
        cues.push({ ...tunedBurst(i, "smoke", out, "area", D, MOTIF_TUNING.small) });
        break;
      }
      case "glitched": {
        const board = tid(testid.board);
        const chaos = FLAVOR_LOOKS.Chaos;
        cues.push({ kind: "sheen", at: board, delayMs: 0, durationMs: D });
        cues.push({ ...tunedBurst(i, "prismatic", board, "area", t.L, MOTIF_TUNING.land), colors: chaos.colors });
        cues.push(...shakeCues(i, FX_SYSTEM_TRAUMA, t.L));
        break;
      }
      case "fengShui": {
        const at = hitAnchor(view, event.instanceId) ?? tid(testid.board);
        const holy = FLAVOR_LOOKS.Holy;
        cues.push(motifRing({ look: holy, scale: i, row: "keep" }, D, at, t.L));
        cues.push({ ...tunedBurst(i, "sparkle", at, "point", t.L, MOTIF_TUNING.small), colors: holy.colors });
        break;
      }
      case "starscapeTravelled": {
        const hero = heroSide(view, event.player);
        const star = paletteLook("Tech/starfield") ?? FLAVOR_LOOKS.Spirit;
        const spec = { look: star, scale: i, row: "keep" as const };
        cues.push(motifShot(spec, "beam", viewportAt(0, 0.5), viewportAt(1, 0.5), 0, D));
        cues.push({ ...tunedBurst(i, "star", hero, "area", t.L, MOTIF_TUNING.land), colors: star.colors });
        cues.push(motifRing(spec, D, hero, t.L));
        break;
      }
      case "awakened": {
        const at = targetFor(event, view);
        if (at === null) break;
        const spec: MotifSpec = { motif: "rise", look: FLAVOR_LOOKS.Holy, scale: i, row: "replace" };
        const sub: AnimationEntry = { ...entry, events: [event] };
        const rise = planMotif(spec, sub, view, env);
        if (rise.length === 0) break;
        cues.push(...rise);
        cues.push(raysCue(D, "holy", tid(at), t.L));
        replaces.add(event);
        break;
      }
      case "boardReplaced": {
        const board = tid(testid.board);
        const nerd = FLAVOR_LOOKS.Nerd;
        cues.push(ringCue(D, "arcane", board, 0));
        cues.push({ ...tunedBurst(i, "arcane", board, "area", t.L, MOTIF_TUNING.land), colors: nerd.colors });
        cues.push({ ...tunedBurst(i, "arcane", center(), "ring", t.L, MOTIF_TUNING.small), colors: nerd.colors });
        cues.push(...shakeCues(i, FX_SYSTEM_TRAUMA, t.L));
        break;
      }
      case "rolledBack": {
        cues.push({ kind: "sheen", at: tid(testid.board), delayMs: 0, durationMs: D });
        cues.push({ ...tunedBurst(i, "arcane", center(), "ring", 0, MOTIF_TUNING.land), colors: FLAVOR_LOOKS.Nerd.colors });
        cues.push(...shakeCues(i, FX_REWIND_TRAUMA, 0));
        replaces.add(event);
        break;
      }
      default:
        break;
    }
  }
  return { cues, replaces };
}
