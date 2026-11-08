// @@ B architecture

/** An axis-aligned box in a part's frame, `(x, y)` its top-left corner. */
function archRect(f: Local, x: number, y: number, w: number, h: number): string {
  return f.poly([[x, y], [x + w, y], [x + w, y + h], [x, y + h]]);
}

/** An arched opening in a part's frame: its feet at `bottom`, its crown at `top`, `w` wide. */
function archDoor(f: Local, cx: number, bottom: number, w: number, top: number): string {
  const hw = w / 2;
  return (
    `M${f.p(cx - hw, bottom)}L${f.p(cx - hw, top + hw)}Q${f.p(cx - hw, top)} ${f.p(cx, top)}` +
    `Q${f.p(cx + hw, top)} ${f.p(cx + hw, top + hw)}L${f.p(cx + hw, bottom)}Z`
  );
}

/** A window with its sill on the anchor, 12 wide and 16 tall at scale 1. */
function windowPart(at: PartPlace, rng: Rng): KitShape[] {
  const f = local(at);
  const j = between(rng, -0.4, 0.4);
  return [
    shape(archRect(f, -7, -1.6, 14, 1.6), "slate-100", 1, INKED),
    shape(archRect(f, -6, -16, 12, 14.4), "white", 1, INKED),
    shape(archRect(f, -4.8, -14.8, 9.6, 12), "sky-300", 0.6, MAIN),
    shape(archRect(f, -0.5, -14.8, 1, 12) + archRect(f, -4.8, -9.2 + j, 4.3, 1) + archRect(f, 0.5, -9.2 + j, 4.3, 1), "white", 0.95),
    shape(f.band([[-3.6, -4.4], [-0.6, -12.6]], 1.5) + f.band([[0.8, -4.4], [2.2, -8]], 0.7), "white", 0.35),
  ];
}

/** A door in its frame standing on the anchor, 9 wide and 16 tall at scale 1. */
function doorPart(at: PartPlace, rng: Rng): KitShape[] {
  const f = local(at);
  const lean = between(rng, 0, 0.5);
  return [
    shape(f.poly([[-4.5, 0], [-4.5, -16], [4.5, -16], [4.5, 0], [3.7, 0], [3.7, -15.2], [-3.7, -15.2], [-3.7, 0]]), "slate-100", 1, INKED),
    shape(archRect(f, -3.7, -15.2, 7.4, 15.2), "wood-600", 1, MAIN_INKED),
    shape(archRect(f, -2.6, -13.6, 5.2, 5.4 + lean) + archRect(f, -2.6, -6.8, 5.2, 5.4), "wood-700", 1),
    shape(f.circle(2.6, -7.5, 0.7), "brass-600", 1),
  ];
}

/** A wall shelf unit hung on the anchor's ground line, 24 wide and 20 tall at scale 1, three boards of stock. */
function shelfPart(at: PartPlace, rng: Rng): KitShape[] {
  const f = local(at);
  const floors = [-1.4, -8.2, -15];
  const kinds = [
    { w: 2.2, h: 3.5, d: "" },
    { w: 3, h: 2.5, d: "" },
    { w: 2.4, h: 2.2, d: "" },
    { w: 1.6, h: 4.5, d: "" },
  ];
  for (const floor of floors) {
    let x = -10.8;
    while (x < 10.4) {
      const k = kinds[whole(rng, 0, 3)] ?? kinds[0];
      if (k === undefined) break;
      if (x + k.w > 10.8) break;
      if (rng() < 0.85) {
        k.d += archRect(f, x, floor - k.h, k.w, k.h);
        x += k.w + between(rng, 0.2, 0.8);
      } else x += 1.4;
    }
  }
  return [
    shape(archRect(f, -12, -20, 1.1, 20) + archRect(f, 10.9, -20, 1.1, 20), "wood-800", 1, INKED),
    shape(archRect(f, -10.9, -1.4, 21.8, 1.4) + archRect(f, -10.9, -8.2, 21.8, 1.2) + archRect(f, -10.9, -15, 21.8, 1.2), "wood-700", 1, MAIN_INKED),
    shape(kinds[0]?.d || archRect(f, -9, -4.9, 2.2, 3.5), "sky-300", 0.85),
    shape(kinds[1]?.d || archRect(f, -5, -3.9, 3, 2.5), "brass-600", 1),
    shape(kinds[2]?.d || archRect(f, 0, -3.6, 2.4, 2.2), "red-600", 1),
    shape(kinds[3]?.d || archRect(f, 5, -5.9, 1.6, 4.5), "green-700", 1),
  ];
}

/** A shop counter standing on the anchor, 40 wide and 10 tall at scale 1. */
function counterPart(at: PartPlace, rng: Rng): KitShape[] {
  const f = local(at);
  let panels = "";
  for (let i = 0; i < 4; i += 1) panels += archRect(f, -18.6 + i * 9.4, -6.2, 8.6, 5);
  void rng;
  return [
    shape(archRect(f, -20, -10, 40, 2.6), "wood-600", 1, MAIN_INKED),
    shape(archRect(f, -20, -10, 40, 0.8), "brass-500", 0.4),
    shape(archRect(f, -19, -7.4, 38, 7.4), "wood-700", 1, INKED),
    shape(panels, "wood-800", 0.7),
  ];
}

/** A tidy whitewashed house standing on the anchor, 22 wide and 20 tall at scale 1. */
function housePart(at: PartPlace, rng: Rng): KitShape[] {
  const f = local(at);
  const cx = rng() < 0.5 ? 5.4 : -5.4;
  return [
    shape(archRect(f, cx - 1.6, -20, 3.2, 8), "red-900", 1),
    shape(archRect(f, -9, -11.5, 18, 11.5), "slate-100", 1, MAIN_INKED),
    shape(archRect(f, 3, -11.5, 6, 11.5), "slate-900", 0.15),
    shape(f.poly([[-11, -10.8], [0, -19.6], [11, -10.8]]), "red-600", 1, INKED),
    shape(f.poly([[-10.4, -11.2], [0, -19], [0, -14.4], [-5, -11.2]]), "white", 0.22),
    shape(archRect(f, -1.7, -6.8, 3.4, 6.8), "wood-700", 1),
    shape(archRect(f, -7.4, -9, 3.6, 4.2) + archRect(f, 3.8, -9, 3.6, 4.2), "sky-300", 0.9),
  ];
}

/** A glass dome arching over the anchor, 50 wide and 30 tall at scale 1. */
function domePart(at: PartPlace, rng: Rng): KitShape[] {
  const f = local(at);
  const rise = between(rng, 28.5, 30);
  const rib = (k: number, stop: number): string => {
    const pts: Point[] = [];
    for (let i = 0; i <= 6; i += 1) {
      const t = (stop * i) / 6;
      pts.push([25 * k * Math.cos(t), -rise * Math.sin(t)]);
    }
    return f.band(pts, 0.8);
  };
  const half = Math.PI / 2;
  const outline: Point[] = [];
  for (let i = 0; i <= 10; i += 1) outline.push([-25 * Math.cos((Math.PI * i) / 10), -rise * Math.sin((Math.PI * i) / 10)]);
  const near = half * 0.83;
  const ribs = f.band(outline, 1) + rib(0.66, near) + rib(-0.66, near) + rib(0.33, near) + rib(-0.33, near) + f.band([[0, 0], [0, -rise * 0.9]], 0.8);
  return [
    shape(`M${f.p(-25, 0)}A${fmt(f.r(25))} ${fmt(f.r(rise))} 0 0 ${f.sweep(1)} ${f.p(25, 0)}Z`, "sky-300", 0.18),
    shape(ribs, "slate-100", 0.95, MAIN),
    shape(f.ellipse(0, 0, 26.5, 2), "slate-400", 0.95, INKED),
  ];
}

/** A low dry-stone wall standing on the anchor, 50 wide and 8 tall at scale 1. */
function stoneWallPart(at: PartPlace, rng: Rng): KitShape[] {
  const f = local(at);
  const rows: readonly (readonly [number, number])[] = [[-8, -5.9], [-5.4, -3], [-2.5, 0]];
  let mortar = archRect(f, -25, -5.9, 50, 0.5) + archRect(f, -25, -3, 50, 0.5);
  let light = "";
  rows.forEach(([top, bottom]) => {
    let x = -25 + between(rng, 0, 3);
    while (x < 25) {
      const w = between(rng, 4.5, 8.5);
      const next = Math.min(25, x + w);
      if (next - x > 2) light += archRect(f, x + 0.5, top + 0.35, next - x - 1.2, 0.55);
      if (next < 25 - 1) mortar += archRect(f, next - 0.25, top, 0.5, bottom - top);
      x = next;
    }
  });
  const edge: Point[] = [[-25, -7.4], [-12, -8 + between(rng, -0.3, 0.3)], [0, -7.8], [12, -8 + between(rng, -0.3, 0.3)], [25, -7.4]];
  return [
    shape(f.poly([[-25, 0], ...edge, [25, 0]]), "stone-500", 1, MAIN_INKED),
    shape(light, "slate-400", 0.5),
    shape(mortar, "slate-700", 0.55),
    shape(archRect(f, -25, -1.4, 50, 1.4), "slate-900", 0.18),
  ];
}

/** A castle wall with merlons standing on the anchor, 50 wide and 16 tall at scale 1. */
function battlementPart(at: PartPlace, rng: Rng): KitShape[] {
  const f = local(at);
  const n = 6;
  const mw = 5.4;
  const gap = (50 - n * mw) / (n - 1);
  const pts: Point[] = [[-25, 0]];
  let tops = "";
  for (let i = 0; i < n; i += 1) {
    const x = -25 + i * (mw + gap);
    pts.push([x, -12], [x, -16], [x + mw, -16], [x + mw, -12]);
    tops += archRect(f, x, -16, mw, 0.7);
    if (i < n - 1) pts.push([x + mw + gap, -12]);
  }
  pts.push([25, 0]);
  const slit = between(rng, -3, 3);
  return [
    shape(f.poly(pts), "stone-500", 1, MAIN_INKED),
    shape(archRect(f, -25, -3.2, 50, 3.2) + archRect(f, -25 + mw, -12, gap, 1.4), "slate-900", 0.25),
    shape(archRect(f, -25, -9.4, 50, 0.4) + archRect(f, -25, -6.4, 50, 0.4), "slate-700", 0.3),
    shape(tops, "white", 0.2),
    shape(archRect(f, slit - 0.6, -9, 1.2, 5), "black", 0.85),
  ];
}

/** A tall standing mirror on little feet, 12 wide and 26 tall at scale 1. */
function mirrorPart(at: PartPlace, rng: Rng): KitShape[] {
  const f = local(at);
  const o = between(rng, -0.6, 0.6);
  return [
    shape(`M${f.p(-6, -3)}L${f.p(-6, -21)}Q${f.p(-6, -26)} ${f.p(0, -26)}Q${f.p(6, -26)} ${f.p(6, -21)}L${f.p(6, -3)}Z`, "wood-700", 1, MAIN_INKED),
    shape(`M${f.p(-4.8, -4.2)}L${f.p(-4.8, -20.6)}Q${f.p(-4.8, -24.8)} ${f.p(0, -24.8)}Q${f.p(4.8, -24.8)} ${f.p(4.8, -20.6)}L${f.p(4.8, -4.2)}Z`, "slate-100", 0.95),
    shape(f.poly([[-4.8, -9 + o], [4.8, -17.5 + o], [4.8, -13.5 + o], [-4.8, -5.2 + o]]), "sky-300", 0.4),
    shape(f.band([[-3.6, -16.8], [-1.6, -20.8]], 1.1) + f.band([[-1.1, -14], [0.6, -17]], 0.6), "white", 0.6),
    shape(f.poly([[-6, -3], [-3, -3], [-4.2, 0], [-6.8, 0]]) + f.poly([[3, -3], [6, -3], [6.8, 0], [4.2, 0]]), "wood-700", 1, INKED),
  ];
}

/** A plinth standing on the anchor, 8 wide and 10 tall at scale 1. */
function pedestalPart(at: PartPlace, rng: Rng): KitShape[] {
  const f = local(at);
  void rng;
  return [
    shape(archRect(f, -4, -10, 8, 1.8) + archRect(f, -2.8, -8.2, 5.6, 6.4) + archRect(f, -4, -1.8, 8, 1.8), "slate-100", 1, MAIN_INKED),
    shape(archRect(f, 1.2, -10, 2.8, 1.8) + archRect(f, 1.2, -8.2, 1.6, 6.4) + archRect(f, 1.2, -1.8, 2.8, 1.8), "slate-900", 0.25),
    shape(archRect(f, -2.8, -8.2, 1, 6.4), "white", 0.4),
  ];
}

/** A rope bridge from 30 left to 30 right of the anchor at the anchor's height, sagging 5 at the middle. */
function bridgePart(at: PartPlace, rng: Rng): KitShape[] {
  const f = local(at);
  const sag = 5 * between(rng, 0.92, 1.08);
  const dip = (x: number): number => sag * (1 - (x / 30) ** 2);
  const slats = 20;
  let planks = "";
  for (let i = 0; i < slats; i += 1) {
    const x0 = -30 + i * 3 + 0.2;
    const x1 = x0 + 2.6;
    planks += f.poly([[x0, dip(x0)], [x1, dip(x1)], [x1, dip(x1) + 1.3], [x0, dip(x0) + 1.3]]);
  }
  const curve = (lift: number): Point[] => [-30, -22, -15, -7.5, 0, 7.5, 15, 22, 30].map((x) => [x, dip(x) - lift] as const);
  let posts = "";
  for (let i = 0; i <= 10; i += 1) {
    const x = -30 + i * 6;
    const x0 = Math.min(Math.max(x, -29.8), 29.6);
    posts += archRect(f, x0, dip(x0) - 7, 0.4, 7);
  }
  return [
    shape(planks, "wood-600", 1, MAIN_INKED),
    shape(f.band(curve(7), 0.8) + f.band(curve(3), 0.6), "wood-700", 1),
    shape(posts, "wood-700", 0.9),
  ];
}

/** A lighthouse standing on the anchor, 8 wide at its base and 36 tall at scale 1. */
function lighthousePart(at: PartPlace, rng: Rng): KitShape[] {
  const f = local(at);
  const half = (y: number): number => 4 - 1.4 * (-y / 26);
  const band = (y0: number, y1: number): Point[] => [[-half(y0), y0], [-half(y1), y1], [half(y1), y1], [half(y0), y0]];
  const o = between(rng, -0.5, 0.5);
  let posts = "";
  for (let i = 0; i < 5; i += 1) posts += archRect(f, -4.1 + i * 2.05, -29, 0.4, 2.2);
  return [
    shape(f.ellipse(0, -31, 8.5, 8.5), "white", 0.12),
    shape(f.ellipse(0, -31, 5.5, 5.5), "orange-400", 0.22),
    shape(f.poly([[-4, 0], [-2.6, -26], [2.6, -26], [4, 0]]), "white", 1, MAIN_INKED),
    shape(f.poly(band(-4.5, -8.5 + o)) + f.poly(band(-13, -17 + o)) + f.poly(band(-21.5, -25)), "red-600", 1),
    shape(archRect(f, -4.4, -26.8, 8.8, 0.8) + posts + archRect(f, -4.4, -29.4, 8.8, 0.4), "slate-900", 0.85),
    shape(archRect(f, -2.2, -32.6, 4.4, 3.2), "orange-400", 0.6),
    shape(archRect(f, -1.2, -32, 2.4, 2), "white", 0.9),
    shape(f.poly([[-3, -32.6], [0, -36], [3, -32.6]]), "slate-900", 1, INKED),
  ];
}

/** A stone wishing well with a little roof standing on the anchor, 16 wide and 18 tall at scale 1. */
function wellPart(at: PartPlace, rng: Rng): KitShape[] {
  const f = local(at);
  const drop = between(rng, -0.6, 0.6);
  return [
    shape(f.poly([[-6.4, 0], [-6.2, -6], [6.2, -6], [6.4, 0]]), "stone-500", 1, MAIN_INKED),
    shape(f.ellipse(0, -6, 6.2, 2), "slate-400", 1, INKED),
    shape(f.ellipse(0, -6, 4.8, 1.4), "slate-900", 0.95),
    shape(archRect(f, 0.9, -12.7, 0.3, 2.8 + drop) + f.poly([[-0.4, -9.9 + drop], [2.4, -9.9 + drop], [2, -8 + drop], [0, -8 + drop]]), "wood-600", 1),
    shape(archRect(f, -5.6, -15, 1.1, 9.2) + archRect(f, 4.5, -15, 1.1, 9.2), "wood-700", 1, INKED),
    shape(archRect(f, -4.5, -13.3, 9, 0.6) + archRect(f, 5.6, -13.3, 1.2, 0.6) + archRect(f, 6.2, -13.3, 0.6, 2.6), "brass-600", 1),
    shape(f.poly([[-8, -14.4], [0, -18], [8, -14.4]]), "wood-700", 1, INKED),
    shape(f.poly([[-7.4, -14.7], [0, -17.7], [0, -14.8]]), "white", 0.18),
  ];
}

/** A glass partition standing on the anchor, 30 wide and 26 tall at scale 1. */
function glassWallPart(at: PartPlace, rng: Rng): KitShape[] {
  const f = local(at);
  void rng;
  return [
    shape(archRect(f, -15, -26, 30, 26), "sky-300", 0.15, MAIN),
    shape(
      archRect(f, -15, -26, 30, 1) + archRect(f, -15, -1, 30, 1) + archRect(f, -15, -25, 1, 24) + archRect(f, 14, -25, 1, 24) +
        archRect(f, -5.4, -25, 0.8, 24) + archRect(f, 4.6, -25, 0.8, 24) +
        archRect(f, -14, -13.6, 8.6, 0.8) + archRect(f, -4.6, -13.6, 9.2, 0.8) + archRect(f, 5.4, -13.6, 8.6, 0.8),
      "slate-400", 1, INKED,
    ),
    shape(f.band([[-12, -3], [-4, -23]], 2.2) + f.band([[-2, -3], [3, -15]], 1.2), "white", 0.25),
  ];
}

/** A city skyline across 80 centred on the anchor, 30 tall at scale 1, one tower with a spire. */
function skylinePart(at: PartPlace, rng: Rng): KitShape[] {
  const f = local(at);
  const n = 7;
  const w = 80 / n;
  const spire = whole(rng, 2, 4);
  let towers = "";
  let lit = "";
  let dots = "";
  for (let i = 0; i < n; i += 1) {
    const x = -40 + i * w;
    const h = i === spire ? 22 : between(rng, 11, 21);
    towers += archRect(f, x, -h, w, h);
    lit += archRect(f, x, -h, w * 0.36, h);
    if (i === spire) towers += f.poly([[x + w / 2 - 0.5, -h], [x + w / 2, -30], [x + w / 2 + 0.5, -h]]);
    for (let row = 0; row < 3; row += 1) {
      for (let col = 0; col < 2; col += 1) {
        const dy = -h + 2.6 + row * 3.4;
        if (dy < -2 && rng() < 0.6) dots += archRect(f, x + 2.4 + col * (w * 0.45), dy, 1.1, 1.1);
      }
    }
  }
  return [
    shape(towers, "blue-800", 1, MAIN_INKED),
    shape(lit, "sky-300", 0.3),
    shape(dots || archRect(f, -30, -8, 1, 1), "white", 0.85),
  ];
}

/** A tall bookcase standing on the anchor, 20 wide and 34 tall at scale 1. */
function bookcasePart(at: PartPlace, rng: Rng): KitShape[] {
  const f = local(at);
  const colours: readonly PaletteName[] = ["red-600", "blue-800", "green-700", "brass-600", "slate-100"];
  const spines = colours.map(() => "");
  for (let row = 0; row < 4; row += 1) {
    const floor = -1 - 8.2 * row;
    let x = -8.6;
    while (x < 7.4) {
      const w = between(rng, 0.9, 1.7);
      if (rng() < 0.9) {
        const k = whole(rng, 0, 4);
        const h = between(rng, 4.6, 6.8);
        spines[k] = (spines[k] ?? "") + archRect(f, x, floor - h, w, h);
      }
      x += w + 0.1;
    }
  }
  return [
    shape(
      archRect(f, -10, -34, 1.2, 34) + archRect(f, 8.8, -34, 1.2, 34) + archRect(f, -8.8, -34, 17.6, 1.4) +
        [0, 1, 2, 3].map((row) => archRect(f, -8.8, -1 - 8.2 * row, 17.6, 1)).join(""),
      "wood-700", 1, MAIN_INKED,
    ),
    ...colours.map((c, k) => shape(spines[k] || archRect(f, 8, -1.5, 0.1, 0.1), c, 1)),
  ];
}

/** A greenhouse standing on the anchor, 70 wide and 40 tall at scale 1. */
function greenhousePart(at: PartPlace, rng: Rng): KitShape[] {
  const f = local(at);
  const roofTop = (x: number): number => -22 - 18 * (1 - Math.abs(x) / 35);
  let bars = "";
  for (let i = 0; i < 8; i += 1) {
    const x = -35 + i * 10;
    const cx = i === 0 ? x + 0.4 : i === 7 ? x - 0.4 : x;
    bars += f.poly([[cx - 0.4, 0], [cx - 0.4, roofTop(cx) + 0.6], [cx + 0.4, roofTop(cx) + 0.6], [cx + 0.4, 0]]);
    if (i < 7) {
      const a = i === 0 ? x + 0.8 : x + 0.4;
      const b = i === 6 ? x + 10 - 0.8 : x + 10 - 0.4;
      bars += archRect(f, a, -22.4, b - a, 0.8) + archRect(f, a, -11.4, b - a, 0.8);
    }
  }
  const band = (sign: number): string =>
    f.band([[sign * 35, -22], [sign * 0.9, roofTop(0.9)]], 0.9);
  const lean = between(rng, -0.3, 0.3);
  return [
    shape(f.poly([[-35, 0], [-35, -22], [0, -40], [35, -22], [35, 0]]), "sky-300", 0.15),
    shape(bars + band(-1) + band(1), "white", 1, MAIN),
    shape(f.poly([[-2, -38.8], [0, -40.8], [2, -38.8]]), "slate-400", 1),
    shape(archRect(f, -3.4, -13 - lean, 6.8, 13 + lean), "wood-700", 1),
    shape(archRect(f, -2.2, -11.6, 4.4, 5.4), "sky-300", 0.55),
  ];
}

/** A tall standing stone on the anchor, 5 wide and 12 tall at scale 1. */
function standingStonePart(at: PartPlace, rng: Rng): KitShape[] {
  const f = local(at);
  const j = (): number => between(rng, -0.25, 0.25);
  return [
    shape(f.poly([[-2.5, 0], [-2.2 + j(), -6], [-1.6 + j(), -10.5], [0.3, -12], [2 + j(), -10.2], [2.4 + j(), -5], [2.5, 0]]), "stone-500", 1, MAIN_INKED),
    shape(f.poly([[-2.4, -0.5], [-2.1, -6], [-1.5, -10.3], [0.1, -11.6], [-0.2, -6], [-0.9, -0.5]]), "slate-100", 0.3),
    shape(f.poly([[0.9, -0.5], [1.6, -5], [1.5, -10], [2.4, -5], [2.5, 0]]), "slate-900", 0.22),
    shape(f.ellipse(1.1, -4.2, 1.1, 0.7) + f.ellipse(-0.7, -8.6, 0.8, 0.55) + f.ellipse(0, -0.2, 2.4, 0.6), "green-700", 0.85),
  ];
}

/** A distant fortress standing on the anchor, 40 wide and 22 tall at scale 1. */
function fortressPart(at: PartPlace, rng: Rng): KitShape[] {
  const f = local(at);
  const m = 6;
  const mw = 2.2;
  const gap = (22 - m * mw) / (m - 1);
  const wall: Point[] = [[-11, 0]];
  for (let i = 0; i < m; i += 1) {
    const x = -11 + i * (mw + gap);
    wall.push([x, -9], [x, -11], [x + mw, -11], [x + mw, -9]);
    if (i < m - 1) wall.push([x + mw + gap, -9]);
  }
  wall.push([11, 0]);
  const r = between(rng, -0.5, 0.5);
  return [
    shape(f.poly(wall) + archRect(f, -20, -15, 9, 15) + archRect(f, 11, -15, 9, 15), "slate-700", 1, MAIN_INKED),
    shape(archRect(f, -15.5, -15, 4.5, 15) + archRect(f, 15.5, -15, 4.5, 15), "slate-900", 0.28),
    shape(f.poly([[-21, -15], [-15.5, -19 + r], [-10, -15]]) + f.poly([[10, -15], [15.5, -19 + r], [21, -15]]), "slate-800", 1, INKED),
    shape(archDoor(f, 0, 0, 4.4, -6) + archRect(f, -15.8, -22, 0.4, 3.4) + archRect(f, 15.2, -22, 0.4, 3.4), "black", 0.9),
    shape(f.poly([[-15.4, -22], [-12.4, -21], [-15.4, -20]]) + f.poly([[15.6, -22], [18.6, -21], [15.6, -20]]), "red-600", 1),
  ];
}

/** A red barn standing on the anchor, 24 wide and 20 tall at scale 1. */
function barnPart(at: PartPlace, rng: Rng): KitShape[] {
  const f = local(at);
  void rng;
  const leaf = (a: number, b: number, up: boolean): string =>
    up ? f.band([[a, -0.4], [b, -7]], 0.55) : f.band([[a, -7], [b, -0.4]], 0.55);
  return [
    shape(f.poly([[-10.5, 0], [-10.5, -9], [-8.5, -14], [0, -18.7], [8.5, -14], [10.5, -9], [10.5, 0]]), "red-600", 1, MAIN_INKED),
    shape(f.band([[-12, -8.6], [-9, -14.6], [0, -19.8], [9, -14.6], [12, -8.6]], 2.2), "slate-700", 1, INKED),
    shape(archRect(f, -3.4, -7.4, 6.8, 7.4), "red-900", 1),
    shape(archRect(f, -1.6, -16, 3.2, 3.8), "black", 0.85),
    shape(
      f.poly([[-4, 0], [-4, -8], [4, -8], [4, 0], [3.4, 0], [3.4, -7.4], [-3.4, -7.4], [-3.4, 0]]) + archRect(f, -0.3, -7.4, 0.6, 7.4) +
        f.poly([[-2.2, -11.6], [-2.2, -16.6], [2.2, -16.6], [2.2, -11.6], [1.6, -11.6], [1.6, -16], [-1.6, -16], [-1.6, -11.6]]),
      "white", 1,
    ),
    shape(leaf(-3.2, -0.6, true) + leaf(0.6, 3.2, true), "white", 0.9),
    shape(leaf(-3.2, -0.6, false) + leaf(0.6, 3.2, false), "white", 0.9),
  ];
}

/** A lakeside pavilion standing on the anchor, 30 wide and 26 tall at scale 1. */
function pavilionPart(at: PartPlace, rng: Rng): KitShape[] {
  const f = local(at);
  const o = between(rng, -0.4, 0.4);
  return [
    shape(archRect(f, -11, -14, 22, 11), "slate-900", 0.35),
    shape(archRect(f, -12.2, -16, 1.4, 13) + archRect(f, -4.7, -14.6, 1.4, 11.6) + archRect(f, 3.3, -14.6, 1.4, 11.6) + archRect(f, 10.8, -16, 1.4, 13), "red-900", 1),
    shape(archRect(f, -14, -3.2, 28, 1.6), "wood-600", 1, INKED),
    shape(archRect(f, -12.5, -1.6, 1, 1.6) + archRect(f, -3, -1.6, 1, 1.6) + archRect(f, 2, -1.6, 1, 1.6) + archRect(f, 11.5, -1.6, 1, 1.6), "wood-700", 1),
    shape(
      `M${f.p(-15, -18)}Q${f.p(-8, -14 + o)} ${f.p(0, -14 + o)}Q${f.p(8, -14 + o)} ${f.p(15, -18)}` +
        `Q${f.p(7, -19.5)} ${f.p(3, -24.5)}L${f.p(-3, -24.5)}Q${f.p(-7, -19.5)} ${f.p(-15, -18)}Z`,
      "red-600", 1, MAIN_INKED,
    ),
    shape(f.poly([[-2.5, -24], [2.5, -24], [4.5, -21.5], [-4.5, -21.5]]), "white", 0.25),
    shape(f.poly([[-0.7, -24.5], [0, -26], [0.7, -24.5]]), "red-900", 1),
  ];
}

/** A treehouse standing on the anchor, 34 wide and 40 tall at scale 1, a rope ladder hanging on its right. */
function treehousePart(at: PartPlace, rng: Rng): KitShape[] {
  const f = local(at);
  const k = (): number => between(rng, -0.4, 0.4);
  let rungs = "";
  for (let y = -22; y < -3; y += 3.2) rungs += archRect(f, 7.6, y, 1.8, 0.5);
  return [
    shape(f.poly([[-5, 0], [-2.6, -6], [-2.2, -26], [2.2, -26], [2.6, -6], [5, 0]]), "wood-700", 1, INKED),
    shape(f.band([[-2.4, -14], [-9, -24.5]], 1.1) + f.band([[2.4, -14], [9, -24.5]], 1.1), "wood-700", 1),
    shape(archRect(f, -13, -26, 26, 1.6), "wood-600", 1, INKED),
    shape(archRect(f, -9, -26, 0.3, 1.6) + archRect(f, -4.5, -26, 0.3, 1.6) + archRect(f, 0, -26, 0.3, 1.6) + archRect(f, 4.5, -26, 0.3, 1.6) + archRect(f, 9, -26, 0.3, 1.6), "wood-800", 0.7),
    shape(archRect(f, -8, -34, 12, 8), "brass-600", 1, MAIN_INKED),
    shape(f.poly([[-9.5, -34], [-2, -39.5], [5.5, -34]]), "wood-800", 1, INKED),
    shape(archRect(f, -5, -32, 3.6, 3.4), "orange-400", 0.9),
    shape(f.circle(-14, -29, 5 + k()) + f.circle(-9.8, -37, 3.2 + k() * 0.4) + f.circle(11, -30, 5.5 + k()) + f.circle(9, -38.5, 3 + k() * 0.4), "green-700", 1, INKED),
    shape(f.circle(-15.5, -30.5, 2.2) + f.circle(9.5, -31.5, 2.4), "green-500", 0.5),
    shape(archRect(f, 7, -24.4, 0.4, 22) + archRect(f, 9.8, -24.4, 0.4, 22) + rungs, "brass-500", 0.9),
  ];
}

/** A skate quarter-pipe standing on the anchor, 26 wide and 12 tall at scale 1, its lip on the right. */
function rampPart(at: PartPlace, rng: Rng): KitShape[] {
  const f = local(at);
  void rng;
  const arc = (t: number): Point => [-1 + 12 * Math.sin(t * DEGREES), -12 + 12 * Math.cos(t * DEGREES)];
  const [sx, sy] = arc(50);
  const rad = fmt(f.r(12));
  const hi: Point[] = [10, 25, 40, 55, 70, 82].map((t) => arc(t));
  return [
    shape(`M${f.p(-13, 0)}L${f.p(-1, 0)}A${rad} ${rad} 0 0 ${f.sweep(0)} ${f.p(11, -12)}L${f.p(13, -12)}L${f.p(13, 0)}Z`, "slate-400", 1, MAIN_INKED),
    shape(`M${f.p(sx, sy)}A${rad} ${rad} 0 0 ${f.sweep(0)} ${f.p(11, -12)}L${f.p(13, -12)}L${f.p(13, 0)}L${f.p(sx, 0)}Z`, "slate-900", 0.28),
    shape(archRect(f, 9.8, -12.9, 3.2, 0.9) + f.band(hi, 0.6), "slate-100", 0.9),
  ];
}

/** A volcanic forge standing on the anchor, 30 wide and 24 tall at scale 1. */
function forgePart(at: PartPlace, rng: Rng): KitShape[] {
  const f = local(at);
  const j = (): number => between(rng, -0.5, 0.5);
  return [
    shape(f.poly([[-15, 0], [-13.5, -11 + j()], [-9, -17 + j()], [-4, -18], [4, -18], [9, -17 + j()], [13.5, -11 + j()], [15, 0]]), "slate-800", 1, MAIN_INKED),
    shape(f.poly([[3, -18], [4, -24], [9, -24], [10, -17]]), "slate-700", 1, INKED),
    shape(f.ellipse(6.5, -24.5, 4.5, 2.2), "orange-400", 0.35),
    shape(f.ellipse(0, -5, 11, 9), "orange-400", 0.14),
    shape(archDoor(f, 0, 0, 12, -12), "orange-400", 1),
    shape(archDoor(f, 0, 0, 8.4, -9.4), "ember-500", 0.95),
    shape(f.ellipse(0, -3.6, 2.6, 3.4), "white", 0.85),
    shape(f.band([[-9, -13], [-9.4, -6]], 1) + f.band([[9, -12], [9.3, -3]], 1.1) + f.band([[-11.5, -8], [-11.6, -3.6]], 0.8) + f.circle(-9.4, -4.9, 0.8) + f.circle(9.3, -1.9, 0.8), "ember-500", 0.95),
  ];
}

/**
 * A twin-towered belfry standing on the anchor, 36 wide and 40 tall at scale 1. Its open bell chambers are
 * centred 22.5 above the anchor and 13 to each side: (-13, -22.5) and (13, -22.5) facing right.
 */
function belfryPart(at: PartPlace, rng: Rng): KitShape[] {
  const f = local(at);
  void rng;
  const chamber = (cx: number): string => archDoor(f, cx, -19, 5.4, -26);
  return [
    shape(archRect(f, -18, -28, 10, 28) + archRect(f, 8, -28, 10, 28) + archRect(f, -8, -12, 16, 12), "slate-100", 1, MAIN_INKED),
    shape(archRect(f, -13, -28, 5, 28) + archRect(f, 13, -28, 5, 28) + archRect(f, 0, -12, 8, 12), "slate-900", 0.14),
    shape(chamber(-13) + chamber(13), "slate-900", 0.9),
    shape(f.poly([[-19, -28], [-13, -40], [-7, -28]]) + f.poly([[7, -28], [13, -40], [19, -28]]), "blue-800", 1, INKED),
    shape(f.poly([[-18.4, -28.4], [-13, -39], [-13, -28.4]]) + f.poly([[7.6, -28.4], [13, -39], [13, -28.4]]), "white", 0.2),
    shape(archDoor(f, 0, 0, 4.2, -7.6), "wood-700", 1),
  ];
}

const ROTUNDA_STEP = 11;
const ROTUNDA_REACH = 42;

/** The measure of the rotunda's doorway `i` (0 to 12, left to right): its offsets and size. */
function rotundaDoor(i: number): { x: number; base: number; w: number; top: number } {
  const phi = ((i - 6) * ROTUNDA_STEP * Math.PI) / 180;
  const c = Math.cos(phi);
  const base = -6 * c;
  return { x: ROTUNDA_REACH * Math.sin(phi), base, w: 6.3 * c, top: base - (8 + 12 * c) };
}

/** Offsets of the rotunda's 13 doorway centres from its anchor at scale 1, facing right, left to right. */
export const ROTUNDA_DOORS: readonly (readonly [number, number])[] = Array.from({ length: 13 }, (_, i) => {
  const d = rotundaDoor(i);
  return [Math.round(d.x * 10) / 10, Math.round(((d.base + d.top) / 2) * 10) / 10] as const;
});

/** A white marble rotunda round the anchor: a curved back wall of 13 arched doorways, 90 wide and 40 tall at scale 1. */
function rotundaPart(at: PartPlace, rng: Rng): KitShape[] {
  const f = local(at);
  const sag = between(rng, 0.95, 1.05);
  const wallTop = (x: number): number => -30 + 6 * (x / 42) ** 2;
  const base: Point[] = [];
  for (let i = -90; i <= 90; i += ROTUNDA_STEP) {
    const phi = Math.abs(i) > 66 ? Math.sign(i) * 90 : i;
    base.push([ROTUNDA_REACH * Math.sin((phi * Math.PI) / 180), 0.9 - 6 * Math.cos((phi * Math.PI) / 180)]);
  }
  const edge: Point[] = [];
  for (let x = -42; x <= 42; x += 14) edge.push([x, wallTop(x)]);
  const doors: string[] = [];
  const glows: string[] = ["", "", "", "", ""];
  const group = [0, 1, 1, 2, 2, 3, 4];
  for (let i = 0; i < 13; i += 1) {
    const d = rotundaDoor(i);
    doors.push(archDoor(f, d.x, d.base, d.w, d.top));
    const g = group[Math.abs(i - 6)] ?? 0;
    glows[g] = (glows[g] ?? "") + archDoor(f, d.x, d.base, d.w * 0.7, d.top + 1.2);
  }
  const colours: readonly PaletteName[] = ["orange-400", "sky-300", "violet-300", "green-400", "cyan-400"];
  return [
    shape(f.ellipse(0, -2.5, 46, 5.5 * sag) + f.ellipse(0, -2.5, 36, 3.4), "stone-500", 1, INKED),
    shape(`M${f.p(-36, -25.6)}Q${f.p(0, -54.4)} ${f.p(36, -25.6)}Z`, "slate-100", 0.9, INKED),
    shape(`M${f.p(0, -40)}Q${f.p(18, -40)} ${f.p(36, -25.6)}L${f.p(0, -25.6)}Z`, "slate-900", 0.15),
    shape(f.poly([...base, ...edge.slice().reverse()]) + doors.join(""), "slate-100", 1, MAIN_INKED),
    shape(f.band(edge.map(([x, y]) => [x, y + 1.3] as const), 2.6), "slate-400", 0.9),
    ...colours.map((c, k) => shape(glows[k] ?? "", c, 0.8)),
  ];
}

/** A row of three townhouses at dusk standing on the anchor, 40 wide and 34 tall at scale 1, a cat-folk in each lit window. */
function townhousePart(at: PartPlace, rng: Rng): KitShape[] {
  const f = local(at);
  const w = 40 / 3;
  const heights = [between(rng, 19, 21), between(rng, 23, 25), between(rng, 20, 22)];
  const cat: readonly Point[] = [[-1.4, 2.3], [-1.3, 0.6], [-0.9, -0.3], [-1, -1.3], [-0.8, -2.1], [-0.2, -1.35], [0.2, -1.35], [0.8, -2.1], [1, -1.3], [0.9, -0.3], [1.3, 0.6], [1.4, 2.3]];
  let walls = "";
  let roofs = "";
  let windows = "";
  let cats = "";
  heights.forEach((h, i) => {
    const x = -20 + i * w;
    const cx = x + w / 2;
    walls += archRect(f, x, -h, w, h);
    roofs += f.poly([[x, -h], [cx, -h - (i === 1 ? 10 : 8)], [x + w, -h]]);
    for (const cy of [-6.2, -14.6]) {
      windows += archRect(f, cx - 1.7, cy - 2.3, 3.4, 4.6);
      cats += f.poly(cat.map(([dx, dy]) => [cx + dx, cy + dy] as const));
    }
  });
  return [
    shape(walls, "slate-700", 1, MAIN_INKED),
    shape(roofs, "slate-800", 1, INKED),
    shape(windows, "orange-400", 0.95),
    shape(cats, "black", 0.9),
    shape(archRect(f, -7, -22, 0.7, 22), "slate-400", 1),
  ];
}

/** A glass stadium of five stacked elliptical rings above the anchor, 80 wide and 50 tall at scale 1, dim below and blazing above. */
function stadiumPart(at: PartPlace, rng: Rng): KitShape[] {
  const f = local(at);
  const tier = between(rng, 8.2, 8.6);
  const paints: readonly PaletteName[] = ["slate-700", "slate-400", "sky-300", "slate-100", "white"];
  const out: KitShape[] = [];
  for (let k = 0; k < 5; k += 1) {
    const rx = 40 - 6 * k;
    const ry = rx * 0.12;
    const yb = -ry - tier * k;
    const yt = yb - tier;
    const r = fmt(f.r(rx));
    const q = fmt(f.r(ry));
    const body =
      `M${f.p(-rx, yb)}A${r} ${q} 0 0 ${f.sweep(0)} ${f.p(rx, yb)}L${f.p(rx, yt)}A${r} ${q} 0 0 ${f.sweep(0)} ${f.p(-rx, yt)}Z`;
    const paint = paints[k] ?? "white";
    out.push(shape(body, paint, k === 4 ? 1 : 0.95, k === 0 ? MAIN_INKED : INKED));
    out.push(shape(f.ellipse(0, yt, rx, ry), k === 4 ? "blue-200" : "white", 0.3 + 0.1 * k));
  }
  out.push(shape(f.ellipse(0, -tier * 5 - 3, 14, 3.4), "white", 0.5));
  return out;
}

/** A frosted steel bastion standing on the anchor, 50 wide and 36 tall at scale 1, three overlapping glassy plates. */
function bastionPart(at: PartPlace, rng: Rng): KitShape[] {
  const f = local(at);
  const plates: readonly (readonly [number, number, number])[] = [
    [-15, 10, 30 + between(rng, -1, 1)],
    [15, 10, 27 + between(rng, -1, 1)],
    [0, 11, 36],
  ];
  const out: KitShape[] = [];
  let rivets = "";
  for (const [x, hw, h] of plates) {
    out.push(shape(f.poly([[x - hw, 0], [x - hw, -h + 5], [x - hw + 3, -h], [x + hw - 3, -h], [x + hw, -h + 5], [x + hw, 0]]), "slate-100", 0.92, MAIN_INKED));
    out.push(shape(f.poly([[x, 0], [x, -h], [x + hw - 3, -h], [x + hw, -h + 5], [x + hw, 0]]), "sky-300", 0.3));
    out.push(shape(archRect(f, x - hw, -h + 5, 1, h - 5) + f.poly([[x - hw, -h + 5], [x - hw + 3, -h], [x + hw - 3, -h], [x + hw - 3, -h + 1], [x - hw + 3.4, -h + 1], [x - hw + 0.7, -h + 5]]), "white", 0.9));
    rivets += f.circle(x - hw + 2.4, -3, 0.5) + f.circle(x - hw + 2.4, -h + 7, 0.5) + f.circle(x + hw - 2.4, -3, 0.5);
  }
  out.push(shape(rivets, "slate-400", 1));
  return out;
}

/** A warrior statue on a plinth standing on the anchor, 20 tall at scale 1, its spear on the right. */
function statuePart(at: PartPlace, rng: Rng): KitShape[] {
  const f = local(at);
  const o = between(rng, -0.3, 0.3);
  return [
    shape(archRect(f, -4.4, -3.4, 8.8, 0.8) + archRect(f, -3.6, -2.6, 7.2, 1.8) + archRect(f, -4.4, -0.8, 8.8, 0.8), "slate-400", 1, INKED),
    shape(
      f.poly([[-2.3, -3.4], [-2.1, -8.4], [-2.8, -11.2], [-2.5, -13.7], [-1.2, -14.2], [-1.4, -15.7], [-0.7, -17], [0.5, -17], [1.1, -15.7], [1, -14.2], [2.5, -13.7], [2.9, -11.2], [2.1, -8.4], [2.3, -3.4], [0.5, -3.4], [0.3, -7.6], [-0.3, -7.6], [-0.5, -3.4]]),
      "stone-500", 1, MAIN_INKED,
    ),
    shape(f.band([[2.4, -12.6], [4.35, -11.9 + o]], 0.9), "stone-500", 1, MAIN),
    shape(archRect(f, 4.1, -18.2, 0.5, 14.8) + f.poly([[3.6, -18.2], [4.35, -20], [5.1, -18.2]]), "slate-100", 1),
    shape(f.band([[-2.1, -8.4], [-2.8, -11.2], [-2.5, -13.7], [-1.2, -14.2]], 0.6), "white", 0.35),
  ];
}

/** A city tower bending flat like folded paper, standing on the anchor, 30 tall at scale 1, bending to the side it faces. */
function paperTowerPart(at: PartPlace, rng: Rng): KitShape[] {
  const f = local(at);
  const bend = between(rng, 0.45, 0.6);
  const rise = Math.sqrt(1 - bend * bend);
  const up = (px: number, q: number): Point => [px + q * bend, -12 - q * rise];
  const len = 18 / rise;
  let windows = "";
  for (const cx of [-3.6, 1.6]) {
    for (const y of [-10.2, -6.6, -3]) windows += archRect(f, cx, y, 2, 1.8);
    for (const q of [3, 8.2, 13.4]) windows += f.poly([up(cx, q), up(cx + 2, q), up(cx + 2, q + 2.2), up(cx, q + 2.2)]);
  }
  return [
    shape(archRect(f, -5, -12, 10, 12) + f.poly([[-5, -12], [5, -12], up(5, len), up(-5, len)]), "slate-400", 1, MAIN_INKED),
    shape(f.poly([[-5, -12], [5, -12], up(5, len), up(-5, len)]), "white", 0.26),
    shape(archRect(f, 0, -12, 5, 12), "slate-900", 0.22),
    shape(archRect(f, -5, -12.3, 10, 0.6), "slate-900", 0.6),
    shape(windows, "white", 0.6),
  ];
}
// @@ B architecture kit
  window: { category: "architecture", draw: windowPart },
  door: { category: "architecture", draw: doorPart },
  shelf: { category: "architecture", draw: shelfPart },
  counter: { category: "architecture", draw: counterPart },
  house: { category: "architecture", draw: housePart },
  dome: { category: "architecture", draw: domePart },
  "stone-wall": { category: "architecture", draw: stoneWallPart },
  battlement: { category: "architecture", draw: battlementPart },
  mirror: { category: "architecture", draw: mirrorPart },
  pedestal: { category: "architecture", draw: pedestalPart },
  bridge: { category: "architecture", draw: bridgePart },
  lighthouse: { category: "architecture", draw: lighthousePart },
  well: { category: "architecture", draw: wellPart },
  "glass-wall": { category: "architecture", draw: glassWallPart },
  skyline: { category: "architecture", draw: skylinePart },
  bookcase: { category: "architecture", draw: bookcasePart },
  greenhouse: { category: "architecture", draw: greenhousePart },
  "standing-stone": { category: "architecture", draw: standingStonePart },
  fortress: { category: "architecture", draw: fortressPart },
  barn: { category: "architecture", draw: barnPart },
  pavilion: { category: "architecture", draw: pavilionPart },
  treehouse: { category: "architecture", draw: treehousePart },
  ramp: { category: "architecture", draw: rampPart },
  forge: { category: "architecture", draw: forgePart },
  belfry: { category: "architecture", draw: belfryPart },
  rotunda: { category: "architecture", draw: rotundaPart },
  townhouse: { category: "architecture", draw: townhousePart },
  stadium: { category: "architecture", draw: stadiumPart },
  bastion: { category: "architecture", draw: bastionPart },
  statue: { category: "architecture", draw: statuePart },
  "paper-tower": { category: "architecture", draw: paperTowerPart },
