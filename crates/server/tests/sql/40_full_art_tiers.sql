-- A card pick keeps which full art it wears (migration 0072, SPEC §14.2; R2655000; issue #2655).
-- Runs after 39_practice_settles.sql; profile 1 (11111111-…) exists by then, and
-- 34_cosmetic_loadouts.sql holds the table's columns, RLS and grants.
\set ON_ERROR_STOP on

-- Same rules as 01-39: every check raises on failure (`raise exception 'FAIL (…): …'`), every
-- expected refusal is matched on its sqlstate, each check that could pass vacuously carries a
-- guard, and every block that writes runs inside a transaction that is rolled back. Each SPEC §11
-- row this file proves is named in a `### Rnnn: … ###` heading.

\echo '### R2655000: a card pick keeps its full art id ###'
begin;

do $$
declare
  p1     constant uuid := '11111111-1111-1111-1111-111111111111';
  v_text text;
  probe  text;
  probes constant text[][] := array[
    -- An id with the switch off, or with no switch, names a full art that never prints.
    ['an id with full_art false',
     format($q$insert into public.card_cosmetic_picks (profile_id, card_id, full_art, full_art_id)
                values (%L, 'core-004', false, 'full-art-gary-the-gambler-signed')$q$, p1)],
    ['an id with no full_art',
     format($q$insert into public.card_cosmetic_picks (profile_id, card_id, foil, full_art_id)
                values (%L, 'core-004', 'foil-rainbow', 'full-art-gary-the-gambler-signed')$q$, p1)],
    -- An id that is no full art's (`full-art-…`), or that the settings could never carry (R633's
    -- slug of at most 40 characters).
    ['an id that is no slug',
     format($q$insert into public.card_cosmetic_picks (profile_id, card_id, full_art, full_art_id)
                values (%L, 'core-004', true, 'full-art-Gary')$q$, p1)],
    ['an id that is no full art''s',
     format($q$insert into public.card_cosmetic_picks (profile_id, card_id, full_art, full_art_id)
                values (%L, 'core-004', true, 'foil-rainbow')$q$, p1)],
    ['an id longer than 40 characters',
     format($q$insert into public.card_cosmetic_picks (profile_id, card_id, full_art, full_art_id)
                values (%L, 'core-004', true, %L)$q$, p1, 'full-art-' || repeat('a', 32))]];
  i int;
begin
  -- The constraint is there, so the refusals below are its own.
  if not exists (select 1 from pg_constraint
                  where conrelid = 'public.card_cosmetic_picks'::regclass
                    and conname = 'card_cosmetic_picks_full_art_id_check') then
    raise exception 'FAIL (R2655000): card_cosmetic_picks_full_art_id_check is missing';
  end if;

  -- An id beside a switch that is on is kept as written, a full art this build does not know too.
  insert into public.card_cosmetic_picks (profile_id, card_id, full_art, full_art_id)
  values (p1, 'core-008', true, 'full-art-mr-vanilla-signed'),
         (p1, 'core-011', true, 'full-art-from-a-newer-client'),
         (p1, 'core-012', true, null);
  select string_agg(format('%s %s %s', card_id, full_art, coalesce(full_art_id, '-')), ', ' order by card_id)
    into v_text
    from public.card_cosmetic_picks
   where profile_id = p1 and card_id in ('core-008', 'core-011', 'core-012');
  if v_text is distinct from 'core-008 t full-art-mr-vanilla-signed, core-011 t full-art-from-a-newer-client, '
                             'core-012 t -' then
    raise exception 'FAIL (R2655000): the picks read back as [%]', coalesce(v_text, '(none)');
  end if;

  for i in 1 .. array_length(probes, 1) loop
    probe := probes[i][1];
    begin
      execute probes[i][2];
      raise exception 'FAIL (R2655000): % was accepted', probe;
    exception when check_violation then
      null;
    end;
  end loop;

  raise notice 'OK (R2655000): a full art id is kept beside full_art true, and refused beside false, with no switch, or when it is no full art''s slug';
end $$;

rollback;
