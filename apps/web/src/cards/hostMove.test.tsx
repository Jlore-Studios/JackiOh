// #3674 (R3351000): a card host's one pointermove listener (hostMove.ts). Every reader of a move reads before any of
// them writes, so a face with live art (artLive.ts) and a tilt (tilt.ts) lays its style out once a move, not once more
// for the second reader's box. The frame times are e2e's cypress/component/fx-phone-living-game.cy.tsx.

import { act, cleanup, render } from "@testing-library/react";
import { useRef, type ReactElement } from "react";
import { afterEach, beforeEach, describe, expect, it, vi } from "vitest";

import { useArtLive } from "./artLive.ts";
import { manualFrames, type ManualFrames } from "../test/frames.ts";
import { hearHostMove } from "./hostMove.ts";
import { resetTiltForTests, useCardTilt } from "./tilt.ts";

/** A box 200 by 280 at the page's origin, which jsdom's layout does not give. */
const BOX = { left: 0, top: 0, width: 200, height: 280, right: 200, bottom: 280, x: 0, y: 0, toJSON: () => ({}) };

let frames: ManualFrames;

/** A move, and the frame the host handles it in (#3610). */
function move(target: EventTarget, x: number, y: number): void {
  act(() => {
    target.dispatchEvent(new MouseEvent("pointermove", { bubbles: true, clientX: x, clientY: y }));
  });
  frames.flush();
}

/** A face that both follows the pointer with its art and tilts toward it, on one host: a hand card's, on the board. */
function LiveTiltFace(): ReactElement {
  const ref = useRef<HTMLSpanElement>(null);
  useArtLive(ref, true);
  useCardTilt(ref, "host", true);
  return (
    <div className="cf-host" data-testid="host">
      <span ref={ref} className="cf" />
    </div>
  );
}

beforeEach(() => {
  frames = manualFrames();
  resetTiltForTests();
});

afterEach(() => {
  frames.restore();
  cleanup();
  vi.restoreAllMocks();
});

describe("R3351000 a card host's moves are read, then written (hostMove.ts)", () => {
  it("R3351000 every reader of a move reads before any of them writes, in the order they came", () => {
    const host = document.createElement("div");
    const order: string[] = [];
    const stopA = hearHostMove(host, () => {
      order.push("read a");
      return () => order.push("write a");
    });
    const stopB = hearHostMove(host, () => {
      order.push("read b");
      return undefined;
    });
    const stopC = hearHostMove(host, () => {
      order.push("read c");
      return () => order.push("write c");
    });
    move(host, 1, 1);
    expect(order).toEqual(["read a", "read b", "read c", "write a", "write c"]);
    stopA();
    stopB();
    stopC();
  });

  it("R3351000 a host has one listener while it has a reader, and none after its last", () => {
    const host = document.createElement("div");
    const add = vi.spyOn(host, "addEventListener");
    const remove = vi.spyOn(host, "removeEventListener");
    const heard: number[] = [];
    const stopA = hearHostMove(host, (event) => {
      heard.push(event.clientX);
      return undefined;
    });
    const stopB = hearHostMove(host, () => undefined);
    expect(add).toHaveBeenCalledTimes(1);
    stopA();
    expect(remove).not.toHaveBeenCalled();
    move(host, 5, 5);
    expect(heard).toEqual([]);
    stopB();
    expect(remove).toHaveBeenCalledTimes(1);
    // A host heard again after its last reader went has a listener of its own again.
    const stopC = hearHostMove(host, (event) => {
      heard.push(event.clientX);
      return undefined;
    });
    expect(add).toHaveBeenCalledTimes(2);
    move(host, 7, 7);
    expect(heard).toEqual([7]);
    stopC();
  });

  it("R3351000 a move over a face that follows the pointer and tilts reads both boxes before either writes", () => {
    const { getByTestId, container, unmount } = render(<LiveTiltFace />);
    const host = getByTestId("host");
    const cf = container.querySelector<HTMLElement>(".cf");
    if (cf === null) throw new Error("no face");
    const log: string[] = [];
    vi.spyOn(HTMLElement.prototype, "getBoundingClientRect").mockImplementation(function (this: HTMLElement) {
      log.push(`read ${this.className}`);
      return BOX as DOMRect;
    });
    const setProperty = CSSStyleDeclaration.prototype.setProperty;
    vi.spyOn(CSSStyleDeclaration.prototype, "setProperty").mockImplementation(function (this: CSSStyleDeclaration, property: string, value: string | null, priority?: string) {
      log.push(`write ${property}`);
      setProperty.call(this, property, value, priority);
    });
    move(host, 150, 70);
    const firstWrite = log.findIndex((entry) => entry.startsWith("write"));
    expect(log.slice(0, firstWrite).sort()).toEqual(["read cf", "read cf-host"]);
    expect(log.slice(firstWrite).every((entry) => entry.startsWith("write"))).toBe(true);
    // Both still write what they read: the art's aim at the pointer's place in the face, the tilt's pose in the host.
    expect(cf.style.getPropertyValue("--art-aim-x")).toBe("0.500");
    expect(cf.style.getPropertyValue("--art-aim-y")).toBe("-0.500");
    expect(cf.style.getPropertyValue("--cf-tilt-x")).toBe("0.5");
    expect(cf.style.getPropertyValue("--cf-tilt-y")).toBe("-0.5");
    unmount();
    log.length = 0;
    move(host, 10, 10);
    expect(log).toEqual([]);
  });
});
