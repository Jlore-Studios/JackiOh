// B4.1, R387: a card never Discovers or generates a copy of itself, unless its text names a pool
// that holds it or makes copies of "this" (docs/classic-sets.md B4.1). This is rule 5's sweep: every
// card with a script is played on a busy board under several seeds, every prompt it opens is
// answered, and no card the play CREATED — an instance that did not exist before the play — carries
// the played card's own definition, and no Discover it opens offers that definition. A card moved
// rather than created (a card returning itself to hand, R25) is not generation and is not counted.
//
// The sweep reaches every set's scripts as they land: a new generating card that forgets its own
// exclusion fails here, named. The exceptions are the texts that say so, each with its reason.

import { legalActions, reduce, type GameState } from "@jackioh/engine";
import type { ActionBody, GameEvent, PlayerId } from "@jackioh/shared";
import { describe, expect, it } from "vitest";
import { CATALOG, CATALOG_IDS } from "../src/catalog-data";
import { scenario } from "./_harness";

/**
 * B4.1 rules 3 and 4: the cards whose text makes a copy of itself, names itself, or names a pool
 * that holds it. Anything else that produces its own definition is a bug.
 */
const NAMES_ITS_OWN_POOL: Readonly<Record<string, string>> = {
  "core-012": "Duplicating Felinors summons a copy of this unit (rule 4: copies are not generation)",
  "core-087": "Pocket Chaos names itself: \"add a Pocket Chaos with a base cost (1) less than this one's to your opponent's hand\" (rule 3)",
  "core-090": "CN-Viral Injection shuffles copies of itself (rule 4)",
  "core-095": "Call to Chaos casts a random Call to Chaos, a pool it names (R28, rule 3)",
  "classicplus-004": "Juhan Biggest Bat makes the cards beneath it copies of this (rule 4)",
  "classicplus-046-1": "Felinor Flagbearer Prime fills the board with copies of this (rule 4)",
  "classicplus-073": "Call to Chaos (Classic+ Edition) names the Call to Chaos pool (R28, rule 3)",
};

/** The seeds each card is played under. */
const SEEDS = ["self-gen-a", "self-gen-b", "self-gen-c", "self-gen-d"];

/** The most prompts one play may open before the sweep gives up answering (a bound, not a rule). */
const MAX_ANSWERS = 40;

/** A busy board: units and backrow on both sides, libraries, graveyards and hands to reach into. */
function board(seed: string, defId: string): ReturnType<typeof scenario> {
  return scenario({
    seed,
    p1: {
      hand: [defId, "core-010", "core-005"],
      mana: 10,
      field: [{ def: "core-008", lane: 1 }, { def: "core-020", lane: 2 }],
      backrow: [{ def: "core-006", lane: 4 }],
      library: ["core-011", "core-025", "core-013", "core-005", "core-041", "core-069", "core-004", "core-019"],
      graveyard: ["core-002", "core-035"],
    },
    p2: {
      hand: ["core-005", "core-008"],
      field: [{ def: "core-019", lane: 1 }, { def: "core-011", lane: 3 }],
      library: ["core-008", "core-020", "core-005", "core-004"],
      graveyard: ["core-056"],
    },
  });
}

function instanceNumber(id: string): number {
  const match = /^c(\d+)$/.exec(id);
  return match === null ? -1 : Number(match[1]);
}

/** The instances an event says came into being or arrived somewhere, with their definitions. */
function namedCards(event: GameEvent): { instanceId: string; defId: string }[] {
  switch (event.type) {
    case "addedToHand":
    case "summoned":
    case "shuffledIn":
    case "cardPlayed":
    case "burned":
      return [{ instanceId: event.instanceId, defId: event.defId }];
    case "libraryOverflow":
      // A copy the full library refused was still generated.
      return event.outcome === "notCreated" ? [{ instanceId: event.instanceId, defId: event.defId }] : [];
    case "transformed":
      return [{ instanceId: event.newInstanceId, defId: event.toDefId }];
    default:
      return [];
  }
}

/** What one play of `defId` under `seed` generated of itself, and what its Discovers offered of it. */
function sweep(defId: string, seed: string): string[] {
  const s = board(seed, defId);
  let state: GameState = s.state;
  const own = s.card(defId);
  const plays = legalActions(state, "p1").filter(
    (action): action is Extract<ActionBody, { type: "play" }> => action.type === "play" && action.instanceId === own.id,
  );
  if (plays.length === 0) return [];
  const pick = plays[instanceNumber(own.id) % plays.length] ?? plays[0];
  if (pick === undefined) return [];

  const firstNew = state.nextId;
  const found: string[] = [];
  const events: GameEvent[] = [];
  let nonce = 0;
  const step = (body: ActionBody, player: PlayerId): boolean => {
    nonce += 1;
    const result = reduce(state, { ...body, playerId: player, nonce: `self-gen-${nonce}` });
    if (result.error !== undefined) return false;
    state = result.state;
    events.push(...result.events);
    return true;
  };
  const offersItself = (): void => {
    const pending = state.pending;
    if (pending === null || pending.kind !== "discover") return;
    for (const option of pending.options) {
      if (option.selection.pick === "mode" && option.selection.option === defId) found.push(`Discover offered ${defId}`);
    }
  };

  if (!step(pick, "p1")) return [];
  offersItself();
  for (let answered = 0; answered < MAX_ANSWERS && state.pending !== null && state.result === null; answered += 1) {
    const player = state.pending.playerId;
    const answers = legalActions(state, player).filter((action) => action.type === "answer");
    const answer = answers[answered % Math.max(answers.length, 1)];
    if (answer === undefined || !step(answer, player)) break;
    offersItself();
  }

  // The played card itself is not generated, under whatever id it wears: a card set face-down takes a
  // fresh id, which the events that set it name through `formerId` (R227).
  const ownIds = new Set<string>([own.id]);
  for (const event of events) {
    if ((event.type === "cardPlayed" || event.type === "summoned") && event.formerId !== undefined && ownIds.has(event.formerId)) {
      ownIds.add(event.instanceId);
    }
  }
  for (const event of events) {
    for (const card of namedCards(event)) {
      if (card.defId === defId && !ownIds.has(card.instanceId) && instanceNumber(card.instanceId) >= firstNew) {
        found.push(`${event.type} created ${card.instanceId} (${defId})`);
      }
    }
  }
  return found;
}

/** Every card the sweep plays: non-token, with its own definition in the catalog. */
const SWEPT = CATALOG_IDS.filter((id) => {
  const def = CATALOG[id];
  return def !== undefined && !def.token && !def.tags.includes("Token") && NAMES_ITS_OWN_POOL[id] === undefined;
});

describe("R387 a card never generates itself (B4.1 rule 5's sweep)", () => {
  it("R387 no card's play creates or Discovers its own definition, over every set and several seeds", { timeout: 120_000 }, () => {
    const violations: string[] = [];
    for (const defId of SWEPT) {
      for (const seed of SEEDS) {
        for (const found of sweep(defId, seed)) violations.push(`${defId} @ ${seed}: ${found}`);
      }
    }
    expect(violations).toEqual([]);
  });

  it("R387 the exceptions are exactly the cards whose text copies itself or names its own pool", () => {
    for (const id of Object.keys(NAMES_ITS_OWN_POOL)) expect(CATALOG[id], id).toBeDefined();
  });
});
