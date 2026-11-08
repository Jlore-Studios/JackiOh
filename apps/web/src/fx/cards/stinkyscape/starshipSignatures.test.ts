// R3334004: the Starship's three signatures (starshipSignatures.ts, preemptiveStrike.ts), planned the way
// FxLayer plans them: the Dormant Starship rolls out of its shipyard in weld sparks; Starfleet Commander
// sends a rocket bolt to its side's Dormant Starship and a ring, a crown of gold and a shake answer there,
// or on itself with none; Preemptive Strike sounds the alarm on its player's hero, then brands the ship it
// wakes red. A hidden play keys none of them, and no cue names a card (R202).

import type { GameEvent, PlayerId, PlayerView } from "@jackioh/shared";
import { describe, expect, it } from "vitest";

import { DORMANT_STARSHIP_ID } from "../../../cards/fused.ts";
import { MARK_PALETTES } from "../../../cards/marks.ts";
import { planEntries, type AnimationEntry } from "../../../game/animations.ts";
import { sideOf, testid } from "../../../game/contract.ts";
import { fullBoardView, withEvents } from "../../../test/fixtures.ts";
import { tid } from "../../build.ts";
import { planCardFx } from "../../cardFx.ts";
import { FX_BRAND_SLAM_AT, FX_INTENSITY_SCALE, FX_SLAM_AT } from "../../constants.ts";
import { planFx } from "../../cues.ts";
import { FLAVOR_LOOKS, paletteLook } from "../../flavors.ts";
import { createFxMemory } from "../../memory.ts";
import { MOTIF_FOOT } from "../../motifs/build.ts";
import { galleryBoardOf, galleryEntry, RECIPE_FIXTURES } from "../../motifs/gallery.ts";
import type { FxCue, FxPlanEnv, FxShotCue } from "../../types.ts";
import { ENTRANCE_TRAUMA } from "../recipe.ts";
import { PREEMPTIVE_BEATS, PREEMPTIVE_BRAND_TAIL_MS, PREEMPTIVE_TRAUMA } from "./preemptiveStrike.ts";
import { FLEET_ORDER_LANDS_AT, fleetBoard, fleetCommand, GALLERY_STARSHIP_LANE, GALLERY_STARSHIP_PIECES, shipyardRollout, WELDS } from "./starshipSignatures.ts";

const COMMANDER = "stinkyscape-137";
const STRIKE = "stinkyscape-176";
const I = FX_INTENSITY_SCALE.normal;
const ROCKET = paletteLook("Tech/rocket") ?? FLAVOR_LOOKS.Tech;
const LASER = paletteLook("Tech/laser") ?? FLAVOR_LOOKS.Tech;
const D = 400;

type Summoned = Extract<GameEvent, { type: "summoned" }>;

function landingOf(defId: string, player: PlayerId, lane: number, instanceId = "landing"): Summoned {
  return { type: "summoned", player, instanceId, defId, row: "units", lane };
}

function envOf(): FxPlanEnv {
  return { intensity: I, card: () => undefined, memory: createFxMemory() };
}

const played = (player: PlayerId, instanceId: string, defId: string): GameEvent => ({ type: "cardPlayed", player, instanceId, defId, costPaid: 2 });

/** A Unit's play and landing, planned as the board plans them over `board`, into an emptied zone. */
function planLanding(board: PlayerView, landing: Summoned): { cues: FxCue[]; D: number } {
  const side = sideOf(board, landing.player);
  const view: PlayerView = { ...board, [side]: { ...board[side], units: board[side].units.map((unit, k) => (k === landing.lane - 1 ? null : unit)) } };
  const events: GameEvent[] = [played(landing.player, landing.instanceId, landing.defId), landing];
  const env = envOf();
  const [entry] = planEntries(events, withEvents(view, events), false);
  if (entry === undefined) throw new Error("no entry");
  env.memory.remember(entry.events);
  return { cues: planFx(entry, view, env), D: entry.durationMs };
}

/** The gallery's ship: the instance id of the unit `fleetBoard` stands in `GALLERY_STARSHIP_LANE`. */
function shipOf(view: PlayerView): string {
  const ship = view.you.units[GALLERY_STARSHIP_LANE - 1];
  if (ship === null || ship === undefined) throw new Error("no ship");
  return ship.instanceId;
}

/** `events` as one entry of `view`, D long. */
function entryOf(events: GameEvent[], view: PlayerView): AnimationEntry {
  return { ...galleryEntry(events, view), durationMs: D };
}

const shotsOf = (cues: readonly FxCue[]): FxShotCue[] => cues.filter((cue): cue is FxShotCue => cue.kind === "shot");

describe("R3334004 the Starship's signatures", () => {
  it("R3334004 the Dormant Starship rolls out of its shipyard: welds before it settles, then a ring, dust at its foot and a shake", () => {
    const landing = landingOf(DORMANT_STARSHIP_ID, "p1", 3);
    const { cues, D } = planLanding(fullBoardView(), landing);
    const tgt = testid.zone("you", "units", 3);
    expect(cues).toEqual(shipyardRollout({ D, tgt, intensity: I, view: fullBoardView(), landing }));
    const slam = Math.round(FX_SLAM_AT * D);
    // A weld at each of its points, each before the slam, in rocket colors.
    const welds = cues.slice(0, WELDS.length);
    welds.forEach((weld, k) => {
      expect(weld).toMatchObject({ kind: "burst", preset: "spark", spread: "point", at: tid(tgt, WELDS[k]?.point), colors: ROCKET.colors });
      expect(weld.delayMs).toBeLessThan(slam);
    });
    // As it settles: a ring, dust at its foot and one shake, at the Dormant Starship's weight.
    const settled = cues.slice(WELDS.length);
    expect(settled.map((cue) => cue.kind)).toEqual(["ring", "burst", "shake"]);
    for (const cue of settled) expect(cue.delayMs).toBe(slam);
    expect(settled[1]).toMatchObject({ preset: "dust", at: tid(tgt, MOTIF_FOOT) });
    expect(cues.filter((cue) => cue.kind === "shake")).toEqual([{ kind: "shake", trauma: ENTRANCE_TRAUMA.shipyardRollout * I, delayMs: slam }]);
    // It throws no gold area burst, the shared Legendary entrance's, and no cue names a card.
    expect(cues.some((cue) => cue.kind === "burst" && cue.preset === "gold")).toBe(false);
    expect(JSON.stringify(cues)).not.toContain(DORMANT_STARSHIP_ID);
    // The beats scale with the entry.
    for (const d of [250, 400, 800]) {
      const scaled = shipyardRollout({ D: d, tgt, intensity: I });
      expect(scaled.slice(0, WELDS.length).map((cue) => cue.delayMs)).toEqual(WELDS.map((weld) => Math.round(weld.at * d)));
    }
  });

  it("R3334004 Starfleet Commander orders its side's Dormant Starship up: a rocket bolt to it, and there a ring, gold and a shake", () => {
    const view = fleetBoard(fullBoardView());
    const ship = shipOf(view);
    const landing = landingOf(COMMANDER, "p1", 3);
    const { cues, D } = planLanding(view, landing);
    const tgt = testid.zone("you", "units", 3);
    expect(cues).toEqual(fleetCommand({ D, tgt, intensity: I, view, landing }));
    const slam = Math.round(FX_SLAM_AT * D);
    const arrive = Math.round(FLEET_ORDER_LANDS_AT * D);
    const [bolt, ...rest] = shotsOf(cues);
    expect(rest).toEqual([]);
    // It leaves as the Commander lands and reaches the ship inside the entry.
    expect(bolt).toMatchObject({ from: tid(tgt), to: tid(testid.card(ship)), delayMs: slam, colors: ROCKET.colors });
    expect((bolt?.delayMs ?? 0) + (bolt?.flightMs ?? 0)).toBe(arrive);
    expect(arrive).toBeLessThanOrEqual(D);
    // There: a ring, a crown of gold and one shake, at the Legendary's weight.
    const answer = cues.filter((cue) => cue.kind !== "shot");
    expect(answer.map((cue) => cue.kind)).toEqual(["ring", "burst", "shake"]);
    for (const cue of answer) expect(cue.delayMs).toBe(arrive);
    expect(answer[1]).toMatchObject({ preset: "gold", at: tid(testid.card(ship)) });
    expect(cues.filter((cue) => cue.kind === "shake")).toEqual([{ kind: "shake", trauma: ENTRANCE_TRAUMA.fleetCommand * I, delayMs: arrive }]);
    // R202: no cue names a card.
    const text = JSON.stringify(cues);
    for (const defId of [COMMANDER, DORMANT_STARSHIP_ID, ...GALLERY_STARSHIP_PIECES]) expect(text).not.toContain(defId);
  });

  it("R3334004 with no Dormant Starship on its side, Starfleet Commander's ring, gold and shake are its own", () => {
    const tgt = testid.zone("you", "units", 3);
    const alone = (view: PlayerView | undefined, player: PlayerId = "p1"): void => {
      const cues = fleetCommand({ D, tgt, intensity: I, view, landing: landingOf(COMMANDER, player, 3) });
      expect(cues.map((cue) => cue.kind)).toEqual(["ring", "burst", "shake"]);
      for (const cue of cues) expect(cue.delayMs).toBe(Math.round(FX_SLAM_AT * D));
      expect(cues[1]).toMatchObject({ preset: "gold", at: tid(tgt) });
    };
    // No ship at all, or no board.
    alone(fullBoardView());
    alone(undefined);
    // The other side's ship is not its own: played by the other seat, it reads that side, none here.
    alone(fleetBoard(fullBoardView()), "p2");
    // A launched Starship is no Dormant one.
    const launched = fleetBoard(fullBoardView());
    const units = launched.you.units.map((unit) => (unit?.starship === undefined ? unit : { ...unit, starship: { ...unit.starship, name: "Ember of Departure", launched: 1 } }));
    alone({ ...launched, you: { ...launched.you, units } });
  });

  it("R3334004 Preemptive Strike sounds the alarm on its hero, then brands the Dormant Starship it wakes", () => {
    const view = fleetBoard(fullBoardView());
    const ship = shipOf(view);
    const env = envOf();
    const play = played("p1", "ps", STRIKE);
    env.memory.remember([play]);
    const hero = tid(testid.hero("you"));
    // Its own play: rings of fire on its player's hero at 0 and 0.3 of D, and a red strike with a shake at a quarter.
    const alarm = planCardFx(play, { entry: entryOf([play], view), view, env, D });
    expect(alarm?.row).toBe("keep");
    const strike = Math.round(PREEMPTIVE_BEATS.strikeAt * D);
    expect(alarm?.cues.map((cue) => [cue.kind, cue.delayMs])).toEqual([
      ...PREEMPTIVE_BEATS.alarmAt.map((share) => ["ring", Math.round(share * D)]),
      ["burst", strike],
      ["shake", strike],
    ]);
    for (const cue of alarm?.cues ?? []) if ("at" in cue) expect(cue.at).toEqual(hero);
    expect(alarm?.cues[2]).toMatchObject({ preset: LASER.particle, colors: LASER.colors });
    expect(alarm?.cues[3]).toEqual({ kind: "shake", trauma: PREEMPTIVE_TRAUMA.strike * I, delayMs: strike });

    // The Dormant Starship it wakes: a red brand on the ship for D and PREEMPTIVE_BRAND_TAIL_MS, a ring and a softer shake.
    const awakened: GameEvent = { type: "awakened", player: "p1", instanceId: ship, defId: DORMANT_STARSHIP_ID };
    env.memory.remember([awakened]);
    const woke = planCardFx(awakened, { entry: entryOf([awakened], view), view, env, D });
    const slam = Math.round(FX_BRAND_SLAM_AT * D);
    expect(woke?.row).toBe("keep");
    expect(woke?.cues).toEqual([
      { kind: "brand", at: tid(testid.card(ship)), tint: { rim: MARK_PALETTES.red.rim, core: MARK_PALETTES.red.core, glow: MARK_PALETTES.red.glow }, delayMs: 0, durationMs: D + PREEMPTIVE_BRAND_TAIL_MS },
      expect.objectContaining({ kind: "ring", at: tid(testid.card(ship)), delayMs: slam }),
      { kind: "shake", trauma: PREEMPTIVE_TRAUMA.brand * I, delayMs: slam },
    ]);
    // Only the first awakened of its play: a second draws its own row alone.
    const again: GameEvent = { type: "awakened", player: "p1", instanceId: "other", defId: DORMANT_STARSHIP_ID };
    env.memory.remember([again]);
    expect(planCardFx(again, { entry: entryOf([again], view), view, env, D })).toBeNull();
    // R202: no cue names a card.
    expect(JSON.stringify([alarm, woke])).not.toMatch(/stinkyscape-/);
  });

  it("R3334004 Preemptive Strike brands its player's hero when the board cannot place the ship, and keys nothing of another play", () => {
    const view = fullBoardView();
    // The other seat's play, its ship off the board: the brand falls on that seat's hero.
    const env = envOf();
    const play = played("p2", "ps", STRIKE);
    const awakened: GameEvent = { type: "awakened", player: "p2", instanceId: "fx-starship", defId: DORMANT_STARSHIP_ID };
    env.memory.remember([play, awakened]);
    const woke = planCardFx(awakened, { entry: entryOf([awakened], view), view, env, D });
    expect(woke?.cues[0]).toMatchObject({ kind: "brand", at: tid(testid.hero("opponent")) });
    // A War Doctrine it casts is a play of its own, which its own entry draws.
    const doctrine = played("p2", "doctrine", "stinkyscape-137-1");
    env.memory.remember([doctrine]);
    expect(planCardFx(doctrine, { entry: entryOf([doctrine], view), view, env, D })).toBeNull();
  });

  it("R3334004 a hidden play or summon keys none of the three", () => {
    const view = fleetBoard(fullBoardView());
    const env = envOf();
    const hidden = played("p2", "hidden", "hidden");
    const awakened: GameEvent = { type: "awakened", player: "p2", instanceId: shipOf(view), defId: DORMANT_STARSHIP_ID };
    env.memory.remember([hidden, awakened]);
    expect(planCardFx(awakened, { entry: entryOf([awakened], view), view, env, D })).toBeNull();
    // A hidden summon names no entrance.
    for (const landing of [landingOf("hidden", "p1", 3, "hidden"), landingOf("hidden", "p2", 3, "hidden")]) {
      expect(planCardFx(landing, { entry: entryOf([landing], view), view, env: envOf(), D })).toBeNull();
    }
  });

  it("R3334004 the gallery plays Starfleet Commander over a Dormant Starship of three pieces, and Preemptive Strike's play wakes one", () => {
    const base = fullBoardView();
    const board = galleryBoardOf(COMMANDER, base);
    expect(board).toEqual(fleetBoard(base));
    const ship = board.you.units[GALLERY_STARSHIP_LANE - 1];
    expect(ship).toMatchObject({ defId: DORMANT_STARSHIP_ID, instanceId: base.you.units[GALLERY_STARSHIP_LANE - 1]?.instanceId, attack: 0, canAct: false });
    expect(ship?.starship).toEqual({ pieces: GALLERY_STARSHIP_PIECES.map((defId) => ({ defId })) });
    // Every other zone stays as it was; the Dormant Starship and Preemptive Strike play over the board as it is.
    expect(board.you.units.filter((_, k) => k !== GALLERY_STARSHIP_LANE - 1)).toEqual(base.you.units.filter((_, k) => k !== GALLERY_STARSHIP_LANE - 1));
    expect(galleryBoardOf(DORMANT_STARSHIP_ID, base)).toBe(base);
    expect(galleryBoardOf(STRIKE, base)).toBe(base);
    expect(RECIPE_FIXTURES.preemptiveStrike(base, "fx-gallery-card", "p1")).toEqual([{ type: "awakened", player: "p1", instanceId: "fx-starship", defId: DORMANT_STARSHIP_ID }]);
  });
});
