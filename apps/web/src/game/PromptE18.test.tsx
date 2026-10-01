// B5 E18 (SPEC §10.6): the pickers for patch v0.2.0's prompt kinds — `number`, `answer`, `cell`,
// `reward` and `pick` — and the two cases E17 and E18 add to the older kinds: a `mode` prompt the
// other player holds (C #8 Pickle) and the opponent's hand as a chooser's options (C #11 Mind Melt).
// Every view is a fixture shaped as the engine's `viewFor` builds it (option keys `mode:<x>`,
// `zone:<player>:<row>:<lane>`, `instance:<id>`); each picker must send exactly
// `{ type: "answer", choiceId, selection }` with the selections its options name.

import { readFileSync } from "node:fs";
import { dirname, join } from "node:path";
import { fileURLToPath } from "node:url";

import { act, cleanup, fireEvent, render, screen } from "@testing-library/react";
import { afterEach, describe, expect, it, vi } from "vitest";

import { CATALOG } from "@jackioh/cards";
import type { ActionBody, PendingOption, PlayerView } from "@jackioh/shared";

import { closeInspect } from "../cards/index.ts";
import Board from "./Board.tsx";
import { IDLE, highlightFor, type Interaction } from "./actions.ts";
import { CatalogContext, lookupFromDefs } from "./catalog.ts";
import Prompt from "./Prompt.tsx";
import { baseView, card, emptySide, faceDownBackrow, pendingFor, unit } from "../test/fixtures.ts";

afterEach(() => {
  act(() => {
    closeInspect();
  });
  cleanup();
});

const lookup = lookupFromDefs(CATALOG);

function nameOf(defId: string): string {
  const name = CATALOG[defId]?.name;
  if (name === undefined) throw new Error(`the catalog has no ${defId}`);
  return name;
}

function renderPrompt(view: PlayerView, props: { interaction?: Interaction; legal?: ActionBody[]; onCancel?: () => void } = {}) {
  const onAction = vi.fn();
  render(
    <CatalogContext.Provider value={lookup}>
      <Prompt
        view={view}
        onAction={onAction}
        {...(props.interaction === undefined ? {} : { interaction: props.interaction })}
        {...(props.legal === undefined ? {} : { legal: props.legal })}
        {...(props.onCancel === undefined ? {} : { onCancel: props.onCancel })}
      />
    </CatalogContext.Provider>,
  );
  return onAction;
}

function modal(): HTMLElement {
  return screen.getByTestId("prompt-modal");
}

function optionKeys(): string[] {
  return [...document.querySelectorAll("[data-testid^='prompt-option-']")].map((node) =>
    (node.getAttribute("data-testid") ?? "").slice("prompt-option-".length),
  );
}

function answered(onAction: ReturnType<typeof vi.fn>, selection: unknown[]): void {
  expect(onAction).toHaveBeenCalledTimes(1);
  expect(onAction).toHaveBeenCalledWith({ type: "answer", choiceId: "ch1", selection });
}

const NUMBERS = Array.from({ length: 11 }, (_, at) => String(at));

// ---------------------------------------------------------------------------------------------
// number
// ---------------------------------------------------------------------------------------------

describe("number (C #18)", () => {
  function numberView(): PlayerView {
    return baseView({
      pending: pendingFor(
        "number",
        NUMBERS.map((n) => ({ key: `mode:${n}`, label: n })),
        { prompt: "Choose a number" },
      ),
    });
  }

  it("draws the offered numbers as a pad of keys, takes the focus, and a key sends its number", () => {
    const onAction = renderPrompt(numberView());
    expect(modal()).toHaveAttribute("data-prompt-kind", "number");
    expect(modal()).toHaveAttribute("data-prompt-source", "engine");
    expect(optionKeys()).toEqual(NUMBERS.map((n) => `mode:${n}`));
    expect(document.activeElement).toBe(screen.getByTestId("prompt-option-mode:0"));

    fireEvent.click(screen.getByTestId("prompt-option-mode:7"));
    answered(onAction, [{ pick: "mode", option: "7" }]);
  });

  it("the arrow keys walk the pad, six keys to a row, and move the focus only", () => {
    const onAction = renderPrompt(numberView());
    const pad = screen.getByRole("group", { name: "Numbers" });
    expect(pad.style.getPropertyValue("--pad-columns")).toBe("6");

    fireEvent.keyDown(document.activeElement ?? pad, { key: "ArrowRight" });
    expect(document.activeElement).toBe(screen.getByTestId("prompt-option-mode:1"));
    fireEvent.keyDown(document.activeElement ?? pad, { key: "ArrowDown" });
    expect(document.activeElement).toBe(screen.getByTestId("prompt-option-mode:7"));
    fireEvent.keyDown(document.activeElement ?? pad, { key: "ArrowLeft" });
    expect(document.activeElement).toBe(screen.getByTestId("prompt-option-mode:6"));
    fireEvent.keyDown(document.activeElement ?? pad, { key: "ArrowUp" });
    expect(document.activeElement).toBe(screen.getByTestId("prompt-option-mode:0"));
    // Past the edge the focus stays where it is.
    fireEvent.keyDown(document.activeElement ?? pad, { key: "ArrowUp" });
    expect(document.activeElement).toBe(screen.getByTestId("prompt-option-mode:0"));
    expect(onAction).not.toHaveBeenCalled();
  });

  it("R81 C #18's number travels in its play: the play's number pad sends the play with that mode", () => {
    const glitch = card({ instanceId: "h1", defId: "classic-018", cost: 3 });
    const view = baseView({ you: emptySide("p1", { hand: [glitch] }) });
    const candidates: ActionBody[] = NUMBERS.map((n) => ({ type: "play", instanceId: "h1", modes: [n] }));
    const interaction: Interaction = { stage: "playing", instanceId: "h1", candidates, picked: {} };
    const onCancel = vi.fn();
    const onAction = renderPrompt(view, { interaction, legal: candidates, onCancel });

    expect(modal()).toHaveAttribute("data-prompt-kind", "number");
    expect(modal()).toHaveAttribute("data-prompt-source", "play");
    expect(modal()).toHaveTextContent(nameOf("classic-018"));
    expect(optionKeys()).toEqual(NUMBERS);

    fireEvent.click(screen.getByTestId("prompt-cancel"));
    expect(onCancel).toHaveBeenCalledTimes(1);
    fireEvent.click(screen.getByTestId("prompt-option-4"));
    expect(onAction).toHaveBeenCalledWith({ type: "play", instanceId: "h1", modes: ["4"] });
  });

  it("an engine prompt has no Cancel: it has paused the game and must be answered", () => {
    renderPrompt(numberView());
    expect(screen.queryByTestId("prompt-cancel")).toBeNull();
  });
});

// ---------------------------------------------------------------------------------------------
// answer
// ---------------------------------------------------------------------------------------------

describe("answer (C+ #42)", () => {
  const PROBLEM = "∫₀¹ 2x dx = ?\n(choose one)";
  const ANSWERS = ["1", "2", "0", "½"];

  function quizView(): PlayerView {
    return baseView({
      pending: pendingFor(
        "answer",
        ANSWERS.map((text, at) => ({ key: `mode:${"ABCD"[at] ?? ""}`, label: text })),
        { prompt: PROBLEM },
      ),
    });
  }

  it("R420 shows the problem and its answers lettered A to D, and a press sends that answer", () => {
    const onAction = renderPrompt(quizView());
    expect(modal()).toHaveAttribute("data-prompt-kind", "answer");
    expect(modal().querySelector(".prompt-title")?.textContent).toBe(PROBLEM);

    const letters = ["A", "B", "C", "D"].map((letter) => screen.getByTestId(`prompt-option-mode:${letter}`));
    expect(letters.map((button) => button.getAttribute("data-letter"))).toEqual(["A", "B", "C", "D"]);
    expect(letters.map((button) => button.getAttribute("aria-label"))).toEqual(["A: 1", "B: 2", "C: 0", "D: ½"]);
    expect(document.activeElement).toBe(letters[0]);

    fireEvent.keyDown(letters[0] as HTMLElement, { key: "ArrowDown" });
    expect(document.activeElement).toBe(letters[1]);

    fireEvent.click(letters[2] as HTMLElement);
    answered(onAction, [{ pick: "mode", option: "C" }]);
  });

  it("R420 nothing in the quiz tells one answer from another but its letter and words", () => {
    renderPrompt(quizView());
    const shapes = optionKeys().map((key) =>
      [...screen.getByTestId(`prompt-option-${key}`).attributes]
        .map((attribute) => attribute.name)
        .sort()
        .join(" "),
    );
    expect(new Set(shapes).size).toBe(1);
  });
});

// ---------------------------------------------------------------------------------------------
// cell
// ---------------------------------------------------------------------------------------------

describe("cell (C+ #62)", () => {
  const LANES_LEFT = [1, 3, 4, 5];

  /** One Papaya prompt with lane 2 used: the 4 cells of every other lane, and "done". */
  function cellView(): PlayerView {
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
    return baseView({
      you: emptySide("p1", { units: [unit("p1", { instanceId: "u1" }), null, null, null, null] }),
      opponent: emptySide("p2", {
        hand: { count: 3 },
        units: [null, null, unit("p2", { instanceId: "e3" }), null, null],
        backrow: [faceDownBackrow, null, null, null, null],
      }),
      pending: pendingFor("cell", options, { prompt: "Choose a point for the curve" }),
    });
  }

  function renderBoardAndPrompt(view: PlayerView) {
    const onAction = vi.fn();
    render(
      <CatalogContext.Provider value={lookup}>
        <Board view={view} highlight={highlightFor(view, [], IDLE)} onClick={() => undefined} />
        <Prompt view={view} legal={[]} onAction={onAction} />
      </CatalogContext.Provider>,
    );
    return onAction;
  }

  it("R514 lists the cells by side and row, lane by lane, and a cell or Done answers from the list", () => {
    const onAction = renderPrompt(cellView());
    expect(modal()).toHaveAttribute("data-prompt-kind", "cell");
    const rows = [...modal().querySelectorAll(".prompt-zone-row")].map((node) => node.textContent);
    expect(rows).toEqual(["Your backrow", "Your units", "Enemy units", "Enemy backrow"]);
    expect(screen.getByTestId("prompt-option-zone:p2:units:3")).toHaveTextContent("Lane 3");
    expect(document.activeElement).toBe(screen.getByTestId("prompt-option-zone:p1:backrow:1"));

    fireEvent.click(screen.getByTestId("prompt-option-zone:p2:units:3"));
    answered(onAction, [{ pick: "zone", player: "p2", row: "units", lane: 3 }]);
  });

  it("R514 Done picks no cell", () => {
    const onAction = renderPrompt(cellView());
    fireEvent.click(screen.getByTestId("prompt-option-none"));
    answered(onAction, [{ pick: "none" }]);
  });

  it("R514 the offered cells glow on the board on both sides, filled or empty, and no other zone does", () => {
    renderBoardAndPrompt(cellView());
    for (const side of ["you", "opponent"] as const) {
      for (const row of ["units", "backrow"] as const) {
        for (const lane of [1, 2, 3, 4, 5]) {
          const zone = screen.getByTestId(`zone-${side}-${row}-${String(lane)}`);
          expect(zone.getAttribute("data-glow"), `${side} ${row} ${String(lane)}`).toBe(LANES_LEFT.includes(lane) ? "ready" : null);
        }
      }
    }
    expect(modal().getAttribute("data-board-testids")?.split(" ")).toContain("zone-opponent-units-3");
  });

  it("R514 a click on an offered zone answers with its cell, on whatever stands in it too", () => {
    const onAction = renderBoardAndPrompt(cellView());
    fireEvent.click(screen.getByTestId("card-e3"));
    answered(onAction, [{ pick: "zone", player: "p2", row: "units", lane: 3 }]);
  });

  it("R514 an empty offered zone answers too, and a zone not on offer answers nothing", () => {
    const onAction = renderBoardAndPrompt(cellView());
    fireEvent.click(screen.getByTestId("zone-you-units-2"));
    expect(onAction).not.toHaveBeenCalled();
    fireEvent.click(screen.getByTestId("zone-opponent-backrow-4"));
    answered(onAction, [{ pick: "zone", player: "p2", row: "backrow", lane: 4 }]);
  });

  it("R514 Enter on a focused offered zone answers, as a click does", () => {
    const onAction = renderBoardAndPrompt(cellView());
    const zone = screen.getByTestId("zone-you-backrow-5");
    zone.focus();
    fireEvent.keyDown(zone, { key: "Enter" });
    answered(onAction, [{ pick: "zone", player: "p1", row: "backrow", lane: 5 }]);
  });

  it("R514 the board listens only while the cell prompt is open", () => {
    const onAction = vi.fn();
    const view = cellView();
    const { rerender } = render(
      <CatalogContext.Provider value={lookup}>
        <Board view={view} highlight={highlightFor(view, [], IDLE)} onClick={() => undefined} />
        <Prompt view={view} legal={[]} onAction={onAction} />
      </CatalogContext.Provider>,
    );
    const answeredView = { ...view, pending: null };
    rerender(
      <CatalogContext.Provider value={lookup}>
        <Board view={answeredView} highlight={highlightFor(answeredView, [], IDLE)} onClick={() => undefined} />
        <Prompt view={answeredView} legal={[]} onAction={onAction} />
      </CatalogContext.Provider>,
    );
    fireEvent.click(screen.getByTestId("zone-opponent-units-3"));
    expect(onAction).not.toHaveBeenCalled();
  });

  it("R514 prompt.css marks the glowing zones with a crosshair while a cell prompt is open", () => {
    const css = readFileSync(join(dirname(fileURLToPath(import.meta.url)), "prompt.css"), "utf8");
    expect(css).toContain('.game:has([data-prompt-kind="cell"]) .zone[data-glow="ready"]::after');
  });
});

// ---------------------------------------------------------------------------------------------
// reward
// ---------------------------------------------------------------------------------------------

describe("reward (C #90)", () => {
  it("R404 offers a completed quest's rewards as tiles under Quest complete!, and a tile takes its reward", () => {
    const onAction = renderPrompt(
      baseView({
        pending: pendingFor(
          "reward",
          [
            { key: "mode:A", label: "Heal your hero 6" },
            { key: "mode:B", label: "Deal 3 damage" },
          ],
          { prompt: "Quest complete: Draw 2 cards" },
        ),
      }),
    );
    expect(modal()).toHaveAttribute("data-prompt-kind", "reward");
    expect(screen.getByTestId("prompt-quest-complete")).toHaveTextContent("Quest complete!");
    expect(modal().querySelector(".prompt-title")?.textContent).toBe("Draw 2 cards");
    expect(screen.getByTestId("prompt-option-mode:A")).toHaveTextContent("Heal your hero 6");
    expect(document.activeElement).toBe(screen.getByTestId("prompt-option-mode:A"));

    fireEvent.click(screen.getByTestId("prompt-option-mode:B"));
    answered(onAction, [{ pick: "mode", option: "B" }]);
  });
});

// ---------------------------------------------------------------------------------------------
// pick
// ---------------------------------------------------------------------------------------------

describe("pick (C #34, C #44)", () => {
  /** C #44 Back from the GY: Units from your graveyard with a total cost of (5) or less. */
  function graveyardView(over: { min?: number; max?: number; budget?: number } = {}): PlayerView {
    return baseView({
      you: emptySide("p1", {
        graveyard: [
          card({ instanceId: "g1", defId: "core-002", cost: 2 }),
          card({ instanceId: "g2", defId: "core-019", cost: 3 }),
          card({ instanceId: "g3", defId: "core-043", cost: 4, radiant: true }),
        ],
      }),
      pending: pendingFor(
        "pick",
        [
          { key: "instance:g1", label: "Bigot", instanceId: "g1", defId: "core-002", cost: 2 },
          { key: "instance:g2", label: "Midrange Menace", instanceId: "g2", defId: "core-019", cost: 3 },
          { key: "instance:g3", label: "Big Felinor", instanceId: "g3", defId: "core-043", cost: 4, radiant: true },
        ],
        { prompt: "Summon Units with a total cost of (5) or less", min: over.min ?? 0, max: over.max ?? 3, ...(over.budget === undefined ? {} : { budget: over.budget }) },
      ),
    });
  }

  const option = (id: string): HTMLElement => screen.getByTestId(`prompt-option-instance:${id}`);
  const greyed = (): string[] =>
    ["g1", "g2", "g3"].filter((id) => option(id).getAttribute("data-over-budget") === "true");

  it("draws each option as its card's face, Radiant where it is", () => {
    renderPrompt(graveyardView({ budget: 5 }));
    expect(modal()).toHaveAttribute("data-prompt-kind", "pick");
    expect(option("g1").querySelector(".cf .card-name")?.textContent).toBe(nameOf("core-002"));
    expect(option("g3").querySelector(".cf")).toHaveAttribute("data-radiant-face", "true");
    expect(option("g2").getAttribute("aria-label")).toBe(`${nameOf("core-019")}, costs (3)`);
  });

  it("R515 keeps a running total, greys what would go over the budget, and refuses to add it", () => {
    const onAction = renderPrompt(graveyardView({ budget: 5 }));
    const total = (): string => screen.getByTestId("prompt-budget").textContent ?? "";
    expect(total()).toBe("(0) of (5) spent");
    expect(greyed()).toEqual([]);

    fireEvent.click(option("g2"));
    expect(total()).toBe("(3) of (5) spent");
    expect(greyed()).toEqual(["g3"]);
    expect(option("g3")).toHaveAttribute("aria-disabled", "true");
    expect(option("g3")).toHaveAttribute("title", "Over the budget");

    fireEvent.click(option("g3"));
    expect(option("g3")).toHaveAttribute("aria-pressed", "false");
    expect(total()).toBe("(3) of (5) spent");

    fireEvent.click(option("g1"));
    expect(total()).toBe("(5) of (5) spent");

    // Picked g2 first, g1 second: the answer names them in the order offered.
    fireEvent.click(screen.getByTestId("prompt-submit"));
    answered(onAction, [
      { pick: "instance", instanceId: "g1" },
      { pick: "instance", instanceId: "g2" },
    ]);
  });

  it("R515 taking a card back frees the budget it held", () => {
    renderPrompt(graveyardView({ budget: 5 }));
    fireEvent.click(option("g2"));
    expect(greyed()).toEqual(["g3"]);
    fireEvent.click(option("g2"));
    expect(greyed()).toEqual([]);
    expect(screen.getByTestId("prompt-budget")).toHaveTextContent("(0) of (5) spent");
  });

  it("respects min and max: Confirm waits for min, and a pick past max is not taken", () => {
    const onAction = renderPrompt(graveyardView({ min: 1, max: 2 }));
    expect(screen.queryByTestId("prompt-budget")).toBeNull();
    expect(screen.getByTestId("prompt-submit")).toHaveAttribute("aria-disabled", "true");
    fireEvent.click(screen.getByTestId("prompt-submit"));
    expect(onAction).not.toHaveBeenCalled();

    fireEvent.click(option("g1"));
    fireEvent.click(option("g2"));
    fireEvent.click(option("g3"));
    expect(option("g3")).toHaveAttribute("aria-pressed", "false");
    expect(screen.getByTestId("prompt-submit")).toHaveAttribute("aria-disabled", "false");
    fireEvent.click(screen.getByTestId("prompt-submit"));
    answered(onAction, [
      { pick: "instance", instanceId: "g1" },
      { pick: "instance", instanceId: "g2" },
    ]);
  });
});

// ---------------------------------------------------------------------------------------------
// E17 and E18 on the older kinds: the other seat, and the opponent's hand
// ---------------------------------------------------------------------------------------------

describe("prompts the other seat holds, and the opponent's hand", () => {
  const THEIR_HAND: PendingOption[] = [
    { key: "instance:o1", label: "Bigot", instanceId: "o1", defId: "core-002" },
    { key: "instance:o2", label: "Big Felinor", instanceId: "o2", defId: "core-043", radiant: true },
  ];

  it("C #8 the holder of the opponent's mode prompt sees an ordinary mode picker and answers it", () => {
    const onAction = renderPrompt(
      baseView({
        viewer: "p2",
        active: "p1",
        you: emptySide("p2", { hand: [card({ instanceId: "o1", defId: "core-002" })] }),
        opponent: emptySide("p1", { hand: { count: 2 } }),
        pending: pendingFor("mode", [
          { key: "mode:discard", label: "Discard 1 card" },
          { key: "mode:exile", label: "Exile the bottom 1 card of your deck" },
          { key: "mode:draw", label: "Your opponent draws 1 card" },
        ]),
      }),
    );
    expect(modal()).toHaveAttribute("data-prompt-kind", "mode");
    fireEvent.click(screen.getByTestId("prompt-option-mode:draw"));
    answered(onAction, [{ pick: "mode", option: "draw" }]);
  });

  it("R177 the seat waiting on the other's prompt reads that its opponent is choosing, and nothing of the prompt", () => {
    renderPrompt(baseView({ pending: { forYou: false, pendingFor: "p2" } }));
    expect(modal()).not.toHaveAttribute("data-prompt-kind");
    expect(modal()).toHaveAttribute("data-pending-for", "p2");
    expect(screen.getByRole("status")).toHaveTextContent("Your opponent is choosing…");
    expect(optionKeys()).toEqual([]);
    expect(modal().textContent).not.toContain(nameOf("core-002"));
  });

  for (const kind of ["hand", "pick"] as const) {
    it(`R177 C #11 the chooser sees the opponent's hand cards as faces in a ${kind} prompt, Radiant where they are`, () => {
      const onAction = renderPrompt(
        baseView({ opponent: emptySide("p2", { hand: { count: 2 } }), pending: pendingFor(kind, THEIR_HAND, { prompt: "Look at your opponent's hand: exile a card from it" }) }),
      );
      expect(option("o1").querySelector(".cf .card-name")?.textContent).toBe(nameOf("core-002"));
      expect(option("o2").querySelector(".cf .card-name")?.textContent).toBe(nameOf("core-043"));
      expect(option("o2").querySelector(".cf")).toHaveAttribute("data-radiant-face", "true");
      fireEvent.click(option("o2"));
      answered(onAction, [{ pick: "instance", instanceId: "o2" }]);
    });
  }

  it("R177 C #11 the hand's owner, whose cards are being looked at, sees only that a prompt is open", () => {
    renderPrompt(
      baseView({
        viewer: "p2",
        you: emptySide("p2", { hand: [card({ instanceId: "o1", defId: "core-002" }), card({ instanceId: "o2", defId: "core-043" })] }),
        opponent: emptySide("p1", { hand: { count: 3 } }),
        pending: { forYou: false, pendingFor: "p1" },
      }),
    );
    expect(optionKeys()).toEqual([]);
    expect(modal()).not.toHaveAttribute("data-prompt-kind");
    expect(modal().textContent).not.toContain(nameOf("core-002"));
    expect(modal().textContent).not.toContain(nameOf("core-043"));
  });

  function option(id: string): HTMLElement {
    return screen.getByTestId(`prompt-option-instance:${id}`);
  }
});
