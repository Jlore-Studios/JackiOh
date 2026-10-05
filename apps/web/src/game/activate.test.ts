// The client half of Activate (B3.2, R384; presentation R510), the play's new payments (B5 E5's
// discards, E11/E19's Plague Counters), a Tribute onto its own zone (B4.5, R391) and plays from the
// graveyard (B5 E11), as `actions.ts` builds them. Every test feeds a hand-built `legal` array and
// checks that what comes out was read from that array and nothing else (CLAUDE.md rule 7): no
// count of uses, no check of a cost, no zone arithmetic.

import type { ActionBody, ActivationView, PlayerView, Selection } from "@jackioh/shared";
import { describe, expect, it } from "vitest";

import {
  IDLE,
  activationControlTestids,
  activationsFor,
  highlightFor,
  isBuilding,
  onClickTarget,
  onControl,
  outstandingNeed,
  pickInPlay,
  playSourceTestid,
  plagueKey,
  type Interaction,
} from "./actions.ts";
import { namedAbility, testid } from "./contract.ts";
import { planDrag, resolveDrop } from "./drag/model.ts";
import { baseView, card, emptySide, faceUpBackrow, heroPower, unit } from "../test/fixtures.ts";

// ---------------------------------------------------------------------------------------------
// Fixtures
// ---------------------------------------------------------------------------------------------

const PING: ActivationView = { ability: "ping", label: "Deal 1 damage", usesLeft: 1, usable: true };
const MODES: ActivationView = { ability: "punish", label: "Choose one", usesLeft: 1, usable: true };
const TURTLE: ActivationView = { ability: "turtle", label: "Tribute a Unit; deal damage", usesLeft: null, usable: true };

/**
 * Your side: a unit `act1` with one Activate ability, a unit `act2` with two, a face-up Field Spell
 * `fs1` with an ability; `u1`, `u2` plain units; two hand cards; a graveyard card `gy1`. The
 * opponent has `e1` and `e2`.
 */
function activateView(over: Partial<PlayerView> = {}): PlayerView {
  return baseView({
    you: emptySide("p1", {
      hand: [card({ instanceId: "h1", defId: "core-002" }), card({ instanceId: "h2", defId: "core-019" })],
      units: [
        unit("p1", { instanceId: "act1", defId: "classicplus-076-1", activations: [PING] }),
        unit("p1", { instanceId: "act2", defId: "classic-021", activations: [PING, TURTLE] }),
        unit("p1", { instanceId: "u1" }),
        unit("p1", { instanceId: "u2" }),
        null,
      ],
      backrow: [faceUpBackrow("p1", { instanceId: "fs1", defId: "classic-020", activations: [MODES] }), null, null, null, null],
      graveyard: [card({ instanceId: "gy1", defId: "core-004" }), card({ instanceId: "gy2", defId: "core-008" })],
    }),
    opponent: emptySide("p2", {
      hand: { count: 3 },
      units: [unit("p2", { instanceId: "e1" }), unit("p2", { instanceId: "e2" }), null, null, null],
    }),
    ...over,
  });
}

const at = (instanceId: string): Selection => ({ pick: "instance", instanceId });
const heroP2: Selection = { pick: "hero", player: "p2" };

const PING_E1: ActionBody = { type: "activate", instanceId: "act1", ability: "ping", targets: [at("e1")] };
const PING_E2: ActionBody = { type: "activate", instanceId: "act1", ability: "ping", targets: [at("e2")] };
const PING_FACE: ActionBody = { type: "activate", instanceId: "act1", ability: "ping", targets: [heroP2] };

function activating(over: Partial<Extract<Interaction, { stage: "activating" }>> & { candidates: ActionBody[] }): Interaction {
  return { stage: "activating", instanceId: "act1", picked: {}, ...over };
}

// ---------------------------------------------------------------------------------------------
// The control and the highlight
// ---------------------------------------------------------------------------------------------

describe("R384 the Activate control is lit by the activations legalActions lists, and by nothing else", () => {
  it("R384 a listed activate lights and glows its card's control; an unlisted card's control is dark", () => {
    const view = activateView();
    const legal: ActionBody[] = [PING_E1, { type: "activate", instanceId: "fs1", ability: "punish", modes: ["discard"] }, { type: "endTurn" }];

    const { legal: lit, glow } = highlightFor(view, legal, IDLE);

    expect(lit.has(testid.activate("act1"))).toBe(true);
    expect(glow?.has(testid.activate("act1"))).toBe(true);
    expect(lit.has(testid.activate("fs1"))).toBe(true);
    // act2 lists two abilities, but legal names neither: neither of its controls lights.
    expect(lit.has(testid.activate("act2", "ping"))).toBe(false);
    expect(lit.has(testid.activate("act2", "turtle"))).toBe(false);
    // An activation is a move left, so End turn does not glow.
    expect(glow?.has(testid.endTurn) ?? false).toBe(false);
  });

  it("R510 a card listing several abilities names the ability in its control's testid; one listing one does not", () => {
    const view = activateView();
    const legal: ActionBody[] = [
      { type: "activate", instanceId: "act2", ability: "turtle", tributes: ["u1"], targets: [at("e1")] },
    ];

    const { legal: lit } = highlightFor(view, legal, IDLE);

    expect(lit.has(testid.activate("act2", "turtle"))).toBe(true);
    expect(lit.has(testid.activate("act2", "ping"))).toBe(false);
    expect(lit.has(testid.activate("act2"))).toBe(false);
    expect(namedAbility(1, "ping")).toBeUndefined();
    expect(namedAbility(2, "ping")).toBe("ping");
  });

  it("R384 nothing lights when legal lists no activation, whatever the view says is usable", () => {
    const view = activateView();
    const { legal: lit, glow } = highlightFor(view, [{ type: "endTurn" }], IDLE);

    expect(lit.has(testid.activate("act1"))).toBe(false);
    expect(lit.has(testid.activate("fs1"))).toBe(false);
    expect(glow?.has(testid.endTurn)).toBe(true);
  });

  it("R510 a Heroic Power's activatePower lights `power` for the first power and `power-<id>` for a further one", () => {
    const second = { ...heroPower, instanceId: "power-2", name: "Draw" };
    const view = activateView({
      you: { ...activateView().you, hero: { health: 30, armor: 0, powers: [heroPower, second], power: heroPower } },
    });
    const legal: ActionBody[] = [
      { type: "activatePower", instanceId: "power-1" },
      { type: "activatePower", instanceId: "power-2" },
    ];

    const { legal: lit, glow } = highlightFor(view, legal, IDLE);

    expect(lit.has(testid.power)).toBe(true);
    expect(lit.has(testid.powerOf("power-2"))).toBe(true);
    expect(glow?.has(testid.powerOf("power-2"))).toBe(true);
    expect(activationControlTestids(view, { type: "activate", instanceId: "power-2", ability: "draw" })).toEqual([
      testid.powerOf("power-2"),
    ]);
  });

  it("R384 an activation of a card the view places nowhere lights the control its type has always meant", () => {
    const view = baseView();
    expect(activationControlTestids(view, { type: "activatePower", instanceId: "gone" })).toEqual([testid.power]);
    expect(activationControlTestids(view, { type: "activate", instanceId: "gone", ability: "x" })).toEqual([
      testid.activate("gone"),
    ]);
  });
});

// ---------------------------------------------------------------------------------------------
// The click reducer
// ---------------------------------------------------------------------------------------------

describe("R384 pressing the control builds the activation as a play is built", () => {
  it("R384 one listed activation is sent at once, exactly as listed", () => {
    const view = activateView();
    const body: ActionBody = { type: "activate", instanceId: "fs1", ability: "punish", modes: ["discard"] };

    const result = onClickTarget(view, [body, { type: "endTurn" }], IDLE, { on: "activate", instanceId: "fs1" });

    expect(result.action).toEqual(body);
    expect(result.interaction).toEqual(IDLE);
  });

  it("R384 several targets enter targeting: the targets glow, and a click on one sends the body listing it", () => {
    const view = activateView();
    const legal = [PING_E1, PING_E2, PING_FACE, { type: "endTurn" } as ActionBody];

    const pressed = onClickTarget(view, legal, IDLE, { on: "activate", instanceId: "act1" });
    expect(pressed.action).toBeUndefined();
    expect(pressed.interaction.stage).toBe("activating");
    expect(isBuilding(pressed.interaction)).toBe(true);

    const { legal: lit, glow, selected } = highlightFor(view, legal, pressed.interaction);
    for (const id of [testid.card("e1"), testid.card("e2"), testid.hero("opponent")]) {
      expect(glow?.has(id), id).toBe(true);
      expect(lit.has(id), id).toBe(true);
    }
    expect(selected.has(testid.activate("act1"))).toBe(true);
    expect(selected.has(testid.card("act1"))).toBe(true);
    expect(glow?.has(testid.activate("act1")) ?? false).toBe(false);
    // A unit no candidate aims at is not a target.
    expect(lit.has(testid.card("u1"))).toBe(false);

    const sent = onClickTarget(view, legal, pressed.interaction, { on: "unit", instanceId: "e2", side: "opponent", lane: 2 });
    expect(sent.action).toEqual(PING_E2);
    expect(sent.interaction).toEqual(IDLE);

    const face = onClickTarget(view, legal, pressed.interaction, { on: "hero", side: "opponent" });
    expect(face.action).toEqual(PING_FACE);
  });

  it("R384 a click on something no candidate aims at changes nothing", () => {
    const view = activateView();
    const legal = [PING_E1, PING_E2];
    const pressed = onClickTarget(view, legal, IDLE, { on: "activate", instanceId: "act1" }).interaction;

    const own = onClickTarget(view, legal, pressed, { on: "hero", side: "you" });
    expect(own.action).toBeUndefined();
    expect(own.interaction).toBe(pressed);
    const ally = onClickTarget(view, legal, pressed, { on: "unit", instanceId: "u1", side: "you", lane: 3 });
    expect(ally.action).toBeUndefined();
    expect(ally.interaction).toBe(pressed);
  });

  it("R384 pressing the control in flight puts it down; a control legal does not list does nothing", () => {
    const view = activateView();
    const legal = [PING_E1, PING_E2];
    const pressed = onClickTarget(view, legal, IDLE, { on: "activate", instanceId: "act1" }).interaction;

    expect(onClickTarget(view, legal, pressed, { on: "activate", instanceId: "act1" }).interaction).toEqual(IDLE);
    const dark = onClickTarget(view, legal, IDLE, { on: "activate", instanceId: "fs1" });
    expect(dark.interaction).toEqual(IDLE);
    expect(dark.action).toBeUndefined();
  });

  it("R384 modes: candidates that differ by mode ask for it, and the pick sends the body listing it", () => {
    const view = activateView();
    const damage: ActionBody = { type: "activate", instanceId: "fs1", ability: "punish", modes: ["damage"], targets: [at("e1")] };
    const discard: ActionBody = { type: "activate", instanceId: "fs1", ability: "punish", modes: ["discard"] };
    const legal = [damage, discard];

    const pressed = onClickTarget(view, legal, IDLE, { on: "activate", instanceId: "fs1" }).interaction;
    expect(outstandingNeed(pressed)).toEqual({ kind: "mode", min: 1, max: 1, options: ["damage", "discard"] });

    expect(pickInPlay(pressed, { modes: ["discard"] }).action).toEqual(discard);
    // The damage mode's one listed target comes with it.
    expect(pickInPlay(pressed, { modes: ["damage"] }).action).toEqual(damage);
  });

  it("R384 a Tribute cost: candidates that differ by the unit tributed ask for it, and a click on the unit pays", () => {
    const view = activateView();
    const legal: ActionBody[] = [
      { type: "activate", instanceId: "act2", ability: "turtle", tributes: ["u1"], targets: [at("e1")] },
      { type: "activate", instanceId: "act2", ability: "turtle", tributes: ["u2"], targets: [at("e1")] },
    ];

    const pressed = onClickTarget(view, legal, IDLE, { on: "activate", instanceId: "act2", ability: "turtle" }).interaction;
    expect(outstandingNeed(pressed)).toEqual({ kind: "tribute", min: 1, max: 1, instanceIds: ["u1", "u2"] });
    const { glow } = highlightFor(view, legal, pressed);
    expect(glow?.has(testid.card("u1"))).toBe(true);
    expect(glow?.has(testid.card("u2"))).toBe(true);
    expect(highlightFor(view, legal, pressed).selected.has(testid.activate("act2", "turtle"))).toBe(true);

    const paid = onClickTarget(view, legal, pressed, { on: "unit", instanceId: "u2", side: "you", lane: 4 });
    expect(paid.action).toEqual(legal[1]);
  });

  it("R384 a control that names an ability takes only that ability's activations", () => {
    const legal: ActionBody[] = [
      { type: "activate", instanceId: "act2", ability: "ping", targets: [at("e1")] },
      { type: "activate", instanceId: "act2", ability: "turtle", tributes: ["u1"], targets: [at("e1")] },
    ];
    expect(activationsFor(legal, "act2", "ping")).toEqual([legal[0]]);
    expect(activationsFor(legal, "act2", "turtle")).toEqual([legal[1]]);
    expect(activationsFor(legal, "act2")).toEqual(legal);

    const view = activateView();
    const ping = onClickTarget(view, legal, IDLE, { on: "activate", instanceId: "act2", ability: "ping" });
    expect(ping.action).toEqual(legal[0]);
  });

  it("R384 while an activation is built, playable hand cards and other controls stay clickable and pick up instead", () => {
    const view = activateView();
    const play: ActionBody = { type: "play", instanceId: "h1" };
    const legal = [PING_E1, PING_E2, play, { type: "activate", instanceId: "fs1", ability: "punish", modes: ["discard"] } as ActionBody];
    const pressed = onClickTarget(view, legal, IDLE, { on: "activate", instanceId: "act1" }).interaction;

    const lit = highlightFor(view, legal, pressed).legal;
    expect(lit.has(testid.handCard("h1"))).toBe(true);
    expect(lit.has(testid.activate("fs1"))).toBe(true);

    expect(onClickTarget(view, legal, pressed, { on: "hand", instanceId: "h1" }).action).toEqual(play);
    expect(onClickTarget(view, legal, pressed, { on: "activate", instanceId: "fs1" }).action).toEqual(legal[3]);
  });

  it("R384 a picker's choice is carried into an activation whose candidate leaves it unset, never into a field it lacks", () => {
    const candidate: ActionBody = { type: "activatePower", instanceId: "power-1" };
    const flight: Interaction = {
      stage: "activating",
      instanceId: "power-1",
      candidates: [candidate],
      picked: { targets: [at("e1")], modes: ["x"], tributes: ["u1"] },
    };
    expect(pickInPlay(flight, {}).action).toEqual({ type: "activatePower", instanceId: "power-1", targets: [at("e1")] });
  });
});

describe("R384 Heroic Power is built through the same activation", () => {
  it("R384 the power's one listed activatePower is sent at once", () => {
    const view = activateView({ you: { ...activateView().you, hero: { health: 30, armor: 0, powers: [heroPower], power: heroPower } } });
    const body: ActionBody = { type: "activatePower", instanceId: "power-1" };

    expect(onClickTarget(view, [body], IDLE, { on: "activate", instanceId: "power-1" }).action).toEqual(body);
    // `onControl` still answers a caller with no view.
    expect(onControl([body], "power")).toEqual(body);
  });

  it("R384 a power listed with targets waits for one and sends the body listing it", () => {
    const view = activateView({ you: { ...activateView().you, hero: { health: 30, armor: 0, powers: [heroPower], power: heroPower } } });
    const legal: ActionBody[] = [
      { type: "activatePower", instanceId: "power-1", targets: [at("e1")] },
      { type: "activatePower", instanceId: "power-1", targets: [heroP2] },
    ];

    const pressed = onClickTarget(view, legal, IDLE, { on: "activate", instanceId: "power-1" }).interaction;
    expect(highlightFor(view, legal, pressed).selected.has(testid.power)).toBe(true);
    expect(highlightFor(view, legal, pressed).glow?.has(testid.hero("opponent"))).toBe(true);

    expect(onClickTarget(view, legal, pressed, { on: "hero", side: "opponent" }).action).toEqual(legal[1]);
  });
});

// ---------------------------------------------------------------------------------------------
// The play's new payments
// ---------------------------------------------------------------------------------------------

describe("B5 E5 a target that costs discards (Classic #89): the discards are picked after the target", () => {
  const ghost = "e2";
  const legal: ActionBody[] = [
    { type: "play", instanceId: "h1", targets: [at("e1")] },
    { type: "play", instanceId: "h1", targets: [at(ghost)], discards: ["h2", "h3"] },
    { type: "play", instanceId: "h1", targets: [at(ghost)], discards: ["h2", "h4"] },
    { type: "play", instanceId: "h1", targets: [at(ghost)], discards: ["h3", "h4"] },
  ];

  it("targeting the ghost asks for the discards, from the cards the listed sets name", () => {
    const view = activateView();
    const picked = onClickTarget(view, legal, IDLE, { on: "hand", instanceId: "h1" }).interaction;
    expect(outstandingNeed(picked)?.kind).toBe("target");

    const atGhost = onClickTarget(view, legal, picked, { on: "unit", instanceId: ghost, side: "opponent", lane: 2 });
    expect(atGhost.action).toBeUndefined();
    expect(outstandingNeed(atGhost.interaction)).toEqual({ kind: "discard", min: 2, max: 2, instanceIds: ["h2", "h3", "h4"] });

    const paid = pickInPlay(atGhost.interaction, { discards: ["h4", "h2"] });
    expect(paid.action).toEqual(legal[2]);
  });

  it("targeting anything else asks for no discards and sends the body without them", () => {
    const view = activateView();
    const picked = onClickTarget(view, legal, IDLE, { on: "hand", instanceId: "h1" }).interaction;
    expect(onClickTarget(view, legal, picked, { on: "unit", instanceId: "e1", side: "opponent", lane: 1 }).action).toEqual(legal[0]);
  });

  it("a set of discards the engine did not list is refused, and the pick stays open", () => {
    const view = activateView();
    const picked = onClickTarget(view, legal, IDLE, { on: "hand", instanceId: "h1" }).interaction;
    const atGhost = onClickTarget(view, legal, picked, { on: "unit", instanceId: ghost, side: "opponent", lane: 2 }).interaction;

    const refused = pickInPlay(atGhost, { discards: ["h2"] });
    expect(refused.action).toBeUndefined();
    expect(refused.interaction).toBe(atGhost);
  });
});

describe("B5 E11, E19 Plague Counters paying a graveyard play (Classic #74)", () => {
  const mana: ActionBody = { type: "play", instanceId: "gy1", zone: { row: "units", lane: 5 } };
  const one: ActionBody = { type: "play", instanceId: "gy1", zone: { row: "units", lane: 5 }, plague: { from: "fs1", tokens: 1 } };
  const two: ActionBody = { type: "play", instanceId: "gy1", zone: { row: "units", lane: 5 }, plague: { from: "fs1", tokens: 2 } };

  it("plays that differ by the tokens they spend ask how many, in the engine's order, and send the one picked", () => {
    const view = activateView();
    const legal = [mana, one, two];

    const picked = onClickTarget(view, legal, IDLE, { on: "graveyard", instanceId: "gy1" }).interaction;
    const need = outstandingNeed(picked);
    expect(need).toEqual({ kind: "plague", min: 1, max: 1, options: ["none", { from: "fs1", tokens: 1 }, { from: "fs1", tokens: 2 }] });

    expect(pickInPlay(picked, { plague: { from: "fs1", tokens: 2 } }).action).toEqual(two);
    expect(pickInPlay(picked, { plague: "none" }).action).toEqual(mana);
  });

  it("a play that must spend tokens offers no mana-only choice", () => {
    const view = activateView();
    const picked = onClickTarget(view, [one, two], IDLE, { on: "graveyard", instanceId: "gy1" }).interaction;
    expect(outstandingNeed(picked)).toEqual({
      kind: "plague",
      min: 1,
      max: 1,
      options: [{ from: "fs1", tokens: 1 }, { from: "fs1", tokens: 2 }],
    });
    expect(pickInPlay(picked, { plague: "none" }).action).toBeUndefined();
  });

  it("the key of a payment never confuses a card with a count", () => {
    expect(plagueKey("none")).toBe("none");
    expect(plagueKey(undefined)).toBe("none");
    expect(plagueKey({ from: "a:1", tokens: 2 })).not.toBe(plagueKey({ from: "a", tokens: 12 }));
  });
});

// ---------------------------------------------------------------------------------------------
// R391: a Tribute onto its own zone
// ---------------------------------------------------------------------------------------------

describe("R391 a Tribute may pay for its own zone, and the client never crosses a zone with another set", () => {
  /** A full unit row (u1..u5): legalActions pairs each Tribute with the zone it empties. */
  function fullRow(): PlayerView {
    return baseView({
      you: emptySide("p1", {
        hand: [card({ instanceId: "t1", defId: "core-066" })],
        units: [1, 2, 3, 4, 5].map((n) => unit("p1", { instanceId: `u${String(n)}` })),
      }),
    });
  }
  const pair = (n: number): ActionBody => ({
    type: "play",
    instanceId: "t1",
    tributes: [`u${String(n)}`],
    zone: { row: "units", lane: n },
  });
  const legal = [pair(3), pair(4)];

  it("R391 the tributed units and their own zones glow; the other units do not", () => {
    const view = fullRow();
    const picked = onClickTarget(view, legal, IDLE, { on: "hand", instanceId: "t1" }).interaction;
    const { glow } = highlightFor(view, legal, picked);

    for (const id of [testid.card("u3"), testid.card("u4"), testid.zone("you", "units", 3), testid.zone("you", "units", 4)]) {
      expect(glow?.has(id), id).toBe(true);
    }
    expect(glow?.has(testid.card("u1")) ?? false).toBe(false);
    expect(glow?.has(testid.zone("you", "units", 1)) ?? false).toBe(false);
  });

  it("R391 a click on the unit tributes it into its own zone", () => {
    const view = fullRow();
    const picked = onClickTarget(view, legal, IDLE, { on: "hand", instanceId: "t1" }).interaction;
    expect(onClickTarget(view, legal, picked, { on: "unit", instanceId: "u4", side: "you", lane: 4 }).action).toEqual(pair(4));
  });

  it("R391 a click on the zone picks the pair listed for it, never another set into that zone", () => {
    const view = fullRow();
    const picked = onClickTarget(view, legal, IDLE, { on: "hand", instanceId: "t1" }).interaction;
    expect(onClickTarget(view, legal, picked, { on: "zone", side: "you", row: "units", lane: 3 }).action).toEqual(pair(3));
    // A zone no pair names is not a move.
    const none = onClickTarget(view, legal, picked, { on: "zone", side: "you", row: "units", lane: 1 });
    expect(none.action).toBeUndefined();
    expect(none.interaction).toBe(picked);
  });

  it("R391 the Tribute picker's pick finishes the pair it belongs to", () => {
    const view = fullRow();
    const picked = onClickTarget(view, legal, IDLE, { on: "hand", instanceId: "t1" }).interaction;
    expect(outstandingNeed(picked)).toEqual({ kind: "tribute", min: 1, max: 1, instanceIds: ["u3", "u4"] });
    expect(pickInPlay(picked, { tributes: ["u3"] }).action).toEqual(pair(3));
  });

  it("R391 with a set that frees two zones, the zone is asked after the set, among that set's own zones only", () => {
    const view = fullRow();
    const both: ActionBody[] = [
      { type: "play", instanceId: "t1", tributes: ["u1", "u2"], zone: { row: "units", lane: 1 } },
      { type: "play", instanceId: "t1", tributes: ["u1", "u2"], zone: { row: "units", lane: 2 } },
      { type: "play", instanceId: "t1", tributes: ["u4", "u5"], zone: { row: "units", lane: 4 } },
    ];
    const picked = onClickTarget(view, both, IDLE, { on: "hand", instanceId: "t1" }).interaction;
    const set = pickInPlay(picked, { tributes: ["u1", "u2"] }).interaction;
    expect(outstandingNeed(set)).toEqual({
      kind: "zone",
      min: 1,
      max: 1,
      zones: [
        { row: "units", lane: 1 },
        { row: "units", lane: 2 },
      ],
    });
    expect(onClickTarget(view, both, set, { on: "zone", side: "you", row: "units", lane: 4 }).action).toBeUndefined();
    expect(onClickTarget(view, both, set, { on: "zone", side: "you", row: "units", lane: 2 }).action).toEqual(both[1]);
  });
});

// ---------------------------------------------------------------------------------------------
// B5 E11: plays from the graveyard
// ---------------------------------------------------------------------------------------------

describe("B5 E11 a card in your graveyard that legal lists is played from the pile", () => {
  it("its pile-play control and the pile glow; a graveyard card legal does not list does not", () => {
    const view = activateView();
    const legal: ActionBody[] = [{ type: "play", instanceId: "gy1", zone: { row: "units", lane: 5 } }];

    const { legal: lit, glow } = highlightFor(view, legal, IDLE);

    expect(playSourceTestid(view, "gy1")).toBe(testid.pilePlay("gy1"));
    expect(playSourceTestid(view, "h1")).toBe(testid.handCard("h1"));
    expect(lit.has(testid.pilePlay("gy1"))).toBe(true);
    expect(glow?.has(testid.pilePlay("gy1"))).toBe(true);
    expect(glow?.has(testid.graveyard("you"))).toBe(true);
    expect(lit.has(testid.pilePlay("gy2"))).toBe(false);
    expect(lit.has(testid.handCard("gy1"))).toBe(false);
  });

  it("pressing Play builds the play as a hand card's: one candidate is sent, a second press puts it down", () => {
    const view = activateView();
    const single: ActionBody = { type: "play", instanceId: "gy1", zone: { row: "units", lane: 5 } };
    expect(onClickTarget(view, [single], IDLE, { on: "graveyard", instanceId: "gy1" }).action).toEqual(single);

    const two: ActionBody[] = [single, { type: "play", instanceId: "gy1", zone: { row: "units", lane: 4 } }];
    const picked = onClickTarget(view, two, IDLE, { on: "graveyard", instanceId: "gy1" }).interaction;
    expect(picked.stage).toBe("playing");
    expect(highlightFor(view, two, picked).selected.has(testid.pilePlay("gy1"))).toBe(true);
    expect(onClickTarget(view, two, picked, { on: "graveyard", instanceId: "gy1" }).interaction).toEqual(IDLE);
    // A card legal does not list is not picked up.
    expect(onClickTarget(view, two, IDLE, { on: "graveyard", instanceId: "gy2" }).interaction).toEqual(IDLE);
  });
});

// ---------------------------------------------------------------------------------------------
// Drag to target (game/drag/model.ts)
// ---------------------------------------------------------------------------------------------

describe("R510 an activation is dragged to its target", () => {
  const legal = [PING_E1, PING_E2, { type: "endTurn" } as ActionBody];

  it("R510 a press on the control lifts the activation with the arrow, and its targets are the drop spots", () => {
    const view = activateView();
    const plan = planDrag(view, legal, IDLE, { on: "activate", instanceId: "act1" });

    expect(plan?.kind).toBe("activate");
    expect(plan?.arrow).toBe(true);
    expect(plan?.freeDrop).toBe(false);
    expect(plan?.sourceTestid).toBe(testid.activate("act1"));
    expect([...(plan?.dropTestids ?? [])].sort()).toEqual([testid.card("e1"), testid.card("e2")].sort());
  });

  it("R510 the drop on a target sends the body listing it; a drop anywhere else sends nothing", () => {
    const view = activateView();
    const plan = planDrag(view, legal, IDLE, { on: "activate", instanceId: "act1" });
    if (plan === null) throw new Error("no plan");

    const hit = resolveDrop(view, legal, plan, {
      at: "target",
      target: { on: "unit", instanceId: "e2", side: "opponent", lane: 2 },
      testid: testid.card("e2"),
    });
    expect(hit.action).toEqual(PING_E2);
    expect(resolveDrop(view, legal, plan, { at: "board" })).toEqual({ interaction: IDLE });
  });

  it("R510 a card of yours with nothing to attack lifts its one ability; one that can attack lifts its attack", () => {
    const view = activateView();
    const fromCard = planDrag(view, legal, IDLE, { on: "unit", instanceId: "act1", side: "you", lane: 1 });
    expect(fromCard?.kind).toBe("activate");
    expect(fromCard?.sourceTestid).toBe(testid.card("act1"));

    const withAttack = [...legal, { type: "attack", attackerId: "act1", targetId: "e1" } as ActionBody];
    expect(planDrag(view, withAttack, IDLE, { on: "unit", instanceId: "act1", side: "you", lane: 1 })?.kind).toBe("attack");
  });

  it("R510 nothing to aim at (off the backrow), a card listing several abilities, or a build in flight: the press stays a click", () => {
    const view = activateView();
    const untargeted: ActionBody[] = [{ type: "activate", instanceId: "fs1", ability: "punish", modes: ["discard"] }];
    expect(planDrag(view, untargeted, IDLE, { on: "activate", instanceId: "fs1" })).toBeNull();
    // R658: a backrow card of yours is the exception, dragged onto the board (below).

    const several: ActionBody[] = [{ type: "activate", instanceId: "act2", ability: "ping", targets: [at("e1")] }];
    expect(planDrag(view, several, IDLE, { on: "unit", instanceId: "act2", side: "you", lane: 2 })).toBeNull();
    expect(planDrag(view, several, IDLE, { on: "activate", instanceId: "act2", ability: "ping" })?.kind).toBe("activate");

    const flight = activating({ candidates: legal });
    expect(planDrag(view, legal, flight, { on: "unit", instanceId: "act1", side: "you", lane: 1 })).toBeNull();
  });

  it("R510 a face-up backrow card of yours lifts its aimed ability", () => {
    const view = activateView();
    const aimed: ActionBody[] = [{ type: "activate", instanceId: "fs1", ability: "punish", modes: ["damage"], targets: [at("e1")] }];
    const plan = planDrag(view, aimed, IDLE, { on: "backrow", instanceId: "fs1", side: "you", lane: 1 });
    expect(plan?.kind).toBe("activate");
    expect(planDrag(view, aimed, IDLE, { on: "backrow", instanceId: "fs1", side: "opponent", lane: 1 })).toBeNull();
  });
});

describe("R658 a backrow card whose ability aims at nothing is dragged onto the board", () => {
  const untargeted: ActionBody = { type: "activate", instanceId: "fs1", ability: "punish", modes: ["discard"] };
  const twoModes: ActionBody = { type: "activate", instanceId: "fs1", ability: "punish", modes: ["draw"] };

  it("R658 the press lifts it as a card ghost that may drop anywhere on the board, and the drop sends it", () => {
    const view = activateView();
    const plan = planDrag(view, [untargeted], IDLE, { on: "backrow", instanceId: "fs1", side: "you", lane: 1 });
    expect(plan?.kind).toBe("activate");
    expect(plan?.arrow).toBe(false);
    expect(plan?.freeDrop).toBe(true);
    if (plan === null) throw new Error("no plan");
    expect(resolveDrop(view, [untargeted], plan, { at: "board" }).action).toEqual(untargeted);
    expect(resolveDrop(view, [untargeted], plan, { at: "outside" })).toEqual({ interaction: IDLE });
  });

  it("R658 with two modes the drop leaves the mode to the picker, as the click does", () => {
    const view = activateView();
    const legal = [untargeted, twoModes];
    const plan = planDrag(view, legal, IDLE, { on: "backrow", instanceId: "fs1", side: "you", lane: 1 });
    if (plan === null) throw new Error("no plan");
    const dropped = resolveDrop(view, legal, plan, { at: "board" });
    expect(dropped.action).toBeUndefined();
    expect(dropped.interaction.stage).toBe("activating");
  });

  it("R658 its control and a Unit with the same unaimed ability stay clicks", () => {
    const view = activateView();
    expect(planDrag(view, [untargeted], IDLE, { on: "activate", instanceId: "fs1" })).toBeNull();
    const unaimedUnit: ActionBody = { type: "activate", instanceId: "act1", ability: "ping" };
    expect(planDrag(view, [unaimedUnit], IDLE, { on: "unit", instanceId: "act1", side: "you", lane: 1 })).toBeNull();
  });
});
