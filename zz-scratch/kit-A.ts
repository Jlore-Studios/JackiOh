// @@ A terrain
/** The box from `top` down past its bottom, across all of it. */
function belowKitA(top: number): string {
  return rectPath(EDGE, top, FAR_EDGE - EDGE, FAR_EDGE - top);
}

/** A zigzag across the box about `y`, `steps` segments, each vertex up to `amp` off. */
function acrossKitA(rng: Rng, y: number, amp: number, steps: number): Point[] {
  const out: Point[] = [];
  for (let i = 0; i <= steps; i += 1) out.push([EDGE + ((FAR_EDGE - EDGE) * i) / steps, y + (i % 2 === 0 ? -1 : 1) * amp * between(rng, 0.5, 1)]);
  return out;
}

/** Where a scene-wide path's offsets are the box's own coordinates. */
const WORLD_KIT_A: PartPlace = { x: 0, y: 0, s: 1, flip: 1 };

/** The `polygon`'s part with `nx*x + ny*y >= c`. */
function clipKitA(polygon: readonly Point[], nx: number, ny: number, c: number): Point[] {
  const out: Point[] = [];
  for (let i = 0; i < polygon.length; i += 1) {
    const a = polygon[i] ?? [0, 0];
    const b = polygon[(i + 1) % polygon.length] ?? [0, 0];
    const da = nx * a[0] + ny * a[1] - c;
    const db = nx * b[0] + ny * b[1] - c;
    if (da >= 0) out.push(a);
    if (da >= 0 !== db >= 0) {
      const t = da / (da - db);
      out.push([a[0] + (b[0] - a[0]) * t, a[1] + (b[1] - a[1]) * t]);
    }
  }
  return out;
}

/** A sea or lake from the placement's height down: waves and glints. */
function waterPart(at: PartPlace, rng: Rng): KitShape[] {
  const top = at.y;
  const depth = FAR_EDGE - top;
  const out: KitShape[] = [shape(belowKitA(top), "blue-600", 1, MAIN)];
  for (let k = 0; k < 3; k += 1) {
    const y = top + depth * (0.2 + 0.28 * k);
    out.push(shape(bandPath(acrossKitA(rng, y, 0.9 * at.s, 8), (1 + 0.7 * k) * at.s), "sky-300", 0.3 + 0.05 * k));
  }
  let glints = "";
  for (let i = 0; i < 5; i += 1) {
    const x = 10 + i * 19 + between(rng, -4, 4);
    const y = top + depth * (0.12 + ((i * 2) % 5) * 0.17) + between(rng, -1, 1);
    glints += bandPath([[x, y], [x + 5, y]], 0.5 * at.s);
  }
  out.push(shape(glints, "white", 0.8));
  return out;
}

/** Desert dunes from the placement's height down: two dune bands and faint ripples. */
function sandPart(at: PartPlace, rng: Rng): KitShape[] {
  const top = at.y;
  const depth = FAR_EDGE - top;
  let ripples = "";
  for (let k = 0; k < 3; k += 1) ripples += bandPath(acrossKitA(rng, top + depth * (0.3 + 0.25 * k) + 5, 0.8, 6), 0.6 * at.s);
  return [
    shape(belowKitA(top), "brass-500", 1, MAIN),
    shape(groundBand(rng, top + depth * 0.3, 3 * at.s, 3), "brass-600", 0.55),
    shape(groundBand(rng, top + depth * 0.58, 3 * at.s, 3), "brass-500", 1, MAIN),
    shape(ripples, "wood-600", 0.14),
  ];
}

/** A snowfield from the placement's height down: drift shadows and a few sparkles. */
function snowPart(at: PartPlace, rng: Rng): KitShape[] {
  const top = at.y;
  const depth = FAR_EDGE - top;
  const w = local(WORLD_KIT_A);
  let glitter = "";
  for (let i = 0; i < 3; i += 1) glitter += sparkle(w, 14 + i * 32 + between(rng, -5, 5), top + depth * (0.2 + 0.3 * ((i * 2) % 3)) + between(rng, -1, 1), between(rng, 1.6, 2.4) * at.s);
  return [
    shape(belowKitA(top), "white", 1, MAIN),
    shape(groundBand(rng, top + depth * 0.28, 2 * at.s, 4), "blue-200", 0.55),
    shape(groundBand(rng, top + depth * 0.28 + 3 * at.s, 2 * at.s, 4), "white", 1, MAIN),
    shape(groundBand(rng, top + depth * 0.66, 2 * at.s, 4), "blue-200", 0.5),
    shape(groundBand(rng, top + depth * 0.66 + 3 * at.s, 2 * at.s, 4), "white", 1, MAIN),
    shape(glitter, "sky-300", 0.8),
  ];
}

/** A planked floor, deck or jetty from the placement's height down: seams meeting at its x. */
function boardsPart(at: PartPlace, rng: Rng): KitShape[] {
  const top = at.y;
  const depth = FAR_EDGE - top;
  const planks = 6;
  const foot = (i: number): number => -30 + (160 * i) / planks;
  const apex = (i: number): number => at.x + (foot(i) - at.x) * 0.25;
  const seamX = (i: number, t: number): number => apex(i) + (foot(i) - apex(i)) * t;
  let seams = "";
  for (let i = 0; i < planks; i += 1) {
    seams += `M${pt(apex(i) - 0.12, top)}L${pt(apex(i) + 0.12, top)}L${pt(foot(i) + 0.5, FAR_EDGE)}L${pt(foot(i) - 0.5, FAR_EDGE)}Z`;
  }
  let ends = "";
  for (let k = 0; k < 3; k += 1) {
    const t = 0.25 + 0.25 * k + between(rng, -0.04, 0.04);
    const i = (k * 2 + 1 + whole(rng, 0, 1)) % (planks - 1);
    const x0 = seamX(i, t);
    ends += rectPath(x0 + 0.3, top + depth * t, seamX(i + 1, t) - x0 - 0.6, 0.35 + 0.5 * t);
  }
  return [
    shape(belowKitA(top), "wood-600", 1, MAIN),
    shape(seams, "wood-900", 0.5),
    shape(ends, "wood-900", 0.4),
    shape(rectPath(EDGE, top + depth * 0.86, FAR_EDGE - EDGE, depth * 0.15), "brass-600", 0.3),
  ];
}

/** A raised stage from the placement's height down: a lower front lip, footlights and a sheen. */
function stagePart(at: PartPlace, rng: Rng): KitShape[] {
  const top = at.y;
  const depth = FAR_EDGE - top;
  const lip = top + depth * 0.8;
  let lights = "";
  for (let i = 0; i < 9; i += 1) lights += circlePath(5 + i * 11.5 + between(rng, -0.6, 0.6), lip + 1.8 * at.s, 0.75 * at.s);
  return [
    shape(belowKitA(top), "slate-800", 1, MAIN),
    shape(rectPath(EDGE, lip, FAR_EDGE - EDGE, FAR_EDGE - lip), "slate-900", 1),
    shape(rectPath(EDGE, lip, FAR_EDGE - EDGE, 0.7 * at.s), "slate-700", 0.9),
    shape(lights, "white", 0.95),
    shape(ellipsePath(at.x + between(rng, -8, 8), top + depth * 0.4, 34, depth * 0.17), "white", 0.07),
  ];
}

/** A dirt road winding from narrow at the placement's point to wide at the bottom edge, verges either side. */
function roadPart(at: PartPlace, rng: Rng): KitShape[] {
  const top = at.y;
  const depth = FAR_EDGE - top;
  const bottomX = at.x + between(rng, -14, 14);
  const bend = between(rng, -12, 12);
  const centre = (t: number): number => at.x + (bottomX - at.x) * t + bend * Math.sin(Math.PI * t);
  const half = (t: number): number => (1.2 + 21 * t ** 1.2) * at.s;
  const steps = [0, 0.2, 0.4, 0.6, 0.8, 1];
  const side = (sign: number, extra: number, along: readonly number[]): Point[] => along.map((t) => [centre(t) + sign * (half(t) + extra * (0.4 + t)), top + depth * t] as const);
  const left = side(-1, 0, steps);
  const right = side(1, 0, steps).reverse();
  const verge = (sign: number): string => bandPath(side(sign, 1.8 * at.s, steps), (0.8 + 0.5) * at.s);
  const rut = (k: number): string => bandPath([0.15, 0.4, 0.65, 0.9, 1].map((t) => [centre(t) + k * half(t) * 0.38, top + depth * t] as const), 0.8 * at.s);
  return [
    shape(belowKitA(top), "green-700", 1),
    shape(`M${[...left, ...right].map(([x, y]) => pt(x, y)).join("L")}Z`, "wood-600", 1, MAIN),
    shape(verge(-1) + verge(1), "green-500", 0.3),
    shape(rut(-1) + rut(1), "wood-800", 0.3),
  ];
}

/** A chequered floor in perspective from the placement's height down, its lines meeting at its x. */
function checkerFloorPart(at: PartPlace, rng: Rng): KitShape[] {
  const top = at.y;
  const depth = FAR_EDGE - top;
  const cols = 6;
  const rows = 5;
  const foot = (i: number): number => -30 + (160 * i) / cols;
  const apex = (i: number): number => at.x + (foot(i) - at.x) * 0.25;
  const xAt = (i: number, y: number): number => apex(i) + ((foot(i) - apex(i)) * (y - top)) / depth;
  const rowY = (k: number): number => top + depth * (k / rows) ** 1.6;
  let dark = "";
  for (let k = 0; k < rows; k += 1) {
    for (let i = 0; i < cols; i += 1) {
      if ((i + k) % 2 === 0) continue;
      const y0 = rowY(k);
      const y1 = rowY(k + 1);
      dark += `M${pt(xAt(i, y0), y0)}L${pt(xAt(i + 1, y0), y0)}L${pt(xAt(i + 1, y1), y1)}L${pt(xAt(i, y1), y1)}Z`;
    }
  }
  return [
    shape(belowKitA(top), "slate-100", 1, MAIN),
    shape(dark, "slate-800", 1),
    shape(ellipsePath(at.x + between(rng, -8, 8), top + depth * 0.45, 26, depth * 0.2), "white", 0.14),
  ];
}

/** A cobbled square from the placement's height down: cobbles in two shapes, smaller toward the top. */
function cobblesPart(at: PartPlace, rng: Rng): KitShape[] {
  const top = at.y;
  const depth = FAR_EDGE - top;
  const widths = [10, 15, 22];
  const fractions = [0.15, 0.45, 0.8];
  let pale = "";
  let dark = "";
  for (let r = 0; r < 3; r += 1) {
    const w = (widths[r] ?? 10) * at.s;
    const rx = w * 0.36;
    const ry = rx * 0.38;
    const y = top + depth * (fractions[r] ?? 0.5);
    const cells = Math.ceil((FAR_EDGE - EDGE) / w);
    for (let i = 0; i < cells; i += 1) {
      const x = EDGE + w * (i + 0.5) + (r % 2) * w * 0.25 + between(rng, -0.1, 0.1) * w;
      const d = ellipsePath(x, y + between(rng, -0.1, 0.1) * ry, rx * between(rng, 0.9, 1), ry);
      if ((i + r) % 2 === 0) pale += d;
      else dark += d;
    }
  }
  return [shape(belowKitA(top), "stone-500", 1, MAIN), shape(pale, "slate-400", 1), shape(dark, "slate-700", 1)];
}

/** A lava field from the placement's height down: a dark crust, glowing cracks and a glow along its top. */
function lavaPart(at: PartPlace, rng: Rng): KitShape[] {
  const top = at.y;
  const depth = FAR_EDGE - top;
  let hot = "";
  let core = "";
  for (let k = 0; k < 3; k += 1) {
    const line = acrossKitA(rng, top + depth * (0.25 + 0.3 * k), 2.2 * at.s, 7);
    hot += bandPath(line, (1.4 + 0.5 * k) * at.s);
    core += bandPath(line, (0.7 + 0.25 * k) * at.s);
  }
  return [
    shape(belowKitA(top), "red-900", 1, MAIN),
    shape(rectPath(EDGE, top - 2, FAR_EDGE - EDGE, 6 * at.s), "orange-400", 0.3),
    shape(rectPath(EDGE, top, FAR_EDGE - EDGE, 1.8 * at.s), "ember-500", 0.3),
    shape(hot, "orange-400", 0.9),
    shape(core, "ember-500", 0.95),
  ];
}

/** A white-tiled wall from the top of the box down to the floor at the placement's height, a floor below it. */
function tileWallPart(at: PartPlace, rng: Rng): KitShape[] {
  const floor = at.y;
  const rowGap = 9;
  const colGap = 13;
  const lines = Math.max(1, Math.floor((floor - 4) / rowGap));
  const offset = between(rng, 0, 3);
  const grout = 0.35 * at.s;
  let grid = "";
  for (let r = 0; r < lines; r += 1) grid += rectPath(EDGE, rowGap * (r + 1) - grout / 2, FAR_EDGE - EDGE, grout);
  for (let r = 0; r <= lines; r += 1) {
    const y0 = r === 0 ? EDGE : rowGap * r + grout / 2;
    const y1 = r === lines ? floor - 2.5 : rowGap * (r + 1) - grout / 2;
    for (let x = offset; x < FAR_EDGE; x += colGap) if (y1 > y0) grid += rectPath(x - grout / 2, y0, grout, y1 - y0);
  }
  return [
    shape(rectPath(EDGE, EDGE, FAR_EDGE - EDGE, floor - EDGE), "slate-100", 1, MAIN),
    shape(grid, "slate-400", 0.35),
    shape(rectPath(EDGE, floor - 2.5, FAR_EDGE - EDGE, 2.5), "slate-700", 0.85),
    shape(belowKitA(floor), "slate-400", 1),
    shape(rectPath(EDGE, floor, FAR_EDGE - EDGE, 1.2), "slate-900", 0.25),
  ];
}

/** A cave round the box: rock walls left and right and a stalactite ceiling, a floor from the placement's height down. */
function cavernPart(at: PartPlace, rng: Rng): KitShape[] {
  const floor = at.y;
  const wallL = between(rng, 12, 18);
  const wallR = SCENE_BOX - between(rng, 12, 18);
  const ceiling = floor * 0.2;
  const inner: Point[] = [
    [wallR, floor],
    [wallR + 3, floor * 0.7],
    [wallR - 2, floor * 0.45],
    [wallR - 4, ceiling],
  ];
  const teeth = 5;
  for (let i = 0; i < teeth; i += 1) {
    const x = wallR - 4 - ((wallR - wallL - 8) * (i + 0.5)) / teeth;
    inner.push([x + ((wallR - wallL - 8) / teeth) * 0.5, ceiling + between(rng, 0, 2)]);
    inner.push([x, ceiling + between(rng, 7, 15)]);
  }
  inner.push([wallL + 4, ceiling], [wallL - 2 + 4, floor * 0.45], [wallL - 3, floor * 0.7], [wallL, floor]);
  const rock = `M${pt(EDGE, floor)}L${pt(EDGE, EDGE)}L${pt(FAR_EDGE, EDGE)}L${pt(FAR_EDGE, floor)}L${inner.map(([x, y]) => pt(x, y)).join("L")}Z`;
  return [
    shape(belowKitA(floor), "slate-700", 1),
    shape(ellipsePath(50, floor + (FAR_EDGE - floor) * 0.45, 30, (FAR_EDGE - floor) * 0.2), "slate-400", 0.12),
    shape(rock, "slate-800", 1, MAIN),
    shape(bandPath(inner.slice(1), 1.4 * at.s), "slate-700", 1),
  ];
}

/** An endless warehouse aisle: shelf walls either side running to a vanishing point just above the placement, box rows, a floor. */
function aislePart(at: PartPlace, rng: Rng): KitShape[] {
  const floor = at.y;
  const vy = floor - 10;
  const gap = 5;
  const lx = at.x - gap;
  const rx = at.x + gap;
  const farTop = vy - 6;
  const farBot = floor;
  const lerp = (a: number, b: number, t: number): number => a + (b - a) * t;
  const wallY = (u: number, v: number): number => lerp(lerp(EDGE, FAR_EDGE, v), lerp(farTop, farBot, v), u);
  const wallX = (side: number, u: number): number => (side < 0 ? lerp(EDGE, lx, u) : lerp(FAR_EDGE, rx, u));
  const boards = [0.34, 0.6, 0.86];
  const walls =
    `M${pt(EDGE, EDGE)}L${pt(lx, farTop)}L${pt(lx, farBot)}L${pt(EDGE, FAR_EDGE)}Z` +
    `M${pt(FAR_EDGE, EDGE)}L${pt(rx, farTop)}L${pt(rx, farBot)}L${pt(FAR_EDGE, FAR_EDGE)}Z`;
  let lines = "";
  for (const v of boards) for (const side of [-1, 1]) lines += bandPath([[wallX(side, 0), wallY(0, v)], [wallX(side, 1), wallY(1, v)]], 1.1 * at.s);
  const spans: readonly (readonly [number, number])[] = [[0.04, 0.3], [0.38, 0.58], [0.64, 0.78], [0.82, 0.9]];
  let crates = "";
  for (const v of [0.6, 0.86]) {
    for (const side of [-1, 1]) {
      for (const [a, b] of spans) {
        const h = 0.16 + between(rng, -0.02, 0.02);
        crates += `M${pt(wallX(side, a), wallY(a, v - h))}L${pt(wallX(side, b), wallY(b, v - h))}L${pt(wallX(side, b), wallY(b, v))}L${pt(wallX(side, a), wallY(a, v))}Z`;
      }
    }
  }
  return [
    shape(rectPath(EDGE, EDGE, FAR_EDGE - EDGE, floor - EDGE), "slate-800", 1),
    shape(`M${pt(lx, farBot)}L${pt(rx, farBot)}L${pt(FAR_EDGE, FAR_EDGE)}L${pt(EDGE, FAR_EDGE)}Z`, "slate-400", 1),
    shape(walls, "wood-600", 1, MAIN),
    shape(lines, "wood-800", 0.8),
    shape(crates, "brass-600", 1),
    shape(rectPath(lx + 1, farTop + 3, gap * 2 - 2, farBot - farTop - 4), "slate-100", 0.3),
  ];
}

/** A sunken amphitheatre centred on the anchor: four tiered steps down to an oval floor, 80 wide and 26 tall at scale 1. */
function amphitheatrePart(at: PartPlace, rng: Rng): KitShape[] {
  const f = local(at);
  const out: KitShape[] = [];
  for (let k = 0; k < 4; k += 1) {
    const rx = 40 - k * 7 + between(rng, -0.4, 0.4);
    const ry = 13 - k * 2.3;
    out.push(shape(f.ellipse(0, k * 1.4, rx, ry), k % 2 === 0 ? "stone-500" : "slate-400", 1, k % 2 === 0 ? MAIN : undefined));
  }
  out.push(shape(f.ellipse(0, 5.6, 12, 3.8), "slate-700", 1));
  out.push(shape(f.ellipse(-3, 4.8, 5, 1.2), "white", 0.12));
  return out;
}

/** A vast glass chessboard floating in perspective, centred on the anchor: 80 wide near, 40 far, 24 deep at scale 1. */
function chessboardPart(at: PartPlace, rng: Rng): KitShape[] {
  const f = local(at);
  const half = (dy: number): number => 20 + (20 * (dy + 12)) / 24;
  const cols = 6;
  const rows = 4;
  let dark = "";
  for (let r = 0; r < rows; r += 1) {
    for (let c = 0; c < cols; c += 1) {
      if ((r + c) % 2 === 0) continue;
      const y0 = -12 + (24 * r) / rows;
      const y1 = -12 + (24 * (r + 1)) / rows;
      const u0 = -1 + (2 * c) / cols;
      const u1 = -1 + (2 * (c + 1)) / cols;
      dark += f.poly([[u0 * half(y0), y0], [u1 * half(y0), y0], [u1 * half(y1), y1], [u0 * half(y1), y1]]);
    }
  }
  const slide = between(rng, -6, 4);
  return [
    shape(f.ellipse(0, 19, 34, 3.5), "slate-900", 0.3),
    shape(f.poly([[-20, -12], [-20, -9], [-40, 15.2], [40, 15.2], [20, -9], [20, -12], [40, 12], [-40, 12]]), "slate-400", 1),
    shape(f.poly([[-20, -12], [20, -12], [40, 12], [-40, 12]]), "slate-100", 0.75, MAIN),
    shape(dark, "slate-700", 0.9),
    shape(f.poly([[slide, -12], [slide + 8, -12], [slide - 12, 12], [slide - 22, 12]]), "white", 0.16),
  ];
}

/** A flat-topped desert mesa standing on the anchor, 50 wide and 20 tall at scale 1. */
function mesaPart(at: PartPlace, rng: Rng): KitShape[] {
  const f = local(at);
  const knots: readonly (readonly [number, number])[] = [[0, 25], [-5, 21], [-12, 18.5], [-20, 16]];
  const hw = (y: number): number => {
    for (let i = 0; i + 1 < knots.length; i += 1) {
      const [y0, w0] = knots[i] ?? [0, 25];
      const [y1, w1] = knots[i + 1] ?? [0, 25];
      if (y <= y0 && y >= y1) return w0 + ((w1 - w0) * (y - y0)) / (y1 - y0);
    }
    return 16;
  };
  const band = (y0: number, y1: number): string => f.poly([[-hw(y0), y0], [hw(y0), y0], [hw(y1), y1], [-hw(y1), y1]]);
  const lip = between(rng, 0, 1);
  const outline: Point[] = [[-25, 0], [-21, -5], [-18.5, -12], [-16, -20], [16, -20], [18.5, -12], [21, -5], [25, 0]];
  return [
    shape(f.poly(outline), "orange-400", 1, MAIN_INKED),
    shape(band(-10.5 - lip, -7.5 - lip), "wood-600", 0.6),
    shape(band(-17, -13.5), "red-600", 0.6),
    shape(f.poly([[8, -20], [16, -20], [18.5, -12], [21, -5], [25, 0], [12, 0]]), "slate-900", 0.22),
    shape(f.poly([[-16, -20], [16, -20], [16.4, -18.6], [-16.4, -18.6]]), "white", 0.18),
  ];
}

/** A candy-striped mountain standing on the anchor, 50 wide and 36 tall at scale 1. */
function candyMountainPart(at: PartPlace, rng: Rng): KitShape[] {
  const f = local(at);
  const peak = between(rng, -2, 2);
  const outline: Point[] = [[-25, 0], [-14, -20], [-5 + peak * 0.5, -33], [peak, -36], [5 + peak * 0.5, -33], [14, -20], [25, 0]];
  const lean = 0.8;
  let stripes = "";
  const start = -32 + between(rng, 0, 3);
  for (let i = 0; i < 6; i += 1) {
    const u0 = start + i * 10;
    const strip = clipKitA(clipKitA(outline, 1, lean, u0), -1, -lean, -(u0 + 5));
    if (strip.length >= 3) stripes += f.poly(strip);
  }
  const shade = clipKitA(outline, 1, 0, 3);
  return [
    shape(f.poly(outline), "white", 1, MAIN_INKED),
    shape(stripes, "red-500", 1),
    shape(f.poly(shade), "slate-900", 0.15),
  ];
}

/** A floating island centred on the anchor, 60 wide, its rocky underside tapering 30 below to a point, roots and a thin waterfall. */
function floatingIslandPart(at: PartPlace, rng: Rng): KitShape[] {
  const f = local(at);
  const knots: readonly (readonly [number, number])[] = [[-3, 30], [6, 24], [16, 14], [24, 5], [30, 0]];
  const hw = (y: number): number => {
    for (let i = 0; i + 1 < knots.length; i += 1) {
      const [y0, w0] = knots[i] ?? [0, 0];
      const [y1, w1] = knots[i + 1] ?? [0, 0];
      if (y >= y0 && y <= y1) return w0 + ((w1 - w0) * (y - y0)) / (y1 - y0) - 0.2;
    }
    return 0;
  };
  const band = (y0: number, y1: number): string => f.poly([[-hw(y0), y0], [hw(y0), y0], [hw(y1), y1], [-hw(y1), y1]]);
  const tip = between(rng, -2, 2);
  const underside: Point[] = [[-30, -3], [-24, 6], [-14, 16], [-5, 24], [tip, 30], [5, 24], [14, 16], [24, 6], [30, -3]];
  let roots = "";
  for (const x of [-18, -6, 9]) {
    const y = x < 0 ? (x < -10 ? 11 : 19) : 19;
    const sway = between(rng, -2, 2);
    roots += f.band([[x, y], [x + sway, y + 4], [x + sway * 1.6, y + between(rng, 7, 10)]], 0.8);
  }
  return [
    shape(f.poly(underside), "slate-700", 1, INKED),
    shape(band(3, 5.2), "stone-500", 0.7),
    shape(band(11.5, 14), "stone-500", 0.6),
    shape(f.poly([[6, -3], [30, -3], [24, 6], [14, 16], [8, 22]]), "slate-900", 0.25),
    shape(roots, "wood-700", 1),
    shape(f.ellipse(0, -3, 30, 5), "green-700", 1, MAIN_INKED),
    shape(f.ellipse(-1.5, -4.4, 28.5, 3.4), "green-500", 0.8),
    shape(f.band([[26, -3], [27.4, 2], [27.8, 10], [27.4, 20]], 1.4), "sky-300", 0.85),
    shape(f.ellipse(27.4, 22, 2.4, 1), "white", 0.45),
  ];
}
// @@ A terrain kit
  water: { category: "terrain", draw: waterPart },
  sand: { category: "terrain", draw: sandPart },
  snow: { category: "terrain", draw: snowPart },
  boards: { category: "terrain", draw: boardsPart },
  stage: { category: "terrain", draw: stagePart },
  road: { category: "terrain", draw: roadPart },
  "checker-floor": { category: "terrain", draw: checkerFloorPart },
  cobbles: { category: "terrain", draw: cobblesPart },
  lava: { category: "terrain", draw: lavaPart },
  "tile-wall": { category: "terrain", draw: tileWallPart },
  cavern: { category: "terrain", draw: cavernPart },
  aisle: { category: "terrain", draw: aislePart },
  amphitheatre: { category: "terrain", draw: amphitheatrePart },
  chessboard: { category: "terrain", draw: chessboardPart },
  mesa: { category: "terrain", draw: mesaPart },
  "candy-mountain": { category: "terrain", draw: candyMountainPart },
  "floating-island": { category: "terrain", draw: floatingIslandPart },
// @@ A plants
/** A fern standing on the anchor: five arching fronds, 8 tall at scale 1. */
function fernPart(at: PartPlace, rng: Rng): KitShape[] {
  const f = local(at);
  const tips: readonly (readonly [number, number])[] = [[-7, -4.5], [7, -4.2], [-4, -7], [4, -7.2], [0, -8]];
  return tips.map(([tx, ty], i) => {
    const x = tx + between(rng, -0.6, 0.6);
    const y = ty * between(rng, 0.93, 1.05);
    const d = `M${f.p(0, 0)}Q${f.p(x * 0.2, y * 1.15)} ${f.p(x, y)}Q${f.p(x * 0.7, y * 0.7)} ${f.p(0, 0)}Z`;
    return i < 2 ? shape(d, "green-700", 1) : shape(d, i === 4 ? "green-500" : "green-700", 1, i === 4 ? MAIN : undefined);
  });
}

/** A pine tree standing on the anchor: three stacked triangles on a trunk, 18 tall at scale 1. */
function pinePart(at: PartPlace, rng: Rng): KitShape[] {
  const f = local(at);
  const tiers: readonly (readonly [number, number, number])[] = [[-3, 6.2, -10], [-7, 5.2, -14], [-11, 4, -18]];
  const out: KitShape[] = [shape(f.poly([[-0.8, 0], [-0.8, -3.5], [0.8, -3.5], [0.8, 0]]), "wood-700", 1, INKED)];
  const lit: KitShape[] = [];
  for (const [base, hw, apex] of tiers) {
    const tip = between(rng, -0.4, 0.4);
    out.push(shape(f.poly([[-hw, base], [tip, apex], [hw, base]]), "green-900", 1, MAIN_INKED));
    lit.push(shape(f.poly([[-hw, base], [tip, apex], [tip, base]]), "green-700", 0.55));
  }
  return [...out, ...lit];
}

/** Reeds standing on the anchor with two cattails, 10 tall at scale 1. */
function reedPart(at: PartPlace, rng: Rng): KitShape[] {
  const f = local(at);
  let blades = "";
  for (const base of [-2, -0.7, 0.7, 2]) {
    const h = between(rng, 6.5, 9);
    blades += f.poly([[base - 0.35, 0], [base * 2.4 + between(rng, -0.4, 0.4), -h], [base + 0.35, 0]]);
  }
  const tall = between(rng, 9, 10.2);
  const short = tall - between(rng, 1.2, 2);
  return [
    shape(blades, "green-700", 1, MAIN),
    shape(f.band([[-1.4, 0], [-1.7, -short * 0.5], [-1.9, -short]], 0.4) + f.band([[1.4, 0], [1.8, -tall * 0.5], [2.1, -tall]], 0.4), "green-700", 1, MAIN),
    shape(f.ellipse(-1.9, -short - 1.1, 0.8, 2.2) + f.ellipse(2.1, -tall - 1.1, 0.8, 2.2), "wood-700", 1),
  ];
}

/** One tall pale flower standing on the anchor, 7 tall at scale 1. */
function flowerPart(at: PartPlace, rng: Rng): KitShape[] {
  const f = local(at);
  const lean = between(rng, -0.6, 0.6);
  let petals = "";
  for (let i = 0; i < 5; i += 1) {
    const a = ((i * 72 - 90) * Math.PI) / 180;
    petals += f.circle(lean + 1.8 * Math.cos(a), -5.8 + 1.8 * Math.sin(a), 1);
  }
  return [
    shape(f.band([[0, 0], [lean * 0.5, -3], [lean, -5.6]], 0.45), "green-500", 1),
    shape(`M${f.p(lean * 0.3, -2)}Q${f.p(2.6, -4)} ${f.p(3.6, -2.4)}Q${f.p(2, -1.4)} ${f.p(lean * 0.3, -2)}Z`, "green-500", 1),
    shape(petals, "white", 1, MAIN),
    shape(f.circle(lean, -5.8, 0.9), "orange-400", 1),
  ];
}

/** A clump of tall swaying grass standing on the anchor, 12 tall and 10 wide at scale 1. */
function tallGrassPart(at: PartPlace, rng: Rng): KitShape[] {
  const f = local(at);
  const lean = between(rng, -1.2, 1.2);
  let light = "";
  let shade = "";
  for (let i = 0; i < 7; i += 1) {
    const base = (i - 3) * 1.3;
    const h = between(rng, 8, 12);
    const tip = base * 1.4 + lean * (h / 12) * 3 + between(rng, -0.3, 0.3);
    const d = f.poly([[base - 0.4, 0], [tip, -h], [base + 0.4, 0]]);
    if (i % 2 === 0) light += d;
    else shade += d;
  }
  return [shape(light, "brass-500", 1, MAIN), shape(shade, "brass-600", 1)];
}

/** A trimmed hedge block standing on the anchor, 20 wide and 10 tall at scale 1. */
function hedgePart(at: PartPlace, rng: Rng): KitShape[] {
  const f = local(at);
  const body = `M${f.p(-10, 0)}L${f.p(-10, -8.4)}Q${f.p(-10, -10)} ${f.p(-8.4, -10)}L${f.p(8.4, -10)}Q${f.p(10, -10)} ${f.p(10, -8.4)}L${f.p(10, 0)}Z`;
  let dots = "";
  for (let r = 0; r < 2; r += 1) {
    for (let c = 0; c < 5; c += 1) dots += f.circle(-8 + c * 4 + (r % 2) * 2 + between(rng, -0.5, 0.5), -3.2 - r * 3 + between(rng, -0.4, 0.4), 0.6);
  }
  return [
    shape(body, "green-700", 1, MAIN_INKED),
    shape(`M${f.p(-10, -6.8)}L${f.p(-10, -8.4)}Q${f.p(-10, -10)} ${f.p(-8.4, -10)}L${f.p(8.4, -10)}Q${f.p(10, -10)} ${f.p(10, -8.4)}L${f.p(10, -6.8)}Z`, "green-500", 0.85),
    shape(dots, "green-900", 0.6),
    shape(f.poly([[-10, -1.8], [10, -1.8], [10, 0], [-10, 0]]), "slate-900", 0.25),
  ];
}

/** A tree stump standing on the anchor, 8 wide and 5 tall at scale 1. */
function stumpPart(at: PartPlace, rng: Rng): KitShape[] {
  const f = local(at);
  const wobble = between(rng, -0.3, 0.3);
  return [
    shape(f.poly([[-4.4, 0], [-3.5, -1.3], [-3.4, -3.9], [3.4, -3.9], [3.5, -1.3], [4.4, 0]]), "wood-700", 1, MAIN_INKED),
    shape(f.band([[-1.6, -0.4], [-1.5, -3]], 0.35) + f.band([[1.2, -0.4], [1.4 + wobble, -2.6]], 0.35), "wood-900", 0.45),
    shape(f.ellipse(0, -3.9, 3.5, 1.1), "brass-500", 1),
    shape(f.ellipse(0, -3.9, 2.5, 0.8) + f.ellipse(0, -3.9, 2.1, 0.66) + f.ellipse(0, -3.9, 1.2, 0.38) + f.ellipse(0, -3.9, 0.8, 0.24), "wood-600", 0.9),
  ];
}
// @@ A plants kit
  fern: { category: "plants", draw: fernPart },
  pine: { category: "plants", draw: pinePart },
  reed: { category: "plants", draw: reedPart },
  flower: { category: "plants", draw: flowerPart },
  "tall-grass": { category: "plants", draw: tallGrassPart },
  hedge: { category: "plants", draw: hedgePart },
  stump: { category: "plants", draw: stumpPart },
// @@ A weather
/** A plus-shaped sparkle about a point, `size` its reach. */
function plusKitA(f: Local, cx: number, cy: number, size: number): string {
  const t = size * 0.28;
  return f.poly([[cx - t, cy - size], [cx + t, cy - size], [cx + t, cy - t], [cx + size, cy - t], [cx + size, cy + t], [cx + t, cy + t], [cx + t, cy + size], [cx - t, cy + size], [cx - t, cy + t], [cx - size, cy + t], [cx - size, cy - t], [cx - t, cy - t]]);
}

/** An arc band about the anchor from angle `from` to `to` (degrees, clockwise), `width` thick at radius `r`. */
function arcKitA(f: Local, r: number, width: number, from: number, to: number): string {
  const ro = r + width / 2;
  const ri = r - width / 2;
  const at = (radius: number, deg: number): string => f.p(radius * Math.cos(deg * DEGREES), radius * Math.sin(deg * DEGREES));
  const large = Math.abs(to - from) > 180 ? 1 : 0;
  return `M${at(ro, from)}A${fmt(f.r(ro))} ${fmt(f.r(ro))} 0 ${large} ${f.sweep(1)} ${at(ro, to)}L${at(ri, to)}A${fmt(f.r(ri))} ${fmt(f.r(ri))} 0 ${large} ${f.sweep(0)} ${at(ri, from)}Z`;
}

/** A ring about the anchor, an outer circle with a hole. */
function ringKitA(f: Local, r: number, width: number): string {
  return f.circle(0, 0, r) + f.circle(0, 0, r - width);
}

/** A flame outline `k` times the full one, base at (0, `base`) of the part's frame, tip leaning by `lean`. */
function flameKitA(f: Local, k: number, base: number, lean: number): string {
  const p = (x: number, y: number): string => f.p(x * k + lean * k * (y / 6) ** 2, base + y * k);
  return `M${p(-2.4, -0.8)}C${p(-3.2, -3)} ${p(-0.8, -4)} ${p(0.4, -6)}C${p(0.6, -4)} ${p(3, -3.2)} ${p(2.4, -0.8)}C${p(2, 0.6)} ${p(-2, 0.6)} ${p(-2.4, -0.8)}Z`;
}

/** A teardrop, tip up, centred on a point, `r` its round's radius. */
function dropKitA(f: Local, cx: number, cy: number, r: number): string {
  return (
    `M${f.p(cx, cy - 2 * r)}Q${f.p(cx + 0.9 * r, cy - 0.9 * r)} ${f.p(cx + r, cy + 0.4 * r)}` +
    `A${fmt(f.r(r))} ${fmt(f.r(r))} 0 0 ${f.sweep(1)} ${f.p(cx - r, cy + 0.4 * r)}Q${f.p(cx - 0.9 * r, cy - 0.9 * r)} ${f.p(cx, cy - 2 * r)}Z`
  );
}

/** A huge cresting wave rising from its base at the anchor and curling toward the side it faces, 50 wide and 34 tall at scale 1. */
function wavePart(at: PartPlace, rng: Rng): KitShape[] {
  const f = local(at);
  const h = 34 * between(rng, 0.94, 1.04);
  const lip = between(rng, 21, 25);
  const body =
    `M${f.p(-25, 0)}C${f.p(-22, -h * 0.5)} ${f.p(-12, -h)} ${f.p(5, -h)}C${f.p(15, -h)} ${f.p(lip + 1, -h * 0.82)} ${f.p(lip, -h * 0.58)}` +
    `C${f.p(17, -h * 0.72)} ${f.p(9, -h * 0.62)} ${f.p(8, -h * 0.4)}C${f.p(7, -h * 0.2)} ${f.p(14, -4)} ${f.p(25, 0)}Z`;
  let spray = "";
  for (let i = 0; i < 5; i += 1) spray += f.circle(lip + 3 + (i % 3) * 3 + between(rng, -0.6, 0.6), -h * 0.9 + i * 3.6 + between(rng, -0.8, 0.8), between(rng, 0.5, 1.1));
  return [
    shape(body, "sky-400", 0.85, MAIN),
    shape(f.band([[lip - 2, -h * 0.6], [16, -h * 0.66], [11.4, -h * 0.55], [9.8, -h * 0.4], [10.6, -h * 0.25], [14, -h * 0.13], [19, -4]], 3), "blue-600", 0.85),
    shape(f.band([[-6, -h + 0.8], [2, -h + 0.5], [9, -h + 0.5], [16, -h * 0.92], [lip - 0.5, -h * 0.72]], 2.4), "white", 0.95),
    shape(f.band([[-24, -0.6], [-12, -1.4], [0, -0.6], [12, -1.4], [24, -0.6]], 1.6), "white", 0.6),
    shape(spray, "white", 0.9),
  ];
}

/** Droplets within 8 of the anchor, white and pale blue. */
function sprayPart(at: PartPlace, rng: Rng): KitShape[] {
  const f = local(at);
  const n = 6;
  let white = "";
  let blue = "";
  const start = between(rng, 0, 360);
  for (let k = 0; k < n; k += 1) {
    const a = (start + (k * 360) / n) * DEGREES;
    const radius = between(rng, 2.6, 7);
    const d = f.circle(radius * Math.cos(a), radius * Math.sin(a), between(rng, 0.5, 1));
    if (k % 2 === 0) white += d;
    else blue += d;
  }
  return [shape(white, "white", 0.95, MAIN), shape(blue, "sky-300", 0.9)];
}

/** Confetti within 20 by 14 of the anchor: small tilted rectangles, one shape per colour. */
function confettiPart(at: PartPlace, rng: Rng): KitShape[] {
  const f = local(at);
  const colours: readonly PaletteName[] = ["red-500", "sky-400", "lime-400", "violet-500", "white"];
  const paths = ["", "", "", "", ""];
  for (let r = 0; r < 3; r += 1) {
    for (let c = 0; c < 5; c += 1) {
      const index = (c + r * 2) % 5;
      const cx = -8 + c * 4 + between(rng, -0.6, 0.6);
      const cy = -4.7 + r * 4.7 + between(rng, -0.6, 0.6);
      const tilt = between(rng, 0, 180);
      const corners = ([[-0.8, -0.4], [0.8, -0.4], [0.8, 0.4], [-0.8, 0.4]] as const).map((corner) => {
        const [x, y] = turned(corner, tilt);
        return [cx + x, cy + y] as const;
      });
      paths[index] = (paths[index] ?? "") + f.poly(corners);
    }
  }
  return colours.map((colour, i) => shape(paths[i] ?? "", colour, 1, i === 0 ? MAIN : undefined));
}

/** Five to eight bubbles in a column 6 wide and 10 tall about the anchor: rings and glints. */
function bubblesPart(at: PartPlace, rng: Rng): KitShape[] {
  const f = local(at);
  const n = whole(rng, 5, 8);
  const slot = 10 / n;
  let rings = "";
  let fills = "";
  let glints = "";
  for (let i = 0; i < n; i += 1) {
    const r = slot * between(rng, 0.32, 0.42);
    const cx = between(rng, -(3 - r), 3 - r);
    const cy = 5 - (i + 0.5) * slot;
    rings += f.circle(cx, cy, r) + f.circle(cx, cy, r * 0.72);
    fills += f.circle(cx, cy, r * 0.72);
    glints += f.circle(cx - r * 0.3, cy - r * 0.32, r * 0.17);
  }
  return [shape(fills, "cyan-400", 0.14), shape(rings, "cyan-400", 0.95, MAIN), shape(glints, "white", 0.9)];
}

/** Spore dots within 10 of the anchor, each with a faint halo. */
function sporesPart(at: PartPlace, rng: Rng): KitShape[] {
  const f = local(at);
  const n = whole(rng, 6, 7);
  const start = between(rng, 0, 360);
  let dots = f.circle(0, 0, 0.9);
  let halos = f.circle(0, 0, 2.1);
  for (let k = 0; k < n; k += 1) {
    const a = (start + (k * 360) / n) * DEGREES;
    const radius = between(rng, 5.4, 8);
    const r = between(rng, 0.5, 0.8);
    dots += f.circle(radius * Math.cos(a), radius * Math.sin(a), r);
    halos += f.circle(radius * Math.cos(a), radius * Math.sin(a), r * 2.3);
  }
  return [shape(halos, "lime-400", 0.2), shape(dots, "green-400", 0.95, MAIN)];
}

/** A rainbow arc of radius 30 about the anchor (its ends on the anchor's line): five translucent bands. */
function rainbowPart(at: PartPlace, rng: Rng): KitShape[] {
  const f = local(at);
  const colours: readonly PaletteName[] = ["red-500", "orange-400", "lime-400", "sky-400", "violet-500"];
  const r = 30 * between(rng, 0.96, 1.04);
  return colours.map((colour, i) => {
    const ro = r - i * 2.4;
    const d = `M${f.p(-ro, 0)}A${fmt(f.r(ro))} ${fmt(f.r(ro))} 0 0 ${f.sweep(1)} ${f.p(ro, 0)}L${f.p(ro - 2.4, 0)}A${fmt(f.r(ro - 2.4))} ${fmt(f.r(ro - 2.4))} 0 0 ${f.sweep(0)} ${f.p(-(ro - 2.4), 0)}Z`;
    return shape(d, colour, 0.6, i === 0 ? MAIN : undefined);
  });
}

/** A shockwave about the anchor: translucent rings out to radius 14 and short radial streaks. */
function shockwavePart(at: PartPlace, rng: Rng): KitShape[] {
  const f = local(at);
  const r = 14 * between(rng, 0.92, 1);
  let streaks = "";
  const start = between(rng, 0, 45);
  for (let k = 0; k < 8; k += 1) {
    const a = (start + k * 45) * DEGREES;
    const from = r * 0.35;
    const to = r * between(rng, 0.6, 0.78);
    streaks += f.band([[from * Math.cos(a), from * Math.sin(a)], [to * Math.cos(a), to * Math.sin(a)]], 0.8);
  }
  return [
    shape(ringKitA(f, r, 1.3), "white", 0.6, MAIN),
    shape(ringKitA(f, r * 0.68, 1.7), "sky-300", 0.5),
    shape(ringKitA(f, r * 0.36, 1.2), "white", 0.4),
    shape(streaks, "white", 0.55),
  ];
}

/** Echo rings: three arcs either side of the anchor at radii 6, 10 and 14, fading outward. */
function echoRingsPart(at: PartPlace, rng: Rng): KitShape[] {
  const f = local(at);
  const spread = between(rng, 36, 44);
  const pair = (r: number, width: number): string => arcKitA(f, r, width, -spread, spread) + arcKitA(f, r, width, 180 - spread, 180 + spread);
  return [
    shape(pair(6, 1.4), "sky-300", 0.85, MAIN),
    shape(pair(10, 1.3), "blue-600", 0.6),
    shape(pair(14, 1.2), "sky-300", 0.35),
  ];
}

/** A crackling zigzag bolt from the anchor 30 toward the side it faces, with a glow, a white core and a branch. */
function zapPart(at: PartPlace, rng: Rng): KitShape[] {
  const f = local(at);
  const bolt: Point[] = [[0, 0]];
  for (let k = 1; k <= 6; k += 1) bolt.push([k * 5, (k % 2 === 0 ? -1 : 1) * between(rng, 1.8, 3.4)]);
  const [bx, by] = bolt[3] ?? [15, 0];
  const branch: Point[] = [[bx, by], [bx + between(rng, 2, 3.5), by - 3], [bx + between(rng, 5, 7), by - 7]];
  return [
    shape(f.band(bolt, 3.6), "sky-300", 0.3),
    shape(f.band(bolt, 1.1), "white", 0.95, MAIN),
    shape(f.band(branch, 0.6), "white", 0.8),
    shape(f.circle(0, 0, 2), "sky-300", 0.4),
  ];
}

/** A healing glow of radius 8 about the anchor with three or four plus-shaped sparkles. */
function healGlowPart(at: PartPlace, rng: Rng): KitShape[] {
  const f = local(at);
  let pluses = "";
  const n = whole(rng, 3, 4);
  const start = between(rng, 0, 90);
  for (let k = 0; k < n; k += 1) {
    const a = (start + (k * 360) / n) * DEGREES;
    const radius = between(rng, 3.2, 5.4);
    pluses += plusKitA(f, radius * Math.cos(a), radius * Math.sin(a), between(rng, 1.2, 1.7));
  }
  return [
    shape(f.circle(0, 0, 8), "green-400", 0.14, MAIN),
    shape(f.circle(0, 0, 5.6), "green-400", 0.2, MAIN),
    shape(f.circle(0, 0, 3.2), "green-400", 0.3, MAIN),
    shape(pluses, "green-200", 0.95),
  ];
}

/** A flame standing on the anchor, 6 tall at scale 1: outer, inner and a white core. */
function flamePart(at: PartPlace, rng: Rng): KitShape[] {
  const f = local(at);
  const lean = between(rng, -0.8, 0.8);
  return [
    shape(flameKitA(f, 1, 0, lean), "orange-400", 1, MAIN),
    shape(flameKitA(f, 0.66, 0, lean * 0.7), "ember-500", 1),
    shape(flameKitA(f, 0.3, 0, lean * 0.4), "white", 0.95),
  ];
}

/** Four to six falling teardrops within 8 of the anchor. */
function dropsPart(at: PartPlace, rng: Rng): KitShape[] {
  const f = local(at);
  const n = whole(rng, 4, 6);
  let drops = "";
  let glints = "";
  for (let i = 0; i < n; i += 1) {
    const r = between(rng, 0.85, 1.15);
    const cx = (i % 2 === 0 ? -1 : 1) * between(rng, 1.5, 5);
    const cy = -6 + (i + 0.5) * (12 / n);
    drops += dropKitA(f, cx, cy, r);
    glints += f.circle(cx - r * 0.35, cy + r * 0.1, r * 0.2);
  }
  return [shape(drops, "blue-600", 1, MAIN), shape(glints, "white", 0.6)];
}

/** Four to six horizontal motion streaks trailing 24 long from the anchor, away from the side it faces. */
function streaksPart(at: PartPlace, rng: Rng): KitShape[] {
  const f = local(at);
  const n = whole(rng, 4, 6);
  let streaks = "";
  let bright = "";
  for (let i = 0; i < n; i += 1) {
    const y = (i - (n - 1) / 2) * 2.2;
    const length = i === Math.floor(n / 2) ? 24 : between(rng, 12, 22);
    const d = f.poly([[-length, y], [0, y - 0.45], [0, y + 0.45]]);
    streaks += d;
    if (i === Math.floor(n / 2)) bright += f.poly([[-length * 0.6, y], [0, y - 0.25], [0, y + 0.25]]);
  }
  return [shape(streaks, "white", 0.35, MAIN), shape(bright, "white", 0.6)];
}

/** A dust cloud about 12 wide about the anchor: translucent blobs that never overlap. */
function dustPart(at: PartPlace, rng: Rng): KitShape[] {
  const f = local(at);
  let low = "";
  for (const x of [-3.8, 0, 3.8]) low += blob(f, rng, x, 0, 1.5, 6);
  let high = "";
  for (const x of [-1.9, 1.9]) high += blob(f, rng, x, -3.2, 1.5, 6);
  return [shape(low, "brass-500", 0.5, MAIN), shape(high, "stone-500", 0.45)];
}

/** Two or three rising S-curved steam wisps above the anchor, 10 tall at scale 1. */
function steamPart(at: PartPlace, rng: Rng): KitShape[] {
  const f = local(at);
  const n = whole(rng, 2, 3);
  let wisps = "";
  let haze = "";
  for (let i = 0; i < n; i += 1) {
    const ox = (i - (n - 1) / 2) * 3.2;
    const phase = between(rng, -0.4, 0.4);
    const line: Point[] = [];
    for (let k = 0; k <= 8; k += 1) {
      const t = k / 8;
      line.push([ox + 1.4 * Math.sin(t * Math.PI * 2 + phase), -t * 10]);
    }
    wisps += f.band(line, 0.9);
    haze += f.band(line, 2);
  }
  return [shape(wisps, "white", 0.3, MAIN), shape(haze, "white", 0.1)];
}

/** Billowing smoke 16 wide about the anchor: pale lobes with dark undersides. */
function smokePart(at: PartPlace, rng: Rng): KitShape[] {
  const f = local(at);
  return [
    shape(blob(f, rng, -4.7, -2, 3.4, 8), "slate-400", 0.55, MAIN),
    shape(blob(f, rng, 4.7, -2.5, 3.6, 8), "slate-400", 0.55, MAIN),
    shape(blob(f, rng, -0.5, -7, 3.2, 8), "slate-400", 0.55, MAIN),
    shape(blob(f, rng, -4.2, 0.6, 2.9, 7) + blob(f, rng, 4.2, 0.4, 3.1, 7) + blob(f, rng, 0, -9.6, 2.4, 6), "slate-700", 0.3),
  ];
}

/** Five to seven glass shards within 10 of the anchor, pale blue and white. */
function shardsPart(at: PartPlace, rng: Rng): KitShape[] {
  const f = local(at);
  const n = whole(rng, 5, 7);
  const start = between(rng, 0, 360);
  const shard = (cx: number, cy: number, reach: number): string => {
    const turn = between(rng, 0, 360);
    const tri = ([[reach, 0], [-reach * 0.55, reach * 0.4], [-reach * 0.3, -reach * 0.35]] as const).map((p) => {
      const [x, y] = turned(p, turn);
      return [cx + x, cy + y] as const;
    });
    return f.poly(tri);
  };
  let pale = shard(0, 0, 2.7);
  let white = "";
  for (let k = 0; k < n; k += 1) {
    const a = (start + (k * 360) / n) * DEGREES;
    const radius = between(rng, 5.6, 8);
    const d = shard(radius * Math.cos(a), radius * Math.sin(a), between(rng, 1.3, 2));
    if (k % 2 === 0) pale += d;
    else white += d;
  }
  return [shape(pale, "sky-300", 0.9, MAIN), shape(white, "white", 0.9)];
}

/** A spiral of six ever-larger square outlines unwinding from the anchor toward the side it faces, the largest about 30 across. */
function squaresPart(at: PartPlace, rng: Rng): KitShape[] {
  const f = local(at);
  const sides = [3, 7, 12, 17, 23, 28];
  const turn = between(rng, 12, 17);
  return sides.map((side, k) => {
    const cx = k * 2.6;
    const cy = 0;
    const angle = 45 + k * turn;
    const outer = polygonPath(f.x(cx), f.y(cy), 4, f.r((side / 2) * Math.SQRT2), angle);
    const inner = polygonPath(f.x(cx), f.y(cy), 4, f.r((side / 2 - 0.5) * Math.SQRT2), angle);
    return shape(outer + inner, "white", 0.5 + 0.08 * k, k === sides.length - 1 ? MAIN : undefined);
  });
}

/** A starburst of radius 10 about the anchor: radiating spikes and a bright core. */
function burstPart(at: PartPlace, rng: Rng): KitShape[] {
  const f = local(at);
  const star = (spikes: number, outer: number, inner: number, turn: number): string => {
    const points: Point[] = [];
    for (let i = 0; i < spikes * 2; i += 1) {
      const a = (turn + (i * 180) / spikes) * DEGREES;
      const reach = i % 2 === 0 ? outer * between(rng, 0.85, 1) : inner;
      points.push([reach * Math.cos(a), reach * Math.sin(a)]);
    }
    return f.poly(points);
  };
  return [
    shape(star(12, 10, 3.2, between(rng, 0, 30)), "white", 0.6, MAIN),
    shape(star(8, 6, 2.4, between(rng, 0, 45)), "white", 0.5),
    shape(f.circle(0, 0, 2.2), "white", 0.95),
  ];
}

/** Three or four curling wind lines about 22 wide about the anchor. */
function gustPart(at: PartPlace, rng: Rng): KitShape[] {
  const f = local(at);
  const n = whole(rng, 3, 4);
  let lines = "";
  for (let i = 0; i < n; i += 1) {
    const y0 = (i - (n - 1) / 2) * 4.6;
    const x0 = between(rng, -11, -8);
    const cx = between(rng, 3, 7);
    const r = 1.8;
    const line: Point[] = [];
    for (let k = 0; k <= 5; k += 1) {
      const x = x0 + ((cx - x0) * k) / 5;
      line.push([x, y0 + 0.6 * Math.sin(k + i)]);
    }
    for (let k = 1; k <= 6; k += 1) {
      const t = k / 6;
      const a = (90 - 270 * t) * DEGREES;
      const rr = r * (1 - 0.45 * t);
      line.push([cx + rr * Math.cos(a), y0 - r + rr * Math.sin(a)]);
    }
    lines += f.band(line, 0.8);
  }
  return [shape(lines, "white", 0.35, MAIN)];
}

/** Five to seven leaves blown within 14 of the anchor: green, orange and brass. */
function leavesPart(at: PartPlace, rng: Rng): KitShape[] {
  const f = local(at);
  const n = whole(rng, 5, 7);
  const start = between(rng, 0, 360);
  const paths = ["", "", ""];
  for (let k = 0; k < n; k += 1) {
    const a = (start + (k * 360) / n) * DEGREES;
    const radius = between(rng, 6, 11);
    const cx = radius * Math.cos(a);
    const cy = radius * Math.sin(a);
    const turn = between(rng, 0, 180);
    const [ax, ay] = turned([-2, 0], turn);
    const [bx, by] = turned([2, 0], turn);
    const [ux, uy] = turned([0, -1.5], turn);
    const [lx, ly] = turned([0, 1.5], turn);
    const d = `M${f.p(cx + ax, cy + ay)}Q${f.p(cx + ux * 2, cy + uy * 2)} ${f.p(cx + bx, cy + by)}Q${f.p(cx + lx * 2, cy + ly * 2)} ${f.p(cx + ax, cy + ay)}Z`;
    paths[k % 3] = (paths[k % 3] ?? "") + d;
  }
  return [shape(paths[0] ?? "", "green-500", 1, MAIN), shape(paths[1] ?? "", "orange-400", 1), shape(paths[2] ?? "", "brass-600", 1)];
}

/** A lumpy round puff cloud of radius 10 about the anchor with a lighter top lobe. */
function puffPart(at: PartPlace, rng: Rng): KitShape[] {
  const f = local(at);
  return [
    shape(blob(f, rng, 0, 0, 9, 9), "red-300", 1, MAIN),
    shape(blob(f, rng, 0, 3.5, 6, 7), "red-400", 0.3),
    shape(blob(f, rng, -2.5, -3.8, 4.6, 7), "white", 0.35),
  ];
}

/** A bright swirling vortex of radius 14 about the anchor: four tapering arms and a bright core. */
function vortexPart(at: PartPlace, rng: Rng): KitShape[] {
  const f = local(at);
  const start = between(rng, 0, 90);
  const sweep = between(rng, 200, 240);
  const arm = (k: number): string => spiralArm(f, start + k * 90, sweep, 3, 14, 3);
  return [
    shape(arm(0) + arm(2), "white", 0.9, MAIN),
    shape(arm(1) + arm(3), "sky-300", 0.85),
    shape(f.circle(0, 0, 4), "sky-300", 0.3),
    shape(f.circle(0, 0, 2), "white", 0.95),
  ];
}

/** The sky folding along a glowing vertical seam through the anchor, spanning the box's height, with dark flaps at its edges. */
function foldSeamPart(at: PartPlace, rng: Rng): KitShape[] {
  const f = local(at);
  const x = f.x(0);
  const wobble = between(rng, -1.5, 1.5);
  const line: Point[] = [[x, EDGE], [x + wobble, 30], [x - wobble, 65], [x, FAR_EDGE]];
  const flap = 17 * at.s;
  return [
    shape(bandPath(line, 7 * at.s), "purple-400", 0.2),
    shape(bandPath(line, 3.2 * at.s), "purple-400", 0.28),
    shape(bandPath(line, 0.9 * at.s), "violet-300", 1, MAIN),
    shape(`M${pt(EDGE, EDGE)}L${pt(EDGE + flap, SCENE_BOX / 2)}L${pt(EDGE, FAR_EDGE)}Z` + `M${pt(FAR_EDGE, EDGE)}L${pt(FAR_EDGE - flap, SCENE_BOX / 2)}L${pt(FAR_EDGE, FAR_EDGE)}Z`, "slate-900", 0.4),
  ];
}

/** A glowing bell curve over the anchor, feet 20 either side and its top 22 above. */
function bellCurvePart(at: PartPlace, rng: Rng): KitShape[] {
  const f = local(at);
  const top = 22 * between(rng, 0.95, 1.05);
  const curve: Point[] = [-20, -14, -10, -7, -4.5, -2.2, 0, 2.2, 4.5, 7, 10, 14, 20].map((x) => [x, -top * Math.exp(-(x * x) / (2 * 7 * 7))] as const);
  return [
    shape(f.band(curve, 4.5), "cyan-400", 0.28),
    shape(f.band(curve, 1.2), "white", 0.95, MAIN),
    shape(f.band([[-22, 0], [22, 0]], 0.5), "cyan-400", 0.45),
  ];
}

/** Five to seven thin taut puppet strings from the top of the box down to points spread across 50 about the anchor. */
function stringsPart(at: PartPlace, rng: Rng): KitShape[] {
  const f = local(at);
  const n = whole(rng, 5, 7);
  let lines = "";
  let knots = "";
  for (let i = 0; i < n; i += 1) {
    const dx = -25 + (50 * (i + 0.5)) / n + between(rng, -1.5, 1.5);
    const dy = between(rng, -3, 3);
    lines += bandPath([[f.x(dx * 0.35), EDGE], [f.x(dx), f.y(dy)]], 0.35 * at.s);
    knots += f.circle(dx, dy, 0.7);
  }
  return [shape(lines, "slate-100", 0.7, MAIN), shape(knots, "slate-100", 0.9)];
}

/** A ring of ten to twelve small tilted cards whirling on an ellipse 30 by 10 about the anchor, the near half bigger. */
function cardRingPart(at: PartPlace, rng: Rng): KitShape[] {
  const f = local(at);
  const n = whole(rng, 10, 12);
  const start = between(rng, 0, 360);
  let bodies = "";
  let borders = "";
  let glints = "";
  for (let k = 0; k < n; k += 1) {
    const theta = (start + (k * 360) / n) * DEGREES;
    const cx = 15 * Math.cos(theta);
    const cy = 5 * Math.sin(theta);
    const size = 0.65 + 0.35 * ((Math.sin(theta) + 1) / 2);
    const tangent = (Math.atan2(5 * Math.cos(theta), -15 * Math.sin(theta)) * 180) / Math.PI;
    const quad = (hw: number, hh: number): Point[] =>
      ([[-hw, -hh], [hw, -hh], [hw, hh], [-hw, hh]] as const).map((corner) => {
        const [x, y] = turned(corner, tangent);
        return [cx + x, cy + y] as const;
      });
    const hw = 1.1 * size;
    const hh = 1.7 * size;
    bodies += f.poly(quad(hw, hh));
    borders += f.poly(quad(hw, hh)) + f.poly(quad(hw - 0.35 * size, hh - 0.35 * size));
    const g = quad(hw * 0.6, hh * 0.6).slice(0, 3).map(([x, y]) => [x * 0.5 + cx * 0.5, y * 0.5 + cy * 0.5] as const);
    glints += f.poly(g);
  }
  return [shape(bodies, "blue-800", 1, MAIN), shape(borders, "white", 0.95), shape(glints, "white", 0.5)];
}

/** One Mythic's own: a spinning cyclone funnel standing on the anchor, 40 tall, 32 wide at its top narrowing to 4, with sand streams at its foot. */
function cyclonePart(at: PartPlace, rng: Rng): KitShape[] {
  const f = local(at);
  const lean = between(rng, -0.12, 0.12);
  const hw = (y: number): number => 2 + 14 * (-y / 40) ** 1.4;
  const P = (x: number, y: number): Point => leant([x, y], lean);
  const levels = [-40, -33, -26, -19, -12, -6, 0];
  const left = levels.map((y) => P(-hw(y), y));
  const right = levels.map((y) => P(hw(y), y)).reverse();
  const swirl = (yc: number, thick: number): string => {
    const yl = yc;
    const yr = yc - 2.4;
    const a = P(-hw(yl), yl);
    const b = P(hw(yr), yr);
    const c = P(hw(yr + thick), yr + thick);
    const d = P(-hw(yl + thick), yl + thick);
    const m = P(0, yl + 3.6);
    const m2 = P(0, yl + thick + 3.6);
    return `M${f.p(...a)}Q${f.p(...m)} ${f.p(...b)}L${f.p(...c)}Q${f.p(...m2)} ${f.p(...d)}Z`;
  };
  const rim = P(0, -40);
  return [
    shape(f.poly([...left, ...right]), "sky-300", 0.55, MAIN),
    shape(f.ellipse(rim[0], -40, 16, 3), "slate-100", 0.45),
    shape(swirl(-34, 2.6) + swirl(-20, 2.2), "slate-100", 0.7),
    shape(swirl(-28, 2.4) + swirl(-14, 1.8) + swirl(-7, 1.2), "white", 0.5),
    shape(f.band([[-0.5, -38], [-0.6, -20], [-0.2, -4]], 0.7), "slate-100", 0.3),
    shape(f.ellipse(0, 0.5, 9, 1.6), "brass-500", 0.4),
    shape(f.band([[-2, -3], [-7, -1.2], [-13, 0]], 1) + f.band([[2, -3], [7, -1.2], [13, 0]], 1) + f.band([[-1.5, -6], [-8, -4], [-11, -2.6]], 0.7) + f.band([[1.5, -6], [8, -4], [12, -2.8]], 0.7), "brass-500", 0.8),
  ];
}

/** The moon's shadow of radius 6 about the anchor: a dark disc and a bright corona a little larger, meant to lie over the moon. */
function eclipsePart(at: PartPlace, rng: Rng): KitShape[] {
  const f = local(at);
  const wobble = between(rng, -0.2, 0.2);
  return [
    shape(f.circle(0, 0, 9.4 + wobble) + f.circle(0, 0, 7.4), "white", 0.14),
    shape(f.circle(0, 0, 7.4) + f.circle(0, 0, 5.8), "white", 0.4),
    shape(f.circle(0, 0, 6), "slate-900", 0.85, MAIN),
  ];
}

/** A bright sun of radius 5 about the anchor, in glow rings. */
function sunPart(at: PartPlace, rng: Rng): KitShape[] {
  const f = local(at);
  const r = 5 * between(rng, 0.97, 1.03);
  return [
    shape(f.circle(0, 0, r * 1.9), "slate-100", 0.14),
    shape(f.circle(0, 0, r * 1.45), "white", 0.2),
    shape(f.circle(0, 0, r * 1.18), "white", 0.3),
    shape(f.circle(0, 0, r), "white", 1, MAIN),
  ];
}
// @@ A weather kit
  wave: { category: "weather", draw: wavePart },
  spray: { category: "weather", draw: sprayPart },
  confetti: { category: "weather", draw: confettiPart },
  bubbles: { category: "weather", draw: bubblesPart },
  spores: { category: "weather", draw: sporesPart },
  rainbow: { category: "weather", draw: rainbowPart },
  shockwave: { category: "weather", draw: shockwavePart },
  "echo-rings": { category: "weather", draw: echoRingsPart },
  zap: { category: "weather", draw: zapPart },
  "heal-glow": { category: "weather", draw: healGlowPart },
  flame: { category: "weather", draw: flamePart },
  drops: { category: "weather", draw: dropsPart },
  streaks: { category: "weather", draw: streaksPart },
  dust: { category: "weather", draw: dustPart },
  steam: { category: "weather", draw: steamPart },
  smoke: { category: "weather", draw: smokePart },
  shards: { category: "weather", draw: shardsPart },
  squares: { category: "weather", draw: squaresPart },
  burst: { category: "weather", draw: burstPart },
  gust: { category: "weather", draw: gustPart },
  leaves: { category: "weather", draw: leavesPart },
  puff: { category: "weather", draw: puffPart },
  vortex: { category: "weather", draw: vortexPart },
  "fold-seam": { category: "weather", draw: foldSeamPart },
  "bell-curve": { category: "weather", draw: bellCurvePart },
  strings: { category: "weather", draw: stringsPart },
  "card-ring": { category: "weather", draw: cardRingPart },
  cyclone: { category: "weather", draw: cyclonePart },
  eclipse: { category: "weather", draw: eclipsePart },
  sun: { category: "weather", draw: sunPart },
