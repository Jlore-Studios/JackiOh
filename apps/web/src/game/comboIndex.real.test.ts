// R3402001 against the real engine (WebAssembly) and its `viewFor` redaction: Combo-Index's grade beat
// keys on the event the engine sends as its end of turn raises the grade, a `counterChanged` of its
// `grade` (crates/engine/src/subsystems/combo_index.rs), never a `questProgressed`, so a game lights the
// grade's ring and shakes as its gallery play does. The grade E it writes as it arrives is no trigger's
// and lights none. Each seat is fed as Game feeds it: the new part of its view's window
// (`newEventsSince`), planned against the view before (`planEntries`, `entranceEntries`,
// `setPieceEntries`, `planFx`), one memory across the actions.

import type { Action, ActionBody, CardType, GameEvent, PlayerId, PlayerView } from "@jackioh/shared";
import { describe, expect, it } from "vitest";

import { devDeck, realPort } from "../audio/test/realGame.ts";
import { FX_GRADE_TRAUMA } from "../fx/constants.ts";
import { catalogFacts } from "../fx/FxLayer.tsx";
import { planFx } from "../fx/cues.ts";
import { createFxMemory } from "../fx/memory.ts";
import type { FxCue } from "../fx/types.ts";
import { entranceEntries, newEventsSince, planEntries, setPieceEntries, takeoverClaim, type EntranceResolver } from "./animations.ts";
import { lookupFromDefs } from "./catalog.ts";
import { testid } from "./contract.ts";

const SEATS = ["p1", "p2"] as const;
/** Core #93 Combo-Index: a Legendary Field Spell whose end of turn raises its grade once a card was played. */
const COMBO_INDEX = "core-093";
/** The full intensity Game plans at by default. */
const INTENSITY = 1;

const port = realPort();
const lookup = lookupFromDefs(port.catalog?.() ?? {});
const resolve: EntranceResolver = (defId) => {
  const info = lookup(defId, false);
  return info === undefined ? null : { type: info.type as CardType, rarity: info.def?.printedRarity ?? info.rarity };
};

type Heard = { fresh: GameEvent[]; cues: FxCue[] };

/** One seat's board: its memory, the cards its queue has entered, and what each new view made it draw. */
function seat(first: PlayerView): (next: PlayerView) => Heard {
  const memory = createFxMemory();
  const entered = new Set<string>();
  let view = first;
  return (next) => {
    const fresh = newEventsSince(view.events, next.events);
    const entries = setPieceEntries(entranceEntries(planEntries(fresh, view, false), resolve, { seen: entered }), takeoverClaim, {
      reduced: false,
      bigMoments: "full",
      speed: 1,
    });
    const env = { intensity: INTENSITY, card: (defId: string) => catalogFacts(lookup, defId), memory };
    const cues = entries.flatMap((entry) => {
      memory.remember(entry.events);
      return planFx(entry, entry.view, env);
    });
    view = next;
    return { fresh, cues };
  };
}

/** A sandbox game at p1's main phase with Combo-Index in p1's hand, both seats' boards following it. */
function comboGame() {
  const cheap = devDeck("cheap20");
  let state = port.beginGame(port.createGame({ seed: "r3402001-combo", decks: [cheap, cheap], sandbox: true } as Parameters<typeof port.createGame>[0])).state;
  let nonce = 0;
  const raw = (player: PlayerId, body: ActionBody): void => {
    nonce += 1;
    const result = port.reduce(state, { ...body, playerId: player, nonce: `r3402001-${String(nonce)}` } as Action);
    if (result.error !== undefined) throw new Error(`${player} ${body.type}: ${result.error}`);
    state = result.state;
  };
  const actor = (): PlayerId | undefined => SEATS.find((p) => port.legalActions(state, p).some((a) => a.type !== "concede" && a.type !== "offerDraw"));
  for (let step = 0; step < 40; step += 1) {
    const v = port.viewFor(state, "p1");
    if (v.active === "p1" && v.phase === "main" && v.pending === null) break;
    const who = actor();
    if (who === undefined) break;
    const legal = port.legalActions(state, who);
    const hand = port.viewFor(state, who).you.hand;
    if (legal.some((a) => a.type === "mulligan")) raw(who, { type: "mulligan", keep: Array.isArray(hand) ? hand.map((c) => c.instanceId) : [] });
    else raw(who, legal.find((a) => a.type === "answer") ?? { type: "endTurn" });
  }
  const sandbox = (op: unknown): void => raw("p1", { type: "sandbox", op } as unknown as ActionBody);
  sandbox({ kind: "setMana", player: "p1", current: 10, max: 10 });
  sandbox({ kind: "spawn", player: "p1", defId: COMBO_INDEX, to: { zone: "hand" } });
  const boards = { p1: seat(port.viewFor(state, "p1")), p2: seat(port.viewFor(state, "p2")) };
  const act = (body: ActionBody): Record<PlayerId, Heard> => {
    raw("p1", body);
    return { p1: boards.p1(port.viewFor(state, "p1")), p2: boards.p2(port.viewFor(state, "p2")) };
  };
  const hand = port.viewFor(state, "p1").you.hand;
  const card = Array.isArray(hand) ? hand.find((c) => c.defId === COMBO_INDEX) : undefined;
  const play = port.legalActions(state, "p1").find((a) => a.type === "play" && a.instanceId === card?.instanceId && a.faceDown === undefined);
  if (play === undefined) throw new Error("no play of Combo-Index");
  const played = act(play);
  return { played, ended: act({ type: "endTurn" }) };
}

/** The id Combo-Index stands under once played: the instance its grade counter names. */
function gradedId(fresh: readonly GameEvent[]): string | undefined {
  const grade = fresh.find((e) => e.type === "counterChanged" && e.counter === "grade");
  return grade?.type === "counterChanged" ? grade.instanceId : undefined;
}

/** The storm-blue rings lit on the stadium itself. */
const gradeRings = (cues: readonly FxCue[], id: string): FxCue[] =>
  cues.filter((cue) => cue.kind === "ring" && cue.preset === "frost" && JSON.stringify(cue.at) === JSON.stringify({ kind: "testid", testid: testid.card(id) }));

describe("R3402001 against the real engine: Combo-Index's grade", () => {
  it("R3402001 its end of turn's grade rise lights a ring on both seats; its arrival's grade lights none", () => {
    const { played, ended } = comboGame();
    for (const viewer of SEATS) {
      // The engine writes grade E as it arrives and raises it at the end of the turn, as a counter, never a quest.
      const id = gradedId(played[viewer].fresh);
      expect(id, `its arrival's grade on ${viewer}`).toBeDefined();
      const rise = ended[viewer].fresh.filter((e) => e.type === "counterChanged" && e.counter === "grade" && e.instanceId === id);
      expect(rise, `its rise on ${viewer}`).toHaveLength(1);
      expect(ended[viewer].fresh.some((e) => e.type === "questProgressed"), `no quest on ${viewer}`).toBe(false);
      // Its arrival lands as the stadium (its entrance), but its grade E lights no grade ring.
      expect(gradeRings(played[viewer].cues, id ?? ""), `its arrival on ${viewer}`).toEqual([]);
      // Its trigger lights the next ring with the grade's shake.
      expect(gradeRings(ended[viewer].cues, id ?? ""), `its trigger on ${viewer}`).toHaveLength(1);
      expect(ended[viewer].cues.some((cue) => cue.kind === "shake" && cue.trauma === Math.min(1, FX_GRADE_TRAUMA * INTENSITY)), `its shake on ${viewer}`).toBe(true);
    }
    // R202: the copy its grade E adds to p1's hand is hidden from p2, and no beat of p2's names a card.
    expect(JSON.stringify(ended.p2.cues)).not.toMatch(/core-\d{3}/);
  });
});
