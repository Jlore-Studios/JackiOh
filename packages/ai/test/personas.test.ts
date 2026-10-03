// R644 (SPEC §9.9, issue #75 §6): the AI's emote personas — Balanced 40%, Polite 25%, BM 20%,
// Silent 15% — and the cosmetic chatter each produces, all of it out of the game: nothing here
// reaches `reduce`, the action log, a replay hash or a play decision.
//
// What is proved, table-first so a config change is the only edit a tuning touch should need:
//
//   - pickPersona's weighted intervals, walked at every boundary.
//   - The trigger and reply tables verbatim: for every persona × every EMOTE_TRIGGERS row and
//     every persona × every EMOTE_REPLY_KEYS row, a dash rolls nothing (and never even draws),
//     a hit sends a pool member inside AI_EMOTE's 0.8–2.5 s window, and a draw at exactly the
//     row's chance misses. Silent has no rows anywhere.
//   - replyKeyOf's grouping of the ten emotes onto the four reply rows (a yawn earns none), and
//     the issue's extra demand on Polite: none of its pools may hold a taunt emote.
//   - The session machine: trigger detection across view/event deltas (mulligan end, a turn
//     started ahead, big hits both ways, top-unit kills both ways, the big play, game over),
//     the per-turn and per-match caps with the greeting and end emotes exempt, replies firing
//     at most once per rule per match, and the shared R642 `emoteGate` judging each intent at
//     the instant it would fire (`now + delayMs`) — a blocked emote is dropped, never queued.
//   - The import isolation R644 requires: nothing in packages/engine or the move search reads
//     the emote module, so it can never sway a decision or a seeded game.
//
// Every roll's draws are scripted: `low` (0 forever) passes any chance, picks a pool's first
// entry and waits `delayMinMs`; `seq` feeds one draw per roll in the order personas.ts makes
// them (chance, then pool, then delay); `exact` throws on a draw that was not scripted, which
// pins how many draws a row spends — a dash spends none. Views are the smallest `PlayerView`s
// a watching client could show: the persona reads nothing else. The AI is SEAT, the human FOE;
// the view is the human's own, exactly as the practice driver hands it over (`you` is the
// human, `opponent` the AI).

import { readdirSync, readFileSync } from "node:fs";
import { basename, join } from "node:path";
import { fileURLToPath } from "node:url";

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
  type PersonaName,
} from "../src/index";

// ---------------------------------------------------------------------------------------------
// scripted draws
// ---------------------------------------------------------------------------------------------

/** Every roll hits its chance, picks a pool's first entry and waits `delayMinMs`. */
const low: () => number = () => 0;

/**
 * One draw per call, in order; past the end it keeps returning the last draw, so a session
 * test scripts only the draws that matter.
 */
function seq(...draws: number[]): () => number {
  let i = 0;
  return () => draws[Math.min(i++, draws.length - 1)] ?? 0;
}

/** One draw per call, in order; a draw past the end throws, so a roll's draw count is pinned. */
function exact(...draws: number[]): () => number {
  let i = 0;
  return () => {
    const draw = draws[i++];
    if (draw === undefined) throw new Error(`draw #${i} was not scripted`);
    return draw;
  };
}

// ---------------------------------------------------------------------------------------------
// the smallest views a watching client could show
// ---------------------------------------------------------------------------------------------

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

/** `onEvents`' intent list as plain {emote, delayMs} — a reply's `key` rides along but is not the wire. */
const asIntents = (out: readonly AiEmote[]): { emote: EmoteId; delayMs: number }[] =>
  out.map(({ emote, delayMs }) => ({ emote, delayMs }));

const PERSONAS = ["balanced", "polite", "bm", "silent"] as const satisfies readonly PersonaName[];

/** One emote per reply row, for the persona × key table; replyKeyOf itself is pinned on all ten. */
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

// ---------------------------------------------------------------------------------------------
// the deal
// ---------------------------------------------------------------------------------------------

describe("R644 pickPersona", () => {
  it("R644 deals balanced below 0.4, polite below 0.65, bm below 0.85 and silent for the rest", () => {
    const deals: [number, PersonaName][] = [
      [0, "balanced"],
      [0.3999, "balanced"],
      [0.4, "polite"],
      [0.6499, "polite"],
      [0.65, "bm"],
      [0.8499, "bm"],
      // A draw of exactly 0.85 is Silent: the boundaries are exact in per-mille, so the float sum
      // 0.4 + 0.25 + 0.2 landing one ulp past 0.85 cannot leak the seam into BM.
      [0.85, "silent"],
      [0.999, "silent"],
      // A hostile rng returning exactly 1 cannot pick nothing: the last interval is closed.
      [1, "silent"],
    ];
    for (const [draw, expected] of deals) {
      expect(pickPersona(() => draw), `draw ${draw}`).toBe(expected);
    }
  });

  it("R644 the weights are §6's 40/25/20/15 in deal order and sum to one", () => {
    expect(PERSONAS.map((persona) => AI_PERSONAS[persona].weight)).toEqual([0.4, 0.25, 0.2, 0.15]);
    expect(PERSONAS.reduce((sum, persona) => sum + AI_PERSONAS[persona].weight, 0)).toBeCloseTo(1, 10);
  });
});

// ---------------------------------------------------------------------------------------------
// the trigger table, every persona × every trigger
// ---------------------------------------------------------------------------------------------

describe("R644 the trigger table", () => {
  for (const persona of PERSONAS) {
    for (const trigger of EMOTE_TRIGGERS) {
      const spec = AI_PERSONAS[persona].triggers[trigger];
      if (spec === undefined) {
        it(`R644 ${persona} × ${trigger}: a dash in the table rolls nothing — and never even draws`, () => {
          expect(rollForTrigger(persona, trigger, exact())).toBeNull();
        });
      } else {
        it(`R644 ${persona} × ${trigger}: a hit sends the row's pool first entry after delayMinMs`, () => {
          expect(rollForTrigger(persona, trigger, exact(0, 0, 0))).toEqual({
            emote: spec.pool[0],
            delayMs: AI_EMOTE.delayMinMs,
          });
        });
        it(`R644 ${persona} × ${trigger}: a high pool draw still lands in the pool, inside the delay window`, () => {
          const roll = rollForTrigger(persona, trigger, exact(0, 0.9999, 0.9999));
          expect(roll).not.toBeNull();
          expect(spec.pool).toContain(roll?.emote);
          expect(roll?.emote).toBe(spec.pool.at(-1));
          expect(roll?.delayMs).toBeGreaterThanOrEqual(AI_EMOTE.delayMinMs);
          expect(roll?.delayMs).toBeLessThanOrEqual(AI_EMOTE.delayMaxMs);
        });
        it(`R644 ${persona} × ${trigger}: the chance ${spec.chance} is a hard boundary — under hits, at-or-over misses`, () => {
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

  it("R644 silent sends nothing ever — no trigger, no reply, whatever the draw", () => {
    for (const trigger of EMOTE_TRIGGERS) {
      expect(rollForTrigger("silent", trigger, exact())).toBeNull();
    }
    for (const emote of EMOTE_IDS) {
      expect(rollForReply("silent", emote, exact())).toBeNull();
    }
  });
});

// ---------------------------------------------------------------------------------------------
// the reply table
// ---------------------------------------------------------------------------------------------

describe("R644 the reply table", () => {
  it("R644 each of the ten emotes lands on its reply row — a yawn on none", () => {
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
    };
    expect(EMOTE_IDS).toHaveLength(10);
    for (const emote of EMOTE_IDS) {
      expect(replyKeyOf(emote), emote).toBe(expected[emote]);
    }
  });

  for (const persona of PERSONAS) {
    for (const key of EMOTE_REPLY_KEYS) {
      const spec = AI_PERSONAS[persona].replies[key];
      const emote = EMOTE_FOR_KEY[key];
      if (spec === undefined) {
        it(`R644 ${persona} × ${key}: no reply row, no reply — and no draw spent`, () => {
          expect(rollForReply(persona, emote, exact())).toBeNull();
        });
      } else {
        it(`R644 ${persona} × ${key}: a hit answers from the pool, the key recorded on the roll`, () => {
          expect(rollForReply(persona, emote, exact(0, 0, 0))).toEqual({
            emote: spec.pool[0],
            delayMs: AI_EMOTE.delayMinMs,
            key,
          });
        });
        it(`R644 ${persona} × ${key}: a draw just under the chance ${spec.chance} still answers from the pool's far end`, () => {
          const roll = rollForReply(persona, emote, exact(spec.chance - 0.0001, 0.9999, 0.9999));
          expect(roll).not.toBeNull();
          expect(spec.pool).toContain(roll?.emote);
          expect(roll?.emote).toBe(spec.pool.at(-1));
          expect(roll?.key).toBe(key);
          expect(roll?.delayMs).toBeGreaterThanOrEqual(AI_EMOTE.delayMinMs);
          expect(roll?.delayMs).toBeLessThanOrEqual(AI_EMOTE.delayMaxMs);
        });
        it(`R644 ${persona} × ${key}: a draw at the chance misses`, () => {
          expect(rollForReply(persona, emote, exact(spec.chance))).toBeNull();
        });
      }
    }
  }

  it("R644 a yawn earns no reply row for any persona", () => {
    for (const persona of PERSONAS) {
      expect(rollForReply(persona, "yawn", exact())).toBeNull();
    }
  });
});

// ---------------------------------------------------------------------------------------------
// the issue's extra rule for Polite
// ---------------------------------------------------------------------------------------------

describe("R644 polite's pools", () => {
  it("R644 polite never picks a taunt emote — no threaten, laugh, wahWah, yawn or angry anywhere it may draw", () => {
    const offLimits: readonly EmoteId[] = ["threaten", "laugh", "wahWah", "yawn", "angry"];
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

// ---------------------------------------------------------------------------------------------
// the session machine: trigger detection
// ---------------------------------------------------------------------------------------------

describe("R644 the session's trigger detection", () => {
  it("R644 the mulligan's end sends the greeting — balanced takes it every time", () => {
    const s = session("balanced", low);
    const prev = view({ phase: "mulligan", mulligan: { youReady: true, opponentReady: true } });
    const next = view({ active: FOE, turn: 1 });
    expect(s.onEvents([], prev, next, 1_000)).toEqual([MIN_DELAY]);
  });

  it("R644 the first snapshot is not a mulligan end — the window must have been open to close", () => {
    const s = session("balanced", low);
    const open = view({ phase: "mulligan", mulligan: { youReady: false, opponentReady: true } });
    expect(s.onEvents([], null, open, 1_000)).toEqual([]);
    expect(s.onEvents([], null, view({ active: FOE, turn: 1 }), 2_000)).toEqual([]);
    // and once it has closed, a view without it is not a new ending
    const prev = view({ phase: "mulligan", mulligan: { youReady: true, opponentReady: true } });
    const next = view({ active: FOE, turn: 1 });
    expect(s.onEvents([], prev, next, 3_000)).toEqual([MIN_DELAY]);
    expect(s.onEvents([], next, view({ active: FOE, turn: 2 }), 13_000)).toEqual([]);
  });

  it("R644 the AI's turnStarted while ahead rolls turnStartAhead — on health or on units", () => {
    const s = session("balanced", low);
    // health lead of exactly leadHealth counts (its hero − yours ≥ 10)
    const healthAhead = view({ active: SEAT, turn: 5, ...heroSides(30, 30 - AI_EMOTE.leadHealth) });
    expect(s.onEvents([turnStarted(SEAT, 5)], null, healthAhead, 1_000)).toEqual([
      { emote: "threaten", delayMs: AI_EMOTE.delayMinMs },
    ]);
    // a unit lead of leadUnits counts the same way (three more units, 3–0)
    const unitAhead = view({
      active: SEAT,
      turn: 7,
      opponent: side(SEAT, { units: [unit(SEAT, 2), unit(SEAT, 2), unit(SEAT, 2), null, null] }),
    });
    expect(s.onEvents([turnStarted(SEAT, 7)], null, unitAhead, 11_000)).toEqual([
      { emote: "threaten", delayMs: AI_EMOTE.delayMinMs },
    ]);
  });

  it("R644 no lead, no turnStartAhead — and the human's turnStarted is only the per-turn reset", () => {
    const s = session("balanced", low);
    const even = view({ active: SEAT, turn: 5 });
    expect(s.onEvents([turnStarted(SEAT, 5)], null, even, 1_000)).toEqual([]);
    // one short on each axis stays quiet
    const justShort = view({
      active: SEAT,
      turn: 6,
      ...heroSides(30, 30 - AI_EMOTE.leadHealth + 1),
    });
    expect(s.onEvents([turnStarted(SEAT, 6)], null, justShort, 11_000)).toEqual([]);
    // the human's turn starts the same window and never rolls, even with the AI ahead
    const humanTurnAhead = view({ active: FOE, turn: 7, ...heroSides(30, 1) });
    expect(s.onEvents([turnStarted(FOE, 7)], null, humanTurnAhead, 21_000)).toEqual([]);
  });

  it("R644 a big hit on the human's hero rolls dealtBigHit; under bigHit or off a hero it rolls nothing", () => {
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

  it("R644 a big hit on the AI's hero rolls tookBigHit — and playerBigPlay off the same hit where the row exists", () => {
    const v = view({ active: FOE, turn: 5 });
    // Balanced has no playerBigPlay row: the same hit produces only the tookBigHit roll (oops).
    expect(session("balanced", low).onEvents([heroHit(SEAT)], null, v, 1_000)).toEqual([
      { emote: "oops", delayMs: AI_EMOTE.delayMinMs },
    ]);
    // Polite has both rows. With the tookBigHit roll scripted to miss (0.35 ≥ 0.3), the
    // wellPlayed that comes out can only be the playerBigPlay roll's — the one hit rolls both.
    const polite = session("polite", seq(0.35, 0, 0, 0));
    expect(polite.onEvents([heroHit(SEAT)], null, v, 1_000)).toEqual([
      { emote: "wellPlayed", delayMs: AI_EMOTE.delayMinMs },
    ]);
  });

  it("R644 a fat healthLost or fatigue hit on the AI counts as tookBigHit too — on the human it does not", () => {
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
    // and below bigHit none of them counts
    expect(
      session("balanced", low).onEvents(
        [healthLost(SEAT, AI_EMOTE.bigHit - 1), fatigue(SEAT, AI_EMOTE.bigHit - 1)],
        null,
        v,
        1_000,
      ),
    ).toEqual([]);
  });

  it("R644 the dead unit's attack must reach the previous view's top for killedTopUnit and lostTopUnit", () => {
    const prev = view({
      you: side(FOE, { units: [unit(FOE, 8), unit(FOE, 2), null, null, null] }),
      opponent: side(SEAT, { units: [unit(SEAT, 6), null, null, null, null] }),
    });
    const v = view({ active: SEAT, turn: 5 });
    // the human's 8-attack top dies to the AI's blow → killedTopUnit (laugh/wahWah)
    expect(session("balanced", low).onEvents([unitDied(FOE, 8)], prev, v, 1_000)).toEqual([
      { emote: "laugh", delayMs: AI_EMOTE.delayMinMs },
    ]);
    // a lesser unit's death is not the top
    expect(session("balanced", low).onEvents([unitDied(FOE, 7)], prev, v, 1_000)).toEqual([]);
    // the AI's own top dying rolls lostTopUnit instead (sob/angry/oops)
    expect(session("balanced", low).onEvents([unitDied(SEAT, 6)], prev, v, 1_000)).toEqual([
      { emote: "sob", delayMs: AI_EMOTE.delayMinMs },
    ]);
    expect(session("balanced", low).onEvents([unitDied(SEAT, 5)], prev, v, 1_000)).toEqual([]);
    // with no previous view the dying unit is its own top — it always qualifies
    expect(session("balanced", low).onEvents([unitDied(FOE, 1)], null, v, 1_000)).toEqual([
      { emote: "laugh", delayMs: AI_EMOTE.delayMinMs },
    ]);
    // a unit that left without a killer (killerId null) is nobody's trigger
    expect(session("balanced", low).onEvents([unitDied(FOE, 8, null)], prev, v, 1_000)).toEqual([]);
  });

  it("R644 two of the AI's units dying on the human's turn roll playerBigPlay — once, at the second", () => {
    const s = session("polite", low);
    const humanTurn = view({ active: FOE, turn: 7 });
    // The AI holds a 9-attack unit, so its 4-attack deaths are no lostTopUnit — the count alone.
    const prev = view({ opponent: side(SEAT, { units: [unit(SEAT, 9), null, null, null, null] }) });

    // the first kill only counts — polite has no lostTopUnit-relevant death here
    expect(s.onEvents([unitDied(SEAT, 4)], prev, humanTurn, 1_000)).toEqual([]);
    // the second reaches killsForBigPlay and rolls playerBigPlay (wellPlayed)
    expect(s.onEvents([unitDied(SEAT, 4)], prev, humanTurn, 11_000)).toEqual([
      { emote: "wellPlayed", delayMs: AI_EMOTE.delayMinMs },
    ]);
    // a third and a fourth that turn roll nothing: the roll was spent at two
    expect(s.onEvents([unitDied(SEAT, 4), unitDied(SEAT, 4)], prev, humanTurn, 21_000)).toEqual([]);
    // the AI's units dying on the AI's own turn never build the count
    expect(s.onEvents([unitDied(SEAT, 4), unitDied(SEAT, 4)], prev, view({ active: SEAT, turn: 8 }), 31_000)).toEqual([]);
    // the next human turn counts fresh and fires again
    const nextTurn = view({ active: FOE, turn: 9 });
    expect(s.onEvents([turnStarted(FOE, 9)], prev, nextTurn, 41_000)).toEqual([]);
    expect(s.onEvents([unitDied(SEAT, 4), unitDied(SEAT, 4)], prev, nextTurn, 51_000)).toEqual([
      { emote: "wellPlayed", delayMs: AI_EMOTE.delayMinMs },
    ]);
    // and the human's OWN units dying on the human's turn count nothing
    const otherTurn = view({ active: FOE, turn: 10 });
    expect(s.onEvents([turnStarted(FOE, 10)], prev, otherTurn, 61_000)).toEqual([]);
    expect(s.onEvents([unitDied(FOE, 4), unitDied(FOE, 4)], prev, otherTurn, 71_000)).toEqual([]);
  });

  it("R644 the second-kill roll is spent whether it hits or misses", () => {
    // Polite's playerBigPlay chance is 0.4; the scripted 0.9 misses it at the second kill.
    const s = session("polite", seq(0.9));
    const humanTurn = view({ active: FOE, turn: 7 });
    const prev = view({ opponent: side(SEAT, { units: [unit(SEAT, 9), null, null, null, null] }) });
    expect(s.onEvents([unitDied(SEAT, 4), unitDied(SEAT, 4)], prev, humanTurn, 1_000)).toEqual([]);
    // more kills that turn do not re-roll
    expect(s.onEvents([unitDied(SEAT, 4), unitDied(SEAT, 4)], prev, humanTurn, 11_000)).toEqual([]);
  });

  it("R644 gameOver sends matchWon when the seat won, matchLost when the human did, nothing on a draw", () => {
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

  it("R644 silent's session is quiet end to end — greeting, kills, lethal, replies all land nothing", () => {
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

  it("R644 a player's long turn rolls playerTurnLong — once per turn, never on the AI's own", () => {
    const s = session("balanced", low);
    const yawn = [{ emote: "yawn" as const, delayMs: AI_EMOTE.delayMinMs }];
    const humanTurn = (turn: number): PlayerView => view({ active: FOE, turn });
    expect(s.onPlayerTurnLong(humanTurn(7), 1_000)).toEqual(yawn);
    // a second arm of the same turn fires nothing (the driver may call more than once)
    expect(s.onPlayerTurnLong(humanTurn(7), 2_000)).toEqual([]);
    // the AI's own turn is never the player's long turn — and does not spend turn 8's roll
    expect(s.onPlayerTurnLong(view({ active: SEAT, turn: 8 }), 3_000)).toEqual([]);
    // the next player turn rolls again (its turnStarted opened a fresh bucket, as the driver feeds)
    s.onEvents([turnStarted(FOE, 9)], null, humanTurn(9), 4_000);
    expect(s.onPlayerTurnLong(humanTurn(9), 5_000)).toEqual(yawn);
  });
});

// ---------------------------------------------------------------------------------------------
// the session machine: replies
// ---------------------------------------------------------------------------------------------

describe("R644 the session's replies", () => {
  it("R644 a player's emote can earn a reply, each reply rule at most once a match", () => {
    const s = session("balanced", low);
    const humanTurn = (turn: number): PlayerView => view({ active: FOE, turn });
    s.onEvents([turnStarted(FOE, 3)], null, humanTurn(3), 1_000);
    expect(asIntents(s.onPlayerEmote("greetings", 11_000))).toEqual([MIN_DELAY]);
    // the greetings rule is spent: even a hitting roll is dropped (rng stays at 0)
    expect(s.onPlayerEmote("greetings", 21_000)).toEqual([]);
    // a different rule still fires — wellPlayed lands on the compliment row, answered "thanks"
    s.onEvents([turnStarted(FOE, 4)], null, humanTurn(4), 31_000);
    expect(asIntents(s.onPlayerEmote("wellPlayed", 41_000))).toEqual([
      { emote: "thanks", delayMs: AI_EMOTE.delayMinMs },
    ]);
    // and the compliment row is spent — a "thanks" emote lands on the same row and earns nothing
    s.onEvents([turnStarted(FOE, 5)], null, humanTurn(5), 51_000);
    expect(s.onPlayerEmote("thanks", 61_000)).toEqual([]);
    // the taunt row is its own rule (balanced's pool leads with laugh)
    s.onEvents([turnStarted(FOE, 6)], null, humanTurn(6), 71_000);
    expect(asIntents(s.onPlayerEmote("threaten", 81_000))).toEqual([
      { emote: "laugh", delayMs: AI_EMOTE.delayMinMs },
    ]);
    // balanced has no apology row, and a yawn has no row at all: both fall silent
    s.onEvents([turnStarted(FOE, 7)], null, humanTurn(7), 91_000);
    expect(s.onPlayerEmote("oops", 101_000)).toEqual([]);
    expect(s.onPlayerEmote("yawn", 102_000)).toEqual([]);
  });

  it("R644 a missed roll does not spend the rule — the same emote can still earn the reply later", () => {
    // balanced's greetings reply is chance 0.7: the scripted 0.7 misses, then the rule hits.
    const s = session("balanced", seq(0.7, 0, 0, 0));
    s.onEvents([turnStarted(FOE, 3)], null, view({ active: FOE, turn: 3 }), 1_000);
    expect(s.onPlayerEmote("greetings", 11_000)).toEqual([]);
    expect(asIntents(s.onPlayerEmote("greetings", 21_000))).toEqual([MIN_DELAY]);
    // now it is spent for good
    expect(s.onPlayerEmote("greetings", 31_000)).toEqual([]);
  });
});

// ---------------------------------------------------------------------------------------------
// the session machine: caps and the shared rate limit
// ---------------------------------------------------------------------------------------------

describe("R644 caps and the shared gate", () => {
  it("R644 the caps live in config: balanced 1/1/8, polite 1/1/6, bm 2/2/20, silent 0/0/0", () => {
    expect(AI_PERSONAS.balanced.caps).toEqual({ ownTurn: 1, otherTurn: 1, match: 8 });
    expect(AI_PERSONAS.polite.caps).toEqual({ ownTurn: 1, otherTurn: 1, match: 6 });
    expect(AI_PERSONAS.bm.caps).toEqual({ ownTurn: 2, otherTurn: 2, match: 20 });
    expect(AI_PERSONAS.silent.caps).toEqual({ ownTurn: 0, otherTurn: 0, match: 0 });
  });

  it("R644 balanced's turn caps are one emote on each side's turn, refilled as the turns move", () => {
    const s = session("balanced", low);
    const aiTurn = view({ active: SEAT, turn: 5 });
    // two triggers, two calls, plenty of clock between them: only the cap can drop the second
    expect(s.onEvents([turnStarted(SEAT, 5), heroHit(FOE)], null, aiTurn, 1_000)).toEqual([
      { emote: "laugh", delayMs: AI_EMOTE.delayMinMs },
    ]);
    expect(s.onEvents([heroHit(FOE)], aiTurn, aiTurn, 11_000)).toEqual([]);
    // the next AI turn opens the bucket again
    const aiTurn6 = view({ active: SEAT, turn: 6 });
    expect(s.onEvents([turnStarted(SEAT, 6), heroHit(FOE)], aiTurn, aiTurn6, 21_000)).toEqual([
      { emote: "laugh", delayMs: AI_EMOTE.delayMinMs },
    ]);
    // and the human's turn has its own bucket: a big hit taken there still emotes
    const humanTurn7 = view({ active: FOE, turn: 7 });
    expect(s.onEvents([turnStarted(FOE, 7), heroHit(SEAT)], aiTurn6, humanTurn7, 31_000)).toEqual([
      { emote: "oops", delayMs: AI_EMOTE.delayMinMs },
    ]);
    expect(s.onEvents([heroHit(SEAT)], humanTurn7, humanTurn7, 41_000)).toEqual([]);
  });

  it("R644 bm gets two a turn — the third trigger of the same turn is the cap's", () => {
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

  it("R644 the match cap counts only the non-exempt emotes — the greeting and the end emotes land past it", () => {
    const s = session("balanced", low);
    let now = 0;
    const step = (): number => (now += 10_000);
    const mulligan = view({ phase: "mulligan", mulligan: { youReady: true, opponentReady: true } });

    // the greeting is exempt from the match cap: it fires but does not spend one of the eight
    expect(s.onEvents([], mulligan, view({ active: FOE, turn: 1 }), step())).toEqual([MIN_DELAY]);
    // eight ordinary emotes fill the cap, alternating sides so a turn cap never blocks first
    const seats: PlayerId[] = [SEAT, FOE, SEAT, FOE, SEAT, FOE, SEAT, FOE];
    seats.forEach((player, i) => {
      const v = view({ active: player, turn: i + 2 });
      expect(s.onEvents([turnStarted(player, i + 2), heroHit(FOE)], null, v, step())).toEqual([
        { emote: "laugh", delayMs: AI_EMOTE.delayMinMs },
      ]);
    });
    // the ninth ordinary trigger has nowhere left to go
    const turn10 = view({ active: SEAT, turn: 10 });
    expect(s.onEvents([turnStarted(SEAT, 10), heroHit(FOE)], null, turn10, step())).toEqual([]);
    // the exempt rows still fire: a mulligan-end transition (manufactured here so the flag itself
    // is what the view tests) and both end-of-match emotes, at the cap and past it
    expect(
      s.onEvents([], mulligan, view({ active: SEAT, turn: 10 }), step()),
    ).toEqual([MIN_DELAY]);
    expect(
      s.onEvents([gameOver(SEAT)], turn10, view({ phase: "over", result: { winner: SEAT, reason: "concede" } }), step()),
    ).toEqual([{ emote: "wahWah", delayMs: AI_EMOTE.delayMinMs }]);
  });

  it("R644 the end-of-match emotes are exempt at the flag, not only via the over phase", () => {
    const s = session("balanced", low);
    let now = 0;
    const step = (): number => (now += 10_000);
    const seats: PlayerId[] = [SEAT, FOE, SEAT, FOE, SEAT, FOE, SEAT, FOE];
    seats.forEach((player, i) => {
      const v = view({ active: player, turn: i + 2 });
      expect(s.onEvents([turnStarted(player, i + 2), heroHit(FOE)], null, v, step())).toHaveLength(1);
    });
    // cap full. A gameOver event with the match still at phase "main" — artificial, but it
    // isolates matchExempt: the bucket is fresh, the cap is full, and the emote still lands.
    const turn11 = view({ active: FOE, turn: 11 });
    expect(s.onEvents([turnStarted(FOE, 11)], null, turn11, step())).toEqual([]);
    expect(s.onEvents([gameOver(FOE)], turn11, turn11, step())).toEqual([
      { emote: "wellPlayed", delayMs: AI_EMOTE.delayMinMs },
    ]);
  });

  it("R644 the shared rate limit judges the send instant (now + delayMs): a fast follow-up is dropped, a delayed one lands", () => {
    const aiTurn = view({ active: SEAT, turn: 5 });
    // Two dealtBigHit rolls in one batch at the minimum delay: the second's send time sits
    // inside the first's 1.5 s cooldown, and it is dropped — not queued. (bm's turn cap of 2
    // leaves the drop to the gate alone.)
    const fast = session("bm", low);
    expect(fast.onEvents([heroHit(FOE), heroHit(FOE)], null, aiTurn, 1_000)).toEqual([
      { emote: "laugh", delayMs: AI_EMOTE.delayMinMs },
    ]);
    // dropped means dropped: the next batch much later starts clean, nothing was held back
    expect(fast.onEvents([heroHit(FOE)], aiTurn, aiTurn, 31_000)).toEqual([
      { emote: "laugh", delayMs: AI_EMOTE.delayMinMs },
    ]);
    // and a second roll whose own delay lands past the cooldown is admitted in the same batch:
    // 0.99 draws a 2483 ms delay against the first's 800 — 1683 ms apart, outside the cooldown.
    const spaced = session("bm", seq(0, 0, 0, 0, 0, 0.99));
    const out = spaced.onEvents([heroHit(FOE), heroHit(FOE)], null, aiTurn, 1_000);
    expect(out).toHaveLength(2);
    expect(out[1]?.emote).toBe("laugh");
    expect(out[1]?.delayMs).toBeGreaterThanOrEqual(AI_EMOTE.delayMinMs + EMOTE_COOLDOWN_MS);
  });
});

// ---------------------------------------------------------------------------------------------
// the import isolation R644 requires
// ---------------------------------------------------------------------------------------------

describe("R644 the module stays out of the game", () => {
  const AI_SRC = fileURLToPath(new URL("../src", import.meta.url));
  const ENGINE_SRC = fileURLToPath(new URL("../../engine/src", import.meta.url));

  const tsFiles = (dir: string): string[] =>
    readdirSync(dir, { recursive: true, encoding: "utf8" })
      .filter((name) => name.endsWith(".ts"))
      .map((name) => join(dir, name));

  it("R644 nothing in packages/engine reads the emote module, the shared gate or the AI package", () => {
    const files = tsFiles(ENGINE_SRC);
    expect(files.length).toBeGreaterThan(0);
    for (const file of files) {
      const source = readFileSync(file, "utf8");
      expect(source, `${file} must not know the emote module`).not.toMatch(/personas|emoteGate|@jackioh\/ai/);
    }
  });

  it("R644 the move search never imports the emote module", () => {
    // The modules that play the game — every ai source except the emote table's own files.
    // config.ts names personas only in its comments (it IS the table); index.ts is the barrel
    // whose job is re-exporting the module for the page — neither is the move search.
    const search = tsFiles(AI_SRC).filter((file) => !/(personas|index|config)\.ts$/.test(file));
    const names = search.map((file) => basename(file));
    for (const file of ["decide.ts", "search.ts", "reply.ts", "lethal.ts", "determinize.ts", "evaluate.ts", "simulate.ts", "match.ts"]) {
      expect(names, `the scan must cover ${file}`).toContain(file);
    }
    for (const file of search) {
      const source = readFileSync(file, "utf8");
      expect(source, `${file} names the persona module`).not.toMatch(/\bpersonas\b/);
      expect(source, `${file} imports the persona module`).not.toMatch(/from\s+["'][^"']*persona/);
    }
    // config.ts may say the word; it may never import the module.
    const config = readFileSync(join(AI_SRC, "config.ts"), "utf8");
    expect(config).not.toMatch(/from\s+["'][^"']*persona/);
  });
});
