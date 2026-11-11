// R3344001 (#3344): the tilt a face with depth takes. The pose is the pointer's place in the face's host box, held at its
// edge, so the face never turns past CARD_TILT_MAX_DEG; the glare lies opposite it; the face springs back (its properties
// are removed) when the pointer leaves; the phone's orientation drives it only once the setting is on, and a touch drag
// does in inspect; one face tilts at a time and none while the animation queue drains; reduced motion draws no depth to
// tilt. The planes and the glare are depth.test.tsx's, the pixels are e2e's cypress/component/card-depth.cy.tsx.

import { act, cleanup, fireEvent, render, screen } from "@testing-library/react";
import { createElement, type ReactElement } from "react";
import { afterEach, beforeEach, describe, expect, it, vi } from "vitest";

import { CATALOG } from "@jackioh/cards";

import { DeviceTiltSwitch } from "../settings/controls.tsx";
import { __resetSettingsForTests, writeSettings } from "../settings/store.ts";
import { resetFxSettingsForTests } from "../fx/settings.ts";
import { CardDefsProvider } from "./refContext.tsx";
import { CARD_GLARE_TRAVEL_PCT, CARD_TILT_MAX_DEG, DIORAMA_TURN_MAX_DEG } from "./constants.ts";
import { CardFace } from "./CardFace.tsx";
import { resetCosmeticsForTests, writeCardLevel } from "./cosmetics.ts";
import { HoverPreview } from "./inspect/HoverPreview.tsx";
import { INSPECT_FACE } from "./inspect/testids.ts";
import { faceModel } from "./model.ts";
import { CARD_SETTINGS_DEFAULTS, readCardSettings, writeCardSettings } from "./settings.ts";
import { devicePose, glareOf, pointerPose, resetTiltForTests, tiltDegrees, useTiltHold, type TiltBox, type TiltMode } from "./tilt.ts";
import { VERSION_ALT_ART_ID } from "./versions.ts";

const LEGENDARY = "core-052";
const MYTHIC = "core-100";

/** A host box 200 by 280 at the page's origin, which jsdom's layout does not give. */
const BOX = { left: 0, top: 0, width: 200, height: 280 };

/** An anchor box clear of the preview's `BOX`: the card the preview is anchored to, where the pointer is. */
const ANCHOR: TiltBox = { left: 400, top: 500, width: 100, height: 140 };

/** The tilt properties a face holds, each read from its inline style. */
const PROPERTIES = ["--cf-tilt-x", "--cf-tilt-y", "--cf-tilt-rx", "--cf-tilt-ry", "--cf-glare-x", "--cf-glare-y"] as const;

function read(cf: HTMLElement): Record<(typeof PROPERTIES)[number], string> {
  return Object.fromEntries(PROPERTIES.map((property) => [property, cf.style.getPropertyValue(property)])) as Record<(typeof PROPERTIES)[number], string>;
}

function expectLevel(cf: HTMLElement, where = ""): void {
  for (const property of PROPERTIES) expect(cf.style.getPropertyValue(property), `${where} ${property}`).toBe("");
  expect(cf.hasAttribute("data-tilt"), `${where} data-tilt`).toBe(false);
}

function modelOf(id: string): ReturnType<typeof faceModel> {
  const def = CATALOG[id];
  if (def === undefined) throw new Error(`no ${id}`);
  return faceModel({ defId: id, def, radiant: false });
}

/** A face in its own host element, whose box is `BOX`: what the tilt reads without an anchor. */
function Face({ id = MYTHIC, tilt, anchor }: { id?: string; tilt: TiltMode; anchor?: TiltBox }): ReactElement {
  return (
    <div className="host" ref={(node) => { if (node !== null) node.getBoundingClientRect = () => ({ ...BOX, right: BOX.left + BOX.width, bottom: BOX.top + BOX.height, x: BOX.left, y: BOX.top, toJSON: () => ({}) }); }}>
      <CardFace face={modelOf(id)} layout="full" worn tilt={tilt} tiltAnchor={anchor} />
    </div>
  );
}

function mount(tilt: TiltMode, id = MYTHIC, anchor?: TiltBox): { cf: HTMLElement; host: HTMLElement; unmount: () => void } {
  const { container, unmount } = render(
    <CardDefsProvider defs={CATALOG}>
      <Face id={id} tilt={tilt} anchor={anchor} />
    </CardDefsProvider>,
  );
  const cf = container.querySelector<HTMLElement>(".cf");
  const host = container.querySelector<HTMLElement>(".host");
  if (cf === null || host === null) throw new Error("no face");
  return { cf, host, unmount };
}

/** A pointer event of `type`; a `MouseEvent` has no `pointerType`, so one is set only where a test asks for a finger's. */
function pointer(target: EventTarget, type: string, x: number, y: number, pointerType?: string): MouseEvent {
  const event = new MouseEvent(type, { bubbles: type !== "pointerenter" && type !== "pointerleave", cancelable: true, clientX: x, clientY: y });
  if (pointerType !== undefined) Object.defineProperty(event, "pointerType", { value: pointerType });
  act(() => {
    target.dispatchEvent(event);
  });
  return event;
}

function orientation(gamma: number | null, beta: number | null): void {
  const event = new Event("deviceorientation");
  Object.assign(event, { gamma, beta });
  act(() => {
    window.dispatchEvent(event);
  });
}

beforeEach(() => {
  localStorage.clear();
  resetCosmeticsForTests();
  resetTiltForTests();
  writeCardSettings({ ...CARD_SETTINGS_DEFAULTS });
  // R3343001: a Legendary's Default draws Extended art, which draws no planes and so never tilts (R3344000); the
  // Legendary under test wears Alt art, the painter's own picture in the normal frame.
  writeCardLevel(LEGENDARY, VERSION_ALT_ART_ID);
  // R3350000: a Mythic's normal picture draws its diorama, which turns to DIORAMA_TURN_MAX_DEG in the preview and inspect
  // (diorama.test.tsx); the Mythic under test wears Alt art, which draws its planes, so it tilts as every other face does.
  writeCardLevel(MYTHIC, VERSION_ALT_ART_ID);
});

afterEach(() => {
  cleanup();
  vi.unstubAllGlobals();
  resetTiltForTests();
  resetFxSettingsForTests();
  resetCosmeticsForTests();
  localStorage.clear();
  writeCardSettings({ ...CARD_SETTINGS_DEFAULTS });
  __resetSettingsForTests();
});

describe("R3344001 the pose, the angles and the glare", () => {
  it("R3344001 a pointer at a box's middle is level, at its edges one each way, and held there past them", () => {
    expect(pointerPose(100, 140, BOX)).toEqual({ x: 0, y: 0 });
    expect(pointerPose(0, 0, BOX)).toEqual({ x: -1, y: -1 });
    expect(pointerPose(200, 280, BOX)).toEqual({ x: 1, y: 1 });
    expect(pointerPose(150, 70, BOX)).toEqual({ x: 0.5, y: -0.5 });
    // Past the box, held at its edge, however far.
    expect(pointerPose(-5000, 99999, BOX)).toEqual({ x: -1, y: 1 });
    expect(pointerPose(1e9, -1e9, BOX)).toEqual({ x: 1, y: -1 });
    // A box offset from the origin is read from its own middle.
    expect(pointerPose(350, 450, { left: 300, top: 400, width: 100, height: 100 })).toEqual({ x: 0, y: 0 });
    // A side of no length is level along it, and no number is level.
    expect(pointerPose(50, 50, { left: 0, top: 0, width: 0, height: 0 })).toEqual({ x: 0, y: 0 });
    expect(pointerPose(Number.NaN, 140, BOX)).toEqual({ x: 0, y: 0 });
  });

  it("R3344001 the angles never pass the cap, each toward the point the pointer is at", () => {
    expect(tiltDegrees({ x: 0, y: 0 })).toEqual({ rx: -0, ry: 0 });
    expect(tiltDegrees({ x: 1, y: 0 }).ry).toBe(CARD_TILT_MAX_DEG);
    expect(tiltDegrees({ x: -1, y: 0 }).ry).toBe(-CARD_TILT_MAX_DEG);
    // A pointer above the middle turns the face's top edge away (rx positive), below its foot.
    expect(tiltDegrees({ x: 0, y: -1 }).rx).toBe(CARD_TILT_MAX_DEG);
    expect(tiltDegrees({ x: 0, y: 1 }).rx).toBe(-CARD_TILT_MAX_DEG);
    for (const x of [-1e9, -300, -1, -0.3, 0, 0.4, 1, 80, 1e9]) {
      for (const y of [-1e9, -50, -1, 0, 0.9, 1, 200, 1e9]) {
        const { rx, ry } = tiltDegrees(pointerPose(x, y, BOX));
        expect(Math.abs(rx), `${String(x)},${String(y)}`).toBeLessThanOrEqual(CARD_TILT_MAX_DEG);
        expect(Math.abs(ry), `${String(x)},${String(y)}`).toBeLessThanOrEqual(CARD_TILT_MAX_DEG);
      }
    }
    expect(CARD_TILT_MAX_DEG).toBe(12);
  });

  it("R3350000 tiltDegrees holds a pose to the cap it is given", () => {
    expect(tiltDegrees({ x: 1, y: 0 }, DIORAMA_TURN_MAX_DEG).ry).toBe(DIORAMA_TURN_MAX_DEG);
    expect(tiltDegrees({ x: -1, y: 1 }, DIORAMA_TURN_MAX_DEG)).toEqual({ rx: -DIORAMA_TURN_MAX_DEG, ry: -DIORAMA_TURN_MAX_DEG });
    expect(tiltDegrees({ x: 0.5, y: -0.5 }, DIORAMA_TURN_MAX_DEG)).toEqual({ rx: DIORAMA_TURN_MAX_DEG / 2, ry: DIORAMA_TURN_MAX_DEG / 2 });
    for (const x of [-1e9, -1, -0.3, 0, 0.4, 1, 1e9]) {
      for (const y of [-1e9, -1, 0, 0.9, 1, 1e9]) {
        const { rx, ry } = tiltDegrees(pointerPose(x, y, BOX), DIORAMA_TURN_MAX_DEG);
        expect(Math.abs(rx), `${String(x)},${String(y)}`).toBeLessThanOrEqual(DIORAMA_TURN_MAX_DEG);
        expect(Math.abs(ry), `${String(x)},${String(y)}`).toBeLessThanOrEqual(DIORAMA_TURN_MAX_DEG);
      }
    }
    // With no cap given, the face's own.
    expect(tiltDegrees({ x: 1, y: 0 })).toEqual(tiltDegrees({ x: 1, y: 0 }, CARD_TILT_MAX_DEG));
    expect(DIORAMA_TURN_MAX_DEG).toBe(25);
  });

  it("R3344001 the glare lies opposite the tilt", () => {
    expect(glareOf({ x: 0, y: 0 })).toEqual({ x: 50, y: 50 });
    for (const pose of [{ x: 1, y: 0 }, { x: -1, y: 0 }, { x: 0, y: 1 }, { x: 0, y: -1 }, { x: 0.5, y: -0.25 }]) {
      const glare = glareOf(pose);
      const { rx, ry } = tiltDegrees(pose);
      // Opposite along x: a turn to the right (ry > 0) puts the glare left of the middle; opposite along y: the pointer
      // below the middle (rx < 0) puts it above.
      const sign = (value: number): number => Math.sign(value) || 0;
      expect(sign(glare.x - 50), JSON.stringify(pose)).toBe(sign(-ry));
      expect(sign(glare.y - 50), JSON.stringify(pose)).toBe(sign(rx));
      expect(glare.x).toBeCloseTo(50 - pose.x * CARD_GLARE_TRAVEL_PCT, 9);
    }
  });

  it("R3344001 a device's orientation is a pose as Immersive reads it, none when it reports none", () => {
    expect(devicePose(0, 45)).toEqual({ x: 0, y: 0 });
    expect(devicePose(30, 45)).toEqual({ x: 1, y: 0 });
    expect(devicePose(-90, 200)).toEqual({ x: -1, y: 1 });
    expect(devicePose(null, 45)).toBeNull();
    expect(devicePose(10, null)).toBeNull();
  });
});

describe("R3344001 the window's pointer tilts a face in the hover preview", () => {
  it("R3344001 the preview reads the pointer's place on the card it is anchored to: level at its middle, the cap at its edge, in between a fraction", () => {
    const { cf } = mount("window", MYTHIC, ANCHOR);
    expectLevel(cf, "at rest");
    // The anchor's middle: level, where the old code read the cap against the preview's own box.
    pointer(window, "pointermove", 450, 570);
    expect(cf.getAttribute("data-tilt")).toBe("on");
    expect(read(cf)).toEqual({
      "--cf-tilt-x": "0",
      "--cf-tilt-y": "0",
      "--cf-tilt-rx": "0deg",
      "--cf-tilt-ry": "0deg",
      "--cf-glare-x": "50%",
      "--cf-glare-y": "50%",
    });
    // The anchor's right edge: the cap along x alone.
    pointer(window, "pointermove", 500, 570);
    expect(read(cf)).toMatchObject({ "--cf-tilt-x": "1", "--cf-tilt-ry": "12deg", "--cf-tilt-rx": "0deg", "--cf-glare-x": "-10%" });
    // The anchor's top-left corner: the cap both ways, the glare opposite.
    pointer(window, "pointermove", 400, 500);
    expect(read(cf)).toMatchObject({ "--cf-tilt-ry": "-12deg", "--cf-tilt-rx": "12deg", "--cf-glare-x": "110%", "--cf-glare-y": "110%" });
    // A pose in between.
    pointer(window, "pointermove", 475, 535);
    expect(read(cf)).toMatchObject({ "--cf-tilt-x": "0.5", "--cf-tilt-y": "-0.5", "--cf-tilt-ry": "6deg", "--cf-tilt-rx": "6deg" });
    // Far past the anchor: the cap, no more.
    pointer(window, "pointermove", 100000, 570);
    expect(cf.style.getPropertyValue("--cf-tilt-ry")).toBe("12deg");
  });

  it("R3344001 a new anchor is read at the next move without the face letting go", () => {
    const { container, rerender } = render(
      <CardDefsProvider defs={CATALOG}>
        <Face tilt="window" anchor={{ left: 400, top: 500, width: 100, height: 140 }} />
      </CardDefsProvider>,
    );
    const cf = container.querySelector<HTMLElement>(".cf");
    if (cf === null) throw new Error("no face");
    pointer(window, "pointermove", 475, 535);
    expect(cf.style.getPropertyValue("--cf-tilt-x")).toBe("0.5");
    // A new anchor object must not re-attach, which would level the face.
    act(() => {
      rerender(
        <CardDefsProvider defs={CATALOG}>
          <Face tilt="window" anchor={{ left: 450, top: 500, width: 100, height: 140 }} />
        </CardDefsProvider>,
      );
    });
    expect(cf.getAttribute("data-tilt")).toBe("on");
    expect(cf.style.getPropertyValue("--cf-tilt-x")).toBe("0.5");
    // The next move reads the new anchor: the same pointer is now left of its middle.
    pointer(window, "pointermove", 475, 535);
    expect(cf.style.getPropertyValue("--cf-tilt-x")).toBe("-0.5");
  });

  it("R3344001 a window face given no anchor reads its host's box, and a pointer far past it reads the cap", () => {
    const { cf } = mount("window");
    expectLevel(cf, "at rest");
    pointer(window, "pointermove", 200, 140);
    expect(cf.getAttribute("data-tilt")).toBe("on");
    expect(read(cf)).toEqual({
      "--cf-tilt-x": "1",
      "--cf-tilt-y": "0",
      "--cf-tilt-rx": "0deg",
      "--cf-tilt-ry": "12deg",
      "--cf-glare-x": "-10%",
      "--cf-glare-y": "50%",
    });
    // Far past the box: the cap, no more.
    pointer(window, "pointermove", 100000, 140);
    expect(cf.style.getPropertyValue("--cf-tilt-ry")).toBe("12deg");
    pointer(window, "pointermove", -100000, -100000);
    expect(read(cf)).toMatchObject({ "--cf-tilt-ry": "-12deg", "--cf-tilt-rx": "12deg", "--cf-glare-x": "110%", "--cf-glare-y": "110%" });
    // A pose in between.
    pointer(window, "pointermove", 150, 70);
    expect(read(cf)).toMatchObject({ "--cf-tilt-x": "0.5", "--cf-tilt-y": "-0.5", "--cf-tilt-ry": "6deg", "--cf-tilt-rx": "6deg" });
  });

  it("R3344001 R3350000 the hover preview gives its face the anchor it is placed by, a Mythic's diorama turning to DIORAMA_TURN_MAX_DEG", () => {
    render(
      <CardDefsProvider defs={CATALOG}>
        <HoverPreview face={modelOf(MYTHIC)} anchor={{ left: 600, top: 400, width: 100, height: 140 }} />
      </CardDefsProvider>,
    );
    const wrapper = screen.getByTestId(INSPECT_FACE);
    // The preview stands above the anchor, as placePreview puts it: the pose must read the anchor, not this box.
    wrapper.getBoundingClientRect = () => ({ left: 560, top: 100, width: 200, height: 280, right: 760, bottom: 380, x: 560, y: 100, toJSON: () => ({}) });
    const cf = wrapper.querySelector<HTMLElement>(".cf");
    if (cf === null) throw new Error("no face");
    expect(cf.hasAttribute("data-depth")).toBe(true);
    // The anchor's middle: level, though the pointer is below the preview's own box.
    pointer(window, "pointermove", 650, 470);
    expect(cf.getAttribute("data-tilt")).toBe("on");
    expect(read(cf)).toMatchObject({ "--cf-tilt-x": "0", "--cf-tilt-y": "0" });
    // The anchor's right edge: the cap along x alone. R3350000: the preview's Mythic draws its diorama (its look is no one's
    // pick here), which turns to DIORAMA_TURN_MAX_DEG.
    expect(cf.getAttribute("data-diorama")).toBe("true");
    pointer(window, "pointermove", 700, 470);
    expect(read(cf)).toMatchObject({ "--cf-tilt-ry": `${DIORAMA_TURN_MAX_DEG}deg`, "--cf-tilt-rx": "0deg" });
  });

  it("R3344001 the face springs back: leaving the document, losing focus and unmounting each remove every property", () => {
    const first = mount("window");
    pointer(window, "pointermove", 180, 20);
    expect(first.cf.getAttribute("data-tilt")).toBe("on");
    pointer(document.documentElement, "pointerleave", 0, 0);
    expectLevel(first.cf, "left the document");
    pointer(window, "pointermove", 180, 20);
    expect(first.cf.getAttribute("data-tilt")).toBe("on");
    act(() => {
      window.dispatchEvent(new Event("blur"));
    });
    expectLevel(first.cf, "lost focus");
    pointer(window, "pointermove", 180, 20);
    expect(first.cf.getAttribute("data-tilt")).toBe("on");
    first.unmount();
    expectLevel(first.cf, "unmounted");
    // Nothing listens after it: a move writes no property.
    pointer(window, "pointermove", 10, 10);
    expectLevel(first.cf, "after unmount");
  });

  it("R3344001 a finger's move over the window never tilts the preview", () => {
    const { cf } = mount("window");
    pointer(window, "pointermove", 180, 20, "touch");
    expectLevel(cf);
    pointer(window, "pointermove", 180, 20, "mouse");
    expect(cf.getAttribute("data-tilt")).toBe("on");
  });
});

describe("R3344001 a card in hand, on the board and in the Collection tilts under its own pointer or touch", () => {
  it("R3344001 host mode tilts on enter and move over the host, and levels when the pointer leaves", () => {
    const { cf, host } = mount("host");
    pointer(host, "pointerenter", 150, 70);
    expect(read(cf)).toMatchObject({ "--cf-tilt-ry": "6deg", "--cf-tilt-rx": "6deg" });
    pointer(host, "pointermove", 200, 280);
    expect(read(cf)).toMatchObject({ "--cf-tilt-ry": "12deg", "--cf-tilt-rx": "-12deg", "--cf-glare-x": "-10%", "--cf-glare-y": "-10%" });
    // A window pointer elsewhere does nothing to a host face.
    pointer(window, "pointermove", 0, 0);
    expect(cf.style.getPropertyValue("--cf-tilt-ry")).toBe("12deg");
    pointer(host, "pointerleave", 0, 0);
    expectLevel(cf, "left");
  });

  it("R3344001 host mode does not tilt on a finger that only moves over it, and does while it presses", () => {
    const { cf, host } = mount("host");
    pointer(host, "pointerenter", 150, 70, "touch");
    pointer(host, "pointermove", 150, 70, "touch");
    expectLevel(cf, "a finger moving");
    pointer(host, "pointerdown", 150, 70, "touch");
    expect(read(cf)).toMatchObject({ "--cf-tilt-ry": "6deg", "--cf-tilt-rx": "6deg" });
    pointer(host, "pointermove", 200, 70, "touch");
    expect(cf.style.getPropertyValue("--cf-tilt-ry")).toBe("12deg");
    pointer(host, "pointerup", 200, 70, "touch");
    expectLevel(cf, "lifted");
    // A press by a mouse tilts, and the mouse's release leaves the tilt under the pointer that still hovers.
    pointer(host, "pointerdown", 100, 140, "mouse");
    expect(cf.getAttribute("data-tilt")).toBe("on");
    pointer(host, "pointerup", 100, 140, "mouse");
    expect(cf.getAttribute("data-tilt")).toBe("on");
    pointer(host, "pointercancel", 100, 140, "touch");
    expectLevel(cf, "cancelled");
  });

  it("R3344001 a face asked for no tilt, or whose card has no depth, listens to nothing", () => {
    const off = mount("off");
    pointer(off.host, "pointermove", 150, 70);
    pointer(window, "pointermove", 150, 70);
    expectLevel(off.cf, "off");
    cleanup();
    const common = mount("host", "core-002");
    expect(common.cf.hasAttribute("data-depth")).toBe(false);
    pointer(common.host, "pointermove", 150, 70);
    expectLevel(common.cf, "a Common");
  });
});

describe("R3344001 a drag tilts the face in inspect and holds off the sheet's swipe", () => {
  it("R3344001 drag mode tilts on a touch drag, holds off the touch pan, and the sheet's own pointerdown never fires", () => {
    const sheetDown = vi.fn();
    const { container, unmount } = render(
      <CardDefsProvider defs={CATALOG}>
        <div className="sheet" onPointerDown={sheetDown}>
          <Face tilt="drag" />
        </div>
      </CardDefsProvider>,
    );
    const cf = container.querySelector<HTMLElement>(".cf");
    const host = container.querySelector<HTMLElement>(".host");
    if (cf === null || host === null) throw new Error("no face");
    expect(host.style.touchAction).toBe("none");
    pointer(host, "pointerdown", 100, 140, "touch");
    expect(cf.getAttribute("data-tilt")).toBe("on");
    expect(sheetDown).not.toHaveBeenCalled();
    pointer(host, "pointermove", 200, 0, "touch");
    expect(read(cf)).toMatchObject({ "--cf-tilt-x": "1", "--cf-tilt-y": "-1", "--cf-tilt-ry": "12deg", "--cf-tilt-rx": "12deg" });
    pointer(host, "pointermove", 0, 280, "touch");
    expect(read(cf)).toMatchObject({ "--cf-tilt-ry": "-12deg", "--cf-tilt-rx": "-12deg" });
    pointer(host, "pointerup", 0, 280, "touch");
    expectLevel(cf, "released");
    // A move after the touch is up tilts nothing, and the mouse tilts by the window as the preview's does.
    pointer(host, "pointermove", 200, 0, "touch");
    expectLevel(cf, "a finger no longer pressing");
    pointer(window, "pointermove", 150, 140);
    expect(cf.getAttribute("data-tilt")).toBe("on");
    unmount();
    expectLevel(cf, "unmounted");
    expect(host.style.touchAction).toBe("");
  });

  it("R3344001 the host mode lets the sheet's pointerdown through, the drag mode alone holds it off", () => {
    for (const [tilt, reached] of [
      ["host", 1],
      ["window", 1],
      ["drag", 0],
    ] as const) {
      const sheetDown = vi.fn();
      const { container } = render(
        <CardDefsProvider defs={CATALOG}>
          <div onPointerDown={sheetDown}>
            <Face tilt={tilt} />
          </div>
        </CardDefsProvider>,
      );
      const host = container.querySelector<HTMLElement>(".host");
      if (host === null) throw new Error("no host");
      pointer(host, "pointerdown", 100, 140, "touch");
      expect(sheetDown, tilt).toHaveBeenCalledTimes(reached);
      // A mouse press is nothing the drag holds off: the sheet's swipe is a finger's.
      pointer(host, "pointerdown", 100, 140, "mouse");
      expect(sheetDown, `${tilt} mouse`).toHaveBeenCalledTimes(reached + 1);
      cleanup();
    }
  });
});

describe("R3344001 the phone's orientation tilts a face only once the setting is on", () => {
  it("R3344001 deviceorientation is ignored until Tilt with my phone is on, then tilts the preview and the sheet", () => {
    const { cf } = mount("window");
    orientation(15, 60);
    expectLevel(cf, "setting off");
    act(() => {
      writeCardSettings({ deviceTilt: true });
    });
    orientation(15, 60);
    // gamma 15 of 30 across, beta 15 past the 45 rest of 30 forward: half each way.
    expect(read(cf)).toMatchObject({ "--cf-tilt-x": "0.5", "--cf-tilt-y": "0.5", "--cf-tilt-ry": "6deg", "--cf-tilt-rx": "-6deg", "--cf-glare-x": "20%", "--cf-glare-y": "20%" });
    // A device with no reading changes nothing, and past the range is held at the cap.
    orientation(null, null);
    expect(cf.style.getPropertyValue("--cf-tilt-ry")).toBe("6deg");
    orientation(500, -500);
    expect(read(cf)).toMatchObject({ "--cf-tilt-ry": "12deg", "--cf-tilt-rx": "12deg" });
    // Turned off again, it stops listening and rests.
    act(() => {
      writeCardSettings({ deviceTilt: false });
    });
    expectLevel(cf, "setting turned off");
    orientation(15, 60);
    expectLevel(cf, "setting turned off, event");
  });

  it("R3344001 a card in hand follows the phone only while it is pressed or hovered", () => {
    writeCardSettings({ deviceTilt: true });
    const { cf, host } = mount("host");
    orientation(15, 60);
    expectLevel(cf, "not engaged");
    pointer(host, "pointerdown", 100, 140, "touch");
    orientation(30, 45);
    expect(read(cf)).toMatchObject({ "--cf-tilt-x": "1", "--cf-tilt-y": "0", "--cf-tilt-ry": "12deg" });
    pointer(host, "pointerup", 100, 140, "touch");
    expectLevel(cf, "lifted");
    orientation(30, 45);
    expectLevel(cf, "lifted, event");
  });

  it("R3344001 the switch asks the phone's leave inside its tap and turns on only when it is granted", async () => {
    const asks: string[] = [];
    vi.stubGlobal(
      "DeviceOrientationEvent",
      class {
        static requestPermission(): Promise<string> {
          asks.push("asked");
          return Promise.resolve(answer);
        }
      },
    );
    let answer = "denied";
    render(createElement(DeviceTiltSwitch));
    const switchEl = screen.getByTestId<HTMLInputElement>("setting-deviceTilt");
    expect(switchEl.checked).toBe(false);
    expect(readCardSettings().deviceTilt).toBe(false);
    // Denied: asked once, and nothing is written.
    fireEvent.click(switchEl);
    await act(async () => {
      await Promise.resolve();
    });
    expect(asks).toEqual(["asked"]);
    expect(readCardSettings().deviceTilt).toBe(false);
    expect(switchEl.checked).toBe(false);
    // Granted: on.
    answer = "granted";
    fireEvent.click(switchEl);
    await act(async () => {
      await Promise.resolve();
    });
    expect(asks).toEqual(["asked", "asked"]);
    expect(readCardSettings().deviceTilt).toBe(true);
    expect(switchEl.checked).toBe(true);
    // Off asks nothing.
    fireEvent.click(switchEl);
    expect(asks).toHaveLength(2);
    expect(readCardSettings().deviceTilt).toBe(false);
  });

  it("R3344001 a refused or failing ask writes nothing, and a platform that asks nothing turns it on at once", async () => {
    vi.stubGlobal("DeviceOrientationEvent", class {
      static requestPermission(): Promise<string> {
        return Promise.reject(new Error("blocked"));
      }
    });
    const { unmount } = render(createElement(DeviceTiltSwitch));
    fireEvent.click(screen.getByTestId("setting-deviceTilt"));
    await act(async () => {
      await Promise.resolve();
    });
    expect(readCardSettings().deviceTilt).toBe(false);
    unmount();
    vi.unstubAllGlobals();
    // jsdom has no DeviceOrientationEvent: nothing to ask.
    render(createElement(DeviceTiltSwitch));
    fireEvent.click(screen.getByTestId("setting-deviceTilt"));
    expect(readCardSettings().deviceTilt).toBe(true);
    expect(screen.getByTestId<HTMLInputElement>("setting-deviceTilt").checked).toBe(true);
  });
});

describe("R3344001 one face tilts at a time, and none while the animation queue drains", () => {
  function Hold({ held }: { held: boolean }): ReactElement | null {
    useTiltHold(held);
    return null;
  }

  it("R3344001 two faces: the one the pointer reaches last has data-tilt, the other has let go", () => {
    const { container } = render(
      <CardDefsProvider defs={CATALOG}>
        <Face id={LEGENDARY} tilt="host" />
        <Face id={MYTHIC} tilt="host" />
      </CardDefsProvider>,
    );
    const [firstHost, secondHost] = [...container.querySelectorAll<HTMLElement>(".host")];
    const [first, second] = [...container.querySelectorAll<HTMLElement>(".cf")];
    if (firstHost === undefined || secondHost === undefined || first === undefined || second === undefined) throw new Error("no faces");
    pointer(firstHost, "pointermove", 100, 140);
    expect(first.getAttribute("data-tilt")).toBe("on");
    expect(second.hasAttribute("data-tilt")).toBe(false);
    pointer(secondHost, "pointermove", 100, 140);
    expect(second.getAttribute("data-tilt")).toBe("on");
    expectLevel(first, "the first, let go");
    expect(container.querySelectorAll('.cf[data-tilt="on"]')).toHaveLength(1);
    pointer(firstHost, "pointermove", 20, 20);
    expect(first.getAttribute("data-tilt")).toBe("on");
    expectLevel(second, "the second, let go");
    expect(container.querySelectorAll('.cf[data-tilt="on"]')).toHaveLength(1);
  });

  it("R3344001 a drag outranks the window's pointer, which outranks a host's: the lower never takes the slot from the higher", () => {
    const { container } = render(
      <CardDefsProvider defs={CATALOG}>
        <Face id={LEGENDARY} tilt="host" />
        <Face id={MYTHIC} tilt="window" />
      </CardDefsProvider>,
    );
    const [host] = [...container.querySelectorAll<HTMLElement>(".host")];
    const [hostFace, windowFace] = [...container.querySelectorAll<HTMLElement>(".cf")];
    if (host === undefined || hostFace === undefined || windowFace === undefined) throw new Error("no faces");
    // The window's pointer takes the slot from a host that holds it. (A host's own `pointerenter` does not bubble to the window.)
    pointer(host, "pointerenter", 100, 140);
    expect(hostFace.getAttribute("data-tilt")).toBe("on");
    pointer(window, "pointermove", 180, 20);
    expect(windowFace.getAttribute("data-tilt")).toBe("on");
    expectLevel(hostFace, "the host let go");
    // And the host cannot take it back while the window face holds it.
    pointer(host, "pointerenter", 100, 140);
    expectLevel(hostFace, "the host, refused");
    expect(windowFace.getAttribute("data-tilt")).toBe("on");
    // The preview closing frees the slot for the host.
    cleanup();
    const { cf, host: again } = mount("host");
    pointer(again, "pointerenter", 100, 140);
    expect(cf.getAttribute("data-tilt")).toBe("on");
  });

  it("R3344001 while the animation queue drains a face tilts not at all, and one tilting lets go at once", () => {
    const { cf, host } = mount("host");
    pointer(host, "pointermove", 150, 70);
    expect(cf.getAttribute("data-tilt")).toBe("on");
    const { rerender } = render(<Hold held={false} />);
    // The queue's entry starts: the tilting face lets go and none may take the slot.
    rerender(<Hold held />);
    expectLevel(cf, "held");
    pointer(host, "pointermove", 150, 70);
    expectLevel(cf, "held, moving");
    // The entry ends: the next move tilts again.
    rerender(<Hold held={false} />);
    pointer(host, "pointermove", 150, 70);
    expect(cf.getAttribute("data-tilt")).toBe("on");
  });

  it("R3344001 a hold mounted before the face keeps it level, two holds need both gone, and unmounting a hold frees the slot", () => {
    const first = render(<Hold held />);
    const second = render(<Hold held />);
    const { cf, host } = mount("host");
    pointer(host, "pointermove", 150, 70);
    expectLevel(cf, "two holds");
    first.unmount();
    pointer(host, "pointermove", 150, 70);
    expectLevel(cf, "one hold");
    second.unmount();
    pointer(host, "pointermove", 150, 70);
    expect(cf.getAttribute("data-tilt")).toBe("on");
  });
});

describe("R3344001 reduced motion draws no depth to tilt", () => {
  it("R3344001 reduced motion draws none: the Reduce motion setting, the fx intensity off and a stilled face take no pointer", async () => {
    writeSettings({ reduceMotion: true });
    const { cf, host } = mount("host");
    expect(cf.hasAttribute("data-depth")).toBe(false);
    pointer(host, "pointermove", 150, 70);
    pointer(host, "pointerdown", 150, 70, "touch");
    expectLevel(cf, "Reduce motion");
    cleanup();
    writeSettings({ reduceMotion: false });
    const { setFxSettings } = await import("../fx/settings.ts");
    setFxSettings({ intensity: "off" });
    const off = mount("window");
    expect(off.cf.hasAttribute("data-depth")).toBe(false);
    pointer(window, "pointermove", 150, 70);
    expectLevel(off.cf, "fx off");
    cleanup();
    // Turned on while a face is mounted, the face draws its depth and tilts; turned off again it levels and goes flat.
    setFxSettings({ intensity: "normal" });
    const live = mount("window");
    pointer(window, "pointermove", 150, 70);
    expect(live.cf.getAttribute("data-tilt")).toBe("on");
    act(() => {
      writeSettings({ reduceMotion: true });
    });
    expect(live.cf.hasAttribute("data-depth")).toBe(false);
    expectLevel(live.cf, "turned off while tilting");
  });
});
