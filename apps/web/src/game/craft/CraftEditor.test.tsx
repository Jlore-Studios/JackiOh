// ME-CRAFT (Meditative #17, R880–R883): the block editor on the real WASM — the meters and the
// preview face are the engine's own `craftPreview`, so these tests prove the editor shows what
// the reducer will say.

import type { ActionBody } from "@jackioh/shared";
import { cleanup, fireEvent, render, screen } from "@testing-library/react";
import { afterEach, describe, expect, it, vi } from "vitest";

import type { CraftRecipe, PendingOption, PendingPromptView } from "../../wire/index.ts";
import { CraftEditor } from "./CraftEditor.tsx";

afterEach(cleanup);

/** A valid Unit preset at cost 2: 5+6−1 = 10 of 12 points, 3 of 24 lines. */
function unitPreset(): CraftRecipe {
  return {
    cost: 2,
    type: "Unit",
    adjective: "Pure",
    noun: "Closure",
    attack: 5,
    health: 6,
    keywords: [],
    echo: 0,
    hats: [],
  };
}

/** A valid Spell preset at cost 2: a When cast dealing 8, 8 of 12 points. */
function spellPreset(): CraftRecipe {
  return {
    cost: 2,
    type: "Spell",
    adjective: "Seeded",
    noun: "Reducer",
    attack: 0,
    health: 1,
    keywords: [],
    echo: 0,
    hats: [{ hat: "whenCast", effects: [{ verb: "damageEnemyHero", n: 8 }] }],
  };
}

function pendingForCraft(options: PendingOption[], budget = 2): PendingPromptView {
  return {
    forYou: true,
    choiceId: "ch-craft",
    kind: "craft",
    options,
    min: 1,
    max: 1,
    prompt: "Craft a card",
    budget,
  };
}

function presetOption(recipe: CraftRecipe, index: number): PendingOption {
  return { key: `craft:preset-${index}`, label: `${recipe.adjective} ${recipe.noun}`, recipe };
}

function editorProps(over: { options?: PendingOption[]; budget?: number } = {}): {
  pending: PendingPromptView;
  onAction: (body: ActionBody) => void;
} {
  const onAction = vi.fn();
  return {
    pending: pendingForCraft(over.options ?? [presetOption(unitPreset(), 0), presetOption(spellPreset(), 1)], over.budget),
    onAction,
  };
}

/** jsdom has no `document.elementFromPoint`, which the drag reads on release to find the drop
 * target: stand one in that always answers `target`, and give back what restores the document. */
function pointAt(target: Element): () => void {
  const own = Object.getOwnPropertyDescriptor(document, "elementFromPoint");
  Object.defineProperty(document, "elementFromPoint", { configurable: true, value: () => target });
  return () => {
    if (own === undefined) Reflect.deleteProperty(document, "elementFromPoint");
    else Object.defineProperty(document, "elementFromPoint", own);
  };
}

describe("the block editor (R880)", () => {
  it("R880 shows the engine's meters and the preview face", () => {
    const props = editorProps();
    render(<CraftEditor pending={props.pending} onAction={props.onAction} />);

    expect(screen.getByTestId("prompt-modal").getAttribute("data-prompt-kind")).toBe("craft");
    // The first preset is a 5/6 Unit: 10 of 12 points, 3 of 24 lines.
    expect(screen.getByTestId("craft-points")).toHaveTextContent("10/12 pts");
    expect(screen.getByTestId("craft-loc")).toHaveTextContent("3/24 lines");
    expect(screen.getByTestId("prompt-submit").getAttribute("aria-disabled")).toBe("false");
    expect(screen.getByRole("status")).toBeEmptyDOMElement();
    // The live face is the card it would make.
    expect(document.body.textContent).toContain("Pure Closure");
  });

  it("R880 a suggestion loads, and Craft sends its recipe", () => {
    const props = editorProps();
    render(<CraftEditor pending={props.pending} onAction={props.onAction} />);

    fireEvent.click(screen.getByTestId("craft-preset-1"));
    // The Spell preset: 8 of 12 points, 2 + 1 + 3 = 6 of 24 lines.
    expect(screen.getByTestId("craft-points")).toHaveTextContent("8/12 pts");

    fireEvent.click(screen.getByTestId("prompt-submit"));
    expect(props.onAction).toHaveBeenCalledTimes(1);
    expect(props.onAction).toHaveBeenCalledWith({
      type: "answer",
      choiceId: "ch-craft",
      selection: [{ pick: "craft", recipe: spellPreset() }],
    });
  });

  it("R880 Craft is disabled while the recipe is refused", () => {
    const over: CraftRecipe = { ...unitPreset(), attack: 10, health: 10 };
    const props = editorProps({ options: [presetOption(over, 0)] });
    render(<CraftEditor pending={props.pending} onAction={props.onAction} />);

    expect(screen.getByTestId("prompt-submit").getAttribute("aria-disabled")).toBe("true");
    expect(screen.getByRole("status").textContent).toContain("over the budget");

    fireEvent.click(screen.getByTestId("prompt-submit"));
    expect(props.onAction).not.toHaveBeenCalled();
  });

  it("R883 clicking a palette block snaps it under the selected hat", () => {
    const props = editorProps({ options: [presetOption(spellPreset(), 0)] });
    render(<CraftEditor pending={props.pending} onAction={props.onAction} />);

    // The Spell preset's When cast holds one effect; snapping a second under it is valid.
    fireEvent.click(screen.getByRole("button", { name: /^Heal your hero ·/ }));
    expect(screen.getByTestId("craft-points")).toHaveTextContent("9/12 pts");
    expect(screen.getByLabelText("Heal your hero 1 under When cast")).toBeInTheDocument();
  });

  it("R880 keyboard stepping and removal", () => {
    const props = editorProps({ options: [presetOption(spellPreset(), 0)] });
    render(<CraftEditor pending={props.pending} onAction={props.onAction} />);

    const block = screen.getByLabelText("Deal damage to the enemy hero 8 under When cast");
    block.focus();
    fireEvent.keyDown(block, { key: "+" });
    expect(screen.getByTestId("craft-points")).toHaveTextContent("9/12 pts");
    fireEvent.keyDown(block, { key: "-" });
    expect(screen.getByTestId("craft-points")).toHaveTextContent("8/12 pts");
    fireEvent.keyDown(block, { key: "Delete" });
    // No effects left: the Spell has no When cast effect, so it is refused.
    expect(screen.getByRole("status").textContent).toContain("no effect");
  });

  it("R880 a drag from the palette snaps the block under the hat", () => {
    const props = editorProps({ options: [presetOption(spellPreset(), 0)] });
    render(<CraftEditor pending={props.pending} onAction={props.onAction} />);

    const palette = screen.getByRole("button", { name: /^Heal your hero ·/ });
    const hat = screen.getByLabelText("When cast hat");
    const restore = pointAt(hat);
    try {
      fireEvent.pointerDown(palette, { clientX: 10, clientY: 10, pointerType: "mouse", button: 0 });
      fireEvent.pointerMove(window, { clientX: 60, clientY: 60 });
      fireEvent.pointerUp(window, { clientX: 60, clientY: 60 });
    } finally {
      restore();
    }
    expect(screen.getByLabelText("Heal your hero 1 under When cast")).toBeInTheDocument();
    expect(screen.getByTestId("craft-points")).toHaveTextContent("9/12 pts");
  });

  it("R880 a drag reorders the canvas", () => {
    const props = editorProps({ options: [presetOption(spellPreset(), 0)] });
    render(<CraftEditor pending={props.pending} onAction={props.onAction} />);

    fireEvent.click(screen.getByRole("button", { name: /^Heal your hero ·/ }));
    const blocks = () =>
      [...document.querySelectorAll(".craft-placed")].map(
        (node) => node.getAttribute("aria-label") ?? "",
      );
    expect(blocks()).toEqual([
      "Deal damage to the enemy hero 8 under When cast",
      "Heal your hero 1 under When cast",
    ]);

    // Drag the first block onto the second: it lands after it.
    const first = screen.getByLabelText("Deal damage to the enemy hero 8 under When cast");
    const second = screen.getByLabelText("Heal your hero 1 under When cast");
    const restore = pointAt(second);
    try {
      fireEvent.pointerDown(first, { clientX: 10, clientY: 10, pointerType: "mouse", button: 0 });
      fireEvent.pointerMove(window, { clientX: 60, clientY: 60 });
      fireEvent.pointerUp(window, { clientX: 60, clientY: 60 });
    } finally {
      restore();
    }
    expect(blocks()).toEqual([
      "Heal your hero 1 under When cast",
      "Deal damage to the enemy hero 8 under When cast",
    ]);
  });

  it("R880 the phone bottom sheet", () => {
    const query = { matches: true, addEventListener: vi.fn(), removeEventListener: vi.fn() };
    vi.stubGlobal("matchMedia", vi.fn(() => query));
    try {
      const props = editorProps();
      render(<CraftEditor pending={props.pending} onAction={props.onAction} />);
      expect(screen.getByTestId("prompt-modal").getAttribute("data-layout")).toBe("sheet");
    } finally {
      vi.unstubAllGlobals();
    }
  });
});
