// @@ E objects
/** A rectangle in a part's frame: its top-left corner, width and height. */
function eRect(f: Local, x: number, y: number, w: number, h: number): string {
  return f.poly([[x, y], [x + w, y], [x + w, y + h], [x, y + h]]);
}

/** A chess piece standing on the anchor, 6 tall at scale 1: a pawn, a rook, a knight's head or a king. */
function chessPiecePart(at: PartPlace, rng: Rng): KitShape[] {
  const f = local(at);
  const kind = whole(rng, 0, 3);
  const base = shape(f.poly([[-2.3, 0], [2.3, 0], [2, -1], [-2, -1]]), "slate-400", 1, INKED);
  if (kind === 0) {
    return [
      base,
      shape(f.poly([[-1.3, -1], [1.3, -1], [0.7, -3.4], [-0.7, -3.4]]), "slate-100", 1, MAIN_INKED),
      shape(f.poly([[-1.2, -3.2], [1.2, -3.2], [1.2, -3.8], [-1.2, -3.8]]), "slate-100", 1, MAIN_INKED),
      shape(f.circle(0, -4.7, 1.3), "slate-100", 1, MAIN_INKED),
      shape(f.poly([[0.1, -1], [1.3, -1], [0.7, -3.4], [0.1, -3.4]]) + f.circle(0.5, -4.5, 0.75), "slate-900", 0.2),
      shape(f.circle(-0.5, -5.1, 0.45), "white", 0.7),
    ];
  }
  if (kind === 1) {
    return [
      base,
      shape(f.poly([[-1.5, -1], [1.5, -1], [1, -4.2], [-1, -4.2]]), "slate-100", 1, MAIN_INKED),
      shape(f.poly([[-1.8, -4.2], [1.8, -4.2], [1.8, -6], [1.3, -6], [1.3, -5.2], [0.45, -5.2], [0.45, -6], [-0.45, -6], [-0.45, -5.2], [-1.3, -5.2], [-1.3, -6], [-1.8, -6]]), "slate-100", 1, MAIN_INKED),
      shape(f.poly([[0.2, -1], [1.5, -1], [1, -4.2], [0.2, -4.2]]) + f.poly([[0.6, -4.2], [1.8, -4.2], [1.8, -6], [1.3, -6], [1.3, -5.2], [0.6, -5.2]]), "slate-900", 0.2),
      shape(f.band([[-0.9, -1.6], [-0.6, -4]], 0.4), "white", 0.7),
    ];
  }
  if (kind === 2) {
    return [
      base,
      shape(f.poly([[-1.6, -1], [1.6, -1], [1.3, -3], [1.7, -4.6], [0.7, -6.1], [0, -5.4], [-0.5, -6], [-1.3, -5.1], [-2.5, -3.8], [-2.3, -3], [-1.1, -3.4], [-0.3, -3.1]]), "slate-100", 1, MAIN_INKED),
      shape(f.poly([[0.3, -1], [1.6, -1], [1.3, -3], [1.7, -4.6], [0.7, -6.1], [0.5, -4]]), "slate-900", 0.2),
      shape(f.circle(-0.7, -4.5, 0.32), "slate-900", 0.9),
      shape(f.band([[-0.6, -5.5], [0.4, -3.5]], 0.35), "white", 0.6),
    ];
  }
  return [
    base,
    shape(f.poly([[-1.5, -1], [1.5, -1], [0.9, -4], [-0.9, -4]]), "slate-100", 1, MAIN_INKED),
    shape(f.poly([[-1.4, -4], [1.4, -4], [1.7, -5], [-1.7, -5]]), "slate-100", 1, MAIN_INKED),
    shape(f.band([[0, -5], [0, -6.4]], 0.55) + f.band([[-0.7, -5.8], [-0.2, -5.8]], 0.5) + f.band([[0.2, -5.8], [0.7, -5.8]], 0.5), "slate-100", 1, MAIN_INKED),
    shape(f.poly([[0.1, -1], [1.5, -1], [0.9, -4], [0.1, -4]]) + f.poly([[0.1, -4], [1.4, -4], [1.7, -5], [0.1, -5]]), "slate-900", 0.2),
    shape(f.band([[-0.8, -1.6], [-0.5, -3.6]], 0.4), "white", 0.7),
  ];
}

/** An hourglass standing on the anchor, 4 wide and 7 tall at scale 1. */
function hourglassPart(at: PartPlace, rng: Rng): KitShape[] {
  const f = local(at);
  const top = between(rng, 0, 0.6);
  return [
    shape(eRect(f, -2.1, -7, 4.2, 0.65) + eRect(f, -2.1, -0.65, 4.2, 0.65), "wood-700", 1, INKED),
    shape(f.band([[-1.8, -6.35], [-1.8, -0.65]], 0.4) + f.band([[1.8, -6.35], [1.8, -0.65]], 0.4), "wood-700", 1),
    shape(f.poly([[-1.5, -6.35], [1.5, -6.35], [0.2, -3.5], [1.5, -0.65], [-1.5, -0.65], [-0.2, -3.5]]), "sky-300", 0.45, INKED),
    shape(f.poly([[-1.05 + top, -5.4 + top], [1.05 - top, -5.4 + top], [0.12, -3.8], [-0.12, -3.8]]) + f.poly([[-1.35, -0.65], [1.35, -0.65], [0.5, -1.9], [-0.5, -1.9]]), "brass-500", 1, MAIN),
    shape(f.band([[0, -3.8], [0, -1.9]], 0.18), "brass-500", 0.9),
    shape(f.band([[-1, -6], [-0.55, -4.4]], 0.3), "white", 0.7),
  ];
}

/** A desk globe on its stand, 7 tall at scale 1, standing on the anchor. */
function globePart(at: PartPlace, rng: Rng): KitShape[] {
  const f = local(at);
  const d = between(rng, -0.4, 0.4);
  return [
    shape(f.poly([[-1.9, 0], [1.9, 0], [1.3, -0.7], [-1.3, -0.7]]), "brass-600", 1, INKED),
    shape(f.band([[0, -0.7], [0, -1.4]], 0.6), "brass-600", 1),
    shape(f.circle(0, -4.1, 2.4), "blue-600", 1, MAIN_INKED),
    shape(f.poly([[-1.9, -4.9 + d], [-0.6, -5.6], [-0.2, -4.6], [-1.2, -3.9]]) + f.poly([[0.5, -3.7], [1.8, -3.9], [1.6, -2.7 + d], [0.6, -2.4]]), "green-500", 1),
    shape(f.circle(0, -4.1, 2.85) + f.circle(0, -4.1, 2.55), "slate-400", 1),
    shape(f.ellipse(-0.8, -5.2, 0.6, 0.35), "white", 0.7),
    shape(f.ellipse(1, -3.2, 1.4, 1.5), "slate-900", 0.15),
  ];
}

/** An open blank spiral notebook lying on the anchor, 14 wide at scale 1, a page lifted. */
function notebookPart(at: PartPlace, rng: Rng): KitShape[] {
  const f = local(at);
  const lift = between(rng, -1.2, 1.2);
  const rings: string[] = [];
  for (let i = 0; i < 5; i += 1) rings.push(f.ellipse(0, -2.5 + i * 0.95, 0.75, 0.3));
  return [
    shape(f.ellipse(0, 1.1, 7.2, 1), "slate-900", 0.25),
    shape(f.poly([[-7, -3.6], [-0.3, -3], [-0.3, 1.5], [-7, 0.7]]), "white", 1, INKED),
    shape(f.poly([[0.3, -3], [7, -3.6], [7, 0.7], [0.3, 1.5]]), "slate-100", 1, MAIN_INKED),
    shape(f.poly([[0.3, -3], [1.2, -2.95], [1.2, 1.4], [0.3, 1.5]]) + f.poly([[-0.3, -3], [-1.1, -3.05], [-1.1, 1.4], [-0.3, 1.5]]), "slate-900", 0.1),
    shape(rings.join(""), "slate-400", 1),
    shape(`M${f.p(0.5, -2.9)}Q${f.p(1.5 + lift, -8.4)} ${f.p(6, -6.6)}Q${f.p(4.6, -5)} ${f.p(6.6, -3.5)}Q${f.p(3.4, -4)} ${f.p(0.5, -2.9)}Z`, "white", 0.95, INKED),
    shape(f.band([[1, -3.6], [3.4, -5.4], [5.8, -6.2]], 0.3), "slate-400", 0.4),
  ];
}

/** A park bench standing on the anchor, 24 wide and 9 tall at scale 1. */
function benchPart(at: PartPlace, rng: Rng): KitShape[] {
  const f = local(at);
  const w = between(rng, 11.7, 12);
  return [
    shape(f.ellipse(0, 0, w + 1, 0.7), "slate-900", 0.25),
    shape(f.poly([[-w - 0.4, 0], [-w + 0.4, 0], [-w + 0.4, -3.4], [-w - 0.4, -3.4]]), "slate-900", 1),
    shape(f.poly([[w - 0.4, 0], [w + 0.4, 0], [w + 0.4, -3.4], [w - 0.4, -3.4]]), "slate-900", 1),
    shape(f.band([[-w + 0.4, -9.2], [-w + 0.4, -6.4]], 0.8) + f.band([[w - 0.4, -9.2], [w - 0.4, -6.4]], 0.8), "slate-900", 1),
    shape(f.poly([[-w - 0.4, -6.5], [-w + 0.8, -6.5], [-w + 0.8, -3.4], [-w - 0.4, -3.4]]) + f.poly([[w - 0.8, -6.5], [w + 0.4, -6.5], [w + 0.4, -3.4], [w - 0.8, -3.4]]), "slate-900", 1),
    shape(eRect(f, -w, -9, w * 2, 0.9) + eRect(f, -w, -7.9, w * 2, 0.9) + eRect(f, -w, -6.8, w * 2, 0.9) + eRect(f, -w, -4.6, w * 2, 0.9) + eRect(f, -w, -3.4, w * 2, 0.9), "wood-600", 1, MAIN_INKED),
    shape(eRect(f, -w, -9, w * 2, 0.3) + eRect(f, -w, -4.6, w * 2, 0.3), "white", 0.18),
  ];
}

/** A twig nest with two speckled eggs on the anchor, 16 wide and 4 tall at scale 1. */
function nestPart(at: PartPlace, rng: Rng): KitShape[] {
  const f = local(at);
  const j = between(rng, -0.3, 0.3);
  return [
    shape(`M${f.p(-8, -2.4)}Q${f.p(-7.2, 0.4)} ${f.p(0, 0.4)}Q${f.p(7.2, 0.4)} ${f.p(8, -2.4)}Z`, "wood-700", 1, INKED),
    shape(f.ellipse(0, -2.4, 8, 1.6), "wood-700", 1, MAIN_INKED),
    shape(f.ellipse(0, -2.5, 6.4, 1.1), "wood-800", 1),
    shape(f.ellipse(-1.9, -2.7, 1.5, 1.05) + f.ellipse(1.7 + j, -2.8, 1.5, 1.05), "slate-100", 1, INKED),
    shape(f.circle(-2.4, -2.9, 0.18) + f.circle(-1.5, -2.4, 0.15) + f.circle(1.3 + j, -3, 0.18) + f.circle(2.2 + j, -2.6, 0.15), "wood-700", 0.8),
    shape(f.band([[-8, -2], [-4, -0.9], [0, -0.7], [4, -0.9], [8, -2]], 1.1), "wood-700", 1, INKED),
    shape(f.band([[-7, -1.4], [-3, -0.6]], 0.3) + f.band([[-1, -0.6], [3, -0.7]], 0.3) + f.band([[4, -1.2], [7.4, -2.2]], 0.3), "wood-600", 0.9),
    shape(f.band([[-6, -3.9], [-2, -3.5]], 0.3) + f.band([[2, -3.6], [6, -3.9]], 0.3) + f.band([[-7.6, -2.6], [-6.5, -3.5]], 0.3), "wood-600", 0.9),
  ];
}

/** A small sailing ship on the anchor (its waterline), 20 wide and 18 tall at scale 1. */
function shipPart(at: PartPlace, rng: Rng): KitShape[] {
  const f = local(at);
  const bel = between(rng, 0.6, 1.4);
  const flag = between(rng, 3.8, 4.8);
  return [
    shape(f.band([[0, -4.8], [0, -17.6]], 0.7), "wood-800", 1, INKED),
    shape(f.poly([[0.5, -16.2], [7.2 + bel, -13.8], [6.8, -7.4], [0.5, -6.6]]), "white", 1, INKED),
    shape(f.poly([[-0.5, -15], [-5.5 - bel * 0.5, -12.8], [-5.4, -8], [-0.5, -7.4]]), "white", 1, INKED),
    shape(f.poly([[3, -7.4], [6.8, -7.4], [7.2 + bel, -13.8], [3.5, -12]]) + f.poly([[-3, -7.7], [-5.4, -8], [-5.9, -12.9], [-3, -11]]), "slate-400", 0.3),
    shape(f.poly([[0.2, -18], [flag, -17.1], [0.2, -16.2]]), "red-600", 1),
    shape(f.poly([[-9.6, -5], [9.6, -5], [7.4, 0], [-7, 0]]), "wood-700", 1, MAIN_INKED),
    shape(f.band([[-8.8, -3.6], [8.8, -3.6]], 0.45) + f.band([[-7.8, -1.7], [7, -1.7]], 0.4), "wood-800", 0.65),
    shape(f.band([[-9.2, -4.8], [9.2, -4.8]], 0.5), "white", 0.2),
    shape(f.circle(-3.8, -2.6, 0.5) + f.circle(0, -2.6, 0.5) + f.circle(3.8, -2.6, 0.5), "slate-900", 0.8),
  ].flat();
}

/** A wrapped gift box on the anchor, 4 wide at scale 1. */
function giftPart(at: PartPlace, rng: Rng): KitShape[] {
  const f = local(at);
  const tilt = between(rng, -0.3, 0.3);
  return [
    shape(eRect(f, -2, -3.3, 4, 3.3), "red-500", 1, MAIN_INKED),
    shape(eRect(f, -2.2, -4.2, 4.4, 1.1), "red-500", 1, MAIN_INKED),
    shape(eRect(f, 0.5, -3.3, 1.5, 3.3), "slate-900", 0.2),
    shape(eRect(f, -0.35, -4.2, 0.7, 4.2) + eRect(f, -2, -2.1, 1.65, 0.7) + eRect(f, 0.35, -2.1, 1.65, 0.7), "white", 1),
    shape(f.ellipse(-1 + tilt, -4.9, 0.95, 0.55) + f.ellipse(1 - tilt, -4.9, 0.95, 0.55), "white", 1, INKED),
    shape(f.circle(0, -4.6, 0.42), "white", 1, INKED),
  ];
}

/** A dripping honeypot on the anchor, 7 wide and 7 tall at scale 1. */
function honeypotPart(at: PartPlace, rng: Rng): KitShape[] {
  const f = local(at);
  const drip = between(rng, 3.4, 4.4);
  return [
    shape(`M${f.p(-1.9, -5.5)}Q${f.p(-3.7, -3)} ${f.p(-3.1, -1)}Q${f.p(-2.6, 0)} ${f.p(0, 0)}Q${f.p(2.6, 0)} ${f.p(3.1, -1)}Q${f.p(3.7, -3)} ${f.p(1.9, -5.5)}Z`, "brass-600", 1, MAIN_INKED),
    shape(`M${f.p(1.3, -5)}Q${f.p(3.4, -3.6)} ${f.p(3, -1)}Q${f.p(2.6, 0)} ${f.p(1.4, 0)}Q${f.p(2.2, -2.5)} ${f.p(1.3, -5)}Z`, "slate-900", 0.25),
    shape(f.poly([[0.8, -6.3], [3.5, -6.9], [3.7, -6.2], [1, -5.6]]) + f.circle(2.2, -7.2, 0.5), "brass-500", 1, INKED),
    shape(f.ellipse(0, -5.5, 2.5, 0.6), "brass-500", 1, INKED),
    shape(f.ellipse(0, -5.6, 2.1, 0.5), "orange-400", 1),
    shape(`M${f.p(-1.7, -5.4)}L${f.p(-0.6, -5.4)}L${f.p(-0.6, -drip)}Q${f.p(-0.6, -drip + 1.2)} ${f.p(-1.15, -drip + 1.2)}Q${f.p(-1.7, -drip + 1.2)} ${f.p(-1.7, -drip)}Z`, "orange-400", 1),
    shape(`M${f.p(0.6, -5.4)}L${f.p(1.5, -5.4)}L${f.p(1.5, -3)}Q${f.p(1.5, -2.2)} ${f.p(1.05, -2.2)}Q${f.p(0.6, -2.2)} ${f.p(0.6, -3)}Z`, "orange-400", 1),
    shape(f.band([[-2.6, -4.4], [-2.4, -1.8]], 0.4), "white", 0.35),
  ];
}

/** A university photocopier standing on the anchor, 20 wide and 14 tall at scale 1. */
function copierPart(at: PartPlace, rng: Rng): KitShape[] {
  const f = local(at);
  const on = whole(rng, 0, 1);
  return [
    shape(f.ellipse(0, 0, 10.4, 0.7), "slate-900", 0.2),
    shape(eRect(f, -9, -10.6, 18, 10.6), "slate-100", 1, MAIN_INKED),
    shape(f.poly([[-9.8, -10.6], [9.8, -10.6], [8.8, -13.6], [-8.8, -13.6]]), "slate-400", 1, INKED),
    shape(f.poly([[-8, -12.2], [8, -12.2], [8.4, -11.2], [-8.4, -11.2]]), "white", 0.4),
    shape(eRect(f, 2.8, -9.4, 5.6, 3.2), "slate-700", 1, INKED),
    shape(f.circle(4, -7.6, 0.4) + f.circle(5.6, -7.6, 0.4), "lime-400", on === 1 ? 1 : 0.85),
    shape(f.circle(7.2, -7.6, 0.4), "red-500", 1),
    shape(f.band([[3.4, -8.7], [7.8, -8.7]], 0.5), "lime-400", 0.7),
    shape(f.band([[9, -5], [10.2, -5], [10.2, -2.6]], 0.7) + f.circle(10.2, -2.3, 0.5), "slate-700", 1),
    shape(f.poly([[-7.5, -3.2], [-0.5, -3.2], [0.5, -0.6], [-8.5, -0.6]]), "slate-400", 1, INKED),
    shape(f.poly([[-6.4, -3.6], [-1.6, -3.6], [-0.9, -1.2], [-7.1, -1.2]]), "white", 1, INKED),
    shape(f.band([[-7, -3.8], [-1, -3.8]], 0.5), "slate-900", 0.7),
  ];
}

/** A spiked crimson mask centred on the anchor, 10 wide at scale 1. */
function maskPart(at: PartPlace, rng: Rng): KitShape[] {
  const f = local(at);
  const spikes: string[] = [];
  for (const a of [150, 180, 210, 240, 270, 300, 330, 0, 30]) {
    const tip = a * DEGREES;
    const l = (a - 7) * DEGREES;
    const r = (a + 7) * DEGREES;
    spikes.push(f.poly([[3.5 * Math.cos(l), 4.3 * Math.sin(l)], [5 * Math.cos(tip), 6.1 * Math.sin(tip)], [3.5 * Math.cos(r), 4.3 * Math.sin(r)]]));
  }
  const brow = between(rng, 0, 0.4);
  return [
    shape(spikes.join(""), "slate-100", 1, INKED),
    shape(`M${f.p(0, -4.5)}Q${f.p(3.9, -4.3)} ${f.p(3.7, 0)}Q${f.p(3.4, 3.2)} ${f.p(0, 5)}Q${f.p(-3.4, 3.2)} ${f.p(-3.7, 0)}Q${f.p(-3.9, -4.3)} ${f.p(0, -4.5)}Z`, "red-600", 1, MAIN_INKED),
    shape(f.poly([[-3.2, -2 - brow], [-0.5, -0.9], [-3.2, -3.3]]) + f.poly([[3.2, -2 - brow], [0.5, -0.9], [3.2, -3.3]]), "red-900", 0.6),
    shape(f.poly([[-3, -1.5], [-0.6, -0.5], [-1.2, 1], [-2.9, 0]]) + f.poly([[3, -1.5], [0.6, -0.5], [1.2, 1], [2.9, 0]]), "black", 1),
    shape(f.band([[0, 0.8], [0, 2.4]], 0.4) + f.band([[-1.3, 3.4], [0, 3.8], [1.3, 3.4]], 0.4), "red-900", 0.8),
    shape(f.poly([[-3.3, -3.4], [-1.2, -4], [-2.4, -2.5]]), "white", 0.5),
  ];
}

/** An empty frying pan lying on the anchor, 10 long at scale 1, its handle to the facing side. */
function panPart(at: PartPlace, rng: Rng): KitShape[] {
  const f = local(at);
  const hang = between(rng, -0.4, 0.4);
  return [
    shape(f.ellipse(-0.6, 0.9, 5.2, 1.2), "slate-900", 0.25),
    shape(f.band([[2, 0], [5, hang]], 1.1), "slate-900", 1, INKED),
    shape(f.band([[2.6, -0.3], [4.8, -0.2 + hang]], 0.3), "slate-400", 0.7),
    shape(f.ellipse(-1.3, 0, 3.8, 2.5), "slate-400", 1, INKED),
    shape(f.ellipse(-1.3, 0.1, 3.2, 2), "slate-900", 1, MAIN_INKED),
    shape(`M${f.p(-3.8, -0.3)}Q${f.p(-3, -1.5)} ${f.p(-1.4, -1.8)}Q${f.p(-2.8, -0.9)} ${f.p(-3.8, -0.3)}Z`, "white", 0.2),
  ];
}

/** A rope snare lying flat on the anchor, 8 wide at scale 1. */
function snarePart(at: PartPlace, rng: Rng): KitShape[] {
  const f = local(at);
  const lean = between(rng, -0.5, 0.5);
  return [
    shape(f.ellipse(0, -0.8, 4, 1.7), "slate-900", 0.18),
    shape(f.band([[3.4, -1.3], [3.8, -1.1], [3.6, -0.4]], 0.5), "wood-600", 1),
    shape(f.ellipse(0, -0.9, 3.7, 1.6) + f.ellipse(0, -0.9, 2.9, 1), "wood-600", 1, MAIN_INKED),
    shape(f.band([[-3, -1.9], [-2.7, -0.3]], 0.3) + f.band([[-1, -2.3], [-0.7, -0.6]], 0.3) + f.band([[1.4, -2.3], [1.4, -0.6]], 0.3) + f.band([[3, -1.5], [2.9, -0.2]], 0.3), "wood-700", 0.9),
    shape(f.band([[-1.2, -0.3], [1.2 + lean, -2.8]], 0.5), "wood-700", 1, INKED),
    shape(f.band([[3.4, -3.2], [3.4, -0.2]], 0.7) + f.circle(3.4, -3.3, 0.45), "wood-800", 1, INKED),
  ];
}

/** A tall rolling library ladder, leaning a little, its wheels on the anchor, 6 wide and 34 tall at scale 1. */
function ladderPart(at: PartPlace, rng: Rng): KitShape[] {
  const f = local(at);
  const lean = between(rng, 0.05, 0.1);
  const P = (dx: number, dy: number): Point => leant([dx, dy], lean);
  const poly = (pts: readonly Point[]): string => f.poly(pts.map(([x, y]) => P(x, y)));
  const rungs: string[] = [];
  for (let i = 0; i < 8; i += 1) {
    const y = -4.2 - i * 4;
    rungs.push(poly([[-2.3, y], [2.3, y], [2.3, y - 0.8], [-2.3, y - 0.8]]));
  }
  const wheel = (dx: number, r: number): string => f.circle(...P(dx, -r), r);
  return [
    shape(poly([[-3, -1.8], [-2.3, -1.8], [-2.3, -34], [-3, -34]]) + poly([[2.3, -1.8], [3, -1.8], [3, -34], [2.3, -34]]), "wood-600", 1, MAIN_INKED),
    shape(rungs.join(""), "wood-600", 1, MAIN_INKED),
    shape(poly([[-3.2, -34.4], [3.2, -34.4], [3.2, -33.6], [-3.2, -33.6]]), "slate-700", 1),
    shape(wheel(-2.65, 0.95) + wheel(2.65, 0.95), "slate-900", 1),
    shape(f.circle(...P(-2.65, -0.95), 0.35) + f.circle(...P(2.65, -0.95), 0.35), "slate-400", 1),
    shape(poly([[-2.95, -2.2], [-2.7, -2.2], [-2.7, -33], [-2.95, -33]]), "white", 0.22),
  ];
}

/** A giant war hammer centred on the anchor, 24 long at scale 1, turned by the stream, its head to the facing side. */
function hammerPart(at: PartPlace, rng: Rng): KitShape[] {
  const f = local(at);
  const ang = between(rng, -60, -25);
  const T = (dx: number, dy: number): Point => turned([dx, dy], ang);
  const poly = (pts: readonly Point[]): string => f.poly(pts.map(([x, y]) => T(x, y)));
  const wrap: string[] = [];
  for (let i = 0; i < 4; i += 1) wrap.push(poly([[-11.6 + i * 1.3, -0.7], [-11.2 + i * 1.3, -0.7], [-11.2 + i * 1.3, 0.7], [-11.6 + i * 1.3, 0.7]]));
  return [
    shape(poly([[-12, -0.65], [6, -0.65], [6, 0.65], [-12, 0.65]]), "wood-700", 1, INKED),
    shape(wrap.join(""), "slate-900", 0.45),
    shape(poly([[5.5, -3.9], [11.7, -3.9], [11.7, 3.9], [5.5, 3.9]]), "slate-400", 1, MAIN_INKED),
    shape(poly([[5.5, -3.9], [11.7, -3.9], [11.7, -2.6], [5.5, -2.6]]), "white", 0.5),
    shape(poly([[5.5, 2.5], [11.7, 2.5], [11.7, 3.9], [5.5, 3.9]]), "slate-900", 0.3),
    shape(poly([[5.5, -3.9], [6.4, -3.9], [6.4, 3.9], [5.5, 3.9]]), "slate-700", 0.7),
    shape(poly([[-12.3, -1], [-11.5, -1], [-11.5, 1], [-12.3, 1]]), "slate-700", 1),
  ];
}

/** A hologram panel centred on the anchor, 30 wide and 20 tall at scale 1, its projector base below it. */
function hologramPart(at: PartPlace, rng: Rng): KitShape[] {
  const f = local(at);
  const tile = (x: number, y: number): string => eRect(f, x, y, 13, 8.2) + eRect(f, x + 0.5, y + 0.5, 12, 7.2);
  const lines: string[] = [];
  const start = between(rng, 0, 1);
  for (let i = 0; i < 9; i += 1) lines.push(eRect(f, -14.5, -9 + i * 2 + start, 29, 0.3));
  return [
    shape(f.poly([[-3, 11], [3, 11], [14, 10], [-14, 10]]), "cyan-400", 0.12),
    shape(eRect(f, -15, -10, 30, 20), "cyan-400", 0.2, MAIN),
    shape(tile(-14, -9.4) + tile(1, -9.4) + tile(-14, 1.2) + tile(1, 1.2), "cyan-400", 0.85),
    shape(lines.join(""), "white", 0.12),
    shape(eRect(f, -15, -10, 30, 20) + eRect(f, -14.4, -9.4, 28.8, 18.8), "cyan-400", 0.35),
    shape(f.poly([[-5.5, 12.4], [5.5, 12.4], [3.6, 10.6], [-3.6, 10.6]]), "slate-700", 1, INKED),
    shape(f.band([[-2.5, 11.2], [2.5, 11.2]], 0.4), "cyan-400", 0.9),
  ];
}

/** A telescope on a tripod standing on the anchor, 14 tall at scale 1, its tube aimed up to the facing side. */
function telescopePart(at: PartPlace, rng: Rng): KitShape[] {
  const f = local(at);
  const ang = between(rng, -58, -46);
  const T = (dx: number): Point => {
    const [x, y] = turned([dx, 0], ang);
    return [x, y - 6.2];
  };
  return [
    shape(f.band([[0, -6], [3.4, 0]], 0.7), "slate-700", 1, INKED),
    shape(f.band([[0, -6], [-3.8, 0]], 0.8), "slate-700", 1, INKED),
    shape(f.band([[0, -6], [-0.3, -0.1]], 0.6), "slate-900", 1),
    shape(f.band([T(-0.6), T(6.8)], 1.9), "slate-400", 1, MAIN_INKED),
    shape(f.band([T(6.8), T(8.4)], 2.5), "slate-700", 1, INKED),
    shape(f.band([T(-2.4), T(-0.6)], 1), "slate-900", 1),
    shape(f.band([T(0.5), T(6)].map(([x, y]) => [x - 0.2, y - 0.5] as const), 0.4), "white", 0.5),
    shape(f.circle(0, -6.2, 0.8), "slate-700", 1, INKED),
  ];
}

/** A rocket-powered cart standing on the anchor, 20 wide and 10 tall at scale 1, facing the side it faces, its flame behind. */
function rocketCartPart(at: PartPlace, rng: Rng): KitShape[] {
  const f = local(at);
  const fl = between(rng, 0, 1.4);
  return [
    shape(f.poly([[-7.3, -5.4], [-8.5 - fl * 0.3, -4.4], [-10 - fl, -3.7], [-8.4 - fl * 0.2, -3], [-7.3, -2.3], [-7.3, -3.8]]), "orange-400", 1),
    shape(f.poly([[-7.3, -4.7], [-8.4, -4.2], [-9 - fl * 0.6, -3.7], [-8.2, -3.2], [-7.3, -3]]), "white", 0.9),
    shape(f.poly([[-3.4, -6.4], [-3.4, -2.2], [8.6, -2.2], [8.6, -3.6], [6.8, -6.4]]), "red-600", 1, MAIN_INKED),
    shape(f.poly([[-3.4, -6.4], [6.8, -6.4], [7.2, -5.6], [-3.4, -5.6]]), "white", 0.3),
    shape(f.band([[-7.2, -3.9], [-3.2, -3.9]], 2.8), "slate-400", 1, INKED),
    shape(f.poly([[-7.2, -5.6], [-7.8, -6.4], [-7.8, -1.4], [-7.2, -2.2]]), "slate-700", 1),
    shape(f.band([[2.8, -6.3], [2.8, -8.6], [4.8, -8.6]], 0.5), "slate-900", 1),
    shape(f.circle(-0.8, -2, 2) + f.circle(6, -2, 2), "slate-900", 1, INKED),
    shape(f.circle(-0.8, -2, 0.8) + f.circle(6, -2, 0.8), "slate-400", 1),
  ];
}

/** A bronze bell hanging from the anchor, 8 wide and 8 tall at scale 1, swung by the stream. */
function bellPart(at: PartPlace, rng: Rng): KitShape[] {
  const f = local(at);
  const ang = between(rng, -14, 14);
  const T = (dx: number, dy: number): Point => turned([dx, dy], ang);
  const q = (dx: number, dy: number): string => f.p(...T(dx, dy));
  const poly = (pts: readonly Point[]): string => f.poly(pts.map(([x, y]) => T(x, y)));
  return [
    shape(f.band([T(0, 0), T(0, 1.8)], 0.8), "wood-700", 1, INKED),
    shape(`M${q(-1.4, 1.5)}L${q(1.4, 1.5)}Q${q(1.8, 4.6)} ${q(4, 5.9)}L${q(-4, 5.9)}Q${q(-1.8, 4.6)} ${q(-1.4, 1.5)}Z`, "brass-600", 1, MAIN_INKED),
    shape(poly([[-4.1, 5.7], [4.1, 5.7], [4, 6.9], [-4, 6.9]]), "brass-500", 1, INKED),
    shape(`M${q(-1, 2)}Q${q(-1.3, 4.6)} ${q(-3.2, 5.6)}L${q(-2.6, 5.6)}Q${q(-0.9, 4.4)} ${q(-0.5, 2)}Z`, "white", 0.4),
    shape(`M${q(1.1, 2)}Q${q(1.5, 4.6)} ${q(3.8, 5.7)}L${q(2.4, 5.7)}Q${q(1.1, 4.4)} ${q(0.8, 2)}Z`, "slate-900", 0.28),
    shape(f.circle(...T(0.3, 7.6), 0.85), "brass-800", 1, INKED),
  ];
}

/** Three soft rice cakes stacked on a white plate on the anchor, 12 wide at scale 1, a red sauce dab on top. */
function riceCakePart(at: PartPlace, rng: Rng): KitShape[] {
  const f = local(at);
  const dab = between(rng, -0.6, 0.6);
  return [
    shape(f.ellipse(0, -0.6, 6, 1.1), "white", 1, INKED),
    shape(f.ellipse(0, -0.7, 4, 0.6), "slate-400", 0.3),
    shape(f.ellipse(0, -2.3, 3.5, 1.5), "slate-100", 1, MAIN_INKED),
    shape(f.ellipse(0.2, -1.9, 3.2, 0.9), "slate-400", 0.3),
    shape(f.ellipse(0, -4, 3.2, 1.45), "slate-100", 1, MAIN_INKED),
    shape(f.ellipse(0.2, -3.6, 2.9, 0.85), "slate-400", 0.3),
    shape(f.ellipse(0, -5.6, 2.9, 1.4), "slate-100", 1, MAIN_INKED),
    shape(f.ellipse(-1.2, -6, 1, 0.4) + f.ellipse(-1.8, -4.4, 0.8, 0.3) + f.ellipse(-2, -2.7, 0.9, 0.3), "white", 0.7),
    shape(`M${f.p(-1 + dab, -6.5)}Q${f.p(0 + dab, -7.3)} ${f.p(1.3 + dab, -6.5)}Q${f.p(1.1 + dab, -5.6)} ${f.p(0.6 + dab, -5.3)}Q${f.p(0.3 + dab, -5.9)} ${f.p(-1 + dab, -6.5)}Z`, "red-600", 1),
  ];
}

/** A sealed envelope centred on the anchor, 7 wide and 5 tall at scale 1. */
function envelopePart(at: PartPlace, rng: Rng): KitShape[] {
  const f = local(at);
  const tilt = between(rng, -0.25, 0.25);
  return [
    shape(eRect(f, -3.5, -2.5, 7, 5), "slate-100", 1, MAIN_INKED),
    shape(f.poly([[-3.5, -2.5], [3.5, -2.5], [0, 0.4 + tilt]]), "slate-400", 0.25),
    shape(f.band([[-3.5, -2.5], [0, 0.4 + tilt], [3.5, -2.5]], 0.25), "slate-400", 1),
    shape(f.band([[-3.5, 2.5], [-1, 0.3]], 0.25) + f.band([[3.5, 2.5], [1, 0.3]], 0.25), "slate-400", 1),
    shape(f.circle(0, 0.4 + tilt, 0.95), "red-600", 1, INKED),
    shape(f.circle(-0.2, 0.2 + tilt, 0.45), "red-300", 0.6),
  ];
}

/** A cracked alchemical crucible on the anchor, 10 wide and 9 tall at scale 1, molten and glowing. */
function cruciblePart(at: PartPlace, rng: Rng): KitShape[] {
  const f = local(at);
  const c = between(rng, -0.6, 0.6);
  const crack: Point[] = [[-1, -6.4], [-1.9 + c, -4.6], [-0.7, -3.2], [-1.6, -1.2]];
  return [
    shape(f.ellipse(0, -7.4, 5.4, 1.8), "red-300", 0.22),
    shape(`M${f.p(-4.6, -6.7)}Q${f.p(-5.1, -1)} ${f.p(-2.6, 0)}L${f.p(2.6, 0)}Q${f.p(5.1, -1)} ${f.p(4.6, -6.7)}Z`, "slate-700", 1, MAIN_INKED),
    shape(`M${f.p(-4.3, -5.8)}Q${f.p(-4.7, -1.5)} ${f.p(-3.4, -0.6)}L${f.p(-2.7, -0.6)}Q${f.p(-3.7, -2)} ${f.p(-3.5, -5.8)}Z`, "slate-400", 0.5),
    shape(`M${f.p(3.6, -5.8)}Q${f.p(4.4, -1.5)} ${f.p(2.6, -0.2)}L${f.p(4, -0.8)}Q${f.p(4.8, -2)} ${f.p(4.6, -5.8)}Z`, "slate-900", 0.3),
    shape(f.ellipse(0, -6.7, 5, 1.1), "slate-400", 1, INKED),
    shape(f.ellipse(0, -6.75, 4.2, 0.78), "red-300", 1),
    shape(f.ellipse(-0.4, -6.8, 2.2, 0.4), "white", 0.9),
    shape(f.band(crack, 0.8), "red-300", 0.9),
    shape(f.band(crack, 0.3), "white", 0.95),
  ];
}

/** A silver crown centred on the anchor, 6 wide at scale 1. */
function crownPart(at: PartPlace, rng: Rng): KitShape[] {
  const f = local(at);
  const hi = between(rng, 0, 0.4);
  return [
    shape(f.poly([[-3, 0.3], [-3, -2.1], [-1.5, -0.7], [0, -2.5 - hi], [1.5, -0.7], [3, -2.1], [3, 0.3]]), "slate-100", 1, MAIN_INKED),
    shape(eRect(f, -3, 0.2, 6, 1.7), "slate-100", 1, MAIN_INKED),
    shape(eRect(f, -3, 1.2, 6, 0.7), "slate-400", 0.9),
    shape(f.poly([[-3, 0.3], [-2.4, -1.2], [-1.9, 0.3]]) + f.poly([[-0.9, 0.3], [0, -1.7], [0.2, 0.3]]), "white", 0.5),
    shape(f.circle(0, 0.95, 0.5), "red-500", 1),
    shape(f.circle(-1.8, 0.95, 0.38) + f.circle(1.8, 0.95, 0.38), "sky-400", 1),
    shape(f.circle(-3, -2.2, 0.3) + f.circle(3, -2.2, 0.3) + f.circle(0, -2.7 - hi, 0.32), "red-500", 1),
  ];
}

/** A glass vial on the anchor, 3 wide and 6 tall at scale 1, its liquid the main colour. */
function vialPart(at: PartPlace, rng: Rng): KitShape[] {
  const f = local(at);
  const fill = between(rng, -0.4, 0.5);
  return [
    shape(`M${f.p(-0.6, -5.2)}L${f.p(-0.6, -3)}Q${f.p(-1.6, -2.4)} ${f.p(-1.6, -1.4)}Q${f.p(-1.6, 0)} ${f.p(0, 0)}Q${f.p(1.6, 0)} ${f.p(1.6, -1.4)}Q${f.p(1.6, -2.4)} ${f.p(0.6, -3)}L${f.p(0.6, -5.2)}Z`, "sky-300", 0.45, INKED),
    shape(`M${f.p(-1.45, -1.9 + fill)}L${f.p(1.45, -1.9 + fill)}Q${f.p(1.55, -0.3)} ${f.p(0, -0.15)}Q${f.p(-1.55, -0.3)} ${f.p(-1.45, -1.9 + fill)}Z`, "lime-400", 1, MAIN),
    shape(f.poly([[-0.85, -6.2], [0.85, -6.2], [0.7, -5.1], [-0.7, -5.1]]), "brass-500", 1, INKED),
    shape(f.band([[-1.05, -2.3], [-1.1, -0.9]], 0.35) + f.band([[-0.35, -5], [-0.35, -3.6]], 0.2), "white", 0.7),
    shape(f.circle(0.4, -1.1 + fill, 0.2), "white", 0.5),
  ];
}

/** An American football centred on the anchor, 5 long at scale 1, turned by the stream. */
function ballPart(at: PartPlace, rng: Rng): KitShape[] {
  const f = local(at);
  const ang = between(rng, -45, 45);
  const T = (dx: number, dy: number): Point => turned([dx, dy], ang);
  const q = (dx: number, dy: number): string => f.p(...T(dx, dy));
  const poly = (pts: readonly Point[]): string => f.poly(pts.map(([x, y]) => T(x, y)));
  return [
    shape(`M${q(-2.5, 0)}Q${q(0, -2.5)} ${q(2.5, 0)}Q${q(0, 2.5)} ${q(-2.5, 0)}Z`, "wood-700", 1, MAIN_INKED),
    shape(`M${q(-2, -0.3)}Q${q(0, -1.5)} ${q(2, -0.3)}Q${q(0, -0.8)} ${q(-2, -0.3)}Z`, "white", 0.28),
    shape(`M${q(-2, 0.4)}Q${q(0, 1.5)} ${q(2, 0.4)}Q${q(0, 1.1)} ${q(-2, 0.4)}Z`, "slate-900", 0.3),
    shape(poly([[-1.7, -0.6], [-1.45, -0.6], [-1.45, 0.6], [-1.7, 0.6]]) + poly([[1.45, -0.6], [1.7, -0.6], [1.7, 0.6], [1.45, 0.6]]), "white", 0.95),
    shape(poly([[-0.9, -0.08], [0.9, -0.08], [0.9, 0.08], [-0.9, 0.08]]), "white", 1),
    shape(poly([[-0.65, -0.5], [-0.5, -0.5], [-0.5, 0.5], [-0.65, 0.5]]) + poly([[-0.08, -0.55], [0.08, -0.55], [0.08, 0.55], [-0.08, 0.55]]) + poly([[0.5, -0.5], [0.65, -0.5], [0.65, 0.5], [0.5, 0.5]]), "white", 1),
  ];
}

/** A floor hatch sprung open on the anchor, 20 wide at scale 1: its lid swung up behind, a green glow below. */
function hatchPart(at: PartPlace, rng: Rng): KitShape[] {
  const f = local(at);
  const lean = between(rng, -1, 1);
  return [
    shape(f.poly([[-7.4, -4.4], [7.4, -4.4], [8.2 + lean, -13.4], [-8.2 + lean, -13.4]]), "slate-400", 1, MAIN_INKED),
    shape(f.poly([[-6, -6], [6, -6], [6.5 + lean, -12], [-6.5 + lean, -12]]), "slate-900", 0.22),
    shape(f.poly([[-7.4, -4.4], [7.4, -4.4], [7.6 + lean, -6.6], [-7.6 + lean, -6.6]]), "slate-900", 0.15),
    shape(f.poly([[-7.5 + lean, -13.4], [-4 + lean, -13.4], [-4.4, -4.6], [-7.2, -4.6]]), "white", 0.22),
    shape(eRect(f, -5.8, -5.1, 1.8, 1) + eRect(f, 4, -5.1, 1.8, 1), "slate-700", 1),
    shape(f.ellipse(0, -1.4, 9.8, 3.5), "slate-700", 1, INKED),
    shape(f.ellipse(0, -1.4, 8.2, 2.8), "black", 1),
    shape(f.ellipse(0, -1.3, 6.8, 2.1), "lime-400", 0.35),
    shape(f.poly([[-6.5, -1.5], [6.5, -1.5], [9, -12], [-9, -12]]), "lime-400", 0.1),
  ];
}

/** A robotic clamp arm reaching up from the anchor toward the side it faces, 22 long at scale 1. */
function clampPart(at: PartPlace, rng: Rng): KitShape[] {
  const f = local(at);
  const bend = between(rng, -1, 1.5);
  const wx = 8.5 + bend;
  return [
    shape(f.poly([[-3.2, 0], [3.2, 0], [2.2, -2.2], [-2.2, -2.2]]), "slate-700", 1, INKED),
    shape(f.band([[0, -3], [4, -9.5]], 2.4), "slate-400", 1, MAIN_INKED),
    shape(f.band([[4, -9.5], [wx, -16]], 2.2), "slate-400", 1, MAIN_INKED),
    shape(f.band([[-0.9, -3.4], [3.1, -9.8]], 0.4) + f.band([[3.1, -9.8], [wx - 0.9, -16.3]], 0.4), "white", 0.4),
    shape(f.circle(0, -3, 1.5) + f.circle(4, -9.5, 1.6) + f.circle(wx, -16, 1.5), "slate-700", 1, INKED),
    shape(f.poly([[wx - 2.2, -16.8], [wx + 2.2, -16.8], [wx + 1.8, -18.4], [wx - 1.8, -18.4]]), "slate-700", 1, INKED),
    shape(f.poly([[wx - 2, -18.2], [wx - 0.7, -18.2], [wx - 0.3, -21], [wx - 0.9, -22], [wx - 2.2, -20.4]]), "slate-400", 1, MAIN_INKED),
    shape(f.poly([[wx + 0.7, -18.2], [wx + 2, -18.2], [wx + 2.2, -20.4], [wx + 0.9, -22], [wx + 0.3, -21]]), "slate-400", 1, MAIN_INKED),
  ];
}

/** A small velvet pocket bursting open, centred on the anchor, 8 wide at scale 1, white light bursting from its mouth. */
function pocketPart(at: PartPlace, rng: Rng): KitShape[] {
  const f = local(at);
  const reach = between(rng, 0, 1);
  const rays: string[] = [];
  const tips = [-4.2, -2.2, 0, 2.2, 4.2];
  for (let i = 0; i < 5; i += 1) {
    const x0 = -2 + i;
    rays.push(f.poly([[x0 - 0.4, -0.6], [x0 + 0.4, -0.6], [tips[i] ?? 0, -5.8 - (i % 2) * 1.2 - reach]]));
  }
  return [
    shape(f.circle(0, -1.5, 4.2), "white", 0.16),
    shape(f.circle(0, -1.5, 2.8), "white", 0.25),
    shape(rays.join(""), "white", 0.55),
    shape(`M${f.p(-3.4, -0.4)}Q${f.p(-4.3, 3.8)} ${f.p(0, 4)}Q${f.p(4.3, 3.8)} ${f.p(3.4, -0.4)}Q${f.p(0, 0.9)} ${f.p(-3.4, -0.4)}Z`, "violet-700", 1, MAIN_INKED),
    shape(f.ellipse(0, -0.6, 3.3, 0.8), "white", 1),
    shape(`M${f.p(-3.2, 0.5)}Q${f.p(0, 1.6)} ${f.p(3.2, 0.5)}`.concat(`L${f.p(3.2, 0.9)}Q${f.p(0, 2)} ${f.p(-3.2, 0.9)}Z`), "brass-500", 1),
    shape(`M${f.p(-2.8, 0.9)}Q${f.p(-3.6, 3)} ${f.p(-2.2, 3.6)}Q${f.p(-2.9, 2.6)} ${f.p(-2.4, 0.9)}Z`, "white", 0.25),
    shape(`M${f.p(2.4, 1.2)}Q${f.p(3.8, 3.2)} ${f.p(0.4, 3.9)}Q${f.p(3.2, 3)} ${f.p(2, 1.2)}Z`, "slate-900", 0.25),
    shape(sparkle(f, 0, -3.4 - reach, 1.7), "white", 0.95),
  ];
}

/** A velvet sack on the anchor, 10 wide and 12 tall at scale 1, silver coins peeking from its neck. */
function sackPart(at: PartPlace, rng: Rng): KitShape[] {
  const f = local(at);
  const flop = between(rng, -0.5, 0.5);
  return [
    shape(`M${f.p(-1.6, -8)}Q${f.p(-5.2, -4)} ${f.p(-4.8, -1)}Q${f.p(-4.5, 0)} ${f.p(0, 0)}Q${f.p(4.5, 0)} ${f.p(4.8, -1)}Q${f.p(5.2, -4)} ${f.p(1.6, -8)}Z`, "red-900", 1, MAIN_INKED),
    shape(f.poly([[-1.6, -8], [1.6, -8], [2.8 + flop, -11.2], [1.2, -10.2], [0, -11.6], [-1.2, -10.2], [-2.8 + flop, -11.2]]), "red-900", 1, MAIN_INKED),
    shape(f.circle(-1.5, -10.9, 0.9) + f.circle(0.5, -11.6, 0.9) + f.circle(2.2, -10.7, 0.8), "slate-100", 1, INKED),
    shape(f.band([[-1.8, -8], [1.8, -8]], 0.8) + f.circle(2.2, -7.6, 0.45), "brass-500", 1),
    shape(`M${f.p(-3.6, -5.6)}Q${f.p(-4.4, -3)} ${f.p(-3.2, -1)}Q${f.p(-4, -3)} ${f.p(-3.2, -5.6)}Z`, "white", 0.25),
    shape(`M${f.p(2.6, -6)}Q${f.p(5, -3)} ${f.p(3.6, -0.4)}L${f.p(4.8, -1)}Q${f.p(5.2, -4)} ${f.p(2.6, -6)}Z`, "slate-900", 0.3),
    shape(f.band([[-0.8, -7.6], [-1.6, -4], [-1.3, -1.2]], 0.3) + f.band([[0.9, -7.6], [1.4, -4]], 0.3), "slate-900", 0.22),
  ];
}

/** A Legendary's towering chaos wheel standing on the anchor, 34 wide and 40 tall at scale 1: cabinet, coloured wheel, reels window, lever. */
function chaosWheelPart(at: PartPlace, rng: Rng): KitShape[] {
  const f = local(at);
  const cx = 0;
  const cy = -23;
  const turn = between(rng, 0, 36);
  const R = 10.8;
  const sector = (k: number): string => {
    const a0 = (turn + k * 36) * DEGREES;
    const a1 = (turn + k * 36 + 36) * DEGREES;
    return `M${f.p(cx, cy)}L${f.p(cx + R * Math.cos(a0), cy + R * Math.sin(a0))}A${fmt(f.r(R))} ${fmt(f.r(R))} 0 0 ${f.sweep(1)} ${f.p(cx + R * Math.cos(a1), cy + R * Math.sin(a1))}Z`;
  };
  const dots: string[] = [];
  for (let k = 0; k < 10; k += 1) {
    const a = (turn + k * 36) * DEGREES;
    dots.push(f.circle(cx + 11.7 * Math.cos(a), cy + 11.7 * Math.sin(a), 0.5));
  }
  const reels = [-5, -1.3, 2.4].map((x) => eRect(f, x, -9, 2.6, 4.4)).join("");
  const colours: PaletteName[] = ["red-500", "sky-400", "lime-400", "orange-400", "violet-300"];
  return [
    shape(`M${f.p(-14, 0)}L${f.p(-14, -30)}Q${f.p(-14, -38)} ${f.p(0, -40)}Q${f.p(14, -38)} ${f.p(14, -30)}L${f.p(14, 0)}Z`, "violet-700", 1, MAIN_INKED),
    shape(f.band([[14, -17], [15.6, -25.5]], 1), "slate-400", 1, INKED),
    shape(f.circle(15.6, -27, 1.5), "red-500", 1, INKED),
    shape(f.circle(cx, cy, 12.6), "slate-900", 1),
    ...colours.map((c, i) => shape(sector(i) + sector(i + 5), c, 1)),
    shape(dots.join("") + f.circle(cx, cy, 1.6) + f.poly([[-1.2, -36.3], [1.2, -36.3], [0, -33.9]]), "white", 0.95),
    shape(eRect(f, -8.5, -10, 17, 6.4), "slate-900", 1, INKED),
    shape(reels, "violet-300", 0.5),
  ];
}

/** A giant stone golem's fist centred on the anchor, 10 wide at scale 1, punching toward the side it faces. */
function fistPart(at: PartPlace, rng: Rng): KitShape[] {
  const f = local(at);
  const j = between(rng, -0.3, 0.3);
  return [
    shape(f.poly([[-5, -3.2], [-1, -4.2], [3.5, -4.4], [5, -3], [5, 3], [3.5, 4.2], [-1, 4], [-5, 3.2]]), "stone-500", 1, MAIN_INKED),
    shape(f.poly([[-5, -3.2], [-1, -4.2], [3.5, -4.4], [5, -3], [3.5, -3.2], [-1, -3], [-5, -2.1]]), "white", 0.3),
    shape(f.poly([[-5, 2.4], [-1, 2.8], [3.5, 3], [5, 3], [3.5, 4.2], [-1, 4], [-5, 3.2]]), "slate-900", 0.25),
    shape(f.band([[1.6, -2.1 + j], [5, -2]], 0.4) + f.band([[1.6, -0.3], [5, -0.2]], 0.4) + f.band([[1.6, 1.6 - j], [5, 1.6]], 0.4), "slate-900", 0.4),
    shape(f.band([[1.7, -3.6], [1.7, 3.6]], 0.4), "slate-900", 0.3),
    shape(f.band([[-3.6, -3], [-3.6, 3]], 0.5) + f.band([[-0.8, -4], [-0.2, -2.6], [-1.3, -1.3], [-0.7, 0.3], [-1.6, 1.6]], 0.3), "slate-900", 0.55),
    shape(f.circle(4, -3.2, 0.5) + f.circle(4, -1.1, 0.5) + f.circle(4, 0.9, 0.5) + f.circle(4, 2.9, 0.5), "white", 0.3),
  ];
}

/** A healing fountain standing on the anchor, 16 wide and 14 tall at scale 1, in a green glow. */
function fountainPart(at: PartPlace, rng: Rng): KitShape[] {
  const f = local(at);
  const k = between(rng, 4.4, 5.4);
  const jet = (sign: number): Point[] => {
    const pts: Point[] = [];
    for (let i = 0; i <= 6; i += 1) {
      const t = i / 6;
      pts.push([sign * 6.5 * t, -9.2 + 4.8 * t - k * 4 * t * (1 - t)]);
    }
    return pts;
  };
  return [
    shape(f.circle(0, -6, 9), "green-400", 0.12),
    shape(f.circle(0, -6, 6), "green-400", 0.15),
    shape(`M${f.p(-8, -3.5)}Q${f.p(-7, 0)} ${f.p(-4, 0)}L${f.p(4, 0)}Q${f.p(7, 0)} ${f.p(8, -3.5)}Z`, "slate-100", 1, MAIN_INKED),
    shape(f.poly([[-1.5, -3.4], [1.5, -3.4], [0.8, -8], [-0.8, -8]]), "slate-100", 1, MAIN_INKED),
    shape(f.ellipse(0, -3.6, 8, 1.5), "slate-100", 1, MAIN_INKED),
    shape(f.ellipse(0, -3.6, 6.8, 1.1), "sky-300", 0.85),
    shape(f.poly([[-2, -8], [2, -8], [1.2, -9.2], [-1.2, -9.2]]), "slate-100", 1, INKED),
    shape(f.band(jet(-1), 0.7), "sky-300", 0.8),
    shape(f.band(jet(1), 0.7), "sky-300", 0.8),
    shape(f.band([[0, -9.2], [0, -13]], 0.8), "sky-300", 0.8),
    shape(f.poly([[3.5, -0.4], [7.2, -0.6], [7.6, -2.4], [4.4, -2]]), "slate-900", 0.18),
  ];
}

/** A crystal chandelier hanging from the anchor, 16 wide and 10 tall at scale 1. */
function chandelierPart(at: PartPlace, rng: Rng): KitShape[] {
  const f = local(at);
  const sway = between(rng, -0.3, 0.3);
  const drops: string[] = [];
  for (const x of [-7, -5, -2.5, 0, 2.5, 5, 7]) {
    const y0 = 3.3 + 3 * (x / 7.5) ** 2;
    const l = x === 0 ? 6.2 : 2.6 + sway * (x > 0 ? 1 : -1);
    drops.push(f.poly([[x, y0], [x + 0.65, y0 + l * 0.4], [x, y0 + l], [x - 0.65, y0 + l * 0.4]]));
  }
  return [
    shape(f.circle(0, 3, 6), "white", 0.1),
    shape(f.band([[0, 0], [0, 2]], 0.4), "slate-400", 1),
    shape(f.band([[-7.8, 6.3], [-4.6, 4.2], [0, 3.2], [4.6, 4.2], [7.8, 6.3]], 0.7), "slate-400", 1, INKED),
    shape(f.circle(0, 3, 1), "slate-400", 1, INKED),
    shape(drops.join(""), "violet-300", 1, MAIN_INKED),
    shape(f.circle(-7.6, 5.4, 0.5) + f.circle(-3.8, 3.5, 0.5) + f.circle(3.8, 3.5, 0.5) + f.circle(7.6, 5.4, 0.5) + f.circle(0, 1.6, 0.5), "white", 1),
    shape(f.band([[-0.3, 4.4], [-0.3, 7.6]], 0.25) + f.band([[-2.8, 4.1], [-2.8, 5.4]], 0.2) + f.band([[2.2, 4.1], [2.2, 5.4]], 0.2), "white", 0.6),
  ];
}

/** A Mythic's huge gloved hand reaching down from above, wrist at the anchor, 30 tall at scale 1, with puppet strings running 30 more below. */
function puppetHandPart(at: PartPlace, rng: Rng): KitShape[] {
  const f = local(at);
  const tilt = between(rng, -3.6, -2.4);
  const sway = between(rng, -1, 1);
  const fingers: Point[][] = [
    [[-6.5, 8], [-10, 11], [-13, 14]],
    [[-4.6, 12], [-5.2 + tilt * 0.4, 20], [-5 + tilt, 28.5]],
    [[-1.4, 12], [-1.4, 21], [-1.5, 30]],
    [[1.8, 12], [2.4, 21], [3.2, 28]],
    [[5, 12], [6, 18], [7.2, 24]],
  ];
  const tips = fingers.map((p) => p[p.length - 1] ?? [0, 0]);
  return [
    shape(f.poly([[-7, -7], [7, -7], [6.6, 0], [-6.6, 0]]), "slate-100", 1, INKED),
    shape(f.poly([[-7.2, -2], [7.2, -2], [7.1, -0.9], [-7.1, -0.9]]), "slate-400", 0.6),
    shape(f.poly([[-6.5, 0], [6.5, 0], [7.2, 13], [-7.2, 13]]), "slate-900", 1, MAIN_INKED),
    shape(fingers.map((p) => f.band(p, 2.7)).join(""), "slate-900", 1, MAIN_INKED),
    shape(f.band([[-6.6, 1], [-7.1, 12.5]], 0.8) + f.band([[-5.6, 13], [-6.2 + tilt * 0.4, 20], [-6 + tilt, 28.5]], 0.5) + f.band([[-2.4, 13], [-2.5, 30]], 0.5) + f.band([[0.9, 13], [1.4, 21], [2.2, 28]], 0.5) + f.band([[4.1, 13], [5.1, 18], [6.3, 24]], 0.5), "slate-700", 0.9),
    shape(tips.map(([x, y]) => f.band([[x, y], [x + sway * 0.4, y + 15], [x + sway, y + 30]], 0.28)).join(""), "slate-100", 0.5),
    shape(f.circle(-4, -3.8, 0.6) + f.circle(4, -3.8, 0.6), "slate-900", 0.6),
  ];
}

/** An anvil on a stump, standing on the anchor, 14 wide and 8 tall at scale 1, its horn to the facing side's back. */
function anvilPart(at: PartPlace, rng: Rng): KitShape[] {
  const f = local(at);
  const D = 1.2;
  const P = (pts: readonly Point[]): string => f.poly(pts.map(([x, y]) => [x, y - D] as const));
  const g = between(rng, -0.4, 0.4);
  return [
    shape(f.poly([[-5.4, 0], [5.4, 0], [4.8, -2.2], [-4.8, -2.2]]), "wood-700", 1, INKED),
    shape(f.band([[-3.6, -0.4], [-3.4 + g, -1.9]], 0.3) + f.band([[1.2, -0.3], [1.4 + g, -1.9]], 0.3) + f.band([[3.8, -0.5], [3.7, -1.8]], 0.3), "wood-800", 0.6),
    shape(P([[-7, -5.4], [-3.2, -6.6], [6.6, -6.6], [6.6, -5.2], [4.8, -4.6], [2.6, -3.6], [2.8, -2.4], [4.4, -1.8], [4.4, -1], [-4.4, -1], [-4.4, -1.8], [-2.6, -2.4], [-2.6, -3.6], [-3.4, -4.4], [-5.2, -4.6]]), "slate-700", 1, MAIN_INKED),
    shape(P([[-3.2, -6.6], [6.6, -6.6], [6.6, -5.8], [-3.8, -5.8]]) + P([[-7, -5.4], [-3.2, -6.6], [-3.4, -5.8], [-6.2, -5.4]]), "slate-400", 1),
    shape(P([[-2.6, -2.4], [2.8, -2.4], [4.4, -1.8], [4.4, -1], [-4.4, -1], [-4.4, -1.8]]), "slate-900", 0.3),
    shape(P([[-1.5, -6.3], [1.5, -6.3], [1.3, -6.1], [-1.7, -6.1]]), "white", 0.6),
  ];
}

/** Two glowing rails running from the box's left and right edges in to the anchor at its height (absolute box coordinates), in slight perspective. */
function railsPart(at: PartPlace, rng: Rng): KitShape[] {
  const drop = at.s * between(rng, 5, 7);
  const wide = at.s * 3.2;
  const narrow = at.s * 1.2;
  const lerp = (a: number, b: number, t: number): number => a + (b - a) * t;
  const sides = [
    { x0: EDGE, x1: at.x - 0.8 * at.s },
    { x0: FAR_EDGE, x1: at.x + 0.8 * at.s },
  ];
  const rail = (s: { x0: number; x1: number }, grow: number): string => {
    const y0 = at.y + drop;
    return `M${pt(s.x0, y0 - (wide * grow) / 2)}L${pt(s.x1, at.y - (narrow * grow) / 2)}L${pt(s.x1, at.y + (narrow * grow) / 2)}L${pt(s.x0, y0 + (wide * grow) / 2)}Z`;
  };
  const ties = (s: { x0: number; x1: number }): string => {
    let d = "";
    for (let i = 0; i < 9; i += 1) {
      const t = (i + 0.5) / 9;
      const cx = lerp(s.x0, s.x1, t);
      const cy = lerp(at.y + drop, at.y, t);
      const h = lerp(wide * 1.9, narrow * 1.9, t);
      const w = lerp(at.s * 2.2, at.s * 0.7, t);
      d += rectPath(cx - w / 2, cy - h / 2, w, h);
    }
    return d;
  };
  const [left, right] = sides as [(typeof sides)[number], (typeof sides)[number]];
  return [
    shape(rail(left, 3), "cyan-400", 0.14),
    shape(rail(right, 3), "cyan-400", 0.14),
    shape(ties(left) + ties(right), "slate-700", 1),
    shape(rail(left, 1) + rail(right, 1), "cyan-400", 1, MAIN),
    shape(rail(left, 0.4) + rail(right, 0.4), "white", 0.9),
  ];
}

/** A glowing green syringe centred on the anchor, 12 long at scale 1, turned by the stream, its needle to the facing side. */
function syringePart(at: PartPlace, rng: Rng): KitShape[] {
  const f = local(at);
  const ang = between(rng, -45, 45);
  const T = (dx: number, dy: number): Point => turned([dx, dy], ang);
  const poly = (pts: readonly Point[]): string => f.poly(pts.map(([x, y]) => T(x, y)));
  const tick: string[] = [];
  for (let i = 0; i < 3; i += 1) tick.push(poly([[0 + i * 1.5, 0.9], [0.15 + i * 1.5, 0.9], [0.15 + i * 1.5, 1.3], [0 + i * 1.5, 1.3]]));
  return [
    shape(f.ellipse(...T(0, 0), 7.4, 3.2), "green-400", 0.14),
    shape(f.ellipse(...T(-0.5, 0), 4.6, 2.2), "green-400", 0.2),
    shape(poly([[-6.2, -1.6], [-5.6, -1.6], [-5.6, 1.6], [-6.2, 1.6]]) + poly([[-5.6, -0.3], [-2.2, -0.3], [-2.2, 0.3], [-5.6, 0.3]]), "slate-400", 1, INKED),
    shape(poly([[-3.4, -2.1], [-2.8, -2.1], [-2.8, 2.1], [-3.4, 2.1]]), "slate-400", 1, INKED),
    shape(poly([[-2.8, -1.3], [3, -1.3], [3, 1.3], [-2.8, 1.3]]), "sky-300", 0.45, INKED),
    shape(poly([[-1.6, -0.95], [2.8, -0.95], [2.8, 0.95], [-1.6, 0.95]]), "green-400", 1, MAIN),
    shape(poly([[-2.2, -0.95], [-1.6, -0.95], [-1.6, 0.95], [-2.2, 0.95]]), "slate-700", 1),
    shape(poly([[3, -0.75], [3.9, -0.75], [3.9, 0.75], [3, 0.75]]), "slate-700", 1, INKED),
    shape(poly([[3.9, -0.3], [6, -0.08], [6, 0.08], [3.9, 0.3]]), "slate-100", 1),
    shape(tick.join("") + poly([[-2, -1.05], [2.6, -1.05], [2.6, -0.8], [-2, -0.8]]), "white", 0.55),
  ];
}

/** A briefing podium standing on the anchor, 12 wide and 14 tall at scale 1, a microphone on its top. */
function podiumPart(at: PartPlace, rng: Rng): KitShape[] {
  const f = local(at);
  const sway = between(rng, -0.4, 0.4);
  const star: Point[] = [];
  for (let i = 0; i < 10; i += 1) {
    const a = (-90 + i * 36) * DEGREES;
    const r = i % 2 === 0 ? 1.8 : 0.75;
    star.push([r * Math.cos(a), -5.2 + r * Math.sin(a)]);
  }
  return [
    shape(f.poly([[-5, 0], [5, 0], [5.6, -9.6], [-5.6, -9.6]]), "wood-700", 1, MAIN_INKED),
    shape(f.poly([[-5, 0], [-3.6, 0], [-4.1, -9.6], [-5.6, -9.6]]), "white", 0.14),
    shape(f.poly([[3.6, 0], [5, 0], [5.6, -9.6], [4.1, -9.6]]), "slate-900", 0.28),
    shape(f.poly([[-6.2, -9.6], [6.2, -9.6], [5.4, -11.2], [-5.4, -11.2]]), "wood-600", 1, INKED),
    shape(f.circle(0, -5.2, 2.9), "slate-100", 1, INKED),
    shape(f.poly(star), "blue-800", 1),
    shape(f.band([[3.2, -11], [3.6 + sway, -13], [2.4 + sway, -13.9]], 0.4), "slate-900", 1),
    shape(f.ellipse(2.2 + sway, -13.9, 0.85, 0.65), "slate-700", 1, INKED),
  ];
}

/** A streetlamp standing on the anchor, 30 tall at scale 1, its head a violet light in a halo. */
function streetlampPart(at: PartPlace, rng: Rng): KitShape[] {
  const f = local(at);
  const glow = between(rng, 0.9, 1.1);
  return [
    shape(f.circle(0, -27.6, 8 * glow), "violet-300", 0.1),
    shape(f.circle(0, -27.6, 5.4 * glow), "violet-300", 0.18),
    shape(f.poly([[-1.8, 0], [1.8, 0], [1.1, -4], [-1.1, -4]]), "slate-900", 1, INKED),
    shape(f.band([[0, -4], [0, -25.8]], 1), "slate-900", 1, INKED),
    shape(f.band([[-0.3, -5], [-0.3, -25]], 0.3), "slate-700", 0.9),
    shape(f.poly([[-1.8, -25.8], [1.8, -25.8], [1.5, -26.6], [-1.5, -26.6]]), "slate-700", 1, INKED),
    shape(f.poly([[-1.5, -26.6], [1.5, -26.6], [2.1, -29.2], [-2.1, -29.2]]), "violet-300", 1, MAIN_INKED),
    shape(f.ellipse(0, -27.9, 0.7, 1.1), "white", 0.8),
    shape(f.poly([[-2.8, -29.2], [2.8, -29.2], [0, -31]]), "slate-900", 1, INKED),
  ];
}

/** A tidy steel scrap bin standing on the anchor, 16 wide and 12 tall at scale 1. */
function binPart(at: PartPlace, rng: Rng): KitShape[] {
  const f = local(at);
  const ribs: string[] = [];
  for (let i = 0; i < 5; i += 1) {
    const x = -5 + i * 2.5;
    ribs.push(f.band([[x, -8.6], [x * 0.84, -1.8]], 0.7));
  }
  const dent = between(rng, 0, 0.5);
  return [
    shape(f.poly([[-7.6, -10.8], [7.6, -10.8], [6.2, 0], [-6.2, 0]]), "slate-400", 1, MAIN_INKED),
    shape(f.poly([[-7.4, -10.4], [7.4, -10.4], [7.2, -8.4 + dent], [-7.2, -8.4 + dent]]), "slate-900", 0.38),
    shape(ribs.join(""), "slate-900", 0.28),
    shape(f.poly([[-7.6, -10.8], [-5.8, -10.8], [-4.6, 0], [-6.2, 0]]), "white", 0.22),
    shape(f.poly([[-6.4, -1.4], [6.4, -1.4], [6.2, 0], [-6.2, 0]]), "slate-700", 0.8),
    shape(f.poly([[-8, -12], [8, -12], [8, -10.4], [-8, -10.4]]), "slate-100", 1, INKED),
    shape(f.poly([[-7.4, -11.8], [-1, -11.8], [-1, -11.3], [-7.4, -11.3]]), "white", 0.6),
  ];
}

/** A bed with a sleeper standing on the anchor, 26 wide and 10 tall at scale 1, the headboard on the far side. */
function bedPart(at: PartPlace, rng: Rng): KitShape[] {
  const f = local(at);
  const hump = between(rng, 0, 0.5);
  return [
    shape(eRect(f, -12.6, -1.8, 1.2, 1.8) + eRect(f, 11.4, -1.8, 1.2, 1.8), "wood-800", 1),
    shape(f.poly([[-13, -9.6], [-11.3, -9.6], [-11.3, -4.2], [11.4, -4.2], [11.4, -6.6], [13, -6.6], [13, -1.8], [-13, -1.8]]), "wood-700", 1, INKED),
    shape(f.ellipse(-8.8, -5.4, 2.6, 1.4), "white", 1, INKED),
    shape(f.circle(-8.8, -6.9, 1.6), "red-300", 1, INKED),
    shape(`M${f.p(-10.5, -6.8)}Q${f.p(-10.3, -9)} ${f.p(-8.4, -8.7)}Q${f.p(-7.1, -8.4)} ${f.p(-7.1, -7.2)}Q${f.p(-8.5, -7.9)} ${f.p(-10.5, -6.8)}Z`, "wood-800", 1),
    shape(f.band([[-9.7, -6.8], [-9.2, -6.5], [-8.8, -6.8]], 0.22) + f.band([[-8.4, -6.8], [-7.9, -6.5], [-7.5, -6.8]], 0.22), "slate-900", 1),
    shape(`M${f.p(-6.4, -4.2)}Q${f.p(-6.5, -7.4 - hump)} ${f.p(-2, -7.2 - hump)}Q${f.p(4, -7.5 - hump)} ${f.p(11.4, -6.4)}L${f.p(11.4, -4.2)}Z`, "violet-700", 1, MAIN_INKED),
    shape(f.band([[-1, -7.2], [-0.4, -4.5]], 0.4) + f.band([[4, -7.2], [4.5, -4.6]], 0.4) + f.band([[8, -6.9], [8.3, -4.6]], 0.4), "slate-900", 0.22),
    shape(f.band([[-5, -7.1], [3, -7.1]], 0.4), "white", 0.25),
  ];
}

/** A string of bunting sagging between 25 left and 25 right of the anchor at its height, triangle flags grouped by colour. */
function buntingPart(at: PartPlace, rng: Rng): KitShape[] {
  const f = local(at);
  const sag = between(rng, 4.5, 6.5);
  const yAt = (x: number): number => sag * (1 - (x / 25) ** 2);
  const line: Point[] = [];
  for (let i = 0; i <= 10; i += 1) {
    const x = -25 + i * 5;
    line.push([x, yAt(x)]);
  }
  const groups: string[] = ["", "", "", ""];
  for (let i = 0; i < 16; i += 1) {
    const x = -22.5 + i * 3;
    const y = yAt(x);
    groups[i % 4] += f.poly([[x - 1.1, y], [x + 1.1, y], [x, y + 3]]);
  }
  const colours: PaletteName[] = ["red-500", "sky-400", "white", "lime-400"];
  return [
    shape(f.band(line, 0.4), "slate-700", 1),
    ...colours.map((c, i) => shape(groups[i] ?? "", c, 1, i === 0 ? MAIN_INKED : INKED)),
  ];
}

/** A strip of five vanity bulbs centred on the anchor, 24 wide at scale 1, each in an orange glow. */
function bulbsPart(at: PartPlace, rng: Rng): KitShape[] {
  const f = local(at);
  const xs = [-9.6, -4.8, 0, 4.8, 9.6];
  const lit = between(rng, 0.8, 1);
  return [
    shape(f.ellipse(0, 0, 13.6, 3.6), "orange-400", 0.12),
    shape(eRect(f, -12, -1.2, 24, 2.4), "slate-700", 1, INKED),
    shape(xs.map((x) => f.circle(x, 0, 2.35)).join(""), "orange-400", 0.3 * lit),
    shape(xs.map((x) => f.circle(x, 0, 1.55)).join(""), "white", 1, MAIN),
    shape(xs.map((x) => f.circle(x - 0.4, -0.4, 0.45)).join(""), "orange-400", 0.5),
  ];
}

/** A ball of gems and trinkets centred on the anchor, radius 6 at scale 1. */
function gemBallPart(at: PartPlace, rng: Rng): KitShape[] {
  const f = local(at);
  const rot = whole(rng, 0, 3);
  const groups: string[] = ["", "", "", ""];
  for (let k = 0; k < 8; k += 1) {
    const a = (22.5 + k * 45) * DEGREES;
    const x = 4 * Math.cos(a);
    const y = 4 * Math.sin(a);
    groups[(k + rot) % 4] += f.poly([[x, y - 1.35], [x + 1.05, y], [x, y + 1.35], [x - 1.05, y]]);
  }
  const colours: PaletteName[] = ["green-500", "red-500", "sky-400", "violet-500"];
  return [
    shape(f.circle(0, 0, 6), "brass-600", 1, MAIN_INKED),
    shape(f.ellipse(2.3, 2.8, 3.2, 2.5), "slate-900", 0.18),
    ...colours.map((c, i) => shape(groups[i] ?? "", c, 1)),
    shape(f.circle(0, 0, 1.6) + f.circle(0, 0, 0.95), "slate-100", 1),
    shape(f.ellipse(-2.3, -3, 2.3, 1.3), "white", 0.18),
    shape(sparkle(f, -3.6, -3.5, 1.2) + sparkle(f, 3.7, 2.3, 0.9), "white", 0.95),
  ];
}

/** An unrolled map centred on the anchor, 8 wide and 6 tall at scale 1, its ends rolled. */
function mapPart(at: PartPlace, rng: Rng): KitShape[] {
  const f = local(at);
  const bend = between(rng, -0.4, 0.4);
  const dots: string[] = [];
  const route: Point[] = [[-2.4, 2.3], [-1.6, 1.5], [-0.6, 1.2], [0.4, 0.4], [1.3, -0.2], [1.6, -1.2]];
  for (const [x, y] of route) dots.push(f.circle(x, y, 0.22));
  return [
    shape(eRect(f, -3.2, -3, 6.4, 6), "slate-100", 1, MAIN_INKED),
    shape(eRect(f, -4, -3.3, 1, 6.6) + eRect(f, 3, -3.3, 1, 6.6), "slate-400", 1, INKED),
    shape(eRect(f, -3.6, -3.2, 0.25, 6.4) + eRect(f, 3.35, -3.2, 0.25, 6.4), "white", 0.5),
    shape(f.band([[-2.4, -3], [-0.8 + bend, -1], [-1.5, 0.5], [0.2, 2], [0.6, 3]], 0.7), "blue-600", 1),
    shape(f.ellipse(-2, 1.6, 0.9, 0.7) + f.ellipse(1.9, -1.9, 1.1, 0.8) + f.ellipse(2, 1.8, 0.8, 0.6), "green-500", 1),
    shape(dots.join("") + f.circle(1.9, -2.1, 0.45), "red-600", 1),
  ];
}

/** A stitched rag doll standing on the anchor, 6 tall at scale 1, yarn hair. */
function dollPart(at: PartPlace, rng: Rng): KitShape[] {
  const f = local(at);
  const hair = between(rng, -0.2, 0.2);
  return [
    shape(f.band([[-0.65, -1.6], [-0.7, 0]], 0.9) + f.band([[0.65, -1.6], [0.7, 0]], 0.9), "red-300", 1, MAIN_INKED),
    shape(f.poly([[-1.1, -3.7], [1.1, -3.7], [1.3, -1.5], [-1.3, -1.5]]), "red-300", 1, MAIN_INKED),
    shape(f.band([[-1, -3.2], [-2.4, -1.7]], 0.8) + f.band([[1, -3.2], [2.4, -1.7]], 0.8), "red-300", 1, MAIN_INKED),
    shape(f.circle(0, -4.7, 1.2), "red-300", 1, MAIN_INKED),
    shape(f.poly([[-1.3, -5], [-1.6, -5.9], [-0.8, -5.6 + hair], [-0.5, -6.4], [0.1, -5.8], [0.7, -6.4], [0.9, -5.6 - hair], [1.6, -5.9], [1.3, -5], [0.4, -5.5], [-0.4, -5.5]]), "orange-400", 1, INKED),
    shape(f.circle(-0.45, -4.9, 0.27) + f.circle(0.45, -4.9, 0.27), "slate-900", 1),
    shape(f.band([[-0.5, -4.2], [0.5, -4.2]], 0.15) + f.band([[0, -3.3], [0, -2.9]], 0.2) + f.band([[0, -2.6], [0, -2.2]], 0.2) + f.band([[0, -1.9], [0, -1.6]], 0.2) + f.band([[-1.9, -2.4], [-1.7, -2.1]], 0.15) + f.band([[0.65, -0.9], [0.7, -0.5]], 0.15) + f.band([[-0.7, -0.9], [-0.7, -0.5]], 0.15), "slate-900", 0.9),
  ];
}

/** An archer's bow centred on the anchor, 10 tall at scale 1, curving away from the facing side. */
function bowPart(at: PartPlace, rng: Rng): KitShape[] {
  const f = local(at);
  const belly = between(rng, 2.2, 2.8);
  const stave: Point[] = [];
  for (let i = 0; i <= 8; i += 1) {
    const y = -5 + i * 1.25;
    stave.push([1 - belly * (1 - (y / 5) ** 2), y]);
  }
  return [
    shape(f.band([[1, -5], [1, 5]], 0.16), "white", 0.9),
    shape(f.band(stave, 0.8), "wood-600", 1, MAIN_INKED),
    shape(f.band(stave.map(([x, y]) => [x - 0.2, y] as const), 0.2), "white", 0.3),
    shape(f.band([[-belly + 1 - 0.1, -0.9], [-belly + 1 - 0.1, 0.9]], 1.2), "wood-800", 1, INKED),
    shape(f.circle(1, -5, 0.35) + f.circle(1, 5, 0.35), "wood-800", 1),
  ];
}

/** A glowing eye symbol centred on the anchor, 6 wide at scale 1. */
function eyePart(at: PartPlace, rng: Rng): KitShape[] {
  const f = local(at);
  const look = between(rng, -0.4, 0.4);
  return [
    shape(f.ellipse(0, 0, 5, 3.6), "cyan-400", 0.12),
    shape(f.ellipse(0, 0, 3.9, 2.4), "cyan-400", 0.18),
    shape(`M${f.p(-3, 0)}Q${f.p(0, -3.4)} ${f.p(3, 0)}Q${f.p(0, 3.4)} ${f.p(-3, 0)}Z`, "white", 1, INKED),
    shape(f.circle(look, 0, 1.6), "cyan-400", 1, MAIN),
    shape(f.circle(look, 0, 0.7), "black", 1),
    shape(f.circle(look - 0.5, -0.5, 0.3), "white", 0.9),
  ];
}

/** A big plain shipping crate standing on the anchor, opened, 40 wide and 36 tall at scale 1: its front panel fallen open toward the viewer, straw spilling. */
function crateBigPart(at: PartPlace, rng: Rng): KitShape[] {
  const f = local(at);
  const h = between(rng, -2, 2);
  return [
    shape(f.ellipse(0, 0.5, 21, 1.6), "slate-900", 0.25),
    shape(eRect(f, -14.5, -33.5, 29, 25.5), "wood-800", 1),
    shape(eRect(f, -14.5, -28, 29, 0.6) + eRect(f, -14.5, -21, 29, 0.6) + eRect(f, -14.5, -14, 29, 0.6), "wood-700", 0.9),
    shape(eRect(f, -17, -36, 34, 28) + eRect(f, -14.5, -33.5, 29, 25.5), "wood-600", 1, MAIN_INKED),
    shape(f.band([[-15.7, -35], [-15.7, -9]], 0.5) + f.band([[15.7, -35], [15.7, -9]], 0.5) + f.band([[-16, -34.8], [16, -34.8]], 0.5), "wood-700", 0.9),
    shape(f.poly([[-17, -8], [17, -8], [20, 0], [-20, 0]]), "wood-600", 1, MAIN_INKED),
    shape(f.band([[-9, -8], [-10.6, 0]], 0.8) + f.band([[0, -8], [0, 0]], 0.8) + f.band([[9, -8], [10.6, 0]], 0.8), "wood-700", 0.9),
    shape(f.poly([[-13, -8.5], [-12, -14], [-9, -13], [-8, -19 + h], [-5, -16], [-2, -23], [1, -17], [4, -21 - h], [6, -15], [9, -19], [11, -13], [13, -14], [13, -8.5]]), "brass-500", 1),
    shape(f.poly([[-9, -8.5], [-7, -4.4], [-4.5, -5.8], [-1, -2.6], [2, -5], [5, -3], [8, -6], [10, -8.5]]), "brass-500", 1),
    shape(f.band([[-6, -12], [-3.5, -16]], 0.35) + f.band([[3, -13], [6, -17]], 0.35) + f.band([[-2, -5.5], [1, -3.6]], 0.35), "brass-600", 0.8),
  ];
}

// @@ E objects kit
  "chess-piece": { category: "objects", draw: chessPiecePart },
  hourglass: { category: "objects", draw: hourglassPart },
  globe: { category: "objects", draw: globePart },
  notebook: { category: "objects", draw: notebookPart },
  bench: { category: "objects", draw: benchPart },
  nest: { category: "objects", draw: nestPart },
  ship: { category: "objects", draw: shipPart },
  gift: { category: "objects", draw: giftPart },
  honeypot: { category: "objects", draw: honeypotPart },
  copier: { category: "objects", draw: copierPart },
  mask: { category: "objects", draw: maskPart },
  pan: { category: "objects", draw: panPart },
  snare: { category: "objects", draw: snarePart },
  ladder: { category: "objects", draw: ladderPart },
  hammer: { category: "objects", draw: hammerPart },
  hologram: { category: "objects", draw: hologramPart },
  telescope: { category: "objects", draw: telescopePart },
  "rocket-cart": { category: "objects", draw: rocketCartPart },
  bell: { category: "objects", draw: bellPart },
  "rice-cake": { category: "objects", draw: riceCakePart },
  envelope: { category: "objects", draw: envelopePart },
  crucible: { category: "objects", draw: cruciblePart },
  crown: { category: "objects", draw: crownPart },
  vial: { category: "objects", draw: vialPart },
  ball: { category: "objects", draw: ballPart },
  hatch: { category: "objects", draw: hatchPart },
  clamp: { category: "objects", draw: clampPart },
  pocket: { category: "objects", draw: pocketPart },
  sack: { category: "objects", draw: sackPart },
  "chaos-wheel": { category: "objects", draw: chaosWheelPart },
  fist: { category: "objects", draw: fistPart },
  fountain: { category: "objects", draw: fountainPart },
  chandelier: { category: "objects", draw: chandelierPart },
  "puppet-hand": { category: "objects", draw: puppetHandPart },
  anvil: { category: "objects", draw: anvilPart },
  rails: { category: "objects", draw: railsPart },
  syringe: { category: "objects", draw: syringePart },
  podium: { category: "objects", draw: podiumPart },
  streetlamp: { category: "objects", draw: streetlampPart },
  bin: { category: "objects", draw: binPart },
  bed: { category: "objects", draw: bedPart },
  bunting: { category: "objects", draw: buntingPart },
  bulbs: { category: "objects", draw: bulbsPart },
  "gem-ball": { category: "objects", draw: gemBallPart },
  map: { category: "objects", draw: mapPart },
  doll: { category: "objects", draw: dollPart },
  bow: { category: "objects", draw: bowPart },
  eye: { category: "objects", draw: eyePart },
  "crate-big": { category: "objects", draw: crateBigPart },
