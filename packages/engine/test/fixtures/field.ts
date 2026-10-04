// Fixtures for the field workstream's systems (docs/classic-sets.md B3.1 Animated, B5 E20 Lock variants
// and Unlock, E21 backrow piles and carriers, E22 Flicker). Test-only scripts, since the engine never
// imports `packages/cards` (CLAUDE.md): each is the shape of a Classic or Classic+ card that uses the
// system, cut down to the part the engine test exercises. Defs are prefixed `fd-` and indexed from
// 9400, so they collide with no other file's catalog (BUILD §0).

import type { Action, ActionInput, CardDef, GameEvent, Keyword, PlayerId } from "@jackioh/shared";
import { registerCatalog, registeredCatalog } from "../../src/catalog";
import { animate } from "../../src/effects/animate";
import { chooseTarget } from "../../src/effects/choose";
import { damage } from "../../src/effects/damage";
import { destroyAll } from "../../src/effects/destroy";
import { flicker } from "../../src/effects/flicker";
import { lockLane, lockOwnZone, lockPlayedZone, lockRandomZone, unlockAll } from "../../src/effects/locks";
import { beginGame, reduce } from "../../src/reduce";
import type { CardScripts, Effect, Script } from "../../src/script";
import { registerScripts, registeredScripts } from "../../src/scripts";
import type { CardInstance, GameState } from "../../src/state";
import { newGame } from "./harness";

let nextIndex = 9400;

function def(name: string, type: CardDef["type"], extra: Partial<CardDef> = {}): CardDef {
  nextIndex += 1;
  return {
    id: `fd-${name}`,
    index: String(nextIndex),
    name: `${name} (field fixture)`,
    set: "Core",
    type,
    tags: [],
    rarity: "Common",
    token: false,
    cost: 1,
    base: { keywords: [], text: name },
    radiant: { keywords: [], text: name },
    ...extra,
  };
}

function face(attack: number, health: number, keywords: Keyword[], text = ""): CardDef["base"] {
  return { attack, health, keywords, text };
}

const ANIMATED: Keyword = { kind: "Animated" };
const ON_YOUR_TURN: Keyword = { kind: "Animated on your turn" };

/** Classic #5 Tesla's shape: a 1/4 Animated Field Trap that zaps each enemy Unit arriving, then animates in Defense. */
export const tesla = def("tesla", "Field Trap", {
  cost: 2,
  base: face(1, 4, [ANIMATED, { kind: "Lifesteal" }]),
  radiant: face(2, 8, [ANIMATED, { kind: "Lifesteal" }]),
});

/** A plain Animated Trap (not a Field Trap): fires once on an enemy play, then animates. */
export const springer = def("springer", "Trap", { base: face(3, 3, [ANIMATED]), radiant: face(6, 6, [ANIMATED]) });

/** An Animated Field Trap whose list asks before it animates: `[chooseTarget, animate]` (R113). */
export const asker = def("asker", "Field Trap", { base: face(2, 2, [ANIMATED]), radiant: face(4, 4, [ANIMATED]) });

/** Classic+ #12.8 Frostspatula's shape: an "Animated on your turn" Field Spell with Rush. */
export const spatula = def("spatula", "Field Spell", {
  cost: 2,
  base: face(10, 3, [ON_YOUR_TURN, { kind: "Rush" }]),
  radiant: face(20, 6, [ON_YOUR_TURN, { kind: "Rush" }]),
});

/** A plain Animated Field Spell, which animates as it enters the field (B3.1 rule 4). */
export const golem = def("golem", "Field Spell", { base: face(3, 3, [ANIMATED]), radiant: face(6, 6, [ANIMATED]) });

/** A stat-less Animated Field Spell (R654): incidentally animated, it fights as a 0/1. */
export const wisp = def("wisp", "Field Spell", {
  base: { keywords: [ANIMATED], text: "wisp" },
  radiant: { keywords: [ANIMATED], text: "wisp" },
});

/** A carrier whose aura gives its controller's cards Stack: Classic+ #33 Ivory Tower's shape before patch v0.2.10. */
export const tower = def("tower", "Field Spell", { cost: 2 });

/** Classic+ #33 Ivory Tower's shape since patch v0.2.10: a carrier that takes one Unit a stay (R653). */
export const fuser = def("fuser", "Field Spell", { cost: 2 });

/** A Field Spell that prints Stack, so it tops an occupied backrow zone without any aura (B5 E21). */
export const cover = def("cover", "Field Spell", {
  base: { keywords: [{ kind: "Stack" }], text: "Stack" },
  radiant: { keywords: [{ kind: "Stack" }], text: "Stack" },
});

/** A Field Spell whose aura gives its controller's units +2 attack: off while it lies under a pile. */
export const banner = def("banner", "Field Spell");

/** A face-down Trap that notes every enemy play it answers: silent while it lies under a pile. */
export const watcher = def("watcher", "Trap");

/** A Field Trap (not Animated) that notes every enemy play, fired again and again. */
export const listener = def("listener", "Field Trap");

/** An Animated Field Trap that notes every enemy play and animates: a turret on `cardPlayed`. */
export const ears = def("ears", "Field Trap", { base: face(1, 5, [ANIMATED]), radiant: face(2, 10, [ANIMATED]) });

/** A Spell that destroys its caster's backrow, then asks (a pause after a carrier has gone, R446). */
export const wrecker = def("wrecker", "Spell");

/** Classic+ #34's first mode as a Spell: Lock a random zone on the opponent's side. */
export const leak = def("leak", "Spell");

/** Classic #84 Lockdown's shape: after a permanent is played, Lock its zone (either player's play). */
export const lockdown = def("lockdown", "Field Spell", { cost: 2 });

/** Classic+ #1 Doom Shroom's shape: fires on an enemy play and Locks its own zone. */
export const doom = def("doom", "Trap");

/** Classic #71 Lane Eater's shape: Cry: Lock this lane (Radiant: the enemy side of it). */
export const eater = def("eater", "Unit", { cost: 3, base: face(4, 4, []), radiant: face(8, 8, []) });

/** A Field Spell that notes every death it answers: silent for the death that uncovers it (R212). */
export const mourner = def("mourner", "Field Spell");

/** Classic+ #77's Unlock half: Unlock every zone. */
export const unlocker = def("unlocker", "Spell");

/** Classic #14's Radiant move as a Spell: flicker a chosen Unit, then ask (a pause after the flicker). */
export const blink = def("blink", "Spell");

export const FIELD_DEFS: CardDef[] = [
  tesla,
  springer,
  asker,
  spatula,
  golem,
  wisp,
  tower,
  fuser,
  cover,
  banner,
  watcher,
  listener,
  ears,
  wrecker,
  leak,
  lockdown,
  doom,
  eater,
  unlocker,
  blink,
  mourner,
];

function both(script: Script): CardScripts {
  return { base: script, radiant: script };
}

/** The memory key the note effects write, on the card whose script runs. */
export const NOTES = "fdNotes";

export function note(name: string): Effect {
  return {
    kind: "fd:note",
    apply(ctx): void {
      const self = ctx.self;
      if (self === null) return;
      const notes = Array.isArray(self.memory[NOTES]) ? (self.memory[NOTES] as string[]) : [];
      self.memory[NOTES] = [...notes, name];
    },
  };
}

export function notesOf(card: CardInstance | undefined | null): string[] {
  const notes = card?.memory[NOTES];
  return Array.isArray(notes) ? (notes as string[]) : [];
}

function enemyPlay(ctx: { controller: PlayerId; event: GameEvent }): boolean {
  return ctx.event.type === "cardPlayed" && ctx.event.player !== ctx.controller;
}

export const FIELD_SCRIPTS: Record<string, CardScripts> = {
  [tesla.id]: {
    base: {
      triggers: [
        {
          id: "fd-tesla-zap",
          on: ["summoned"],
          when: ({ event, controller }) => event.type === "summoned" && event.player !== controller && event.row === "units",
          run: ({ event }) =>
            event.type === "summoned"
              ? [damage({ to: { of: "instance", instanceId: event.instanceId }, amount: 4 }), animate({ position: "DEF" })]
              : [],
        },
      ],
    },
    radiant: {
      triggers: [
        {
          id: "fd-tesla-zap",
          on: ["summoned"],
          when: ({ event, controller }) => event.type === "summoned" && event.player !== controller && event.row === "units",
          run: ({ event }) =>
            event.type === "summoned"
              ? [damage({ to: { of: "instance", instanceId: event.instanceId }, amount: 8 }), animate({ position: "DEF" })]
              : [],
        },
      ],
    },
  },
  [springer.id]: both({
    triggers: [{ id: "fd-spring", on: ["cardPlayed"], when: enemyPlay, run: () => [note("sprang"), animate()] }],
  }),
  [asker.id]: both({
    triggers: [
      {
        id: "fd-ask",
        on: ["cardPlayed"],
        when: enemyPlay,
        run: () => [chooseTarget({ step: "hit", scope: { side: "enemy", of: ["unit", "hero"] } }), note("tail"), animate()],
      },
    ],
    resume: { hit: () => [damage({ to: { of: "chosen" }, amount: 1 }), note("answered")] },
  }),
  [spatula.id]: both({ endOfTurn: (ctx) => [note(ctx.self?.zone.z === "field" ? ctx.self.zone.row : "gone")] }),
  [tower.id]: both({
    staticFlags: { carrier: true },
    aura: ({ self }) => [{ applies: (card) => card.controller === self.controller, mod: { keywords: [{ kind: "Stack" }] } }],
  }),
  [fuser.id]: both({ staticFlags: { fusesCarried: true } }),
  [wisp.id]: both({}),
  [banner.id]: both({
    aura: ({ self }) => [
      { applies: (card) => card.controller === self.controller && card.zone.z === "field", mod: { attack: 2 } },
    ],
  }),
  [watcher.id]: both({
    triggers: [{ id: "fd-watch", on: ["cardPlayed"], when: enemyPlay, run: () => [note("watched")] }],
  }),
  [listener.id]: both({
    triggers: [{ id: "fd-listen", on: ["cardPlayed"], when: enemyPlay, run: () => [note("heard")] }],
  }),
  [ears.id]: both({
    triggers: [{ id: "fd-ears", on: ["cardPlayed"], when: enemyPlay, run: () => [note("heard"), animate()] }],
  }),
  [wrecker.id]: both({
    cry: () => [
      destroyAll({ side: "self", rows: ["backrow"] }),
      chooseTarget({ step: "after", scope: { side: "enemy", of: ["hero"] } }),
    ],
    resume: { after: () => [] },
  }),
  [leak.id]: both({ cry: () => [lockRandomZone({ side: "enemy" })] }),
  [lockdown.id]: both({
    triggers: [{ id: "fd-lockdown", on: ["cardPlayed"], run: () => [lockPlayedZone()] }],
  }),
  [doom.id]: both({
    triggers: [{ id: "fd-doom", on: ["cardPlayed"], when: enemyPlay, run: () => [note("doomed"), lockOwnZone()] }],
  }),
  [eater.id]: { base: { cry: () => [lockLane()] }, radiant: { cry: () => [lockLane({ side: "enemy" })] } },
  [unlocker.id]: both({ cry: () => [unlockAll()] }),
  [mourner.id]: both({ triggers: [{ id: "fd-mourn", on: ["destroyed"], run: () => [note("mourned")] }] }),
  [blink.id]: both({
    targets: [{ kind: "target", min: 1, max: 1 }],
    cry: () => [
      flicker({ target: { of: "chosen" } }),
      chooseTarget({ step: "then", scope: { side: "enemy", of: ["hero"] } }),
    ],
    resume: { then: () => [] },
  }),
};

/** A game with the field fixtures registered beside the engine's own. */
export function fieldGame(seed: string): GameState {
  const state = newGame(seed);
  registerCatalog({ ...registeredCatalog(), ...Object.fromEntries(FIELD_DEFS.map((d) => [d.id, d])) });
  registerScripts({ ...registeredScripts(), ...FIELD_SCRIPTS });
  return state;
}

let nonce = 0;

export function actResult(state: GameState, body: ActionInput): ReturnType<typeof reduce> {
  nonce += 1;
  return reduce(state, { ...body, nonce: `fd${nonce}` } as Action);
}

export function act(state: GameState, body: ActionInput): GameState {
  const result = actResult(state, body);
  if (result.error !== undefined) throw new Error(result.error);
  return result.state;
}

/** Past both mulligans, in p1's main phase of turn 1, with every card of the opening hands kept. */
export function playing(seed: string): GameState {
  let state = beginGame(fieldGame(seed)).state;
  state = act(state, { type: "mulligan", keep: state.players.p1.hand.map((c) => c.id), playerId: "p1" });
  state = act(state, { type: "mulligan", keep: state.players.p2.hand.map((c) => c.id), playerId: "p2" });
  return state;
}

/** Gives a player plenty of mana for a test's plays. */
export function flush(state: GameState, player: PlayerId, mana = 10): void {
  state.players[player].mana.current = mana;
  state.players[player].mana.max = mana;
}
