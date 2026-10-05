// C #90 In Too Deep (SPEC §8.6 row 90; BUILD M9 row C 90; R404, R540–R543). (1) Field Spell,
// Quickdraw, Mythic: Indestructible, and a quest line — ten quests, thirteen rewards, a `reward`
// prompt on the base face, every reward and every path on the Radiant face. Quest 1's text lives in
// the quest line, not the card text: it appears on the card face only after the card is played.
//
// The quest machinery is the engine's (`subsystems/quests.ts`, proved alone by
// `packages/engine/test/quests.test.ts`); this file proves the card: its tree, each of its ten quests
// as R404 reads them, each of its thirteen rewards and the quest it leads to, both faces, both views,
// a pause through JSON and a game folded from its log.
//
// Reaching a deep quest by playing the whole path before it would make each test the length of the
// tree, so a test of quest N writes the quest line the path would have left (`line`), as the Heroic
// Power tests write the power its arrival rolls: the path itself is proved by the reward tests, each
// of which checks the quest it opens. A reward test completes its quest by writing its count at the
// goal and letting the next state check notice it (`completeAtNextCheck`), the counting being each
// quest's own test.

import { describe, expect, it } from "vitest";
import {
  flagsOf,
  hashState,
  legalActions,
  reduce,
  subsystems,
  beginGame,
  createGame,
  fold,
  viewFor,
  type GameState,
} from "@jackioh/engine";
import type { Action, ActionInput, CardView, GameEvent, PlayerId } from "@jackioh/shared";
import { hasKeyword } from "@jackioh/shared";
import { base, def, radiant } from "../../src/scripts/classic/090-in-too-deep";
import { scenario, type Scenario, type SideSetup } from "../_harness";

const ITD = "classic-090";

const STOCKPILE = "core-005"; // Spell 1: draw 2, heal your hero 2
const NOTEBOOK = "core-051-1"; // Spell 1: draw 1
const HIT_JOB = "core-016"; // Spell 3: destroy target Unit
const NETHER = "core-088"; // Spell 4: destroy all permanents
const LUNAR = "core-035"; // Spell 1: 3 damage to a target
const TRUE_STRIKE = "core-044"; // Spell 1: Pierce, 4 damage, exile this
const COLLATERAL = "core-034"; // Spell 4: exile target permanent and a random card of the enemy deck
const MAGIC_JAMMED = "core-036"; // Spell 1: destroy target backrow card, Lock its zone
const EUGENICS = "core-042"; // Spell 2: exile 7 random cards from your deck
const VANILLA = "core-008"; // 4/4
const TIMMY = "core-011"; // 3/3 Rush, First Strike
const MENACE = "core-019"; // 9/9 Taunt
const TOKEN_MAKER = "core-015"; // 1/1, Cry: summon a Rush Token
const VIRUS = "core-090-1"; // cast on draw: take 1 damage
const RUSH_TOKEN = "core-t-rush";

/** The cards a side holds so §2.5 never ends a turn on its own. */
const SPARE: SideSetup = { hand: [NOTEBOOK, STOCKPILE, TIMMY], library: [VANILLA, VANILLA, VANILLA, VANILLA] };
const MIND_CONTROL = "core-049"; // Spell 4: steal target enemy permanent
const MANA_WELL = "core-006"; // Field Spell 3
const GIFTED = "core-064"; // Field Spell 2
const EXPERIMENT = "core-085"; // Trap: fuse the opponent's played permanent onto one of yours of its type
const LOCKDOWN = "classic-084"; // Field Spell: Indestructible; Activate: Tribute this

function line(s: Scenario): subsystems.QuestMemory | null {
  return subsystems.questMemoryOf(s.card(ITD));
}

function must<T>(value: T | null | undefined, what: string): T {
  if (value === null || value === undefined) throw new Error(`expected ${what}`);
  return value;
}

/** Write the quest line a path through the tree would have left (see the header). */
function setLine(s: Scenario, memory: Partial<subsystems.QuestMemory>): void {
  s.card(ITD).memory[subsystems.QUEST_MEMORY_KEY] = {
    active: [],
    progress: {},
    done: [],
    auras: [],
    waiting: [],
    ...memory,
  };
}

/** Quest `id` open with its count at `progress`. */
function onQuest(s: Scenario, id: string, progress = 0, extra: Partial<subsystems.QuestMemory> = {}): void {
  setLine(s, { active: [id], progress: { [id]: progress }, ...extra });
}

/** The Notebook's draw on a deck of spares: one action, so one state check after it. */
function anyAction(s: Scenario): void {
  s.play(NOTEBOOK);
}

/** The In Too Deep card view as `viewer` is shown it. */
function shown(s: Scenario, viewer: PlayerId): CardView {
  const view = s.view(viewer);
  const card = s.card(ITD);
  const side = view.you.player === card.controller ? view.you : view.opponent;
  for (const entry of side.backrow) {
    if (entry !== null && !entry.faceDown && entry.instanceId === card.id) return entry;
  }
  throw new Error(`${viewer} is not shown In Too Deep`);
}

function completedIn(events: readonly GameEvent[]): string[] {
  return events.flatMap((event) => (event.type === "questCompleted" ? [event.quest] : []));
}

function rewardKeys(s: Scenario, player: PlayerId = "p1"): string[] {
  const pending = must(s.state.pending, "a reward prompt");
  expect(pending.kind).toBe("reward");
  expect(pending.playerId).toBe(player);
  return pending.options.map((option) => option.key);
}

function pendingKind(s: Scenario): string | null {
  return s.state.pending?.kind ?? null;
}

describe("C #90 In Too Deep", () => {
  describe("base", () => {
    it("§2.1 Quickdraw: both faces carry the flag that starts it in the opening hand", () => {
      const s = scenario({ p1: { library: [ITD, { def: ITD, radiant: true }] } });
      const [plain, shiny] = s.pile("p1", "library");
      expect(flagsOf(must(plain, "base")).quickdraw).toBe(true);
      expect(flagsOf(must(shiny, "radiant")).quickdraw).toBe(true);
    });

    it("R404 the quest tree is data in the card file: ten quests, their rewards, and where each reward leads", () => {
      for (const script of [base, radiant]) {
        const book = must(script.quests, "the tree");
        expect(book.first).toBe("1");
        expect(Object.fromEntries(book.quests.map((q) => [q.id, q.rewards.join("")]))).toEqual({
          1: "AB", 2: "CD", 3: "DE", 4: "FG", 5: "GH", 6: "HI", 7: "J", 8: "K", 9: "L", 10: "M",
        });
        expect(Object.fromEntries(book.rewards.map((r) => [r.id, r.next]))).toEqual({
          A: "2", B: "3", C: "4", D: "5", E: "6", F: "7", G: "8", H: "9", I: "10", J: null, K: null, L: null, M: null,
        });
      }
    });

    it("R46 Indestructible: a destroy leaves it in its zone with its quest line", () => {
      const s = scenario({ p1: { backrow: [ITD], ...SPARE }, p2: { hand: [MAGIC_JAMMED, STOCKPILE] }, active: "p2" });
      onQuest(s, "2", 1);
      s.play(MAGIC_JAMMED, { targets: [{ pick: "instance", instanceId: s.card(ITD).id }] });
      s.expectInZone(ITD, "field");
      expect(line(s)).toMatchObject({ active: ["2"], progress: { "2": 1 } });
    });

    it("quest 1 lives in the quest line, not the card text: the static face names no quest", () => {
      // Balance patch 1: the first quest appears on the card face only after it is played.
      expect(def.base.text).not.toContain("Quest:");
      expect(def.radiant.text).not.toContain("Quest:");
      const s = scenario({ p1: { hand: [ITD, ...(SPARE.hand ?? [])], library: SPARE.library }, p2: SPARE });
      s.play(ITD);
      expect(shown(s, "p1").quest?.open.map((quest) => quest.id)).toEqual(["1"]);
      expect(shown(s, "p2").quest?.open.map((quest) => quest.id)).toEqual(["1"]);
    });

    it("R404 quest 1 opens as it enters: 0 of 2, and both views show it with rewards A and B on offer", () => {
      const s = scenario({ p1: { hand: [ITD, ...(SPARE.hand ?? [])], library: SPARE.library }, p2: SPARE });
      s.play(ITD);
      expect(line(s)).toEqual({ active: ["1"], progress: { "1": 0 }, done: [], auras: [], waiting: [] });
      for (const viewer of ["p1", "p2"] as const) {
        expect(shown(s, viewer).quest).toEqual({
          open: [
            {
              id: "1",
              text: "Draw 2 cards",
              progress: 0,
              goal: 2,
              rewards: [
                { id: "A", text: "Heal your hero 6" },
                { id: "B", text: "Deal 3 damage to a target" },
              ],
            },
          ],
          auras: [],
        });
        // `questProgressed` names the public card in both views.
        const reports = s.view(viewer).events.filter((e) => e.type === "questProgressed");
        expect(reports.map((e) => (e.type === "questProgressed" ? e.instanceId : ""))).toEqual([s.card(ITD).id]);
      }
    });

    it("R404 counted from the moment it opens: draws before it entered do not count, two after it complete quest 1", () => {
      const s = scenario({ p1: { hand: [STOCKPILE, ITD, STOCKPILE, TIMMY], library: [VANILLA, VANILLA, VANILLA, VANILLA, VANILLA] }, p2: SPARE });
      s.play(STOCKPILE);
      s.play(ITD);
      expect(line(s)?.progress).toEqual({ "1": 0 });
      s.play(STOCKPILE);
      expect(completedIn(s.lastEvents)).toEqual(["1"]);
      expect(rewardKeys(s)).toEqual(["mode:A", "mode:B"]);
      expect(must(s.state.pending, "the prompt").options.map((o) => o.label)).toEqual(["Heal your hero 6", "Deal 3 damage to a target"]);
      // `legalActions` offers the two answers to the prompt's holder and none to the other seat.
      const answers = legalActions(s.state, "p1").filter((a) => a.type === "answer");
      expect(answers.map((a) => (a.type === "answer" ? a.selection : []))).toEqual([
        [{ pick: "mode", option: "A" }],
        [{ pick: "mode", option: "B" }],
      ]);
      expect(legalActions(s.state, "p2").some((a) => a.type === "answer")).toBe(false);
      expect(line(s)).toMatchObject({ active: [], done: ["1"] });
    });

    it("R404 the opponent's draws count for nothing of yours", () => {
      const s = scenario({ p1: { backrow: [ITD], ...SPARE }, p2: SPARE });
      s.endTurn();
      expect(s.state.active).toBe("p2");
      expect(s.events.some((e) => e.type === "drawn" && e.player === "p2")).toBe(true);
      expect(line(s)?.progress).toEqual({ "1": 0 });
    });

    it("R541 a draw burned at the hand cap and a draw cast on draw are draws of yours", () => {
      // Eleven cards: the Notebook leaves ten, and its draw comes to a full hand (§2.4, R4).
      const burn = scenario({
        p1: { backrow: [ITD], hand: [NOTEBOOK, ...Array.from({ length: 10 }, () => VANILLA)], library: [VANILLA] },
        p2: SPARE,
      });
      burn.play(NOTEBOOK);
      expect(burn.lastEvents.some((e) => e.type === "burned")).toBe(true);
      expect(line(burn)?.progress).toEqual({ "1": 1 });

      const cast = scenario({ p1: { backrow: [ITD], hand: [NOTEBOOK, TIMMY], library: [VIRUS, VANILLA, VANILLA] }, p2: SPARE });
      cast.play(NOTEBOOK);
      // The virus is drawn and cast (R58), and the draw goes on: two draws of yours.
      expect(cast.lastEvents.filter((e) => e.type === "drawn" && e.player === "p1")).toHaveLength(2);
      expect(completedIn(cast.lastEvents)).toEqual(["1"]);
    });

    it("R404 reward A: heal your hero 6, then quest 2 opens", () => {
      const s = scenario({ p1: { backrow: [ITD], health: 20, ...SPARE }, p2: SPARE });
      s.play(STOCKPILE);
      // Stockpile healed 2; A heals 6.
      s.answer("A");
      s.expectHealth("p1", 28);
      expect(line(s)).toMatchObject({ active: ["2"], done: ["1"] });
      expect(shown(s, "p2").quest?.open.map((q) => [q.id, q.progress, q.goal])).toEqual([["2", 0, 2]]);
    });

    it("R404 reward B: deal 3 damage to a target, asked as it resolves, then quest 3 opens", () => {
      const s = scenario({ p1: { backrow: [ITD], ...SPARE }, p2: { field: [VANILLA], ...SPARE } });
      s.play(STOCKPILE);
      s.answer("B");
      expect(pendingKind(s)).toBe("target");
      // Quest 3 waits for the reward to finish.
      expect(line(s)?.active).toEqual([]);
      const foe = must(s.unit("p2", 1), "the enemy unit");
      s.answer(foe.id);
      expect(s.card(foe).damage).toBe(3);
      expect(line(s)).toMatchObject({ active: ["3"], done: ["1"] });
    });

    it("R404 quest 2: two enemy permanents destroyed by anything; your own deaths count for nothing", () => {
      const s = scenario({
        p1: { backrow: [ITD], field: [TIMMY], hand: [HIT_JOB, NETHER, STOCKPILE], library: SPARE.library, mana: 10 },
        p2: { field: [VANILLA, { def: RUSH_TOKEN }], ...SPARE },
      });
      onQuest(s, "2");
      s.play(HIT_JOB, { targets: [{ pick: "instance", instanceId: must(s.unit("p1", 1), "Timmy").id }] });
      expect(line(s)?.progress).toEqual({ "2": 0 });
      // Twisting Nether destroys both enemy permanents (a token among them) and leaves In Too Deep.
      s.play(NETHER);
      s.expectInZone(ITD, "field");
      expect(completedIn(s.lastEvents)).toEqual(["2"]);
      expect(rewardKeys(s)).toEqual(["mode:C", "mode:D"]);
    });

    it("R542 quest 2 counts the side a permanent died on: an enemy Unit you have stolen is yours as it dies", () => {
      const s = scenario({
        p1: { backrow: [ITD], hand: [MIND_CONTROL, HIT_JOB, HIT_JOB, STOCKPILE], library: SPARE.library, mana: 10 },
        p2: { field: [VANILLA, TIMMY], ...SPARE },
      });
      onQuest(s, "2");
      const vanilla = must(s.unit("p2", 1), "Vanilla");
      s.play(MIND_CONTROL, { targets: [{ pick: "instance", instanceId: vanilla.id }] });
      expect(s.card(vanilla).controller).toBe("p1");
      s.play(HIT_JOB, { targets: [{ pick: "instance", instanceId: vanilla.id }] });
      expect(s.lastEvents.find((e) => e.type === "destroyed")).toMatchObject({ owner: "p2", controller: "p1" });
      expect(line(s)?.progress).toEqual({ "2": 0 });
      const timmy = must(s.unit("p2", 2), "Timmy");
      s.play(HIT_JOB, { targets: [{ pick: "instance", instanceId: timmy.id }] });
      expect(s.lastEvents.find((e) => e.type === "destroyed")).toMatchObject({ owner: "p2", controller: "p2" });
      expect(line(s)?.progress).toEqual({ "2": 1 });
    });

    it("R404 reward C: 2 different random cards from your graveyard to your hand, then quest 4", () => {
      const s = scenario({ p1: { backrow: [ITD], graveyard: [VANILLA, TIMMY, MENACE], ...SPARE }, p2: SPARE });
      onQuest(s, "2", 2);
      anyAction(s);
      // The Notebook has drawn and gone to the graveyard: four cards lie there, three in hand.
      expect([s.hand("p1").length, s.pile("p1", "graveyard").length]).toEqual([3, 4]);
      s.answer("C");
      const back = s.lastEvents.flatMap((e) => (e.type === "addedToHand" ? [e.instanceId] : []));
      expect(new Set(back).size).toBe(2);
      expect([s.hand("p1").length, s.pile("p1", "graveyard").length]).toEqual([5, 2]);
      expect(line(s)).toMatchObject({ active: ["4"], done: ["2"] });
    });

    it("R60 reward C with one card in the graveyard returns that one", () => {
      const s = scenario({ p1: { backrow: [ITD], graveyard: [MENACE], ...SPARE }, p2: SPARE });
      onQuest(s, "2", 2);
      anyAction(s);
      // The Notebook itself reached the graveyard before the reward: two cards lie there now.
      s.answer("C");
      expect(s.hand("p1").map((c) => c.defId)).toEqual(expect.arrayContaining([MENACE, NOTEBOOK]));
      expect(s.pile("p1", "graveyard")).toEqual([]);
    });

    it("R471 R669 reward D: three Plague Token placements on the one permanent a single prompt of yours names, then quest 5", () => {
      const s = scenario({ p1: { backrow: [ITD], field: [VANILLA], ...SPARE }, p2: { field: [MENACE], ...SPARE } });
      onQuest(s, "2", 2);
      anyAction(s);
      s.answer("D");
      const foe = must(s.unit("p2", 1), "Menace");
      expect(pendingKind(s)).toBe("target");
      expect(line(s)?.active).toEqual([]);
      // One answer puts all three on the pick: no second prompt opens.
      s.answer(foe.id);
      expect(s.card(foe).counters.plague).toBe(3);
      expect(line(s)).toMatchObject({ active: ["5"], done: ["2"] });
    });

    it("R404 quest 3: 3 or more permanents of yours at once, this card included; the opponent's do not count", () => {
      const s = scenario({ p1: { backrow: [ITD], field: [TIMMY], hand: [VANILLA, STOCKPILE], library: SPARE.library }, p2: { field: [MENACE, VANILLA], ...SPARE } });
      onQuest(s, "3");
      expect(shown(s, "p1").quest?.open[0]).toMatchObject({ id: "3", progress: 2, goal: 3 });
      s.play(VANILLA);
      expect(completedIn(s.lastEvents)).toEqual(["3"]);
      expect(rewardKeys(s)).toEqual(["mode:D", "mode:E"]);
    });

    it("R60 reward E: a random Unit of yours gets +3/+3, then quest 6; with none, nothing, and quest 6 still opens", () => {
      // Two small Units (their totals stay under quest 6's 10 after the +3/+3) and In Too Deep.
      const s = scenario({ p1: { backrow: [ITD], field: [TIMMY, TOKEN_MAKER], ...SPARE }, p2: { field: [MENACE], ...SPARE } });
      onQuest(s, "3");
      anyAction(s);
      s.answer("E");
      const buffed = s.lastEvents.filter((e) => e.type === "buffed");
      expect(buffed).toHaveLength(1);
      expect(buffed[0]).toMatchObject({ attack: 3, health: 3 });
      expect([must(s.unit("p1", 1), "a").id, must(s.unit("p1", 2), "b").id]).toContain(buffed[0]?.type === "buffed" ? buffed[0].instanceId : "");
      expect(line(s)).toMatchObject({ active: ["6"], done: ["3"] });

      // No Unit of yours: three backrow permanents complete quest 3, and E buffs nothing.
      const none = scenario({ p1: { backrow: [ITD, MANA_WELL, GIFTED], ...SPARE }, p2: { field: [MENACE], ...SPARE } });
      onQuest(none, "3");
      anyAction(none);
      none.answer("E");
      expect(none.lastEvents.some((e) => e.type === "buffed")).toBe(false);
      expect(line(none)).toMatchObject({ active: ["6"], done: ["3"] });
    });

    it("R404 quest 4: a turn of yours ending with 3 or more unspent mana; 2 is not enough, and the opponent's turn is not yours", () => {
      const s = scenario({ p1: { backrow: [ITD], ...SPARE, mana: 3 }, p2: SPARE });
      onQuest(s, "4");
      s.endTurn();
      expect(completedIn(s.lastEvents)).toEqual(["4"]);
      expect(rewardKeys(s)).toEqual(["mode:F", "mode:G"]);

      const short = scenario({ p1: { backrow: [ITD], ...SPARE, mana: 2 }, p2: SPARE });
      onQuest(short, "4");
      short.endTurn();
      expect(short.state.pending).toBeNull();
      expect(short.state.active).toBe("p2");
      short.state.players.p2.mana.current = 4;
      short.endTurn();
      expect(line(short)?.progress).toEqual({ "4": 0 });
    });

    it("R404 reward F: bounce a target permanent, then quest 7", () => {
      const s = scenario({ p1: { backrow: [ITD], ...SPARE }, p2: { field: [MENACE], ...SPARE } });
      onQuest(s, "4", 1);
      anyAction(s);
      s.answer("F");
      expect(pendingKind(s)).toBe("target");
      const foe = must(s.unit("p2", 1), "Menace");
      s.answer(foe.id);
      s.expectInZone(foe, "hand");
      expect(line(s)).toMatchObject({ active: ["7"], done: ["4"] });
    });

    it("R78 reward F on In Too Deep itself: it goes to your hand, its quest line ends, and played again it starts at quest 1", () => {
      const s = scenario({ p1: { backrow: [ITD], ...SPARE }, p2: SPARE });
      onQuest(s, "4", 1);
      anyAction(s);
      s.answer("F");
      s.answer(s.card(ITD).id);
      s.expectInZone(ITD, "hand");
      expect(line(s)).toBeNull();
      s.play(ITD);
      expect(line(s)).toEqual({ active: ["1"], progress: { "1": 0 }, done: [], auras: [], waiting: [] });
    });

    it("R16 reward G: your opponent discards 2 cards of their choice, their own prompt, then quest 8", () => {
      const s = scenario({ p1: { backrow: [ITD], ...SPARE }, p2: { hand: [VANILLA, TIMMY, MENACE], library: SPARE.library } });
      onQuest(s, "4", 1);
      anyAction(s);
      s.answer("G");
      const pending = must(s.state.pending, "the discard prompt");
      expect(pending.kind).toBe("hand");
      expect(pending.playerId).toBe("p2");
      expect([pending.min, pending.max]).toEqual([2, 2]);
      // p1 sees only that a prompt is open (R81, §10.8).
      expect(s.view("p1").pending).toEqual({ forYou: false, pendingFor: "p2" });
      const [first, second] = s.hand("p2");
      s.answer([must(first, "a card").id, must(second, "a card").id]);
      expect(s.hand("p2").map((c) => c.defId)).toEqual([MENACE]);
      expect(line(s)).toMatchObject({ active: ["8"], done: ["4"] });
    });

    it("R16 reward G with one card in the opponent's hand discards that one", () => {
      const s = scenario({ p1: { backrow: [ITD], ...SPARE }, p2: { hand: [MENACE], library: SPARE.library } });
      onQuest(s, "4", 1);
      anyAction(s);
      s.answer("G");
      expect([s.state.pending?.min, s.state.pending?.max]).toEqual([1, 1]);
      s.answer(must(s.hand("p2")[0], "Menace").id);
      expect(s.hand("p2")).toEqual([]);
    });

    it("R542 quest 5: damage your cards deal to the enemy hero and enemy units, 12 in all; your own side and the opponent's hits count for nothing", () => {
      const s = scenario({
        p1: { backrow: [ITD], field: [MENACE], hand: [LUNAR, LUNAR, STOCKPILE], library: SPARE.library, mana: 10 },
        p2: { field: [VANILLA], ...SPARE },
      });
      onQuest(s, "5");
      s.play(LUNAR, { targets: [{ pick: "hero", player: "p1" }] });
      expect(line(s)?.progress).toEqual({ "5": 0 });
      s.play(LUNAR, { targets: [{ pick: "instance", instanceId: must(s.unit("p2", 1), "Vanilla").id }] });
      expect(line(s)?.progress).toEqual({ "5": 3 });
      s.attack(MENACE, "hero");
      expect(completedIn(s.lastEvents)).toEqual(["5"]);
      expect(rewardKeys(s)).toEqual(["mode:G", "mode:H"]);
      expect(shown(s, "p2").quest?.open).toEqual([]);
    });

    it("R404 reward H: draw 2, then quest 9 — whose count starts after those draws", () => {
      const s = scenario({ p1: { backrow: [ITD], ...SPARE }, p2: SPARE });
      onQuest(s, "5", 12);
      anyAction(s);
      s.answer("H");
      expect(s.lastEvents.filter((e) => e.type === "drawn")).toHaveLength(2);
      expect(line(s)).toMatchObject({ active: ["9"], done: ["5"], progress: { "9": 0 } });
    });

    it("R404 reward H that empties your deck opens quest 9 on an empty deck, which completes it at once", () => {
      const s = scenario({ p1: { backrow: [ITD], hand: [NOTEBOOK, TIMMY], library: [VANILLA, VANILLA, VANILLA] }, p2: SPARE });
      onQuest(s, "5", 12);
      s.play(NOTEBOOK);
      s.answer("H");
      expect(s.pile("p1", "library")).toEqual([]);
      expect(completedIn(s.lastEvents)).toEqual(["9"]);
      expect(rewardKeys(s)).toEqual(["mode:L"]);
    });

    it("R404 quest 6: your Units' total attack and total health both 10 or more at once", () => {
      const s = scenario({ p1: { backrow: [ITD], field: [MENACE], hand: [TIMMY, STOCKPILE], library: SPARE.library }, p2: { field: [MENACE], ...SPARE } });
      onQuest(s, "6");
      expect(shown(s, "p1").quest?.open[0]).toMatchObject({ id: "6", progress: 9, goal: 10 });
      s.play(TIMMY);
      expect(completedIn(s.lastEvents)).toEqual(["6"]);
      expect(rewardKeys(s)).toEqual(["mode:H", "mode:I"]);
    });

    it("R404 reward I: Recruit the first permanent from the top of your deck, then quest 10", () => {
      const s = scenario({
        p1: { backrow: [ITD], field: [MENACE, TIMMY], hand: [NOTEBOOK, TIMMY], library: [STOCKPILE, STOCKPILE, VANILLA, MENACE] },
        p2: SPARE,
      });
      onQuest(s, "6");
      s.play(NOTEBOOK);
      s.answer("I");
      // The Notebook drew the top Stockpile; the Recruit passes the next one over for the Vanilla.
      expect(must(s.unit("p1", 3), "the recruit").defId).toBe(VANILLA);
      expect(s.pile("p1", "library").map((c) => c.defId)).toEqual([STOCKPILE, MENACE]);
      expect(line(s)).toMatchObject({ active: ["10"], done: ["6"] });
    });

    it("R404 quest 7: a turn of yours ending with 5 or more unspent mana; 4 is not enough", () => {
      const s = scenario({ p1: { backrow: [ITD], ...SPARE, mana: 5 }, p2: SPARE });
      onQuest(s, "7");
      s.endTurn();
      expect(completedIn(s.lastEvents)).toEqual(["7"]);
      expect(rewardKeys(s)).toEqual(["mode:J"]);

      const short = scenario({ p1: { backrow: [ITD], ...SPARE, mana: 4 }, p2: SPARE });
      onQuest(short, "7");
      short.endTurn();
      expect(short.state.pending).toBeNull();
    });

    it("R540 reward J: 100 mana on your next turn (next-turn mana), and the line ends there", () => {
      const s = scenario({ p1: { backrow: [ITD], ...SPARE, mana: 5 }, p2: SPARE });
      onQuest(s, "7");
      s.endTurn();
      s.answer("J");
      expect(line(s)).toMatchObject({ active: [], done: ["7"] });
      expect(s.state.active).toBe("p2");
      expect(s.state.players.p1.mana.nextTurnMod).toBe(100);
      s.endTurn();
      expect(s.state.active).toBe("p1");
      const mana = s.state.players.p1.mana;
      expect(mana.current).toBe(mana.max + 100);
    });

    it("R404 quest 8: three cards entering either exile pile; a unit token ceases to exist instead (R11)", () => {
      const s = scenario({
        p1: { backrow: [ITD], hand: [TRUE_STRIKE, COLLATERAL, STOCKPILE], library: SPARE.library, mana: 10 },
        p2: { field: [VANILLA, RUSH_TOKEN], ...SPARE },
      });
      onQuest(s, "8");
      // True Strike exiles itself as it resolves: 1.
      s.play(TRUE_STRIKE, { targets: [{ pick: "instance", instanceId: must(s.unit("p2", 2), "the token").id }] });
      expect(line(s)?.progress).toEqual({ "8": 1 });
      // Collateral Damage exiles the Vanilla and a card of p2's deck: 3.
      s.play(COLLATERAL, { targets: [{ pick: "instance", instanceId: must(s.unit("p2", 1), "Vanilla").id }] });
      expect(completedIn(s.lastEvents)).toEqual(["8"]);
      expect(rewardKeys(s)).toEqual(["mode:K"]);
    });

    it("R404 quest 8 does not count a unit token exiled from the field", () => {
      const s = scenario({ p1: { backrow: [ITD], hand: [COLLATERAL, STOCKPILE], library: SPARE.library, mana: 10 }, p2: { field: [RUSH_TOKEN], ...SPARE } });
      onQuest(s, "8");
      s.play(COLLATERAL, { targets: [{ pick: "instance", instanceId: must(s.unit("p2", 1), "the token").id }] });
      // The token ceased to exist; the deck card did enter exile.
      expect(line(s)?.progress).toEqual({ "8": 1 });
    });

    it("R404 reward K: your opponent's deck, every card of it, to their exile", () => {
      const s = scenario({ p1: { backrow: [ITD], ...SPARE }, p2: { hand: [STOCKPILE], library: [VANILLA, TIMMY, MENACE] } });
      onQuest(s, "8", 3);
      anyAction(s);
      s.answer("K");
      expect(s.pile("p2", "library")).toEqual([]);
      expect(s.pile("p2", "exile").map((c) => c.defId).sort()).toEqual([MENACE, TIMMY, VANILLA].sort());
      expect(line(s)).toMatchObject({ active: [], done: ["8"] });
    });

    it("R404 quest 9: only the draw of yours that takes your deck's last card; a deck emptied any other way does not", () => {
      const s = scenario({ p1: { backrow: [ITD], hand: [NOTEBOOK, NOTEBOOK, TIMMY], library: [VANILLA, VANILLA] }, p2: SPARE });
      onQuest(s, "9");
      s.play(NOTEBOOK);
      expect(line(s)?.progress).toEqual({ "9": 0 });
      s.play(NOTEBOOK);
      expect(completedIn(s.lastEvents)).toEqual(["9"]);
      expect(rewardKeys(s)).toEqual(["mode:L"]);

      const milled = scenario({ p1: { backrow: [ITD], hand: [EUGENICS, NOTEBOOK, TIMMY], library: [VANILLA, VANILLA] }, p2: SPARE });
      onQuest(milled, "9");
      milled.play(EUGENICS);
      expect(milled.pile("p1", "library")).toEqual([]);
      expect(line(milled)?.progress).toEqual({ "9": 0 });
      // A fatigue draw takes no card either.
      milled.play(NOTEBOOK);
      expect(line(milled)?.done).toEqual([]);
    });

    it("R454 reward L: you may play cards from your graveyard while it stands; an exile ends the aura and the line (R78)", () => {
      const s = scenario({
        p1: { backrow: [ITD], graveyard: [TIMMY], ...SPARE },
        p2: { hand: [COLLATERAL, STOCKPILE], library: SPARE.library, mana: 10 },
      });
      onQuest(s, "9", 1);
      anyAction(s);
      s.answer("L");
      expect(line(s)).toMatchObject({ auras: ["L"], active: [], done: ["9"] });
      expect(shown(s, "p2").quest?.auras).toEqual([{ id: "L", text: "Aura: you may play cards from your graveyard" }]);
      const timmy = must(s.pile("p1", "graveyard").find((c) => c.defId === TIMMY), "Timmy in the graveyard");
      expect(legalActions(s.state, "p1").some((a) => a.type === "play" && a.instanceId === timmy.id)).toBe(true);

      s.endTurn();
      s.play(COLLATERAL, { targets: [{ pick: "instance", instanceId: s.card(ITD).id }] });
      s.expectInZone(ITD, "exile");
      expect(line(s)).toBeNull();
      s.endTurn();
      expect(legalActions(s.state, "p1").some((a) => a.type === "play" && a.instanceId === timmy.id)).toBe(false);
    });

    it("R347 reward M: your Units have Indestructible, and so no Taunt, while it stands; an exile ends it", () => {
      const s = scenario({
        p1: { backrow: [ITD], field: [MENACE], ...SPARE },
        p2: { hand: [COLLATERAL, STOCKPILE], library: SPARE.library, mana: 10 },
      });
      const menace = must(s.unit("p1", 1), "Menace");
      expect(hasKeyword(s.stats(menace).keywords, "Taunt")).toBe(true);
      // The line as reward M leaves it (the reward itself: quest 10's test).
      setLine(s, { active: [], done: ["10"], auras: ["M"] });
      expect(hasKeyword(s.stats(menace).keywords, "Indestructible")).toBe(true);
      expect(hasKeyword(s.stats(menace).keywords, "Taunt")).toBe(false);
      s.endTurn();
      s.play(COLLATERAL, { targets: [{ pick: "instance", instanceId: s.card(ITD).id }] });
      expect(hasKeyword(s.stats(menace).keywords, "Indestructible")).toBe(false);
      expect(hasKeyword(s.stats(menace).keywords, "Taunt")).toBe(true);
    });

    it("R102 a Lockdown fused onto it (Unlicensed Experimentation) leaves its quest line and its tree: quest 2 counts on and asks", () => {
      const s = scenario({
        active: "p2",
        p1: { backrow: [ITD, { def: EXPERIMENT, faceUp: false }], ...SPARE },
        p2: { field: [VANILLA], hand: [LOCKDOWN, HIT_JOB, STOCKPILE], library: SPARE.library, mana: 10 },
      });
      onQuest(s, "2", 1);
      const itd = s.card(ITD);
      s.play(LOCKDOWN);
      expect(s.lastEvents.some((e) => e.type === "fused")).toBe(true);
      expect(s.card(itd.id).defId).not.toBe(ITD);
      expect(subsystems.questMemoryOf(s.card(itd.id))).toMatchObject({ active: ["2"], progress: { "2": 1 } });
      s.play(HIT_JOB, { targets: [{ pick: "instance", instanceId: must(s.unit("p2", 1), "Vanilla").id }] });
      expect(completedIn(s.lastEvents)).toEqual(["2"]);
      expect(rewardKeys(s, "p1")).toEqual(["mode:C", "mode:D"]);
    });

    it("R78 a Tribute ends aura M: fused with a Lockdown, its Activate Tributes the card, and your Units lose Indestructible", () => {
      const s = scenario({
        active: "p2",
        p1: { backrow: [ITD, { def: EXPERIMENT, faceUp: false }], field: [MENACE], ...SPARE },
        p2: { hand: [LOCKDOWN, STOCKPILE], library: SPARE.library, mana: 10 },
      });
      setLine(s, { active: [], done: ["10"], auras: ["M"] });
      const itd = s.card(ITD);
      const menace = must(s.unit("p1", 1), "Menace");
      s.play(LOCKDOWN);
      expect(s.card(itd.id).defId).not.toBe(ITD);
      expect(hasKeyword(s.stats(menace).keywords, "Indestructible")).toBe(true);
      s.endTurn();
      s.activate(itd.id, { ability: "tribute" });
      s.expectInZone(itd.id, "graveyard");
      expect(hasKeyword(s.stats(menace).keywords, "Indestructible")).toBe(false);
    });

    it("R404 quest 10: six Units in your graveyard (a Spell there is none), then reward M", () => {
      const s = scenario({
        p1: { backrow: [ITD], field: [TIMMY], graveyard: [VANILLA, VANILLA, VANILLA, VANILLA, VANILLA, STOCKPILE], hand: [HIT_JOB, STOCKPILE], library: SPARE.library },
        p2: SPARE,
      });
      onQuest(s, "10");
      expect(shown(s, "p1").quest?.open[0]).toMatchObject({ id: "10", progress: 5, goal: 6 });
      s.play(HIT_JOB, { targets: [{ pick: "instance", instanceId: must(s.unit("p1", 1), "Timmy").id }] });
      expect(completedIn(s.lastEvents)).toEqual(["10"]);
      expect(rewardKeys(s)).toEqual(["mode:M"]);
      s.answer("M");
      expect(line(s)).toMatchObject({ active: [], auras: ["M"], done: ["10"] });
    });

    it("R79 a quest completed on the opponent's turn is noticed then, and the reward prompt is yours", () => {
      const s = scenario({ active: "p2", p1: { backrow: [ITD], ...SPARE }, p2: { field: [VANILLA], hand: [HIT_JOB, STOCKPILE], library: SPARE.library } });
      onQuest(s, "2", 1);
      // p2 destroys its own unit: an enemy permanent of p1's, destroyed by anything.
      s.play(HIT_JOB, { targets: [{ pick: "instance", instanceId: must(s.unit("p2", 1), "Vanilla").id }] });
      expect(s.state.active).toBe("p2");
      expect(rewardKeys(s, "p1")).toEqual(["mode:C", "mode:D"]);
      expect(s.view("p2").pending).toEqual({ forYou: false, pendingFor: "p1" });
      // The reports name the public card in the other player's view too.
      const reports = s.view("p2").events.filter((e) => e.type === "questCompleted");
      expect(reports.map((e) => (e.type === "questCompleted" ? e.instanceId : ""))).toEqual([s.card(ITD).id]);
    });

    it("R113 a reward paused on its own question survives a JSON round trip and finishes the same", () => {
      const s = scenario({ p1: { backrow: [ITD], ...SPARE }, p2: { field: [VANILLA], ...SPARE } });
      s.play(STOCKPILE);
      s.answer("B");
      const paused = s.state;
      const revived = JSON.parse(JSON.stringify(paused)) as GameState;
      expect(revived).toEqual(paused);
      const foe = must(s.unit("p2", 1), "Vanilla");
      const choiceId = must(revived.pending, "the target prompt").id;
      const result = reduce(revived, {
        type: "answer",
        playerId: "p1",
        choiceId,
        selection: [{ pick: "instance", instanceId: foe.id }],
        nonce: "itd-round-trip",
      });
      expect(result.error).toBeUndefined();
      s.answer(foe.id);
      expect(hashState(result.state)).toBe(hashState(s.state));
      expect(subsystems.questMemoryOf(must(result.state.players.p1.backrow[0], "In Too Deep"))?.active).toEqual(["3"]);
    });

    it("§9.2 a game through quest 1 and its reward folds from its log to the same state and views", () => {
      const deck = [ITD, "core-002", "core-005", "core-006", "core-008", "core-011", "core-012", "core-013", "core-015", "core-016",
        "core-019", "core-020", "core-025", "core-026", "core-032", "core-036", "core-043", "core-044", "core-053", "core-055"];
      const other = deck.map((id) => (id === ITD ? "core-003" : id));
      const seed = "itd-replay";
      const log: Action[] = [];
      let n = 0;
      const act = (state: GameState, body: ActionInput): GameState => {
        n += 1;
        const action = { ...body, nonce: `itd-${n}` } as Action;
        const result = reduce(state, action);
        if (result.error !== undefined) throw new Error(result.error);
        log.push(action);
        return result.state;
      };
      let state = beginGame(createGame({ seed, decks: [deck, other] })).state;
      for (const player of ["p1", "p2"] as PlayerId[]) {
        state = act(state, { type: "mulligan", keep: state.players[player].hand.map((card) => card.id), playerId: player });
      }
      // Quickdraw put it in p1's opening hand.
      const card = must(state.players.p1.hand.find((c) => c.defId === ITD), "In Too Deep in the opening hand");
      state = act(state, { type: "play", instanceId: card.id, playerId: "p1" } as ActionInput);
      // Two of p1's turn draws complete quest 1.
      for (let turn = 0; turn < 4 && state.pending === null; turn += 1) {
        state = act(state, { type: "endTurn", playerId: state.active });
      }
      const pending = must(state.pending, "the reward prompt");
      expect(pending.kind).toBe("reward");
      state = act(state, { type: "answer", playerId: "p1", choiceId: pending.id, selection: [{ pick: "mode", option: "A" }] });
      expect(subsystems.questMemoryOf(must(state.players.p1.backrow.find((c) => c?.defId === ITD), "ITD"))?.active).toEqual(["2"]);

      const replayed = fold({ seed, decks: [deck, other], log });
      expect(replayed.errors).toEqual([]);
      expect(hashState(replayed.state)).toBe(hashState(state));
      for (const viewer of ["p1", "p2"] as PlayerId[]) expect(viewFor(replayed.state, viewer)).toEqual(viewFor(state, viewer));
    });
  });

  describe("radiant", () => {
    const RADIANT_ITD = { def: ITD, radiant: true };

    it("R543 quest 1 grants A and B with no reward prompt — B still asks its target — and only then opens quests 2 and 3", () => {
      const s = scenario({ p1: { backrow: [RADIANT_ITD], health: 20, ...SPARE }, p2: { field: [VANILLA], ...SPARE } });
      s.play(STOCKPILE);
      expect(s.events.some((e) => e.type === "promptOpened" && e.kind === "reward")).toBe(false);
      // A has healed (Stockpile 2 + A 6); B asks.
      s.expectHealth("p1", 28);
      expect(pendingKind(s)).toBe("target");
      expect(line(s)?.active).toEqual([]);
      s.answer(must(s.unit("p2", 1), "Vanilla").id);
      expect(line(s)).toMatchObject({ active: ["2", "3"], done: ["1"] });
      expect(shown(s, "p2").quest?.open.map((q) => q.id)).toEqual(["2", "3"]);
    });

    it("R404 a quest reached by two paths opens once, and a reward two completed quests offer (D) is granted by each", () => {
      const s = scenario({
        // Small Units, so the +3/+3 does not complete quest 6 as well.
        p1: { backrow: [RADIANT_ITD], field: [TIMMY, TOKEN_MAKER], graveyard: [MENACE, STOCKPILE], ...SPARE },
        p2: { field: [MENACE], ...SPARE },
      });
      // Quests 2 and 3 open, as quest 1's Radiant rewards leave them; quest 2 at its goal, and quest 3
      // met by the board (In Too Deep and two Units).
      setLine(s, { active: ["2", "3"], progress: { "2": 2 }, done: ["1"] });
      anyAction(s);
      // Quest 2: C (two graveyard cards), D (three placements on one pick); quest 3: D again, E.
      const foe = must(s.unit("p2", 1), "Menace");
      let placements = 0;
      while (s.state.pending !== null) {
        expect(pendingKind(s)).toBe("target");
        s.answer(foe.id);
        placements += 1;
      }
      // One prompt per D grant (R669): two answers, six counters.
      expect(placements).toBe(2);
      expect(s.card(foe).counters.plague).toBe(6);
      expect(s.events.filter((e) => e.type === "buffed")).toHaveLength(1);
      const memory = must(line(s), "the line");
      expect(memory.done).toEqual(["1", "2", "3"]);
      // 4 and 5 from quest 2's rewards, 6 from quest 3's; 5 opened once.
      expect(memory.active).toEqual(["4", "5", "6"]);
    });

    it("R404 quest 5 grants G — the opponent's own discard — and H, then opens quests 8 and 9", () => {
      const s = scenario({ p1: { backrow: [RADIANT_ITD], ...SPARE }, p2: { hand: [VANILLA, TIMMY, MENACE], library: SPARE.library } });
      onQuest(s, "5", 12, { done: ["1", "2"] });
      anyAction(s);
      const pending = must(s.state.pending, "G's discard");
      expect([pending.kind, pending.playerId]).toEqual(["hand", "p2"]);
      const [a, b] = s.hand("p2");
      s.answer([must(a, "a").id, must(b, "b").id]);
      expect(s.hand("p2")).toHaveLength(1);
      expect(line(s)).toMatchObject({ active: ["8", "9"], done: ["1", "2", "5"], progress: { "9": 0 } });
    });

    it("R404 quests 7 to 10 end their lines: quest 7 grants J and opens nothing", () => {
      const s = scenario({ p1: { backrow: [RADIANT_ITD], ...SPARE, mana: 5 }, p2: SPARE });
      onQuest(s, "7");
      s.endTurn();
      expect(s.events.some((e) => e.type === "promptOpened" && e.kind === "reward")).toBe(false);
      expect(s.state.players.p1.mana.nextTurnMod).toBe(100);
      expect(line(s)).toMatchObject({ active: [], done: ["7"] });
    });
  });
});
