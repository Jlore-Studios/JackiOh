// R3405001: Meditative's first batch of Unit signatures, card by card (devinBot.ts, keymasterKeenus.ts,
// touchedByKy.ts, mayorMedinamogger.ts, tatchesTheTotem.ts, cnGolem.ts, prison.ts, jlockeedHq.ts): each
// entrance's own figures, its one shake at the slam with its key's trauma and, for a Mythic, nothing before
// its takeover's hold; then each beat of its text planned as the board's effects layer plans it (remember,
// then plan through the dispatch), under the hook a game runs it in: what it draws, which pile its card
// backs fly to, a sheen on the owner's seat alone, and an event its recipe does not claim left to its own
// row.

import type { GameEvent, Keyword, PlayerId, PlayerView, UnitView } from "@jackioh/shared";
import { describe, expect, it } from "vitest";

import { animTestid } from "../../../game/animations.ts";
import { testid } from "../../../game/contract.ts";
import { fullBoardView } from "../../../test/fixtures.ts";
import { HOLD, beats, run, tid } from "../../../test/signatures.ts";
import { frac } from "../../build.ts";
import { FX_INTENSITY_SCALE, FX_SLAM_AT } from "../../constants.ts";
import { ENTRANCES } from "../../entrances.ts";
import type { FxCue } from "../../types.ts";
import { ENTRANCE_TRAUMA, FOOT, type EntranceKey } from "../recipe.ts";
import { KEENUS_KEYS, KEENUS_KEYS_MAX } from "./keymasterKeenus.ts";

const DEVIN = "meditative-051";
const KEENUS = "meditative-065";
const KY = "meditative-079";
const MAYOR = "meditative-086";
const TATCHES = "meditative-087";
const GOLEM = "meditative-095-1";
const PRISON = "meditative-097-8";
const HQ = "meditative-097-9";

/** A Legendary's entry; a Mythic's, its takeover's hold and 1400 ms of motion after it. */
const D = 400;
const MYTHIC_D = HOLD + 1400;

/** The unit lane each entrance lands in, where the card stands as "ex" for its text's beats. */
const LANE = 3;

/** A plain 1/1 Common Unit with no entrance of its own; a plain Jlockeed Unit with none. */
const ONE_ONE = "core-004";
const JLOCKEED_UNIT = "core-013";

/** `key`'s entrance for `defId` landing in `player`'s unit zone LANE on `view`, its entry lasting `duration` after `lead`; its zone and slam. */
function entrance(key: EntranceKey, defId: string, duration: number, lead = 0, player: PlayerId = "p1", view: PlayerView = fullBoardView()): { cues: FxCue[]; tgt: string; slam: number } {
  const landing: Extract<GameEvent, { type: "summoned" }> = { type: "summoned", player, instanceId: "ex", defId, row: "units", lane: LANE };
  const tgt = testid.zone(player === view.viewer ? "you" : "opponent", "units", LANE);
  const cues = ENTRANCES[key]({ D: duration, tgt, intensity: FX_INTENSITY_SCALE.normal, view, landing, leadMs: lead });
  return { cues, tgt, slam: lead + frac(FX_SLAM_AT, duration - lead) };
}

/** An entrance's frame: its one shake at the slam with its key's trauma, and no cue before its lead. */
function expectFrame(cues: readonly FxCue[], key: EntranceKey, slam: number, lead = 0): void {
  expect(cues.filter((cue) => cue.kind === "shake")).toEqual([{ kind: "shake", trauma: ENTRANCE_TRAUMA[key], delayMs: slam }]);
  for (const cue of cues) expect(cue.delayMs, `${key} ${cue.kind}`).toBeGreaterThanOrEqual(lead);
}

/** `side`'s Unit in board slot `k` (lane k + 1). */
function unitAt(view: PlayerView, side: "you" | "opponent", k: number): UnitView {
  const unit = view[side].units[k];
  if (unit === undefined || unit === null) throw new Error(`no ${side} unit in slot ${String(k)}`);
  return unit;
}

/** The board with your Unit in lane LANE standing as "ex", the instance `run` plays (`keywords` its own when given), and your lanes past it empty when `open`. */
function standing(open = false, keywords?: Keyword[]): PlayerView {
  const view = fullBoardView();
  const units = view.you.units.map((unit, k) => (k === LANE - 1 && unit !== null ? { ...unit, instanceId: "ex", ...(keywords === undefined ? {} : { keywords }) } : open && k >= LANE ? null : unit));
  return { ...view, you: { ...view.you, units } };
}

/** Each cue's figure name, else its kind. */
const shapes = (cues: readonly FxCue[] = []): string[] => cues.map((cue) => (cue.kind === "figure" ? cue.figure : cue.kind));

/** A Unit `player` summons into unit lane `lane`. */
const summon = (player: PlayerId, lane: number, defId = ONE_ONE): GameEvent => ({ type: "summoned", player, instanceId: `fx-unit-${String(lane)}`, defId, row: "units", lane });

describe("R3405001 Meditative's Unit signatures, first batch", () => {
  it("R3405001 Devin Bot: he drops onto his cushion in a puff at the slam and his eye-lights dim; his Cry beams a hologram wireframe out of him into each zone it fills, solidifying with a bounce", () => {
    const { cues, tgt, slam } = entrance("cushionDrop", DEVIN, D);
    expectFrame(cues, "cushionDrop", slam);
    expect(beats(cues)).toEqual(["figure:cushion@0", `burst@${String(slam)}`, `ring@${String(slam)}`, `shake@${String(slam)}`, `burst@${String(frac(0.8, D))}`]);
    expect(cues[0]).toMatchObject({ at: tid(tgt), side: "you" });
    expect(cues[1]).toMatchObject({ preset: "dust", spread: "area", at: tid(tgt, FOOT) });
    // His Cry fills his two empty zones, left to right: each a hum, a beam and a wireframe solidifying in it.
    const view = standing(true);
    const him = tid(testid.card("ex"));
    const fills = run(DEVIN, [summon("p1", 4), summon("p1", 5)], D, "p1", view);
    expect(fills.map((entry) => entry.row)).toEqual(["replace", "replace"]);
    fills.forEach((entry, k) => {
      const zone = testid.zone("you", "units", LANE + 1 + k);
      expect(shapes(entry.cues)).toEqual(["ring", "shot", "wireframe", "burst"]);
      expect(entry.cues).toMatchObject([
        { at: him, delayMs: 0 },
        { from: him, to: tid(zone), delayMs: 0 },
        { at: tid(zone), side: "you", delayMs: 0 },
        { spread: "point", at: tid(zone, FOOT) },
      ]);
      const [, beam, , bounce] = entry.cues;
      if (beam?.kind !== "shot") throw new Error("no beam");
      // It solidifies after the beam lands.
      expect(bounce?.delayMs).toBeGreaterThan(beam.flightMs);
    });
    // The other seat's Devin fills its own side.
    expect(run(DEVIN, [summon("p2", 4)], D, "p2")[0]?.cues[1]).toMatchObject({ to: tid(testid.zone("opponent", "units", 4)) });
    // The opponent's summon is none of his: its row plays.
    expect(run(DEVIN, [summon("p2", 4)], D, "p1", view)).toEqual([{ cues: [], row: undefined }]);
  });

  it("R3405001 Keymaster Keenus: his key ring unwinds and jangles at the slam, then one key clicks open over him per keyword his unit shows; his Equip turns the same keys over the Unit he fuses into", () => {
    const { cues, tgt, slam } = entrance("keyRing", KEENUS, D);
    expectFrame(cues, "keyRing", slam);
    // Not on the board's view: his base face's KEENUS_KEYS, clicking left to right after his bow.
    const clicks = Array.from({ length: KEENUS_KEYS }, (_, k) => `burst@${String(slam + frac(0.05, D) + Math.round((k * frac(0.3, D)) / KEENUS_KEYS))}`);
    expect(beats(cues)).toEqual(["figure:keyRing@0", `ring@${String(slam)}`, `burst@${String(slam)}`, `shake@${String(slam)}`, `figure:keyTurn@${String(slam)}`, ...clicks]);
    expect(cues[0]).toMatchObject({ at: tid(tgt), side: "you" });
    expect(cues[5]).toMatchObject({ preset: "spark", spread: "point", at: { kind: "testid", testid: tgt } });
    // One key per keyword he shows, his Equip aside; at most KEENUS_KEYS_MAX.
    const keyed = (view: PlayerView): number => entrance("keyRing", KEENUS, D, 0, "p1", view).cues.slice(5).length;
    expect(keyed(standing(false, [{ kind: "Rush" }, { kind: "Taunt" }, { kind: "Equip" }]))).toBe(2);
    expect(keyed(standing(false, Array.from({ length: KEENUS_KEYS_MAX + 2 }, (): Keyword => ({ kind: "Armor", n: 1 }))))).toBe(KEENUS_KEYS_MAX);
    // His Equip play fuses him into your first Unit: the keys turn over it.
    const view = fullBoardView();
    const host = unitAt(view, "you", 0);
    const fuse = (instanceIds: string[], result: string): GameEvent => ({ type: "fused", instanceIds, resultInstanceId: result, defId: host.defId });
    const [equip, other] = run(KEENUS, [fuse([host.instanceId, "ex"], host.instanceId), fuse([host.instanceId, "fx-other"], host.instanceId)], D, "p1", view);
    const box = testid.card(host.instanceId);
    expect(equip?.row).toBe("replace");
    expect(shapes(equip?.cues)).toEqual(["keyTurn", ...Array.from({ length: KEENUS_KEYS }, () => "burst")]);
    expect(equip?.cues[0]).toMatchObject({ at: tid(box), side: "you", delayMs: 0 });
    for (const click of equip?.cues.slice(1) ?? []) expect(click).toMatchObject({ preset: "spark", at: { kind: "testid", testid: box } });
    // A Unit the board no longer shows: the keys turn at his hero.
    expect(run(KEENUS, [fuse(["fx-gone", "ex"], "fx-gone")], D, "p1", view)[0]?.cues[0]).toMatchObject({ at: tid(testid.hero("you")) });
    // A fusion without him is none of his: its row plays.
    expect(other).toEqual({ cues: [], row: undefined });
  });

  it("R3405001 Touched by KY: a chalk handprint presses down and he materialises at the slam, dust raining from his hair; his Cry's first draw taps his brow and stretches his hand pile in chalk outlines, each draw a card back from his deck pile", () => {
    const { cues, tgt, slam } = entrance("chalkHand", KY, D);
    expectFrame(cues, "chalkHand", slam);
    expect(beats(cues)).toEqual(["figure:chalkHand@0", `burst@${String(slam)}`, `ring@${String(slam)}`, `shake@${String(slam)}`, `burst@${String(frac(0.8, D))}`]);
    expect(cues[0]).toMatchObject({ at: tid(tgt), side: "you" });
    expect(cues[4]).toMatchObject({ spread: "point", at: tid(tgt, { x: 0.5, y: 0.12 }) });
    // His Cry's two draws, then the opponent's own.
    const draw = (player: PlayerId, n: number): GameEvent => (player === "p1" ? { type: "drawn", player, instanceId: `fx-draw-${String(n)}`, defId: "core-002" } : { type: "drawn", player, instanceId: "hidden", defId: "hidden" });
    const [first, second, theirs] = run(KY, [draw("p1", 1), draw("p1", 2), draw("p2", 3)], D, "p1", standing());
    const deck = tid(animTestid.library("you"));
    const hand = tid(animTestid.hand("you"));
    expect(first?.row).toBe("replace");
    expect(first?.cues).toMatchObject([
      { kind: "figure", figure: "chalkHand", at: tid(testid.card("ex")), size: "small", side: "you", delayMs: 0 },
      { kind: "burst", at: tid(testid.card("ex"), { x: 0.5, y: 0.3 }) },
      { kind: "figure", figure: "handOutline", at: hand, side: "you" },
      { kind: "ghost", from: deck, to: hand },
    ]);
    // The tap, then the outlines stretching, then the card back flying in, landing as the entry ends.
    const [, tap, stretch, back] = first?.cues ?? [];
    if (tap === undefined || stretch === undefined || back?.kind !== "ghost") throw new Error("no draw beat");
    expect(tap.delayMs).toBeLessThan(stretch.delayMs);
    expect(stretch.delayMs).toBeLessThan(back.delayMs);
    expect(back.delayMs + back.durationMs).toBe(D);
    expect(second).toEqual({ cues: [{ kind: "ghost", from: deck, to: hand, delayMs: 0, durationMs: D }], row: "replace" });
    // The opponent's draw is none of his: its row plays.
    expect(theirs).toEqual({ cues: [], row: undefined });
    // The other seat's: a bare card back from its deck pile to its hand pile.
    expect(run(KY, [draw("p2", 1)], D, "p2")[0]?.cues[3]).toMatchObject({ kind: "ghost", from: tid(animTestid.library("opponent")), to: tid(animTestid.hand("opponent")) });
  });

  it("R3405001 Mayor Medinamogger: bunting snaps across his zone and his silver coin flips, caught at the slam in a glint and the crowd's confetti; no event names his aura, so his play claims none", () => {
    const { cues, tgt, slam } = entrance("coinFlip", MAYOR, D);
    expectFrame(cues, "coinFlip", slam);
    expect(beats(cues)).toEqual(["figure:bunting@0", `figure:silverCoin@${String(frac(0.15, D))}`, `burst@${String(slam)}`, `ring@${String(slam)}`, `burst@${String(slam)}`, `shake@${String(slam)}`]);
    expect(cues.slice(0, 2)).toMatchObject([
      { at: tid(tgt), side: "you" },
      { at: tid(tgt), side: "you" },
    ]);
    expect(cues[2]).toMatchObject({ preset: "sparkle", spread: "point", at: tid(tgt, { x: 0.5, y: 0.3 }) });
    expect(cues[4]).toMatchObject({ spread: "area", at: tid(tgt, { x: 0.5, y: 0.05 }) });
    // The other seat's Mayor strides onto its own side.
    const theirs = entrance("coinFlip", MAYOR, D, 0, "p2");
    expect(theirs.cues[0]).toMatchObject({ at: tid(theirs.tgt), side: "opponent" });
    // No recipe: the hit of a target his aura picked plays its own row.
    const view = fullBoardView();
    expect(run(MAYOR, [{ type: "damage", sourceId: "ex", targetId: unitAt(view, "opponent", 0).instanceId, amount: 2, combat: false }], D, "p1", view)).toEqual([{ cues: [], row: undefined }]);
  });

  it("R3405001 Tatches the Totem: it bolts up out of its owner's deck pile and its five tiers wake bottom first, settling at the slam; its end-of-turn Buff climbs the tiers and arcs to the hand or deck pile, sheened for its owner alone, or onto the Unit beside it", () => {
    const { cues, tgt, slam } = entrance("totemStack", TATCHES, D);
    expectFrame(cues, "totemStack", slam);
    const wakes = [0, 1, 2, 3, 4].map((k) => `burst@${String(frac(0.25 + k * 0.08, D))}`);
    expect(beats(cues)).toEqual(["burst@0", "shot@0", `figure:totem@${String(frac(0.2, D))}`, ...wakes, `ring@${String(slam)}`, `burst@${String(slam)}`, `shake@${String(slam)}`]);
    const library = tid(animTestid.library("you"));
    expect(cues.slice(0, 3)).toMatchObject([{ at: library }, { from: library, to: tid(tgt) }, { at: tid(tgt), side: "you" }]);
    // Its Buffs: a hand card its owner reads, a card no seat reads (a deck card), the Unit beside it (its Radiant face's).
    const view = standing();
    const own = Array.isArray(view.you.hand) ? view.you.hand[0] : undefined;
    if (own === undefined) throw new Error("no hand card");
    const beside = unitAt(view, "you", LANE);
    const buff = (card: { instanceId: string; defId: string }): GameEvent => ({ type: "upgraded", instanceId: card.instanceId, defId: card.defId, change: { kind: "stats", attack: 1, health: 1 } });
    const hidden = { instanceId: "hidden", defId: "hidden" };
    const [hand, deck, adjacent, unclaimed] = run(TATCHES, [buff(own), buff(hidden), buff(beside), { type: "healed", targetId: beside.instanceId, amount: 1 }], D, "p1", view, "endOfTurn");
    // The light climbs its five tiers bottom first, then arcs off its crown.
    const climb = hand?.cues.slice(0, 5) ?? [];
    expect(climb.map((cue) => cue.delayMs)).toEqual([0, 1, 2, 3, 4].map((k) => frac(k * 0.07, D)));
    expect(climb[0]).toMatchObject({ kind: "burst", at: tid(testid.card("ex"), { x: 0.5, y: 0.85 }) });
    for (const [entry, pile] of [
      [hand, animTestid.hand("you")],
      [deck, animTestid.library("you")],
    ] as const) {
      expect(entry?.row).toBe("replace");
      expect(shapes(entry?.cues)).toEqual(["burst", "burst", "burst", "burst", "burst", "projectile", "sheen"]);
      expect(entry?.cues.slice(5)).toMatchObject([{ from: tid(testid.card("ex"), { x: 0.5, y: 0.1 }), to: tid(pile) }, { at: tid(pile) }]);
    }
    // The Unit beside it: the light lands on it and its own stat tick plays.
    expect(adjacent?.row).toBe("keep");
    expect(shapes(adjacent?.cues).slice(5)).toEqual(["projectile"]);
    expect(adjacent?.cues[5]).toMatchObject({ to: tid(testid.card(beside.instanceId)) });
    expect(unclaimed).toEqual({ cues: [], row: undefined });
    // The other seat's Buff: arcing to its deck pile, no sheen here, never naming hand or deck.
    const [theirs] = run(TATCHES, [buff(hidden)], D, "p2", fullBoardView(), "endOfTurn");
    expect(shapes(theirs?.cues).slice(5)).toEqual(["projectile"]);
    expect(theirs?.cues[5]).toMatchObject({ from: tid(testid.hero("opponent"), { x: 0.5, y: 0.1 }), to: tid(animTestid.library("opponent")) });
  });

  it("R3405001 CN Golem: lacquer panels and stone blocks lock together at the slam, dust bursting from its joints; each Super-Virus its kill shuffles into the opponent's deck leaks out of it as a green mote, a card back flying into that deck pile", () => {
    const { cues, tgt, slam } = entrance("golemLock", GOLEM, D);
    expectFrame(cues, "golemLock", slam);
    expect(beats(cues)).toEqual(["figure:golemBlocks@0", `burst@${String(slam)}`, `burst@${String(slam)}`, `burst@${String(slam)}`, `ring@${String(slam)}`, `shake@${String(slam)}`]);
    expect(cues[0]).toMatchObject({ at: tid(tgt), side: "you" });
    expect(cues.slice(1, 5)).toMatchObject([
      { preset: "shard", spread: "ring", at: tid(tgt) },
      { spread: "point", at: tid(tgt, { x: 0.25, y: 0.35 }) },
      { spread: "point", at: tid(tgt, { x: 0.75, y: 0.35 }) },
      { at: tid(tgt, FOOT) },
    ]);
    // Its kill trigger: a Super-Virus into the opponent's deck; the kill itself and one into its own deck beside it.
    const view = standing();
    const foe = unitAt(view, "opponent", 0);
    const killed: GameEvent = { type: "destroyed", instanceId: foe.instanceId, defId: foe.defId, owner: "p2", controller: "p2", attack: 4, maxHealth: 4, killerId: "ex" };
    const virus = (player: PlayerId): GameEvent => ({ type: "shuffledIn", player, instanceId: "fx-virus", defId: "core-090-1", position: 0, creator: player === "p1" ? "p2" : "p1" });
    const [kill, leak, ownDeck] = run(GOLEM, [killed, virus("p2"), virus("p1")], D, "p1", view, "whenever");
    const golem = tid(testid.card("ex"));
    const pile = tid(animTestid.library("opponent"));
    expect(leak?.row).toBe("replace");
    expect(leak?.cues).toMatchObject([
      { kind: "burst", at: golem, delayMs: 0 },
      { kind: "projectile", from: golem, to: pile, delayMs: 0 },
      { kind: "ghost", from: golem, to: pile, delayMs: 0 },
    ]);
    // The mote and its card back reach the pile together, before the entry ends.
    const [, mote, back] = leak?.cues ?? [];
    if (mote?.kind !== "projectile" || back?.kind !== "ghost") throw new Error("no leak");
    expect(back.durationMs).toBe(mote.flightMs);
    expect(back.durationMs).toBeLessThan(D);
    // The kill is its attack's, and a shuffle into its own deck none of its: their rows play.
    expect([kill, ownDeck]).toEqual([
      { cues: [], row: undefined },
      { cues: [], row: undefined },
    ]);
    // Gone from the field since its kill, it leaks from its hero; the other seat's leaks into this seat's deck.
    expect(run(GOLEM, [virus("p2")], D, "p1", fullBoardView(), "whenever")[0]?.cues[2]).toMatchObject({ from: tid(testid.hero("you")) });
    expect(run(GOLEM, [virus("p1")], D, "p2", fullBoardView(), "whenever")[0]?.cues[2]).toMatchObject({ from: tid(testid.hero("opponent")), to: tid(animTestid.library("you")) });
  });

  it("R3405001 Prison: iron bars clang down at the slam over a rising stone cell; after an enemy Unit is played, its capture swings the lamp's beam onto it and chains drag it into the cell beneath Prison", () => {
    const { cues, tgt, slam } = entrance("prisonBars", PRISON, D);
    expectFrame(cues, "prisonBars", slam);
    expect(beats(cues)).toEqual(["figure:prisonBars@0", `burst@${String(slam)}`, `burst@${String(slam)}`, `ring@${String(slam)}`, `shake@${String(slam)}`]);
    expect(cues[0]).toMatchObject({ at: tid(tgt), side: "you" });
    expect(cues.slice(1, 3)).toMatchObject([
      { preset: "spark", spread: "point", at: tid(tgt, { x: 0.5, y: 0.05 }) },
      { spread: "area", at: tid(tgt, FOOT) },
    ]);
    // Captures into Prison's zone: an enemy Unit the board shows, and one the board shows only as landed.
    const view = standing();
    const foe = unitAt(view, "opponent", 0);
    const capture = (instanceId: string, controller: PlayerId = "p1"): GameEvent => ({ type: "controlChanged", instanceId, controller, row: "units", lane: LANE, how: "steal" });
    const [shown, landed, caught, theirs] = run(PRISON, [capture(foe.instanceId), summon("p2", 2), capture("fx-unit-2"), capture(foe.instanceId, "p2")], D, "p1", view, "whenever");
    const cell = testid.zone("you", "units", LANE);
    const at = tid(testid.card(foe.instanceId));
    expect(shown?.row).toBe("replace");
    expect(shown?.cues).toMatchObject([
      { kind: "shot", shot: { motion: "beam" }, from: tid(cell), to: at, delayMs: 0 },
      { kind: "figure", figure: "chains", at, side: "you" },
      { kind: "shot", from: at, to: tid(cell) },
      { kind: "burst", spread: "area", at: tid(cell, FOOT) },
    ]);
    // The beam lands, the chains lock, then drag it in, the dust settling as it reaches the cell.
    const [beam, chains, drag, settle] = shown?.cues ?? [];
    if (beam?.kind !== "shot" || drag?.kind !== "shot" || chains === undefined || settle === undefined) throw new Error("no capture");
    expect(chains.delayMs).toBe(beam.flightMs);
    expect(drag.delayMs).toBeGreaterThan(chains.delayMs);
    expect(settle.delayMs).toBe(drag.delayMs + drag.flightMs);
    expect(caught?.cues[2]).toMatchObject({ from: tid(testid.zone("opponent", "units", 2)), to: tid(cell) });
    // The Unit's own landing and the other player taking control are none of Prison's: their rows play.
    expect([landed, theirs]).toEqual([
      { cues: [], row: undefined },
      { cues: [], row: undefined },
    ]);
    // The other seat's Prison drags this seat's Unit into its own cell.
    const mine = unitAt(view, "you", 0);
    expect(run(PRISON, [capture(mine.instanceId, "p2")], D, "p2", fullBoardView(), "whenever")[0]?.cues[2]).toMatchObject({ from: tid(testid.card(mine.instanceId)), to: tid(testid.zone("opponent", "units", LANE)) });
  });

  it("R3405001 Jlockeed's Headquarters: after its takeover's hold the fortress shrinks into its zone, its beacons flashing twice, and settles at the slam; its Activate rolls a contractor's truck out of its hangar to each zone it fills, unloading in a puff of dust", () => {
    const { cues, tgt, slam } = entrance("fortressRise", HQ, MYTHIC_D, HOLD);
    expectFrame(cues, "fortressRise", slam, HOLD);
    const flash = (at: number): string => `burst@${String(HOLD + frac(at, MYTHIC_D - HOLD))}`;
    expect(beats(cues)).toEqual([`figure:fortress@${String(HOLD)}`, flash(0.15), flash(0.15), flash(0.4), flash(0.4), `ring@${String(slam)}`, `burst@${String(slam)}`, `shake@${String(slam)}`]);
    expect(cues[0]).toMatchObject({ at: tid(tgt), side: "you" });
    expect(cues.slice(1, 3)).toMatchObject([
      { preset: "spark", at: tid(tgt, { x: 0.12, y: 0.08 }) },
      { preset: "spark", at: tid(tgt, { x: 0.88, y: 0.08 }) },
    ]);
    expect(cues[6]).toMatchObject({ spread: "area", at: tid(tgt, FOOT) });
    // Its Activate fills its two empty zones, left to right; the opponent's summon beside them.
    const hq = tid(testid.card("ex"));
    const [first, second, theirs] = run(HQ, [summon("p1", 4, JLOCKEED_UNIT), summon("p1", 5, JLOCKEED_UNIT), summon("p2", 4, JLOCKEED_UNIT)], D, "p1", standing(true), "activate");
    [first, second].forEach((entry, k) => {
      const zone = testid.zone("you", "units", LANE + 1 + k);
      expect(entry?.row).toBe("replace");
      expect(entry?.cues).toMatchObject([
        { kind: "figure", figure: "convoy", at: hq, side: "you", delayMs: 0 },
        { kind: "shot", from: hq, to: tid(zone) },
        { kind: "burst", preset: "dust", at: tid(zone, FOOT), delayMs: D },
      ]);
      // The truck rolls out after the doors open and reaches its zone as the entry ends.
      const truck = entry?.cues[1];
      if (truck?.kind !== "shot") throw new Error("no truck");
      expect(truck.delayMs).toBeGreaterThan(0);
      expect(truck.delayMs + truck.flightMs).toBe(D);
    });
    expect(theirs).toEqual({ cues: [], row: undefined });
    // The other seat's unloads into its own side, from its hero while the board does not show it.
    expect(run(HQ, [summon("p2", 4, JLOCKEED_UNIT)], D, "p2", fullBoardView(), "activate")[0]?.cues[1]).toMatchObject({ from: tid(testid.hero("opponent")), to: tid(testid.zone("opponent", "units", 4)) });
  });
});
