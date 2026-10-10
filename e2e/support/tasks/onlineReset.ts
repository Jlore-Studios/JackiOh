// Reset only 99-online-smoke: no HTTP leave-match route or match id means abandoned matches block
// room and queue calls, so reset needs the database (R79).
// It ends rather than deletes matches because append-only `(seed, log)` is match truth (SPEC §9.3).
// `app.end_match` is the idempotent path (§9.5), using a ceiling draw and unchanged ratings.
// CI leaves `E2E_DATABASE_URL` off, so frozen M8 specs cannot call this task.

import { Client } from "pg";

export type OnlineResetResult = {
  ok: boolean;
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
    // `end_match` clears both players' current-match state (§9.5).
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
      // An unclaimed room cannot use `end_match`: its p2 constraint rejects it as over. With no
      // actions, deleting it is safe (§9.5).
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
      // `any()` does not preserve row order, so map ratings by id.
      const byId = new Map(ratings.rows.map((row2) => [row2.id, row2.rating]));
      const p1Rating = row.p1 === null ? 1000 : (byId.get(row.p1) ?? 1000);
      const p2Rating = row.p2 === null ? p1Rating : (byId.get(row.p2) ?? p1Rating);
      if (row.server_owned) {
        // Server-owned matches need server-only result writes; record a ceiling draw with unchanged
        // ratings, then end the related series so its players are freed.
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
      // Current double ratings make this an unchanged-rating draw.
      await client.query(
        "select app.end_match($1::uuid, null, 'match-ceiling', 0, $2::double precision, $3::double precision)",
        [row.id, p1Rating, p2Rating],
      );
      ended += 1;
    }

    // Clear any current-match id left by a match this task did not own.
    const profiles = await client.query(
      "update public.profiles set current_match_id = null where current_match_id is not null",
    );

    // Tickets are not append-only; remove stale ones so the matchmaker can continue.
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
