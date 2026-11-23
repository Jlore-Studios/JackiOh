// R3394000: the scene kit's parts that Stinkyscape's early Alt art pass (#3394) added (ART.md, "Stinkyscape's early
// Alt art restaged"), so an early Starship Piece's and the Dormant Starship's Alt art draws its own piece in a scene of
// the kit and a brief's subject is no stand-in: a chrome face mask, a bare chassis frame, a beam emitter, a repeater
// gun, an orca-shaped mining hull, a pair of solar panels and a dormant hull. sceneKit.ts spreads them into `SCENE_KIT`
// after every other module, #3406's (sceneKitStinkyAlt.ts) among them. Every colour is a palette.css name, none in
// Radiant's gold window; a glow is a stack of translucent shapes, never a gradient; shapes are filled even-odd, so a
// hole is a second sub-path and no two sub-paths of one shape overlap otherwise.

import { between, blob, INKED, local, MAIN_INKED, shape, sparkle, whole, type Point } from "./kitDraw.ts";
import type { KitPart, KitShape, PartPlace, Rng } from "./scene.ts";
import { box, rounded, tilted, type Pen } from "./sceneKitStinkyAlt.ts";

// Helpers

/** A tongue of flame from `base`, `half` to either side of it, tapering `length` back along -x. */
function flameBack(f: Pen, base: Point, length: number, half: number): string {
  const [x, y] = base;
  return `M${f.p(x, y - half)}Q${f.p(x - length * 0.45, y - half)} ${f.p(x - length, y)}Q${f.p(x - length * 0.45, y + half)} ${f.p(x, y + half)}Z`;
}

/** A pointed slit about (cx, cy), `half` to either side and `open` tall: an eye hole, drawn as a sub-path. */
function slit(f: Pen, cx: number, cy: number, half: number, open: number): string {
  return `M${f.p(cx - half, cy)}Q${f.p(cx, cy - open)} ${f.p(cx + half, cy)}Q${f.p(cx, cy + open * 0.5)} ${f.p(cx - half, cy)}Z`;
}

// Objects

/** An unbroken chrome face mask, centred on the anchor: a pale oval face with two eye slits and a mouth slot cut through it, a nose ridge, a shaded cheek, a highlight band, two faint bruise-shaped reflections and a glint; about 12 wide and 16 tall at scale 1. */
function chromeMask(at: PartPlace, rng: Rng): KitShape[] {
  const f = local(at);
  const bruise = between(rng, -0.6, 0.6);
  return [
    shape(f.ellipse(0.6, 0.6, 7, 9), "slate-700", 0.3),
    shape(f.ellipse(0, 0, 6, 8) + slit(f, -2.6, -1.6, 1.8, 1.4) + slit(f, 2.6, -1.6, 1.8, 1.4) + slit(f, 0, 4.4, 1.6, 0.8), "slate-100", 1, MAIN_INKED),
    shape(`M${f.p(2.6, -7.4)}Q${f.p(7.2, 0)} ${f.p(2.6, 7.4)}Q${f.p(5, 0)} ${f.p(2.6, -7.4)}Z`, "slate-400", 0.5),
    shape(f.band([[0, -0.6], [0.2, 2.4]], 0.6), "slate-400", 0.6),
    shape(f.band([[-4.4, -5.2], [-5, -1.2], [-4.4, 3]], 0.8), "white", 0.6),
    shape(blob(f, rng, 2.6 + bruise, 2.6, 1.3, 6) + blob(f, rng, -2.8, 5.4 - bruise * 0.5, 0.8, 5), "red-300", 0.35),
    shape(sparkle(f, -3, -5.2, 1.2), "white", 0.9),
  ];
}

/** A beam emitter standing on the anchor, facing +x and aimed up a little: a turret base and dome, a long barrel with cooling fins, an emitter head and a cyan lens glowing at its tip; about 34 long and 10 tall at scale 1. */
function beamEmitter(at: PartPlace, rng: Rng): KitShape[] {
  const L = local(at);
  const f = tilted(L, -between(rng, 6, 14), 0, -4.4);
  return [
    shape(L.poly([[-6, 0], [6, 0], [4.4, -3], [-4.4, -3]]), "slate-800", 1, INKED),
    shape(L.circle(0, -4.4, 3), "slate-700", 1, INKED),
    shape(box(f, 0, -1.4, 24, 1.4), "slate-400", 1, MAIN_INKED),
    shape([6, 9, 12, 15].map((x) => box(f, x, -2.2, x + 1, 2.2)).join(""), "slate-800", 0.9),
    shape(f.band([[2, -0.8], [23, -0.8]], 0.4), "white", 0.3),
    shape(box(f, 24, -2.4, 27, 2.4), "slate-700", 1, INKED),
    shape(f.circle(28.6, 0, 5), "sky-300", 0.2),
    shape(f.circle(28.6, 0, 3), "cyan-400", 0.45),
    shape(f.circle(28.6, 0, 1.4), "white", 0.9),
  ];
}

/** A repeater gun standing on the anchor, facing +x: a mount and post, a drum housing with three or four short barrels and their collars, an ammo belt feeding it from a can behind, an orange muzzle glow; about 30 wide and 12 tall at scale 1. */
function repeaterGun(at: PartPlace, rng: Rng): KitShape[] {
  const f = local(at);
  const barrels = whole(rng, 3, 4);
  const ys = Array.from({ length: barrels }, (_, k) => -9.4 + (k * 4.4) / (barrels - 1));
  const belt: Point[] = [[-5.4, -4.6], [-7.4, -3], [-9.6, -2.6]];
  return [
    shape(f.poly([[-5, 0], [5, 0], [3.6, -2], [-3.6, -2]]), "slate-800", 1, INKED),
    shape(box(f, -1.2, -4, 1.2, -2), "slate-700", 1, INKED),
    shape(rounded(f, -6, -11, 4, -4, 1.6), "slate-400", 1, MAIN_INKED),
    shape(ys.map((y) => box(f, 4, y - 0.5, 12, y + 0.5)).join(""), "slate-700", 1, INKED),
    shape(ys.map((y) => box(f, 11.2, y - 0.8, 12.4, y + 0.8)).join(""), "slate-800", 1),
    shape(f.band(belt, 1.3), "brass-600", 1, INKED),
    shape(box(f, -14, -4, -9.4, 0), "slate-700", 1, INKED),
    shape(f.circle(14, -7.2, 4.2), "orange-400", 0.25),
    shape(f.circle(14, -7.2, 2.2), "orange-400", 0.6),
    shape(f.circle(14, -7.2, 1), "white", 0.9),
  ];
}

/** A pair of solar panels, centred on the anchor and turned a little: two wide gridded blue wings on a strut, a hub between them, a sheen across the cells and lit hinges; about 28 wide and 8 tall at scale 1. */
function solarPanels(at: PartPlace, rng: Rng): KitShape[] {
  const f = tilted(local(at), between(rng, -8, 8));
  const grid = [-11, -8.5, -6, 6, 8.5, 11].map((x) => f.band([[x, -3.6], [x, 3.6]], 0.3)).join("") + f.band([[-13.6, 0], [-3, 0]], 0.3) + f.band([[3, 0], [13.6, 0]], 0.3);
  return [
    shape(f.band([[-13.6, 0], [13.6, 0]], 0.8), "slate-400", 1, INKED),
    shape(box(f, -13.6, -3.6, -3, 3.6) + box(f, 3, -3.6, 13.6, 3.6), "blue-700", 1, MAIN_INKED),
    shape(grid, "slate-800", 0.8),
    shape(f.band([[-12.4, 3], [-7.4, -3]], 0.9) + f.band([[4.6, 3], [9.6, -3]], 0.9), "white", 0.25),
    shape(box(f, -2, -1.8, 2, 1.8), "slate-700", 1, INKED),
    shape(f.circle(-3, 0, 0.6) + f.circle(3, 0, 0.6), "cyan-400", 0.9),
  ];
}

// Craft

/** A bare ultralight chassis, centred on the anchor and facing +x: two rails braced by a zigzag of struts, bolts at the joints, an open seat, a nose cone ahead and a small engine block behind burning a short flame; about 24 long (its flame past that) and 7 tall at scale 1. */
function chassisFrame(at: PartPlace, rng: Rng): KitShape[] {
  const f = tilted(local(at), between(rng, -3, 3));
  const burn = between(rng, 4, 6);
  const struts: Point[] = [];
  for (let k = 0; k <= 6; k += 1) struts.push([-9.6 + k * 2.8, k % 2 === 0 ? -2.4 : 2.4]);
  return [
    shape(flameBack(f, [-12.6, 0], burn * 1.4, 2.2), "orange-400", 0.3),
    shape(flameBack(f, [-12.6, 0], burn, 1.3), "orange-400", 0.9),
    shape(flameBack(f, [-12.6, 0], burn * 0.5, 0.6), "white", 0.85),
    shape(f.band(struts, 0.5), "slate-700", 1, INKED),
    shape(f.band([[-11, -2.6], [8, -2.2]], 0.9) + f.band([[-11, 2.6], [8, 2.2]], 0.9), "slate-400", 1, MAIN_INKED),
    shape(box(f, -12.6, -3, -10, 3), "slate-700", 1, INKED),
    shape(f.poly([[8, -2.9], [13, 0], [8, 2.9]]), "slate-100", 1, INKED),
    shape(rounded(f, -3, -5.4, 0.4, -2.6, 0.8), "slate-800", 1, INKED),
    shape(struts.map(([x, y]) => f.circle(x, y, 0.45)).join(""), "slate-100", 1),
  ];
}

/** An orca-shaped mining ship, centred on the anchor and facing +x: a whale's body with a dark back and tail flukes, a pale belly, a dorsal fin, a row of lit ports, and its nose bay open like jaws, grinders along them and lit within; about 42 long and 20 tall (its fin) at scale 1. */
function orcaHull(at: PartPlace, rng: Rng): KitShape[] {
  const f = tilted(local(at), between(rng, -3, 3));
  const gape = between(rng, 0.8, 1.6);
  const top: Point = [19, -2.4 - gape];
  const bottom: Point = [18, 3.4 + gape * 0.6];
  const teeth = [11, 13.4, 15.8]
    .map((x) => {
      const t = (x - 9) / (top[0] - 9);
      const yt = 0.3 + (top[1] - 0.3) * t;
      const yb = 0.3 + (bottom[1] - 0.3) * t;
      return f.poly([[x - 0.8, yt], [x + 0.8, yt], [x, yt + 1.1]]) + f.poly([[x - 0.8, yb], [x, yb - 1.1], [x + 0.8, yb]]);
    })
    .join("");
  return [
    shape(f.poly([top, [9, 0.3], bottom]), "orange-400", 0.85),
    shape(f.circle(15, 0.4, 4.6), "orange-400", 0.25),
    shape(
      `M${f.p(...top)}Q${f.p(16, -8)} ${f.p(6, -7.8)}Q${f.p(-8, -7.6)} ${f.p(-17, -2)}L${f.p(-21, -5.6)}L${f.p(-22.6, -4.6)}L${f.p(-20.4, 0)}L${f.p(-22.6, 4.6)}L${f.p(-21, 5.6)}L${f.p(-17, 2)}Q${f.p(-8, 7)} ${f.p(6, 6.6)}Q${f.p(15, 6.2)} ${f.p(...bottom)}L${f.p(9, 0.3)}Z`,
      "slate-800",
      1,
      MAIN_INKED,
    ),
    shape(`M${f.p(-15, 2.4)}Q${f.p(-6, 6.4)} ${f.p(6, 6.1)}Q${f.p(13, 5.8)} ${f.p(16.4, 4)}L${f.p(10, 1.8)}Q${f.p(0, 3.2)} ${f.p(-15, 2.4)}Z`, "slate-100", 1, INKED),
    shape(f.poly([[-2, -7.4], [-6.4, -12.4], [-8.4, -11.8], [-7.2, -7.2]]), "slate-800", 1, INKED),
    shape(teeth, "slate-100", 1),
    shape([-10, -6, -2, 2].map((x) => f.circle(x, -3.2, 0.6)).join(""), "cyan-400", 0.9),
    shape(f.circle(14, 0.4, 1.6), "white", 0.6),
  ];
}

/** A dormant starship, centred on the anchor and facing +x: a long unlit hull with no flame, its cold nozzles dark, dark plates and seams along it, a dorsal and a ventral fin, a dark canopy holding a faint reflection of an open blue sky, and cable stubs trailing loose behind; about 46 long and 18 tall at scale 1. */
function dormantHull(at: PartPlace, rng: Rng): KitShape[] {
  const f = tilted(local(at), between(rng, -4, 4));
  const drift = between(rng, -1.2, 1.2);
  const plates = [-18, -9, 4].map((x) => box(f, x, -4.2, x + 4.2, -1.6)).join("") + [-14, -2].map((x) => box(f, x, 1.6, x + 5, 4.2)).join("");
  return [
    shape(f.poly([[-19, -4.6], [-21, -9.4], [-17.4, -9.4], [-12, -4.6]]) + f.poly([[-19, 4.8], [-20.6, 9], [-17, 9], [-12, 4.8]]), "slate-800", 1, INKED),
    shape(f.poly([[-21, -5.4], [-24.4, -4.4], [-24.4, 4.4], [-21, 5.4]]), "slate-800", 1, INKED),
    shape(f.circle(-24.4, -2.2, 1.1) + f.circle(-24.4, 2.2, 1.1), "black", 0.8),
    shape(f.poly([[-21, -5], [6, -5], [16, -3], [24, 0.4], [16, 3.4], [6, 5], [-21, 5]]), "slate-700", 1, MAIN_INKED),
    shape(plates + f.band([[-5.4, -4.8], [-5.4, 4.8]], 0.4) + f.band([[10.4, -3.6], [10.4, 3.8]], 0.4), "slate-800", 0.8),
    shape(`M${f.p(5, -5)}Q${f.p(9, -9)} ${f.p(15, -3.4)}L${f.p(5, -3.4)}Z`, "slate-900", 1, INKED),
    shape(`M${f.p(6.4, -5.2)}Q${f.p(9, -7.6)} ${f.p(12.6, -4.6)}L${f.p(11.4, -4.2)}Q${f.p(9, -6.2)} ${f.p(6.8, -4.4)}Z`, "sky-300", 0.35),
    shape(f.band([[-24.4, -3.6], [-28.6, -4.6 + drift], [-32, -3 + drift]], 0.5) + f.band([[-22, 4.6], [-26, 7.4 - drift], [-29.4, 7.8 - drift]], 0.5), "slate-400", 0.9),
    shape(box(f, -32.8, -3.8 + drift, -31.4, -2.2 + drift) + box(f, -30.2, 7 - drift, -28.8, 8.6 - drift), "slate-700", 1, INKED),
  ];
}

/** R3394000: the parts Stinkyscape's early Alt art pass added, by id, each with its kind. */
export const STINKY_EARLY_ALT_KIT = {
  "chrome-mask": { category: "objects", draw: chromeMask },
  "chassis-frame": { category: "craft", draw: chassisFrame },
  "beam-emitter": { category: "objects", draw: beamEmitter },
  "repeater-gun": { category: "objects", draw: repeaterGun },
  "orca-hull": { category: "craft", draw: orcaHull },
  "solar-panels": { category: "objects", draw: solarPanels },
  "dormant-hull": { category: "craft", draw: dormantHull },
} as const satisfies Readonly<Record<string, KitPart>>;
