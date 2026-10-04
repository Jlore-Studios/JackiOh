// The Activate control on the board (R384, its presentation R510), Heroic Power through the same
// build, the play's new payments in the inline pickers (B5 E5, E11, E19) and "Play" in the
// graveyard pile (B5 E11), rendered from fixture views and driven through `<Game/>` as a player
// drives it. Every action expected is a body the fixture's `legal` array lists (CLAUDE.md rule 7).

import { CATALOG } from "@jackioh/cards";
import type { ActionBody, ActivationView, HeroPowerView, PlayerView, Selection } from "@jackioh/shared";
import { act, cleanup, fireEvent, render, screen, within } from "@testing-library/react";
import userEvent from "@testing-library/user-event";
import { afterEach, beforeEach, describe, expect, it, vi } from "vitest";

import { closeInspect } from "../cards/index.ts";
import { __resetSettingsForTests } from "../settings/index.ts";
import { baseView, card, emptySide, faceUpBackrow, heroPower, unit } from "../test/fixtures.ts";
import { abilityLabel, usesText } from "./ActivateControl.tsx";
import Board from "./Board.tsx";
import Game from "./Game.tsx";
import { IDLE, highlightFor } from "./actions.ts";
import { CatalogContext, lookupFromDefs } from "./catalog.ts";
import { testid } from "./contract.ts";
import { DRAG_THRESHOLD_PX } from "./drag/model.ts";

const lookup = lookupFromDefs(CATALOG);

// ---------------------------------------------------------------------------------------------
// Fixtures
// ---------------------------------------------------------------------------------------------

const PING: ActivationView = { ability: "ping", label: "Deal {damage} damage.", usesLeft: 1, usable: true };
const PING_TWICE: ActivationView = { ability: "ping", label: "Deal {damage} damage.", usesLeft: 2, usable: true };
const TURTLE: ActivationView = {
  ability: "turtle",
  label: "Tribute a Unit. Deal damage equal to its Attack to any target.",
  usesLeft: null,
  usable: true,
};
const USED: ActivationView = {
  ability: "punish",
  label: "Choose one",
  usesLeft: 0,
  usable: false,
  reason: "that ability has already been used this turn",
};
const PUNISH: ActivationView = { ...USED, usesLeft: 1, usable: true, reason: undefined } as ActivationView;

const SECOND_POWER: HeroPowerView = {
  instanceId: "power-2",
  defId: "core-098",
  name: "draw",
  ability: "draw",
  radiant: false,
  x: 1,
  usedThisTurn: false,
};

/**
 * `act1` Brother Ping (one ability, its damage moved to 3), `act2` a Turtinator listing two
 * abilities, `fs1` a Field Spell whose ability is spent (greyed), `fs2` one with modes; `u1`, `u2`
 * plain units; hand `h1`..`h4`; graveyard `gy1`, `gy2`; two Heroic Powers (the first `power`, with
 * `over.power` in its place when given, its card in the backrow).
 */
function activateView(over: { fs1?: ActivationView[]; power?: HeroPowerView } = {}): PlayerView {
  const first = over.power ?? heroPower;
  return baseView({
    you: emptySide("p1", {
      hero: { health: 30, armor: 0, powers: [first, SECOND_POWER], power: first },
      hand: [
        card({ instanceId: "h1", defId: "core-002" }),
        card({ instanceId: "h2", defId: "core-019" }),
        card({ instanceId: "h3", defId: "core-005" }),
        card({ instanceId: "h4", defId: "core-008" }),
      ],
      units: [
        unit("p1", { instanceId: "act1", defId: "classicplus-076-1", params: { damage: 3 }, activations: [PING] }),
        unit("p1", { instanceId: "act2", defId: "classic-021", activations: [PING_TWICE, TURTLE] }),
        unit("p1", { instanceId: "u1" }),
        unit("p1", { instanceId: "u2" }),
        null,
      ],
      backrow: [
        faceUpBackrow("p1", { instanceId: "fs1", defId: "classic-020", activations: over.fs1 ?? [USED] }),
        faceUpBackrow("p1", { instanceId: "fs2", defId: "classic-020", activations: [PUNISH] }),
        faceUpBackrow("p1", { instanceId: "power-1", defId: "core-098", activations: [PING] }),
        null,
        null,
      ],
      graveyard: [card({ instanceId: "gy1", defId: "core-004" }), card({ instanceId: "gy2", defId: "core-008" })],
    }),
    opponent: emptySide("p2", {
      hand: { count: 3 },
      units: [unit("p2", { instanceId: "e1" }), unit("p2", { instanceId: "e2" }), null, null, null],
    }),
  });
}

const at = (instanceId: string): Selection => ({ pick: "instance", instanceId });
const PING_E1: ActionBody = { type: "activate", instanceId: "act1", ability: "ping", targets: [at("e1")] };
const PING_E2: ActionBody = { type: "activate", instanceId: "act1", ability: "ping", targets: [at("e2")] };
const DAMAGE: ActionBody = { type: "activate", instanceId: "fs2", ability: "punish", modes: ["damage"], targets: [at("e1")] };
const DISCARD: ActionBody = { type: "activate", instanceId: "fs2", ability: "punish", modes: ["discard"] };
const TURTLE_U1: ActionBody = { type: "activate", instanceId: "act2", ability: "turtle", tributes: ["u1"], targets: [at("e1")] };
const TURTLE_U2: ActionBody = { type: "activate", instanceId: "act2", ability: "turtle", tributes: ["u2"], targets: [at("e1")] };

/**
 * Patch v0.2.1 (R43, R657): a Ping Heroic Power, whose power is the card's one Activate ability
 * `ping`, its target (any unit or hero) declared in the `activate` that `legalActions` lists — one
 * body per target, as the engine lists them.
 */
const PING_POWER: HeroPowerView = { ...heroPower, name: "ping", ability: "ping", x: 1 };
const powerPing = (target: Selection): ActionBody => ({ type: "activate", instanceId: "power-1", ability: "ping", targets: [target] });
const PING_POWER_LEGAL: readonly ActionBody[] = [
  powerPing(at("e1")),
  powerPing(at("e2")),
  powerPing(at("u1")),
  powerPing({ pick: "hero", player: "p2" }),
  powerPing({ pick: "hero", player: "p1" }),
];

const el = (id: string): HTMLElement => screen.getByTestId(id);

function renderGame(legal: readonly ActionBody[], view: PlayerView = activateView()) {
  const onAction = vi.fn();
  const utils = render(
    <CatalogContext.Provider value={lookup}>
      <Game view={view} legal={legal} onAction={onAction} />
    </CatalogContext.Provider>,
  );
  return { ...utils, onAction };
}

function renderBoard(legal: readonly ActionBody[], over: Partial<Parameters<typeof Board>[0]> = {}) {
  const view = over.view ?? activateView();
  const onClick = vi.fn();
  render(
    <CatalogContext.Provider value={lookup}>
      <Board view={view} highlight={highlightFor(view, legal, IDLE)} onClick={onClick} {...over} />
    </CatalogContext.Provider>,
  );
  return { onClick };
}

beforeEach(() => {
  __resetSettingsForTests();
});

afterEach(() => {
  cleanup();
  closeInspect();
  document.documentElement.removeAttribute("data-dragging");
  __resetSettingsForTests();
  try {
    window.localStorage.clear();
  } catch {
    // Storage is optional.
  }
});

// ---------------------------------------------------------------------------------------------
// The control
// ---------------------------------------------------------------------------------------------

describe("R510 the Activate control on the card", () => {
  it("R510 a card listing one ability wears one control: the glyph, its uses left, its words filled in, live and glowing", () => {
    renderBoard([PING_E1, PING_E2]);

    const control = el(testid.activate("act1"));
    expect(control.tagName).toBe("BUTTON");
    expect(control).toHaveAttribute("type", "button");
    expect(control).toHaveAttribute("data-legal", "true");
    expect(control).toHaveAttribute("data-glow", "ready");
    expect(control).not.toHaveAttribute("aria-disabled");
    // B3.4: the label's {damage} is the view's number, never a raw placeholder.
    expect(control.getAttribute("title")).toBe("Activate: Deal 3 damage. (1 use left this turn)");
    expect(control.getAttribute("aria-label")).toBe("Activate Brother Ping: Deal 3 damage. 1 use left this turn.");
    expect(control.getAttribute("title")).not.toMatch(/[{}]/);
    expect(el(testid.activateUses("act1"))).toHaveTextContent("1");
    expect(el(testid.activateUses("act1"))).toHaveAttribute("data-uses", "1");
    // The control sits on the card, inside its root.
    expect(el(testid.card("act1")).contains(control)).toBe(true);
  });

  it("R510 a card listing several abilities wears one control each, named by ability; ♾️ counts as ∞", () => {
    renderBoard([TURTLE_U1]);

    expect(screen.queryByTestId(testid.activate("act2"))).toBeNull();
    expect(el(testid.activateUses("act2", "ping"))).toHaveTextContent("2");
    expect(el(testid.activateUses("act2", "turtle"))).toHaveTextContent("∞");
    expect(el(testid.activateUses("act2", "turtle"))).toHaveAttribute("data-uses", "unlimited");
    expect(el(testid.activate("act2", "turtle"))).toHaveAttribute("data-legal", "true");
    // legal lists no `ping` for it, so that control is greyed though the view calls it usable.
    expect(el(testid.activate("act2", "ping"))).toHaveAttribute("data-legal", "false");
    expect(usesText(null)).toBe("∞");
  });

  it("R510 a control legal does not list is greyed, says the engine's reason, and a press sends nothing", () => {
    const { onClick } = renderBoard([PING_E1]);

    const spent = el(testid.activate("fs1"));
    expect(spent).toHaveAttribute("data-legal", "false");
    expect(spent).toHaveAttribute("aria-disabled", "true");
    expect(spent).toHaveAttribute("data-usable", "false");
    expect(spent.getAttribute("title")).toContain("That ability has already been used this turn");
    expect(spent.getAttribute("aria-label")).toContain("That ability has already been used this turn.");
    expect(el(testid.activateUses("fs1"))).toHaveTextContent("0");

    fireEvent.click(spent);
    expect(onClick).not.toHaveBeenCalled();
  });

  it("R510 cards that list no ability wear no control, and a press on a live one reports the activation, not the card", () => {
    const { onClick } = renderBoard([PING_E1, { type: "attack", attackerId: "act1", targetId: "e1" }]);

    expect(within(el(testid.card("u1"))).queryByRole("button", { name: /Activate/ })).toBeNull();
    expect(within(el(testid.card("e1"))).queryByRole("button", { name: /Activate/ })).toBeNull();

    fireEvent.click(el(testid.activate("act1")));
    expect(onClick).toHaveBeenCalledTimes(1);
    expect(onClick).toHaveBeenCalledWith({ on: "activate", instanceId: "act1" });
  });

  it("R510 Heroic Power's card wears no Activate control because its one control is on the hero", () => {
    renderBoard([{ type: "activatePower", instanceId: "power-1" }]);

    expect(within(el(testid.card("power-1"))).queryByRole("button", { name: /Activate/ })).toBeNull();
    expect(el(testid.power)).toBeInTheDocument();
  });

  it("R510 the keyboard presses it with Enter and Space, and the card under it does not take the key", async () => {
    const user = userEvent.setup();
    const { onClick } = renderBoard([PING_E1, { type: "attack", attackerId: "act1", targetId: "e1" }]);

    el(testid.activate("act1")).focus();
    await user.keyboard("{Enter}");
    await user.keyboard(" ");

    expect(onClick.mock.calls.map((call) => call[0])).toEqual([
      { on: "activate", instanceId: "act1" },
      { on: "activate", instanceId: "act1" },
    ]);
  });

  it("R510 the control flashes while the `activated` row plays on its card, and so does a Heroic Power's button", () => {
    renderBoard([], {
      animating: new Map([
        [testid.card("act1"), "activated"],
        [testid.card("power-1"), "activated"],
      ]),
    });

    expect(el(testid.activate("act1"))).toHaveAttribute("data-flash", "activated");
    expect(el(testid.activate("fs2"))).not.toHaveAttribute("data-flash");
    expect(el(testid.power)).toHaveAttribute("data-flash", "activated");
    expect(el(testid.powerOf("power-2"))).not.toHaveAttribute("data-flash");
  });

  it("R510 an ability's words read the card's declared numbers on the face it has", () => {
    const def = CATALOG["classic-021"];
    const info = { name: "Turtinator", type: "Unit" as const, text: "", tags: [], ...(def === undefined ? {} : { def }) };
    const radiant = card({ defId: "classic-021", radiant: true });
    expect(abilityLabel(info, radiant, "Deal {multiplier}× its Attack.")).toBe("Deal 2× its Attack.");
    expect(abilityLabel(info, { ...radiant, params: { multiplier: 5 } }, "Deal {multiplier}× its Attack.")).toBe("Deal 5× its Attack.");
    expect(abilityLabel(info, radiant, "No numbers.")).toBe("No numbers.");
  });
});

// ---------------------------------------------------------------------------------------------
// Building an activation through the board
// ---------------------------------------------------------------------------------------------

describe("R384 an activation is built through the board as a play is", () => {
  it("R384 one listed activation: a press sends exactly that body", () => {
    const { onAction } = renderGame([DISCARD]);

    fireEvent.click(el(testid.activate("fs2")));

    expect(onAction).toHaveBeenCalledTimes(1);
    expect(onAction).toHaveBeenCalledWith(DISCARD);
  });

  it("R384 several targets: the press selects the control and lights the targets, and a click on one sends its body", () => {
    const { onAction } = renderGame([PING_E1, PING_E2]);

    fireEvent.click(el(testid.activate("act1")));
    expect(onAction).not.toHaveBeenCalled();
    expect(el(testid.activate("act1"))).toHaveAttribute("data-selected", "true");
    expect(el(testid.card("e2"))).toHaveAttribute("data-glow", "ready");
    expect(el(testid.card("u1"))).not.toHaveAttribute("data-glow");

    fireEvent.click(el(testid.card("e2")));
    expect(onAction).toHaveBeenCalledWith(PING_E2);
  });

  it("R384 modes: the inline mode picker opens, headed by the card, and the pick sends the listed body", () => {
    const { onAction } = renderGame([DAMAGE, DISCARD]);

    fireEvent.click(el(testid.activate("fs2")));
    const modal = el("prompt-modal");
    expect(modal).toHaveAttribute("data-prompt-kind", "mode");
    expect(modal).toHaveAttribute("data-prompt-source", "play");
    expect(modal).toHaveTextContent("The Power to Punish");

    fireEvent.click(el("prompt-option-discard"));
    expect(onAction).toHaveBeenCalledWith(DISCARD);
  });

  it("R384 a Tribute cost: the picker lists the units the sets name, and a click on a unit on the board pays it", () => {
    const { onAction } = renderGame([TURTLE_U1, TURTLE_U2]);

    fireEvent.click(el(testid.activate("act2", "turtle")));
    expect(el("prompt-modal")).toHaveAttribute("data-prompt-kind", "tribute");
    expect(el(testid.card("u2"))).toHaveAttribute("data-glow", "ready");

    fireEvent.click(el(testid.card("u2")));
    expect(onAction).toHaveBeenCalledWith(TURTLE_U2);
  });

  it("R384 Cancel puts the activation down and sends nothing", () => {
    const { onAction } = renderGame([DAMAGE, DISCARD]);

    fireEvent.click(el(testid.activate("fs2")));
    fireEvent.click(el("prompt-cancel"));

    expect(screen.queryByTestId("prompt-modal")).toBeNull();
    expect(el(testid.activate("fs2"))).not.toHaveAttribute("data-selected");
    expect(onAction).not.toHaveBeenCalled();
  });
});

describe("R384 Heroic Power works through the same build", () => {
  it("R510 `power` sends its listed activatePower, and a further power is its own live control", () => {
    const first: ActionBody = { type: "activatePower", instanceId: "power-1" };
    const second: ActionBody = { type: "activatePower", instanceId: "power-2" };
    const { onAction } = renderGame([first, second]);

    expect(el(testid.power)).toHaveAttribute("data-glow", "ready");
    fireEvent.click(el(testid.power));
    expect(onAction).toHaveBeenLastCalledWith(first);

    fireEvent.click(el(testid.powerOf("power-2")));
    expect(onAction).toHaveBeenLastCalledWith(second);
  });

  it("R384 a power listed with targets waits for its target on the board", () => {
    const legal: ActionBody[] = [
      { type: "activatePower", instanceId: "power-1", targets: [at("e1")] },
      { type: "activatePower", instanceId: "power-1", targets: [{ pick: "hero", player: "p2" }] },
    ];
    const { onAction } = renderGame(legal);

    fireEvent.click(el(testid.power));
    expect(onAction).not.toHaveBeenCalled();
    expect(el(testid.power)).toHaveAttribute("data-selected", "true");

    fireEvent.click(el(testid.hero("opponent")));
    expect(onAction).toHaveBeenCalledWith(legal[1]);
  });

  it("R43 patch v0.2.1: a power with no target sends its one listed `activate`, naming its ability, at once", () => {
    const body: ActionBody = { type: "activate", instanceId: "power-1", ability: "discover" };
    const { onAction } = renderGame([body]);

    expect(el(testid.power)).toHaveAttribute("data-glow", "ready");
    fireEvent.click(el(testid.power));
    expect(onAction).toHaveBeenCalledTimes(1);
    expect(onAction).toHaveBeenCalledWith(body);
  });

  it("R657 Ping: a press waits for its target, the targets light, and a click on an enemy unit sends that `activate`", () => {
    const { onAction } = renderGame(PING_POWER_LEGAL, activateView({ power: PING_POWER }));

    fireEvent.click(el(testid.power));
    expect(onAction).not.toHaveBeenCalled();
    expect(el(testid.power)).toHaveAttribute("data-selected", "true");
    for (const lit of [testid.card("e1"), testid.card("e2"), testid.card("u1"), testid.hero("opponent"), testid.hero("you")]) {
      expect(el(lit), lit).toHaveAttribute("data-glow", "ready");
    }
    // A unit `legal` names no target on is not one.
    expect(el(testid.card("u2"))).not.toHaveAttribute("data-glow");

    fireEvent.click(el(testid.card("e2")));
    expect(onAction).toHaveBeenCalledTimes(1);
    expect(onAction).toHaveBeenCalledWith(powerPing(at("e2")));
  });

  it("R657 Ping: a click on either hero sends the `activate` naming that hero", () => {
    const { onAction } = renderGame(PING_POWER_LEGAL, activateView({ power: PING_POWER }));

    fireEvent.click(el(testid.power));
    fireEvent.click(el(testid.hero("opponent")));
    expect(onAction).toHaveBeenLastCalledWith(powerPing({ pick: "hero", player: "p2" }));

    fireEvent.click(el(testid.power));
    fireEvent.click(el(testid.hero("you")));
    expect(onAction).toHaveBeenLastCalledWith(powerPing({ pick: "hero", player: "p1" }));
    expect(onAction).toHaveBeenCalledTimes(2);
  });

  it("R657 Ping: a second press on the power puts it down and sends nothing", () => {
    const { onAction } = renderGame(PING_POWER_LEGAL, activateView({ power: PING_POWER }));

    fireEvent.click(el(testid.power));
    fireEvent.click(el(testid.power));
    expect(el(testid.power)).not.toHaveAttribute("data-selected");
    expect(onAction).not.toHaveBeenCalled();
  });

  it("R384 a power legal does not list is disabled and sends nothing", () => {
    const { onAction } = renderGame([{ type: "endTurn" }]);
    expect(el(testid.power)).toBeDisabled();
    expect(el(testid.powerOf("power-2"))).toHaveAttribute("data-legal", "false");
    fireEvent.click(el(testid.powerOf("power-2")));
    expect(onAction).not.toHaveBeenCalled();
  });
});

// ---------------------------------------------------------------------------------------------
// Drag to target
// ---------------------------------------------------------------------------------------------

describe("R510 an activation is dragged from its control onto its target", () => {
  let under: Element[] = [];
  const START = { x: 200, y: 400 };

  beforeEach(() => {
    under = [];
    document.elementsFromPoint = (() => under) as Document["elementsFromPoint"];
  });

  afterEach(() => {
    delete (document as Partial<Document>).elementsFromPoint;
  });

  function drag(source: Element, onto: Element): void {
    under = [source];
    fireEvent.pointerDown(source, { pointerId: 1, button: 0, clientX: START.x, clientY: START.y });
    fireEvent.pointerMove(window, { pointerId: 1, clientX: START.x, clientY: START.y - DRAG_THRESHOLD_PX - 4 });
    under = [onto];
    fireEvent.pointerMove(window, { pointerId: 1, clientX: 420, clientY: 180 });
  }

  it("R510 the arrow runs from the control, the reticle frames the target, and the release sends its body", () => {
    const { onAction } = renderGame([PING_E1, PING_E2]);

    drag(el(testid.activate("act1")), el(testid.card("e2")));
    expect(el("drag-layer")).toHaveAttribute("data-kind", "activate");
    expect(el("drag-arrow")).toHaveAttribute("data-from", testid.activate("act1"));
    expect(el("drag-reticle")).toHaveAttribute("data-target", testid.card("e2"));
    expect(document.documentElement).toHaveAttribute("data-dragging", "activate");
    expect(onAction).not.toHaveBeenCalled();

    act(() => {
      fireEvent.pointerUp(window, { pointerId: 1, button: 0, clientX: 420, clientY: 180 });
    });
    expect(onAction).toHaveBeenCalledTimes(1);
    expect(onAction).toHaveBeenCalledWith(PING_E2);
    expect(screen.queryByTestId("drag-layer")).toBeNull();
  });

  it("R510 a Heroic Power is dragged from its button to its target", () => {
    const legal: ActionBody[] = [
      { type: "activatePower", instanceId: "power-1", targets: [at("e1")] },
      { type: "activatePower", instanceId: "power-1", targets: [at("e2")] },
    ];
    const { onAction } = renderGame(legal);

    drag(el(testid.power), el(testid.card("e1")));
    expect(el("drag-arrow")).toHaveAttribute("data-from", testid.power);
    fireEvent.pointerUp(window, { pointerId: 1, button: 0, clientX: 420, clientY: 180 });

    expect(onAction).toHaveBeenCalledWith(legal[0]);
  });

  it("R657 patch v0.2.1: Ping is dragged from the power on the hero to an enemy unit, and the release sends that `activate`", () => {
    const { onAction } = renderGame(PING_POWER_LEGAL, activateView({ power: PING_POWER }));

    drag(el(testid.power), el(testid.card("e1")));
    expect(el("drag-layer")).toHaveAttribute("data-kind", "activate");
    expect(el("drag-arrow")).toHaveAttribute("data-from", testid.power);
    expect(el("drag-reticle")).toHaveAttribute("data-target", testid.card("e1"));
    expect(onAction).not.toHaveBeenCalled();

    act(() => {
      fireEvent.pointerUp(window, { pointerId: 1, button: 0, clientX: 420, clientY: 180 });
    });
    expect(onAction).toHaveBeenCalledTimes(1);
    expect(onAction).toHaveBeenCalledWith(powerPing(at("e1")));
  });

  it("R657 patch v0.2.1: Ping is dragged from the power to the enemy hero", () => {
    const { onAction } = renderGame(PING_POWER_LEGAL, activateView({ power: PING_POWER }));

    drag(el(testid.power), el(testid.hero("opponent")));
    expect(el("drag-reticle")).toHaveAttribute("data-target", testid.hero("opponent"));
    act(() => {
      fireEvent.pointerUp(window, { pointerId: 1, button: 0, clientX: 420, clientY: 180 });
    });

    expect(onAction).toHaveBeenCalledWith(powerPing({ pick: "hero", player: "p2" }));
  });

  it("R510 a release on something that is not a target sends nothing and puts the activation down", () => {
    const { onAction } = renderGame([PING_E1, PING_E2]);

    drag(el(testid.activate("act1")), el(testid.card("u1")));
    fireEvent.pointerUp(window, { pointerId: 1, button: 0, clientX: 420, clientY: 180 });

    expect(onAction).not.toHaveBeenCalled();
    expect(el(testid.activate("act1"))).not.toHaveAttribute("data-selected");
  });
});

// ---------------------------------------------------------------------------------------------
// The play's payments in the pickers
// ---------------------------------------------------------------------------------------------

describe("B5 E5 a target that costs discards (Classic #89): the hand picker takes the cards", () => {
  const legal: ActionBody[] = [
    { type: "play", instanceId: "h1", targets: [at("e1")] },
    { type: "play", instanceId: "h1", targets: [at("e2")], discards: ["h2", "h3"] },
    { type: "play", instanceId: "h1", targets: [at("e2")], discards: ["h2", "h4"] },
  ];

  it("after the target, the hand picker offers the listed cards, and two picks confirm the listed body", () => {
    const { onAction } = renderGame(legal);

    fireEvent.click(el(testid.handCard("h1")));
    fireEvent.click(el(testid.card("e2")));
    const modal = el("prompt-modal");
    expect(modal).toHaveAttribute("data-prompt-kind", "hand");
    expect(modal).toHaveTextContent("Discard 2 cards to target it");
    expect(screen.getByTestId("prompt-option-h2")).toBeInTheDocument();
    expect(screen.getByTestId("prompt-option-h4")).toBeInTheDocument();
    expect(screen.queryByTestId("prompt-option-h1")).toBeNull();

    fireEvent.click(el("prompt-option-h4"));
    fireEvent.click(el("prompt-option-h2"));
    fireEvent.click(el("prompt-submit"));

    expect(onAction).toHaveBeenCalledWith(legal[2]);
  });
});

describe("B5 E11 a card in your graveyard that legal lists is played from the pile", () => {
  const mana: ActionBody = { type: "play", instanceId: "gy1", zone: { row: "units", lane: 5 } };
  const tokens: ActionBody = { type: "play", instanceId: "gy1", zone: { row: "units", lane: 5 }, plague: { from: "fs2", tokens: 2 } };

  it("the pile glows, and its sheet offers Play on the listed card only", () => {
    renderGame([mana]);

    const pile = el(testid.graveyard("you"));
    expect(pile).toHaveAttribute("data-glow", "ready");
    fireEvent.click(pile);

    const sheet = el("inspect-list-sheet");
    expect(within(sheet).getByTestId(testid.pilePlay("gy1"))).toHaveTextContent("Play");
    expect(within(sheet).queryByTestId(testid.pilePlay("gy2"))).toBeNull();
    // The opponent's graveyard never offers it.
    expect(el(testid.graveyard("opponent"))).not.toHaveAttribute("data-glow");
  });

  it("Play sends a one-candidate play at once and closes the sheet", () => {
    const { onAction } = renderGame([mana]);

    fireEvent.click(el(testid.graveyard("you")));
    fireEvent.click(el(testid.pilePlay("gy1")));

    expect(onAction).toHaveBeenCalledWith(mana);
    expect(screen.queryByTestId("inspect-list-sheet")).toBeNull();
  });

  it("Play offers the face opened large too", () => {
    const { onAction } = renderGame([mana]);

    fireEvent.click(el(testid.graveyard("you")));
    const sheet = el("inspect-list-sheet");
    const gy1 = within(sheet)
      .getAllByTestId("inspect-list-card")
      .find((element) => element.getAttribute("data-entry-key") === "gy1");
    if (gy1 === undefined) throw new Error("gy1 is not in the sheet");
    fireEvent.click(gy1);
    expect(el("inspect-list-detail")).toBeInTheDocument();

    fireEvent.click(el(testid.pilePlay("gy1")));
    expect(onAction).toHaveBeenCalledWith(mana);
  });

  it("B5 E19 a play that Plague Tokens may pay asks how many in a number picker, and sends the listed body", () => {
    const { onAction } = renderGame([mana, tokens]);

    fireEvent.click(el(testid.graveyard("you")));
    fireEvent.click(el(testid.pilePlay("gy1")));
    expect(onAction).not.toHaveBeenCalled();

    const modal = el("prompt-modal");
    expect(modal).toHaveAttribute("data-prompt-kind", "number");
    const options = within(modal).getAllByRole("button", { pressed: false }).map((button) => button.textContent);
    expect(options).toContain("Pay in mana only");
    expect(options).toContain("Spend 2 Plague Tokens");

    fireEvent.click(within(modal).getByText("Spend 2 Plague Tokens"));
    expect(onAction).toHaveBeenCalledWith(tokens);
  });
});
