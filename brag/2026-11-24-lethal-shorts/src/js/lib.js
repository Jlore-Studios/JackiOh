// Page-side helpers, installed as window.__brag.
() => {
  const BUSY = "[data-animating], [data-speaking], [data-showcase], [data-fx-banner], [data-fx-hold]";
  const j = () => window.__jackioh;
  const name = (id) => {
    const all = [...j().state.players.p1.hand, ...j().state.players.p2.hand];
    return all.find((c) => c.id === id)?.defId;
  };
  const handId = (seat, defId) => j().state.players[seat].hand.find((c) => c.defId === defId)?.id;
  const play = (seat, defId, row, lane) => {
    j().setSeat(seat);
    const id = handId(seat, defId);
    if (!id) throw new Error(`no ${defId} in ${seat}'s hand`);
    const a = j().legal().find((x) => x.type === "play" && x.instanceId === id && (!row || (x.zone && x.zone.row === row && x.zone.lane === lane)));
    if (!a) throw new Error(`no legal play of ${defId} at ${row}/${lane}: ` + JSON.stringify(j().legal().filter((x) => x.instanceId === id).slice(0, 6)));
    j().dispatch({ ...a, playerId: seat });
    return a;
  };
  const settle = async (ms = 20000) => {
    const until = Date.now() + ms;
    await new Promise((r) => setTimeout(r, 80));
    while (document.querySelector(BUSY) && Date.now() < until) await new Promise((r) => setTimeout(r, 50));
    await new Promise((r) => setTimeout(r, 300));
    return document.querySelector(BUSY) ? "still animating: " + [...document.querySelectorAll(BUSY)].map((e) => Object.keys(e.dataset).join("+")).join(",") : "settled";
  };
  const ready = async () => {
    const until = Date.now() + 20000;
    while (!j() && Date.now() < until) await new Promise((r) => setTimeout(r, 50));
    await document.fonts.ready;
    return !!j();
  };
  const keep = () => {
    for (const seat of ["p1", "p2"]) {
      j().setSeat(seat);
      const k = j().legal().filter((a) => a.type === "mulligan").sort((a, b) => b.keep.length - a.keep.length)[0];
      j().dispatch({ ...k, playerId: seat });
    }
    j().setSeat("p1");
  };
  const endTurn = (seat) => { j().setSeat(seat); j().dispatch({ type: "endTurn", playerId: seat }); };
  const legal = (seat) => { j().setSeat(seat); return j().legal(); };
  window.__brag = { play, settle, ready, keep, endTurn, legal, handId, name };
  return "installed";
}
