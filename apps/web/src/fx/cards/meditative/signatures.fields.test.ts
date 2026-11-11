// R3405001: Meditative's nine Field Spell signatures, card by card (Introspection, Double Header, Alternate
// Fate, Feng Shui, Skull of J'Nari, Pareto Optimality, The Cane, Ever Growing Tree, Aluneth): each entrance's
// key figures, its one shake at the slam with its own trauma, a Mythic's nothing before its takeover's hold;
// then each beat of its text, planned from the events its script emits under its play or its hook's marker as
// the board's effects layer plans them (remember, then plan): the figures, ghosts and bursts it draws, every
// card flying to its pile as a card back, an owner-only sheen on its owner's seat alone, and an event its
// recipe does not claim left to its own row.

import type { GameEvent, PlayerId, PlayerView } from "@jackioh/shared";
import { describe, expect, it } from "vitest";

import { animTestid, planEntries } from "../../../game/animations.ts";
import { sideOf, testid } from "../../../game/contract.ts";
import { faceUpBackrow, fullBoardView } from "../../../test/fixtures.ts";
import { HOLD, added, beats, envOf, landOf, run, tid } from "../../../test/signatures.ts";
import { frac } from "../../build.ts";
import { planCardFx } from "../../cardFx.ts";
import { planFx } from "../../cues.ts";
import { FX_INTENSITY_SCALE, FX_SLAM_AT } from "../../constants.ts";
import { ENTRANCES } from "../../entrances.ts";
import { galleryEntry } from "../../motifs/gallery.ts";
import type { FxCue } from "../../types.ts";
import { ENTRANCE_TRAUMA, FOOT, type EntranceKey } from "../recipe.ts";

const INTROSPECTION = "meditative-007";
const DOUBLE_HEADER = "meditative-011";
const ALTERNATE_FATE = "meditative-026";
const FENG_SHUI = "meditative-040";
const SKULL = "meditative-063";
const PARETO = "meditative-071";
const CANE = "meditative-071-1";
const TREE = "meditative-075";
const ALUNETH = "meditative-080";

/** A Legendary's entry, its slam at FX_SLAM_AT of it; a Mythic's, its takeover's hold and as long again after it. */
const D = 400;
const MYTHIC_D = HOLD + D;
const SLAM = frac(FX_SLAM_AT, D);

/** Where a Field Spell lands: its player's first backrow zone, as "ex". */
const LANE = 1;
const TGT = testid.zone("you", "backrow", LANE);
const OWN = tid(testid.card("ex"));

type Landing = Extract<GameEvent, { type: "summoned" }>;
type Hook = Extract<GameEvent, { type: "hookResolving" }>["hook"];

/** The marker a hook of the card "ex" opens its entry with. */
const marker = (defId: string, controller: PlayerId, hook: Hook): GameEvent => ({ type: "hookResolving", sourceId: "ex", defId, controller, hook });

/**
 * `key`'s entrance as `defId` lands face up in p1's first backrow zone, its motion after `lead`: no cue
 * starts before it, its one shake lands at the slam with its own trauma, and the dispatch plays it on the
 * card's own landing held as long.
 */
function entrance(key: EntranceKey, defId: string, entryMs: number, lead = 0): FxCue[] {
  const view = fullBoardView();
  const landing: Landing = { type: "summoned", player: "p1", instanceId: "ex", defId, row: "backrow", lane: LANE };
  const cues = ENTRANCES[key]({ D: entryMs, tgt: TGT, intensity: FX_INTENSITY_SCALE.normal, view, landing, leadMs: lead });
  const slam = lead + frac(FX_SLAM_AT, entryMs - lead);
  expect(cues.filter((cue) => cue.kind === "shake"), key).toEqual([{ kind: "shake", trauma: ENTRANCE_TRAUMA[key], delayMs: slam }]);
  expect(Math.min(...cues.map((cue) => cue.delayMs)), key).toBeGreaterThanOrEqual(lead);
  expect(landOf(landing, entryMs, lead, view)).toEqual({ cues, row: "replace" });
  return cues;
}

/** The board with `defId` face up as "ex" in `player`'s first backrow zone, where its beats stand. */
function fielded(defId: string, player: PlayerId = "p1"): PlayerView {
  const view = fullBoardView();
  const side = sideOf(view, player);
  const seat = view[side];
  const backrow = [faceUpBackrow(player, { instanceId: "ex", defId }), ...seat.backrow.slice(1)];
  return side === "you" ? { ...view, you: { ...seat, backrow } } : { ...view, opponent: { ...seat, backrow } };
}

const firstUnit = (view: PlayerView, side: "you" | "opponent"): string => view[side].units.find((unit) => unit !== null)?.instanceId ?? "";

describe("R3405001 Meditative's Field Spell signatures", () => {
  it("R3405001 Introspection: a still pool and its breath land; its start of turn mends, draws a card back, dims the gems for its owner alone and darts each enemy", () => {
    const cues = entrance("poolBreath", INTROSPECTION, D);
    expect(beats(cues)).toEqual(["figure:stillPool@0", "burst@0", `figure:breathRing@${String(SLAM)}`, `ring@${String(SLAM)}`, `burst@${String(SLAM)}`, `shake@${String(SLAM)}`]);
    expect(cues[0]).toMatchObject({ at: tid(TGT), side: "you" });
    const view = fielded(INTROSPECTION);
    const mine = firstUnit(view, "you");
    const foe = firstUnit(view, "opponent");
    const [heroMend, unitMend, draw, gem, gem2, dart, unclaimed] = run(
      INTROSPECTION,
      [
        { type: "healed", targetId: "hero-p1", amount: 2 },
        { type: "healed", targetId: mine, amount: 2 },
        { type: "drawn", player: "p1", instanceId: "fx-draw", defId: "core-002" },
        { type: "costChanged", instanceId: "fx-draw", cost: 0 },
        { type: "costChanged", instanceId: "fx-gem", cost: 1 },
        { type: "damage", sourceId: "ex", targetId: foe, amount: 1, combat: false },
        { type: "promptOpened", player: "p1" },
      ],
      D,
      "p1",
      view,
      "startOfTurn",
    );
    // The breath draws in on its card at the first mend alone; each mend glows with its number where it lands.
    expect(heroMend?.row).toBe("replace");
    expect(heroMend?.cues[0]).toMatchObject({ kind: "figure", figure: "breathRing", at: OWN, size: "small", side: "you" });
    expect(heroMend?.cues.slice(1)).toMatchObject([{ kind: "burst", at: tid(testid.hero("you")) }, { kind: "splat", tone: "heal", amount: 2 }]);
    expect(unitMend?.cues.map((cue) => cue.kind)).toEqual(["burst", "splat"]);
    expect(unitMend?.cues[0]).toMatchObject({ at: tid(testid.card(mine)) });
    expect(draw).toEqual({ cues: [{ kind: "ghost", from: tid(animTestid.library("you")), to: tid(animTestid.hand("you")), delayMs: 0, durationMs: D }], row: "replace" });
    // The gems dim once for the whole hand, on its owner's seat.
    expect(gem).toEqual({ cues: [{ kind: "sheen", at: tid(animTestid.hand("you")), delayMs: 0, durationMs: D }], row: "replace" });
    expect(gem2).toEqual({ cues: [], row: "replace" });
    expect(dart?.cues.map((cue) => cue.kind)).toEqual(["projectile", "burst", "splat"]);
    expect(dart?.cues[0]).toMatchObject({ from: OWN, to: tid(testid.card(foe)) });
    expect(unclaimed).toEqual({ cues: [], row: undefined });
    // The other seat's Introspection: its gems dim unseen here.
    const [theirs] = run(INTROSPECTION, [{ type: "costChanged", instanceId: "hidden", cost: 0 }], D, "p2", fielded(INTROSPECTION, "p2"), "startOfTurn");
    expect(theirs).toEqual({ cues: [], row: "replace" });
  });

  it("R3405000 Introspection's start of turn as the board plans it: its mends and thought-darts draw in the sweeps, out of the fog", () => {
    const view = fielded(INTROSPECTION);
    const ids = (side: "you" | "opponent"): string[] => view[side].units.flatMap((unit) => (unit === null ? [] : [unit.instanceId]));
    const mine = ids("you");
    const foes = ids("opponent");
    expect(mine.length).toBeGreaterThan(1);
    expect(foes.length).toBeGreaterThan(1);
    const heals: GameEvent[] = ["hero-p1", ...mine].map((targetId) => ({ type: "healed", targetId, amount: 1 }));
    const hits: GameEvent[] = [...foes, "hero-p2"].map((targetId) => ({ type: "damage", sourceId: "ex", targetId, amount: 1, combat: false }));
    const events: GameEvent[] = [marker(INTROSPECTION, "p1", "startOfTurn"), ...heals, { type: "drawn", player: "p1", instanceId: "fx-draw", defId: "core-002" }, ...hits];
    const entries = planEntries(events, view, false);
    // The board makes each run a sweep (game/runs.ts `sweepAt`), as a game does.
    const sweeps = entries.filter((entry) => entry.sweep !== undefined);
    expect(sweeps.map((entry) => entry.sweep?.tone)).toEqual(["heal", "damage"]);
    const env = envOf();
    const planned = entries.map((entry) => {
      env.memory.remember(entry.events);
      return { entry, cues: planFx(entry, entry.view, env) };
    });
    const [mend, dart] = planned.filter(({ entry }) => entry.sweep !== undefined).map(({ cues }) => cues);
    // The breath draws in once, a glow and a number on each it mends, and no fog.
    expect(mend?.filter((cue) => cue.kind === "figure")).toMatchObject([{ figure: "breathRing", at: OWN }]);
    expect(mend?.filter((cue) => cue.kind === "splat").map((cue) => cue.at)).toEqual([tid(testid.hero("you")), ...mine.map((id) => tid(testid.card(id)))]);
    expect(mend?.some((cue) => cue.kind === "fog")).toBe(false);
    // A dart flies from its card to each enemy it hits, the hero last.
    expect(dart?.filter((cue) => cue.kind === "projectile")).toMatchObject([...foes.map((id) => ({ from: OWN, to: tid(testid.card(id)) })), { from: OWN, to: tid(testid.hero("opponent")) }]);
    expect(dart?.some((cue) => cue.kind === "fog")).toBe(false);
    // A sweep no signature resolves still rolls its fog.
    const plain = planEntries(hits, view, false).find((entry) => entry.sweep !== undefined);
    expect(plain === undefined ? [] : planFx(plain, view, envOf()).filter((cue) => cue.kind === "fog")).toHaveLength(1);
  });

  it("R3405001 Double Header: floodlights then a torn stub land; the copy peels off the turn's first card as a ticket half, a card back to its owner's hand, its face theirs alone", () => {
    const cues = entrance("floodlights", DOUBLE_HEADER, D);
    expect(beats(cues)).toEqual(["figure:floodlights@0", `figure:ticketStub@${String(SLAM)}`, `ring@${String(SLAM)}`, `burst@${String(SLAM)}`, `shake@${String(SLAM)}`]);
    const hand = tid(animTestid.hand("you"));
    // A Spell's first card is gone from the board: the copy peels off its own zone.
    const [copy, unclaimed] = run(DOUBLE_HEADER, [added("p1", "fx-copy", "core-055"), added("p2", "hidden", "hidden")], D, "p1", fielded(DOUBLE_HEADER), "whenever");
    expect(copy?.row).toBe("replace");
    expect(copy?.cues).toMatchObject([
      { kind: "burst", preset: "arcane", at: OWN },
      { kind: "figure", figure: "ticketStub", at: OWN, size: "small" },
      { kind: "ghost", from: OWN, to: hand },
      { kind: "sheen", at: hand },
    ]);
    // The half folds and flies, its sheen with it, landing as the entry ends.
    const fold = copy?.cues[2]?.delayMs ?? 0;
    expect(fold).toBeGreaterThan(0);
    expect(copy?.cues[2]).toMatchObject({ durationMs: D - fold });
    expect(copy?.cues[3]).toMatchObject({ delayMs: fold, durationMs: D - fold });
    expect(unclaimed).toEqual({ cues: [], row: undefined });
    // A Unit the board still shows was the first card: the after-image peels off it (its `cardResolved` before the trigger's marker).
    const view = fielded(DOUBLE_HEADER);
    const first = firstUnit(view, "you");
    const env = envOf();
    const later = added("p1", "fx-copy", "core-004");
    env.memory.remember([{ type: "cardResolved", player: "p1", instanceId: first, defId: "core-004", permanent: true, costPaid: 1 }, marker(DOUBLE_HEADER, "p1", "whenever"), later]);
    const peeled = planCardFx(later, { entry: { ...galleryEntry([later], view), durationMs: D }, view, env, D });
    expect(peeled?.cues[0]).toMatchObject({ kind: "burst", at: tid(testid.card(first)) });
    expect(peeled?.cues[2]).toMatchObject({ kind: "ghost", from: tid(testid.card(first)), to: hand });
    // The other seat's copy flies as a bare card back to its hand pile, no sheen here.
    const [theirs] = run(DOUBLE_HEADER, [added("p2", "hidden", "hidden")], D, "p2", fielded(DOUBLE_HEADER, "p2"), "whenever");
    expect(theirs?.cues.map((cue) => cue.kind)).toEqual(["burst", "figure", "ghost"]);
    expect(theirs?.cues[2]).toMatchObject({ to: tid(animTestid.hand("opponent")) });
  });

  it("R3405001 Alternate Fate: both sides' crystals crack dry under cracked earth; at its end of turn a stone well shoves up out of each backrow zone it fills, on either side", () => {
    const cues = entrance("crackedEarth", ALTERNATE_FATE, D);
    expect(beats(cues)).toEqual(["figure:crackedEarth@0", "fracture@48", "fracture@84", `ring@${String(SLAM)}`, `burst@${String(SLAM)}`, `shake@${String(SLAM)}`]);
    expect(cues.slice(1, 3)).toMatchObject([{ at: tid(animTestid.mana("you")) }, { at: tid(animTestid.mana("opponent")) }]);
    const well = (player: PlayerId, lane: number): Landing => ({ type: "summoned", player, instanceId: `fx-well-${String(lane)}`, defId: "core-006", row: "backrow", lane });
    const [mine, theirs, unit] = run(ALTERNATE_FATE, [well("p1", 3), well("p2", 2), { ...well("p1", 2), row: "units" }], D, "p1", fielded(ALTERNATE_FATE), "endOfTurn");
    for (const [result, side, lane] of [
      [mine, "you", 3],
      [theirs, "opponent", 2],
    ] as const) {
      const zone = testid.zone(side, "backrow", lane);
      expect(result?.row).toBe("replace");
      expect(result?.cues).toMatchObject([
        { kind: "burst", preset: "dust", at: tid(zone, FOOT), delayMs: 0 },
        { kind: "figure", figure: "stoneWell", at: tid(zone), side, delayMs: 0 },
        { kind: "burst", at: tid(zone), delayMs: 0 },
      ]);
    }
    // A Unit summoned is no well.
    expect(unit).toEqual({ cues: [], row: undefined });
  });

  it("R3405001 Feng Shui: a five-colour compass aligns; its Activate blossoms and rolls the Rock to its owner's hand as a card back, its face theirs alone; its judging is the board's", () => {
    const cues = entrance("fiveCompass", FENG_SHUI, D);
    expect(beats(cues)).toEqual(["figure:baguaCompass@0", `ring@${String(SLAM)}`, `burst@${String(SLAM)}`, `shake@${String(SLAM)}`]);
    const view = fielded(FENG_SHUI);
    const judged: GameEvent = { type: "fengShui", instanceId: firstUnit(view, "you"), sourceId: "ex", player: "p1", outcome: "positive" };
    const [rock, judging] = run(FENG_SHUI, [added("p1", "fx-rock", "meditative-039-1"), judged], D, "p1", view, "activate");
    const hand = tid(animTestid.hand("you"));
    expect(rock?.row).toBe("replace");
    expect(rock?.cues).toMatchObject([
      { kind: "figure", figure: "fengBlossom", at: OWN, side: "you", delayMs: 0 },
      { kind: "burst", at: OWN, delayMs: 0 },
      { kind: "ghost", from: OWN, to: hand, spin: true },
      { kind: "sheen", at: hand },
    ]);
    const roll = rock?.cues[2]?.delayMs ?? 0;
    expect(roll).toBeGreaterThan(0);
    expect(rock?.cues[3]).toMatchObject({ delayMs: roll, durationMs: D - roll });
    expect(judging).toEqual({ cues: [], row: undefined });
    // The other seat's Rock: a bare tumbling card back, no sheen here.
    const [theirs] = run(FENG_SHUI, [added("p2", "hidden", "hidden")], D, "p2", fielded(FENG_SHUI, "p2"), "activate");
    expect(theirs?.cues.map((cue) => cue.kind)).toEqual(["figure", "burst", "ghost"]);
    expect(theirs?.cues[2]).toMatchObject({ to: tid(animTestid.hand("opponent")), spin: true });
  });

  it("R3405001 Skull of J'Nari: a horned skull thuds and its eye-fire kindles; at its end of turn it shrieks and its fire spills into the zone a Unit rises in from its owner's hand", () => {
    const cues = entrance("skullThud", SKULL, D);
    expect(beats(cues)).toEqual(["figure:hornedSkull@0", `burst@${String(SLAM)}`, `figure:eyeFire@${String(SLAM)}`, `ring@${String(SLAM)}`, `burst@${String(SLAM)}`, `shake@${String(SLAM)}`]);
    expect(cues[1]).toMatchObject({ preset: "dust", at: tid(TGT, FOOT) });
    const raised = (player: PlayerId, row: "units" | "backrow", lane: number): Landing => ({ type: "summoned", player, instanceId: "fx-raised", defId: "core-008", row, lane });
    const [rise, enemy, backrow] = run(SKULL, [raised("p1", "units", 1), raised("p2", "units", 2), raised("p1", "backrow", 2)], D, "p1", fielded(SKULL), "endOfTurn");
    const zone = testid.zone("you", "units", 1);
    expect(rise?.row).toBe("replace");
    expect(rise?.cues).toMatchObject([
      { kind: "ring", at: OWN, delayMs: 0 },
      { kind: "projectile", from: OWN, to: tid(zone), delayMs: 0 },
      { kind: "figure", figure: "eyeFire", at: tid(zone) },
      { kind: "burst", at: tid(zone, FOOT), spread: "ring" },
    ]);
    // The fire burns and the Unit rises as the spill lands.
    const spill = rise?.cues[1]?.kind === "projectile" ? rise.cues[1].flightMs : -1;
    expect(rise?.cues.slice(2).map((cue) => cue.delayMs)).toEqual([spill, spill]);
    expect([enemy, backrow]).toEqual([
      { cues: [], row: undefined },
      { cues: [], row: undefined },
    ]);
    // The other seat's Skull raises into its own side.
    const [theirs] = run(SKULL, [raised("p2", "units", 2)], D, "p2", fielded(SKULL, "p2"), "endOfTurn");
    expect(theirs?.cues[1]).toMatchObject({ from: OWN, to: tid(testid.zone("opponent", "units", 2)) });
  });

  it("R3405001 Pareto Optimality: after its hold a white curve settles down its lane; its trigger's one tell is its Cane crackling up and whipping a white streak down the lane", () => {
    const cues = entrance("studyCurve", PARETO, MYTHIC_D, HOLD);
    const slam = HOLD + SLAM;
    expect(beats(cues)).toEqual([`figure:paretoCurve@${String(HOLD)}`, `burst@${String(HOLD)}`, `ring@${String(slam)}`, `burst@${String(slam)}`, `burst@${String(slam)}`, `shake@${String(slam)}`]);
    expect(cues[4]).toMatchObject({ preset: "sparkle", at: tid(testid.zone("you", "units", LANE)) });
    const animated = (player: PlayerId, lane: number): GameEvent => ({ type: "animated", player, instanceId: "fx-cane", defId: CANE, backrowLane: lane, unitLane: lane });
    const strike: GameEvent = { type: "damage", sourceId: "fx-cane", targetId: firstUnit(fullBoardView(), "opponent"), amount: 3, combat: true };
    const [crackle, hit] = run(PARETO, [animated("p1", 2), strike], D, "p1", fielded(PARETO), "whenever");
    const stand = tid(testid.zone("you", "units", 2));
    expect(crackle?.row).toBe("replace");
    expect(crackle?.cues).toMatchObject([
      { kind: "burst", preset: "spark", at: tid(testid.zone("you", "backrow", 2)), delayMs: 0 },
      { kind: "figure", figure: "caneStreak", at: stand, side: "you", delayMs: 0 },
      { kind: "shot", shot: { motion: "bolt" }, from: stand, to: tid(testid.zone("opponent", "units", 2)) },
    ]);
    const bolt = crackle?.cues[2];
    if (bolt?.kind !== "shot") throw new Error("no streak");
    expect(bolt.delayMs).toBeGreaterThan(0);
    expect(bolt.delayMs + bolt.flightMs).toBeLessThan(D);
    // The strike's hit is its own row's (after the forced attack ends the trigger's entry).
    expect(hit).toEqual({ cues: [], row: undefined });
    // The Cane its Cry summons drops in with The Cane's own entrance.
    const [drop] = run(PARETO, [{ type: "summoned", player: "p1", instanceId: "fx-cane", defId: CANE, row: "backrow", lane: 2 }], D);
    expect(drop?.cues[0]).toMatchObject({ kind: "figure", figure: "bambooCane" });
    // The other seat's Cane whips toward this side.
    const [theirs] = run(PARETO, [animated("p2", 4)], D, "p2", fielded(PARETO, "p2"), "whenever");
    expect(theirs?.cues[1]).toMatchObject({ side: "opponent", at: tid(testid.zone("opponent", "units", 4)) });
    expect(theirs?.cues[2]).toMatchObject({ to: tid(testid.zone("you", "units", 4)) });
  });

  it("R3405001 The Cane: after its hold a red-corded cane floats up crackling white and settles in a white ring; its own moves are the event rows'", () => {
    const cues = entrance("caneRise", CANE, MYTHIC_D, HOLD);
    const slam = HOLD + SLAM;
    const crackle = HOLD + frac(0.3, D);
    expect(beats(cues)).toEqual([`figure:bambooCane@${String(HOLD)}`, `burst@${String(HOLD)}`, `burst@${String(crackle)}`, `ring@${String(slam)}`, `burst@${String(slam)}`, `shake@${String(slam)}`]);
    expect(cues[0]).toMatchObject({ at: tid(TGT), side: "you" });
    expect(cues.slice(1, 3)).toMatchObject([
      { preset: "spark", at: tid(TGT, FOOT) },
      { preset: "spark", spread: "point" },
    ]);
    // No recipe: its Animated on its owner's turn and its Pierce strike play their own rows.
    const events: GameEvent[] = [
      { type: "animated", player: "p1", instanceId: "ex", defId: CANE, backrowLane: LANE, unitLane: LANE },
      { type: "damage", sourceId: "ex", targetId: firstUnit(fullBoardView(), "opponent"), amount: 3, combat: true },
    ];
    expect(run(CANE, events, D, "p1", fielded(CANE))).toEqual([
      { cues: [], row: undefined },
      { cues: [], row: undefined },
    ]);
  });

  it("R3405001 Ever Growing Tree: a sapling corkscrews up and grips its zone; its Activate glows, its Discover shows its owner alone a sheen, and the field chosen fuses in as a branch", () => {
    const cues = entrance("treeSprout", TREE, D);
    expect(beats(cues)).toEqual(["figure:sapling@0", `burst@${String(frac(0.3, SLAM))}`, `ring@${String(SLAM)}`, `burst@${String(SLAM)}`, `shake@${String(SLAM)}`]);
    const grow = (player: PlayerId, instanceId = "ex"): GameEvent[] => [
      { type: "activated", player, instanceId, defId: TREE, ability: "grow" },
      { type: "promptOpened", player },
      { type: "fused", instanceIds: [instanceId, "fx-field"], resultInstanceId: instanceId, defId: TREE },
    ];
    const [glow, offer, branch, other] = run(TREE, [...grow("p1"), grow("p1", "fx-other")[2] ?? { type: "promptOpened", player: "p2" }], D, "p1", fielded(TREE), "activate");
    expect(glow).toMatchObject({ cues: [{ kind: "ring", at: OWN, delayMs: 0 }, { kind: "burst", at: OWN, spread: "area" }], row: "replace" });
    expect(offer).toEqual({ cues: [{ kind: "sheen", at: OWN, delayMs: 0, durationMs: D }], row: "replace" });
    expect(branch).toMatchObject({ cues: [{ kind: "figure", figure: "branch", at: OWN, side: "you" }, { kind: "ring", at: OWN }], row: "replace" });
    // A fusion into another card is no branch of its own.
    expect(other).toEqual({ cues: [], row: undefined });
    // The other seat's Tree: it glows and branches, but its Discover shows nothing here.
    const theirs = run(TREE, grow("p2"), D, "p2", fielded(TREE, "p2"), "activate");
    expect(theirs.map((result) => result.cues.map((cue) => cue.kind))).toEqual([["ring", "burst"], [], ["figure", "ring"]]);
    expect(theirs[1]?.row).toBeUndefined();
    expect(theirs[2]?.cues[0]).toMatchObject({ side: "opponent" });
  });

  it("R3405001 Aluneth: an iron lantern lights ice-blue; at its end of turn each book flaps off as an ink bird into its owner's hand as a card back, its gem frosting cheaper for its owner alone", () => {
    const cues = entrance("lanternLight", ALUNETH, D);
    expect(beats(cues)).toEqual(["figure:lantern@0", `burst@${String(SLAM)}`, `ring@${String(SLAM)}`, `burst@${String(SLAM)}`, `shake@${String(SLAM)}`]);
    expect(cues[3]).toMatchObject({ preset: "frost", at: tid(TGT, FOOT) });
    const books = [1, 2, 3].flatMap((n): GameEvent[] => [
      { type: "drawn", player: "p1", instanceId: `fx-book-${String(n)}`, defId: "core-002" },
      { type: "costChanged", instanceId: `fx-book-${String(n)}`, cost: 0 },
    ]);
    const results = run(ALUNETH, [...books, { type: "drawn", player: "p2", instanceId: "hidden", defId: "hidden" }], D, "p1", fielded(ALUNETH), "endOfTurn");
    const hand = tid(animTestid.hand("you"));
    const [bird, frost] = results;
    expect(bird?.row).toBe("replace");
    expect(bird?.cues).toMatchObject([
      { kind: "figure", figure: "inkBird", at: OWN, side: "you", delayMs: 0 },
      { kind: "burst", at: OWN, delayMs: 0 },
      { kind: "ghost", from: OWN, to: hand },
    ]);
    const fly = bird?.cues[2]?.delayMs ?? 0;
    expect(fly).toBeGreaterThan(0);
    expect(bird?.cues[2]).toMatchObject({ durationMs: D - fly });
    expect(frost).toMatchObject({ cues: [{ kind: "sheen", at: hand, delayMs: 0, durationMs: D }, { kind: "burst", preset: "frost", at: hand }], row: "replace" });
    // All three books fly alike; the other player's draw is no book of its.
    expect(results.slice(0, 6)).toEqual([bird, frost, bird, frost, bird, frost]);
    expect(results[6]).toEqual({ cues: [], row: undefined });
    // The other seat's Aluneth: its books fly as card backs to its hand pile, their gems frost unseen here.
    const theirs = run(ALUNETH, [{ type: "drawn", player: "p2", instanceId: "hidden", defId: "hidden" }, { type: "costChanged", instanceId: "hidden", cost: 0 }], D, "p2", fielded(ALUNETH, "p2"), "endOfTurn");
    expect(theirs[0]?.cues[2]).toMatchObject({ kind: "ghost", to: tid(animTestid.hand("opponent")) });
    expect(theirs[1]).toEqual({ cues: [], row: "replace" });
  });
});
