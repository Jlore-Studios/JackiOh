import { carriedAt, defOf, hashState, legalActions, reduce, stackedOnto, unitView, registeredScripts, registerScripts, effectiveCost } from "@jackioh/engine";
import type { GameState } from "@jackioh/engine";
import { describe, expect, it } from "vitest";
import { appendFileSync } from "node:fs";
const log = (...a: unknown[]): void => appendFileSync("/tmp/zz-review.log", a.map(String).join(" ") + "\n");
import { bounceCard } from "../../../engine/src/effects/move";
import { scenario, type FieldSetup, type Scenario } from "../_harness";

const TOWER = "classicplus-033";
const FILLER = "core-005";
const DECK = [FILLER, FILLER, FILLER, FILLER, FILLER, FILLER];

function setup(rider: string, opts: { radiant?: boolean; hand?: string[]; p2Backrow?: readonly FieldSetup[]; field?: readonly FieldSetup[]; mana?: number } = {}): Scenario {
  return scenario({
    p1: {
      hand: [rider, ...(opts.hand ?? [FILLER])],
      field: opts.field ?? [],
      backrow: [{ def: TOWER, lane: 2, radiant: opts.radiant === true }],
      library: DECK,
      mana: opts.mana ?? 10,
    },
    p2: { hand: [FILLER], backrow: opts.p2Backrow ?? [], library: DECK, field: [{ def: "core-019", lane: 3 }] },
  });
}

const BR = { player: "p1" as const, row: "backrow" as const, lane: 2 };

describe("scratch review", () => {
  it("Scarab prompt pause + JSON round trip", () => {
    const run = (roundTrip: boolean) => {
      const s = setup("core-007");
      const tower = s.card(TOWER).id;
      const rider = s.card("core-007").id;
      s.play(rider, { zone: 2, row: "backrow" });
      const paused = s.state;
      log("pending", paused.pending?.kind, "carried", carriedAt(paused, BR)?.id, "stacked", stackedOnto(s.card(tower)));
      expect(paused.pending).not.toBeNull();
      expect(carriedAt(paused, BR)?.id).toBe(rider);
      const state: GameState = roundTrip ? (JSON.parse(JSON.stringify(paused)) as GameState) : paused;
      const opt = (state.pending as unknown as { options: unknown[] }).options[0];
      log("option", JSON.stringify(opt));
      const result = reduce(state, { type: "answer", playerId: "p1", choiceId: state.pending?.id ?? "", selection: [(opt as any).selection], nonce: "x1" });
      expect(result.error).toBeUndefined();
      const done = result.state;
      const t = done.players.p1.backrow[1];
      log("after", t?.defId, carriedAt(done, BR)?.id, result.events.map((e) => e.type).join(","));
      expect(t?.defId).toMatch(/core-007\+classicplus-033/);
      expect(carriedAt(done, BR)).toBeNull();
      return { paused: hashState(paused), done: hashState(done) };
    };
    const live = run(false);
    expect(run(true)).toEqual(live);
  });

  it("Felinor Fuser nested fusion", () => {
    const s = setup("classicplus-030");
    const tower = s.card(TOWER).id;
    s.play("classicplus-030", { zone: 2, row: "backrow" });
    let guard = 0;
    while (s.state.pending !== null && guard < 5) {
      const opt = (s.state.pending as unknown as { options: unknown[] }).options[0];
      const r = reduce(s.state, { type: "answer", playerId: "p1", choiceId: s.state.pending.id, selection: [(opt as any).selection], nonce: `f${guard}` });
      expect(r.error).toBeUndefined();
      (s as unknown as { current: GameState }).current = r.state;
      guard += 1;
    }
    const t = s.card(tower);
    const d = defOf(s.state, t.defId);
    log("felinor fused", t.defId, d.type, d.cost, d.base.attack, d.base.health, JSON.stringify(d.base.keywords));
    expect(d.type).toBe("Field Spell");
    expect(carriedAt(s.state, BR)).toBeNull();
    // stats on a field spell: unitView
    log("unitView tower", JSON.stringify(unitView(s.state, t)));
  });

  it("Reborn + Divine Shield unit fused: destroy the Tower", () => {
    const s = setup("core-003", { hand: ["core-036", FILLER] });
    const tower = s.card(TOWER).id;
    s.play("core-003", { zone: 2, row: "backrow" });
    const d = defOf(s.state, s.card(tower).defId);
    log("fused kws", JSON.stringify(d.base.keywords), "cost", d.cost);
    s.play("core-036", { targets: [{ pick: "instance", instanceId: tower }] });
    const where = s.card(tower);
    log("after MJ tower zone", JSON.stringify(where.zone), "events", s.lastEvents.map((e) => e.type).join(","));
    log("backrow2", s.state.players.p1.backrow[1]?.id, "units", s.state.players.p1.units.map((p) => p?.[0]?.defId ?? null).join("|"));
  });

  it("Back Breaker fused: destroy the Tower with its Death", () => {
    const s = setup("classic-051", { hand: ["core-036", FILLER] });
    const tower = s.card(TOWER).id;
    s.play("classic-051", { zone: 2, row: "backrow" });
    log("bb fused", s.card(tower).defId);
    s.play("core-036", { targets: [{ pick: "instance", instanceId: tower }] });
    log("after", JSON.stringify(s.card(tower).zone), s.lastEvents.map((e) => e.type).join(","));
  });

  it("fused cost and all-units effects", () => {
    const s = setup("core-019", { hand: ["core-017", FILLER] });
    const tower = s.card(TOWER).id;
    s.play("core-019", { zone: 2, row: "backrow" });
    const d = defOf(s.state, s.card(tower).defId);
    log("menace fused cost", d.cost, d.base.attack, d.base.health, JSON.stringify(d.base.keywords));
    expect(d.cost).toBe(4);
    // Flood: bounce all Units -> tower stays
    s.play("core-017");
    log("tower after Flood", JSON.stringify(s.card(tower).zone));
    expect(s.card(tower).zone.z).toBe("field");
  });

  it("bounced fused Tower replays as a new arrival and can take one again; its fused Cry fires", () => {
    const s = setup("core-015", { hand: ["core-017", "core-008", FILLER] });
    const tower = s.card(TOWER).id;
    s.play("core-015", { zone: 2, row: "backrow" });
    (s as unknown as { direct: (f: (sink: any) => void) => void }).direct((sink: any) => {
      const card = sink.state.players.p1.backrow[1];
      bounceCard(sink, card);
    });
    log("bounced tower", JSON.stringify(s.card(tower).zone), s.card(tower).defId, "memory", JSON.stringify(s.card(tower).memory));
    s.play(tower, { zone: 3, row: "backrow" });
    log("replayed events", s.lastEvents.map((e) => e.type + (e.type === "summoned" ? ":" + (e as any).defId : "")).join(","));
    const plays = legalActions(s.state, "p1").filter((a) => a.type === "play" && a.zone?.row === "backrow");
    log("backrow plays after replay", JSON.stringify(plays.map((a: any) => a.zone)));
  });

  it("countered Unit (Exile) leaves the Tower free", () => {
    const s = setup("core-008", { hand: ["core-015", FILLER], p2Backrow: [{ def: "classic-010", faceUp: false }] });
    const tower = s.card(TOWER).id;
    s.play("core-008", { zone: 2, row: "backrow" });
    log("countered events", s.lastEvents.map((e) => e.type).join(","), "stacked", stackedOnto(s.card(tower)), "carried", carriedAt(s.state, BR)?.id);
    const plays = legalActions(s.state, "p1").filter((a) => a.type === "play" && a.zone?.row === "backrow");
    log("backrow plays after counter", plays.length);
  });

  it("The Rock with tribute onto the Tower", () => {
    const s = setup("core-066", { field: [{ def: "core-008", lane: 1 }] });
    const tower = s.card(TOWER).id;
    const tribute = s.unit("p1", 1);
    s.play("core-066", { zone: 2, row: "backrow", tributes: [tribute!.id] });
    const d = defOf(s.state, s.card(tower).defId);
    log("rock fused", d.cost, JSON.stringify(d.base.keywords), "unit1", s.unit("p1", 1)?.defId);
  });

  it("Silly Silas rotation on the Tower", () => {
    const s = setup("core-052");
    const tower = s.card(TOWER).id;
    const silas = s.card("core-052").id;
    const plays = legalActions(s.state, "p1").filter((a) => a.type === "play" && a.instanceId === silas && a.zone?.row === "backrow");
    log("silas plays", JSON.stringify(plays.slice(0, 2)));
    const p = plays[0] as any;
    const r = reduce(s.state, { ...p, playerId: "p1", nonce: "s1" });
    log("silas err", r.error);
    if (r.error === undefined) {
      const st = r.state;
      const towerNow = [...st.players.p1.backrow, ...st.players.p2.backrow].find((c) => c?.id === tower);
      log("tower now", towerNow?.defId, JSON.stringify(towerNow?.zone), "carried p1", JSON.stringify(st.players.p1.carried?.map((c) => c?.id ?? null)), "p2", JSON.stringify(st.players.p2.carried?.map((c) => c?.id ?? null)));
      log("events", r.events.map((e) => e.type).join(","));
    }
  });

  it("Reborn fused Tower destroyed by Guy Att (no lock)", () => {
    const s = setup("core-003", { hand: ["classicplus-005", FILLER] });
    const tower = s.card(TOWER).id;
    s.play("core-003", { zone: 2, row: "backrow" });
    s.play("classicplus-005", { zone: 1 });
    const t = s.card(tower);
    log("REBORN tower zone", JSON.stringify(t.zone), t.defId, "damage", t.damage, "rebornSpent", t.rebornSpent, "events", s.lastEvents.map((e) => e.type + ((e as any).row ? ":" + (e as any).row : "")).join(","));
    log("REBORN unitView", JSON.stringify(unitView(s.state, t)));
  });

  it("Lane Eater on the Tower", () => {
    const s = setup("classic-071");
    const tower = s.card(TOWER).id;
    const le = s.card("classic-071").id;
    s.play(le, { zone: 2, row: "backrow" });
    log("LANE EATER tower", JSON.stringify(s.card(tower).zone), s.card(tower).defId, "events", s.lastEvents.map((e) => e.type).join(","), "locks", JSON.stringify(s.state.players.p1.locks), JSON.stringify(s.state.players.p2.locks));
  });

  it("Mid Runner tributes itself on the Tower in lane 3", () => {
    const s = scenario({
      p1: { hand: ["classic-022", "core-015", FILLER], backrow: [{ def: TOWER, lane: 3 }], library: DECK, mana: 10 },
      p2: { hand: [FILLER], library: DECK },
    });
    const tower = s.card(TOWER).id;
    const mr = s.card("classic-022").id;
    s.play(mr, { zone: 3, row: "backrow" });
    log("MID RUNNER", "tower", s.card(tower).defId, "stacked", stackedOnto(s.card(tower)), "events", s.lastEvents.map((e) => e.type).join(","));
    const plays = legalActions(s.state, "p1").filter((a) => a.type === "play" && a.zone?.row === "backrow");
    log("MID RUNNER backrow plays", plays.length);
  });

  it("Sheepish vs Radiant Tower order", () => {
    const s = setup("core-015", { radiant: true, p2Backrow: [{ def: "core-041", faceUp: false }] });
    const tower = s.card(TOWER).id;
    s.play("core-015", { zone: 2, row: "backrow" });
    log("SHEEP RADIANT", s.card(tower).defId, s.lastEvents.map((e) => e.type).join(","));
  });

  it("Unlicensed Experimentation takes the carried Unit", () => {
    const s = setup("core-008", { p2Backrow: [{ def: "core-085", faceUp: false }] });
    const tower = s.card(TOWER).id;
    const rider = s.card("core-008").id;
    s.play(rider, { zone: 2, row: "backrow" });
    log("UE", s.card(tower).defId, "carried", carriedAt(s.state, BR)?.id, "p2 unit3", s.unit("p2", 3)?.defId, "stacked", stackedOnto(s.card(tower)), s.lastEvents.map((e) => e.type).join(","));
  });

  it("Bear Honeypot on a carried Unit", () => {
    const s = setup("core-008", { p2Backrow: [{ def: "core-060", faceUp: false }] });
    const tower = s.card(TOWER).id;
    s.play("core-008", { zone: 2, row: "backrow" });
    log("HONEYPOT", s.card(tower).defId, "carried", carriedAt(s.state, BR)?.id, s.lastEvents.map((e) => e.type).join(","));
  });

  it("Ivory Tower fused into C+ #74", () => {
    const s = scenario({
      p1: { hand: ["classicplus-074", "core-008", FILLER], library: DECK, mana: 10 },
      p2: { hand: ["core-010", TOWER, "core-010", "core-008", FILLER], library: DECK },
    });
    s.play("classicplus-074", { zone: 2 }).endTurn();
    s.state.players.p2.mana.current = 10;
    s.play("core-010");
    s.play(TOWER, { zone: 1 });
    const trap = s.backrow("p1", 2)!;
    log("TF74 trap", trap.defId, "faceUp", trap.faceUp);
    s.play("core-010");
    log("TF74 3rd play events", s.lastEvents.map((e) => e.type).join(","));
    s.play("core-008", { zone: 1 });
    log("TF74 4th play events", s.lastEvents.map((e) => e.type).join(","), "trap now", s.backrow("p1", 2)?.defId);
    s.endTurn();
    log("TF74 p1 backrow zones for a unit", JSON.stringify(legalActions(s.state, "p1").filter((a) => a.type === "play" && a.zone?.row === "backrow").map((a: any) => a.zone)));
  });

  it("registry rebuild for the fused Tower (R179)", () => {
    const run = (drop: boolean) => {
      const s = setup("classicplus-049", { hand: ["core-035", FILLER] });
      const tower = s.card(TOWER).id;
      s.play("classicplus-049", { zone: 2, row: "backrow" });
      const id = s.card(tower).defId;
      const st = JSON.parse(JSON.stringify(s.state)) as GameState;
      const saved = registeredScripts()[id];
      if (drop) { const all = { ...registeredScripts() }; delete all[id]; registerScripts(all); }
      const r = reduce(st, { type: "endTurn", playerId: "p1", nonce: "e1" });
      if (drop && saved !== undefined) registerScripts({ ...registeredScripts(), [id]: saved });
      expect(r.error).toBeUndefined();
      log("R179", drop, id, r.events.filter((e) => e.type === "costChanged").length, hashState(r.state));
      return hashState(r.state);
    };
    expect(run(true)).toEqual(run(false));
  });
});
