/**
 * Rematch offers after a finished non-series match (SPEC §9.5, R659).
 *
 * After the death screen lands, either seat may offer a rematch — a normal one or a
 * double-or-nothing — and a new match is created only when both seats offer equal stakes.
 * "The opponent is still here" is their match socket being open (`matches.presenceOf`), so the
 * buttons disappear when they leave or log out. Games of a Conquest series offer nothing: the
 * series' continue flow owns what comes next.
 *
 * Two endpoints:
 *  - `POST /api/matches/:matchId/rematch { stakes }` upserts the caller's offer and, when the
 *    seats' stakes match, creates the game and answers its id;
 *  - `GET /api/matches/:matchId/rematch` answers what each seat offered, whether the opponent is
 *    here, and the created game, if any.
 *
 * The response carries only offer stakes, presence booleans and ids — never decks, hands or
 * ratings (CLAUDE.md rule 7).
 *
 * The offers live in a module-level map, like `queue.ts`'s `e2eSeedByTicket`: they are rendezvous
 * state for two sockets, not rows — a restart drops them, and both clients re-offer. Entries
 * without a created game are swept on read past `REMATCH_OFFER_TTL_MS`; an entry that made a game
 * keeps its id so a late poller still learns where to go.
 */

import { pickPortraitFromSeed, type PlayerId } from "@jackioh/shared";

import { REMATCH_OFFER_TTL_MS } from "../config";
import { callerProfile } from "./collection";
import { ApiError, ok, route, type Route } from "./http";
import type { MatchRow, QueueMode, ServerDeps } from "./ports";

/** R659: a normal rematch moves the rating once; a double-or-nothing moves it twice. */
export const STAKE_NORMAL = 1;
/** R659: a double-or-nothing rematch. Ranked matches only (`double_requires_ranked`). */
export const STAKE_DOUBLE = 2;

export type RematchStakes = 1 | 2;

/** `POST /api/matches/:matchId/rematch`: the created game, or null while the seats disagree. */
export type RematchOfferBody = { matchId: string | null };

/** `GET /api/matches/:matchId/rematch`: both offers, the opponent's presence, the created game. */
export type RematchStatusBody = {
  youOffered: RematchStakes | null;
  opponentOffer: RematchStakes | null;
  opponentHere: boolean;
  matchId: string | null;
};

type OfferEntry = {
  /** Seat -> the stakes it offered. */
  offers: Map<PlayerId, RematchStakes>;
  /** `timers.now()` of the latest offer; the sweep reads this. */
  at: number;
  /** The game equal stakes made, once made. */
  matchId: string | null;
};

const offersByMatch = new Map<string, OfferEntry>();

/** Exported for the tests; nothing in `src/` calls it. */
export function clearRematchOffers(): void {
  offersByMatch.clear();
}

/** Exported for the tests; nothing in `src/` calls it. */
export function rematchOfferCount(): number {
  return offersByMatch.size;
}

function stakesOf(body: Readonly<Record<string, unknown>>): RematchStakes {
  const stakes = body["stakes"];
  if (stakes !== STAKE_NORMAL && stakes !== STAKE_DOUBLE) {
    throw new ApiError("bad_request", '"stakes" must be 1 (rematch) or 2 (double-or-nothing)');
  }
  return stakes;
}

/**
 * The live entry for a match, sweeping one whose offers went stale. An entry that already made a
 * game is never swept: the id is how a late poller finds the game.
 */
function liveEntry(matchId: string, now: number): OfferEntry | null {
  const entry = offersByMatch.get(matchId);
  if (entry === undefined) return null;
  if (entry.matchId === null && now - entry.at > REMATCH_OFFER_TTL_MS) {
    offersByMatch.delete(matchId);
    return null;
  }
  return entry;
}

function seatOf(match: MatchRow, profileId: string): PlayerId | null {
  if (match.players[0] === profileId) return "p1";
  if (match.players[1] === profileId) return "p2";
  return null;
}

function other(seat: PlayerId): PlayerId {
  return seat === "p1" ? "p2" : "p1";
}

/**
 * Creates the rematch both seats offered, exactly like `queue.ts`'s `startPairedMatch` makes a
 * paired match: a Best-of-1 replays the finished row's frozen decks and portraits under a fresh
 * seed, an All Random deals fresh decks from that seed (`${seed}:p1-deck`, `${seed}:p2-deck`, the
 * same suffix scheme), and both profiles go in-match in one transaction first.
 */
async function createRematch(
  deps: ServerDeps,
  finished: MatchRow,
  mode: QueueMode,
  stakes: RematchStakes,
  matchId: string,
): Promise<void> {
  const seed = deps.ids.seed();
  const random = mode === "random";
  const seats = [
    {
      profileId: finished.players[0],
      player: "p1" as const,
      deck: random ? deps.dealRandomDeck(`${seed}:p1-deck`) : [...finished.decks[0]],
      portrait: random
        ? pickPortraitFromSeed(`${seed}:portrait:p1`)
        : (finished.portraits?.[0] ?? undefined),
    },
    {
      profileId: finished.players[1],
      player: "p2" as const,
      deck: random ? deps.dealRandomDeck(`${seed}:p2-deck`) : [...finished.decks[1]],
      portrait: random
        ? pickPortraitFromSeed(`${seed}:portrait:p2`)
        : (finished.portraits?.[1] ?? undefined),
    },
  ] as const;

  await deps.store.tx(async (t) => {
    for (const seat of seats) {
      const profile = (await t.profiles.getMany([seat.profileId]))[0];
      // One of them found another game while the offers were coming in: the rematch loses, and
      // neither seat is stolen out of the game it is actually in.
      if (profile !== undefined && profile.inMatchId !== null) {
        throw new ApiError("already_in_match", "finish your current match first");
      }
      await t.profiles.setInMatch(seat.profileId, matchId);
      // A stray open ticket would block the re-queue M7-T1 promises after this game ends, so the
      // ending is not the only place that clears one (`results.ts`).
      const ticket = await t.tickets.openForProfile(seat.profileId);
      if (ticket !== null) await t.tickets.cancel(ticket.id, deps.timers.now());
    }
  });

  try {
    await deps.matches.start({
      matchId,
      seed,
      catalogVersion: finished.catalogVersion,
      // The rematch is ranked exactly when the finished match was (§9.5).
      ranked: finished.ranked ?? false,
      mode,
      // Absent reads as 1 downstream; only a double writes its stakes, and only ranked games
      // reach here with 2 (`double_requires_ranked` above).
      ...(stakes === STAKE_DOUBLE ? { stake: stakes } : {}),
      seats: [seats[0], seats[1]],
    });
  } catch (error) {
    // The `setInMatch` transaction above has already committed, so a failed start would lock both
    // players out of the queue, rooms and account deletion (`queue.ts` undoes the same way).
    try {
      await deps.store.tx(async (t) => {
        await t.profiles.setInMatch(finished.players[0], null);
        await t.profiles.setInMatch(finished.players[1], null);
      });
      await deps.store.matches.discardOpen(matchId);
    } catch (cleanupError) {
      deps.log.alert("rematch.cleanup_failed", {
        matchId,
        message: cleanupError instanceof Error ? cleanupError.message : String(cleanupError),
      });
    }
    throw error;
  }
  deps.log.info("rematch.created", { from: finished.id, matchId, stakes });
}

async function rematchMode(deps: ServerDeps, finished: MatchRow): Promise<QueueMode> {
  // A series game never reaches here (refused below), so `bo3` below is a store that lost a row,
  // not a player: refuse it like one.
  const mode = await deps.store.matches.modeOf(finished.id);
  if (mode === "bo3") throw new ApiError("series_game", "series games continue from the series screen");
  // A match nothing made (no tickets, room, series or rematch row): replay its frozen decks as a
  // Best of 1 rather than refusing a game both seats want.
  return mode ?? "bo1";
}

export function createRematchRoutes(): Route[] {
  return [
    /**
     * Offer a rematch (or meet one). `auth: "active"` is §9.4's gate, as on the queue and the
     * rooms; a seat of another match — or of none — learns nothing beyond "no such match" (§9.1).
     */
    route("POST", "/api/matches/:matchId/rematch", "active", async (req, deps) => {
      const profile = callerProfile(req);
      const stakes = stakesOf(req.body);
      const match = await deps.store.matches.get(req.params["matchId"] ?? "");
      if (match === null) throw new ApiError("not_found", "no such match");
      const seat = seatOf(match, profile.id);
      if (seat === null) throw new ApiError("not_found", "no such match");
      // R659: only a ranked match can spawn a double-or-nothing.
      if (stakes === STAKE_DOUBLE && !match.ranked) {
        throw new ApiError("double_requires_ranked", "double-or-nothing needs a ranked match");
      }
      if (match.status !== "finished") {
        throw new ApiError("match_not_finished", "offer a rematch once the match is over");
      }
      if ((await deps.store.series.withGame(match.id)) !== null) {
        throw new ApiError("series_game", "series games continue from the series screen");
      }

      const now = deps.timers.now();
      const entry = liveEntry(match.id, now) ?? { offers: new Map(), at: now, matchId: null };
      offersByMatch.set(match.id, entry);
      entry.at = now;
      entry.offers.set(seat, stakes);

      // One new game for one pair of equal offers. The check and the claim run synchronously, so
      // two offers landing together cannot mint two games; a failed create clears the claim and
      // rethrows, so a retry mints the next id.
      if (entry.offers.get(other(seat)) === stakes && entry.matchId === null) {
        entry.matchId = deps.ids.uuid();
        try {
          await createRematch(deps, match, await rematchMode(deps, match), stakes, entry.matchId);
        } catch (error) {
          if (offersByMatch.get(match.id) === entry) entry.matchId = null;
          throw error;
        }
      }
      const created: RematchOfferBody = { matchId: entry.matchId };
      return ok(created);
    }),

    /**
     * What each seat offered, whether the opponent is still on the match, and the created game.
     * A seat's own row only: anyone else gets the same "no such match" as for a missing id (§9.1).
     */
    route("GET", "/api/matches/:matchId/rematch", "active", async (req, deps) => {
      const profile = callerProfile(req);
      const match = await deps.store.matches.get(req.params["matchId"] ?? "");
      if (match === null) throw new ApiError("not_found", "no such match");
      const seat = seatOf(match, profile.id);
      if (seat === null) throw new ApiError("not_found", "no such match");

      const entry = liveEntry(match.id, deps.timers.now());
      const presence = deps.matches.presenceOf(match.id);
      const status: RematchStatusBody = {
        youOffered: entry?.offers.get(seat) ?? null,
        opponentOffer: entry?.offers.get(other(seat)) ?? null,
        // Gone whenever no live actor holds the match (a restart, the reaper) or the opponent's
        // socket closed: leaving, logging out and closing the tab all read as gone.
        opponentHere: presence?.[other(seat)] ?? false,
        matchId: entry?.matchId ?? null,
      };
      return ok(status);
    }),
  ];
}
