-- A card pick keeps a card's Animated art (migration 0074, SPEC §14.2; R2715000; issue #2715).
-- Runs after 40_full_art_tiers.sql; profile 1 (11111111-…) exists by then, and
-- 34_cosmetic_loadouts.sql holds the table's columns, RLS and grants.
\set ON_ERROR_STOP on

-- Same rules as 01-40: every check raises on failure (`raise exception 'FAIL (…): …'`), every
-- expected refusal is matched on its sqlstate, each check that could pass vacuously carries a
-- guard, and every block that writes runs inside a transaction that is rolled back. Each SPEC §11
-- row this file proves is named in a `### Rnnn: … ###` heading.

\echo '### R2715000: a card pick keeps an animated art id ###'
begin;

do $$
declare
  p1     constant uuid := '11111111-1111-1111-1111-111111111111';
  v_text text;
  probe  text;
  probes constant text[][] := array[
    -- An Animated art's id that is no slug, or that the settings could never carry (R633's slug of
    -- at most 40 characters).
    ['an animated id that is no slug',
     format($q$insert into public.card_cosmetic_picks (profile_id, card_id, full_art, full_art_id)
                values (%L, 'core-004', true, 'animated-Mr')$q$, p1)],
    ['an animated id longer than 40 characters',
     format($q$insert into public.card_cosmetic_picks (profile_id, card_id, full_art, full_art_id)
                values (%L, 'core-004', true, %L)$q$, p1, 'animated-' || repeat('a', 32))],
    -- The same over-long id with the switch off, which holds no id at all.
    ['an over-long animated id with full_art false',
     format($q$insert into public.card_cosmetic_picks (profile_id, card_id, full_art, full_art_id)
                values (%L, 'core-004', false, %L)$q$, p1, 'animated-' || repeat('a', 32))],
    -- A well-shaped Animated art's id with the switch off names an art that never prints.
    ['an animated id with full_art false',
     format($q$insert into public.card_cosmetic_picks (profile_id, card_id, full_art, full_art_id)
                values (%L, 'core-004', false, 'animated-gary-the-gambler')$q$, p1)]];
  i int;
begin
  -- The constraint is there, and it is 0074's, so the refusals below are its own.
  select pg_get_constraintdef(oid)
    into v_text
    from pg_constraint
   where conrelid = 'public.card_cosmetic_picks'::regclass
     and conname = 'card_cosmetic_picks_full_art_id_check';
  if v_text is null then
    raise exception 'FAIL (R2715000): card_cosmetic_picks_full_art_id_check is missing';
  end if;
  if position('animated' in v_text) = 0 then
    raise exception 'FAIL (R2715000): card_cosmetic_picks_full_art_id_check is not 0074''s: %', v_text;
  end if;

  -- An Animated art's id beside a switch that is on is kept as written, beside a still one's.
  insert into public.card_cosmetic_picks (profile_id, card_id, full_art, full_art_id)
  values (p1, 'core-008', true, 'animated-mr-vanilla'),
         (p1, 'core-011', true, 'full-art-tempo-timmy-signed');
  select string_agg(format('%s %s %s', card_id, full_art, coalesce(full_art_id, '-')), ', ' order by card_id)
    into v_text
    from public.card_cosmetic_picks
   where profile_id = p1 and card_id in ('core-008', 'core-011');
  if v_text is distinct from 'core-008 t animated-mr-vanilla, core-011 t full-art-tempo-timmy-signed' then
    raise exception 'FAIL (R2715000): the picks read back as [%]', coalesce(v_text, '(none)');
  end if;

  for i in 1 .. array_length(probes, 1) loop
    probe := probes[i][1];
    begin
      execute probes[i][2];
      raise exception 'FAIL (R2715000): % was accepted', probe;
    exception when check_violation then
      null;
    end;
  end loop;

  raise notice 'OK (R2715000): an animated art id is kept beside full_art true, and refused beside false or when it is no slug of 40 characters at most';
end $$;

rollback;

\echo '### R2715000: 0074 re-applies and keeps the widened check ###'
begin;
\i /tmp/0074_animated_art.sql
do $$
declare
  p1 constant uuid := '11111111-1111-1111-1111-111111111111';
begin
  insert into public.card_cosmetic_picks (profile_id, card_id, full_art, full_art_id)
  values (p1, 'core-008', true, 'animated-mr-vanilla');
  begin
    insert into public.card_cosmetic_picks (profile_id, card_id, full_art, full_art_id)
    values (p1, 'core-004', true, 'animated-Mr');
    raise exception 'FAIL (R2715000): after re-applying 0074, animated-Mr was accepted';
  exception when check_violation then
    null;
  end;
  raise notice 'OK (R2715000): 0074 re-applies, and its check still keeps an animated art id and refuses a bad one';
end $$;
rollback;
