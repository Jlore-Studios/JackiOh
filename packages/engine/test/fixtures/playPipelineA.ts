// Fixture cards for play pipeline A (docs/classic-sets.md B5 E1, E2, E4, E5, E9; Classic #4, #10, #17,
// #23, #33, #72, #87, #89; Classic+ #37, #64, #68; AI Refusal, Autocomplete, Scaling Law). The engine
// never imports `packages/cards` (CLAUDE.md), so each system is proved on a card of its shape here.
// Ids are `pa-*`, indices from 5000, registered on top of the shared fixture catalog by `withPlayA`.

import type { CardDef, CardType, Keyword } from "@jackioh/shared";
import { registerCatalog, registeredCatalog } from "../../src/catalog";
import {
  addPlayerModifier,
  chooseMode,
  chooseTarget,
  chosenOptions,
  counterPlay,
  damage,
  discardHand,
  sacrifice,
} from "../../src/effects";
import { lastSpellPlayed } from "../../src/query";
import type { CardScripts, Script } from "../../src/script";
import { registerScripts, registeredScripts } from "../../src/scripts";
import type { GameState } from "../../src/state";

let nextIndex = 5000;

function def(
  id: string,
  type: CardType,
  extra: Partial<CardDef> & { attack?: number; health?: number; keywords?: Keyword[] } = {},
): CardDef {
  nextIndex += 1;
  const { attack, health, keywords = [], ...rest } = extra;
  const stats = type === "Unit" ? { attack: attack ?? 2, health: health ?? 2 } : {};
  const radiantStats = type === "Unit" ? { attack: (attack ?? 2) * 2, health: (health ?? 2) * 2 } : {};
  return {
    id: `pa-${id}`,
    index: String(nextIndex),
    name: `${id} (play A)`,
    set: "Core",
    type,
    tags: [],
    rarity: "Common",
    token: false,
    cost: 0,
    base: { ...stats, keywords, text: id },
    radiant: { ...radiantStats, keywords, text: `${id} radiant` },
    ...rest,
  };
}

function both(script: Script): CardScripts {
  return { base: script, radiant: script };
}

const ANY_TARGET = [{ kind: "target" as const, min: 1, max: 1, filter: { of: ["unit" as const, "hero" as const] } }];

export const PA = {
  /** A Spell that deals 2 to a declared target (unit or hero); Radiant deals 4. */
  bolt: def("bolt", "Spell", { cost: 1 }),
  /** A Spell with no choices that deals 1 to the enemy hero, so its resolution shows. */
  ping: def("ping", "Spell", { cost: 1 }),
  /** A Unit whose Cry deals 3 to the enemy hero. */
  crier: def("crier", "Unit", { cost: 1 }),
  /** A Unit whose Cry declares a target (not a Spell: Immune to Spells does not stop it). */
  zapper: def("zapper", "Unit", { cost: 1 }),
  /** A Trap that counters the opponent's play, whatever it is (Classic #72's shape). */
  counterTrap: def("counter-trap", "Trap"),
  /** A second one, to prove the rest find no card and stay set. */
  counterTrap2: def("counter-trap-2", "Trap"),
  /** A Trap that counters the opponent's Spell into exile when it cost 1 or less (Classic #10). */
  exileTrap: def("exile-trap", "Trap"),
  /** A Trap that steals the opponent's play (Classic #72 Radiant). */
  stealTrap: def("steal-trap", "Trap"),
  /** A Trap that counters a Spell targeting one of your units (AI Refusal). */
  refusal: def("refusal", "Trap"),
  /** A Field Spell whose aura counters any play that paid 1 (Classic #87's shape, count fixed at 1). */
  chalice: def("chalice", "Field Spell"),
  /** A Field Spell that asks, on the opponent's Spell, whether to Tribute itself to steal it (Classic #4). */
  palantir: def("palantir", "Field Spell"),
  /** A Field Spell that makes its controller discard their hand when the opponent announces anything. */
  shredder: def("shredder", "Field Spell"),
  /** A Trap the opponent sets face-down, and a Field Trap. */
  hiddenTrap: def("hidden-trap", "Trap"),
  hiddenFieldTrap: def("hidden-field-trap", "Field Trap"),
  /** Classic #33 Joro: intercepts the opponent's targeting of your units from your hand. */
  joro: def("joro", "Unit", { attack: 1, health: 3 }),
  /** Classic #89 Paul Allen's Ghost: 2 more cards to target it. */
  ghost: def("ghost", "Unit", { cost: 2, attack: 5, health: 6 }),
  /** A Unit Immune to Spells. */
  immune: def("immune", "Unit", { keywords: [{ kind: "Immune to Spells" }] }),
  /** A Spell whose Cry asks for a unit target (a card continuation's prompt), then deals 2 to it. */
  chooser: def("chooser", "Spell", { cost: 1 }),
  /** A Spell with Echo 1 that deals 1 to a declared unit target: its repeat asks the pipeline's own prompt. */
  echoBolt: def("echo-bolt", "Spell", { cost: 1 }),
  /** Declarations with the v0.2.0 filter fields. */
  graveRaiser: def("grave-raiser", "Spell", { cost: 1 }),
  cheapHunter: def("cheap-hunter", "Spell", { cost: 1 }),
  medic: def("medic", "Spell", { cost: 1 }),
  plagueHunter: def("plague-hunter", "Spell", { cost: 1 }),
  laneHunter: def("lane-hunter", "Spell", { cost: 1 }),
  /** Classic #23 Devil's Pact's modifier, installed by a Spell: plays become Book of Flame. */
  pact: def("pact", "Spell", { cost: 0 }),
  book: def("book", "Spell", { cost: 1, tags: ["Book"] }),
  /** Classic+ #68 Organic Produce: Fruits you play are Radiant. */
  produce: def("produce", "Field Spell", { tags: ["Fruit"] }),
  apple: def("apple", "Spell", { cost: 1, tags: ["Fruit"] }),
  pear: def("pear", "Unit", { cost: 1, tags: ["Fruit"] }),
  /** Classic #57 Echo's record: it records the last Spell played, never itself (R451). */
  echoCopy: def("echo-copy", "Spell", { cost: 1 }),
  /** An AI generated card (B8): the last face-up record passes it over. */
  aiCard: def("ai-card", "Spell", { cost: 0, tags: ["AI", "Token"], rarity: "Token", token: true }),
  /** A Field Spell, for plays by type. */
  field: def("field", "Field Spell"),
  /** A Spell that deals 1 to each of two declared units. */
  twin: def("twin", "Spell", { cost: 1 }),
  /** A Field Spell that asks its controller a question whenever the opponent announces a play. */
  watcher: def("watcher", "Field Spell"),
  /** Quickdraw copies (§6.2: they start in the opening hand), for games folded from a log. */
  qBolt: def("q-bolt", "Spell", { cost: 1 }),
  qPalantir: def("q-palantir", "Field Spell"),
} as const;

const SCRIPTS: Record<string, CardScripts> = {
  [PA.bolt.id]: {
    base: { targets: ANY_TARGET, cry: () => [damage({ to: { of: "chosen" }, amount: 2 })] },
    radiant: { targets: ANY_TARGET, cry: () => [damage({ to: { of: "chosen" }, amount: 4 })] },
  },
  [PA.ping.id]: both({ cry: () => [damage({ to: { of: "enemyHero" }, amount: 1 })] }),
  [PA.crier.id]: both({ cry: () => [damage({ to: { of: "enemyHero" }, amount: 3 })] }),
  [PA.zapper.id]: both({
    targets: [{ kind: "target", min: 1, max: 1 }],
    cry: () => [damage({ to: { of: "chosen" }, amount: 1 })],
  }),
  [PA.counterTrap.id]: both({
    triggers: [
      {
        id: "counter",
        on: ["cardAnnounced"],
        when: (ctx) => ctx.event.type === "cardAnnounced" && ctx.event.player !== ctx.controller,
        run: (ctx) =>
          ctx.event.type === "cardAnnounced"
            ? [counterPlay({ target: { of: "instance", instanceId: ctx.event.instanceId } })]
            : [],
      },
    ],
  }),
  [PA.counterTrap2.id]: both({
    triggers: [
      {
        id: "counter",
        on: ["cardAnnounced"],
        when: (ctx) => ctx.event.type === "cardAnnounced" && ctx.event.player !== ctx.controller,
        run: () => [counterPlay()],
      },
    ],
  }),
  [PA.exileTrap.id]: both({
    triggers: [
      {
        id: "exile",
        on: ["cardAnnounced"],
        when: (ctx) =>
          ctx.event.type === "cardAnnounced" && ctx.event.player !== ctx.controller && ctx.event.costPaid <= 1,
        run: (ctx) =>
          ctx.event.type === "cardAnnounced"
            ? [counterPlay({ to: "exile", target: { of: "instance", instanceId: ctx.event.instanceId } })]
            : [],
      },
    ],
  }),
  [PA.stealTrap.id]: both({
    triggers: [
      {
        id: "steal",
        on: ["cardAnnounced"],
        when: (ctx) => ctx.event.type === "cardAnnounced" && ctx.event.player !== ctx.controller,
        run: () => [counterPlay({ to: "thief" })],
      },
    ],
  }),
  [PA.refusal.id]: both({
    triggers: [
      {
        id: "refuse",
        on: ["cardAnnounced"],
        when: (ctx) => {
          const event = ctx.event;
          if (event.type !== "cardAnnounced" || event.player === ctx.controller || event.cardType !== "Spell") return false;
          const mine = new Set(
            ctx.state.players[ctx.controller].units.flatMap((pile) => (pile === null ? [] : [pile[0]?.id ?? ""])),
          );
          return event.targets.some((id) => mine.has(id));
        },
        run: () => [counterPlay()],
      },
    ],
  }),
  [PA.chalice.id]: both({
    triggers: [
      {
        id: "chalice",
        on: ["cardAnnounced"],
        run: (ctx) =>
          ctx.event.type === "cardAnnounced" && ctx.event.costPaid === 1
            ? [counterPlay({ target: { of: "instance", instanceId: ctx.event.instanceId } })]
            : [],
      },
    ],
  }),
  [PA.palantir.id]: both({
    triggers: [
      {
        id: "palantir",
        on: ["cardAnnounced"],
        run: (ctx) =>
          ctx.event.type === "cardAnnounced" && ctx.event.player !== ctx.controller && ctx.event.cardType === "Spell"
            ? [chooseMode({ options: ["steal", "pass"], step: "decide", data: { stolen: ctx.event.instanceId } })]
            : [],
      },
    ],
    resume: {
      decide: (ctx) =>
        chosenOptions(ctx).includes("steal")
          ? [
              sacrifice({ target: { of: "self" } }),
              counterPlay({ to: "thief", target: { of: "instance", instanceId: String(ctx.data.stolen) } }),
            ]
          : [],
    },
  }),
  [PA.shredder.id]: both({
    triggers: [
      {
        id: "shred",
        on: ["cardAnnounced"],
        run: (ctx) =>
          ctx.event.type === "cardAnnounced" && ctx.event.player !== ctx.controller
            ? [discardHand({ player: "enemy" })]
            : [],
      },
    ],
  }),
  // Its Cry would hit the enemy hero for 5: an interception summons it, so the Cry never fires.
  [PA.joro.id]: both({
    staticFlags: { interceptsTargeting: true },
    cry: () => [damage({ to: { of: "enemyHero" }, amount: 5 })],
  }),
  [PA.ghost.id]: both({ targetingDiscards: () => 2 }),
  [PA.chooser.id]: both({
    cry: () => [chooseTarget({ step: "hit", scope: { side: "any", of: ["unit"] } })],
    resume: { hit: () => [damage({ to: { of: "chosen" }, amount: 2 })] },
  }),
  [PA.echoBolt.id]: both({
    staticFlags: { echo: 1 },
    targets: [{ kind: "target", min: 1, max: 1 }],
    cry: () => [damage({ to: { of: "chosen" }, amount: 1 })],
  }),
  [PA.graveRaiser.id]: both({
    targets: [{ kind: "target", min: 1, max: 1, filter: { side: "any", of: ["graveyard"], type: "Unit" } }],
    cry: () => [],
  }),
  [PA.cheapHunter.id]: both({
    targets: [{ kind: "target", min: 1, max: 1, filter: { of: ["unit", "hand", "hero"], costRange: { max: 1 } } }],
    cry: () => [],
  }),
  [PA.medic.id]: both({
    targets: [{ kind: "target", min: 1, max: 1, filter: { of: ["unit", "hero"], damaged: true } }],
    cry: () => [],
  }),
  [PA.plagueHunter.id]: both({
    targets: [{ kind: "target", min: 1, max: 1, filter: { of: ["unit", "backrow"], plague: true } }],
    cry: () => [],
  }),
  [PA.laneHunter.id]: both({
    targets: [{ kind: "target", min: 1, max: 1, filter: { of: ["unit", "hero", "zone"], check: "lane2" } }],
    targetChecks: {
      lane2: ({ candidate, selection }) =>
        candidate !== null
          ? candidate.zone.z === "field" && candidate.zone.lane === 2
          : selection.pick === "zone" && selection.lane === 2,
    },
    cry: () => [],
  }),
  [PA.pact.id]: {
    base: {
      cry: (ctx) => [
        addPlayerModifier({
          mod: { kind: "replacePlays", defId: PA.book.id, radiant: false, expiry: { until: "thisTurn", turn: ctx.state.turn } },
        }),
      ],
    },
    radiant: {
      cry: (ctx) => [
        addPlayerModifier({
          mod: { kind: "replacePlays", defId: PA.book.id, radiant: true, expiry: { until: "thisTurn", turn: ctx.state.turn } },
        }),
      ],
    },
  },
  [PA.book.id]: {
    base: { targets: ANY_TARGET, cry: () => [damage({ to: { of: "chosen" }, amount: 4 })] },
    radiant: { targets: ANY_TARGET, cry: () => [damage({ to: { of: "chosen" }, amount: 8 })] },
  },
  [PA.produce.id]: both({ staticFlags: { radiantPlaysTagged: ["Fruit"] } }),
  // R214: its Radiant face declares a target its base face does not, so the face step 1 reads shows.
  [PA.apple.id]: {
    base: { cry: () => [] },
    radiant: { targets: ANY_TARGET, cry: () => [damage({ to: { of: "chosen" }, amount: 3 })] },
  },
  [PA.twin.id]: both({
    targets: [{ kind: "target", min: 2, max: 2 }],
    cry: () => [damage({ to: { of: "chosen", index: 0 }, amount: 1 }), damage({ to: { of: "chosen", index: 1 }, amount: 1 })],
  }),
  [PA.watcher.id]: both({
    triggers: [
      {
        id: "watch",
        on: ["cardAnnounced"],
        run: (ctx) =>
          ctx.event.type === "cardAnnounced" && ctx.event.player !== ctx.controller
            ? [chooseMode({ options: ["noted"], step: "noted" })]
            : [],
      },
    ],
    resume: { noted: () => [] },
  }),
  [PA.echoCopy.id]: both({ recordsPlayAs: ({ state }) => lastSpellPlayed(state) }),
  [PA.aiCard.id]: both({ cry: () => [] }),
};

SCRIPTS[PA.qBolt.id] = {
  base: { ...(SCRIPTS[PA.bolt.id]?.base ?? {}), staticFlags: { quickdraw: true } },
  radiant: { ...(SCRIPTS[PA.bolt.id]?.radiant ?? {}), staticFlags: { quickdraw: true } },
};
SCRIPTS[PA.qPalantir.id] = {
  base: { ...(SCRIPTS[PA.palantir.id]?.base ?? {}), staticFlags: { quickdraw: true } },
  radiant: { ...(SCRIPTS[PA.palantir.id]?.radiant ?? {}), staticFlags: { quickdraw: true } },
};

export const PA_DEFS: readonly CardDef[] = Object.values(PA);

/** Register this file's defs and scripts on top of whatever is registered now. */
export function registerPlayA(): void {
  registerCatalog({ ...registeredCatalog(), ...Object.fromEntries(PA_DEFS.map((card) => [card.id, card])) });
  registerScripts({ ...registeredScripts(), ...SCRIPTS });
}

/** `registerPlayA` for a game `newGame` has just made (its catalog is registered by then). */
export function withPlayA(state: GameState): GameState {
  registerPlayA();
  return state;
}

