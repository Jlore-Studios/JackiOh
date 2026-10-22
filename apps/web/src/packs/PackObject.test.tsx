// The pack object (#1663): its name and emblem for a set, `global` and `cosmetic`, its state from idle
// through a tear to open, the pointer's sheen and tilt, reduced motion holding it all still, and the
// two stylesheets it and the scene draw from animating only transform and opacity. The sheets are read
// as text, as `fx/css.test.ts` reads its own, because Vitest stubs CSS imports.

import { existsSync, readFileSync } from "node:fs";
import { resolve } from "node:path";

import { cleanup, fireEvent, render, screen } from "@testing-library/react";
import { GLOBAL_PACK_NAME } from "@jackioh/server-config";
import { afterEach, beforeEach, describe, expect, it } from "vitest";

import { setMarkOf } from "../cards/setMark.ts";
import { resetFxSettingsForTests, setFxSettings } from "../fx/settings.ts";
import { __resetSettingsForTests, writeSettings } from "../settings/store.ts";
import { shopPacks } from "../shop/shop.ts";
import { COSMETIC_PACK, PACK_TEAR_MS, PACK_TILT_DEG } from "./packs.ts";
import PackObject, { type PackObjectState } from "./PackObject.tsx";

const ID = "object";

beforeEach(() => {
  __resetSettingsForTests();
  resetFxSettingsForTests();
});

afterEach(() => {
  cleanup();
  __resetSettingsForTests();
  resetFxSettingsForTests();
  try {
    window.localStorage.clear();
  } catch {
    // Storage is optional.
  }
});

const object = (): HTMLElement => screen.getByTestId(ID);

describe("#1663 the pack object", () => {
  it("shows its pack's name and emblem", () => {
    const { rerender } = render(<PackObject pack="Core" size="shelf" testid={ID} />);
    expect(screen.getByTestId(`${ID}-name`)).toHaveTextContent("Core pack");
    expect(screen.getByTestId(`${ID}-emblem`)).toHaveAttribute("src", setMarkOf("Core").src);
    expect(object()).toHaveAttribute("data-pack", "Core");
    expect(object()).toHaveAttribute("data-size", "shelf");

    const emblems = new Set<string>();
    for (const [pack, name] of [
      ["Classic", "Classic pack"],
      ["Classic+", "Classic+ pack"],
      ["Meditative", "Meditative pack"],
      [GLOBAL_PACK_NAME, "Global pack"],
      [COSMETIC_PACK, "Cosmetic pack"],
    ] as const) {
      rerender(<PackObject pack={pack} size="scene" testid={ID} />);
      expect(screen.getByTestId(`${ID}-name`), pack).toHaveTextContent(name);
      expect(object(), pack).toHaveAttribute("data-pack", pack);
      expect(object(), pack).toHaveAttribute("data-size", "scene");
      const src = screen.getByTestId(`${ID}-emblem`).getAttribute("src") ?? "";
      expect(src, `${pack}'s emblem`).toMatch(/^data:image\/svg\+xml,/);
      emblems.add(src);
    }
    expect(emblems.size, "each pack wears its own emblem").toBe(5);
    expect(emblems.has(setMarkOf("Core").src)).toBe(false);
  });

  it("draws what it is given on its face, and nothing is named without a testid", () => {
    const { container } = render(
      <PackObject pack="Core" size="scene">
        <span>Swipe or tap to open</span>
      </PackObject>,
    );
    expect(container.querySelector(".pack-object")).toHaveTextContent("Swipe or tap to open");
    expect(container.querySelector("[data-testid]")).toBeNull();
  });

  it("moves idle, then tearing, then open on a tear, and presses on the way", () => {
    const { rerender } = render(<PackObject pack="Core" size="scene" testid={ID} />);
    expect(object(), "idle by default").toHaveAttribute("data-state", "idle");

    const states: PackObjectState[] = ["pressed", "tearing", "open"];
    for (const state of states) {
      rerender(<PackObject pack="Core" size="scene" state={state} testid={ID} />);
      expect(object()).toHaveAttribute("data-state", state);
    }
    expect(object().querySelectorAll(".pack-object__seal"), "a crimped seal at each end").toHaveLength(2);
    expect(object().style.getPropertyValue("--pack-tear-ms")).toBe(String(PACK_TEAR_MS));
    expect(object().style.getPropertyValue("--pack-tilt-deg")).toBe(String(PACK_TILT_DEG));
  });

  it("the pointer moves the sheen and the tilt through two custom properties, and lets go on leaving", () => {
    render(<PackObject pack="Core" size="scene" testid={ID} />);
    expect(object()).toHaveAttribute("data-motion", "full");
    object().getBoundingClientRect = () => ({ left: 100, top: 200, width: 200, height: 280 }) as DOMRect;

    fireEvent.pointerMove(object(), { clientX: 250, clientY: 270 });
    expect(object().style.getPropertyValue("--pack-px")).toBe("0.75");
    expect(object().style.getPropertyValue("--pack-py")).toBe("0.25");

    fireEvent.pointerMove(object(), { clientX: 900, clientY: -50 });
    expect(object().style.getPropertyValue("--pack-px"), "held within the face").toBe("1");
    expect(object().style.getPropertyValue("--pack-py")).toBe("0");

    fireEvent.pointerLeave(object());
    expect(object().style.getPropertyValue("--pack-px")).toBe("");
    expect(object().style.getPropertyValue("--pack-py")).toBe("");
  });

  it("under the panel's reduced motion it carries data-motion=reduce and the pointer moves nothing", () => {
    writeSettings({ reduceMotion: true });
    render(<PackObject pack="Core" size="scene" state="tearing" testid={ID} />);
    expect(object()).toHaveAttribute("data-motion", "reduce");
    object().getBoundingClientRect = () => ({ left: 0, top: 0, width: 100, height: 100 }) as DOMRect;
    fireEvent.pointerMove(object(), { clientX: 80, clientY: 80 });
    expect(object().style.getPropertyValue("--pack-px")).toBe("");
    expect(object().style.getPropertyValue("--pack-py")).toBe("");
    expect(object(), "it still says what state it is in").toHaveAttribute("data-state", "tearing");
  });

  it("under the effects setting's reduced motion it carries data-motion=reduce too", () => {
    setFxSettings({ motion: "reduce" });
    render(<PackObject pack="Core" size="shelf" testid={ID} />);
    expect(object()).toHaveAttribute("data-motion", "reduce");
  });
});

/* ---------------------------------------------------------------------------- the stylesheets --- */

function readSheet(name: string): string {
  for (const candidate of [`src/packs/${name}`, `apps/web/src/packs/${name}`]) {
    const path = resolve(process.cwd(), candidate);
    if (existsSync(path)) return readFileSync(path, "utf8").replace(/\/\*[\s\S]*?\*\//g, "");
  }
  throw new Error(`${name} not found from ${process.cwd()}`);
}

function closingBrace(src: string, open: number): number {
  let depth = 0;
  for (let i = open; i < src.length; i += 1) {
    if (src[i] === "{") depth += 1;
    else if (src[i] === "}") {
      depth -= 1;
      if (depth === 0) return i;
    }
  }
  return src.length;
}

describe("#1663 every pack's colours", () => {
  it("each pack the store shelves, and the cosmetic pack, has a colour block of its own in pack-object.css", () => {
    const sheet = readSheet("pack-object.css");
    for (const pack of [...shopPacks(), COSMETIC_PACK]) {
      const at = sheet.indexOf(`.pack-object[data-pack="${pack}"] {`);
      expect(at, `${pack} has its colours`).toBeGreaterThanOrEqual(0);
      expect(sheet.slice(at, closingBrace(sheet, sheet.indexOf("{", at))), pack).toContain("--pack-foil-a:");
    }
  });
});

/** Every `@keyframes name { … }` of a comment-free sheet and the properties its stops set. */
function keyframeProperties(sheet: string): Map<string, string[]> {
  const out = new Map<string, string[]>();
  const head = /@(?:-webkit-)?keyframes\s+([\w-]+)\s*\{/g;
  for (let match = head.exec(sheet); match !== null; match = head.exec(sheet)) {
    const open = match.index + match[0].length - 1;
    const body = sheet.slice(open + 1, closingBrace(sheet, open));
    const names: string[] = [];
    for (const stop of body.matchAll(/\{([^{}]*)\}/g)) {
      for (const decl of stop[1]!.split(";")) {
        const colon = decl.indexOf(":");
        if (colon > 0) names.push(decl.slice(0, colon).trim().toLowerCase());
      }
    }
    out.set(match[1]!, names);
  }
  return out;
}

/** `animation-timing-function` is a stop's easing, not a property it animates. */
const ANIMATED = new Set(["transform", "opacity", "animation-timing-function"]);

describe("#1663 the pack's stylesheets", () => {
  for (const name of ["pack-object.css", "packs.css"]) {
    it(`${name}: every @keyframes animates only transform and opacity`, () => {
      const keyframes = keyframeProperties(readSheet(name));
      expect(keyframes.size, `${name} has keyframes`).toBeGreaterThan(0);
      const offenders: string[] = [];
      for (const [keyframe, names] of keyframes) {
        for (const property of names) if (!ANIMATED.has(property)) offenders.push(`${keyframe}: ${property}`);
      }
      expect(offenders).toEqual([]);
    });
  }

  it("the pack object's tear, swell and flash are among them, and the check would catch a shadow", () => {
    expect([...keyframeProperties(readSheet("pack-object.css")).keys()].sort()).toEqual([
      "pack-object-flash",
      "pack-object-swell",
      "pack-object-tear",
    ]);
    const bad = keyframeProperties("@keyframes x { from { transform: none; box-shadow: none } to { opacity: 0 } }");
    expect(bad.get("x")).toEqual(["transform", "box-shadow", "opacity"]);
  });
});
