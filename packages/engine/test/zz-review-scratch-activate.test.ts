import type { Action, ActionInput, CardDef, GameEvent, PlayerId } from "@jackioh/shared";
import { describe, expect, it } from "vitest";
import { registerCatalog, registeredCatalog } from "../src/catalog";
import { HERO_HEALTH } from "../src/config";
import { beginGame, legalActions, reduce } from "../src/reduce";
import { hashState } from "../src/replay";
import { registerScripts, registeredScripts } from "../src/scripts";
import { damage } from "../src/effects";
import { param, paramsView } from "../src/params";
import { recalled } from "../src/query";
import { abilitiesOf, whyCannotActivateAbility } from "../src/subsystems/activate";
import { fuse } from "../src/subsystems/fuse";
import { PART_KEY, rememberOn, memoryOfPart, rerootRemembered } from "../src/work";
import type { GameState } from "../src/state";
import type { CardScripts } from "../src/script";
import {
  ACTIVATE_SCRIPTS,
  KEEPER_KEY,
  LOG_LANE,
  activateCatalog,
  askController,
  asker,
  endless,
  keeper,
  logCard,
  note,
  notes,
  spark,
  zapper,
  turtle,
  mourner,
  lockdown,
  sentry,
} from "./fixtures/activate";
import { inHand, newGame, put, sinkFor, slot } from "./fixtures/harness";

function mkDef(name: string, params?: CardDef["params"]): CardDef {
  return {
    id: `zz-${name}`,
    index: String(9900 + name.length),
    name,
    set: "Core",
    type: "Field Spell",
    tags: [],
    rarity: "Common",
    token: false,
    cost: 0,
    base: { keywords: [], text: name },
    radiant: { keywords: [], text: name },
    ...(params === undefined ? {} : { params }),
  };
}
const askzap = mkDef("askzap", [{ key: "damage", base: 3, radiant: 6, better: "up", step: 1, min: 1 }]);
const twoask = mkDef("twoask", [{ key: "damage", base: 5, radiant: 10, better: "up", step: 1, min: 1 }]);
const needer = mkDef("needer", [{ key: "damage", base: 3, radiant: 6, better: "up", step: 1, min: 1 }]);
const twoask7 = mkDef("twoask7", [{ key: "damage", base: 7, radiant: 14, better: "up", step: 1, min: 1 }]);
const EXTRA: Record<string, CardScripts> = {
  [askzap.id]: {
    base: {
      activations: [
        {
          id: "az",
          label: "Ask then zap",
          uses: 1,
          run: (ctx) => [askController("asked"), damage({ to: { of: "selfHero" } as never, amount: param(ctx, "damage") })],
        },
      ],
      resume: { asked: (ctx) => [note(`answered:${param(ctx, "damage")}`)] },
    },
    radiant: { activations: [] },
  },
  [needer.id]: {
    base: {
      activations: [
        {
          id: "need",
          label: "Need {damage}",
          uses: 1,
          canActivate: ({ state, self, radiant }) => param({ state, self, radiant }, "damage") === 3,
          run: (ctx) => [note(`need:${param(ctx, "damage")}`)],
        },
      ],
    },
    radiant: { activations: [] },
  },
  [twoask7.id]: {
    base: {
      activations: [
        {
          id: "az",
          label: "Ask then zap",
          uses: 1,
          run: (ctx) => [askController("asked"), note(`tail:${param(ctx, "damage")}`)],
        },
      ],
      resume: { asked: (ctx) => [note(`answered:${param(ctx, "damage")}`)] },
    },
    radiant: { activations: [] },
  },
  [twoask.id]: {
    base: {
      activations: [
        {
          id: "az",
          label: "Ask then zap",
          uses: 1,
          run: (ctx) => [askController("asked"), note(`tail:${param(ctx, "damage")}`)],
        },
      ],
      resume: { asked: (ctx) => [note(`answered:${param(ctx, "damage")}`)] },
    },
    radiant: { activations: [] },
  },
};

function register(): void {
  registerCatalog({ ...activateCatalog(registeredCatalog()), [askzap.id]: askzap, [twoask.id]: twoask, [twoask7.id]: twoask7, [needer.id]: needer });
  registerScripts({ ...registeredScripts(), ...ACTIVATE_SCRIPTS, ...EXTRA });
}
let nonce = 0;
function act(state: GameState, body: ActionInput): { state: GameState; events: GameEvent[] } {
  nonce += 1;
  const r = reduce(state, { ...body, nonce: `zz${nonce}` } as Action);
  if (r.error !== undefined) throw new Error(r.error);
  return { state: r.state, events: r.events };
}
function playing(seed: string): GameState {
  let state = newGame(`zz-${seed}`);
  register();
  state = beginGame(state).state;
  for (const player of ["p1", "p2"] as const) {
    state = act(state, { type: "mulligan", keep: state.players[player].hand.map((c) => c.id), playerId: player }).state;
  }
  put(state, logCard.id, slot("p2", "backrow", LOG_LANE));
  state.players.p1.autoEndTurn = false;
  state.players.p2.autoEndTurn = false;
  return state;
}
const activate = (player: PlayerId, instanceId: string, extra: Record<string, unknown> = {}): ActionInput =>
  ({ type: "activate", instanceId, playerId: player, ...extra }) as ActionInput;
function answer(state: GameState) {
  const p = state.pending;
  if (p === null) throw new Error("no prompt");
  const o = p.options[0]!;
  return act(state, { type: "answer", choiceId: p.id, selection: [o.selection], playerId: p.playerId });
}
function hand(state: GameState, defId: string) {
  const [c] = inHand(state, defId, "p1");
  if (c === undefined) throw new Error("no card");
  return c;
}

describe("scratch review", () => {
  it("memoryOfPart inverts rerootRemembered (flat and nested)", () => {
    const m: Record<string, unknown> = {};
    rememberOn(m, {}, "k", "own");
    rememberOn(m, {}, "j", "jj");
    m.engine = 1;
    rerootRemembered(m, 1);
    expect(memoryOfPart(m, 1)).toEqual({ k: "own", j: "jj", engine: 1, __remembered: ["k", "j"] });
    rerootRemembered(m, 2);
    // k@2.1
    expect(m["k@2.1"]).toBe("own");
    expect(memoryOfPart(memoryOfPart(m, 2), 1)).toEqual({ k: "own", j: "jj", engine: 1, __remembered: ["k", "j"] });
    expect(memoryOfPart(memoryOfPart(m, 2), 0).k).toBeUndefined();
    expect(memoryOfPart(m, 0).k).toBeUndefined();
  });

  it("nested fusion: keeper kept twice still activates and reads", () => {
    const state = playing("nested-keeper");
    const card = put(state, keeper.id, slot("p1", "backrow", 1));
    rememberOn(card.memory, {}, KEEPER_KEY, "deep");
    fuse(sinkFor(state), { ingredients: [hand(state, endless.id)], target: card });
    fuse(sinkFor(state), { ingredients: [hand(state, spark.id)], target: card });
    expect(card.memory[`${KEEPER_KEY}@1.1`]).toBe("deep");
    expect(abilitiesOf(state, card).map((d) => d.id)).toEqual(["again", "recall"]);
    expect(whyCannotActivateAbility(state, "p1", card.id, "recall")).toBeNull();
    expect(notes(act(state, activate("p1", card.id, { ability: "recall" })).state)).toEqual(["recall:deep"]);
  });

  it("nested fusion of two keepers + endless: only the one that remembered is activatable", () => {
    const state = playing("nested-keepers");
    const card = put(state, keeper.id, slot("p1", "backrow", 1));
    rememberOn(card.memory, {}, KEEPER_KEY, "mine");
    fuse(sinkFor(state), { ingredients: [hand(state, keeper.id)], target: card });
    fuse(sinkFor(state), { ingredients: [hand(state, endless.id)], target: card });
    expect(abilitiesOf(state, card).map((d) => d.id)).toEqual(["again", "recall", "recall#2"]);
    expect(whyCannotActivateAbility(state, "p1", card.id, "recall")).not.toBeNull();
    expect(whyCannotActivateAbility(state, "p1", card.id, "recall#2")).toBeNull();
    expect(notes(act(state, activate("p1", card.id, { ability: "recall#2" })).state)).toEqual(["recall:mine"]);
  });

  it("nested: memory written by an inner part at index 0 is read by that part only", () => {
    const state = playing("nested-part0");
    const card = put(state, keeper.id, slot("p1", "backrow", 1));
    fuse(sinkFor(state), { ingredients: [hand(state, keeper.id)], target: card });
    fuse(sinkFor(state), { ingredients: [hand(state, endless.id)], target: card });
    // as if the inner ingredient 0's text (path [1,0]) remembered
    rememberOn(card.memory, { [PART_KEY]: [1, 0] }, KEEPER_KEY, "zero");
    expect(whyCannotActivateAbility(state, "p1", card.id, "recall")).toBeNull();
    expect(whyCannotActivateAbility(state, "p1", card.id, "recall#2")).not.toBeNull();
    expect(notes(act(state, activate("p1", card.id, { ability: "recall" })).state)).toEqual(["recall:zero"]);
  });

  it("fused zapper nested reads own param", () => {
    const state = playing("nested-zap");
    const card = put(state, zapper.id, slot("p1", "backrow", 1));
    fuse(sinkFor(state), { ingredients: [hand(state, spark.id)], target: card });
    fuse(sinkFor(state), { ingredients: [hand(state, spark.id)], target: card });
    const after = act(state, activate("p1", card.id, { ability: "zap", targets: [{ pick: "hero", player: "p2" }] })).state;
    expect(after.players.p2.hero.health).toBe(HERO_HEALTH - 3);
  });

  it("resume at index 1: tail and answered step read own param", () => {
    const state = playing("resume-zap");
    const card = put(state, twoask.id, slot("p1", "backrow", 1));
    fuse(sinkFor(state), { ingredients: [hand(state, spark.id)], target: card });
    const paused = act(state, activate("p1", card.id, { ability: "az" })).state;
    const rt = JSON.parse(JSON.stringify(paused)) as GameState;
    const live = answer(paused).state;
    expect(notes(live)).toEqual(["answered:5", "tail:5"]);
    expect(hashState(answer(rt).state)).toBe(hashState(live));
  });

  it("resume nested (twoask + twoask, then spark): answered step and tail go to the right inner part", () => {
    const state = playing("resume-nested");
    const card = put(state, twoask.id, slot("p1", "backrow", 1));
    fuse(sinkFor(state), { ingredients: [hand(state, twoask.id)], target: card });
    fuse(sinkFor(state), { ingredients: [hand(state, spark.id)], target: card });
    expect(abilitiesOf(state, card).map((d) => d.id)).toEqual(["az", "az#2"]);
    const paused = act(state, activate("p1", card.id, { ability: "az#2" })).state;
    console.log("work", JSON.stringify(paused.work.map((w) => w.resume)));
    console.log("pending resume", JSON.stringify((paused.pending as unknown as { resume: unknown }).resume));
    const live = answer(paused).state;
    console.log("notes", notes(live));
    // each answered once: the step runs once
    expect(notes(live).filter((n) => n.startsWith("answered")).length).toBe(1);
  });

  it("nested resume where two inner ingredients share a step name: which one answers?", () => {
    const state = playing("resume-nested-asker");
    const card = put(state, asker.id, slot("p1", "backrow", 1));
    fuse(sinkFor(state), { ingredients: [hand(state, asker.id)], target: card });
    fuse(sinkFor(state), { ingredients: [hand(state, endless.id)], target: card });
    console.log(abilitiesOf(state, card).map((d) => d.id));
    const paused = act(state, activate("p1", card.id, { ability: "ask#2" })).state;
    const live = answer(paused).state;
    console.log("notes asker", notes(live));
    expect(notes(live)).toEqual(["ask", "answered", "ask:tail"]);
  });

  it("nested resume, inner ingredients with same step name and different params", () => {
    const state = playing("resume-nested-7");
    const card = put(state, twoask.id, slot("p1", "backrow", 1));
    fuse(sinkFor(state), { ingredients: [hand(state, twoask7.id)], target: card });
    // flat first: az (7, index 0), az#2 (5, index 1)
    const flat = JSON.parse(JSON.stringify(state)) as GameState;
    const flatLive = answer(act(flat, activate("p1", card.id, { ability: "az#2" })).state).state;
    console.log("FLAT notes", notes(flatLive));
    fuse(sinkFor(state), { ingredients: [hand(state, spark.id)], target: card });
    expect(abilitiesOf(state, card).map((d) => d.id)).toEqual(["az", "az#2"]);
    const paused = act(state, activate("p1", card.id, { ability: "az#2" })).state;
    console.log("NESTED pending resume", JSON.stringify((paused.pending as unknown as { resume: unknown }).resume));
    const live = answer(paused).state;
    console.log("NESTED notes", notes(live));
    expect(notes(live)).toEqual(["answered:5", "tail:5"]);
  });

  it("fused turtle: activationPaid survives the wrapper, with a cost that pauses (mourner)", () => {
    const state = playing("fused-turtle");
    const card = put(state, turtle.id, slot("p1", "units", 1));
    const victim = put(state, mourner.id, slot("p1", "units", 2));
    const big = put(state, sentry.id, slot("p1", "units", 3));
    fuse(sinkFor(state), { ingredients: [hand(state, spark.id)], target: card });
    expect(abilitiesOf(state, card).map((d) => d.id)).toEqual(["eat"]);
    const paused = act(state, activate("p1", card.id, { ability: "eat", tributes: [victim.id], targets: [{ pick: "hero", player: "p2" }] })).state;
    expect(paused.pending).not.toBeNull();
    const rt = JSON.parse(JSON.stringify(paused)) as GameState;
    const live = answer(paused).state;
    expect(live.players.p2.hero.health).toBe(HERO_HEALTH - 1);
    expect(hashState(answer(rt).state)).toBe(hashState(live));
    const s2 = act(live, { type: "endTurn", playerId: "p1" } as ActionInput).state;
    void s2; void big;
  });

  it("fused lockdown (tributeSelf): runs from the graveyard in its place", () => {
    const state = playing("fused-lock");
    const card = put(state, lockdown.id, slot("p1", "backrow", 1));
    fuse(sinkFor(state), { ingredients: [hand(state, spark.id)], target: card });
    const after = act(state, activate("p1", card.id, { ability: "leave" })).state;
    console.log("LOCK notes", notes(after));
    expect(notes(after)).toEqual(["left:graveyard"]);
  });

  it("canActivate reading param on a fused card reads the first-declaring ingredient's number", () => {
    const state = playing("needer");
    const card = put(state, needer.id, slot("p1", "backrow", 1));
    expect(whyCannotActivateAbility(state, "p1", card.id, "need")).toBeNull();
    fuse(sinkFor(state), { ingredients: [hand(state, spark.id)], target: card });
    expect({ view: paramsView(state, card), why: whyCannotActivateAbility(state, "p1", card.id, "need") }).toEqual({ view: { damage: 3 }, why: null });
  });
});
