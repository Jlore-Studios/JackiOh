// @@ D objects
type DkSeg = readonly ["L", number, number] | readonly ["Q", number, number, number, number];

/** A closed path from `start` through straight and quadratic segments, in the part's frame. */
function dkPath(f: Local, start: Point, segs: readonly DkSeg[]): string {
  let d = `M${f.p(start[0], start[1])}`;
  for (const s of segs) d += s[0] === "L" ? `L${f.p(s[1], s[2])}` : `Q${f.p(s[1], s[2])} ${f.p(s[3], s[4])}`;
  return `${d}Z`;
}

function dkBox(x0: number, y0: number, x1: number, y1: number): Point[] {
  return [[x0, y0], [x1, y0], [x1, y1], [x0, y1]];
}

function dkRot(points: readonly Point[], degrees: number): Point[] {
  return points.map((p) => turned(p, degrees));
}

/** An ellipse whose axis is tilted `degrees` (as drawn facing right). */
function dkEll(f: Local, flip: number, cx: number, cy: number, rx: number, ry: number, degrees: number): string {
  const c = Math.cos(degrees * DEGREES);
  const s = Math.sin(degrees * DEGREES);
  const a = f.p(cx - rx * c, cy - rx * s);
  const arc = `A${fmt(f.r(rx))} ${fmt(f.r(ry))} ${fmt(flip * degrees)} 1 0`;
  return `M${a}${arc} ${f.p(cx + rx * c, cy + rx * s)}${arc} ${a}Z`;
}

/** A round ring: the circle and a hole inside it. */
function dkRing(f: Local, cx: number, cy: number, r: number, inner: number): string {
  return f.circle(cx, cy, r) + f.circle(cx, cy, inner);
}

/** A swirl arm about an ellipse's centre, `sx` squashing its width. */
function dkSwirl(f: Local, cx: number, cy: number, sx: number, start: number, sweep: number, r0: number, r1: number, width: number): string {
  const steps = 8;
  const outer: Point[] = [];
  const inner: Point[] = [];
  for (let i = 0; i <= steps; i += 1) {
    const t = i / steps;
    const r = r0 + (r1 - r0) * t;
    const half = (width * (1 - t)) / 2 + 0.15;
    const angle = (start + sweep * t) * DEGREES;
    outer.push([cx + sx * (r + half) * Math.cos(angle), cy + (r + half) * Math.sin(angle)]);
    inner.push([cx + sx * Math.max(0.3, r - half) * Math.cos(angle), cy + Math.max(0.3, r - half) * Math.sin(angle)]);
  }
  return f.poly([...outer, ...inner.reverse()]);
}

/** A playing card upright on the anchor, 6 wide and 8.4 tall at scale 1, tilted a little. */
function cardPart(at: PartPlace, rng: Rng): KitShape[] {
  const f = local(at);
  const t = between(rng, -9, 9);
  const rect = (w: number, h: number): string => f.poly(dkRot(dkBox(-w / 2, -h / 2, w / 2, h / 2), t));
  return [
    shape(rect(6, 8.4), "blue-800", 1, MAIN_INKED),
    shape(rect(6, 8.4) + rect(4.8, 7.2), "white", 1),
    shape(f.poly(dkRot([[0, -1.9], [1.2, 0], [0, 1.9], [-1.2, 0]], t)), "cyan-400", 1),
  ];
}

/** A fan of five cards, base at the anchor, about 17 wide and 9 tall at scale 1. */
function cardFanPart(at: PartPlace, rng: Rng): KitShape[] {
  const f = local(at);
  const out: KitShape[] = [];
  const diamonds: string[] = [];
  const tilt = between(rng, -3, 3);
  for (let i = 0; i < 5; i += 1) {
    const a = (i - 2) * 26 + tilt + between(rng, -1.5, 1.5);
    out.push(shape(f.poly(dkRot(dkBox(-3, -8.4, 3, 0), a)), "white", 1, INKED));
    out.push(shape(f.poly(dkRot(dkBox(-2.4, -7.8, 2.4, -0.6), a)), "blue-800", 1, MAIN));
    diamonds.push(f.poly(dkRot([[0, -7.2], [1.1, -6], [0, -4.8], [-1.1, -6]], a)));
  }
  out.push(shape(diamonds.join(""), "cyan-400", 1));
  return out;
}

/** A stack of cards on the anchor, 6 wide and 4 tall at scale 1. */
function deckPart(at: PartPlace, rng: Rng): KitShape[] {
  const f = local(at);
  const skew = between(rng, -0.3, 0.3);
  const lines = [-1.6, -1, -0.4].map((y) => f.band([[-2.9 + between(rng, 0, 0.3), y], [2.9, y]], 0.22)).join("");
  const top = (inset: number): string => f.poly([[-2.4 + skew + inset, -4 + inset], [3 + skew - inset, -4 + inset], [3 - inset, -2.2 - inset], [-3 + inset, -2.2 - inset]]);
  return [
    shape(f.poly(dkBox(-3, -2.2, 3, 0)), "white", 1, INKED),
    shape(lines, "slate-400", 0.9),
    shape(top(0), "white", 1),
    shape(top(0.35), "blue-800", 1, MAIN),
    shape(f.poly([[0.1 + skew, -3.75], [0.8 + skew, -3.1], [0.1 + skew, -2.45], [-0.6 + skew, -3.1]]), "cyan-400", 1),
  ];
}

/** A silver coin centred on the anchor, radius 2.5 at scale 1, face-on or edge-on. */
function coinPart(at: PartPlace, rng: Rng): KitShape[] {
  const f = local(at);
  if (rng() > 0.25) {
    return [
      shape(f.circle(0, 0, 2.5), "slate-400", 1, INKED),
      shape(f.circle(0, 0, 2.05), "slate-100", 1, MAIN),
      shape(dkRing(f, 0, 0, 1.45, 1.15), "slate-400", 0.8),
      shape(dkEll(f, at.flip, -0.9, -0.9, 0.55, 0.22, -45), "white", 0.85),
    ];
  }
  return [
    shape(f.ellipse(0, 0, 0.8, 2.5), "slate-400", 1, INKED),
    shape(f.ellipse(-0.15, 0, 0.4, 2.2), "slate-100", 1, MAIN),
    shape(f.band([[-0.3, -1.7], [-0.3, -0.7]], 0.25), "white", 0.85),
  ];
}

/** A stack of four silver coins on the anchor, 5 wide and 8 tall at scale 1. */
function coinStackPart(at: PartPlace, rng: Rng): KitShape[] {
  const f = local(at);
  const out: KitShape[] = [];
  const seams: string[] = [];
  const glints: string[] = [];
  let top = { x: 0, y: 0 };
  for (let k = 0; k < 4; k += 1) {
    const x = between(rng, -0.3, 0.3);
    const yb = -0.8 - 1.6 * k;
    const yt = yb - 1.6;
    const arc = `A${fmt(f.r(2.5))} ${fmt(f.r(0.8))} 0 0 ${f.sweep(0)}`;
    out.push(shape(`M${f.p(x - 2.5, yt)}L${f.p(x - 2.5, yb)}${arc} ${f.p(x + 2.5, yb)}L${f.p(x + 2.5, yt)}Z`, "slate-400", 1, MAIN_INKED));
    const rim: Point[] = [];
    for (let i = 0; i <= 4; i += 1) rim.push([x - 2.5 * Math.cos((Math.PI * i) / 4), yb + 0.8 * Math.sin((Math.PI * i) / 4) + 0.15]);
    seams.push(f.band(rim, 0.3));
    glints.push(f.band([[x - 1.7, yt + 0.4], [x - 1.7, yb - 0.1]], 0.35));
    top = { x, y: yt };
  }
  out.push(shape(seams.join(""), "slate-900", 0.35));
  out.push(shape(f.ellipse(top.x, top.y, 2.5, 0.8), "slate-100", 1, MAIN));
  out.push(shape(f.ellipse(top.x, top.y, 1.5, 0.5) + f.ellipse(top.x, top.y, 1.2, 0.35), "slate-400", 0.6));
  out.push(shape(glints.join(""), "white", 0.6));
  return out;
}

/** A table on the anchor (ground level), 30 wide and 12 tall at scale 1, seen in slight perspective. */
function tablePart(at: PartPlace, rng: Rng): KitShape[] {
  const f = local(at);
  const legs = [
    ...[[-14, -12.4], [12.4, 14]].map(([a, b]) => dkBox(a ?? 0, -7.4, b ?? 0, 0)),
    ...[[-11.6, -10.3], [10.3, 11.6]].map(([a, b]) => dkBox(a ?? 0, -8.8, b ?? 0, -1.4)),
  ];
  const lean = between(rng, -0.4, 0.4);
  return [
    shape(f.ellipse(0, 0.2, 16.5, 1.4), "slate-900", 0.25),
    shape(legs.map((l) => f.poly(l)).join(""), "wood-700", 1, INKED),
    shape(f.poly(dkBox(-14.4, -8.8, 14.4, -7.4)), "wood-700", 1),
    shape(f.poly([[-12.5 + lean, -12], [12.5 + lean, -12], [15, -8.8], [-15, -8.8]]), "wood-600", 1, MAIN_INKED),
    shape(f.band([[-11, -10.9], [11, -10.9]], 0.3) + f.band([[-13, -9.8], [13, -9.8]], 0.3), "slate-900", 0.15),
    shape(f.band([[-15, -8.7], [15, -8.7]], 0.55), "white", 0.35),
  ];
}

/** A wooden chair on the anchor, 8 wide and 14 tall at scale 1, seen from the front. */
function chairPart(at: PartPlace, rng: Rng): KitShape[] {
  const f = local(at);
  const slat = between(rng, 0, 0.3);
  const parts = [
    dkBox(-4, -6.5, 4, -5), dkBox(-3.9, -5, -2.9, 0), dkBox(2.9, -5, 3.9, 0),
    dkBox(-3.9, -14, -2.9, -6.5), dkBox(2.9, -14, 3.9, -6.5),
    dkBox(-2.9, -13.2 + slat, 2.9, -12.1 + slat), dkBox(-2.9, -10.6, 2.9, -9.6),
  ];
  return [
    shape(parts.map((p) => f.poly(p)).join(""), "wood-700", 1, MAIN_INKED),
    shape(f.band([[-3.4, -13.6], [-3.4, -7]], 0.3) + f.band([[-3.8, -6.2], [3.8, -6.2]], 0.35), "white", 0.3),
  ];
}

/** A big closed book lying on the anchor, 10 wide and 4 tall at scale 1, spine to the left. */
function tomePart(at: PartPlace, rng: Rng): KitShape[] {
  const f = local(at);
  const lock = between(rng, -2.4, -1.6);
  return [
    shape(f.ellipse(0.2, 0, 5.6, 0.5), "slate-900", 0.25),
    shape(f.poly(dkBox(-4.5, -3.2, 4.9, -0.8)), "slate-100", 1, INKED),
    shape([-2.6, -2, -1.4].map((y) => f.band([[-4.2, y], [4.9, y]], 0.12)).join(""), "slate-400", 0.5),
    shape(f.poly(dkBox(-4.5, -4, 5.2, -3.2)) + f.poly(dkBox(-4.5, -0.8, 5.2, 0)) + f.poly(dkBox(-5.3, -4, -4.5, 0)), "red-900", 1, MAIN_INKED),
    shape([-3.6, -2.4].map((x) => f.poly(dkBox(x, -4, x + 0.6, -3.2)) + f.poly(dkBox(x, -0.8, x + 0.6, 0))).join(""), "brass-600", 0.9),
    shape(f.poly(dkBox(4.4, -4.2, 5.4, 0.2)), "slate-400", 1, INKED),
    shape(f.circle(4.9, lock, 0.5), "slate-100", 1),
    shape(f.band([[-4.4, -3.85], [4.8, -3.85]], 0.25), "white", 0.25),
  ];
}

/** A glass jar on the anchor, 5 wide and 7 tall at scale 1, filled with something red. */
function jarPart(at: PartPlace, rng: Rng): KitShape[] {
  const f = local(at);
  const level = between(rng, -4.2, -3.4);
  const wave = between(rng, 0.3, 0.6);
  return [
    shape(dkPath(f, [-2.2, level], [["Q", -1.1, level - wave, 0, level], ["Q", 1.1, level + wave, 2.2, level], ["L", 2.2, -0.8], ["Q", 2.2, -0.2, 1.6, -0.2], ["L", -1.6, -0.2], ["Q", -2.2, -0.2, -2.2, -0.8]]), "red-500", 0.7, MAIN),
    shape(dkPath(f, [-1.8, -6], [["L", -1.8, -5.5], ["Q", -2.5, -5, -2.5, -3.5], ["L", -2.5, -0.8], ["Q", -2.5, 0, -1.7, 0], ["L", 1.7, 0], ["Q", 2.5, 0, 2.5, -0.8], ["L", 2.5, -3.5], ["Q", 2.5, -5, 1.8, -5.5], ["L", 1.8, -6]]), "sky-300", 0.35, INKED),
    shape(f.poly(dkBox(-2.1, -7, 2.1, -6)), "slate-400", 1, INKED),
    shape(f.band([[-1.7, -4.6], [-1.8, -1.6]], 0.5), "white", 0.5),
  ];
}

/** A teacup on a saucer, standing on the anchor, 4 wide at scale 1. */
function cupPart(at: PartPlace, rng: Rng): KitShape[] {
  const f = local(at);
  const level = between(rng, 0.2, 0.3);
  return [
    shape(dkRing(f, 1.6, -1.8, 0.7, 0.35), "white", 1, INKED),
    shape(f.ellipse(0, -0.35, 2, 0.4), "slate-100", 1, INKED),
    shape(dkPath(f, [-1.2, -2.6], [["L", -0.95, -1.1], ["Q", -0.8, -0.55, 0, -0.55], ["Q", 0.8, -0.55, 0.95, -1.1], ["L", 1.2, -2.6]]), "white", 1, MAIN_INKED),
    shape(f.ellipse(0, -2.6, 1.2, 0.35), "slate-400", 0.7),
    shape(f.ellipse(0, -2.55 + level * 0.1, 1, 0.25), "wood-700", 1),
  ];
}

/** A dagger centred on the anchor, 8 long at scale 1, turned up to the side it faces. */
function daggerPart(at: PartPlace, rng: Rng): KitShape[] {
  const f = local(at);
  const a = between(rng, -70, -10);
  const [px, py] = turned([-3.5, 0], a);
  return [
    shape(f.poly(dkRot(dkBox(-3, -0.4, -1, 0.4), a)) + f.circle(px, py, 0.5), "wood-700", 1, INKED),
    shape(f.poly(dkRot(dkBox(-1, -1.5, -0.4, 1.5), a)), "brass-600", 1, INKED),
    shape(f.poly(dkRot([[-0.4, -0.55], [2.6, -0.55], [4, 0], [2.6, 0.55], [-0.4, 0.55]], a)), "slate-100", 1, MAIN_INKED),
    shape(f.poly(dkRot([[-0.4, 0], [4, 0], [2.6, 0.55], [-0.4, 0.55]], a)), "slate-400", 0.5),
    shape(f.band(dkRot([[-0.2, -0.3], [3.2, -0.25]], a), 0.18), "white", 0.7),
  ];
}

/** A long floating sword blade centred on the anchor, 16 long at scale 1, turned. */
function bladePart(at: PartPlace, rng: Rng): KitShape[] {
  const f = local(at);
  const a = between(rng, -55, 25);
  const [px, py] = turned([-7.6, 0], a);
  const [gx, gy] = turned([4.6, -0.5], a);
  return [
    shape(f.poly(dkRot([[-3.9, -1.1], [6, -1], [8, 0], [6, 1], [-3.9, 1.1]], a)), "slate-100", 1, MAIN_INKED),
    shape(f.poly(dkRot(dkBox(-7.1, -0.4, -4.6, 0.4), a)) + f.poly(dkRot(dkBox(-4.6, -2.4, -3.9, 2.4), a)) + f.circle(px, py, 0.5), "slate-700", 1, INKED),
    shape(f.band(dkRot([[-3.2, 0], [5.4, 0]], a), 0.5), "slate-400", 0.7),
    shape(f.band(dkRot([[-3.6, -0.85], [5.8, -0.8]], a), 0.2), "white", 0.55),
    shape(sparkle(f, gx, gy, 1.6), "white", 0.9),
  ];
}

/** An arrow in flight, tip at the anchor pointing the side it faces, 16 long at scale 1. */
function arrowPart(at: PartPlace, rng: Rng): KitShape[] {
  const f = local(at);
  const a = between(rng, -12, 12);
  const r = (pts: readonly Point[]): Point[] => dkRot(pts, a);
  return [
    shape(f.band(r([[-2.4, 0], [-15.5, 0]]), 0.5), "wood-600", 1, INKED),
    shape(f.poly(r([[-12.4, 0], [-14.2, -1.7], [-16, -1.7], [-15.2, 0]])) + f.poly(r([[-12.4, 0], [-15.2, 0], [-16, 1.7], [-14.2, 1.7]])), "red-600", 1),
    shape(f.poly(r([[0, 0], [-2.8, -1.2], [-2.2, 0], [-2.8, 1.2]])), "slate-400", 1, MAIN_INKED),
    shape(f.band(r([[-0.4, -0.2], [-2.4, -0.9]]), 0.2), "white", 0.5),
  ];
}

/** A straw archery target on a stand, 12 across and about 17 tall at scale 1, standing on the anchor. */
function targetPart(at: PartPlace, rng: Rng): KitShape[] {
  const f = local(at);
  const cy = -10.8 + between(rng, -0.3, 0.3);
  return [
    shape(f.band([[-2, -6], [-5, 0]], 0.9) + f.band([[2, -6], [5, 0]], 0.9), "wood-700", 1, INKED),
    shape(f.circle(0, cy, 6), "brass-500", 1, INKED),
    shape(dkRing(f, 0, cy, 6, 5.6), "brass-600", 0.7),
    shape(f.circle(0, cy, 5.2), "red-600", 1, MAIN),
    shape(f.circle(0, cy, 3.6), "white", 1),
    shape(f.circle(0, cy, 2), "red-600", 1, MAIN),
  ];
}

/** A treasure chest standing on the anchor, 10 wide and 8 tall at scale 1. */
function chestPart(at: PartPlace, rng: Rng): KitShape[] {
  const f = local(at);
  const plank = between(rng, -2.6, -2);
  return [
    shape(f.poly(dkBox(-5, -4.6, 5, 0)), "wood-600", 1, MAIN_INKED),
    shape(dkPath(f, [-5, -4.6], [["L", -5, -5.8], ["Q", -4.6, -8, 0, -8], ["Q", 4.6, -8, 5, -5.8], ["L", 5, -4.6]]), "wood-600", 1, MAIN_INKED),
    shape(f.band([[-4.8, plank], [4.8, plank]], 0.25), "slate-900", 0.25),
    shape(f.band([[-4.9, -7.2], [-1.8, -7.8]], 1.2) + f.band([[1.8, -7.8], [4.9, -7.2]], 1.2), "white", 0.18),
    shape([-3.2, 3.2].map((x) => f.poly(dkBox(x - 0.45, -4.3, x + 0.45, 0)) + f.poly(dkBox(x - 0.45, -7.2, x + 0.45, -4.9))).join("") + f.poly(dkBox(-5, -4.9, 5, -4.3)), "slate-400", 1, INKED),
    shape(f.poly(dkBox(-0.9, -5.7, 0.9, -3.3)), "slate-100", 1, INKED),
    shape(f.circle(0, -4.9, 0.3) + f.poly(dkBox(-0.15, -4.7, 0.15, -3.9)), "slate-900", 0.9),
  ];
}

/** A die centred on the anchor, 3 wide at scale 1, in three-quarter view and tumbled a little. */
function dicePart(at: PartPlace, rng: Rng): KitShape[] {
  const f = local(at);
  const a = between(rng, -25, 25);
  const r = (pts: readonly Point[]): Point[] => dkRot(pts, a);
  const pip = (x: number, y: number): string => {
    const [px, py] = turned([x, y], a);
    return f.circle(px, py, 0.2);
  };
  const left = (u: number, v: number): [number, number] => [-1.5 + 1.5 * u, -0.85 + 0.85 * u + 1.7 * v];
  const right = (u: number, v: number): [number, number] => [1.5 * u, -0.85 * u + 1.7 * v];
  const pips = pip(0, -0.85) + pip(...left(0.3, 0.3)) + pip(...left(0.7, 0.7)) + pip(...right(0.25, 0.25)) + pip(...right(0.5, 0.5)) + pip(...right(0.75, 0.75));
  return [
    shape(f.poly(r([[0, -1.7], [1.5, -0.85], [0, 0], [-1.5, -0.85]])) + f.poly(r([[-1.5, -0.85], [0, 0], [0, 1.7], [-1.5, 0.85]])), "white", 1, MAIN_INKED),
    shape(f.poly(r([[0, 0], [1.5, -0.85], [1.5, 0.85], [0, 1.7]])), "slate-400", 1, INKED),
    shape(pips, "black", 0.9),
  ];
}

/** A chain from the anchor along the side it faces, 20 long at scale 1, sagging between its ends. */
function chainPart(at: PartPlace, rng: Rng): KitShape[] {
  const f = local(at);
  const sag = between(rng, 3, 5);
  const links: string[] = [];
  const glints: string[] = [];
  for (let i = 0; i < 10; i += 1) {
    const x = 1 + 1.9 * i;
    const t = x / 20;
    const y = sag * 4 * t * (1 - t);
    links.push(dkRing(f, x, y, 0.9, 0.45));
    glints.push(f.circle(x - 0.55, y - 0.55, 0.18));
  }
  return [shape(links.join(""), "slate-400", 1, MAIN_INKED), shape(glints.join(""), "white", 0.7)];
}

/** A glowing rune circle flat on the ground, centred on the anchor, an ellipse 14 wide and 4 tall at scale 1. */
function runePart(at: PartPlace, rng: Rng): KitShape[] {
  const f = local(at);
  const phase = between(rng, 0, 45);
  const ticks: string[] = [];
  for (let k = 0; k < 8; k += 1) {
    const a = (phase + k * 45) * DEGREES;
    const c = Math.cos(a);
    const s = Math.sin(a);
    ticks.push(f.band([[4.8 * c, 1.35 * s], [6 * c, 1.7 * s]], 0.4));
  }
  return [
    shape(f.ellipse(0, 0, 7.6, 2.4), "cyan-400", 0.12),
    shape(f.ellipse(0, 0, 7, 2) + f.ellipse(0, 0, 6.4, 1.8), "cyan-400", 0.7, MAIN),
    shape(f.ellipse(0, 0, 4.4, 1.25) + f.ellipse(0, 0, 4, 1.1), "cyan-400", 0.6),
    shape(ticks.join(""), "cyan-400", 0.7),
    shape(f.poly([[0, -0.6], [1.8, 0], [0, 0.6], [-1.8, 0]]) + f.poly([[0, -0.3], [-0.9, 0], [0, 0.3], [0.9, 0]]), "cyan-400", 0.8),
  ];
}

/** A swirling portal standing on its base at the anchor, 14 wide and 22 tall at scale 1. */
function portalPart(at: PartPlace, rng: Rng): KitShape[] {
  const f = local(at);
  const start = between(rng, 0, 120);
  const arms = [0, 120, 240].map((k) => dkSwirl(f, 0, -11, 0.58, start + k, 200, 1.5, 9, 2.4)).join("");
  const sparks = [[-7.8, -17], [7.6, -5], [-7.9, -6]].map(([x, y]) => sparkle(f, (x ?? 0) + between(rng, -0.4, 0.4), y ?? 0, 1)).join("");
  return [
    shape(f.ellipse(0, -0.2, 6.5, 1), "slate-900", 0.3),
    shape(f.ellipse(0, -11, 8.2, 12), "sky-300", 0.18),
    shape(f.ellipse(0, -11, 7, 11) + f.ellipse(0, -11, 5.6, 9.6), "blue-600", 1, MAIN_INKED),
    shape(f.ellipse(0, -11, 5.6, 9.6), "sky-300", 0.6),
    shape(f.ellipse(0, -11, 3.6, 6.8), "white", 0.2),
    shape(arms, "white", 0.85),
    shape(sparks, "white", 0.9),
  ];
}

/** A spotlight hanging from the anchor: lamp 7 wide, its beam 40 long widening to 24 wide, at scale 1. */
function spotlightPart(at: PartPlace, rng: Rng): KitShape[] {
  const f = local(at);
  const sway = between(rng, -1, 1);
  return [
    shape(f.poly([[-3, 4.5], [3, 4.5], [12 + sway, 44.5], [-12 + sway, 44.5]]), "white", 0.15, MAIN),
    shape(f.poly([[-1.5, 4.5], [1.5, 4.5], [6 + sway, 44.5], [-6 + sway, 44.5]]), "white", 0.15),
    shape(f.band([[0, -3], [0, 0.5]], 0.8), "slate-900", 1),
    shape(f.poly([[-2.2, 0], [2.2, 0], [3.4, 4.5], [-3.4, 4.5]]), "slate-700", 1, INKED),
    shape(f.ellipse(0, 4.5, 2.6, 0.7), "white", 0.85),
  ];
}

/** A glass pipe from the anchor along the side it faces, 30 long at scale 1, with one bend down. */
function pipePart(at: PartPlace, rng: Rng): KitShape[] {
  const f = local(at);
  const drop = between(rng, 5, 8);
  const from = between(rng, 8, 12);
  const ease = (x: number): number => {
    const t = Math.min(1, Math.max(0, (x - from) / 8));
    return drop * t * t * (3 - 2 * t);
  };
  const line = (off: number): Point[] => [0, 3, 6, 9, 12, 15, 18, 21, 24, 27, 29].map((x) => [x, ease(x) + off] as const);
  return [
    shape(f.band(line(0), 2.6), "sky-300", 0.5, MAIN),
    shape(f.band(line(-0.65), 0.5), "white", 0.6),
    shape(f.poly(dkBox(-0.5, -1.9, 1, 1.9)) + f.poly(dkBox(from - 6, -1.9, from - 4.6, 1.9)) + f.poly(dkBox(28.2, drop - 1.9, 29.7, drop + 1.9)), "slate-400", 1, INKED),
  ];
}

/** A gear centred on the anchor, radius 5 at scale 1, turned. */
function gearPart(at: PartPlace, rng: Rng): KitShape[] {
  const f = local(at);
  const rot = between(rng, 0, 36);
  const pts: Point[] = [];
  for (let i = 0; i < 10; i += 1) {
    for (const [da, r] of [[-9, 4], [-6, 5], [6, 5], [9, 4]] as const) {
      const a = (rot + i * 36 + da) * DEGREES;
      pts.push([r * Math.cos(a), r * Math.sin(a)]);
    }
  }
  const spokes = [0, 1, 2, 3, 4].map((k) => {
    const a = (rot + k * 72) * DEGREES;
    return f.band([[0, 0], [3.4 * Math.cos(a), 3.4 * Math.sin(a)]], 0.9);
  });
  return [
    shape(f.poly(pts) + f.circle(0, 0, 3.1), "slate-400", 1, MAIN_INKED),
    shape(spokes.join(""), "slate-400", 1, INKED),
    shape(f.circle(0, 0, 1.4), "slate-700", 1, INKED),
    shape(f.circle(0, 0, 0.5), "slate-900", 0.9),
  ];
}

/** A heater shield centred on the anchor, 8 wide and 10 tall at scale 1. */
function shieldPart(at: PartPlace, _rng: Rng): KitShape[] {
  const f = local(at);
  return [
    shape(dkPath(f, [-4, -5], [["L", 4, -5], ["L", 4, 0], ["Q", 4, 3.5, 0, 5], ["Q", -4, 3.5, -4, 0]]), "slate-400", 1, INKED),
    shape(dkPath(f, [-3.4, -4.4], [["L", 3.4, -4.4], ["L", 3.4, 0], ["Q", 3.4, 3, 0, 4.3], ["Q", -3.4, 3, -3.4, 0]]), "blue-700", 1, MAIN),
    shape(dkPath(f, [-3.4, -4.4], [["L", 0, -4.4], ["L", 0, 4.3], ["Q", -3.4, 3, -3.4, 0]]), "white", 0.18),
    shape(f.circle(0, -0.5, 1.7), "slate-400", 1, INKED),
    shape(f.circle(0, -0.5, 1.2), "slate-100", 1),
  ];
}

/** A straw training dummy on a post, 16 tall at scale 1, standing on the anchor. */
function dummyPart(at: PartPlace, rng: Rng): KitShape[] {
  const f = local(at);
  const bar = between(rng, -0.3, 0.3);
  const tuft = (s: 1 | -1): string => f.poly([[6 * s, -10.5 + bar], [7.4 * s, -11.4 + bar], [7 * s, -10 + bar], [7.6 * s, -8.9 + bar], [6 * s, -9.5 + bar]]);
  return [
    shape(f.band([[0, -0.5], [0, -15]], 1.2) + f.poly(dkBox(-2.5, -0.5, 2.5, 0)), "wood-700", 1, INKED),
    shape(f.band([[-6, -10 + bar], [6, -10 - bar]], 0.9), "wood-700", 1, INKED),
    shape(tuft(1) + tuft(-1), "brass-500", 1, MAIN),
    shape(dkPath(f, [-2.6, -11.8], [["Q", 0, -12.6, 2.6, -11.8], ["L", 3, -6], ["Q", 0, -5, -3, -6]]), "brass-500", 1, MAIN_INKED),
    shape(f.band([[-2.9, -7.6], [2.9, -7.6]], 0.5) + f.band([[-1.4, -12.3], [1.4, -12.3]], 0.4), "brass-800", 1),
    shape(f.circle(0, -14.2, 2), "stone-500", 1, INKED),
    shape(f.circle(-0.8, -14.4, 0.3) + f.circle(0.8, -14.4, 0.3), "black", 0.7),
  ];
}

/** A toaster-shaped trap snapping open, standing on the anchor, 18 wide and 12 tall at scale 1. */
function toasterPart(at: PartPlace, rng: Rng): KitShape[] {
  const f = local(at);
  const open = between(rng, -4, 4);
  const jaw = (hx: number, deg: number): string =>
    f.poly(dkRot(dkBox(0, -1.1, 5.5, 1.1), deg).map(([x, y]) => [x + hx, y - 7.8] as const));
  const slots = [[-6, -1.6], [1.6, 6]] as const;
  return [
    shape(f.ellipse(0, -9, 6.5, 3), "orange-400", 0.2),
    shape(f.ellipse(0, -8.6, 4, 1.8), "orange-400", 0.3),
    shape(dkPath(f, [-9, -0.8], [["L", -9, -5.5], ["Q", -9, -6.5, -8, -6.5], ["L", 8, -6.5], ["Q", 9, -6.5, 9, -5.5], ["L", 9, -0.8]]), "slate-400", 1, MAIN_INKED),
    shape(f.poly(dkBox(-9, -5.6, -7.6, -0.8)), "slate-100", 0.8),
    shape(f.band([[-4, -5.5], [-5.5, -1.5]], 0.8), "white", 0.35),
    shape(f.poly([[-7.8, -8.3], [7.8, -8.3], [9, -6.5], [-9, -6.5]]), "slate-100", 0.6),
    shape(slots.map(([a, b]) => f.poly(dkBox(a, -7.7, b, -7))).join("") + f.band([[7.6, -5], [7.6, -1.5]], 0.6) + f.poly(dkBox(-8, -0.8, -6, 0)) + f.poly(dkBox(6, -0.8, 8, 0)), "slate-900", 1),
    shape(slots.map(([a, b]) => f.poly(dkBox(a + 0.2, -7.55, b - 0.2, -7.15))).join(""), "orange-400", 0.9),
    shape(f.band([[0, -7.8], [-0.9, -8.5], [0.9, -9.1], [-0.9, -9.7], [0, -10.4]], 0.45), "slate-700", 1),
    shape(jaw(-1.3, 215 + open) + jaw(1.3, 325 - open), "slate-700", 1, INKED),
    shape(f.circle(7.6, -3.2, 1), "slate-700", 1, INKED),
  ];
}

/** A glowing jelly bean centred on the anchor, 4 wide at scale 1, tilted. */
function jellyBeanPart(at: PartPlace, rng: Rng): KitShape[] {
  const f = local(at);
  const a = between(rng, -35, 35);
  return [
    shape(f.circle(0, 0, 3.4), "lime-200", 0.15),
    shape(f.circle(0, 0, 2.7), "lime-200", 0.25),
    shape(dkEll(f, at.flip, 0, 0, 2, 1.25, a), "lime-400", 1, MAIN_INKED),
    shape(dkEll(f, at.flip, 0, 0.45, 1.8, 0.6, a), "green-700", 0.45),
    shape(dkEll(f, at.flip, -0.7, -0.5, 0.6, 0.25, a - 20), "white", 0.85),
  ];
}

/** A battered over-taped parcel standing on the anchor, 12 wide and 9 tall at scale 1, top burst open. */
function parcelPart(at: PartPlace, rng: Rng): KitShape[] {
  const f = local(at);
  const j = (): number => between(rng, -0.4, 0.4);
  return [
    shape(
      f.poly([[-6, 0], [-6 + j(), -6], [6 + j(), -6], [6, 0]]) +
        f.poly([[-6, -6], [-0.8, -6], [-3.5, -8.6 + j()], [-8.4, -8 + j()]]) +
        f.poly([[0.8, -6], [6, -6], [8.6, -7.6 + j()], [3.6, -9 + j()]]),
      "brass-600",
      1,
      MAIN_INKED,
    ),
    shape(f.ellipse(0, -6, 4.7, 0.8), "slate-900", 0.7),
    shape(f.poly(dkBox(-2.6, -5.4, -1.4, 0)) + f.poly(dkBox(-6, -3.1, -2.6, -2.1)) + f.poly(dkBox(-1.4, -3.1, 6, -2.1)), "slate-100", 0.85),
    shape(f.poly([[3, -5], [4.2, -4.4], [3.2, -3.6]]) + f.poly([[-5.2, -1.6], [-4.4, -0.8], [-5.4, -0.6]]), "slate-900", 0.28),
    shape(f.poly([[6, 0], [6, -6], [4.6, -5], [4.6, 0]]), "slate-900", 0.15),
  ];
}

/** A bicycle standing on the anchor, 16 wide and 10 tall at scale 1, facing the side it faces. */
function bicyclePart(at: PartPlace, rng: Rng): KitShape[] {
  const f = local(at);
  const hubs = (x: number): string => f.circle(x, -4.2, 0.5);
  const lean = between(rng, -0.4, 0.4);
  return [
    shape(dkRing(f, -5.5, -4.2, 4.2, 3.6) + dkRing(f, 5.5, -4.2, 4.2, 3.6), "slate-900", 1, INKED),
    shape(hubs(-5.5) + hubs(5.5), "slate-400", 0.9),
    shape(f.band([[-5.5, -4.2], [-2.5, -8.2], [3.2 + lean, -8], [5.5, -4.2]], 0.7), "red-600", 1, MAIN_INKED),
    shape(f.band([[-5.5, -4.2], [0, -3.2], [3.2 + lean, -8]], 0.7), "red-600", 1, MAIN),
    shape(f.band([[-2.5, -8.2], [0, -3.2]], 0.7), "red-600", 1, MAIN),
    shape(f.circle(0, -3.2, 1.1), "slate-400", 1),
    shape(f.band([[-3.4, -9], [-1.2, -9]], 0.9) + f.band([[3.2 + lean, -8], [3, -9.5], [4.8, -9.3]], 0.5), "slate-900", 1),
  ];
}

/** A striped deckchair standing on the anchor, 12 wide and 10 tall at scale 1. */
function deckchairPart(at: PartPlace, rng: Rng): KitShape[] {
  const f = local(at);
  const sag = between(rng, -0.3, 0.3);
  const path: Point[] = [[-4.3, -10], [-1.5, -3.5], [1.5, -2.8 + sag], [5, -5.5]];
  const pieces: Point[][] = [];
  const split = (a: Point, b: Point, n: number): void => {
    for (let i = 0; i < n; i += 1) pieces.push([[a[0] + ((b[0] - a[0]) * i) / n, a[1] + ((b[1] - a[1]) * i) / n], [a[0] + ((b[0] - a[0]) * (i + 1)) / n, a[1] + ((b[1] - a[1]) * (i + 1)) / n]]);
  };
  split(path[0] ?? [0, 0], path[1] ?? [0, 0], 4);
  split(path[1] ?? [0, 0], path[2] ?? [0, 0], 2);
  split(path[2] ?? [0, 0], path[3] ?? [0, 0], 2);
  const stripe = (parity: number): string => pieces.filter((_, i) => i % 2 === parity).map((p) => f.band(p, 2.4)).join("");
  return [
    shape(f.band([[-6, -0.2], [-4.5, -10]], 0.8) + f.band([[6, -0.2], [5, -5.8]], 0.8), "wood-600", 1, INKED),
    shape(f.band([[-6, -0.3], [6, -0.3]], 0.7), "wood-600", 1, INKED),
    shape(stripe(0), "red-600", 1, MAIN),
    shape(stripe(1), "white", 1),
  ];
}

/** A hat tumbling, centred on the anchor, 6 wide at scale 1, turned any way. */
function hatPart(at: PartPlace, rng: Rng): KitShape[] {
  const f = local(at);
  const a = between(rng, 0, 360);
  const [bx, by] = turned([0, 1.6], a);
  const [tx, ty] = turned([0, -1.8], a);
  return [
    shape(dkEll(f, at.flip, bx, by, 3, 0.7, a), "slate-700", 1, MAIN_INKED),
    shape(f.poly(dkRot(dkBox(-1.7, -1.8, 1.7, 1.4), a)), "slate-700", 1, MAIN_INKED),
    shape(f.poly(dkRot(dkBox(-1.7, 0.2, 1.7, 1.2), a)), "slate-900", 1),
    shape(dkEll(f, at.flip, tx, ty, 1.7, 0.45, a), "slate-900", 0.3),
    shape(f.band(dkRot([[-1.1, -1.5], [-1.1, -0.1]], a), 0.35), "white", 0.2),
  ];
}

/** An assembly-line belt, its top at the anchor's height, spanning 30 either side of the anchor at scale 1. */
function conveyorPart(at: PartPlace, rng: Rng): KitShape[] {
  const f = local(at);
  const phase = between(rng, -1, 1);
  const rollers: string[] = [];
  for (let x = -28; x <= 28; x += 4) rollers.push(f.circle(x + phase, 1.5, 0.8));
  const legs = [-26, -13, 0, 13, 26].map((x) => f.poly(dkBox(x - 0.8, 3, x + 0.8, 11))).join("");
  return [
    shape(legs, "slate-700", 1, INKED),
    shape(f.poly(dkBox(-30, 0, 30, 3)), "slate-800", 1, MAIN_INKED),
    shape(rollers.join(""), "slate-400", 0.9),
    shape(f.band([[-30, 0.25], [30, 0.25]], 0.5), "white", 0.25),
  ];
}

/** A spinning circular saw blade centred on the anchor, radius 5 at scale 1. */
function sawPart(at: PartPlace, rng: Rng): KitShape[] {
  const f = local(at);
  const rot = between(rng, 0, 30);
  const pts: Point[] = [];
  for (let i = 0; i < 12; i += 1) {
    for (const [da, r] of [[0, 4.2], [24, 5]] as const) {
      const a = (rot + i * 30 + da) * DEGREES;
      pts.push([r * Math.cos(a), r * Math.sin(a)]);
    }
  }
  const arc = (start: number): Point[] => [0, 1, 2, 3, 4].map((k) => {
    const a = (start + k * 17) * DEGREES;
    return [5.9 * Math.cos(a), 5.9 * Math.sin(a)] as const;
  });
  return [
    shape(f.poly(pts), "slate-100", 1, MAIN_INKED),
    shape(dkRing(f, 0, 0, 3.3, 3.1), "slate-400", 0.6),
    shape(f.circle(0, 0, 1.5), "slate-700", 1, INKED),
    shape(f.circle(0, 0, 0.5), "slate-900", 0.9),
    shape(f.band(arc(rot + 20), 0.5) + f.band(arc(rot + 200), 0.5), "white", 0.4),
  ];
}

/** A sleek rifle lying level, centred on the anchor, 18 long at scale 1, muzzle to the side it faces. */
function riflePart(at: PartPlace, _rng: Rng): KitShape[] {
  const f = local(at);
  return [
    shape(f.poly([[-9, -0.2], [-5.5, -1.2], [1.6, -1.2], [1.6, 0.8], [-5.5, 0.8], [-8.6, 2.2]]), "slate-700", 1, MAIN_INKED),
    shape(f.poly(dkBox(1.6, -0.9, 9, -0.1)), "slate-800", 1, INKED),
    shape(
      f.poly(dkBox(-3, -2.75, 2.5, -1.65)) + f.poly(dkBox(-2.4, -1.65, -1.9, -1.2)) + f.poly(dkBox(1.2, -1.65, 1.7, -1.2)) +
        f.poly([[-1, 0.8], [0.8, 0.8], [1, 3.5], [-1.2, 3.5]]) + f.poly(dkBox(8.6, -1.2, 9.5, 0)) +
        f.poly(dkBox(2.5, 0, 6.3, 1)) + f.poly([[2.8, 1], [4.2, 1], [3.9, 3.3], [2.7, 3.3]]),
      "slate-900",
      1,
    ),
    shape(f.circle(2.5, -2.2, 0.5) + f.circle(-3, -2.2, 0.4), "sky-300", 0.7),
    shape(f.band([[-5, -1], [1.5, -1]], 0.4) + f.band([[-8.4, 0], [-5.6, -0.8]], 0.4), "white", 0.2),
  ];
}

/** A dented knight's helmet lying on the anchor, 5 wide at scale 1, visor to the side it faces. */
function helmetPart(at: PartPlace, rng: Rng): KitShape[] {
  const f = local(at);
  const dx = between(rng, -0.3, 0.3);
  return [
    shape(dkPath(f, [-2.5, 0], [["L", -2.5, -1.8], ["Q", -2.5, -4.2, 0, -4.2], ["Q", 2.5, -4.2, 2.5, -1.8], ["L", 2.5, 0]]), "slate-400", 1, MAIN_INKED),
    shape(dkPath(f, [2.5, 0], [["L", 2.5, -1.8], ["Q", 2.5, -4.2, 0.8, -4.1], ["Q", 1.4, -2.5, 1.2, 0]]), "slate-900", 0.2),
    shape(f.band([[0.4, -2.2], [2.5, -2.2]], 0.45), "slate-900", 0.95),
    shape(f.ellipse(-0.9 + dx, -3.1, 0.8, 0.5), "slate-900", 0.3),
    shape(f.band([[-2.2, -1.2], [-2, -3], [-0.8, -3.9], [0.5, -4]], 0.5), "white", 0.35),
  ];
}

/** A blackboard standing on the anchor, 24 wide and 16 tall at scale 1, chalked with curves and squares. */
function blackboardPart(at: PartPlace, rng: Rng): KitShape[] {
  const f = local(at);
  const wave: Point[] = [];
  for (let i = 0; i <= 6; i += 1) wave.push([-10 + 1.5 * i, -5 + between(rng, 0.4, 0.8) * Math.sin(i * 1.5)]);
  const arch: Point[] = [];
  for (let i = 0; i <= 6; i += 1) arch.push([-9 + (i * 8) / 6, -6 - 6 * Math.sin((Math.PI * i) / 6)]);
  const square = (x: number, y: number, w: number): string => f.poly(dkBox(x, y, x + w, y + w)) + f.poly(dkBox(x + 0.3, y + 0.3, x + w - 0.3, y + w - 0.3));
  return [
    shape(f.poly(dkBox(-12, -16, 12, -2.8)), "green-900", 1, MAIN),
    shape(f.poly(dkBox(-12, -16, 12, -2.8)) + f.poly(dkBox(-11, -15, 11, -3.8)), "wood-600", 1, INKED),
    shape(f.poly(dkBox(-12.4, -2.8, 12.4, -1.8)) + f.poly(dkBox(-10.5, -1.8, -9.5, 0)) + f.poly(dkBox(9.5, -1.8, 10.5, 0)), "wood-700", 1, INKED),
    shape(f.band(arch, 0.35) + f.band(wave, 0.35), "white", 0.6),
    shape(square(3.5, -13, 3.2) + square(7, -9, 2.6), "white", 0.6),
    shape(f.poly(dkBox(3, -3.4, 4.2, -2.9)), "white", 0.75),
  ];
}

/** A twin-chamber cloning pod standing on the anchor, 30 wide and 26 tall at scale 1. */
function podPart(at: PartPlace, rng: Rng): KitShape[] {
  const f = local(at);
  const cs = [-8.8, 8.8];
  const each = (g: (c: number) => string): string => cs.map(g).join("");
  const body = (c: number): string =>
    f.circle(c, -18.6, 1.7) + f.poly([[c - 2.4, -16.2], [c + 2.4, -16.2], [c + 2, -11], [c + 1.6, -6.8], [c + 0.2, -6.8], [c, -10.8], [c - 0.2, -6.8], [c - 1.6, -6.8], [c - 2, -11]]);
  const bubbles = each((c) => f.circle(c + 3, -8 - between(rng, 0, 1), 0.35) + f.circle(c + 3.9, -12 - between(rng, 0, 1), 0.3) + f.circle(c - 3.2, -19, 0.3));
  return [
    shape(f.poly(dkBox(-15, -3.4, 15, 0)), "slate-700", 1, INKED),
    shape(f.band([[-2.6, -24.8], [2.6, -24.8]], 1.4) + f.band([[-2.6, -4.4], [2.6, -4.4]], 1.2), "slate-800", 1),
    shape(each((c) => f.poly(dkBox(c - 5.6, -23.5, c + 5.6, -5.5))), "sky-300", 0.3, INKED),
    shape(each((c) => f.ellipse(c, -12, 4.6, 9)), "cyan-400", 0.15),
    shape(each(body), "cyan-400", 0.6),
    shape(bubbles, "white", 0.5),
    shape(each((c) => f.poly(dkBox(c - 6.2, -26, c + 6.2, -23.5)) + f.poly(dkBox(c - 6.2, -5.5, c + 6.2, -3.4))), "slate-400", 1, MAIN_INKED),
    shape(each((c) => f.poly(dkBox(c - 4.8, -22.5, c - 4.1, -7))), "white", 0.3),
  ];
}

/** A crane wrecking ball hanging from the anchor: cable 20 down, ball radius 5 at scale 1 below it. */
function wreckingBallPart(at: PartPlace, _rng: Rng): KitShape[] {
  const f = local(at);
  const cy = 27.2;
  const rim: Point[] = [0, 1, 2, 3, 4].map((k) => {
    const a = (15 + k * 22) * DEGREES;
    return [4.1 * Math.cos(a), cy + 4.1 * Math.sin(a)] as const;
  });
  return [
    shape(f.band([[0, 0], [0, 20.4]], 0.5), "slate-900", 1),
    shape(dkRing(f, 0, 21.2, 1, 0.6), "slate-400", 1, INKED),
    shape(f.circle(0, cy, 5), "slate-700", 1, MAIN_INKED),
    shape(f.band(rim, 1.6), "slate-900", 0.3),
    shape(f.circle(-1.6, cy - 1.6, 2.3), "white", 0.18),
    shape(f.circle(-1.9, cy - 1.9, 0.9), "white", 0.45),
  ];
}

/** A padlock centred on the anchor, 4 wide and 6 tall at scale 1. */
function padlockPart(at: PartPlace, _rng: Rng): KitShape[] {
  const f = local(at);
  const arch: Point[] = [];
  for (let i = 0; i <= 6; i += 1) {
    const a = Math.PI * (i / 6);
    arch.push([-1.1 * Math.cos(a), -0.4 - 1.5 * Math.sin(a) - 1.2 * Math.sin(a) * 0]);
  }
  return [
    shape(f.band([[-1.1, 0], ...arch.slice(1, -1), [1.1, 0]], 0.7), "slate-400", 1, INKED),
    shape(f.poly([[-2, -0.2], [-1.6, -0.6], [1.6, -0.6], [2, -0.2], [2, 2.4], [1.6, 2.8], [-1.6, 2.8], [-2, 2.4]]), "slate-700", 1, MAIN_INKED),
    shape(f.poly(dkBox(-2, -0.2, -1.4, 2.4)), "white", 0.2),
    shape(f.circle(0, 0.9, 0.5) + f.poly([[-0.25, 1.4], [0.25, 1.4], [0.45, 2.2], [-0.45, 2.2]]), "slate-900", 0.95),
  ];
}

/** A lit candle standing on the anchor, 2 wide and 5 tall at scale 1. */
function candlePart(at: PartPlace, rng: Rng): KitShape[] {
  const f = local(at);
  const lean = between(rng, -0.1, 0.1);
  return [
    shape(f.circle(0, -4.2, 1.6), "orange-400", 0.2),
    shape(f.poly(dkBox(-1, -3.2, 1, 0)), "slate-100", 1, MAIN_INKED),
    shape(f.poly(dkBox(0.3, -3.2, 1, 0)), "slate-900", 0.15),
    shape(dkPath(f, [0.55, -3.2], [["L", 1.4, -3.2], ["L", 1.4, -2.2], ["Q", 1.4, -1.5, 1, -1.5], ["Q", 0.55, -1.5, 0.55, -2.2]]), "slate-400", 0.4),
    shape(f.band([[0, -3.2], [lean, -3.8]], 0.2), "slate-900", 1),
    shape(dkPath(f, [lean, -5], [["Q", 0.85, -4.1, 0.6, -3.7], ["Q", 0, -3.3, -0.6, -3.7], ["Q", -0.85, -4.1, 0, -5]]), "orange-400", 1),
    shape(dkPath(f, [lean, -4.4], [["Q", 0.3, -4, 0.2, -3.75], ["Q", 0, -3.6, -0.2, -3.75], ["Q", -0.3, -4, 0, -4.4]]), "white", 0.9),
  ];
}

/** A glowing chute: a funnel mouth 16 wide at the anchor narrowing 14 down, at scale 1. */
function chutePart(at: PartPlace, _rng: Rng): KitShape[] {
  const f = local(at);
  const rivets = [4, 8, 12].map((y) => {
    const e = 8 - (5.8 * y) / 14 - 0.6;
    return f.circle(e, y, 0.3) + f.circle(-e, y, 0.3);
  });
  return [
    shape(f.poly([[-8, 0], [8, 0], [2.2, 14], [-2.2, 14]]), "slate-400", 1, MAIN_INKED),
    shape(f.poly([[-6.8, 1], [6.8, 1], [1.5, 13], [-1.5, 13]]), "cyan-400", 0.55),
    shape(f.poly([[-3.5, 1.5], [3.5, 1.5], [0.8, 12.5], [-0.8, 12.5]]), "white", 0.25),
    shape(f.ellipse(0, 14.2, 2.2, 0.8), "cyan-400", 0.3),
    shape(f.ellipse(0, 0, 8, 1.5) + f.ellipse(0, 0, 7.2, 1), "slate-100", 0.6),
    shape(rivets.join(""), "slate-100", 0.8),
  ];
}

/** A huge two-stick game controller centred on the anchor, 30 wide and 16 tall at scale 1, tilted. */
function controllerPart(at: PartPlace, rng: Rng): KitShape[] {
  const f = local(at);
  const tilt = between(rng, -7, 7);
  const T = (x: number, y: number): Point => turned([x, y], tilt);
  const poly = (pts: readonly Point[]): string => f.poly(pts.map(([x, y]) => T(x, y)));
  const dot = (x: number, y: number, r: number): string => {
    const [px, py] = T(x, y);
    return f.circle(px, py, r);
  };
  const pad = ((cx: number, cy: number, l: number, w: number): Point[] => [
    [cx - w, cy - l], [cx + w, cy - l], [cx + w, cy - w], [cx + l, cy - w], [cx + l, cy + w], [cx + w, cy + w],
    [cx + w, cy + l], [cx - w, cy + l], [cx - w, cy + w], [cx - l, cy + w], [cx - l, cy - w], [cx - w, cy - w],
  ])(-3.4, 1.2, 1.7, 0.55);
  const button = (x: number, y: number): string => dot(9.6 + x, -1.2 + y, 1.05);
  return [
    shape(poly([[-9, -6.5], [9, -6.5], [13, -5.2], [15, -1], [15, 4], [14.2, 8], [11.6, 8.3], [9.2, 4.4], [5, 3.6], [-5, 3.6], [-9.2, 4.4], [-11.6, 8.3], [-14.2, 8], [-15, 4], [-15, -1], [-13, -5.2]]), "slate-800", 1, MAIN_INKED),
    shape(poly([[-13.2, -5.3], [-8.8, -6.5], [-8.8, -8], [-12.4, -7.2]]) + poly([[13.2, -5.3], [8.8, -6.5], [8.8, -8], [12.4, -7.2]]), "slate-700", 1, INKED),
    shape(dot(-8, -0.8, 2.8) + dot(3.6, 1, 2.4) + dot(-1.2, -3.6, 0.55) + dot(1.2, -3.6, 0.55), "slate-700", 1),
    shape(dot(-8, -0.8, 1.8) + dot(3.6, 1, 1.8), "slate-900", 1),
    shape(dot(-8, -0.8, 1) + dot(3.6, 1, 1) + dot(0, -5.2, 0.8), "slate-400", 0.5),
    shape(poly(pad), "slate-400", 1, INKED),
    shape(button(2.1, 0), "red-500", 1),
    shape(button(0, 2.1), "lime-400", 1),
    shape(button(-2.1, 0), "sky-400", 1),
    shape(button(0, -2.1), "violet-500", 1),
    shape(f.band([[-9, -6.1], [9, -6.1]].map(([x, y]) => T(x ?? 0, y ?? 0)), 0.5), "white", 0.12),
  ];
}
// @@ D objects kit
  card: { category: "objects", draw: cardPart },
  "card-fan": { category: "objects", draw: cardFanPart },
  deck: { category: "objects", draw: deckPart },
  coin: { category: "objects", draw: coinPart },
  "coin-stack": { category: "objects", draw: coinStackPart },
  table: { category: "objects", draw: tablePart },
  chair: { category: "objects", draw: chairPart },
  tome: { category: "objects", draw: tomePart },
  jar: { category: "objects", draw: jarPart },
  cup: { category: "objects", draw: cupPart },
  dagger: { category: "objects", draw: daggerPart },
  blade: { category: "objects", draw: bladePart },
  arrow: { category: "objects", draw: arrowPart },
  target: { category: "objects", draw: targetPart },
  chest: { category: "objects", draw: chestPart },
  dice: { category: "objects", draw: dicePart },
  chain: { category: "objects", draw: chainPart },
  rune: { category: "objects", draw: runePart },
  portal: { category: "objects", draw: portalPart },
  spotlight: { category: "objects", draw: spotlightPart },
  pipe: { category: "objects", draw: pipePart },
  gear: { category: "objects", draw: gearPart },
  shield: { category: "objects", draw: shieldPart },
  dummy: { category: "objects", draw: dummyPart },
  toaster: { category: "objects", draw: toasterPart },
  "jelly-bean": { category: "objects", draw: jellyBeanPart },
  parcel: { category: "objects", draw: parcelPart },
  bicycle: { category: "objects", draw: bicyclePart },
  deckchair: { category: "objects", draw: deckchairPart },
  hat: { category: "objects", draw: hatPart },
  conveyor: { category: "objects", draw: conveyorPart },
  saw: { category: "objects", draw: sawPart },
  rifle: { category: "objects", draw: riflePart },
  helmet: { category: "objects", draw: helmetPart },
  blackboard: { category: "objects", draw: blackboardPart },
  pod: { category: "objects", draw: podPart },
  "wrecking-ball": { category: "objects", draw: wreckingBallPart },
  padlock: { category: "objects", draw: padlockPart },
  candle: { category: "objects", draw: candlePart },
  chute: { category: "objects", draw: chutePart },
  controller: { category: "objects", draw: controllerPart },
