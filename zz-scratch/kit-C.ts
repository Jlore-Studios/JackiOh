// @@ C creatures

/** A seagull, 5 wide, centred on the anchor: in flight with wings out, or perched standing on it (3 tall at scale 1). */
function gullPart(at: PartPlace, rng: Rng): KitShape[] {
  const f = local(at);
  if (rng() < 0.5) {
    const flap = between(rng, -0.8, 0.8);
    const wing = (side: number): string =>
      `M${f.p(side * 0.2, -0.2)}Q${f.p(side * 1.2, -1.8 + flap)} ${f.p(side * 2.5, -1.5 + flap)}Q${f.p(side * 1.5, -0.5)} ${f.p(side * 0.3, 0.4)}Z`;
    return [
      shape(wing(-1), "slate-400", 1),
      shape(wing(1), "slate-400", 0.9),
      shape(f.ellipse(0, 0.1, 1.2, 0.5), "white", 1, MAIN),
      shape(f.poly([[-1, 0], [-1.9, 0.2], [-1, 0.5]]), "white", 1, MAIN),
      shape(f.circle(1.3, -0.2, 0.38), "white", 1, MAIN),
      shape(f.poly([[1.6, -0.3], [2.3, -0.1], [1.6, 0.05]]), "orange-400", 1),
      shape(f.circle(1.4, -0.28, 0.08), "slate-900", 1),
    ];
  }
  return [
    shape(f.band([[0, -1], [0, 0]], 0.14) + f.band([[0.5, -1], [0.5, 0]], 0.14), "orange-400", 1),
    shape(f.poly([[-1.1, -1.7], [-2.5, -1.1], [-1, -1.1]]), "white", 1, MAIN),
    shape(f.ellipse(0, -1.6, 1.3, 0.75), "white", 1, MAIN),
    shape(f.ellipse(0.8, -2.1, 0.5, 0.8), "white", 1, MAIN),
    shape(f.circle(1.1, -2.7, 0.45), "white", 1, MAIN),
    shape(`M${f.p(-1.1, -1.9)}Q${f.p(-0.2, -2.4)} ${f.p(0.8, -1.8)}Q${f.p(-0.4, -1.2)} ${f.p(-2.2, -1.1)}Z`, "slate-400", 1),
    shape(f.poly([[1.5, -2.8], [2.3, -2.55], [1.5, -2.4]]), "orange-400", 1),
    shape(f.circle(1.25, -2.8, 0.09), "slate-900", 1),
  ];
}

/** A small house cat sitting on the anchor, facing the side the part faces, 6 tall at scale 1. */
function catPart(at: PartPlace, rng: Rng): KitShape[] {
  const f = local(at);
  const curl = between(rng, -0.4, 0.4);
  return [
    shape(f.band([[-2.3, -0.4], [-3.4, -0.6], [-3.6 + curl, -2], [-2.8 + curl, -2.8]], 0.7), "orange-400", 1, MAIN),
    shape(`M${f.p(-2.4, 0)}Q${f.p(-2.6, -2.8)} ${f.p(-0.6, -3.4)}Q${f.p(1.8, -3.6)} ${f.p(2, -1.5)}Q${f.p(2.1, -0.4)} ${f.p(1.8, 0)}Z`, "orange-400", 1, MAIN_INKED),
    shape(f.circle(1.1, -4.3, 1.25), "orange-400", 1, MAIN_INKED),
    shape(f.poly([[0.1, -5.2], [0.3, -6.1], [1.1, -5.5]]) + f.poly([[1.4, -5.5], [2.2, -6.1], [2.4, -5.1]]), "orange-400", 1, MAIN),
    shape(f.ellipse(1.4, -1.8, 0.7, 1.3), "white", 0.95),
    shape(f.ellipse(1.9, -3.95, 0.65, 0.5), "white", 0.95),
    shape(f.ellipse(1.6, -0.15, 0.6, 0.25), "white", 0.95),
    shape(f.circle(1.0, -4.6, 0.26) + f.circle(1.85, -4.6, 0.26), "lime-400", 1),
    shape(f.ellipse(1.0, -4.6, 0.08, 0.2) + f.ellipse(1.85, -4.6, 0.08, 0.2), "slate-900", 1),
    shape(f.circle(2.35, -4.1, 0.15), "red-300", 1),
  ];
}

/** A fluffy, slightly puzzled sheep standing on the anchor, facing the side the part faces, 14 wide and 11 tall at scale 1. */
function sheepPart(at: PartPlace, rng: Rng): KitShape[] {
  const f = local(at);
  return [
    shape(
      [-3.8, -2.2, 2.6, 4.2].map((x) => f.band([[x, -3.6], [x, 0]], 1.1)).join(""),
      "black",
      1,
    ),
    shape(blob(f, rng, -0.5, -6, 4.4, 8), "white", 1, MAIN),
    shape(blob(f, rng, -3.2, -8.2, 3, 7), "white", 1, MAIN),
    shape(blob(f, rng, 1.8, -8.6, 3.2, 7), "white", 1, MAIN),
    shape(f.ellipse(-0.5, -3.5, 4.4, 0.9), "slate-400", 0.35),
    shape(f.ellipse(6, -6.3, 1.9, 2.3), "black", 1),
    shape(f.poly([[5.2, -7.8], [3.6, -9.4], [4.4, -7]]) + f.poly([[5, -6.4], [2.8, -5.8], [4.6, -5.3]]), "black", 1),
    shape(f.circle(5.4, -7, 0.55) + f.circle(7, -7, 0.55), "white", 1),
    shape(f.circle(5.3, -7.3, 0.25) + f.circle(6.9, -7.3, 0.25), "slate-900", 1),
    shape(f.band([[4.7, -8.1], [5.9, -8.4]], 0.3) + f.band([[6.4, -7.9], [7.5, -8.8]], 0.3), "slate-700", 1),
    shape(blob(f, rng, 5.8, -8.9, 1.1, 6), "white", 1, MAIN),
  ];
}

/** A bee, 3 wide, centred on the anchor, facing the side the part faces. */
function beePart(at: PartPlace, rng: Rng): KitShape[] {
  const f = local(at);
  const flap = between(rng, -0.2, 0.2);
  return [
    shape(f.ellipse(-0.5, -1.1 + flap, 0.9, 0.4), "white", 0.6),
    shape(f.ellipse(0.5, -1.3 + flap, 0.8, 0.35), "white", 0.5),
    shape(f.ellipse(0, 0, 1.1, 0.8), "brass-500", 1, MAIN),
    shape(f.band([[-0.2, -0.65], [-0.2, 0.65]], 0.32) + f.band([[0.55, -0.55], [0.55, 0.55]], 0.3), "black", 1),
    shape(f.circle(1.25, 0.05, 0.45), "black", 1),
    shape(f.poly([[-1, -0.15], [-1.7, 0], [-1, 0.15]]), "black", 1),
  ];
}

/** A jewel-shelled scarab beetle, side-on and facing the side the part faces, standing on the anchor, 18 wide and 11 tall at scale 1. */
function scarabPart(at: PartPlace, rng: Rng): KitShape[] {
  const f = local(at);
  const feel = between(rng, -0.4, 0.4);
  const leg = (x: number, reach: number): string => f.band([[x, -3.4], [x + reach, -1.8], [x + reach * 1.5, 0]], 0.7);
  const far = (x: number): string => f.band([[x, -3.4], [x - 0.8, -1.6], [x - 1.4, 0]], 0.6);
  return [
    shape(far(-4) + far(0.5) + far(4.5), "slate-800", 1),
    shape(leg(-5, -1.2) + leg(-0.8, 0.2) + leg(3.4, 1.4), "slate-900", 1),
    shape(`M${f.p(-7, -3.5)}C${f.p(-7.4, -10)} ${f.p(4.6, -12.4)} ${f.p(6.6, -3.5)}Z`, "green-500", 1, MAIN_INKED),
    shape(f.poly([[-5.2, -4.4], [-4, -8], [-2, -4.4]]) + f.poly([[1.8, -4.4], [3, -8.2], [4.6, -4.4]]), "green-700", 0.6),
    shape(f.band([[-0.2, -9.6], [-0.6, -6.6], [-0.1, -3.6]], 0.4), "green-900", 0.8),
    shape(f.band([[-5.4, -6.6], [-3.6, -9.2], [-0.8, -10]], 0.8), "lime-200", 0.7),
    shape(f.ellipse(7.7, -4.6, 2, 1.7), "slate-900", 1),
    shape(f.poly([[9, -5.8], [10.8, -6.4], [9.6, -4.9]]) + f.poly([[9, -3.6], [10.8, -3], [9.6, -4.4]]), "slate-800", 1),
    shape(f.band([[8.4, -6], [10, -8.2 + feel], [11.6, -8.4 + feel]], 0.28) + f.band([[7.2, -6], [7.8, -8.6], [9.4, -9.6 + feel]], 0.28), "slate-900", 1),
  ];
}

/** A translucent green gelatin cube in three-quarter view standing on the anchor (front, top and right side), 30 wide and 30 tall at scale 1; its jagged-toothed mouth is on the front. */
function cubePart(at: PartPlace, rng: Rng): KitShape[] {
  const f = local(at);
  const bite = between(rng, -0.5, 0.5);
  const top = (c: number): string => f.poly([[c - 1.3, -14], [c, -11.5 + bite], [c + 1.3, -14]]);
  const bottom = (c: number): string => f.poly([[c - 1.3, -8.5], [c, -11 - bite], [c + 1.3, -8.5]]);
  return [
    shape(f.poly([[-14.5, 0], [8.5, 0], [8.5, -23], [-14.5, -23]]), "green-400", 0.45, MAIN),
    shape(f.poly([[-14.5, -23], [8.5, -23], [15.5, -30], [-7.5, -30]]), "lime-200", 0.5),
    shape(f.poly([[8.5, 0], [15.5, -7], [15.5, -30], [8.5, -23]]), "green-700", 0.5),
    shape(f.poly([[-10.5, -14.2], [4.5, -14.2], [5, -8.3], [-11, -8.3]]), "slate-900", 0.55),
    shape([-8.3, -5.1, -1.9, 1.3].map(top).join(""), "white", 0.95),
    shape([-6.7, -3.5, -0.3, 2.9].map(bottom).join(""), "white", 0.95),
    shape(f.band([[-14, -1], [-14, -22.6], [8, -22.6]], 0.8), "white", 0.5),
    shape(f.circle(-9, -5, 1.4) + f.circle(-4, -18, 1) + f.circle(3, -4.5, 0.8) + f.circle(5, -19, 1.2) + f.circle(11.8, -12, 0.7), "lime-200", 0.5),
  ];
}

/** A sleek black panther bounding through the air, centred on the anchor and facing the side the part faces, 32 long and 14 tall at scale 1. */
function pantherPart(at: PartPlace, rng: Rng): KitShape[] {
  const f = local(at);
  const arch = between(rng, -0.5, 0.5);
  return [
    shape(f.band([[-9, -0.5], [-13, -2.5 + arch], [-15.5, -1.5], [-16, -5.5 - arch]], 0.9), "slate-900", 1, MAIN),
    shape(f.band([[-5.5, 1], [-10, 1.5], [-13, 2.5]], 1.8) + f.band([[5.5, 0.5], [10, 0.5], [14.5, 1.5]], 1.8), "slate-800", 1),
    shape(f.band([[-6.5, 1.5], [-11, 4], [-14, 6]], 2) + f.band([[6, 1.5], [11, 3.5], [15, 5.5]], 2), "slate-900", 1, MAIN),
    shape(
      `M${f.p(-9, 0)}Q${f.p(-8, -3.5)} ${f.p(-2, -3.8)}Q${f.p(5, -4.2)} ${f.p(8, -2.5)}Q${f.p(9, 0.5)} ${f.p(5, 2.4)}Q${f.p(-1, 3)} ${f.p(-5, 2.6)}Q${f.p(-9, 2.2)} ${f.p(-9, 0)}Z`,
      "slate-900",
      1,
      MAIN_INKED,
    ),
    shape(`M${f.p(9, -3.5)}Q${f.p(11, -5.5)} ${f.p(13.5, -4.4)}Q${f.p(16, -3.5)} ${f.p(16.2, -1.8)}Q${f.p(15, -0.5)} ${f.p(12, -0.7)}Q${f.p(9.5, -1)} ${f.p(9, -3.5)}Z`, "slate-900", 1, MAIN),
    shape(f.poly([[9.8, -4.6], [9.6, -6.9], [11.6, -5.3]]) + f.poly([[12, -5.1], [13.2, -6.8], [13.8, -4.6]]), "slate-900", 1, MAIN),
    shape(f.band([[-7, -3], [-2, -3.2], [4, -3.4], [7.5, -2.4]], 0.9), "blue-800", 0.8),
    shape(f.circle(13.6, -3.2, 0.45), "green-400", 1),
    shape(f.circle(16, -2.1, 0.35), "slate-700", 1),
  ];
}

/** A great sapphire dragon mother coiled and facing the side the part faces, standing on the anchor, 40 wide and 34 tall at scale 1. Her two forearms are held forward: the near claws are at (18.5, -11.6) and the far claws at (18, -14.6), and a recipe places two hatchlings standing on the forearms, their feet at offsets (15, -16) and (19, -15.8). */
function dragonPart(at: PartPlace, rng: Rng): KitShape[] {
  const f = local(at);
  const taper = (points: readonly Point[], widths: readonly number[]): string => {
    const left: string[] = [];
    const right: string[] = [];
    points.forEach(([x, y], i) => {
      const [px, py] = points[Math.max(0, i - 1)] ?? [x, y];
      const [nx, ny] = points[Math.min(points.length - 1, i + 1)] ?? [x, y];
      const length = Math.hypot(nx - px, ny - py) || 1;
      const half = (widths[i] ?? 1) / 2;
      left.push(f.p(x + (-(ny - py) / length) * half, y + ((nx - px) / length) * half));
      right.push(f.p(x - (-(ny - py) / length) * half, y - ((nx - px) / length) * half));
    });
    return `M${left.join("L")}L${right.reverse().join("L")}Z`;
  };
  const lift = between(rng, -0.6, 0.6);
  const claws = (hx: number, hy: number): string =>
    f.poly([[hx, hy - 1.2], [hx + 2.6, hy - 0.8], [hx, hy - 0.3]]) + f.poly([[hx, hy + 0.1], [hx + 2.8, hy + 0.6], [hx, hy + 1]]);
  return [
    shape(taper([[-8, -3], [-15, -1.5], [-20, -5], [-20, -11], [-16, -14.5 + lift]], [5.5, 4.6, 3.6, 2.4, 0.4]), "blue-700", 1, MAIN),
    shape(f.ellipse(0, -10, 14, 10), "blue-700", 1, MAIN_INKED),
    shape(taper([[7, -14], [10, -20], [10, -25.5]], [8, 6, 5]), "blue-700", 1, MAIN),
    shape(`M${f.p(-11, -13)}Q${f.p(-12, -25)} ${f.p(-5, -30)}Q${f.p(-1, -25)} ${f.p(2, -21)}Q${f.p(4, -17)} ${f.p(3, -12)}Q${f.p(-4, -14)} ${f.p(-11, -13)}Z`, "blue-800", 1),
    shape(f.band([[-5, -29], [-7, -20], [-8, -14]], 0.7) + f.band([[-3, -26], [0, -22], [3, -14]], 0.7), "blue-900", 0.6),
    shape(f.band([[12.5, -23], [11.5, -17], [10, -11], [5, -4], [-3, -1.5]], 3.2), "sky-300", 0.95),
    shape(`M${f.p(8, -28)}Q${f.p(9, -31)} ${f.p(12, -31)}Q${f.p(15, -30.5)} ${f.p(15.5, -27.5)}Q${f.p(20, -27)} ${f.p(20.5, -24.5)}Q${f.p(20, -22.5)} ${f.p(16, -22.5)}Q${f.p(12, -21.5)} ${f.p(9, -23)}Q${f.p(7, -25)} ${f.p(8, -28)}Z`, "blue-700", 1, MAIN_INKED),
    shape(f.poly([[9, -30], [12, -31], [5, -35]]) + f.poly([[12.5, -31], [14.5, -30.2], [10, -34]]), "slate-400", 1),
    shape(f.circle(12.8, -27.4, 1), "orange-400", 1),
    shape(f.ellipse(13, -27.4, 0.4, 0.8), "slate-900", 1),
    shape(f.band([[7, -12], [13, -11.8], [18.5, -11.6]], 2.6) + f.band([[8, -15.2], [13, -14.8], [18, -14.6]], 2.6), "blue-700", 1, MAIN),
    shape(claws(18.5, -11.6) + claws(18, -14.6), "slate-100", 1),
  ];
}

/** A baby dragon standing on the anchor, facing the side the part faces, 6 tall at scale 1. */
function hatchlingPart(at: PartPlace, rng: Rng): KitShape[] {
  const f = local(at);
  const tail = between(rng, -0.4, 0.4);
  return [
    shape(f.band([[-1.2, -1], [-2.8, -0.6], [-3.4, -1.8 + tail]], 0.6), "blue-600", 1, MAIN),
    shape(f.poly([[-0.8, -3.2], [-2.6, -4.6], [-1.6, -2.4]]), "blue-800", 1),
    shape(f.ellipse(0, -2.2, 1.6, 1.7), "blue-600", 1, MAIN_INKED),
    shape(f.ellipse(0.6, -2, 0.9, 1.3), "sky-300", 0.95),
    shape(f.ellipse(0.5, -0.2, 0.7, 0.28) + f.ellipse(-0.9, -0.2, 0.7, 0.28), "blue-700", 1),
    shape(f.circle(0.9, -4.6, 1.5), "blue-600", 1, MAIN_INKED),
    shape(f.poly([[0.1, -5.9], [-0.3, -6.7], [0.8, -6.1]]) + f.poly([[1.3, -6.1], [1.9, -6.7], [2, -5.7]]), "slate-100", 1),
    shape(f.circle(1.5, -4.8, 0.6), "white", 1),
    shape(f.circle(1.7, -4.8, 0.32), "slate-900", 1),
  ];
}

/** A plump velvet pillow bristling with steel spikes, standing on the anchor, 24 wide and 14 tall at scale 1. */
function spikyPillowPart(at: PartPlace, rng: Rng): KitShape[] {
  const f = local(at);
  let topSpikes = "";
  let shade = "";
  for (let i = 0; i < 6; i += 1) {
    const x = -9 + 3.6 * i;
    const tip = -14 + between(rng, -0.4, 0.4);
    topSpikes += f.poly([[x - 1.2, -11], [x, tip], [x + 1.2, -11]]);
    shade += f.poly([[x, -11], [x, tip], [x + 1.2, -11]]);
  }
  let side = "";
  for (const s of [-1, 1]) for (const y of [-4.5, -8]) side += f.poly([[s * 10.2, y - 1], [s * 12.6, y + between(rng, -0.3, 0.3)], [s * 10.2, y + 1]]);
  return [
    shape(`M${f.p(-10.5, -1)}Q${f.p(-11.5, -6)} ${f.p(-10.2, -10)}Q${f.p(-5, -12.2)} ${f.p(0, -11.4)}Q${f.p(5, -12.2)} ${f.p(10.2, -10)}Q${f.p(11.5, -6)} ${f.p(10.5, -1)}Q${f.p(0, 1)} ${f.p(-10.5, -1)}Z`, "violet-700", 1, MAIN_INKED),
    shape(`M${f.p(-9.5, -9.5)}Q${f.p(-5, -11.6)} ${f.p(0, -10.9)}Q${f.p(5, -11.6)} ${f.p(9.5, -9.5)}Q${f.p(0, -7.5)} ${f.p(-9.5, -9.5)}Z`, "violet-500", 0.7),
    shape(f.circle(-10.8, -0.6, 1) + f.circle(10.8, -0.6, 1) + f.circle(-10.4, -10.2, 0.9) + f.circle(10.4, -10.2, 0.9), "brass-600", 1),
    shape(topSpikes + side, "slate-100", 1),
    shape(shade, "slate-400", 0.9),
    shape(f.ellipse(-5, -6.5, 3, 1.2), "white", 0.25),
  ];
}

/** A literal boulder with a weathered raised-eyebrow face and a tiny apron, standing on the anchor, 30 wide and 26 tall at scale 1. */
function boulderPart(at: PartPlace, rng: Rng): KitShape[] {
  const f = local(at);
  const j = between(rng, -0.6, 0.6);
  return [
    shape(`M${f.p(-15, 0)}Q${f.p(-16.5, -10)} ${f.p(-10, -19)}Q${f.p(-3, -26 + j)} ${f.p(6, -24)}Q${f.p(14, -20)} ${f.p(15.2, -9)}Q${f.p(15.5, -3)} ${f.p(14, 0)}Z`, "stone-500", 1, MAIN_INKED),
    shape(`M${f.p(-12, -17)}Q${f.p(-7, -23.5)} ${f.p(2, -24)}Q${f.p(8, -24)} ${f.p(11, -21)}Q${f.p(4, -21.5)} ${f.p(-3, -19)}Q${f.p(-8, -17.5)} ${f.p(-12, -17)}Z`, "slate-100", 0.35),
    shape(f.band([[9, -23], [8.5, -18], [10.5, -14], [9.5, -10]], 0.6) + f.band([[-13, -9], [-10.5, -6], [-11.5, -2]], 0.6) + f.band([[11, -6], [8, -4], [9, -1.5]], 0.6), "slate-900", 0.45),
    shape(f.ellipse(-5, -14.5, 2.1, 1.7) + f.ellipse(4.5, -14.5, 2.1, 1.7), "white", 0.95),
    shape(f.circle(-4.4, -14.3, 0.8) + f.circle(5.1, -14.3, 0.8), "slate-900", 1),
    shape(f.band([[-8.5, -17.8], [-1.8, -17.8]], 1.3) + f.band([[1.8, -19.5], [5.5, -22.3], [9, -21.3]], 1.3), "slate-900", 0.85),
    shape(f.band([[-5, -9], [0, -8.4], [5, -9.2], [7.2, -11]], 0.9), "slate-900", 0.9),
    shape(f.poly([[-5, -7], [5, -7], [6.6, -0.4], [-6.6, -0.4]]), "white", 0.95),
    shape(f.poly([[-2.3, -4.2], [2.3, -4.2], [2.3, -2], [-2.3, -2]]), "slate-400", 0.5),
    shape(f.band([[-5, -6.4], [-14.5, -6.8]], 0.5) + f.band([[5, -6.4], [14.5, -6.8]], 0.5), "white", 0.9),
  ];
}

/** A plump crusty loaf with stubby legs standing on the anchor, facing the side the part faces, 18 wide and 12 tall at scale 1. */
function loafPart(at: PartPlace, rng: Rng): KitShape[] {
  const f = local(at);
  let flour = "";
  for (let i = 0; i < 5; i += 1) flour += f.circle(-7 + 3.4 * i + between(rng, -0.4, 0.4), -4.6 + between(rng, -0.6, 0.6), 0.28);
  return [
    shape([-6.5, -3.2, 3.2, 6.5].map((x) => f.band([[x, -3.2], [x, 0]], 1.8)).join(""), "wood-700", 1),
    shape(`M${f.p(-9, -3)}Q${f.p(-10, -10)} ${f.p(-4, -11.5)}Q${f.p(0, -12.4)} ${f.p(4, -11.5)}Q${f.p(10, -10)} ${f.p(9, -3)}Q${f.p(0, -1.5)} ${f.p(-9, -3)}Z`, "brass-600", 1, MAIN_INKED),
    shape(f.ellipse(0, -9.2, 6.2, 2.2), "brass-500", 0.95),
    shape(f.band([[-3.8, -10.6], [-2.6, -8]], 0.5) + f.band([[-0.6, -11], [0.6, -8.2]], 0.5) + f.band([[2.6, -10.6], [3.8, -8]], 0.5), "brass-800", 0.5),
    shape(f.circle(-2.4, -6, 0.6) + f.circle(2.4, -6, 0.6), "slate-900", 1),
    shape(f.circle(-2.2, -6.2, 0.2) + f.circle(2.6, -6.2, 0.2), "white", 0.9),
    shape(f.band([[-1.8, -4.4], [-0.8, -3.7], [0.8, -3.7], [1.8, -4.4]], 0.38), "slate-900", 0.9),
    shape(f.circle(-4.6, -4.6, 0.8) + f.circle(4.6, -4.6, 0.8), "red-300", 0.5),
    shape(flour, "white", 0.8),
  ];
}

/** A small round-bodied sprinter with springy legs and a headband, running toward the side the part faces, standing on the anchor, 12 tall at scale 1. */
function rushTokenPart(at: PartPlace, rng: Rng): KitShape[] {
  const f = local(at);
  const flutter = between(rng, -0.5, 0.5);
  const spring = (side: number): string =>
    f.band([[side * 1.2, -3.4], [side * 2.4, -2.8], [side * 1, -2.2], [side * 2.6, -1.6], [side * 1.6, -1], [side * 3.2, 0]], 0.5);
  return [
    shape(spring(-1) + spring(1), "slate-400", 1),
    shape(f.circle(0.3, -7.6, 4.3), "red-500", 1, MAIN_INKED),
    shape(f.ellipse(-1.6, -8.6, 1.3, 2.3), "white", 0.25),
    shape(f.band([[-3.6, -9.4], [0, -11], [3.8, -9.2]], 1.1), "white", 1),
    shape(f.poly([[-3.3, -9.8], [-6.4, -11 + flutter], [-5.7, -9.9]]) + f.poly([[-3.3, -9.1], [-6.8, -8.7 + flutter], [-5.6, -8.2]]), "white", 1),
    shape(f.ellipse(1.6, -7.9, 0.4, 0.55) + f.ellipse(3.3, -7.7, 0.4, 0.55), "slate-900", 1),
    shape(f.band([[0.9, -9], [2.2, -8.5]], 0.35) + f.band([[2.7, -8.8], [4.1, -8.2]], 0.35), "slate-900", 1),
    shape(f.band([[1.8, -6], [3.8, -5.2], [5, -6.6]], 0.9) + f.band([[-2.5, -6], [-4.4, -4.8], [-5.2, -6.2]], 0.9), "red-500", 1, MAIN),
  ];
}

/** A horned four-legged beast standing on the anchor, facing the side the part faces, 18 wide and 12 tall at scale 1. */
function beastPart(at: PartPlace, rng: Rng): KitShape[] {
  const f = local(at);
  const swish = between(rng, -1, 1);
  return [
    shape(f.band([[-5.6, -7], [-7.6, -6 + swish], [-8.4, -4.6 + swish]], 0.9), "wood-700", 1, MAIN),
    shape(f.band([[-3.6, -4], [-3.8, 0]], 1.5) + f.band([[3.2, -4], [3.4, 0]], 1.5), "wood-800", 1),
    shape(f.ellipse(0, -6, 6, 3.2), "wood-700", 1, MAIN_INKED),
    shape(f.band([[-4.9, -4], [-5, 0]], 1.5) + f.band([[4.6, -4], [4.8, 0]], 1.5), "wood-700", 1, MAIN),
    shape(f.ellipse(7, -8, 2.5, 2.1), "wood-700", 1, MAIN_INKED),
    shape(f.ellipse(9, -7.1, 1.5, 1.1), "wood-600", 1),
    shape(f.band([[6.2, -9.6], [5.6, -11.4], [7.4, -12.2]], 0.7) + f.band([[7.8, -9.7], [8.8, -11.4], [10.4, -11.2]], 0.7), "slate-100", 1),
    shape(f.circle(7.8, -8.4, 0.3), "orange-400", 1),
  ];
}

/** A small green goblin tumbling, centred on the anchor, 8 tall at scale 1 (tilted by the seed). */
function goblinPart(at: PartPlace, rng: Rng): KitShape[] {
  const f = local(at);
  const tilt = between(rng, -40, 40);
  const P = (x: number, y: number): Point => turned([x, y], tilt);
  const band = (points: readonly Point[], width: number): string => f.band(points.map(([x, y]) => P(x, y)), width);
  const poly = (points: readonly Point[]): string => f.poly(points.map(([x, y]) => P(x, y)));
  const ball = (x: number, y: number, r: number): string => f.circle(...P(x, y), r);
  return [
    shape(
      band([[-0.9, -0.3], [-2.3, -1.6], [-3, -2.8]], 0.6) + band([[0.9, -0.3], [2.4, 0.4], [3.4, -0.6]], 0.6) +
        band([[-0.5, 1.2], [-1.4, 2.6], [-0.6, 3.6]], 0.6) + band([[0.5, 1.2], [1.8, 2.2], [3, 1.6]], 0.6),
      "green-500",
      1,
      MAIN,
    ),
    shape(poly([[-1.3, -0.8], [1.3, -0.8], [1.6, 1.6], [-1.6, 1.6]]), "slate-800", 1),
    shape(poly([[-1, -2.4], [-3.8, -3.6], [-1.2, -1.4]]) + poly([[1, -2.4], [3.8, -3.6], [1.2, -1.4]]), "green-500", 1, MAIN),
    shape(ball(0, -2.3, 1.5), "green-500", 1, MAIN_INKED),
    shape(ball(-0.55, -2.5, 0.28) + ball(0.55, -2.5, 0.28), "slate-900", 1),
    shape(band([[-0.6, -1.7], [0, -1.45], [0.6, -1.7]], 0.2), "slate-900", 1),
  ];
}

/** A fluffy white snow bunny with hypnotic swirl eyes sitting on the anchor, facing the side the part faces, 14 tall at scale 1. */
function bunnyPart(at: PartPlace, rng: Rng): KitShape[] {
  const f = local(at);
  const turn = between(rng, 0, 90);
  const swirl = (cx: number, cy: number): string => {
    const points: Point[] = [];
    for (let i = 0; i <= 10; i += 1) {
      const t = i / 10;
      const angle = (turn + t * 2.2 * FULL_TURN) * DEGREES;
      points.push([cx + 0.15 + t * 0.75 * Math.cos(angle), cy + t * 0.75 * Math.sin(angle)]);
    }
    return f.band(points, 0.2);
  };
  return [
    shape(f.circle(-4.2, -3.6, 1.3), "slate-100", 0.95),
    shape(f.ellipse(0, -3.5, 4, 3.5), "white", 1, MAIN_INKED),
    shape(f.ellipse(0, -1, 3, 1.2), "slate-400", 0.25),
    shape(f.ellipse(2.4, -0.3, 1.5, 0.6), "white", 1, MAIN_INKED),
    shape(`M${f.p(0, -10.6)}Q${f.p(-1.8, -13.5)} ${f.p(-0.4, -14)}Q${f.p(1.4, -13.2)} ${f.p(1.6, -10.6)}Z`, "white", 1, MAIN_INKED),
    shape(`M${f.p(2.2, -10.8)}Q${f.p(2.4, -13.8)} ${f.p(3.8, -13.6)}Q${f.p(4.8, -12.4)} ${f.p(3.8, -10.4)}Z`, "white", 1, MAIN_INKED),
    shape(f.circle(1.6, -8, 3), "white", 1, MAIN_INKED),
    shape(`M${f.p(0.2, -11)}Q${f.p(-0.9, -12.8)} ${f.p(-0.3, -13)}Q${f.p(0.8, -12.4)} ${f.p(1, -11)}Z`, "red-300", 0.9),
    shape(swirl(0.6, -8.4) + swirl(2.9, -8.4), "violet-500", 1),
    shape(f.ellipse(4.3, -7.2, 0.45, 0.32), "red-300", 1),
  ];
}

/** A grinning brown bear bursting up, its upper body standing on the anchor, facing the viewer, 26 wide and 26 tall at scale 1. */
function bearPart(at: PartPlace, rng: Rng): KitShape[] {
  const f = local(at);
  const reach = between(rng, -0.5, 0.5);
  const clawSet = (s: number): string =>
    [10.4, 11.9, 13.3].map((c) => f.poly([[s * (c - 0.55), -24.3], [s * (c + 0.1), -26.4 + reach * 0.3], [s * (c + 0.55), -24.3]])).join("");
  const teeth = [-2.4, -0.9, 0.9, 2.4].map((c) => f.poly([[c - 0.7, -13.6], [c, -12.4], [c + 0.7, -13.6]])).join("");
  return [
    shape(f.band([[-8, -9], [-11.4, -15], [-11.8, -20]], 4.6) + f.band([[8, -9], [11.4, -15], [11.8, -20]], 4.6), "wood-600", 1, MAIN),
    shape(f.ellipse(0, -8, 9, 9.5), "wood-600", 1, MAIN_INKED),
    shape(f.circle(-4.6, -21.4, 2) + f.circle(4.6, -21.4, 2), "wood-600", 1, MAIN),
    shape(f.circle(0, -16.5, 5.6), "wood-600", 1, MAIN_INKED),
    shape(f.ellipse(0, -14.4, 3.3, 2.5), "brass-500", 1),
    shape(f.ellipse(0, -15.7, 1.1, 0.75), "slate-900", 1),
    shape(`M${f.p(-3.2, -13.8)}Q${f.p(0, -9.8)} ${f.p(3.2, -13.8)}Q${f.p(0, -13.2)} ${f.p(-3.2, -13.8)}Z`, "slate-900", 1),
    shape(teeth, "white", 1),
    shape(f.circle(-2.4, -18.2, 0.75) + f.circle(2.4, -18.2, 0.75), "slate-900", 1),
    shape(f.circle(-11.8, -21.8, 2.6) + f.circle(11.8, -21.8, 2.6), "wood-600", 1, MAIN),
    shape(clawSet(-1) + clawSet(1), "slate-100", 1),
  ];
}

/** A packed concert crowd spanning 60 about the anchor (30 each way), standing on it, about 14 tall at scale 1: heads and raised arms in two rows, light sticks above. */
function crowdPart(at: PartPlace, rng: Rng): KitShape[] {
  const f = local(at);
  let heads2 = "";
  let arms2 = "";
  let heads = "";
  let arms = "";
  let blue = "";
  let red = "";
  for (let i = 0; i < 17; i += 1) heads2 += f.circle(-29 + 3.7 * i + between(rng, -0.4, 0.4), -9 + between(rng, -0.6, 0.6), 1.3);
  for (let i = 0; i < 8; i += 1) {
    const x = -27 + 7.7 * i + between(rng, -0.8, 0.8);
    const top = -15.5 - between(rng, 0, 1.5);
    arms2 += f.band([[x, -8], [x + between(rng, -0.8, 0.8), top]], 0.9);
  }
  for (let i = 0; i < 18; i += 1) heads += f.circle(-28.5 + 3.4 * i + between(rng, -0.4, 0.4), -5.6 + between(rng, -0.7, 0.7), 1.4);
  for (let i = 0; i < 9; i += 1) {
    const x = -28 + 7 * i + between(rng, -0.8, 0.8);
    const top = -11.5 - between(rng, 0, 2);
    const lean = between(rng, -1, 1);
    arms += f.band([[x, -3], [x + lean, top]], 1);
    const dot = f.circle(x + lean, top - 0.9, 0.55);
    if (i % 2 === 0) blue += dot;
    else red += dot;
  }
  return [
    shape(heads2, "slate-800", 1),
    shape(arms2, "slate-800", 1),
    shape(f.poly([[-30, 0], [-30, -3.6], [-15, -4.2], [0, -3.8], [15, -4.2], [30, -3.6], [30, 0]]), "slate-900", 1, MAIN),
    shape(heads, "slate-900", 1, MAIN),
    shape(arms, "slate-900", 1, MAIN),
    shape(blue, "sky-300", 0.95),
    shape(red, "red-300", 0.95),
  ];
}

/** A small simple soldier standing on the anchor, facing the side the part faces, 14 tall (9 when crouched) at scale 1; with a spear upright or levelled forward. */
function soldierPart(at: PartPlace, rng: Rng): KitShape[] {
  const f = local(at);
  if (rng() < 0.5) {
    return [
      shape(f.band([[-1, -5.5], [-1.2, 0]], 1.5) + f.band([[1, -5.5], [1.2, 0]], 1.5), "slate-700", 1),
      shape(f.poly([[-2.2, -5.8], [2.2, -5.8], [2.6, -10.5], [-2.6, -10.5]]), "blue-700", 1, MAIN_INKED),
      shape(f.band([[2.2, -9.5], [3.4, -7.8], [3, -6.6]], 1), "blue-700", 1, MAIN),
      shape(f.band([[3, -0.5], [3, -16]], 0.5), "wood-700", 1),
      shape(f.poly([[2.4, -16], [3, -18], [3.6, -16]]), "slate-100", 1),
      shape(f.circle(0.2, -11.8, 1.3), "red-300", 1),
      shape(`M${f.p(-1.6, -11.8)}Q${f.p(-1.6, -14.2)} ${f.p(0.2, -14.2)}Q${f.p(2.1, -14.2)} ${f.p(2.1, -11.8)}L${f.p(2.6, -11.5)}L${f.p(-2.1, -11.5)}Z`, "slate-400", 1),
    ];
  }
  return [
    shape(f.band([[-1.6, -3], [-3.2, -1.6], [-2.4, 0]], 1.5) + f.band([[1, -3], [2.8, -3.2], [2.4, 0]], 1.5), "slate-700", 1),
    shape(f.poly([[-2.4, -3.6], [2, -3.6], [3.4, -7], [-1, -7]]), "blue-700", 1, MAIN_INKED),
    shape(f.band([[2.8, -6.2], [4.4, -5], [5.6, -4.8]], 1), "blue-700", 1, MAIN),
    shape(f.band([[-2, -4.4], [11, -5.2]], 0.5), "wood-700", 1),
    shape(f.poly([[11, -6.1], [13, -5.3], [11, -4.5]]), "slate-100", 1),
    shape(f.circle(2.3, -8, 1.3), "red-300", 1),
    shape(`M${f.p(0.7, -8)}Q${f.p(0.7, -10.2)} ${f.p(2.3, -10.2)}Q${f.p(4, -10.2)} ${f.p(4, -8)}L${f.p(4.4, -7.7)}L${f.p(0.3, -7.7)}Z`, "slate-400", 1),
  ];
}

/** The same small soldier lying fallen on the anchor, head to the side the part faces, 14 long at scale 1, helmet off beside it. */
function soldierFallenPart(at: PartPlace): KitShape[] {
  const f = local(at);
  return [
    shape(f.band([[-7, -0.5], [-3, -0.4]], 1.2), "slate-700", 1),
    shape(f.poly([[-3.4, -0.1], [-3.2, -2.5], [2.6, -2.7], [2.8, -0.1]]), "blue-700", 1, MAIN_INKED),
    shape(f.band([[2.4, -1.1], [4.2, -0.5]], 0.9), "blue-700", 1, MAIN),
    shape(f.circle(4, -1.3, 1.25), "red-300", 1),
    shape(f.band([[-6, -0.15], [6.6, -0.1]], 0.3), "wood-700", 0.9),
    shape(`M${f.p(5.2, -0.1)}Q${f.p(5.3, -1.8)} ${f.p(6.8, -1.8)}Q${f.p(8, -1.8)} ${f.p(8.1, -0.1)}Z`, "slate-400", 1),
  ];
}

/** A faint ghost-figure standing on the anchor, 20 tall at scale 1: a translucent cloaked body with dark eye holes and a wispy hem. */
function ghostPart(at: PartPlace, rng: Rng): KitShape[] {
  const f = local(at);
  const sway = between(rng, -0.8, 0.8);
  return [
    shape(
      `M${f.p(-5, -1)}Q${f.p(-6.5, -12)} ${f.p(-4 + sway, -17)}Q${f.p(sway, -21.5)} ${f.p(4 + sway, -17)}Q${f.p(6.5, -12)} ${f.p(5, -1)}` +
        `Q${f.p(3.5, 1)} ${f.p(2.5, -1)}Q${f.p(1, 1.5)} ${f.p(0, -0.5)}Q${f.p(-1.5, 1.5)} ${f.p(-2.5, -1)}Q${f.p(-3.5, 1)} ${f.p(-5, -1)}Z`,
      "slate-100",
      0.35,
      MAIN,
    ),
    shape(f.ellipse(-0.5 + sway * 0.5, -11, 2.8, 7), "white", 0.18),
    shape(f.ellipse(-1.9 + sway * 0.5, -15, 0.9, 1.3) + f.ellipse(1.9 + sway * 0.5, -15, 0.9, 1.3), "slate-900", 0.75),
    shape(f.ellipse(sway * 0.5, -11.8, 0.7, 1.1), "slate-900", 0.6),
    shape(f.band([[-3, 0.5], [-3.6, 1.8]], 0.5) + f.band([[0.2, 0.5], [0.6, 2]], 0.5) + f.band([[3, 0.5], [3.8, 1.6]], 0.5), "slate-100", 0.2),
  ];
}

/** A little card-shaped imp centred on the anchor, 10 tall at scale 1, tilted by the seed. */
function cardImpPart(at: PartPlace, rng: Rng): KitShape[] {
  const f = local(at);
  const tilt = between(rng, -15, 15);
  const P = (x: number, y: number): Point => turned([x, y], tilt);
  const poly = (points: readonly Point[]): string => f.poly(points.map(([x, y]) => P(x, y)));
  const band = (points: readonly Point[], width: number): string => f.band(points.map(([x, y]) => P(x, y)), width);
  const ball = (x: number, y: number, r: number): string => f.circle(...P(x, y), r);
  return [
    shape(band([[-2.7, -0.5], [-4.2, 0.5], [-4.6, -0.8]], 0.45) + band([[2.7, -0.5], [4.2, -1.8], [4.8, -3]], 0.45) + band([[-1, 2.2], [-1.4, 3.8], [-2.4, 4.4]], 0.45) + band([[1, 2.2], [1.6, 3.6], [2.8, 3.4]], 0.45), "slate-900", 1),
    shape(poly([[-2.7, -4.2], [2.7, -4.2], [2.7, 2.2], [-2.7, 2.2]]), "white", 1),
    shape(poly([[-2.2, -3.7], [2.2, -3.7], [2.2, 1.7], [-2.2, 1.7]]), "red-500", 1, MAIN),
    shape(poly([[-2.2, -4.2], [-2.9, -5.4], [-1.2, -4.2]]) + poly([[1.2, -4.2], [2.9, -5.4], [2.2, -4.2]]), "red-500", 1, MAIN),
    shape(ball(-1, -1.8, 0.55) + ball(1, -1.8, 0.55), "white", 1),
    shape(ball(-0.9, -1.7, 0.22) + ball(1.1, -1.7, 0.22), "slate-900", 1),
    shape(band([[-1.4, 0], [-0.5, 0.6], [0.5, 0.6], [1.4, 0]], 0.4), "white", 1),
  ];
}

/** An open hand, palm up, its wrist at the anchor, reaching toward the side the part faces, 10 long at scale 1. */
function openHandPart(at: PartPlace, rng: Rng): KitShape[] {
  const f = local(at);
  const curl = between(rng, -0.4, 0.4);
  const finger = (y: number, spread: number, length: number): string =>
    f.band([[5.3, y], [7.4, y + spread * 0.5], [5.3 + length, y + spread + curl]], 0.7);
  return [
    shape(finger(-1.3, -0.4, 4.2) + finger(-0.45, -0.1, 4.7) + finger(0.45, 0.2, 4.5) + finger(1.3, 0.6, 3.8), "red-300", 1, MAIN),
    shape(f.ellipse(3.8, 0, 2.4, 1.7), "red-300", 1, MAIN_INKED),
    shape(f.band([[3.4, -1.4], [5.2, -2.6], [6.4, -3.4 + curl]], 0.8), "red-300", 1, MAIN),
    shape(f.ellipse(3.8, 0.1, 1.5, 1), "white", 0.25),
    shape(f.poly([[0, -1.8], [1.6, -1.8], [1.6, 1.8], [0, 1.8]]), "slate-100", 1),
  ];
}

// @@ C creatures kit
  gull: { category: "creatures", draw: gullPart },
  cat: { category: "creatures", draw: catPart },
  sheep: { category: "creatures", draw: sheepPart },
  bee: { category: "creatures", draw: beePart },
  scarab: { category: "creatures", draw: scarabPart },
  cube: { category: "creatures", draw: cubePart },
  panther: { category: "creatures", draw: pantherPart },
  dragon: { category: "creatures", draw: dragonPart },
  hatchling: { category: "creatures", draw: hatchlingPart },
  "spiky-pillow": { category: "creatures", draw: spikyPillowPart },
  boulder: { category: "creatures", draw: boulderPart },
  loaf: { category: "creatures", draw: loafPart },
  "rush-token": { category: "creatures", draw: rushTokenPart },
  beast: { category: "creatures", draw: beastPart },
  goblin: { category: "creatures", draw: goblinPart },
  bunny: { category: "creatures", draw: bunnyPart },
  bear: { category: "creatures", draw: bearPart },
  crowd: { category: "creatures", draw: crowdPart },
  soldier: { category: "creatures", draw: soldierPart },
  "soldier-fallen": { category: "creatures", draw: soldierFallenPart },
  ghost: { category: "creatures", draw: ghostPart },
  "card-imp": { category: "creatures", draw: cardImpPart },
  "open-hand": { category: "creatures", draw: openHandPart },
