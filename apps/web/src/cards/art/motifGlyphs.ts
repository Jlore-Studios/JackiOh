// The glyphs v0.2.0 adds to the procedural art (R503): the Book, Pancake and AI families' emblems and
// the small pictures a card's motif is drawn with (motifs.ts). Each is drawn for this project in the
// same 24×24 box as emblems.ts, centred on (12, 12), as plain path data with no text, no external
// reference and no borrowed artwork, and `svg.ts` places it with a transform exactly as it places an
// emblem.
//
// Every glyph here is filled nonzero, so its parts may overlap (a sheep's fleece is a heap of
// circles, crossed bones cross): a solid part is always wound one way (anticlockwise on screen) and
// a hole the other, which `solid` and `hole` see to, so an overlap never punches a hole by accident.

import { fmt } from "./hash.ts";

export type GlyphPath = { d: string; rule: "nonzero" | "evenodd" };

export type MotifGlyph =
  // The families' emblems.
  | "tome" | "quill" | "pancakes" | "spatula" | "drop" | "chip" | "neural"
  // The motifs' pictures.
  | "spore" | "sheep" | "ghost" | "boulder" | "monkey" | "shrimp" | "turtle" | "lizard" | "grapes"
  | "die" | "clock" | "padlock" | "broken-rune" | "house" | "bomb" | "cross" | "bones" | "gust"
  | "drum" | "sun" | "snake" | "bat" | "mushroom" | "portal" | "brain" | "target" | "heart" | "flag"
  | "halo" | "gear" | "leaf" | "cycle" | "echo" | "helix" | "jaws" | "snowflake" | "infinity"
  | "crate" | "blades" | "flask" | "scales" | "chevrons" | "bubbles" | "mask" | "horns" | "note"
  | "cards" | "candy" | "bread" | "jet" | "web";

type Pt = readonly [number, number];

const C = 12;
const DEG = Math.PI / 180;

/** Twice the signed area; on screen (y down) a negative value runs anticlockwise. */
function signedArea(points: readonly Pt[]): number {
  let sum = 0;
  for (let i = 0; i < points.length; i += 1) {
    const [ax, ay] = points[i] ?? [0, 0];
    const [bx, by] = points[(i + 1) % points.length] ?? [0, 0];
    sum += ax * by - bx * ay;
  }
  return sum;
}

function trace(points: readonly Pt[]): string {
  return `M${points.map(([x, y]) => `${fmt(x)} ${fmt(y)}`).join("L")}Z`;
}

/** A filled part: wound anticlockwise on screen. */
function solid(points: readonly Pt[]): string {
  return trace(signedArea(points) > 0 ? [...points].reverse() : points);
}

/** A hole in a part: wound clockwise on screen. */
function hole(points: readonly Pt[]): string {
  return trace(signedArea(points) < 0 ? [...points].reverse() : points);
}

function ellipse(cx: number, cy: number, rx: number, ry: number, turn = 0, steps = 20): Pt[] {
  const out: Pt[] = [];
  const cos = Math.cos(turn * DEG);
  const sin = Math.sin(turn * DEG);
  for (let k = 0; k < steps; k += 1) {
    const t = (k / steps) * 2 * Math.PI;
    const x = rx * Math.cos(t);
    const y = ry * Math.sin(t);
    out.push([cx + x * cos - y * sin, cy + x * sin + y * cos]);
  }
  return out;
}

function disc(cx: number, cy: number, r: number): Pt[] {
  return ellipse(cx, cy, r, r, 0, r > 3 ? 24 : 12);
}

function rect(x: number, y: number, w: number, h: number): Pt[] {
  return [
    [x, y],
    [x + w, y],
    [x + w, y + h],
    [x, y + h],
  ];
}

/** A rectangle whose corners are rounded by `r`. */
function roundRect(x: number, y: number, w: number, h: number, r: number): Pt[] {
  const out: Pt[] = [];
  const corners: readonly (readonly [number, number, number])[] = [
    [x + w - r, y + r, -90],
    [x + w - r, y + h - r, 0],
    [x + r, y + h - r, 90],
    [x + r, y + r, 180],
  ];
  for (const [cx, cy, start] of corners) {
    for (let k = 0; k <= 3; k += 1) {
      const a = (start + k * 30) * DEG;
      out.push([cx + r * Math.cos(a), cy + r * Math.sin(a)]);
    }
  }
  return out;
}

/** A straight bar `w` wide from one point to another. */
function bar(x1: number, y1: number, x2: number, y2: number, w: number): Pt[] {
  const length = Math.hypot(x2 - x1, y2 - y1) || 1;
  const nx = (-(y2 - y1) / length) * (w / 2);
  const ny = ((x2 - x1) / length) * (w / 2);
  return [
    [x1 + nx, y1 + ny],
    [x2 + nx, y2 + ny],
    [x2 - nx, y2 - ny],
    [x1 - nx, y1 - ny],
  ];
}

/** A band `w` wide (or tapering from `w` to `w2`) along an open polyline. */
function band(line: readonly Pt[], w: number, w2 = w): Pt[] {
  const left: Pt[] = [];
  const right: Pt[] = [];
  for (let i = 0; i < line.length; i += 1) {
    const [px, py] = line[Math.max(0, i - 1)] ?? [0, 0];
    const [nx, ny] = line[Math.min(line.length - 1, i + 1)] ?? [0, 0];
    const [x, y] = line[i] ?? [0, 0];
    const length = Math.hypot(nx - px, ny - py) || 1;
    const half = (w + ((w2 - w) * i) / Math.max(1, line.length - 1)) / 2;
    const ox = (-(ny - py) / length) * half;
    const oy = ((nx - px) / length) * half;
    left.push([x + ox, y + oy]);
    right.push([x - ox, y - oy]);
  }
  return [...left, ...right.reverse()];
}

/** Points along an arc of a circle, `from` to `to` degrees (0 = right, clockwise on screen). */
function arc(cx: number, cy: number, r: number, from: number, to: number, steps = 10): Pt[] {
  const out: Pt[] = [];
  for (let k = 0; k <= steps; k += 1) {
    const a = (from + ((to - from) * k) / steps) * DEG;
    out.push([cx + r * Math.cos(a), cy + r * Math.sin(a)]);
  }
  return out;
}

/** Points along an arc of an ellipse, `from` to `to` degrees. */
function ellipseArc(cx: number, cy: number, rx: number, ry: number, from: number, to: number, steps = 12): Pt[] {
  const out: Pt[] = [];
  for (let k = 0; k <= steps; k += 1) {
    const a = (from + ((to - from) * k) / steps) * DEG;
    out.push([cx + rx * Math.cos(a), cy + ry * Math.sin(a)]);
  }
  return out;
}

/** An ellipse whose rim bulges out in `bumps` round lobes: a fleece, a cloud, a brain. */
function scalloped(cx: number, cy: number, rx: number, ry: number, bumps: number, depth: number, steps = 60): Pt[] {
  const out: Pt[] = [];
  for (let k = 0; k < steps; k += 1) {
    const t = (k / steps) * 2 * Math.PI;
    const swell = 1 - depth + depth * Math.abs(Math.sin((t * bumps) / 2));
    out.push([cx + rx * swell * Math.cos(t), cy + ry * swell * Math.sin(t)]);
  }
  return out;
}

/** Points along a quadratic Bézier. */
function quad(a: Pt, b: Pt, c: Pt, steps = 8): Pt[] {
  const out: Pt[] = [];
  for (let k = 0; k <= steps; k += 1) {
    const t = k / steps;
    const u = 1 - t;
    out.push([u * u * a[0] + 2 * u * t * b[0] + t * t * c[0], u * u * a[1] + 2 * u * t * b[1] + t * t * c[1]]);
  }
  return out;
}

function turn(points: readonly Pt[], degrees: number, cx = C, cy = C): Pt[] {
  const cos = Math.cos(degrees * DEG);
  const sin = Math.sin(degrees * DEG);
  return points.map(([x, y]) => [cx + (x - cx) * cos - (y - cy) * sin, cy + (x - cx) * sin + (y - cy) * cos]);
}

function mirror(points: readonly Pt[]): Pt[] {
  return points.map(([x, y]) => [2 * C - x, y]);
}

function glyph(...parts: string[]): GlyphPath {
  return { d: parts.join(""), rule: "nonzero" };
}

/** A four-pointed sparkle. */
function sparkle(cx: number, cy: number, r: number): Pt[] {
  const out: Pt[] = [];
  for (let k = 0; k < 8; k += 1) {
    const a = (k * 45 - 90) * DEG;
    const radius = k % 2 === 0 ? r : r * 0.3;
    out.push([cx + radius * Math.cos(a), cy + radius * Math.sin(a)]);
  }
  return out;
}

function tome(): GlyphPath {
  return glyph(
    solid(roundRect(4.5, 2.5, 14.5, 19, 1.2)),
    solid(rect(19, 9.5, 2.4, 3.4)),
    hole(rect(6.6, 3.4, 0.9, 17.2)),
    hole(rect(16.9, 4, 0.8, 16)),
    hole([[12.2, 6.4], [15.4, 11.2], [12.2, 16], [9, 11.2]]),
    solid([[12.2, 8.6], [13.8, 11.2], [12.2, 13.8], [10.6, 11.2]]),
  );
}

function quill(): GlyphPath {
  const vane: Pt[] = [
    [5.2, 19.4],
    ...quad([6.4, 16.6], [9, 7.8], [21.2, 2.2]).slice(1),
    ...quad([21.2, 2.2], [18.6, 12.2], [7.4, 18.4]).slice(1),
  ];
  return glyph(
    solid(vane),
    hole(band(quad([7.6, 17], [12, 11.4], [18.4, 5]), 0.7)),
    solid([[2.6, 22.2], [5, 19.2], [6.2, 20.1]]),
  );
}

function pancakes(): GlyphPath {
  return glyph(
    solid(roundRect(1.5, 20.4, 21, 1.8, 0.9)),
    solid(roundRect(3, 16.4, 18, 3.6, 1.8)),
    solid(roundRect(3.6, 12.4, 16.8, 3.6, 1.8)),
    solid(roundRect(4.2, 8.4, 15.6, 3.6, 1.8)),
    solid(turn(rect(9.9, 4.9, 4.2, 3), -8)),
    // Syrup running over the top cake's lip.
    hole([[6, 10.9], [9, 10.2], [13, 10.6], [17.2, 10], [17.6, 11], [15.8, 11.3], [15.4, 12], [14.6, 11.2], [9.4, 11.4], [8.6, 12], [8, 11.2]]),
  );
}

function spatula(): GlyphPath {
  const head = solid(turn(roundRect(7.6, 1.6, 8.8, 9.4, 1.4), 32));
  const slots = [9.3, 11.6, 13.9].map((x) => hole(turn(rect(x - 0.45, 3.2, 0.9, 5.6), 32)));
  const neck = solid(turn(rect(11, 11, 2, 2.4), 32));
  const handle = solid(turn(roundRect(10.6, 13.4, 2.8, 9.2, 1.2), 32));
  return glyph(head, ...slots, neck, handle);
}

function drop(): GlyphPath {
  const body: Pt[] = [...quad([12, 1.8], [7.4, 9.4], [5.4, 13.6]), ...arc(12, 15, 6.75, 168, 12, 12).slice(1), ...quad([18.6, 13.6], [16.6, 9.4], [12, 1.8]).slice(1, -1)];
  return glyph(solid(body), hole(ellipse(9.2, 15, 1, 2, 20, 12)));
}

function chip(): GlyphPath {
  const parts = [solid(roundRect(6, 6, 12, 12, 1)), hole(rect(8.8, 8.8, 6.4, 6.4)), solid(rect(10.4, 10.4, 3.2, 3.2))];
  for (const at of [8.4, 12, 15.6]) {
    parts.push(solid(rect(at - 0.6, 2.4, 1.2, 3.6)), solid(rect(at - 0.6, 18, 1.2, 3.6)));
    parts.push(solid(rect(2.4, at - 0.6, 3.6, 1.2)), solid(rect(18, at - 0.6, 3.6, 1.2)));
  }
  return glyph(...parts);
}

function neural(): GlyphPath {
  const ring: Pt[] = [-90, -18, 54, 126, 198].map((a): Pt => [12 + 8.2 * Math.cos(a * DEG), 12 + 8.2 * Math.sin(a * DEG)]);
  const parts: string[] = [];
  ring.forEach(([x, y], k) => {
    const [nx, ny] = ring[(k + 1) % ring.length] ?? [0, 0];
    parts.push(solid(bar(12, 12, x, y, 1.3)), solid(bar(x, y, nx, ny, 1.1)));
  });
  parts.push(solid(disc(12, 12, 3)), hole(disc(12, 12, 1.2)));
  for (const [x, y] of ring) parts.push(solid(disc(x, y, 2.3)));
  return glyph(...parts);
}

function spore(): GlyphPath {
  return glyph(
    solid(disc(12, 13, 5.6)),
    hole(disc(10, 11.4, 1.1)),
    hole(disc(13.9, 12.3, 0.9)),
    hole(disc(11.7, 15.6, 1)),
    solid(disc(4.5, 6, 1.7)),
    solid(disc(19.4, 5.2, 1.4)),
    solid(disc(20.4, 17.6, 1.6)),
    solid(disc(4, 18.6, 1.3)),
    solid(disc(12.2, 3.4, 1.2)),
  );
}

function sheep(): GlyphPath {
  const legs = [8.4, 10.8, 14.6, 17].map((x) => solid(roundRect(x - 0.75, 15.4, 1.5, 6.4, 0.6)));
  return glyph(
    ...legs,
    solid(scalloped(13.4, 11.4, 8, 5.8, 11, 0.16)),
    hole(band(quad([10.4, 9.6], [13.4, 8], [16.6, 9.8]), 0.6)),
    hole(band(quad([11, 13.2], [14, 11.8], [17.4, 13.6]), 0.6)),
    solid(ellipse(4.8, 11.2, 2.9, 2.3, -15)),
    solid(ellipse(5.6, 8.6, 1.6, 0.8, -35, 12)),
    hole(disc(4.2, 10.6, 0.55)),
  );
}

function ghost(): GlyphPath {
  const body: Pt[] = [...arc(12, 11, 7, 180, 360, 14), [19, 20.5], [16.7, 18.4], [14.3, 20.8], [12, 18.4], [9.7, 20.8], [7.3, 18.4], [5, 20.5]];
  return glyph(solid(body), hole(ellipse(9.6, 11.2, 1.1, 1.6, 0, 12)), hole(ellipse(14.4, 11.2, 1.1, 1.6, 0, 12)), hole(ellipse(12, 15, 1.2, 0.8, 0, 12)));
}

function boulder(): GlyphPath {
  return glyph(
    solid([[2.8, 19.6], [4.6, 11.4], [9.4, 6.2], [15.6, 5.4], [20.2, 9.4], [21.6, 16.2], [19, 20.4], [7.2, 21.2]]),
    hole([[9.4, 6.9], [11.6, 12.4], [15.3, 6.1], [12, 13.4]]),
    hole([[5.2, 11.8], [11.2, 13.3], [6.4, 18.8], [10.4, 13.9]]),
    hole([[12.4, 14], [20.7, 15.8], [13, 15]]),
  );
}

function monkey(): GlyphPath {
  const face: Pt[] = [
    ...quad([12, 8.6], [9.6, 6.4], [7.6, 8.6]),
    ...quad([7.6, 8.6], [6.6, 11.2], [8, 13.2]).slice(1),
    ...quad([8, 13.2], [7.6, 17.6], [12, 17.6]).slice(1),
    ...quad([12, 17.6], [16.4, 17.6], [16, 13.2]).slice(1),
    ...quad([16, 13.2], [17.4, 11.2], [16.4, 8.6]).slice(1),
    ...quad([16.4, 8.6], [14.4, 6.4], [12, 8.6]).slice(1, -1),
  ];
  return glyph(
    solid(disc(3.4, 10.6, 2.8)),
    solid(disc(20.6, 10.6, 2.8)),
    hole(disc(3.4, 10.6, 1.3)),
    hole(disc(20.6, 10.6, 1.3)),
    solid(disc(12, 12.2, 6.8)),
    hole(face),
    solid(ellipse(10.2, 11, 0.9, 1.2, 0, 12)),
    solid(ellipse(13.8, 11, 0.9, 1.2, 0, 12)),
    solid(ellipse(12, 14.8, 1.8, 0.9, 0, 12)),
  );
}

function shrimp(): GlyphPath {
  const body: Pt[] = [...arc(11, 12, 8.4, -40, -320, 22), ...arc(11, 12, 4.2, -320, -40, 12)];
  const parts = [solid(body)];
  for (const at of [-95, -140, -185, -230, -275]) {
    parts.push(hole(bar(11 + 4.9 * Math.cos(at * DEG), 12 + 4.9 * Math.sin(at * DEG), 11 + 7.7 * Math.cos(at * DEG), 12 + 7.7 * Math.sin(at * DEG), 0.55)));
  }
  parts.push(hole(disc(15, 7.3, 0.75)));
  parts.push(solid([[16.4, 16.8], [21.8, 14.6], [20.8, 18], [22, 21.2]]));
  parts.push(solid(band(quad([17.4, 6.6], [19, 2.8], [23, 1.6]), 0.6)));
  parts.push(solid(band(quad([17.6, 7.4], [20.6, 5.4], [23.4, 6]), 0.6)));
  for (const at of [-300, -265, -230]) {
    const x = 11 + 4.2 * Math.cos(at * DEG);
    const y = 12 + 4.2 * Math.sin(at * DEG);
    parts.push(solid(bar(x, y, x + 1.9 * Math.cos((at + 180) * DEG) + 1.2, y + 1.9 * Math.sin((at + 180) * DEG), 0.6)));
  }
  return glyph(...parts);
}

function turtle(): GlyphPath {
  const parts = [solid(ellipse(12, 13, 6.5, 7.2, 0, 24)), solid(disc(12, 3.5, 2.1))];
  for (const [x, y, t] of [[4.4, 8.5, -30], [19.6, 8.5, 30], [4.8, 18, 30], [19.2, 18, -30]] as const) {
    parts.push(solid(ellipse(x, y, 2, 1.3, t, 12)));
  }
  parts.push(solid([[11.2, 20.7], [12.8, 20.7], [12, 22.8]]));
  const hex = (r: number): Pt[] => Array.from({ length: 6 }, (_, k): Pt => [12 + r * Math.cos((k * 60 - 90) * DEG), 13 + r * Math.sin((k * 60 - 90) * DEG)]);
  parts.push(hole(hex(3.2)), solid(hex(1.9)));
  for (let k = 0; k < 6; k += 1) {
    const a = (k * 60 - 90) * DEG;
    parts.push(hole(bar(12 + 3.7 * Math.cos(a), 13 + 3.7 * Math.sin(a), 12 + 5.3 * Math.cos(a), 13 + 6 * Math.sin(a), 0.5)));
  }
  return glyph(...parts);
}

function lizard(): GlyphPath {
  const spine = quad([12, 6], [11.4, 14], [12.6, 16.2], 6).concat(quad([12.6, 16.2], [14, 21], [20, 21.4], 8).slice(1));
  const parts = [solid(band(spine, 3.4, 0.5)), solid(ellipse(12, 4.4, 2.5, 2.9, 0, 16))];
  for (const [x1, y1, x2, y2] of [[11, 8.8, 5.6, 6.4], [13, 8.8, 18.4, 6.4], [11, 14, 5.8, 17.4], [13.2, 14, 18.6, 16.8]] as const) {
    parts.push(solid(bar(x1, y1, x2, y2, 1.4)));
    for (const spread of [-35, 0, 35]) {
      const a = Math.atan2(y2 - y1, x2 - x1) + spread * DEG;
      parts.push(solid(bar(x2, y2, x2 + 1.8 * Math.cos(a), y2 + 1.8 * Math.sin(a), 0.6)));
    }
  }
  parts.push(hole(disc(10.9, 3.6, 0.55)), hole(disc(13.1, 3.6, 0.55)));
  return glyph(...parts);
}

function grapes(): GlyphPath {
  const parts: string[] = [];
  const rows: readonly (readonly [number, readonly number[]])[] = [
    [8.8, [4.8, 9.6, 14.4, 19.2]],
    [12.9, [7.2, 12, 16.8]],
    [17, [9.6, 14.4]],
    [21.1, [12]],
  ];
  for (const [y, xs] of rows) for (const x of xs) parts.push(solid(disc(x, y, 2.3)), hole(disc(x - 0.8, y - 0.8, 0.5)));
  parts.push(solid(band(quad([12, 6.6], [12.2, 4.4], [13.2, 2.4]), 0.9)));
  parts.push(solid([...quad([13, 3.6], [15.4, 1], [20.6, 3.2]), ...quad([20.6, 3.2], [16.4, 6], [13, 3.6]).slice(1, -1)]));
  return glyph(...parts);
}

function die(): GlyphPath {
  return glyph(
    solid(roundRect(3, 3, 18, 18, 3.2)),
    ...([[7.5, 7.5], [16.5, 7.5], [12, 12], [7.5, 16.5], [16.5, 16.5]] as const).map(([x, y]) => hole(disc(x, y, 1.7))),
  );
}

function clock(): GlyphPath {
  const parts = [solid(disc(12, 12, 10)), hole(disc(12, 12, 8.2)), solid(disc(12, 12, 1.3))];
  parts.push(solid(bar(12, 11, 12, 5.6, 1.5)), solid(bar(12.8, 12.6, 16.4, 14.6, 1.3)));
  for (let k = 0; k < 12; k += 1) {
    const a = k * 30 * DEG;
    const inner = k % 3 === 0 ? 5.8 : 6.8;
    parts.push(solid(bar(12 + inner * Math.cos(a), 12 + inner * Math.sin(a), 12 + 7.7 * Math.cos(a), 12 + 7.7 * Math.sin(a), k % 3 === 0 ? 1.2 : 0.7)));
  }
  return glyph(...parts);
}

function padlock(): GlyphPath {
  const shackle = [...arc(12, 7.4, 4.9, 180, 360, 12), [16.9, 10.6], [14.6, 10.6], ...arc(12, 7.4, 2.6, 0, -180, 10), [9.4, 10.6], [7.1, 10.6]] as Pt[];
  const keyhole: Pt[] = [...arc(12, 14.6, 1.7, 120, 420, 12), [13.2, 19], [10.8, 19]];
  return glyph(solid(shackle), solid(roundRect(4.6, 10.6, 14.8, 11.2, 1.8)), hole(keyhole));
}

function brokenRune(): GlyphPath {
  return glyph(
    solid([[11, 2], [13, 2], [13, 9.6], [11, 10.8]]),
    solid([[11, 13], [13, 11.8], [13, 22], [11, 22]]),
    solid([[13, 4.6], [19, 9.1], [17.8, 10.7], [13, 7.1]]),
    solid([[11, 15.2], [5, 19.7], [6.2, 21.3], [11, 17.7]]),
    solid([[15.6, 13.4], [19.4, 12.2], [18.2, 15.4]]),
    solid([[5.6, 6.8], [8.8, 5.4], [8, 8.6]]),
    solid(bar(3.2, 20.8, 20.8, 3.2, 1.3)),
  );
}

function house(): GlyphPath {
  return glyph(
    solid([[2.4, 11.8], [12, 3], [21.6, 11.8], [20.2, 13.2], [12, 5.8], [3.8, 13.2]]),
    solid([[5.8, 12.4], [12, 6.8], [18.2, 12.4], [18.2, 21.6], [5.8, 21.6]]),
    hole([[10.3, 21.6], ...arc(12, 16.6, 1.7, 180, 360, 8), [13.7, 21.6]]),
    hole(rect(14.4, 13.4, 2.4, 2.4)),
    hole(rect(7.2, 13.4, 2.4, 2.4)),
    solid([[15.4, 3.2], [17.6, 3.2], [17.6, 7.4], [15.4, 5.4]]),
  );
}

function bomb(): GlyphPath {
  return glyph(
    solid(disc(10.5, 14.5, 7)),
    hole(ellipse(7.8, 11.9, 1.1, 2.3, 40, 12)),
    solid(turn(rect(15.1, 7, 3.2, 3.2), 45, 16.7, 8.6)),
    solid(band(quad([17.4, 7.6], [18.4, 4.2], [20.6, 3.6]), 0.9)),
    solid(sparkle(21.3, 2.9, 2.6)),
  );
}

function cross(): GlyphPath {
  return glyph(
    solid([[9.2, 2.8], [14.8, 2.8], [14.8, 9.2], [21.2, 9.2], [21.2, 14.8], [14.8, 14.8], [14.8, 21.2], [9.2, 21.2], [9.2, 14.8], [2.8, 14.8], [2.8, 9.2], [9.2, 9.2]]),
    hole([[10.8, 4.4], [13.2, 4.4], [13.2, 10.8], [19.6, 10.8], [19.6, 13.2], [13.2, 13.2], [13.2, 19.6], [10.8, 19.6], [10.8, 13.2], [4.4, 13.2], [4.4, 10.8], [10.8, 10.8]]),
    solid(rect(11.6, 5.2, 0.8, 13.6)),
    solid(rect(5.2, 11.6, 13.6, 0.8)),
  );
}

function bone(degrees: number): string[] {
  const shaft = turn(rect(4.6, 11, 14.8, 2), degrees);
  const knobs = [[4.4, 10.4], [4.4, 13.6], [19.6, 10.4], [19.6, 13.6]].map(([x, y]) => solid(turn(disc(x ?? 0, y ?? 0, 1.8), degrees)));
  return [solid(shaft), ...knobs];
}

function bones(): GlyphPath {
  return glyph(...bone(45), ...bone(-45));
}

function gust(): GlyphPath {
  const first: Pt[] = [[2, 8.4], [14.6, 8.4], ...arc(14.6, 5.4, 3, 90, -180, 12).slice(1)];
  const second: Pt[] = [[1.6, 13.2], [17.6, 13.2], ...arc(17.6, 16, 2.8, -90, 180, 12).slice(1)];
  return glyph(solid(band(first, 1.7, 1)), solid(band(second, 1.7, 1)), solid(band([[6.4, 18.6], [13, 18.6]], 1.6)));
}

function drum(): GlyphPath {
  // The shell runs from the drumhead's front rim down to its own curved foot.
  const shell: Pt[] = [...ellipseArc(12, 9, 8, 3, 0, 180), ...ellipseArc(12, 17, 8, 3, 180, 0)];
  return glyph(
    solid(shell),
    solid(ellipse(12, 9, 8, 3, 0, 24)),
    hole(ellipse(12, 9, 6.4, 1.8, 0, 20)),
    hole(band([[5.4, 12.6], [8, 17], [10.6, 13], [13.4, 17.4], [16, 13], [18.6, 16.6]], 0.8)),
    solid(bar(3, 1.8, 8.6, 5.2, 1)),
    solid(disc(8.9, 5.3, 0.9)),
    solid(bar(21, 1.8, 15.4, 5.2, 1)),
    solid(disc(15.1, 5.3, 0.9)),
  );
}

function sun(): GlyphPath {
  const parts = [solid(disc(12, 12, 5.2))];
  for (let k = 0; k < 12; k += 1) {
    const a = k * 30 * DEG;
    const outer = k % 2 === 0 ? 11 : 9.2;
    const side = 1.5 * (k % 2 === 0 ? 1 : 0.8);
    parts.push(
      solid([
        [12 + 6.6 * Math.cos(a) - side * Math.sin(a), 12 + 6.6 * Math.sin(a) + side * Math.cos(a)],
        [12 + outer * Math.cos(a), 12 + outer * Math.sin(a)],
        [12 + 6.6 * Math.cos(a) + side * Math.sin(a), 12 + 6.6 * Math.sin(a) - side * Math.cos(a)],
      ]),
    );
  }
  return glyph(...parts);
}

function snake(): GlyphPath {
  const line = [
    ...quad([3, 20.6], [9, 22.6], [12.4, 18.8], 6),
    ...quad([12.4, 18.8], [15.4, 15], [10, 12.6], 6).slice(1),
    ...quad([10, 12.6], [5, 10.2], [9.4, 6.6], 6).slice(1),
    ...quad([9.4, 6.6], [11.6, 5], [13.8, 5.2], 4).slice(1),
  ];
  return glyph(
    solid(band(line, 1, 3.2)),
    solid(ellipse(15.4, 5, 2.9, 2.2, -8, 16)),
    solid([[18, 4.8], [21.4, 3.8], [20.4, 4.9], [21.6, 5.9]]),
    hole(disc(15.8, 4.2, 0.55)),
  );
}

function bat(): GlyphPath {
  const half: Pt[] = [[12, 9.4], [13.2, 6.6], [13.8, 9.2], [16, 8.2], [19.6, 6.8], [23.2, 8.6], [21.6, 11.4], [20.2, 10.8], [18.8, 13.8], [17, 12.6], [15.2, 15.4], [12, 17.2]];
  const whole: Pt[] = [...half, ...mirror(half).reverse().slice(1, -1)];
  return glyph(solid(whole), hole(disc(11, 11, 0.55)), hole(disc(13, 11, 0.55)));
}

function mushroom(): GlyphPath {
  const cap: Pt[] = [...arc(12, 12.4, 9.6, 180, 360, 16), ...quad([21.6, 12.4], [12, 15], [2.4, 12.4]).slice(1, -1)];
  const stem: Pt[] = [...quad([9, 14.3], [12, 14.9], [15, 14.3]), [15.8, 21], ...quad([15.8, 21], [12, 22.6], [8.2, 21]).slice(1)];
  return glyph(solid(cap), solid(stem), hole(disc(7, 8.4, 1.6)), hole(disc(12.6, 5.8, 1.3)), hole(disc(16.8, 9.2, 1.7)), hole(disc(11.4, 10.8, 1.1)));
}

function portal(): GlyphPath {
  const spiral: Pt[] = [];
  for (let k = 0; k <= 36; k += 1) {
    const t = k / 36;
    const a = t * 540 * DEG;
    const r = 0.8 + 4.4 * t;
    spiral.push([12 + r * Math.cos(a), 12 + r * 1.3 * Math.sin(a)]);
  }
  return glyph(solid(ellipse(12, 12, 8.2, 10.6, 0, 28)), hole(ellipse(12, 12, 6.6, 8.9, 0, 28)), solid(band(spiral, 0.5, 1.4)));
}

function brain(): GlyphPath {
  return glyph(
    solid(bar(13.6, 16.6, 15.2, 22.4, 2)),
    solid(scalloped(12, 11, 9.8, 7.6, 12, 0.12)),
    hole(band(quad([12, 3.8], [10.8, 11], [12.2, 18.2]), 0.8)),
    hole(band([[4.6, 9.4], [6.8, 8.2], [8, 10.4], [9.8, 9.6]], 0.7)),
    hole(band([[4.4, 13.6], [6.6, 14.6], [8.2, 12.6], [10, 14]], 0.7)),
    hole(band([[14.2, 8.8], [16, 10.4], [17.6, 8.2], [19.6, 9.4]], 0.7)),
    hole(band([[14, 14.4], [15.8, 12.8], [17.8, 14.8], [19.8, 13.2]], 0.7)),
  );
}

function target(): GlyphPath {
  const parts = [solid(disc(12, 12, 9)), hole(disc(12, 12, 7.4)), solid(disc(12, 12, 4.4)), hole(disc(12, 12, 2.9)), solid(disc(12, 12, 1.2))];
  for (let k = 0; k < 4; k += 1) {
    const a = k * 90;
    parts.push(solid(turn(rect(11.35, 0.6, 1.3, 2.4), a)), solid(turn(rect(11.35, 4.6, 1.3, 2.8), a)));
  }
  return glyph(...parts);
}

function heart(): GlyphPath {
  const outline: Pt[] = [
    [12, 21],
    ...quad([12, 21], [2.4, 14.2], [2.6, 8.4], 8).slice(1),
    ...arc(7.7, 8, 5.1, 175, 305, 8),
    [12, 5.6],
    ...arc(16.3, 8, 5.1, 235, 365, 8),
    ...quad([21.4, 8.4], [21.6, 14.2], [12, 21], 8).slice(1, -1),
  ];
  return glyph(solid(outline), hole(ellipse(7.4, 8.2, 1.1, 2, 35, 12)));
}

function flag(): GlyphPath {
  const cloth: Pt[] = [
    ...quad([5.6, 4.2], [10, 1.8], [14.4, 4.6], 6),
    ...quad([14.4, 4.6], [18, 6.4], [21.4, 4.4], 6).slice(1),
    [21.4, 13.4],
    ...quad([21.4, 13.4], [18, 15.4], [14.4, 13.6], 6).slice(1),
    ...quad([14.4, 13.6], [10, 10.8], [5.6, 13.2], 6).slice(1),
  ];
  return glyph(solid(rect(4, 3.6, 1.6, 18.8)), solid(disc(4.8, 2.4, 1.3)), solid(cloth), hole(sparkle(13.2, 8.8, 2.8)));
}

function halo(): GlyphPath {
  const shoulders: Pt[] = [...ellipseArc(12, 22.6, 8.4, 7.4, 180, 360, 16)];
  return glyph(
    solid(ellipse(12, 4.2, 7.8, 2.9, 0, 24)),
    hole(ellipse(12, 4.2, 6, 1.5, 0, 20)),
    solid(disc(12, 11.4, 3.8)),
    solid(shoulders),
  );
}

function gear(): GlyphPath {
  const outline: Pt[] = [];
  const teeth = 9;
  for (let k = 0; k < teeth; k += 1) {
    const base = (k * 360) / teeth;
    for (const [offset, r] of [[-13, 7.6], [-8, 10.4], [8, 10.4], [13, 7.6]] as const) {
      outline.push([12 + r * Math.cos((base + offset) * DEG), 12 + r * Math.sin((base + offset) * DEG)]);
    }
  }
  return glyph(solid(outline), hole(disc(12, 12, 3.4)), solid(disc(12, 12, 1.4)));
}

function leaf(): GlyphPath {
  const blade: Pt[] = [...quad([4.4, 19.6], [4, 5], [21.2, 2.8], 12), ...quad([21.2, 2.8], [19.6, 19.6], [4.4, 19.6], 12).slice(1, -1)];
  const parts = [solid(blade), solid(bar(2.4, 22, 5.4, 18.6, 1.1)), hole(band(quad([6, 18], [11, 12], [18.8, 5]), 0.7))];
  for (const [x, y] of [[9.4, 14.6], [12.6, 11], [15.6, 7.8]] as const) {
    parts.push(hole(bar(x, y, x - 2.4, y - 3.6, 0.5)), hole(bar(x + 0.6, y + 0.4, x + 3.8, y + 1.8, 0.5)));
  }
  return glyph(...parts);
}

function arrowArc(from: number, to: number): string[] {
  const line = arc(12, 12, 7.2, from, to, 12);
  const end = line[line.length - 1] ?? [0, 0];
  const a = to * DEG;
  const tangent = Math.sign(to - from);
  const ux = -Math.sin(a) * tangent;
  const uy = Math.cos(a) * tangent;
  const rx = Math.cos(a);
  const ry = Math.sin(a);
  return [
    solid(band(line, 2.2)),
    solid([
      [end[0] + rx * 2.6, end[1] + ry * 2.6],
      [end[0] + ux * 3.4, end[1] + uy * 3.4],
      [end[0] - rx * 2.6, end[1] - ry * 2.6],
    ]),
  ];
}

function cycle(): GlyphPath {
  return glyph(...arrowArc(195, 330), ...arrowArc(15, 150));
}

function echo(): GlyphPath {
  const parts = [solid(disc(5, 12, 2.2))];
  for (const r of [6, 10, 14]) parts.push(solid(band(arc(5, 12, r, -48, 48, 10), 1.6)));
  return glyph(...parts);
}

function helix(): GlyphPath {
  const strand = (phase: number): Pt[] =>
    Array.from({ length: 21 }, (_, k): Pt => [12 + 6 * Math.sin(((k / 20) * 2.5 * Math.PI) + phase), 2 + k]);
  const parts = [solid(band(strand(0), 1.5)), solid(band(strand(Math.PI), 1.5))];
  for (let k = 1; k < 20; k += 2) {
    const t = (k / 20) * 2.5 * Math.PI;
    const a = 12 + 6 * Math.sin(t);
    const b = 12 + 6 * Math.sin(t + Math.PI);
    if (Math.abs(a - b) > 4) parts.push(solid(bar(Math.min(a, b) + 0.6, 2 + k, Math.max(a, b) - 0.6, 2 + k, 0.8)));
  }
  return glyph(...parts);
}

function jaws(): GlyphPath {
  const upper: Pt[] = [...quad([2.4, 10], [12, 1], [21.6, 10], 10), [18.8, 10], [17.4, 13], [16, 10], [13.4, 10], [12, 13.2], [10.6, 10], [8, 10], [6.6, 13], [5.2, 10]];
  const lower: Pt[] = [[2.4, 14], [7.8, 14], [9.2, 11], [10.6, 14], [13.4, 14], [14.8, 11], [16.2, 14], [21.6, 14], ...quad([21.6, 14], [12, 22.6], [2.4, 14], 10).slice(1, -1)];
  return glyph(solid(upper), solid(lower));
}

function snowflake(): GlyphPath {
  const parts = [solid(disc(12, 12, 1.8))];
  for (let k = 0; k < 6; k += 1) {
    const a = k * 60 - 90;
    parts.push(solid(turn(rect(11.2, 1.6, 1.6, 10.4), a + 90)));
    for (const side of [-1, 1]) {
      parts.push(solid(turn(bar(12, 6.2, 12 + side * 3, 3.4, 1.1), a + 90)));
    }
  }
  return glyph(...parts);
}

function infinity(): GlyphPath {
  const loop: Pt[] = [];
  for (let k = 0; k <= 48; k += 1) {
    const t = (k / 48) * 2 * Math.PI;
    const d = 1 + Math.sin(t) * Math.sin(t);
    loop.push([12 + (10 * Math.cos(t)) / d, 12 + (10 * Math.sin(t) * Math.cos(t)) / d]);
  }
  return glyph(solid(band(loop, 2.4)));
}

function crate(): GlyphPath {
  return glyph(
    solid(roundRect(3.4, 3.4, 17.2, 17.2, 1)),
    hole(rect(5.6, 5.6, 12.8, 12.8)),
    solid([[5.6, 16.2], [16.2, 5.6], [18.4, 5.6], [18.4, 7.8], [7.8, 18.4], [5.6, 18.4]]),
    solid(rect(5.6, 11.4, 12.8, 1.2)),
  );
}

function sword(degrees: number): string[] {
  return [
    solid(turn([[12, 1.4], [13.3, 3.4], [13.3, 15.4], [10.7, 15.4], [10.7, 3.4]], degrees)),
    solid(turn(rect(7.6, 15.4, 8.8, 1.6), degrees)),
    solid(turn(rect(11.2, 17, 1.6, 3.4), degrees)),
    solid(turn(disc(12, 21.4, 1.3), degrees)),
  ];
}

function blades(): GlyphPath {
  return glyph(...sword(40), ...sword(-40));
}

function flask(): GlyphPath {
  const body: Pt[] = [[9.4, 2.6], [14.6, 2.6], [14.6, 4.2], [13.6, 4.2], [13.6, 9], [20.2, 19.2], [19.4, 21.6], [4.6, 21.6], [3.8, 19.2], [10.4, 9], [10.4, 4.2], [9.4, 4.2]];
  return glyph(
    solid(body),
    hole([[11.6, 5.2], [12.4, 5.2], [12.4, 9.6], [15.2, 14], [8.8, 14], [11.6, 9.6]]),
    hole(disc(10.4, 17.2, 1)),
    hole(disc(14.2, 16, 0.8)),
    hole(disc(13, 19.2, 0.7)),
  );
}

function pan(cx: number): string[] {
  const bowl: Pt[] = [[cx - 4.2, 13.2], [cx + 4.2, 13.2], ...arc(cx, 13.2, 4.2, 0, 180, 10).slice(1, -1)];
  return [solid(bowl), solid(bar(cx, 6, cx - 3.8, 13.2, 0.6)), solid(bar(cx, 6, cx + 3.8, 13.2, 0.6))];
}

function scales(): GlyphPath {
  return glyph(
    solid(rect(11.3, 4, 1.4, 16.4)),
    solid([[8, 20.2], [16, 20.2], [17.2, 22.4], [6.8, 22.4]]),
    solid(rect(3.6, 5.4, 16.8, 1.2)),
    solid(disc(12, 3.8, 1.4)),
    ...pan(4.6),
    ...pan(19.4),
  );
}

function chevrons(): GlyphPath {
  const chevron = (y: number): Pt[] => [[12, y], [20.4, y + 7], [17.6, y + 9.2], [12, y + 4.4], [6.4, y + 9.2], [3.6, y + 7]];
  return glyph(solid(chevron(2.2)), solid(chevron(11.6)));
}

function bubbles(): GlyphPath {
  const parts: string[] = [];
  for (const [x, y, r] of [[8.4, 15.4, 5.2], [16.6, 8.4, 3.8], [18.2, 17.4, 2.6], [6.6, 5.4, 2]] as const) {
    parts.push(solid(disc(x, y, r)), hole(disc(x, y, r - 1.1)), solid(ellipse(x - r * 0.45, y - r * 0.45, r * 0.22, r * 0.34, 40, 10)));
  }
  return glyph(...parts);
}

function mask(): GlyphPath {
  const face: Pt[] = [...quad([3, 6], [12, 2.2], [21, 6], 8), ...quad([21, 6], [21.4, 17.6], [12, 21.8], 8).slice(1), ...quad([12, 21.8], [2.6, 17.6], [3, 6], 8).slice(1, -1)];
  const mouth: Pt[] = [...quad([8, 15], [12, 18.6], [16, 15], 6), ...quad([16, 15], [12, 16.6], [8, 15], 6).slice(1, -1)];
  return glyph(solid(face), hole([[6.2, 9.4], [10.2, 8.6], [10, 11.4], [6.8, 11.6]]), hole([[17.8, 9.4], [13.8, 8.6], [14, 11.4], [17.2, 11.6]]), hole(mouth));
}

function horns(): GlyphPath {
  const horn: Pt[] = [[9.6, 12.6], ...quad([9.6, 12.6], [3.6, 11.6], [3.2, 2.6], 8).slice(1), ...quad([3.2, 2.6], [5.8, 8.4], [11.6, 9.8], 8).slice(1)];
  return glyph(solid(horn), solid(mirror(horn)), solid(ellipse(12, 16.2, 6.2, 5.6, 0, 24)), hole([[8.6, 14.8], [11, 15.6], [8.8, 16.6]]), hole([[15.4, 14.8], [13, 15.6], [15.2, 16.6]]));
}

function note(): GlyphPath {
  return glyph(
    solid(ellipse(7.2, 18.4, 3.4, 2.5, -20, 16)),
    solid(ellipse(17.4, 16.2, 3.4, 2.5, -20, 16)),
    solid(rect(9.4, 4.8, 1.4, 13.6)),
    solid(rect(19.6, 2.6, 1.4, 13.6)),
    solid([[9.4, 4.8], [21, 2.2], [21, 5.4], [9.4, 8]]),
  );
}

function cards(): GlyphPath {
  const card = (degrees: number): string[] => [
    solid(turn(roundRect(8.2, 5, 7.6, 12, 1), degrees, 12, 34)),
    hole(turn([[12, 8.2], [14, 11], [12, 13.8], [10, 11]], degrees, 12, 34)),
  ];
  return glyph(...card(-22), ...card(0), ...card(22));
}

function candy(): GlyphPath {
  return glyph(
    solid(ellipse(12, 12, 5.8, 4.4, -30, 20)),
    solid(turn([[5.6, 12], [1.8, 8.6], [2.4, 15.6]], -30)),
    solid(turn([[18.4, 12], [22.2, 8.6], [21.6, 15.6]], -30)),
    hole(turn(band([[8.6, 14.6], [10.6, 9.2]], 0.9), -30)),
    hole(turn(band([[12.4, 14.8], [14.4, 9.4]], 0.9), -30)),
  );
}

function bread(): GlyphPath {
  const loaf: Pt[] = [...arc(7.6, 10.4, 5.2, 150, 270, 8), ...arc(16.4, 10.4, 5.2, 270, 390, 8), [21, 12.8], [20.4, 20.6], [3.6, 20.6], [3, 12.8]];
  const parts = [solid(loaf)];
  for (const x of [8, 12, 16]) parts.push(hole(bar(x - 1.2, 9.6, x + 1.2, 6.8, 0.8)));
  return glyph(...parts);
}

function jet(): GlyphPath {
  return glyph(
    solid([[12, 1.4], [13.4, 5], [13.6, 9.6], [21.6, 15.6], [21.6, 17.4], [13.6, 14.6], [13.2, 19], [16.4, 21.4], [16.4, 22.6], [12, 21.6], [7.6, 22.6], [7.6, 21.4], [10.8, 19], [10.4, 14.6], [2.4, 17.4], [2.4, 15.6], [10.4, 9.6], [10.6, 5]]),
    hole([[12, 4.2], [12.7, 6.8], [11.3, 6.8]]),
  );
}

function web(): GlyphPath {
  const parts: string[] = [];
  const spokes = 6;
  const angle = (k: number): number => (k * 60 + 30) * DEG;
  for (let k = 0; k < spokes; k += 1) parts.push(solid(bar(12, 12, 12 + 10.8 * Math.cos(angle(k)), 12 + 10.8 * Math.sin(angle(k)), 1)));
  for (const r of [4.6, 8.6]) {
    for (let k = 0; k < spokes; k += 1) {
      parts.push(solid(bar(12 + r * Math.cos(angle(k)), 12 + r * Math.sin(angle(k)), 12 + r * Math.cos(angle(k + 1)), 12 + r * Math.sin(angle(k + 1)), 1)));
    }
  }
  parts.push(solid(bar(17.6, 1.4, 17.6, 12.4, 0.8)), solid(disc(17.6, 15, 2.4)), solid(disc(17.6, 12, 1.3)));
  return glyph(...parts);
}

export const MOTIF_GLYPHS: Readonly<Record<MotifGlyph, GlyphPath>> = {
  tome: tome(),
  quill: quill(),
  pancakes: pancakes(),
  spatula: spatula(),
  drop: drop(),
  chip: chip(),
  neural: neural(),
  spore: spore(),
  sheep: sheep(),
  ghost: ghost(),
  boulder: boulder(),
  monkey: monkey(),
  shrimp: shrimp(),
  turtle: turtle(),
  lizard: lizard(),
  grapes: grapes(),
  die: die(),
  clock: clock(),
  padlock: padlock(),
  "broken-rune": brokenRune(),
  house: house(),
  bomb: bomb(),
  cross: cross(),
  bones: bones(),
  gust: gust(),
  drum: drum(),
  sun: sun(),
  snake: snake(),
  bat: bat(),
  mushroom: mushroom(),
  portal: portal(),
  brain: brain(),
  target: target(),
  heart: heart(),
  flag: flag(),
  halo: halo(),
  gear: gear(),
  leaf: leaf(),
  cycle: cycle(),
  echo: echo(),
  helix: helix(),
  jaws: jaws(),
  snowflake: snowflake(),
  infinity: infinity(),
  crate: crate(),
  blades: blades(),
  flask: flask(),
  scales: scales(),
  chevrons: chevrons(),
  bubbles: bubbles(),
  mask: mask(),
  horns: horns(),
  note: note(),
  cards: cards(),
  candy: candy(),
  bread: bread(),
  jet: jet(),
  web: web(),
};
