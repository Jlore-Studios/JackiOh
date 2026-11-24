// The greedy policy, page-side (window.__fast): one deterministic action for the acting seat, or null at the end.
() => {
  const j = () => window.__jackioh;
  const LANES = [3, 2, 4, 1, 5];
  const cost = (seat, id) => { const c = j().state.players[seat].hand.find((x) => x.id === id); return c ? (c.cost ?? 0) : -1; };
  const unitOf = (id) => { for (const s of ["p1", "p2"]) for (const u of j().state.players[s].units.flat()) if (u && u.id === id) return u; return null; };
  const foe = (seat) => (seat === "p1" ? "p2" : "p1");
  const next = () => {
    const s = j().state;
    if (s.result) return null;
    const seat = s.active;
    j().setSeat(seat);
    const L = j().legal();
    const other = L.filter((a) => !["play", "attack", "endTurn", "offerDraw", "concede", "switchPosition", "mulligan"].includes(a.type));
    if (other.length) return { seat, a: other[0], kind: "other" };
    const plays = L.filter((a) => a.type === "play" && !a.target).sort((x, y) => cost(seat, y.instanceId) - cost(seat, x.instanceId) || x.instanceId.localeCompare(y.instanceId, "en", { numeric: true }));
    if (plays.length) {
      const id = plays[0].instanceId;
      const mine = plays.filter((a) => a.instanceId === id);
      const pick = mine.find((a) => !a.zone) || LANES.map((l) => mine.find((a) => a.zone && a.zone.lane === l)).find(Boolean) || mine[0];
      return { seat, a: pick, kind: "play" };
    }
    const atk = L.filter((a) => a.type === "attack");
    if (atk.length) {
      const hero = atk.find((a) => !unitOf(a.targetId));
      if (hero) return { seat, a: hero, kind: "attackHero" };
      return { seat, a: atk[0], kind: "attackUnit" };
    }
    return { seat, a: L.find((a) => a.type === "endTurn"), kind: "end" };
  };
  window.__fast = { next, unitOf, foe };
  return "policy";
}
