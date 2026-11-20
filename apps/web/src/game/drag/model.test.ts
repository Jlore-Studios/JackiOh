// The pure drag model beside its integration suites (`./model.ts`, S9, R384, R510, R658).
// The lifts and drops themselves are proved through `<Game/>` in `test/ux/drag-model.test.ts`
// (B34/B35), `test/ux/drag-layer.test.tsx`, `test/ux/drag-continue.test.tsx` and
// `game/activate.test.ts`; here the module's own constants and null edges are pinned, so a
// changed threshold or a lifted nothing fails colocated with the source, and so are the edges of
// a lift those suites leave open (#4038).

import type { ActionBody, ActivationView, PlayerView, Selection } from "@jackioh/shared";
import { RANDOM_ATTACK_TARGET } from "@jackioh/engine/config";
import { describe, expect, it } from "vitest";

import { baseView, card, emptySide, faceUpBackrow, heroPower, unit } from "../../test/fixtures.ts";
import { IDLE, onClickTarget, pickInPlay, type Interaction } from "../actions.ts";
import { testid, type ClickTarget } from "../contract.ts";
import { DRAG_THRESHOLD_PX, planDrag, resolveDrop } from "./model.ts";

describe("DRAG_THRESHOLD_PX", () => {
  it("keeps the press a click below eight CSS px", () => {
    expect(DRAG_THRESHOLD_PX).toBe(8);
  });
});

describe("planDrag null edges", () => {
  it("lifts nothing from an empty legal list", () => {
    expect(planDrag(baseView(), [], IDLE, { on: "hand", instanceId: "h1" })).toBeNull();
  });

  it("lifts nothing from a hero, a zone or a switch control", () => {
    const view = baseView();
    expect(planDrag(view, [], IDLE, { on: "hero", side: "you" })).toBeNull();
    expect(planDrag(view, [], IDLE, { on: "zone", side: "you", row: "units", lane: 1 })).toBeNull();
  });
});

describe("planDrag switch control", () => {
  it("lifts nothing from a switch control", () => {
    expect(
      planDrag(baseView(), [], IDLE, { on: "switch", instanceId: "u2" } as never),
    ).toBeNull();
  });
});

// The edges of a lift.

const PING: ActivationView = { ability: "ping", label: "Deal 1 damage", usesLeft: 1, usable: true };
const TURTLE: ActivationView = { ability: "turtle", label: "Tribute a Unit; deal damage", usesLeft: null, usable: true };
const MODES: ActivationView = { ability: "punish", label: "Choose one", usesLeft: 1, usable: true };

/**
 * Your hand: t1 a targeted spell, s1 a spell, h1 a unit. Your field: u1 and u2 plain units in lanes 1
 * and 2, act1 a unit with one ability (lane 3), act2 one with two (lane 4), and fs1 a face-up backrow
 * card with an ability (backrow lane 1). The opponent's units e1 and e2.
 */
function view(): PlayerView {
  return baseView({
    you: emptySide("p1", {
      hand: [
        card({ instanceId: "t1", defId: "core-013" }),
        card({ instanceId: "s1", defId: "core-005" }),
        card({ instanceId: "h1", defId: "core-008" }),
      ],
      units: [
        unit("p1", { instanceId: "u1" }),
        unit("p1", { instanceId: "u2" }),
        unit("p1", { instanceId: "act1", defId: "classicplus-076-1", activations: [PING] }),
        unit("p1", { instanceId: "act2", defId: "classic-021", activations: [PING, TURTLE] }),
        null,
      ],
      backrow: [faceUpBackrow("p1", { instanceId: "fs1", defId: "classic-020", activations: [MODES] }), null, null, null, null],
    }),
    opponent: emptySide("p2", {
      hand: { count: 3 },
      units: [unit("p2", { instanceId: "e1" }), unit("p2", { instanceId: "e2" }), null, null, null],
    }),
  });
}

const at = (instanceId: string): Selection => ({ pick: "instance", instanceId });
const heroP2: Selection = { pick: "hero", player: "p2" };
const hand = (instanceId: string): ClickTarget => ({ on: "hand", instanceId });
const yours = (instanceId: string, lane: number): ClickTarget => ({ on: "unit", instanceId, side: "you", lane });
const zone = (lane: number): ClickTarget => ({ on: "zone", side: "you", row: "units", lane });
const FS1: ClickTarget = { on: "backrow", instanceId: "fs1", side: "you", lane: 1 };

const T1_E1: ActionBody = { type: "play", instanceId: "t1", targets: [at("e1")] };
const T1_HERO: ActionBody = { type: "play", instanceId: "t1", targets: [heroP2] };
const PING_E1: ActionBody = { type: "activate", instanceId: "act1", ability: "ping", targets: [at("e1")] };
const PING_E2: ActionBody = { type: "activate", instanceId: "act1", ability: "ping", targets: [at("e2")] };

const sorted = (set: ReadonlySet<string> | undefined): string[] => [...(set ?? [])].sort();

describe("B34 a hand card's lift", () => {
  it("B34 a hand card lifts its plays alone, not another action that names it (a trade)", () => {
    const trade: ActionBody = { type: "trade", instanceId: "t1" };
    const plan = planDrag(view(), [T1_E1, T1_HERO, trade], IDLE, hand("t1"));

    expect(plan?.lifted).toEqual({ stage: "playing", instanceId: "t1", candidates: [T1_E1, T1_HERO], picked: {} });
  });

  it("R1044 a targeted spell that may also be set face-down still draws the arrow: a set play is never what a drag aims", () => {
    const set: ActionBody = { type: "play", instanceId: "t1", zone: { row: "backrow", lane: 2 }, faceDown: "endOfThisTurn" };
    const plan = planDrag(view(), [T1_E1, T1_HERO, set], IDLE, hand("t1"));

    expect(plan?.arrow).toBe(true);
    // The set play's zone still glows: a drop there sets the card.
    expect(sorted(plan?.dropTestids)).toEqual(["card-e1", "hero-opponent", "zone-you-backrow-2"]);
    expect(plan?.freeDrop).toBe(false);
  });

  it("B34 a play that lists an empty set of targets aims at nothing: a card ghost, dropped anywhere", () => {
    const plan = planDrag(view(), [{ type: "play", instanceId: "s1", targets: [] }], IDLE, hand("s1"));

    expect(plan?.arrow).toBe(false);
    expect(plan?.freeDrop).toBe(true);
  });
});

describe("R1200 an attack that names no target", () => {
  const random: ActionBody = { type: "attack", attackerId: "u1", targetId: RANDOM_ATTACK_TARGET };

  it("R1200 while a Mayor acts, an attacker whose one attack names no target lifts nothing", () => {
    expect(planDrag(view(), [random], IDLE, yours("u1", 1))).toBeNull();
  });

  it("R1200 an attacker with a target to aim at besides lifts its attack", () => {
    const plan = planDrag(view(), [random, { type: "attack", attackerId: "u1", targetId: "e1" }], IDLE, yours("u1", 1));

    expect(plan?.kind).toBe("attack");
    expect(plan?.dropTestids.has("card-e1")).toBe(true);
  });
});

describe("R510 an activation's lift", () => {
  it("R510 the lift holds the activation unsettled, naming the ability of the control pressed, and none for a control that names none", () => {
    const pings: ActionBody[] = [
      { type: "activate", instanceId: "act2", ability: "ping", targets: [at("e1")] },
      { type: "activate", instanceId: "act2", ability: "ping", targets: [at("e2")] },
    ];
    const named = planDrag(view(), pings, IDLE, { on: "activate", instanceId: "act2", ability: "ping" });
    expect(named?.lifted).toStrictEqual({ stage: "activating", instanceId: "act2", ability: "ping", candidates: pings, picked: {} });

    const plain = planDrag(view(), [PING_E1, PING_E2], IDLE, { on: "activate", instanceId: "act1" });
    expect(plain?.lifted).toStrictEqual({ stage: "activating", instanceId: "act1", candidates: [PING_E1, PING_E2], picked: {} });
  });

  it("R384 an ability whose target is optional (one listing names none) still has a target to drag to", () => {
    const untargeted: ActionBody = { type: "activate", instanceId: "act1", ability: "ping" };
    const plan = planDrag(view(), [PING_E1, untargeted], IDLE, { on: "activate", instanceId: "act1" });

    expect(plan?.kind).toBe("activate");
    expect(plan?.arrow).toBe(true);
  });

  it("R510 an aimed ability is never dropped freely, even with one target", () => {
    const plan = planDrag(view(), [PING_E1], IDLE, { on: "activate", instanceId: "act1" });

    expect(sorted(plan?.dropTestids)).toEqual(["card-e1"]);
    expect(plan?.freeDrop).toBe(false);
  });

  it("R658 an unaimed backrow ability with a Tribute to choose glows the Tributes and must drop on one; with one Tribute it drops anywhere", () => {
    const tribute = (id: string): ActionBody => ({ type: "activate", instanceId: "fs1", ability: "punish", tributes: [id] });

    const two = planDrag(view(), [tribute("u1"), tribute("u2")], IDLE, FS1);
    expect(two?.arrow).toBe(false);
    expect(sorted(two?.dropTestids)).toEqual(["card-u1", "card-u2"]);
    expect(two?.freeDrop).toBe(false);

    const one = planDrag(view(), [tribute("u1")], IDLE, FS1);
    expect(sorted(one?.dropTestids)).toEqual(["card-u1"]);
    expect(one?.freeDrop).toBe(true);
  });

  it("R658 a free drop of an unaimed ability with a mode to choose leaves the mode to the picker, picking nothing else", () => {
    const modes: ActionBody[] = [
      { type: "activate", instanceId: "fs1", ability: "punish", modes: ["discard"] },
      { type: "activate", instanceId: "fs1", ability: "punish", modes: ["draw"] },
    ];
    const plan = planDrag(view(), modes, IDLE, FS1);
    if (plan === null) throw new Error("no plan");

    const dropped = resolveDrop(view(), modes, plan, { at: "board" });
    expect(dropped).toEqual(pickInPlay(plan.lifted, {}));
    expect(dropped.interaction.stage === "activating" && dropped.interaction.picked).toEqual({});
  });

  it("R510 a Heroic Power's activation is dragged from the power button it was pressed on", () => {
    const base = view();
    const withPower: PlayerView = { ...base, you: { ...base.you, hero: { ...base.you.hero, power: heroPower, powers: [heroPower] } } };
    const legal: ActionBody[] = [
      { type: "activatePower", instanceId: heroPower.instanceId, targets: [at("e1")] },
      { type: "activatePower", instanceId: heroPower.instanceId, targets: [heroP2] },
    ];
    const plan = planDrag(withPower, legal, IDLE, { on: "activate", instanceId: heroPower.instanceId });

    expect(plan?.sourceTestid).toBe(testid.power);
    expect(plan?.arrow).toBe(true);
  });

  it("R510 an Activate control that nothing lists lifts nothing, nor does a switch control on a card with an ability", () => {
    expect(planDrag(view(), [], IDLE, { on: "activate", instanceId: "act1" })).toBeNull();
    expect(planDrag(view(), [PING_E1, PING_E2], IDLE, { on: "switch", instanceId: "act1" })).toBeNull();
  });
});

describe("R658 a build lifted again from its picks", () => {
  it("R658 a press with no build in flight lifts the source as ever", () => {
    expect(planDrag(view(), [T1_E1, T1_HERO], IDLE, hand("t1"), "hand-card-t1")?.lifted).toEqual({
      stage: "playing",
      instanceId: "t1",
      candidates: [T1_E1, T1_HERO],
      picked: {},
    });
  });

  it("R658 an activation with nothing picked yet is lifted afresh from its control, not again", () => {
    const legal = [PING_E1, PING_E2];
    const build = onClickTarget(view(), legal, IDLE, { on: "activate", instanceId: "act1" }).interaction;
    expect(build.stage).toBe("activating");

    const plan = planDrag(view(), legal, build, { on: "activate", instanceId: "act1" }, testid.activate("act1"));
    expect(plan?.kind).toBe("activate");
    expect(plan?.lifted).not.toBe(build);
    expect(plan?.missed).toBeUndefined();
  });

  it("R658 a play placed in its zone is lifted again from it, as from its hand card, onto its targets", () => {
    const legal: ActionBody[] = [
      { type: "play", instanceId: "h1", zone: { row: "units", lane: 5 }, targets: [at("e1")] },
      { type: "play", instanceId: "h1", zone: { row: "units", lane: 5 }, targets: [heroP2] },
    ];
    const placed = onClickTarget(view(), legal, onClickTarget(view(), legal, IDLE, hand("h1")).interaction, zone(5)).interaction;
    expect(placed.stage === "playing" && placed.picked.zone).toEqual({ row: "units", lane: 5 });

    const plan = planDrag(view(), legal, placed, zone(5), "zone-you-units-5");
    expect(plan?.kind).toBe("play");
    expect(plan?.source).toStrictEqual({ on: "hand", instanceId: "h1" });
    expect(plan?.lifted).toBe(placed);
  });

  it("R658 a play whose picks leave only a picker's choice, nothing on the board, is not lifted again", () => {
    const zoneX = (lane: number, x: number): ActionBody => ({ type: "play", instanceId: "h1", zone: { row: "units", lane }, x });
    const legal = [zoneX(5, 1), zoneX(5, 2), zoneX(4, 1)];
    const placed = onClickTarget(view(), legal, onClickTarget(view(), legal, IDLE, hand("h1")).interaction, zone(5)).interaction;
    expect(placed.stage === "playing" && placed.picked.zone).toEqual({ row: "units", lane: 5 });

    expect(planDrag(view(), legal, placed, zone(5), "zone-you-units-5")).toBeNull();
  });

  it("R658 a build with nothing left to choose is not lifted again, though its target still glows", () => {
    const only: ActionBody = { type: "play", instanceId: "h1", zone: { row: "units", lane: 5 }, targets: [at("e1")] };
    const oneLeft: Interaction = { stage: "playing", instanceId: "h1", candidates: [only], picked: { zone: { row: "units", lane: 5 } } };

    expect(planDrag(view(), [only], oneLeft, zone(5), "zone-you-units-5")).toBeNull();
  });

  /** An activation of `instanceId`'s, from the control naming `ability`, its Tribute u1 picked and its target still to pick. */
  function tributePicked(instanceId: string, ability: string | undefined): { legal: ActionBody[]; build: Interaction } {
    const body = (tribute: string, target: string): ActionBody => ({
      type: "activate",
      instanceId,
      ability: ability ?? "ping",
      tributes: [tribute],
      targets: [at(target)],
    });
    const legal = [body("u1", "e1"), body("u1", "e2"), body("u2", "e1")];
    const control: ClickTarget = ability === undefined ? { on: "activate", instanceId } : { on: "activate", instanceId, ability };
    const opened = onClickTarget(view(), legal, IDLE, control).interaction;
    return { legal, build: onClickTarget(view(), legal, opened, yours("u1", 1)).interaction };
  }

  it("R658 an activation whose Tribute is picked is lifted again from it, as from its own control, onto its targets", () => {
    for (const [instanceId, ability] of [["act2", "turtle"], ["act1", undefined]] as const) {
      const { legal, build } = tributePicked(instanceId, ability);
      expect(build.stage === "activating" && build.picked.tributes).toEqual(["u1"]);

      const plan = planDrag(view(), legal, build, yours("u1", 1), "card-u1");
      expect(plan?.kind).toBe("activate");
      expect(plan?.source).toStrictEqual(ability === undefined ? { on: "activate", instanceId } : { on: "activate", instanceId, ability });
      expect(plan?.sourceTestid).toBe("card-u1");
      expect(plan?.lifted).toBe(build);
      expect(plan?.missed).toBe(build);
      expect(sorted(plan?.dropTestids)).toEqual(["card-e1", "card-e2"]);
      expect(plan?.arrow).toBe(true);
      expect(plan?.freeDrop).toBe(false);
    }
  });
});
