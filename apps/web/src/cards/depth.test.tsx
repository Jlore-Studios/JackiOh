// R3344000 (#3344): the depth a rare card's picture has. An Epic or rarer face, and every full-art-shaped level, draws its
// picture as four planes by its painter's roles; a Common in its normal look, a delivered art file, a Starship in play, a
// Legacy card, a restyle and the versions with layers of their own stay one picture; reduced motion, the system's setting
// and the fx intensity off draw the flat picture with the foil still; the glare is by area for each foil; and a Legendary
// or Mythic face in hand and inspect may break the frame. The tilt is tilt.test.tsx's, the pixels are e2e's
// cypress/component/card-depth.cy.tsx.

import { readFileSync } from "node:fs";
import { dirname, join } from "node:path";
import { fileURLToPath } from "node:url";

import { cleanup, render } from "@testing-library/react";
import { afterEach, beforeEach, describe, expect, it } from "vitest";

import { CATALOG } from "@jackioh/cards";

import { CardDefsProvider } from "./refContext.tsx";
import { CardArt, hasArtPlanes } from "./art/CardArt.tsx";
import { artMotionFor, type ArtMotion } from "./art/motion.ts";
import { ART_PLANES, PLANE_OF_ROLE, partDraws, partHolds, type ArtPart, type ArtPlane } from "./art/pictures.ts";
import { CARD_PERSPECTIVE_PX, CARD_FRAME_BREAK_PX, CARD_FRAME_BREAK_SCALE, CARD_DEPTH_STEP_CQW } from "./constants.ts";
import { CardFace } from "./CardFace.tsx";
import { DEFAULT_CARD_COSMETICS, cosmeticsOf, resetCosmeticsForTests, writeCardLevel, writeCardPick } from "./cosmetics.ts";
import { GLARE_BY_FOIL, NO_FOIL_ID, breaksFrame, glareFor, hasDepth } from "./depth.ts";
import { isFullArtCard } from "./fullArt.ts";
import { faceModel } from "./model.ts";
import { CARD_SETTINGS_DEFAULTS, writeCardSettings } from "./settings.ts";
import {
  VERSION_ALT_ART_ID,
  VERSION_ALT_FULL_ART_ANIMATED_ID,
  VERSION_ALT_FULL_ART_ID,
  VERSION_ALT_FULL_ART_SIGNED_ID,
  VERSION_COMIC_ID,
  VERSION_EXTENDED_ART_ID,
  VERSION_FULL_ART_ANIMATED_ID,
  VERSION_FULL_ART_ID,
  VERSION_FULL_ART_SIGNED_ID,
  VERSION_IMMERSIVE_ID,
  VERSION_OUT_OF_FRAME_ID,
  VERSION_PIXEL_ID,
  VERSION_SKETCH_FULL_ART_ANIMATED_ID,
  VERSION_SKETCH_FULL_ART_ID,
  VERSION_SKETCH_FULL_ART_SIGNED_ID,
  VERSION_SKETCH_ID,
  offeredOn,
} from "./versions.ts";
import { __resetSettingsForTests, writeSettings } from "../settings/store.ts";
import { resetFxSettingsForTests, setFxSettings } from "../fx/settings.ts";
import { facePicture, layersOf, wholePicture } from "../test/artPicture.ts";
import { setReducedMotion } from "../test/setup.ts";

const HERE = dirname(fileURLToPath(import.meta.url));

/** A stylesheet as text, comments stripped. */
function sheet(name: string): string {
  return readFileSync(join(HERE, name), "utf8").replace(/\/\*[\s\S]*?\*\//g, "");
}

// Cards that are no full-art card, one of each rarity the issue names, a Legacy card and the Starship.
const EPIC = "core-022";
const LEGENDARY = "core-052";
const MYTHIC = "core-100";
const COMMON = "core-002";
const LEGACY = "traininggrounds-044";
const STARSHIP = "stinkyscape-t-dormant-starship";

function faceModelOf(id: string, radiant: boolean, inPlay?: Parameters<typeof faceModel>[0]["inPlay"]): ReturnType<typeof faceModel> {
  const def = CATALOG[id];
  if (def === undefined) throw new Error(`no ${id}`);
  return faceModel({ defId: id, def, radiant, ...(inPlay === undefined ? {} : { inPlay }) });
}

interface FaceOptions {
  readonly radiant?: boolean;
  readonly ownHand?: boolean;
  readonly tilt?: "off" | "host" | "window" | "drag";
  readonly layout?: "full" | "compact";
  readonly inPlay?: Parameters<typeof faceModel>[0]["inPlay"];
}

function faceIn(id: string, { radiant = false, ownHand = false, tilt = "off", layout = "full", inPlay }: FaceOptions = {}): HTMLElement {
  const { container } = render(
    <CardDefsProvider defs={CATALOG}>
      <CardFace face={faceModelOf(id, radiant, inPlay)} layout={layout} worn ownHand={ownHand} tilt={tilt} />
    </CardDefsProvider>,
  );
  const cf = container.querySelector<HTMLElement>(".cf");
  if (cf === null) throw new Error(`no face for ${id}`);
  return cf;
}

/** The parts the art frame's pictures draw, in the order they stack. */
function partsOf(cf: HTMLElement): (string | null)[] {
  return [...cf.querySelectorAll<HTMLElement>(".cf-art-frame [data-art]")].map((art) => art.getAttribute("data-art-part"));
}

/** A face's pictures, decoded, in the order they stack. */
function picturesOf(cf: HTMLElement): string[] {
  return [...cf.querySelectorAll<HTMLElement>(".cf-art-frame [data-art]")].map((art) => decodeURIComponent(art.style.backgroundImage));
}

beforeEach(() => {
  localStorage.clear();
  resetCosmeticsForTests();
  writeCardSettings({ ...CARD_SETTINGS_DEFAULTS });
});

afterEach(() => {
  cleanup();
  setReducedMotion(false);
  resetFxSettingsForTests();
  resetCosmeticsForTests();
  localStorage.clear();
  writeCardSettings({ ...CARD_SETTINGS_DEFAULTS });
  __resetSettingsForTests();
});

describe("R3344000 the cards under test", () => {
  it("R3344000 are the rarities the tests name, and none is a full-art card", () => {
    expect([EPIC, LEGENDARY, MYTHIC, COMMON].map((id) => CATALOG[id]?.rarity)).toEqual(["Epic", "Legendary", "Mythic", "Common"]);
    for (const id of [EPIC, LEGENDARY, MYTHIC, COMMON, LEGACY, STARSHIP]) expect(isFullArtCard(id), id).toBe(false);
  });
});

describe("R3344000 an Epic or rarer face draws its picture as depth planes", () => {
  it("R3344000 an Epic or rarer face draws far, middle, near and nearest planes", () => {
    for (const id of [EPIC, LEGENDARY, MYTHIC]) {
      for (const radiant of [false, true]) {
        const where = `${id} ${radiant ? "Radiant" : "base"}`;
        const cf = faceIn(id, { radiant });
        expect(cf.getAttribute("data-depth"), where).toBe("true");
        expect(partsOf(cf), where).toEqual([...ART_PLANES]);
        // The far plane is the window; the rest are planes over it, each its own box.
        const planes = [...cf.querySelectorAll<HTMLElement>(".cf-art-frame > .cf-planes > .cf-plane")];
        expect(planes.map((plane) => plane.getAttribute("data-plane")), where).toEqual(["middle", "near", "nearest"]);
        expect(planes.map((plane) => plane.style.getPropertyValue("--cf-plane")), where).toEqual(["1", "2", "3"]);
        for (const plane of planes) expect(plane.querySelector(".cf-art")?.getAttribute("data-art-variant"), where).toBe(radiant ? "radiant" : "base");
        expect(cf.querySelector(".cf-art-frame > .cf-art")?.getAttribute("data-art-part"), where).toBe("far");
        // Four distinct pictures, none empty.
        const drawn = picturesOf(cf);
        expect(drawn, where).toHaveLength(4);
        expect(new Set(drawn).size, where).toBe(4);
        for (const picture of drawn) expect(picture, where).toContain("<svg");
        // The glare's two layers: one over the art, one over the frame.
        expect(cf.querySelectorAll(".cf-art-frame > .cf-glare--art"), where).toHaveLength(1);
        expect(cf.querySelectorAll(".cf-scale > .cf-glare--frame"), where).toHaveLength(1);
        expect(cf.querySelector(".cf-scale")?.firstElementChild?.classList.contains("cf-glare--frame"), where).toBe(true);
        // Nothing in them names a card or a word: the planes are aria-hidden art (rule 7).
        expect(cf.querySelector(".cf-planes")?.getAttribute("aria-hidden"), where).toBe("true");
      }
    }
  });

  it("R3344000 the planes together are the picture the face drew whole, each layer in exactly one plane", () => {
    const motion: ArtMotion = {
      back: { move: "drift", seconds: 8 },
      subject: { move: "bob", seconds: 4 },
      over: { move: "swell", seconds: 5 },
      light: { move: "pulse", seconds: 3 },
      props: { move: "float", seconds: 3 },
      motes: { move: "twinkle", seconds: 2 },
      particles: { kind: "spark", beat: { move: "glint", seconds: 2 } },
    };
    // A procedural card, a Stinkyscape one and a Training Grounds one; base and Radiant; still and moving.
    const cards = [EPIC, LEGENDARY, MYTHIC, "stinkyscape-154", "traininggrounds-040", "classic-040"];
    let seen = 0;
    for (const id of cards) {
      const def = CATALOG[id];
      if (def === undefined) throw new Error(`no ${id}`);
      for (const radiant of [false, true]) {
        for (const moving of [null, motion]) {
          const where = `${id} ${radiant ? "Radiant" : "base"} ${moving === null ? "still" : "moving"}`;
          const art = (part: ArtPart): HTMLElement => {
            const { container } = render(
              <CardDefsProvider defs={CATALOG}>
                <CardArt defId={id} radiant={radiant} tags={def.tags} type={def.type} shape="full" name={def.name} motion={moving} part={part} />
              </CardDefsProvider>,
            );
            const window = container.querySelector<HTMLElement>(".cf-art");
            if (window === null) throw new Error("no art");
            return window;
          };
          const drawn = (part: ArtPart): string => decodeURIComponent(art(part).style.backgroundImage);
          const whole = drawn("whole");
          cleanup();
          const planes = ART_PLANES.map((plane) => ({ plane, picture: drawn(plane) }));
          cleanup();
          const backdrop = drawn("backdrop");
          cleanup();
          const subject = drawn("subject");
          cleanup();
          const split = planes.map(({ plane, picture }) => ({ plane, picture, ...layersOf(picture.slice(picture.indexOf("<svg"), picture.lastIndexOf("</svg>") + 6)) }));
          // Every plane draws one picture's definitions and style, and the planes' layers are the whole's, each once.
          for (const part of split) {
            expect(part.defs, `${where} ${part.plane}`).toBe(split[0]?.defs);
            expect(part.style, `${where} ${part.plane}`).toBe(split[0]?.style);
          }
          const union = split.flatMap((part) => part.layers).sort();
          const wholeLayers = layersOf(whole.slice(whole.indexOf("<svg"), whole.lastIndexOf("</svg>") + 6)).layers.sort();
          expect(union, where).toEqual(wholeLayers);
          // And the backdrop and the subject, the parts Out of frame draws, are the planes of theirs.
          const backdropLayers = layersOf(backdrop.slice(backdrop.indexOf("<svg"), backdrop.lastIndexOf("</svg>") + 6)).layers.sort();
          const subjectLayers = layersOf(subject.slice(subject.indexOf("<svg"), subject.lastIndexOf("</svg>") + 6)).layers.sort();
          const nearLayers = split.find((part) => part.plane === "near")?.layers.sort();
          expect(subjectLayers, where).toEqual(nearLayers);
          expect([...backdropLayers, ...subjectLayers].sort(), where).toEqual(wholeLayers);
          seen += 1;
        }
      }
    }
    expect(seen).toBe(cards.length * 4);
  });

  it("R3344000 each role lands in exactly one plane, the roles by PLANE_OF_ROLE", () => {
    // The table: the background far, the props middle, the subject near, the rest nearest.
    expect(PLANE_OF_ROLE).toEqual({ back: "far", props: "middle", subject: "near", over: "nearest", light: "nearest", motes: "nearest" });
    expect([...ART_PLANES]).toEqual(["far", "middle", "near", "nearest"]);
    for (const role of Object.keys(PLANE_OF_ROLE) as (keyof typeof PLANE_OF_ROLE)[]) {
      expect(ART_PLANES.filter((plane) => partDraws(plane, role)), role).toEqual([PLANE_OF_ROLE[role]]);
      expect(partDraws("whole", role), role).toBe(true);
      expect(partDraws("subject", role), role).toBe(role === "subject");
      expect(partDraws("backdrop", role), role).toBe(role !== "subject");
    }
    // What is held in no role: the sky, the rays and the vignette far, the particles nearest.
    expect(ART_PLANES.filter((plane) => partHolds(plane, "far"))).toEqual(["far"]);
    expect(ART_PLANES.filter((plane) => partHolds(plane, "nearest"))).toEqual(["nearest"]);
    expect(partHolds("whole", "far") && partHolds("whole", "nearest") && partHolds("whole", "near")).toBe(true);
    expect(partHolds("backdrop", "far") && partHolds("backdrop", "nearest")).toBe(true);
    expect(partHolds("backdrop", "near")).toBe(false);
    expect(partHolds("subject", "far") || partHolds("subject", "nearest")).toBe(false);

    // In a picture drawn with every role moving: each role's layer is in its plane alone, the painters' own way.
    const motion: ArtMotion = {
      back: { move: "drift", seconds: 8 },
      subject: { move: "bob", seconds: 4 },
      over: { move: "swell", seconds: 5 },
      light: { move: "pulse", seconds: 3 },
      props: { move: "float", seconds: 3 },
      motes: { move: "twinkle", seconds: 2 },
      particles: { kind: "spark", beat: { move: "glint", seconds: 2 } },
    };
    const classes = (part: ArtPlane, id: string): Set<string> => {
      const def = CATALOG[id];
      if (def === undefined) throw new Error(`no ${id}`);
      const { container } = render(
        <CardDefsProvider defs={CATALOG}>
          <CardArt defId={id} radiant={false} tags={def.tags} type={def.type} shape="full" name={def.name} motion={motion} part={part} />
        </CardDefsProvider>,
      );
      const svg = decodeURIComponent(container.querySelector<HTMLElement>(".cf-art")?.style.backgroundImage ?? "");
      cleanup();
      // The layers of the picture itself, not the style that names every role it moves.
      const body = svg.slice(svg.indexOf("</defs>"));
      const found = new Set<string>();
      for (const match of body.replace(/<style>[\s\S]*?<\/style>/, "").matchAll(/class="art-m-([a-z]+)"/g)) found.add(match[1] ?? "");
      return found;
    };
    for (const id of [EPIC, LEGENDARY, MYTHIC, "classic-040"]) {
      const byPlane = new Map(ART_PLANES.map((plane) => [plane, classes(plane, id)] as const));
      const roles = new Set([...byPlane.values()].flatMap((found) => [...found]));
      expect(roles.has("back"), id).toBe(true);
      expect(roles.has("subject"), id).toBe(true);
      for (const role of roles) {
        if (role === "particle") continue;
        const inPlanes = ART_PLANES.filter((plane) => byPlane.get(plane)?.has(role) === true);
        expect(inPlanes, `${id} ${role}`).toEqual([PLANE_OF_ROLE[role as keyof typeof PLANE_OF_ROLE]]);
      }
      // The particles are the nearest plane's alone.
      expect(ART_PLANES.filter((plane) => byPlane.get(plane)?.has("particle") === true), id).toEqual(["nearest"]);
    }
    // Stinkyscape's and Training Grounds': the background far, the subject near, nothing between.
    for (const id of ["stinkyscape-154", "traininggrounds-040"]) {
      const byPlane = new Map(ART_PLANES.map((plane) => [plane, classes(plane, id)] as const));
      expect([...(byPlane.get("far") ?? [])], id).toEqual(["back"]);
      expect([...(byPlane.get("middle") ?? [])], id).toEqual([]);
      expect([...(byPlane.get("near") ?? [])], id).toEqual(["subject"]);
      expect([...(byPlane.get("nearest") ?? [])], id).toEqual(["particle"]);
    }
  });

  it("R3344000 every full-art-shaped level draws planes, at any rarity; the Sketch's stay one picture", () => {
    // Levels that add the full-art frame, and the animated and signed ones above them (R3015002).
    const FULL_ART_SHAPED = [
      VERSION_FULL_ART_ID,
      VERSION_FULL_ART_ANIMATED_ID,
      VERSION_FULL_ART_SIGNED_ID,
      VERSION_ALT_FULL_ART_ID,
      VERSION_ALT_FULL_ART_ANIMATED_ID,
      VERSION_ALT_FULL_ART_SIGNED_ID,
    ];
    const SKETCH_FULL_ART = [VERSION_SKETCH_FULL_ART_ID, VERSION_SKETCH_FULL_ART_ANIMATED_ID, VERSION_SKETCH_FULL_ART_SIGNED_ID];
    // A Common and a Rare, which in their normal look have no depth, and an Epic, which has.
    const offered = (rarity: string): string | undefined =>
      Object.values(CATALOG).find((def) => def.rarity === rarity && !isFullArtCard(def.id) && offeredOn(VERSION_ALT_FULL_ART_ID, def.id))?.id;
    const common = offered("Common");
    const rare = offered("Rare");
    expect(common, "a Common offered Alt full art").toBeDefined();
    expect(rare, "a Rare offered Alt full art").toBeDefined();
    const ids = [common ?? "", rare ?? "", EPIC];
    let drawn = 0;
    for (const id of ids) {
      expect(offeredOn(VERSION_ALT_FULL_ART_ID, id), `${id} is offered Alt full art`).toBe(true);
      for (const radiant of [false, true]) {
        writeCardLevel(id, null);
        const normal = faceIn(id, { radiant });
        expect(normal.getAttribute("data-depth") === "true", `${id} normal`).toBe(CATALOG[id]?.rarity === "Epic");
        cleanup();
        for (const level of FULL_ART_SHAPED) {
          if (!offeredOn(level, id)) continue;
          const where = `${id} ${radiant ? "Radiant" : "base"} ${level}`;
          writeCardLevel(id, level);
          const cf = faceIn(id, { radiant });
          expect(cf.getAttribute("data-full-art"), where).toBe("true");
          expect(cf.getAttribute("data-depth"), where).toBe("true");
          expect(partsOf(cf), where).toEqual([...ART_PLANES]);
          // The planes take the full face's shape.
          for (const plane of cf.querySelectorAll(".cf-plane")) expect(plane.classList.contains("cf-art--full"), where).toBe(true);
          drawn += 1;
          cleanup();
        }
        for (const level of SKETCH_FULL_ART) {
          if (!offeredOn(level, id)) continue;
          const where = `${id} ${radiant ? "Radiant" : "base"} ${level}`;
          writeCardLevel(id, level);
          const cf = faceIn(id, { radiant });
          expect(cf.getAttribute("data-full-art"), where).toBe("true");
          expect(cf.hasAttribute("data-depth"), where).toBe(false);
          expect(cf.querySelector(".cf-planes"), where).toBeNull();
          expect(partsOf(cf), where).toEqual([null]);
          cleanup();
        }
      }
    }
    expect(drawn).toBeGreaterThan(6);
    // The hero cards, which print full art in their own right (no pick of their own), draw planes too.
    expect(isFullArtCard("core-001")).toBe(true);
    const cf = faceIn("core-001");
    expect(cf.getAttribute("data-full-art")).toBe("true");
    expect(cf.getAttribute("data-depth")).toBe("true");
    expect(partsOf(cf)).toEqual([...ART_PLANES]);
  });

  it("R3344000 a Common, a delivered file, a Starship and a Legacy card stay one plane", () => {
    // A Common in its normal look, base and Radiant.
    for (const radiant of [false, true]) {
      const cf = faceIn(COMMON, { radiant });
      expect(cf.hasAttribute("data-depth")).toBe(false);
      expect(cf.querySelector(".cf-planes")).toBeNull();
      expect(cf.querySelector(".cf-glare")).toBeNull();
      expect(partsOf(cf)).toEqual([null]);
      cleanup();
    }
    // A delivered art file has no layers: the painter draws no planes in its place, except under Alt art's own picture.
    const file = { [LEGENDARY]: { base: true as const } };
    expect(hasArtPlanes(LEGENDARY, null, "normal", false)).toBe(true);
    expect(hasArtPlanes(LEGENDARY, null, "normal", false, file)).toBe(false);
    expect(hasArtPlanes(LEGENDARY, null, "normal", true, file)).toBe(false);
    expect(hasArtPlanes(LEGENDARY, null, "alt", false, file)).toBe(true);
    // A Starship in play and a Legacy card draw one picture, whatever the rarity.
    expect(hasArtPlanes(STARSHIP, { launched: false, list: [] }, "normal", false)).toBe(false);
    expect(hasArtPlanes(LEGACY, null, "normal", false)).toBe(false);
    expect(hasArtPlanes(STARSHIP, null, "normal", false)).toBe(true);
    // A restyle is a filter over one picture.
    for (const picture of ["sketch", "pixel", "comic", "depth", "prismatic", "woodcut", "neon", "night", "watercolor", "stained-glass", "tarot", "showcase-core"] as const) {
      expect(hasArtPlanes(LEGENDARY, null, picture, false), picture).toBe(false);
    }
    const legacy = faceIn(LEGACY);
    expect(legacy.hasAttribute("data-depth")).toBe(false);
    expect(partsOf(legacy)).toEqual([null]);
    cleanup();
    const starship = faceIn(STARSHIP, { inPlay: { starship: { pieces: 0, name: null, list: [] } } });
    expect(starship.hasAttribute("data-depth")).toBe(false);
    expect(partsOf(starship)).toEqual([null]);
    cleanup();
    // A face in the version a card wears has its own layers or none: Immersive, Out of frame and Extended art keep theirs, the
    // restyles one picture, at a rarity that would have depth.
    const epicPlus = (level: string): string => {
      const found = Object.values(CATALOG).find(
        (def) => (def.rarity === "Epic" || def.rarity === "Legendary" || def.rarity === "Mythic") && !isFullArtCard(def.id) && offeredOn(level, def.id),
      );
      expect(found, `an Epic or rarer card offered ${level}`).toBeDefined();
      return found?.id ?? "";
    };
    for (const level of [VERSION_IMMERSIVE_ID, VERSION_OUT_OF_FRAME_ID, VERSION_EXTENDED_ART_ID, VERSION_SKETCH_ID, VERSION_PIXEL_ID, VERSION_COMIC_ID]) {
      const id = epicPlus(level);
      writeCardLevel(id, level);
      const cf = faceIn(id);
      expect(cf.getAttribute("data-version"), level).toBe(level);
      expect(cf.hasAttribute("data-depth"), level).toBe(false);
      expect(cf.querySelector(".cf-planes"), level).toBeNull();
      if (level === VERSION_IMMERSIVE_ID) expect(cf.querySelector(".cf-immersive"), level).not.toBeNull();
      if (level === VERSION_OUT_OF_FRAME_ID) expect(cf.querySelector(".cf-art--cutout"), level).not.toBeNull();
      cleanup();
    }
    const id = epicPlus(VERSION_ALT_ART_ID);
    // Alt art draws the painter's own picture, which has planes at any rarity this face has.
    writeCardLevel(id, VERSION_ALT_ART_ID);
    expect(faceIn(id).getAttribute("data-depth")).toBe("true");
  });

  it("R3344000 hasDepth is rarity, full art and a picture of planes, none of Glitch, Immersive, Out of frame or Extended art", () => {
    const base = { rarity: "Epic" as const, fullArt: false, version: null, glitch: false, planed: true };
    expect(hasDepth(base)).toBe(true);
    expect(hasDepth({ ...base, rarity: "Legendary" })).toBe(true);
    expect(hasDepth({ ...base, rarity: "Mythic" })).toBe(true);
    for (const rarity of ["Common", "Rare", "Token", null] as const) {
      expect(hasDepth({ ...base, rarity }), String(rarity)).toBe(false);
      expect(hasDepth({ ...base, rarity, fullArt: true }), `${String(rarity)} full art`).toBe(true);
    }
    expect(hasDepth({ ...base, planed: false })).toBe(false);
    expect(hasDepth({ ...base, planed: false, fullArt: true })).toBe(false);
    expect(hasDepth({ ...base, glitch: true })).toBe(false);
    for (const version of [VERSION_IMMERSIVE_ID, VERSION_OUT_OF_FRAME_ID, VERSION_EXTENDED_ART_ID]) {
      expect(hasDepth({ ...base, version }), version).toBe(false);
    }
  });

  it("R3344000 the planes and the glare are aria-hidden art, and a flat face draws the picture the planes draw together", () => {
    const cf = faceIn(MYTHIC, { ownHand: true });
    expect(cf.querySelector(".cf-glare--art")?.getAttribute("aria-hidden")).toBe("true");
    expect(cf.querySelector(".cf-glare--frame")?.getAttribute("aria-hidden")).toBe("true");
    for (const art of cf.querySelectorAll(".cf-art-frame [data-art]")) expect(art.getAttribute("aria-hidden")).toBe("true");
    const planed = facePicture(cf);
    cleanup();
    writeSettings({ reduceMotion: true });
    const flat = faceIn(MYTHIC, { ownHand: true });
    expect(flat.hasAttribute("data-depth")).toBe(false);
    // A flat face and the planed one draw the same picture, however it is cut.
    expect(facePicture(flat)).toBe(planed);
    expect(wholePicture(decodeURIComponent(flat.querySelector<HTMLElement>(".cf-art")?.style.backgroundImage ?? ""))).toBe(planed);
  });
});

describe("R3344000 reduced motion, the OS setting and fx intensity off draw the flat picture with the still foil", () => {
  /** A Legendary wearing the holo sweep, and a Radiant Mythic, whose foil is its own. */
  function wornFoil(): void {
    writeCardPick(LEGENDARY, "foil", "foil-holo-sweep");
  }

  it("R3344000 reduced motion, the OS setting and fx intensity off draw the flat picture with the still foil", () => {
    wornFoil();
    const moving = faceIn(LEGENDARY);
    expect(moving.getAttribute("data-depth")).toBe("true");
    expect(moving.getAttribute("data-cosmetic-foil")).toBe("foil-holo-sweep");
    expect(moving.getAttribute("data-cosmetic-motion")).toBe("animated");
    cleanup();
    const radiantMoving = faceIn(MYTHIC, { radiant: true });
    expect(radiantMoving.getAttribute("data-foil")).toBe("animated");
    cleanup();

    // The Reduce motion setting.
    writeSettings({ reduceMotion: true });
    const panel = faceIn(LEGENDARY);
    expect(panel.hasAttribute("data-depth")).toBe(false);
    expect(panel.querySelector(".cf-planes")).toBeNull();
    expect(panel.querySelector(".cf-glare")).toBeNull();
    expect(partsOf(panel)).toEqual([null]);
    cleanup();
    writeSettings({ reduceMotion: false });

    // The system's setting.
    setReducedMotion(true);
    const system = faceIn(LEGENDARY);
    expect(system.hasAttribute("data-depth")).toBe(false);
    expect(partsOf(system)).toEqual([null]);
    cleanup();
    setReducedMotion(false);

    // The fx intensity off: the flat picture, and its foil standing still.
    setFxSettings({ intensity: "off" });
    const fx = faceIn(LEGENDARY);
    expect(fx.hasAttribute("data-depth")).toBe(false);
    expect(partsOf(fx)).toEqual([null]);
    expect(fx.getAttribute("data-cosmetic-foil")).toBe("foil-holo-sweep");
    expect(fx.getAttribute("data-cosmetic-motion")).toBe("static");
    cleanup();
    const radiantStill = faceIn(MYTHIC, { radiant: true });
    expect(radiantStill.hasAttribute("data-depth")).toBe(false);
    expect(radiantStill.getAttribute("data-foil")).toBe("static");
    cleanup();
    // A face with no depth in any case keeps its foil moving under fx off: only a face that would have depth is stilled.
    writeCardPick(COMMON, "foil", "foil-holo-sweep");
    const common = faceIn(COMMON);
    expect(common.getAttribute("data-cosmetic-motion")).toBe("animated");
    cleanup();
    setFxSettings({ intensity: "normal" });
    expect(faceIn(LEGENDARY).getAttribute("data-depth")).toBe("true");
  });

  it("R3344000 the depth stylesheet gives no motion outside the gate that reduced motion and the Reduce motion switch close", () => {
    const css = sheet("depth.css");
    const gate = css.indexOf("@media (prefers-reduced-motion: no-preference)");
    expect(gate).toBeGreaterThan(0);
    const outside = css.slice(0, gate);
    const inside = css.slice(gate);
    // Rotation, parallax, glare and the springs sit inside it, each under the Reduce motion switch's `:root:not(...)`.
    for (const motion of ["rotateX(", "translate:", "transition", "will-change", 'data-tilt="on"']) {
      expect(outside, `${motion} outside the gate`).not.toContain(motion);
      expect(inside, `${motion} inside the gate`).toContain(motion);
    }
    for (const line of inside.split("\n").filter((each) => each.includes("{") && each.includes(".cf"))) {
      expect(line, line).toContain(':root:not([data-reduce-motion="true"])');
    }
  });
});

describe("R3344000 the glare is by area for each foil", () => {
  it("R3344000 the glare is by area for each foil", () => {
    // One row for every foil the registry has, and for none; every share a fraction.
    const ids = cosmeticsOf("foil").map((foil) => foil.id);
    expect(ids).toContain(NO_FOIL_ID);
    expect(ids.length).toBeGreaterThanOrEqual(10);
    expect(Object.keys(GLARE_BY_FOIL).sort()).toEqual([...ids].sort());
    expect(DEFAULT_CARD_COSMETICS.foil).toBe(NO_FOIL_ID);
    for (const id of ids) {
      const row = GLARE_BY_FOIL[id];
      expect(row, id).toBeDefined();
      for (const share of [row?.art ?? -1, row?.frame ?? -1]) {
        expect(share, id).toBeGreaterThanOrEqual(0);
        expect(share, id).toBeLessThanOrEqual(1);
      }
      expect(row?.art, id).toBeGreaterThan(0);
      expect(row?.frame, id).toBeGreaterThan(0);
      expect(glareFor(id, false), id).toEqual(row);
    }
    // By area: the art and the frame take their own shares, so no finish is one flat gradient over the face.
    expect(ids.filter((id) => GLARE_BY_FOIL[id]?.art !== GLARE_BY_FOIL[id]?.frame).length).toBeGreaterThanOrEqual(ids.length - 1);
    // Each foil's pair is its own.
    expect(new Set(ids.map((id) => `${GLARE_BY_FOIL[id]?.art}/${GLARE_BY_FOIL[id]?.frame}`)).size).toBe(ids.length);
    // A face with no foil, or a foil the table does not know, takes the none row; a Radiant face none on its frame.
    expect(glareFor(undefined, false)).toEqual(GLARE_BY_FOIL[NO_FOIL_ID]);
    expect(glareFor("foil-unheard-of", false)).toEqual(GLARE_BY_FOIL[NO_FOIL_ID]);
    for (const id of [undefined, ...ids]) {
      expect(glareFor(id, true).frame, String(id)).toBe(0);
      expect(glareFor(id, true).art, String(id)).toBe(GLARE_BY_FOIL[NO_FOIL_ID]?.art);
    }
    // On the face: each foil's row lands on the root's custom properties, a Radiant face's frame at none.
    for (const id of ids) {
      if (id === NO_FOIL_ID) continue;
      writeCardPick(LEGENDARY, "foil", id);
      const cf = faceIn(LEGENDARY);
      expect(cf.getAttribute("data-cosmetic-foil"), id).toBe(id);
      expect(cf.style.getPropertyValue("--cf-glare-art"), id).toBe(String(GLARE_BY_FOIL[id]?.art));
      expect(cf.style.getPropertyValue("--cf-glare-frame"), id).toBe(String(GLARE_BY_FOIL[id]?.frame));
      cleanup();
    }
    const radiant = faceIn(LEGENDARY, { radiant: true });
    expect(radiant.style.getPropertyValue("--cf-glare-frame")).toBe("0");
    expect(radiant.style.getPropertyValue("--cf-glare-art")).toBe(String(GLARE_BY_FOIL[NO_FOIL_ID]?.art));
    cleanup();
    writeCardPick(LEGENDARY, "foil", DEFAULT_CARD_COSMETICS.foil);
    const plain = faceIn(LEGENDARY);
    expect(plain.style.getPropertyValue("--cf-glare-art")).toBe(String(GLARE_BY_FOIL[NO_FOIL_ID]?.art));
    expect(plain.style.getPropertyValue("--cf-glare-frame")).toBe(String(GLARE_BY_FOIL[NO_FOIL_ID]?.frame));
    expect(plain.style.getPropertyValue("--cf-depth-step")).toBe(`${Number(CARD_DEPTH_STEP_CQW.toFixed(2))}cqw`);
  });

  it("R3344000 the glare layers blend by area in an isolated group and add no foil of their own", () => {
    const css = sheet("depth.css");
    expect(css).toContain("mix-blend-mode: color-dodge");
    expect(css).toContain("mix-blend-mode: overlay");
    // The art's glare is dodge, the frame's overlay.
    expect(/\.cf-glare--art\s*\{[^}]*mix-blend-mode: color-dodge/.test(css)).toBe(true);
    expect(/\.cf-glare--frame\s*\{[^}]*mix-blend-mode: overlay/.test(css)).toBe(true);
    // A face at rest carries no layer that blends: a blend mode alone, even at opacity 0, prints the face's text with
    // another antialiasing (card-versions.cy.tsx's gem pixels), so the glare is drawn only while the face tilts.
    expect(/\.cf-glare\s*\{[^}]*display: none/.test(css)).toBe(true);
    expect(/\[data-tilt="on"\] \.cf-glare\s*\{[^}]*display: block/.test(css)).toBe(true);
    // Each blends inside a group of its own area's: the art frame and the scale.
    expect(/\.cf\[data-depth\] \.cf-art-frame,\s*\.cf\[data-depth\] \.cf-scale\s*\{[^}]*isolation: isolate/.test(css)).toBe(true);
    // Radiant's shimmer and every worn foil are the other sheets', and no foil rule is here.
    expect(css).not.toMatch(/data-foil|data-cosmetic-foil|data-radiant-face/);
    for (const [, selector] of css.matchAll(/([^{}]+)\{[^{}]*content: none[^{}]*\}/g)) {
      expect(selector, "only the planes' own pseudo-elements are turned off").toMatch(/\.cf-plane/);
    }
    for (const [selector] of css.matchAll(/[^{}]*::(?:after|before)[^{}]*/g)) {
      for (const each of selector.split(",")) expect(each.trim(), "a pseudo-element of the planes' art alone").toMatch(/^\.cf\[data-depth\] \.cf-plane > \.cf-art::(?:after|before)$/);
    }
  });
});

describe("R3344000 Legendary and Mythic faces in hand and inspect may break the frame", () => {
  it("R3344000 Legendary and Mythic faces in hand and inspect may break the frame, Epic and grid faces do not", () => {
    // The rule: Legendary and Mythic, in the viewer's own hand or a view that is large (the preview and the sheet).
    for (const rarity of ["Legendary", "Mythic"] as const) {
      expect(breaksFrame(rarity, true, "host"), `${rarity} hand`).toBe(true);
      expect(breaksFrame(rarity, false, "window"), `${rarity} preview`).toBe(true);
      expect(breaksFrame(rarity, false, "drag"), `${rarity} inspect`).toBe(true);
      expect(breaksFrame(rarity, false, "host"), `${rarity} Collection or board`).toBe(false);
      expect(breaksFrame(rarity, false, "off"), `${rarity} list`).toBe(false);
    }
    for (const rarity of ["Common", "Rare", "Epic", "Token", null] as const) {
      expect(breaksFrame(rarity, true, "host"), String(rarity)).toBe(false);
      expect(breaksFrame(rarity, true, "drag"), String(rarity)).toBe(false);
    }
    // On the face.
    for (const [id, options, breaks] of [
      [LEGENDARY, { ownHand: true, tilt: "host" }, true],
      [MYTHIC, { ownHand: true, tilt: "host" }, true],
      [LEGENDARY, { tilt: "drag" }, true],
      [MYTHIC, { tilt: "window" }, true],
      [LEGENDARY, { tilt: "host" }, false],
      [LEGENDARY, {}, false],
      [EPIC, { ownHand: true, tilt: "host" }, false],
      [EPIC, { tilt: "drag" }, false],
    ] as const) {
      const where = `${id} ${JSON.stringify(options)}`;
      const cf = faceIn(id, options);
      expect(cf.getAttribute("data-frame-break"), where).toBe(breaks ? "true" : null);
      expect(cf.style.getPropertyValue("--cf-frame-break"), where).toBe(breaks ? `${CARD_FRAME_BREAK_PX}px` : "");
      expect(cf.style.getPropertyValue("--cf-frame-break-scale"), where).toBe(breaks ? String(CARD_FRAME_BREAK_SCALE) : "");
      cleanup();
    }
    // A flat face breaks nothing.
    writeSettings({ reduceMotion: true });
    expect(faceIn(LEGENDARY, { ownHand: true, tilt: "host" }).hasAttribute("data-frame-break")).toBe(false);
  });

  it("R3344000 the frame-break lifts the near plane's subject over the window's top edge and no further than the face", () => {
    const css = sheet("depth.css");
    const rule = /\.cf\[data-frame-break="true"\] \.cf-plane\[data-plane="near"\]\s*\{([^}]*)\}/.exec(css)?.[1] ?? "";
    expect(rule).toContain("overflow: visible");
    expect(rule).toMatch(/clip-path: inset\(calc\(-1 \* var\(--cf-frame-break, 0px\)\) 0 0 0\)/);
    expect(css).toMatch(/\.cf-plane\[data-plane="near"\] > \.cf-art\s*\{[^}]*transform-origin: 50% 100%;[^}]*scale: var\(--cf-frame-break-scale, 1\)/);
    // The scale is a rise, and the break less than the near plane's window.
    expect(CARD_FRAME_BREAK_SCALE).toBeGreaterThan(1);
    expect(CARD_FRAME_BREAK_PX).toBeGreaterThan(0);
  });
});

describe("R3344000 the depth stylesheet's perspective, planes and spring", () => {
  it("R3344000 depth.css gives the face's parent the perspective, the tilting face preserve-3d, and the face's springs", () => {
    const css = sheet("depth.css");
    // The perspective on the parent of a face with depth is the constant's.
    const perspective = /:has\(> \.cf\[data-depth\]\)\s*\{[^}]*perspective: (\d+)px/.exec(css)?.[1];
    expect(Number(perspective)).toBe(CARD_PERSPECTIVE_PX);
    // The face declares preserve-3d as it tilts, in the gate, and not at rest.
    expect(/\.cf\[data-depth\]\[data-tilt="on"\]\s*\{[^}]*transform-style: preserve-3d/.test(css)).toBe(true);
    expect(css.slice(0, css.indexOf("@media (prefers-reduced-motion: no-preference)"))).not.toContain("transform-style");
    // The tilt turns the face and shifts each plane by its depth, springing back by a transition.
    expect(/\.cf\[data-depth\]\[data-tilt="on"\]\s*\{[^}]*transform: rotateX\(var\(--cf-tilt-rx, 0deg\)\) rotateY\(var\(--cf-tilt-ry, 0deg\)\)/.test(css)).toBe(true);
    expect(css).toContain("transition: transform var(--motion-base) var(--ease-overshoot)");
    expect(css).toMatch(/translate:\s*calc\(var\(--cf-tilt-x, 0\) \* var\(--cf-plane, 0\) \* var\(--cf-depth-step, 0px\)\)\s*calc\(var\(--cf-tilt-y, 0\) \* var\(--cf-plane, 0\) \* var\(--cf-depth-step, 0px\)\)/);
    // The planes take no pointer, no ground, no ring and no foil of their own.
    expect(/\.cf-planes\s*\{[^}]*pointer-events: none/.test(css)).toBe(true);
    expect(/\.cf-plane\s*\{[^}]*pointer-events: none/.test(css)).toBe(true);
    expect(/\.cf\[data-depth\] \.cf-plane > \.cf-art\s*\{[^}]*background-color: transparent;[^}]*box-shadow: none/.test(css)).toBe(true);
    // No raw time, curve or layer: tokens only.
    expect(css).not.toMatch(/\d(?:ms|s)\b/);
    expect(css).not.toMatch(/cubic-bezier|ease-in|ease-out\b(?!\))/);
    expect(css).not.toMatch(/z-index:\s*\d/);
  });

  it("R3344000 CardFace imports its stylesheet and the art draws the plane shape without a full-art mark", () => {
    const source = readFileSync(join(HERE, "CardFace.tsx"), "utf8");
    expect(source).toContain('import "./depth.css";');
    const cf = faceIn(MYTHIC);
    // A plane's picture is a `cf-art--plane`, never a `cf-art--full`, which marks a face printing full art.
    expect(cf.querySelectorAll(".cf-art--plane")).toHaveLength(3);
    expect(cf.querySelector(".cf-art--full")).toBeNull();
    expect(cf.hasAttribute("data-full-art")).toBe(false);
    const art = readFileSync(join(HERE, "art", "art.css"), "utf8");
    expect(art).toMatch(/\.cf-art--plane\s*\{\s*border-radius: 0;\s*box-shadow: none;\s*\}/);
  });
});

describe("R3344000 the whole, backdrop and subject parts are the pictures they were", () => {
  it("R3344000 a whole picture with no part given is the whole part's, and each part keeps its own cache key", () => {
    const def = CATALOG[LEGENDARY];
    if (def === undefined) throw new Error("no card");
    const draw = (part?: ArtPart): string => {
      const { container } = render(
        <CardDefsProvider defs={CATALOG}>
          <CardArt defId={LEGENDARY} radiant={false} tags={def.tags} type={def.type} shape="full" name={def.name} motion={artMotionFor(LEGENDARY)} {...(part === undefined ? {} : { part })} />
        </CardDefsProvider>,
      );
      const uri = container.querySelector<HTMLElement>(".cf-art")?.style.backgroundImage ?? "";
      cleanup();
      return uri;
    };
    expect(draw()).toBe(draw("whole"));
    const parts = new Set<string>(["whole", "backdrop", "subject", ...ART_PLANES].map((part) => draw(part as ArtPart)));
    // Seven parts, seven different pictures, though the far plane may match another's for a painter with nothing to draw there.
    expect(parts.size).toBeGreaterThanOrEqual(6);
    // A part never named is the whole.
    expect(draw("whole")).not.toBe(draw("backdrop"));
    expect(draw("whole")).not.toBe(draw("subject"));
  });
});
