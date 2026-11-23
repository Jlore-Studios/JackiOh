// R3405001: Keymaster Keenus' signature (keymasterKeenus.ts), cue by cue, past what signatures.units-a.test.ts
// holds: his entrance's key ring, ring, jangle, shake, key turn and clicks in Physical's steel look, every one
// after his entry's lead (R3405000), the ring and the turn mirrored for the landing's seat only when the
// entrance knows both the board and the landing; each click at its lock on the arc over him; his Equip's keys
// over the Unit he fuses into (or his hero, off the board), under his play alone (a fusion under a hook of his,
// one he is no part of, or another event is none of his); his gallery fixture's fusion into the first Unit; and
// the look his module builds as it loads.

import type { GameEvent, PlayerId, PlayerView } from "@jackioh/shared";
import { describe, expect, it, vi } from "vitest";

import { testid } from "../../../game/contract.ts";
import { baseView, emptySide, fullBoardView, unit } from "../../../test/fixtures.ts";
import { run, tid } from "../../../test/signatures.ts";
import { frac } from "../../build.ts";
import { FX_INTENSITY_SCALE, FX_SLAM_AT } from "../../constants.ts";
import { figureTint } from "../../figures.ts";
import { FLAVOR_LOOKS, paletteLook, type FlavorLook } from "../../flavors.ts";
import type { FxCue } from "../../types.ts";
import { ENTRANCE_TRAUMA } from "../recipe.ts";
import { KEENUS_KEYS, keenusKeysFixture, keyRing } from "./keymasterKeenus.ts";

const KEENUS = "meditative-065";
const D = 1000;
const LEAD = 200;
const LANE = 3;
const TGT = testid.zone("you", "units", LANE);
const NORMAL = FX_INTENSITY_SCALE.normal;
const UNCLAIMED = { cues: [], row: undefined };

/** His five locks, left to right along the arc over his box: `top` at its middle, `dip` lower at its ends. */
const LOCKS = [
  { x: 0.22, y: 0.157632 },
  { x: 0.36, y: 0.129408 },
  { x: 0.5, y: 0.12 },
  { x: 0.64, y: 0.129408 },
  { x: 0.78, y: 0.157632 },
];

function lookOf(palette: string): FlavorLook {
  const look = paletteLook(palette);
  if (look === undefined) throw new Error(`no look ${palette}`);
  return look;
}

/** Physical's steel sub-look, his aura's: not Physical's own. */
const STEEL = lookOf("Physical/steel");

const landing = (player: PlayerId): Extract<GameEvent, { type: "summoned" }> => ({ type: "summoned", player, instanceId: "ex", defId: KEENUS, row: "units", lane: LANE });
const fused = (host: string, instanceIds: readonly string[] = [host, "ex"]): GameEvent => ({ type: "fused", instanceIds: [...instanceIds], resultInstanceId: host, defId: "core-004" });

/** The clicks among `cues`, each checked as one key's spark: where it clicks and when. */
function clicksOf(cues: readonly FxCue[], box: string): { x: number; y: number; delayMs: number }[] {
  return cues.flatMap((cue) => {
    if (cue.kind !== "burst" || cue.preset !== "spark") return [];
    expect(cue).toMatchObject({ count: 4, spread: "point", power: 0.5, colors: STEEL.colors, at: { kind: "testid", testid: box } });
    const point = cue.at.kind === "testid" ? cue.at.at : undefined;
    return [{ x: point?.x ?? Number.NaN, y: point?.y ?? Number.NaN, delayMs: cue.delayMs }];
  });
}

/** Every click at its lock, at its delay. */
function expectLocks(clicks: readonly { x: number; y: number; delayMs: number }[], delays: readonly number[]): void {
  expect(clicks).toHaveLength(LOCKS.length);
  clicks.forEach((click, k) => {
    expect(click.x).toBeCloseTo(LOCKS[k]?.x ?? Number.NaN, 9);
    expect(click.y).toBeCloseTo(LOCKS[k]?.y ?? Number.NaN, 9);
    expect(click.delayMs).toBe(delays[k]);
  });
}

describe("R3405001 Keymaster Keenus' signature, cue by cue", () => {
  it("R3405000 his key ring spins in from his entry's lead and outlasts it by 300 ms; at the slam after the lead it rings and jangles, and his keys turn, clicking in a 30% share of the motion from 5% past the slam, all in steel", () => {
    const span = D - LEAD;
    const slam = LEAD + frac(FX_SLAM_AT, span);
    const tint = figureTint(STEEL);
    const cues = keyRing({ D, tgt: TGT, intensity: NORMAL, view: fullBoardView(), landing: landing("p1"), leadMs: LEAD });
    expect(cues.slice(0, 5)).toEqual([
      { kind: "figure", figure: "keyRing", at: tid(TGT), tint, delayMs: LEAD, durationMs: span + 300, side: "you" },
      { kind: "ring", preset: STEEL.particle, at: tid(TGT), delayMs: slam, durationMs: expect.any(Number) as number },
      { kind: "burst", preset: STEEL.particle, at: tid(TGT), delayMs: slam, count: 16, spread: "ring", power: 0.9, colors: STEEL.colors },
      { kind: "shake", trauma: ENTRANCE_TRAUMA.keyRing, delayMs: slam },
      { kind: "figure", figure: "keyTurn", at: tid(TGT), tint, delayMs: slam, durationMs: D - slam + 300, side: "you" },
    ]);
    expect(STEEL.colors).not.toEqual(FLAVOR_LOOKS.Physical.colors);
    // He is not on this board, so he turns his base face's KEENUS_KEYS.
    const from = slam + frac(0.05, span);
    const share = frac(0.3, span);
    expect(KEENUS_KEYS).toBe(LOCKS.length);
    expectLocks(
      clicksOf(cues.slice(5), TGT),
      LOCKS.map((_, k) => from + Math.round((k * share) / LOCKS.length)),
    );
  });

  it("R3405001 with no board and no landing his ring spins in at once and he still turns his KEENUS_KEYS", () => {
    const cues = keyRing({ D, tgt: TGT, intensity: NORMAL });
    expect(cues[0]).toMatchObject({ delayMs: 0, durationMs: D + 300 });
    expect(clicksOf(cues, TGT)).toHaveLength(KEENUS_KEYS);
  });

  it("R3405001 he turns one key per keyword his unit shows on the board, his Equip aside, past the empty zones before him", () => {
    const keywords = [{ kind: "Rush" }, { kind: "Taunt" }, { kind: "Equip" }] as const;
    const view = baseView({ you: emptySide("p1", { units: [null, unit("p1", { instanceId: "ex", defId: KEENUS, keywords: [...keywords] }), null, null, null] }) });
    expect(clicksOf(keyRing({ D, tgt: TGT, intensity: NORMAL, view, landing: landing("p1") }), TGT)).toHaveLength(2);
  });

  it("R3405001 his ring and his key turn face the landing's seat only when the entrance has both the board and the landing", () => {
    const view = fullBoardView();
    const theirs = keyRing({ D, tgt: TGT, intensity: NORMAL, view, landing: landing("p2") });
    expect([theirs[0], theirs[4]]).toMatchObject([{ side: "opponent" }, { side: "opponent" }]);
    for (const plan of [{}, { view }, { landing: landing("p2") }]) {
      const cues = keyRing({ D, tgt: TGT, intensity: NORMAL, ...plan });
      expect(cues[0]).not.toHaveProperty("side");
      expect(cues[4]).not.toHaveProperty("side");
    }
  });

  it("R3405001 his Equip turns his keys over the Unit he fuses into, 300 ms past the entry, clicking at 150, 230, 310, 390 and 470 ms; at his hero when the board does not show it", () => {
    const view = fullBoardView();
    const host = view.you.units[0]?.instanceId ?? "";
    const box = testid.card(host);
    const [equip] = run(KEENUS, [fused(host)], D, "p1", view);
    expect(equip?.row).toBe("replace");
    expect(equip?.cues[0]).toEqual({ kind: "figure", figure: "keyTurn", at: tid(box), tint: figureTint(STEEL), delayMs: 0, durationMs: D + 300, side: "you" });
    expectLocks(clicksOf(equip?.cues ?? [], box), [150, 230, 310, 390, 470]);
    const [unseen] = run(KEENUS, [fused("hidden")], D, "p1", view);
    expect(unseen?.cues[0]).toMatchObject({ figure: "keyTurn", at: tid(testid.hero("you")) });
  });

  it("R2797000 only a fusion of his own under his play is his Equip's: one under a hook of his, one he is no part of, and another event keep their own rows", () => {
    const view = fullBoardView();
    const host = view.you.units[0]?.instanceId ?? "";
    const locked: GameEvent = { type: "locked", player: "p1", row: "units", lane: 1 };
    expect(run(KEENUS, [fused(host, [host, "other"]), locked], D, "p1", view)).toEqual([UNCLAIMED, UNCLAIMED]);
    for (const hook of ["death", "activate"] as const) expect(run(KEENUS, [fused(host)], D, "p1", view, hook)).toEqual([UNCLAIMED]);
  });

  it("R3405001 his gallery fixture fuses him into the playing seat's first Unit, and into none on an empty board", () => {
    const view: PlayerView = baseView({
      you: emptySide("p1", { units: [null, unit("p1", { instanceId: "mine", defId: "core-011" }), null, null, null] }),
      opponent: emptySide("p2", { hand: { count: 4 }, units: [null, null, unit("p2", { instanceId: "foe", defId: "core-008" }), null, null] }),
    });
    expect(keenusKeysFixture(view, "ex", "p1")).toEqual([{ type: "fused", instanceIds: ["mine", "ex"], resultInstanceId: "mine", defId: "core-011" }]);
    expect(keenusKeysFixture(view, "ex", "p2")).toEqual([{ type: "fused", instanceIds: ["foe", "ex"], resultInstanceId: "foe", defId: "core-008" }]);
    expect(keenusKeysFixture(baseView(), "ex", "p1")).toEqual([]);
  });
});

describe("R3405001 Keymaster Keenus' signature as a page load builds it", () => {
  it("R3405001 his look is the steel sub-look", async () => {
    // His look is read as the module loads: read from a module loaded inside the test, as scornboi.test.ts does.
    vi.resetModules();
    const fresh = await import("./keymasterKeenus.ts");
    const cues = fresh.keyRing({ D, tgt: TGT, intensity: NORMAL });
    expect(cues[0]).toMatchObject({ figure: "keyRing", tint: figureTint(STEEL) });
    expect(cues[2]).toMatchObject({ kind: "burst", preset: STEEL.particle, colors: STEEL.colors });
  });
});
