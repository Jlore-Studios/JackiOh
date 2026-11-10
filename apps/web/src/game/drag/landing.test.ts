// The card a drop has just played (`./landing.ts`, S9): DragLayer names it here, Hand reads it out of
// the fan meanwhile, and the fx stand-in flies on from where the pointer let go (#3347). One `it` per
// rule; the module's state is cleared before each test, as no other test may read this one's landing.

import { beforeEach, describe, expect, it, vi } from "vitest";

import {
  currentLanding,
  dropPointOfTestid,
  setLanding,
  subscribeLanding,
} from "./landing.ts";

beforeEach(() => {
  setLanding(null);
});

describe("currentLanding", () => {
  it("starts with no card landing", () => {
    expect(currentLanding()).toBeNull();
  });

  it("names the card a drop has just played", () => {
    setLanding("instance-1");
    expect(currentLanding()).toBe("instance-1");
  });

  it("clears the name with null once the board shows a newer view", () => {
    setLanding("instance-1");
    setLanding(null);
    expect(currentLanding()).toBeNull();
  });
});

describe("setLanding", () => {
  it("notifies once on a change and stays silent on the same name", () => {
    const listener = vi.fn();
    subscribeLanding(listener);
    setLanding("instance-1");
    setLanding("instance-1");
    expect(listener).toHaveBeenCalledTimes(1);
  });

  it("moves the release point without a second notice when the name is unchanged", () => {
    const listener = vi.fn();
    setLanding("instance-1", { x: 1, y: 2 });
    subscribeLanding(listener);
    setLanding("instance-1", { x: 3, y: 4 });
    expect(listener).not.toHaveBeenCalled();
    expect(dropPointOfTestid("hand-card-instance-1")).toEqual({ x: 3, y: 4 });
  });

  it("forgets the release point when the call carries none", () => {
    setLanding("instance-1", { x: 1, y: 2 });
    setLanding("instance-1");
    expect(dropPointOfTestid("hand-card-instance-1")).toBeNull();
  });

  it("stops telling an unsubscribed listener", () => {
    const listener = vi.fn();
    const stop = subscribeLanding(listener);
    stop();
    setLanding("instance-1");
    expect(listener).not.toHaveBeenCalled();
  });
});

describe("dropPointOfTestid", () => {
  it("answers where the pointer let go for that card's hand slot", () => {
    setLanding("instance-1", { x: 10, y: 20 });
    expect(dropPointOfTestid("hand-card-instance-1")).toEqual({ x: 10, y: 20 });
  });

  it("answers null for another card's slot", () => {
    setLanding("instance-1", { x: 10, y: 20 });
    expect(dropPointOfTestid("hand-card-instance-2")).toBeNull();
  });

  it("answers null with no landing and hands out a copy, not the kept point", () => {
    expect(dropPointOfTestid("hand-card-instance-1")).toBeNull();
    setLanding("instance-1", { x: 10, y: 20 });
    const point = dropPointOfTestid("hand-card-instance-1");
    expect(point).toEqual({ x: 10, y: 20 });
    point!.x = 999;
    expect(dropPointOfTestid("hand-card-instance-1")).toEqual({ x: 10, y: 20 });
  });
});
