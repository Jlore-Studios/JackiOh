-- 2v2 between people in a private lobby (migration 0091, SPEC §9.5, §13.9; R3731000, R3731003; issue #3731).
-- Runs after 54_lobbies.sql; profiles 1 and 2 (11111111-…, 22222222-…) are active by then (03 activated them),
-- and the two more people a 2v2 needs are made here, as 23_duos.sql makes its own.
\set ON_ERROR_STOP on

-- Same rules as 01-54: every check raises on failure (`raise exception 'FAIL (…): …'`), every expected refusal is
-- matched on its sqlstate, each check that could pass vacuously carries a guard, and every block that writes runs
-- inside a transaction that is rolled back.

\echo '### R3731000: a 2v2 lobby keeps its other team, and no one sits in two of its seats ###'
begin;
insert into auth.users (id, email, email_confirmed_at)
values ('d3731000-0000-4000-8000-000000000003', 'two-v-two-3@example.test', now()),
       ('d3731000-0000-4000-8000-000000000004', 'two-v-two-4@example.test', now());
do $$
declare
  v_host  constant uuid := '11111111-1111-1111-1111-111111111111';
  v_guest constant uuid := '22222222-2222-2222-2222-222222222222';
  third   constant uuid := 'd3731000-0000-4000-8000-000000000003';
  fourth  constant uuid := 'd3731000-0000-4000-8000-000000000004';
  v_text text;
  refused_by text;
begin
  select string_agg(format('%s %s %s', column_name, data_type, is_nullable), ', ' order by column_name collate "C")
    into v_text from information_schema.columns
   where table_schema = 'public' and table_name = 'lobbies' and column_name like 'guest%';
  if v_text is distinct from 'guest uuid YES, guest2 uuid YES, guest2_pick jsonb YES, guest2_ready boolean NO, '
                             'guest3 uuid YES, guest3_pick jsonb YES, guest3_ready boolean NO, '
                             'guest_pick jsonb YES, guest_ready boolean NO' then
    raise exception 'FAIL (R3731000): public.lobbies has the guest columns [%]', coalesce(v_text, '(none)');
  end if;
  if (select count(*) from public.profiles where id in (third, fourth)) <> 2 then
    raise exception 'FAIL (R3731000): the two more people have no profile; the checks below would prove nothing';
  end if;

  insert into public.lobbies (code, host, guest, guest2, guest3, guest2_pick, guest3_ready, settings,
                              catalog_version, created_at, expires_at)
  values ('LOBTWO', v_host, v_guest, third, fourth, '{"choice":{"random":true}}', true, '{"format":"2v2"}', 'check',
          now(), now() + interval '30 minutes');
  if not exists (select 1 from public.lobbies
                  where code = 'LOBTWO' and guest2 = third and guest3 = fourth
                    and guest2_pick is not null and not guest2_ready and guest3_ready) then
    raise exception 'FAIL (R3731000): a 2v2 lobby did not keep its other team';
  end if;

  -- Each pair of seats, one at a time: the same person in two is refused by the check.
  foreach v_text in array array['guest2 = host', 'guest3 = host', 'guest2 = guest', 'guest3 = guest',
                                'guest3 = guest2'] loop
    begin
      execute format('update public.lobbies set %s where code = %L', v_text, 'LOBTWO');
      raise exception 'FAIL (R3731000): a lobby took % (one person in two seats)', v_text;
    exception
      when check_violation then
        get stacked diagnostics refused_by = constraint_name;
        if refused_by is distinct from 'lobbies_members_distinct_check' then
          raise exception 'FAIL (R3731000): % was refused by "%"', v_text, refused_by;
        end if;
    end;
  end loop;

  if (select confdeltype from pg_constraint where conname = 'lobbies_guest2_fkey') is distinct from 'c'
     or (select confdeltype from pg_constraint where conname = 'lobbies_guest3_fkey') is distinct from 'c' then
    raise exception 'FAIL (R3731000): public.lobbies'' other team does not cascade with its profiles';
  end if;
  delete from auth.users where id = fourth;
  if exists (select 1 from public.lobbies where code = 'LOBTWO') then
    raise exception 'FAIL (R3731000): a deleted guest3 left the lobby they sat in';
  end if;
  raise notice 'OK (R3731000): a 2v2 lobby keeps its other team, seats no one twice, and goes with any of its people';
end $$;
rollback;

\echo '### R3731003: a 2v2 match keeps team B''s people, and a live one keeps them both ###'
begin;
insert into auth.users (id, email, email_confirmed_at)
values ('d3731000-0000-4000-8000-000000000013', 'two-v-two-13@example.test', now()),
       ('d3731000-0000-4000-8000-000000000014', 'two-v-two-14@example.test', now());
do $$
declare
  v_a1 constant uuid := '11111111-1111-1111-1111-111111111111';
  v_a2 constant uuid := '22222222-2222-2222-2222-222222222222';
  v_b1 constant uuid := 'd3731000-0000-4000-8000-000000000013';
  v_b2 constant uuid := 'd3731000-0000-4000-8000-000000000014';
  mid  constant uuid := 'd3731000-0000-4000-8000-000000000017';
  coop constant uuid := 'd3731000-0000-4000-8000-000000000018';
  v_text text;
  refused_by text;
begin
  select string_agg(format('%s %s %s', column_name, data_type, is_nullable), ', ' order by column_name collate "C")
    into v_text from information_schema.columns
   where table_schema = 'public' and table_name = 'duos_matches' and column_name in ('b1', 'b2');
  if v_text is distinct from 'b1 uuid YES, b2 uuid YES' then
    raise exception 'FAIL (R3731003): public.duos_matches has the team B columns [%]', coalesce(v_text, '(none)');
  end if;
  if (select count(*) from public.profiles where id in (v_b1, v_b2)) <> 2 then
    raise exception 'FAIL (R3731003): team B has no profiles; the checks below would prove nothing';
  end if;

  -- A co-op match holds no team B people, as every match before 0091 did.
  insert into public.duos_matches (id, seed, a1, a2, difficulty, options)
  values (coop, 'seed-coop', v_a1, v_a2, 'easy', '{}');
  insert into public.duos_matches (id, seed, a1, a2, b1, b2, difficulty, options)
  values (mid, 'seed-2v2', v_a1, v_a2, v_b1, v_b2, 'easy', '{"teamB":"people"}');
  if not exists (select 1 from public.duos_matches where id = mid and b1 = v_b1 and b2 = v_b2)
     or not exists (select 1 from public.duos_matches where id = coop and b1 is null and b2 is null) then
    raise exception 'FAIL (R3731003): a match did not keep its team B as written';
  end if;

  -- A live 2v2 holds both or neither of team B's people.
  begin
    insert into public.duos_matches (id, seed, a1, a2, b1, difficulty, options)
    values ('d3731000-0000-4000-8000-000000000019', 'seed-half', v_a1, v_a2, v_b1, 'easy', '{}');
    raise exception 'FAIL (R3731003): a live match took one team B person';
  exception
    when check_violation then
      get stacked diagnostics refused_by = constraint_name;
      if refused_by is distinct from 'duos_matches_team_b_paired_unless_over_check' then
        raise exception 'FAIL (R3731003): a half team B was refused by "%"', refused_by;
      end if;
  end;

  -- R1497000: team B's person in a live 2v2 is not deleted; once it has ended, the seat empties.
  begin
    delete from auth.users where id = v_b2;
    raise exception 'FAIL (R3731003): team B''s person in a live 2v2 was deleted';
  exception
    when check_violation then
      get stacked diagnostics refused_by = constraint_name;
      if refused_by is distinct from 'duos_matches_team_b_paired_unless_over_check' then
        raise exception 'FAIL (R3731003): the live 2v2''s delete was refused by "%"', refused_by;
      end if;
  end;
  if not exists (select 1 from public.profiles where id = v_b2) then
    raise exception 'FAIL (R3731003): the refused delete took the profile';
  end if;
  update public.duos_matches set ended_at = now(), result = '{"outcome":"draw"}' where id = mid;
  delete from auth.users where id = v_b2;
  if not exists (select 1 from public.duos_matches where id = mid and b1 = v_b1 and b2 is null) then
    raise exception 'FAIL (R3731003): once the 2v2 ended, the delete did not empty team B''s seat';
  end if;
  raise notice 'OK (R3731003): a 2v2 match keeps team B''s people, a live one both of them, and an ended one lets one go';
end $$;
rollback;
