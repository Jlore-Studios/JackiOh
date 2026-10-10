// The rules box's reading floor (fit.ts), against a modelled layout. jsdom has no layout, so
// `useFitText` is a no-op there (CardFace.test.tsx B15); a rules box gets a small model instead: a
// known box height, a font of base size × tier scale × `--cf-fit` (no tier scale in the long
// layout, as cards.css has it) and text that needs `need × font²` px of height. The component specs
// (card-faces B15, deckbuilder-layout) prove the same on real layout. The last describe is the
// scheduler: fits queued together are read together, round by round.

import { cleanup, render } from "@testing-library/react";
import { useRef, type ReactElement } from "react";
import { afterEach, describe, expect, it, vi } from "vitest";

import { FIT_FLOOR_PX, FIT_MIN } from "./constants.ts";
import { flushFits, LONG_ATTRIBUTE, scheduleFit, SKIPPABLE_ATTRIBUTE, useFitText } from "./fit.ts";

type Model = {
  /** The font at full size (4.4cqh on a real face), in px. */
  basePx: number;
  /** The length tier's head start (TIER_SCALE). */
  tierScale: number;
  /** Box heights in px: the ordinary rules box and the long layout's. */
  box: number;
  longBox: number;
  /** Height the text needs is `need × font²`. */
  need: number;
};

const WIDTH = 150;
const PADDING = 4;
const LINE_HEIGHT = 1.18;

function fitOf(element: HTMLElement): number {
  return Number(element.style.getPropertyValue("--cf-fit")) || 1;
}

/** The model's font for this element, as cards.css would compute it. */
function fontOf(element: HTMLElement, model: Model): number {
  const long = element.closest(".cf")?.getAttribute(LONG_ATTRIBUTE) === "true";
  const inline = element.style.getPropertyValue("--cf-text-scale");
  const scale = long ? 1 : inline === "" ? model.tierScale : Number(inline);
  return model.basePx * scale * fitOf(element);
}

function boxOf(element: HTMLElement, model: Model): number {
  return element.closest(".cf")?.getAttribute(LONG_ATTRIBUTE) === "true" ? model.longBox : model.box;
}

/** Counts the layouts a run pays for: a write dirties layout, and the next read of it pays once. */
const meter = { dirty: false, paid: 0 };

/** Wires the model into the one element the probe renders. */
function modelLayout(model: Model): void {
  meter.dirty = false;
  meter.paid = 0;
  const setProperty = CSSStyleDeclaration.prototype.setProperty;
  vi.spyOn(CSSStyleDeclaration.prototype, "setProperty").mockImplementation(function (
    this: CSSStyleDeclaration,
    ...args: Parameters<typeof setProperty>
  ) {
    meter.dirty = true;
    setProperty.apply(this, args);
  });
  const text = (element: Element): element is HTMLElement =>
    element instanceof HTMLElement && element.dataset.probe === "text";
  const real = window.getComputedStyle.bind(window);
  vi.spyOn(window, "getComputedStyle").mockImplementation((element: Element) => {
    if (!text(element)) return real(element);
    const font = fontOf(element, model);
    return {
      fontSize: `${String(font)}px`,
      lineHeight: `${String(font * LINE_HEIGHT)}px`,
      paddingTop: `${String(PADDING)}px`,
      paddingBottom: `${String(PADDING)}px`,
      maxHeight: `${String(boxOf(element, model))}px`,
    } as CSSStyleDeclaration;
  });
  const proto = HTMLElement.prototype;
  vi.spyOn(proto, "clientWidth", "get").mockImplementation(function (this: HTMLElement) {
    return text(this) ? WIDTH : 0;
  });
  vi.spyOn(proto, "scrollWidth", "get").mockImplementation(function (this: HTMLElement) {
    return text(this) ? WIDTH : 0;
  });
  vi.spyOn(proto, "clientHeight", "get").mockImplementation(function (this: HTMLElement) {
    return text(this) ? boxOf(this, model) : 0;
  });
  vi.spyOn(proto, "scrollHeight", "get").mockImplementation(function (this: HTMLElement) {
    if (!text(this)) return 0;
    if (meter.dirty) {
      meter.dirty = false;
      meter.paid += 1;
    }
    const font = fontOf(this, model);
    const needed = model.need * font * font + 2 * PADDING;
    // A clamped box shows its clamped lines only.
    const lines = this.style.getPropertyValue("--cf-clamp-lines");
    return lines === "" ? needed : Math.min(needed, Number(lines) * font * LINE_HEIGHT + 2 * PADDING);
  });
}

function Probe({ content }: { content: string }): ReactElement {
  const ref = useRef<HTMLSpanElement>(null);
  useFitText(ref, content, { floorPx: FIT_FLOOR_PX });
  return (
    <span className="cf">
      <span data-probe="text" ref={ref}>
        <span className="cf-text-base">{content}</span>
      </span>
    </span>
  );
}

function fitted(model: Model): { text: HTMLElement; face: HTMLElement; font: number } {
  modelLayout(model);
  const { container } = render(<Probe content="rules" />);
  flushFits();
  const text = container.querySelector<HTMLElement>('[data-probe="text"]');
  const face = container.querySelector<HTMLElement>(".cf");
  if (text === null || face === null) throw new Error("probe did not render");
  return { text, face, font: fontOf(text, model) };
}

afterEach(() => {
  vi.restoreAllMocks();
});

describe("the rules box's reading floor", () => {
  it("leaves a text that fits at or above the floor exactly as the tier fitted it", () => {
    const { text, face, font } = fitted({ basePx: 11, tierScale: 1, box: 80, longBox: 120, need: 0.5 });
    expect(font).toBeGreaterThanOrEqual(FIT_FLOOR_PX);
    expect(face.hasAttribute(LONG_ATTRIBUTE)).toBe(false);
    expect(text.hasAttribute("data-clamped")).toBe(false);
    expect(text.style.getPropertyValue("--cf-text-scale")).toBe("");
  });

  it("drops the tier's head start when that alone lifts the text to the floor", () => {
    // Tier scale 0.72 caps it at 7.9 px, though the box holds it at 9+ px from full size.
    const { text, face, font } = fitted({ basePx: 11, tierScale: 0.72, box: 80, longBox: 120, need: 0.8 });
    expect(text.style.getPropertyValue("--cf-text-scale")).toBe("1");
    expect(font).toBeGreaterThanOrEqual(FIT_FLOOR_PX - 0.05);
    expect(face.hasAttribute(LONG_ATTRIBUTE)).toBe(false);
    expect(text.hasAttribute("data-clamped")).toBe(false);
  });

  it("takes the long layout when the ordinary box cannot hold the text at the floor", () => {
    // Needs 1.2 × 9² ≈ 97 px: over the 80 px box, inside the 120 px one.
    const { text, face, font } = fitted({ basePx: 11, tierScale: 0.62, box: 80, longBox: 120, need: 1.2 });
    expect(face.getAttribute(LONG_ATTRIBUTE)).toBe("true");
    expect(font).toBeGreaterThanOrEqual(FIT_FLOOR_PX - 0.05);
    expect(text.hasAttribute("data-clamped")).toBe(false);
  });

  it("clamps at the floor, to the lines the long box holds, when even that is too small", () => {
    const { text, face, font } = fitted({ basePx: 11, tierScale: 0.62, box: 80, longBox: 120, need: 3 });
    expect(face.getAttribute(LONG_ATTRIBUTE)).toBe("true");
    expect(text.getAttribute("data-clamped")).toBe("true");
    expect(font).toBeGreaterThanOrEqual(FIT_FLOOR_PX);
    expect(font).toBeLessThan(FIT_FLOOR_PX + 0.05);
    const lines = Number(text.style.getPropertyValue("--cf-clamp-lines"));
    expect(lines).toBe(Math.floor((120 - 2 * PADDING) / (font * LINE_HEIGHT)));
  });

  it("fits a face too small for the floor as before: shrink, and clamp only past FIT_MIN", () => {
    // Full size is 6 px, so no layout could print it at 9: the floor does not apply.
    const { text, face } = fitted({ basePx: 6, tierScale: 0.82, box: 40, longBox: 60, need: 1.4 });
    expect(face.hasAttribute(LONG_ATTRIBUTE)).toBe(false);
    expect(text.style.getPropertyValue("--cf-text-scale")).toBe("");
    expect(text.hasAttribute("data-clamped")).toBe(false);
    expect(fitOf(text)).toBeGreaterThanOrEqual(FIT_MIN);
    expect(fitOf(text)).toBeLessThan(1);
  });
});

/** A flat set of probe elements that share the model, each with a box of its own. */
function renderProbes(count: number, content: string): HTMLElement[] {
  const { container } = render(
    <>
      {Array.from({ length: count }, (_, index) => (
        <Probe key={String(index)} content={content} />
      ))}
    </>,
  );
  return [...container.querySelectorAll<HTMLElement>('[data-probe="text"]')];
}

describe("the batched scheduler", () => {
  const model: Model = { basePx: 11, tierScale: 0.62, box: 80, longBox: 120, need: 1.2 };

  it("leaves a card unfitted until the queue runs, and fitted once it has", () => {
    modelLayout(model);
    const [text] = renderProbes(1, "rules");
    expect(text?.style.getPropertyValue("--cf-fit")).toBe("");
    flushFits();
    expect(text?.closest(".cf")?.getAttribute(LONG_ATTRIBUTE)).toBe("true");
    expect(Number(text?.style.getPropertyValue("--cf-fit"))).toBeGreaterThan(0);
  });

  it("runs the queue on the microtask after the commit, before anything can paint", async () => {
    modelLayout(model);
    const [text] = renderProbes(1, "rules");
    expect(text?.style.getPropertyValue("--cf-fit")).toBe("");
    await Promise.resolve();
    expect(text?.style.getPropertyValue("--cf-fit")).not.toBe("");
  });

  it("fits many cards to exactly what fitting them one at a time gives", () => {
    modelLayout(model);
    const alone = renderProbes(1, "rules");
    flushFits();
    const expected = {
      fit: alone[0]?.style.getPropertyValue("--cf-fit"),
      scale: alone[0]?.style.getPropertyValue("--cf-text-scale"),
      long: alone[0]?.closest(".cf")?.getAttribute(LONG_ATTRIBUTE),
    };
    cleanup();

    const probes = renderProbes(12, "rules");
    flushFits();
    expect(probes).toHaveLength(12);
    for (const probe of probes) {
      expect(probe.style.getPropertyValue("--cf-fit")).toBe(expected.fit);
      expect(probe.style.getPropertyValue("--cf-text-scale")).toBe(expected.scale);
      expect(probe.closest(".cf")?.getAttribute(LONG_ATTRIBUTE)).toBe(expected.long);
    }
  });

  it("reads layout once per round for every card at once, not once per card", () => {
    // The layouts a run pays for must not grow with the number of cards.
    const layouts = (count: number): number => {
      modelLayout(model);
      renderProbes(count, "rules");
      flushFits();
      const paid = meter.paid;
      cleanup();
      vi.restoreAllMocks();
      return paid;
    };
    const one = layouts(1);
    const many = layouts(40);
    expect(one).toBeGreaterThan(0);
    expect(many).toBe(one);
  });

  it("drops a queued pass that is cancelled, and a pass's element is left as it was", () => {
    modelLayout(model);
    const { container } = render(<Probe content="rules" />);
    const text = container.querySelector<HTMLElement>('[data-probe="text"]');
    if (text === null) throw new Error("probe did not render");
    cleanup();
    const spare = document.createElement("span");
    spare.dataset.probe = "text";
    const done = vi.fn();
    const cancel = scheduleFit(spare, {}, done);
    cancel();
    flushFits();
    expect(done).not.toHaveBeenCalled();
    expect(spare.style.getPropertyValue("--cf-fit")).toBe("");
  });

  it("calls onDone when a pass has finished, with the fit already written", () => {
    modelLayout(model);
    const element = document.createElement("span");
    element.dataset.probe = "text";
    document.body.append(element);
    let seen = "";
    scheduleFit(element, {}, () => {
      seen = element.style.getPropertyValue("--cf-fit");
    });
    expect(seen).toBe("");
    flushFits();
    expect(seen).not.toBe("");
    element.remove();
  });

  it("hands onDone the box the element was left at, read with the rest of the batch", () => {
    modelLayout(model);
    const element = document.createElement("span");
    element.dataset.probe = "text";
    document.body.append(element);
    const done = vi.fn();
    scheduleFit(element, {}, done);
    flushFits();
    expect(done).toHaveBeenCalledExactlyOnceWith(`${String(WIDTH)}x${String(model.box)}`);
    element.remove();
  });

  it("lets one failing pass end alone: the rest finish and the first error is rethrown", () => {
    modelLayout(model);
    const good = document.createElement("span");
    good.dataset.probe = "text";
    document.body.append(good);
    const bad = document.createElement("span");
    bad.dataset.probe = "text";
    document.body.append(bad);
    // On the element itself: the model's getter is on the prototype, and is every element's.
    Object.defineProperty(bad, "clientWidth", {
      get() {
        throw new Error("detached mid-pass");
      },
    });
    const goodDone = vi.fn();
    scheduleFit(bad, {}, vi.fn());
    scheduleFit(good, {}, goodDone);
    expect(() => {
      flushFits();
    }).toThrow("detached mid-pass");
    expect(goodDone).toHaveBeenCalledTimes(1);
    // The queue is empty again: a later flush has nothing left to throw.
    expect(() => {
      flushFits();
    }).not.toThrow();
    good.remove();
    bad.remove();
  });
});

describe("a web font landing (fonts.css)", () => {
  afterEach(() => {
    delete (document as { fonts?: unknown }).fonts;
  });

  it("refits every mounted box when the document's fonts finish loading, and none once unmounted", () => {
    // jsdom has no FontFaceSet: an EventTarget stands in for `document.fonts`.
    const fonts = new EventTarget();
    Object.defineProperty(document, "fonts", { value: fonts, configurable: true });
    const model: Model = { basePx: 10, tierScale: 1, box: 60, longBox: 60, need: 0.4 };
    const { text, font: before } = fitted(model);

    // The web face sets wider than its fallback did: the same text now needs more room.
    model.need = 0.6;
    fonts.dispatchEvent(new Event("loadingdone"));
    flushFits();
    const after = fontOf(text, model);
    expect(after, "refitted smaller for the wider face").toBeLessThan(before);
    expect(model.need * after * after + 2 * PADDING, "and it fits its box again").toBeLessThanOrEqual(model.box + 1);

    cleanup();
    const setProperty = vi.mocked(CSSStyleDeclaration.prototype.setProperty);
    setProperty.mockClear();
    fonts.dispatchEvent(new Event("loadingdone"));
    flushFits();
    expect(setProperty, "an unmounted box is not refitted").not.toHaveBeenCalled();
  });
});

describe("a face the browser skips waits for it (#263)", () => {
  const model: Model = { basePx: 11, tierScale: 0.62, box: 80, longBox: 120, need: 1.2 };

  /**
   * A skippable item holding one text box. `state.skipped` is what `content-visibility: auto` would
   * say; `checkVisibility({ contentVisibilityAuto: true })` answers false for it.
   */
  function skippable(state: { skipped: boolean }, holder = true): { item: HTMLElement; text: HTMLElement } {
    const item = document.createElement("div");
    if (holder) item.setAttribute(SKIPPABLE_ATTRIBUTE, "");
    const text = document.createElement("span");
    text.dataset.probe = "text";
    item.append(text);
    document.body.append(item);
    Object.defineProperty(text, "checkVisibility", {
      value: (options?: { contentVisibilityAuto?: boolean }) => !(options?.contentVisibilityAuto === true && state.skipped),
    });
    return { item, text };
  }

  function unskip(item: HTMLElement, skipped = false): void {
    const event = new Event("contentvisibilityautostatechange");
    Object.defineProperty(event, "skipped", { value: skipped });
    item.dispatchEvent(event);
  }

  afterEach(() => {
    document.body.replaceChildren();
  });

  it("parks a skipped face's fit, and fits it once its item is un-skipped", async () => {
    modelLayout(model);
    const state = { skipped: true };
    const { item, text } = skippable(state);
    const done = vi.fn();
    scheduleFit(text, { floorPx: FIT_FLOOR_PX }, done);
    flushFits();
    expect(done).not.toHaveBeenCalled();
    expect(text.style.getPropertyValue("--cf-fit")).toBe("");
    expect(meter.paid).toBe(0);

    state.skipped = false;
    unskip(item);
    await Promise.resolve();
    expect(done).toHaveBeenCalledTimes(1);
    expect(text.style.getPropertyValue("--cf-fit")).not.toBe("");
  });

  it("fits every face one scroll un-skipped in one batch, from the first event, and leaves the rest parked", async () => {
    const layouts = async (count: number): Promise<{ paid: number; done: number; waiting: number }> => {
      modelLayout(model);
      const states = Array.from({ length: count + 1 }, () => ({ skipped: true }));
      const items = states.map((state) => skippable(state));
      const done = vi.fn();
      for (const { text } of items) scheduleFit(text, { floorPx: FIT_FLOOR_PX }, done);
      flushFits();
      // The scroll brings all but the last near; only the first item's event has been heard yet.
      for (const state of states.slice(0, count)) state.skipped = false;
      const first = items[0];
      if (first === undefined) throw new Error("no items");
      unskip(first.item);
      await Promise.resolve();
      const result = {
        paid: meter.paid,
        done: done.mock.calls.length,
        waiting: items.filter(({ text }) => text.style.getPropertyValue("--cf-fit") === "").length,
      };
      // The later events of the same scroll find nothing left to resume.
      for (const { item } of items.slice(1, count)) unskip(item);
      await Promise.resolve();
      expect(done.mock.calls.length).toBe(result.done);
      document.body.replaceChildren();
      vi.restoreAllMocks();
      return result;
    };
    const one = await layouts(1);
    const many = await layouts(12);
    expect(one).toEqual({ paid: one.paid, done: 1, waiting: 1 });
    expect(many).toEqual({ paid: one.paid, done: 12, waiting: 1 });
  });

  it("resumes nothing for an event that skips, or for a subtree with no parked fit", async () => {
    modelLayout(model);
    const state = { skipped: true };
    const { item, text } = skippable(state);
    const other = document.createElement("div");
    document.body.append(other);
    const done = vi.fn();
    scheduleFit(text, {}, done);
    flushFits();
    state.skipped = false;
    unskip(item, true);
    unskip(other);
    await Promise.resolve();
    expect(done).not.toHaveBeenCalled();
    unskip(item);
    await Promise.resolve();
    expect(done).toHaveBeenCalledTimes(1);
  });

  it("does not park a box whose holder said it is not skipped, nor one outside any holder", () => {
    modelLayout(model);
    // Hidden some other way: the holder's own event said its contents are laid out.
    const { item, text } = skippable({ skipped: true });
    unskip(item);
    const hidden = vi.fn();
    scheduleFit(text, {}, hidden);
    flushFits();
    expect(hidden).toHaveBeenCalledTimes(1);

    const outside = skippable({ skipped: true }, false);
    const loose = vi.fn();
    scheduleFit(outside.text, {}, loose);
    flushFits();
    expect(loose).toHaveBeenCalledTimes(1);
  });

  it("drops a parked fit that is cancelled", async () => {
    modelLayout(model);
    const state = { skipped: true };
    const { item, text } = skippable(state);
    const done = vi.fn();
    const cancel = scheduleFit(text, {}, done);
    flushFits();
    cancel();
    state.skipped = false;
    unskip(item);
    await Promise.resolve();
    expect(done).not.toHaveBeenCalled();
    expect(text.style.getPropertyValue("--cf-fit")).toBe("");
  });
});
