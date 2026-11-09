// R3407000: Stinkyscape's two Calls to Chaos and their Golem (chaosCalls.ts), planned as FxLayer plans them
// (remember, then plan): the Stinky Edition's stink cloud, its roll's capped panel and ring, the card backs,
// puffs and tethers it spits out; the Golem's crash landing, after its hold when it has one; the Scape
// Edition's indigo sky, starfield and tilt, its roll, its drones and its rings, yielding to the Anomaly's
// rift and the Starship's set pieces. Each gallery play keeps its ceiling, each outcome beat keeps it from
// its own entry, nothing draws with the effects off, R200 holds at every D, and from the caster's
// opponent's seat no cue names a card (R202).

import type { GameEvent, PlayerId, PlayerView } from "@jackioh/shared";
import { describe, expect, it } from "vitest";

import { ALLURE_MARK, ALLURE_MARK_COLOR } from "../../../cards/marks.ts";
import { animTestid, HIDDEN_ID } from "../../../game/animations.ts";
import { LANES, testid } from "../../../game/contract.ts";
import { FROZEN_MARK, FROZEN_MARK_COLOR } from "../../../stats/config.ts";
import { fullBoardView } from "../../../test/fixtures.ts";
import { ceilingOf, endOf, envOf, expectGalleryWithinCeiling, expectR200, latestEnd, planWithOffsets, type PlannedEntry } from "../../../test/signaturePass.ts";
import { frac } from "../../build.ts";
import { planCardFx } from "../../cardFx.ts";
import { chaosLandAt } from "../../chaos.ts";
import {
  FX_CENTER,
  FX_CHAOS_DICE_PANEL_MS,
  FX_CHAOS_DICE_REEL_MS,
  FX_INTENSITY_SCALE,
  FX_MAX_TAIL_MS,
  FX_SLAM_AT,
  SIGNATURE_PERMANENT_MAX_TRAUMA,
  SIGNATURE_SPELL_MAX_TRAUMA,
} from "../../constants.ts";
import { planFx } from "../../cues.ts";
import { ENTRANCES } from "../../entrances.ts";
import { FLAVOR_LOOKS } from "../../flavors.ts";
import { galleryEntry } from "../../motifs/gallery.ts";
import type { FxAnchor, FxCue, FxPlanEnv } from "../../types.ts";
import { ENTRANCE_TRAUMA, FOOT } from "../recipe.ts";
import { FX_CARD_TABLE } from "../tables.ts";
import { markLook } from "./markLook.ts";
import { SCAPE_GALLERY_EFFECT, STINKY_GALLERY_EFFECT, STINKYSCAPE_GOLEM_ID, stinkCloudFixture, tabletopTiltFixture } from "./chaosCalls.ts";
import { landMsAfter } from "./signature.ts";

const STINKY = "stinkyscape-150";
const GOLEM = STINKYSCAPE_GOLEM_ID;
const SCAPE = "stinkyscape-151";
/** Stinkyscape #7 The Greatest Heist, a Quest Field Spell, landing with an entrance of its own (R3395000). */
const HEIST = "stinkyscape-007";
const ID = "fx-chaos";
const DS = [120, 200, 400, 732, 1400] as const;
/** R202: a card id of any set, and an anchor reaching for a card in a hand. */
const CARD_ID = /(core|classic|meditative|stinkyscape|traininggrounds)[a-z]*-\d/;
const HAND_CARD = '"kind":"handCard"';

/** The Stinky Edition's and the Scape Edition's clauses `chaosRolled` names (crates/engine/src/subsystems). */
const STINKY_HAND = "Add A Very Dandy Lad and an Arctic Citadel to your hand, which cost (0)";
const STINKY_FIELDS = "Summon 3 random Quest or Wincon Field Spells into your backrow";
const STINKY_FREEZE = "Switch every enemy Unit to Attack Position and Freeze them";
const STINKY_ALLURE = "Allure every enemy Unit";
const SCAPE_ANOMALY = "Replace your side of the field with a random Anomaly, any of the ten";
const SCAPE_PIECES = "Add 3 random Starship Pieces to your hand, which cost (0)";

const tid = (value: string, at?: { x: number; y: number }): FxAnchor => (at === undefined ? { kind: "testid", testid: value } : { kind: "testid", testid: value, at });
const CENTRE = tid(testid.board, FX_CENTER);
const kinds = (cues: readonly FxCue[]): string[] => cues.map((cue) => cue.kind);

const played = (player: PlayerId, defId: string): Extract<GameEvent, { type: "cardPlayed" }> => ({ type: "cardPlayed", player, instanceId: ID, defId, costPaid: 4 });
const resolved = (player: PlayerId, defId: string): GameEvent => ({ type: "cardResolved", player, instanceId: ID, defId, permanent: false, costPaid: 4 });
const rolled = (player: PlayerId, defId: string, effects: string[]): Extract<GameEvent, { type: "chaosRolled" }> => ({ type: "chaosRolled", player, instanceId: ID, defId, effects });
const added = (player: PlayerId): GameEvent => ({ type: "addedToHand", player, instanceId: "fx-added", defId: "core-003" });
const summoned = (player: PlayerId, instanceId: string, defId: string, row: "units" | "backrow", lane: number): GameEvent => ({ type: "summoned", player, instanceId, defId, row, lane });
const marked = (instanceId: string, mark: string, color: string, isAdded = true): GameEvent => ({ type: "marked", instanceId, mark, color, added: isAdded });

type Planned = { cues: FxCue[]; row: string | undefined };

/** Each of `events` as its own entry lasting D, after `before`: remember, then the signature's answer (its entrance and its recipe). */
function signature(events: readonly GameEvent[], D: number, before: readonly GameEvent[], view: PlayerView = fullBoardView(), env: FxPlanEnv = envOf()): Planned[] {
  env.memory.remember(before);
  return events.map((event) => {
    env.memory.remember([event]);
    const entry = { ...galleryEntry([event], view), durationMs: D };
    const result = planCardFx(event, { entry, view, env, D });
    return { cues: result?.cues ?? [], row: result?.row };
  });
}

/** Each of `events` as its own entry lasting D, planned whole (rows and all), for R200. */
function entries(events: readonly GameEvent[], D: number, view: PlayerView = fullBoardView(), env: FxPlanEnv = envOf()): PlannedEntry[] {
  return events.map((event) => {
    env.memory.remember([event]);
    const entry = { ...galleryEntry([event], view), durationMs: D };
    return { entry, offset: 0, cues: planFx(entry, view, env) };
  });
}

/** `defId` cast by `player`: its play, its roll of `effects`, `outcome`, and its resolve. */
function call(defId: string, player: PlayerId, effects: string[], outcome: readonly GameEvent[]): GameEvent[] {
  return [played(player, defId), rolled(player, defId, effects), ...outcome, resolved(player, defId)];
}

/** The latest end of the entry holding `type` and every entry after it, from that entry's start (its marker). */
function endFromMarker(run: readonly PlannedEntry[], type: GameEvent["type"]): number {
  const k = run.findIndex((planned) => planned.entry.events.some((event) => event.type === type));
  const marker = run[k];
  if (marker === undefined) throw new Error(`no ${type} entry`);
  return latestEnd(run.slice(k)) - marker.offset;
}

/** R202: no cue names a card or reaches into a hand. */
function expectNoCard(cues: readonly FxCue[], where: string): void {
  for (const cue of cues) {
    expect(JSON.stringify(cue), where).not.toMatch(CARD_ID);
    expect(JSON.stringify(cue), where).not.toContain(HAND_CARD);
  }
}

describe("R3407000 Stinkyscape's Calls to Chaos and their Golem", () => {
  it("R3407000 the three table entries are signatures, the Golem's tile in stink vapour", () => {
    expect(FX_CARD_TABLE.get(STINKY)).toEqual({ kind: "bespoke", key: "stinkCloud", signature: true });
    expect(FX_CARD_TABLE.get(GOLEM)).toEqual({ kind: "hooks", hooks: {}, entrance: "golemCrash", aura: { look: "stinkvapour", palette: "Toxic", scope: "tile" }, signature: true });
    expect(FX_CARD_TABLE.get(SCAPE)).toEqual({ kind: "bespoke", key: "tabletopTilt", signature: true });
  });

  it("R3407000 the Golem crash-lands from the board's top edge on a green exhaust trail, a ring, scrap stones and a stink puff at its slam, after its hold when it has one", () => {
    const D = 400;
    const tgt = testid.zone("you", "units", 3);
    const slam = frac(FX_SLAM_AT, D);
    const cues = ENTRANCES.golemCrash({ D, tgt, intensity: FX_INTENSITY_SCALE.normal });
    expect(kinds(cues)).toEqual(["shot", "ring", "burst", "burst", "burst", "shake"]);
    expect(cues[0]).toMatchObject({ from: tid(testid.board, { x: 0.5, y: 0 }), to: tid(tgt), delayMs: 0, flightMs: slam, preset: "exhaust", colors: FLAVOR_LOOKS.Toxic.colors });
    expect(cues[1]).toMatchObject({ at: tid(tgt), delayMs: slam });
    expect(cues[2]).toMatchObject({ at: tid(tgt, FOOT), preset: "exhaust", delayMs: slam });
    expect(cues[3]).toMatchObject({ at: tid(tgt, FOOT), preset: "crumb", spread: "ring", delayMs: slam });
    expect(cues[4]).toMatchObject({ at: tid(tgt, FOOT), preset: "smoke", colors: FLAVOR_LOOKS.Toxic.colors });
    expect(cues[4]?.delayMs).toBeGreaterThan(slam);
    expect(cues[5]).toEqual({ kind: "shake", trauma: ENTRANCE_TRAUMA.golemCrash, delayMs: slam });
    expect(ENTRANCE_TRAUMA.golemCrash).toBeLessThanOrEqual(SIGNATURE_PERMANENT_MAX_TRAUMA);
    // Its hold first: every beat after it, the slam FX_SLAM_AT of what follows.
    const wait = 150;
    const held = ENTRANCES.golemCrash({ D, tgt, intensity: FX_INTENSITY_SCALE.normal, landMs: landMsAfter(wait, D) });
    for (const cue of held) expect(cue.delayMs, cue.kind).toBeGreaterThanOrEqual(wait);
    expect(held[5]).toMatchObject({ delayMs: wait + frac(FX_SLAM_AT, D - wait) });
  });

  it("R3407000 the Stinky Edition's cast: a lime-green stink cloud bursts at the board's centre, flies spiralling out of it", () => {
    const D = 400;
    const [cast] = signature([played("p1", STINKY)], D, []);
    expect(cast?.row).toBe("replace");
    expect(kinds(cast?.cues ?? [])).toEqual(["burst", "burst", "ring", "burst"]);
    for (const cue of cast?.cues ?? []) expect(cue, cue.kind).toMatchObject({ at: CENTRE });
    expect(cast?.cues[0]).toMatchObject({ preset: "poison", colors: FLAVOR_LOOKS.Toxic.colors });
    expect(cast?.cues[1]).toMatchObject({ preset: "smoke", colors: FLAVOR_LOOKS.Toxic.colors });
    expect(cast?.cues[3]).toMatchObject({ preset: "ember", spread: "ring" });
  });

  it("R3407000 each Call's roll: R436's panel names it within the cap, a ring at the centre as its first line lands", () => {
    const D = 900;
    for (const [defId, effects] of [
      [STINKY, [STINKY_GALLERY_EFFECT]],
      [SCAPE, [SCAPE_GALLERY_EFFECT]],
      [STINKY, [STINKY_HAND, STINKY_FREEZE, STINKY_ALLURE]],
    ] as const) {
      const [roll] = signature([rolled("p1", defId, [...effects])], D, [played("p1", defId)]);
      expect(roll?.row, defId).toBe("replace");
      expect(kinds(roll?.cues ?? []), defId).toEqual(["chaos", "ring"]);
      const panel = roll?.cues[0];
      if (panel?.kind !== "chaos") throw new Error("no panel");
      expect(panel.durationMs).toBe(FX_CHAOS_DICE_PANEL_MS);
      expect(panel.lines.map((line) => line.text)).toEqual(effects);
      for (const line of panel.lines) expect(line.landMs).toBeLessThanOrEqual(FX_CHAOS_DICE_REEL_MS);
      expect(roll?.cues[1]).toMatchObject({ at: CENTRE, delayMs: frac(chaosLandAt(0, effects.length), FX_CHAOS_DICE_REEL_MS) });
    }
    // Another card's roll is not its.
    expect(signature([{ ...rolled("p1", STINKY, [STINKY_HAND]), instanceId: "other" }], D, [played("p1", STINKY)])).toEqual([{ cues: [], row: undefined }]);
  });

  it("R3407000 the Stinky Edition spits out its outcome: card backs to each seat's hand pile, a stink puff on each zone a card lands in, the Golem's own landing, and a tether in each mark's colors", () => {
    const D = 400;
    const view = fullBoardView();
    const before = [played("p1", STINKY), rolled("p1", STINKY, [STINKY_HAND])];
    const [mine, theirs] = signature([added("p1"), { type: "addedToHand", player: "p2", instanceId: HIDDEN_ID, defId: HIDDEN_ID }], D, before, view);
    expect(mine).toEqual({ cues: [{ kind: "ghost", from: CENTRE, to: tid(animTestid.hand("you")), delayMs: 0, durationMs: D }], row: "replace" });
    expect(theirs).toEqual({ cues: [{ kind: "ghost", from: CENTRE, to: tid(animTestid.hand("opponent")), delayMs: 0, durationMs: D }], row: "replace" });
    // A Field Spell into the backrow: a puff at its zone's foot. Every Quest or Wincon Field Spell lands with an
    // entrance of its own (R3395000's Heist among them), so both draw (R3407000): its entrance, then the puff.
    const [field] = signature([summoned("p1", "q1", HEIST, "backrow", 2)], D, before, view);
    expect(field?.row).toBe("replace");
    expect(field?.cues.length).toBeGreaterThan(1);
    expect(field?.cues.at(-1)).toMatchObject({ kind: "burst", at: tid(testid.zone("you", "backrow", 2), FOOT), preset: "smoke", colors: FLAVOR_LOOKS.Toxic.colors });
    // The Golem: its own entrance alone, as it lands anywhere.
    const tgt = testid.zone("you", "units", 2);
    const [golem] = signature([summoned("p1", "g1", GOLEM, "units", 2)], D, before, view);
    expect(golem?.row).toBe("replace");
    expect(kinds(golem?.cues ?? [])).toEqual(["shot", "ring", "burst", "burst", "burst", "shake"]);
    expect(golem?.cues[1]).toMatchObject({ at: tid(tgt) });
    // Frost and Allure: a bolt from the cloud to each enemy, in the mark's colors, over the brand.
    const foe = view.opponent.units[0]?.instanceId ?? "";
    for (const [mark, color] of [
      [FROZEN_MARK, FROZEN_MARK_COLOR],
      [ALLURE_MARK, ALLURE_MARK_COLOR],
    ] as const) {
      const [tether] = signature([marked(foe, mark, color)], D, before, view);
      expect(tether?.row, mark).toBe("keep");
      expect(kinds(tether?.cues ?? []), mark).toEqual(["shot"]);
      expect(tether?.cues[0], mark).toMatchObject({ from: CENTRE, to: tid(testid.card(foe)), colors: markLook(color, FLAVOR_LOOKS.Toxic).colors });
    }
    // A thaw, and a mark on a card the board does not show, draw nothing of its own.
    expect(signature([marked(foe, FROZEN_MARK, FROZEN_MARK_COLOR, false), marked("gone", FROZEN_MARK, FROZEN_MARK_COLOR)], D, before, view).map((entry) => entry.cues)).toEqual([[], []]);
  });

  it("R3407000 the Scape Edition's cast: an indigo sky over the board, stars pouring over its top rim, rays turning and the board tilting", () => {
    const D = 400;
    const [cast] = signature([played("p1", SCAPE)], D, []);
    expect(cast?.row).toBe("replace");
    expect(kinds(cast?.cues ?? [])).toEqual(["figure", "burst", "burst", "burst", "rays", "burst", "shake"]);
    expect(cast?.cues[0]).toMatchObject({ figure: "dusk", at: tid(testid.board), delayMs: 0 });
    for (const cue of cast?.cues.slice(1, 4) ?? []) expect(cue).toMatchObject({ preset: "star", at: { kind: "testid", testid: testid.board, at: { y: 0 } } });
    expect(cast?.cues[4]).toMatchObject({ tone: "radiant", at: CENTRE });
    expect(cast?.cues[5]).toMatchObject({ at: CENTRE, colors: FLAVOR_LOOKS.Tech.colors });
    const shake = cast?.cues[6];
    expect(shake?.kind === "shake" ? shake.trauma : Infinity).toBeLessThanOrEqual(SIGNATURE_SPELL_MAX_TRAUMA);
  });

  it("R3407000 the Scape Edition: drones drop in starlight, a replacement naming no Anomaly rings that side's zones, and the Anomaly's rift and the Starship draw their own set pieces alone", () => {
    const D = 500;
    const view = fullBoardView();
    const before = [played("p1", SCAPE), rolled("p1", SCAPE, [SCAPE_GALLERY_EFFECT])];
    const [drone] = signature([summoned("p1", "d1", "stinkyscape-t-drone-corvette", "units", 2)], D, before, view);
    expect(drone?.row).toBe("keep");
    expect(drone?.cues).toHaveLength(1);
    expect(drone?.cues[0]).toMatchObject({ kind: "burst", preset: "star", at: tid(testid.zone("you", "units", 2)) });
    const [replaced] = signature([{ type: "boardReplaced", player: "p1" }], D, before, view);
    expect(replaced?.row).toBe("keep");
    expect(kinds(replaced?.cues ?? [])).toEqual(LANES.map(() => "ring"));
    expect(replaced?.cues.map((cue) => (cue.kind === "ring" ? cue.at : null))).toEqual(LANES.map((lane) => tid(testid.zone("you", "units", lane))));
    // R3337003: an Anomaly's lay is its rift's set piece: inside it this draws nothing.
    const rift = planWithOffsets(call(SCAPE, "p1", [SCAPE_ANOMALY], [{ type: "boardReplaced", player: "p1", anomaly: "WH" }, summoned("p1", "w1", "stinkyscape-t-drone-corvette", "units", 1)]), view);
    const riftEntry = rift.find((planned) => planned.entry.rift !== undefined);
    if (riftEntry === undefined) throw new Error("no rift");
    const env = envOf();
    for (const planned of rift) {
      env.memory.remember(planned.entry.events);
      if (planned !== riftEntry) continue;
      for (const event of planned.entry.events) expect(planCardFx(event, { entry: planned.entry, view: planned.entry.view, env, D: planned.entry.durationMs }), event.type).toBeNull();
    }
    // The Starship's fuse and launch are the Starship's.
    const ship: GameEvent[] = [
      { type: "starshipFused", player: "p1", instanceId: "ship", pieceId: "piece", defId: "stinkyscape-t-dormant-starship", pieces: 1 },
      { type: "starshipLaunched", player: "p1", instanceId: "ship", defId: "stinkyscape-t-dormant-starship", name: "Starship", pieces: [] },
    ];
    expect(signature(ship, D, before, view)).toEqual([
      { cues: [], row: undefined },
      { cues: [], row: undefined },
    ]);
  });

  it("R3407000 each gallery play ends within its ceiling", () => {
    for (const defId of [STINKY, GOLEM, SCAPE]) expectGalleryWithinCeiling(defId);
  });

  it("R3407000 each outcome beat outside the gallery plays ends within its Call's ceiling from its own entry", () => {
    const view = fullBoardView();
    const foe = view.opponent.units[0]?.instanceId ?? "";
    const beats: readonly [string, GameEvent["type"], GameEvent[]][] = [
      [STINKY, "addedToHand", call(STINKY, "p1", [STINKY_HAND], [added("p1"), added("p1")])],
      [STINKY, "summoned", call(STINKY, "p1", [STINKY_FIELDS], [summoned("p1", "q1", HEIST, "backrow", 2)])],
      [STINKY, "summoned", call(STINKY, "p1", [STINKY_GALLERY_EFFECT], [summoned("p1", "g1", GOLEM, "units", 2)])],
      [STINKY, "marked", call(STINKY, "p1", [STINKY_FREEZE], [marked(foe, FROZEN_MARK, FROZEN_MARK_COLOR)])],
      [STINKY, "marked", call(STINKY, "p1", [STINKY_ALLURE], [marked(foe, ALLURE_MARK, ALLURE_MARK_COLOR)])],
      [SCAPE, "summoned", call(SCAPE, "p1", [SCAPE_GALLERY_EFFECT], [summoned("p1", "d1", "stinkyscape-t-drone-corvette", "units", 2)])],
      [SCAPE, "boardReplaced", call(SCAPE, "p1", [SCAPE_ANOMALY], [{ type: "boardReplaced", player: "p1" }])],
    ];
    for (const [defId, type, events] of beats) {
      const run = planWithOffsets(events, view);
      const own = run.find((planned) => planned.entry.events.some((event) => event.type === type));
      expect(own?.cues.length, `${defId} ${type} draws`).toBeGreaterThan(0);
      expect(endFromMarker(run, type), `${defId} ${type}`).toBeLessThanOrEqual(ceilingOf(defId));
    }
  });

  it("R3407000 nothing draws with the effects off", () => {
    const view = fullBoardView();
    const runs = [
      call(STINKY, "p1", [STINKY_HAND], [added("p1"), added("p1")]),
      call(STINKY, "p1", [STINKY_GALLERY_EFFECT], [summoned("p1", "g1", GOLEM, "units", 2)]),
      call(SCAPE, "p1", [SCAPE_GALLERY_EFFECT], [summoned("p1", "d1", "stinkyscape-t-drone-corvette", "units", 2)]),
    ];
    for (const events of runs) expect(planWithOffsets(events, view, envOf(0)).flatMap((planned) => planned.cues)).toEqual([]);
    expect(ENTRANCES.golemCrash({ D: 400, tgt: testid.zone("you", "units", 3), intensity: 0 }).filter((cue) => cue.kind === "shake")).toEqual([]);
  });

  it("R3407000 every cue starts inside its entry and ends within FX_MAX_TAIL_MS of it, at every D (R200)", () => {
    const view = fullBoardView();
    const foe = view.opponent.units[0]?.instanceId ?? "";
    const tgt = testid.zone("you", "units", 3);
    const plays = [
      call(STINKY, "p1", [STINKY_HAND], [added("p1"), { type: "addedToHand", player: "p2", instanceId: HIDDEN_ID, defId: HIDDEN_ID }]),
      call(STINKY, "p1", [STINKY_FIELDS], [summoned("p1", "q1", HEIST, "backrow", 2), marked(foe, FROZEN_MARK, FROZEN_MARK_COLOR), marked(foe, ALLURE_MARK, ALLURE_MARK_COLOR)]),
      call(STINKY, "p1", [STINKY_GALLERY_EFFECT], [summoned("p1", "g1", GOLEM, "units", 2)]),
      call(SCAPE, "p1", [SCAPE_GALLERY_EFFECT], [summoned("p1", "d1", "stinkyscape-t-drone-corvette", "units", 2), { type: "boardReplaced", player: "p1" }]),
    ];
    for (const D of DS) {
      for (const events of plays) {
        const run = entries(events, D, view);
        expect(run.flatMap((planned) => planned.cues).length, `D ${String(D)}`).toBeGreaterThan(0);
        expectR200(run, `D ${String(D)}`);
      }
      for (const wait of [0, D / 2, D]) {
        const cues = ENTRANCES.golemCrash({ D, tgt, intensity: FX_INTENSITY_SCALE.normal, landMs: landMsAfter(wait, D) });
        for (const cue of cues) {
          expect(cue.delayMs, `golemCrash D ${String(D)} wait ${String(wait)} ${cue.kind}`).toBeGreaterThanOrEqual(0);
          expect(cue.delayMs, `golemCrash D ${String(D)} wait ${String(wait)} ${cue.kind}`).toBeLessThanOrEqual(D);
          expect(endOf(cue), `golemCrash D ${String(D)} wait ${String(wait)} ${cue.kind}`).toBeLessThanOrEqual(D + FX_MAX_TAIL_MS);
        }
      }
    }
  });

  it("R3407000 from the caster's opponent's seat each Call's cards still fly as backs, and no cue names a card or reaches into a hand (R202)", () => {
    const view = fullBoardView();
    const hidden: GameEvent = { type: "addedToHand", player: "p2", instanceId: HIDDEN_ID, defId: HIDDEN_ID };
    for (const [defId, effect] of [
      [STINKY, STINKY_HAND],
      [SCAPE, SCAPE_PIECES],
    ] as const) {
      // Cast by p2 and watched by p1: its cards are "hidden", and no prompt reaches this seat.
      const run = planWithOffsets(call(defId, "p2", [effect], [hidden, hidden]), view);
      const flown = run.filter((planned) => planned.entry.events.some((event) => event.type === "addedToHand"));
      expect(flown.length, defId).toBeGreaterThan(0);
      for (const planned of flown) expect(planned.cues.length, defId).toBeGreaterThan(0);
      expectNoCard(
        run.flatMap((planned) => planned.cues),
        defId,
      );
    }
    // The Stinky Edition's are card backs to the caster's hand pile.
    const stinky = planWithOffsets(call(STINKY, "p2", [STINKY_HAND], [hidden]), view);
    const back = stinky.find((planned) => planned.entry.events.some((event) => event.type === "addedToHand"));
    expect(back?.cues).toEqual([{ kind: "ghost", from: CENTRE, to: tid(animTestid.hand("opponent")), delayMs: 0, durationMs: back?.entry.durationMs }]);
  });

  it("R3407000 each Call's gallery fixture is its roll", () => {
    const view = fullBoardView();
    expect(stinkCloudFixture(view, ID, "p1")).toEqual([rolled("p1", STINKY, [STINKY_GALLERY_EFFECT])]);
    expect(tabletopTiltFixture(view, ID, "p1")).toEqual([rolled("p1", SCAPE, [SCAPE_GALLERY_EFFECT])]);
  });
});
