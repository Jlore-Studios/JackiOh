// Fixtures for the engine half of patch v0.2.0's Core patches and cosmetics (test/corePatches.test.ts):
// the play count R429 keeps (`timesPlayed`) and the mark R437 puts on a card a delayed effect waits
// for. Test-only
// definitions and scripts, prefixed `cp-` and indexed from 4400, so they collide with no other file's.

import type { CardDef, CardDefs } from "@jackioh/shared";
import { damage, delay } from "../../src/effects";
import { RESUME_HOOK } from "../../src/prompts";
import type { CardScripts, Script } from "../../src/script";

let nextIndex = 4400;

function def(name: string, type: CardDef["type"], extra: Partial<CardDef> & { attack?: number; health?: number } = {}): CardDef {
  nextIndex += 1;
  const { attack = 1, health = 1, ...rest } = extra;
  const face = type === "Unit" ? { attack, health, keywords: [], text: name } : { keywords: [], text: name };
  return {
    id: `cp-${name}`,
    index: String(nextIndex),
    name: `${name} (core patches)`,
    set: "Core",
    type,
    tags: [],
    rarity: "Common",
    token: false,
    cost: 1,
    base: { ...face },
    radiant: { ...face },
    ...rest,
  };
}

/** A Spell that asks the engine to count its plays (R429). */
export const counted = def("counted", "Spell");
/** A Spell that does not. */
export const uncounted = def("uncounted", "Spell");
/** A Unit whose Cry marks an enemy permanent for a hit at the start of its controller's next turn (R437). */
export const marker = def("marker", "Unit", { health: 3 });
/** A Trap that watches nothing: a face-down card to mark (R33). */
export const quietTrap = def("quiet-trap", "Trap");

export const CORE_PATCH_DEFS: readonly CardDef[] = [counted, uncounted, marker, quietTrap];

export function corePatchCatalog(): CardDefs {
  return Object.fromEntries(CORE_PATCH_DEFS.map((card) => [card.id, card]));
}

/** R437: the mark the marker's delayed hit puts on its target. */
export const TEST_MARK = { mark: "hit", color: "green" } as const;

const HIT = "hit";

const markerScript: Script = {
  targets: [{ kind: "target", min: 1, max: 1, filter: { side: "enemy", of: ["unit", "backrow"] } }],
  cry: (ctx) => {
    const chosen = ctx.targets[0];
    if (chosen === undefined || chosen.pick !== "instance") return [];
    return [
      delay({
        at: { phase: "start", player: "self" },
        step: HIT,
        hook: RESUME_HOOK,
        data: { target: chosen.instanceId },
        watch: chosen.instanceId,
        mark: TEST_MARK,
      }),
    ];
  },
  resume: {
    [HIT]: (ctx) => {
      const target = ctx.data.target;
      return typeof target === "string" ? [damage({ to: { of: "instance", instanceId: target }, amount: 1 })] : [];
    },
  },
};

function both(script: Script): CardScripts {
  return { base: script, radiant: script };
}

export const CORE_PATCH_SCRIPTS: Record<string, CardScripts> = {
  [counted.id]: both({ staticFlags: { countsPlays: true }, cry: () => [] }),
  [uncounted.id]: both({ cry: () => [] }),
  [marker.id]: both(markerScript),
  [quietTrap.id]: both({}),
};
