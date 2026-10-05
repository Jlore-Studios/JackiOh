// C #90 In Too Deep (SPEC §8.6 row 90). (1) Field Spell, Quickdraw, Mythic.
//   Base:    "Indestructible\nQuest: Draw 2 cards. Each quest you complete offers rewards; the reward
//            you choose sets your next quest."
//   Radiant: "Indestructible\nQuest: Draw 2 cards. Each quest you complete gives every reward it
//            offers, and you follow every path."
//   Engine:  "Quests (§10.1, R404): `memory.quest`, the tree data in the card file. Quest 1 opens as
//            the card enters the field. … Completion is noticed at the state check after the event
//            that completes it, on either player's turn, and the reward choice is a `reward` prompt
//            (§10.6) for the card's controller then. … Radiant: each completed quest grants all of its
//            rewards and opens all of their quests, with no reward prompt; a quest reached by two paths
//            opens once, and a reward two completed quests offer (D, G, H) is granted by each. …
//            Quickdraw: it starts in your opening hand (§2.1). Tunes: none."
//
// THE SPLIT (B5 E33). The machinery is the engine's (`subsystems/quests.ts`): it keeps the count on
// the instance (`memory.quest`), opens quest 1 as the card enters the field, counts each event the
// resolution loop reaches from the moment a quest opens, notices a completion at the state check and
// reports it (`questCompleted`), and shows the open quests in both views (§10.8). The TREE below is
// this card's data (`Script.quests`): the ten quests as R404 reads them countable, and the thirteen
// rewards with the quest each leads to. The REWARDS are this card's verbs, run by its own trigger on
// its own `questCompleted`.
//
// BASE. The trigger asks the card's controller a `reward` prompt (§10.6) over the completed quest's
// rewards — on the other player's turn too, a non-active player's prompt (R79) — and the answer runs
// the chosen reward and opens the quest it leads to (`resume.reward`). A quest of one reward (7–10)
// still asks: SPEC makes the reward a prompt. RADIANT. The trigger runs every reward of the quest in
// the tree's order, then opens every quest they lead to: the rewards are what completing the quest
// gives, and the paths begin after them, as the base face's next quest opens after its reward. A
// quest reached by two paths opens once (`openQuest` opens only a quest never opened), and a reward
// two completed quests offer (D, G, H) is granted by each, since each completion runs its own list.
// Counting from the moment a quest opens holds inside a reward too: reward H draws its 2 before quest
// 9 opens, and those draws are not its (R404); a deck H emptied is a deck "already empty when the
// quest opens", which completes quest 9 at once.
//
// THE REWARDS, each with the reading it needs:
//   A  heal your hero 6.
//   B  deal 3 damage to a target — any Unit or hero, either side (§8 Conventions), asked as the reward
//      resolves; no target fizzles and the next quest still opens.
//   C  return 2 random cards from your graveyard to your hand — two different cards (R60), all of them
//      if fewer; a full hand burns one back once (§2.4, `returnRandomFromGraveyard`).
//   D  place 3 Plague Counters (§6.3, R471, R689): three placements, all on the one permanent you choose,
//      either side (`placePlagueTokens`).
//   E  a random Unit of yours gets +3/+3 (`buffRandomUnit`, R60); none, nothing.
//   F  bounce a target permanent — a Unit or a backrow card, either side, face-down ones and this card
//      included; this card bounced ends its own quest line (R78), so its next quest never opens.
//   G  your opponent discards 2 cards of their choice (R16): their own hand prompt; fewer, all they have.
//   H  draw 2.
//   I  Recruit a card (§6.3): the first permanent from the top of your deck.
//   J  gain 100 mana, as next-turn mana (R540): quest 7 completes as your turn ends, so mana for "this
//      turn" would lapse unspent; it is the next refresh's rider, a badge until your next turn.
//   K  exile your opponent's deck: every card of it, bottom up (no draw, no fatigue).
//   L  Aura: you may play cards from your graveyard (§6.3 Play, E11): this card's `graveyardPlay`.
//   M  Aura: your Units have Indestructible: this card's `aura` (§10.4 layer 5); Indestructible gives
//      no Taunt (R347).
// L and M hold while the card stays on the field; a Tribute or an exile ends them, and leaving the
// field resets the quest line (R78). It is Indestructible (catalog keyword): a destroy leaves it (R46).
//
// THE QUESTS, as R404 reads them (counted from the moment each opens):
//   1 draws of yours, 2 (R541: every draw that took a card — burned, cast on draw or kept; a draw a
//     limit stopped and a fatigue draw take none); 2 enemy permanents destroyed by anything, 2 (on the
//     side they died on); 3 permanents you control at once, 3, this card included (board); 4 a turn of
//     yours ending with 3+ unspent mana; 5 damage your cards deal to enemies, 12 (R542: the side the
//     source and the target stood on as the hit landed); 6 your Units' total attack and total health,
//     both 10+, at once (board); 7 a turn of yours ending with 5+ unspent mana; 8 cards entering either
//     exile, 3; 9 a draw of yours that takes your deck's last card; 10 Units in your graveyard, 6
//     (board).
//
// Rulings: R404 (the card), R540 (J's mana), R541 (what a draw is), R542 (whose damage, whose death),
// R543 (Radiant: rewards before paths), R78, R46, R347, R60, R16. No declared numbers (Tunes: none),
// so the amounts are this file's named constants. Its proof: `test/classic/090-in-too-deep.test.ts`.

import { LIBRARY_CAP, subsystems, type Effect, type EffectContext, type Script, type TriggerDef } from "@jackioh/engine";
import {
  bounce,
  buffRandomUnit,
  chooseFromHand,
  chooseReward,
  chooseTarget,
  chosenOptions,
  damage,
  discard,
  draw,
  exileBottomOfLibrary,
  heal,
  nextTurnMana,
  placePlagueTokens,
  recruit,
  returnRandomFromGraveyard,
} from "@jackioh/engine/effects";
import type { GameEvent } from "@jackioh/shared";
import { cardDef } from "../../catalog-data";

export const def = cardDef("classic-090");

// The tree's numbers (SPEC §8.6 row 90; no declared params).
const QUEST_DRAWS = 2;
const QUEST_KILLS = 2;
const QUEST_PERMANENTS = 3;
const QUEST_FLOAT = 3;
const QUEST_DAMAGE = 12;
const QUEST_STATS = 10;
const QUEST_BIG_FLOAT = 5;
const QUEST_EXILES = 3;
const QUEST_GRAVE_UNITS = 6;
const REWARD_HEAL = 6;
const REWARD_DAMAGE = 3;
const REWARD_RETURN = 2;
const REWARD_PLAGUE = 3;
const REWARD_BUFF = 3;
const REWARD_DISCARD = 2;
const REWARD_DRAW = 2;
const REWARD_MANA = 100;

/** The quest tree (SPEC §8.6 row 90, R404): what completes each quest, and where each reward leads. */
const IN_TOO_DEEP_QUESTS: subsystems.QuestBook = {
  first: "1",
  quests: [
    { id: "1", text: "Draw 2 cards", goal: { kind: "draws", count: QUEST_DRAWS }, rewards: ["A", "B"] },
    { id: "2", text: "Destroy 2 enemy permanents", goal: { kind: "enemyPermanentsDestroyed", count: QUEST_KILLS }, rewards: ["C", "D"] },
    { id: "3", text: "Control 3 or more permanents at once", goal: { kind: "permanentsControlled", count: QUEST_PERMANENTS }, rewards: ["D", "E"] },
    { id: "4", text: "End a turn with 3 or more unspent mana", goal: { kind: "unspentManaAtTurnEnd", mana: QUEST_FLOAT }, rewards: ["F", "G"] },
    { id: "5", text: "Your cards deal 12 damage to enemies", goal: { kind: "damageToEnemies", amount: QUEST_DAMAGE }, rewards: ["G", "H"] },
    {
      id: "6",
      text: "Your Units have 10 or more total Attack and 10 or more total health at once",
      goal: { kind: "unitTotals", total: QUEST_STATS },
      rewards: ["H", "I"],
    },
    { id: "7", text: "End a turn with 5 or more unspent mana", goal: { kind: "unspentManaAtTurnEnd", mana: QUEST_BIG_FLOAT }, rewards: ["J"] },
    { id: "8", text: "3 cards enter either exile", goal: { kind: "cardsExiled", count: QUEST_EXILES }, rewards: ["K"] },
    { id: "9", text: "A draw of yours takes the last card of your deck", goal: { kind: "deckEmptiedByDraw" }, rewards: ["L"] },
    { id: "10", text: "Have 6 or more Units in your graveyard", goal: { kind: "unitsInGraveyard", count: QUEST_GRAVE_UNITS }, rewards: ["M"] },
  ],
  rewards: [
    { id: "A", text: "Heal your hero 6", next: "2" },
    { id: "B", text: "Deal 3 damage to a target", next: "3" },
    { id: "C", text: "Return 2 random cards from your graveyard to your hand", next: "4" },
    { id: "D", text: "Place 3 Plague Counters", next: "5" },
    { id: "E", text: "A random Unit of yours gets +3/+3", next: "6" },
    { id: "F", text: "Bounce a target permanent", next: "7" },
    { id: "G", text: "Your opponent discards 2 cards of their choice", next: "8" },
    { id: "H", text: "Draw 2 cards", next: "9" },
    { id: "I", text: "Recruit a card", next: "10" },
    { id: "J", text: "Gain 100 mana on your next turn", next: null },
    { id: "K", text: "Exile your opponent's deck", next: null },
    { id: "L", text: "Aura: you may play cards from your graveyard", next: null },
    { id: "M", text: "Aura: your Units have Indestructible", next: null },
  ],
};

/** The aura rewards, held on the instance while the card stands (`subsystems.holdQuestAura`). */
const GRAVEYARD_AURA = "L";
const INDESTRUCTIBLE_AURA = "M";

// The continuations a reward that asks re-enters.
const TARGET_DAMAGE_STEP = "rewardB";
const BOUNCE_STEP = "rewardF";
const DISCARD_STEP = "rewardG";
const REWARD_STEP = "reward";

/** One reward's own effects, before the quest it leads to opens. */
function rewardEffects(id: string): Effect[] {
  switch (id) {
    case "A":
      return [heal({ target: { of: "selfHero" }, amount: REWARD_HEAL })];
    case "B":
      return [chooseTarget({ step: TARGET_DAMAGE_STEP, scope: { side: "any", of: ["unit", "hero"] }, prompt: "Deal 3 damage" })];
    case "C":
      return [returnRandomFromGraveyard({ count: REWARD_RETURN })];
    case "D":
      return [placePlagueTokens({ count: REWARD_PLAGUE })];
    case "E":
      return [buffRandomUnit({ attack: REWARD_BUFF, health: REWARD_BUFF })];
    case "F":
      return [chooseTarget({ step: BOUNCE_STEP, scope: { side: "any", of: ["unit", "backrow"] }, prompt: "Bounce a permanent" })];
    case "G":
      return [
        chooseFromHand({ of: "enemy", by: "enemy", count: REWARD_DISCARD, step: DISCARD_STEP, prompt: "In Too Deep: discard 2 cards" }),
      ];
    case "H":
      return [draw({ count: REWARD_DRAW })];
    case "I":
      return [recruit()];
    case "J":
      return [nextTurnMana({ amount: REWARD_MANA })];
    case "K":
      // "The deck" is every card of it: the bottom card, again and again, until none is left (a
      // library never holds more than LIBRARY_CAP).
      return [exileBottomOfLibrary({ player: "enemy", count: LIBRARY_CAP })];
    case GRAVEYARD_AURA:
    case INDESTRUCTIBLE_AURA:
      return [subsystems.holdQuestAura(id)];
    default:
      return [];
  }
}

/** The quest a reward leads to, opened once (`openQuest` passes over a quest ever opened). */
function pathOf(id: string): Effect[] {
  const next = subsystems.questRewardOf(IN_TOO_DEEP_QUESTS, id)?.next ?? null;
  return next === null ? [] : [subsystems.openQuest(next)];
}

/** The quest this card's own `questCompleted` names, or null for another card's report. */
function completedHere(ctx: EffectContext & { event: GameEvent }): subsystems.QuestDef | null {
  const event = ctx.event;
  if (event.type !== "questCompleted" || event.instanceId !== ctx.self?.id) return null;
  return subsystems.questDefOf(IN_TOO_DEEP_QUESTS, event.quest);
}

function rewardOptions(quest: subsystems.QuestDef): { id: string; label: string }[] {
  return quest.rewards.map((id) => ({ id, label: subsystems.questRewardOf(IN_TOO_DEEP_QUESTS, id)?.text ?? id }));
}

function questCompleted(radiant: boolean): TriggerDef {
  return {
    id: "quest-completed",
    on: ["questCompleted"],
    run: (ctx) => {
      const quest = completedHere(ctx);
      if (quest === null) return [];
      // R543: every reward, in the tree's order, then every path they lead to.
      if (radiant) return [...quest.rewards.flatMap(rewardEffects), ...quest.rewards.flatMap(pathOf)];
      return [chooseReward({ step: REWARD_STEP, rewards: rewardOptions(quest), prompt: `Quest complete: ${quest.text}` })];
    },
  };
}

function inTooDeep(radiant: boolean): Script {
  return {
    staticFlags: { quickdraw: true },
    quests: IN_TOO_DEEP_QUESTS,
    triggers: [questCompleted(radiant)],
    resume: {
      // The base face's chosen reward, then its path.
      [REWARD_STEP]: (ctx) => {
        const id = chosenOptions(ctx)[0];
        return id === undefined ? [] : [...rewardEffects(id), ...pathOf(id)];
      },
      [TARGET_DAMAGE_STEP]: () => [damage({ to: { of: "chosen" }, amount: REWARD_DAMAGE })],
      [BOUNCE_STEP]: () => [bounce({ target: { of: "chosen" } })],
      [DISCARD_STEP]: (ctx) => ctx.targets.map((_, index) => discard({ target: { of: "chosen", index } })),
    },
    // Reward M: your Units have Indestructible while this stands.
    aura: ({ self }) =>
      subsystems.heldQuestAuras(self).includes(INDESTRUCTIBLE_AURA)
        ? [{ applies: (unit) => unit.controller === self.controller, mod: { keywords: [{ kind: "Indestructible" }] } }]
        : [],
    // Reward L: you may play cards from your graveyard while this stands (E11, R454).
    graveyardPlay: ({ self }) => (subsystems.heldQuestAuras(self).includes(GRAVEYARD_AURA) ? [{}] : []),
  };
}

export const base: Script = inTooDeep(false);

export const radiant: Script = inTooDeep(true);
