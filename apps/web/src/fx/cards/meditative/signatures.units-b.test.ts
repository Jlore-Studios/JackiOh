// R3405001: Meditative's second batch of Unit signatures, card by card (jadeBeauty.ts, knowledgeBreaker.ts,
// knowledgeBreakerPrime.ts, taotie.ts, yileGpt.ts, yileGptPrime.ts, yilesVirus.ts, aiGirlfriend.ts): each
// entrance's own figures, its one shake at the slam with its key's trauma and, for a Mythic, nothing before
// its takeover's hold; then each beat of its text planned as the board's effects layer plans it (remember,
// then plan through the dispatch), under the hook a game runs it in: what it draws, which pile its card
// backs fly to, a sheen on the owner's seat alone, a summon of a card with an entrance of its own playing
// that entrance, and an event its recipe does not claim left to its own row.

import type { GameEvent, PlayerId, PlayerView, UnitView } from "@jackioh/shared";
import { describe, expect, it } from "vitest";

import { ALLURE_MARK, ALLURE_MARK_COLOR } from "../../../cards/marks.ts";
import { animTestid } from "../../../game/animations.ts";
import { testid } from "../../../game/contract.ts";
import { fullBoardView } from "../../../test/fixtures.ts";
import { HOLD, added, beats, run, tid } from "../../../test/signatures.ts";
import { frac } from "../../build.ts";
import { FX_INTENSITY_SCALE, FX_SLAM_AT } from "../../constants.ts";
import { ENTRANCES } from "../../entrances.ts";
import type { FxCue } from "../../types.ts";
import { ENTRANCE_TRAUMA, FOOT, type EntranceKey } from "../recipe.ts";

const JADE_BEAUTY = "meditative-039-5";
const BREAKER = "meditative-045";
const PRIME = "meditative-045-1";
const TAOTIE = "meditative-047";
const YILE = "meditative-049";
const YILE_PRIME = "meditative-049-1";
const VIRUS = "meditative-049-2";
const GIRLFRIEND = "meditative-049-3";

/** A Legendary's entry; a Mythic's, its takeover's hold and 1400 ms of motion after it. */
const D = 400;
const MYTHIC_D = HOLD + 1400;

/** The unit lane each entrance lands in. */
const LANE = 3;

/** A Nerf of one attack and one health. */
const NERF = { kind: "stats", attack: -1, health: -1 } as const;

/** `key`'s entrance for `defId` landing in `player`'s unit zone LANE, its entry lasting `duration` after `lead`; its zone and slam. */
function entrance(key: EntranceKey, defId: string, duration: number, lead = 0, player: PlayerId = "p1"): { cues: FxCue[]; tgt: string; slam: number } {
  const view = fullBoardView();
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

/** The viewer's first hand card. */
function handCard(view: PlayerView): { instanceId: string; defId: string } {
  const card = Array.isArray(view.you.hand) ? view.you.hand[0] : undefined;
  if (card === undefined) throw new Error("no hand card");
  return card;
}

/** The board with your Unit in lane LANE standing as "ex", the instance `run` plays: the card itself on the board. */
function standing(): PlayerView {
  const view = fullBoardView();
  return { ...view, you: { ...view.you, units: view.you.units.map((unit, k) => (k === LANE - 1 && unit !== null ? { ...unit, instanceId: "ex" } : unit)) } };
}

/** Each cue's figure name, else its kind. */
const shapes = (cues: readonly FxCue[] = []): string[] => cues.map((cue) => (cue.kind === "figure" ? cue.figure : cue.kind));

const at = (n: number): string => `@${String(n)}`;

describe("R3405001 Meditative's Unit signatures, second batch", () => {
  it("R3405001 Jade Beauty: after her takeover's hold the jade boulder splits at the slam, dust falling from her sleeves; at her end of turn she lifts her fan once and a ribbon charms each enemy Unit", () => {
    const { cues, tgt, slam } = entrance("jadeBoulder", JADE_BEAUTY, MYTHIC_D, HOLD);
    expectFrame(cues, "jadeBoulder", slam, HOLD);
    expect(beats(cues).slice(0, 3)).toEqual([`figure:jadeBoulder${at(HOLD)}`, `burst${at(slam)}`, `ring${at(slam)}`]);
    expect(cues[0]).toMatchObject({ figure: "jadeBoulder", at: tid(tgt), side: "you" });
    expect(cues[1]).toMatchObject({ spread: "ring", at: tid(tgt) });
    // Dust falls from her two sleeves after the split.
    const dust = cues.filter((cue) => cue.kind === "burst" && cue.preset === "dust");
    expect(dust).toHaveLength(2);
    for (const cue of dust) expect(cue.delayMs).toBeGreaterThan(slam);
    // Her end of turn: Allure on two enemy Units; she stands at her hero, her hook's card off this board.
    const view = fullBoardView();
    const foes = [unitAt(view, "opponent", 0), unitAt(view, "opponent", 1)];
    const allure = (instanceId: string, added = true): GameEvent => ({ type: "marked", instanceId, mark: ALLURE_MARK, color: ALLURE_MARK_COLOR, added });
    const marks = run(JADE_BEAUTY, foes.map((foe) => allure(foe.instanceId)), D, "p1", view, "endOfTurn");
    const her = tid(testid.hero("you"));
    expect(marks.map((entry) => entry.row)).toEqual(["replace", "replace"]);
    expect(marks.map((entry) => shapes(entry.cues))).toEqual([["silkFan", "shot", "burst"], ["shot", "burst"]]);
    expect(marks[0]?.cues[0]).toMatchObject({ figure: "silkFan", at: her, delayMs: 0, side: "you" });
    marks.forEach((entry, k) => {
      const charmed = tid(testid.card(foes[k]?.instanceId ?? ""));
      const [shot, charm] = entry.cues.slice(-2);
      expect(shot).toMatchObject({ kind: "shot", preset: "sparkle", from: her, to: charmed });
      // The ribbon charms its Unit in a ring of sparkles as it lands.
      if (shot?.kind !== "shot") throw new Error("no ribbon");
      expect(charm).toMatchObject({ kind: "burst", preset: "sparkle", spread: "ring", at: charmed, delayMs: shot.delayMs + shot.flightMs });
    });
    // Allure lifted is no charm: its row plays.
    expect(run(JADE_BEAUTY, [allure(foes[0]?.instanceId ?? "", false)], D, "p1", view, "endOfTurn")).toEqual([{ cues: [], row: undefined }]);
  });

  it("R3405001 Knowledge Breaker: chalk puffs and his mask turns, ringing out at the slam; his Cry lashes a tendril and arrows down onto each card it Nerfs that this seat reads, and his Death shuffles a card back into his deck", () => {
    const { cues, tgt, slam } = entrance("chalkMask", BREAKER, D);
    expectFrame(cues, "chalkMask", slam);
    expect(beats(cues)).toEqual(["burst@0", "figure:porcelainMask@0", `ring${at(slam)}`, `burst${at(slam)}`, `shake${at(slam)}`]);
    expect(cues[0]).toMatchObject({ spread: "area", at: tid(tgt, FOOT) });
    expect(cues[1]).toMatchObject({ at: tid(tgt), side: "you" });
    // His Cry: an enemy Unit and the viewer's own hand card, each lashed where it stands.
    const view = fullBoardView();
    const foe = unitAt(view, "opponent", 0);
    const own = handCard(view);
    const nerf = (instanceId: string, defId: string, hiddenFrom?: PlayerId[]): GameEvent => ({ type: "degraded", instanceId, defId, change: NERF, ...(hiddenFrom === undefined ? {} : { hiddenFrom }) });
    const [unit, hand, unread, theirs] = run(BREAKER, [nerf(foe.instanceId, foe.defId), nerf(own.instanceId, own.defId), nerf(own.instanceId, own.defId, ["p1"]), nerf("hidden", "hidden")], D, "p1", view);
    for (const [entry, card] of [
      [unit, testid.card(foe.instanceId)],
      [hand, testid.handCard(own.instanceId)],
    ] as const) {
      expect(entry?.row).toBe("replace");
      expect(shapes(entry?.cues)).toEqual(["chalkTendril", "arrows", "burst"]);
      expect(entry?.cues[0]).toMatchObject({ at: tid(card), side: "you" });
      expect(entry?.cues[1]).toMatchObject({ direction: "down", at: tid(card) });
    }
    // R202: a Nerf hidden from this seat, or on a card in the opponent's hand, draws nothing in its place.
    expect([unread, theirs]).toEqual([
      { cues: [], row: "replace" },
      { cues: [], row: "replace" },
    ]);
    // His Death: from his hero (his card gone from this board), his Prime's card back into his deck pile.
    const shuffled = (player: PlayerId): GameEvent => ({ type: "shuffledIn", player, instanceId: "hidden", defId: "hidden", position: 0 });
    const [death] = run(BREAKER, [shuffled("p1")], D, "p1", view, "death");
    expect(death?.row).toBe("replace");
    expect(death?.cues[1]).toEqual({ kind: "ghost", from: tid(testid.hero("you")), to: tid(animTestid.library("you")), delayMs: 0, durationMs: D });
    expect(run(BREAKER, [shuffled("p2")], D, "p2", view, "death")[0]?.cues[1]).toMatchObject({ kind: "ghost", to: tid(animTestid.library("opponent")) });
    // A shuffle outside his Death is no beat of his: its row plays.
    expect(run(BREAKER, [shuffled("p1")], D, "p1", view)).toEqual([{ cues: [], row: undefined }]);
  });

  it("R3405001 Knowledge Breaker Prime: the mask splits under holy rays and he strides toward the enemy's half at the slam; his Cry slams a card back into each zone it sets, then one chalk wave and each CN or KY permanent crumbling", () => {
    const { cues, tgt, slam } = entrance("maskSplit", PRIME, D);
    expectFrame(cues, "maskSplit", slam);
    const pour = cues[1]?.delayMs ?? 0;
    expect(beats(cues)).toEqual(["figure:maskSplit@0", `rays${at(pour)}`, `burst${at(pour)}`, `ring${at(slam)}`, `burst${at(slam)}`, `shake${at(slam)}`]);
    expect(pour).toBeGreaterThan(0);
    expect(pour).toBeLessThan(slam);
    expect(cues[1]).toMatchObject({ tone: "holy", at: tid(tgt) });
    // His stride kicks chalk up at his zone's edge toward the enemy: its top on this seat, its foot on the other.
    expect(cues[4]).toMatchObject({ spread: "ring", at: tid(tgt, { x: 0.5, y: 0 }) });
    const theirs = entrance("maskSplit", PRIME, D, 0, "p2");
    expect(theirs.cues[4]).toMatchObject({ at: tid(theirs.tgt, FOOT) });
    // His Cry: a Trap set face down in backrow lane 3, then two enemy Units destroyed and a face-down card.
    const view = fullBoardView();
    const [foe, next] = [unitAt(view, "opponent", 0), unitAt(view, "opponent", 1)];
    const destroyed = (card: { instanceId: string; defId: string }): GameEvent => ({ type: "destroyed", instanceId: card.instanceId, defId: card.defId, owner: "p2", controller: "p2", attack: 1, maxHealth: 1, killerId: null });
    const set: GameEvent = { type: "summoned", player: "p1", instanceId: "hidden", defId: "hidden", row: "backrow", lane: 3 };
    const [slammed, first, second, faceDown] = run(PRIME, [set, destroyed(foe), destroyed(next), destroyed({ instanceId: "hidden", defId: "hidden" })], D, "p1", view);
    const zone = tid(testid.zone("you", "backrow", 3));
    const hero = tid(testid.hero("you"));
    expect(slammed?.row).toBe("replace");
    const [back, landed] = slammed?.cues ?? [];
    expect(back).toMatchObject({ kind: "ghost", from: hero, to: zone, delayMs: 0 });
    if (back?.kind !== "ghost") throw new Error("no card back");
    expect(landed).toMatchObject({ kind: "burst", at: zone, delayMs: back.durationMs });
    // The wave rolls out from him at the first removal alone; each card on the board crumbles and cracks.
    expect(shapes(first?.cues)).toEqual(["chalkWave", "ring", "burst", "crack"]);
    expect(first?.cues[0]).toMatchObject({ at: hero, side: "you" });
    expect(first?.cues[3]).toMatchObject({ at: tid(testid.card(foe.instanceId)) });
    expect(shapes(second?.cues)).toEqual(["burst", "crack"]);
    // R202: a face-down card crumbles at its owner's pile, named nowhere.
    expect(faceDown?.cues).toEqual([expect.objectContaining({ kind: "burst", spread: "point", at: tid(animTestid.graveyard("opponent")) })]);
    // The Radiant face exiles in place of destroying: the same wave.
    expect(shapes(run(PRIME, [{ type: "exiled", instanceId: foe.instanceId, defId: foe.defId, owner: "p2" }], D, "p1", view)[0]?.cues)).toEqual(["chalkWave", "ring", "burst", "crack"]);
    // The opponent's own set is none of his: its row plays.
    expect(run(PRIME, [{ ...set, player: "p2" }], D, "p1", view)).toEqual([{ cues: [], row: undefined }]);
  });

  it("R3405001 饕餮: a bronze cauldron slams down in steam and the taotie's face rises from it, eyes kindling; at its end of turn its maw yawns and a vortex drags the Unit it eats into it as a spinning card back", () => {
    const { cues, tgt, slam } = entrance("bronzeCauldron", TAOTIE, D);
    expectFrame(cues, "bronzeCauldron", slam);
    const eyes = cues[5]?.delayMs ?? 0;
    expect(beats(cues)).toEqual(["figure:cauldron@0", `ring${at(slam)}`, `burst${at(slam)}`, `shake${at(slam)}`, `figure:taotieMaw${at(slam)}`, `burst${at(eyes)}`]);
    expect(eyes).toBeGreaterThan(slam);
    expect(eyes).toBeLessThan(D);
    expect(cues[0]).toMatchObject({ at: tid(tgt), side: "you" });
    expect(cues[2]).toMatchObject({ preset: "smoke", spread: "area", at: tid(tgt, FOOT) });
    expect(cues[4]).toMatchObject({ at: tid(tgt), side: "you" });
    // Its end of turn, standing as "ex": it eats the first enemy Unit, keeping its own instance (R77).
    const view = standing();
    const foe = unitAt(view, "opponent", 0);
    const fused = (eaten: string, kept = "ex"): GameEvent => ({ type: "fused", instanceIds: [kept, eaten], resultInstanceId: kept, defId: TAOTIE });
    const [meal, course, other] = run(TAOTIE, [fused(foe.instanceId), fused("hidden"), fused(foe.instanceId, "fx-other")], D, "p1", view, "endOfTurn");
    const maw = tid(testid.card("ex"));
    const prey = tid(testid.card(foe.instanceId));
    expect(meal?.row).toBe("replace");
    expect(shapes(meal?.cues)).toEqual(["taotieMaw", "ring", "burst", "ghost", "ring", "burst"]);
    expect(meal?.cues.slice(0, 3)).toMatchObject([{ at: maw, delayMs: 0, side: "you" }, { at: prey, delayMs: 0 }, { spread: "ring", at: prey, delayMs: 0 }]);
    const [drag, gulp, swell] = meal?.cues.slice(3) ?? [];
    expect(drag).toMatchObject({ kind: "ghost", from: prey, to: maw, spin: true });
    if (drag?.kind !== "ghost") throw new Error("no card back");
    expect(drag.delayMs).toBeGreaterThan(0);
    expect(drag.delayMs + drag.durationMs).toBeLessThan(D);
    // Its face swells as the card back reaches it.
    for (const cue of [gulp, swell]) expect(cue).toMatchObject({ at: maw, delayMs: drag.delayMs + drag.durationMs });
    // R202: the Radiant face's second course, a card the board does not show, is dragged from the opponent's deck pile.
    expect(course?.cues[3]).toMatchObject({ kind: "ghost", from: tid(animTestid.library("opponent")), to: maw });
    // A fusion it is not in is no meal of its: its row plays.
    expect(other).toEqual({ cues: [], row: undefined });
  });

  it("R3405001 YileGPT: after its takeover's hold the ink scholar steps out brush first and folds into its zone at the slam, an ink drop skittering across its lane to where its Virus boots up with its own entrance; at its Death its light folds into a card back flying to its Deck pile", () => {
    const { cues, tgt, slam } = entrance("scrollScholar", YILE, MYTHIC_D, HOLD);
    expectFrame(cues, "scrollScholar", slam, HOLD);
    expect(beats(cues)).toEqual([`figure:inkScholar${at(HOLD)}`, `burst${at(HOLD)}`, `ring${at(slam)}`, `burst${at(slam)}`, `shot${at(slam)}`, `figure:inkDrop${at(slam)}`, `shake${at(slam)}`]);
    const across = tid(testid.zone("opponent", "units", LANE));
    expect(cues[0]).toMatchObject({ at: tid(tgt), side: "you" });
    expect(cues[1]).toMatchObject({ spread: "area", at: tid(tgt, FOOT) });
    expect(cues[4]).toMatchObject({ preset: "circuit", from: tid(tgt), to: across, flightMs: MYTHIC_D - slam });
    expect(cues[5]).toMatchObject({ at: across, side: "you" });
    // Its Cry's one event, the Virus summoned for the opponent across from it, plays the Virus's own entrance.
    const virus: GameEvent = { type: "summoned", player: "p2", instanceId: "fx-virus", defId: VIRUS, row: "units", lane: LANE };
    expect(run(YILE, [virus], D)).toEqual([{ cues: entrance("virusBoot", VIRUS, D, 0, "p2").cues, row: "replace" }]);
    // Its Death, standing as "ex": its light flares on its card, whose card back flies to its Deck pile within the entry.
    const shuffled = (player: PlayerId): GameEvent => ({ type: "shuffledIn", player, instanceId: "hidden", defId: "hidden", position: 0 });
    const [death] = run(YILE, [shuffled("p1")], D, "p1", standing(), "death");
    const card = tid(testid.card("ex"));
    expect(death?.row).toBe("replace");
    expect(death?.cues).toMatchObject([
      { kind: "burst", spread: "area", at: card, delayMs: 0 },
      { kind: "ghost", from: card, to: tid(animTestid.library("you")) },
    ]);
    const back = death?.cues[1];
    if (back?.kind !== "ghost") throw new Error("no card back");
    expect(back.delayMs).toBeGreaterThan(0);
    expect(back.delayMs + back.durationMs).toBe(D);
    // Gone from the board, it leaves from its Graveyard pile, on either seat.
    expect(run(YILE, [shuffled("p1")], D, "p1", fullBoardView(), "death")[0]?.cues[1]).toMatchObject({ from: tid(animTestid.graveyard("you")), to: tid(animTestid.library("you")) });
    expect(run(YILE, [shuffled("p2")], D, "p2", fullBoardView(), "death")[0]?.cues[1]).toMatchObject({ from: tid(animTestid.graveyard("opponent")), to: tid(animTestid.library("opponent")) });
    // A shuffle into the other seat's deck is none of its: its row plays.
    expect(run(YILE, [shuffled("p2")], D, "p1", fullBoardView(), "death")).toEqual([{ cues: [], row: undefined }]);
  });

  it("R3405001 YileGPT Prime: after its takeover's hold the colossal scholar unfurls in ink, its brush ring spinning, and lands under mythic rays; its Cry's ring stops on each Datacenter it raises, each enemy card it dims and each card it adds, a card back to the hand pile with a sheen on its owner's seat alone", () => {
    const { cues, tgt, slam } = entrance("pavilionUnfurl", YILE_PRIME, MYTHIC_D, HOLD);
    expectFrame(cues, "pavilionUnfurl", slam, HOLD);
    expect(beats(cues)).toEqual([`figure:brushRing${at(HOLD)}`, `burst${at(HOLD)}`, `rays${at(slam)}`, `ring${at(slam)}`, `burst${at(slam)}`, `shake${at(slam)}`]);
    expect(cues.slice(0, 3)).toMatchObject([{ at: tid(tgt), side: "you" }, { spread: "area", at: tid(tgt, FOOT) }, { tone: "mythic", at: tid(tgt) }]);
    // Its Cry, standing as "ex": a Datacenter into its backrow, an enemy Unit dimmed, a card added to its hand.
    const view = standing();
    const foe = tid(testid.card(unitAt(view, "opponent", 0).instanceId));
    const datacenter: Extract<GameEvent, { type: "summoned" }> = { type: "summoned", player: "p1", instanceId: "fx-datacenter", defId: "classicplus-078", row: "backrow", lane: 2 };
    const dim = (card: { instanceId: string; defId: string }): GameEvent => ({ type: "degraded", instanceId: card.instanceId, defId: card.defId, change: NERF });
    const [raised, dimmed, gift, unseen] = run(YILE_PRIME, [datacenter, dim(unitAt(view, "opponent", 0)), added("p1", "fx-glitch", "classic-018"), dim({ instanceId: "hidden", defId: "hidden" })], D, "p1", view);
    const zone = tid(testid.zone("you", "backrow", 2));
    const card = tid(testid.card("ex"));
    const hand = tid(animTestid.hand("you"));
    // R3399000: the Datacenter lands by its own entrance (Classic+'s obeliskRise) before the ring stops on it.
    const obelisks = ENTRANCES.obeliskRise({ D, tgt: testid.zone("you", "backrow", 2), intensity: FX_INTENSITY_SCALE.normal, view, landing: datacenter, leadMs: 0 });
    expect(raised?.cues.slice(0, obelisks.length)).toEqual(obelisks);
    expect(raised?.cues.slice(obelisks.length)).toMatchObject([{ kind: "figure", figure: "brushRing", size: "small", at: zone, delayMs: 0 }, { kind: "burst", spread: "area", at: zone }]);
    expect(raised?.row).toBe("replace");
    expect(dimmed).toMatchObject({ cues: [{ figure: "brushRing", size: "small", at: foe }, { kind: "burst", preset: "smoke", at: foe }], row: "replace" });
    expect(gift).toMatchObject({ cues: [{ figure: "brushRing", at: card }, { kind: "ghost", from: card, to: hand }, { kind: "sheen", at: hand, delayMs: 0, durationMs: D }], row: "replace" });
    // R202: a card dimmed in a Deck or a hand this seat cannot read has no anchor: its row plays.
    expect(unseen).toEqual({ cues: [], row: undefined });
    // The opponent's Prime, off this board: the viewer's own hand card dims where it is, with no stop of the
    // ring, and its gift flies from its hero to its hand pile with no sheen here.
    const own = handCard(view);
    const [mine, theirs] = run(YILE_PRIME, [dim(own), added("p2", "hidden", "hidden")], D, "p2", fullBoardView());
    expect(mine).toEqual({ cues: [expect.objectContaining({ kind: "burst", preset: "smoke", at: tid(testid.handCard(own.instanceId)) })], row: "replace" });
    expect(shapes(theirs?.cues)).toEqual(["brushRing", "ghost"]);
    expect(theirs?.cues[1]).toMatchObject({ from: tid(testid.hero("opponent")), to: tid(animTestid.hand("opponent")) });
    // Its AI Girlfriend and the Viruses it floods the far side with enter by their own entrances.
    const girlfriend: GameEvent = { type: "summoned", player: "p1", instanceId: "fx-girlfriend", defId: GIRLFRIEND, row: "units", lane: LANE };
    const virus: GameEvent = { type: "summoned", player: "p2", instanceId: "fx-virus", defId: VIRUS, row: "units", lane: LANE };
    expect(run(YILE_PRIME, [girlfriend, virus], D)).toEqual([
      { cues: entrance("dojoBoot", GIRLFRIEND, D).cues, row: "replace" },
      { cues: entrance("virusBoot", VIRUS, D, 0, "p2").cues, row: "replace" },
    ]);
  });

  it("R3405001 Yile's Virus: after its takeover's hold pixel snow falls and its face tumbles in, booting at the slam; its start of turn zaps each Unit it hits with the hit's number, its Activate pops it, and as it dies a hand card tears away at its controller's hand pile", () => {
    const { cues, tgt, slam } = entrance("virusBoot", VIRUS, MYTHIC_D, HOLD);
    expectFrame(cues, "virusBoot", slam, HOLD);
    expect(beats(cues)).toEqual([`burst${at(HOLD)}`, `figure:virusFace${at(HOLD)}`, `ring${at(slam)}`, `burst${at(slam)}`, `shake${at(slam)}`]);
    expect(cues[1]).toMatchObject({ at: tid(tgt), side: "you" });
    // Its start of turn, standing as "ex": a zap and the hit's number on each Unit it hits; a hit from another source is not its.
    const view = standing();
    const foe = unitAt(view, "opponent", 0).instanceId;
    const zap = (sourceId: string): GameEvent => ({ type: "damage", sourceId, targetId: foe, amount: 2, combat: false });
    const [hit, other] = run(VIRUS, [zap("ex"), zap("fx-other")], D, "p1", view, "startOfTurn");
    const struck = tid(testid.card(foe));
    expect(hit?.row).toBe("replace");
    expect(shapes(hit?.cues)).toEqual(["virusZap", "burst", "splat"]);
    expect(hit?.cues[2]).toMatchObject({ kind: "splat", tone: "damage", amount: 2, at: struck, delayMs: 0 });
    expect(other).toEqual({ cues: [], row: undefined });
    // Its Activate pops it where it stands; its Death's discard zaps its controller's hand pile and flies a card back to the Graveyard.
    const card = view.you.units[LANE - 1];
    if (card === undefined || card === null) throw new Error("no Virus on the board");
    const popped: GameEvent = { type: "destroyed", instanceId: "ex", defId: VIRUS, owner: "p1", controller: "p1", attack: card.attack, maxHealth: card.maxHealth, killerId: null };
    const [pop] = run(VIRUS, [popped], D, "p1", view, "activate");
    expect(pop).toMatchObject({ cues: [{ kind: "burst", spread: "area", at: tid(testid.card("ex")) }, { kind: "ring" }], row: "replace" });
    const torn = (owner: PlayerId): GameEvent => ({ type: "discarded", instanceId: "hidden", defId: "hidden", owner });
    const [mine, theirs] = run(VIRUS, [torn("p1"), torn("p2")], D, "p1", view, "death");
    const hand = tid(animTestid.hand("you"));
    expect(mine?.cues).toMatchObject([
      { kind: "figure", figure: "virusZap", size: "small", at: hand },
      { kind: "burst", at: hand },
      { kind: "ghost", from: hand, to: tid(animTestid.graveyard("you")), delayMs: 0, durationMs: D },
    ]);
    // R202: the pile, never the card; a discard from the other seat's hand is not its.
    expect(JSON.stringify(mine)).not.toContain("hidden");
    expect(theirs).toEqual({ cues: [], row: undefined });
  });

  it("R3405001 AI Girlfriend: after her takeover's hold a wireframe dojo boots up and she bows into her zone at the slam; each blow she parries flashes a hex shield at her palm and dims its attacker", () => {
    const { cues, tgt, slam } = entrance("dojoBoot", GIRLFRIEND, MYTHIC_D, HOLD);
    expectFrame(cues, "dojoBoot", slam, HOLD);
    expect(beats(cues)).toEqual([`figure:wireDojo${at(HOLD)}`, `burst${at(HOLD)}`, `ring${at(slam)}`, `burst${at(slam)}`, `shake${at(slam)}`]);
    expect(cues[0]).toMatchObject({ at: tid(tgt), side: "you" });
    expect(cues[3]).toMatchObject({ spread: "ring", at: tid(tgt, FOOT) });
    // A parried blow, she standing as "ex": the shield at her palm, the attacker dimming.
    const view = standing();
    const attacker = unitAt(view, "opponent", 0);
    const dim = (instanceId: string, defId: string): GameEvent => ({ type: "degraded", instanceId, defId, change: NERF });
    const [parry] = run(GIRLFRIEND, [dim(attacker.instanceId, attacker.defId)], D, "p1", view);
    const palm = tid(testid.card("ex"));
    expect(parry).toMatchObject({
      cues: [
        { kind: "figure", figure: "hexShield", size: "small", at: palm, delayMs: 0 },
        { kind: "burst", spread: "point", at: palm },
        { kind: "burst", preset: "smoke", at: tid(testid.card(attacker.instanceId)) },
      ],
      row: "replace",
    });
    // Off the board, the shield flashes on the attacker; a card neither she nor the view places draws nothing.
    expect(run(GIRLFRIEND, [dim(attacker.instanceId, attacker.defId)], D, "p1", fullBoardView())[0]?.cues[0]).toMatchObject({ figure: "hexShield", at: tid(testid.card(attacker.instanceId)) });
    expect(run(GIRLFRIEND, [dim("hidden", "hidden")], D, "p1", fullBoardView())).toEqual([{ cues: [], row: undefined }]);
    // Any other event is not hers.
    expect(run(GIRLFRIEND, [added("p1", "fx-card", "core-006")], D, "p1", view)).toEqual([{ cues: [], row: undefined }]);
  });
});
