// @@ R2 extra
/** A ripe purple fig splitting open, centred on the anchor, 8 wide and 8 tall at scale 1: two halves, red flesh with seeds, a stem. */
function figPart(at: PartPlace, rng: Rng): KitShape[] {
  const f = local(at);
  const gap = between(rng, 0.8, 1.4);
  return [
    shape(f.band([[0, -4.2], [0.4, -5.4], [1.4, -6]], 0.6), "green-700", 1),
    shape(`M${f.p(-gap, 3.6)}Q${f.p(-4.6, 2)} ${f.p(-3.6, -2)}Q${f.p(-2, -4.4)} ${f.p(-gap, -4)}Z`, "violet-700", 1, MAIN_INKED),
    shape(`M${f.p(gap, 3.6)}Q${f.p(4.6, 2)} ${f.p(3.6, -2)}Q${f.p(2, -4.4)} ${f.p(gap, -4)}Z`, "violet-700", 1, MAIN_INKED),
    shape(`M${f.p(-gap + 0.4, 3)}Q${f.p(-3.2, 1.6)} ${f.p(-2.6, -1.8)}Q${f.p(-1.8, -3.4)} ${f.p(-gap + 0.4, -3.2)}Z${f.poly([[gap - 0.4, -3.2], [gap - 0.4, 3], [2.6, 1.6], [2.2, -1.8]])}`, "red-400", 1),
    shape(f.circle(-1.8, -0.4, 0.28) + f.circle(-1.4, 1.2, 0.28) + f.circle(1.8, -0.8, 0.28) + f.circle(1.5, 0.9, 0.28) + f.circle(1.9, 2, 0.28), "white", 0.85),
    shape(f.ellipse(-3, -1.6, 0.5, 1), "white", 0.3),
  ];
}
// @@ R2 extra kit
  fig: { category: "objects", draw: figPart },
