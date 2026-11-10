// R3350000 (#3350): the faces that draw a Mythic's diorama (art/dioramas/). A face with depth whose normal picture has one draws
// its slices in place of the planes, the farthest in the base window and each other a plane of its own at its depth, bent as
// it says, moving by its loop only while the face holds a motion slot; together they are the still picture. Alt art, reduced
// motion, the system's setting and the fx intensity off draw none. Where the player turns it to look at it (inspect's drag,
// the hover preview, the Collection's grid) it turns to DIORAMA_TURN_MAX_DEG, in hand CARD_TILT_MAX_DEG. A board Mythic draws
// every slice in its window while it moves. The slices' data is dioramas.test.ts's; the pixels are e2e's
// cypress/component/card-diorama.cy.tsx.

import { readFileSync } from "node:fs";
import { dirname, join } from "node:path";
import { fileURLToPath } from "node:url";

import { act, cleanup, render } from "@testing-library/react";
import type { ReactElement } from "react";
import { afterEach, beforeEach, describe, expect, it } from "vitest";

import { CATALOG } from "@jackioh/cards";

import { resetFxSettingsForTests, setFxSettings } from "../fx/settings.ts";
import { __resetSettingsForTests, writeSettings } from "../settings/store.ts";
import { unit } from "../test/fixtures.ts";
import { facePicture, wholePicture } from "../test/artPicture.ts";
import { setReducedMotion } from "../test/setup.ts";
import { DIORAMAS, dioramaOf, dioramaSliceUri } from "./art/dioramas/index.ts";
import { artPlaneUri } from "./art/pictures.ts";
import { CardFace } from "./CardFace.tsx";
import {
  CARD_DEPTH_STEP_CQW,
  CARD_TILT_MAX_DEG,
  COSMETIC_FOIL_ANIMATED_MAX,
  DIORAMA_BEND_PERSPECTIVE_PX,
  DIORAMA_DEPTH_STEPS,
  DIORAMA_TURN_MAX_DEG,
} from "./constants.ts";
import { resetCosmeticsForTests, writeCardLevel } from "./cosmetics.ts";
import { FullArtOwned, artMotionOf } from "./fullArt.ts";
import { MinionFace } from "./MinionFace.tsx";
import { faceModel } from "./model.ts";
import { resetMotionSlotsForTests } from "./motionSlots.ts";
import { CARD_SETTINGS_DEFAULTS, writeCardSettings } from "./settings.ts";
import { CardDefsProvider } from "./refContext.tsx";
import { resetTiltForTests, type TiltMode } from "./tilt.ts";
import { VERSION_ALT_ART_ID, VERSION_ANIMATED_ID, offeredOn } from "./versions.ts";

const HERE = dirname(fileURLToPath(import.meta.url));
const MYTHIC = "core-100";
/** A token printed Mythic, drawn by the Stinkyscape painter itself. */
const TOKEN = "stinkyscape-004-5";
const FACES = [false, true] as const;
/** The box a face's host reads the pointer against. */
const BOX = { left: 0, top: 0, width: 200, height: 280 };

function defOf(id: string) {
  const def = CATALOG[id];
  if (def === undefined) throw new Error(`no ${id}`);
  return def;
}

function modelOf(id: string, radiant = false): ReturnType<typeof faceModel> {
  return faceModel({ defId: id, def: defOf(id), radiant });
}

function Host({ children }: { children: ReactElement }): ReactElement {
  return (
    <div
      className="host"
      ref={(node) => {
        if (node !== null) node.getBoundingClientRect = () => ({ ...BOX, right: BOX.width, bottom: BOX.height, x: 0, y: 0, toJSON: () => ({}) });
      }}
    >
      {children}
    </div>
  );
}

interface FaceOptions {
  readonly radiant?: boolean;
  readonly tilt?: TiltMode;
  readonly ownHand?: boolean;
  readonly turn?: boolean;
  readonly lazyArt?: boolean;
}

function faceIn(id: string, { radiant = false, tilt = "off", ownHand = false, turn = false, lazyArt = false }: FaceOptions = {}): HTMLElement {
  const { container } = render(
    <CardDefsProvider defs={CATALOG}>
      <Host>
        <CardFace face={modelOf(id, radiant)} layout="full" worn ownHand={ownHand} tilt={tilt} turn={turn} lazyArt={lazyArt} />
      </Host>
    </CardDefsProvider>,
  );
  const cf = container.querySelector<HTMLElement>(".cf");
  if (cf === null) throw new Error(`no face for ${id}`);
  return cf;
}

/** A window's picture as its background holds it: the data URI between `url("` and `")`. */
function uriOf(element: Element | null | undefined): string {
  return /^url\("(.*)"\)$/u.exec((element as HTMLElement | null | undefined)?.style.backgroundImage ?? "")?.[1] ?? "";
}

function pointer(target: EventTarget, type: string, x: number, y: number): void {
  act(() => {
    target.dispatchEvent(new MouseEvent(type, { bubbles: true, cancelable: true, clientX: x, clientY: y }));
  });
}

beforeEach(() => {
  localStorage.clear();
  resetCosmeticsForTests();
  resetMotionSlotsForTests();
  resetTiltForTests();
  writeCardSettings({ ...CARD_SETTINGS_DEFAULTS });
});

afterEach(() => {
  cleanup();
  setReducedMotion(false);
  resetFxSettingsForTests();
  resetCosmeticsForTests();
  resetMotionSlotsForTests();
  resetTiltForTests();
  localStorage.clear();
  writeCardSettings({ ...CARD_SETTINGS_DEFAULTS });
  __resetSettingsForTests();
});

describe("R3350000 a Mythic's face draws its diorama", () => {
  it("R3350000 a Mythic's face with depth draws its diorama", () => {
    for (const id of [MYTHIC, TOKEN]) {
      const diorama = dioramaOf(id);
      if (diorama === null) throw new Error(`${id} has no diorama`);
      for (const radiant of FACES) {
        const where = `${id} ${radiant ? "Radiant" : "base"}`;
        const cf = faceIn(id, { radiant });
        expect(cf.getAttribute("data-depth"), where).toBe("true");
        expect(cf.getAttribute("data-diorama"), where).toBe("true");
        const moving = cf.getAttribute("data-diorama-motion") === "animated";
        // The farthest slice is the base window's picture.
        const base = cf.querySelector<HTMLElement>(".cf-art-frame > .cf-art");
        expect(base?.getAttribute("data-art-slice"), where).toBe("0");
        expect(uriOf(base), where).toBe(dioramaSliceUri(id, radiant, 0, moving));
        // Each other slice is a plane of its own over it, at its depth in parallax steps, bent as it says.
        const planes = [...cf.querySelectorAll<HTMLElement>(".cf-art-frame > .cf-planes > .cf-plane")];
        expect(planes.length, where).toBe(diorama.slices.length - 1);
        diorama.slices.slice(1).forEach((slice, k) => {
          const plane = planes[k];
          const at = `${where} ${slice.name}`;
          expect(plane?.getAttribute("data-slice"), at).toBe(String(k + 1));
          expect(plane?.getAttribute("data-plane"), at).toBe(slice.plane);
          expect(plane?.style.getPropertyValue("--cf-plane"), at).toBe(String(Number((slice.depth * DIORAMA_DEPTH_STEPS).toFixed(2))));
          expect(plane?.getAttribute("data-bend") ?? undefined, at).toBe(slice.bend?.axis);
          expect(plane?.style.getPropertyValue("--cf-slice-bend"), at).toBe(slice.bend === undefined ? "" : `${slice.bend.deg}deg`);
          const art = plane?.querySelector<HTMLElement>(":scope > .cf-art");
          expect(art?.getAttribute("data-art-slice"), at).toBe(String(k + 1));
          expect(uriOf(art), at).toBe(dioramaSliceUri(id, radiant, k + 1, moving));
        });
        // The glare and the aria-hidden planes are the depth face's own.
        expect(cf.querySelector(".cf-planes")?.getAttribute("aria-hidden"), where).toBe("true");
        expect(cf.querySelectorAll(".cf-art-frame > .cf-glare--art"), where).toHaveLength(1);
        cleanup();
      }
    }
  });

  describe("R3350000 every diorama face draws its still picture", () => {
    // One card a test: each draws its face twice and decodes every slice.
    for (const id of DIORAMAS.keys()) {
      it(`R3350000 ${id}'s face, base and Radiant`, () => {
        for (const radiant of FACES) {
          const where = `${id} ${radiant ? "Radiant" : "base"}`;
          const cf = faceIn(id, { radiant });
          expect(cf.getAttribute("data-diorama"), where).toBe("true");
          expect(facePicture(cf), where).toBe(wholePicture(decodeURIComponent(artPlaneUri(id, "whole", radiant))));
          cleanup();
        }
      });
    }
  });

  describe("R3350000 at an animated level a diorama's slices move as the card's moving picture does", () => {
    // R2715001: an animated level moves the art's own layers; on a Mythic its slices carry them, under their own loops. A
    // token is offered no level of its own.
    const offered = [...DIORAMAS.keys()].filter((id) => offeredOn(VERSION_ANIMATED_ID, id));
    it("R3350000 every Mythic card is offered its Animated level", () => {
      expect(offered.length).toBe([...DIORAMAS.keys()].filter((id) => defOf(id).rarity === "Mythic").length);
    });
    for (const id of offered) {
      it(`R3350000 ${id} at its Animated level`, () => {
        writeCardLevel(id, VERSION_ANIMATED_ID);
        const cf = faceIn(id);
        expect(cf.getAttribute("data-diorama")).toBe("true");
        expect(cf.getAttribute("data-art-motion")).toBe("animated");
        expect(cf.getAttribute("data-diorama-motion")).toBe("animated");
        expect(facePicture(cf)).toBe(wholePicture(decodeURIComponent(artPlaneUri(id, "whole", false, artMotionOf(id)))));
      });
    }
  });

  it("R3350000 Alt art, reduced motion, the system's setting and fx off draw none", () => {
    // Alt art draws its own picture, which has no diorama: the planes, as any face with depth.
    writeCardLevel(MYTHIC, VERSION_ALT_ART_ID);
    const alt = faceIn(MYTHIC);
    expect(alt.getAttribute("data-version")).toBe(VERSION_ALT_ART_ID);
    expect(alt.getAttribute("data-depth")).toBe("true");
    expect(alt.hasAttribute("data-diorama")).toBe(false);
    expect(alt.querySelector("[data-art-slice], [data-slice]")).toBeNull();
    expect([...alt.querySelectorAll(".cf-plane")].map((plane) => plane.getAttribute("data-plane"))).toEqual(["middle", "near", "nearest"]);
    cleanup();
    writeCardLevel(MYTHIC, null);
    const still = (where: string): void => {
      const cf = faceIn(MYTHIC);
      expect(cf.hasAttribute("data-depth"), where).toBe(false);
      expect(cf.hasAttribute("data-diorama"), where).toBe(false);
      expect(cf.hasAttribute("data-diorama-motion"), where).toBe(false);
      expect(cf.querySelector(".cf-plane, [data-art-slice], .cf-art-slice"), where).toBeNull();
      // The flat picture, whole.
      expect(facePicture(cf), where).toBe(wholePicture(decodeURIComponent(artPlaneUri(MYTHIC, "whole"))));
      cleanup();
    };
    act(() => {
      writeSettings({ reduceMotion: true });
    });
    still("Reduce motion");
    act(() => {
      __resetSettingsForTests();
    });
    setReducedMotion(true);
    still("the system's setting");
    setReducedMotion(false);
    act(() => {
      setFxSettings({ intensity: "off" });
    });
    still("fx intensity off");
  });

  it("R3350000 a diorama turns to DIORAMA_TURN_MAX_DEG in inspect, the preview and the Collection, CARD_TILT_MAX_DEG in hand", () => {
    const cases: readonly (readonly [string, FaceOptions, number])[] = [
      ["inspect's drag", { tilt: "drag" }, DIORAMA_TURN_MAX_DEG],
      ["the hover preview", { tilt: "window" }, DIORAMA_TURN_MAX_DEG],
      ["the Collection", { tilt: "host", turn: true, lazyArt: true }, DIORAMA_TURN_MAX_DEG],
      ["the hand", { tilt: "host", ownHand: true }, CARD_TILT_MAX_DEG],
    ];
    for (const [where, options, cap] of cases) {
      const cf = faceIn(MYTHIC, options);
      expect(cf.getAttribute("data-diorama"), where).toBe("true");
      const host = cf.parentElement as HTMLElement;
      // The right edge of the host turns it the whole way along y, the top-left corner the whole way back on both.
      const target = options.tilt === "host" ? host : window;
      pointer(target, "pointermove", BOX.width, BOX.height / 2);
      expect(cf.style.getPropertyValue("--cf-tilt-ry"), where).toBe(`${cap}deg`);
      pointer(target, "pointermove", 0, 0);
      expect(cf.style.getPropertyValue("--cf-tilt-ry"), where).toBe(`${-cap}deg`);
      expect(cf.style.getPropertyValue("--cf-tilt-rx"), where).toBe(`${cap}deg`);
      // The parallax step scales with the turn, so a slice shifts as far for its angle.
      expect(cf.style.getPropertyValue("--cf-depth-step"), where).toBe(`${Number(((CARD_DEPTH_STEP_CQW * cap) / CARD_TILT_MAX_DEG).toFixed(2))}cqw`);
      cleanup();
      resetTiltForTests();
    }
    // A face with depth but no diorama keeps the face's cap in inspect.
    writeCardLevel(MYTHIC, VERSION_ALT_ART_ID);
    const alt = faceIn(MYTHIC, { tilt: "drag" });
    pointer(window, "pointermove", BOX.width, BOX.height / 2);
    expect(alt.style.getPropertyValue("--cf-tilt-ry")).toBe(`${CARD_TILT_MAX_DEG}deg`);
  });

  it("R3350000 a diorama never breaks the frame, where the planes of a Mythic in Alt art do", () => {
    // In hand, the hover preview and inspect: where R3344000 lets a Mythic's near plane break the window's top edge.
    const where = [{ ownHand: true, tilt: "host" }, { tilt: "window" }, { tilt: "drag" }] as const;
    for (const id of [MYTHIC, TOKEN]) {
      for (const options of where) {
        const at = `${id} ${JSON.stringify(options)}`;
        const cf = faceIn(id, options);
        expect(cf.getAttribute("data-diorama"), at).toBe("true");
        expect(cf.hasAttribute("data-frame-break"), at).toBe(false);
        expect(cf.style.getPropertyValue("--cf-frame-break"), at).toBe("");
        expect(cf.style.getPropertyValue("--cf-frame-break-scale"), at).toBe("");
        cleanup();
        resetTiltForTests();
      }
    }
    writeCardLevel(MYTHIC, VERSION_ALT_ART_ID);
    for (const options of where) {
      const cf = faceIn(MYTHIC, options);
      expect(cf.hasAttribute("data-diorama"), JSON.stringify(options)).toBe(false);
      expect(cf.getAttribute("data-frame-break"), JSON.stringify(options)).toBe("true");
      cleanup();
      resetTiltForTests();
    }
  });

  it("R3350000 the loop runs only in a slot", () => {
    // Alone, the face holds a slot: every slice that has a loop draws it, and those with none their still picture.
    const diorama = dioramaOf(MYTHIC);
    if (diorama === null) throw new Error("no diorama");
    const one = faceIn(MYTHIC);
    expect(one.getAttribute("data-diorama-motion")).toBe("animated");
    const windows = [one.querySelector(".cf-art-frame > .cf-art"), ...one.querySelectorAll(".cf-planes .cf-art")];
    windows.forEach((art, index) => {
      expect(uriOf(art), `slice ${index}`).toBe(dioramaSliceUri(MYTHIC, false, index, true));
      expect(decodeURIComponent(uriOf(art)).includes("<style>"), `slice ${index}`).toBe(diorama.slices[index]?.loop !== undefined);
    });
    cleanup();
    resetMotionSlotsForTests();
    // More Mythics than slots: the oldest holds none, and its slices hold still.
    const { container } = render(
      <CardDefsProvider defs={CATALOG}>
        <>
          {Array.from({ length: COSMETIC_FOIL_ANIMATED_MAX + 1 }, (_, index) => (
            <CardFace key={index} face={modelOf(MYTHIC)} layout="full" worn />
          ))}
        </>
      </CardDefsProvider>,
    );
    const faces = [...container.querySelectorAll<HTMLElement>(".cf")];
    expect(faces.map((cf) => cf.getAttribute("data-diorama-motion"))).toEqual(["static", ...Array<string>(COSMETIC_FOIL_ANIMATED_MAX).fill("animated")]);
    const oldest = faces[0];
    expect(uriOf(oldest?.querySelector(".cf-art-frame > .cf-art"))).toBe(dioramaSliceUri(MYTHIC, false, 0, false));
    for (const art of oldest?.querySelectorAll(".cf-planes .cf-art") ?? []) expect(decodeURIComponent(uriOf(art))).not.toContain("<style");
    cleanup();
    resetMotionSlotsForTests();
    // Animated foil off: none moves, and a grid's lazy face neither.
    writeCardSettings({ ...CARD_SETTINGS_DEFAULTS, animatedFoil: false });
    expect(faceIn(MYTHIC).getAttribute("data-diorama-motion")).toBe("static");
    cleanup();
    writeCardSettings({ ...CARD_SETTINGS_DEFAULTS });
    expect(faceIn(MYTHIC, { lazyArt: true }).getAttribute("data-diorama-motion")).toBe("static");
  });

  it("R3350000 a board Mythic draws its slices while moving", () => {
    const Minion = (): ReactElement => {
      const printed = defOf(MYTHIC).base;
      const health = printed.health ?? 4;
      const u = unit("p1", { defId: MYTHIC, attack: printed.attack ?? 0, health, maxHealth: health });
      const face = faceModel({ defId: MYTHIC, def: defOf(MYTHIC), radiant: false, live: { attack: u.attack, health: u.health, maxHealth: u.maxHealth, keywords: u.keywords } });
      return (
        <div className="card cf-host">
          <MinionFace face={face} unit={u} worn />
        </div>
      );
    };
    const diorama = dioramaOf(MYTHIC);
    if (diorama === null) throw new Error("no diorama");
    const board = (): HTMLElement => {
      const { container } = render(
        <FullArtOwned owned="every">
          <Minion />
        </FullArtOwned>,
      );
      const cf = container.querySelector<HTMLElement>(".cf--minion");
      if (cf === null) throw new Error("no minion");
      return cf;
    };
    // A Mythic on the board moves in its slot: its window holds every slice, each on its loop and moving as its art does
    // (R3340000), and no live layers.
    const motion = artMotionOf(MYTHIC);
    const moving = board();
    expect(moving.getAttribute("data-art-motion")).toBe("animated");
    const art = moving.querySelector<HTMLElement>(".cf-portrait > .cf-art");
    expect(art?.getAttribute("data-art-slice")).toBe("0");
    expect(art?.hasAttribute("data-art-live")).toBe(false);
    expect(uriOf(art) === dioramaSliceUri(MYTHIC, false, 0, true, motion)).toBe(true);
    const layers = [...(art?.querySelectorAll<HTMLElement>(":scope > .cf-art-slice") ?? [])];
    expect(layers.map((layer) => layer.getAttribute("data-slice"))).toEqual(diorama.slices.slice(1).map((_, k) => String(k + 1)));
    layers.forEach((layer, k) => expect(uriOf(layer) === dioramaSliceUri(MYTHIC, false, k + 1, true, motion), `slice ${String(k + 1)}`).toBe(true));
    expect(art?.querySelector(".cf-art-subject, .cf-art-motes")).toBeNull();
    // Together they are the moving picture.
    expect(facePicture(moving)).toBe(wholePicture(decodeURIComponent(artPlaneUri(MYTHIC, "whole", false, motion))));
    cleanup();
    resetMotionSlotsForTests();
    // Held still (Animated foil off), it draws its one still picture, as before.
    writeCardSettings({ ...CARD_SETTINGS_DEFAULTS, animatedFoil: false });
    const still = board();
    const plain = still.querySelector<HTMLElement>(".cf-portrait > .cf-art");
    expect(plain?.hasAttribute("data-art-slice")).toBe(false);
    expect(plain?.querySelector(".cf-art-slice")).toBeNull();
  });

  it("R3350000 a bent slice turns under DIORAMA_BEND_PERSPECTIVE_PX, about the axis it names", () => {
    const css = readFileSync(join(HERE, "depth.css"), "utf8").replace(/\/\*[\s\S]*?\*\//g, "");
    expect(css).toMatch(new RegExp(`\\.cf\\[data-diorama\\] \\.cf-plane\\[data-bend\\]\\s*\\{\\s*perspective: ${String(DIORAMA_BEND_PERSPECTIVE_PX)}px;\\s*\\}`));
    expect(css).toMatch(/\.cf\[data-diorama\] \.cf-plane\[data-bend="y"\] > \.cf-art\s*\{\s*rotate: y var\(--cf-slice-bend, 0deg\);\s*\}/);
    expect(css).toMatch(/\.cf\[data-diorama\] \.cf-plane\[data-bend="x"\] > \.cf-art\s*\{\s*rotate: x var\(--cf-slice-bend, 0deg\);\s*\}/);
  });
});
