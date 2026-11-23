\set ON_ERROR_STOP on

-- Every check below must FAIL LOUDLY when the invariant in its heading is violated.
-- A `select` that prints rows is a diagnostic, never the check: psql prints "(0 rows)"
-- and moves on whether the answer was right or wrong. So each heading keeps its
-- `select` for the human reading the log, and follows it with a `do $$ ... raise
-- exception 'FAIL (CHECK n): ...' $$` block that is the actual assertion. CHECK 12 has
-- worked this way since it was fixed; the rest now do too.
--
-- `raise exception` + `\set ON_ERROR_STOP on` makes psql exit non-zero, which run.sh
-- turns into a non-zero exit for the whole suite. `raise notice 'FAIL ...'` does NOT
-- stop psql — run.sh greps the output for it as a second net, but inside this file the
-- FAIL path is always `raise exception`.
--
-- Several checks also assert that there was something to measure (a table with rows, a
-- policy that could have leaked, a grant that could have been too wide). An invariant
-- that holds because the schema is empty is the failure mode this file exists to catch.

-- The starter cards (R1310000), which the activation grant gives a new account. This does
-- seed-catalog's job: in a real deployment `seed-catalog` writes app.settings.starter_cards from
-- crates/server/src/config.rs STARTER_CARDS before it stamps the catalog version, and this suite
-- never runs seed-catalog, so 0050 left the list empty. Both non-Token cards CHECK 10 seeds are on
-- it; it holds for every file after this one, and 29_going_live.sql proves the filter.
update app.settings set value = '["core-001","core-002"]'::jsonb where key = 'starter_cards';
do $$
begin
  if app.setting('starter_cards') is distinct from '["core-001","core-002"]'::jsonb then
    raise exception 'FAIL (R1310000): app.settings.starter_cards reads %, so 0050 did not seed the key',
      coalesce(app.setting('starter_cards')::text, '(no row)');
  end if;
end $$;

\echo '=== CHECK 1: every table in public has RLS enabled (must be 0 rows) ==='
select c.relname as table_without_rls
  from pg_class c join pg_namespace n on n.oid = c.relnamespace
 where n.nspname = 'public' and c.relkind = 'r' and not c.relrowsecurity
 order by 1;
do $$
declare
  offenders text;
  total     bigint;
begin
  select count(*) into total
    from pg_class c join pg_namespace n on n.oid = c.relnamespace
   where n.nspname = 'public' and c.relkind = 'r';
  if total = 0 then
    raise exception 'FAIL (CHECK 1): no tables in public — the migrations did not apply';
  end if;

  select string_agg(c.relname, ', ' order by c.relname) into offenders
    from pg_class c join pg_namespace n on n.oid = c.relnamespace
   where n.nspname = 'public' and c.relkind = 'r' and not c.relrowsecurity;
  if offenders is not null then
    raise exception 'FAIL (CHECK 1): public tables without RLS: %', offenders;
  end if;

  raise notice 'OK (CHECK 1): all % public tables have RLS enabled', total;
end $$;

\echo '=== CHECK 2: the 84 tables of migrations 0001-0097 ==='
select count(*) as public_tables from pg_class c
  join pg_namespace n on n.oid = c.relnamespace
 where n.nspname = 'public' and c.relkind = 'r';
select string_agg(c.relname, ', ' order by c.relname) as tables from pg_class c
  join pg_namespace n on n.oid = c.relnamespace
 where n.nspname = 'public' and c.relkind = 'r';
do $$
declare
  -- BUILD M6-T1..T4: profiles/invite_codes/code_attempts (0001), cards/collection/
  -- collection_grants (0002), loadouts/loadout_decks/loadout_deck_cards (0003),
  -- matches/match_actions/tickets/results (0004); then decks/trios (0007, R250, R252),
  -- series (0009, R263), tutorial_progress (0011, R320), game_records (0014, R376), last_boards
  -- (0017, R565), player_settings (0018, R633), player_stats (0021, R654), the ranked ladder's
  -- seasons/season_ranks/bot_ratings/rated_games (0022, R603-R612) and the play telemetry's
  -- action_timings/emote_events/match_signals (0029, R1442), the ladder bots' bots (0032, R1454),
  -- the age_checks/terms_acceptances of consent (0034, R757000, R757001), and the friend_requests, friendships, blocks, presence, challenges, chat_messages,
  -- chat_reports and replay_shares of Play together (0038, #936) and its Duos co-op's duos_rooms,
  -- duos_matches and duos_match_actions (0039, #946), the card statistics' record_totals,
  -- card_totals and decklist_totals (0041, R1044000), the live matches' match_checkpoints
  -- (0042, #1169), and the economy's wallets, jlore_grants, inventory, inventory_grants,
  -- economy_counters, economy_counter_grants, set_tracks, set_track_grants, held_quests,
  -- quest_schedules, quest_grants, cosmetics, card_cosmetics, account_cosmetics and cosmetic_grants
  -- (0045, SPEC §14.11, #1299), practice earning's practice_seeds (0047, #1303), the files
  -- stats-import --totals-only folded, game_record_batches (0060, R1976000), the account's
  -- per-card picks, card_cosmetic_picks (0061, R1834000), each patch's whole games,
  -- game_totals (0063, R1994000), a saved deck's earlier versions, deck_versions (0066,
  -- R2100000), a spend's idempotency_keys (0067, R2124000), a news entry's claims, news_claims
  -- (0077, R2875000), the practice claims' practice_claim_counts (0078, #2884), the dev
  -- dashboard's admin_actions, hand_set_ratings, rating_grants and news_posts (0080, R3074000),
  -- and each card's plays, level and stamp, card_proficiency (0081, R3013000), and the Fleet track's
  -- fleet_tracks and fleet_track_grants (0086, R3335001), and the daily games' daily_games,
  -- daily_invites, daily_notify and push_subscriptions (0089, SPEC §9.13, #3727), and a private
  -- lobby's lobbies (0090, R3730000), and the player mail's daily_email_days (0092, #3843), and the
  -- in-game notification feed's notifications (0093, R4039000), and the live co-op matches'
  -- duos_match_checkpoints (0097, #4169).
  -- 0005, 0006, 0008, 0010, 0012, 0013, 0015, 0016, 0019, 0020, 0023, 0024, 0025, 0026, 0027, 0028,
  -- 0030, 0031, 0033, 0035, 0036, 0040, 0043, 0044, 0046, 0048, 0049, 0050, 0051, 0052, 0053, 0054,
  -- 0055, 0056, 0057, 0058, 0059, 0062, 0064, 0065, 0068, 0069, 0070, 0071, 0072, 0073, 0074, 0075,
  -- 0076 and 0079 add no table, 0082 drops custom_cards, which 0037 had added, and 0083, 0084, 0085, 0087, 0088 and
  -- 0091 add none, nor do 0094, 0095 and 0096.
  -- The three loadout tables stay after 0007, unread and unwritten (R254), so they are still expected
  -- here.
  expected constant text[] := array[
    'account_cosmetics', 'action_timings', 'admin_actions', 'age_checks', 'blocks', 'bot_ratings', 'bots', 'card_cosmetic_picks',
    'card_cosmetics', 'card_proficiency', 'card_totals', 'cards', 'challenges',
    'chat_messages', 'chat_reports', 'code_attempts', 'collection', 'collection_grants', 'cosmetic_grants',
    'cosmetics', 'daily_email_days', 'daily_games', 'daily_invites', 'daily_notify',
    'deck_versions', 'decklist_totals', 'decks', 'duos_match_actions', 'duos_match_checkpoints', 'duos_matches', 'duos_rooms', 'economy_counter_grants',
    'economy_counters', 'emote_events',
    'fleet_track_grants', 'fleet_tracks', 'friend_requests', 'friendships', 'game_record_batches', 'game_records', 'game_totals', 'hand_set_ratings', 'held_quests',
    'idempotency_keys', 'inventory',
    'inventory_grants',
    'invite_codes', 'jlore_grants', 'last_boards', 'loadout_deck_cards', 'lobbies',
    'loadout_decks', 'loadouts', 'match_actions', 'match_checkpoints', 'match_signals', 'matches',
    'news_claims', 'news_posts', 'notifications', 'player_settings',
    'player_stats', 'practice_claim_counts', 'practice_seeds', 'presence', 'profiles', 'push_subscriptions', 'quest_grants', 'quest_schedules', 'rated_games', 'rating_grants', 'record_totals',
    'replay_shares', 'results',
    'season_ranks', 'seasons', 'series', 'set_track_grants', 'set_tracks', 'terms_acceptances', 'tickets',
    'trios', 'tutorial_progress', 'wallets'];
  actual  text[];
  missing text[];
  extra   text[];
begin
  select coalesce(array_agg(c.relname order by c.relname), '{}'::text[]) into actual
    from pg_class c join pg_namespace n on n.oid = c.relnamespace
   where n.nspname = 'public' and c.relkind = 'r';

  select coalesce(array_agg(e order by e), '{}'::text[]) into missing
    from unnest(expected) e where e <> all (actual);
  select coalesce(array_agg(a order by a), '{}'::text[]) into extra
    from unnest(actual) a where a <> all (expected);

  if cardinality(missing) > 0 then
    raise exception 'FAIL (CHECK 2): missing table(s): %', array_to_string(missing, ', ');
  end if;
  if cardinality(extra) > 0 then
    raise exception
      'FAIL (CHECK 2): unexpected table(s) in public: % — a new table needs an RLS policy, a grant decision and a row here',
      array_to_string(extra, ', ');
  end if;

  raise notice 'OK (CHECK 2): exactly the % expected tables', cardinality(expected);
end $$;

\echo '=== CHECK 3: L4 unique index ==='
select indexdef from pg_indexes where indexname = 'loadout_card_unique';
do $$
declare
  r record;
begin
  -- SPEC §9.4 L4: "a card id appears in at most one deck, also enforced by a unique
  -- index on (profile_id, card_id)". The failure this asserts against is `slot` creeping
  -- into the key: the index would still exist, still be unique, still be named the same,
  -- and enforce nothing beyond the primary key. 0003's own comment says "Do not add
  -- `slot` to this index"; this is that comment as a test.
  select i.indisunique,
         i.indnkeyatts,
         i.indpred is null   as no_predicate,
         i.indexprs is null  as no_expressions,
         t.relname           as table_name,
         (select string_agg(pg_get_indexdef(i.indexrelid, k, true), ', ' order by k)
            from generate_series(1, i.indnkeyatts) k) as key_columns
    into r
    from pg_index i
    join pg_class ic on ic.oid = i.indexrelid
    join pg_class t  on t.oid  = i.indrelid
    join pg_namespace n on n.oid = ic.relnamespace
   where n.nspname = 'public' and ic.relname = 'loadout_card_unique';

  if not found then
    raise exception 'FAIL (CHECK 3): index loadout_card_unique does not exist (SPEC §9.4 L4)';
  end if;
  if r.table_name <> 'loadout_deck_cards' then
    raise exception 'FAIL (CHECK 3): loadout_card_unique is on public.%, not loadout_deck_cards',
      r.table_name;
  end if;
  if not r.indisunique then
    raise exception 'FAIL (CHECK 3): loadout_card_unique is not a UNIQUE index — it enforces nothing';
  end if;
  if r.key_columns is distinct from 'profile_id, card_id' then
    raise exception
      'FAIL (CHECK 3): loadout_card_unique keys on (%), expected (profile_id, card_id) — with `slot` in the key it only repeats the primary key and L4 is unenforced',
      r.key_columns;
  end if;
  if not r.no_predicate or not r.no_expressions then
    raise exception
      'FAIL (CHECK 3): loadout_card_unique is partial or expression-based; L4 must hold for every row';
  end if;

  raise notice 'OK (CHECK 3): unique index loadout_card_unique on loadout_deck_cards (%)',
    r.key_columns;
end $$;

\echo '=== CHECK 4: no SECURITY DEFINER function in the exposed public schema ==='
select p.proname from pg_proc p join pg_namespace n on n.oid = p.pronamespace
 where n.nspname = 'public' and p.prosecdef order by 1;
do $$
declare
  offenders text;
begin
  -- No vacuity guard is possible here: `public` holds no functions at all today, and the
  -- point of the check is that it stays that way. Every privileged routine lives in `app`,
  -- which anon/authenticated have no USAGE on (0001). A SECURITY DEFINER function in the
  -- PostgREST-exposed schema is a callable escalation, so one appearing is the failure.
  select string_agg(p.proname, ', ' order by p.proname) into offenders
    from pg_proc p join pg_namespace n on n.oid = p.pronamespace
   where n.nspname = 'public' and p.prosecdef;
  if offenders is not null then
    raise exception
      'FAIL (CHECK 4): SECURITY DEFINER function(s) in the exposed public schema: % — privileged code belongs in app',
      offenders;
  end if;
  raise notice 'OK (CHECK 4): no SECURITY DEFINER function in public';
end $$;

\echo '=== CHECK 5: no INSERT/UPDATE/DELETE policy for anon or authenticated ==='
select schemaname, tablename, policyname, cmd, roles
  from pg_policies
 where schemaname = 'public' and cmd <> 'SELECT'
 order by tablename, policyname;
do $$
declare
  offenders  text;
  select_pol bigint;
begin
  -- SPEC §9.4: "no client path writes either". RLS denies by default, so the invariant is
  -- that no write policy ever opens that default. `public` counts as a client role: it
  -- includes anon and authenticated.
  select string_agg(format('%s.%s (%s for %s)', tablename, policyname, cmd,
                           array_to_string(roles, '/')), ', ' order by tablename, policyname)
    into offenders
    from pg_policies
   where schemaname = 'public'
     and cmd <> 'SELECT'
     and roles && array['anon', 'authenticated', 'public']::name[];
  if offenders is not null then
    raise exception 'FAIL (CHECK 5): write policy reachable by a client role: %', offenders;
  end if;

  -- Vacuity guard: "no write policies" means nothing if there are no policies at all.
  select count(*) into select_pol
    from pg_policies
   where schemaname = 'public' and cmd = 'SELECT'
     and roles && array['anon', 'authenticated', 'public']::name[];
  if select_pol = 0 then
    raise exception
      'FAIL (CHECK 5): no SELECT policy for any client role — the schema has no RLS to speak of, so this check measured nothing';
  end if;

  raise notice 'OK (CHECK 5): % client SELECT policies, 0 client write policies', select_pol;
end $$;

\echo '=== CHECK 6: client write privileges on public tables (must be 0 rows) ==='
-- Read the ACLs directly rather than information_schema.role_table_grants, which hides
-- grants whose grantor and grantee are both roles the caller is not a member of, and which
-- does not report grants to PUBLIC at all.
select c.relname as table_name,
       coalesce(g.rolname, 'PUBLIC') as grantee,
       a.privilege_type
  from pg_class c
  join pg_namespace n on n.oid = c.relnamespace
  cross join lateral aclexplode(coalesce(c.relacl, acldefault('r', c.relowner))) a
  left join pg_roles g on g.oid = a.grantee
 where n.nspname = 'public' and c.relkind = 'r'
   and coalesce(g.rolname, 'PUBLIC') in ('anon', 'authenticated', 'PUBLIC')
   and a.privilege_type in ('INSERT', 'UPDATE', 'DELETE', 'TRUNCATE')
 order by 1, 2, 3;
do $$
declare
  offenders  text;
  read_grant bigint;
begin
  select string_agg(format('%s:%s:%s', c.relname, coalesce(g.rolname, 'PUBLIC'),
                           a.privilege_type), ', ' order by c.relname, a.privilege_type)
    into offenders
    from pg_class c
    join pg_namespace n on n.oid = c.relnamespace
    cross join lateral aclexplode(coalesce(c.relacl, acldefault('r', c.relowner))) a
    left join pg_roles g on g.oid = a.grantee
   where n.nspname = 'public' and c.relkind = 'r'
     and coalesce(g.rolname, 'PUBLIC') in ('anon', 'authenticated', 'PUBLIC')
     and a.privilege_type in ('INSERT', 'UPDATE', 'DELETE', 'TRUNCATE');
  if offenders is not null then
    raise exception
      'FAIL (CHECK 6): a client role holds write privileges: % — every write goes through an app.* SECURITY DEFINER function',
      offenders;
  end if;

  -- Vacuity guard: authenticated must still be able to READ its projections (SPEC §9.1),
  -- so a schema that revoked everything from everyone is not a pass.
  select count(*) into read_grant
    from pg_class c
    join pg_namespace n on n.oid = c.relnamespace
    cross join lateral aclexplode(coalesce(c.relacl, acldefault('r', c.relowner))) a
    join pg_roles g on g.oid = a.grantee
   where n.nspname = 'public' and c.relkind = 'r'
     and g.rolname = 'authenticated' and a.privilege_type = 'SELECT';
  if read_grant = 0 then
    raise exception
      'FAIL (CHECK 6): authenticated has SELECT on no public table — nothing was granted, so "no write grants" measured nothing';
  end if;

  raise notice 'OK (CHECK 6): 0 client write grants, % authenticated SELECT grants', read_grant;
end $$;

\echo '=== CHECK 7: append-only triggers ==='
select c.relname as table_name, t.tgname
  from pg_trigger t join pg_class c on c.oid = t.tgrelid
  join pg_namespace n on n.oid = c.relnamespace
 where n.nspname = 'public' and not t.tgisinternal
 order by 1, 2;
do $$
declare
  -- SPEC §9.4 (collection_grants) and §9.3 (the match action log) both say append-only,
  -- and both are enforced by app.deny_row_mutation() fired BEFORE UPDATE OR DELETE FOR
  -- EACH ROW. A listing cannot tell a live guard from a disabled one or from a trigger
  -- that only fires on INSERT, so the flags are asserted, not printed.
  -- SPEC §14.11 (0045) holds the economy's six ledgers to the same rule, and 0077 the news claims
  -- (R2875000), which go only with their account as a ledger's rows do; 0080 the dev dashboard's
  -- audit and rating ledger (R3074000, R3074001); 0086 the Fleet track's ledger (R3335001).
  ledgers constant text[] := array['collection_grants', 'match_actions', 'jlore_grants', 'inventory_grants',
                                   'economy_counter_grants', 'set_track_grants', 'quest_grants',
                                   'cosmetic_grants', 'news_claims', 'admin_actions', 'rating_grants',
                                   'fleet_track_grants'];
  tbl     text;
  r       record;
begin
  foreach tbl in array ledgers loop
    select t.tgname, t.tgtype, t.tgenabled, p.proname, pn.nspname as pronsp
      into r
      from pg_trigger t
      join pg_class c on c.oid = t.tgrelid
      join pg_namespace n on n.oid = c.relnamespace
      join pg_proc p on p.oid = t.tgfoid
      join pg_namespace pn on pn.oid = p.pronamespace
     where n.nspname = 'public' and c.relname = tbl and not t.tgisinternal
       and p.proname = 'deny_row_mutation';

    if not found then
      raise exception
        'FAIL (CHECK 7): public.% has no app.deny_row_mutation() trigger — the append-only ledger can be rewritten',
        tbl;
    end if;
    if r.pronsp <> 'app' then
      raise exception 'FAIL (CHECK 7): public.% fires %.deny_row_mutation, not app.deny_row_mutation',
        tbl, r.pronsp;
    end if;
    -- tgtype bits: 1 = FOR EACH ROW, 2 = BEFORE, 8 = DELETE, 16 = UPDATE.
    if (r.tgtype & 1) = 0 then
      raise exception 'FAIL (CHECK 7): %.% is a statement trigger; it must be FOR EACH ROW', tbl, r.tgname;
    end if;
    if (r.tgtype & 2) = 0 then
      raise exception 'FAIL (CHECK 7): %.% does not fire BEFORE the write', tbl, r.tgname;
    end if;
    if (r.tgtype & 16) = 0 then
      raise exception 'FAIL (CHECK 7): %.% does not fire on UPDATE — the ledger is editable', tbl, r.tgname;
    end if;
    if (r.tgtype & 8) = 0 then
      raise exception 'FAIL (CHECK 7): %.% does not fire on DELETE — the ledger is erasable', tbl, r.tgname;
    end if;
    -- tgenabled: 'O' fires normally, 'A' always; 'D' disabled, 'R' replica-only.
    if r.tgenabled not in ('O', 'A') then
      raise exception 'FAIL (CHECK 7): %.% is disabled (tgenabled = %)', tbl, r.tgname, r.tgenabled;
    end if;

    raise notice 'OK (CHECK 7): public.% guarded by % (before update or delete, each row)',
      tbl, r.tgname;
  end loop;
end $$;

\echo '=== CHECK 8: signup trigger creates a pending profile ==='
insert into auth.users (id, email, email_confirmed_at)
values ('11111111-1111-1111-1111-111111111111', 'p1@example.test', now()),
       ('22222222-2222-2222-2222-222222222222', 'p2@example.test', null);
select id, status, rating from public.profiles order by id;
do $$
declare
  n   bigint;
  bad text;
begin
  -- SPEC §9.4: a signup lands as `pending` at rating 1000 and owns nothing until a code is
  -- redeemed. The two auth.users rows above are the only input; app.handle_new_user does
  -- the rest, so two profiles is the assertion, not a number to eyeball.
  --
  -- Counted over these two ids, not the whole table: 03b_legacy_loadout_seed.sql signed up and
  -- activated profile 44444444-… before migration 0007 ran, so that 0007 had a loadout to
  -- convert (R254), and that profile is neither new nor pending.
  select count(*) into n from public.profiles
   where id in ('11111111-1111-1111-1111-111111111111', '22222222-2222-2222-2222-222222222222');
  if n <> 2 then
    raise exception
      'FAIL (CHECK 8): % profile row(s) after 2 signups — app.handle_new_user did not fire for each',
      n;
  end if;

  select string_agg(format('%s status=%s rating=%s activated=%s', id, status, rating,
                           activated_at is not null), '; ' order by id)
    into bad
    from public.profiles
   where id in ('11111111-1111-1111-1111-111111111111', '22222222-2222-2222-2222-222222222222')
     and (status <> 'pending' or rating <> 1000 or activated_at is not null);
  if bad is not null then
    raise exception 'FAIL (CHECK 8): a fresh profile is not pending/1000/unactivated: %', bad;
  end if;

  select count(*) into n from public.collection
   where profile_id in ('11111111-1111-1111-1111-111111111111', '22222222-2222-2222-2222-222222222222');
  if n <> 0 then
    raise exception
      'FAIL (CHECK 8): a pending profile already owns % collection row(s) — the launch grant fired before activation',
      n;
  end if;

  raise notice 'OK (CHECK 8): 2 pending profiles at rating 1000, no collection';
end $$;

\echo '=== CHECK 9: redemption — rejections ==='
insert into public.invite_codes (code_hash, label, max_uses)
values ('hash-good', 'bring-up', 2), ('hash-revoked', 'revoked', 1);
update public.invite_codes set revoked_at = now() where code_hash = 'hash-revoked';
do $$
declare
  unverified text;
  missing    text;
  revoked    text;
  attempts   bigint;
  n          bigint;
begin
  -- Each redemption is called exactly once. It is not a pure read: it logs a code_attempt
  -- and counts against the 5-per-hour-per-profile limit (SPEC §9.4), and `hash-good` has
  -- exactly 2 uses which CHECK 10 and 03_match_lifecycle spend. So the values are captured
  -- into variables and raised as a notice rather than re-run for a printable table.
  unverified := app.redeem_invite_code('22222222-2222-2222-2222-222222222222', 'hash-good', 'ip-a');
  missing    := app.redeem_invite_code('11111111-1111-1111-1111-111111111111', 'hash-missing', 'ip-a');
  revoked    := app.redeem_invite_code('11111111-1111-1111-1111-111111111111', 'hash-revoked', 'ip-a');
  raise notice 'CHECK 9: unverified_email=%  missing_code=%  revoked_code=%',
    unverified, missing, revoked;

  if unverified <> 'email_unverified' then
    raise exception
      'FAIL (CHECK 9): profile 2 has an unverified email and redemption returned "%" — SPEC §9.4 step (1)',
      unverified;
  end if;
  -- SPEC §9.4: "Missing, expired and exhausted codes return an identical error in identical
  -- time." Distinguishable answers are the brute-force oracle the invite gate exists to deny.
  if missing <> 'invalid_code' or revoked <> 'invalid_code' then
    raise exception
      'FAIL (CHECK 9): missing="%" revoked="%" — both must be the same opaque invalid_code',
      missing, revoked;
  end if;

  -- SPEC §9.4 step (4): "log the attempt either way". Two attempts, not three: the unverified
  -- email is rejected at step (1), before the attempt log, so only the two code lookups count.
  select count(*) into attempts from public.code_attempts;
  if attempts <> 2 then
    raise exception
      'FAIL (CHECK 9): % code_attempts row(s) after 2 code lookups — a rejected attempt went unlogged',
      attempts;
  end if;
  if exists (select 1 from public.code_attempts where succeeded) then
    raise exception 'FAIL (CHECK 9): a rejected redemption was logged as succeeded';
  end if;

  -- Nothing may have moved: a rejection activates no account and spends no code. (The two
  -- profiles this check redeems for; 03b's legacy profile was active before this file ran.)
  select count(*) into n from public.profiles
   where id in ('11111111-1111-1111-1111-111111111111', '22222222-2222-2222-2222-222222222222')
     and status <> 'pending';
  if n <> 0 then
    raise exception 'FAIL (CHECK 9): a rejected redemption activated % profile(s)', n;
  end if;
  select count(*) into n from public.invite_codes where uses <> 0;
  if n <> 0 then
    raise exception 'FAIL (CHECK 9): a rejected redemption consumed % code(s)', n;
  end if;

  raise notice 'OK (CHECK 9): 3 rejections, 2 logged attempts, 0 activations, 0 codes spent';
end $$;

\echo '=== CHECK 10: redemption — success flips status and fires the launch grant ==='
insert into public.cards (id, card_index, name, set_id, type, tags, rarity, token, cost,
                          catalog_version)
values ('core-001', '1', 'Test Unit', 'Core', 'Unit', '{}', 'Common', false, '1'::jsonb, 'core-1'),
       ('core-002', '2', 'Test Spell', 'Core', 'Spell', '{}', 'Rare', false, '2'::jsonb, 'core-1'),
       ('tok-001', '1.1', 'Test Token', 'Core', 'Unit', '{Token}', 'Token', true, '0'::jsonb,
        'core-1');
do $$
declare
  redeemed text;
begin
  -- One call only: `hash-good` has 2 uses and 03_match_lifecycle needs the second.
  redeemed := app.redeem_invite_code('11111111-1111-1111-1111-111111111111', 'hash-good', 'ip-a');
  raise notice 'CHECK 10: redeem_ok=%', redeemed;
  if redeemed <> 'ok' then
    raise exception 'FAIL (CHECK 10): redemption returned "%", expected ok', redeemed;
  end if;
end $$;
select status, activated_at is not null as activated from public.profiles
 where id = '11111111-1111-1111-1111-111111111111';
select card_id, quantity from public.collection
 where profile_id = '11111111-1111-1111-1111-111111111111' order by card_id;
select card_id, delta, reason from public.collection_grants
 where profile_id = '11111111-1111-1111-1111-111111111111' order by card_id;
do $$
declare
  caller constant uuid := '11111111-1111-1111-1111-111111111111';
  p      record;
  owned  text;
  grants text;
  n      bigint;
begin
  select status, activated_at is not null as activated into p
    from public.profiles where id = caller;
  if p.status <> 'active' or not p.activated then
    raise exception 'FAIL (CHECK 10): profile 1 is status=% activated=% after a successful redemption',
      p.status, p.activated;
  end if;

  -- R111, R1310000: one copy of each starter card, which the top of this file lists as both
  -- non-Token cards here, and never the Token.
  select string_agg(format('%s x%s', card_id, quantity), ', ' order by card_id) into owned
    from public.collection where profile_id = caller;
  if owned is distinct from 'core-001 x1, core-002 x1' then
    raise exception
      'FAIL (CHECK 10): collection is [%], expected [core-001 x1, core-002 x1] — the launch grant missed a starter card or granted a Token',
      coalesce(owned, '(empty)');
  end if;

  -- "Every collection change writes `collection` and `collection_grants` in one transaction":
  -- the ledger must mirror the balance, not trail it.
  select string_agg(format('%s %s %s', card_id, delta, reason), ', ' order by card_id) into grants
    from public.collection_grants where profile_id = caller;
  if grants is distinct from 'core-001 1 launch, core-002 1 launch' then
    raise exception 'FAIL (CHECK 10): collection_grants is [%], expected one launch grant of 1 per card',
      coalesce(grants, '(empty)');
  end if;

  select count(*) into n from public.invite_codes where code_hash = 'hash-good' and uses = 1;
  if n <> 1 then
    raise exception 'FAIL (CHECK 10): hash-good was not consumed exactly once';
  end if;
  if exists (select 1 from public.code_attempts where profile_id = caller and succeeded) = false then
    raise exception 'FAIL (CHECK 10): the successful redemption was not logged in code_attempts';
  end if;

  raise notice 'OK (CHECK 10): profile 1 active, 2 cards owned, 2 launch grants, 1 code use spent';
end $$;

\echo '=== CHECK 11: collection_grants is append-only (must raise) ==='
do $$
declare
  rows_to_touch bigint;
begin
  -- Vacuity guard: an UPDATE that matches no row fires no row trigger and raises nothing,
  -- which would report the guard as missing rather than as untested. CHECK 10 wrote two
  -- launch grants; assert they are there before trying to edit one.
  select count(*) into rows_to_touch from public.collection_grants where profile_id is not null;
  if rows_to_touch = 0 then
    raise exception 'FAIL (CHECK 11): no collection_grants rows to update — nothing was tested';
  end if;

  update public.collection_grants set delta = 99 where profile_id is not null;
  raise exception 'FAIL (CHECK 11): collection_grants accepted an UPDATE of % row(s)', rows_to_touch;
exception when others then
  if sqlerrm like 'FAIL%' then raise; end if;
  if sqlerrm not like 'append-only table public.collection_grants may not be updated%' then
    raise exception
      'FAIL (CHECK 11): the UPDATE failed with "%" (%), not app.deny_row_mutation — something other than the append-only guard refused it',
      sqlerrm, sqlstate;
  end if;
  raise notice 'OK (CHECK 11): collection_grants refused the UPDATE — %', sqlerrm;
end $$;

\echo '=== CHECK 12: authenticated can read its own row and not the other profile ==='
-- `set local role` only takes effect inside a transaction block. Issued outside one — as this
-- check used to — Postgres answers `WARNING: SET LOCAL can only be used in transaction blocks`,
-- keeps the superuser session, which BYPASSES RLS, and every count below comes back as the whole
-- table while the check still passes. So the probes run inside an explicit transaction, the way
-- 02_rls_as_client.sql does it, and the block at the end raises on a wrong answer instead of
-- leaving a human to notice a number.
--
-- The transaction also seeds one collection row owned by the *other* profile, so "its own rows
-- only" has something to hide (profile 2 is pending here and owns nothing, which would make the
-- collection half of this check vacuous). The rollback puts the database back as CHECK 11 left it.
begin;

insert into public.collection (profile_id, card_id, quantity)
values ('22222222-2222-2222-2222-222222222222', 'core-002', 1);

-- Read as superuser, with RLS bypassed: the totals the client's answers are measured against.
do $$
begin
  perform set_config('check12.profiles_total',
                     (select count(*) from public.profiles)::text, true);
  perform set_config('check12.cards_total',
                     (select count(*) from public.cards)::text, true);
  perform set_config('check12.collection_own',
                     (select count(*) from public.collection
                       where profile_id = '11111111-1111-1111-1111-111111111111')::text, true);
  perform set_config('check12.collection_other',
                     (select count(*) from public.collection
                       where profile_id <> '11111111-1111-1111-1111-111111111111')::text, true);
end $$;

set local role authenticated;
select set_config('request.jwt.claim.sub', '11111111-1111-1111-1111-111111111111', true);

select count(*) as own_profile_rows from public.profiles;
select count(*) as own_collection_rows from public.collection;
select count(*) as visible_cards from public.cards;

do $$
declare
  caller constant uuid := '11111111-1111-1111-1111-111111111111';
  -- Read with `missing_ok`, defaulting to -1: the totals above are transaction-scoped too, so if
  -- this check ever loses its transaction again the guards below report *that* rather than dying
  -- in a cast.
  profiles_total   bigint := coalesce(nullif(current_setting('check12.profiles_total', true), ''), '-1')::bigint;
  cards_total      bigint := coalesce(nullif(current_setting('check12.cards_total', true), ''), '-1')::bigint;
  collection_own   bigint := coalesce(nullif(current_setting('check12.collection_own', true), ''), '-1')::bigint;
  collection_other bigint := coalesce(nullif(current_setting('check12.collection_other', true), ''), '-1')::bigint;
  seen_profiles    bigint;
  seen_collection  bigint;
  seen_cards       bigint;
  seen_id          uuid;
begin
  -- Without this the whole check is theatre: a superuser reads every row and passes.
  if current_user <> 'authenticated' then
    raise exception 'FAIL (CHECK 12): running as %, not authenticated — SET LOCAL did not take',
      current_user;
  end if;
  if profiles_total < 2 or collection_other < 1 then
    raise exception 'FAIL (CHECK 12): nothing to hide (% profiles, % foreign collection rows)',
      profiles_total, collection_other;
  end if;

  select count(*) into seen_profiles from public.profiles;
  select id into seen_id from public.profiles limit 1;
  select count(*) into seen_collection from public.collection;
  select count(*) into seen_cards from public.cards;

  -- SPEC §9.1: a profile may read a projection of its own row and of nobody else's.
  if seen_profiles <> 1 or seen_id is distinct from caller then
    raise exception 'FAIL (CHECK 12): profiles showed % of % rows (first %), expected only %',
      seen_profiles, profiles_total, seen_id, caller;
  end if;
  if seen_collection <> collection_own then
    raise exception 'FAIL (CHECK 12): collection showed % rows, expected % own (% foreign)',
      seen_collection, collection_own, collection_other;
  end if;
  -- §9.4: the catalog is public to a logged-in user, so hiding it would be a failure too.
  if seen_cards <> cards_total then
    raise exception 'FAIL (CHECK 12): cards showed % of % rows; the catalog is readable',
      seen_cards, cards_total;
  end if;

  raise notice 'OK: 1 own profile row, % own collection rows, % cards, % foreign rows hidden',
    seen_collection, seen_cards, collection_other;
end $$;

reset role;
rollback;

\echo '=== CHECK 13: authenticated cannot read invite_codes or matches ==='
-- Same shape as CHECK 12: an explicit transaction so SET LOCAL ROLE really takes, and a
-- current_user guard so a superuser session cannot pass this by reading everything and
-- being refused nothing. The old version answered an unexpected error ("OK-ish") with a
-- notice, so a dropped table or a renamed schema passed.
begin;
set local role authenticated;
select set_config('request.jwt.claim.sub', '11111111-1111-1111-1111-111111111111', true);
do $$
declare
  forbidden constant text[] := array['invite_codes', 'matches'];
  t text;
  n bigint;
begin
  if current_user <> 'authenticated' then
    raise exception 'FAIL (CHECK 13): running as %, not authenticated — SET LOCAL did not take',
      current_user;
  end if;

  foreach t in array forbidden loop
    begin
      execute format('select count(*) from public.%I', t) into n;
      raise exception 'FAIL (CHECK 13): authenticated read public.% (% rows)', t, n;
    exception
      when insufficient_privilege then
        raise notice 'OK (CHECK 13): public.% refused to authenticated', t;
      when others then
        if sqlerrm like 'FAIL%' then raise; end if;
        raise exception
          'FAIL (CHECK 13): public.% raised "%" (%), not insufficient_privilege — the table is not being refused, it is missing or broken',
          t, sqlerrm, sqlstate;
    end;
  end loop;
end $$;
reset role;
rollback;

\echo '=== CHECK 14: L4 — the unique index refuses a card in two decks ==='
-- BUILD M6-T3 acceptance: "the unique index rejects a duplicate across decks even when the
-- application check is bypassed (raw SQL test)". This is that test — the inserts go straight
-- at the table, never through app.save_loadout.
do $$
declare
  redeemed text;
begin
  -- Profile 2 still has an unverified email, so this must be refused. Asserted rather than
  -- printed: an unexpected 'ok' here would mean CHECK 9's rejection path had stopped working.
  redeemed := app.redeem_invite_code('22222222-2222-2222-2222-222222222222', 'hash-good', 'ip-b');
  if redeemed <> 'email_unverified' then
    raise exception 'FAIL (CHECK 14): profile 2 has no verified email but redeem returned "%"',
      redeemed;
  end if;
  raise notice 'OK (CHECK 14 preflight): profile 2 still needs a verified email';
end $$;
insert into public.loadouts (profile_id, catalog_version)
values ('11111111-1111-1111-1111-111111111111', 'core-1');
insert into public.loadout_decks (profile_id, slot, name) values
  ('11111111-1111-1111-1111-111111111111', 1, 'One'),
  ('11111111-1111-1111-1111-111111111111', 2, 'Two');
insert into public.loadout_deck_cards (profile_id, slot, card_id, count)
values ('11111111-1111-1111-1111-111111111111', 1, 'core-001', 1);
do $$
declare
  seeded     bigint;
  refused_by text;
begin
  -- Vacuity guard: without the deck-1 row above there is no duplicate to make.
  select count(*) into seeded from public.loadout_deck_cards
   where profile_id = '11111111-1111-1111-1111-111111111111'
     and slot = 1 and card_id = 'core-001';
  if seeded <> 1 then
    raise exception 'FAIL (CHECK 14): the deck-1 row was not seeded; there is no duplicate to refuse';
  end if;

  insert into public.loadout_deck_cards (profile_id, slot, card_id, count)
  values ('11111111-1111-1111-1111-111111111111', 2, 'core-001', 1);
  raise exception 'FAIL (CHECK 14): L4 index allowed the same card in two decks';
exception
  when unique_violation then
    get stacked diagnostics refused_by = constraint_name;
    -- The two rows differ in `slot`, so the (profile_id, slot, card_id) primary key cannot be
    -- what refused this. Naming the index makes that explicit: if `slot` were ever added to
    -- loadout_card_unique, the insert would succeed and the FAIL above would fire instead.
    if refused_by is distinct from 'loadout_card_unique' then
      raise exception
        'FAIL (CHECK 14): the cross-deck duplicate was refused by "%", not loadout_card_unique',
        refused_by;
    end if;
    raise notice 'OK (CHECK 14): loadout_card_unique refused the cross-deck duplicate';
  when others then
    -- The old version raised a `notice` here, so a foreign-key violation, a missing table or a
    -- check constraint printed "UNEXPECTED" and the check still passed.
    if sqlerrm like 'FAIL%' then raise; end if;
    raise exception
      'FAIL (CHECK 14): the cross-deck insert raised "%" (%), not unique_violation — L4 was not what refused it',
      sqlerrm, sqlstate;
end $$;

\echo '=== CHECK 15 (R105): stale catalog version says "update required" ==='
do $$
begin
  perform app.assert_catalog_version('core-999');
  raise exception 'FAIL (CHECK 15): a stale catalog version was accepted';
exception when others then
  if sqlerrm like 'FAIL%' then raise; end if;
  -- 0002 raises exactly 'update required' so the API layer can pass it through (BUILD M6-T2).
  -- Accepting any error at all would have let "function does not exist" pass as a rejection.
  if sqlerrm <> 'update required' then
    raise exception
      'FAIL (CHECK 15): stale catalog version raised "%" (%), expected exactly "update required"',
      sqlerrm, sqlstate;
  end if;
  raise notice 'OK (CHECK 15): %', sqlerrm;
end $$;

\echo '=== CHECK 16: matchmaking and match tables exist with their indexes ==='
select indexname from pg_indexes
 where schemaname = 'public' and tablename in ('tickets', 'matches', 'match_actions', 'results')
 order by 1;
do $$
declare
  -- The three *_key indexes are load-bearing behavior, not tuning: nonce dedupe (SPEC §9.3),
  -- one open match per room code (R110) and one queued ticket per profile (§9.5) are all
  -- "unique index or it does not happen". Losing UNIQUE would leave the name in place and the
  -- rule gone, so uniqueness is asserted per index, not just existence.
  expected constant text[][] := array[
    ['match_actions_nonce_key',       'unique'],
    ['match_actions_pkey',            'unique'],
    ['matches_pkey',                  'unique'],
    ['matches_room_code_open_key',    'unique'],
    ['results_pkey',                  'unique'],
    ['tickets_pkey',                  'unique'],
    ['tickets_profile_queued_key',    'unique'],
    ['matches_p1_profile_id_idx',     'any'],
    ['matches_p2_profile_id_idx',     'any'],
    ['matches_status_ceiling_at_idx', 'any'],
    ['matches_status_created_at_idx', 'any'],
    ['results_p1_ended_at_idx',       'any'],
    ['results_p2_ended_at_idx',       'any'],
    ['tickets_pairing_scan_idx',      'any']];
  idx_name text;
  idx_kind text;
  is_unique boolean;
  i int;
begin
  for i in 1 .. array_length(expected, 1) loop
    idx_name := expected[i][1];
    idx_kind := expected[i][2];
    select x.indisunique into is_unique
      from pg_index x
      join pg_class ic on ic.oid = x.indexrelid
      join pg_namespace n on n.oid = ic.relnamespace
     where n.nspname = 'public' and ic.relname = idx_name;
    if not found then
      raise exception 'FAIL (CHECK 16): index public.% is missing', idx_name;
    end if;
    if idx_kind = 'unique' and not is_unique then
      raise exception
        'FAIL (CHECK 16): index public.% exists but is not UNIQUE — the rule it enforces is gone',
        idx_name;
    end if;
  end loop;
  raise notice 'OK (CHECK 16): all % match/matchmaking indexes present', array_length(expected, 1);
end $$;

\echo '=== CHECK 17: app schema functions ==='
select p.proname, p.prosecdef as security_definer
  from pg_proc p join pg_namespace n on n.oid = p.pronamespace
 where n.nspname = 'app' order by 1;
do $$
declare
  -- Every privileged path the server drives lives here. The invariant has three parts:
  --   1. each function still exists (a rename breaks apps/server silently until runtime);
  --   2. its SECURITY DEFINER flag is what it should be — losing DEFINER breaks the RPC,
  --      gaining it turns a helper into an escalation;
  --   3. no *other* SECURITY DEFINER function has appeared in app unreviewed.
  expected constant text[][] := array[
    ['append_match_action',         'definer'],
    ['assert_catalog_version',      'definer'],
    ['claim_ticket_pair',           'definer'],
    ['create_room',                 'definer'],
    ['end_match',                   'definer'],
    ['grant_cards',                 'definer'],
    ['grant_launch_collection',     'definer'],
    -- 0050, R1310001: app.grant_launch_collection_all and app.on_catalog_version_stamped, the
    -- catalog stamp's grant (0016, R481), are gone, so the stray check below fails if either
    -- comes back as DEFINER; 29_going_live.sql asserts both are gone.
    -- 0055, R1705002: the starter grant to every active account, DEFINER like the grant it calls,
    -- and service_role's alone (31_deck_size.sql asserts a client cannot call it).
    ['grant_starter_cards_to_active', 'definer'],
    ['handle_new_user',             'definer'],
    ['join_room',                   'definer'],
    ['live_matches',                'definer'],
    ['on_profile_activated',        'definer'],
    ['profile_is_active',           'definer'],
    ['reap_stale_matches',          'definer'],
    ['redeem_invite_code',          'definer'],
    ['resolve_deck',                'definer'],
    ['save_loadout',                'definer'],
    ['start_match',                 'definer'],
    -- 0007: the one write path for a saved deck and for a saved trio (R250, R252), DEFINER
    -- like every other write path here: they run as the owner whoever calls them, and only
    -- service_role may call them (02 CHECK 4 asserts a client cannot).
    ['upsert_deck',                 'definer'],
    ['upsert_trio',                 'definer'],
    -- 0011: the one write path for tutorial progress (R320), DEFINER for the same reason, and
    -- service_role's alone (02's R320 block asserts a client cannot call it).
    ['merge_tutorial_progress',     'definer'],
    -- 0018: the one write path for a player's settings (R633, R634), DEFINER and service_role's
    -- alone for the same reason (02's R633 block asserts a client cannot call it).
    ['merge_player_settings',       'definer'],
    -- 0028, R1434: the BEFORE INSERT trigger that names a new profile the lowest free Player#n,
    -- DEFINER for handle_new_user's reason: it reads every profile's username whichever role
    -- inserts the row (15 CHECK 8 asserts no client role may attach it to a table of its own).
    ['assign_default_username',     'definer'],
    -- 0032, R1452: the two triggers that stand in for the foreign key to auth.users a bot's
    -- profile cannot have, DEFINER for handle_new_user's reason: the role deleting from auth.users
    -- does not own public.profiles, and the role inserting a profile need not read auth.users.
    ['forget_auth_user',            'definer'],
    ['profile_needs_auth_user',     'definer'],
    ['catalog_version',             'invoker'],
    ['current_profile_id',          'invoker'],
    ['deny_row_mutation',           'invoker'],
    ['set_updated_at',              'invoker'],
    ['setting',                     'invoker']];
  fn_name  text;
  fn_kind  text;
  secdef   boolean;
  cfg      text[];
  known    text[] := '{}'::text[];
  stray    text;
  i int;
begin
  for i in 1 .. array_length(expected, 1) loop
    fn_name := expected[i][1];
    fn_kind := expected[i][2];
    known := known || fn_name;

    select p.prosecdef, p.proconfig into secdef, cfg
      from pg_proc p join pg_namespace n on n.oid = p.pronamespace
     where n.nspname = 'app' and p.proname = fn_name;
    if not found then
      raise exception 'FAIL (CHECK 17): app.% is missing', fn_name;
    end if;
    if fn_kind = 'definer' and not secdef then
      raise exception
        'FAIL (CHECK 17): app.% lost SECURITY DEFINER — it runs as the caller and its writes will be refused',
        fn_name;
    end if;
    if fn_kind = 'invoker' and secdef then
      raise exception
        'FAIL (CHECK 17): app.% gained SECURITY DEFINER — a helper that runs with the owner is an escalation',
        fn_name;
    end if;
    -- A SECURITY DEFINER function without a pinned search_path is hijackable by any role
    -- that can create objects in a schema it resolves through. Every app.* function pins
    -- `search_path = ''` today; this keeps the next one honest.
    if not coalesce(cfg, '{}'::text[]) @> array['search_path=""'] then
      raise exception
        'FAIL (CHECK 17): app.% does not pin search_path (proconfig = %)',
        fn_name, coalesce(array_to_string(cfg, ','), '(none)');
    end if;
  end loop;

  select string_agg(p.proname, ', ' order by p.proname) into stray
    from pg_proc p join pg_namespace n on n.oid = p.pronamespace
   where n.nspname = 'app' and p.prosecdef and p.proname <> all (known);
  if stray is not null then
    raise exception
      'FAIL (CHECK 17): unreviewed SECURITY DEFINER function(s) in app: % — add them here with the reason they need the owner',
      stray;
  end if;

  raise notice 'OK (CHECK 17): % app functions, flags and pinned search_path as expected',
    array_length(expected, 1);
end $$;

\echo '=== CHECK 18 (R278): the cards tag check admits every catalog tag, Jlockeed, Book, Pancake, AI, Plague, Catalyst, Prime, Acclaimed, Wincon, Potion, Terminus, Quest, Otherworldly, Other-Origins, Jelly Bean, Drone Core, Weapon, War Doctrine, Swarm and Legacy included, and refuses any other ==='
\echo '### R1411: cards_tags_check admits Wincon and a Meditative row as seed-catalog writes it ###'
\echo '### R1178000: cards_tags_check admits Potion and a Meditative+ row as seed-catalog writes it ###'
\echo '### R1837000: cards_tags_check admits the five tags #1837 puts on shipped cards, rows as seed-catalog writes them ###'
\echo '### R1857000: cards_tags_check admits the four Stinkyscape tags and rows of the set as seed-catalog writes them ###'
\echo '### R2511000: cards_tags_check admits Legacy and a Training Grounds Legacy card as seed-catalog writes it ###'
-- 0002's cards_tags_check had no 'Jlockeed', so `db:seed-catalog` failed on #13 and #14; 0010
-- re-adds the check with it, 0015 with patch v0.2.0's Book, Pancake and AI (B2.4), 0020 with the
-- mechanics patch's Plague, 0026 with patch v0.2.Y's Catalyst, Prime and Acclaimed, 0030 with
-- the Meditative set's Wincon (R1411), 0043 with the Meditative+ set's Potion (R1178000), 0058
-- with #1837's Terminus, Quest, Otherworldly, Other-Origins and Jelly Bean (R1837000), and 0065
-- with the Stinkyscape set's Drone Core, Weapon, War Doctrine and Swarm (R1857000), and 0069 with
-- the Training Grounds Legacy cards' Legacy (R2511000), and 0070 with #2521's Resource and
-- Faction, and 0083 with #3038's Starship (R3038000), and 0087 with #3535's Mindustrial, Mineral,
-- Automa-α, Automa-β, Automa-γ, Automa-δ, Automa-Ω, Automa-ζ and Paperclip (R3535002). Each probe
-- row is removed before the next, and each probe runs in a block of its own, so later checks see
-- only the cards CHECK 10 seeded.
do $$
declare
  -- The `Tag` union in crates/engine/src/wire/catalog_types.rs, in its order.
  catalog_tags constant text[] := array[
    'Human', 'Felinor', 'KY', 'CN', 'Fruit', 'Call to Chaos', 'Quickdraw', 'Jlockeed', 'Book', 'Pancake',
    'AI', 'Plague', 'Catalyst', 'Prime', 'Acclaimed', 'Wincon', 'Potion', 'Terminus', 'Quest',
    'Otherworldly', 'Other-Origins', 'Jelly Bean', 'Drone Core', 'Weapon', 'War Doctrine', 'Swarm',
    'Legacy', 'Resource', 'Faction', 'Starship', 'Mindustrial', 'Mineral', 'Automa-α', 'Automa-β',
    'Automa-γ', 'Automa-δ', 'Automa-Ω', 'Automa-ζ', 'Paperclip', 'Token'];
  tag      text;
  refused  boolean;
begin
  foreach tag in array catalog_tags loop
    begin
      insert into public.cards (id, card_index, name, set_id, type, tags, rarity, token, cost,
                                catalog_version)
      values ('check18-probe', '18', 'Check 18 probe', 'Core', 'Unit', array[tag], 'Common', false,
              '1'::jsonb, 'core-1');
    exception when check_violation then
      raise exception 'FAIL (CHECK 18): cards_tags_check refuses the catalog tag %', tag;
    end;
    delete from public.cards where id = 'check18-probe';
  end loop;

  -- All forty on one card: `<@` holds for the whole list, not only one tag at a time.
  begin
    insert into public.cards (id, card_index, name, set_id, type, tags, rarity, token, cost,
                              catalog_version)
    values ('check18-probe', '18', 'Check 18 probe', 'Core', 'Unit', catalog_tags, 'Common', false,
            '1'::jsonb, 'core-1');
  exception when check_violation then
    raise exception 'FAIL (CHECK 18): cards_tags_check refuses a card carrying all forty tags';
  end;
  delete from public.cards where id = 'check18-probe';

  -- A row as `seed-catalog` writes Meditative #8 Reach the Summit, a Wincon Spell of the
  -- Meditative set, and a token of the set's (#39.1, a Field Spell) are admitted: no check names a
  -- set, and the types and rarities are the ones every set uses.
  begin
    insert into public.cards (id, card_index, name, set_id, type, tags, rarity, token, cost,
                              catalog_version)
    values ('check18-probe', '8', 'Check 18 probe', 'Meditative', 'Spell', array['Wincon'], 'Mythic',
            false, '1'::jsonb, 'core-1'),
           ('check18-probe-token', '39.1', 'Check 18 probe token', 'Meditative', 'Field Spell',
            array['CN', 'Token'], 'Token', true, '0'::jsonb, 'core-1');
  exception when check_violation then
    raise exception 'FAIL (CHECK 18): a Meditative row was refused (R1411)';
  end;
  delete from public.cards where id in ('check18-probe', 'check18-probe-token');

  -- A row as `seed-catalog` writes Meditative+ #11 Living Mana Potion, a Potion Unit of the
  -- Meditative+ set, and a token of the set's (#16.1 Jazakus Prime, a Unit) are admitted, as the
  -- Meditative set's were.
  begin
    insert into public.cards (id, card_index, name, set_id, type, tags, rarity, token, cost,
                              catalog_version)
    values ('check18-probe', '11', 'Check 18 probe', 'Meditative+', 'Unit', array['Potion'], 'Rare',
            false, '1'::jsonb, 'core-1'),
           ('check18-probe-token', '16.1', 'Check 18 probe token', 'Meditative+', 'Unit',
            array['Human', 'Prime', 'Token'], 'Token', true, '0'::jsonb, 'core-1');
  exception when check_violation then
    raise exception 'FAIL (CHECK 18): a Meditative+ row was refused (R1178000)';
  end;
  delete from public.cards where id in ('check18-probe', 'check18-probe-token');

  -- Rows as `seed-catalog` writes Core #26 Glowy Jelly Bean, a Jelly Bean Spell, Classic #90 In Too
  -- Deep, a Quickdraw and Quest Field Spell, and Classic+ #32.1 Execute, an Other-Origins token,
  -- are admitted (R1837000).
  begin
    insert into public.cards (id, card_index, name, set_id, type, tags, rarity, token, cost,
                              catalog_version)
    values ('check18-probe', '26', 'Check 18 probe', 'Core', 'Spell', array['Jelly Bean'], 'Rare',
            false, '3'::jsonb, 'core-1'),
           ('check18-probe-quest', '90', 'Check 18 probe quest', 'Classic', 'Field Spell',
            array['Quickdraw', 'Quest'], 'Mythic', false, '1'::jsonb, 'core-1'),
           ('check18-probe-token', '32.1', 'Check 18 probe token', 'Classic+', 'Spell',
            array['Other-Origins', 'Token'], 'Token', true, '1'::jsonb, 'core-1');
  exception when check_violation then
    raise exception 'FAIL (CHECK 18): a row carrying a tag of #1837 was refused (R1837000)';
  end;
  delete from public.cards where id in ('check18-probe', 'check18-probe-quest', 'check18-probe-token');

  -- Rows as `seed-catalog` writes Stinkyscape #27 Drone Core I, a Drone Core Spell, #50 Beam v3, a
  -- Weapon Unit, #91 The Propagation's Wasp, a Swarm Unit, and #137.1 Land Doctrine, a War
  -- Doctrine token, are admitted (R1857000).
  begin
    insert into public.cards (id, card_index, name, set_id, type, tags, rarity, token, cost,
                              catalog_version)
    values ('check18-probe', '27', 'Check 18 probe', 'Stinkyscape', 'Spell', array['Drone Core'],
            'Common', false, '0'::jsonb, 'core-1'),
           ('check18-probe-quest', '50', 'Check 18 probe weapon', 'Stinkyscape', 'Unit',
            array['Weapon'], 'Rare', false, '2'::jsonb, 'core-1'),
           ('check18-probe-swarm', '91', 'Check 18 probe swarm', 'Stinkyscape', 'Unit',
            array['Swarm'], 'Common', false, '1'::jsonb, 'core-1'),
           ('check18-probe-token', '137.1', 'Check 18 probe token', 'Stinkyscape', 'Spell',
            array['War Doctrine', 'Token'], 'Token', true, '4'::jsonb, 'core-1');
  exception when check_violation then
    raise exception 'FAIL (CHECK 18): a row carrying a tag of the Stinkyscape set was refused (R1857000)';
  end;
  delete from public.cards
   where id in ('check18-probe', 'check18-probe-quest', 'check18-probe-swarm', 'check18-probe-token');

  -- A row as `seed-catalog` writes Training Grounds #41 Legacy: Speciesist, a Human Legacy Unit, is
  -- admitted (R2511000).
  begin
    insert into public.cards (id, card_index, name, set_id, type, tags, rarity, token, cost,
                              catalog_version)
    values ('check18-probe', '41', 'Legacy: Check 18 probe', 'Training Grounds', 'Unit',
            array['Human', 'Legacy'], 'Common', false, '2'::jsonb, 'core-1');
  exception when check_violation then
    raise exception 'FAIL (CHECK 18): a row carrying the Legacy tag was refused (R2511000)';
  end;
  delete from public.cards where id = 'check18-probe';

  -- A near miss is still refused: the check was widened by one name, not dropped.
  refused := false;
  begin
    insert into public.cards (id, card_index, name, set_id, type, tags, rarity, token, cost,
                              catalog_version)
    values ('check18-probe', '18', 'Check 18 probe', 'Core', 'Unit', array['Jlocked'], 'Common',
            false, '1'::jsonb, 'core-1');
  exception when check_violation then
    refused := true;
  end;
  if not refused then
    delete from public.cards where id = 'check18-probe';
    raise exception 'FAIL (CHECK 18): cards_tags_check admitted the unknown tag Jlocked';
  end if;

  if exists (select 1 from public.cards where id in ('check18-probe', 'check18-probe-token')) then
    raise exception 'FAIL (CHECK 18): the probe row was left behind';
  end if;

  raise notice 'OK (CHECK 18): all % catalog tags admitted, alone and together; an unknown tag refused',
    array_length(catalog_tags, 1);
end $$;

\echo '=== CHECK 19: every foreign key in public, but those into the catalog, has an index that leads with its columns ==='
-- A delete of a referenced row (an account, a match, a deck) looks up the rows that point at it
-- by the key's columns, as the server's reads by such a link do, and with no index leading with
-- those columns each such lookup reads the whole table (#1800, migration 0056). An index counts
-- only if it is a valid btree whose leading key columns are the foreign key's, in any order, and
-- it is whole or partial on nothing but one of the key's own columns `is not null`: a lookup by
-- the key never matches a null, so that predicate never hides a row it needs, while any other one
-- (tickets_profile_queued_key's `status = 'queued'`) does. The keys into the catalog tables
-- (`cards`, `cosmetics`) are exempt: the server never deletes a catalog row.
create temp view check19_foreign_keys as
select c.conrelid::regclass::text as tbl,
       c.conname,
       (select string_agg(a.attname, ', ' order by k.ord)
          from unnest(c.conkey) with ordinality k(attnum, ord)
          join pg_attribute a on a.attrelid = c.conrelid and a.attnum = k.attnum) as cols,
       c.confrelid::regclass::text as refs,
       exists (
         select 1
           from pg_index x
           join pg_class ic on ic.oid = x.indexrelid
           join pg_am am on am.oid = ic.relam
          where x.indrelid = c.conrelid
            and x.indisvalid
            and am.amname = 'btree'
            and x.indnkeyatts >= cardinality(c.conkey)
            and (select array_agg(k order by k)
                   from unnest((x.indkey::int2[])[0:cardinality(c.conkey) - 1]) k)
                = (select array_agg(k order by k) from unnest(c.conkey) k)
            and (x.indpred is null
                 or pg_get_expr(x.indpred, x.indrelid) in (
                      select format('(%s IS NOT NULL)', quote_ident(a.attname))
                        from pg_attribute a
                       where a.attrelid = c.conrelid and a.attnum = any (c.conkey)))) as indexed
  from pg_constraint c
  join pg_namespace n on n.oid = c.connamespace
 where c.contype = 'f'
   and n.nspname = 'public'
   and c.confrelid <> all (array['public.cards'::regclass, 'public.cosmetics'::regclass]);
select tbl, conname, cols, refs from check19_foreign_keys where not indexed order by 1, 2;
do $$
declare
  total     int;
  offenders text;
begin
  select count(*),
         string_agg(format('%s.%s (%s) -> %s', tbl, conname, cols, refs), '; ' order by tbl, conname)
           filter (where not indexed)
    into total, offenders
    from check19_foreign_keys;
  if total = 0 then
    raise exception 'FAIL (CHECK 19): no foreign key in public outside the catalog — the migrations did not apply';
  end if;
  if offenders is not null then
    raise exception
      'FAIL (CHECK 19): foreign key(s) with no index leading with their columns: % — add one in a new migration',
      offenders;
  end if;
  raise notice 'OK (CHECK 19): all % foreign keys in public outside the catalog have an index leading with their columns',
    total;
end $$;

\echo '=== CHECK 20 (#1996): the queue readout and the refile sweep read only their window, each through its own index ==='
-- GET /api/queue/population runs tickets_recent_waits and results_count_since (src/db/pg.rs) on
-- every waiting client's poll, over the last QUEUE_READOUT_WINDOW_SECONDS, and the refile sweep
-- (api/retention.rs) runs game_records_unfiled over the last GAME_RECORD_REFILE_WINDOW_SECONDS. No
-- purge shortens the three tables, so each must reach its rows through an index on its time bound
-- (migration 0062). With sequential scans priced out, each plan must read none of the three whole
-- and must use its own index. The name is what matters: without 0062 the planner still avoids a
-- Seq Scan, through an index that only filters (tickets_pairing_scan_idx on status, which holds
-- every claimed ticket ever written) or one it reads whole (results_p2_ended_at_idx). Each probe is
-- pg.rs's query with its bounds as literals, the values a call binds.
do $$
declare
  now_ms   constant bigint := (extract(epoch from clock_timestamp()) * 1000)::bigint;
  hour_ms  constant bigint := 3600 * 1000;      -- QUEUE_READOUT_WINDOW_SECONDS
  day_ms   constant bigint := 24 * 3600 * 1000; -- GAME_RECORD_REFILE_WINDOW_SECONDS
  grace_ms constant bigint := 10 * 60 * 1000;   -- GAME_RECORD_REFILE_GRACE_SECONDS
  probes    text[];
  plan_text text;
  line      text;
begin
  probes := array[
    array['tickets_claimed_at_idx', format(
      'select mode, enqueued_at, claimed_at from public.tickets
        where status = ''claimed'' and claimed_at >= to_timestamp(%s::double precision / 1000.0)
        order by claimed_at, id', now_ms - hour_ms)],
    array['results_ended_at_idx', format(
      'select count(*) as games from public.results
        where ended_at >= to_timestamp(%s::double precision / 1000.0)', now_ms - hour_ms)],
    array['matches_over_ended_at_idx', format(
      'select m.id from public.matches m
        where m.status = ''over'' and m.final_hash is not null
          and m.ended_at >= to_timestamp(%s::double precision / 1000.0)
          and m.ended_at < to_timestamp(%s::double precision / 1000.0)
          and exists (select 1 from public.results r where r.match_id = m.id)
          and exists (select 1 from public.match_actions a where a.match_id = m.id)
          and not exists (select 1 from public.game_records g where g.id = m.id::text)
        order by m.ended_at, m.id', now_ms - day_ms, now_ms - grace_ms)]];
  -- Local to this block's transaction: the files after this one plan as they always have.
  perform set_config('enable_seqscan', 'off', true);
  for i in 1 .. array_length(probes, 1) loop
    plan_text := '';
    for line in execute 'explain ' || probes[i][2] loop
      plan_text := plan_text || line || E'\n';
    end loop;
    if plan_text ~ 'Seq Scan on (tickets|results|matches)\M' then
      raise exception E'FAIL (CHECK 20): the query % serves reads its table whole:\n%', probes[i][1], plan_text;
    end if;
    if plan_text !~ ('\m' || probes[i][1] || '\M') then
      raise exception E'FAIL (CHECK 20): the query % serves does not use it:\n%', probes[i][1], plan_text;
    end if;
  end loop;
  raise notice 'OK (CHECK 20): the readout''s two queries and the refile scan each read their window through % and read no table whole',
    'tickets_claimed_at_idx, results_ended_at_idx and matches_over_ended_at_idx';
end $$;

\echo '=== CHECK 21 (#2022): the rank reads take a season''s asked rows and its Mythic Grape rows, each through an index ==='
-- own_rank, match_ranks and ladder_tiers (src/api/ranked.rs) read ranked_standings_subset (src/db/pg.rs):
-- a season's rows for the profiles shown plus every row from the Jlorious floor up (R608), on every
-- match screen, rank read and match's telemetry. Over 2,000 analysed probe rows with sequential scans
-- priced out, the Mythic range must be an Index Cond of season_ranks_season_ladder_idx (migration
-- 0064), never a Filter over the season, and the asked ids an Index Cond. Without 0064 the planner
-- walks season_ranks_profile_idx whole and filters. The probe is pg.rs's query with its binds as
-- literals: 36 is jlorious_floor(), Mythic Grape's index 4 x RANK_DIVISIONS_PER_TIER 3 x
-- RANK_PIPS_PER_DIVISION 3. R2778000: the probe reads the All Random ladder, `format` between the
-- season and the ladder in 0076's rebuilt index. Rolled back, rows and column statistics; ANALYZE's
-- in-place pg_class counts stay, and no later file plans a query.
begin;
insert into public.seasons (id, patch_version, started_at) values ('check-21', 'check-21.0', now());
insert into auth.users (id, email, email_confirmed_at)
  select gen_random_uuid(), format('check-21-%s@example.test', n), now() from generate_series(1, 2000) as n;
insert into public.season_ranks (season_id, profile_id, games, wins, losses, draws, ladder, floor,
                                 streak, peak_ladder, peak_jlorious, updated_at)
  select 'check-21', u.id, 10, 5, 5, 0, (row_number() over (order by u.id))::integer % 40, 0, 0,
         null, null, now()
    from auth.users u
   where u.email like 'check-21-%';
-- R2778000 (0076): each row on both ladders, as the split left every player of the season.
insert into public.season_ranks (season_id, profile_id, format, games, wins, losses, draws, ladder, floor,
                                 streak, peak_ladder, peak_jlorious, updated_at)
  select season_id, profile_id, 'random', games, wins, losses, draws, ladder, floor, streak,
         peak_ladder, peak_jlorious, updated_at
    from public.season_ranks
   where season_id = 'check-21';
analyze public.season_ranks;
do $$
declare
  asked     text;
  probe     text;
  plan_text text := '';
  line      text;
begin
  select string_agg(profile_id::text, ',') into asked
    from (select profile_id from public.season_ranks
           where season_id = 'check-21' and format = 'constructed' and ladder < 36
           order by profile_id limit 2) as two;
  probe := format(
    'select r.season_id, r.profile_id, r.format, r.games, r.wins, r.losses, r.draws, r.ladder,
            r.floor, r.streak, r.peak_ladder, r.peak_jlorious, r.updated_at,
            coalesce(b.rating, case when r.format = ''random'' then p.random_rating else p.rating end)
              as rating,
            p.kind
       from public.season_ranks r
       join public.profiles p on p.id = r.profile_id
       left join public.bot_ratings b
         on p.kind = ''bot'' and b.bot_id = p.id::text and b.format = r.format
      where r.season_id = ''check-21''::text and r.format = ''random''::text
        and (r.profile_id = any(''{%s}''::uuid[]) or r.ladder >= 36)
      order by r.profile_id', asked);
  -- Local to this file's transaction, which the rollback below ends.
  perform set_config('enable_seqscan', 'off', true);
  for line in execute 'explain ' || probe loop
    plan_text := plan_text || line || E'\n';
  end loop;
  if plan_text ~ 'Seq Scan on season_ranks\M' then
    raise exception E'FAIL (CHECK 21): the rank read reads season_ranks whole:\n%', plan_text;
  end if;
  if plan_text !~ '\mseason_ranks_season_ladder_idx\M'
     or plan_text !~ 'Index Cond: [^\n]*ladder >= 36'
     or plan_text ~ 'Filter: [^\n]*ladder >= 36' then
    raise exception E'FAIL (CHECK 21): the rank read filters its season for the Mythic rows instead of reading their range through season_ranks_season_ladder_idx:\n%', plan_text;
  end if;
  if plan_text !~ 'Index Cond: [^\n]*profile_id = ANY' then
    raise exception E'FAIL (CHECK 21): the rank read does not take the asked rows through an index:\n%', plan_text;
  end if;
  raise notice 'OK (CHECK 21): the rank read takes its asked rows through an index and its Mythic rows as a range of season_ranks_season_ladder_idx, and reads no season whole';
end $$;
rollback;

\echo '=== CHECK 22 (#4012): no public view bypasses RLS — every view runs as its caller ==='
-- A Postgres view runs with its owner's rights unless it is created with
-- security_invoker = true, so a view over an RLS table shows every row to a
-- caller whose policies should filter them. Materialized views cannot carry
-- RLS or security_invoker at all and are CHECK 23's alone.
select c.relname as view_without_security_invoker
  from pg_class c join pg_namespace n on n.oid = c.relnamespace
 where n.nspname = 'public' and c.relkind = 'v'
   and not (coalesce(c.reloptions, '{}') @> array['security_invoker=true'])
 order by 1;
do $$
declare
  offenders text;
begin
  -- Failing case first: a plain view must be named.
  create view public.check22_probe as select 1;
  select string_agg(c.relname, ', ' order by c.relname) into offenders
    from pg_class c join pg_namespace n on n.oid = c.relnamespace
   where n.nspname = 'public' and c.relkind = 'v'
     and not (coalesce(c.reloptions, '{}') @> array['security_invoker=true']);
  if offenders is null or position('check22_probe' in offenders) = 0 then
    raise exception
      'FAIL (CHECK 22): the probe view was not detected (saw %) — the detector matches nothing',
      coalesce(offenders, '(none)');
  end if;

  -- A security_invoker view must be ignored.
  create view public.check22_ok with (security_invoker = true) as select 1;
  select string_agg(c.relname, ', ' order by c.relname) into offenders
    from pg_class c join pg_namespace n on n.oid = c.relnamespace
   where n.nspname = 'public' and c.relkind = 'v'
     and not (coalesce(c.reloptions, '{}') @> array['security_invoker=true']);
  if offenders is distinct from 'check22_probe' then
    raise exception
      'FAIL (CHECK 22): the security_invoker probe leaked or vanished (saw %)',
      coalesce(offenders, '(none)');
  end if;
  drop view public.check22_ok;
  drop view public.check22_probe;

  select string_agg(c.relname, ', ' order by c.relname) into offenders
    from pg_class c join pg_namespace n on n.oid = c.relnamespace
   where n.nspname = 'public' and c.relkind = 'v'
     and not (coalesce(c.reloptions, '{}') @> array['security_invoker=true']);
  if offenders is not null then
    raise exception
      'FAIL (CHECK 22): public view(s) without security_invoker = true: % — a view runs as its owner past RLS',
      offenders;
  end if;

  raise notice 'OK (CHECK 22): no public view without security_invoker = true';
end $$;

\echo '=== CHECK 23 (#4012): no public materialized view ==='
-- A materialized view can carry neither RLS nor security_invoker, so any one
-- in public is a snapshot of rows no policy filters. None exists today; one
-- appearing is the failure.
select c.relname as public_matview
  from pg_class c join pg_namespace n on n.oid = c.relnamespace
 where n.nspname = 'public' and c.relkind = 'm'
 order by 1;
do $$
declare
  offenders text;
begin
  -- Failing case first: a materialized view must be named.
  create materialized view public.check23_probe as select 1;
  select string_agg(c.relname, ', ' order by c.relname) into offenders
    from pg_class c join pg_namespace n on n.oid = c.relnamespace
   where n.nspname = 'public' and c.relkind = 'm';
  if offenders is distinct from 'check23_probe' then
    raise exception
      'FAIL (CHECK 23): the probe materialized view was not detected (saw %) — the detector matches nothing',
      coalesce(offenders, '(none)');
  end if;
  drop materialized view public.check23_probe;

  select string_agg(c.relname, ', ' order by c.relname) into offenders
    from pg_class c join pg_namespace n on n.oid = c.relnamespace
   where n.nspname = 'public' and c.relkind = 'm';
  if offenders is not null then
    raise exception
      'FAIL (CHECK 23): public materialized view(s): % — a materialized view can carry neither RLS nor security_invoker',
      offenders;
  end if;

  raise notice 'OK (CHECK 23): no public materialized view';
end $$;

\echo '=== CHECK 24 (#4012): every SECURITY DEFINER function in public or app pins its search_path ==='
-- A SECURITY DEFINER function without a pinned search_path is hijackable by
-- any role that can create objects in a schema it resolves through. This
-- reads the catalog where CHECK 17 follows a hand-maintained list, so a stale
-- list cannot hide a function; the predicate is CHECK 17's (line 920).
select p.proname as definer_without_search_path
  from pg_proc p join pg_namespace n on n.oid = p.pronamespace
 where n.nspname in ('public', 'app') and p.prosecdef
   and not (coalesce(p.proconfig, '{}'::text[]) @> array['search_path=""'])
 order by 1;
do $$
declare
  offenders text;
begin
  -- Failing case first: a DEFINER function with no SET clause must be named.
  create function app.check24_probe() returns int
    language sql security definer
    as $fn$ select 1 $fn$;
  select string_agg(p.proname, ', ' order by p.proname) into offenders
    from pg_proc p join pg_namespace n on n.oid = p.pronamespace
   where n.nspname in ('public', 'app') and p.prosecdef
     and not (coalesce(p.proconfig, '{}'::text[]) @> array['search_path=""']);
  if offenders is null or position('check24_probe' in offenders) = 0 then
    raise exception
      'FAIL (CHECK 24): the probe function was not detected (saw %) — the detector matches nothing',
      coalesce(offenders, '(none)');
  end if;
  drop function app.check24_probe();

  select string_agg(n.nspname || '.' || p.proname, ', ' order by n.nspname, p.proname)
    into offenders
    from pg_proc p join pg_namespace n on n.oid = p.pronamespace
   where n.nspname in ('public', 'app') and p.prosecdef
     and not (coalesce(p.proconfig, '{}'::text[]) @> array['search_path=""']);
  if offenders is not null then
    raise exception
      'FAIL (CHECK 24): SECURITY DEFINER function(s) without a pinned search_path: %',
      offenders;
  end if;

  raise notice 'OK (CHECK 24): every SECURITY DEFINER function in public and app pins its search_path';
end $$;

\echo '=== CHECK 25 (#4012): no SECURITY DEFINER function in public or app is executable by a client role outside the allowlist ==='
-- A function defaults to EXECUTE for PUBLIC, so a DEFINER function no
-- migration ever revoked stays callable by anon and authenticated: a client
-- path into privileged code. Read the ACLs directly as CHECK 6 does — a null
-- proacl IS the default PUBLIC grant, via acldefault. Scoped to public/app:
-- auth.uid() in auth is granted to clients by design.
select p.proname as function_name,
       coalesce(g.rolname, 'PUBLIC') as grantee
  from pg_proc p
  join pg_namespace n on n.oid = p.pronamespace
  cross join lateral aclexplode(coalesce(p.proacl, acldefault('f', p.proowner))) a
  left join pg_roles g on g.oid = a.grantee
 where n.nspname in ('public', 'app')
   and p.prosecdef
   and coalesce(g.rolname, 'PUBLIC') in ('anon', 'authenticated', 'PUBLIC')
   and not (n.nspname = 'app' and p.proname = 'profile_is_active' and p.pronargs = 0
            and coalesce(g.rolname, 'PUBLIC') = 'authenticated')
 order by 1, 2;
do $$
declare
  offenders text;
  measured  bigint;
begin
  -- The allowlist: the zero-argument app.profile_is_active() stays callable
  -- by authenticated alone because the RLS policies of 0002/0003/0004 call
  -- it, and it is DEFINER so it reads public.profiles (0001) without
  -- recursing into policies. Any other grantee (anon, PUBLIC), any overload
  -- with arguments, and any other function still fail this check.
  -- app.current_profile_id() is also granted to authenticated but is
  -- SECURITY INVOKER, so it is out of this check's scope.
  -- No synthetic probe: the schema itself is the failing case until 0094 —
  -- app.on_profile_activated() (0002, the pending-to-active launch-grant
  -- trigger) never had PUBLIC execute revoked.
  select string_agg(format('%s.%s:%s', n.nspname, p.proname,
                           coalesce(g.rolname, 'PUBLIC')),
                    ', ' order by n.nspname, p.proname, coalesce(g.rolname, 'PUBLIC'))
    into offenders
    from pg_proc p
    join pg_namespace n on n.oid = p.pronamespace
    cross join lateral aclexplode(coalesce(p.proacl, acldefault('f', p.proowner))) a
    left join pg_roles g on g.oid = a.grantee
   where n.nspname in ('public', 'app')
     and p.prosecdef
     and coalesce(g.rolname, 'PUBLIC') in ('anon', 'authenticated', 'PUBLIC')
     and not (n.nspname = 'app' and p.proname = 'profile_is_active' and p.pronargs = 0
              and coalesce(g.rolname, 'PUBLIC') = 'authenticated');
  if offenders is not null then
    raise exception
      'FAIL (CHECK 25): SECURITY DEFINER function(s) executable by a client role outside the allowlist: % — revoke PUBLIC execute; the server calls them as service_role',
      offenders;
  end if;

  -- Vacuity guard: "no offender" means nothing if there is no DEFINER
  -- function in scope to measure.
  select count(*) into measured
    from pg_proc p join pg_namespace n on n.oid = p.pronamespace
   where n.nspname in ('public', 'app') and p.prosecdef;
  if measured = 0 then
    raise exception
      'FAIL (CHECK 25): no SECURITY DEFINER function in public or app — the migrations did not apply, so this check measured nothing';
  end if;

  raise notice 'OK (CHECK 25): % SECURITY DEFINER function(s) in public/app, none executable by a client role outside the allowlist', measured;
end $$;

\echo '=== CHECK 26 (#4468): the public players list walks player_stats_public_games_idx in order and sorts no row ==='
-- GET /api/stats/players, open to anyone, reads player_stats_list_public (src/db/pg.rs): unbanned players'
-- public rows, most games first (R654, R3911000), optionally matched on the username key (R1436). Over
-- 2,000 analysed public rows of several KB of stats, some private and one banned, with sequential scans
-- priced out, the default page and a search must walk player_stats_public_games_idx (migration 0098) and
-- sort nothing; without 0098 the planner reads player_stats_pkey whole and sorts every public row. The
-- probes are pg.rs's query with its binds as literals (no search, then a username key; `limit`
-- PLAYER_STATS_PAGE_LIMIT, `offset` 0): an edit of pg.rs's `order by` edits this probe too. Rolled back, as CHECK 21.
begin;
insert into auth.users (id, email, email_confirmed_at)
  select gen_random_uuid(), format('check-26-%s@example.test', n), now() from generate_series(1, 2000) as n;
insert into public.player_stats (profile_id, stats, is_private, updated_at)
  select u.id,
         jsonb_build_object('games', (row_number() over (order by u.id)) % 500, 'cards', repeat('x', 4000)),
         (row_number() over (order by u.id)) % 50 = 0,
         now() - make_interval(mins => (row_number() over (order by u.id))::integer)
    from auth.users u
   where u.email like 'check-26-%';
update public.profiles set status = 'banned'
 where id = (select id from auth.users where email = 'check-26-1@example.test');
analyze public.player_stats;
analyze public.profiles;
do $$
declare
  probes    text[];
  plan_text text;
  line      text;
begin
  probes := array[array['the default page', 'null'], array['a search', '''player#12''']];
  -- Local to this file's transaction, which the rollback below ends.
  perform set_config('enable_seqscan', 'off', true);
  for i in 1 .. array_length(probes, 1) loop
    plan_text := '';
    for line in execute format(
      'explain select ps.profile_id, p.username_base, p.username_tag, ps.stats, ps.updated_at
         from public.player_stats ps
         join public.profiles p on p.id = ps.profile_id
        where ps.is_private = false
          and p.status <> ''banned''
          and (%1$s::text is null
               or position(%1$s::text in (p.username_key || coalesce(''#'' || p.username_tag::text, ''''))) > 0)
        order by
          case when (ps.stats->>''games'') ~ ''^[0-9]+$'' then (ps.stats->>''games'')::bigint else 0 end desc,
          ps.updated_at desc,
          ps.profile_id asc
        limit 50::int offset 0::int', probes[i][2]) loop
      plan_text := plan_text || line || E'\n';
    end loop;
    if plan_text ~ 'Seq Scan on player_stats\M' then
      raise exception E'FAIL (CHECK 26): % of player_stats_list_public reads player_stats whole:\n%', probes[i][1], plan_text;
    end if;
    if plan_text !~ '\mplayer_stats_public_games_idx\M' or plan_text ~ '\mSort\M' then
      raise exception E'FAIL (CHECK 26): % of player_stats_list_public sorts player_stats instead of walking player_stats_public_games_idx:\n%', probes[i][1], plan_text;
    end if;
  end loop;
  raise notice 'OK (CHECK 26): the players list''s default page and its search walk player_stats_public_games_idx in order, sort no row and read no player_stats whole';
end $$;
rollback;

\echo '=== CHECK 27 (#4468): the daily pay sums read one day of one account''s game rows, each through its own index ==='
-- A paid game's payout (economy/rewards.rs) and the daily-cap reads (api/results.rs, duos/mod.rs) sum an
-- account's `game` rows of one UTC day: game_jlore_totals over jlore_grants and fleet_get, which GET
-- /api/economy also runs, over fleet_track_grants (src/economy/store.rs; R1483000). Over 20 accounts of
-- 2,000 analysed rows each, spread over 400 days, with sequential scans priced out, both day bounds must
-- be an Index Cond of jlore_grants_profile_reason_at_idx and fleet_track_grants_profile_reason_at_idx
-- (migration 0098), never a Filter; without 0098 each sum reads the account's rows through (profile_id,
-- id) (0045, 0086) and filters the day. The probes are store.rs's queries with their binds as literals.
-- Rolled back, as CHECK 21.
begin;
insert into auth.users (id, email, email_confirmed_at)
  select gen_random_uuid(), format('check-27-%s@example.test', n), now() from generate_series(1, 20) as n;
insert into public.jlore_grants (profile_id, delta, reason, ref, at)
  select u.id, 10, case when n % 5 = 0 then 'quest' else 'game' end, format('check-27-%s', n),
         now() - make_interval(hours => n * 5)
    from auth.users u cross join generate_series(1, 2000) as n
   where u.email like 'check-27-%';
insert into public.fleet_track_grants (profile_id, xp, reason, ref, at)
  select u.id, 10, 'game', format('check-27-%s', n), now() - make_interval(hours => n * 5)
    from auth.users u cross join generate_series(1, 2000) as n
   where u.email like 'check-27-%';
analyze public.jlore_grants;
analyze public.fleet_track_grants;
do $$
declare
  day_ms    constant bigint := 24 * 3600 * 1000;
  from_ms   constant bigint :=
    (extract(epoch from date_trunc('day', clock_timestamp() at time zone 'utc')) * 1000)::bigint;
  until_ms  constant bigint := from_ms + day_ms;
  profile   uuid;
  probes    text[];
  plan_text text;
  line      text;
begin
  select id into profile from auth.users where email = 'check-27-1@example.test';
  probes := array[
    array['jlore_grants_profile_reason_at_idx', 'game_jlore_totals', format(
      'select coalesce(sum(delta), 0)::bigint,
              coalesce(sum(delta) filter (where ref = %L::text), 0)::bigint
         from public.jlore_grants
        where profile_id = %L::uuid and reason = ''game''::text
          and at >= to_timestamp(%s::double precision / 1000.0)
          and at < to_timestamp(%s::double precision / 1000.0)', 'check-27-1', profile, from_ms, until_ms)],
    array['fleet_track_grants_profile_reason_at_idx', 'fleet_get', format(
      'select coalesce(sum(xp), 0)::bigint from public.fleet_track_grants
        where profile_id = %L::uuid and reason = ''game''::text
          and at >= to_timestamp(%s::double precision / 1000.0)
          and at < to_timestamp(%s::double precision / 1000.0)', profile, from_ms, until_ms)]];
  -- Local to this file's transaction, which the rollback below ends.
  perform set_config('enable_seqscan', 'off', true);
  for i in 1 .. array_length(probes, 1) loop
    plan_text := '';
    for line in execute 'explain ' || probes[i][3] loop
      plan_text := plan_text || line || E'\n';
    end loop;
    if plan_text ~ 'Seq Scan on (jlore_grants|fleet_track_grants)\M' then
      raise exception E'FAIL (CHECK 27): % reads its ledger whole:\n%', probes[i][2], plan_text;
    end if;
    if plan_text !~ ('\m' || probes[i][1] || '\M')
       or plan_text !~ 'Index Cond: [^\n]*\mat >= '
       or plan_text !~ 'Index Cond: [^\n]*\mat < '
       or plan_text ~ 'Filter: [^\n]*\mat [<>]' then
      raise exception E'FAIL (CHECK 27): % filters the account''s rows by day instead of reading the day through %:\n%',
        probes[i][2], probes[i][1], plan_text;
    end if;
  end loop;
  raise notice 'OK (CHECK 27): game_jlore_totals and fleet_get read one day of one account''s game rows through % and %, and filter no day',
    'jlore_grants_profile_reason_at_idx', 'fleet_track_grants_profile_reason_at_idx';
end $$;
rollback;

\echo '=== CHECK 28 (#4510): the daily list reads its games'' portraits and the daily sweep its due games, each through an index ==='
-- daily::games::list reads the portraits of every game it lists in one query on matches_pkey
-- (db/pg.rs matches_portraits), and the hourly sweep reads its due games in two (daily_games_due): the
-- running games past their deadline through daily_games_due_idx, and the finished games whose match is
-- still live from the live matches, through matches_status_created_at_idx and daily_games_pkey. One
-- `or` over both left the finished arm (`ended_at <= $1`) without an index, so Postgres read every
-- daily game ever played, and no row is deleted. With sequential scans priced out, no plan may read
-- daily_games or matches whole, and each must name its index. Each probe is pg.rs's query with its binds
-- as literals (the portraits' ids are uuid literals: a volatile gen_random_uuid() would not be an index
-- bound). Over 3,000 analysed matches, all of them live or over, and 2,010 daily games, 2,000 of them
-- finished with their match over, ten finished with it live and ten running. An empty table lets the
-- planner read `status in ('live','over')`, which every row meets, over the key it was asked. Rolled back,
-- as CHECK 21.
begin;
insert into auth.users (id, email, email_confirmed_at)
  select gen_random_uuid(), format('check-28-%s@example.test', n), now() from generate_series(1, 2) as n;
insert into public.matches (id, status, seed, p1_profile_id, p2_profile_id, p1_deck, p2_deck,
                            catalog_version, ceiling_at, started_at, ended_at)
  select gen_random_uuid(), case when n <= 20 then 'live' else 'over' end, 'c28-' || n,
         (select id from auth.users where email = 'check-28-1@example.test'),
         (select id from auth.users where email = 'check-28-2@example.test'),
         '[]', '[]', 'core-1', now(), now(), case when n <= 20 then null else now() end
    from generate_series(1, 3000) as n;
-- Matches 1-10: a game over whose result was never written (match live); 11-20: a running game;
-- 21-2010: a game over, its match over.
insert into public.daily_games (match_id, p1_profile_id, p2_profile_id, mode, per_move_days, clock, board,
                                next_deadline_at, created_at, ended_at)
  select m.id, m.p1_profile_id, m.p2_profile_id, 'bo1', 1, '{}', '{}',
         case when n between 11 and 20 then 2000 + n end, 500 + n,
         case when n between 11 and 20 then null else 600 + n end
    from (select m.*, split_part(m.seed, '-', 2)::integer as n from public.matches m
           where m.seed like 'c28-%') m
   where n <= 2010;
analyze public.matches;
analyze public.daily_games;
do $$
declare
  probes    text[];
  plan_text text;
  line      text;
begin
  probes := array[
    array['daily_games_due_idx', 'the running games due',
      'select d.match_id, d.next_deadline_at as at from public.daily_games d
        where d.ended_at is null and d.next_deadline_at <= 1000::bigint'],
    array['matches_status_created_at_idx', 'the finished games whose match is still live',
      'select d.match_id, d.ended_at as at from public.matches m
         join public.daily_games d on d.match_id = m.id
        where m.status = ''live'' and d.ended_at <= 1000::bigint'],
    array['matches_pkey', 'the portraits read',
      'select id, p1_portrait, p2_portrait from public.matches
        where id = any(array[''00000000-0000-4000-8000-000000000001'', ''00000000-0000-4000-8000-000000000002'']::uuid[])
          and status in (''live'', ''over'')']];
  -- Local to this block's transaction: the files after this one plan as they always have.
  perform set_config('enable_seqscan', 'off', true);
  for i in 1 .. array_length(probes, 1) loop
    plan_text := '';
    for line in execute 'explain ' || probes[i][3] loop
      plan_text := plan_text || line || E'\n';
    end loop;
    if plan_text ~ 'Seq Scan on (daily_games|matches)\M' then
      raise exception E'FAIL (CHECK 28): % reads its table whole:\n%', probes[i][2], plan_text;
    end if;
    if plan_text !~ ('\m' || probes[i][1] || '\M') then
      raise exception E'FAIL (CHECK 28): % does not use %:\n%', probes[i][2], probes[i][1], plan_text;
    end if;
    if i = 2 and plan_text !~ '\mdaily_games_pkey\M' then
      raise exception E'FAIL (CHECK 28): % does not use daily_games_pkey:\n%', probes[i][2], plan_text;
    end if;
  end loop;
  raise notice 'OK (CHECK 28): the sweep''s two reads and the list''s portraits read go through % and read no table whole',
    'daily_games_due_idx, matches_status_created_at_idx with daily_games_pkey, and matches_pkey';
end $$;
rollback;

\echo '=== ALL CHECKS RAN ==='
