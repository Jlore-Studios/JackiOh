// #3864 (R3864000): a choice of one board zone (a play's zone, a `cell` prompt, a declared zone target) is
// answered on the board alone: the zones glow and the modal is the hint, a play's Cancel and any option that
// names no cell. Driven through `<Game/>` as a player drives it; every action expected is a body `legal` lists
// (CLAUDE.md rule 7).

import { CATALOG } from "@jackioh/cards";
import type { ActionBody, PendingOption, PlayerView } from "@jackioh/shared";
import { cleanup, fireEvent, render, screen } from "@testing-library/react";
import { afterEach, beforeEach, describe, expect, it, vi } from "vitest";

import { closeInspect } from "../cards/index.ts";
import { __resetSettingsForTests } from "../settings/index.ts";
import { baseView, card, emptySide, faceDownBackrow, faceUpBackrow, pendingFor, unit } from "../test/fixtures.ts";
import { highlightFor, IDLE } from "./actions.ts";
import Board from "./Board.tsx";
import { CatalogContext, lookupFromDefs } from "./catalog.ts";
import Game from "./Game.tsx";
import Prompt from "./Prompt.tsx";

const lookup = lookupFromDefs(CATALOG);
const HINT = "Tap a highlighted zone on the board.";

const el = (id: string): HTMLElement => screen.getByTestId(id);
const optionIds = (): string[] =>
  [...document.querySelectorAll<HTMLElement>('[data-testid^="prompt-option-"]')].map((node) => node.getAttribute("data-testid") ?? "");
const hint = (): string | null => document.querySelector(".prompt-board-hint")?.textContent ?? null;

const playZone = (lane: number, row: "units" | "backrow" = "units"): ActionBody => ({ type: "play", instanceId: "h1", zone: { row, lane } });
const playTargets = (...targets: Extract<ActionBody, { type: "play" }>["targets"] & object): ActionBody => ({ type: "play", instanceId: "h1", targets });

function handView(over: Partial<PlayerView> = {}): PlayerView {
  return baseView({
    you: emptySide("p1", {
      hand: [card({ instanceId: "h1", defId: "core-008", cost: 1 })],
      units: [null, unit("p1", { instanceId: "u2" }), null, null, null],
      backrow: [null, faceUpBackrow("p1", { instanceId: "tower", defId: "classicplus-033" }), null, null, null],
    }),
    opponent: emptySide("p2", { hand: { count: 3 }, units: [null, null, null, null, unit("p2", { instanceId: "e5" })] }),
    ...over,
  });
}

function startPlay(legal: readonly ActionBody[], view: PlayerView = handView()) {
  const onAction = vi.fn();
  render(
    <CatalogContext.Provider value={lookup}>
      <Game view={view} legal={legal} onAction={onAction} />
    </CatalogContext.Provider>,
  );
  fireEvent.click(el("hand-card-h1"));
  return onAction;
}

describe("R3864000 a play's zone", () => {
  const legal = [playZone(3), playZone(4), playZone(2, "backrow")];

  it("R3864000 a play's zone opens no list, only the hint and Cancel, and a tap on a lit zone sends the play", () => {
    const onAction = startPlay(legal);
    expect(el("prompt-modal")).toHaveAttribute("data-prompt-kind", "zone");
    expect(document.querySelector(".prompt-zones")).toBeNull();
    expect(optionIds()).toEqual([]);
    expect(hint()).toBe(HINT);
    expect(el("prompt-cancel")).toBeInTheDocument();
    expect(el("zone-you-units-3").getAttribute("data-glow")).toBe("ready");

    fireEvent.click(el("zone-you-units-3"));
    expect(onAction).toHaveBeenCalledWith(playZone(3));
  });

  it("R3864000 Enter on a focused lit zone places the card, and a click on the card standing in a covered zone does too (R446)", () => {
    const onAction = startPlay(legal);
    const zone = el("zone-you-units-4");
    expect(zone).toHaveAttribute("tabindex", "0");
    zone.focus();
    expect(document.activeElement).toBe(zone);
    fireEvent.keyDown(zone, { key: "Enter" });
    expect(onAction).toHaveBeenLastCalledWith(playZone(4));
    cleanup();

    const again = startPlay(legal);
    fireEvent.click(el("card-tower"));
    expect(again).toHaveBeenCalledWith(playZone(2, "backrow"));
  });

  it("R3864000 Cancel drops the play", () => {
    startPlay(legal);
    fireEvent.click(el("prompt-cancel"));
    expect(screen.queryByTestId("prompt-modal")).toBeNull();
  });
});

describe("R3864000 a declared target", () => {
  const zoneTarget = (lane: number): ActionBody => ({ type: "play", instanceId: "h1", targets: [{ pick: "zone", player: "p2", row: "units", lane }] });

  it("R3864000 a declared target of one zone is board only, and Enter on a lit zone sends the play", () => {
    const onAction = startPlay([zoneTarget(2), zoneTarget(3)]);
    expect(el("prompt-modal")).toHaveAttribute("data-prompt-kind", "target");
    expect(optionIds()).toEqual([]);
    expect(hint()).toBe(HINT);
    expect(el("prompt-cancel")).toBeInTheDocument();

    const zone = el("zone-opponent-units-3");
    zone.focus();
    fireEvent.keyDown(zone, { key: " " });
    expect(onAction).toHaveBeenCalledWith(zoneTarget(3));
  });

  it("R3864000 a target pick offering a card and a zone keeps its list", () => {
    startPlay([
      { type: "play", instanceId: "h1", targets: [{ pick: "instance", instanceId: "e5" }] },
      zoneTarget(3),
    ]);
    expect(optionIds()).toEqual(["prompt-option-instance:e5", "prompt-option-zone:p2:units:3"]);
    expect(el("prompt-modal")).not.toHaveAttribute("data-board-pick");
  });

  it("R3864000 a target pick offering a hero keeps its list", () => {
    startPlay([playTargets({ pick: "hero", player: "p2" }), zoneTarget(3)]);
    expect(optionIds()).toContain("prompt-option-hero:p2");
  });
});

describe("R3864000 a cell prompt", () => {
  const LANES_LEFT = [1, 3, 4, 5];

  function cellOptions(): PendingOption[] {
    const options: PendingOption[] = [];
    for (const [player, rows] of [
      ["p1", ["backrow", "units"]],
      ["p2", ["units", "backrow"]],
    ] as const) {
      for (const row of rows) {
        for (const lane of LANES_LEFT) {
          options.push({ key: `zone:${player}:${row}:${String(lane)}`, label: `${player} ${row} ${String(lane)}`, player, row, lane });
        }
      }
    }
    options.push({ key: "none", label: "Done" });
    return options;
  }

  function cellView(over: Partial<Extract<NonNullable<PlayerView["pending"]>, { forYou: true }>> = {}): PlayerView {
    return baseView({
      you: emptySide("p1", { units: [unit("p1", { instanceId: "u1" }), null, null, null, null] }),
      opponent: emptySide("p2", {
        hand: { count: 3 },
        units: [null, null, unit("p2", { instanceId: "e3" }), null, null],
        backrow: [faceDownBackrow, null, null, null, null],
      }),
      pending: pendingFor("cell", cellOptions(), { prompt: "Choose a point for the curve", ...over }),
    });
  }

  function renderBoardAndPrompt(view: PlayerView, readOnly = false) {
    const onAction = vi.fn();
    render(
      <CatalogContext.Provider value={lookup}>
        <Board view={view} highlight={highlightFor(view, [], IDLE)} onClick={() => undefined} />
        <Prompt view={view} legal={[]} onAction={onAction} readOnly={readOnly} />
      </CatalogContext.Provider>,
    );
    return onAction;
  }

  const answered = (onAction: ReturnType<typeof vi.fn>, selection: unknown): void => {
    expect(onAction).toHaveBeenCalledWith({ type: "answer", choiceId: "ch1", selection });
  };

  it("R3864000 a cell prompt draws no cell list, keeps Done as a button, and a tap on a lit zone answers", () => {
    const onAction = renderBoardAndPrompt(cellView());
    expect(document.querySelector(".prompt-zones")).toBeNull();
    expect(document.querySelector(".prompt-zone-row")).toBeNull();
    expect(optionIds()).toEqual(["prompt-option-none"]);
    expect(hint()).toBe(HINT);

    fireEvent.click(el("prompt-option-none"));
    answered(onAction, [{ pick: "none" }]);
    fireEvent.click(el("zone-opponent-units-3"));
    answered(onAction, [{ pick: "zone", player: "p2", row: "units", lane: 3 }]);
  });

  it("R3864000 a cell prompt opens with the first lit zone focused, and Enter on another lit zone answers", () => {
    const onAction = renderBoardAndPrompt(cellView());
    expect(document.activeElement).toBe(el("zone-you-backrow-1"));
    const other = el("zone-you-backrow-5");
    expect(other).toHaveAttribute("tabindex", "0");
    other.focus();
    fireEvent.keyDown(other, { key: "Enter" });
    answered(onAction, [{ pick: "zone", player: "p1", row: "backrow", lane: 5 }]);
  });

  it("R3864000 a cell prompt that may pick several cells keeps its list and Confirm", () => {
    renderBoardAndPrompt(cellView({ max: 2 }));
    expect(document.querySelector(".prompt-zones")).not.toBeNull();
    expect(el("prompt-option-zone:p2:units:3")).toBeInTheDocument();
    expect(el("prompt-submit")).toBeInTheDocument();
    expect(el("prompt-modal")).not.toHaveAttribute("data-board-pick");
  });

  it("R3864000 a replay shows the title and no list for a cell prompt", () => {
    renderBoardAndPrompt(cellView(), true);
    expect(el("prompt-modal")).toHaveTextContent("Choose a point for the curve");
    expect(document.querySelector(".prompt-zones")).toBeNull();
    expect(optionIds()).toEqual(["prompt-option-none"]);
    expect(el("prompt-option-none")).toHaveAttribute("aria-disabled", "true");
  });

  it("R3864000 a Discover of cells (a mode prompt) keeps its choice cards", () => {
    const view = baseView({
      pending: pendingFor("mode", [
        { key: "zone:p1:units:1", label: "Your Unit lane 1" },
        { key: "zone:p1:units:3", label: "Your Unit lane 3" },
      ]),
    });
    render(
      <CatalogContext.Provider value={lookup}>
        <Prompt view={view} legal={[]} onAction={vi.fn()} />
      </CatalogContext.Provider>,
    );
    expect(el("prompt-cards")).toBeInTheDocument();
    expect(el("prompt-modal")).not.toHaveAttribute("data-board-pick");
  });
});

beforeEach(() => {
  __resetSettingsForTests();
});

afterEach(() => {
  cleanup();
  closeInspect();
  vi.restoreAllMocks();
  __resetSettingsForTests();
});
