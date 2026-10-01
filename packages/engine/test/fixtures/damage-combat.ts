// Fixture cards for the damage and combat systems of patch v0.2.0 (docs/classic-sets.md B5 E5–E9,
// E35, E37, and §4.5's backrow Death). The engine never imports `packages/cards` (CLAUDE.md), so each
// system is proved through a card shaped like the one that will use it: a Final Gambit, a Voidwalker,
// a Blood Moon, a Joro. Ids are `dc-…`, indexed from 4700, so they collide with no other test file.

import type { Action, ActionInput, CardDef, CardType, GameEvent, Keyword, PlayerId } from "@jackioh/shared";
import { registerCatalog, registeredCatalog } from "../../src/catalog";
import { afterAttackOf } from "../../src/combat";
import {
  cancelAttack,
  damage,
  damageAll,
  damageSplit,
  draw,
  forcedAttackOwnHero,
  heal,
  setHealth,
  steal,
} from "../../src/effects";
import { openPrompt, resumeSelf } from "../../src/prompts";
import { killerOf } from "../../src/query";
import { beginGame, reduce } from "../../src/reduce";
import { replacementOf } from "../../src/replacements";
import { isBerserk } from "../../src/restrictions";
import { hashState } from "../../src/replay";
import type { CardScripts, Effect, Script } from "../../src/script";
import { registerScripts, registeredScripts } from "../../src/scripts";
import { cloneState, type CardInstance, type GameState } from "../../src/state";
import { newGame, put, slot } from "./harness";

let nextIndex = 4700;

function def(name: string, type: CardType, extra: Partial<CardDef> = {}): CardDef {
  nextIndex += 1;
  const face = { keywords: [] as Keyword[], text: name };
  return {
    id: `dc-${name}`,
    index: String(nextIndex),
    name: `${name} (damage and combat)`,
    set: "Core",
    type,
    tags: [],
    rarity: "Common",
    token: false,
    cost: 0,
    base: { ...face },
    radiant: { ...face },
    ...extra,
  };
}

function unit(name: string, attack: number, health: number, keywords: Keyword[] = [], radiantKeywords = keywords): CardDef {
  return def(name, "Unit", {
    base: { attack, health, keywords, text: name },
    radiant: { attack: attack * 2, health: health * 2, keywords: radiantKeywords, text: name },
  });
}

// ---------------------------------------------------------------------------
// The note log: a Field Spell in p1's backrow lane 5 whose memory records what ran, in order.
// ---------------------------------------------------------------------------

export const LOG_LANE = 5;
export const logCard = def("log", "Field Spell");

function logOf(state: GameState): CardInstance | null {
  return state.players.p1.backrow[LOG_LANE - 1] ?? null;
}

export function notes(state: GameState): string[] {
  const log = logOf(state);
  return Array.isArray(log?.memory.steps) ? (log.memory.steps as string[]) : [];
}

function write(state: GameState, entry: string): void {
  const log = logOf(state);
  if (log === null) return;
  log.memory.steps = [...notes(state), entry];
}

export function note(entry: string | ((ctx: Parameters<Effect["apply"]>[0]) => string)): Effect {
  return {
    kind: "dc:note",
    apply(ctx): void {
      write(ctx.state, typeof entry === "string" ? entry : entry(ctx));
    },
  };
}

/** A prompt for the running card's controller with one answer, so answering is trivial (§10.6). */
export function askController(step: string): Effect {
  return {
    kind: "dc:ask",
    apply(ctx): void {
      openPrompt(ctx, {
        player: ctx.controller,
        kind: "target",
        prompt: "the card asks its controller",
        options: [{ key: "none", label: "nothing", selection: { pick: "none" } }],
        resume: resumeSelf(ctx, step),
      });
    },
  };
}

/** R426: the units the attack destroyed, whether its attacker survived it, and the player the hook acts for. */
function attackFacts(ctx: Parameters<Effect["apply"]>[0]): string {
  const facts = afterAttackOf(ctx);
  return `after:${(facts?.destroyedIds ?? []).join("+")}:${String(facts?.survived)}:${ctx.controller}`;
}

function both(script: Script, radiant: Script = script): CardScripts {
  return { base: script, radiant };
}

// ---------------------------------------------------------------------------
// E5 replacements
// ---------------------------------------------------------------------------

/** Classic #52's shape: redirect a lethal hit on its hero to the enemy hero, then heal 10 and draw 3. */
export const gambit = def("gambit", "Trap");
/** A Field Trap that redirects every lethal hit on its hero and stays: two of them could ping-pong for ever. */
export const echoGambit = def("echo-gambit", "Field Trap");
/** The same, whose follow-up asks its controller before it heals: a pause inside the follow-up. */
export const gambitAsker = def("gambit-asker", "Trap");
/** Classic #50's shape: a Unit whose aura exiles every card that would go to a graveyard (Radiant: the enemy's). */
export const voidwalker = unit("voidwalker", 6, 3);
/** Classic #28's shape: a Field Spell exiling its controller's own cards on their way to a graveyard. */
export const secondWind = def("second-wind", "Field Spell");
/** Classic #60's shape: a Spell that goes to the bottom of its owner's library instead of its graveyard. */
export const pileOn = def("pile-on", "Spell");
/** Classic #14's Radiant shape: its controller's dying units flicker instead; the follow-up notes them. */
export const shadowstep = def("shadowstep", "Trap");
/** Classic+ #22's shape: an enemy's heal becomes Pierce damage, for the rest of the turn. */
export const bloodMoon = def("blood-moon", "Trap", {
  // B2.7: the Radiant face is a Field Trap that converts from then on, while it stays on the field.
  radiant: { type: "Field Trap", keywords: [], text: "blood moon radiant" },
});
/** Classic #33's shape: from the hand, interposes when the opponent targets a friendly unit. */
export const joro = unit("joro", 1, 1);

// ---------------------------------------------------------------------------
// E6, E7, E8 pipeline pieces
// ---------------------------------------------------------------------------

/** Classic #75's shape: the hero's hits are halved (Radiant: quartered), rounded up, after Armor. */
export const argus = def("argus", "Field Spell");
/** A Trap guarding its hero like Argusland: it guards nothing while it is face-down (R463). */
export const hiddenArgus = def("hidden-argus", "Trap");
/** Classic+ #11's shape: the hero takes at most 1 per hit. */
export const animeArmor = unit("anime-armor", 4, 4);
/** Classic+ #38's shape: Spell Damage +2 (Radiant +5). */
export const solar = unit("solar", 3, 2, [{ kind: "Spell Damage", n: 2 }], [{ kind: "Spell Damage", n: 5 }]);
/** A Field Spell printing Spell Damage +1, for summing and for the face-down rule. */
export const lens = def("lens", "Field Spell", {
  base: { keywords: [{ kind: "Spell Damage", n: 1 }], text: "lens" },
  radiant: { keywords: [{ kind: "Spell Damage", n: 1 }], text: "lens" },
});
/** A Trap printing Spell Damage +3, which counts for nothing while it is face-down. */
export const hiddenLens = def("hidden-lens", "Trap", {
  base: { keywords: [{ kind: "Spell Damage", n: 3 }], text: "hidden lens" },
  radiant: { keywords: [{ kind: "Spell Damage", n: 3 }], text: "hidden lens" },
});
/** A Spell: deal 3 damage to a declared target. */
export const bolt = def("bolt", "Spell");
/** Classic #83's shape: a Spell with printed Trample, "deal 11 damage to a Unit". */
export const lance = def("lance", "Spell", {
  base: { keywords: [{ kind: "Trample" }], text: "lance" },
  radiant: { keywords: [{ kind: "Trample" }], text: "lance" },
});
/** Classic #29's shape: set a declared hero's health to 13. */
export const vitalKill = def("vital-kill", "Spell");
/** A Spell: heal a declared target 5. */
export const mend = def("mend", "Spell");
/** A Field Spell: its controller's start of turn, deal 4 to every unit (a board sweep that is no Spell). */
export const sweep = def("sweep", "Field Spell");
/** A Spell: deal 2 damage to every unit (a Spell's sweep). */
export const storm = def("storm", "Spell");

// ---------------------------------------------------------------------------
// E35 restrictions and statuses
// ---------------------------------------------------------------------------

/** Classic+ #51's shape: can't be attacked. */
export const fighter = unit("fighter", 7, 2);
/** Classic+ #19.1's shape: only units in its lane may attack it; Radiant adds Immune to Spells. */
export const topLoser = unit("top-loser", 5, 5, [{ kind: "Taunt" }], [{ kind: "Taunt" }, { kind: "Immune to Spells" }]);
/** Neither attacks nor is attacked. */
export const statue = unit("statue", 3, 3);
/** Classic #69's shape: First Strike only while it carries a Plague Token. */
export const charger = unit("charger", 4, 2);
/** Classic+ #19.5's shape: while Berserk, attacks its own hero at its controller's start of turn; Radiant can't go Berserk. */
export const botLoser = unit("bot-loser", 5, 5);
/** An immune unit: Immune to Spells printed. */
export const warded = unit("warded", 2, 4, [{ kind: "Immune to Spells" }]);

// ---------------------------------------------------------------------------
// E37 and backrow Death
// ---------------------------------------------------------------------------

/** Classic+ #3's shape: Death — deal 1 damage to a random enemy for each Plague Token on it. */
export const snake = unit("snake", 1, 6);
/** Classic+ #61's shape: a Field Spell with a Death hook. */
export const bauble = def("bauble", "Field Spell");
/** A unit with a Death hook that notes itself, for R68's order beside a backrow Death. */
export const rattle = unit("rattle", 1, 1);
/** A Unit with Reborn, to show R461: an exiled unit comes back from nothing. */
export const phoenix = unit("phoenix", 2, 2, [{ kind: "Reborn" }]);
/** A Unit with Lifesteal, for E8's conversion of Lifesteal. */
export const leech = unit("leech", 3, 5, [{ kind: "Lifesteal" }]);
/** A plain 2/2 and a plain 1/8, bodies to attack with and at. */
export const grunt = unit("grunt", 2, 2);
export const wall = unit("wall", 1, 8);
/** A Field Trap that notes every declared attack and every summon it is offered (it fires again and again). */
export const watcher = def("watcher", "Field Trap");
/** "After this attacks": notes the combat's facts (Classic #13's shape). */
export const veteran = unit("veteran", 2, 2);
/** R426: "After this attacks" with Cleave: notes the units destroyed, whether it survived, and for whom it acts. */
export const cleaveVeteran = unit("cleave-veteran", 3, 6, [{ kind: "Cleave" }]);
/** R426: the same with Reborn, 2/2, so a combat can kill it and bring a new body back. */
export const rebornVeteran = unit("reborn-veteran", 2, 2, [{ kind: "Reborn" }]);
/** #86 Mrow's shape: "Death: Take control of the Unit that destroyed this." */
export const turncoat = unit("turncoat", 1, 1);
/** The same, whose hook asks its controller before it finishes. */
export const veteranAsker = unit("veteran-asker", 2, 2);
/** A 1/1 whose Death asks its controller something (a pause inside §4.5 step 3). */
export const deathAsker = unit("death-asker", 1, 1);
/** #96 My Pawn's shape: cancels every declared attack (R44). */
export const pawn = def("pawn", "Trap");

export const DC_DEFS: CardDef[] = [
  logCard,
  gambit,
  echoGambit,
  gambitAsker,
  voidwalker,
  secondWind,
  pileOn,
  shadowstep,
  bloodMoon,
  joro,
  argus,
  hiddenArgus,
  animeArmor,
  solar,
  lens,
  hiddenLens,
  bolt,
  lance,
  vitalKill,
  mend,
  sweep,
  storm,
  fighter,
  topLoser,
  statue,
  charger,
  botLoser,
  warded,
  snake,
  bauble,
  rattle,
  phoenix,
  leech,
  cleaveVeteran,
  rebornVeteran,
  turncoat,
  grunt,
  wall,
  watcher,
  veteran,
  veteranAsker,
  deathAsker,
  pawn,
];

const anyTarget = [{ kind: "target" as const, min: 1, max: 1, filter: { of: ["unit" as const, "hero" as const] } }];
const unitTarget = [{ kind: "target" as const, min: 1, max: 1, filter: { of: ["unit" as const] } }];
const heroTarget = [{ kind: "target" as const, min: 1, max: 1, filter: { of: ["hero" as const] } }];

function gambitScript(heal_: number): Script {
  return {
    replacements: [{ id: "gambit", on: "lethalHit", instead: { redirect: "enemyHero" }, then: "after" }],
    resume: {
      after: (ctx) => [
        note(`gambit:after:${replacementOf(ctx)?.redirectedTo ?? "?"}`),
        heal({ target: { of: "selfHero" }, amount: heal_ }),
        draw({ count: 3 }),
      ],
    },
  };
}

export const DC_SCRIPTS: Record<string, CardScripts> = {
  [gambit.id]: both(gambitScript(10), gambitScript(20)),
  [echoGambit.id]: both({ replacements: [{ id: "echo", on: "lethalHit", instead: { redirect: "enemyHero" } }] }),
  [gambitAsker.id]: both({
    replacements: [{ id: "gambit", on: "lethalHit", instead: { redirect: "enemyHero" }, then: "after" }],
    resume: {
      after: () => [note("asker:before"), askController("answered"), note("asker:tail")],
      answered: () => [note("asker:answered"), heal({ target: { of: "selfHero" }, amount: 10 })],
    },
  }),
  [voidwalker.id]: both(
    { replacements: [{ id: "void", on: "toGraveyard", instead: { to: "exile" } }] },
    {
      replacements: [
        { id: "void", on: "toGraveyard", when: (ctx) => ctx.event.owner !== ctx.controller, instead: { to: "exile" } },
      ],
    },
  ),
  [secondWind.id]: both({
    replacements: [
      { id: "wind", on: "toGraveyard", when: (ctx) => ctx.event.owner === ctx.controller, instead: { to: "exile" } },
    ],
  }),
  [pileOn.id]: both(
    {
      cry: () => [note("pile-on:cry")],
      replacements: [{ id: "pile", on: "toGraveyard", where: "self", instead: { to: "bottomOfLibrary" } }],
    },
    { cry: () => [note("pile-on:cry")] },
  ),
  [shadowstep.id]: both({
    replacements: [{ id: "step", on: "wouldDie", instead: { flicker: "yours" }, then: "copies" }],
    resume: {
      copies: (ctx) => [
        note(`shadowstep:${(replacementOf(ctx)?.flickered ?? []).map((card) => card.instanceId).join(",")}`),
      ],
    },
  }),
  [bloodMoon.id]: both(
    { replacements: [{ id: "moon", on: "healed", instead: { damage: "pierce", lasting: "thisTurn" } }] },
    {
      staticFlags: { healToDamage: true },
      replacements: [{ id: "moon", on: "healed", instead: { damage: "pierce" } }],
    },
  ),
  [joro.id]: both({ replacements: [{ id: "joro", on: "targeted", where: "hand", instead: { interpose: true } }] }),
  [argus.id]: both({ heroGuard: () => [{ divisor: 2 }] }, { heroGuard: () => [{ divisor: 4 }] }),
  [hiddenArgus.id]: both({ heroGuard: () => [{ divisor: 2 }] }),
  [animeArmor.id]: both({ heroGuard: () => [{ cap: 1 }] }),
  [bolt.id]: both({ targets: anyTarget, cry: () => [damage({ to: { of: "chosen" }, amount: 3 })] }),
  [lance.id]: both({ targets: unitTarget, cry: () => [damage({ to: { of: "chosen" }, amount: 11, trample: true })] }),
  [vitalKill.id]: both({ targets: heroTarget, cry: () => [setHealth({ to: { of: "chosen" }, value: 13 })] }),
  [mend.id]: both({ targets: anyTarget, cry: () => [heal({ target: { of: "chosen" }, amount: 5 })] }),
  [sweep.id]: both({ startOfTurn: () => [damageAll({ amount: 4, side: "any" })] }),
  [storm.id]: both({ cry: () => [damageAll({ amount: 2, side: "any" })] }),
  [fighter.id]: both({ staticFlags: { cantBeAttacked: true } }),
  [topLoser.id]: both({ staticFlags: { attackedOnlyFromLane: true } }),
  [statue.id]: both({ staticFlags: { cantAttackOrBeAttacked: true } }),
  [charger.id]: both({
    conditionalKeywords: ({ self }) => ((self.counters.plague ?? 0) > 0 ? [{ kind: "First Strike" }] : []),
  }),
  [botLoser.id]: both(
    { startOfTurn: (ctx) => (ctx.self !== null && isBerserk(ctx.self) ? [forcedAttackOwnHero({ attacker: { of: "self" } })] : []) },
    { staticFlags: { neverBerserk: true } },
  ),
  [snake.id]: both({
    death: (ctx) => [damageSplit({ amount: ctx.self?.counters.plague ?? 0, among: "enemies" })],
  }),
  [bauble.id]: both({ death: () => [note("bauble:death")] }),
  [rattle.id]: both({ death: (ctx) => [note(`rattle:death:${ctx.self?.controller ?? "?"}`)] }),
  [veteran.id]: both({
    afterAttack: (ctx) => {
      const facts = afterAttackOf(ctx);
      return [
        note(
          `after:${facts?.targetId ?? "?"}:${(facts?.destroyedIds ?? []).join("+")}:${String(facts?.survived)}:${String(facts?.forced)}:${ctx.self?.zone.z ?? "none"}`,
        ),
      ];
    },
  }),
  [cleaveVeteran.id]: both({ afterAttack: (ctx) => [note(attackFacts(ctx))] }),
  [rebornVeteran.id]: both({ afterAttack: (ctx) => [note(attackFacts(ctx))] }),
  [turncoat.id]: both({
    death: (ctx) => {
      const killer = killerOf(ctx.state, ctx.self);
      return killer === null ? [] : [steal({ instanceId: killer.id })];
    },
  }),
  [veteranAsker.id]: both({
    afterAttack: () => [note("after:before"), askController("answered"), note("after:tail")],
    resume: { answered: (ctx) => [note(`after:answered:${String(afterAttackOf(ctx)?.survived)}`)] },
  }),
  [deathAsker.id]: both({
    death: () => [note("death:ask"), askController("answered")],
    resume: { answered: () => [note("death:answered")] },
  }),
  [pawn.id]: both({
    triggers: [
      {
        id: "pawn",
        on: ["attackDeclared"],
        when: (ctx) => ctx.event.type === "attackDeclared" && !ctx.event.forced,
        run: () => [cancelAttack()],
      },
    ],
  }),
  [watcher.id]: both({
    triggers: [
      {
        id: "watch",
        on: ["attackDeclared", "summoned"],
        run: (ctx) => [
          note(
            ctx.event.type === "attackDeclared"
              ? `watch:attack:${ctx.event.targetId}`
              : ctx.event.type === "summoned"
                ? `watch:summon:${ctx.event.instanceId}`
                : "watch",
          ),
        ],
      },
    ],
  }),
};

// ---------------------------------------------------------------------------
// Harness
// ---------------------------------------------------------------------------

export function register(): void {
  registerCatalog({ ...registeredCatalog(), ...Object.fromEntries(DC_DEFS.map((d) => [d.id, d])) });
  registerScripts({ ...registeredScripts(), ...DC_SCRIPTS });
}

let nonce = 0;

export function actResult(state: GameState, body: ActionInput): ReturnType<typeof reduce> {
  nonce += 1;
  return reduce(state, { ...body, nonce: `dc${nonce}` } as Action);
}

export function act(state: GameState, body: ActionInput): GameState {
  const result = actResult(state, body);
  if (result.error !== undefined) throw new Error(result.error);
  return result.state;
}

/** A game past both mulligans, in p1's first main phase, with the note log in p1's backrow lane 5. */
export function playing(seed: string): GameState {
  const created = newGame(seed);
  register();
  let state = beginGame(created).state;
  state = act(state, { type: "mulligan", keep: state.players.p1.hand.map((c) => c.id), playerId: "p1" });
  state = act(state, { type: "mulligan", keep: state.players.p2.hand.map((c) => c.id), playerId: "p2" });
  put(state, logCard.id, slot("p1", "backrow", LOG_LANE));
  return state;
}

/** Answer the one open prompt, whoever it belongs to. */
export function answer(state: GameState): ReturnType<typeof reduce> {
  const pending = state.pending;
  if (pending === null) throw new Error("expected a prompt to be open");
  const result = actResult(state, { type: "answer", choiceId: pending.id, selection: [{ pick: "none" }], playerId: pending.playerId });
  if (result.error !== undefined) throw new Error(result.error);
  return result;
}

export const roundTrip = (state: GameState): GameState => JSON.parse(JSON.stringify(state)) as GameState;

/**
 * §9.3: the live game and its replay agree. `start` is the state the log was played from; the log is
 * folded again from a clone of it, action by action through `reduce`, and the two end states hash
 * the same (`replay.hashState`).
 */
export function replaysTo(start: GameState, log: readonly Action[], live: GameState): boolean {
  let state = cloneState(start);
  for (const action of log) {
    const result = reduce(state, action);
    if (result.error !== undefined) throw new Error(`replay rejected ${action.type}: ${result.error}`);
    state = result.state;
  }
  return hashState(state) === hashState(live);
}

/** A recorder: every action it plays goes into its log, so a test can replay what it played. */
export function recorder(start: GameState): {
  state: () => GameState;
  play: (body: ActionInput) => ReturnType<typeof reduce>;
  log: Action[];
  start: GameState;
} {
  let state = start;
  const log: Action[] = [];
  const begin = cloneState(start);
  return {
    state: () => state,
    log,
    start: begin,
    play(body) {
      nonce += 1;
      const action = { ...body, nonce: `dcr${nonce}` } as Action;
      const result = reduce(state, action);
      if (result.error !== undefined) throw new Error(result.error);
      log.push(action);
      state = result.state;
      return result;
    },
  };
}

export function eventTypes(events: readonly GameEvent[]): string[] {
  return events.map((event) => event.type);
}

export function inPile(state: GameState, player: PlayerId, pile: "hand" | "library" | "graveyard" | "exile", id: string): boolean {
  return state.players[player][pile].some((card) => card.id === id);
}
