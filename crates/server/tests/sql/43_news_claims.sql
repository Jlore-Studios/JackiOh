-- News claims, one per account and entry, and the ledgers' reason for a news reward (migration 0077,
-- SPEC §14.11, §14.13, §9.1; R2875000; issue #2875). Runs after 42_animated_art.sql; profiles 1 and 2
-- (11111111-…, 22222222-…) are active by then (03 activated them).
\set ON_ERROR_STOP on

-- Same rules as 01-42: every check raises on failure (`raise exception 'FAIL (…): …'`), every
-- expected refusal is matched on its sqlstate (and on the constraint, the table or the message that
-- names it), each check that could pass vacuously carries a guard, and every block that writes runs
-- inside a transaction that is rolled back: a file that re-applies 0050's reasons, which know no
-- 'news' (29_going_live.sql does), would fail on a news row this file left. Nothing here selects
-- into the log: the do blocks raise or say OK.

\echo '### R2875000: public.news_claims keys one claim per account and entry, follows its account, and is append-only ###'
do $$
declare
  v_text text;
  r      record;
begin
  -- RLS on (CHECK 1 holds every table to it; this names the table so its absence is caught here too).
  if not exists (select 1 from pg_class
                  where oid = 'public.news_claims'::regclass and relrowsecurity) then
    raise exception 'FAIL (R2875000): public.news_claims has no row level security';
  end if;

  -- The columns, as the store reads and writes them.
  select string_agg(format('%s %s %s', column_name, data_type, is_nullable), ', ' order by ordinal_position)
    into v_text
    from information_schema.columns
   where table_schema = 'public' and table_name = 'news_claims';
  if v_text is distinct from 'profile_id uuid NO, news_id text NO, claimed_at timestamp with time zone NO' then
    raise exception 'FAIL (R2875000): public.news_claims has the columns [%]', coalesce(v_text, '(none)');
  end if;

  -- Once per account and entry: the key is what makes a claim once.
  select string_agg(a.attname, ', ' order by k.ord) into v_text
    from pg_constraint c
    cross join lateral unnest(c.conkey) with ordinality as k(attnum, ord)
    join pg_attribute a on a.attrelid = c.conrelid and a.attnum = k.attnum
   where c.conrelid = 'public.news_claims'::regclass and c.contype = 'p';
  if v_text is distinct from 'profile_id, news_id' then
    raise exception 'FAIL (R2875000): the primary key is (%), expected (profile_id, news_id)', coalesce(v_text, 'none');
  end if;

  -- Its one foreign key is its account's, cascading (0012's rule).
  select string_agg(format('%s %s', c.confrelid::regclass, c.confdeltype), ', ') into v_text
    from pg_constraint c
   where c.conrelid = 'public.news_claims'::regclass and c.contype = 'f';
  if v_text is distinct from 'profiles c' then
    raise exception 'FAIL (R2875000): the foreign keys are [%], expected the profile''s alone, on delete cascade',
      coalesce(v_text, '(none)');
  end if;

  -- The entry is named by its id, a slug, as economy::news and cosmetics::is_slug have it.
  select pg_get_constraintdef(c.oid) into v_text
    from pg_constraint c
   where c.conrelid = 'public.news_claims'::regclass and c.conname = 'news_claims_news_check' and c.contype = 'c';
  if v_text is distinct from 'CHECK ((news_id ~ ''^[a-z][a-z0-9-]*$''::text))' then
    raise exception 'FAIL (R2875000): news_claims_news_check is [%], expected news_id held to a slug',
      coalesce(v_text, '(none)');
  end if;

  -- Append-only, as a ledger is: one row trigger, before an update or a delete, through
  -- app.deny_row_mutation keyed on the row's profile, so only the account's own cascade passes.
  select count(*) as n, min(pg_get_triggerdef(t.oid)) as def, min(t.tgenabled::text) as enabled into r
    from pg_trigger t
   where t.tgrelid = 'public.news_claims'::regclass and not t.tgisinternal;
  if r.n <> 1 then
    raise exception 'FAIL (R2875000): public.news_claims has % triggers, expected news_claims_deny_mutation alone', r.n;
  end if;
  if r.def is distinct from 'CREATE TRIGGER news_claims_deny_mutation BEFORE DELETE OR UPDATE ON public.news_claims '
                            'FOR EACH ROW EXECUTE FUNCTION app.deny_row_mutation(''profile_id'')' then
    raise exception 'FAIL (R2875000): the trigger on public.news_claims is [%]', r.def;
  end if;
  if r.enabled not in ('O', 'A') then
    raise exception 'FAIL (R2875000): news_claims_deny_mutation is disabled (tgenabled = %)', r.enabled;
  end if;

  raise notice 'OK (R2875000): news_claims has RLS on, its columns, one row per (profile_id, news_id), one foreign key, its account''s, cascading, a slug check on news_id and its append-only trigger';
end $$;

\echo '### R2875000, §9.1: authenticated reads its own news claims through one select policy and writes none; anon reaches nothing ###'
do $$
declare
  v_text text;
  op     text;
  who    text;
  wrong  text[] := array[]::text[];
begin
  select string_agg(format('%s %s %s %s', policyname, cmd, array_to_string(roles, ','), qual), '; ')
    into v_text
    from pg_policies
   where schemaname = 'public' and tablename = 'news_claims';
  if v_text is distinct from 'news_claims_select_own SELECT authenticated (profile_id = app.current_profile_id())' then
    raise exception 'FAIL (R2875000): the policies are [%], expected one select-own policy to authenticated',
      coalesce(v_text, '(none)');
  end if;

  foreach who in array array['authenticated', 'anon', 'public'] loop
    foreach op in array array['SELECT', 'INSERT', 'UPDATE', 'DELETE', 'TRUNCATE', 'REFERENCES', 'TRIGGER'] loop
      if has_table_privilege(who, 'public.news_claims', op) <> (who = 'authenticated' and op = 'SELECT') then
        wrong := wrong || format('%s %s', who, op);
      end if;
    end loop;
  end loop;
  if cardinality(wrong) > 0 then
    raise exception 'FAIL (R2875000): the client privileges on news_claims are wrong for [%]',
      array_to_string(wrong, ', ');
  end if;
  -- What the server's role may do is Supabase's grant, which this suite's stub leaves out
  -- (tests/db/grants.sql makes it, and tests/store/contract.rs writes the table as the server).
  raise notice 'OK (R2875000): one select-own policy to authenticated, SELECT its only client privilege, anon and public none';
end $$;

\echo '### R2875000: a claim is held once per account and entry, its news id is a slug, and a written claim is never changed ###'
begin;
do $$
declare
  p1 constant uuid := '11111111-1111-1111-1111-111111111111';
  p2 constant uuid := '22222222-2222-2222-2222-222222222222';
  -- [what, news_id]. Each breaks news_claims_news_check and nothing else.
  refused constant text[][] := array[
    ['an id that starts upper-case', 'Patch-v0-4-0d'],
    ['an id with an upper-case letter', 'patch-V0-4-0d'],
    ['an id that starts with a digit', '0-4-0d'],
    ['an id that starts with a hyphen', '-patch-v0-4-0d'],
    ['an empty id', ''],
    ['an id with an underscore', 'patch_v0_4_0d'],
    ['an id with a space', 'patch v0'],
    ['an id with a dot', 'patch-v0.4.0d'],
    ['an id that ends in a newline', E'patch-v0-4-0d\n'],
    ['an id with a letter outside ASCII', 'patch-é']];
  i        int;
  n        bigint;
  v_con    text;
  first_at timestamptz;
begin
  if not exists (select 1 from public.profiles where id = p1) or not exists (select 1 from public.profiles where id = p2) then
    raise exception 'FAIL (R2875000): profile 1 or 2 is missing, so every insert below would fail on its foreign key';
  end if;
  for i in 1 .. array_length(refused, 1) loop
    begin
      insert into public.news_claims (profile_id, news_id, claimed_at) values (p1, refused[i][2], now());
      raise exception 'FAIL (R2875000): the schema took %', refused[i][1];
    exception
      when check_violation then
        get stacked diagnostics v_con = constraint_name;
        if v_con is distinct from 'news_claims_news_check' then
          raise exception 'FAIL (R2875000): % was refused by %, not news_claims_news_check', refused[i][1], v_con;
        end if;
    end;
  end loop;

  -- The vacuity guard: what the store writes is taken, at the bounds of a slug: one letter, and
  -- letters, digits and hyphens to the end.
  insert into public.news_claims (profile_id, news_id, claimed_at)
  values (p1, 'patch-v0-4-0d', '2026-11-03T00:00:00Z'), (p1, 'a', now()), (p1, 'z9-', now());
  if (select count(*) from public.news_claims where profile_id = p1) <> 3 then
    raise exception 'FAIL (R2875000): the three claims the check allows were not all written';
  end if;

  -- Once per account and entry: a second row conflicts on the key.
  begin
    insert into public.news_claims (profile_id, news_id, claimed_at) values (p1, 'patch-v0-4-0d', now());
    raise exception 'FAIL (R2875000): a second claim of one entry by one account was taken';
  exception
    when unique_violation then
      get stacked diagnostics v_con = constraint_name;
      if v_con is distinct from 'news_claims_pkey' then
        raise exception 'FAIL (R2875000): the second claim was refused by %, not news_claims_pkey', v_con;
      end if;
  end;
  -- The store's own statement (economy::store's news_claim): the conflict writes nothing, and the
  -- first claim's time stands.
  insert into public.news_claims (profile_id, news_id, claimed_at)
  values (p1, 'patch-v0-4-0d', now())
  on conflict (profile_id, news_id) do nothing;
  get diagnostics n = row_count;
  if n <> 0 then
    raise exception 'FAIL (R2875000): the store''s second claim wrote % row(s), expected none', n;
  end if;
  select claimed_at into first_at from public.news_claims where profile_id = p1 and news_id = 'patch-v0-4-0d';
  if first_at is distinct from '2026-11-03T00:00:00Z'::timestamptz then
    raise exception 'FAIL (R2875000): the first claim reads as claimed at %, not when it was written', first_at;
  end if;
  -- The same entry is another account's to claim.
  insert into public.news_claims (profile_id, news_id, claimed_at) values (p2, 'patch-v0-4-0d', now());
  if (select count(*) from public.news_claims where news_id = 'patch-v0-4-0d') <> 2 then
    raise exception 'FAIL (R2875000): the same entry claimed by two accounts is not two rows';
  end if;

  -- Never changed, not even by the owner: the trigger refuses an update and a delete of a claim whose
  -- account still exists (the rows above are there, so the statements touch rows and fire it).
  begin
    update public.news_claims set claimed_at = now() where profile_id = p1;
    raise exception 'FAIL (R2875000): an update of an account''s claims was taken';
  exception
    when raise_exception then
      if sqlerrm like 'FAIL%' then raise; end if;
      if sqlerrm is distinct from 'append-only table public.news_claims may not be updated or deleted (attempted UPDATE)' then
        raise exception 'FAIL (R2875000): the update was refused by "%" (%), not app.deny_row_mutation', sqlerrm, sqlstate;
      end if;
  end;
  begin
    delete from public.news_claims where profile_id = p1 and news_id = 'patch-v0-4-0d';
    raise exception 'FAIL (R2875000): a delete of an account''s claim was taken, so it could be claimed again';
  exception
    when raise_exception then
      if sqlerrm like 'FAIL%' then raise; end if;
      if sqlerrm is distinct from 'append-only table public.news_claims may not be updated or deleted (attempted DELETE)' then
        raise exception 'FAIL (R2875000): the delete was refused by "%" (%), not app.deny_row_mutation', sqlerrm, sqlstate;
      end if;
  end;
  if (select count(*) from public.news_claims) <> 4 then
    raise exception 'FAIL (R2875000): % claims after the refused update and delete, expected the 4 written',
      (select count(*) from public.news_claims);
  end if;

  raise notice 'OK (R2875000): % ids that are no slug are refused, a second claim of an entry too (and the store''s writes nothing), another account''s claim of it is taken, and an update or a delete is refused',
    array_length(refused, 1);
end $$;
rollback;

\echo '### R2875000: the two ledgers a reward lands in take the reason news, keep every reason they had, and refuse one they do not know ###'
begin;
do $$
declare
  p1 constant uuid := '11111111-1111-1111-1111-111111111111';
  jlore_reasons constant text[] := array['game', 'quest', 'track', 'pack_purchase', 'dev', 'launch', 'news'];
  inventory_reasons constant text[] := array['game', 'quest', 'track', 'pack_purchase', 'pack_open', 'token_bar',
                                             'token_redeem', 'chase_shard', 'dev', 'launch', 'news'];
  -- [table, reason]: what neither ledger knows.
  unknown constant text[][] := array[
    ['jlore_grants', 'News'], ['jlore_grants', 'news_claim'], ['jlore_grants', 'jlorious'],
    ['inventory_grants', 'News'], ['inventory_grants', 'news_claim'], ['inventory_grants', 'jlorious']];
  why    text;
  i      int;
  v_con  text;
  v_text text;
begin
  if not exists (select 1 from public.profiles where id = p1) then
    raise exception 'FAIL (R2875000): profile 1 is missing, so every insert below would fail on its foreign key';
  end if;

  -- A claim as economy::news pays it, in one transaction: the claim's row, and the reward's ledger
  -- rows of the reason news, each with the entry's id as its ref and no dev mark. Jlore lands in the
  -- wallet's ledger; a card pack, a card token, a cosmetic token or a cosmetic pack in the inventory's.
  insert into public.news_claims (profile_id, news_id, claimed_at) values (p1, 'patch-v0-4-0d', now());
  insert into public.jlore_grants (profile_id, delta, reason, ref, dev, at)
  values (p1, 50, 'news', 'patch-v0-4-0d', false, now());
  insert into public.inventory_grants (profile_id, item, key, delta, reason, ref, dev, at)
  values (p1, 'set_pack', 'Core', 1, 'news', 'patch-v0-4-0d', false, now()),
         (p1, 'card_token', '', 1, 'news', 'patch-v0-4-0d', false, now()),
         (p1, 'cosmetic_token', 'Rare', 1, 'news', 'patch-v0-4-0d', false, now()),
         (p1, 'cosmetic_pack', '', 1, 'news', 'patch-v0-4-0d', false, now());
  select string_agg(format('%s %s %s', delta, reason, ref), ', ') into v_text
    from public.jlore_grants where profile_id = p1 and reason = 'news';
  if v_text is distinct from '50 news patch-v0-4-0d' then
    raise exception 'FAIL (R2875000): the Jlore ledger''s news rows are [%]', coalesce(v_text, '(none)');
  end if;
  if (select count(*) from public.inventory_grants where profile_id = p1 and reason = 'news' and ref = 'patch-v0-4-0d') <> 4 then
    raise exception 'FAIL (R2875000): the inventory ledger did not take the four news rewards';
  end if;

  -- 0077 re-listed each set: every reason it had before (0050's) is still taken.
  foreach why in array jlore_reasons loop
    insert into public.jlore_grants (profile_id, delta, reason, ref, dev)
    values (p1, 1, why, 'check-r2875000', why = 'dev');
  end loop;
  foreach why in array inventory_reasons loop
    insert into public.inventory_grants (profile_id, item, key, delta, reason, ref, dev)
    values (p1, 'card_token', '', 1, why, 'check-r2875000', why = 'dev');
  end loop;
  if (select count(*) from public.jlore_grants where ref = 'check-r2875000') <> cardinality(jlore_reasons)
     or (select count(*) from public.inventory_grants where ref = 'check-r2875000') <> cardinality(inventory_reasons) then
    raise exception 'FAIL (R2875000): a ledger did not take one row of each of its reasons';
  end if;

  -- And a reason neither knows is still refused, by its reason check.
  for i in 1 .. array_length(unknown, 1) loop
    begin
      if unknown[i][1] = 'jlore_grants' then
        insert into public.jlore_grants (profile_id, delta, reason, ref) values (p1, 1, unknown[i][2], 'patch-v0-4-0d');
      else
        insert into public.inventory_grants (profile_id, item, key, delta, reason, ref)
        values (p1, 'card_token', '', 1, unknown[i][2], 'patch-v0-4-0d');
      end if;
      raise exception 'FAIL (R2875000): public.% took the reason %', unknown[i][1], unknown[i][2];
    exception
      when check_violation then
        get stacked diagnostics v_con = constraint_name;
        if v_con is distinct from unknown[i][1] || '_reason_check' then
          raise exception 'FAIL (R2875000): the reason % on public.% was refused by %, not %_reason_check',
            unknown[i][2], unknown[i][1], v_con, unknown[i][1];
        end if;
    end;
  end loop;

  raise notice 'OK (R2875000): a claim''s Jlore and inventory rows of the reason news are taken, each ledger takes all % and % of its reasons, and % unknown reasons are refused',
    cardinality(jlore_reasons), cardinality(inventory_reasons), array_length(unknown, 1);
end $$;
rollback;

\echo '### R2875000: an account reads only its own news claims and writes none of them ###'
begin;

-- Claims for EACH profile, as the owner: profile 2's are the rows to hide from profile 1, and
-- profile 1's from profile 2. Raw inserts, as superuser, because the point is what RLS shows.
insert into public.news_claims (profile_id, news_id, claimed_at) values
  ('11111111-1111-1111-1111-111111111111', 'patch-v0-4-0d', now()),
  ('11111111-1111-1111-1111-111111111111', 'patch-v0-4-0e', now()),
  ('22222222-2222-2222-2222-222222222222', 'patch-v0-4-0d', now());

-- Preflight: the insert the client is about to be refused is one the owner can make, inside a
-- subtransaction rolled back at once. (An update or a delete the owner cannot make either: the
-- block above proves the trigger refuses them, and a client is refused them on privilege first.)
do $$
begin
  begin
    insert into public.news_claims (profile_id, news_id, claimed_at)
    values ('11111111-1111-1111-1111-111111111111', 'patch-v0-4-0f', now());
    raise exception 'owner-control-rollback';
  exception when others then
    if sqlerrm <> 'owner-control-rollback' then
      raise exception
        'FAIL (R2875000): the owner could not make the claim the client is about to be refused ("%", %) — the refusals below would prove nothing about privileges',
        sqlerrm, sqlstate;
    end if;
  end;
  perform set_config('r2875000.total', (select count(*) from public.news_claims)::text, true);
end $$;

set local role authenticated;
do $$ begin perform set_config('request.jwt.claim.sub', '11111111-1111-1111-1111-111111111111', true); end $$;

do $$
declare
  caller constant uuid := '11111111-1111-1111-1111-111111111111';
  total  bigint := coalesce(nullif(current_setting('r2875000.total', true), ''), '-1')::bigint;
  seen   text;
  probe  text;
  probes constant text[][] := array[
    ['news_claims INSERT',
     $q$insert into public.news_claims (profile_id, news_id, claimed_at)
        values ('11111111-1111-1111-1111-111111111111', 'patch-v0-4-0f', now())$q$],
    ['news_claims INSERT ON CONFLICT DO NOTHING',
     $q$insert into public.news_claims (profile_id, news_id, claimed_at)
        values ('11111111-1111-1111-1111-111111111111', 'patch-v0-4-0f', now())
        on conflict (profile_id, news_id) do nothing$q$],
    ['news_claims UPDATE', $q$update public.news_claims set claimed_at = now()$q$],
    ['news_claims DELETE', $q$delete from public.news_claims$q$],
    ['news_claims TRUNCATE', $q$truncate public.news_claims$q$]];
  i int;
begin
  if current_user <> 'authenticated' then
    raise exception 'FAIL (R2875000): running as %, not authenticated — SET LOCAL did not take', current_user;
  end if;
  if total <> 3 then
    raise exception 'FAIL (R2875000): % news_claims rows to measure against, expected 3 (two of profile 1, one of profile 2)', total;
  end if;

  select string_agg(news_id, ', ' order by news_id) into seen from public.news_claims;
  if seen is distinct from 'patch-v0-4-0d, patch-v0-4-0e'
     or exists (select 1 from public.news_claims where profile_id <> caller) then
    raise exception 'FAIL (R2875000): profile 1 read the claims [%] of % — expected its own two and no other account''s',
      coalesce(seen, '(none)'), total;
  end if;

  for i in 1 .. array_length(probes, 1) loop
    probe := probes[i][1];
    begin
      execute probes[i][2];
      raise exception 'FAIL (R2875000): % succeeded as a client — a claim is written by the server alone', probe;
    exception
      when insufficient_privilege then
        if sqlerrm not like '%news_claims%' then
          raise exception 'FAIL (R2875000): % was refused by "%" (%), which does not name news_claims',
            probe, sqlerrm, sqlstate;
        end if;
        raise notice 'OK (R2875000): % refused — % (%)', probe, sqlerrm, sqlstate;
      when others then
        if sqlerrm like 'FAIL%' then raise; end if;
        raise exception 'FAIL (R2875000): % raised "%" (%), not insufficient_privilege', probe, sqlerrm, sqlstate;
    end;
  end loop;

  raise notice 'OK (R2875000): profile 1 reads its own two claims and not profile 2''s, and % writes were refused',
    array_length(probes, 1);
end $$;

do $$ begin perform set_config('request.jwt.claim.sub', '22222222-2222-2222-2222-222222222222', true); end $$;
do $$
declare
  seen text;
begin
  if current_user <> 'authenticated' then
    raise exception 'FAIL (R2875000): running as %, not authenticated', current_user;
  end if;
  select string_agg(profile_id || ' ' || news_id, ', ') into seen from public.news_claims;
  if seen is distinct from '22222222-2222-2222-2222-222222222222 patch-v0-4-0d' then
    raise exception 'FAIL (R2875000): profile 2 read the claims [%], expected only its own one', coalesce(seen, '(none)');
  end if;
  raise notice 'OK (R2875000): profile 2 reads only its own claim';
end $$;

-- Signed in as nobody: the policy's app.current_profile_id() is null, so no row is anybody's.
do $$ begin perform set_config('request.jwt.claim.sub', '', true); end $$;
do $$
begin
  if app.current_profile_id() is not null then
    raise exception 'FAIL (R2875000): the cleared claim still names profile %', app.current_profile_id();
  end if;
  if exists (select 1 from public.news_claims) then
    raise exception 'FAIL (R2875000): a client with no profile read % claims', (select count(*) from public.news_claims);
  end if;
  raise notice 'OK (R2875000): a client with no profile reads no claim';
end $$;

reset role;
set local role anon;
do $$
begin
  if current_user <> 'anon' then
    raise exception 'FAIL (R2875000): running as %, not anon', current_user;
  end if;
  begin
    perform count(*) from public.news_claims;
    raise exception 'FAIL (R2875000): anon read news_claims';
  exception
    when insufficient_privilege then null;
  end;
  raise notice 'OK (R2875000): anon reads no claims';
end $$;

reset role;
rollback;

\echo '### R2875000: a deleted account takes its news claims and its news rewards with it, and nobody else''s ###'
begin;
insert into auth.users (id, email, email_confirmed_at)
values ('e2875000-0000-4000-8000-000000000001', 'r2875000-leaver@example.test', now());
do $$
declare
  gone constant uuid := 'e2875000-0000-4000-8000-000000000001';
  p1   constant uuid := '11111111-1111-1111-1111-111111111111';
begin
  if not exists (select 1 from public.profiles where id = gone) then
    raise exception 'FAIL (R2875000): the leaver has no profile; the cascade below would prove nothing';
  end if;
  insert into public.news_claims (profile_id, news_id, claimed_at)
  values (gone, 'patch-v0-4-0d', now()), (gone, 'patch-v0-4-0e', now()), (p1, 'patch-v0-4-0d', now());
  insert into public.jlore_grants (profile_id, delta, reason, ref) values (gone, 50, 'news', 'patch-v0-4-0d');
  insert into public.inventory_grants (profile_id, item, key, delta, reason, ref)
  values (gone, 'card_token', '', 1, 'news', 'patch-v0-4-0e');

  -- The trigger passes the cascade, and only it: the profile is gone when its rows go.
  delete from auth.users where id = gone;

  if exists (select 1 from public.profiles where id = gone) then
    raise exception 'FAIL (R2875000): the leaver''s profile outlived its auth.users row';
  end if;
  if exists (select 1 from public.news_claims where profile_id = gone) then
    raise exception 'FAIL (R2875000): a deleted account''s news claims outlived it';
  end if;
  if exists (select 1 from public.jlore_grants where profile_id = gone)
     or exists (select 1 from public.inventory_grants where profile_id = gone) then
    raise exception 'FAIL (R2875000): a deleted account''s news rewards outlived it in a ledger';
  end if;
  if not exists (select 1 from public.news_claims where profile_id = p1 and news_id = 'patch-v0-4-0d') then
    raise exception 'FAIL (R2875000): deleting one account took another''s news claim';
  end if;
  raise notice 'OK (R2875000): deleting an account deletes its news claims and news rewards and no other account''s';
end $$;
rollback;

\echo '### R2875000: 0077 re-applies over rows of its own and changes none ###'
begin;
insert into public.news_claims (profile_id, news_id, claimed_at)
values ('11111111-1111-1111-1111-111111111111', 'patch-v0-4-0d', now());
insert into public.jlore_grants (profile_id, delta, reason, ref)
values ('11111111-1111-1111-1111-111111111111', 50, 'news', 'patch-v0-4-0d');
insert into public.inventory_grants (profile_id, item, key, delta, reason, ref)
values ('11111111-1111-1111-1111-111111111111', 'card_token', '', 1, 'news', 'patch-v0-4-0d');
create temp table r2875000_rows (tbl text primary key, n bigint not null) on commit drop;
insert into r2875000_rows
select 'news_claims', count(*) from public.news_claims
union all select 'jlore_grants', count(*) from public.jlore_grants
union all select 'inventory_grants', count(*) from public.inventory_grants;
-- What run.sh's apply_migration filters, a re-run's "already exists, skipping", is quieted here.
set local client_min_messages = warning;
\i /tmp/0077_news_claims.sql
set local client_min_messages = notice;
do $$
declare
  r record;
  n bigint;
begin
  if (select x.n from r2875000_rows x where x.tbl = 'news_claims') <> 1 then
    raise exception 'FAIL (R2875000): % claims before the re-apply, expected the one written',
      (select x.n from r2875000_rows x where x.tbl = 'news_claims');
  end if;
  for r in select x.tbl, x.n as before from r2875000_rows x loop
    execute format('select count(*) from public.%I', r.tbl) into n;
    if n <> r.before then
      raise exception 'FAIL (R2875000): re-applying 0077 changed public.% from % rows to %', r.tbl, r.before, n;
    end if;
  end loop;
  if (select count(*) from pg_policies where schemaname = 'public' and tablename = 'news_claims') <> 1 then
    raise exception 'FAIL (R2875000): re-applying 0077 left other than one policy on news_claims';
  end if;
  if (select count(*) from pg_trigger where tgrelid = 'public.news_claims'::regclass and not tgisinternal) <> 1 then
    raise exception 'FAIL (R2875000): re-applying 0077 left other than one trigger on news_claims';
  end if;
  if (select count(*) from pg_constraint
       where conname in ('jlore_grants_reason_check', 'inventory_grants_reason_check')
         and position('''news''' in pg_get_constraintdef(oid)) > 0) <> 2 then
    raise exception 'FAIL (R2875000): after re-applying 0077 a ledger''s reason check lacks news';
  end if;
  raise notice 'OK (R2875000): 0077 re-applies over its own rows and changes none';
end $$;
rollback;

\echo '=== ALL CHECKS RAN ==='
