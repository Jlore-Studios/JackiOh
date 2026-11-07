// #3347: the hand's fan springs where it reflows: FLIP deltas, and the hook's use of them.

import { act, cleanup, render } from "@testing-library/react";
import { useRef, type ReactElement } from "react";
import { afterEach, describe, expect, it } from "vitest";

import { HAND_SPRING_MIN_PX } from "./config.ts";
import { flipDeltas, useHandSpring } from "./handSpring.ts";

const places = (entries: readonly [string, number][]): Map<string, number> => new Map(entries);

describe("#3347 flipDeltas", () => {
  it("#3347 signs before − after: a card that moved right starts that far to the left of its new place", () => {
    const deltas = flipDeltas(places([["a", 10], ["b", 100]]), places([["a", 40], ["b", 70]]), HAND_SPRING_MIN_PX);
    expect(deltas.get("a")).toBe(-30);
    expect(deltas.get("b")).toBe(30);
  });

  it("#3347 ignores a move under HAND_SPRING_MIN_PX, and keeps one of exactly that size", () => {
    const deltas = flipDeltas(
      places([["a", 10], ["b", 10]]),
      places([["a", 10 + HAND_SPRING_MIN_PX - 0.5], ["b", 10 + HAND_SPRING_MIN_PX]]),
      HAND_SPRING_MIN_PX,
    );
    expect([...deltas.keys()]).toEqual(["b"]);
  });

  it("#3347 ignores a key that is new or gone", () => {
    const deltas = flipDeltas(places([["gone", 0], ["kept", 0]]), places([["kept", 50], ["new", 80]]), HAND_SPRING_MIN_PX);
    expect([...deltas.keys()]).toEqual(["kept"]);
  });
});

describe("#3347 useHandSpring", () => {
  afterEach(() => {
    cleanup();
    Reflect.deleteProperty(HTMLElement.prototype, "offsetLeft");
  });

  function Fan({ ids }: { ids: string[] }): ReactElement {
    const ref = useRef<HTMLDivElement>(null);
    useHandSpring(ref, ids);
    return (
      <div ref={ref} data-testid="fan">
        {ids.map((id) => (
          <div key={id} className="hand-slot" data-slot={id} />
        ))}
      </div>
    );
  }

  /** jsdom lays nothing out: a slot's `offsetLeft` is its place in the row times 100. */
  function layout(): void {
    Object.defineProperty(HTMLElement.prototype, "offsetLeft", {
      configurable: true,
      get(this: HTMLElement) {
        const parent = this.parentElement;
        return parent === null ? 0 : Array.from(parent.children).indexOf(this) * 100;
      },
    });
  }

  /** Records what is written to a slot's `style.translate`, which jsdom keeps as a plain property. */
  function watch(slot: HTMLElement, writes: string[]): void {
    let held = "";
    Object.defineProperty(slot.style, "translate", {
      configurable: true,
      get: () => held,
      set: (value: string) => {
        writes.push(value);
        held = value;
      },
    });
  }

  it("#3347 a card that leaves springs the card after it from where it was, then lets go", () => {
    layout();
    const { rerender, getByTestId } = render(<Fan ids={["a", "b", "c"]} />);
    const slot = (id: string): HTMLElement => getByTestId("fan").querySelector<HTMLElement>(`[data-slot="${id}"]`) as HTMLElement;
    const aWrites: string[] = [];
    const cWrites: string[] = [];
    watch(slot("a"), aWrites);
    watch(slot("c"), cWrites);
    act(() => {
      rerender(<Fan ids={["a", "c"]} />);
    });
    // c was at 200 and is now at 100: it starts 100px to the right of its new place, then is released.
    expect(cWrites).toEqual(["100px 0", ""]);
    expect(aWrites).toEqual([]);
  });

  it("#3347 a render that moves nothing writes nothing", () => {
    layout();
    const { rerender, getByTestId } = render(<Fan ids={["a", "b"]} />);
    const writes: string[] = [];
    for (const element of Array.from(getByTestId("fan").children) as HTMLElement[]) watch(element, writes);
    act(() => {
      rerender(<Fan ids={["a", "b"]} />);
    });
    expect(writes).toEqual([]);
  });

  it("#3347 a card that arrives has no old place, and the cards it pushes aside spring from theirs", () => {
    layout();
    const { rerender, getByTestId } = render(<Fan ids={["a", "b"]} />);
    const slot = (id: string): HTMLElement => getByTestId("fan").querySelector<HTMLElement>(`[data-slot="${id}"]`) as HTMLElement;
    const bWrites: string[] = [];
    watch(slot("b"), bWrites);
    act(() => {
      rerender(<Fan ids={["a", "new", "b"]} />);
    });
    expect(bWrites).toEqual(["-100px 0", ""]);
    const newWrites: string[] = [];
    watch(slot("new"), newWrites);
    expect(newWrites).toEqual([]);
  });
});
