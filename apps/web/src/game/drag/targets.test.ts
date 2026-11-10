// The DOM target reader (`./targets.ts`, S9, R384, R510, R1081000): board identity comes from
// `data-testid` and zone data attributes. Built with real elements, one `it` per rule.

import { describe, expect, it } from "vitest";

import { lockedZoneAt, pickDropSpot, placeOf, targetFromElement } from "./targets.ts";

function element(tag: string, attrs: Record<string, string>, parent?: Element): Element {
  const el = document.createElement(tag);
  for (const [key, value] of Object.entries(attrs)) el.setAttribute(key, value);
  (parent ?? document.body).appendChild(el);
  return el;
}

function zone(side: string, row: string, lane: string, parent?: Element): Element {
  return element("div", {
    "data-testid": `zone-${side}-${row}-${lane}`,
    "data-side": side,
    "data-row": row,
    "data-lane": lane,
  }, parent);
}

describe("placeOf", () => {
  it("reads a zone's side, row and lane from its data attributes", () => {
    expect(placeOf(zone("you", "units", "3"))).toEqual({ side: "you", row: "units", lane: 3 });
  });

  it("refuses a lane that is not a positive integer and an unknown side or row", () => {
    expect(placeOf(zone("you", "units", "0"))).toBeNull();
    expect(placeOf(zone("you", "units", "1.5"))).toBeNull();
    expect(placeOf(zone("me", "units", "1"))).toBeNull();
    expect(placeOf(zone("you", "hand", "1"))).toBeNull();
  });
});

describe("targetFromElement", () => {
  it("reads a hand card by its testid", () => {
    const el = element("div", { "data-testid": "hand-card-abc" });
    expect(targetFromElement(el)).toEqual({ target: { on: "hand", instanceId: "abc" }, testid: "hand-card-abc" });
  });

  it("reads a unit in a units zone with its side and lane", () => {
    const zoneEl = zone("you", "units", "2");
    const el = element("div", { "data-testid": "card-abc" }, zoneEl);
    expect(targetFromElement(el)).toEqual({
      target: { on: "unit", instanceId: "abc", side: "you", lane: 2 },
      testid: "card-abc",
    });
  });

  it("reads a backrow card as backrow, and a Unit standing there as a unit (R1081000)", () => {
    const zoneEl = zone("opponent", "backrow", "4");
    const back = element("div", { "data-testid": "card-spell" }, zoneEl);
    expect(targetFromElement(back)?.target).toEqual({
      on: "backrow",
      instanceId: "spell",
      side: "opponent",
      lane: 4,
    });
    const unit = element("div", { "data-testid": "card-brute", class: "card-unit" }, zoneEl);
    expect(targetFromElement(unit)?.target).toEqual({
      on: "unit",
      instanceId: "brute",
      side: "opponent",
      lane: 4,
    });
  });

  it("refuses a card outside any zone", () => {
    const el = element("div", { "data-testid": "card-abc" });
    expect(targetFromElement(el)).toBeNull();
  });

  it("reads an activation from its attributes, never its testid (R384, R510)", () => {
    const el = element("button", {
      "data-testid": "whatever",
      "data-activate-for": "card-1",
      "data-activate-ability": "boom",
    });
    expect(targetFromElement(el)).toEqual({
      target: { on: "activate", instanceId: "card-1", ability: "boom" },
      testid: "whatever",
    });
    const bare = element("button", { "data-testid": "other", "data-activate-for": "card-2" });
    expect(targetFromElement(bare)).toEqual({
      target: { on: "activate", instanceId: "card-2" },
      testid: "other",
    });
  });

  it("reads a Heroic Power by its data-instance-id", () => {
    const el = element("div", { "data-testid": "power", "data-instance-id": "hero-1" });
    expect(targetFromElement(el)).toEqual({
      target: { on: "activate", instanceId: "hero-1" },
      testid: "power",
    });
    const missing = element("div", { "data-testid": "power" });
    expect(targetFromElement(missing)).toBeNull();
  });

  it("reads a hero and a zone", () => {
    expect(targetFromElement(element("div", { "data-testid": "hero-opponent" }))).toEqual({
      target: { on: "hero", side: "opponent" },
      testid: "hero-opponent",
    });
    const zoneEl = zone("you", "backrow", "1");
    expect(targetFromElement(zoneEl)).toEqual({
      target: { on: "zone", side: "you", row: "backrow", lane: 1 },
      testid: "zone-you-backrow-1",
    });
  });

  it("treats a press on a control inside a hand card as a press, never a drag source", () => {
    const hand = element("div", { "data-testid": "hand-card-abc" });
    const button = element("button", {}, hand);
    const inner = element("span", {}, button);
    expect(targetFromElement(inner)).toBeNull();
    expect(targetFromElement(hand)).not.toBeNull();
  });

  it("answers null where nothing names a target", () => {
    expect(targetFromElement(element("div", {}))).toBeNull();
  });
});

describe("pickDropSpot", () => {
  it("lands on the first allowed testid up the stack", () => {
    const zoneEl = zone("you", "units", "1");
    expect(pickDropSpot([zoneEl], new Set(["zone-you-units-1"]))).toEqual({
      at: "target",
      target: { on: "zone", side: "you", row: "units", lane: 1 },
      testid: "zone-you-units-1",
    });
  });

  it("skips a hit the plan does not offer", () => {
    const zoneEl = zone("you", "units", "1");
    const hero = element("div", { "data-testid": "hero-opponent" });
    expect(pickDropSpot([zoneEl, hero], new Set(["hero-opponent"]))).toEqual({
      at: "target",
      target: { on: "hero", side: "opponent" },
      testid: "hero-opponent",
    });
  });

  it("calls an empty stack and the ground outside the board outside", () => {
    expect(pickDropSpot([], new Set())).toEqual({ at: "outside" });
    const elsewhere = element("div", { "data-testid": "somewhere" });
    expect(pickDropSpot([elsewhere], new Set())).toEqual({ at: "outside" });
  });

  it("calls a drop on the player's own hand outside, and on the board a board drop", () => {
    const board = element("div", { "data-testid": "board" });
    const hand = element("div", { "data-testid": "hand-you" }, board);
    const card = element("span", {}, hand);
    expect(pickDropSpot([card], new Set())).toEqual({ at: "outside" });
    const felt = element("span", {}, board);
    expect(pickDropSpot([felt], new Set())).toEqual({ at: "board" });
  });
});

describe("lockedZoneAt", () => {
  const plan = (testids: string[]) => ({ kind: "play" as const, dropTestids: new Set(testids) });

  it("answers null when the drag is not a play", () => {
    const zoneEl = zone("you", "units", "2");
    zoneEl.setAttribute("data-locked", "true");
    expect(
      lockedZoneAt([zoneEl], { kind: "attack" as const, dropTestids: new Set() }),
    ).toBeNull();
  });

  it("reports a locked zone only while another zone in its row is offered", () => {
    const row = element("div", {});
    const locked = zone("you", "units", "2", row);
    locked.setAttribute("data-locked", "true");
    expect(lockedZoneAt([locked], plan(["zone-you-units-1"]))?.testid).toBe("zone-you-units-2");
    expect(lockedZoneAt([locked], plan(["zone-you-backrow-1"]))).toBeNull();
  });

  it("stays silent for an offered zone and an unlocked one", () => {
    const row = element("div", {});
    const offered = zone("you", "units", "1", row);
    expect(lockedZoneAt([offered], plan(["zone-you-units-1"]))).toBeNull();
    const open = zone("you", "units", "3", row);
    expect(lockedZoneAt([open], plan(["zone-you-units-1"]))).toBeNull();
  });
});
