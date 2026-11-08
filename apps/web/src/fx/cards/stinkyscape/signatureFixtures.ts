// R3395000: the /dev/fx gallery's fixture for each of Stinkyscape 001 to 090's signature recipes
// (motifs/gallery.ts `RECIPE_FIXTURES`), as `player` would see the play of `instanceId` resolve on `view`:
// what its text does, a hook's under its own marker. Tylannt completes the Quest standing in its side's
// first backrow zone and adds its reward; The Greatest Heist Discovers a card, then its quest progresses and
// completes; Rod of Ages gains a keyword at its End of turn and turns Radiant; ZolarKeth's Cry summons a
// University; Drone Core VIII stamps three Interceptors and Drone Core IX launches three Corvettes; Kwzka
// Allures the first enemy Unit; The Narcor Project's Activate feeds a Narcor back and lowers its Brittle;
// Great Paperclip Theory adds Drone Core I and sets its rule; Beam v5 hits the middle enemy Unit and Cleaves
// its neighbours, Repeater v5 and Cannon v5 hit it, Flak v5's first wave hits every enemy Unit and
// Miner v5 gains a Narcor; Z'Zoth raises two Units from its graveyard; A'Zoth calls one back and replays its
// Cry on the first enemy Unit; Galaxy and Prototype-X1 Equip two picks, Prototype-X2 three; Siva 1 Allures the
// first enemy Unit; Siva 2's Radiant Cry adds three Speciesists.

import type { GameEvent, PlayerId, PlayerView } from "@jackioh/shared";

import { ALLURE_MARK, ALLURE_MARK_COLOR } from "../../../cards/marks.ts";

type Fixture = (view: PlayerView, instanceId: string, player: PlayerId) => GameEvent[];
type Unit = NonNullable<PlayerView["you"]["units"][number]>;

const UNIVERSITY = "meditative-097-6";
const INTERCEPTOR = "stinkyscape-t-drone-interceptor";
const CORVETTE = "stinkyscape-t-drone-corvette";
const NARCOR = "stinkyscape-t-narcor";
const DRONE_CORE_I = "stinkyscape-027";
const SPECIESIST = "core-002";
const MINERS = ["stinkyscape-062", "stinkyscape-063"] as const;
const WEAPONS = ["stinkyscape-058", "stinkyscape-059", "stinkyscape-060"] as const;

/** The lanes the gallery's Interceptors and Corvettes land in, and the University's. */
const INTERCEPTOR_LANES = [3, 4, 5] as const;
const CORVETTE_LANES = [2, 3, 4] as const;
const CAMPUS_LANE = 4;

/** The damage each weapon's gallery hit deals (its card's base `damage`). */
const HITS = { beam: 11, repeater: 3, cannon: 9, flak: 2 } as const;

/** The Brittle Narcor Project's Activate lowers to, from its printed 4. */
const BRITTLE_AFTER = 3;

const mine = (view: PlayerView, player: PlayerId): PlayerView["you"] => (player === view.viewer ? view.you : view.opponent);
const theirs = (view: PlayerView, player: PlayerId): PlayerView["you"] => (player === view.viewer ? view.opponent : view.you);
const unitsOf = (side: PlayerView["you"]): Unit[] => side.units.filter((unit): unit is Unit => unit !== null);
const marker = (instanceId: string, defId: string, player: PlayerId, hook: "whenever" | "activate" | "endOfTurn"): GameEvent => ({ type: "hookResolving", sourceId: instanceId, defId, controller: player, hook });
const allure = (unit: Unit | undefined): GameEvent[] => (unit === undefined ? [] : [{ type: "marked", instanceId: unit.instanceId, mark: ALLURE_MARK, color: ALLURE_MARK_COLOR, added: true }]);
const hit = (instanceId: string, unit: Unit | undefined, amount: number): GameEvent[] => (unit === undefined ? [] : [{ type: "damage", sourceId: instanceId, targetId: unit.instanceId, amount, combat: false }]);
const fusedPicks = (instanceId: string, defId: string, picks: readonly string[]): GameEvent[] =>
  picks.map((_pick, k) => ({ type: "fused", instanceIds: [instanceId, `fx-pick-${String(k + 1)}`], resultInstanceId: instanceId, defId }));

/** `view` with the viewer's `row` zones of `lanes` emptied, so the gallery's summons land in open zones (§3.2). */
function clearing(row: "units" | "backrow", lanes: readonly number[]): (view: PlayerView) => PlayerView {
  return (view) => {
    const cleared = <T>(slots: readonly (T | null)[]): (T | null)[] => slots.map((slot, k) => (lanes.includes(k + 1) ? null : slot));
    return { ...view, you: row === "units" ? { ...view.you, units: cleared(view.you.units) } : { ...view.you, backrow: cleared(view.you.backrow) } };
  };
}

/** The lanes Z'Zoth raises into and A'Zoth calls into. */
const RISEN_LANES = [2, 4] as const;
const CALLED_LANE = 2;
/** The fallen the gallery raises: Units its graveyard holds. */
const RISEN = ["core-003", "core-005"] as const;

/**
 * R3395000: the boards the summoning signatures play over, the zones their fixtures fill left open, as a
 * summon needs (§3.2): the MAGI's three backrow zones, the Interceptors', the Corvettes', the University's,
 * the risen and the called Units' (motifs/gallery.ts `RECIPE_BOARDS`).
 */
export const STINKYSCAPE_SIGNATURE_BOARDS = {
  magiBoot: clearing("backrow", [3, 4, 5]),
  droneConveyor: clearing("units", INTERCEPTOR_LANES),
  seedPod: clearing("units", CORVETTE_LANES),
  campusLaunch: clearing("units", [CAMPUS_LANE]),
  ghostHulls: clearing("units", RISEN_LANES),
  echoRings: clearing("units", [CALLED_LANE]),
} as const;

export const STINKYSCAPE_SIGNATURE_FIXTURES = {
  vaultHeist: ((view, _instanceId, player) => {
    const quest = mine(view, player).backrow.find((slot) => slot !== null && !slot.faceDown);
    const questId = quest !== null && quest !== undefined && !quest.faceDown ? quest.instanceId : "fx-quest";
    return [
      { type: "questCompleted", player, instanceId: questId, quest: "Initiation" },
      { type: "addedToHand", player, instanceId: "fx-loot", defId: NARCOR },
    ];
  }) satisfies Fixture,
  heistLocks: ((_view, instanceId, player) => [
    { type: "addedToHand", player, instanceId: "fx-heist-pick", defId: "stinkyscape-087" },
    marker(instanceId, "stinkyscape-007", player, "whenever"),
    { type: "questProgressed", player, instanceId, quest: "Comradery", progress: 1, goal: 2 },
    { type: "questCompleted", player, instanceId, quest: "Comradery" },
  ]) satisfies Fixture,
  rodBands: ((_view, instanceId, player) => [
    marker(instanceId, "stinkyscape-016", player, "endOfTurn"),
    { type: "keywordGranted", instanceId, keyword: { kind: "Taunt" } },
    { type: "radiantSet", instanceId, defId: "stinkyscape-016", zone: { z: "field", player, row: "backrow", lane: 1 } },
  ]) satisfies Fixture,
  campusLaunch: ((_view, _instanceId, player) => [{ type: "summoned", player, instanceId: "fx-campus", defId: UNIVERSITY, row: "units", lane: CAMPUS_LANE }]) satisfies Fixture,
  droneConveyor: ((_view, _instanceId, player) =>
    INTERCEPTOR_LANES.map((lane, k): GameEvent => ({ type: "summoned", player, instanceId: `fx-interceptor-${String(k + 1)}`, defId: INTERCEPTOR, row: "units", lane }))) satisfies Fixture,
  seedPod: ((_view, _instanceId, player) =>
    CORVETTE_LANES.map((lane, k): GameEvent => ({ type: "summoned", player, instanceId: `fx-corvette-${String(k + 1)}`, defId: CORVETTE, row: "units", lane }))) satisfies Fixture,
  allureRibbon: ((view, instanceId, player) => [marker(instanceId, "stinkyscape-037", player, "whenever"), ...allure(unitsOf(theirs(view, player))[0])]) satisfies Fixture,
  narcorCases: ((_view, instanceId, player) => [
    marker(instanceId, "stinkyscape-039", player, "activate"),
    { type: "discarded", instanceId: "fx-narcor", defId: NARCOR, owner: player },
    { type: "counterChanged", instanceId, counter: "brittle", value: BRITTLE_AFTER },
  ]) satisfies Fixture,
  paperclipLink: ((_view, _instanceId, player) => [
    { type: "addedToHand", player, instanceId: "fx-core-i", defId: DRONE_CORE_I },
    { type: "modifierChanged", player, modifierId: "fx-paperclip", added: true },
  ]) satisfies Fixture,
  beamCharge: ((view, instanceId, player) => {
    const foes = unitsOf(theirs(view, player));
    return [marker(instanceId, "stinkyscape-058", player, "activate"), ...hit(instanceId, foes[2], HITS.beam), ...hit(instanceId, foes[1], HITS.beam), ...hit(instanceId, foes[3], HITS.beam)];
  }) satisfies Fixture,
  repeaterBurst: ((view, instanceId, player) => [marker(instanceId, "stinkyscape-059", player, "activate"), ...hit(instanceId, unitsOf(theirs(view, player))[2], HITS.repeater)]) satisfies Fixture,
  cannonShell: ((view, instanceId, player) => [marker(instanceId, "stinkyscape-060", player, "activate"), ...hit(instanceId, unitsOf(theirs(view, player))[2], HITS.cannon)]) satisfies Fixture,
  flakWaves: ((view, instanceId, player) => {
    // One wave, as the engine sends it: a hit on every enemy Unit, which the planner draws as one sweep.
    return [marker(instanceId, "stinkyscape-061", player, "activate"), ...unitsOf(theirs(view, player)).flatMap((foe) => hit(instanceId, foe, HITS.flak))];
  }) satisfies Fixture,
  asteroidCrunch: ((_view, instanceId, player) => [marker(instanceId, "stinkyscape-066", player, "activate"), { type: "addedToHand", player, instanceId: "fx-resource", defId: NARCOR }]) satisfies Fixture,
  ghostHulls: ((_view, _instanceId, player) =>
    RISEN_LANES.map((lane, k): GameEvent => ({ type: "summoned", player, instanceId: `fx-risen-${String(k + 1)}`, defId: RISEN[k] ?? "core-003", row: "units", lane }))) satisfies Fixture,
  echoRings: ((view, _instanceId, player) => [
    { type: "summoned", player, instanceId: "fx-called", defId: "core-005", row: "units", lane: CALLED_LANE },
    { type: "hookResolving", sourceId: "fx-called", defId: "core-005", controller: player, hook: "cry" },
    // Its Cry replayed with a random target: the first enemy Unit.
    ...hit("fx-called", unitsOf(theirs(view, player))[0], 1),
  ]) satisfies Fixture,
  bellyBay: ((_view, instanceId) => fusedPicks(instanceId, "stinkyscape-080", MINERS)) satisfies Fixture,
  wingMount: ((_view, instanceId) => fusedPicks(instanceId, "stinkyscape-085", WEAPONS.slice(0, 2))) satisfies Fixture,
  spineWeld: ((_view, instanceId) => fusedPicks(instanceId, "stinkyscape-086", WEAPONS)) satisfies Fixture,
  glowLeash: ((view, _instanceId, player) => allure(unitsOf(theirs(view, player))[0])) satisfies Fixture,
  venomHand: ((_view, _instanceId, player) =>
    [1, 2, 3].map((n): GameEvent => ({ type: "addedToHand", player, instanceId: `fx-speciesist-${String(n)}`, defId: SPECIESIST }))) satisfies Fixture,
} as const;
