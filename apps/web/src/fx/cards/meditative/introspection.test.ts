// R3405001: Introspection's signature (fx/cards/meditative/introspection.ts), past what
// signatures.fields.test.ts holds: its play-order helpers (where a Field Spell's beat stands, and whether
// an event is the first of its type its play or hook has seen); its entrance cue by cue in Holy's look,
// every cue after its entry's lead (R3405000), the pool and breath mirrored for the landing's seat only when
// the entrance knows both the board and the landing; a mend on a card this seat cannot see glowing at the
// board's centre; another player's draw left to its own row; and its gallery fixture's two beats.

import { describe, expect, it } from "vitest";

import type { GameEvent, PlayerId, PlayerView } from "@jackioh/shared";

import { testid } from "../../../game/contract.ts";
import { baseView, card, emptySide, faceUpBackrow, fullBoardView, unit } from "../../../test/fixtures.ts";
import { run, tid } from "../../../test/signatures.ts";
import { frac } from "../../build.ts";
import { FX_CENTER, FX_HEAL_SPLAT_AT, FX_INTENSITY_SCALE, FX_SLAM_AT, FX_SPLAT_HOLD_MS } from "../../constants.ts";
import { figureTint } from "../../figures.ts";
import { FLAVOR_LOOKS } from "../../flavors.ts";
import type { FxMemory, FxPlay } from "../../types.ts";
import { ENTRANCE_TRAUMA, type CardFxPlan } from "../recipe.ts";
import type { AnimationEntry } from "../../../game/animations.ts";
import { fieldTile, firstOfPlay, introspectionFixture, poolBreath } from "./introspection.ts";

const INTROSPECTION = "meditative-007";
const D = 1000;
const LEAD = 200;
const TGT = testid.zone("you", "backrow", 1);
const NORMAL = FX_INTENSITY_SCALE.normal;
const HOLY = FLAVOR_LOOKS.Holy;
const UNCLAIMED = { cues: [], row: undefined };

const landing = (player: PlayerId): Extract<GameEvent, { type: "summoned" }> => ({ type: "summoned", player, instanceId: "ex", defId: INTROSPECTION, row: "backrow", lane: 1 });

function play(over: Partial<FxPlay> = {}): FxPlay {
  return { player: "p1", instanceId: "f1", defId: "meditative-007", castOnDraw: false, step: 1, seen: {}, ...over };
}

function plan(events: readonly GameEvent[], resolving?: FxPlay): CardFxPlan {
  const memory = { resolving: () => resolving } as FxMemory;
  return {
    entry: { events } as AnimationEntry,
    view: baseView(),
    env: { intensity: 1, card: () => undefined, memory },
    D: 1000,
  };
}

function healed(id: string): GameEvent {
  return { type: "healed", targetId: id, amount: 2 };
}

describe("R3405001 the first of its type the play has seen", () => {
  it("is the entry's own first event of the type, when the play has seen nothing", () => {
    const first = healed("u1");
    const second = healed("u2");
    const p = plan([first, second]);
    expect(firstOfPlay(first, p)).toBe(true);
    expect(firstOfPlay(second, p)).toBe(false);
  });

  it("is false for everything once the play saw the type before this entry", () => {
    const first = healed("u1");
    const p = plan([first], play({ seen: { healed: 2 } }));
    expect(firstOfPlay(first, p)).toBe(false);
  });

  it("counts each type on its own: the seen counts include the entry's own events (R202)", () => {
    const heal = healed("u1");
    const damage: GameEvent = { type: "damage", sourceId: null, targetId: "u1", amount: 1, combat: true };
    const p = plan([heal, damage], play({ seen: { healed: 1, damage: 1 } }));
    expect(firstOfPlay(heal, p)).toBe(true);
    expect(firstOfPlay(damage, p)).toBe(true);
  });
});

describe("R3405001 where a Field Spell's beat stands", () => {
  it("is its card while the board shows it face up", () => {
    const view = baseView({ you: emptySide("p1", { units: [unit("p1", { instanceId: "u1" }), null, null, null, null] }) });
    expect(fieldTile(view, play({ instanceId: "u1" }))).toEqual({ kind: "testid", testid: testid.card("u1") });
  });

  it("is else the first face-up backrow card of its definition on its side", () => {
    const back = faceUpBackrow("p1", { instanceId: "b9", defId: "meditative-007" });
    const view = baseView({ you: emptySide("p1", { backrow: [back, null, null, null, null] }) });
    expect(fieldTile(view, play({ instanceId: "gone" }))).toEqual({
      kind: "testid",
      testid: testid.zone("you", "backrow", 1),
    });
  });

  it("is never its copy in a hand: only a card on the board is its tile", () => {
    const view = baseView({ you: emptySide("p1", { hand: [card({ instanceId: "f1", defId: INTROSPECTION })] }) });
    expect(fieldTile(view, play({ instanceId: "f1" }))).toEqual({ kind: "testid", testid: testid.hero("you") });
  });

  it("is else its hero: a hidden card, or a backrow of other definitions, names nothing", () => {
    const view = baseView({
      opponent: emptySide("p2", { backrow: [faceUpBackrow("p2", { defId: "core-020" }), null, null, null, null] }),
    });
    expect(fieldTile(view, play({ player: "p2", instanceId: "gone" }))).toEqual({
      kind: "testid",
      testid: testid.hero("opponent"),
    });
  });
});

describe("R3405001 Introspection's signature, cue by cue", () => {
  it("R3405000 its still pool opens with a drip at its entry's lead and outlasts it by 300 ms; at the slam after the lead one breath of Holy light rings out, the breath lasting 300 ms past the entry", () => {
    const slam = LEAD + frac(FX_SLAM_AT, D - LEAD);
    const tint = figureTint(HOLY);
    const cues = poolBreath({ D, tgt: TGT, intensity: NORMAL, view: fullBoardView(), landing: landing("p1"), leadMs: LEAD });
    expect(cues).toEqual([
      { kind: "figure", figure: "stillPool", at: tid(TGT), tint, delayMs: LEAD, durationMs: D - LEAD + 300, side: "you" },
      { kind: "burst", preset: HOLY.particle, at: tid(TGT), delayMs: LEAD, count: 8, spread: "point", power: 0.4, colors: HOLY.colors },
      { kind: "figure", figure: "breathRing", at: tid(TGT), tint, delayMs: slam, durationMs: D - slam + 300, side: "you" },
      { kind: "ring", preset: HOLY.particle, at: tid(TGT), delayMs: slam, durationMs: expect.any(Number) as number },
      { kind: "burst", preset: HOLY.particle, at: tid(TGT), delayMs: slam, count: 24, spread: "ring", power: 1, colors: HOLY.colors },
      { kind: "shake", trauma: ENTRANCE_TRAUMA.poolBreath, delayMs: slam },
    ]);
    // With no lead its pool opens at once.
    expect(poolBreath({ D, tgt: TGT, intensity: NORMAL })[0]).toMatchObject({ delayMs: 0, durationMs: D + 300 });
  });

  it("R3405001 its pool and breath face the landing's seat only when the entrance has both the board and the landing", () => {
    const view = fullBoardView();
    const theirs = poolBreath({ D, tgt: TGT, intensity: NORMAL, view, landing: landing("p2") });
    expect([theirs[0], theirs[2]]).toMatchObject([{ side: "opponent" }, { side: "opponent" }]);
    for (const plan of [{}, { view }, { landing: landing("p2") }]) {
      const cues = poolBreath({ D, tgt: TGT, intensity: NORMAL, ...plan });
      expect(cues[0]).not.toHaveProperty("side");
      expect(cues[2]).not.toHaveProperty("side");
    }
  });

  it("R3405001 R202 a mend on a card this seat cannot see glows at the board's centre, with its number, the breath drawing in on its tile", () => {
    const centre = tid(testid.board, FX_CENTER);
    const splatAt = frac(FX_HEAL_SPLAT_AT, D);
    const [mend] = run(INTROSPECTION, [{ type: "healed", targetId: "hidden", amount: 2 }], D, "p1", fullBoardView(), "startOfTurn");
    expect(mend).toEqual({
      cues: [
        { kind: "figure", figure: "breathRing", at: tid(testid.hero("you")), tint: figureTint(HOLY), delayMs: 0, durationMs: D, side: "you", size: "small" },
        { kind: "burst", preset: HOLY.particle, at: centre, delayMs: 0, count: 10, spread: "area", power: 0.6, colors: HOLY.colors },
        { kind: "splat", tone: "heal", amount: 2, at: centre, delayMs: splatAt, durationMs: D - splatAt + FX_SPLAT_HOLD_MS },
      ],
      row: "replace",
    });
  });

  it("R2797000 only its owner's draw flies home under it: another player's draw keeps its own row", () => {
    const drawn = (player: PlayerId): GameEvent => ({ type: "drawn", player, instanceId: "hidden", defId: "hidden" });
    expect(run(INTROSPECTION, [drawn("p2")], D, "p1", fullBoardView(), "startOfTurn")).toEqual([UNCLAIMED]);
    expect(run(INTROSPECTION, [drawn("p1")], D, "p1", fullBoardView(), "startOfTurn")[0]).toMatchObject({ cues: [{ kind: "ghost" }], row: "replace" });
  });
});

describe("R3405001 Introspection's gallery fixture", () => {
  /** A board whose first hand card costs 3 and whose opponent's first unit zone is empty. */
  function board(): PlayerView {
    return baseView({
      you: emptySide("p1", { hand: [card({ instanceId: "h1", cost: 3 }), card({ instanceId: "h2", cost: 1 })], units: [unit("p1", { instanceId: "mine" }), null, null, null, null] }),
      opponent: emptySide("p2", { hand: { count: 4 }, units: [null, unit("p2", { instanceId: "foe" }), null, null, null] }),
    });
  }

  it("R3405001 for the viewer: its first hand card's gem dims by one, then a dart hits the first enemy Unit", () => {
    expect(introspectionFixture(board(), "ex", "p1")).toEqual([
      { type: "costChanged", instanceId: "h1", cost: 2 },
      { type: "damage", sourceId: "ex", targetId: "foe", amount: 1, combat: false },
    ]);
  });

  it("R3405001 for the other seat: its hand is a count, so no gem dims, and the dart hits the viewer's first Unit", () => {
    expect(introspectionFixture(board(), "ex", "p2")).toEqual([{ type: "damage", sourceId: "ex", targetId: "mine", amount: 1, combat: false }]);
  });

  it("R3405001 on an empty board with an empty hand: the dart hits the enemy hero alone", () => {
    expect(introspectionFixture(baseView(), "ex", "p1")).toEqual([{ type: "damage", sourceId: "ex", targetId: "hero-p2", amount: 1, combat: false }]);
    expect(introspectionFixture(baseView(), "ex", "p2")).toEqual([{ type: "damage", sourceId: "ex", targetId: "hero-p1", amount: 1, combat: false }]);
  });
});
