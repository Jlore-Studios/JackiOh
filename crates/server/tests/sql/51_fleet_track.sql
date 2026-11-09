-- The Fleet track and the Starship cosmetics, the Jlorious Starship among them (migration 0086, SPEC §14.2,
-- §14.11, §14.15; R3335001, R3335003, R2755000, R607, R640000; issue #3335). Runs after 50_match_win_rates.sql;
-- profiles 1 and 2 (11111111-…, 22222222-…) are active by then (03 activated them). The first blocks check the
-- Fleet track's two tables; the last applies the file again (/tmp/0086_fleet_track.sql, which run.sh copied
-- there) over people of its own: one who has held Jlorious, one who never has, one who owns the Starship
-- already, and a ladder bot whose row says it held Jlorious, which no bot does.
\set ON_ERROR_STOP on

-- Same rules as 01-49: every check raises on failure (`raise exception 'FAIL (…): …'`), every expected refusal is
-- matched on its sqlstate and the constraint that names it, each check that could pass vacuously carries a guard,
-- and every block that writes runs inside a transaction that is rolled back. Nothing here selects into the log:
-- the do blocks raise or say OK.

\echo '### R3335001: 0086 wrote the 35 registry rows and the Fleet track''s two tables, each read by its own account alone ###'
do $$
declare
  v_text text;
  tbl    text;
  op     text;
  who    text;
  wrong  text[] := array[]::text[];
begin
  select format('%s %s %s %s default=%s chase=%s', id, kind, name, rarity, is_default::text, chase::text)
    into v_text from public.cosmetics where id = 'starship-hull-jlorious';
  if v_text is distinct from 'starship-hull-jlorious starshipHull Jlorious Mythic default=false chase=false' then
    raise exception 'FAIL (R3335003): the Jlorious Starship''s row of public.cosmetics is [%]', coalesce(v_text, '(none)');
  end if;
  if (select count(*) from public.cosmetics
       where kind in ('starshipHull', 'starshipLivery', 'starshipTrail', 'starshipPieceSkin')
          or id in ('card-back-stinkyscape', 'table-stinkyscape', 'field-stinkyscape')) <> 35 then
    raise exception 'FAIL (R3335001): public.cosmetics does not hold the 35 rows 0086 writes';
  end if;
  if (select count(*) from public.cosmetics
       where kind in ('starshipHull', 'starshipLivery', 'starshipTrail', 'starshipPieceSkin') and is_default) <> 4
     or exists (select 1 from public.cosmetics
                 where kind in ('starshipHull', 'starshipLivery', 'starshipTrail', 'starshipPieceSkin') and chase) then
    raise exception 'FAIL (R3335001): the Starship kinds do not have one default each and no chase';
  end if;

  foreach tbl in array array['fleet_tracks', 'fleet_track_grants'] loop
    if not exists (select 1 from pg_class where oid = format('public.%I', tbl)::regclass and relrowsecurity) then
      raise exception 'FAIL (R3335001): public.% has no row level security', tbl;
    end if;
    select string_agg(format('%s %s %s', policyname, cmd, array_to_string(roles, ',')), '; ') into v_text
      from pg_policies where schemaname = 'public' and tablename = tbl;
    if v_text is distinct from format('%s_select_own SELECT authenticated', tbl) then
      raise exception 'FAIL (R3335001): the policies of % are [%], expected one select-own policy to authenticated',
        tbl, coalesce(v_text, '(none)');
    end if;
    foreach who in array array['authenticated', 'anon', 'public'] loop
      foreach op in array array['SELECT', 'INSERT', 'UPDATE', 'DELETE', 'TRUNCATE', 'REFERENCES', 'TRIGGER'] loop
        if has_table_privilege(who, format('public.%I', tbl), op) <> (who = 'authenticated' and op = 'SELECT') then
          wrong := wrong || format('%s %s %s', tbl, who, op);
        end if;
      end loop;
    end loop;
  end loop;
  if cardinality(wrong) > 0 then
    raise exception 'FAIL (R3335001): the client privileges are wrong for [%]', array_to_string(wrong, ', ');
  end if;

  select string_agg(format('%s %s %s', column_name, data_type, is_nullable), ', ' order by ordinal_position)
    into v_text from information_schema.columns
   where table_schema = 'public' and table_name = 'fleet_tracks';
  if v_text is distinct from 'profile_id uuid NO, xp bigint NO, level bigint NO, updated_at timestamp with time zone NO' then
    raise exception 'FAIL (R3335001): public.fleet_tracks has the columns [%]', coalesce(v_text, '(none)');
  end if;
  select pg_get_constraintdef(c.oid) into v_text from pg_constraint c
   where c.conrelid = 'public.fleet_track_grants'::regclass and c.conname = 'fleet_track_grants_reason_check';
  if v_text is distinct from 'CHECK ((reason = ANY (ARRAY[''game''::text, ''dev''::text])))' then
    raise exception 'FAIL (R3335001): fleet_track_grants'' reasons are [%]', coalesce(v_text, '(none)');
  end if;

  raise notice 'OK (R3335001): the 35 rows, and fleet_tracks and fleet_track_grants with RLS on, one select-own policy each and SELECT their only client privilege';
end $$;

\echo '### R3335001: a Fleet grant changes something, for a recorded game or a dev account, and its ledger is append-only ###'
begin;
do $$
declare
  p1 constant uuid := '11111111-1111-1111-1111-111111111111';
  -- [what, xp, levels, reason, dev, the check it breaks].
  refused constant text[][] := array[
    ['a grant of nothing', '0', '0', 'game', 'false', 'fleet_track_grants_change_check'],
    ['a level''s reward reason', '10', '0', 'track', 'false', 'fleet_track_grants_reason_check'],
    ['a quest''s reason', '10', '0', 'quest', 'false', 'fleet_track_grants_reason_check'],
    ['a dev grant not marked dev', '10', '0', 'dev', 'false', 'fleet_track_grants_dev_check']];
  i     int;
  v_con text;
begin
  if not exists (select 1 from public.profiles where id = p1) then
    raise exception 'FAIL (R3335001): profile 1 is missing, so every insert below would fail on its foreign key';
  end if;
  for i in 1 .. array_length(refused, 1) loop
    begin
      insert into public.fleet_track_grants (profile_id, xp, levels, reason, dev)
      values (p1, refused[i][2]::bigint, refused[i][3]::bigint, refused[i][4], refused[i][5]::boolean);
      raise exception 'FAIL (R3335001): % was taken', refused[i][1];
    exception when check_violation then
      get stacked diagnostics v_con = constraint_name;
      if v_con is distinct from refused[i][6] then
        raise exception 'FAIL (R3335001): % broke [%], expected [%]', refused[i][1], v_con, refused[i][6];
      end if;
    end;
  end loop;

  insert into public.fleet_track_grants (profile_id, xp, levels, reason, ref) values (p1, 120, 0, 'game', 'match:a');
  insert into public.fleet_track_grants (profile_id, xp, levels, reason, dev) values (p1, -120, 0, 'dev', true);
  begin
    update public.fleet_track_grants set xp = 1 where profile_id = p1;
    raise exception 'FAIL (R3335001): a Fleet ledger row was updated';
  exception when others then
    if sqlerrm not like 'append-only table public.fleet_track_grants may not be updated%' then
      raise;
    end if;
  end;

  begin
    insert into public.fleet_tracks (profile_id, xp, level) values (p1, 0, -1);
    raise exception 'FAIL (R3335001): a Fleet track below level 0 was taken';
  exception when check_violation then
    null;
  end;
  insert into public.fleet_tracks (profile_id, xp, level) values (p1, 600, 1);
  update public.fleet_tracks set xp = xp + 100 where profile_id = p1;

  raise notice 'OK (R3335001): a Fleet grant of nothing, of any reason but game and dev, or dev unmarked is refused; its ledger is append-only and its track takes no level below 0';
end $$;
rollback;

\echo '### R3335003: 0086 over the people of a database: each person who held Jlorious owns the Jlorious Starship, never a bot ###'
begin;
insert into auth.users (id, email, email_confirmed_at)
values ('e3335003-0000-4000-8000-000000000001', 'r3335003-held@example.test', now()),
       ('e3335003-0000-4000-8000-000000000002', 'r3335003-never@example.test', now()),
       ('e3335003-0000-4000-8000-000000000003', 'r3335003-owns@example.test', now());
insert into public.profiles (id, kind, status) values ('e3335003-0000-4000-8000-000000000004', 'bot', 'active');
insert into public.seasons (id, patch_version, started_at) values ('check-r3335003', 'check-r3335003.0', now());
-- Held Jlorious (#7 at best), never did, held it (#1) and owns the Starship already, and a bot whose row says #3.
insert into public.season_ranks (season_id, profile_id, games, wins, losses, draws, ladder, floor,
                                 streak, peak_ladder, peak_jlorious, updated_at)
values ('check-r3335003', 'e3335003-0000-4000-8000-000000000001', 40, 30, 10, 0, 1400, 1200, 3, 1400, 7, now()),
       ('check-r3335003', 'e3335003-0000-4000-8000-000000000002', 40, 30, 10, 0, 1400, 1200, 3, 1400, null, now()),
       ('check-r3335003', 'e3335003-0000-4000-8000-000000000003', 40, 30, 10, 0, 1400, 1200, 3, 1400, 1, now()),
       ('check-r3335003', 'e3335003-0000-4000-8000-000000000004', 40, 30, 10, 0, 1400, 1200, 3, 1400, 3, now());
insert into public.account_cosmetics (profile_id, cosmetic_id)
values ('e3335003-0000-4000-8000-000000000003', 'starship-hull-jlorious');
insert into public.cosmetic_grants (profile_id, card_id, cosmetic_id, delta, reason, ref)
values ('e3335003-0000-4000-8000-000000000003', null, 'starship-hull-jlorious', 1, 'jlorious', 'match:earlier');

do $$
begin
  if exists (select 1 from public.account_cosmetics
              where profile_id in ('e3335003-0000-4000-8000-000000000001', 'e3335003-0000-4000-8000-000000000002',
                                   'e3335003-0000-4000-8000-000000000004')) then
    raise exception 'FAIL (R3335003): a person or the bot owns an account cosmetic before 0086 runs again';
  end if;
  if (select count(*) from public.profiles
       where id in ('e3335003-0000-4000-8000-000000000001', 'e3335003-0000-4000-8000-000000000002',
                    'e3335003-0000-4000-8000-000000000003') and kind = 'human') <> 3
     or not exists (select 1 from public.profiles where id = 'e3335003-0000-4000-8000-000000000004' and kind = 'bot') then
    raise exception 'FAIL (R3335003): the four profiles are not three people and a bot';
  end if;
end $$;

-- The grant, as it runs on a database these people are signed up to, twice: the second finds every holder and
-- writes nothing.
set local client_min_messages = warning;
\i /tmp/0086_fleet_track.sql
\i /tmp/0086_fleet_track.sql
set local client_min_messages = notice;

do $$
declare
  held  constant uuid := 'e3335003-0000-4000-8000-000000000001';
  never constant uuid := 'e3335003-0000-4000-8000-000000000002';
  owns  constant uuid := 'e3335003-0000-4000-8000-000000000003';
  bot   constant uuid := 'e3335003-0000-4000-8000-000000000004';
  v_text text;
begin
  -- The person who held Jlorious owns the Starship, with one ledger row of the reason jlorious; the one who owned
  -- it already keeps their one row, and the second application wrote nothing.
  select string_agg(format('%s %s %s', profile_id, cosmetic_id, reason || coalesce(' ' || ref, '')), ', '
                    order by profile_id)
    into v_text from public.cosmetic_grants where profile_id in (held, never, owns, bot);
  if v_text is distinct from format('%1$s starship-hull-jlorious jlorious jlorious:0086, '
                                    '%2$s starship-hull-jlorious jlorious match:earlier', held, owns) then
    raise exception 'FAIL (R3335003): the cosmetic ledgers after 0086 are [%]', coalesce(v_text, '(empty)');
  end if;
  if not exists (select 1 from public.account_cosmetics where profile_id = held and cosmetic_id = 'starship-hull-jlorious') then
    raise exception 'FAIL (R3335003): the person who held Jlorious does not own the Jlorious Starship';
  end if;

  -- Never Jlorious, and a ladder bot, which is never Jlorious whatever its row says (R640000): nothing.
  if exists (select 1 from public.account_cosmetics where profile_id in (never, bot)) then
    raise exception 'FAIL (R3335003): 0086 gave the Jlorious Starship to someone who never held Jlorious, or to the bot';
  end if;

  raise notice 'OK (R3335003): 0086 gives the Jlorious Starship once to each person who held Jlorious, and to no one else';
end $$;
rollback;
