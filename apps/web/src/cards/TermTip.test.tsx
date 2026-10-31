// R2098000: a glossary term in a card's text shows its glossary line where the face takes the
// pointer. A mouse or pen resting on it shows the tooltip and leaving hides it; a touch held on
// it shows it too, while an ordinary tap still reaches the card, and the term's long press takes
// over from the card's. Where the surface allows a control the term is a tab stop.

import { readFileSync } from "node:fs";
import { dirname, join } from "node:path";
import { fileURLToPath } from "node:url";

import { act, cleanup, fireEvent, render, screen } from "@testing-library/react";
import type { ReactElement } from "react";
import { afterEach, describe, expect, it, vi } from "vitest";

import { CATALOG } from "@jackioh/cards";

import { CardFace } from "./CardFace.tsx";
import { TERM_HOVER_DELAY_MS } from "./constants.ts";
import { GLOSSARY } from "./glossary.ts";
import { faceModel, type FaceModel } from "./model.ts";
import { CardDefsProvider, RefsInteractive } from "./refContext.tsx";
import { TERM_TOOLTIP_TESTID, TermTips } from "./TermTip.tsx";
import { CardDetail } from "./inspect/CardDetail.tsx";
import { LONG_PRESS_MS, LONG_PRESS_SLOP_PX } from "./inspect/constants.ts";
import { closeInspect } from "./inspect/store.ts";
import { INSPECT_FACE_BASE, INSPECT_HOVER, INSPECT_SHEET } from "./inspect/testids.ts";
import { useInspectTrigger } from "./inspect/useInspectTrigger.tsx";
import { standInCopy } from "../fx/dom.ts";
import { must } from "../test/stubServer.ts";

/** Bigot (core-002): "Cry: Destroy an enemy non-Human Unit." */
const BIGOT = "core-002";
/** Shorter than any long-press: a tap. */
const TAP_MS = 50;
/** Longer than any hover or long-press delay, so "nothing opened" is not "not yet". */
const PAST_ANY_DELAY_MS = 2_000;
const TOUCH = { pointerType: "touch", pointerId: 1, clientX: 10, clientY: 10, button: 0 };

function bigotFace(): FaceModel {
  return faceModel({ defId: BIGOT, def: must(CATALOG[BIGOT], "Bigot in the catalog"), radiant: false });
}

/** Bigot's Cry term inside `root`. */
function cry(root: ParentNode): HTMLElement {
  const found = root.querySelector<HTMLElement>('.cf-term[data-term="Cry"]');
  return must(found, "Bigot's Cry term");
}

function advance(ms: number): void {
  act(() => {
    vi.advanceTimersByTime(ms);
  });
}

/** A card with an inspect trigger, as the board's Card root carries both. */
function Host({ onClick }: { onClick?: () => void }): ReactElement {
  const inspect = useInspectTrigger({ key: "term-host", face: bigotFace() });
  return (
    <>
      <div data-testid="host" {...inspect.handlers} onClick={onClick}>
        <TermTips>
          <CardFace face={bigotFace()} layout="full" />
        </TermTips>
      </div>
      {inspect.overlay}
    </>
  );
}

afterEach(() => {
  act(() => closeInspect());
  cleanup();
  vi.useRealTimers();
});

describe("R2098000 a term in a card's text shows its glossary line", () => {
  it("R2098000 hovering a term shows its glossary line, and leaving it hides it", () => {
    vi.useFakeTimers();
    render(<Host />);
    const term = cry(screen.getByTestId("host"));

    fireEvent.pointerEnter(term, { pointerType: "mouse" });
    expect(screen.queryByTestId(TERM_TOOLTIP_TESTID)).toBeNull();
    act(() => {
      vi.advanceTimersByTime(TERM_HOVER_DELAY_MS);
    });
    const tooltip = screen.getByRole("tooltip");
    expect(tooltip).toHaveAttribute("data-testid", TERM_TOOLTIP_TESTID);
    expect(tooltip).toHaveAttribute("data-term", "Cry");
    expect(tooltip).toHaveTextContent(`${GLOSSARY.Cry.label} ${GLOSSARY.Cry.rule}`);
    expect(term).toHaveAttribute("aria-describedby", tooltip.id);
    expect(term).toHaveAttribute("data-tip-open", "true");

    fireEvent.pointerLeave(term, { pointerType: "mouse" });
    expect(screen.queryByTestId(TERM_TOOLTIP_TESTID)).toBeNull();
    expect(term).not.toHaveAttribute("aria-describedby");
  });

  it("R2098000 a touch held on a term shows its rule, takes the card's long press and swallows the lift's click; an ordinary tap reaches the card", () => {
    vi.useFakeTimers();
    const onClick = vi.fn();
    render(<Host onClick={onClick} />);
    const term = cry(screen.getByTestId("host"));
    expect(term).toHaveAttribute("data-tip", "pointer");
    expect(term).not.toHaveAttribute("tabindex");

    // An ordinary tap reaches the card and opens nothing.
    fireEvent.pointerDown(term, TOUCH);
    advance(TAP_MS);
    fireEvent.pointerUp(term, TOUCH);
    fireEvent.click(term);
    expect(onClick).toHaveBeenCalledTimes(1);
    advance(PAST_ANY_DELAY_MS);
    expect(screen.queryByTestId(TERM_TOOLTIP_TESTID)).toBeNull();
    expect(screen.queryByTestId(INSPECT_SHEET)).toBeNull();

    // A held touch shows the rule and never opens the card's preview or sheet.
    fireEvent.pointerDown(term, TOUCH);
    advance(LONG_PRESS_MS);
    expect(screen.getByRole("tooltip")).toHaveTextContent(GLOSSARY.Cry.rule);
    advance(PAST_ANY_DELAY_MS);
    expect(screen.queryByTestId(INSPECT_HOVER)).toBeNull();
    expect(screen.queryByTestId(INSPECT_SHEET)).toBeNull();
    fireEvent.pointerUp(term, TOUCH);
    fireEvent.click(term);
    expect(onClick).toHaveBeenCalledTimes(1);
    expect(screen.getByRole("tooltip")).toHaveTextContent(GLOSSARY.Cry.rule);

    // A press anywhere else closes it.
    fireEvent.pointerDown(document.body);
    expect(screen.queryByTestId(TERM_TOOLTIP_TESTID)).toBeNull();

    // Control: a long press off the terms still opens the card's preview, the hold's (R2617000).
    const name = must(screen.getByTestId("host").querySelector(".card-name"), "the host's card name");
    fireEvent.pointerDown(name, TOUCH);
    advance(LONG_PRESS_MS);
    expect(screen.getByTestId(INSPECT_HOVER)).toBeInTheDocument();
  });

  it("R2098000 a touch that moves past the slop is a drag and shows nothing", () => {
    vi.useFakeTimers();
    render(<Host />);
    const term = cry(screen.getByTestId("host"));

    fireEvent.pointerDown(term, TOUCH);
    // How a drag that captured the pointer sends it: the move arrives on the window's behalf.
    fireEvent.pointerMove(document.body, {
      pointerType: "touch",
      pointerId: 1,
      clientX: 10 + LONG_PRESS_SLOP_PX + 1,
      clientY: 10,
    });
    advance(PAST_ANY_DELAY_MS);
    expect(screen.queryByTestId(TERM_TOOLTIP_TESTID)).toBeNull();
  });

  it("R2098000 a term takes the pointer only where its card does: never mid-animation, taken away by a burst, or as a stand-in on the fx layer", () => {
    const css = readFileSync(join(dirname(fileURLToPath(import.meta.url)), "cards.css"), "utf8").replace(/\/\*[\s\S]*?\*\//g, "");
    const rules = [...css.matchAll(/([^{}]+)\{([^{}]*)\}/g)].map((match) => ({ selector: (match[1] ?? "").trim(), body: match[2] ?? "" }));
    const takes = rules.findIndex((rule) => rule.selector === ".cf-term[data-tip]" && /pointer-events:\s*auto/.test(rule.body));
    const gives = rules.findIndex((rule) => rule.selector.endsWith(" .cf-term[data-tip]") && /pointer-events:\s*none/.test(rule.body));
    expect(takes, "the term takes the pointer back from its face").toBeGreaterThanOrEqual(0);
    // The same selector under an ancestor, and later in the sheet: it wins wherever it matches.
    expect(gives, "the term gives the pointer up again under an ancestor that switches it off").toBeGreaterThan(takes);
    const off = must(rules[gives], "the rule giving the pointer up").selector;

    render(<Host />);
    const host = screen.getByTestId("host");
    const term = cry(host);
    expect(term.matches(off), "a card at rest").toBe(false);
    // animations.css's `[data-animating]` and fx.css's concealed card (B47), on the card's root.
    const switchedOff: [string, string][] = [
      ["data-animating", "cardPlayed"],
      ["data-fx-concealed", "now"],
      ["data-fx-concealed", "after"],
    ];
    for (const [name, value] of switchedOff) {
      host.setAttribute(name, value);
      expect(term.matches(off), `${name}="${value}"`).toBe(true);
      host.removeAttribute(name);
    }

    // fx.css's stand-in (B46): a copy of the card on the fx layer, above the board.
    const layer = document.createElement("div");
    layer.className = "fx-layer";
    layer.appendChild(standInCopy(host));
    document.body.appendChild(layer);
    const copied = cry(layer);
    expect(!copied.hasAttribute("data-tip") || copied.matches(off), "the stand-in's term").toBe(true);
    layer.remove();
  });

  it("R2098000 where the surface allows a control a term is a tab stop: focus shows the tooltip and describes the term, blur hides it", () => {
    const trees = [
      <CardDefsProvider key="refs" defs={CATALOG}>
        <RefsInteractive>
          <CardFace face={bigotFace()} layout="full" />
        </RefsInteractive>
      </CardDefsProvider>,
      <TermTips key="tips" focusable>
        <CardFace face={bigotFace()} layout="full" />
      </TermTips>,
    ];
    for (const tree of trees) {
      const { unmount } = render(tree);
      const term = cry(document.body);
      expect(term).toHaveAttribute("data-tip", "control");
      expect(term).toHaveAttribute("tabindex", "0");
      act(() => term.focus());
      const tooltip = screen.getByRole("tooltip");
      expect(tooltip).toHaveAttribute("data-testid", TERM_TOOLTIP_TESTID);
      expect(term).toHaveAttribute("aria-describedby", tooltip.id);
      act(() => term.blur());
      expect(screen.queryByTestId(TERM_TOOLTIP_TESTID)).toBeNull();
      expect(term).not.toHaveAttribute("aria-describedby");
      unmount();
    }
  });

  it("R2098000 in the detail view Escape closes an open term's tooltip and not the view", () => {
    const onClose = vi.fn();
    render(
      <CardDefsProvider defs={CATALOG}>
        <CardDetail def={must(CATALOG["core-043"], "core-043 in the catalog")} onClose={onClose} />
      </CardDefsProvider>,
    );
    const term = cry(screen.getByTestId(INSPECT_FACE_BASE));
    act(() => term.focus());
    expect(screen.getByTestId(TERM_TOOLTIP_TESTID)).toBeInTheDocument();
    fireEvent.keyDown(window, { key: "Escape" });
    expect(screen.queryByTestId(TERM_TOOLTIP_TESTID)).toBeNull();
    expect(onClose).not.toHaveBeenCalled();
    fireEvent.keyDown(window, { key: "Escape" });
    expect(onClose).toHaveBeenCalledTimes(1);
  });

  it("R2098000 a face outside TermTips and any control surface prints its terms plain", () => {
    vi.useFakeTimers();
    const { container } = render(<CardFace face={bigotFace()} layout="full" />);
    const term = cry(container);
    expect(term).not.toHaveAttribute("data-tip");
    expect(term).not.toHaveAttribute("tabindex");
    fireEvent.pointerEnter(term, { pointerType: "mouse" });
    advance(PAST_ANY_DELAY_MS);
    expect(screen.queryByTestId(TERM_TOOLTIP_TESTID)).toBeNull();
  });
});
