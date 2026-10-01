// The v0.2.0 controls against the REAL engine. The fixture tests (ActivateControl, activate,
// PromptE18) pin each control's behaviour; here the view and the legal list are `viewFor` and
// `legalActions` of real states built by the cards harness, so a field the engine renames or a body
// it shapes differently fails here, not in production. Every action a control sends must be one the
// engine accepts (CLAUDE.md rule 7).

import { CATALOG } from "@jackioh/cards";
import { legalActions, reduce } from "@jackioh/engine";
import type { Action, ActionBody, PlayerId } from "@jackioh/shared";
import { act, cleanup, fireEvent, render, screen } from "@testing-library/react";
import { afterEach, beforeEach, describe, expect, it, vi } from "vitest";

import { scenario, type Scenario } from "../../../../packages/cards/test/_harness";
import { closeInspect } from "../cards/index.ts";
import { __resetSettingsForTests } from "../settings/index.ts";
import { CatalogContext, lookupFromDefs } from "./catalog.ts";
import { testid } from "./contract.ts";
import Game from "./Game.tsx";

const lookup = lookupFromDefs(CATALOG);
const el = (id: string): HTMLElement => screen.getByTestId(id);

beforeEach(() => {
  __resetSettingsForTests();
});

afterEach(() => {
  act(() => {
    closeInspect();
  });
  cleanup();
});

/** `<Game/>` on `player`'s real view and legal list; returns the spy every control reports to. */
function renderReal(s: Scenario, player: PlayerId = "p1") {
  const legal = legalActions(s.state, player);
  const onAction = vi.fn<(body: ActionBody) => void>();
  render(
    <CatalogContext.Provider value={lookup}>
      <Game view={s.view(player)} legal={legal} onAction={onAction} />
    </CatalogContext.Provider>,
  );
  return { legal, onAction };
}

/** The one body the controls sent, which the engine must list and accept. */
function sent(s: Scenario, legal: readonly ActionBody[], onAction: ReturnType<typeof vi.fn>, player: PlayerId = "p1"): ActionBody {
  expect(onAction).toHaveBeenCalledTimes(1);
  const body = onAction.mock.calls[0]?.[0] as ActionBody;
  expect(legal).toContainEqual(body);
  expect(reduce(s.state, { ...body, playerId: player, nonce: "v020-real" } as Action).error).toBeUndefined();
  return body;
}

describe("the Activate control on real views", () => {
  it("C #23 Devil's Pact: the Field Spell's one ability is a control whose press sends the listed activate", () => {
    const s = scenario({ p1: { backrow: ["classic-023"], field: ["core-008"], mana: 10 }, p2: { field: ["core-008"] } });
    const pact = s.backrow("p1", 1);
    if (pact === null) throw new Error("Devil's Pact is not in the backrow");
    const { legal, onAction } = renderReal(s);

    expect(el(testid.activate(pact.id))).toHaveAttribute("data-legal", "true");
    fireEvent.click(el(testid.activate(pact.id)));
    expect(sent(s, legal, onAction)).toEqual({ type: "activate", instanceId: pact.id, ability: "pact" });
  });

  it("C #21 Turtinator: the press asks for the Tribute, then the target, and sends a body the engine lists", () => {
    const s = scenario({ p1: { field: ["classic-021", "core-011"] }, p2: { field: ["core-008"] } });
    const turtle = s.unit("p1", 1);
    const timmy = s.unit("p1", 2);
    if (turtle === null || timmy === null) throw new Error("the board is not set");
    const { legal, onAction } = renderReal(s);

    fireEvent.click(el(testid.activate(turtle.id)));
    expect(el("prompt-modal")).toHaveAttribute("data-prompt-kind", "tribute");
    fireEvent.click(el(testid.card(timmy.id)));
    fireEvent.click(el(testid.hero("opponent")));
    expect(sent(s, legal, onAction)).toEqual(
      expect.objectContaining({ type: "activate", instanceId: turtle.id, ability: "eat", tributes: [timmy.id], targets: [{ pick: "hero", player: "p2" }] }),
    );
  });
});

describe("the E18 pickers on real views", () => {
  it("C #18 Glitch in the System: the play's number is a number picker from 0 to 10, and the pick sends that play", () => {
    const s = scenario({ p1: { hand: ["classic-018"], field: ["core-012"] }, p2: { field: ["core-008"] } });
    const glitch = s.hand("p1")[0];
    if (glitch === undefined) throw new Error("Glitch is not in hand");
    const { legal, onAction } = renderReal(s);

    fireEvent.click(el(testid.handCard(glitch.id)));
    expect(el("prompt-modal")).toHaveAttribute("data-prompt-kind", "number");
    expect(screen.getAllByTestId(/^prompt-option-/).map((option) => option.textContent)).toEqual(
      ["0", "1", "2", "3", "4", "5", "6", "7", "8", "9", "10"],
    );
    fireEvent.click(el("prompt-option-2"));
    expect(sent(s, legal, onAction)).toEqual({ type: "play", instanceId: glitch.id, modes: ["2"] });
  });

  it("C #44 Back from the GY: the budgeted pick shows the engine's budget and costs, and its answer is accepted", () => {
    const s = scenario({
      p1: { hand: ["classic-044", "core-008"], graveyard: ["core-002", "core-019", "core-025"] },
      p2: { hand: ["core-008"] },
    });
    s.play("classic-044");
    const pending = s.view("p1").pending;
    if (pending === null || !pending.forYou) throw new Error("no pick is open for p1");
    expect(pending.kind).toBe("pick");
    const { legal, onAction } = renderReal(s);

    expect(el("prompt-modal")).toHaveAttribute("data-prompt-kind", "pick");
    expect(el("prompt-budget").textContent).toBe(`(0) of (${String(pending.budget)}) spent`);
    const cheapest = [...pending.options].sort((a, b) => (a.cost ?? 0) - (b.cost ?? 0))[0];
    if (cheapest === undefined) throw new Error("the pick offers nothing");
    fireEvent.click(el(`prompt-option-${cheapest.key}`));
    expect(el("prompt-budget").textContent).toBe(`(${String(cheapest.cost)}) of (${String(pending.budget)}) spent`);
    fireEvent.click(el("prompt-submit"));
    expect(sent(s, legal, onAction)).toEqual(expect.objectContaining({ type: "answer", selection: [{ pick: "instance", instanceId: cheapest.instanceId }] }));
  });
});
