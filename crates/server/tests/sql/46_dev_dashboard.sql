-- The dev dashboard's tables: its audit, the hand-set ratings and their ledger, and the news posts
-- (migration 0080, SPEC §14.10, §14.11, §14.13, §9.1; R3074000, R3074001, R3074003; issue #3074).
-- Runs after 45_card_versions.sql; profiles 1 and 2 (11111111-…, 22222222-…) are active by then (03
-- activated them).
\set ON_ERROR_STOP on

-- Same rules as 01-45: every check raises on failure (`raise exception 'FAIL (…): …'`), every
-- expected refusal is matched on its sqlstate and on the constraint or the message that names it,
-- each check that could pass vacuously carries a guard, and every block that writes runs inside a
-- transaction that is rolled back.

\echo '### R3074000: public.admin_actions is the append-only audit, server-only, and outlives its account ###'
do $$
declare
  v_text text;
  r      record;
  tbl    text;
  op     text;
  who    text;
  wrong  text[] := array[]::text[];
begin
  -- RLS on and no policy, and no client privilege, on all four tables: the server alone reads and
  -- writes them (CHECK 1 holds every table to RLS; this names these four).
  foreach tbl in array array['admin_actions', 'hand_set_ratings', 'rating_grants', 'news_posts'] loop
    if not exists (select 1 from pg_class
                    where oid = format('public.%I', tbl)::regclass and relrowsecurity) then
      raise exception 'FAIL (R3074000): public.% has no row level security', tbl;
    end if;
    if exists (select 1 from pg_policies where schemaname = 'public' and tablename = tbl) then
      raise exception 'FAIL (R3074000): public.% has a policy; no client reads it', tbl;
    end if;
    foreach who in array array['authenticated', 'anon', 'public'] loop
      foreach op in array array['SELECT', 'INSERT', 'UPDATE', 'DELETE', 'TRUNCATE', 'REFERENCES', 'TRIGGER'] loop
        if has_table_privilege(who, format('public.%I', tbl), op) then
          wrong := wrong || format('%s %s %s', tbl, who, op);
        end if;
      end loop;
    end loop;
  end loop;
  if cardinality(wrong) > 0 then
    raise exception 'FAIL (R3074000): client privileges on the dashboard''s tables: [%]', array_to_string(wrong, ', ');
  end if;

  select string_agg(format('%s %s %s', column_name, data_type, is_nullable), ', ' order by ordinal_position)
    into v_text
    from information_schema.columns
   where table_schema = 'public' and table_name = 'admin_actions';
  if v_text is distinct from 'id bigint NO, profile_id uuid YES, route text NO, request jsonb NO, '
                             'before_value jsonb NO, after_value jsonb NO, at timestamp with time zone NO' then
    raise exception 'FAIL (R3074000): public.admin_actions has the columns [%]', coalesce(v_text, '(none)');
  end if;

  -- Its one foreign key is its account's, set null: the row outlives the account.
  select string_agg(format('%s %s', c.confrelid::regclass, c.confdeltype), ', ') into v_text
    from pg_constraint c
   where c.conrelid = 'public.admin_actions'::regclass and c.contype = 'f';
  if v_text is distinct from 'profiles n' then
    raise exception 'FAIL (R3074000): the audit''s foreign keys are [%], expected the profile''s, on delete set null',
      coalesce(v_text, '(none)');
  end if;

  select count(*) as n, min(pg_get_triggerdef(t.oid)) as def, min(t.tgenabled::text) as enabled into r
    from pg_trigger t
   where t.tgrelid = 'public.admin_actions'::regclass and not t.tgisinternal;
  if r.n <> 1 or r.def is distinct from 'CREATE TRIGGER admin_actions_deny_mutation BEFORE DELETE OR UPDATE ON public.admin_actions '
                                        'FOR EACH ROW EXECUTE FUNCTION app.deny_row_mutation(''profile_id'')'
     or r.enabled not in ('O', 'A') then
    raise exception 'FAIL (R3074000): the audit''s triggers are % [%] (%)', r.n, r.def, r.enabled;
  end if;

  raise notice 'OK (R3074000): the four tables have RLS on, no policy and no client privilege; the audit has its columns, a set-null foreign key and its append-only trigger';
end $$;

\echo '### R3074000: an audit row names a dev or admin route, is never changed, and forgets a deleted account ###'
begin;
insert into auth.users (id, email, email_confirmed_at)
values ('e3074000-0000-4000-8000-000000000001', 'r3074000-dev@example.test', now());
do $$
declare
  gone constant uuid := 'e3074000-0000-4000-8000-000000000001';
  p1   constant uuid := '11111111-1111-1111-1111-111111111111';
  refused constant text[] := array['GET /api/news', 'POST /api/dev', 'FETCH /api/dev/jlore', '/api/admin/me',
                                   'post /api/admin/news', 'POST /api/devtools/x'];
  i     int;
  v_con text;
begin
  if not exists (select 1 from public.profiles where id = gone) then
    raise exception 'FAIL (R3074000): the leaver has no profile; the set-null below would prove nothing';
  end if;
  for i in 1 .. array_length(refused, 1) loop
    begin
      insert into public.admin_actions (profile_id, route, request, before_value, after_value, at)
      values (p1, refused[i], '{}', 'null', 'null', now());
      raise exception 'FAIL (R3074000): the audit took the route %', refused[i];
    exception
      when check_violation then
        get stacked diagnostics v_con = constraint_name;
        if v_con is distinct from 'admin_actions_route_check' then
          raise exception 'FAIL (R3074000): % was refused by %, not admin_actions_route_check', refused[i], v_con;
        end if;
    end;
  end loop;

  -- The vacuity guard: what the server writes is taken.
  insert into public.admin_actions (profile_id, route, request, before_value, after_value, at)
  values (gone, 'POST /api/dev/jlore', '{"jlore": 5}', '{"jlore": 0}', '{"jlore": 5}', now()),
         (gone, 'DELETE /api/admin/news/post-abc', '{"postId": "post-abc"}', '{}', '{}', now()),
         (p1, 'PUT /api/admin/rating', '{"ladder": "random", "rating": 1700}', '{}', '{}', now());
  if (select count(*) from public.admin_actions) <> 3 then
    raise exception 'FAIL (R3074000): the three audit rows were not all written';
  end if;

  begin
    update public.admin_actions set route = 'POST /api/dev/fill' where profile_id = p1;
    raise exception 'FAIL (R3074000): an audit row was updated';
  exception
    when raise_exception then
      if sqlerrm like 'FAIL%' then raise; end if;
      if sqlerrm is distinct from 'append-only table public.admin_actions may not be updated or deleted (attempted UPDATE)' then
        raise exception 'FAIL (R3074000): the update was refused by "%", not app.deny_row_mutation', sqlerrm;
      end if;
  end;
  begin
    delete from public.admin_actions where profile_id = p1;
    raise exception 'FAIL (R3074000): an audit row was deleted';
  exception
    when raise_exception then
      if sqlerrm like 'FAIL%' then raise; end if;
      if sqlerrm is distinct from 'append-only table public.admin_actions may not be updated or deleted (attempted DELETE)' then
        raise exception 'FAIL (R3074000): the delete was refused by "%", not app.deny_row_mutation', sqlerrm;
      end if;
  end;

  -- The account's deletion is the one update the guard lets through: its rows stay, forgetting who.
  delete from auth.users where id = gone;
  if exists (select 1 from public.profiles where id = gone) then
    raise exception 'FAIL (R3074000): the leaver''s profile outlived its auth.users row';
  end if;
  if (select count(*) from public.admin_actions where profile_id is null) <> 2
     or (select count(*) from public.admin_actions) <> 3 then
    raise exception 'FAIL (R3074000): after the deletion the audit holds % rows, % of them nobody''s; expected 3 and 2',
      (select count(*) from public.admin_actions), (select count(*) from public.admin_actions where profile_id is null);
  end if;
  if (select route from public.admin_actions where profile_id is null order by id limit 1) is distinct from 'POST /api/dev/jlore' then
    raise exception 'FAIL (R3074000): a forgotten row changed more than its profile';
  end if;
  raise notice 'OK (R3074000): % routes refused, the server''s taken, an update and a delete refused, and a deleted account''s rows kept with no profile',
    array_length(refused, 1);
end $$;
rollback;

\echo '### R3074001: a hand-set rating is one row per account and ladder, and its ledger is marked dev and append-only ###'
begin;
insert into auth.users (id, email, email_confirmed_at)
values ('e3074001-0000-4000-8000-000000000001', 'r3074001-dev@example.test', now());
do $$
declare
  gone constant uuid := 'e3074001-0000-4000-8000-000000000001';
  p1   constant uuid := '11111111-1111-1111-1111-111111111111';
  v_con  text;
  v_text text;
begin
  select string_agg(a.attname, ', ' order by k.ord) into v_text
    from pg_constraint c
    cross join lateral unnest(c.conkey) with ordinality as k(attnum, ord)
    join pg_attribute a on a.attrelid = c.conrelid and a.attnum = k.attnum
   where c.conrelid = 'public.hand_set_ratings'::regclass and c.contype = 'p';
  if v_text is distinct from 'profile_id, format' then
    raise exception 'FAIL (R3074001): hand_set_ratings'' key is (%), expected (profile_id, format)', coalesce(v_text, 'none');
  end if;
  select string_agg(format('%s %s', c.conrelid::regclass, c.confdeltype), ', ' order by c.conrelid::regclass::text) into v_text
    from pg_constraint c
   where c.conrelid in ('public.hand_set_ratings'::regclass, 'public.rating_grants'::regclass) and c.contype = 'f';
  if v_text is distinct from 'hand_set_ratings c, rating_grants c' then
    raise exception 'FAIL (R3074001): the foreign keys are [%], expected each its profile''s, cascading', coalesce(v_text, '(none)');
  end if;

  insert into public.hand_set_ratings (profile_id, format, earned_rating, earned_deviation, earned_volatility, set_at)
  values (p1, 'constructed', 1000, 120, 0.06, now()), (p1, 'random', 1100, 130, 0.06, now()),
         (gone, 'constructed', 900, 200, 0.06, now());
  begin
    insert into public.hand_set_ratings (profile_id, format, earned_rating, earned_deviation, earned_volatility, set_at)
    values (p1, 'constructed', 2500, 120, 0.06, now());
    raise exception 'FAIL (R3074001): a second hand-set row of one ladder was taken';
  exception
    when unique_violation then null;
  end;
  begin
    insert into public.hand_set_ratings (profile_id, format, earned_rating, earned_deviation, earned_volatility, set_at)
    values (p1, 'conquest', 1000, 120, 0.06, now());
    raise exception 'FAIL (R3074001): a ladder that is none was taken';
  exception
    when check_violation then
      get stacked diagnostics v_con = constraint_name;
      if v_con is distinct from 'hand_set_ratings_format_check' then
        raise exception 'FAIL (R3074001): the ladder was refused by %', v_con;
      end if;
  end;

  -- The ledger: a set and a reset, reason dev, marked dev.
  insert into public.rating_grants (profile_id, format, kind, rating_before, deviation_before, volatility_before,
                                    rating_after, deviation_after, volatility_after, reason, dev, at)
  values (p1, 'constructed', 'set', 1000, 120, 0.06, 2000, 120, 0.06, 'dev', true, now()),
         (p1, 'constructed', 'reset', 2000, 120, 0.06, 1000, 120, 0.06, 'dev', true, now()),
         (gone, 'random', 'set', 900, 200, 0.06, 1500, 200, 0.06, 'dev', true, now());
  for v_text in select unnest(array['unmarked', 'kind', 'reason', 'format']) loop
    begin
      insert into public.rating_grants (profile_id, format, kind, rating_before, deviation_before, volatility_before,
                                        rating_after, deviation_after, volatility_after, reason, dev, at)
      values (p1,
              case when v_text = 'format' then 'conquest' else 'constructed' end,
              case when v_text = 'kind' then 'grant' else 'set' end,
              1, 1, 0.06, 2, 1, 0.06,
              case when v_text = 'reason' then 'game' else 'dev' end,
              v_text <> 'unmarked', now());
      raise exception 'FAIL (R3074001): the ledger took a row with a bad %', v_text;
    exception
      when check_violation then
        get stacked diagnostics v_con = constraint_name;
        if v_con is distinct from (case v_text when 'unmarked' then 'rating_grants_dev_check'
                                               when 'kind' then 'rating_grants_kind_check'
                                               when 'reason' then 'rating_grants_reason_check'
                                               else 'rating_grants_format_check' end) then
          raise exception 'FAIL (R3074001): the bad % was refused by %', v_text, v_con;
        end if;
    end;
  end loop;
  begin
    update public.rating_grants set rating_after = 2999 where profile_id = p1;
    raise exception 'FAIL (R3074001): a rating ledger row was updated';
  exception
    when raise_exception then
      if sqlerrm like 'FAIL%' then raise; end if;
      if sqlerrm is distinct from 'append-only table public.rating_grants may not be updated or deleted (attempted UPDATE)' then
        raise exception 'FAIL (R3074001): the update was refused by "%", not app.deny_row_mutation', sqlerrm;
      end if;
  end;

  -- A reset deletes the current-state row (no trigger keeps it).
  delete from public.hand_set_ratings where profile_id = p1 and format = 'constructed';
  if (select count(*) from public.hand_set_ratings where profile_id = p1) <> 1 then
    raise exception 'FAIL (R3074001): a reset did not leave the other ladder alone';
  end if;

  -- Both go with their account, and nobody else's.
  delete from auth.users where id = gone;
  if exists (select 1 from public.hand_set_ratings where profile_id = gone)
     or exists (select 1 from public.rating_grants where profile_id = gone) then
    raise exception 'FAIL (R3074001): a deleted account''s hand-set rating or ledger row outlived it';
  end if;
  if (select count(*) from public.rating_grants where profile_id = p1) <> 2 then
    raise exception 'FAIL (R3074001): deleting one account took another''s ledger rows';
  end if;
  raise notice 'OK (R3074001): one hand-set row per account and ladder, the ledger''s four checks and its append-only trigger, a reset''s delete, and both cascade with their account alone';
end $$;
rollback;

\echo '### R3074003: a post has a post- slug and text, and outlives its author ###'
begin;
insert into auth.users (id, email, email_confirmed_at)
values ('e3074003-0000-4000-8000-000000000001', 'r3074003-dev@example.test', now());
do $$
declare
  gone constant uuid := 'e3074003-0000-4000-8000-000000000001';
  refused constant text[] := array['news-1', 'post-', 'post-Upper', 'post--x', 'post-a b', 'post-a-', 'patch-v0-5-11d',
                                   E'post-a\n'];
  i     int;
  v_con text;
begin
  if not exists (select 1 from public.profiles where id = gone) then
    raise exception 'FAIL (R3074003): the author has no profile; the set-null below would prove nothing';
  end if;
  for i in 1 .. array_length(refused, 1) loop
    begin
      insert into public.news_posts (id, title, summary, posted_at, posted_by)
      values (refused[i], 'T', 'Fine.', now(), gone);
      raise exception 'FAIL (R3074003): the posts took the id %', refused[i];
    exception
      when check_violation then
        get stacked diagnostics v_con = constraint_name;
        if v_con is distinct from 'news_posts_id_check' then
          raise exception 'FAIL (R3074003): % was refused by %, not news_posts_id_check', refused[i], v_con;
        end if;
    end;
  end loop;
  begin
    insert into public.news_posts (id, title, summary, posted_at, posted_by) values ('post-blank', '  ', 'Fine.', now(), gone);
    raise exception 'FAIL (R3074003): a blank title was taken';
  exception
    when check_violation then
      get stacked diagnostics v_con = constraint_name;
      if v_con is distinct from 'news_posts_text_check' then
        raise exception 'FAIL (R3074003): the blank title was refused by %', v_con;
      end if;
  end;

  insert into public.news_posts (id, title, summary, posted_at, posted_by)
  values ('post-0a1b2c3d4e5f', 'Maintenance', 'The server restarts at noon.', now(), gone),
         ('post-a', 'A', 'B.', now(), gone);
  -- A take-down is an update of the row, which stays.
  update public.news_posts set taken_down_at = now() where id = 'post-a';
  -- Its author's deletion leaves the posts, naming nobody.
  delete from auth.users where id = gone;
  if (select count(*) from public.news_posts where posted_by is null) <> 2 then
    raise exception 'FAIL (R3074003): a deleted author''s posts did not stay with no author';
  end if;
  if (select taken_down_at from public.news_posts where id = 'post-a') is null then
    raise exception 'FAIL (R3074003): the take-down did not stay';
  end if;
  -- A claim of a post fits news_claims' slug check (0077).
  insert into public.news_claims (profile_id, news_id, claimed_at)
  values ('11111111-1111-1111-1111-111111111111', 'post-0a1b2c3d4e5f', now());
  raise notice 'OK (R3074003): % ids refused, a blank title refused, a post taken down stays, an author''s deletion forgets them, and a claim names one',
    array_length(refused, 1);
end $$;
rollback;

\echo '### R3074000: 0080 re-applies over rows of its own and changes none ###'
begin;
insert into public.admin_actions (profile_id, route, request, before_value, after_value, at)
values ('11111111-1111-1111-1111-111111111111', 'POST /api/dev/fill', '{}', 'null', '{}', now());
insert into public.news_posts (id, title, summary, posted_at) values ('post-kept', 'Kept', 'Still here.', now());
set local client_min_messages = warning;
\i /tmp/0080_dev_dashboard.sql
set local client_min_messages = notice;
do $$
begin
  if (select count(*) from public.admin_actions) <> 1 or (select count(*) from public.news_posts) <> 1 then
    raise exception 'FAIL (R3074000): re-applying 0080 changed the rows';
  end if;
  if (select count(*) from pg_trigger where tgrelid = 'public.admin_actions'::regclass and not tgisinternal) <> 1
     or (select count(*) from pg_trigger where tgrelid = 'public.rating_grants'::regclass and not tgisinternal) <> 1 then
    raise exception 'FAIL (R3074000): re-applying 0080 left other than one trigger on each ledger';
  end if;
  raise notice 'OK (R3074000): 0080 re-applies over its own rows and changes none';
end $$;
rollback;

\echo '=== ALL CHECKS RAN ==='
