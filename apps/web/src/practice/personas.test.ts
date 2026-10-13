// R645 (SPEC §9.9, §6): cosmetic persona chatter cannot affect game state, replays, or decisions.
// Table-driven tests cover configuration boundaries, session rules, and redacted human views.
// R1344 starts pools with §6 emotes, adds MN03 emoji, and limits rolls to the dealt hand.
// R643's `emoteGate` judges send time; blocked emotes are dropped, never queued.

import { describe, expect, it } from "vitest";

import type { EmoteId, GameEvent, PlayerId, PlayerView, SideView, UnitView } from "@jackioh/shared";
import { EMOTE_COOLDOWN_MS, EMOTE_IDS, opponentOf } from "@jackioh/shared";
import {
  AI_EMOTE,
  AI_PERSONAS,
  EMOTE_REPLY_KEYS,
  EMOTE_TRIGGERS,
  createEmotePersona,
  pickPersona,
  replyKeyOf,
  rollForReply,
  rollForTrigger,
  type AiEmote,
  type EmotePersona,
  type EmoteReplyKey,
  type EmoteTrigger,
  type PersonaName,
} from "./personas.ts";

// Scripted draws

const low: () => number = () => 0;

function seq(...draws: number[]): () => number {
  let i = 0;
  return () => draws[Math.min(i++, draws.length - 1)] ?? 0;
}

function exact(...draws: number[]): () => number {
  let i = 0;
  return () => {
    const draw = draws[i++];
    if (draw === undefined) throw new Error(`draw #${i} was not scripted`);
    return draw;
  };
}

// Minimal watching-client views

const SEAT: PlayerId = "p1"; // the AI
const FOE: PlayerId = opponentOf(SEAT); // the human — the view's own seat

function unit(owner: PlayerId, attack: number): UnitView {
  return {
    instanceId: `u-${owner}-${attack}`,
    defId: "core-004",
    radiant: false,
    cost: 1,
    owner,
    controller: owner,
    attack,
    maxHealth: 2,
    health: 2,
    keywords: [],
    armor: 0,
    position: "ATK",
    counters: {},
    buried: 0,
    canAct: true,
  };
}

function side(player: PlayerId, over: Partial<SideView> = {}): SideView {
  return {
    player,
    hero: { health: 30, armor: 0, powers: [], power: null },
    modifiers: [],
    mana: { current: 0, max: 0 },
    hand: { count: 0 },
    libraryCount: 0,
    graveyard: [],
    exile: [],
    resolving: [],
    units: [null, null, null, null, null],
    backrow: [null, null, null, null, null],
    locks: { units: [false, false, false, false, false], backrow: [false, false, false, false, false] },
    reserved: { units: [false, false, false, false, false], backrow: [false, false, false, false, false] },
    fatigueCount: 0,
    ...over,
  };
}

/** The human's view of an unremarkable main phase; `over` moves whatever the test moves. */
function view(over: Partial<PlayerView> = {}): PlayerView {
  return {
    viewer: FOE,
    turn: 4,
    active: FOE,
    phase: "main",
    you: side(FOE),
    opponent: side(SEAT),
    pending: null,
    events: [],
    result: null,
    clockMs: null,
    ...over,
  };
}

/** The four heroes' healths a turnStartAhead check reads: [AI, human]. */
function heroSides(aiHealth: number, humanHealth: number): Pick<PlayerView, "you" | "opponent"> {
  return {
    you: side(FOE, { hero: { health: humanHealth, armor: 0, powers: [], power: null } }),
    opponent: side(SEAT, { hero: { health: aiHealth, armor: 0, powers: [], power: null } }),
  };
}

const turnStarted = (player: PlayerId, turn: number): GameEvent => ({ type: "turnStarted", player, turn });

const heroHit = (player: PlayerId, amount: number = AI_EMOTE.bigHit): GameEvent => ({
  type: "damage",
  sourceId: "u-src",
  targetId: `hero-${player}`,
  amount,
  combat: true,
});

const unitDied = (controller: PlayerId, attack: number, killerId: string | null = "u-killer"): GameEvent => ({
  type: "destroyed",
  instanceId: `dead-${controller}-${attack}`,
  defId: "core-004",
  owner: controller,
  controller,
  attack,
  maxHealth: 2,
  killerId,
});

const healthLost = (player: PlayerId, amount: number): GameEvent => ({ type: "healthLost", player, amount });
const fatigue = (player: PlayerId, amount: number): GameEvent => ({ type: "fatigue", player, count: 1, amount });
const gameOver = (winner: PlayerId | "draw"): GameEvent => ({ type: "gameOver", winner, reason: "concede" });

const asIntents = (out: readonly AiEmote[]): { emote: EmoteId; delayMs: number }[] =>
  out.map(({ emote, delayMs }) => ({ emote, delayMs }));

const PERSONAS = ["balanced", "polite", "bm", "silent"] as const satisfies readonly PersonaName[];

const EMOTE_FOR_KEY: Record<EmoteReplyKey, EmoteId> = {
  greetings: "greetings",
  compliment: "wellPlayed",
  taunt: "threaten",
  apology: "oops",
};

function session(persona: PersonaName, rng: () => number): EmotePersona {
  return createEmotePersona({ persona, seat: SEAT, rng });
}

const MIN_DELAY = { emote: "greetings", delayMs: AI_EMOTE.delayMinMs } as const;

// The deal

describe("R645 pickPersona", () => {
  it("R645 deals balanced below 0.4, polite below 0.65, bm below 0.85 and silent for the rest", () => {
    const deals: [number, PersonaName][] = [
      [0, "balanced"],
      [0.3999, "balanced"],
      [0.4, "polite"],
      [0.6499, "polite"],
      [0.65, "bm"],
      [0.8499, "bm"],
      [0.85, "silent"],
      [0.999, "silent"],
      [1, "silent"],
    ];
    for (const [draw, expected] of deals) {
      expect(pickPersona(() => draw), `draw ${draw}`).toBe(expected);
    }
  });

  it("R645 the weights are §6's 40/25/20/15 in deal order and sum to one", () => {
    expect(PERSONAS.map((persona) => AI_PERSONAS[persona].weight)).toEqual([0.4, 0.25, 0.2, 0.15]);
    expect(PERSONAS.reduce((sum, persona) => sum + AI_PERSONAS[persona].weight, 0)).toBeCloseTo(1, 10);
  });
});

// Trigger table

describe("R645 the trigger table", () => {
  for (const persona of PERSONAS) {
    for (const trigger of EMOTE_TRIGGERS) {
      const spec = AI_PERSONAS[persona].triggers[trigger];
      if (spec === undefined) {
        it(`R645 ${persona} × ${trigger}: a dash in the table rolls nothing — and never even draws`, () => {
          expect(rollForTrigger(persona, trigger, exact())).toBeNull();
        });
      } else {
        it(`R645 ${persona} × ${trigger}: a hit sends the row's pool first entry after delayMinMs`, () => {
          expect(rollForTrigger(persona, trigger, exact(0, 0, 0))).toEqual({
            emote: spec.pool[0],
            delayMs: AI_EMOTE.delayMinMs,
          });
        });
        it(`R645 ${persona} × ${trigger}: a high pool draw still lands in the pool, inside the delay window`, () => {
          const roll = rollForTrigger(persona, trigger, exact(0, 0.9999, 0.9999));
          expect(roll).not.toBeNull();
          expect(spec.pool).toContain(roll?.emote);
          expect(roll?.emote).toBe(spec.pool.at(-1));
          expect(roll?.delayMs).toBeGreaterThanOrEqual(AI_EMOTE.delayMinMs);
          expect(roll?.delayMs).toBeLessThanOrEqual(AI_EMOTE.delayMaxMs);
        });
        it(`R645 ${persona} × ${trigger}: the chance ${spec.chance} is a hard boundary — under hits, at-or-over misses`, () => {
          expect(rollForTrigger(persona, trigger, exact(spec.chance - 0.0001, 0, 0))).toEqual({
            emote: spec.pool[0],
            delayMs: AI_EMOTE.delayMinMs,
          });
          expect(rollForTrigger(persona, trigger, exact(spec.chance))).toBeNull();
          expect(rollForTrigger(persona, trigger, exact(1))).toBeNull();
        });
      }
    }
  }

  it("R645 silent sends nothing ever — no trigger, no reply, whatever the draw", () => {
    for (const trigger of EMOTE_TRIGGERS) {
      expect(rollForTrigger("silent", trigger, exact())).toBeNull();
    }
    for (const emote of EMOTE_IDS) {
      expect(rollForReply("silent", emote, exact())).toBeNull();
    }
  });
});

// Reply table

describe("R645 the reply table", () => {
  it("R645 R1344 each emote of the pool lands on its reply row — a yawn, a shrug, a thinking face and a gasp on none", () => {
    const expected: Record<EmoteId, EmoteReplyKey | null> = {
      greetings: "greetings",
      wellPlayed: "compliment",
      thanks: "compliment",
      threaten: "taunt",
      laugh: "taunt",
      wahWah: "taunt",
      angry: "taunt",
      oops: "apology",
      sob: "apology",
      yawn: null,
      wave: "greetings",
      clap: "compliment",
      thumbsUp: "compliment",
      heart: "compliment",
      salute: "compliment",
      fire: "taunt",
      skull: "taunt",
      cool: "taunt",
      party: "taunt",
      facepalm: "apology",
      sweat: "apology",
      shrug: null,
      thinking: null,
      gasp: null,
    };
    expect(EMOTE_IDS).toHaveLength(24);
    for (const emote of EMOTE_IDS) {
      expect(replyKeyOf(emote), emote).toBe(expected[emote]);
    }
  });

  for (const persona of PERSONAS) {
    for (const key of EMOTE_REPLY_KEYS) {
      const spec = AI_PERSONAS[persona].replies[key];
      const emote = EMOTE_FOR_KEY[key];
      if (spec === undefined) {
        it(`R645 ${persona} × ${key}: no reply row, no reply — and no draw spent`, () => {
          expect(rollForReply(persona, emote, exact())).toBeNull();
        });
      } else {
        it(`R645 ${persona} × ${key}: a hit answers from the pool, the key recorded on the roll`, () => {
          expect(rollForReply(persona, emote, exact(0, 0, 0))).toEqual({
            emote: spec.pool[0],
            delayMs: AI_EMOTE.delayMinMs,
            key,
          });
        });
        it(`R645 ${persona} × ${key}: a draw just under the chance ${spec.chance} still answers from the pool's far end`, () => {
          const roll = rollForReply(persona, emote, exact(spec.chance - 0.0001, 0.9999, 0.9999));
          expect(roll).not.toBeNull();
          expect(spec.pool).toContain(roll?.emote);
          expect(roll?.emote).toBe(spec.pool.at(-1));
          expect(roll?.key).toBe(key);
          expect(roll?.delayMs).toBeGreaterThanOrEqual(AI_EMOTE.delayMinMs);
          expect(roll?.delayMs).toBeLessThanOrEqual(AI_EMOTE.delayMaxMs);
        });
        it(`R645 ${persona} × ${key}: a draw at the chance misses`, () => {
          expect(rollForReply(persona, emote, exact(spec.chance))).toBeNull();
        });
      }
    }
  }

  it("R645 a yawn earns no reply row for any persona", () => {
    for (const persona of PERSONAS) {
      expect(rollForReply(persona, "yawn", exact())).toBeNull();
    }
  });
});

// Polite pools

describe("R645 polite's pools", () => {
  it("R645 R1344 polite never picks a taunt emote — no threaten, laugh, wahWah, yawn, angry, fire, skull, cool or party anywhere it may draw", () => {
    const offLimits: readonly EmoteId[] = ["threaten", "laugh", "wahWah", "yawn", "angry", "fire", "skull", "cool", "party"];
    const pools = [...Object.values(AI_PERSONAS.polite.triggers), ...Object.values(AI_PERSONAS.polite.replies)].map(
      (row) => row.pool,
    );
    expect(pools.length).toBeGreaterThan(0);
    for (const pool of pools) {
      for (const emote of offLimits) {
        expect(pool, `pool [${pool.join(", ")}] must not hold ${emote}`).not.toContain(emote);
      }
    }
  });
});

// Session trigger detection

describe("R645 the session's trigger detection", () => {
  it("R645 the mulligan's end sends the greeting — balanced takes it every time", () => {
    const s = session("balanced", low);
    const prev = view({ phase: "mulligan", mulligan: { youReady: true, opponentReady: true } });
    const next = view({ active: FOE, turn: 1 });
    expect(s.onEvents([], prev, next, 1_000)).toEqual([MIN_DELAY]);
  });

  it("R645 the first snapshot is not a mulligan end — the window must have been open to close", () => {
    const s = session("balanced", low);
    const open = view({ phase: "mulligan", mulligan: { youReady: false, opponentReady: true } });
    expect(s.onEvents([], null, open, 1_000)).toEqual([]);
    expect(s.onEvents([], null, view({ active: FOE, turn: 1 }), 2_000)).toEqual([]);
    const prev = view({ phase: "mulligan", mulligan: { youReady: true, opponentReady: true } });
    const next = view({ active: FOE, turn: 1 });
    expect(s.onEvents([], prev, next, 3_000)).toEqual([MIN_DELAY]);
    expect(s.onEvents([], next, view({ active: FOE, turn: 2 }), 13_000)).toEqual([]);
  });

  it("R645 the AI's turnStarted while ahead rolls turnStartAhead — on health or on units", () => {
    const s = session("balanced", low);
    const healthAhead = view({ active: SEAT, turn: 5, ...heroSides(30, 30 - AI_EMOTE.leadHealth) });
    expect(s.onEvents([turnStarted(SEAT, 5)], null, healthAhead, 1_000)).toEqual([
      { emote: "threaten", delayMs: AI_EMOTE.delayMinMs },
    ]);
    const unitAhead = view({
      active: SEAT,
      turn: 7,
      opponent: side(SEAT, { units: [unit(SEAT, 2), unit(SEAT, 2), unit(SEAT, 2), null, null] }),
    });
    expect(s.onEvents([turnStarted(SEAT, 7)], null, unitAhead, 11_000)).toEqual([
      { emote: "threaten", delayMs: AI_EMOTE.delayMinMs },
    ]);
  });

  it("R645 no lead, no turnStartAhead — and the human's turnStarted is only the per-turn reset", () => {
    const s = session("balanced", low);
    const even = view({ active: SEAT, turn: 5 });
    expect(s.onEvents([turnStarted(SEAT, 5)], null, even, 1_000)).toEqual([]);
    const justShort = view({
      active: SEAT,
      turn: 6,
      ...heroSides(30, 30 - AI_EMOTE.leadHealth + 1),
    });
    expect(s.onEvents([turnStarted(SEAT, 6)], null, justShort, 11_000)).toEqual([]);
    const humanTurnAhead = view({ active: FOE, turn: 7, ...heroSides(30, 1) });
    expect(s.onEvents([turnStarted(FOE, 7)], null, humanTurnAhead, 21_000)).toEqual([]);
  });

  it("R645 a big hit on the human's hero rolls dealtBigHit; under bigHit or off a hero it rolls nothing", () => {
    const v = view({ active: SEAT, turn: 5 });
    expect(session("balanced", low).onEvents([heroHit(FOE)], null, v, 1_000)).toEqual([
      { emote: "laugh", delayMs: AI_EMOTE.delayMinMs },
    ]);
    expect(
      session("balanced", low).onEvents([heroHit(FOE, AI_EMOTE.bigHit - 1)], null, v, 1_000),
    ).toEqual([]);
    expect(
      session("balanced", low).onEvents(
        [{ type: "damage", sourceId: "u-a", targetId: "u-b", amount: AI_EMOTE.bigHit, combat: true }],
        null,
        v,
        1_000,
      ),
    ).toEqual([]);
  });

  it("R645 a big hit on the AI's hero rolls tookBigHit — and playerBigPlay off the same hit where the row exists", () => {
    const v = view({ active: FOE, turn: 5 });
    expect(session("balanced", low).onEvents([heroHit(SEAT)], null, v, 1_000)).toEqual([
      { emote: "oops", delayMs: AI_EMOTE.delayMinMs },
    ]);
    const polite = session("polite", seq(0.35, 0, 0, 0));
    expect(polite.onEvents([heroHit(SEAT)], null, v, 1_000)).toEqual([
      { emote: "wellPlayed", delayMs: AI_EMOTE.delayMinMs },
    ]);
  });

  it("R645 a fat healthLost or fatigue hit on the AI counts as tookBigHit too — on the human it does not", () => {
    const v = view({ active: FOE, turn: 5 });
    const oops = [{ emote: "oops" as const, delayMs: AI_EMOTE.delayMinMs }];
    expect(session("balanced", low).onEvents([healthLost(SEAT, AI_EMOTE.bigHit)], null, v, 1_000)).toEqual(oops);
    expect(session("balanced", low).onEvents([fatigue(SEAT, AI_EMOTE.bigHit)], null, v, 1_000)).toEqual(oops);
    expect(
      session("balanced", low).onEvents(
        [healthLost(FOE, AI_EMOTE.bigHit), fatigue(FOE, AI_EMOTE.bigHit)],
        null,
        v,
        1_000,
      ),
    ).toEqual([]);
    expect(
      session("balanced", low).onEvents(
        [healthLost(SEAT, AI_EMOTE.bigHit - 1), fatigue(SEAT, AI_EMOTE.bigHit - 1)],
        null,
        v,
        1_000,
      ),
    ).toEqual([]);
  });

  it("R645 the dead unit's attack must reach the previous view's top for killedTopUnit and lostTopUnit", () => {
    const prev = view({
      you: side(FOE, { units: [unit(FOE, 8), unit(FOE, 2), null, null, null] }),
      opponent: side(SEAT, { units: [unit(SEAT, 6), null, null, null, null] }),
    });
    const v = view({ active: SEAT, turn: 5 });
    expect(session("balanced", low).onEvents([unitDied(FOE, 8)], prev, v, 1_000)).toEqual([
      { emote: "laugh", delayMs: AI_EMOTE.delayMinMs },
    ]);
    expect(session("balanced", low).onEvents([unitDied(FOE, 7)], prev, v, 1_000)).toEqual([]);
    expect(session("balanced", low).onEvents([unitDied(SEAT, 6)], prev, v, 1_000)).toEqual([
      { emote: "sob", delayMs: AI_EMOTE.delayMinMs },
    ]);
    expect(session("balanced", low).onEvents([unitDied(SEAT, 5)], prev, v, 1_000)).toEqual([]);
    expect(session("balanced", low).onEvents([unitDied(FOE, 1)], null, v, 1_000)).toEqual([
      { emote: "laugh", delayMs: AI_EMOTE.delayMinMs },
    ]);
    expect(session("balanced", low).onEvents([unitDied(FOE, 8, null)], prev, v, 1_000)).toEqual([]);
  });

  it("R645 two of the AI's units dying on the human's turn roll playerBigPlay — once, at the second", () => {
    const s = session("polite", low);
    const humanTurn = view({ active: FOE, turn: 7 });
    const prev = view({ opponent: side(SEAT, { units: [unit(SEAT, 9), null, null, null, null] }) });

    expect(s.onEvents([unitDied(SEAT, 4)], prev, humanTurn, 1_000)).toEqual([]);
    expect(s.onEvents([unitDied(SEAT, 4)], prev, humanTurn, 11_000)).toEqual([
      { emote: "wellPlayed", delayMs: AI_EMOTE.delayMinMs },
    ]);
    expect(s.onEvents([unitDied(SEAT, 4), unitDied(SEAT, 4)], prev, humanTurn, 21_000)).toEqual([]);
    expect(s.onEvents([unitDied(SEAT, 4), unitDied(SEAT, 4)], prev, view({ active: SEAT, turn: 8 }), 31_000)).toEqual([]);
    const nextTurn = view({ active: FOE, turn: 9 });
    expect(s.onEvents([turnStarted(FOE, 9)], prev, nextTurn, 41_000)).toEqual([]);
    expect(s.onEvents([unitDied(SEAT, 4), unitDied(SEAT, 4)], prev, nextTurn, 51_000)).toEqual([
      { emote: "wellPlayed", delayMs: AI_EMOTE.delayMinMs },
    ]);
    const otherTurn = view({ active: FOE, turn: 10 });
    expect(s.onEvents([turnStarted(FOE, 10)], prev, otherTurn, 61_000)).toEqual([]);
    expect(s.onEvents([unitDied(FOE, 4), unitDied(FOE, 4)], prev, otherTurn, 71_000)).toEqual([]);
  });

  it("R645 the second-kill roll is spent whether it hits or misses", () => {
    const s = session("polite", seq(0.9));
    const humanTurn = view({ active: FOE, turn: 7 });
    const prev = view({ opponent: side(SEAT, { units: [unit(SEAT, 9), null, null, null, null] }) });
    expect(s.onEvents([unitDied(SEAT, 4), unitDied(SEAT, 4)], prev, humanTurn, 1_000)).toEqual([]);
    expect(s.onEvents([unitDied(SEAT, 4), unitDied(SEAT, 4)], prev, humanTurn, 11_000)).toEqual([]);
  });

  it("R645 gameOver sends matchWon when the seat won, matchLost when the human did, nothing on a draw", () => {
    const over = (winner: PlayerId | "draw"): PlayerView =>
      view({ phase: "over", result: { winner, reason: "concede" } });
    expect(session("balanced", low).onEvents([gameOver(SEAT)], null, over(SEAT), 1_000)).toEqual([
      { emote: "wahWah", delayMs: AI_EMOTE.delayMinMs },
    ]);
    expect(session("balanced", low).onEvents([gameOver(FOE)], null, over(FOE), 1_000)).toEqual([
      { emote: "wellPlayed", delayMs: AI_EMOTE.delayMinMs },
    ]);
    expect(session("balanced", low).onEvents([gameOver("draw")], null, over("draw"), 1_000)).toEqual([]);
  });

  it("R645 silent's session is quiet end to end — greeting, kills, lethal, replies all land nothing", () => {
    const s = session("silent", low);
    const mulligan = view({ phase: "mulligan", mulligan: { youReady: true, opponentReady: true } });
    expect(s.onEvents([], mulligan, view({ active: FOE, turn: 1 }), 1_000)).toEqual([]);
    expect(
      s.onEvents(
        [heroHit(FOE), heroHit(SEAT), unitDied(FOE, 9), unitDied(FOE, 9), unitDied(SEAT, 9)],
        null,
        view({ active: FOE, turn: 2 }),
        2_000,
      ),
    ).toEqual([]);
    expect(s.onPlayerEmote("greetings", 3_000)).toEqual([]);
    expect(s.onPlayerTurnLong(view({ active: FOE, turn: 2 }), 4_000)).toEqual([]);
    expect(
      s.onEvents([gameOver(SEAT)], null, view({ phase: "over", result: { winner: SEAT, reason: "concede" } }), 5_000),
    ).toEqual([]);
  });

  it("R645 a player's long turn rolls playerTurnLong — once per turn, never on the AI's own", () => {
    const s = session("balanced", low);
    const yawn = [{ emote: "yawn" as const, delayMs: AI_EMOTE.delayMinMs }];
    const humanTurn = (turn: number): PlayerView => view({ active: FOE, turn });
    expect(s.onPlayerTurnLong(humanTurn(7), 1_000)).toEqual(yawn);
    expect(s.onPlayerTurnLong(humanTurn(7), 2_000)).toEqual([]);
    expect(s.onPlayerTurnLong(view({ active: SEAT, turn: 8 }), 3_000)).toEqual([]);
    s.onEvents([turnStarted(FOE, 9)], null, humanTurn(9), 4_000);
    expect(s.onPlayerTurnLong(humanTurn(9), 5_000)).toEqual(yawn);
  });
});

// Session replies

describe("R645 the session's replies", () => {
  it("R645 a player's emote can earn a reply, each reply rule at most once a match", () => {
    const s = session("balanced", low);
    const humanTurn = (turn: number): PlayerView => view({ active: FOE, turn });
    s.onEvents([turnStarted(FOE, 3)], null, humanTurn(3), 1_000);
    expect(asIntents(s.onPlayerEmote("greetings", 11_000))).toEqual([MIN_DELAY]);
    expect(s.onPlayerEmote("greetings", 21_000)).toEqual([]);
    s.onEvents([turnStarted(FOE, 4)], null, humanTurn(4), 31_000);
    expect(asIntents(s.onPlayerEmote("wellPlayed", 41_000))).toEqual([
      { emote: "thanks", delayMs: AI_EMOTE.delayMinMs },
    ]);
    s.onEvents([turnStarted(FOE, 5)], null, humanTurn(5), 51_000);
    expect(s.onPlayerEmote("thanks", 61_000)).toEqual([]);
    s.onEvents([turnStarted(FOE, 6)], null, humanTurn(6), 71_000);
    expect(asIntents(s.onPlayerEmote("threaten", 81_000))).toEqual([
      { emote: "laugh", delayMs: AI_EMOTE.delayMinMs },
    ]);
    s.onEvents([turnStarted(FOE, 7)], null, humanTurn(7), 91_000);
    expect(s.onPlayerEmote("oops", 101_000)).toEqual([]);
    expect(s.onPlayerEmote("yawn", 102_000)).toEqual([]);
  });

  it("R645 a missed roll does not spend the rule — the same emote can still earn the reply later", () => {
    const s = session("balanced", seq(0.7, 0, 0, 0));
    s.onEvents([turnStarted(FOE, 3)], null, view({ active: FOE, turn: 3 }), 1_000);
    expect(s.onPlayerEmote("greetings", 11_000)).toEqual([]);
    expect(asIntents(s.onPlayerEmote("greetings", 21_000))).toEqual([MIN_DELAY]);
    expect(s.onPlayerEmote("greetings", 31_000)).toEqual([]);
  });
});

// Session caps and rate limit

describe("R645 caps and the shared gate", () => {
  it("R645 the caps live in config: balanced 1/1/8, polite 1/1/6, bm 2/2/20, silent 0/0/0", () => {
    expect(AI_PERSONAS.balanced.caps).toEqual({ ownTurn: 1, otherTurn: 1, match: 8 });
    expect(AI_PERSONAS.polite.caps).toEqual({ ownTurn: 1, otherTurn: 1, match: 6 });
    expect(AI_PERSONAS.bm.caps).toEqual({ ownTurn: 2, otherTurn: 2, match: 20 });
    expect(AI_PERSONAS.silent.caps).toEqual({ ownTurn: 0, otherTurn: 0, match: 0 });
  });

  it("R645 balanced's turn caps are one emote on each side's turn, refilled as the turns move", () => {
    const s = session("balanced", low);
    const aiTurn = view({ active: SEAT, turn: 5 });
    expect(s.onEvents([turnStarted(SEAT, 5), heroHit(FOE)], null, aiTurn, 1_000)).toEqual([
      { emote: "laugh", delayMs: AI_EMOTE.delayMinMs },
    ]);
    expect(s.onEvents([heroHit(FOE)], aiTurn, aiTurn, 11_000)).toEqual([]);
    const aiTurn6 = view({ active: SEAT, turn: 6 });
    expect(s.onEvents([turnStarted(SEAT, 6), heroHit(FOE)], aiTurn, aiTurn6, 21_000)).toEqual([
      { emote: "laugh", delayMs: AI_EMOTE.delayMinMs },
    ]);
    const humanTurn7 = view({ active: FOE, turn: 7 });
    expect(s.onEvents([turnStarted(FOE, 7), heroHit(SEAT)], aiTurn6, humanTurn7, 31_000)).toEqual([
      { emote: "oops", delayMs: AI_EMOTE.delayMinMs },
    ]);
    expect(s.onEvents([heroHit(SEAT)], humanTurn7, humanTurn7, 41_000)).toEqual([]);
  });

  it("R645 bm gets two a turn — the third trigger of the same turn is the cap's", () => {
    const s = session("bm", low);
    const aiTurn = view({ active: SEAT, turn: 5 });
    expect(s.onEvents([turnStarted(SEAT, 5), heroHit(FOE)], null, aiTurn, 1_000)).toEqual([
      { emote: "laugh", delayMs: AI_EMOTE.delayMinMs },
    ]);
    expect(s.onEvents([heroHit(FOE)], aiTurn, aiTurn, 11_000)).toEqual([
      { emote: "laugh", delayMs: AI_EMOTE.delayMinMs },
    ]);
    expect(s.onEvents([heroHit(FOE)], aiTurn, aiTurn, 21_000)).toEqual([]);
  });

  it("R645 the match cap counts only the non-exempt emotes — the greeting and the end emotes land past it", () => {
    const s = session("balanced", low);
    let now = 0;
    const step = (): number => (now += 10_000);
    const mulligan = view({ phase: "mulligan", mulligan: { youReady: true, opponentReady: true } });

    expect(s.onEvents([], mulligan, view({ active: FOE, turn: 1 }), step())).toEqual([MIN_DELAY]);
    const seats: PlayerId[] = [SEAT, FOE, SEAT, FOE, SEAT, FOE, SEAT, FOE];
    seats.forEach((player, i) => {
      const v = view({ active: player, turn: i + 2 });
      expect(s.onEvents([turnStarted(player, i + 2), heroHit(FOE)], null, v, step())).toEqual([
        { emote: "laugh", delayMs: AI_EMOTE.delayMinMs },
      ]);
    });
    const turn10 = view({ active: SEAT, turn: 10 });
    expect(s.onEvents([turnStarted(SEAT, 10), heroHit(FOE)], null, turn10, step())).toEqual([]);
    expect(
      s.onEvents([], mulligan, view({ active: SEAT, turn: 10 }), step()),
    ).toEqual([MIN_DELAY]);
    expect(
      s.onEvents([gameOver(SEAT)], turn10, view({ phase: "over", result: { winner: SEAT, reason: "concede" } }), step()),
    ).toEqual([{ emote: "wahWah", delayMs: AI_EMOTE.delayMinMs }]);
  });

  it("R645 the end-of-match emotes are exempt at the flag, not only via the over phase", () => {
    const s = session("balanced", low);
    let now = 0;
    const step = (): number => (now += 10_000);
    const seats: PlayerId[] = [SEAT, FOE, SEAT, FOE, SEAT, FOE, SEAT, FOE];
    seats.forEach((player, i) => {
      const v = view({ active: player, turn: i + 2 });
      expect(s.onEvents([turnStarted(player, i + 2), heroHit(FOE)], null, v, step())).toHaveLength(1);
    });
    const turn11 = view({ active: FOE, turn: 11 });
    expect(s.onEvents([turnStarted(FOE, 11)], null, turn11, step())).toEqual([]);
    expect(s.onEvents([gameOver(FOE)], turn11, turn11, step())).toEqual([
      { emote: "wellPlayed", delayMs: AI_EMOTE.delayMinMs },
    ]);
  });

  it("R645 the shared rate limit judges the send instant (now + delayMs): a fast follow-up is dropped, a delayed one lands", () => {
    const aiTurn = view({ active: SEAT, turn: 5 });
    const fast = session("bm", low);
    expect(fast.onEvents([heroHit(FOE), heroHit(FOE)], null, aiTurn, 1_000)).toEqual([
      { emote: "laugh", delayMs: AI_EMOTE.delayMinMs },
    ]);
    expect(fast.onEvents([heroHit(FOE)], aiTurn, aiTurn, 31_000)).toEqual([
      { emote: "laugh", delayMs: AI_EMOTE.delayMinMs },
    ]);
    const spaced = session("bm", seq(0, 0, 0, 0, 0, 0.99));
    const out = spaced.onEvents([heroHit(FOE), heroHit(FOE)], null, aiTurn, 1_000);
    expect(out).toHaveLength(2);
    expect(out[1]?.emote).toBe("laugh");
    expect(out[1]?.delayMs).toBeGreaterThanOrEqual(AI_EMOTE.delayMinMs + EMOTE_COOLDOWN_MS);
  });
});

// R1344: dealt-hand choices

const SECTION_6_POOLS: Record<Exclude<PersonaName, "silent">, Partial<Record<EmoteTrigger | EmoteReplyKey, readonly EmoteId[]>>> = {
  balanced: {
    mulliganEnd: ["greetings"],
    turnStartAhead: ["threaten", "laugh", "yawn"],
    dealtBigHit: ["laugh", "threaten", "wahWah"],
    killedTopUnit: ["laugh", "wahWah"],
    tookBigHit: ["oops", "sob", "angry"],
    lostTopUnit: ["sob", "angry", "oops"],
    playerTurnLong: ["yawn"],
    matchWon: ["wahWah", "laugh", "wellPlayed"],
    matchLost: ["wellPlayed"],
    greetings: ["greetings"],
    compliment: ["thanks"],
    taunt: ["laugh", "wahWah", "yawn", "threaten"],
  },
  polite: {
    mulliganEnd: ["greetings"],
    dealtBigHit: ["oops"],
    tookBigHit: ["wellPlayed", "oops"],
    lostTopUnit: ["wellPlayed"],
    playerBigPlay: ["wellPlayed"],
    matchWon: ["wellPlayed"],
    matchLost: ["wellPlayed"],
    greetings: ["greetings"],
    compliment: ["thanks"],
    taunt: ["oops", "greetings"],
    apology: ["thanks"],
  },
  bm: {
    mulliganEnd: ["threaten", "laugh"],
    turnStartAhead: ["threaten", "laugh", "yawn", "wahWah"],
    dealtBigHit: ["laugh", "wahWah", "threaten"],
    killedTopUnit: ["laugh", "wahWah", "yawn"],
    tookBigHit: ["angry", "threaten"],
    lostTopUnit: ["angry"],
    playerTurnLong: ["yawn"],
    matchWon: ["wahWah", "laugh"],
    matchLost: ["sob", "angry"],
    greetings: ["threaten", "laugh"],
    compliment: ["yawn"],
    taunt: ["laugh", "wahWah", "yawn", "threaten"],
    apology: ["laugh", "wahWah"],
  },
};

describe("R1344 the persona chooses within its seat's dealt hand", () => {
  it("R1344 every pool is §6's own emotes first, verbatim, then emoji of the pool alone, none twice", () => {
    for (const [persona, rows] of Object.entries(SECTION_6_POOLS) as [Exclude<PersonaName, "silent">, (typeof SECTION_6_POOLS)["bm"]][]) {
      const spec = AI_PERSONAS[persona];
      const widened: Record<string, readonly EmoteId[]> = {
        ...Object.fromEntries(Object.entries(spec.triggers).map(([key, row]) => [key, row.pool])),
        ...Object.fromEntries(Object.entries(spec.replies).map(([key, row]) => [key, row.pool])),
      };
      expect(Object.keys(widened).sort(), persona).toEqual(Object.keys(rows).sort());
      for (const [key, original] of Object.entries(rows)) {
        const pool = widened[key] ?? [];
        expect(pool.slice(0, original.length), `${persona}.${key}`).toEqual(original);
        expect(new Set(pool).size, `${persona}.${key} repeats an emote`).toBe(pool.length);
        expect(pool.every((emote) => (EMOTE_IDS as readonly string[]).includes(emote)), `${persona}.${key}`).toBe(true);
      }
    }
  });

  it("R1344 a trigger roll picks only from what the hand holds, in the pool's order", () => {
    const noGreeting: readonly EmoteId[] = ["oops", "thanks", "threaten", "sob", "wave", "heart", "salute", "party"];
    expect(rollForTrigger("balanced", "mulliganEnd", low, noGreeting)).toEqual({ emote: "wave", delayMs: AI_EMOTE.delayMinMs });
    expect(rollForTrigger("balanced", "mulliganEnd", seq(0, 0.999, 0), noGreeting)?.emote).toBe("salute");
  });

  it("R1344 a hit on a row the hand holds none of sends nothing", () => {
    const quiet: readonly EmoteId[] = ["wellPlayed", "oops", "thanks", "sob", "yawn", "laugh", "angry", "wahWah"];
    expect(rollForTrigger("balanced", "mulliganEnd", low, quiet)).toBeNull();
    expect(rollForReply("polite", "greetings", low, quiet)).toBeNull();
    expect(rollForReply("polite", "greetings", low, ["wave", ...quiet.slice(1)])).toMatchObject({ emote: "wave", key: "greetings" });
  });

  it("R1344 a session made with a hand sends from that hand alone, greeting and replies included", () => {
    const hand: readonly EmoteId[] = ["wellPlayed", "oops", "thanks", "sob", "clap", "skull", "salute", "gasp"];
    const s = createEmotePersona({ persona: "balanced", seat: SEAT, rng: low, hand });
    const mulligan = view({ phase: "mulligan", mulligan: { youReady: true, opponentReady: true } });
    expect(asIntents(s.onEvents([], mulligan, view({ active: FOE, turn: 1 }), 1_000))).toEqual([
      { emote: "salute", delayMs: AI_EMOTE.delayMinMs },
    ]);
    const replies = createEmotePersona({ persona: "balanced", seat: SEAT, rng: low, hand });
    expect(replies.onPlayerEmote("wellPlayed", 60_000).map((intent) => intent.emote)).toEqual(["thanks"]);
    expect(replies.onPlayerEmote("laugh", 90_000)).toEqual([]);
  });

  it("R1344 over dealt hands, nothing any persona rolls is outside its hand", () => {
    const hands: readonly (readonly EmoteId[])[] = [
      ["greetings", "thanks", "threaten", "laugh", "wahWah", "wave", "thumbsUp", "party"],
      ["wellPlayed", "oops", "thanks", "sob", "clap", "skull", "cool", "gasp"],
      ["greetings", "oops", "threaten", "yawn", "facepalm", "shrug", "fire", "sweat"],
      ["wellPlayed", "thanks", "threaten", "angry", "thinking", "heart", "salute", "party"],
    ];
    const draws = [0, 0.13, 0.27, 0.41, 0.58, 0.72, 0.86, 0.99];
    for (const hand of hands) {
      for (const persona of PERSONAS) {
        for (const trigger of EMOTE_TRIGGERS) {
          for (const draw of draws) {
            const roll = rollForTrigger(persona, trigger, seq(0, draw, 0), hand);
            if (roll !== null) expect(hand, `${persona} ${trigger}`).toContain(roll.emote);
          }
        }
        for (const emote of EMOTE_IDS) {
          for (const draw of draws) {
            const roll = rollForReply(persona, emote, seq(0, draw, 0), hand);
            if (roll !== null) expect(hand, `${persona} reply to ${emote}`).toContain(roll.emote);
          }
        }
      }
    }
  });
});
