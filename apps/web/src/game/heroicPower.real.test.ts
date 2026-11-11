// R3402001 against the real engine (WebAssembly) and its `viewFor` redaction: Heroic Power's Activate
// flares its pedestal in a game. The engine sends an Activate's `activated` before its hook's
// `hookResolving` marker (crates/engine/src/subsystems/activate.rs), so no `activate` hook of it is open
// when `activated` is planned: the flare keys on the card's own `activated` (its signature's claim,
// fx/cardFx.ts), not on the hook. Each seat is fed as Game feeds it: the new part of its view's window
// (`newEventsSince`), planned against the view before (`planEntries`, `entranceEntries`,
// `setPieceEntries`, `planFx`), one memory across the actions.

import type { Action, ActionBody, CardType, GameEvent, PlayerId, PlayerView } from "@jackioh/shared";
import { describe, expect, it } from "vitest";

import { devDeck, realPort } from "../audio/test/realGame.ts";
import { catalogFacts } from "../fx/FxLayer.tsx";
import { planFx } from "../fx/cues.ts";
import { createFxMemory } from "../fx/memory.ts";
import type { FxCue } from "../fx/types.ts";
import { entranceEntries, newEventsSince, planEntries, setPieceEntries, takeoverClaim, type EntranceResolver } from "./animations.ts";
import { lookupFromDefs } from "./catalog.ts";
import { testid } from "./contract.ts";

const SEATS = ["p1", "p2"] as const;
/** Core #98 Heroic Power: a Mythic Field Spell whose Cry Discovers a power it Activates. */
const HEROIC_POWER = "core-098";
/** The full intensity Game plans at by default. */
const INTENSITY = 1;
/** More than the setup's mulligans and its prompts, or a power's prompts, ever take. */
const STEPS_MAX = 40;

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

/** A sandbox game at p1's main phase: Heroic Power played, its Discover answered, then its power Activated. */
function heroicGame() {
  const cheap = devDeck("cheap20");
  let state = port.beginGame(port.createGame({ seed: "r3402001-heroic", decks: [cheap, cheap], sandbox: true } as Parameters<typeof port.createGame>[0])).state;
  let nonce = 0;
  const raw = (player: PlayerId, body: ActionBody): void => {
    nonce += 1;
    const result = port.reduce(state, { ...body, playerId: player, nonce: `r3402001-${String(nonce)}` } as Action);
    if (result.error !== undefined) throw new Error(`${player} ${body.type}: ${result.error}`);
    state = result.state;
  };
  const actor = (): PlayerId | undefined => SEATS.find((p) => port.legalActions(state, p).some((a) => a.type !== "concede" && a.type !== "offerDraw"));
  for (let step = 0; step < STEPS_MAX; step += 1) {
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
  sandbox({ kind: "spawn", player: "p1", defId: HEROIC_POWER, to: { zone: "hand" } });
  const boards = { p1: seat(port.viewFor(state, "p1")), p2: seat(port.viewFor(state, "p2")) };
  const heard = (): Record<PlayerId, Heard> => ({ p1: boards.p1(port.viewFor(state, "p1")), p2: boards.p2(port.viewFor(state, "p2")) });
  /** p1's action, then the first listed answer to every prompt it opens, each seat's board fed after each. */
  const act = (body: ActionBody): Record<PlayerId, Heard>[] => {
    raw("p1", body);
    const out = [heard()];
    for (let step = 0; step < STEPS_MAX && port.viewFor(state, "p1").pending !== null; step += 1) {
      const answer = port.legalActions(state, "p1").find((a) => a.type === "answer");
      if (answer === undefined) break;
      raw("p1", answer);
      out.push(heard());
    }
    return out;
  };
  const hand = port.viewFor(state, "p1").you.hand;
  const card = Array.isArray(hand) ? hand.find((c) => c.defId === HEROIC_POWER) : undefined;
  const play = port.legalActions(state, "p1").find((a) => a.type === "play" && a.instanceId === card?.instanceId && a.faceDown === undefined);
  if (card === undefined || play === undefined) throw new Error("no play of Heroic Power");
  const played = act(play);
  const activate = port.legalActions(state, "p1").find((a) => (a.type === "activate" || a.type === "activatePower") && "instanceId" in a && a.instanceId === card.instanceId);
  if (activate === undefined) throw new Error("no Activate of Heroic Power");
  return { id: card.instanceId, played, activated: act(activate) };
}

/** The holy rings lit on the pedestal itself. */
const flares = (cues: readonly FxCue[], id: string): FxCue[] =>
  cues.filter((cue) => cue.kind === "ring" && cue.preset === "holy" && JSON.stringify(cue.at) === JSON.stringify({ kind: "testid", testid: testid.card(id) }));

describe("R3402001 against the real engine: Heroic Power's Activate", () => {
  it("R3402001 its Activate flares the pedestal on both seats, though `activated` comes before its hook's marker", () => {
    const { id, activated } = heroicGame();
    for (const viewer of SEATS) {
      const fresh = activated.flatMap((step) => step[viewer].fresh);
      const cues = activated.flatMap((step) => step[viewer].cues);
      // The engine's order: the card's `activated`, then its Activate's hook marker.
      const at = fresh.findIndex((e) => e.type === "activated" && e.instanceId === id);
      const marker = fresh.findIndex((e) => e.type === "hookResolving" && e.hook === "activate" && e.sourceId === id);
      expect(at, `its activated on ${viewer}`).toBeGreaterThanOrEqual(0);
      expect(marker, `its marker after it on ${viewer}`).toBeGreaterThan(at);
      // The pedestal flares once, in place of the shared arcane flare an Activate draws on its card.
      expect(flares(cues, id), `its flare on ${viewer}`).toHaveLength(1);
      expect(
        cues.some((cue) => cue.kind === "ring" && cue.preset === "arcane" && JSON.stringify(cue.at) === JSON.stringify({ kind: "testid", testid: testid.card(id) })),
        `no shared flare on ${viewer}`,
      ).toBe(false);
    }
    // R202: no beat of p2's names a card.
    expect(JSON.stringify(activated.map((step) => step.p2.cues))).not.toMatch(/core-\d{3}/);
  });
});
