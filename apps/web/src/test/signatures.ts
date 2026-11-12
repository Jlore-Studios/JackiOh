// R3405001: what Meditative's and Meditative+'s signature tests (fx/cards/meditative/signatures*.test.ts,
// fx/cards/meditativeplus/signatures.test.ts) share: a card's play or hook planned as the board's effects
// layer plans it, an entry each (remember, then plan through the dispatch, cardFx.ts `planCardFx`); its own
// cast under a Mythic's takeover hold; its entrance as it lands, after its entry's lead (R3405000); a cue's
// end as R200 counts it; and, for every effects test (#3701), the one `endOf` and the one `envOf`.

import type { GameEvent, PlayerId, PlayerView } from "@jackioh/shared";

import { TAKEOVER_HOLD_MS, TAKEOVER_SET_PIECE } from "../game/animations.ts";
import { planCardFx } from "../fx/cardFx.ts";
import { FX_CARD_TABLE, type FxCardEntry } from "../fx/cards/tables.ts";
import { FX_INTENSITY_SCALE, FX_MAX_PARTICLE_LIFE_MS } from "../fx/constants.ts";
import { createFxMemory } from "../fx/memory.ts";
import { galleryEntry } from "../fx/motifs/gallery.ts";
import type { FxAnchor, FxCue, FxPlanEnv, FxPoint } from "../fx/types.ts";
import { fullBoardView } from "./fixtures.ts";

/** A Mythic's takeover hold, the lead its cast and its entrance start after. */
export const HOLD = TAKEOVER_HOLD_MS;

/** The effects tests' planning setup: normal intensity, no card facts and a memory of its own, `over` replacing any of them (#3701). */
export function envOf(over: Partial<FxPlanEnv> = {}): FxPlanEnv {
  return { intensity: FX_INTENSITY_SCALE.normal, card: () => undefined, memory: createFxMemory(), ...over };
}

/** The cues that fly: a `flightMs`, no `durationMs`, and a burst where they land. */
type FlightCue = Extract<FxCue, { flightMs: number }>;

/** Every `FxCue` kind with a `flightMs`, and no other: a new flying kind fails `typecheck` here until it is listed, so `endOf` counts its flight (#3701). */
const FLIGHT_KINDS: Record<FlightCue["kind"], true> = { projectile: true, shot: true };

const flies = (cue: FxCue): cue is FlightCue => Object.hasOwn(FLIGHT_KINDS, cue.kind);

/** Where a cue ends, in ms after its entry starts (R200's bound is D + FX_MAX_TAIL_MS): a burst's particles burn out, a flight lands and bursts, a shake rides its delay, the rest run their `durationMs`. */
export function endOf(cue: FxCue): number {
  if (flies(cue)) return cue.delayMs + cue.flightMs + FX_MAX_PARTICLE_LIFE_MS;
  switch (cue.kind) {
    case "burst":
      return cue.delayMs + FX_MAX_PARTICLE_LIFE_MS;
    case "shake":
      return cue.delayMs;
    default:
      return cue.delayMs + ("durationMs" in cue ? cue.durationMs : 0);
  }
}

/** Each cue as `<kind>@<delay>`, a figure as `figure:<name>@<delay>`. */
export const beats = (cues: readonly FxCue[]): string[] => cues.map((cue) => `${cue.kind === "figure" ? `figure:${cue.figure}` : cue.kind}@${String(cue.delayMs)}`);

export const tid = (value: string, at?: FxPoint): FxAnchor => (at === undefined ? { kind: "testid", testid: value } : { kind: "testid", testid: value, at });

export const played = (player: PlayerId, instanceId: string, defId: string): Extract<GameEvent, { type: "cardPlayed" }> => ({ type: "cardPlayed", player, instanceId, defId, costPaid: 1 });

export const added = (player: PlayerId, instanceId: string, defId: string): GameEvent => ({ type: "addedToHand", player, instanceId, defId });

/** One entry's planned cues and its row (`undefined` when no recipe claimed it). */
export type Planned = { cues: FxCue[]; row: string | undefined };

/** The play's own cast, its entry held `hold` (a Mythic's takeover) and lasting D. */
export function castOf(defId: string, D: number, hold: number = HOLD, player: PlayerId = "p1", view: PlayerView = fullBoardView()): Planned {
  const env = envOf();
  const cast = played(player, "ex", defId);
  env.memory.remember([cast]);
  const entry = { ...galleryEntry([cast], view), durationMs: D, ...(hold > 0 ? { setPiece: { key: TAKEOVER_SET_PIECE, holdMs: hold } } : {}) };
  const result = planCardFx(cast, { entry, view, env, D });
  return { cues: result?.cues ?? [], row: result?.row };
}

/**
 * Plays `defId` as `player` and each of `events` under it, an entry each lasting D, as the layer does:
 * remember, then plan through the dispatch. With `hook`, the events run under that hook's marker
 * (`hookResolving`) of the card on the board as `instanceId` "ex" in place of its play (R2797000).
 * With `table`, the dispatch reads those entries alone (R3399000: a summoned card's own entrance is that card's).
 */
export function run(
  defId: string,
  events: readonly GameEvent[],
  D: number,
  player: PlayerId = "p1",
  view: PlayerView = fullBoardView(),
  hook?: Exclude<HookKind, "cry">,
  table: ReadonlyMap<string, FxCardEntry> = FX_CARD_TABLE,
): Planned[] {
  const env = envOf();
  env.memory.remember([hook === undefined ? played(player, "ex", defId) : { type: "hookResolving", sourceId: "ex", defId, controller: player, hook }]);
  return events.map((event) => {
    env.memory.remember([event]);
    const entry = { ...galleryEntry([event], view), durationMs: D };
    const result = planCardFx(event, { entry, view, env, D }, table);
    return { cues: result?.cues ?? [], row: result?.row };
  });
}

/** Its landing (`summoned`) through the dispatch, its entry lasting D and held `hold` first (a Mythic's takeover). */
export function landOf(landing: Extract<GameEvent, { type: "summoned" }>, D: number, hold = 0, view: PlayerView = fullBoardView()): Planned {
  const env = envOf();
  env.memory.remember([played(landing.player, landing.instanceId, landing.defId), landing]);
  const entry = { ...galleryEntry([landing], view), durationMs: D, ...(hold > 0 ? { setPiece: { key: TAKEOVER_SET_PIECE, holdMs: hold } } : {}) };
  const result = planCardFx(landing, { entry, view, env, D });
  return { cues: result?.cues ?? [], row: result?.row };
}

/** The hook a card's `hookResolving` marker names (a Cry opens none). */
type HookKind = Extract<GameEvent, { type: "hookResolving" }>["hook"];

export const cuesOf = (results: readonly Planned[]): FxCue[][] => results.map((result) => result.cues);
