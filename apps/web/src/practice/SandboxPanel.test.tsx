// The practice sandbox's dev tools panel (practice/SandboxPanel.tsx, #923): every tool sends the
// engine's `sandbox` op built from the player's picks and nothing else (CLAUDE.md rule 7).

import { cleanup, fireEvent, render, screen, within } from "@testing-library/react";
import { afterEach, describe, expect, it, vi } from "vitest";

import { registeredCatalog } from "@jackioh/engine";
import type { CustomCard, SandboxOp } from "@jackioh/shared";

import { customDefsOf } from "../creator/useCustomCards.ts";
import { baseView, card, emptySide, unit } from "../test/fixtures.ts";
import { SandboxPanel } from "./SandboxPanel.tsx";
import { practiceTestid } from "./testids.ts";

afterEach(() => {
  cleanup();
});

function setup(held = false): { onTool: ReturnType<typeof vi.fn<(op: SandboxOp) => void>>; onHold: ReturnType<typeof vi.fn<(held: boolean) => void>> } {
  const onTool = vi.fn<(op: SandboxOp) => void>();
  const onHold = vi.fn<(held: boolean) => void>();
  const view = baseView({
    you: emptySide("p1", { units: [unit("p1", { instanceId: "mine-1" }), null, null, null, null], hand: [card({ instanceId: "held-1" })] }),
    opponent: emptySide("p2", { hand: { count: 2 }, units: [null, unit("p2", { instanceId: "theirs-1" }), null, null, null] }),
  });
  render(<SandboxPanel view={view} defs={registeredCatalog()} held={held} onHold={onHold} onTool={onTool} />);
  return { onTool, onHold };
}

function type(testid: string, value: string): void {
  fireEvent.change(screen.getByTestId(testid), { target: { value } });
}

describe("R923000 the sandbox's dev tools panel", () => {
  it("R923000 opens over the board, hides to a button and comes back; its hold checkbox asks to hold the AI", () => {
    const { onHold } = setup();
    expect(screen.getByTestId(practiceTestid.sandboxPanel)).toBeInTheDocument();
    fireEvent.click(screen.getByTestId(practiceTestid.sandboxHold));
    expect(onHold).toHaveBeenCalledWith(true);

    fireEvent.click(screen.getByTestId(practiceTestid.sandboxToggle));
    expect(screen.queryByTestId(practiceTestid.sandboxPanel)).toBeNull();
    fireEvent.click(screen.getByTestId(practiceTestid.sandboxToggle));
    expect(screen.getByTestId(practiceTestid.sandboxPanel)).toBeInTheDocument();
  });

  it("R923001 a found card is spawned for the seat and zone picked, Radiant when asked", () => {
    const { onTool } = setup();
    const bigot = Object.values(registeredCatalog()).find((def) => def.name === "Bigot");
    expect(bigot).toBeDefined();
    if (bigot === undefined) return;

    expect(screen.getByTestId(practiceTestid.sandboxSpawn), "nothing picked yet").toBeDisabled();
    type(practiceTestid.sandboxSearch, "bigot");
    fireEvent.click(screen.getByTestId(practiceTestid.sandboxResult(bigot.id)));
    expect(screen.getByTestId(practiceTestid.sandboxResult(bigot.id))).toHaveAttribute("aria-pressed", "true");

    // The hand by default, for the human's seat.
    fireEvent.click(screen.getByTestId(practiceTestid.sandboxSpawn));
    expect(onTool).toHaveBeenLastCalledWith({ kind: "spawn", player: "p1", defId: bigot.id, to: { zone: "hand" } });

    // The AI's unit row, lane 3, Radiant; every zone of the field is offered, and the piles.
    fireEvent.click(screen.getByTestId(practiceTestid.sandboxRadiant));
    fireEvent.change(screen.getByTestId(practiceTestid.sandboxSpawnSeat), { target: { value: "opponent" } });
    const where = screen.getByTestId(practiceTestid.sandboxSpawnWhere);
    const values = within(where)
      .getAllByRole("option")
      .map((option) => (option as HTMLOptionElement).value);
    expect(values).toEqual(["hand", "units:1", "units:2", "units:3", "units:4", "units:5", "backrow:1", "backrow:2", "backrow:3", "backrow:4", "backrow:5", "graveyard", "libraryTop"]);
    fireEvent.change(where, { target: { value: "units:3" } });
    fireEvent.click(screen.getByTestId(practiceTestid.sandboxSpawn));
    expect(onTool).toHaveBeenLastCalledWith({
      kind: "spawn",
      player: "p2",
      defId: bigot.id,
      radiant: true,
      to: { zone: "field", row: "units", lane: 3 },
    });

    fireEvent.change(where, { target: { value: "libraryTop" } });
    fireEvent.click(screen.getByTestId(practiceTestid.sandboxSpawn));
    expect(onTool).toHaveBeenLastCalledWith({ kind: "spawn", player: "p2", defId: bigot.id, radiant: true, to: { zone: "libraryTop" } });
  });

  it("R923002 sets a hero's health and Armor and a player's mana, sending only the values given", () => {
    const { onTool } = setup();
    expect(screen.getByTestId(practiceTestid.sandboxSetHero), "nothing to set yet").toBeDisabled();
    type(practiceTestid.sandboxHealth, "12");
    fireEvent.click(screen.getByTestId(practiceTestid.sandboxSetHero));
    expect(onTool).toHaveBeenLastCalledWith({ kind: "setHero", player: "p1", health: 12 });

    fireEvent.change(screen.getByTestId(practiceTestid.sandboxHeroSeat), { target: { value: "opponent" } });
    type(practiceTestid.sandboxHealth, "");
    type(practiceTestid.sandboxArmor, "5");
    fireEvent.click(screen.getByTestId(practiceTestid.sandboxSetHero));
    expect(onTool).toHaveBeenLastCalledWith({ kind: "setHero", player: "p2", armor: 5 });

    expect(screen.getByTestId(practiceTestid.sandboxSetMana)).toBeDisabled();
    type(practiceTestid.sandboxManaCurrent, "3");
    type(practiceTestid.sandboxManaMax, "7");
    fireEvent.click(screen.getByTestId(practiceTestid.sandboxSetMana));
    expect(onTool).toHaveBeenLastCalledWith({ kind: "setMana", player: "p1", current: 3, max: 7 });
  });

  it("R923002 draws the count asked for the seat picked", () => {
    const { onTool } = setup();
    fireEvent.click(screen.getByTestId(practiceTestid.sandboxDraw));
    expect(onTool).toHaveBeenLastCalledWith({ kind: "draw", player: "p1", count: 1 });
    fireEvent.change(screen.getByTestId(practiceTestid.sandboxDrawSeat), { target: { value: "opponent" } });
    type(practiceTestid.sandboxDrawCount, "3");
    fireEvent.click(screen.getByTestId(practiceTestid.sandboxDraw));
    expect(onTool).toHaveBeenLastCalledWith({ kind: "draw", player: "p2", count: 3 });
    type(practiceTestid.sandboxDrawCount, "");
    expect(screen.getByTestId(practiceTestid.sandboxDraw)).toBeDisabled();
  });

  it("R923003 destroys a card on the field and removes any card it can see; a card in hand is removed, never destroyed", () => {
    const { onTool } = setup();
    const target = screen.getByTestId(practiceTestid.sandboxTarget);
    const offered = within(target)
      .getAllByRole("option")
      .map((option) => (option as HTMLOptionElement).value);
    expect(offered).toEqual(["", "mine-1", "held-1", "theirs-1"]);
    expect(screen.getByTestId(practiceTestid.sandboxDestroy)).toBeDisabled();
    expect(screen.getByTestId(practiceTestid.sandboxRemove)).toBeDisabled();

    fireEvent.change(target, { target: { value: "theirs-1" } });
    fireEvent.click(screen.getByTestId(practiceTestid.sandboxDestroy));
    expect(onTool).toHaveBeenLastCalledWith({ kind: "destroy", instanceId: "theirs-1" });
    fireEvent.click(screen.getByTestId(practiceTestid.sandboxRemove));
    expect(onTool).toHaveBeenLastCalledWith({ kind: "remove", instanceId: "theirs-1" });

    fireEvent.change(target, { target: { value: "held-1" } });
    expect(screen.getByTestId(practiceTestid.sandboxDestroy)).toBeDisabled();
    fireEvent.click(screen.getByTestId(practiceTestid.sandboxRemove));
    expect(onTool).toHaveBeenLastCalledWith({ kind: "remove", instanceId: "held-1" });
  });
});

/** A 1-cost 1/2 Unit with no script, as a game carries it (R915006). */
const IMP: CustomCard = {
  id: "custom-ember-imp",
  version: 1,
  frame: { name: "Ember Imp", type: "Unit", cost: 1, rarity: "Common", attack: 1, health: 2, text: "" },
  source: "",
};

describe("R962000 the panel offers a sandbox game's own custom cards", () => {
  it("R962000 a sandbox game's own custom card is offered only when given, and spawned like a shipped one", () => {
    const onTool = vi.fn<(op: SandboxOp) => void>();
    const panel = (customs?: ReturnType<typeof customDefsOf>) => (
      <SandboxPanel
        view={baseView({ you: emptySide("p1"), opponent: emptySide("p2") })}
        defs={registeredCatalog()}
        {...(customs === undefined ? {} : { customs })}
        held={false}
        onHold={vi.fn()}
        onTool={onTool}
      />
    );
    const { rerender } = render(panel());
    type(practiceTestid.sandboxSearch, "ember imp");
    expect(screen.queryByTestId(practiceTestid.sandboxResult(IMP.id)), "no game card without the game's").toBeNull();

    rerender(panel(customDefsOf([IMP])));
    fireEvent.click(screen.getByTestId(practiceTestid.sandboxResult(IMP.id)));
    expect(screen.getByRole("heading", { name: "Spawn Ember Imp" })).toBeInTheDocument();
    fireEvent.change(screen.getByTestId(practiceTestid.sandboxSpawnWhere), { target: { value: "units:2" } });
    fireEvent.click(screen.getByTestId(practiceTestid.sandboxSpawn));
    expect(onTool).toHaveBeenLastCalledWith({ kind: "spawn", player: "p1", defId: IMP.id, to: { zone: "field", row: "units", lane: 2 } });
  });
});
