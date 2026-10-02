// Test-only cards for the quests subsystem (docs/classic-sets.md B5 E33; `src/subsystems/quests.ts`),
// so the engine half of Classic #90 In Too Deep is proved without `packages/cards` (CLAUDE.md: the
// engine never imports it). The real card arrives with its own script and test file.
//
//   - one Field Spell per goal kind (`GOAL_CARDS`), whose only quest is that goal and offers nothing:
//     each count, and each board condition, proved alone;
//   - `tree`, a small quest tree shaped like In Too Deep's: a reward prompt on the base face, every
//     reward and every path on the Radiant face, a reward that asks a question of its own (a pause
//     mid-reward), a quest reached by two paths, a reward two quests offer, a reward whose own draws
//     come before the quest it opens, and an aura reward;
//   - plain helper Spells and a draw-limiting Field Spell the tests drive the counts with.

import type { CardDef, GameEvent } from "@jackioh/shared";
import { registerCatalog, registeredCatalog } from "../../src/catalog";
import {
  buffRandomUnit,
  chooseReward,
  chooseTarget,
  chosenOptions,
  damage,
  destroy,
  draw,
  exile,
  exileBottomOfLibrary,
  heal,
  recruit,
  returnRandomFromGraveyard,
} from "../../src/effects";
import type { CardScripts, Effect, EffectContext, Script, TriggerDef } from "../../src/script";
import { registerScripts, registeredScripts } from "../../src/scripts";
import {
  heldQuestAuras,
  holdQuestAura,
  openQuest,
  questDefOf,
  questRewardOf,
  type QuestBook,
  type QuestGoal,
} from "../../src/subsystems/quests";

let nextIndex = 3300;
function def(name: string, type: CardDef["type"], extra: Partial<CardDef> = {}): CardDef {
  nextIndex += 1;
  return {
    id: `qf-${name}`,
    index: String(nextIndex),
    name: `${name} (quests)`,
    set: "Core",
    type,
    tags: [],
    rarity: "Common",
    token: false,
    cost: 0,
    base: { keywords: [], text: name },
    radiant: { keywords: [], text: name },
    ...extra,
  };
}

function both(script: Script): CardScripts {
  return { base: script, radiant: script };
}

// ---------------------------------------------------------------------------------------------
// One goal kind per card
// ---------------------------------------------------------------------------------------------

export const GOALS = {
  draws: { kind: "draws", count: 2 },
  enemyPermanentsDestroyed: { kind: "enemyPermanentsDestroyed", count: 2 },
  unspentManaAtTurnEnd: { kind: "unspentManaAtTurnEnd", mana: 3 },
  damageToEnemies: { kind: "damageToEnemies", amount: 5 },
  cardsExiled: { kind: "cardsExiled", count: 2 },
  deckEmptiedByDraw: { kind: "deckEmptiedByDraw" },
  permanentsControlled: { kind: "permanentsControlled", count: 3 },
  unitTotals: { kind: "unitTotals", total: 6 },
  unitsInGraveyard: { kind: "unitsInGraveyard", count: 2 },
} as const satisfies Record<QuestGoal["kind"], QuestGoal>;

export type GoalKind = keyof typeof GOALS;

/** The id of the one quest each goal card has. */
export const ONLY_QUEST = "q";

function goalBook(goal: QuestGoal): QuestBook {
  return { first: ONLY_QUEST, quests: [{ id: ONLY_QUEST, text: `goal: ${goal.kind}`, goal, rewards: [] }], rewards: [] };
}

export const GOAL_CARDS = Object.fromEntries(
  (Object.keys(GOALS) as GoalKind[]).map((kind) => [kind, def(`goal-${kind}`, "Field Spell", { cost: 1 })]),
) as Record<GoalKind, CardDef>;

// ---------------------------------------------------------------------------------------------
// The tree
// ---------------------------------------------------------------------------------------------

export const tree = def("tree", "Field Spell", { cost: 1 });

/** The amounts the tree's rewards deal, heal and draw. */
export const TREE_HEAL = 2;
export const TREE_PING = 1;
export const TREE_BUFF = 2;

/**
 * draws 2 → heal (→ kill) or ask (→ board); kill → both (→ mana); board → both (→ mana) or hand
 * (→ fresh); mana → aura (end); fresh (draws 2) → heal (→ kill). So `mana` is reached by two paths,
 * `both` is offered by two quests, `ask` pauses for a target, `hand` draws 2 before `fresh` opens,
 * and `aura` is held.
 */
export const TREE: QuestBook = {
  first: "draws",
  quests: [
    { id: "draws", text: "Draw 2 cards", goal: { kind: "draws", count: 2 }, rewards: ["heal", "ask"] },
    { id: "kill", text: "Destroy an enemy permanent", goal: { kind: "enemyPermanentsDestroyed", count: 1 }, rewards: ["both"] },
    { id: "board", text: "Control 3 permanents", goal: { kind: "permanentsControlled", count: 3 }, rewards: ["both", "hand"] },
    { id: "mana", text: "End a turn with 3 mana", goal: { kind: "unspentManaAtTurnEnd", mana: 3 }, rewards: ["aura"] },
    { id: "fresh", text: "Draw 2 more cards", goal: { kind: "draws", count: 2 }, rewards: ["heal"] },
  ],
  rewards: [
    { id: "heal", text: "Heal your hero 2", next: "kill" },
    { id: "ask", text: "Deal 1 damage to a target", next: "board" },
    { id: "both", text: "A random Unit of yours gets +2/+2", next: "mana" },
    { id: "hand", text: "Draw 2 cards", next: "fresh" },
    { id: "aura", text: "Aura: your Units have Indestructible", next: null },
  ],
};

const TARGET_STEP = "aimed";

/** A reward's own effects, before the quest it opens. */
function rewardEffects(id: string): Effect[] {
  switch (id) {
    case "heal":
      return [heal({ target: { of: "selfHero" }, amount: TREE_HEAL })];
    case "ask":
      return [chooseTarget({ step: TARGET_STEP, scope: { side: "any", of: ["unit", "hero"] }, prompt: "Deal 1 damage" })];
    case "both":
      return [buffRandomUnit({ attack: TREE_BUFF, health: TREE_BUFF })];
    case "hand":
      return [draw({ count: 2 })];
    case "aura":
      return [holdQuestAura("aura")];
    default:
      return [];
  }
}

function nextOf(id: string): Effect[] {
  const next = questRewardOf(TREE, id)?.next ?? null;
  return next === null ? [] : [openQuest(next)];
}

function completedQuest(ctx: EffectContext & { event: GameEvent }): string | null {
  const event = ctx.event;
  if (event.type !== "questCompleted" || event.instanceId !== ctx.self?.id) return null;
  return event.quest;
}

function treeScript(radiant: boolean): Script {
  const answer: TriggerDef = {
    id: "quest-completed",
    on: ["questCompleted"],
    run: (ctx) => {
      const id = completedQuest(ctx);
      const quest = id === null ? null : questDefOf(TREE, id);
      if (quest === null) return [];
      if (radiant) {
        // Every reward, then every path.
        return [...quest.rewards.flatMap(rewardEffects), ...quest.rewards.flatMap(nextOf)];
      }
      return [
        chooseReward({
          step: "reward",
          rewards: quest.rewards.map((reward) => ({ id: reward, label: questRewardOf(TREE, reward)?.text ?? reward })),
          prompt: `${quest.text}: choose a reward`,
        }),
      ];
    },
  };
  return {
    // Quickdraw, so a replayable game deals it in the opening hand (§2.1, R225).
    staticFlags: { quickdraw: true },
    quests: TREE,
    triggers: [answer],
    resume: {
      reward: (ctx) => {
        const id = chosenOptions(ctx)[0];
        return id === undefined ? [] : [...rewardEffects(id), ...nextOf(id)];
      },
      [TARGET_STEP]: () => [damage({ to: { of: "chosen" }, amount: TREE_PING })],
    },
    aura: ({ self }) =>
      heldQuestAuras(self).includes("aura")
        ? [{ applies: (unit) => unit.controller === self.controller, mod: { keywords: [{ kind: "Indestructible" }] } }]
        : [],
  };
}

// ---------------------------------------------------------------------------------------------
// Helpers the tests drive the counts with
// ---------------------------------------------------------------------------------------------

/** Draw 1, and draw 2. */
export const drawOne = def("draw-one", "Spell");
export const drawTwo = def("draw-two", "Spell");
/** Destroy a declared unit (either side). */
export const slay = def("slay", "Spell");
/** Exile a declared unit (either side). */
export const banish = def("banish", "Spell");
/** Deal 3 to a declared unit or hero. */
export const bolt = def("bolt", "Spell");
export const BOLT = 3;
/** Draw 2, then Recruit (§6.3): a quest card recruited after the draws does not count them (R212). */
export const drawThenRecruit = def("draw-then-recruit", "Spell");
/** Exile a declared backrow card (either side). */
export const banishAny = def("banish-any", "Spell");
/** Exile the bottom card of your library: it leaves the deck, but no draw takes it. */
export const mill = def("mill", "Spell");
/** "You can't draw more than 1 card each turn" (B5 E3). */
export const limiter = def("limiter", "Field Spell", { cost: 1 });
/** A Field Spell with no text: the other half of a Fuse with the quest tree (R102). */
export const blank = def("blank", "Field Spell", { cost: 1 });

const unitTarget = [{ kind: "target" as const, min: 1, max: 1, filter: { side: "any" as const, of: ["unit" as const] } }];
const backrowTarget = [{ kind: "target" as const, min: 1, max: 1, filter: { side: "any" as const, of: ["backrow" as const] } }];
const anyTarget = [{ kind: "target" as const, min: 1, max: 1, filter: { side: "any" as const, of: ["unit" as const, "hero" as const] } }];

const HELPER_SCRIPTS: Record<string, CardScripts> = {
  [drawOne.id]: both({ cry: () => [draw({ count: 1 })] }),
  [drawTwo.id]: both({ staticFlags: { quickdraw: true }, cry: () => [draw({ count: 2 })] }),
  [drawThenRecruit.id]: both({ cry: () => [draw({ count: 2 }), recruit()] }),
  [banishAny.id]: both({ targets: backrowTarget, cry: () => [exile({ target: { of: "chosen" } })] }),
  [slay.id]: both({ targets: unitTarget, cry: () => [destroy({ target: { of: "chosen" } })] }),
  [banish.id]: both({ targets: unitTarget, cry: () => [exile({ target: { of: "chosen" } })] }),
  [bolt.id]: both({ targets: anyTarget, cry: () => [damage({ to: { of: "chosen" }, amount: BOLT })] }),
  [mill.id]: both({ cry: () => [exileBottomOfLibrary({ count: 1 })] }),
  [limiter.id]: both({ drawLimit: () => [{ player: "self", count: 1 }] }),
};

// ---------------------------------------------------------------------------------------------
// Registration
// ---------------------------------------------------------------------------------------------

/** Graveyard cards back to hand: `returnRandomFromGraveyard` proved through a Spell. */
export const recall = def("recall", "Spell");
export const RECALL = 2;
/** A random Unit of yours +1/+1: `buffRandomUnit` proved through a Spell. */
export const bless = def("bless", "Spell");

export const QUEST_DEFS: CardDef[] = [tree, drawOne, drawTwo, drawThenRecruit, banishAny, slay, banish, bolt, mill, limiter, blank, recall, bless, ...Object.values(GOAL_CARDS)];

const QUEST_SCRIPTS: Record<string, CardScripts> = {
  [tree.id]: { base: treeScript(false), radiant: treeScript(true) },
  ...HELPER_SCRIPTS,
  [recall.id]: both({ cry: () => [returnRandomFromGraveyard({ count: RECALL })] }),
  [bless.id]: both({ cry: () => [buffRandomUnit({ attack: 1, health: 1 })] }),
  ...Object.fromEntries(
    (Object.keys(GOALS) as GoalKind[]).map((kind) => [GOAL_CARDS[kind].id, both({ quests: goalBook(GOALS[kind]) })]),
  ),
};

/** Add these fixtures to whatever the harness registered (`newGame` registers its own first). */
export function registerQuestFixtures(): void {
  registerCatalog({ ...registeredCatalog(), ...Object.fromEntries(QUEST_DEFS.map((card) => [card.id, card])) });
  registerScripts({ ...registeredScripts(), ...QUEST_SCRIPTS });
}
