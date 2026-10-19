-- The economy's holdings and ledgers (migration 0044, SPEC §14.10, §14.11; issue #1299). Runs after
-- 24_card_totals.sql; profiles 1 and 2 are active by then (03 activated them), and core-001 is in
-- public.cards (01's CHECK 10).
\set ON_ERROR_STOP on

-- Same rules as 01-24: every check raises on failure (`raise exception 'FAIL (…): …'`), every
-- expected refusal is matched on its sqlstate, each check that could pass vacuously carries a guard,
-- and every block runs inside a transaction that is rolled back.

\echo '### 0044: a row in every economy table for two profiles, the migration re-applied twice over them ###'
begin;
insert into public.cosmetics (id, kind, name, rarity)
values ('test-foil', 'foil', 'Test foil', 'Rare'), ('test-board', 'board', 'Test board', 'Epic');
insert into public.wallets (profile_id, jlore)
values ('11111111-1111-1111-1111-111111111111', 10), ('22222222-2222-2222-2222-222222222222', 20);
insert into public.jlore_grants (profile_id, delta, reason)
values ('11111111-1111-1111-1111-111111111111', 10, 'game'), ('22222222-2222-2222-2222-222222222222', 20, 'quest');
insert into public.inventory (profile_id, item, key, count)
values ('11111111-1111-1111-1111-111111111111', 'set_pack', 'Core', 1),
       ('22222222-2222-2222-2222-222222222222', 'cosmetic_token', 'Rare', 2);
insert into public.inventory_grants (profile_id, item, key, delta, reason)
values ('11111111-1111-1111-1111-111111111111', 'set_pack', 'Core', 1, 'pack_purchase'),
       ('22222222-2222-2222-2222-222222222222', 'cosmetic_token', 'Rare', 2, 'token_bar');
insert into public.economy_counters (profile_id, counter, pack, value)
values ('11111111-1111-1111-1111-111111111111', 'pity_epic', '', 3),
       ('22222222-2222-2222-2222-222222222222', 'chase_shard_bar', '', 1);
insert into public.economy_counter_grants (profile_id, counter, pack, delta, reason)
values ('11111111-1111-1111-1111-111111111111', 'pity_epic', '', 3, 'pack_open'),
       ('22222222-2222-2222-2222-222222222222', 'chase_shard_bar', '', 1, 'pack_open');
insert into public.set_tracks (profile_id, set_id, xp)
values ('11111111-1111-1111-1111-111111111111', 'Core', 50), ('22222222-2222-2222-2222-222222222222', 'Core', 5);
insert into public.set_track_grants (profile_id, set_id, xp, reason)
values ('11111111-1111-1111-1111-111111111111', 'Core', 50, 'game'),
       ('22222222-2222-2222-2222-222222222222', 'Core', 5, 'game');
insert into public.held_quests (profile_id, quest_id, arrived_at)
values ('11111111-1111-1111-1111-111111111111', 'win-games', now()),
       ('22222222-2222-2222-2222-222222222222', 'play-spells', now());
insert into public.quest_schedules (profile_id, next_due_at)
values ('11111111-1111-1111-1111-111111111111', now() + interval '1 hour'),
       ('22222222-2222-2222-2222-222222222222', null);
insert into public.quest_grants (profile_id, quest_id, change, reason)
values ('11111111-1111-1111-1111-111111111111', 'win-games', 'arrived', 'quest'),
       ('22222222-2222-2222-2222-222222222222', 'play-spells', 'arrived', 'quest');
insert into public.card_cosmetics (profile_id, card_id, cosmetic_id)
values ('11111111-1111-1111-1111-111111111111', 'core-001', 'test-foil'),
       ('22222222-2222-2222-2222-222222222222', 'core-001', 'test-foil');
insert into public.account_cosmetics (profile_id, cosmetic_id)
values ('11111111-1111-1111-1111-111111111111', 'test-board'),
       ('22222222-2222-2222-2222-222222222222', 'test-board');
insert into public.cosmetic_grants (profile_id, card_id, cosmetic_id, delta, reason)
values ('11111111-1111-1111-1111-111111111111', 'core-001', 'test-foil', 1, 'pack_open'),
       ('22222222-2222-2222-2222-222222222222', null, 'test-board', 1, 'token_redeem');
insert into public.collection_grants (profile_id, card_id, delta, reason)
values ('11111111-1111-1111-1111-111111111111', 'core-001', 1, 'admin');

create temp table economy_counts (tbl text primary key, n bigint not null) on commit drop;
do $$
declare
  t text;
  n bigint;
begin
  foreach t in array array['wallets', 'jlore_grants', 'inventory', 'inventory_grants', 'economy_counters',
                           'economy_counter_grants', 'set_tracks', 'set_track_grants', 'held_quests',
                           'quest_schedules', 'quest_grants', 'cosmetics', 'card_cosmetics',
                           'account_cosmetics', 'cosmetic_grants', 'collection_grants'] loop
    execute format('select count(*) from public.%I', t) into n;
    if n = 0 then
      raise exception 'FAIL (0044): public.% has no row; the re-apply below would prove nothing', t;
    end if;
    insert into economy_counts values (t, n);
  end loop;
end $$;

-- What run.sh's apply_migration filters, a re-run's "already exists, skipping", is quieted here.
set local client_min_messages = warning;
\i /tmp/0044_economy.sql
\i /tmp/0044_economy.sql
set local client_min_messages = notice;

do $$
declare
  r record;
  n bigint;
  col record;
begin
  for r in select c.tbl, c.n as before from economy_counts c loop
    execute format('select count(*) from public.%I', r.tbl) into n;
    if n <> r.before then
      raise exception 'FAIL (0044): re-applying the migration changed public.% from % rows to %', r.tbl, r.before, n;
    end if;
  end loop;

  select column_default, is_nullable, data_type into col
    from information_schema.columns
   where table_schema = 'public' and table_name = 'collection_grants' and column_name = 'dev';
  if not found or col.data_type <> 'boolean' or col.is_nullable <> 'NO' or col.column_default <> 'false' then
    raise exception 'FAIL (§14.10): collection_grants.dev is not a boolean not null default false: %', col;
  end if;
  if exists (select 1 from public.collection_grants where dev) then
    raise exception 'FAIL (§14.10): a collection grant no dev tool wrote is marked dev';
  end if;
  raise notice 'OK (0044): the migration re-applies over a row in every table and changes none, and collection_grants.dev defaults to false';
end $$;

\echo '### 0044: the checks refuse what the store refuses (src/economy/store.rs) ###'
do $$
declare
  p1 constant uuid := '11111111-1111-1111-1111-111111111111';
  -- [what, statement]. Each breaks one of 0044's checks and nothing else.
  refused constant text[][] := array[
    ['a wallet below zero', format($q$update public.wallets set jlore = -1 where profile_id = %L$q$, p1)],
    ['a dev reason without the dev mark',
     format($q$insert into public.jlore_grants (profile_id, delta, reason) values (%L, 5, 'dev')$q$, p1)],
    ['a Jlore reason of another ledger',
     format($q$insert into public.jlore_grants (profile_id, delta, reason) values (%L, 5, 'pack_open')$q$, p1)],
    ['a zero Jlore change',
     format($q$insert into public.jlore_grants (profile_id, delta, reason) values (%L, 0, 'game')$q$, p1)],
    ['a global pack under a key',
     format($q$insert into public.inventory (profile_id, item, key, count) values (%L, 'global_pack', 'Core', 1)$q$, p1)],
    ['a set pack under no key',
     format($q$insert into public.inventory (profile_id, item, count) values (%L, 'set_pack', 1)$q$, p1)],
    ['a cosmetic token under no rarity',
     format($q$insert into public.inventory_grants (profile_id, item, key, delta, reason)
               values (%L, 'cosmetic_token', '', 1, 'token_bar')$q$, p1)],
    ['an inventory count below zero',
     format($q$update public.inventory set count = -1 where profile_id = %L$q$, p1)],
    ['a bar kept per pack',
     format($q$insert into public.economy_counters (profile_id, counter, pack, value)
               values (%L, 'card_token_bar', 'global', 1)$q$, p1)],
    ['a counter no section names',
     format($q$insert into public.economy_counter_grants (profile_id, counter, delta, reason)
               values (%L, 'pity_common', 1, 'pack_open')$q$, p1)],
    ['a level below the first',
     format($q$update public.set_tracks set level = 0, rewards_claimed = 0 where profile_id = %L$q$, p1)],
    ['more levels paid than reached',
     format($q$update public.set_tracks set rewards_claimed = level + 1 where profile_id = %L$q$, p1)],
    ['a track grant of nothing',
     format($q$insert into public.set_track_grants (profile_id, set_id, reason) values (%L, 'Core', 'game')$q$, p1)],
    ['progress made without progressing',
     format($q$insert into public.quest_grants (profile_id, quest_id, change, progress, reason)
               values (%L, 'win-games', 'arrived', 1, 'quest')$q$, p1)],
    ['progressing without progress',
     format($q$insert into public.quest_grants (profile_id, quest_id, change, reason)
               values (%L, 'win-games', 'progressed', 'quest')$q$, p1)],
    ['a cosmetic granted twice in one row',
     format($q$insert into public.cosmetic_grants (profile_id, cosmetic_id, delta, reason)
               values (%L, 'test-board', 2, 'pack_open')$q$, p1)],
    ['a cosmetic id that is no slug',
     $q$insert into public.cosmetics (id, kind, name, rarity) values ('Test-Foil', 'foil', 'x', 'Rare')$q$],
    ['a cosmetic of no rarity',
     $q$insert into public.cosmetics (id, kind, name, rarity) values ('test-token', 'foil', 'x', 'Token')$q$]];
  i int;
begin
  for i in 1 .. array_length(refused, 1) loop
    begin
      execute refused[i][2];
      raise exception 'FAIL (0044): the schema took %', refused[i][1];
    exception
      when check_violation then null;
    end;
  end loop;
  -- The vacuity guard: a dev grant with its mark, and another reason's row marked, are taken.
  insert into public.jlore_grants (profile_id, delta, reason, dev) values (p1, 5, 'dev', true);
  insert into public.jlore_grants (profile_id, delta, reason, dev) values (p1, 5, 'quest', true);
  raise notice 'OK (0044): % rows the checks refuse are refused, and a marked dev grant is taken',
    array_length(refused, 1);
end $$;

\echo '### §14.11: every economy ledger and the collection''s refuse an edit or a delete, even the superuser''s ###'
do $$
declare
  p1 constant uuid := '11111111-1111-1111-1111-111111111111';
  t  text;
  op text;
begin
  foreach t in array array['jlore_grants', 'inventory_grants', 'economy_counter_grants', 'set_track_grants',
                           'quest_grants', 'cosmetic_grants', 'collection_grants'] loop
    if not exists (select 1 from economy_counts where tbl = t) then
      raise exception 'FAIL (§14.11): public.% had no row to edit; the refusals below would prove nothing', t;
    end if;
    foreach op in array array[
      format('update public.%I set dev = dev where profile_id = %L', t, p1),
      format('delete from public.%I where profile_id = %L', t, p1)] loop
      begin
        execute op;
        raise exception 'FAIL (§14.11): % ran on the append-only public.%', op, t;
      exception
        when raise_exception then
          if sqlerrm not like format('append-only table public.%s %%', t) then
            raise;
          end if;
      end;
    end loop;
  end loop;
  raise notice 'OK (§14.11): the six economy ledgers and collection_grants refuse an update and a delete';
end $$;

\echo '### §14.11, §9.1: no client role writes any economy table, and anon reads none ###'
do $$
declare
  tables constant text[] := array['wallets', 'jlore_grants', 'inventory', 'inventory_grants', 'economy_counters',
                                  'economy_counter_grants', 'set_tracks', 'set_track_grants', 'held_quests',
                                  'quest_schedules', 'quest_grants', 'cosmetics', 'card_cosmetics',
                                  'account_cosmetics', 'cosmetic_grants'];
  who  text;
  t    text;
  op   text;
  col  text;
  refusals int := 0;
begin
  foreach who in array array['authenticated', 'anon'] loop
    execute format('set local role %I', who);
    perform set_config('request.jwt.claim.sub', '11111111-1111-1111-1111-111111111111', true);
    if current_user <> who then
      raise exception 'FAIL (§14.11): running as %, not % — SET LOCAL did not take', current_user, who;
    end if;
    foreach t in array tables loop
      col := case when t = 'cosmetics' then 'name' else 'profile_id' end;
      foreach op in array array[
        format('insert into public.%I default values', t),
        format('update public.%I set %I = %I where false', t, col, col),
        format('delete from public.%I where false', t),
        case when who = 'anon' then format('select count(*) from public.%I', t) end] loop
        continue when op is null;
        begin
          execute op;
          raise exception 'FAIL (§14.11): % ran "%"', who, op;
        exception
          -- Only the privilege check answers 42501: a statement that broke anywhere else is no refusal.
          when insufficient_privilege then refusals := refusals + 1;
        end;
      end loop;
    end loop;
    reset role;
  end loop;
  if refusals <> 2 * 3 * array_length(tables, 1) + array_length(tables, 1) then
    raise exception 'FAIL (§14.11): % refusals, expected one per write per role and per anon read', refusals;
  end if;
  raise notice 'OK (§14.11): authenticated and anon insert, update and delete none of the % tables, and anon reads none',
    array_length(tables, 1);
end $$;

\echo '### §14 (lead), §9.1: an account reads its own economy rows and nobody else''s; every account reads the cosmetics ###'
do $$
declare
  p1 constant uuid := '11111111-1111-1111-1111-111111111111';
  p2 constant uuid := '22222222-2222-2222-2222-222222222222';
  own constant text[] := array['wallets', 'jlore_grants', 'inventory', 'inventory_grants', 'economy_counters',
                               'economy_counter_grants', 'set_tracks', 'set_track_grants', 'held_quests',
                               'quest_schedules', 'quest_grants', 'card_cosmetics', 'account_cosmetics',
                               'cosmetic_grants'];
  who   uuid;
  t     text;
  seen  bigint;
  mine  bigint;
begin
  set local role authenticated;
  foreach who in array array[p1, p2] loop
    perform set_config('request.jwt.claim.sub', who::text, true);
    foreach t in array own loop
      execute format('select count(*), count(*) filter (where profile_id = %L) from public.%I', who, t)
        into seen, mine;
      if mine = 0 then
        raise exception 'FAIL (§14): profile % reads none of its own rows of public.%', who, t;
      end if;
      if seen <> mine then
        raise exception 'FAIL (§14): profile % reads % rows of public.% that are not its own', who, seen - mine, t;
      end if;
    end loop;
    select count(*) into seen from public.cosmetics where id in ('test-foil', 'test-board');
    if seen <> 2 then
      raise exception 'FAIL (§14.2): profile % reads % of the two cosmetics', who, seen;
    end if;
  end loop;
  reset role;
  raise notice 'OK (§14): each of two profiles reads its own rows of all % tables and no other''s, and both read the cosmetics',
    array_length(own, 1);
end $$;
rollback;

\echo '### 0012''s rule: a deleted account takes every economy row with it, and nobody else''s ###'
begin;
insert into public.cosmetics (id, kind, name, rarity) values ('test-foil', 'foil', 'Test foil', 'Rare');
insert into auth.users (id, email, email_confirmed_at)
values ('e0440000-0000-4000-8000-000000000001', 'economy-leaver@example.test', now());
do $$
declare
  gone constant uuid := 'e0440000-0000-4000-8000-000000000001';
  p1   constant uuid := '11111111-1111-1111-1111-111111111111';
  tables constant text[] := array['wallets', 'jlore_grants', 'inventory', 'inventory_grants', 'economy_counters',
                                  'economy_counter_grants', 'set_tracks', 'set_track_grants', 'held_quests',
                                  'quest_schedules', 'quest_grants', 'card_cosmetics', 'account_cosmetics',
                                  'cosmetic_grants'];
  who uuid;
  t   text;
  n   bigint;
begin
  if not exists (select 1 from public.profiles where id = gone) then
    raise exception 'FAIL (0044): the leaver has no profile; the cascade below would prove nothing';
  end if;
  foreach who in array array[gone, p1] loop
    insert into public.wallets (profile_id, jlore) values (who, 1);
    insert into public.jlore_grants (profile_id, delta, reason) values (who, 1, 'game');
    insert into public.inventory (profile_id, item, count) values (who, 'global_pack', 1);
    insert into public.inventory_grants (profile_id, item, delta, reason) values (who, 'global_pack', 1, 'track');
    insert into public.economy_counters (profile_id, counter, value) values (who, 'card_token_bar', 1);
    insert into public.economy_counter_grants (profile_id, counter, delta, reason)
    values (who, 'card_token_bar', 1, 'pack_open');
    insert into public.set_tracks (profile_id, set_id, xp) values (who, 'Classic', 1);
    insert into public.set_track_grants (profile_id, set_id, xp, reason) values (who, 'Classic', 1, 'game');
    insert into public.held_quests (profile_id, quest_id, arrived_at) values (who, 'deal-damage', now());
    insert into public.quest_schedules (profile_id, next_due_at) values (who, now());
    insert into public.quest_grants (profile_id, quest_id, change, reason) values (who, 'deal-damage', 'arrived', 'quest');
    insert into public.card_cosmetics (profile_id, card_id, cosmetic_id) values (who, 'core-002', 'test-foil');
    insert into public.account_cosmetics (profile_id, cosmetic_id) values (who, 'test-foil');
    insert into public.cosmetic_grants (profile_id, card_id, cosmetic_id, delta, reason, dev)
    values (who, 'core-002', 'test-foil', 1, 'dev', true);
  end loop;

  delete from auth.users where id = gone;

  foreach t in array tables loop
    execute format('select count(*) from public.%I where profile_id = %L', t, gone) into n;
    if n <> 0 then
      raise exception 'FAIL (0044): % rows of public.% outlived their deleted account', n, t;
    end if;
    execute format('select count(*) from public.%I where profile_id = %L', t, p1) into n;
    if n = 0 then
      raise exception 'FAIL (0044): deleting one account took another''s rows of public.%', t;
    end if;
  end loop;
  if not exists (select 1 from public.cosmetics where id = 'test-foil') then
    raise exception 'FAIL (0044): deleting an account took a cosmetic of the registry';
  end if;
  raise notice 'OK (0044): deleting an account deletes its rows of all % tables, ledgers included, and no other account''s',
    array_length(tables, 1);
end $$;
rollback;
