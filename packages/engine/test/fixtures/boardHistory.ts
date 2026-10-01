// Fixtures for E29's board snapshots (docs/classic-sets.md B5 E29; C+ #35 Rollback, R419, R563): test-only
// scripts in the card's shape, since the engine never imports `packages/cards`. Defs are prefixed `bh-`
// and indexed from 9700, so they collide with no other file's catalog (BUILD §0).

import type { CardDef } from "@jackioh/shared";
import { registerCatalog, registeredCatalog } from "../../src/catalog";
import { ROLLBACK_MAX_TURNS } from "../../src/config";
import { chosenNumber } from "../../src/effects/choose";
import type { CardScripts, Script } from "../../src/script";
import { registerScripts, registeredScripts } from "../../src/scripts";
import { rollBack } from "../../src/subsystems/boardHistory";

const face = { keywords: [], text: "" };

/** C+ #35 Rollback's shape: N declared with the play (R81), both sides, at (0). */
export const rewind: CardDef = {
  id: "bh-rewind",
  index: "9701",
  name: "rewind (board history fixture)",
  set: "Core",
  type: "Spell",
  tags: [],
  rarity: "Common",
  token: false,
  cost: 0,
  base: face,
  radiant: face,
};

/** A Reborn Unit whose Death rolls the board back one turn while its own zone is held for it (R64, R563). */
export const phoenix: CardDef = {
  ...rewind,
  id: "bh-phoenix",
  index: "9702",
  name: "phoenix (board history fixture)",
  type: "Unit",
  cost: 1,
  base: { attack: 2, health: 2, keywords: [{ kind: "Reborn" }], text: "Reborn" },
  radiant: { attack: 4, health: 4, keywords: [{ kind: "Reborn" }], text: "Reborn" },
};

const rewindScript: Script = {
  modes: [{ kind: "number", options: Array.from({ length: ROLLBACK_MAX_TURNS }, (_, n) => String(n + 1)) }],
  cry: (ctx) => [rollBack({ turnsAgo: chosenNumber(ctx) ?? ROLLBACK_MAX_TURNS, sides: "both" })],
};
const phoenixScript: Script = { death: () => [rollBack({ turnsAgo: 1, sides: "both" })] };

const SCRIPTS: Record<string, CardScripts> = {
  [rewind.id]: { base: rewindScript, radiant: rewindScript },
  [phoenix.id]: { base: phoenixScript, radiant: phoenixScript },
};

/** Adds the fixtures to whatever catalog and scripts are registered. */
export function registerBoardHistoryFixtures(): void {
  registerCatalog({ ...registeredCatalog(), [rewind.id]: rewind, [phoenix.id]: phoenix });
  registerScripts({ ...registeredScripts(), ...SCRIPTS });
}
