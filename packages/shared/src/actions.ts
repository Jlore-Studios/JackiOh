// The action union (SPEC §10.2). Every action carries playerId and a client nonce, deduped by the reducer.

import type { PlayerId, Row } from "./catalog-types";

/** Where a permanent is being played (§3.2: the player picks the zone). */
export type ZoneChoice = { row: Row; lane: number };

/** One selection inside a play: an instance, a hero, or a zone (R81). */
export type Selection =
  | { pick: "instance"; instanceId: string }
  | { pick: "hero"; player: PlayerId }
  | { pick: "zone"; player: PlayerId; row: Row; lane: number }
  | { pick: "mode"; option: string }
  | { pick: "none" };

export type ActionBody =
  | { type: "mulligan"; keep: string[] }
  | {
      type: "play";
      instanceId: string;
      zone?: ZoneChoice;
      x?: number;
      embiggen?: boolean;
      /** Units sacrificed to pay a Tribute cost (§6.3). */
      tributes?: string[];
      targets?: Selection[];
      modes?: string[];
      /**
       * B5 E11, E19: Plague Tokens paying part of the price of a play from the graveyard (Classic #74
       * Corpse Plantation): `from` is the card they come off, `tokens` how many — each pays (1).
       */
      plague?: { from: string; tokens: number };
    }
  | { type: "attack"; attackerId: string; targetId: string }
  | { type: "switchPosition"; instanceId: string }
  /**
   * B3.2, R384: use a card's Activate ability. `ability` names it when the card has several; the
   * targets and modes it declares travel here as a play's do (R81), and `tributes` pays a Tribute its
   * cost names (Classic #21 Turtinator).
   */
  | {
      type: "activate";
      instanceId: string;
      ability?: string;
      targets?: Selection[];
      modes?: string[];
      tributes?: string[];
    }
  /** R43, R384: Heroic Power's activation, kept as an alias of `activate` so old logs replay. */
  | { type: "activatePower"; instanceId: string; targets?: Selection[] }
  | { type: "answer"; choiceId: string; selection: Selection[] }
  | { type: "offerDraw" }
  | { type: "answerDraw"; accept: boolean }
  | { type: "concede" }
  | { type: "endTurn" }
  /**
   * R345: the sender's own preference for R82's automatic turn end. A setting, not a move: it is
   * accepted from either seat at any moment of a live game, changes nothing on the board, and is
   * never offered by `legalActions`, so no policy ever sends it.
   */
  | { type: "setAutoEndTurn"; enabled: boolean }
  // Server-only (R79): never sent by a client.
  | { type: "timeout" }
  | { type: "disconnectExpired"; player: PlayerId }
  | { type: "ceilingReached" };

export type Action = ActionBody & { playerId: PlayerId; nonce: string };

/** Omit over a union, member by member: plain Omit would collapse it to the shared keys. */
export type DistributiveOmit<T, K extends PropertyKey> = T extends unknown ? Omit<T, K> : never;

/** An action without its nonce, which the caller or the server adds. */
export type ActionInput = ActionBody & { playerId: PlayerId };

export type ActionType = ActionBody["type"];

/** Actions the non-active player may take (BUILD M1-T3). */
export const NON_ACTIVE_ACTION_TYPES = [
  "answer",
  "concede",
  "answerDraw",
  "setAutoEndTurn",
  "disconnectExpired",
  "timeout",
  "ceilingReached",
] as const;

/** The actions that may be sent while a prompt is open (BUILD M1-T3). */
export const PROMPT_OPEN_ACTION_TYPES = [
  "answer",
  "mulligan",
  "concede",
  "setAutoEndTurn",
  "timeout",
  "disconnectExpired",
  "ceilingReached",
] as const;
