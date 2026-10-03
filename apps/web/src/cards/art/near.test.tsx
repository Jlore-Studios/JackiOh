// A grid's lazy art (near.ts, CardArt `lazy`): drawn once the window is near the box that scrolls it.
// jsdom has no IntersectionObserver, so this file stands one in and fires it by hand.

import { act, cleanup, render } from "@testing-library/react";
import { afterEach, beforeEach, describe, expect, it, vi } from "vitest";

import { CATALOG } from "@jackioh/cards";

import { ART_NEAR_MARGIN_PX } from "../constants.ts";
import { CardArt } from "./index.ts";
import { whenNear } from "./near.ts";

type Observer = {
  callback: IntersectionObserverCallback;
  options: IntersectionObserverInit | undefined;
  targets: Set<Element>;
  disconnected: boolean;
};

const observers: Observer[] = [];

class FakeIntersectionObserver {
  readonly state: Observer;
  constructor(callback: IntersectionObserverCallback, options?: IntersectionObserverInit) {
    this.state = { callback, options, targets: new Set(), disconnected: false };
    observers.push(this.state);
  }
  observe(target: Element): void {
    this.state.targets.add(target);
  }
  unobserve(target: Element): void {
    this.state.targets.delete(target);
  }
  disconnect(): void {
    this.state.disconnected = true;
    this.state.targets.clear();
  }
}

/** Reports `target` as inside (or outside) its observer's margin. */
function intersect(observer: Observer, target: Element, isIntersecting = true): void {
  act(() => {
    observer.callback([{ target, isIntersecting } as IntersectionObserverEntry], {} as IntersectionObserver);
  });
}

beforeEach(() => {
  observers.length = 0;
  vi.stubGlobal("IntersectionObserver", FakeIntersectionObserver);
});

afterEach(() => {
  cleanup();
  vi.unstubAllGlobals();
});

function Art({ lazy }: { lazy: boolean }) {
  const card = CATALOG["core-002"];
  if (card === undefined) throw new Error("the catalog has no core-002");
  return <CardArt defId={card.id} radiant={false} tags={card.tags} type={card.type} shape="portrait" manifest={{}} lazy={lazy} />;
}

function art(container: HTMLElement): HTMLElement {
  const found = container.querySelector<HTMLElement>(".cf-art");
  if (found === null) throw new Error("no .cf-art");
  return found;
}

describe("lazy procedural art", () => {
  it("draws nothing for a window that is not near, and says so", () => {
    const { container } = render(<Art lazy />);
    const window = art(container);
    expect(window.getAttribute("data-art")).toBe("procedural");
    expect(window.getAttribute("data-art-pending")).toBe("true");
    expect(window.style.backgroundImage).toBe("");
  });

  it("draws the picture once the window comes near, and keeps it", () => {
    const { container } = render(<Art lazy />);
    const window = art(container);
    const [observer] = observers;
    if (observer === undefined) throw new Error("no observer");
    intersect(observer, window, false);
    expect(window.getAttribute("data-art-pending")).toBe("true");
    intersect(observer, window);
    expect(window.hasAttribute("data-art-pending")).toBe(false);
    expect(window.style.backgroundImage).toContain("data:image/svg+xml");
    // Once drawn it is no longer watched.
    expect(observer.targets.size).toBe(0);
  });

  it("draws at once when it is not asked to be lazy, and at once when nothing can watch", () => {
    const eager = render(<Art lazy={false} />);
    expect(art(eager.container).style.backgroundImage).toContain("data:image/svg+xml");
    expect(observers).toHaveLength(0);
    cleanup();

    vi.unstubAllGlobals();
    const unwatched = render(<Art lazy />);
    expect(art(unwatched.container).style.backgroundImage).toContain("data:image/svg+xml");
  });

  it("stops watching a window that unmounts before it was near", () => {
    const { unmount } = render(<Art lazy />);
    const [observer] = observers;
    expect(observer?.targets.size).toBe(1);
    unmount();
    expect(observer?.disconnected).toBe(true);
  });
});

describe("whenNear", () => {
  it("watches every window of one scrolling box with one observer, widened by the margin", () => {
    const box = document.createElement("div");
    box.style.overflowY = "auto";
    const first = document.createElement("span");
    const second = document.createElement("span");
    box.append(first, second);
    document.body.append(box);

    const near = vi.fn();
    const stopFirst = whenNear(first, () => {
      near("first");
    });
    const stopSecond = whenNear(second, () => {
      near("second");
    });

    expect(observers).toHaveLength(1);
    const [observer] = observers;
    expect(observer?.options?.root).toBe(box);
    expect(observer?.options?.rootMargin).toBe(`${String(ART_NEAR_MARGIN_PX)}px 0px`);
    expect(observer?.targets.size).toBe(2);

    if (observer === undefined) throw new Error("no observer");
    intersect(observer, second);
    expect(near).toHaveBeenCalledExactlyOnceWith("second");
    // A window reported twice is called once.
    intersect(observer, second);
    expect(near).toHaveBeenCalledTimes(1);

    // The second left when it was near; the first is the last one watched, and its stop ends the watch.
    expect(observer.disconnected).toBe(false);
    stopFirst();
    expect(observer.disconnected).toBe(true);
    // Stopping a window that has already left is harmless.
    stopSecond();
    box.remove();
  });

  it("watches against the viewport when nothing above the window scrolls", () => {
    const loose = document.createElement("span");
    document.body.append(loose);
    const stop = whenNear(loose, () => undefined);
    expect(observers[0]?.options?.root).toBeNull();
    stop();
    expect(observers[0]?.disconnected).toBe(true);
    loose.remove();
  });

  it("uses the nearest scrolling ancestor, and not one that merely clips", () => {
    const outer = document.createElement("div");
    outer.style.overflowY = "scroll";
    const clip = document.createElement("div");
    clip.style.overflowY = "hidden";
    const inner = document.createElement("div");
    inner.style.overflowY = "auto";
    const window = document.createElement("span");
    inner.append(window);
    clip.append(inner);
    outer.append(clip);
    document.body.append(outer);

    const stop = whenNear(window, () => undefined);
    expect(observers[0]?.options?.root).toBe(inner);
    stop();
    outer.remove();
  });
});
