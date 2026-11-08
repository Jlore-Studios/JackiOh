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
import { HOLD, beats, run, tid } from "../../../test/signatures.ts";
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
    const own = Array.isArray(view.you.hand) ? view.you.hand[0] : undefined;
    if (own === undefined) throw new Error("no hand card");
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
});
