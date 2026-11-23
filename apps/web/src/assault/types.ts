// Hand mirror of the Infinite Assault's frozen Rust surface until the fullsend reconcile replaces
// these with `wire/generated/` exports. Rust serializes every field and enum below in camelCase.

import type { Rarity } from "../wire/index.ts";
import type { DeckMark } from "../wire/generated/DeckMark.ts";
import type { Handicap } from "../wire/engineConfig.ts";

export type AssaultContent = {
  cards: AssaultCard[];
  doctrines: AssaultDoctrine[];
  commanders: AssaultCommander[];
};

export type AssaultCard = {
  id: string;
  rarity: Rarity;
  doctrine: string | null;
  commander: boolean;
};

export type AssaultDoctrine = {
  id: string;
  name: string;
  line: string;
  enemies: string[];
};

export type AssaultCommander = {
  id: string;
  name: string;
  rank: number;
  doctrine: string | null;
  cards: string[];
};

export type AssaultUpgradeKind =
  | "openingCards"
  | "deckSize"
  | "radiant"
  | "buffed"
  | "manaBonus"
  | "manaCap"
  | "heroHealth"
  | "heroArmor"
  | "extraDraw";

export type AssaultUpgrade = { kind: AssaultUpgradeKind; stacks: number };

export type AssaultEscalation =
  | { kind: "cards" }
  | { kind: "upgrade"; upgrade: AssaultUpgradeKind };

export type AssaultLevel = {
  level: number;
  enemy: string;
  doctrine: string | null;
  commander: string | null;
  escalations: AssaultEscalation[];
  upgrades: AssaultUpgrade[];
  rarities: readonly [number, number, number, number, number];
  deck: string[];
  marks: DeckMark[];
  handicap: Handicap;
};

export type AssaultDay = { date: string; seed: string; levels: AssaultLevel[] };

export type AssaultDeckSource = "saved" | "preset" | "dayDeck";

export type AssaultSetup = {
  date: string;
  deck: string[];
  source: AssaultDeckSource;
  startLevel: number;
  attempt: number;
};

export type AssaultPhase = "briefing" | "fight" | "lost" | "repelled" | "abandoned";

export type AssaultState = {
  setup: AssaultSetup;
  level: number;
  phase: AssaultPhase;
  beaten: number[];
  turns: number;
};

export type AssaultAction =
  | { kind: "fight" }
  | { kind: "fightOver"; won: boolean; turns: number }
  | { kind: "abandon" };

export type AssaultFightSetup = {
  seed: string;
  humanDeck: string[];
  humanHandicap: Handicap;
  enemyDeck: string[];
  enemyHandicap: Handicap;
  enemyMarks: DeckMark[];
  level: AssaultLevel;
};

export type AssaultRecord = {
  date: string;
  best: number;
  checkpoints: number[];
  attempts: number;
  repelledOn: number | null;
};

export type AssaultFile = {
  version: string;
  setup: AssaultSetup;
  record: AssaultRecord;
  actions: AssaultAction[];
};

export type AssaultConstants = {
  levels: number;
  commanderLevels: number[];
  checkpoints: number[];
  historyDays: number;
};

export type AssaultResult = { state: AssaultState } | { error: string };
export type AssaultActionResult = { action: AssaultAction } | { error: string };
