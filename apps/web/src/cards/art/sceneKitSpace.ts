// R3403000: the scene kit's places in space (ART.md, "Scene recipes and kits"), which Stinkyscape's later cards
// (#3403) added: terrain that spans the box from its placement's height down (a ship's deck, a bulkhead, a moon's
// regolith, a crater, a hive's honeycomb, a ventilation duct, a planet's limb, an auditorium's tiers, a scrap heap)
// and a moon-sized asteroid centred on its anchor; and the buildings, rooms and fittings of stations and ships that
// stand on their anchor, hang from it or centre on it. sceneKit.ts spreads them into `SCENE_KIT`. Every colour is a
// palette.css name, none in Radiant's gold window; a glow is a stack of translucent shapes, never a gradient; shapes
// are filled even-odd, so a hole is a second sub-path and no two sub-paths of one shape overlap otherwise.

import { fmt } from "./hash.ts";
import {
  between,
  DEGREES,
  EDGE,
  ellipsePath,
  FAR_EDGE,
  groundBand,
  INKED,
  local,
  MAIN,
  MAIN_INKED,
  pt,
  rectPath,
  shape,
  sparkle,
  turned,
  whole,
  type Local,
  type Point,
} from "./kitDraw.ts";
import type { KitPart, KitShape, PartPlace, Rng } from "./scene.ts";

/** The box as a frame of its own, for terrain drawn in the box's coordinates. */
const BOX_FRAME = local({ x: 0, y: 0, s: 1, flip: 1 });

/** A terrain's size for its small details (rivets, pebbles, ribs): its scale, held to a readable range. */
function detail(s: number): number {
  return Math.min(1.6, Math.max(0.6, s));
}

/** The shapes whose paths are not empty. */
function kept(shapes: readonly KitShape[]): KitShape[] {
  return shapes.filter((item) => item.d !== "");
}

/** A length in a part's frame as an arc's radius is written. */
function radius(f: Local, length: number): string {
  return fmt(f.r(length));
}

/** A point `r` from (`cx`, `cy`) at `degrees`, clockwise from the right on screen. */
function polar(r: number, degrees: number, cx = 0, cy = 0): Point {
  return [cx + r * Math.cos(degrees * DEGREES), cy + r * Math.sin(degrees * DEGREES)];
}

/** Points along an ellipse about (`cx`, `cy`) from `from` to `to` degrees, clockwise from the right on screen. */
function arc(cx: number, cy: number, rx: number, ry: number, from: number, to: number, steps = 12): Point[] {
  const points: Point[] = [];
  for (let i = 0; i <= steps; i += 1) {
    const angle = (from + ((to - from) * i) / steps) * DEGREES;
    points.push([cx + rx * Math.cos(angle), cy + ry * Math.sin(angle)]);
  }
  return points;
}

/** The face between two runs of points, the second walked back: an arc's band, a wall's strip. */
function strip(f: Local, outer: readonly Point[], inner: readonly Point[]): string {
  return f.poly([...outer, ...[...inner].reverse()]);
}

/** A rectangle in a part's frame, from its left and top to its right and bottom. */
function rect(f: Local, left: number, top: number, right: number, bottom: number): string {
  return f.poly([
    [left, top],
    [right, top],
    [right, bottom],
    [left, bottom],
  ]);
}

/** A ring about (`dx`, `dy`), from `inner` out to `outer`. */
function ring(f: Local, dx: number, dy: number, outer: number, inner: number): string {
  return f.circle(dx, dy, outer) + f.circle(dx, dy, inner);
}

/** The front (lower) half of a flat ring about (0, `cy`), from `rx` by `ry` in by `inset` and `drop`. */
function frontHalf(f: Local, cy: number, rx: number, ry: number, inset: number, drop: number): string {
  return (
    `M${f.p(-rx, cy)}A${radius(f, rx)} ${radius(f, ry)} 0 0 ${f.sweep(0)} ${f.p(rx, cy)}` +
    `L${f.p(rx - inset, cy)}A${radius(f, rx - inset)} ${radius(f, ry - drop)} 0 0 ${f.sweep(1)} ${f.p(-(rx - inset), cy)}Z`
  );
}

/** An upper half ellipse standing on (0, `base`): a dome, `rx` wide each side and `ry` tall. */
function domePath(f: Local, base: number, rx: number, ry: number): string {
  return `M${f.p(-rx, base)}A${radius(f, rx)} ${radius(f, ry)} 0 0 ${f.sweep(1)} ${f.p(rx, base)}Z`;
}

/** A crescent on the right of an ellipse's inside, as drawn facing right: a crater's lit wall. */
function litWall(f: Local, cx: number, cy: number, rx: number, ry: number, inner: number): string {
  return (
    `M${f.p(cx, cy - ry)}A${radius(f, rx)} ${radius(f, ry)} 0 0 ${f.sweep(1)} ${f.p(cx, cy + ry)}` +
    `A${radius(f, rx * inner)} ${radius(f, ry)} 0 0 ${f.sweep(0)} ${f.p(cx, cy - ry)}Z`
  );
}

// Terrain

/** A ship's metal deck from the placement's y down across the box: plates in perspective, their seams meeting toward the placement's x, rivets, a hazard stripe band and a sheen. */
function deckFloor(at: PartPlace, rng: Rng): KitShape[] {
  const top = at.y;
  const depth = Math.max(1, FAR_EDGE - top);
  const width = FAR_EDGE - EDGE;
  const d = detail(at.s);
  const vx = at.x + between(rng, -3, 3);
  const columns = 8;
  const rows = 5;
  const xAt = (i: number, y: number): number => {
    const foot = -40 + (180 * i) / columns;
    const apex = vx + (foot - vx) * 0.3;
    return apex + ((foot - apex) * (y - top)) / depth;
  };
  const yAt = (k: number): number => top + depth * (k / rows) ** 1.6;
  let plates = "";
  for (let k = 0; k < rows; k += 1) {
    const y0 = yAt(k);
    const y1 = yAt(k + 1);
    for (let i = (k % 2) - 0; i < columns; i += 2) {
      plates += `M${pt(xAt(i, y0), y0)}L${pt(xAt(i + 1, y0), y0)}L${pt(xAt(i + 1, y1), y1)}L${pt(xAt(i, y1), y1)}Z`;
    }
  }
  let seams = "";
  for (let i = 1; i < columns; i += 1) {
    const a = xAt(i, top);
    const b = xAt(i, FAR_EDGE);
    const w = 0.35 * d;
    seams += `M${pt(a - 0.08, top)}L${pt(a + 0.08, top)}L${pt(b + w, FAR_EDGE)}L${pt(b - w, FAR_EDGE)}Z`;
  }
  let joints = "";
  let rivets = "";
  for (let k = 1; k < rows; k += 1) {
    const y = yAt(k);
    const t = (y - top) / depth;
    const thick = (0.15 + 0.5 * t) * d;
    joints += rectPath(EDGE, y - thick / 2, width, thick);
    const r = (0.15 + 0.4 * t) * d;
    const yy = y + thick / 2 + r * 2.2;
    for (let i = 0; i < columns; i += 1) {
      for (const u of [0.25, 0.75]) {
        const x = xAt(i, yy) + (xAt(i + 1, yy) - xAt(i, yy)) * u;
        if (x > EDGE && x < FAR_EDGE) rivets += ellipsePath(x, yy, r, r * 0.8);
      }
    }
  }
  const t0 = between(rng, 0.3, 0.48);
  const y0 = top + depth * t0;
  const band = Math.max(0.5, Math.min(depth * 0.12, (1.2 + 3 * t0) * d));
  const y1 = y0 + band;
  const clamp = (x: number): number => Math.min(FAR_EDGE, Math.max(EDGE, x));
  const stripe = band * 0.9;
  let stripes = "";
  for (let x = EDGE - band; x < FAR_EDGE; x += stripe * 2) {
    stripes += `M${pt(clamp(x), y1)}L${pt(clamp(x + stripe), y1)}L${pt(clamp(x + stripe + band), y0)}L${pt(clamp(x + band), y0)}Z`;
  }
  return kept([
    shape(rectPath(EDGE, top, width, depth), "slate-700", 1, MAIN),
    shape(plates, "slate-800", 0.5),
    shape(seams, "slate-900", 0.6),
    shape(joints, "slate-900", 0.55),
    shape(rectPath(EDGE, top, width, depth * 0.16), "slate-900", 0.35),
    shape(rectPath(EDGE, y0, width, band), "orange-400", 0.9),
    shape(stripes, "slate-900", 0.95),
    shape(rivets, "slate-400", 0.75),
    shape(ellipsePath(vx + between(rng, -12, 12), top + depth * 0.62, 34, depth * 0.16), "white", 0.07),
    shape(rectPath(EDGE, top, width, 0.4 * d), "slate-400", 0.55),
  ]);
}

/** A ship's inner back wall from the top of the box down to its floor at the placement's y: ribbed panels, a pipe run with couplings, rivets, a strip of status lights and a kick plate. */
function bulkhead(at: PartPlace, rng: Rng): KitShape[] {
  const floor = at.y;
  const height = Math.max(1, floor - EDGE);
  const width = FAR_EDGE - EDGE;
  const d = detail(at.s);
  const spacing = between(rng, 13, 17) * d;
  const xs: number[] = [];
  for (let x = EDGE + (between(rng, 0.2, 0.8) - 1) * spacing; x < FAR_EDGE + spacing; x += spacing) xs.push(x);
  const rib = 1.8 * d;
  const pipeH = 2.4 * d;
  const pipeY = EDGE + height * between(rng, 0.3, 0.42);
  const kick = Math.min(3.2 * d, height * 0.15);
  let ribs = "";
  let lit = "";
  let insets = "";
  let couplings = "";
  let rivets = "";
  for (const [j, x] of xs.entries()) {
    ribs += rectPath(x - rib / 2, EDGE, rib, height);
    lit += rectPath(x + rib / 2 - 0.4 * d, EDGE, 0.4 * d, height);
    for (const y of [2 + 1.2 * d, pipeY - 2.2 * d, pipeY + pipeH + 2.2 * d, floor - kick - 1.6 * d]) {
      if (y > EDGE + 0.5 && y < floor - kick) rivets += ellipsePath(x, y, 0.38 * d, 0.38 * d);
    }
    const next = xs[j + 1];
    if (next === undefined) continue;
    const l = x + rib / 2 + 1.4 * d;
    const r = next - rib / 2 - 1.4 * d;
    const panels: [number, number][] = [
      [EDGE + 2 + 1.2 * d, pipeY - 1.4 * d],
      [pipeY + pipeH + 1.4 * d, floor - kick - 1.2 * d],
    ];
    for (const [t, b] of panels) if (b - t > 1.5 * d && r - l > 1.5 * d) insets += rectPath(l, t, r - l, b - t);
    couplings += rectPath((x + next) / 2 - 1.1 * d, pipeY - 0.45 * d, 2.2 * d, pipeH + 0.9 * d);
  }
  const below = pipeY + pipeH + 3 * d;
  const ly = below + 2.4 * d < floor - kick - 0.5 ? below : pipeY - 4.4 * d;
  const lx = between(rng, 18, 64);
  const plate = 9.4 * d;
  let lights = "";
  for (let i = 0; i < 5; i += 1) if (i === 0 || rng() < 0.8) lights += rectPath(lx + (0.75 + i * 1.7) * d, ly + 0.65 * d, 0.95 * d, 0.95 * d);
  return kept([
    shape(rectPath(EDGE, EDGE, width, height), "slate-800", 1, MAIN),
    shape(insets, "slate-900", 0.35),
    shape(ribs, "slate-700", 1, INKED),
    shape(lit, "slate-400", 0.35),
    shape(rectPath(EDGE, pipeY, width, pipeH), "slate-400", 1, INKED),
    shape(rectPath(EDGE, pipeY + 0.35 * d, width, 0.5 * d), "white", 0.35),
    shape(couplings, "slate-700", 1, INKED),
    shape(rivets, "slate-400", 0.7),
    shape(rectPath(EDGE, floor - kick, width, kick), "slate-900", 0.45),
    shape(rectPath(lx - 1.2 * d, ly - 1.2 * d, plate + 2.4 * d, 4.6 * d), "cyan-400", 0.15),
    shape(rectPath(lx, ly, plate, 2.25 * d), "slate-900", 0.9),
    shape(lights, "cyan-400", 1),
  ]);
}

/** A moon's or planet's ground from the placement's y down across the box: a rolling surface and a fainter ridge behind it, shallow craters with lit walls and rims, and pebbles. */
function regolith(at: PartPlace, rng: Rng): KitShape[] {
  const top = at.y;
  const depth = Math.max(1, FAR_EDGE - top);
  const d = detail(at.s);
  const count = whole(rng, 3, 5);
  const slot = (FAR_EDGE - EDGE) / count;
  let bowls = "";
  let walls = "";
  let lips = "";
  for (let i = 0; i < count; i += 1) {
    const t = between(rng, 0.18, 0.85);
    const rx = Math.min(slot * 0.4, (2.5 + 7 * t) * d);
    const ry = Math.min(rx * 0.3, depth * 0.1);
    const cx = EDGE + slot * (i + 0.5) + between(rng, -1, 1) * (slot * 0.5 - rx * 1.15);
    const cy = top + depth * t;
    bowls += ellipsePath(cx, cy, rx, ry);
    walls += litWall(BOX_FRAME, cx, cy, rx, ry, 0.55);
    lips += `M${pt(cx - rx, cy)}A${fmt(rx)} ${fmt(ry * 1.35)} 0 0 0 ${pt(cx + rx, cy)}A${fmt(rx)} ${fmt(ry)} 0 0 1 ${pt(cx - rx, cy)}Z`;
  }
  let dark = "";
  let light = "";
  for (let k = 0; k < 8; k += 1) {
    const t = between(rng, 0.1, 0.95);
    const r = (0.35 + 1.1 * t) * d;
    const pebble = ellipsePath(EDGE + ((FAR_EDGE - EDGE) * (k + between(rng, 0.2, 0.8))) / 8, top + depth * t, r, r * 0.55);
    if (k % 2 === 0) dark += pebble;
    else light += pebble;
  }
  return [
    shape(groundBand(rng, top - 2.5 * d, 1.6 * d, 3), "slate-700", 0.55, MAIN),
    shape(groundBand(rng, top, 1.2 * d, 4), "slate-700", 1, MAIN),
    shape(rectPath(EDGE, top + depth * 0.55, FAR_EDGE - EDGE, depth * 0.5), "black", 0.15),
    shape(bowls, "black", 0.32),
    shape(walls, "white", 0.14),
    shape(lips, "white", 0.2),
    shape(dark, "black", 0.3),
    shape(light, "white", 0.22),
  ];
}

/** A bowl crater on the ground centred on the anchor: a raised rim, a dark inner bowl with its wall lit on the right, ejecta flecks round it; about 30 wide and 8 deep at scale 1. */
function crater(at: PartPlace, rng: Rng): KitShape[] {
  const f = local(at);
  const rx = 15 * between(rng, 0.92, 1.06);
  const ry = 4 * between(rng, 0.92, 1.08);
  const bx = rx * 0.78;
  const by = ry * 0.7;
  const bowlY = 0.35;
  const turn = between(rng, 0, 45);
  let flecks = "";
  for (let k = 0; k < 8; k += 1) {
    const [x, y] = polar(between(rng, 1.12, 1.4), turn + k * 45 + between(rng, -12, 12));
    const size = between(rng, 0.5, 1.1);
    flecks += f.ellipse(x * rx, y * ry + 0.3, size, size * 0.5);
  }
  return [
    shape(f.ellipse(0.8, 1.2, rx + 1.5, ry + 1.2), "black", 0.25),
    shape(flecks, "slate-400", 0.75),
    shape(f.ellipse(0, 0, rx, ry), "slate-400", 1, MAIN_INKED),
    shape(f.ellipse(0, bowlY, bx, by), "slate-900", 0.92),
    shape(litWall(f, 0, bowlY, bx, by, 0.5), "slate-400", 0.45),
    shape(f.ellipse(-bx * 0.25, bowlY + by * 0.3, bx * 0.45, by * 0.4), "black", 0.4),
    shape(f.band(arc(0, 0, rx - 0.5, ry - 0.35, -50, 60, 10), 0.6), "white", 0.3),
  ];
}

/** A hive asteroid's ground from the placement's y down across the box, pocked with rows of hexagonal cells that grow toward the viewer, a few of them glowing acid green. */
function honeycomb(at: PartPlace, rng: Rng): KitShape[] {
  const top = at.y;
  const depth = Math.max(1, FAR_EDGE - top);
  const d = detail(at.s);
  const hex = (cx: number, cy: number, rx: number, ry: number): string =>
    BOX_FRAME.poly([
      [cx - rx, cy],
      [cx - rx / 2, cy - ry],
      [cx + rx / 2, cy - ry],
      [cx + rx, cy],
      [cx + rx / 2, cy + ry],
      [cx - rx / 2, cy + ry],
    ]);
  let rims = "";
  let holes = "";
  let walls = "";
  const cells: (readonly [number, number, number, number])[] = [];
  for (const t of [0.16, 0.45, 0.8]) {
    const cy = top + depth * t;
    const rx = (2.6 + 6.5 * t) * d;
    const ry = Math.min(rx * 0.42, depth * 0.1);
    const ix = rx * 0.78;
    const iy = ry * 0.74;
    const spacing = rx * 2.25;
    for (let x = EDGE - spacing + between(rng, 0, spacing); x < FAR_EDGE + rx; x += spacing) {
      rims += hex(x, cy, rx, ry) + hex(x, cy, ix, iy);
      holes += hex(x, cy, ix, iy);
      walls += BOX_FRAME.poly([
        [x - ix, cy],
        [x - ix / 2, cy - iy],
        [x + ix / 2, cy - iy],
        [x + ix, cy],
        [x + ix / 2, cy - iy * 0.15],
        [x - ix / 2, cy - iy * 0.15],
      ]);
      if (x > 6 && x < 94) cells.push([x, cy, ix, iy]);
    }
  }
  const glowing = whole(rng, 2, 3);
  const stride = Math.max(2, Math.floor(cells.length / glowing));
  const start = whole(rng, 0, stride - 1);
  let cores = "";
  let inner = "";
  let outer = "";
  for (let j = 0; j < glowing; j += 1) {
    const cell = cells[(start + j * stride) % Math.max(1, cells.length)];
    if (cell === undefined) continue;
    const [cx, cy, rx, ry] = cell;
    cores += hex(cx, cy, rx, ry);
    inner += ellipsePath(cx, cy, rx * 1.45, ry * 1.5);
    outer += ellipsePath(cx, cy - ry * 0.4, rx * 2.2, ry * 1.9);
  }
  return kept([
    shape(groundBand(rng, top, d, 4), "slate-800", 1, MAIN),
    shape(rectPath(EDGE, top + depth * 0.6, FAR_EDGE - EDGE, depth * 0.45), "black", 0.15),
    shape(rims, "slate-700", 0.95),
    shape(holes, "slate-900", 0.92),
    shape(walls, "slate-700", 0.5),
    shape(outer, "lime-400", 0.08),
    shape(inner, "lime-400", 0.2),
    shape(cores, "lime-400", 0.6),
  ]);
}

/** The inside of a cramped square ventilation duct filling the box, its far opening a small dark square at the anchor: four walls converging on it, rib bands, a floor grille; the opening about 10 across at scale 1. */
function ventDuct(at: PartPlace, rng: Rng): KitShape[] {
  const h = 5 * at.s;
  const { x, y } = at;
  type Box = readonly [number, number, number, number];
  const box = (u: number): Box => [
    x - h + (EDGE - (x - h)) * u,
    y - h + (EDGE - (y - h)) * u,
    x + h + (FAR_EDGE - (x + h)) * u,
    y + h + (FAR_EDGE - (y + h)) * u,
  ];
  const square = ([l, t, r, b]: Box): string => `M${pt(l, t)}L${pt(r, t)}L${pt(r, b)}L${pt(l, b)}Z`;
  const [nl, nt, nr, nb] = box(0);
  const left = BOX_FRAME.poly([[EDGE, EDGE], [nl, nt], [nl, nb], [EDGE, FAR_EDGE]]);
  const right = BOX_FRAME.poly([[FAR_EDGE, EDGE], [FAR_EDGE, FAR_EDGE], [nr, nb], [nr, nt]]);
  const [shaded, lit] = at.flip === 1 ? [left, right] : [right, left];
  let ribs = "";
  let floors = "";
  for (const u of [0.1, 0.24, 0.46, 0.8]) {
    const v = u + between(rng, -0.02, 0.02);
    const outer = box(v);
    const inner = box(v - (0.02 + 0.05 * v));
    ribs += square(outer) + square(inner);
    floors += BOX_FRAME.poly([[outer[0], outer[3]], [outer[2], outer[3]], [inner[2], inner[3]], [inner[0], inner[3]]]);
  }
  const floorAt = (u: number, across: number): Point => {
    const [l, , r, b] = box(u);
    return [l + (r - l) * across, b];
  };
  const g0 = 0.52;
  const g1 = 0.72;
  const count = whole(rng, 4, 5);
  let slats = "";
  for (let k = 1; k <= count; k += 1) {
    const u = g0 + ((g1 - g0) * k) / (count + 1);
    slats += BOX_FRAME.poly([floorAt(u - 0.009, 0.34), floorAt(u - 0.009, 0.66), floorAt(u + 0.009, 0.66), floorAt(u + 0.009, 0.34)]);
  }
  const w = 0.5 * detail(at.s);
  const seams =
    BOX_FRAME.band([[EDGE, EDGE], [nl, nt]], w) +
    BOX_FRAME.band([[FAR_EDGE, EDGE], [nr, nt]], w) +
    BOX_FRAME.band([[FAR_EDGE, FAR_EDGE], [nr, nb]], w) +
    BOX_FRAME.band([[EDGE, FAR_EDGE], [nl, nb]], w);
  return [
    shape(rectPath(EDGE, EDGE, FAR_EDGE - EDGE, FAR_EDGE - EDGE), "slate-700", 1, MAIN),
    shape(BOX_FRAME.poly([[EDGE, EDGE], [FAR_EDGE, EDGE], [nr, nt], [nl, nt]]), "black", 0.32),
    shape(shaded, "black", 0.16),
    shape(lit, "white", 0.06),
    shape(seams, "slate-900", 0.5),
    shape(ribs, "slate-800", 0.95),
    shape(floors, "slate-400", 0.25),
    shape(BOX_FRAME.poly([floorAt(g0, 0.32), floorAt(g0, 0.68), floorAt(g1, 0.68), floorAt(g1, 0.32)]), "slate-900", 0.85),
    shape(slats, "slate-400", 0.5),
    shape(square(box(0)), "black", 0.92),
  ];
}

/** A planet's curved horizon from the placement's y down: a huge arc of its surface about a centre far below the placement's x, an atmosphere's glow over the edge, dark land and cloud streaks on it, its night side away from its facing. */
function planetLimb(at: PartPlace, rng: Rng): KitShape[] {
  const f = local(at);
  const R = Math.max(150, 70 / at.s);
  const reach = (Math.max(at.x - EDGE, FAR_EDGE - at.x) + 4) / at.s;
  const span = Math.asin(Math.min(1, reach / R)) / DEGREES;
  const up = -90;
  const around = (inset: number, from: number, to: number): Point[] => arc(0, R, R - inset, R - inset, from, to, 16);
  const patch = (near: number, far: number, from: number, to: number): string => strip(f, around(near, from, to), around(far, from, to));
  const streak = (inset: number, width: number): string => {
    const centre = up + span * between(rng, -0.7, 0.7);
    const half = span * between(rng, 0.2, 0.45);
    return f.band(around(inset, centre - half, centre + half), width);
  };
  let land = "";
  for (const sign of [-1, 1]) {
    const centre = up + sign * span * between(rng, 0.2, 0.75);
    const half = span * between(rng, 0.1, 0.2);
    const near = between(rng, 2.5, 5);
    land += patch(near, near + between(rng, 3, 7), centre - half, centre + half);
  }
  const thin = streak(1.6, 0.8) + streak(4.4, 0.8);
  const thick = streak(8, 1.8) + streak(13, 1.8) + streak(19, 1.8);
  return [
    shape(f.circle(0, R, R + 7), "sky-300", 0.1),
    shape(f.circle(0, R, R + 3.5), "sky-300", 0.16),
    shape(f.circle(0, R, R + 1.5), "sky-300", 0.25),
    shape(f.circle(0, R, R), "blue-600", 1, MAIN),
    shape(land, "black", 0.2),
    shape(thick, "white", 0.2),
    shape(thin, "white", 0.4),
    shape(patch(0, R * 0.45, up - span - 8, up - span * 0.35), "black", 0.25),
    shape(patch(0, R * 0.45, up - span - 8, up - span * 0.7), "black", 0.2),
    shape(f.band(around(1.4, up + span * 0.1, up + span * 0.9), 1), "white", 0.22),
    shape(ring(f, 0, R, R, R - 0.7), "sky-300", 0.55),
  ];
}

/** Tiered rows of seats curving across the box from the placement's y down, stepping toward the viewer about a central aisle of steps under the placement's x, small lights at the rows' ends by the aisle. */
function auditorium(at: PartPlace, rng: Rng): KitShape[] {
  const top = at.y;
  const depth = Math.max(1, FAR_EDGE - top);
  const d = detail(at.s);
  const cx = at.x;
  const rows = whole(rng, 4, 5);
  const aspect = between(rng, 2.6, 3.2);
  const on = (rx: number, ry: number, degrees: number): Point => [cx + rx * Math.cos(degrees * DEGREES), top + ry * Math.sin(degrees * DEGREES)];
  let seats = "";
  let fronts = "";
  let tops = "";
  const fronted: [number, number][] = [];
  let previous = 0;
  for (let k = 0; k < rows; k += 1) {
    const ry = depth * 1.05 * ((k + 1) / rows) ** 1.2;
    const thick = (ry - previous) * 0.62;
    const rx = ry * aspect;
    const iry = ry - thick;
    const irx = rx - thick * 1.2;
    const bump = thick * 0.28;
    const inner: Point[] = [];
    const m = 9 + 3 * k;
    for (let j = 0; j < m; j += 1) {
      const a0 = (180 * j) / m;
      const a1 = (180 * (j + 1)) / m;
      const sa = a0 + (a1 - a0) * 0.15;
      const sb = a1 - (a1 - a0) * 0.15;
      inner.push(on(irx, iry, a0), on(irx, iry, sa), on(irx - bump, iry - bump, sa), on(irx - bump, iry - bump, sb), on(irx, iry, sb));
    }
    inner.push(on(irx, iry, 180));
    seats += strip(BOX_FRAME, arc(cx, top, rx, ry, 0, 180, 24), inner);
    fronts += BOX_FRAME.band(arc(cx, top, rx - thick * 0.26, ry - thick * 0.22, 6, 174, 20), thick * 0.3);
    tops += BOX_FRAME.band(arc(cx, top, irx + thick * 0.12, iry + thick * 0.1, 6, 174, 20), thick * 0.1);
    fronted.push([top + ry, thick]);
    previous = ry;
  }
  const halfAt = (y: number): number => (0.8 + 6 * ((y - top) / depth)) * d;
  let steps = "";
  let glows = "";
  let lights = "";
  for (const [y, thick] of fronted) {
    for (const step of [y - thick, y - thick * 0.45]) {
      steps += rectPath(cx - halfAt(step), step, halfAt(step) * 2, Math.min(thick * 0.4, (0.25 + 0.5 * ((step - top) / depth)) * d));
    }
    const ly = y - thick * 0.4;
    const r = Math.min(thick * 0.15, (0.3 + 0.35 * ((ly - top) / depth)) * d);
    for (const side of [-1, 1]) {
      const lx = cx + side * (halfAt(ly) + r * 2);
      glows += ellipsePath(lx, ly, r * 3, r * 3);
      lights += ellipsePath(lx, ly, r, r);
    }
  }
  return [
    shape(rectPath(EDGE, top, FAR_EDGE - EDGE, depth), "slate-900", 1),
    shape(seats, "slate-800", 1, MAIN),
    shape(fronts, "black", 0.3),
    shape(tops, "white", 0.16),
    shape(BOX_FRAME.poly([[cx - halfAt(top), top], [cx + halfAt(top), top], [cx + halfAt(FAR_EDGE), FAR_EDGE], [cx - halfAt(FAR_EDGE), FAR_EDGE]]), "slate-700", 1),
    shape(steps, "slate-900", 0.6),
    shape(glows, "cyan-400", 0.22),
    shape(lights, "cyan-400", 1),
  ];
}

/** A pile of junk from the placement's y down across the box: a lumpy mound of jumbled plates, a gear and a bent pipe, a broken robot arm poking up at the placement's x and reaching toward its facing, sparks. */
function scrapHeap(at: PartPlace, rng: Rng): KitShape[] {
  const top = at.y;
  const s = at.s;
  const f = local(at);
  const slot = (FAR_EDGE - EDGE) / 5;
  let platesA = "";
  let platesB = "";
  for (let i = 0; i < 5; i += 1) {
    const g = local({ x: EDGE + slot * (i + 0.5 + between(rng, -0.25, 0.25)), y: top + between(rng, -0.5, 3) * s, s, flip: at.flip });
    const w = Math.min(between(rng, 6, 10), (slot * 0.85) / s) / 2;
    const h = between(rng, 1.1, 1.7);
    const tilt = between(rng, -30, 30);
    const corners = ([[-w, -h], [w, -h], [w, h], [-w, h]] as const).map((p) => turned(p, tilt));
    const hole = turned([w * 0.5, 0], tilt);
    const plate = g.poly(corners) + g.circle(hole[0], hole[1], Math.min(0.5, h * 0.4));
    if (i % 2 === 0) platesA += plate;
    else platesB += plate;
  }
  const side = rng() < 0.5 ? -1 : 1;
  const gear = local({ x: Math.min(92, Math.max(8, at.x + side * between(rng, 16, 28) * s)), y: top + between(rng, -1, 1.5) * s, s, flip: at.flip });
  const spin = between(rng, 0, 40);
  const teeth: Point[] = [];
  for (let i = 0; i < 9; i += 1) {
    const a = spin + i * 40;
    teeth.push(polar(3.6, a - 13), polar(4.6, a - 7), polar(4.6, a + 7), polar(3.6, a + 13));
  }
  const pipe = local({ x: Math.min(95, Math.max(5, at.x - side * between(rng, 12, 22) * s)), y: top, s, flip: at.flip });
  const lean = between(rng, -1, 1);
  const elbow: Point = [-1.5 + lean, -9];
  const wrist: Point = [5 + lean, -15 + between(rng, -1, 1)];
  const dir = Math.atan2(wrist[1] - elbow[1], wrist[0] - elbow[0]) / DEGREES;
  const claw = (x: number, y: number): Point => {
    const [tx, ty] = turned([x, y], dir);
    return [wrist[0] + tx, wrist[1] + ty];
  };
  const jaw: Point[] = [
    [0.3, -1.3],
    [1.9, -1.3],
    [3.9, -2.8],
    [4.9, -1.8],
    [2.7, -0.45],
    [2.7, 0.45],
    [4.9, 1.8],
    [3.9, 2.8],
    [1.9, 1.3],
    [0.3, 1.3],
  ];
  const spark = claw(5.8, -0.2);
  const fizz: Point = [elbow[0] - 2.2, elbow[1] - 1.5];
  return [
    shape(groundBand(rng, top, 2.6 * s, 7), "slate-700", 1, MAIN),
    shape(platesB, "slate-800", 1, INKED),
    shape(platesA, "slate-400", 1, INKED),
    shape(gear.poly(teeth) + gear.circle(0, 0, 1.5), "slate-400", 1, INKED),
    shape(pipe.band([[0, 2], [0.4, -3], [2.4, -6.4], [6.4, -8]], 1.7), "slate-400", 1, INKED),
    shape(pipe.circle(6.4, -8, 0.7), "black", 0.7),
    shape(groundBand(rng, top + 9 * s, 2 * s, 5), "black", 0.25),
    shape(f.band([[0, 3], elbow, wrist], 2.2) + f.poly(jaw.map(([x, y]) => claw(x, y))), "slate-800", 1, INKED),
    shape(f.circle(...elbow, 1.2) + f.circle(...claw(0.15, 0), 0.75), "slate-400", 0.9),
    shape(f.circle(...spark, 2.6) + f.circle(...fizz, 1.6), "orange-400", 0.25),
    shape(sparkle(f, ...spark, 1.8) + sparkle(f, ...fizz, 1.2), "orange-400", 1),
  ];
}

/** A moon-sized asteroid centred on the anchor: a craggy rim, craters, a scowling cliff face (a heavy brow, two shadowed eye hollows, a frowning crack of a mouth) and a jagged crack glowing with its violet core; about 60 across at scale 1. */
function giantAsteroid(at: PartPlace, rng: Rng): KitShape[] {
  const f = local(at);
  const n = 16;
  const rim: Point[] = [];
  for (let i = 0; i < n; i += 1) {
    const reach = 30 * (i % 2 === 0 ? between(rng, 0.92, 1.04) : between(rng, 0.84, 0.95));
    rim.push(polar(reach, (i * 360) / n + between(rng, -5, 5)));
  }
  const scaled = (indexes: readonly number[], k: number): Point[] =>
    indexes.map((i) => {
      const [x, y] = rim[i % n] ?? [0, 0];
      return [x * k, y * k];
    });
  const shaded = [5, 6, 7, 8, 9, 10, 11];
  const lit = [14, 15, 16, 17, 18];
  const craters: readonly (readonly [number, number, number])[] = [
    [9, -18, 3.2],
    [19, 6, 2.8],
    [-4, 19.5, 3],
    [-17, 13, 2.2],
  ];
  let pits = "";
  let shine = "";
  for (const [x0, y0, r] of craters) {
    const x = x0 + between(rng, -1, 1);
    const y = y0 + between(rng, -1, 1);
    pits += f.ellipse(x, y, r, r * 0.75);
    shine += litWall(f, x, y, r, r * 0.75, 0.5);
  }
  const brow = f.poly([
    [-17, -9],
    [-9, -8.6],
    [-1.5, -4.2],
    [1.5, -4.2],
    [9, -8.6],
    [17, -9],
    [16, -6],
    [9, -5.2],
    [1.5, -1.2],
    [-1.5, -1.2],
    [-9, -5.2],
    [-16, -6],
  ]);
  const browLit =
    f.band([[-16.5, -8.6], [-9, -8.2], [-2, -4]], 0.6) +
    f.band([[2, -4], [9, -8.2], [16.5, -8.6]], 0.6);
  const eye = (sd: number): string =>
    f.poly([
      [sd * 14, -5.4],
      [sd * 9, -4.5],
      [sd * 3.5, -1],
      [sd * 4.5, 2],
      [sd * 9, 3],
      [sd * 13, 1.5],
    ]);
  const frown: Point[] = [
    [-11, 14.5],
    [-7.5, 11.6],
    [-5, 12.3],
    [-2, 9.9],
    [1, 10.7],
    [4, 9.7],
    [6.6, 11.9],
    [8.6, 11.3],
    [11, 14.5],
  ];
  const mouth = frown.map(([x, y]): Point => [x, y + between(rng, -0.4, 0.4)]);
  const crack = (
    [
      [-12, -21],
      [-15, -17],
      [-13, -13],
      [-18, -9.5],
      [-16.5, -4],
      [-21, 1],
      [-19, 6.5],
    ] as const
  ).map(([x, y]): Point => [x + between(rng, -0.8, 0.8), y]);
  return [
    shape(f.poly(rim), "slate-700", 1, MAIN_INKED),
    shape(strip(f, scaled(shaded, 1), scaled(shaded, 0.68)), "black", 0.3),
    shape(strip(f, scaled(lit, 1), scaled(lit, 0.86)), "slate-400", 0.35),
    shape(pits, "slate-800", 0.85),
    shape(brow, "slate-700", 1, MAIN_INKED),
    shape(shine + browLit, "slate-400", 0.4),
    shape(eye(-1) + eye(1), "black", 0.7),
    shape(f.band(mouth, 1.5), "black", 0.75),
    shape(f.band(crack, 4.4), "purple-400", 0.14),
    shape(f.band(crack, 2.2), "purple-400", 0.3),
    shape(f.band(crack, 0.9), "purple-400", 1),
  ];
}

// Architecture

/** A wide observation window centred on the anchor: a thick frame with two mullions, its three panes left open on whatever lies behind, a faint glint across the glass, a sill; about 40 wide and 22 tall at scale 1. */
function viewport(at: PartPlace, rng: Rng): KitShape[] {
  const f = local(at);
  const hw = 20;
  const hh = 11;
  const frame = 2.4;
  const mullion = 1.6;
  const l0 = -hw + frame;
  const t0 = -hh + frame;
  const b0 = hh - frame;
  const pane = (2 * (hw - frame) - 2 * mullion) / 3;
  const slope = (b0 - t0) * 0.3;
  let holes = "";
  let bevels = "";
  let glints = "";
  for (let i = 0; i < 3; i += 1) {
    const l = l0 + i * (pane + mullion);
    const r = l + pane;
    holes += rect(f, l, t0, r, b0);
    bevels += rect(f, l, t0, r, b0) + rect(f, l + 0.6, t0 + 0.6, r - 0.6, b0 - 0.6);
    const g = l + between(rng, 0.3, 2.6);
    glints +=
      f.poly([[g, b0 - 0.7], [g + 1.4, b0 - 0.7], [g + 1.4 + slope, t0 + 0.7], [g + slope, t0 + 0.7]]) +
      f.poly([[g + 2.3, b0 - 0.7], [g + 2.8, b0 - 0.7], [g + 2.8 + slope, t0 + 0.7], [g + 2.3 + slope, t0 + 0.7]]);
  }
  let bolts = "";
  for (const x of [-hw + 1.2, l0 + pane + mullion / 2, l0 + 2 * pane + 1.5 * mullion, hw - 1.2]) bolts += f.circle(x, -hh + 1.2, 0.45) + f.circle(x, hh - 1.2, 0.45);
  return [
    shape(rect(f, -hw - 1.2, hh, hw + 1.2, hh + 1.5), "slate-800", 1, INKED),
    shape(rect(f, -hw, -hh, hw, hh) + holes, "slate-700", 1, MAIN_INKED),
    shape(bevels, "slate-400", 0.45),
    shape(f.band([[-hw + 0.5, -hh + 0.5], [hw - 0.5, -hh + 0.5], [hw - 0.5, hh - 0.5]], 0.6), "white", 0.18),
    shape(bolts, "slate-400", 0.8),
    shape(glints, "white", 0.1),
  ];
}

/** A round porthole centred on the anchor: a thick bolted ring, its middle left open on whatever lies behind, a glint on the glass; about 10 across at scale 1. */
function porthole(at: PartPlace, rng: Rng): KitShape[] {
  const f = local(at);
  const count = whole(rng, 6, 8);
  const turn = between(rng, 0, 360 / count);
  let bolts = "";
  for (let i = 0; i < count; i += 1) bolts += f.circle(...polar(4.2, turn + (i * 360) / count), 0.38);
  return [
    shape(f.circle(0, 0, 2.9), "sky-300", 0.08),
    shape(ring(f, 0, 0, 5, 3.4), "slate-400", 1, MAIN_INKED),
    shape(ring(f, 0, 0, 3.4, 2.9), "slate-800", 0.85),
    shape(bolts, "slate-700", 1),
    shape(f.band(arc(0, 0, 4.55, 4.55, -60, 30, 8), 0.4), "white", 0.35),
    shape(f.band(arc(0, 0, 2.1, 2.1, -75, -25, 6), 0.45), "white", 0.4),
  ];
}

/** A station bench standing on the anchor: a seat slab on two pedestal legs, a back rail, a strip light under its front edge; about 18 wide and 6 tall at scale 1. */
function bench(at: PartPlace, rng: Rng): KitShape[] {
  const f = local(at);
  const half = 9 * between(rng, 0.94, 1.04);
  const leg = (x: number, dir: number): string =>
    f.poly([
      [x - 0.7, -2.2],
      [x + 0.7, -2.2],
      [x + 0.9 + dir * 0.2, 0],
      [x - 0.9 + dir * 0.2, 0],
    ]);
  return [
    shape(f.ellipse(0, 0.2, half, 0.8), "black", 0.25),
    shape(leg(-half + 2.6, -1) + leg(half - 2.6, 1), "slate-800", 1, INKED),
    shape(
      rect(f, -half, -3.4, half, -2.2) + rect(f, -half + 0.6, -6.2, half - 0.6, -5.3) + rect(f, -half + 2.3, -5.3, -half + 3, -3.4) + rect(f, half - 3, -5.3, half - 2.3, -3.4),
      "slate-700",
      1,
      MAIN_INKED,
    ),
    shape(f.band([[-half, -3.25], [half, -3.25]], 0.3) + f.band([[-half + 0.6, -6.05], [half - 0.6, -6.05]], 0.25), "white", 0.25),
    shape(rect(f, -half, -2.65, half, -2.2), "black", 0.3),
    shape(rect(f, -half + 1, -2.1, half - 1, -1.8), "cyan-400", 0.75),
  ];
}

/** A makeshift boxing ring standing on the anchor in three-quarter view: a canvas platform on a dark apron, four corner posts, three ropes round it and corner pads; about 50 wide and 20 tall at scale 1. */
function boxingRing(at: PartPlace, rng: Rng): KitShape[] {
  const f = local(at);
  const fl = -24;
  const fr = 20;
  const bl = -19;
  const br = 25;
  const front = -4;
  const back = -9;
  const frontPost = 13;
  const backPost = 10.5;
  let near = "";
  let far = "";
  for (const t of [0.32, 0.6, 0.88]) {
    const yf = front - frontPost * t;
    const yb = back - backPost * t;
    near += f.band([[fl, yf], [(fl + fr) / 2, yf + between(rng, 0, 0.6)], [fr, yf], [br, yb]], 0.8);
    far += f.band([[fl, yf], [bl, yb], [(bl + br) / 2, yb + between(rng, 0, 0.4)], [br, yb]], 0.65);
  }
  const pad = (x: number, y: number, w: number): string => rect(f, x - w, y - 1.3, x + w, y + 1.3);
  const padF = front - frontPost * 0.88;
  const padB = back - backPost * 0.88;
  const post = (x: number, foot: number, head: number, width: number): string => f.band([[x, foot], [x, head]], width);
  return [
    shape(f.ellipse(0.5, 0.4, 27, 1.6), "black", 0.25),
    shape(rect(f, fl, front, fr, 0) + f.poly([[fr, front], [br, back], [br, back + 4], [fr, 0]]), "slate-800", 1, INKED),
    shape(f.poly([[fl, front], [fr, front], [br, back], [bl, back]]), "slate-400", 1, INKED),
    shape(f.ellipse(between(rng, -6, 6), -6.5, 11, 1.4), "white", 0.12),
    shape(post(bl, back, back - backPost - 0.6, 1) + post(br, back + 4, back - backPost - 0.6, 1), "slate-700", 1, INKED),
    shape(far, "red-600", 0.9, MAIN),
    shape(near, "red-600", 1, MAIN),
    shape(post(fl, 0, front - frontPost - 0.6, 1.3) + post(fr, 0, front - frontPost - 0.6, 1.3), "slate-700", 1, INKED),
    shape(pad(fl, padF, 0.95) + pad(br, padB, 0.8), "blue-600", 1, INKED),
    shape(pad(fr, padF, 0.95) + pad(bl, padB, 0.8), "slate-100", 1, INKED),
  ];
}

/** A metal walkway across the whole box at the placement's y: a grated deck on a girder, railing posts and two rails above it, cross-braced trusses below; its posts about 12 apart and its top rail about 10 above the deck at scale 1. */
function catwalk(at: PartPlace, rng: Rng): KitShape[] {
  const y = at.y;
  const s = at.s;
  const width = FAR_EDGE - EDGE;
  const spacing = Math.max(6, between(rng, 11, 13) * s);
  const xs: number[] = [];
  for (let x = at.x - Math.ceil((at.x - EDGE) / spacing) * spacing; x <= FAR_EDGE + spacing; x += spacing) xs.push(x);
  let grate = "";
  for (let x = EDGE; x < FAR_EDGE; x += Math.max(0.8, 1.4 * s)) {
    grate += BOX_FRAME.poly([
      [x, y - 0.15 * s],
      [x + 0.5 * s, y - 0.15 * s],
      [x + 0.85 * s, y - 1.05 * s],
      [x + 0.35 * s, y - 1.05 * s],
    ]);
  }
  let railing = rectPath(EDGE, y - 9.6 * s, width, 0.8 * s) + rectPath(EDGE, y - 5.6 * s, width, 0.6 * s);
  let truss = rectPath(EDGE, y + 7 * s, width, 0.7 * s);
  let down = "";
  let up = "";
  let lights = "";
  for (const [i, x] of xs.entries()) {
    railing += rectPath(x - 0.4 * s, y - 8.8 * s, 0.8 * s, 3.2 * s) + rectPath(x - 0.4 * s, y - 5 * s, 0.8 * s, 3.8 * s);
    truss += rectPath(x - 0.35 * s, y + 1.6 * s, 0.7 * s, 5.4 * s);
    const next = xs[i + 1];
    if (next === undefined) continue;
    const a = x + 0.35 * s;
    const b = next - 0.35 * s;
    down += BOX_FRAME.band([[a, y + 1.8 * s], [b, y + 6.8 * s]], 0.5 * s);
    up += BOX_FRAME.band([[a, y + 6.8 * s], [b, y + 1.8 * s]], 0.5 * s);
    lights += ellipsePath((x + next) / 2, y + 0.8 * s, 0.4 * s, 0.4 * s);
  }
  return [
    shape(down, "slate-800", 0.9),
    shape(up, "slate-800", 0.9),
    shape(truss, "slate-700", 1, INKED),
    shape(rectPath(EDGE, y - 1.2 * s, width, 1.2 * s), "slate-700", 1),
    shape(grate, "slate-900", 0.6),
    shape(rectPath(EDGE, y, width, 1.6 * s), "slate-800", 1, INKED),
    shape(railing, "slate-400", 1, MAIN_INKED),
    shape(rectPath(EDGE, y - 9.6 * s, width, 0.25 * s), "white", 0.35),
    shape(lights, "cyan-400", 0.9),
  ];
}

/** A round airlock hatch centred on the anchor, standing open: a heavy ring frame banded with hazard chevrons, its door swung aside on hinge arms toward its facing, the opening a dark void; the frame about 20 across at scale 1. */
function airlock(at: PartPlace, rng: Rng): KitShape[] {
  const f = local(at);
  const turn = between(rng, 0, 30);
  let chevrons = "";
  for (let i = 0; i < 12; i += 1) {
    const a = turn + i * 30;
    chevrons += f.poly([polar(7.9, a), polar(7.9, a + 12), polar(9.3, a + 20), polar(9.3, a + 8)]);
  }
  const swing = between(rng, 2.6, 3.6);
  const dx = 10 + swing;
  return [
    shape(f.circle(0, 0, 7.2), "black", 0.9),
    shape(ring(f, 0, 0, 10, 7.2), "slate-400", 1, MAIN_INKED),
    shape(ring(f, 0, 0, 9.4, 7.8), "slate-900", 1),
    shape(chevrons, "orange-400", 1),
    shape(f.band(arc(0, 0, 9.7, 9.7, -60, 40, 10), 0.45), "white", 0.3),
    shape(f.band([[9.2, -4.6], [dx, -4.6]], 1.4) + f.band([[9.2, 4.6], [dx, 4.6]], 1.4), "slate-800", 1, INKED),
    shape(f.ellipse(dx, 0, swing, 9.6), "slate-700", 1, INKED),
    shape(
      f.ellipse(dx, 0, swing * 0.7, 7.4) + f.ellipse(dx, 0, swing * 0.5, 6.6) + f.ellipse(dx, 0, swing * 0.32, 2.4) + f.ellipse(dx, 0, swing * 0.18, 1.6),
      "slate-400",
      0.55,
    ),
  ];
}

/** A tall scaffold tower standing on the anchor: two lattice legs with cross braces, a platform partway up and one at the top with rails, a red beacon on its mast; about 10 wide and 40 tall at scale 1. */
function gantry(at: PartPlace, rng: Rng): KitShape[] {
  const f = local(at);
  const height = between(rng, 33, 36);
  const t = -height;
  const legAt = (side: number, y: number): number => side * (4.5 - (1.2 * -y) / height);
  const cells = 6;
  const cell = (height - 2.5) / cells;
  let down = "";
  let up = "";
  for (let c = 0; c < cells; c += 1) {
    const y0 = -1.5 - c * cell;
    const y1 = y0 - cell;
    down += f.band([[legAt(-1, y1) + 0.4, y1], [legAt(1, y0) - 0.4, y0]], 0.5);
    up += f.band([[legAt(-1, y0) + 0.4, y0], [legAt(1, y1) - 0.4, y1]], 0.5);
  }
  const mid = t * between(rng, 0.46, 0.56);
  const rails =
    f.band([[-6, t - 3.2], [6, t - 3.2]], 0.4) +
    f.band([[-5.6, t - 1], [-5.6, t - 3]], 0.4) +
    f.band([[5.6, t - 1], [5.6, t - 3]], 0.4) +
    f.band([[0, t - 3.4], [0, t - 5]], 0.4) +
    f.band([[3.4, mid - 3.2], [6.4, mid - 3.2]], 0.4) +
    f.band([[6, mid - 0.9], [6, mid - 3]], 0.4);
  return [
    shape(down, "slate-400", 0.9, MAIN),
    shape(up, "slate-400", 0.9, MAIN),
    shape(f.band([[legAt(-1, 0), 0], [legAt(-1, t), t]], 1.1) + f.band([[legAt(1, 0), 0], [legAt(1, t), t]], 1.1), "slate-400", 1, MAIN_INKED),
    shape(f.band([[legAt(1, 0) + 0.3, -0.5], [legAt(1, t) + 0.3, t + 0.5]], 0.25), "white", 0.35),
    shape(rect(f, -6.5, mid - 0.9, 6.5, mid) + rect(f, -6, t - 1, 6, t), "slate-700", 1, INKED),
    shape(rails, "slate-400", 0.9, MAIN),
    shape(rect(f, -6, -0.8, -3, 0.4) + rect(f, 3, -0.8, 6, 0.4), "slate-800", 1, INKED),
    shape(f.circle(0, t - 5.8, 4.2), "red-500", 0.12),
    shape(f.circle(0, t - 5.8, 2), "red-500", 0.3),
    shape(f.circle(0, t - 5.8, 0.85), "red-400", 1),
  ];
}

/** A warp gate centred on the anchor: a thick plated ring, its middle left open, segment seams with cyan lights between them, an inner glow, two struts below; about 36 across at scale 1 (its struts reach 26 below the anchor). */
function warpGate(at: PartPlace, rng: Rng): KitShape[] {
  const f = local(at);
  const outer = 18;
  const inner = 13.4;
  const count = whole(rng, 6, 8);
  const turn = between(rng, 0, 360 / count);
  let seams = "";
  let glows = "";
  let lights = "";
  for (let i = 0; i < count; i += 1) {
    const a = turn + (i * 360) / count;
    seams += f.band([polar(inner + 0.1, a), polar(outer - 0.1, a)], 0.5);
    const light = polar((outer + inner) / 2, a + 180 / count);
    glows += f.circle(...light, 1.8);
    lights += f.circle(...light, 0.75);
  }
  return [
    shape(f.band([polar(15.5, 122), polar(28, 116)], 2.6) + f.band([polar(15.5, 58), polar(28, 64)], 2.6), "slate-800", 1, INKED),
    shape(ring(f, 0, 0, outer, inner), "slate-700", 1, MAIN_INKED),
    shape(seams, "slate-900", 0.7),
    shape(ring(f, 0, 0, inner + 0.9, inner), "slate-400", 0.5),
    shape(f.band(arc(0, 0, outer - 0.7, outer - 0.7, -80, 10, 12), 0.6), "white", 0.22),
    shape(ring(f, 0, 0, inner, inner - 1.6), "cyan-400", 0.28),
    shape(ring(f, 0, 0, inner - 1.6, inner - 4.2), "cyan-400", 0.12),
    shape(glows, "cyan-400", 0.3),
    shape(lights, "cyan-400", 1),
  ];
}

/** A launch and landing pad, its deck a flat disc in perspective centred on the anchor: hazard markings and landing lights round its rim, a painted ring and H, a blast deflector curving up behind it; about 40 wide and 8 deep at scale 1 (its deflector rises 4 more). */
function launchPad(at: PartPlace, rng: Rng): KitShape[] {
  const f = local(at);
  const rx = 20;
  const ry = 4;
  const thick = 1.6;
  const backRim = arc(0, 0, rx - 0.6, ry - 0.15, 195, 345, 14);
  const lip = backRim.map(([x, y]): Point => [x, y - 1.2 - 2.4 * (1 - (x / rx) ** 2)]);
  const on = (k: number, degrees: number): Point => [rx * k * Math.cos(degrees * DEGREES), ry * k * Math.sin(degrees * DEGREES)];
  const turn = between(rng, 0, 22.5);
  let stripes = "";
  for (let i = 0; i < 16; i += 1) {
    const a = turn + i * 22.5;
    stripes += f.poly([on(0.86, a), on(0.86, a + 9), on(0.95, a + 13), on(0.95, a + 4)]);
  }
  let glows = "";
  let lights = "";
  for (let i = 0; i < 12; i += 1) {
    const a = i * 30;
    const x = (rx - 0.4) * Math.cos(a * DEGREES);
    const y = (ry - 0.1) * Math.sin(a * DEGREES);
    glows += f.circle(x, y, 1.2);
    lights += f.circle(x, y, 0.42);
  }
  const letter = f.poly([
    [-4.2, -1.1],
    [-2.6, -1.1],
    [-2.6, -0.2],
    [2.6, -0.2],
    [2.6, -1.1],
    [4.2, -1.1],
    [4.2, 1.1],
    [2.6, 1.1],
    [2.6, 0.2],
    [-2.6, 0.2],
    [-2.6, 1.1],
    [-4.2, 1.1],
  ]);
  const side = `M${f.p(-rx, 0)}A${radius(f, rx)} ${radius(f, ry)} 0 0 ${f.sweep(0)} ${f.p(rx, 0)}L${f.p(rx, thick)}A${radius(f, rx)} ${radius(f, ry)} 0 0 ${f.sweep(1)} ${f.p(-rx, thick)}Z`;
  return [
    shape(strip(f, backRim, lip), "slate-400", 1, INKED),
    shape(side, "slate-800", 1, INKED),
    shape(f.ellipse(0, 0, rx, ry), "slate-700", 1, MAIN_INKED),
    shape(f.ellipse(2, -0.6, 10, 1.6), "white", 0.07),
    shape(f.ellipse(0, 0, 13, 2.6) + f.ellipse(0, 0, 12, 2.4), "white", 0.4),
    shape(letter, "white", 0.6),
    shape(stripes, "orange-400", 0.85),
    shape(glows, "cyan-400", 0.25),
    shape(lights, "cyan-400", 1),
  ];
}

/** A slender glass observation spire standing on the anchor: a tapering glass shaft in a pale frame lit on its right edge, struts, a viewing cabin with a band of lit windows near the top, a needle; about 10 wide and 50 tall at scale 1. */
function glassSpire(at: PartPlace, rng: Rng): KitShape[] {
  const f = local(at);
  const h = between(rng, 38, 41);
  const b0 = 3.2;
  const b1 = 1.4;
  const halfAt = (y: number): number => b0 + ((b1 - b0) * (-y - 2)) / (h - 2);
  let frame = f.band([[-b0 + 0.25, -2], [-b1 + 0.25, -h]], 0.5) + f.band([[b0 - 0.25, -2], [b1 - 0.25, -h]], 0.5);
  for (let k = 1; k <= 5; k += 1) {
    const y = -2 - ((h - 2) * k) / 6;
    const w = halfAt(y) - 0.52;
    frame += f.band([[-w, y], [w, y]], 0.35);
  }
  frame += f.poly([[-0.45, -h - 6], [0.45, -h - 6], [0, -h - 10.5]]);
  return [
    shape(f.poly([[-b0, -2], [b0, -2], [b1, -h], [-b1, -h]]), "slate-100", 0.28),
    shape(f.poly([[b0 * 0.3, -2], [b0 * 0.55, -2], [b1 * 0.55, -h], [b1 * 0.3, -h]]), "white", 0.14),
    shape(
      f.band([[-b1 - 0.1, -h + 4.5], [-5.2, -h - 0.6]], 0.55) +
        f.band([[b1 + 0.1, -h + 4.5], [5.2, -h - 0.6]], 0.55) +
        f.band([[-halfAt(-8) + 0.1, -8], [-5, -1.8]], 0.6) +
        f.band([[halfAt(-8) - 0.1, -8], [5, -1.8]], 0.6),
      "slate-400",
      0.95,
    ),
    shape(frame, "slate-100", 0.85, MAIN),
    shape(f.band([[b0 - 0.15, -2.2], [b1 - 0.15, -h]], 0.3), "white", 0.7),
    shape(
      f.poly([[-5, 0], [5, 0], [4.2, -2], [-4.2, -2]]) + f.ellipse(0, -h - 0.8, 6, 1.4) + f.poly([[-3.8, -h - 5.4], [3.8, -h - 5.4], [3, -h - 6], [-3, -h - 6]]),
      "slate-700",
      1,
      INKED,
    ),
    shape(f.poly([[-4.6, -h - 1.4], [4.6, -h - 1.4], [3.6, -h - 5.4], [-3.6, -h - 5.4]]), "slate-100", 0.4),
    shape(f.poly([[-4.3, -h - 2.6], [4.3, -h - 2.6], [3.95, -h - 3.8], [-3.95, -h - 3.8]]), "cyan-400", 0.75),
    shape(f.ellipse(0, -h - 3.4, 7.5, 4.2), "cyan-400", 0.12),
  ];
}

/** A pneumatic tube standing on the anchor: a glass tube rising about 30 from a funnel intake mouth and bending over toward its facing, metal bands round it, a status light; about 12 wide at its foot. */
function intakeTube(at: PartPlace, rng: Rng): KitShape[] {
  const f = local(at);
  const rise = between(rng, 20, 23);
  const bend = 7;
  const reach = between(rng, 5, 8);
  const path: Point[] = [
    [0, -5],
    [0, -rise],
  ];
  for (let k = 1; k <= 6; k += 1) path.push(polar(bend, 180 + k * 15, bend, -rise));
  path.push([bend + reach, -rise - bend]);
  const offset = (by: number): Point[] =>
    path.map(([x, y], i): Point => {
      const [px, py] = path[Math.max(0, i - 1)] ?? [x, y];
      const [nx, ny] = path[Math.min(path.length - 1, i + 1)] ?? [x, y];
      const length = Math.hypot(nx - px, ny - py) || 1;
      return [x + (-(ny - py) / length) * by, y + ((nx - px) / length) * by];
    });
  const collar = ([x, y]: Point, [nx, ny]: Point, half: number, width: number): string =>
    f.band([[x - nx * half, y - ny * half], [x + nx * half, y + ny * half]], width);
  const bands =
    collar([0, -9], [1, 0], 3.1, 1.3) +
    collar([0, -rise + 3], [1, 0], 3.1, 1.3) +
    collar(polar(bend, 225, bend, -rise), polar(1, 225), 3.1, 1.3) +
    collar([bend + reach - 0.6, -rise - bend], [0, 1], 3.4, 1.6);
  return [
    shape(f.band(path, 5), "slate-100", 0.22),
    shape(f.band(offset(1.1), 0.5), "white", 0.3),
    shape(f.band(offset(2.3), 0.45) + f.band(offset(-2.3), 0.45), "slate-100", 0.65),
    shape(f.poly([[-2.6, -5], [2.6, -5], [5.8, -0.6], [-5.8, -0.6]]), "slate-700", 1, INKED),
    shape(f.poly([[-3.6, -1.2], [3.6, -1.2], [2.4, -3.6], [-2.4, -3.6]]), "black", 0.75),
    shape(rect(f, -3, -2.25, 3, -1.9) + rect(f, -2.6, -3.05, 2.6, -2.7), "slate-400", 0.6),
    shape(bands + rect(f, -6.2, -0.6, 6.2, 0), "slate-400", 1, MAIN_INKED),
    shape(f.circle(0, -4.3, 0.4), "cyan-400", 1),
  ];
}

/** A row of three or four fluorescent strip lights hanging from the anchor on short rods across about 60, each a bright tube in a housing with a soft cone of light below it about 22 long. */
function ceilingLights(at: PartPlace, rng: Rng): KitShape[] {
  const f = local(at);
  const count = whole(rng, 3, 4);
  const pitch = 60 / count;
  const half = Math.min(5.5, pitch * 0.34);
  const drop = between(rng, 2.5, 4);
  const length = between(rng, 20, 26);
  const spread = pitch * 0.47;
  const y = drop + 1.9;
  let rods = "";
  let housings = "";
  let tubes = "";
  let halos = "";
  let outer = "";
  let inner = "";
  for (let i = 0; i < count; i += 1) {
    const x = -30 + pitch * (i + 0.5);
    rods += f.band([[x - half * 0.7, 0], [x - half * 0.7, drop]], 0.3) + f.band([[x + half * 0.7, 0], [x + half * 0.7, drop]], 0.3);
    housings += rect(f, x - half - 0.5, drop, x + half + 0.5, drop + 1.1);
    tubes += rect(f, x - half, drop + 1.1, x + half, y);
    halos += f.ellipse(x, drop + 1.5, half + 2, 1.9);
    outer += f.poly([[x - half, y], [x + half, y], [x + spread, y + length], [x - spread, y + length]]);
    inner += f.poly([[x - half * 0.8, y], [x + half * 0.8, y], [x + spread * 0.6, y + length * 0.85], [x - spread * 0.6, y + length * 0.85]]);
  }
  return [
    shape(outer, "white", 0.07, MAIN),
    shape(inner, "white", 0.08, MAIN),
    shape(rods, "slate-800", 0.9),
    shape(housings, "slate-700", 1, INKED),
    shape(halos, "white", 0.25, MAIN),
    shape(tubes, "white", 1, MAIN),
  ];
}

/** Velvet stage curtains hanging from the anchor at their top centre, framing an opening about 40 wide and 50 tall at scale 1: two heavy drapes with folds, caught back by tasselled tie-backs, under a swagged pelmet with a fringe. */
function drapes(at: PartPlace, rng: Rng): KitShape[] {
  const f = local(at);
  const tie = between(rng, 28, 32);
  const bottom = 50;
  const drape = (sd: number): string =>
    `M${f.p(sd * 26.5, 3)}L${f.p(sd * 12, 3)}Q${f.p(sd * 13.5, tie * 0.7)} ${f.p(sd * 21.5, tie)}` +
    `Q${f.p(sd * 19.5, tie + 10)} ${f.p(sd * 17.5, bottom)}L${f.p(sd * 27, bottom)}Z`;
  const fold = (sd: number, u: number, width: number): string => {
    const mix = (a: number, b: number): number => a + (b - a) * u;
    const j = between(rng, -0.4, 0.4);
    return (
      f.band([[sd * mix(12.6, 26.2), 4.5], [sd * (mix(16.1, 26.4) + j), 0.48 + 0.696 * tie], [sd * mix(21.6, 26.6), tie - 1.6]], width) +
      f.band([[sd * mix(21.6, 26.6), tie + 1.8], [sd * (mix(19.4, 26.7) + j), tie + 10], [sd * mix(17.9, 26.6), bottom - 0.6]], width * 1.15)
    );
  };
  const swags = whole(rng, 3, 4);
  const hem: Point[] = [];
  for (let k = 0; k <= swags * 6; k += 1) hem.push([28 - (56 * k) / (swags * 6), 5 + 3.2 * Math.sin((Math.PI * (k % 6)) / 6)]);
  const tieBack = (sd: number): string =>
    f.band([[sd * 21, tie - 0.6], [sd * 27.3, tie + 0.6]], 1.5) +
    f.poly([
      [sd * 21.3, tie + 0.9],
      [sd * 22.3, tie + 0.9],
      [sd * 22.7, tie + 4.6],
      [sd * 20.9, tie + 4.6],
    ]);
  return [
    shape(drape(-1) + drape(1), "violet-700", 1, MAIN_INKED),
    shape(fold(-1, 0.3, 0.9) + fold(-1, 0.64, 0.9) + fold(1, 0.3, 0.9) + fold(1, 0.64, 0.9), "black", 0.3),
    shape(fold(-1, 0.47, 0.6) + fold(-1, 0.82, 0.6) + fold(1, 0.47, 0.6) + fold(1, 0.82, 0.6), "violet-300", 0.18),
    shape(tieBack(-1) + tieBack(1), "brass-600", 1, INKED),
    shape(f.poly([[-28, -1.2], [28, -1.2], ...hem]), "violet-700", 1, MAIN_INKED),
    shape(f.band(hem.map(([x, y]): Point => [x, y - 1.8]), 0.8), "black", 0.22),
    shape(f.band(hem, 0.9), "brass-600", 0.9),
  ];
}

/** An armoured blast door standing on the anchor: a thick frame, its slab shutter half down with a band of hazard stripes along its foot over a dark gap, bolts and a warning lamp; about 30 wide and 24 tall at scale 1. */
function blastDoor(at: PartPlace, rng: Rng): KitShape[] {
  const f = local(at);
  const ow = 15;
  const oh = 24;
  const iw = 11;
  const ih = 20;
  const foot = -ih + ih * between(rng, 0.45, 0.62);
  const band = 4;
  let slats = "";
  for (let y = -ih + 2.6; y < foot - band - 0.8; y += 2.6) slats += f.band([[-iw, y], [iw, y]], 0.35);
  const clamp = (x: number): number => Math.min(iw, Math.max(-iw, x));
  let stripes = "";
  for (let x = -iw - 4; x < iw; x += 3.2) {
    stripes += f.poly([
      [clamp(x), foot],
      [clamp(x + 1.6), foot],
      [clamp(x + 4), foot - band],
      [clamp(x + 2.4), foot - band],
    ]);
  }
  let bolts = "";
  for (const y of [-2, -8, -14, -20]) bolts += f.circle(-13, y, 0.5) + f.circle(13, y, 0.5);
  for (const x of [-8, 8]) bolts += f.circle(x, -22, 0.5);
  return [
    shape(rect(f, -iw, foot - 0.4, iw, 0), "black", 0.85),
    shape(rect(f, -iw, -ih, iw, foot), "slate-700", 1, MAIN_INKED),
    shape(slats, "slate-900", 0.5),
    shape(rect(f, -iw, foot - band, iw, foot), "orange-400", 1),
    shape(stripes, "slate-900", 1),
    shape(f.poly([[-ow, 0], [-ow, -oh], [ow, -oh], [ow, 0], [iw, 0], [iw, -ih], [-iw, -ih], [-iw, 0]]), "slate-400", 1, INKED),
    shape(bolts, "slate-800", 0.9),
    shape(f.band([[-ow + 0.5, -oh + 0.4], [ow - 0.4, -oh + 0.4], [ow - 0.4, -0.3]], 0.5), "white", 0.25),
    shape(f.circle(0, -22, 2.6), "orange-400", 0.3),
    shape(f.circle(0, -22, 0.9), "orange-400", 1),
  ];
}

/** A gun turret standing on the anchor: a domed base on a mount ring, twin barrels raised toward its facing from a mantlet, a red sensor light; about 10 wide and 8 tall at scale 1 (its barrels reach 3 past the dome). */
function turret(at: PartPlace, rng: Rng): KitShape[] {
  const f = local(at);
  const elevation = -between(rng, 18, 34);
  const pivot: Point = [1.2, -4.3];
  const on = (along: number, across: number): Point => {
    const [x, y] = turned([along, across], elevation);
    return [pivot[0] + x, pivot[1] + y];
  };
  const barrel = (across: number): string => f.band([on(0.5, across), on(7.4, across)], 0.9);
  const shine =
    `M${f.p(0, -6.4)}A${radius(f, 4.6)} ${radius(f, 5)} 0 0 ${f.sweep(1)} ${f.p(4.6, -1.4)}` +
    `L${f.p(3.5, -1.4)}A${radius(f, 3.5)} ${radius(f, 4.2)} 0 0 ${f.sweep(0)} ${f.p(0, -5.6)}Z`;
  return [
    shape(f.poly([[-5.2, 0], [5.2, 0], [4.8, -1.4], [-4.8, -1.4]]), "slate-700", 1, INKED),
    shape(barrel(-0.65) + barrel(0.65), "slate-700", 1, INKED),
    shape(domePath(f, -1.4, 4.6, 5), "slate-400", 1, MAIN_INKED),
    shape(f.band([[-4.1, -3.2], [4.1, -3.2]], 0.35), "slate-800", 0.5),
    shape(shine, "white", 0.22),
    shape(f.poly([on(-0.6, -1.6), on(2.2, -1.6), on(2.2, 1.6), on(-0.6, 1.6)]), "slate-700", 1, INKED),
    shape(f.circle(-2, -3.9, 1.4), "red-500", 0.3),
    shape(f.circle(-2, -3.9, 0.55), "red-400", 1),
  ];
}

/** A colossal space station centred on the anchor: stacked decks of lit windows tapering to a spike below, two or three plated armour rings round it (their front halves over the decks), docking arms, a glowing lobby dome and an antenna on top; about 50 wide and 50 tall at scale 1. */
function spaceStation(at: PartPlace, rng: Rng): KitShape[] {
  const f = local(at);
  const profile: Point[] = [
    [0.8, 25],
    [5, 17],
    [9, 15],
    [11, 5],
    [10, -5],
    [8, -7],
    [7, -13],
    [5, -15],
  ];
  const mirrored = profile.map(([x, y]): Point => [-x, y]);
  const halfAt = (y: number): number => {
    for (let i = 1; i < profile.length; i += 1) {
      const [x0, y0] = profile[i - 1] ?? [0, 0];
      const [x1, y1] = profile[i] ?? [0, 0];
      if (y <= y0 && y >= y1) return x0 + ((x1 - x0) * (y - y0)) / (y1 - y0 || 1);
    }
    return 0;
  };
  let windows = "";
  for (const y of [13, 10, 7, 2.5, -1.5, -10, -12.5]) {
    const hw = halfAt(y) - 1.6;
    for (let x = -hw; x < hw - 0.9; x += 1.9) if (rng() < 0.72) windows += rect(f, x, y - 0.35, x + 0.95, y + 0.35);
  }
  const rings: (readonly [number, number, number])[] = [
    [-7, 20, 3.6],
    [8, 25, 4.4],
  ];
  if (rng() < 0.5) rings.push([19, 15, 3]);
  let back = "";
  let front = "";
  let studs = "";
  for (const [y, rx, ry] of rings) {
    back += f.ellipse(0, y, rx, ry) + f.ellipse(0, y, rx - 2.4, ry - 1.1);
    front += frontHalf(f, y, rx, ry, 2.4, 1.1);
    for (let a = 20; a <= 160; a += 28) studs += f.circle((rx - 1.2) * Math.cos(a * DEGREES), y + (ry - 0.55) * Math.sin(a * DEGREES), 0.45);
  }
  const arms = f.band([[-10, 0.5], [-24, 0.5]], 1.8) + f.band([[10, 0.5], [24, 0.5]], 1.8) + rect(f, -27, -1.2, -24, 2.2) + rect(f, 24, -1.2, 27, 2.2);
  return [
    shape(arms, "slate-700", 1, INKED),
    shape(back, "slate-700", 1, INKED),
    shape(f.poly([...profile, ...[...mirrored].reverse()]) + f.band([[0, -20.6], [0, -26]], 0.5), "slate-400", 1, MAIN_INKED),
    shape(strip(f, mirrored, mirrored.map(([x, y]): Point => [x * 0.45, y])), "black", 0.25),
    shape(f.band([[8.3, 15.4], [10.3, 5], [9.3, -5]], 0.5), "white", 0.3),
    shape(windows, "cyan-400", 0.9),
    shape(front, "slate-700", 1, INKED),
    shape(studs, "cyan-400", 1),
    shape(f.circle(0, -17.5, 8.5), "cyan-400", 0.12),
    shape(domePath(f, -15, 5, 5.2), "cyan-400", 0.4),
    shape(domePath(f, -15, 3, 3.4), "white", 0.35),
  ];
}

/** A reactor core column standing on the anchor: a cylindrical housing on a plinth with vents either side of a tall glowing core window, a cap, coolant pipes bending down its sides; about 14 wide and 30 tall at scale 1 (its pipes 3 more each side). */
function reactor(at: PartPlace, rng: Rng): KitShape[] {
  const f = local(at);
  const housing = `M${f.p(-5.5, -27)}L${f.p(-5.5, -3)}A${radius(f, 5.5)} ${radius(f, 1.4)} 0 0 ${f.sweep(0)} ${f.p(5.5, -3)}L${f.p(5.5, -27)}Z`;
  const high = between(rng, -22, -19);
  const low = between(rng, -17, -14);
  const pipes =
    f.band([[5.3, high], [7.4, high], [8.4, high + 1], [8.4, 0]], 1.4) +
    f.band([[-5.3, low], [-7.4, low], [-8.4, low + 1], [-8.4, 0]], 1.4);
  let vents = "";
  for (let k = 0; k < 6; k += 1) {
    const y = -22 + k * 2.6;
    vents += rect(f, -4.8, y, -3.6, y + 0.8) + rect(f, 3.6, y, 4.8, y + 0.8);
  }
  return [
    shape(pipes, "slate-400", 1, INKED),
    shape(f.poly([[-7, 0], [7, 0], [6.4, -2.6], [-6.4, -2.6]]), "slate-800", 1, INKED),
    shape(housing, "slate-700", 1, INKED),
    shape(vents, "slate-900", 0.75),
    shape(
      f.ellipse(0, -27, 5.9, 1.5) + f.poly([[-2.4, -28.6], [2.4, -28.6], [1.8, -30.4], [-1.8, -30.4]]) + rect(f, -3, -24, 3, -6) + rect(f, -2.2, -23.2, 2.2, -6.8),
      "slate-400",
      1,
      INKED,
    ),
    shape(rect(f, -2.2, -23.2, 2.2, -6.8), "cyan-400", 1, MAIN),
    shape(rect(f, -0.6, -21.5, 0.6, -8.5), "white", 0.7),
    shape(f.ellipse(0, -15, 6.5, 12), "cyan-400", 0.22, MAIN),
    shape(f.ellipse(0, -15, 10, 16), "cyan-400", 0.1, MAIN),
    shape(f.band([[4.9, -26.5], [4.9, -3.4]], 0.5), "white", 0.22),
  ];
}

/** A shipyard crane standing on the anchor: a lattice mast about 40 tall, a lattice jib reaching about 30 toward its facing with a counterweight behind, tie cables to its peak, a cab, a cable hanging from a trolley near the jib's end with a hook. */
function crane(at: PartPlace, rng: Rng): KitShape[] {
  const f = local(at);
  const mast = between(rng, 38, 42);
  const jib = between(rng, 28, 32);
  const leg = 2.2;
  const top = -mast;
  const zig: Point[] = [];
  for (let k = 0; k <= 8; k += 1) zig.push([k % 2 === 0 ? -leg : leg, -1 - ((mast - 2) * k) / 8]);
  const jibZig: Point[] = [];
  for (let k = 0; k <= 12; k += 1) jibZig.push([-10 + ((jib - 9) * k) / 12, k % 2 === 0 ? top : top - 3]);
  const trolley = jib - between(rng, 2, 6);
  const hy = top + between(rng, 12, 24);
  const lines =
    f.band([[jib - 1, top - 3.2], [0.3, top - 8]], 0.3) +
    f.band([[-0.3, top - 8], [-10.5, top - 3.2]], 0.3) +
    f.band([[trolley, top + 0.4], [trolley, hy]], 0.3) +
    f.band([[trolley, hy + 1.6], [trolley, hy + 3.2], [trolley - 0.4, hy + 4], [trolley - 1.2, hy + 4.2], [trolley - 1.8, hy + 3.6]], 0.5);
  return [
    shape(lines, "slate-400", 0.95),
    shape(f.band(zig, 0.45), "orange-400", 1, MAIN),
    shape(f.band([[-leg, 0], [-leg, top]], 0.8) + f.band([[leg, 0], [leg, top]], 0.8) + f.band([[0, top - 3.4], [0, top - 8]], 0.7), "orange-400", 1, MAIN_INKED),
    shape(f.band(jibZig, 0.4), "orange-400", 1, MAIN),
    shape(f.band([[-11, top], [jib, top]], 0.8) + f.band([[-11, top - 3], [jib - 1, top - 3]], 0.8), "orange-400", 1, MAIN_INKED),
    shape(
      rect(f, -4.5, -1.2, 4.5, 0) + rect(f, -11.5, top + 0.4, -6.5, top + 4) + rect(f, 2.6, top + 0.4, 6.4, top + 3.6) + rect(f, trolley - 1, hy, trolley + 1, hy + 1.6),
      "slate-800",
      1,
      INKED,
    ),
    shape(rect(f, 4.2, top + 1.1, 6, top + 2.4), "cyan-400", 0.85),
    shape(f.circle(jib - 0.4, top - 3.9, 1.8), "red-500", 0.3),
    shape(f.circle(jib - 0.4, top - 3.9, 0.6), "red-400", 1),
  ];
}

/** A glass colony dome standing on the anchor: a pale glass hemisphere ribbed with a lattice on a lit base ring, small lit buildings inside under a warm glow; about 36 wide and 18 tall at scale 1. */
function colonyDome(at: PartPlace, rng: Rng): KitShape[] {
  const f = local(at);
  const rx = 18;
  const ry = 17;
  const base = -1.6;
  const roofAt = (x: number): number => ry * Math.sqrt(Math.max(0, 1 - (x / rx) ** 2));
  const count = whole(rng, 4, 6);
  const slot = 28 / count;
  let buildings = "";
  let windows = "";
  for (let i = 0; i < count; i += 1) {
    const l = -14 + slot * i + 0.4;
    const r = l + slot - 0.8;
    const h = Math.max(2.5, roofAt(Math.max(Math.abs(l), Math.abs(r))) * 0.75 * between(rng, 0.4, 1));
    buildings += rect(f, l, base - h, r, base);
    for (let y = base - 1.4; y > base - h + 0.8; y -= 1.8) {
      for (let x = l + 0.5; x < r - 1.1; x += 1.5) if (rng() < 0.55) windows += rect(f, x, y - 0.6, x + 0.7, y);
    }
  }
  const meridian = (x0: number, to: number): string => {
    const points: Point[] = [];
    for (let k = 0; k <= 10; k += 1) {
      const t = ((to * k) / 10) * DEGREES;
      points.push([x0 * Math.cos(t), base - ry * Math.sin(t)]);
    }
    return f.band(points, 0.6);
  };
  const meridians =
    meridian(-rx + 0.3, 90) +
    meridian(rx - 0.3, 90) +
    meridian(-12, 62) +
    meridian(12, 62) +
    meridian(-6, 62) +
    meridian(6, 62) +
    f.band([[0, base], [0, base - ry + 0.3]], 0.6);
  let latitudes = "";
  for (const level of [0.38, 0.7, 0.883]) {
    const across = Math.sqrt(1 - level * level);
    latitudes += f.band(arc(0, base - ry * level, rx * across - 0.3, 1.8 * across, 0, 180, 14), 0.5);
  }
  let lights = "";
  for (let x = -16; x <= 16; x += 4) lights += f.circle(x, -1.1, 0.4);
  return kept([
    shape(f.ellipse(0, base - 4, 14, 6), "orange-400", 0.12),
    shape(buildings, "slate-800", 1, INKED),
    shape(windows, "orange-400", 0.9),
    shape(domePath(f, base, rx, ry), "sky-300", 0.16),
    shape(strip(f, arc(0, base, rx - 1.2, ry - 1.2, -78, -18, 10), arc(0, base, rx - 3.4, ry - 3, -78, -18, 10)), "white", 0.2),
    shape(meridians, "slate-400", 0.9, MAIN),
    shape(latitudes, "slate-400", 0.8, MAIN),
    shape(rect(f, -19.2, -2.2, 19.2, 0) + f.circle(0, base - ry, 1.1), "slate-700", 1, INKED),
    shape(lights, "cyan-400", 1),
  ]);
}

/** A camp tent standing on the anchor in three-quarter view, its front toward its facing: a ridged canvas, its door flap pinned back on a lamp-lit mouth, poles and guy lines, the lamp's glow spilling on the ground; about 16 wide and 10 tall at scale 1. */
function tent(at: PartPlace, rng: Rng): KitShape[] {
  const f = local(at);
  const apex: Point = [3, -10];
  const rear: Point = [-6, -9.2 + between(rng, -0.4, 0.4)];
  const left: Point = [-2.5, 0];
  const right: Point = [8.5, 0];
  const back: Point = [-8.5, 0];
  const mix = (a: Point, b: Point, t: number): Point => [a[0] + (b[0] - a[0]) * t, a[1] + (b[1] - a[1]) * t];
  const seams = [0.34, 0.67].map((t) => f.band([mix(apex, rear, t), mix(left, back, t)], 0.35)).join("");
  const flicker = between(rng, -0.2, 0.2);
  return [
    shape(f.ellipse(4, 0.6, 10, 2.6), "orange-400", 0.08),
    shape(f.ellipse(0, 0.3, 11, 1.2), "black", 0.25),
    shape(f.ellipse(4, 0.6, 6, 1.6), "orange-400", 0.2),
    shape(f.poly([apex, rear, back, left]) + f.poly([apex, right, left]), "brass-600", 1, MAIN_INKED),
    shape(f.poly([apex, rear, back, left]), "black", 0.22),
    shape(seams, "wood-800", 0.4),
    shape(f.poly([[3, -7.8], [6.4, 0], [0.4, 0]]), "black", 0.8),
    shape(f.ellipse(3.2 + flicker, -2, 2.2, 2), "orange-400", 0.5),
    shape(f.circle(3.2, -2.2, 0.7), "orange-400", 1),
    shape(f.poly([[3, -7.8], [6.4, 0], [8.9, -1.6], [6.6, -5.4]]), "brass-500", 1, INKED),
    shape(
      f.band([apex, rear], 0.55) +
        f.band([[3, -10.3], [3, -11.4]], 0.45) +
        f.band([[rear[0], rear[1] - 0.3], [rear[0], rear[1] - 1.3]], 0.45) +
        f.band([[3.3, -11.2], [13, 0]], 0.25) +
        f.band([[rear[0] - 0.3, rear[1] - 1.1], [-12.5, 0]], 0.25),
      "wood-800",
      0.9,
    ),
  ];
}

/** A crumbling trading spire standing on the anchor: five stacked tiers narrowing upward under broken cornices, torn striped awnings flapping off them, cracks, dark and lit windows, a doorway up a short stair at its foot; about 20 wide and 55 tall at scale 1. */
function spiceTower(at: PartPlace, rng: Rng): KitShape[] {
  const f = local(at);
  const tiers: readonly (readonly [number, number, number, number])[] = [
    [0, -14, 10, 9.4],
    [-15.2, -26, 8, 7.4],
    [-27.2, -36, 6.2, 5.7],
    [-37.2, -44.5, 4.6, 4.2],
    [-45.7, -51, 3.1, 2.8],
  ];
  const side = (sd: number): Point[] => {
    const points: Point[] = [];
    for (const [k, [bottom, top, wb, wt]] of tiers.entries()) {
      points.push([sd * (wb + between(rng, -0.35, 0.35)), bottom], [sd * (wt + between(rng, -0.35, 0.35)), top]);
      const next = tiers[k + 1];
      if (next === undefined) continue;
      const lip = rng() < 0.3 ? 0.3 : 0.9;
      points.push([sd * (wt + lip), top], [sd * (wt + lip + between(rng, -0.2, 0.2)), next[0]]);
    }
    return points;
  };
  const right = side(1);
  const left = side(-1);
  const body = f.poly([...right, [between(rng, -0.6, 0.6), -55], ...left.reverse()]);
  const shade = tiers.map(([bottom, top, wb, wt]) => f.poly([[-wb + 0.4, bottom], [-wb * 0.45, bottom], [-wt * 0.45, top], [-wt + 0.4, top]])).join("");
  const ledges = tiers
    .slice(0, -1)
    .map(([, top, , wt]) => f.band([[-(wt + 0.4), top - 1.05], [wt + 0.4, top - 1.05]], 0.3))
    .join("");
  let dark = "";
  let lit = "";
  for (const [k, [bottom, top, wb]] of tiers.entries()) {
    const y = (bottom + top) / 2 + 1;
    const xs = k === 0 ? [-6, 6] : k === tiers.length - 1 ? [0] : [-wb * 0.5, 0, wb * 0.5];
    for (const x of xs) {
      const roll = rng();
      if (roll < 0.15) continue;
      const pane = rect(f, x - 0.7, y - 1.2, x + 0.7, y + 1.2);
      if (roll < 0.4) lit += pane;
      else dark += pane;
    }
  }
  const crack = (x: number, y: number, dir: number): string => {
    const points: Point[] = [[x, y]];
    for (let k = 0; k < 4; k += 1) {
      const [px, py] = points[k] ?? [x, y];
      points.push([px + dir * between(rng, 0.3, 1.2), py + between(rng, 1.2, 2)]);
    }
    return f.band(points, 0.4);
  };
  const cracks = crack(between(rng, 3, 6), -13.6, -1) + crack(between(rng, -4, -1), -35.6, 1);
  let awnings = "";
  let stripes = "";
  const first = rng() < 0.5 ? 1 : -1;
  for (const k of [0, 1, 2]) {
    const tier = tiers[k];
    if (tier === undefined) continue;
    const [, top, , wt] = tier;
    const sd = k % 2 === 0 ? first : -first;
    const droop = k === 1 ? between(rng, 12, 32) : between(rng, -4, 6);
    const w0 = wt - 0.2;
    const y0 = top + 1.6;
    const P = (x: number, y: number): Point => {
      const [tx, ty] = turned([x, y], droop);
      return [sd * (w0 + tx), y0 + ty];
    };
    awnings += f.poly([P(0, 0), P(5.5, 2.6), P(5.1, 3.7), P(4.2, 3.1), P(3.4, 4.1), P(2.5, 3.3), P(1.5, 3.9), P(0.6, 3.1), P(0, 3)]);
    stripes += f.band([P(1.8, 1.05), P(1.8, 3.3)], 0.6) + f.band([P(3.8, 2), P(3.8, 3)], 0.6);
  }
  const door = `M${f.p(-2.4, -2.4)}L${f.p(-2.4, -6.5)}A${radius(f, 2.4)} ${radius(f, 2.4)} 0 0 ${f.sweep(1)} ${f.p(2.4, -6.5)}L${f.p(2.4, -2.4)}Z`;
  const stairs =
    rect(f, -5, -0.8, 5, 0) +
    rect(f, -4.2, -1.6, 4.2, -0.8) +
    rect(f, -3.4, -2.4, 3.4, -1.6) +
    f.circle(-8.2, -0.6, 0.9) +
    f.circle(7.8, -0.5, 0.7) +
    f.circle(9.2, -0.4, 0.5);
  return kept([
    shape(body, "wood-700", 1, MAIN_INKED),
    shape(shade, "black", 0.25),
    shape(ledges, "white", 0.16),
    shape(cracks, "black", 0.45),
    shape(dark, "black", 0.7),
    shape(lit, "orange-400", 0.9),
    shape(door, "black", 0.85),
    shape(stairs, "wood-600", 1, INKED),
    shape(awnings, "red-600", 1, INKED),
    shape(stripes, "slate-100", 0.45),
  ]);
}

/** A capital's skyline standing on the anchor across about 80: seven to ten towers of varied heights and tops before a hazier row, rows of lit windows, one antenna spire with a beacon, over the city's glow; its tallest about 50 with its antenna at scale 1. */
function citySkyline(at: PartPlace, rng: Rng): KitShape[] {
  const f = local(at);
  const count = whole(rng, 7, 10);
  const raw = Array.from({ length: count }, () => between(rng, 0.7, 1.3));
  const total = raw.reduce((sum, value) => sum + value, 0);
  const tallest = whole(rng, 1, count - 2);
  const tower = (l: number, r: number, h: number, style: number): string => {
    const w = r - l;
    if (style === 1) return f.poly([[l, 0], [r, 0], [r, -h], [r - w * 0.22, -h], [r - w * 0.22, -h - 3], [l + w * 0.22, -h - 3], [l + w * 0.22, -h], [l, -h]]);
    if (style === 2) return f.poly([[l, 0], [r, 0], [r, -h], [(l + r) / 2, -h - 4], [l, -h]]);
    return rect(f, l, -h, r, 0);
  };
  let front = "";
  let edges = "";
  let lit = "";
  let warm = "";
  let spire: Point = [0, -30];
  let x = -40;
  for (let i = 0; i < count; i += 1) {
    const w = ((raw[i] ?? 1) / total) * 80;
    const l = x + 0.3;
    const r = x + w - 0.3;
    const h = i === tallest ? between(rng, 32, 36) : between(rng, 10, 28);
    const style = i === tallest ? 2 : whole(rng, 0, 2);
    front += tower(l, r, h, style);
    edges += rect(f, r - 0.5, -h, r, 0);
    if (i === tallest) spire = [(l + r) / 2, -h - 4];
    for (let y = -2.5; y > -h + 1.5; y -= 3) {
      for (let wx = l + 0.9; wx < r - 1.6; wx += 2.2) {
        const roll = rng();
        if (roll < 0.08) warm += rect(f, wx, y - 0.7, wx + 0.9, y);
        else if (roll < 0.55) lit += rect(f, wx, y - 0.7, wx + 0.9, y);
      }
    }
    x += w;
  }
  let back = "";
  for (let bx = -38 + between(rng, -3, 3); bx < 36; ) {
    const w = between(rng, 7, 12);
    back += rect(f, bx, -between(rng, 18, 34), Math.min(40, bx + w - 0.4), 0);
    bx += w;
  }
  const [sx, sy] = spire;
  return kept([
    shape(f.ellipse(0, -4, 46, 12), "cyan-400", 0.08),
    shape(back, "slate-800", 0.5, MAIN),
    shape(front, "slate-800", 1, MAIN_INKED),
    shape(edges, "slate-400", 0.25),
    shape(lit, "cyan-400", 0.85),
    shape(warm, "orange-400", 0.85),
    shape(f.band([[sx, sy + 0.5], [sx, sy - 12]], 0.5), "slate-400", 0.95),
    shape(f.circle(sx, sy - 12.6, 2), "red-500", 0.3),
    shape(f.circle(sx, sy - 12.6, 0.7), "red-400", 1),
  ]);
}

/** A hangar rack standing on the anchor: two or three tiers of docking cradles in a frame, each holding a small delta-winged drone pointed toward its facing with a red eye, a few left empty, status lights on its posts; about 24 wide and 20 tall at scale 1. */
function droneRack(at: PartPlace, rng: Rng): KitShape[] {
  const f = local(at);
  const tiers = whole(rng, 2, 3);
  const hw = 12;
  const H = 20;
  const post = 1.4;
  const shelf = 0.9;
  const bay = (H - shelf * (tiers + 1)) / tiers;
  const size = Math.min(1.3, bay / 5.2);
  const slot = (2 * (hw - post)) / 3;
  let frame = rect(f, -hw, -H, -hw + post, 0) + rect(f, hw - post, -H, hw, 0);
  for (let k = 0; k <= tiers; k += 1) {
    const yb = -k * (bay + shelf);
    frame += rect(f, -hw + post, yb - shelf, hw - post, yb);
  }
  let cradles = "";
  let drones = "";
  let tops = "";
  let glows = "";
  let eyes = "";
  let status = "";
  for (let k = 0; k < tiers; k += 1) {
    const floor = -k * (bay + shelf) - shelf;
    status += rect(f, -hw + 0.4, floor - 2, -hw + 1, floor - 1.2) + rect(f, hw - 1, floor - 2, hw - 0.4, floor - 1.2);
    for (let j = 0; j < 3; j += 1) {
      const x = -hw + post + slot * (j + 0.5);
      cradles += f.poly([[x - 2.2 * size, floor], [x + 2.2 * size, floor], [x + 1.6 * size, floor - 0.9 * size], [x - 1.6 * size, floor - 0.9 * size]]);
      if (rng() < 0.2) continue;
      const c = floor - 2.6 * size;
      drones += f.poly([[x + 2.8 * size, c], [x - 2.4 * size, c - 1.7 * size], [x - 1.4 * size, c], [x - 2.4 * size, c + 1.7 * size]]);
      tops += f.band([[x + 2.2 * size, c - 0.25 * size], [x - 1.9 * size, c - 1.6 * size]], 0.3 * size);
      glows += f.circle(x + size, c, 1.1 * size);
      eyes += f.circle(x + size, c, 0.42 * size);
    }
  }
  return kept([
    shape(rect(f, -hw + post, -H + shelf, hw - post, -shelf), "slate-900", 0.85),
    shape(frame, "slate-700", 1, MAIN_INKED),
    shape(cradles, "slate-800", 1, INKED),
    shape(drones, "slate-400", 1, INKED),
    shape(tops, "white", 0.3),
    shape(glows, "red-500", 0.3),
    shape(eyes, "red-400", 1),
    shape(status, "cyan-400", 0.9),
  ]);
}

/** An AI core standing on the anchor: a stepped plinth, a tall glowing glass cylinder with a bright ring of an eye inside, two tilted data rings orbiting it, a cap; about 10 wide and 34 tall at scale 1 (its rings 14 across). */
function aiCore(at: PartPlace, rng: Rng): KitShape[] {
  const f = local(at);
  const top = -29;
  const bottom = -4.5;
  const hw = 3;
  const ey = -17 + between(rng, -1.5, 1.5);
  const tilt = between(rng, 5, 12);
  const oval = (cy: number, rx: number, ry: number, degrees: number): string => {
    const [ax, ay] = turned([-rx, 0], degrees);
    const [bx, by] = turned([rx, 0], degrees);
    const arcTo = `A${radius(f, rx)} ${radius(f, ry)} ${fmt(degrees * at.flip)} 1 0 `;
    return `M${f.p(ax, cy + ay)}${arcTo}${f.p(bx, cy + by)}${arcTo}${f.p(ax, cy + ay)}Z`;
  };
  const orbits: readonly (readonly [number, number])[] = [
    [-11.5, tilt],
    [-22.5, -tilt],
  ];
  let rings = "";
  let dots = "";
  for (const [cy, degrees] of orbits) {
    rings += oval(cy, 6.8, 1.7, degrees) + oval(cy, 6.2, 1.25, degrees);
    for (const a of [30, 150, 250]) {
      const [x, y] = turned([6.5 * Math.cos(a * DEGREES), 1.48 * Math.sin(a * DEGREES)], degrees);
      dots += rect(f, x - 0.3, cy + y - 0.3, x + 0.3, cy + y + 0.3);
    }
  }
  const stone =
    f.poly([[-5, 0], [5, 0], [5, -2.6], [4, -2.6], [4, bottom], [-4, bottom], [-4, -2.6], [-5, -2.6]]) +
    f.poly([[-4, top], [4, top], [4, top - 1.6], [2.4, top - 1.6], [1.4, top - 4], [-1.4, top - 4], [-2.4, top - 1.6], [-4, top - 1.6]]);
  return [
    shape(f.ellipse(0, -17, 9, 17), "cyan-400", 0.08, MAIN),
    shape(stone, "slate-700", 1, INKED),
    shape(rect(f, -hw, top, hw, bottom), "cyan-400", 0.3, MAIN),
    shape(f.ellipse(0, -17, 5.6, 13.5), "cyan-400", 0.14, MAIN),
    shape(f.band([[-hw + 0.2, top], [-hw + 0.2, bottom]], 0.4) + f.band([[hw - 0.2, top], [hw - 0.2, bottom]], 0.4), "slate-100", 0.45),
    shape(rings, "cyan-400", 0.85, MAIN),
    shape(f.circle(0, ey, 3.6), "cyan-400", 0.3, MAIN),
    shape(ring(f, 0, ey, 2.2, 1.4), "cyan-400", 1, MAIN),
    shape(f.circle(0, ey, 0.75), "white", 0.95),
    shape(dots, "white", 0.85),
  ];
}

/** A holographic tactical table standing on the anchor: a console with an emitter ring on its top, a beam rising to a translucent disc map ringed and gridded above it, tiny ship and enemy markers on the map; about 24 wide and 6 tall at scale 1, its map floating to 15 above the anchor. */
function holoTable(at: PartPlace, rng: Rng): KitShape[] {
  const f = local(at);
  const my = -12.5;
  let ships = "";
  let enemies = "";
  for (let i = 0; i < 3; i += 1) {
    const x = -8 + 2.4 * i + between(rng, -0.3, 0.3);
    const y = my + between(rng, -0.9, 0.9);
    ships += f.poly([[x + 0.9, y], [x - 0.6, y - 0.6], [x - 0.6, y + 0.6]]);
  }
  for (let i = 0; i < 2; i += 1) {
    const x = 3 + 3 * i + between(rng, -0.5, 0.5);
    const y = my + between(rng, -0.9, 0.9);
    enemies += f.poly([[x, y - 0.7], [x + 0.6, y], [x, y + 0.7], [x - 0.6, y]]);
  }
  const grid = f.band([[-10.4, my], [-0.4, my]], 0.25) + f.band([[0.4, my], [10.4, my]], 0.25) + f.band([[0, my - 2.6], [0, my + 2.6]], 0.25);
  return [
    shape(f.poly([[-8, 0], [8, 0], [11, -4.6], [-11, -4.6]]), "slate-800", 1, INKED),
    shape(f.ellipse(0, -5, 12, 2.2), "slate-700", 1, INKED),
    shape(f.band(arc(0, -5, 11.6, 1.85, 15, 165, 12), 0.3), "white", 0.2),
    shape(f.poly([[-7, -5.2], [7, -5.2], [10.5, my], [-10.5, my]]), "cyan-400", 0.1, MAIN),
    shape(f.ellipse(0, -5.2, 7.5, 1.3) + f.ellipse(0, -5.2, 6.6, 1.1), "cyan-400", 0.85, MAIN),
    shape(f.ellipse(0, my, 11, 2.8), "cyan-400", 0.22, MAIN),
    shape(f.ellipse(0, my, 11, 2.8) + f.ellipse(0, my, 10.5, 2.6) + f.ellipse(0, my, 6.4, 1.6) + f.ellipse(0, my, 6, 1.45), "cyan-400", 0.75, MAIN),
    shape(grid, "cyan-400", 0.45, MAIN),
    shape(ships, "white", 0.95),
    shape(enemies, "red-400", 0.95),
  ];
}

/** R3403000: the places in space Stinkyscape's later cards added, by id, each with its kind. */
export const SPACE_KIT = {
  "deck-floor": { category: "terrain", draw: deckFloor },
  bulkhead: { category: "terrain", draw: bulkhead },
  regolith: { category: "terrain", draw: regolith },
  "bowl-crater": { category: "terrain", draw: crater },
  honeycomb: { category: "terrain", draw: honeycomb },
  "vent-duct": { category: "terrain", draw: ventDuct },
  "planet-limb": { category: "terrain", draw: planetLimb },
  auditorium: { category: "terrain", draw: auditorium },
  "scrap-heap": { category: "terrain", draw: scrapHeap },
  "giant-asteroid": { category: "terrain", draw: giantAsteroid },
  viewport: { category: "architecture", draw: viewport },
  porthole: { category: "architecture", draw: porthole },
  "obs-bench": { category: "architecture", draw: bench },
  "boxing-ring": { category: "architecture", draw: boxingRing },
  catwalk: { category: "architecture", draw: catwalk },
  airlock: { category: "architecture", draw: airlock },
  gantry: { category: "architecture", draw: gantry },
  "warp-gate": { category: "architecture", draw: warpGate },
  "launch-pad": { category: "architecture", draw: launchPad },
  "glass-spire": { category: "architecture", draw: glassSpire },
  "intake-tube": { category: "architecture", draw: intakeTube },
  "ceiling-lights": { category: "architecture", draw: ceilingLights },
  drapes: { category: "architecture", draw: drapes },
  "blast-door": { category: "architecture", draw: blastDoor },
  turret: { category: "architecture", draw: turret },
  "space-station": { category: "architecture", draw: spaceStation },
  reactor: { category: "architecture", draw: reactor },
  "shipyard-crane": { category: "architecture", draw: crane },
  "colony-dome": { category: "architecture", draw: colonyDome },
  "lamp-tent": { category: "architecture", draw: tent },
  "spice-tower": { category: "architecture", draw: spiceTower },
  "city-skyline": { category: "architecture", draw: citySkyline },
  "drone-rack": { category: "architecture", draw: droneRack },
  "ai-core": { category: "architecture", draw: aiCore },
  "holo-table": { category: "architecture", draw: holoTable },
} as const satisfies Readonly<Record<string, KitPart>>;
