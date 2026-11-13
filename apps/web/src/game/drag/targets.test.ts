// The DOM target reader (`./targets.ts`, S9, R384, R510, R1081000): board identity comes from
// `data-testid` and zone data attributes. Built with real elements, one `it` per rule.

import { describe, expect, it, vi } from "vitest";

import { lockedZoneAt, pickDropSpot, placeOf, targetFromElement } from "./targets.ts";

type TargetsModule = typeof import("./targets.ts");

/** targets.ts as a page load builds it, its testid patterns included: a fresh module imported inside the test. */
async function freshTargets(): Promise<TargetsModule> {
  vi.resetModules();
  return import("./targets.ts");
}

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

  it("R384 R510 reads no activation from an empty attribute: no card, no ability and no Heroic Power", () => {
    const hand = element("div", { "data-testid": "hand-card-abc", "data-activate-for": "" });
    expect(targetFromElement(hand)).toEqual({ target: { on: "hand", instanceId: "abc" }, testid: "hand-card-abc" });
    const bare = element("button", {
      "data-testid": "other",
      "data-activate-for": "card-2",
      "data-activate-ability": "",
    });
    expect(targetFromElement(bare)).toStrictEqual({
      target: { on: "activate", instanceId: "card-2" },
      testid: "other",
    });
    const power = element("div", { "data-testid": "power", "data-instance-id": "" });
    expect(targetFromElement(power)).toBeNull();
  });

  it("reads nothing from an element with no testid, whatever else it carries", () => {
    const el = element("span", { "data-activate-for": "card-1", "data-instance-id": "hero-1" });
    expect(targetFromElement(el)).toBeNull();
  });

  it("finds a card's zone past the wrappers between them", () => {
    const zoneEl = zone("you", "units", "2");
    const wrapper = element("div", {}, zoneEl);
    const el = element("div", { "data-testid": "card-abc" }, wrapper);
    expect(targetFromElement(el)?.target).toEqual({ on: "unit", instanceId: "abc", side: "you", lane: 2 });
  });

  it("looks past a testid that names nothing to the target above it", () => {
    const hand = element("div", { "data-testid": "hand-card-abc" });
    const art = element("div", { "data-testid": "art-frame" }, hand);
    expect(targetFromElement(art)).toEqual({ target: { on: "hand", instanceId: "abc" }, testid: "hand-card-abc" });
  });

  it("refuses a zone testid whose zone attributes are missing", () => {
    expect(targetFromElement(element("div", { "data-testid": "zone-you-units-1" }))).toBeNull();
  });

  it("reads a hero and a zone", () => {
    expect(targetFromElement(element("div", { "data-testid": "hero-opponent" }))).toEqual({
      target: { on: "hero", side: "opponent" },
      testid: "hero-opponent",
    });
    expect(targetFromElement(element("div", { "data-testid": "hero-you" }))).toEqual({
      target: { on: "hero", side: "you" },
      testid: "hero-you",
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

  it("counts a button, an input and a role=button as controls, and nothing else", () => {
    const hand = element("div", { "data-testid": "hand-card-abc" });
    const span = element("span", {}, hand);
    expect(targetFromElement(span)?.target).toEqual({ on: "hand", instanceId: "abc" });
    expect(targetFromElement(element("input", {}, hand))).toBeNull();
    expect(targetFromElement(element("div", { role: "button" }, hand))).toBeNull();
  });

  it("treats a press on a control inside a card on the board as a press, never a drag source", () => {
    const zoneEl = zone("you", "units", "1");
    const card = element("div", { "data-testid": "card-abc" }, zoneEl);
    const button = element("button", {}, card);
    expect(targetFromElement(button)).toBeNull();
    expect(targetFromElement(card)).not.toBeNull();
  });

  it("answers null where nothing names a target", () => {
    expect(targetFromElement(element("div", {}))).toBeNull();
  });
});

describe("the testid patterns, as a page load builds them", () => {
  it("read a hand card's and a card's whole id, however long", async () => {
    const { targetFromElement: read } = await freshTargets();
    expect(read(element("div", { "data-testid": "hand-card-abc" }))?.target).toEqual({ on: "hand", instanceId: "abc" });
    const zoneEl = zone("you", "units", "3");
    expect(read(element("div", { "data-testid": "card-abc" }, zoneEl))?.target).toEqual({
      on: "unit",
      instanceId: "abc",
      side: "you",
      lane: 3,
    });
  });

  it("read a testid only whole: anything before or after a name, a line break included, names nothing", async () => {
    const { targetFromElement: read } = await freshTargets();
    for (const testid of ["xhand-card-1", "hand-card-a\nb"]) {
      expect(read(element("div", { "data-testid": testid })), testid).toBeNull();
    }
    for (const testid of ["superpower", "powerful"]) {
      expect(read(element("div", { "data-testid": testid, "data-instance-id": "hero-1" })), testid).toBeNull();
    }
    for (const testid of ["xzone-you-units-1", "zone-you-units-1x"]) {
      const el = element("div", { "data-testid": testid, "data-side": "you", "data-row": "units", "data-lane": "1" });
      expect(read(el), testid).toBeNull();
    }
    // Not a card: the press falls through to the zone the element stands in.
    const zoneEl = zone("you", "units", "5");
    for (const testid of ["xcard-1", "card-a\nb"]) {
      expect(read(element("div", { "data-testid": testid }, zoneEl))?.target, testid).toEqual({
        on: "zone",
        side: "you",
        row: "units",
        lane: 5,
      });
    }
  });

  it("R510 read a Heroic Power under power and under any power- testid, and a zone of a two-digit lane", async () => {
    const { targetFromElement: read } = await freshTargets();
    for (const testid of ["power", "power-ab"]) {
      const el = element("div", { "data-testid": testid, "data-instance-id": "hero-1" });
      expect(read(el)?.target, testid).toEqual({ on: "activate", instanceId: "hero-1" });
    }
    expect(read(zone("you", "units", "12"))?.target).toEqual({ on: "zone", side: "you", row: "units", lane: 12 });
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
    // Even with another zone of its row offered, which would report it for a play.
    expect(
      lockedZoneAt([zoneEl], { kind: "attack" as const, dropTestids: new Set(["zone-you-units-1"]) }),
    ).toBeNull();
  });

  it("reports a locked zone only while another zone in its row is offered", () => {
    const row = element("div", {});
    const locked = zone("you", "units", "2", row);
    locked.setAttribute("data-locked", "true");
    expect(lockedZoneAt([locked], plan(["zone-you-units-1"]))?.testid).toBe("zone-you-units-2");
    expect(lockedZoneAt([locked], plan(["zone-you-backrow-1"]))).toBeNull();
    // One zone of its row among others is enough.
    expect(lockedZoneAt([locked], plan(["zone-you-units-1", "zone-you-backrow-1"]))?.testid).toBe(
      "zone-you-units-2",
    );
  });

  it("never reports the opponent's locked zone", () => {
    const theirs = zone("opponent", "units", "2");
    theirs.setAttribute("data-locked", "true");
    expect(lockedZoneAt([theirs], plan(["zone-you-units-1"]))).toBeNull();
  });

  it("stays silent for an offered zone and an unlocked one", () => {
    const row = element("div", {});
    const offered = zone("you", "units", "1", row);
    expect(lockedZoneAt([offered], plan(["zone-you-units-1"]))).toBeNull();
    const open = zone("you", "units", "3", row);
    expect(lockedZoneAt([open], plan(["zone-you-units-1"]))).toBeNull();
  });
});
