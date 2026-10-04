// Deterministic cleanup for 99-online-smoke.cy.ts, and only for it.
//
// WHY THIS NEEDS THE DATABASE. The spec's second run failed where its first had passed, with
// `409 Conflict` on `POST /api/rooms`: the first run had left both accounts inside a live match,
// and `assertNotInMatch` then refuses every room and queue call. There is no HTTP way out — the
// route table has no leave-match or concede endpoint, so a match ends only through the WebSocket
// `concede` action, R79's clocks, or the reaper. Conceding over the socket needs the match id,
// and `/api/auth/me` does not return one, so a harness that has lost the match URL cannot even
// find the match it is stuck in.
//
// That is a gap in the product worth fixing separately. For the suite it means the only reliable
// reset is the one below.
//
// IT ENDS MATCHES, IT DOES NOT DELETE THEM. The first version deleted from `match_actions` and
// the database refused: "append-only table public.match_actions may not be updated or deleted".
// That guard is SPEC §9.3 — `(seed, log)` IS the truth of a match, so the log cannot be rewritten,
// and `matches` cannot be deleted either once rows reference it. An earlier hand-run of the same
// DELETE appeared to work only because the table was empty, so it touched no rows and the trigger
// never fired.
//
// So the reset goes through `app.end_match`, which migration 0004 calls "the one path that
// terminates a match" and which is idempotent by design (§9.5's reaper and a client ending could
// race). The reason is `match-ceiling` — one of the seven `results_reason_check` allows, and the
// honest one for a match nobody finished, since §9.5 makes the ceiling a draw. Ratings are passed
// back unchanged so a reset cannot move anyone's hidden rating.
//
// INERT WITHOUT `E2E_DATABASE_URL`. CI sets neither it nor `E2E_ONLINE`, so the spec is skipped
// there and this task is never called. Nothing here is reachable from the frozen M8 specs.

import { Client } from "pg";

export type OnlineResetResult = {
  ok: boolean;
  /** What was cleared, for the spec to log. */
  cleared?: { profiles: number; tickets: number; matches: number };
  skipped?: string;
  error?: string;
};

export async function onlineReset(): Promise<OnlineResetResult> {
  const connectionString = process.env["E2E_DATABASE_URL"];
  if (connectionString === undefined || connectionString === "") {
    return { ok: false, skipped: "E2E_DATABASE_URL is not set" };
  }

  const client = new Client({ connectionString, ssl: { rejectUnauthorized: false } });
  try {
    await client.connect();
    // 1. End every unfinished match through the sanctioned path. `end_match` clears both
    //    players' `current_match_id` itself (§9.5: "every ending ... clears both players'
    //    in-match state"), which is the whole point of using it rather than a bare UPDATE.
    const live = await client.query<{
      id: string;
      p1: string | null;
      p2: string | null;
      server_owned: boolean;
    }>(
      `select m.id, m.p1_profile_id as p1, m.p2_profile_id as p2,
              (m.ranked or exists (
                select 1 from public.series s where s.next_match_id = m.id)) as server_owned
         from public.matches m
        where m.status <> 'over'`,
    );
    let ended = 0;
    for (const row of live.rows) {
      // An UNCLAIMED ROOM cannot go through `end_match`, and that is a defect rather than a
      // quirk: the function sets `status = 'over'` while `p2_profile_id` is still null, which
      // `matches_p2_required_when_not_open_check` rejects —
      //   new row for relation "matches" violates check constraint
      //   "matches_p2_required_when_not_open_check"
      // — so a room nobody joined can never be terminated by the sanctioned path, and §9.5's
      // reaper hits the same wall. Such a row has no `match_actions` (nothing was played), so
      // nothing append-only protects it and deleting it is safe here.
      if (row.p2 === null) {
        await client.query("delete from public.matches where id = $1::uuid and p2_profile_id is null", [
          row.id,
        ]);
        ended += 1;
        continue;
      }
      const ratings = await client.query<{ id: string; rating: number }>(
        "select id, rating from public.profiles where id = any($1::uuid[])",
        [[row.p1, row.p2].filter((id): id is string => id !== null)],
      );
      // Map by id: `any()` promises no order, so a positional rows[0]/rows[1] could hand p1's
      // rating to p2.
      const byId = new Map(ratings.rows.map((row2) => [row2.id, row2.rating]));
      const p1Rating = row.p1 === null ? 1000 : (byId.get(row.p1) ?? 1000);
      const p2Rating = row.p2 === null ? p1Rating : (byId.get(row.p2) ?? p1Rating);
      if (row.server_owned) {
        // A RANKED match, or one a series calls its game in play: `app.end_match` refuses both
        // since migration 0021, because the rating move, the ladder write and the series
        // transition live only in the server's result write. The reset cannot reproduce that
        // write — but it can end the match the honest way the reset always has: a winnerless
        // ceiling draw with ratings passed back unchanged (which is also what an abandoned
        // series' game records), plus the one extra statement the series itself needs — marked
        // 'over', abandoned, so its players are freed like everyone else's.
        await client.query(
          `insert into public.results (
             match_id, p1_profile_id, p2_profile_id, winner_profile_id, reason, turns,
             p1_rating_before, p1_rating_after, p2_rating_before, p2_rating_after, ended_at)
           values ($1::uuid, $2::uuid, $3::uuid, null, 'match-ceiling', 0,
                   $4::double precision, $4::double precision,
                   $5::double precision, $5::double precision, now())
           on conflict (match_id) do nothing`,
          [row.id, row.p1, row.p2, p1Rating, p2Rating],
        );
        await client.query(
          "update public.matches set status = 'over', ended_at = now() where id = $1::uuid",
          [row.id],
        );
        await client.query(
          `update public.series
              set status = 'over', version = version + 1, pick_deadline_at = null,
                  ended_at = now(), updated_at = now(),
                  state = state || '{"endReason": "abandoned"}'::jsonb
            where next_match_id = $1::uuid and status <> 'over'`,
          [row.id],
        );
        ended += 1;
        continue;
      }
      // No winner: a draw, so neither rating is meant to move, and passing the current values
      // back is how `end_match` is told that. The ratings are doubles since migration 0021
      // widened the column and the function from int — an ::int cast would refuse a real one.
      await client.query(
        "select app.end_match($1::uuid, null, 'match-ceiling', 0, $2::double precision, $3::double precision)",
        [row.id, p1Rating, p2Rating],
      );
      ended += 1;
    }

    // 2. Any straggler still pointing at a match `end_match` did not own.
    const profiles = await client.query(
      "update public.profiles set current_match_id = null where current_match_id is not null",
    );

    // 3. Tickets carry no append-only guard; a stale one would keep the matchmaker busy.
    const tickets = await client.query("delete from public.tickets");

    return {
      ok: true,
      cleared: {
        profiles: profiles.rowCount ?? 0,
        tickets: tickets.rowCount ?? 0,
        matches: ended,
      },
    };
  } catch (error) {
    return { ok: false, error: error instanceof Error ? error.message : String(error) };
  } finally {
    await client.end().catch(() => undefined);
  }
}
