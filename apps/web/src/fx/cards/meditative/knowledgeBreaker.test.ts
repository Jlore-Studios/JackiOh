// R3405001: Knowledge Breaker's signature (knowledgeBreaker.ts), cue by cue, past what
// signatures.units-b.test.ts holds: where the card a batch resolves for stands (its card on the board, else
// the zone its own landing named, else its hero); his entrance's chalk puff, mask, ring, chalk ring and shake in
// Nerd's chalk look, every one after his entry's lead (R3405000), the mask mirrored for the landing's seat only
// when the entrance knows both the board and the landing; his Cry's Nerf and his Death's card back, each cue by
// cue, under his play and his Death alone; his gallery fixture's Nerfs; and the look and Nerf his module builds
// as it loads.

import type { GameEvent, PlayerId, PlayerView } from "@jackioh/shared";
import { describe, expect, it, vi } from "vitest";

import { animTestid, type AnimationEntry } from "../../../game/animations.ts";
import { testid } from "../../../game/contract.ts";
import { baseView, card, emptySide, fullBoardView, unit } from "../../../test/fixtures.ts";
import { envOf, played, run, tid } from "../../../test/signatures.ts";
import { frac } from "../../build.ts";
import { FX_ARROWS_TAIL_MS, FX_INTENSITY_SCALE, FX_SLAM_AT } from "../../constants.ts";
import { figureTint } from "../../figures.ts";
import { FLAVOR_LOOKS, paletteLook, type FlavorLook } from "../../flavors.ts";
import type { FxPlay } from "../../types.ts";
import { ENTRANCE_TRAUMA, FOOT, type CardFxPlan } from "../recipe.ts";
import { chalkMask, knowledgeBreakerFixture, standingAt } from "./knowledgeBreaker.ts";

const BREAKER = "meditative-045";
const D = 1000;
const LEAD = 200;
const LANE = 3;
const TGT = testid.zone("you", "units", LANE);
const NORMAL = FX_INTENSITY_SCALE.normal;
const UNCLAIMED = { cues: [], row: undefined };
const NERF = { kind: "stats", attack: -1, health: -1 } as const;

function lookOf(palette: string): FlavorLook {
  const look = paletteLook(palette);
  if (look === undefined) throw new Error(`no look ${palette}`);
  return look;
}

/** Nerd's chalk sub-look: not Nerd's own. */
const CHALK = lookOf("Nerd/chalk");

const landing = (player: PlayerId, instanceId = "ex", lane = LANE): Extract<GameEvent, { type: "summoned" }> => ({ type: "summoned", player, instanceId, defId: BREAKER, row: "units", lane });
const nerf = (instanceId: string, defId: string): GameEvent => ({ type: "degraded", instanceId, defId, change: NERF });
const shuffled = (player: PlayerId): GameEvent => ({ type: "shuffledIn", player, instanceId: "hidden", defId: "hidden", position: 0 });

/** The board with him standing as "ex" in your lane LANE: the card itself on the board. */
function standing(): PlayerView {
  const view = fullBoardView();
  return { ...view, you: { ...view.you, units: view.you.units.map((shown, k) => (k === LANE - 1 && shown !== null ? { ...shown, instanceId: "ex" } : shown)) } };
}

/** The opponent's Unit in board slot `k`. */
function foeAt(view: PlayerView, k: number): { instanceId: string; defId: string } {
  const foe = view.opponent.units[k];
  if (foe === undefined || foe === null) throw new Error(`no foe in slot ${String(k)}`);
  return foe;
}

describe("R3405001 where the card a batch resolves for stands", () => {
  const play: FxPlay = { player: "p1", instanceId: "ex", defId: BREAKER, castOnDraw: false, step: 1, seen: {} };

  /** A plan over `view` whose memory has seen `events`, oldest first. */
  function planAfter(view: PlayerView, events: readonly GameEvent[]): CardFxPlan {
    const env = envOf();
    env.memory.remember([...events]);
    return { entry: { events: [] } as unknown as AnimationEntry, view, env, D };
  }

  it("is its card while the board's view shows it", () => {
    expect(standingAt(planAfter(standing(), []), play)).toEqual(tid(testid.card("ex")));
  });

  it("is else the zone its own landing named, never another card's landing nor its play", () => {
    const events = [played("p1", "ex", BREAKER), landing("p1", "other", 1), landing("p1")];
    expect(standingAt(planAfter(fullBoardView(), events), play)).toEqual(tid(testid.zone("you", "units", LANE)));
  });

  it("is else its hero", () => {
    expect(standingAt(planAfter(fullBoardView(), [played("p1", "ex", BREAKER), landing("p1", "other", 1)]), play)).toEqual(tid(testid.hero("you")));
  });
});

describe("R3405001 Knowledge Breaker's signature, cue by cue", () => {
  it("R3405000 chalk puffs at his foot at his entry's lead and his mask turns from it, outlasting it by 300 ms; at the slam after the lead chalk rings out from his zone, all in chalk", () => {
    const slam = LEAD + frac(FX_SLAM_AT, D - LEAD);
    const cues = chalkMask({ D, tgt: TGT, intensity: NORMAL, view: fullBoardView(), landing: landing("p1"), leadMs: LEAD });
    expect(cues).toEqual([
      { kind: "burst", preset: CHALK.particle, at: tid(TGT, FOOT), delayMs: LEAD, count: 20, spread: "area", power: 0.8, colors: CHALK.colors },
      { kind: "figure", figure: "porcelainMask", at: tid(TGT), tint: figureTint(CHALK), delayMs: LEAD, durationMs: D - LEAD + 300, side: "you" },
      { kind: "ring", preset: CHALK.particle, at: tid(TGT), delayMs: slam, durationMs: expect.any(Number) as number },
      { kind: "burst", preset: CHALK.particle, at: tid(TGT), delayMs: slam, count: 16, spread: "ring", power: 1, colors: CHALK.colors },
      { kind: "shake", trauma: ENTRANCE_TRAUMA.chalkMask, delayMs: slam },
    ]);
    expect(CHALK.colors).not.toEqual(FLAVOR_LOOKS.Nerd.colors);
    // With no lead his mask turns at once.
    expect(chalkMask({ D, tgt: TGT, intensity: NORMAL })[1]).toMatchObject({ delayMs: 0, durationMs: D + 300 });
  });

  it("R3405001 his mask faces the landing's seat only when the entrance has both the board and the landing", () => {
    const view = fullBoardView();
    expect(chalkMask({ D, tgt: TGT, intensity: NORMAL, view, landing: landing("p2") })[1]).toMatchObject({ side: "opponent" });
    for (const plan of [{}, { view }, { landing: landing("p2") }]) {
      expect(chalkMask({ D, tgt: TGT, intensity: NORMAL, ...plan })[1]).not.toHaveProperty("side");
    }
  });

  it("R3405001 his Cry's Nerf: a chalk tendril 250 ms past the entry, arrows down for its arrows' tail past it, and a lash of chalk, all on the card", () => {
    const view = fullBoardView();
    const foe = foeAt(view, 0);
    const at = tid(testid.card(foe.instanceId));
    const [lash] = run(BREAKER, [nerf(foe.instanceId, foe.defId)], D, "p1", view);
    expect(lash).toEqual({
      cues: [
        { kind: "figure", figure: "chalkTendril", at, tint: figureTint(CHALK), delayMs: 0, durationMs: D + 250, side: "you" },
        { kind: "arrows", direction: "down", at, delayMs: 0, durationMs: D + FX_ARROWS_TAIL_MS },
        { kind: "burst", preset: CHALK.particle, at, delayMs: 0, count: 6, spread: "point", power: 0.7, colors: CHALK.colors },
      ],
      row: "replace",
    });
  });

  it("R3405001 his Death: chalk gathers where his card stood and its card back flies into his owner's deck pile", () => {
    const from = tid(testid.card("ex"));
    const [death] = run(BREAKER, [shuffled("p1")], D, "p1", standing(), "death");
    expect(death).toEqual({
      cues: [
        { kind: "burst", preset: CHALK.particle, at: from, delayMs: 0, count: 10, spread: "area", power: 0.6, colors: CHALK.colors },
        { kind: "ghost", from, to: tid(animTestid.library("you")), delayMs: 0, durationMs: D },
      ],
      row: "replace",
    });
  });

  it("R2797000 his Death draws its shuffle alone: a Nerf or a hit under it keeps its own row", () => {
    const view = standing();
    const foe = foeAt(view, 0);
    const hit: GameEvent = { type: "damage", sourceId: "ex", targetId: foe.instanceId, amount: 1, combat: false };
    expect(run(BREAKER, [nerf(foe.instanceId, foe.defId), hit], D, "p1", view, "death")).toEqual([UNCLAIMED, UNCLAIMED]);
  });
});

describe("R3405001 Knowledge Breaker's gallery fixture", () => {
  /** The viewer's hand holds one card; the opponent's first unit zone is empty. */
  function board(): PlayerView {
    return baseView({
      you: emptySide("p1", { hand: [card({ instanceId: "h1", defId: "core-002" })], units: [unit("p1", { instanceId: "mine", defId: "core-011" }), null, null, null, null] }),
      opponent: emptySide("p2", { hand: { count: 4 }, units: [null, unit("p2", { instanceId: "foe", defId: "core-008" }), null, null, null] }),
    });
  }

  it("R3405001 for the viewer: his Cry Nerfs the first enemy Unit and the viewer's own first hand card", () => {
    expect(knowledgeBreakerFixture(board(), "ex", "p1")).toEqual([
      { type: "degraded", instanceId: "foe", defId: "core-008", change: NERF },
      { type: "degraded", instanceId: "h1", defId: "core-002", change: NERF },
    ]);
  });

  it("R3405001 for the other seat: his Cry Nerfs the viewer's first Unit, and no hand card, which that seat could not read", () => {
    expect(knowledgeBreakerFixture(board(), "ex", "p2")).toEqual([{ type: "degraded", instanceId: "mine", defId: "core-011", change: NERF }]);
  });

  it("R3405001 on an empty board with an empty hand it Nerfs nothing", () => {
    expect(knowledgeBreakerFixture(baseView(), "ex", "p1")).toEqual([]);
  });
});

describe("R3405001 Knowledge Breaker's signature as a page load builds it", () => {
  it("R3405001 his look is the chalk sub-look and his fixture's Nerf takes one attack and one health", async () => {
    // His look and his fixture's Nerf are built as the module loads: read from a module loaded inside the test, as scornboi.test.ts does.
    vi.resetModules();
    const fresh = await import("./knowledgeBreaker.ts");
    expect(fresh.chalkMask({ D, tgt: TGT, intensity: NORMAL })[1]).toMatchObject({ figure: "porcelainMask", tint: figureTint(CHALK) });
    expect(fresh.knowledgeBreakerFixture(board(), "ex", "p2")).toEqual([{ type: "degraded", instanceId: "mine", defId: "core-011", change: NERF }]);
  });

  function board(): PlayerView {
    return baseView({ you: emptySide("p1", { units: [unit("p1", { instanceId: "mine", defId: "core-011" }), null, null, null, null] }) });
  }
});
