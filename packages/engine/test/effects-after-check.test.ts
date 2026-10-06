// §4.5's check at a point of an effect list, then the rest on a stay that begins after it —
// `afterStateCheck` (effects/afterCheck.ts), the verb Classic #43 Plague Nuke needs ("Destroy all
// Units. Gain 1 mana for each Plague Counter … Then summon … each of those Units … from its owner's
// graveyard"). SPEC §4.5; R59, R78, R113, R174. The real card's test (packages/cards/test/classic/
// 043-plague-nuke.test.ts) covers the card's cases again through the card.
//
// Fixtures are this file's own: defs are prefixed `ac-` and indexed from 5300, so they cannot collide
// with another test file's catalog (BUILD §0).

import type { Action, ActionInput, CardDef, GameEvent } from "@jackioh/shared";
import { describe, expect, it } from "vitest";
import { registerCatalog, registeredCatalog } from "../src/catalog";
import { afterStateCheck } from "../src/effects/afterCheck";
import { destroyAll } from "../src/effects/destroy";
import { forEachCard } from "../src/effects/each";
import { gainMana } from "../src/effects/mana";
import { summon } from "../src/effects/summon";
import { openPrompt, resumeSelf } from "../src/prompts";
import { zoneCards } from "../src/query";
import { beginGame, reduce } from "../src/reduce";
import type { CardScripts, Effect, EffectContext, Script } from "../src/script";
import { registerScripts, registeredScripts } from "../src/scripts";
import { DEATHS_WORK } from "../src/stateCheck";
import type { CardInstance, GameState } from "../src/state";
import { owedWork } from "../src/work";
import { eventsOfType, inHand, newGame, put, slot } from "./fixtures/harness";

let nextIndex = 5300;

function def(name: string, type: CardDef["type"]): CardDef {
  nextIndex += 1;
  const face = type === "Unit" ? { attack: 1, health: 1, keywords: [], text: name } : { keywords: [], text: name };
  return {
    id: `ac-${name}`,
    index: String(nextIndex),
    name: `${name} (after check)`,
    set: "Core",
    type,
    tags: [],
    rarity: "Common",
    token: false,
    cost: 0,
    base: { ...face },
    radiant: { ...face },
  };
}

const logCard = def("log", "Field Spell");
const asker = def("asker", "Unit");
const quiet = def("quiet", "Unit");
/** Destroy all Units, then — after the check — note what the graveyards hold and gain 1 mana. */
const sweep = def("sweep", "Spell");
/** Destroy all Units, then — after the check — summon every Unit card in p2's graveyard for p1. */
const sweepAndTake = def("sweep-take", "Spell");
/** The control: the same summon in the same list with no check between, aimed at the ids read first. */
const sweepNoCheck = def("sweep-no-check", "Spell");

const DEFS = [logCard, asker, quiet, sweep, sweepAndTake, sweepNoCheck];

const NOTE_LANE = 5;

function write(state: GameState, entry: string): void {
  const log = state.players.p1.backrow[NOTE_LANE - 1];
  if (log === null || log === undefined) return;
  const steps = Array.isArray(log.memory.steps) ? (log.memory.steps as string[]) : [];
  log.memory.steps = [...steps, entry];
}

function notes(state: GameState): string[] {
  const log = state.players.p1.backrow[NOTE_LANE - 1];
  return Array.isArray(log?.memory.steps) ? (log.memory.steps as string[]) : [];
}

function note(entry: string | ((ctx: EffectContext) => string)): Effect {
  return {
    kind: "ac:note",
    apply(ctx): void {
      write(ctx.state, typeof entry === "string" ? entry : entry(ctx));
    },
  };
}

function askController(): Effect {
  return {
    kind: "ac:ask",
    apply(ctx): void {
      openPrompt(ctx, {
        player: ctx.controller,
        kind: "target",
        prompt: "the dying card asks",
        options: [{ key: "none", label: "nothing", selection: { pick: "none" } }],
        resume: resumeSelf(ctx, "asked"),
      });
    },
  };
}

function both(script: Script): CardScripts {
  return { base: script, radiant: script };
}

const unitCardsIn = (ctx: EffectContext, player: "p1" | "p2"): CardInstance[] =>
  zoneCards(ctx.state, player, "graveyard").filter((card) => card.defId === asker.id || card.defId === quiet.id);

const SCRIPTS: Record<string, CardScripts> = {
  [asker.id]: both({
    death: () => [note("ask"), askController(), note("tail")],
    resume: { asked: () => [note("answered")] },
  }),
  [quiet.id]: both({}),
  [sweep.id]: both({
    cry: () => [
      destroyAll({ side: "any" }),
      afterStateCheck((ctx) => [note(`rest:${unitCardsIn(ctx, "p2").length}`), gainMana({ amount: 1 })]),
    ],
  }),
  [sweepAndTake.id]: both({
    cry: () => [
      destroyAll({ side: "any" }),
      afterStateCheck((ctx) =>
        unitCardsIn(ctx, "p2").map((card) => summon({ instance: { of: "instance", instanceId: card.id }, player: "self" })),
      ),
    ],
  }),
  [sweepNoCheck.id]: both({
    cry: (ctx) => {
      const ids = ctx.state.players.p2.units.flatMap((pile) => (pile ?? []).map((card) => card.id));
      return [
        destroyAll({ side: "any" }),
        forEachCard({ cards: () => ids, each: (instanceId) => summon({ instance: { of: "instance", instanceId }, player: "self" }) }),
      ];
    },
  }),
};

function game(seed: string): GameState {
  const state = newGame(seed);
  registerCatalog({ ...registeredCatalog(), ...Object.fromEntries(DEFS.map((d) => [d.id, d])) });
  registerScripts({ ...registeredScripts(), ...SCRIPTS });
  return state;
}

let nonce = 0;
function actResult(state: GameState, body: ActionInput): ReturnType<typeof reduce> {
  nonce += 1;
  return reduce(state, { ...body, nonce: `ac${nonce}` } as Action);
}

function playing(seed: string): GameState {
  let state = beginGame(game(seed)).state;
  for (const playerId of ["p1", "p2"] as const) {
    const result = actResult(state, { type: "mulligan", keep: state.players[playerId].hand.map((c) => c.id), playerId });
    if (result.error !== undefined) throw new Error(result.error);
    state = result.state;
  }
  put(state, logCard.id, slot("p1", "backrow", NOTE_LANE));
  return state;
}

function playSpell(state: GameState, defId: string): ReturnType<typeof reduce> {
  const card = inHand(state, defId, "p1")[0];
  if (card === undefined) throw new Error("no spell");
  const result = actResult(state, { type: "play", instanceId: card.id, playerId: "p1" });
  if (result.error !== undefined) throw new Error(result.error);
  return result;
}

describe("§4.5 afterStateCheck (C #43 Plague Nuke)", () => {
  it("R59 runs the check at its point of the list: the rest finds the destroyed units in their graveyard, after their deaths", () => {
    const state = playing("after-check-order");
    put(state, quiet.id, slot("p2", "units", 1));
    put(state, quiet.id, slot("p2", "units", 2));

    const { state: after, events } = playSpell(state, sweep.id);

    expect(notes(after)).toEqual(["rest:2"]);
    const kinds = events.map((event) => event.type);
    expect(kinds.lastIndexOf("destroyed")).toBeLessThan(kinds.lastIndexOf("manaChanged"));
    expect(eventsOfType(events, "destroyed")).toHaveLength(2);
  });

  it("R174 the rest is a new stay: a unit the check sent to its graveyard is nameable there by id", () => {
    const state = playing("after-check-stay");
    const a = put(state, quiet.id, slot("p2", "units", 1));
    const b = put(state, quiet.id, slot("p2", "units", 3));

    const { state: after } = playSpell(state, sweepAndTake.id);

    expect(after.players.p1.units[0]?.[0]?.id).toBe(a.id);
    expect(after.players.p1.units[1]?.[0]?.id).toBe(b.id);
    expect(after.players.p2.graveyard.filter((card) => card.defId === quiet.id)).toEqual([]);
  });

  it("the control: without the check in between, the same summon finds the units still on the field and takes nothing", () => {
    const state = playing("after-check-control");
    put(state, quiet.id, slot("p2", "units", 1));

    const { state: after } = playSpell(state, sweepNoCheck.id);

    expect(after.players.p1.units.every((pile) => pile === null)).toBe(true);
    expect(after.players.p2.graveyard.filter((card) => card.defId === quiet.id)).toHaveLength(1);
  });

  it("R113 a Death hook asking inside the check parks the rest behind the pass; after the answer the rest runs once, and the pause survives JSON", () => {
    const state = playing("after-check-pause");
    put(state, asker.id, slot("p2", "units", 1));
    put(state, quiet.id, slot("p2", "units", 2));

    const { state: paused } = playSpell(state, sweep.id);

    expect(notes(paused)).toEqual(["ask"]);
    expect(paused.pending).not.toBeNull();
    expect(owedWork(paused, DEATHS_WORK)).toHaveLength(1);
    expect(paused.work.length).toBeGreaterThanOrEqual(2);

    const revived = JSON.parse(JSON.stringify(paused)) as GameState;
    expect(revived).toEqual(paused);
    const pending = revived.pending;
    if (pending === null) throw new Error("a prompt");
    const answered = actResult(revived, { type: "answer", choiceId: pending.id, selection: [{ pick: "none" }], playerId: pending.playerId });

    expect(answered.error).toBeUndefined();
    expect(notes(answered.state)).toEqual(["ask", "answered", "tail", "rest:2"]);
    expect(answered.state.pending).toBeNull();
    expect(answered.state.work).toEqual([]);
    const events: GameEvent[] = answered.events;
    expect(eventsOfType(events, "manaChanged").length).toBeGreaterThan(0);
  });
});
