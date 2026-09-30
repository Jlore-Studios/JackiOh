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

/** Classic+ #33 Ivory Tower's shape: a carrier whose aura gives its controller's cards Stack. */
export const tower = def("tower", "Field Spell", { cost: 2 });

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

export const FIELD_DEFS: CardDef[] = [tesla, springer, asker, spatula, golem, tower, cover, banner, watcher, listener, ears];

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
