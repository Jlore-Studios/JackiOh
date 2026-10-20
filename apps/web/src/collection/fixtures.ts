// Fixture responses for the collection screen's tests and its component spec (#1307): what
// `GET /api/collection`, `GET /api/tracks` and `GET /api/quests` answer, as the server's JSON. The
// wallet and the packs come from `shop/fixtures.ts` and `packs/fixtures.ts`. No number is a shipped
// one where a test reads it, so a test that finds one on screen has found the response's value and
// not a copy of the server's config.

import { SHIPPED_SETS, newestShippedSet } from "@jackioh/shared";

import type { CollectionResponse, QuestsResponse, TrackView, TracksResponse } from "../net/api.ts";
import { COLLECTED_DEFS } from "./collection.ts";

/** `GET /api/collection` for an account owning one copy of each of `ids`. */
export function collectionOwning(ids: readonly string[]): CollectionResponse {
  return { catalogVersion: "test", entries: ids.map((cardId) => ({ cardId, quantity: 1 })) };
}

/** `GET /api/collection` for the launch grant: one copy of every collected card. */
export function fullCollection(): CollectionResponse {
  return collectionOwning(COLLECTED_DEFS.map((def) => def.id));
}

/** One track: by default a locked Core track at level 1 with no XP, its first reward next. */
export function track(over: Partial<TrackView> = {}): TrackView {
  return {
    setId: "Core",
    level: 1,
    xp: 0,
    levelXp: 100,
    xpToNext: 100,
    nextRewards: { jlore: 40, setPacks: 0, cardTokens: 0 },
    rewardsClaimed: 0,
    unlocked: false,
    ...over,
  };
}

/** `GET /api/tracks`: every shipped set's track, the newest unlocked at level 7 with 650 XP to go. */
export function tracksOf(): TracksResponse {
  return {
    tracks: SHIPPED_SETS.map((setId) =>
      setId === newestShippedSet()
        ? track({
            setId,
            level: 7,
            xp: 2150,
            levelXp: 700,
            xpToNext: 650,
            nextRewards: { jlore: 25, setPacks: 2, cardTokens: 3 },
            rewardsClaimed: 7,
            unlocked: true,
          })
        : track({ setId }),
    ),
  };
}

/** `GET /api/quests` for an account holding one quest and waiting on no other. */
export function questsHeld(): QuestsResponse {
  return {
    quests: [{ questId: "win-3", title: "Win 3 games", progress: 1, target: 3, jlore: 80, xp: 4000, arrivedAt: 0 }],
    nextDueAt: null,
  };
}
