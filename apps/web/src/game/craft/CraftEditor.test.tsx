// ME-CRAFT (Meditative #17, R880–R883): the block editor on the real WASM — the meters and the
// preview face are the engine's own `craftPreview`, so these tests prove the editor shows what
// the reducer will say.

import type { ActionBody } from "@jackioh/shared";
import { act, cleanup, fireEvent, render, screen, within } from "@testing-library/react";
import { afterEach, describe, expect, it, vi } from "vitest";

import type { CraftMode, CraftOffer, CraftRecipe, PendingOption, PendingPromptView } from "../../wire/index.ts";
import { CRAFT_HAT_PRICES } from "../../wire/engineConfig.ts";
import { INSPECT_SHEET, closeInspect } from "../../cards/index.ts";
import { LONG_PRESS_MS } from "../../cards/inspect/index.ts";
import { CraftEditor } from "./CraftEditor.tsx";
import { HAT_LABELS } from "./recipe.ts";

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

describe("R883 the palettes offer what the card's type takes", () => {
  it("R883 on a Spell, Taunt's click is inert and aria-disabled while Lifesteal still toggles", () => {
    const props = editorProps({ options: [presetOption(spellPreset(), 0)] });
    render(<CraftEditor pending={props.pending} onAction={props.onAction} />);

    const taunt = screen.getByRole("button", { name: /^Taunt ·/ });
    const lifesteal = screen.getByRole("button", { name: /^Lifesteal ·/ });
    expect(taunt.getAttribute("aria-disabled")).toBe("true");
    expect(taunt.getAttribute("title")).toBe("A Spell cannot take Taunt");
    expect(lifesteal.getAttribute("aria-disabled")).toBe("false");

    fireEvent.click(taunt);
    expect(taunt.getAttribute("aria-pressed")).toBe("false");
    expect(screen.queryByLabelText("Chosen keywords")).toBeNull();
    expect(screen.getByRole("status")).toBeEmptyDOMElement();

    fireEvent.click(lifesteal);
    expect(lifesteal.getAttribute("aria-pressed")).toBe("true");
    expect(screen.getByLabelText("Chosen keywords")).toHaveTextContent("Lifesteal");
    expect(taunt.getAttribute("aria-pressed")).toBe("false");
  });

  it("R883 a drag from an aria-disabled keyword still snaps a Give a unit a keyword effect under the hat", () => {
    const props = editorProps({ options: [presetOption(spellPreset(), 0)] });
    render(<CraftEditor pending={props.pending} onAction={props.onAction} />);

    const taunt = screen.getByRole("button", { name: /^Taunt ·/ });
    expect(taunt.getAttribute("aria-disabled")).toBe("true");
    const restore = pointAt(screen.getByLabelText("When cast hat"));
    try {
      fireEvent.pointerDown(taunt, { clientX: 10, clientY: 10, pointerType: "mouse", button: 0 });
      fireEvent.pointerMove(window, { clientX: 60, clientY: 60 });
      fireEvent.pointerUp(window, { clientX: 60, clientY: 60 });
    } finally {
      restore();
    }
    expect(screen.getByLabelText("Give a unit a keyword under When cast")).toBeInTheDocument();
    expect(screen.queryByLabelText("Chosen keywords")).toBeNull();
  });

  it("R883 the hat palette offers only the hats the card's type takes", () => {
    const props = editorProps({ options: [presetOption(spellPreset(), 0)] });
    render(<CraftEditor pending={props.pending} onAction={props.onAction} />);

    const offered = () =>
      within(screen.getByRole("region", { name: "Hats" }))
        .getAllByRole("button")
        .map((button) => button.textContent?.split(" · ")[0]);
    expect(offered()).toEqual(["When cast"]);

    fireEvent.click(screen.getByRole("button", { name: "Unit" }));
    expect(offered()).toEqual(CRAFT_HAT_PRICES.filter((row) => row.types.includes("Unit")).map((row) => HAT_LABELS[row.hat]));
  });
});

// #552 (MN09, mobile): the card being crafted is read as any card is: a touch long-press on its face
// opens the inspect sheet, and crafts nothing.
describe("R880 the crafted card opens on a long-press", () => {
  afterEach(() => {
    act(() => {
      closeInspect();
    });
    vi.useRealTimers();
  });

  it("R880 a touch long-press on the preview face opens its inspect sheet and sends no answer", () => {
    vi.useFakeTimers();
    const props = editorProps();
    render(<CraftEditor pending={props.pending} onAction={props.onAction} />);
    const face = screen.getByTestId("craft-face");
    fireEvent.pointerDown(face, { pointerType: "touch", pointerId: 1, clientX: 10, clientY: 10 });
    act(() => {
      vi.advanceTimersByTime(LONG_PRESS_MS);
    });
    expect(screen.getByTestId(INSPECT_SHEET)).toHaveTextContent("Pure Closure");
    fireEvent.pointerUp(face, { pointerType: "touch", pointerId: 1 });
    expect(props.onAction).not.toHaveBeenCalled();
  });
});

/** A Potion preset (Jazakus's frame): a Spell whose When cast deals 4, 4 of 4 points (R1563008). */
function potionPreset(): CraftRecipe {
  return {
    cost: 1,
    type: "Spell",
    adjective: "Pure",
    noun: "Closure",
    attack: 0,
    health: 1,
    keywords: [],
    echo: 0,
    hats: [{ hat: "whenCast", effects: [{ verb: "damageEnemyHero", n: 4 }] }],
    frame: "potion",
  };
}

/** The editor's props for a prompt in `mode`, with the prompt's offer when it has one. */
function modeProps(
  mode: CraftMode | undefined,
  options: PendingOption[],
  budget: number,
  craftOffer?: CraftOffer,
): { pending: PendingPromptView; onAction: (body: ActionBody) => void } {
  const pending: PendingPromptView = { ...pendingForCraft(options, budget) };
  if (mode !== undefined) pending.craftMode = mode;
  if (craftOffer !== undefined) pending.craftOffer = craftOffer;
  return { pending, onAction: vi.fn() };
}

/** Type `code` into Scratch Pad's editor. */
function typeCode(code: string): void {
  fireEvent.change(screen.getByTestId("craft-code"), { target: { value: code } });
}

// Meditative+ #26 Scratch Pad (R1182002, R1563010): code typed as text, with no presets, tooltips or
// hints.
describe("R1182002 R1563010 the from-scratch code editor", () => {
  it("R1182002 R1563010 opens as the empty card's code, with no suggestion, price, tooltip or refusal reason", () => {
    const props = modeProps("scratch", [presetOption(unitPreset(), 0), presetOption(spellPreset(), 1)], 2);
    render(<CraftEditor pending={props.pending} onAction={props.onAction} />);

    const modal = screen.getByTestId("prompt-modal");
    expect(modal.getAttribute("data-craft-mode")).toBe("scratch");
    expect(modal.getAttribute("aria-label")).toBe("Code a card from scratch");
    expect(screen.queryByTestId("craft-preset-0")).toBeNull();
    expect(screen.queryByLabelText("Suggestions")).toBeNull();
    expect(screen.queryByLabelText("Why this recipe is refused")).toBeNull();
    expect(screen.getByRole("navigation", { name: "Blocks" }).querySelectorAll("[title]")).toHaveLength(0);
    // An empty canvas: a 0/1 Unit with nothing on it, as code, which the meters price at 0 points and
    // 3 lines.
    expect((screen.getByTestId("craft-code") as HTMLTextAreaElement).value).toBe(
      'card Unit "Pure Closure" {\n  stats(0, 1)\n}\n',
    );
    expect(screen.getByTestId("craft-points")).toHaveTextContent("0/12 pts");
    expect(screen.getByTestId("craft-loc")).toHaveTextContent("3/24 lines");
    // The blocks name themselves in the code's words and nothing more; a retired verb is no block.
    const names = within(screen.getByRole("navigation", { name: "Blocks" }));
    expect(names.getByRole("button", { name: "healYourHero" })).toBeInTheDocument();
    expect(names.getByRole("button", { name: "Taunt" })).toBeInTheDocument();
    expect(names.getByRole("button", { name: "cry" })).toBeInTheDocument();
    expect(names.queryByRole("button", { name: "gainArmor" })).toBeNull();
  });

  it("R1563010 the code typed is the card crafted, and Craft sends its recipe", () => {
    const props = modeProps("scratch", [presetOption(unitPreset(), 0)], 2);
    render(<CraftEditor pending={props.pending} onAction={props.onAction} />);

    typeCode('card Unit "Lazy Iterator" {\n  stats(3, 4)\n  keyword(Taunt)\n  cry {\n    damageTarget(2)\n  }\n}\n');
    // 3 + 4 − 1 + 2 (Taunt) + 2 = 10 of 12 points; 3 + 1 + 2 + 2 = 8 of 24 lines.
    expect(screen.getByTestId("craft-points")).toHaveTextContent("10/12 pts");
    expect(screen.getByTestId("craft-face")).toHaveTextContent("Lazy Iterator");
    expect(screen.getByTestId("prompt-submit").getAttribute("aria-disabled")).toBe("false");
    fireEvent.click(screen.getByTestId("prompt-submit"));
    expect(props.onAction).toHaveBeenCalledWith({
      type: "answer",
      choiceId: "ch-craft",
      selection: [
        {
          pick: "craft",
          recipe: {
            cost: 2,
            type: "Unit",
            adjective: "Lazy",
            noun: "Iterator",
            attack: 3,
            health: 4,
            keywords: [{ kind: "Taunt" }],
            echo: 0,
            hats: [{ hat: "cry", effects: [{ verb: "damageTarget", n: 2 }] }],
          },
        },
      ],
    });
  });

  it("R1563010 a syntax error names its line and keeps Craft off", () => {
    const props = modeProps("scratch", [presetOption(unitPreset(), 0)], 2);
    render(<CraftEditor pending={props.pending} onAction={props.onAction} />);

    typeCode('card Unit "Pure Closure" {\n  stats(3 4)\n}\n');
    expect(screen.getByTestId("craft-code-error")).toHaveTextContent("line 2");
    expect(screen.getByTestId("craft-code").getAttribute("aria-invalid")).toBe("true");
    expect(screen.getByTestId("prompt-submit").getAttribute("aria-disabled")).toBe("true");
    fireEvent.click(screen.getByTestId("prompt-submit"));
    expect(props.onAction).not.toHaveBeenCalled();
  });

  it("R1182002 Craft stays off while the engine refuses the card, with no reason shown", () => {
    const props = modeProps("scratch", [presetOption(unitPreset(), 0)], 2);
    render(<CraftEditor pending={props.pending} onAction={props.onAction} />);

    // A Spell with no When cast reads, but the engine refuses it.
    typeCode('card Spell "Pure Closure" {\n}\n');
    expect(screen.getByTestId("craft-code-error")).toBeEmptyDOMElement();
    expect(screen.getByTestId("prompt-submit").getAttribute("aria-disabled")).toBe("true");
    expect(screen.queryByLabelText("Why this recipe is refused")).toBeNull();
    fireEvent.click(screen.getByTestId("prompt-submit"));
    expect(props.onAction).not.toHaveBeenCalled();
  });

  it("R1563010 a block's name types itself at the caret", () => {
    const props = modeProps("scratch", [presetOption(unitPreset(), 0)], 2);
    render(<CraftEditor pending={props.pending} onAction={props.onAction} />);

    const area = screen.getByTestId("craft-code") as HTMLTextAreaElement;
    area.setSelectionRange(area.value.length, area.value.length);
    fireEvent.click(screen.getByRole("button", { name: "draw" }));
    expect(area.value.endsWith("}\ndraw()")).toBe(true);
  });

  it("R1182002 the face opens no glossary on a long-press", () => {
    vi.useFakeTimers();
    try {
      const props = modeProps("scratch", [presetOption(unitPreset(), 0)], 2);
      render(<CraftEditor pending={props.pending} onAction={props.onAction} />);
      const face = screen.getByTestId("craft-face");
      fireEvent.pointerDown(face, { pointerType: "touch", pointerId: 1, clientX: 10, clientY: 10 });
      act(() => {
        vi.advanceTimersByTime(LONG_PRESS_MS);
      });
      expect(screen.queryByTestId(INSPECT_SHEET)).toBeNull();
      fireEvent.pointerUp(face, { pointerType: "touch", pointerId: 1 });
    } finally {
      vi.useRealTimers();
    }
  });
});

// Meditative+ #16 Jazakus and #16.1 Jazakus Prime (R1182000, R1182001, R1563008): the potion frame.
describe("R1182000 R1563008 the potion frame", () => {
  it("R1182000 R1563008 pins a Spell whose one hat is its When cast, with no type or hat palette", () => {
    const props = modeProps("potion", [presetOption(potionPreset(), 0)], 1);
    render(<CraftEditor pending={props.pending} onAction={props.onAction} />);

    const modal = screen.getByTestId("prompt-modal");
    expect(modal.getAttribute("data-craft-mode")).toBe("potion");
    expect(modal.getAttribute("aria-label")).toBe("Craft a Potion");
    expect(screen.queryByRole("button", { name: "Unit" })).toBeNull();
    expect(screen.queryByRole("region", { name: "Hats" })).toBeNull();
    expect(screen.queryByRole("group", { name: "Attack" })).toBeNull();
    expect(screen.getByLabelText("When cast hat")).toBeInTheDocument();
    expect(screen.queryByLabelText("Remove When cast hat")).toBeNull();
    expect(screen.getByTestId("craft-points")).toHaveTextContent("4/4 pts");
    expect(screen.getByTestId("craft-face")).toHaveTextContent("Pure Closure Potion");
    // A suggestion is still offered in the potion frame.
    expect(screen.getByTestId("craft-preset-0")).toHaveTextContent("Pure Closure");
  });

  it("R1182000 R1563008 a palette block snaps under the When cast, and Craft sends the framed recipe", () => {
    const props = modeProps("potion", [presetOption(potionPreset(), 0)], 1);
    render(<CraftEditor pending={props.pending} onAction={props.onAction} />);

    fireEvent.click(screen.getByRole("button", { name: /^Heal your hero ·/ }));
    expect(screen.getByLabelText("Heal your hero 1 under When cast")).toBeInTheDocument();
    expect(screen.getByTestId("craft-points")).toHaveTextContent("5/4 pts");
    expect(screen.getByTestId("prompt-submit").getAttribute("aria-disabled")).toBe("true");
    expect(screen.getByRole("status").textContent).toContain("over the budget of 4");

    fireEvent.click(screen.getByLabelText("Remove Heal your hero 1"));
    fireEvent.click(screen.getByTestId("prompt-submit"));
    expect(props.onAction).toHaveBeenCalledWith({
      type: "answer",
      choiceId: "ch-craft",
      selection: [{ pick: "craft", recipe: potionPreset() }],
    });
  });

  it("R1182001 R1563008 a HUGE Potion has five times a Potion's points", () => {
    const huge: CraftRecipe = { ...potionPreset(), cost: 3, frame: "hugePotion" };
    const props = modeProps("hugePotion", [presetOption(huge, 0)], 3);
    render(<CraftEditor pending={props.pending} onAction={props.onAction} />);

    expect(screen.getByTestId("prompt-modal").getAttribute("aria-label")).toBe("Craft a HUGE Potion");
    expect(screen.getByTestId("craft-points")).toHaveTextContent("4/20 pts");
    expect(screen.getByTestId("craft-face")).toHaveTextContent("HUGE Pure Closure Potion");
  });

  it("R1563008 the editor's verdict is the prompt's: a Unit is no Potion", () => {
    const unit: CraftRecipe = { ...potionPreset(), type: "Unit", attack: 1, health: 1 };
    const props = modeProps("potion", [presetOption(unit, 0)], 1);
    render(<CraftEditor pending={props.pending} onAction={props.onAction} />);

    expect(screen.getByTestId("prompt-submit").getAttribute("aria-disabled")).toBe("true");
    expect(screen.getByRole("status").textContent).toContain("a Potion is a Spell");
  });
});

// R1563007: a craft prompt offers four keywords, two hats and three effects, Kazakus's way.
describe("R1563007 the palette is the prompt's offer", () => {
  const offer: CraftOffer = {
    keywords: ["Taunt", "Rush", "Lifesteal", "Armor"],
    hats: ["cry", "death"],
    verbs: ["damageTarget", "healYourHero", "draw"],
  };

  /** The names of a palette section's blocks, before their prices. */
  function names(section: string): (string | undefined)[] {
    return within(screen.getByRole("region", { name: section }))
      .getAllByRole("button")
      .map((button) => button.textContent?.split(" · ")[0]);
  }

  it("R1563007 shows only the offered keywords, hats and effects", () => {
    const props = modeProps(undefined, [presetOption(unitPreset(), 0)], 2, offer);
    render(<CraftEditor pending={props.pending} onAction={props.onAction} />);

    expect(names("Keywords")).toEqual(["Taunt", "Rush", "Lifesteal", "Armor"]);
    expect(names("Hats")).toEqual(["Cry", "Death"]);
    expect(names("Effects")).toEqual(["Deal damage to a target", "Heal your hero", "Draw cards"]);
  });

  it("R1563007 a block the offer lacks turns Craft off, as the engine refuses it", () => {
    const outside: CraftRecipe = {
      ...unitPreset(),
      attack: 1,
      health: 1,
      hats: [{ hat: "cry", effects: [{ verb: "damageEnemyHero", n: 1 }] }],
    };
    const props = modeProps(undefined, [presetOption(outside, 0)], 2, offer);
    render(<CraftEditor pending={props.pending} onAction={props.onAction} />);

    expect(screen.getByTestId("prompt-submit").getAttribute("aria-disabled")).toBe("true");
    expect(screen.getByRole("status").textContent).toContain("was not offered");
  });

  it("R1563009 with no offer, Gain Armor is still no block", () => {
    const props = editorProps({ options: [presetOption(spellPreset(), 0)] });
    render(<CraftEditor pending={props.pending} onAction={props.onAction} />);

    expect(screen.queryByRole("button", { name: /^Gain Armor ·/ })).toBeNull();
    expect(screen.getByRole("button", { name: /^Heal your hero ·/ })).toBeInTheDocument();
  });
});
