// Classic #90 In Too Deep's quest line on the board and in the inspect overlays (B5 E33, R404, SPEC
// §10.8): the view carries `quest` on every view of the card, and the client draws a badge, inspect
// notes and log lines; the reward prompt takes the reward the engine offers. The fixtures follow
// `QuestView` (`crates/engine/src/wire/view.rs`); the last block plays the real engine, so a renamed
// field or a reworded event breaks it. R2450000: a Quest card's face prints only the stage it stands
// at, and In Too Deep's its open quests (questFace.ts).

import { readFileSync } from "node:fs";
import { dirname, join } from "node:path";
import { fileURLToPath } from "node:url";

import { CATALOG } from "@jackioh/cards";
import { DECK_SIZE } from "@jackioh/engine/config";
import type { ActionBody, BackrowView, CardView, PlayerId, PlayerView, QuestView } from "@jackioh/shared";
import { act, cleanup, fireEvent, render, screen, within } from "@testing-library/react";
import { afterEach, describe, expect, it, vi } from "vitest";

import { answerPrompts, devDeck, playOf, realGame, type RealGame } from "../audio/test/realGame.ts";
import Board from "../game/Board.tsx";
import { lookupFromDefs } from "../game/catalog.ts";
import { testid } from "../game/contract.ts";
import { listedFace } from "../game/faces.ts";
import Log from "../game/Log.tsx";
import Prompt from "../game/Prompt.tsx";
import { resetLocaleForTests, setLocale } from "../i18n/i18n.ts";
import { withCatalog } from "../test/board.tsx";
import { baseView, emptySide, faceUpBackrow } from "../test/fixtures.ts";
import { must } from "../test/stubServer.ts";
import { CardFace, faceText } from "./CardFace.tsx";
import { questProgress, questWords, stageWords } from "./cardState.ts";
import { HOVER_DELAY_MS } from "./inspect/constants.ts";
import { closeInspect } from "./inspect/store.ts";
import { INSPECT_EXTENDED, INSPECT_FACE, INSPECT_HOVER, INSPECT_SHEET, INSPECT_STATES } from "./inspect/testids.ts";
import { faceModel } from "./model.ts";
import { questFaceText } from "./questFace.ts";

afterEach(() => {
  act(() => {
    closeInspect();
  });
  cleanup();
  vi.useRealTimers();
});

const ITD = "classic-090";

const FIRST: QuestView = {
  open: [
    {
      id: "1",
      text: "Draw 2 cards",
      progress: 1,
      goal: 2,
      rewards: [
        { id: "A", text: "Heal your hero 6" },
        { id: "B", text: "Deal 3 damage to a target" },
      ],
    },
  ],
  auras: [],
};

/** A Radiant line further in: two quests open at once, and an aura held. */
const DEEPER: QuestView = {
  open: [
    {
      id: "2",
      text: "Destroy 2 enemy permanents",
      progress: 0,
      goal: 2,
      rewards: [
        { id: "C", text: "Return 2 random cards from your graveyard to your hand" },
        { id: "D", text: "Place 3 Plague Counters" },
      ],
    },
    {
      id: "5",
      text: "Your cards deal 12 damage to enemies",
      progress: 12,
      goal: 12,
      rewards: [
        { id: "G", text: "Your opponent discards 2 cards of their choice" },
        { id: "H", text: "Draw 2 cards" },
      ],
    },
  ],
  auras: [{ id: "M", text: "Aura: your Units have Indestructible" }],
};

function questCard(quest: QuestView | undefined, over: Partial<Extract<BackrowView, { faceDown: false }>> = {}): BackrowView {
  return faceUpBackrow("p1", { instanceId: "itd", defId: ITD, type: "Field Spell", ...(quest === undefined ? {} : { quest }), ...over });
}

function boardWith(entry: BackrowView, side: "you" | "opponent" = "you"): PlayerView {
  const backrow = [entry, null, null, null, null];
  return side === "you"
    ? baseView({ you: emptySide("p1", { backrow }) })
    : baseView({ opponent: emptySide("p2", { backrow, hand: { count: 4 } }) });
}

function badges(root: HTMLElement): HTMLElement[] {
  return Array.from(root.querySelectorAll<HTMLElement>('.cf-state[data-state="quest"]'));
}

function hover(element: HTMLElement): HTMLElement {
  fireEvent.pointerEnter(element, { pointerType: "mouse" });
  act(() => {
    vi.advanceTimersByTime(HOVER_DELAY_MS * 2);
  });
  return screen.getByTestId(INSPECT_HOVER);
}

/** The I key on `element` opens its inspect sheet (a touch hold shows the preview, R2617000), and the sheet. */
function sheetOf(element: HTMLElement): HTMLElement {
  fireEvent.keyDown(element, { key: "i" });
  return screen.getByTestId(INSPECT_SHEET);
}

function questLines(overlay: HTMLElement): { id: string | null; words: string; rewards: string[] }[] {
  return Array.from(within(overlay).getByTestId(INSPECT_STATES).querySelectorAll<HTMLElement>('li[data-state="quest"]')).map((line) => ({
    id: line.getAttribute("data-quest"),
    words: line.textContent ?? "",
    rewards: Array.from(line.querySelectorAll("li[data-reward]")).map((reward) => reward.textContent ?? ""),
  }));
}

describe("R404 In Too Deep's quest line on the board", () => {
  it("R404 the words: \"1/2\" and \"Quest: Draw 2 cards (1/2)\"", () => {
    const quest = must(FIRST.open[0], "quest 1");
    expect(questProgress(quest)).toBe("1/2");
    expect(questWords(quest)).toBe("Quest: Draw 2 cards (1/2)");
  });

  it("R404 the face-up Field Spell wears a progress badge per open quest, named by the quest's text", () => {
    render(withCatalog(<Board view={boardWith(questCard(FIRST))} />));
    const root = screen.getByTestId(testid.card("itd"));
    expect(root.querySelector(".cf")?.getAttribute("data-layout")).toBe("compact");
    const [badge] = badges(root);
    const one = must(badge, "the quest badge");
    expect(one.textContent).toBe("1/2");
    expect(one.getAttribute("role")).toBe("img");
    expect(one.getAttribute("aria-label")).toBe("Quest: Draw 2 cards (1/2)");
    expect(one.getAttribute("title")).toBe("Quest: Draw 2 cards (1/2)");
    expect(one.getAttribute("data-quest")).toBe("1");
    expect(one.getAttribute("data-progress")).toBe("1");
    expect(one.getAttribute("data-goal")).toBe("2");
  });

  it("R404 the opponent's In Too Deep wears it too, and a Radiant line wears one badge per open quest", () => {
    render(withCatalog(<Board view={boardWith(questCard(DEEPER, { radiant: true }), "opponent")} />));
    const shown = badges(screen.getByTestId(testid.card("itd")));
    expect(shown.map((badge) => badge.textContent)).toEqual(["0/2", "12/12"]);
    expect(shown.map((badge) => badge.getAttribute("aria-label"))).toEqual([
      "Quest: Destroy 2 enemy permanents (0/2)",
      "Quest: Your cards deal 12 damage to enemies (12/12)",
    ]);
  });

  it("R404 a card with no quest line, or a line with no quest open, wears no quest badge", () => {
    render(
      withCatalog(
        <Board
          view={baseView({
            you: emptySide("p1", {
              backrow: [questCard(undefined), questCard({ open: [], auras: DEEPER.auras }, { instanceId: "done" }), null, null, null],
            }),
          })}
        />,
      ),
    );
    expect(badges(screen.getByTestId(testid.card("itd")))).toEqual([]);
    expect(badges(screen.getByTestId(testid.card("done")))).toEqual([]);
  });

  it("R404 the badge never moves: its rules in cardstate.css carry no animation or transition", () => {
    const css = readFileSync(join(dirname(fileURLToPath(import.meta.url)), "cardstate.css"), "utf8");
    const rules = css.split("}").filter((rule) => rule.includes('[data-state="quest"]'));
    expect(rules.length).toBeGreaterThan(0);
    for (const rule of rules) expect(rule).not.toMatch(/animation|transition/);
  });
});

describe("R404 In Too Deep's quest line in the inspect overlays", () => {
  it("R404 the hover preview lists each open quest with its progress and the rewards it offers, then each aura", () => {
    vi.useFakeTimers();
    render(withCatalog(<Board view={boardWith(questCard(DEEPER, { radiant: true }))} />));
    const preview = hover(screen.getByTestId(testid.card("itd")));
    const states = within(preview).getByTestId(INSPECT_STATES);
    const quests = Array.from(states.querySelectorAll<HTMLElement>('li[data-state="quest"]'));
    expect(quests.map((line) => line.getAttribute("data-quest"))).toEqual(["2", "5"]);
    expect(quests[0]?.textContent).toContain("Quest: Destroy 2 enemy permanents (0/2)");
    expect(quests[1]?.textContent).toContain("Quest: Your cards deal 12 damage to enemies (12/12)");
    expect(Array.from(must(quests[0], "quest 2").querySelectorAll("li[data-reward]")).map((reward) => reward.textContent)).toEqual([
      "Return 2 random cards from your graveyard to your hand",
      "Place 3 Plague Counters",
    ]);
    const aura = must(states.querySelector<HTMLElement>('li[data-state="questAura"]'), "the aura line");
    expect(aura.getAttribute("data-aura")).toBe("M");
    expect(aura.querySelector(".inspect-state-words")?.textContent).toBe("Aura: your Units have Indestructible");
    // The texts go through RulesText: its glossary terms are bolded, and no brace is ever drawn.
    expect(Array.from(aura.querySelectorAll(".cf-term")).map((term) => term.textContent)).toContain("Indestructible");
    expect(states.textContent).not.toContain("{");
  });

  it("R404 the sheet lists the same, and a line holding only an aura still says so", () => {
    vi.useFakeTimers();
    render(withCatalog(<Board view={boardWith(questCard(FIRST))} />));
    const sheet = sheetOf(screen.getByTestId(testid.card("itd")));
    const lines = questLines(sheet);
    expect(lines.map((line) => line.id)).toEqual(["1"]);
    expect(lines[0]?.words).toContain("Quest: Draw 2 cards (1/2)");
    expect(lines[0]?.rewards).toEqual(["Heal your hero 6", "Deal 3 damage to a target"]);
    act(() => {
      closeInspect();
    });
    cleanup();

    render(withCatalog(<Board view={boardWith(questCard({ open: [], auras: DEEPER.auras }))} />));
    const preview = hover(screen.getByTestId(testid.card("itd")));
    expect(within(preview).getByTestId(INSPECT_STATES).textContent).toBe("✦Aura: your Units have Indestructible");
  });
});

describe("R404 the log reads a quest's reports in words", () => {
  function logLines(view: PlayerView): string[] {
    render(withCatalog(<Log view={view} />));
    return Array.from(screen.getByTestId(testid.log).querySelectorAll("li")).map((line) => line.textContent ?? "");
  }

  it("R404 a quest's progress names its card and its text, a completion its card, and each opens In Too Deep", () => {
    const view = baseView({
      you: emptySide("p1", { backrow: [questCard(FIRST), null, null, null, null] }),
      events: [
        { type: "questProgressed", player: "p1", instanceId: "itd", quest: "1", progress: 1, goal: 2 },
        { type: "questProgressed", player: "p1", instanceId: "itd", quest: "9", progress: 1, goal: 1 },
        { type: "questCompleted", player: "p1", instanceId: "itd", quest: "9" },
      ],
    });
    expect(logLines(view)).toEqual([
      "Your In Too Deep quest: Draw 2 cards, 1/2",
      "Your In Too Deep quest: 1/1",
      "Your In Too Deep completed a quest",
    ]);
    const opens = Array.from(screen.getByTestId(testid.log).querySelectorAll("[data-testid='log-card']"));
    expect(opens.map((button) => button.getAttribute("data-def-id"))).toEqual([ITD, ITD, ITD]);
  });

  it("R404 the opponent's quest reads as theirs", () => {
    const view = baseView({
      opponent: emptySide("p2", { backrow: [questCard(FIRST, { owner: "p2", controller: "p2" }), null, null, null, null], hand: { count: 4 } }),
      events: [{ type: "questProgressed", player: "p2", instanceId: "itd", quest: "1", progress: 1, goal: 2 }],
    });
    expect(logLines(view)).toEqual(["The opponent's In Too Deep quest: Draw 2 cards, 1/2"]);
  });
});

// The real engine

/** The step budget for one of the game's waits: a few turns of End turn and first answers. */
const STEPS_MAX = 40;

function questCardIn(view: PlayerView, player: PlayerId): Extract<BackrowView, { faceDown: false }> | null {
  const side = view.you.player === player ? view.you : view.opponent;
  for (const entry of side.backrow) {
    if (entry !== null && !entry.faceDown && entry.defId === ITD) return entry;
  }
  return null;
}

/** Ends turns and answers every prompt but p1's reward, until `done` holds. */
function until(game: RealGame, done: () => boolean): void {
  for (let step = 0; step < STEPS_MAX && !done(); step += 1) {
    const who = must(game.actor(), "a seat to act");
    const legal = game.legal(who);
    game.act(who, legal.find((action) => action.type === "answer") ?? { type: "endTurn" });
  }
  expect(done()).toBe(true);
}

function progressOf(game: RealGame): number | undefined {
  return questCardIn(game.view("p1"), "p1")?.quest?.open[0]?.progress;
}

describe("R404 In Too Deep through the real engine", () => {
  it("R404 R2450000 its view's quest line becomes the badge, the face, the notes, the log and the reward picker, and the picked reward opens the next quest", () => {
    const deck = [ITD, ...devDeck("cheap20").filter((id) => id !== ITD)].slice(0, DECK_SIZE);
    const game = realGame("quest-panel", [deck, devDeck("first20")]);
    const catalog = lookupFromDefs(game.catalog);
    // Quickdraw: it starts in p1's opening hand, and p1 plays it on turn 1.
    const hand = game.view("p1").you.hand as CardView[];
    const itd = must(hand.find((card) => card.defId === ITD), "In Too Deep in the opening hand");
    game.act("p1", must(playOf(game, "p1", itd.instanceId), "a legal play of In Too Deep"));
    answerPrompts(game);

    for (const viewer of ["p1", "p2"] as const) {
      const view = game.view(viewer);
      const entry = must(questCardIn(view, "p1"), `In Too Deep on ${viewer}'s board`);
      expect(entry.quest?.open.map((quest) => [quest.id, quest.text, quest.progress, quest.goal])).toEqual([["1", "Draw 2 cards", 0, 2]]);
      render(withCatalog(<Board view={view} />, catalog));
      const [badge] = badges(screen.getByTestId(testid.card(entry.instanceId)));
      expect(must(badge, "the quest badge").textContent).toBe("0/2");
      expect(badge?.getAttribute("aria-label")).toBe("Quest: Draw 2 cards (0/2)");
      // R2450000: the face prints the quest the view has open and the rewards it offers, on either seat.
      expect(questFaceText(must(listedFace(catalog, view, entry), "the face"))?.full).toContain(
        "\nQuest: Draw 2 cards (0/2). Rewards: Heal your hero 6; Deal 3 damage to a target.",
      );
      cleanup();
    }

    until(game, () => progressOf(game) === 1);
    const view = game.view("p1");
    render(withCatalog(<Board view={view} />, catalog));
    expect(badges(screen.getByTestId(testid.card(itd.instanceId))).map((badge) => badge.textContent)).toEqual(["1/2"]);
    cleanup();
    render(withCatalog(<Log view={view} />, catalog));
    expect(screen.getByTestId(testid.log).textContent).toContain("Your In Too Deep quest: Draw 2 cards, 1/2");
    cleanup();

    until(game, () => {
      const pending = game.view("p1").pending;
      return pending?.forYou === true && pending.kind === "reward";
    });
    const asked = game.view("p1");
    render(withCatalog(<Log view={asked} />, catalog));
    expect(screen.getByTestId(testid.log).textContent).toContain("Your In Too Deep completed a quest");
    cleanup();
    // The other seat is told p1 is choosing, and has no picker of its own.
    expect(game.view("p2").pending).toEqual({ forYou: false, pendingFor: "p1" });

    const onAction = vi.fn<(action: ActionBody) => void>();
    render(withCatalog(<Prompt view={asked} legal={game.legal("p1")} onAction={onAction} />, catalog));
    const modal = screen.getByTestId("prompt-modal");
    expect(modal.getAttribute("data-prompt-kind")).toBe("reward");
    expect(screen.getByTestId("prompt-quest-complete").textContent).toBe("Quest complete!");
    expect(modal.querySelector(".prompt-title")?.textContent).toBe("Draw 2 cards");
    const tiles = Array.from(modal.querySelectorAll<HTMLElement>("[data-testid^='prompt-option-']"));
    // #2646: each reward is a choice card, its words its text.
    expect(tiles.map((tile) => tile.querySelector(".choice-card-text")?.textContent)).toEqual(["Heal your hero 6", "Deal 3 damage to a target"]);
    fireEvent.click(must(tiles[0], "reward A's tile"));
    expect(onAction).toHaveBeenCalledTimes(1);
    const answer = must(onAction.mock.calls[0]?.[0], "the answer");
    expect(game.legal("p1")).toContainEqual(answer);
    cleanup();

    game.act("p1", answer);
    answerPrompts(game);
    const after = game.view("p2");
    const entry = must(questCardIn(after, "p1"), "In Too Deep after the reward");
    render(withCatalog(<Board view={after} />, catalog));
    expect(badges(screen.getByTestId(testid.card(entry.instanceId))).map((badge) => badge.getAttribute("aria-label"))).toEqual([
      "Quest: Destroy 2 enemy permanents (0/2)",
    ]);
    const face = must(questFaceText(must(listedFace(catalog, after, entry), "the face after the reward")), "the quest face");
    expect(face.full).toContain("\nQuest: Destroy 2 enemy permanents (0/2). Rewards: ");
    expect(face.full).not.toContain("Draw 2 cards (");
  });
});

/** R1854000: The Greatest Heist's shape at its second stage, as `QuestView.stages` carries it. */
const STAGED: QuestView = {
  open: [
    {
      id: "deaths",
      text: "Have 2 Acclaimed Units you control die",
      progress: 1,
      goal: 2,
      rewards: [{ id: "comradery", text: "Gain 1 mana" }],
    },
  ],
  auras: [],
  stages: [
    { name: "Initiation", state: "done", goals: ["Make an Acclaimed Unit Radiant"], reward: "Draw a card" },
    { name: "Comradery", state: "current", goals: ["Have 2 Acclaimed Units you control die"], reward: "Gain 1 mana" },
    { name: "Empty the Vaults", state: "upcoming", goals: ["Steal an enemy card"], reward: "Steal your enemy's mana crystals" },
  ],
};

/** R1854004: Job Application's shape while it waits to hear back, the Interview's three goals still to draw. */
const HEARING: QuestView = {
  open: [],
  auras: [],
  stages: [
    { name: "Upload Resume", state: "done", goals: ["Send a card from your hand in as your resume"], reward: "Hear back in 2-6 turns" },
    {
      name: "The Interview",
      state: "upcoming",
      goals: [
        "Play a card that costs the same as your resume",
        "Play 3 cards of different costs in a single turn",
        "Take or deal 12+ damage in a single instance",
      ],
      reward: "Get your resume back",
      hearBack: 2,
    },
    { name: "The Offer", state: "upcoming", goals: ["Activate to claim next turn"], reward: "A Paycheck every turn" },
  ],
};

function stageBadge(root: HTMLElement): HTMLElement | null {
  return root.querySelector<HTMLElement>('.cf-state[data-state="questStage"]');
}

describe("R1854000 a staged quest's stages, from the view alone", () => {
  it("R1854000 the card wears its stage, \"2/3\", beside its open quest, on either seat", () => {
    for (const side of ["you", "opponent"] as const) {
      render(withCatalog(<Board view={boardWith(questCard(STAGED), side)} />));
      const root = screen.getByTestId(testid.card("itd"));
      expect(badges(root).map((badge) => badge.textContent)).toEqual(["1/2"]);
      const stage = must(stageBadge(root), "the stage badge");
      expect(stage.textContent).toBe("2/3");
      expect(stage.getAttribute("aria-label")).toBe("Stage 2 of 3, Comradery");
      expect(stage.getAttribute("data-stage")).toBe("2");
      expect(stage.getAttribute("data-stage-state")).toBe("current");
      cleanup();
    }
  });

  it("R1854000 the inspect overlay lists every stage, done, open now and to come, with its goals and its reward", () => {
    vi.useFakeTimers();
    render(withCatalog(<Board view={boardWith(questCard(STAGED))} />));
    const preview = hover(screen.getByTestId(testid.card("itd")));
    const list = must(
      within(preview).getByTestId(INSPECT_STATES).querySelector<HTMLElement>('li[data-state="questStages"]'),
      "the stages",
    );
    expect(list.textContent).toContain("Stage 2 of 3, Comradery");
    const stages = Array.from(list.querySelectorAll<HTMLElement>("li[data-stage]"));
    expect(stages.map((stage) => stage.getAttribute("data-stage-state"))).toEqual(["done", "current", "upcoming"]);
    expect(stages[2]?.textContent).toContain("Empty the Vaults");
    expect(stages[2]?.textContent).toContain("Steal an enemy card");
    expect(stages[2]?.textContent).toContain("Steal your enemy's mana crystals");
    // The open quest still has its own line, as In Too Deep's does.
    expect(questLines(preview).map((line) => line.id)).toEqual(["deaths"]);
  });

  it("R1854000 a line with every stage done says the quest is complete, and a tree's line shows no stages", () => {
    const done: QuestView = { open: [], auras: [], stages: (STAGED.stages ?? []).map((stage) => ({ ...stage, state: "done" as const })) };
    expect(stageWords(done.stages ?? [])).toBe("Quest complete");
    render(withCatalog(<Board view={boardWith(questCard(done))} />));
    expect(must(stageBadge(screen.getByTestId(testid.card("itd"))), "the stage badge").textContent).toBe("3/3");
    cleanup();
    render(withCatalog(<Board view={boardWith(questCard(FIRST))} />));
    expect(stageBadge(screen.getByTestId(testid.card("itd")))).toBeNull();
  });

  it("R1854002 a quest that asks for a cost names it", () => {
    const quest = { id: "prove", text: "Play a card that costs the same as your resume", progress: 0, goal: 1, rewards: [], cost: 2 };
    expect(questWords(quest)).toBe("Quest: Play a card that costs the same as your resume, a card that costs (2) (0/1)");
  });
});

describe("R1854004 a staged quest waiting to hear back", () => {
  it("R1854004 the badge and the overlay say how many turns are left, the stage's goals all still to draw", () => {
    vi.useFakeTimers();
    render(withCatalog(<Board view={boardWith(questCard(HEARING))} />));
    const root = screen.getByTestId(testid.card("itd"));
    const stage = must(stageBadge(root), "the stage badge");
    expect(stage.textContent).toBe("2/3");
    expect(stage.getAttribute("aria-label")).toBe("Stage 2 of 3, The Interview, hearing back in 2 turns");
    expect(stage.getAttribute("data-hear-back")).toBe("2");
    const preview = hover(root);
    const interview = must(preview.querySelector<HTMLElement>('li[data-stage="2"]'), "the Interview");
    expect(interview.getAttribute("data-hear-back")).toBe("2");
    expect(interview.querySelectorAll(".inspect-quest-stage-goals li")).toHaveLength(3);
    const next: QuestView = { ...HEARING, stages: (HEARING.stages ?? []).map((one) => (one.hearBack === undefined ? one : { ...one, hearBack: 1 })) };
    expect(stageWords(next.stages ?? [])).toBe("Stage 2 of 3, The Interview, hearing back next turn");
  });
});

const HEIST = "stinkyscape-007";
const JOB = "stinkyscape-112";

/** Each face's lines that are no stage, which every face of it prints as they are. */
const HEIST_CRY = {
  base: "Cry: Discover an Acclaimed Unit or a Jelly Bean.",
  radiant: "Cry: Discover a Radiant Acclaimed Unit or a Radiant Jelly Bean.",
} as const;
const JOB_HEAD = { base: "Death: Resummon this at Stage 1.", radiant: "Lucky 1\nDeath: Resummon this at Stage 1." } as const;

const FACES = [
  { label: "base", radiant: false },
  { label: "radiant", radiant: true },
] as const;

/** R1854000: The Greatest Heist's first stage open, as its line stands once it enters the field. */
const HEIST_FIRST: QuestView = {
  open: [{ id: "radiant", text: "Make an Acclaimed Unit Radiant", progress: 0, goal: 1, rewards: [] }],
  auras: [],
  stages: [
    { name: "Initiation", state: "current", goals: ["Make an Acclaimed Unit Radiant"], reward: "Summon a random Acclaimed Unit" },
    { name: "Comradery", state: "upcoming", goals: ["Have 2 Acclaimed Units you control die"], reward: "Gain 1 mana" },
    { name: "Empty the Vaults", state: "upcoming", goals: ["Steal an enemy card"], reward: "Steal your enemy's mana crystals" },
  ],
};

/** R1854002: Job Application's Interview open once it has heard back, the goal it drew in play. */
const INTERVIEW: QuestView = {
  open: [{ id: "behavioral", text: "Play 3 cards of different costs in one turn", progress: 1, goal: 3, rewards: [] }],
  auras: [],
  stages: (HEARING.stages ?? []).map((stage, at) => {
    if (at === 0) return { ...stage, state: "done" as const };
    if (at === 1) return { name: stage.name, state: "current" as const, goals: ["Play 3 cards of different costs in one turn"], reward: stage.reward };
    return stage;
  }),
};

/** Job Application's last stage open, after the Interview's reward. */
const OFFER: QuestView = {
  open: [{ id: "offer", text: "Activate this to accept the offer, or to send in another application", progress: 0, goal: 1, rewards: [] }],
  auras: [],
  stages: (INTERVIEW.stages ?? []).map((stage, at) => (at < 2 ? { ...stage, state: "done" as const } : { ...stage, state: "current" as const })),
};

/** A staged line with every stage done. */
function done(quest: QuestView): QuestView {
  return { open: [], auras: [], stages: (quest.stages ?? []).map((stage) => ({ ...stage, state: "done" as const })) };
}

/** A catalog face, in play holding `quest` when one is given. */
function questFace(defId: string, radiant: boolean, quest?: QuestView) {
  return faceModel({ defId, def: must(CATALOG[defId], defId), radiant, ...(quest === undefined ? {} : { inPlay: { quest } }) });
}

/** What a Quest card's rules box prints (questFace.ts). */
function printed(defId: string, radiant: boolean, quest?: QuestView): string {
  return must(questFaceText(questFace(defId, radiant, quest)), defId).full;
}

describe("R2450000 a Quest card's face prints only the stage it stands at", () => {
  it("R2450000 off the field a staged Quest prints Stage 1 of 3, its other lines kept", () => {
    expect(printed(HEIST, false)).toBe(
      `${HEIST_CRY.base}\nStage 1 of 3, Initiation. Quest: Make an Acclaimed Unit Radiant. Reward: Summon a random Acclaimed Unit.`,
    );
    expect(printed(HEIST, true)).toBe(
      `${HEIST_CRY.radiant}\nStage 1 of 3, Initiation. Quest: Make an Acclaimed Unit Radiant. Reward: Summon a random Radiant Acclaimed Unit.`,
    );
    const upload = "Stage 1 of 3, Upload Resume. Quest: Activate this to send a card from your hand in as your resume. Reward: Hear back in 2 to 6 turns.";
    for (const face of FACES) {
      expect(printed(JOB, face.radiant), face.label).toBe(`${JOB_HEAD[face.label]}\n${upload}`);
      // A card in hand is in a game but off the field: its view carries no quest line.
      for (const defId of [HEIST, JOB]) {
        const inHand = must(questFaceText(faceModel({ defId, def: must(CATALOG[defId], defId), radiant: face.radiant, inPlay: {} })), defId);
        expect(inHand.full, `${defId} ${face.label} in hand`).toBe(printed(defId, face.radiant));
      }
    }
  });

  it("R2450000 on the field it prints the current stage and its progress, and moves on after a reward", () => {
    for (const face of FACES) {
      const cry = HEIST_CRY[face.label];
      const radiant = face.radiant ? "Radiant " : "";
      expect(printed(HEIST, face.radiant, HEIST_FIRST), face.label).toBe(
        `${cry}\nStage 1 of 3, Initiation. Quest: Make an Acclaimed Unit Radiant (0/1). Reward: Summon a random ${radiant}Acclaimed Unit.`,
      );
      const anomaly = face.radiant ? "a Radiant Anomaly Found!" : "an Anomaly Found!";
      expect(printed(HEIST, face.radiant, STAGED), face.label).toBe(
        `${cry}\nStage 2 of 3, Comradery. Quest: Have 2 Acclaimed Units you control die (1/2). Reward: Do this 3 times: at random, add ${anomaly} to your hand or gain a random ${radiant}Stinkyscape Resource.`,
      );
      const buffs = face.radiant ? 10 : 5;
      expect(printed(JOB, face.radiant, INTERVIEW), face.label).toBe(
        `${JOB_HEAD[face.label]}\nStage 2 of 3, The Interview. Quest: Play 3 cards of different costs in one turn (1/3). Reward: Add your resume to your hand and Buff it ${String(buffs)} times. It costs (0).`,
      );
      expect(printed(JOB, face.radiant, OFFER), face.label).toBe(
        `${JOB_HEAD[face.label]}\nStage 3 of 3, The Offer. Quest: Activate this to accept the offer, or to send in another application (0/1). Reward: From your next turn on, add a Paycheck to your hand at the start of each of your turns.`,
      );
    }
  });

  it("R2450000 waiting to hear back it says the turns left, and a finished line says Quest complete", () => {
    for (const face of FACES) {
      const buffs = face.radiant ? 10 : 5;
      const interview = `Quest: One of three, drawn at random: play a card that costs the same as your resume; play 3 cards of different costs in one turn; or take or deal 12 or more damage in one hit. Reward: Add your resume to your hand and Buff it ${String(buffs)} times. It costs (0).`;
      expect(printed(JOB, face.radiant, HEARING), face.label).toBe(
        `${JOB_HEAD[face.label]}\nStage 2 of 3, The Interview, hearing back in 2 turns. ${interview}`,
      );
      const next: QuestView = { ...HEARING, stages: (HEARING.stages ?? []).map((one) => (one.hearBack === undefined ? one : { ...one, hearBack: 1 })) };
      expect(printed(JOB, face.radiant, next), face.label).toBe(`${JOB_HEAD[face.label]}\nStage 2 of 3, The Interview, hearing back next turn. ${interview}`);
      expect(printed(HEIST, face.radiant, done(STAGED)), face.label).toBe(`${HEIST_CRY[face.label]}\nQuest complete`);
      expect(printed(JOB, face.radiant, done(HEARING)), face.label).toBe(`${JOB_HEAD[face.label]}\nQuest complete`);
    }
  });

  it("R2450000 In Too Deep keeps its text and adds its open quests, their rewards and its auras", () => {
    for (const face of FACES) {
      const full = questFace(ITD, face.radiant).text.full;
      expect(printed(ITD, face.radiant, FIRST), face.label).toBe(`${full}\nQuest: Draw 2 cards (1/2). Rewards: Heal your hero 6; Deal 3 damage to a target.`);
      expect(printed(ITD, face.radiant, DEEPER), face.label).toBe(
        `${full}\nQuest: Destroy 2 enemy permanents (0/2). Rewards: Return 2 random cards from your graveyard to your hand; Place 3 Plague Counters.` +
          "\nQuest: Your cards deal 12 damage to enemies (12/12). Rewards: Your opponent discards 2 cards of their choice; Draw 2 cards." +
          "\nAura: your Units have Indestructible",
      );
      expect(printed(ITD, face.radiant, { open: [], auras: DEEPER.auras }), face.label).toBe(`${full}\nQuest complete\nAura: your Units have Indestructible`);
      // Off the field its tree has no quest open, and it prints its text as it is.
      const off = questFace(ITD, face.radiant);
      expect(questFaceText(off), face.label).toBeNull();
      // R2655001: this card prints full art now; the test reads its words.
      expect(faceText(off, undefined, false).shown, face.label).toBe(full);
    }
  });

  it("R2450000 a Radiant face keeps its gold marks where they now stand", () => {
    const marked = (face: ReturnType<typeof questFace>): string[] => {
      const text = must(questFaceText(face), face.defId);
      return text.marks.map((mark) => text.full.slice(mark.start, mark.end));
    };
    expect(marked(questFace(HEIST, true))).toEqual(["a Radiant", "Radiant", "Radiant"]);
    expect(marked(questFace(HEIST, true, STAGED))).toEqual(["a Radiant", "Radiant", "a Radiant", "Radiant"]);
    expect(marked(questFace(JOB, true, INTERVIEW))).toEqual(["Lucky 1", "10"]);
    expect(marked(questFace(HEIST, false, STAGED))).toEqual([]);

    const { container } = render(withCatalog(<CardFace face={questFace(HEIST, true)} />));
    expect(Array.from(container.querySelectorAll(".cf-text-base .cf-mark")).map((mark) => mark.textContent)).toEqual(["a Radiant", "Radiant", "Radiant"]);
    const box = must(container.querySelector<HTMLElement>(".card-text"), "the rules box");
    expect(box.getAttribute("data-short")).toBe("true");
    expect(box.getAttribute("data-less")).toBe("true");
  });

  it("R2450000 a reader of Chinese reads the stage in the Chinese face's layout", async () => {
    try {
      await setLocale("zh-CN");
      const death = "亡语：重新召唤这张牌，回到第1阶段。";
      expect(printed(JOB, false)).toBe(`${death}\n第1/3阶段，上传简历。任务：激活这张牌，将你手牌中的一张牌作为简历投递出去。奖励：2到6个回合后收到回复。`);
      expect(printed(JOB, true, INTERVIEW)).toBe(
        `幸运1\n${death}\n第2/3阶段，面试。任务：Play 3 cards of different costs in one turn（1/3）。奖励：将你的简历置入你的手牌并将其强化10次。其法力值消耗为(0)。`,
      );
      expect(printed(JOB, false, HEARING)).toContain("\n第2/3阶段，面试，2个回合后收到回复。任务：从三个任务中随机抽取一个：");
      expect(printed(HEIST, false, done(STAGED))).toBe("战吼：发现一个赞誉单位或一张软糖豆牌。\n任务完成");
      expect(printed(ITD, false, FIRST)).toContain("\n任务：抽2张牌（1/2）。奖励：为你的英雄恢复6点生命值；对一个目标造成3点伤害。");
    } finally {
      resetLocaleForTests();
    }
  });

  it("R2450000 the hover preview's face prints the stage on either seat, its whole text and every stage beside it", () => {
    vi.useFakeTimers();
    for (const side of ["you", "opponent"] as const) {
      render(withCatalog(<Board view={boardWith(questCard(STAGED, { defId: HEIST }), side)} />));
      const preview = hover(screen.getByTestId(testid.card("itd")));
      const face = within(preview).getByTestId(INSPECT_FACE);
      expect(face.querySelector(".cf-text-base")?.textContent, side).toBe(printed(HEIST, false, STAGED));
      const whole = within(preview).getByTestId(INSPECT_EXTENDED).textContent;
      for (const stage of ["Quest: Stage 1, Initiation.", "Quest: Stage 2, Comradery.", "Quest: Stage 3, Empty the Vaults."]) {
        expect(whole, `${side}: ${stage}`).toContain(stage);
      }
      expect(within(preview).getByTestId(INSPECT_STATES).querySelectorAll("li[data-stage]"), side).toHaveLength(3);
      act(() => {
        closeInspect();
      });
      cleanup();
    }
  });
});
