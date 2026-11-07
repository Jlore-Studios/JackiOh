// Polish 1 (docs/polish/1-animations.md, S10): `mountDomEffect`, B39, to the S10 table's contract: one
// appended element per cue, `class="fx-<kind>"`, `data-fx="<kind>"`, `aria-hidden="true"`, no text node
// (words ride in `data-amount` / `data-text`, shown only through CSS `content: attr(…)`), geometry and
// timing in `--fx-*` properties. The boxes are whole with even sizes, so lengths are plain `<n>px`.

import { afterEach, beforeEach, describe, expect, it, vi } from "vitest";

import { FX_HOLD_TURN_DEG, FX_HOLD_TURN_PX } from "./constants.ts";
import { mountDomEffect, mountHold, type DomEffect } from "./dom.ts";
import type { FxBox, FxDomCue, FxHoldCue } from "./types.ts";

const AT: FxBox = { x: 100, y: 200, width: 80, height: 120 }; // center (140, 260)
const TO: FxBox = { x: 700, y: 40, width: 60, height: 60 }; // center (730, 70)

let root: HTMLElement;

beforeEach(() => {
  root = document.createElement("div");
  document.body.appendChild(root);
});

afterEach(() => {
  vi.useRealTimers();
  document.body.innerHTML = "";
});

function cssVar(el: HTMLElement, name: string): string {
  return el.style.getPropertyValue(name).trim();
}

function textNodeCount(el: Node): number {
  const walker = document.createTreeWalker(el, NodeFilter.SHOW_TEXT);
  let n = 0;
  while (walker.nextNode() !== null) n += 1;
  return n;
}

/** The part of the S10 contract every kind shares. */
function expectMounted(effect: DomEffect | null, cue: FxDomCue): HTMLElement {
  expect(effect).not.toBeNull();
  if (effect === null) throw new Error("nothing mounted");
  const el = effect.el;
  expect(el.parentElement).toBe(root);
  expect(root.childElementCount).toBe(1);
  expect(el.classList.contains(`fx-${cue.kind}`)).toBe(true);
  expect(el.getAttribute("data-fx")).toBe(cue.kind);
  expect(el.getAttribute("aria-hidden")).toBe("true");
  expect(cssVar(el, "--fx-ms")).toBe(`${cue.durationMs}ms`);
  expect(textNodeCount(el)).toBe(0);
  expect(el.textContent).toBe("");
  return el;
}

function expectCenteredOn(el: HTMLElement, box: FxBox): void {
  expect(cssVar(el, "--fx-x")).toBe(`${box.x + box.width / 2}px`);
  expect(cssVar(el, "--fx-y")).toBe(`${box.y + box.height / 2}px`);
  expect(cssVar(el, "--fx-w")).toBe(`${box.width}px`);
  expect(cssVar(el, "--fx-h")).toBe(`${box.height}px`);
}

function expectCovering(el: HTMLElement, box: FxBox): void {
  expect(cssVar(el, "--fx-x")).toBe(`${box.x}px`);
  expect(cssVar(el, "--fx-y")).toBe(`${box.y}px`);
  expect(cssVar(el, "--fx-w")).toBe(`${box.width}px`);
  expect(cssVar(el, "--fx-h")).toBe(`${box.height}px`);
}

const anchor = { kind: "testid", testid: "card-u1" } as const;

const SPLAT_DAMAGE: FxDomCue = { kind: "splat", tone: "damage", amount: 5, at: anchor, delayMs: 0, durationMs: 950 };
const SPLAT_HEAL: FxDomCue = { kind: "splat", tone: "heal", amount: 3, at: anchor, delayMs: 60, durationMs: 890 };
const SPLAT_LOSS: FxDomCue = { kind: "splat", tone: "loss", amount: 2, at: anchor, delayMs: 0, durationMs: 950 };
const SPLAT_ABSORB: FxDomCue = { kind: "splat", tone: "absorb", amount: 4, at: anchor, delayMs: 0, durationMs: 950 };
const RAYS: FxDomCue = { kind: "rays", tone: "mythic", at: anchor, delayMs: 0, durationMs: 850 };
const SHEEN: FxDomCue = { kind: "sheen", at: anchor, delayMs: 0, durationMs: 400 };
const GHOST: FxDomCue = { kind: "ghost", from: anchor, to: { kind: "testid", testid: "hand-you" }, delayMs: 0, durationMs: 250 };
const ARROWS_UP: FxDomCue = { kind: "arrows", direction: "up", at: anchor, delayMs: 0, durationMs: 550 };
const ARROWS_DOWN: FxDomCue = { kind: "arrows", direction: "down", at: anchor, delayMs: 0, durationMs: 550 };
const BANNER: FxDomCue = { kind: "banner", text: "Your turn", tone: "you", delayMs: 0, durationMs: 1500 };
const RESULT: FxDomCue = { kind: "result", outcome: "defeat", text: "Defeat", delayMs: 0, durationMs: 3200 };
const TINT = { rim: "#ffb35c", core: "#fff1c2", glow: "#ff6a2b" };
const FOG: FxDomCue = {
  kind: "fog",
  tone: "damage",
  from: anchor,
  to: { kind: "testid", testid: "card-u2" },
  tint: TINT,
  icon: { d: "M0 0h24v24H0z", rule: "nonzero" },
  delayMs: 0,
  durationMs: 1100,
};
const ZONE: FxDomCue = { kind: "zone", at: anchor, tint: TINT, text: "×12", direction: "down", delayMs: 0, durationMs: 1100 };

/* Each kind mounts one element to the contract */

describe("B39 — mountDomEffect mounts one element per cue", () => {
  it("B39 a damage splat carries its tone and a hyphen-minus amount, centered on its box", () => {
    const el = expectMounted(mountDomEffect(root, SPLAT_DAMAGE, { at: AT }), SPLAT_DAMAGE);
    expect(el.getAttribute("data-tone")).toBe("damage");
    const amount = el.getAttribute("data-amount") ?? "";
    expect(amount).toBe("-5");
    expect(amount.charCodeAt(0)).toBe(0x2d);
    expectCenteredOn(el, AT);
  });

  it("B39 a heal splat carries a plus-sign amount", () => {
    const el = expectMounted(mountDomEffect(root, SPLAT_HEAL, { at: AT }), SPLAT_HEAL);
    expect(el.getAttribute("data-tone")).toBe("heal");
    const amount = el.getAttribute("data-amount") ?? "";
    expect(amount).toBe("+3");
    expect(amount.charCodeAt(0)).toBe(0x2b);
    expectCenteredOn(el, AT);
  });

  it("B39 a health-loss splat is signed like damage", () => {
    const el = expectMounted(mountDomEffect(root, SPLAT_LOSS, { at: AT }), SPLAT_LOSS);
    expect(el.getAttribute("data-tone")).toBe("loss");
    expect(el.getAttribute("data-amount")).toBe("-2");
  });

  it("Issue #2790 an absorb splat carries its tone and a hyphen-minus amount", () => {
    const el = expectMounted(mountDomEffect(root, SPLAT_ABSORB, { at: AT }), SPLAT_ABSORB);
    expect(el.getAttribute("data-tone")).toBe("absorb");
    const amount = el.getAttribute("data-amount") ?? "";
    expect(amount).toBe("-4");
    expect(amount.charCodeAt(0)).toBe(0x2d);
    expectCenteredOn(el, AT);
  });

  it("B39 rays carry their tone and are centered on their box", () => {
    const el = expectMounted(mountDomEffect(root, RAYS, { at: AT }), RAYS);
    expect(el.getAttribute("data-tone")).toBe("mythic");
    expectCenteredOn(el, AT);
  });

  it("B39 a sheen covers its box from the top-left corner", () => {
    const el = expectMounted(mountDomEffect(root, SHEEN, { at: AT }), SHEEN);
    expectCovering(el, AT);
  });

  it("B39 a ghost starts at its source's center and flies by the offset to its destination's center", () => {
    const el = expectMounted(mountDomEffect(root, GHOST, { from: AT, to: TO }), GHOST);
    expect(cssVar(el, "--fx-x")).toBe("140px");
    expect(cssVar(el, "--fx-y")).toBe("260px");
    expect(cssVar(el, "--fx-dx")).toBe("590px");
    expect(cssVar(el, "--fx-dy")).toBe("-190px");
  });

  it("B39 arrows carry their direction and cover their box", () => {
    const up = expectMounted(mountDomEffect(root, ARROWS_UP, { at: AT }), ARROWS_UP);
    expect(up.getAttribute("data-direction")).toBe("up");
    expectCovering(up, AT);

    root.innerHTML = "";
    const down = expectMounted(mountDomEffect(root, ARROWS_DOWN, { at: AT }), ARROWS_DOWN);
    expect(down.getAttribute("data-direction")).toBe("down");
  });

  it("B39 a banner needs no box and carries its tone and text as attributes", () => {
    const el = expectMounted(mountDomEffect(root, BANNER, {}), BANNER);
    expect(el.getAttribute("data-tone")).toBe("you");
    expect(el.getAttribute("data-text")).toBe("Your turn");
  });

  it("B39 a result needs no box and carries its outcome and text as attributes", () => {
    const el = expectMounted(mountDomEffect(root, RESULT, {}), RESULT);
    expect(el.getAttribute("data-outcome")).toBe("defeat");
    expect(el.getAttribute("data-text")).toBe("Defeat");
  });

  it("issue #124 a fog spans its row's boxes, tinted, with puffs and one icon per emblem", () => {
    const el = expectMounted(mountDomEffect(root, FOG, { from: AT, to: TO }), FOG);
    expect(el.getAttribute("data-tone")).toBe("damage");
    // The smallest box holding both, grown by FX_FOG_PAD of its height on every side.
    expect(root.firstElementChild).toBe(el);
    expect(el.querySelectorAll(".fx-fog-puff").length).toBeGreaterThan(0);
    expect(el.querySelectorAll("svg.fx-fog-icon path").length).toBeGreaterThan(0);
    expect(cssVar(el, "--fx-tint-glow")).toBe(TINT.glow);
    // Without both row ends there is no row to roll over: nothing mounts.
    root.innerHTML = "";
    expect(mountDomEffect(root, FOG, { from: AT }), "a fog without its far end").toBe(null);
    expect(root.childElementCount).toBe(0);
  });

  it("issue #124 a zone wave covers its pile, carrying its direction, text and tint", () => {
    const el = expectMounted(mountDomEffect(root, ZONE, { at: AT }), ZONE);
    expect(el.getAttribute("data-direction")).toBe("down");
    expect(el.getAttribute("data-text")).toBe("×12");
    expect(cssVar(el, "--fx-tint-rim")).toBe(TINT.rim);
    expectCovering(el, AT);
    root.innerHTML = "";
    expect(mountDomEffect(root, ZONE, {}), "a zone wave without its pile").toBe(null);
  });

  it("B39 remove() takes out its own element and leaves the others", () => {
    const first = mountDomEffect(root, SHEEN, { at: AT });
    const second = mountDomEffect(root, SPLAT_DAMAGE, { at: AT });
    expect(root.childElementCount).toBe(2);

    first?.remove();
    expect(root.childElementCount).toBe(1);
    expect(root.firstElementChild).toBe(second?.el);
  });

  it("R2800000 a set piece mounts its shape's layers, its side, its spin and its swallow, centred on its box", () => {
    const vortex: FxDomCue = { kind: "setPiece", shape: "vortex", tint: TINT, spin: -1, side: "opponent", at: anchor, holdMs: 2000, delayMs: 0, durationMs: 3200 };
    const el = expectMounted(mountDomEffect(root, vortex, { at: AT }), vortex);
    expect(el.getAttribute("data-shape")).toBe("vortex");
    expect(el.getAttribute("data-side")).toBe("opponent");
    expect(cssVar(el, "--fx-spin")).toBe("-1");
    expect(cssVar(el, "--fx-hold")).toBe("2000ms");
    expect(cssVar(el, "--fx-tint-rim")).toBe(TINT.rim);
    expect([...el.children].map((child) => child.className)).toEqual(["fx-setPiece-dark", "fx-setPiece-spiral", "fx-setPiece-core"]);
    // It names no card (R202): no text at all.
    expect(textNodeCount(el)).toBe(0);
    el.remove();

    const fold: FxDomCue = { ...vortex, shape: "fold", spin: 1, side: null };
    const folded = expectMounted(mountDomEffect(root, fold, { at: AT }), fold);
    expect(folded.getAttribute("data-side")).toBe("both");
    expect([...folded.children].map((child) => child.className)).toEqual(["fx-setPiece-edges", "fx-setPiece-light"]);
    expect(mountDomEffect(root, fold, {})).toBeNull();
  });

  it("R2801000 launch mounts its counts, flame and trail; still when asked", () => {
    const launch: FxDomCue = { kind: "launch", at: anchor, tint: TINT, size: "large", countsMs: [0, 600, 1200], igniteMs: 1800, delayMs: 0, durationMs: 3400 };
    const el = expectMounted(mountDomEffect(root, launch, { at: AT }), launch);
    expect(el.getAttribute("data-size")).toBe("large");
    expect(el.hasAttribute("data-still")).toBe(false);
    expect(cssVar(el, "--fx-ignite")).toBe("1800ms");
    expect(cssVar(el, "--fx-tint-glow")).toBe(TINT.glow);
    expect([...el.children].map((child) => child.className)).toEqual(["fx-launch-count", "fx-launch-count", "fx-launch-count", "fx-launch-flame", "fx-launch-trail"]);
    // The countdown, 3-2-1, each count at its time; the digits are CSS's (`attr(data-text)`), never a text node.
    const counts = [...el.querySelectorAll<HTMLElement>(".fx-launch-count")];
    expect(counts.map((count) => count.getAttribute("data-text"))).toEqual(["3", "2", "1"]);
    expect(counts.map((count) => cssVar(count, "--fx-count-at"))).toEqual(["0ms", "600ms", "1200ms"]);
    // It names no card (R202).
    expect(textNodeCount(el)).toBe(0);
    el.remove();

    const still: FxDomCue = { ...launch, countsMs: [], igniteMs: 0, still: true };
    const frame = expectMounted(mountDomEffect(root, still, { at: AT }), still);
    expect(frame.getAttribute("data-still")).toBe("true");
    expect([...frame.children].map((child) => child.className)).toEqual(["fx-launch-flame", "fx-launch-trail"]);
    expect(mountDomEffect(root, still, {})).toBeNull();
  });
});

/* What it refuses, and what it never does */

describe("B39 — a cue without the box it needs mounts nothing", () => {
  for (const cue of [SPLAT_DAMAGE, RAYS, SHEEN, ARROWS_UP]) {
    it(`B39 a ${cue.kind} with no \`at\` box mounts nothing`, () => {
      expect(mountDomEffect(root, cue, {})).toBeNull();
      expect(root.childElementCount).toBe(0);
    });

    it(`B39 a ${cue.kind} whose \`at\` box is null mounts nothing, even when from and to are given`, () => {
      expect(mountDomEffect(root, cue, { at: null, from: AT, to: TO })).toBeNull();
      expect(root.childElementCount).toBe(0);
    });
  }

  it("B39 a ghost with no source box mounts nothing", () => {
    expect(mountDomEffect(root, GHOST, { to: TO })).toBeNull();
    expect(mountDomEffect(root, GHOST, { from: null, to: TO })).toBeNull();
    expect(root.childElementCount).toBe(0);
  });

  it("B39 a ghost with no destination box mounts nothing", () => {
    expect(mountDomEffect(root, GHOST, { from: AT })).toBeNull();
    expect(mountDomEffect(root, GHOST, { from: AT, to: null })).toBeNull();
    expect(root.childElementCount).toBe(0);
  });

  it("B39 a ghost given only an `at` box mounts nothing", () => {
    expect(mountDomEffect(root, GHOST, { at: AT })).toBeNull();
    expect(root.childElementCount).toBe(0);
  });

  it("B39 text that looks like markup stays an attribute and never becomes a node", () => {
    const cue: FxDomCue = { kind: "banner", text: "<b>Your turn</b>", tone: "you", delayMs: 0, durationMs: 1500 };
    const el = expectMounted(mountDomEffect(root, cue, {}), cue);
    expect(el.getAttribute("data-text")).toBe("<b>Your turn</b>");
    expect(el.children).toHaveLength(0);
    expect(root.querySelector("b")).toBeNull();
  });

  it("B39 mounting sets no timer, and a mounted effect stays until something removes it", () => {
    vi.useFakeTimers();
    const mounted = [
      mountDomEffect(root, SPLAT_DAMAGE, { at: AT }),
      mountDomEffect(root, RAYS, { at: AT }),
      mountDomEffect(root, SHEEN, { at: AT }),
      mountDomEffect(root, GHOST, { from: AT, to: TO }),
      mountDomEffect(root, ARROWS_UP, { at: AT }),
      mountDomEffect(root, BANNER, {}),
      mountDomEffect(root, RESULT, {}),
    ];
    expect(mounted.every((effect) => effect !== null)).toBe(true);
    expect(vi.getTimerCount()).toBe(0);

    vi.advanceTimersByTime(60_000);
    expect(root.childElementCount).toBe(7);
  });

  it("B39 no mounted kind contains a text node, so no text search can match an effect", () => {
    mountDomEffect(root, SPLAT_DAMAGE, { at: AT });
    mountDomEffect(root, SPLAT_HEAL, { at: AT });
    mountDomEffect(root, RAYS, { at: AT });
    mountDomEffect(root, SHEEN, { at: AT });
    mountDomEffect(root, GHOST, { from: AT, to: TO });
    mountDomEffect(root, ARROWS_DOWN, { at: AT });
    mountDomEffect(root, BANNER, {});
    mountDomEffect(root, RESULT, {});
    expect(root.childElementCount).toBe(8);
    expect(textNodeCount(root)).toBe(0);
    expect(root.textContent).toBe("");
  });
});

describe("R502 R437 R436 — the v0.2.0 DOM kinds", () => {
  const FRACTURE: FxDomCue = { kind: "fracture", at: { kind: "crystal", side: "you", index: 2 }, delayMs: 90, durationMs: 910 };
  const BRAND: FxDomCue = {
    kind: "brand",
    at: anchor,
    tint: { rim: "#b46bff", core: "#f2e2ff", glow: "#7a2cff" },
    delayMs: 0,
    durationMs: 1100,
  };
  const CHAOS: FxDomCue = {
    kind: "chaos",
    title: "Call to Chaos:",
    lines: [
      { text: "Heal the caster's hero 30", reel: ["Summon a Chaos Golem", "Cast a random Call to Chaos", "Heal the caster's hero 30"], landMs: 360 },
      { text: "Summon a Chaos Golem", reel: ["<b>Summon a Chaos Golem</b>"], landMs: 540 },
    ],
    delayMs: 0,
    durationMs: 1800,
  };

  it("R502 a fracture covers its crystal's box", () => {
    const el = expectMounted(mountDomEffect(root, FRACTURE, { at: AT }), FRACTURE);
    expectCovering(el, AT);
  });

  it("R437 a brand is centered on its card and paints in the mark's colors", () => {
    const el = expectMounted(mountDomEffect(root, BRAND, { at: AT }), BRAND);
    expectCenteredOn(el, AT);
    expect(cssVar(el, "--fx-mark-rim")).toBe("#b46bff");
    expect(cssVar(el, "--fx-mark-core")).toBe("#f2e2ff");
    expect(cssVar(el, "--fx-mark-glow")).toBe("#7a2cff");
  });

  it("R436 a chaos reveal needs no box: a title, then one window per line over a reel of names that lands on the rolled one, all in attributes", () => {
    const el = expectMounted(mountDomEffect(root, CHAOS, {}), CHAOS);
    expect(el.getAttribute("data-text")).toBe("Call to Chaos:");
    expect(cssVar(el, "--fx-lines")).toBe("2");
    const lines = Array.from(el.querySelectorAll<HTMLElement>(".fx-chaos-line"));
    expect(lines).toHaveLength(2);
    expect(lines.map((line) => line.getAttribute("data-landed-text"))).toEqual(["Heal the caster's hero 30", "Summon a Chaos Golem"]);
    expect(lines.map((line) => cssVar(line, "--fx-land-ms"))).toEqual(["360ms", "540ms"]);
    expect(lines.map((line) => cssVar(line, "--fx-steps"))).toEqual(["2", "0"]);
    const cells = Array.from(lines[0]?.querySelectorAll(".fx-chaos-cell") ?? []);
    expect(cells.map((cell) => cell.getAttribute("data-text"))).toEqual(CHAOS.kind === "chaos" ? CHAOS.lines[0]?.reel : []);
    expect(cells.map((cell) => cell.getAttribute("data-landed"))).toEqual([null, null, "true"]);
    // Words that look like markup stay words in an attribute.
    expect(el.querySelector("b")).toBeNull();
  });

  it("R502 R437 a fracture or a brand with no box mounts nothing", () => {
    expect(mountDomEffect(root, FRACTURE, {})).toBeNull();
    expect(mountDomEffect(root, BRAND, { at: null })).toBeNull();
    expect(root.childElementCount).toBe(0);
  });
});

describe("Patch v0.2.3c — Classic+ #24 Crushing Walls' walls", () => {
  const WALLS: FxDomCue = { kind: "walls", at: { kind: "testid", testid: "board" }, reach: 0.2, delayMs: 0, durationMs: 1250 };

  it("the walls cover the board's box and reach `reach` of its width in from each side", () => {
    const el = expectMounted(mountDomEffect(root, WALLS, { at: AT }), WALLS);
    expectCovering(el, AT);
    expect(cssVar(el, "--fx-reach")).toBe(`${String(AT.width * 0.2)}px`);
    expect(textNodeCount(el)).toBe(0);
  });

  it("with no board to cover they mount nothing", () => {
    expect(mountDomEffect(root, WALLS, { at: null })).toBeNull();
    expect(root.childElementCount).toBe(0);
  });
});

describe("R2789000 — a keyword's callout", () => {
  const callout = (keyword: Extract<FxDomCue, { kind: "callout" }>["keyword"], still = false): FxDomCue => ({
    kind: "callout",
    keyword,
    at: anchor,
    delayMs: 0,
    durationMs: 600,
    ...(still ? { still: true as const } : {}),
  });

  it("R2789000 a callout carries its keyword and icon, centered on its unit's box, still when asked", () => {
    const cue = callout("Cleave");
    const el = expectMounted(mountDomEffect(root, cue, { at: AT }), cue);
    expectCenteredOn(el, AT);
    expect(el.getAttribute("data-callout")).toBe("Cleave");
    // Not `data-keyword`, which the card's rules text matches.
    expect(el.hasAttribute("data-keyword")).toBe(false);
    expect(el.hasAttribute("data-still")).toBe(false);
    const icon = el.querySelector("svg.fx-callout-icon");
    expect(icon?.getAttribute("viewBox")).toBe("0 0 100 100");
    expect(icon?.getAttribute("aria-hidden")).toBe("true");
    expect(icon?.querySelectorAll("path")).toHaveLength(1);
    expect(icon?.querySelector("path")?.getAttribute("d")).toMatch(/^M/);
    expect(icon?.querySelector("path")?.hasAttribute("data-line")).toBe(false);

    root.innerHTML = "";
    const stillCue = callout("Ranged", true);
    const still = expectMounted(mountDomEffect(root, stillCue, { at: AT }), stillCue);
    expect(still.getAttribute("data-still")).toBe("true");
    // A line icon is stroked, not filled.
    expect(still.querySelector("path")?.getAttribute("data-line")).toBe("true");
  });

  it("R2789000 a callout with no box, or no icon for its keyword, mounts nothing", () => {
    expect(mountDomEffect(root, callout("Cleave"), {})).toBeNull();
    expect(mountDomEffect(root, callout("Cleave"), { at: null })).toBeNull();
    // Rush had no icon when this probed it; R2791000 gave it one, and Issue #2790 gave
    // Taunt one, so the iconless probe moves to Cloaked, which neither lane iconified.
    expect(mountDomEffect(root, callout("Cloaked"), { at: AT })).toBeNull();
    expect(root.childElementCount).toBe(0);
  });
});

describe("a stand-in swells inward from an edge (integration QA: lane 1 on a phone)", () => {
  // Imported here so the header's contract above stays about mountDomEffect.
  const view = { width: 390, height: 844 };

  it("keeps the resting origin where the swell has room", async () => {
    const { holdOrigin } = await import("./dom.ts");
    expect(holdOrigin({ x: 150, y: 400, width: 66, height: 90 }, 1.3, view)).toEqual({ x: 0.5, y: 0.6 });
  });

  it("scales from the left edge of a box in lane 1, so nothing crosses the screen's left edge", async () => {
    const { holdOrigin } = await import("./dom.ts");
    const land = { x: 8, y: 400, width: 66, height: 90 };
    const scale = 1.6;
    const origin = holdOrigin(land, scale, view);
    expect(origin.x).toBeLessThan(0.5);
    const left = land.x + origin.x * land.width * (1 - scale);
    expect(left).toBeGreaterThanOrEqual(0);
  });

  it("scales from the right for a box at the right edge", async () => {
    const { holdOrigin } = await import("./dom.ts");
    const land = { x: 316, y: 400, width: 66, height: 90 };
    const scale = 1.6;
    const origin = holdOrigin(land, scale, view);
    const right = land.x + origin.x * land.width + (1 - origin.x) * land.width * scale;
    expect(origin.x).toBeGreaterThan(0.5);
    expect(right).toBeLessThanOrEqual(view.width);
  });
});

/* #3347: a hold's stand-in leans the way it travels, and straightens as it lands. */
describe("#3347 mountHold turns the stand-in toward its zone", () => {
  const LAND: FxBox = { x: 500, y: 300, width: 60, height: 82 }; // center (530, 341)
  const cue: FxHoldCue = {
    kind: "hold",
    from: null,
    to: { kind: "testid", testid: "zone-you-units-3" },
    delayMs: 0,
    landMs: 240,
    durationMs: 6000,
  };
  const rot = (from: FxBox | null): number => {
    const hold = mountHold(root, cue, { source: null, from, land: LAND, font: null });
    return Number.parseFloat(hold.el.style.getPropertyValue("--fx-rot"));
  };
  const startedAt = (x: number): FxBox => ({ x: x - 30, y: 600, width: 60, height: 82 });

  it("#3347 the sign follows the start side: from the left it starts turned clockwise, from the right counterclockwise, from straight below not at all", () => {
    expect(rot(startedAt(530 - FX_HOLD_TURN_PX / 2))).toBeGreaterThan(0);
    expect(rot(startedAt(530 + FX_HOLD_TURN_PX / 2))).toBeLessThan(0);
    expect(rot(startedAt(530))).toBe(0);
  });

  it("#3347 the turn is proportional up to FX_HOLD_TURN_PX and capped at FX_HOLD_TURN_DEG beyond it", () => {
    expect(rot(startedAt(530 - FX_HOLD_TURN_PX / 2))).toBeCloseTo(FX_HOLD_TURN_DEG / 2, 1);
    expect(rot(startedAt(530 - FX_HOLD_TURN_PX))).toBeCloseTo(FX_HOLD_TURN_DEG, 1);
    expect(rot(startedAt(530 - 10 * FX_HOLD_TURN_PX))).toBeCloseTo(FX_HOLD_TURN_DEG, 1);
    expect(rot(startedAt(530 + 10 * FX_HOLD_TURN_PX))).toBeCloseTo(-FX_HOLD_TURN_DEG, 1);
  });

  it("#3347 a stand-in with nothing to start from (a light that drops in) is not turned", () => {
    expect(rot(null)).toBe(0);
  });
});
