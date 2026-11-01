// BUILD M5-T4, the assertion that matters: **every row of the animation table names an element the
// client actually renders.** `animations.test.ts` renders nothing, so a row can point at a name no
// component has ever rendered and stay green forever, as `modifierChanged` did: a player could not
// see that #77 Professor Curvature or #78 /fullsend was active on them (R169 put the list in the view).
// The proof is the whole client (`Game`, so the shell's banner, toast, overlay and modal are in the
// tree) rendered from one `PlayerView`, and for every `GameEvent["type"]` one sample event built
// against THAT view.

import { GAME_EVENT_TYPES, type GameEvent, type GameEventType, type PlayerView } from "@jackioh/shared";
import { cleanup, render, screen } from "@testing-library/react";
import { afterEach, describe, expect, it } from "vitest";

import { targetFor } from "./animations.ts";
import Game from "./Game.tsx";
import { fullBoardView, pendingFor } from "../test/fixtures.ts";
import { must } from "../test/stubServer.ts";

afterEach(cleanup);

/* The view: a full board, plus the three pieces of chrome that only appear on a condition. */

function handIds(view: PlayerView): string[] {
  const hand = view.you.hand;
  if (!Array.isArray(hand)) throw new Error("the viewer's own hand should be full cards (SPEC §10.8)");
  return hand.map((card) => card.instanceId);
}

function faceUpBackrowIds(view: PlayerView): string[] {
  return view.you.backrow.flatMap((slot) => (slot !== null && slot.faceDown === false ? [slot.instanceId] : []));
}

/**
 * The prompt modal, the draw toast (R269's `drawOffer`) and the result overlay each wait on something;
 * all three are in the view at once here, so one render proves every row, though the engine never
 * sends an offer with a result (R216).
 */
function chromeView(): PlayerView {
  return fullBoardView({
    pending: pendingFor("target", [{ key: "hero:p2", label: "Opponent hero", player: "p2" }]),
    // The toast belongs to the seat that must ANSWER, so the offer is the opponent's (Game.tsx).
    drawOffer: { by: "p2" },
    events: [{ type: "drawOffered", player: "p2" }],
    result: { winner: "p1", reason: "concede" },
  });
}

/* One sample per event type, every id taken from the view above. */

function samplesFor(view: PlayerView): { [K in GameEventType]: Extract<GameEvent, { type: K }> } {
  const unit = must(view.you.units[0], "your unit in lane 1").instanceId;
  const enemy = must(view.opponent.units[0], "the opponent's unit in lane 1").instanceId;
  const hand = must(handIds(view)[0], "a card in the viewer's hand");
  const trap = must(faceUpBackrowIds(view).at(-1), "a face-up trap in the viewer's backrow");
  const modifier = must(view.you.modifiers[0], "a modifier on the viewer's own seat").id;

  return {
    cardPlayed: { type: "cardPlayed", player: "p1", instanceId: hand, defId: "core-002", costPaid: 1 },
    cardResolved: { type: "cardResolved", player: "p1", instanceId: unit, defId: "core-004", permanent: true, costPaid: 1 },
    summoned: { type: "summoned", player: "p1", instanceId: unit, defId: "core-004", row: "units", lane: 2 },
    damage: { type: "damage", sourceId: enemy, targetId: "hero-p1", amount: 4, combat: true },
    healthLost: { type: "healthLost", player: "p1", amount: 3 },
    healed: { type: "healed", targetId: unit, amount: 2 },
    divineShieldLost: { type: "divineShieldLost", instanceId: enemy },
    destroyed: { type: "destroyed", instanceId: unit, defId: "core-004", owner: "p1", controller: "p1", attack: 2, maxHealth: 3, killerId: enemy },
    enteredGraveyard: { type: "enteredGraveyard", instanceId: unit, defId: "core-004", owner: "p1" },
    exiled: { type: "exiled", instanceId: unit, defId: "core-004", owner: "p1" },
    bounced: { type: "bounced", instanceId: unit, defId: "core-004", owner: "p1" },
    burned: { type: "burned", instanceId: "gone", defId: "core-041", owner: "p2" },
    fatigue: { type: "fatigue", player: "p1", count: 1, amount: 1 },
    libraryOverflow: { type: "libraryOverflow", player: "p2", instanceId: "gone", defId: "core-090-1", outcome: "notCreated" },
    discarded: { type: "discarded", instanceId: hand, defId: "core-002", owner: "p1" },
    drawn: { type: "drawn", player: "p1", instanceId: "gone", defId: "core-055" },
    addedToHand: { type: "addedToHand", player: "p2", instanceId: "gone", defId: "core-060" },
    shuffledIn: { type: "shuffledIn", player: "p1", instanceId: "gone", defId: "core-070", position: -1 },
    buffed: { type: "buffed", instanceId: unit, attack: 1, health: 1 },
    keywordGranted: { type: "keywordGranted", instanceId: unit, keyword: { kind: "Taunt" } },
    counterChanged: { type: "counterChanged", instanceId: unit, counter: "plague", value: 3 },
    costChanged: { type: "costChanged", instanceId: hand, cost: 0 },
    // The row this file exists for: the badge list beside the hero (R169, BUILD M5-T4).
    modifierChanged: { type: "modifierChanged", player: "p1", modifierId: modifier, added: true },
    radiantSet: { type: "radiantSet", instanceId: unit, defId: "core-004", zone: { z: "field", player: "p1", row: "units", lane: 1 } },
    deradianted: { type: "deradianted", instanceId: unit, defId: "core-004", zone: { z: "field", player: "p1", row: "units", lane: 1 } },
    transformed: { type: "transformed", instanceId: unit, fromDefId: "core-004", toDefId: "token-sheep", newInstanceId: "gone" },
    fused: { type: "fused", instanceIds: [unit, enemy], resultInstanceId: "gone", defId: "core-088" },
    positionSwitched: { type: "positionSwitched", instanceId: unit, position: "DEF" },
    controlChanged: { type: "controlChanged", instanceId: enemy, controller: "p1", row: "units", lane: 4 },
    rotated: { type: "rotated", direction: "left" },
    swapped: { type: "swapped", what: "health" },
    locked: { type: "locked", player: "p2", row: "backrow", lane: 1 },
    trapFired: { type: "trapFired", instanceId: trap, defId: "core-084", controller: "p1", row: "backrow", lane: 5 },
    attackDeclared: { type: "attackDeclared", attackerId: unit, targetId: enemy, forced: false },
    attackCancelled: { type: "attackCancelled", attackerId: unit, targetId: enemy, byInstanceId: trap },
    manaChanged: { type: "manaChanged", player: "p1", current: 2, max: 4 },
    manaSpent: { type: "manaSpent", player: "p1", amount: 2, for: "play" },
    turnStarted: { type: "turnStarted", player: "p1", turn: 3 },
    turnEnded: { type: "turnEnded", player: "p1", turn: 3, unspentMana: 2 },
    turnAutoEnded: { type: "turnAutoEnded", player: "p1", turn: 3 },
    promptOpened: { type: "promptOpened", player: "p1", choiceId: "ch1", kind: "target" },
    promptAnswered: { type: "promptAnswered", player: "p1", choiceId: "ch1" },
    drawOffered: { type: "drawOffered", player: "p2" },
    drawAnswered: { type: "drawAnswered", player: "p1", accept: false },
    emoted: { type: "emoted", player: "p2", emote: "greetings" },
    gameOver: { type: "gameOver", winner: "p1", reason: "concede" },
    // Patch v0.2.0 (docs/classic-sets.md B3, B5).
    cardAnnounced: { type: "cardAnnounced", player: "p1", instanceId: hand, defId: "core-002", cardType: "Unit", costPaid: 1, targets: [] },
    countered: { type: "countered", player: "p1", instanceId: hand, defId: "core-002", byInstanceId: trap, to: "graveyard" },
    stolen: { type: "stolen", instanceId: hand, defId: "core-002", from: "p2", to: "p1", zone: "hand" },
    unlocked: { type: "unlocked", player: "p2", row: "backrow", lane: 1 },
    activated: { type: "activated", player: "p1", instanceId: unit, defId: "core-004", ability: "activate" },
    animated: { type: "animated", player: "p1", instanceId: trap, defId: "core-084", backrowLane: 5, unitLane: 3 },
    deanimated: { type: "deanimated", player: "p1", instanceId: trap, defId: "core-084", unitLane: 3, backrowLane: 5 },
    crumbled: { type: "crumbled", instanceId: unit, defId: "core-004", owner: "p1", zone: "field" },
    degraded: { type: "degraded", instanceId: unit, defId: "core-004", change: { kind: "cost", delta: 1 } },
    upgraded: { type: "upgraded", instanceId: unit, defId: "core-004", change: { kind: "stats", attack: 2, health: 2 } },
    numberChanged: { type: "numberChanged", instanceId: unit, defId: "core-004", key: "attack", value: 3 },
    redirected: { type: "redirected", what: "attack", fromId: unit, toId: enemy, byInstanceId: trap },
    healthSet: { type: "healthSet", player: "p2", health: 13, sourceId: null },
    questProgressed: { type: "questProgressed", player: "p1", instanceId: trap, quest: "1", progress: 1, goal: 2 },
    questCompleted: { type: "questCompleted", player: "p1", instanceId: trap, quest: "1" },
    rolledBack: { type: "rolledBack", player: "p1", turnsAgo: 1, sides: ["p1"] },
    chaosRolled: { type: "chaosRolled", player: "p1", instanceId: "gone", defId: "core-095", effects: ["Summon 3 random Units"] },
    flickered: { type: "flickered", player: "p1", instanceId: unit, defId: "core-004", row: "units", lane: 1 },
    drawLimited: { type: "drawLimited", player: "p1" },
    turnCutShort: { type: "turnCutShort", player: "p1", byInstanceId: null },
    marked: { type: "marked", instanceId: enemy, mark: "steal", color: "purple", added: true },
    glitched: { type: "glitched", player: "p1", outcome: "swap" },
    fengShui: { type: "fengShui", instanceId: unit, sourceId: "gone", player: "p1", outcome: "positive" },
    translated: { type: "translated", instanceId: enemy },
    discardPrevented: { type: "discardPrevented", player: "p1", count: 1 },
    damageAbsorbed: { type: "damageAbsorbed", sourceId: unit, targetId: enemy, absorbed: 2, combat: true },
    jadeChanged: { type: "jadeChanged", player: "p1", value: 3 },
    secretChosen: { type: "secretChosen", player: "p1", secretId: "secret-1" },
    secretRevealed: { type: "secretRevealed", player: "p1", secretId: "secret-1", choice: "greed" },
    predicted: { type: "predicted", player: "p2", secretId: "secret-1", guess: "attack", outcome: "won" },
    scriptFailed: { type: "scriptFailed", player: "p1", instanceId: unit, defId: "custom-fixture", message: "cry failed on line 3: boom" },
    curveDrawn: { type: "curveDrawn", player: "p1", cells: [{ player: "p1", row: "backrow", lane: 1 }, { player: "p2", row: "units", lane: 3 }] },
    autopilotEngaged: { type: "autopilotEngaged", player: "p1", turn: 9 },
    awakened: { type: "awakened", player: "p1", instanceId: unit, defId: "core-008" },
    traded: { type: "traded", player: "p1" },
    // Issue #1840 (Credits, R1840005); its Resources are hand cards since #2250 (R2250000).
    marketCreditsChanged: { type: "marketCreditsChanged", player: "p1", value: 2 },
    // Issue #1853 (the Marketplace, R1853003).
    marketBought: { type: "marketBought", player: "p1", instanceId: "m1", defId: "core-008", credits: 40 },
    marketSold: { type: "marketSold", player: "p1", instanceId: "r9", defId: "fx-res-rare", credits: 2 },
    starshipFused: { type: "starshipFused", player: "p1", instanceId: unit, pieceId: "u2", defId: "core-008", pieces: 2 },
    starshipLaunched: { type: "starshipLaunched", player: "p1", instanceId: unit, defId: "core-008", name: "Ember of Departure", pieces: ["core-008"] },
    voteCast: { type: "voteCast", player: "p1", instanceId: unit, defId: "core-004", votes: 1, needed: 3 },
    votedOut: { type: "votedOut", player: "p1", ballots: [{ instanceId: unit, defId: "core-004" }] },
    fled: { type: "fled", player: "p1", opponent: "draw" },
    starscapeTravelled: { type: "starscapeTravelled", player: "p1", node: 3, reward: "heal" },
    held: { type: "held", player: "p2", instanceId: enemy, defId: "core-004", holderId: trap },
    // Issue #1855 (R1855000).
    boardReplaced: { type: "boardReplaced", player: "p1" },
    // Issue #2652 (R2652001).
    rolled: { type: "rolled", player: "p1", instanceId: unit, defId: "core-004", kind: "coin", sides: 2, rolls: [{ faces: [1], kept: 0 }] },
  };
}

/**
 * The only legitimate `null`s: §10.6's "the opponent's view shows only that a prompt is open", and the
 * toast that belongs to the seat which owes the answer. The third test swaps the player on every
 * sample, so a row that starts going dark joins this list by failing, not by being added to it.
 */
const SEAT_CONDITIONAL: readonly GameEventType[] = ["promptOpened", "promptAnswered", "drawOffered"];

/** The same event aimed at the other seat, for the rows whose target depends on whose it is. */
function forOtherPlayer<E extends GameEvent>(event: E): E {
  return "player" in event ? { ...event, player: event.player === "p1" ? "p2" : "p1" } : event;
}

function renderClient(view: PlayerView): void {
  render(<Game view={view} legal={[]} onAction={() => undefined} />);
}

function rendered(testId: string): Element | null {
  return screen.queryByTestId(testId);
}

describe("every animation target is an element the client renders (BUILD M5-T4, R169)", () => {
  it("resolves every event type to a data-testid that is in the document", () => {
    const view = chromeView();
    const samples = samplesFor(view);
    renderClient(view);

    const missing: string[] = [];
    for (const type of GAME_EVENT_TYPES) {
      const target = targetFor(samples[type], view);
      if (target === null) {
        missing.push(`${type}: the table resolved no element at all`);
        continue;
      }
      if (rendered(target) === null) missing.push(`${type} → [data-testid="${target}"] is not rendered`);
    }

    expect(missing, "BUILD M5-T4: every row animates an element some component draws").toEqual([]);
  });

  it("modifierChanged animates the badge list on whichever seat changed", () => {
    const view = chromeView();
    renderClient(view);

    for (const [player, side] of [
      ["p1", "you"],
      ["p2", "opponent"],
    ] as const) {
      const target = targetFor({ type: "modifierChanged", player, modifierId: "m1", added: false }, view);
      expect(target, `${player} has a modifier target`).toBe(`modifiers-${side}`);
      expect(rendered(must(target, "the modifier target")), `modifiers-${side} is rendered`).not.toBeNull();
    }
  });

  it("only the three seat-conditional rows resolve to nothing, and only on the seat that shows nothing", () => {
    const view = chromeView();
    const samples = samplesFor(view);
    renderClient(view);

    const wrong: string[] = [];
    for (const type of GAME_EVENT_TYPES) {
      const target = targetFor(forOtherPlayer(samples[type]), view);
      if (SEAT_CONDITIONAL.includes(type)) {
        // §10.6: the other seat has no modal, and the offerer has no toast. Nothing to animate.
        if (target !== null) wrong.push(`${type} should show nothing on the other seat, got ${target}`);
        continue;
      }
      // Every other row is two-sided: swapping whose event it is must still land somewhere real.
      if (target === null) {
        wrong.push(`${type} went dark when the event was the other seat's — is it seat-conditional?`);
        continue;
      }
      if (rendered(target) === null) wrong.push(`${type} → [data-testid="${target}"] is not rendered`);
    }

    expect(wrong, "the null rows are exactly promptOpened, promptAnswered and drawOffered").toEqual([]);
  });
});
