-- A card pick keeps a version and its augments (migration 0079, SPEC §14.2; R2725000; issue #2725).
-- Runs after 42_animated_art.sql; profile 1 (11111111-…) exists by then, and
-- 34_cosmetic_loadouts.sql holds the table's columns, RLS and grants.
\set ON_ERROR_STOP on

-- Same rules as 01-42: every check raises on failure (`raise exception 'FAIL (…): …'`), every
-- expected refusal is matched on its sqlstate, each check that could pass vacuously carries a
-- guard, and every block that writes runs inside a transaction that is rolled back. Each SPEC §11
-- row this file proves is named in a `### Rnnn: … ###` heading.

\echo '### R2725000: a card pick keeps a version id and a list of augments ###'
begin;

do $$
declare
  p1     constant uuid := '11111111-1111-1111-1111-111111111111';
  v_text text;
  probe  text;
  probes constant text[][] := array[
    -- A version's id that is no slug, longer than 40 characters, or beside a switch that is off.
    ['a version id that is no slug',
     format($q$insert into public.card_cosmetic_picks (profile_id, card_id, full_art, full_art_id)
                values (%L, 'core-004', true, 'version-Full')$q$, p1)],
    ['a version id longer than 40 characters',
     format($q$insert into public.card_cosmetic_picks (profile_id, card_id, full_art, full_art_id)
                values (%L, 'core-004', true, %L)$q$, p1, 'version-' || repeat('a', 33))],
    ['a version id with full_art false',
     format($q$insert into public.card_cosmetic_picks (profile_id, card_id, full_art, full_art_id)
                values (%L, 'core-004', false, 'version-full-art')$q$, p1)],
    -- An augments list that is empty, names no augment, holds a bad slug or an id over 40 characters.
    ['an empty augments list',
     format($q$insert into public.card_cosmetic_picks (profile_id, card_id, augments)
                values (%L, 'core-004', '{}'::text[])$q$, p1)],
    ['an augments list naming a foil',
     format($q$insert into public.card_cosmetic_picks (profile_id, card_id, augments)
                values (%L, 'core-004', array['foil-rainbow'])$q$, p1)],
    ['an augments list with a bad slug after a good one',
     format($q$insert into public.card_cosmetic_picks (profile_id, card_id, augments)
                values (%L, 'core-004', array['augment-animated', 'augment-Signed'])$q$, p1)],
    ['an augment id longer than 40 characters',
     format($q$insert into public.card_cosmetic_picks (profile_id, card_id, augments)
                values (%L, 'core-004', array['augment-' || repeat('a', 33)])$q$, p1)],
    ['an augment id with a comma in it',
     format($q$insert into public.card_cosmetic_picks (profile_id, card_id, augments)
                values (%L, 'core-004', array['augment-a,augment-b'])$q$, p1)],
    ['an augments list holding a NULL',
     format($q$insert into public.card_cosmetic_picks (profile_id, card_id, augments)
                values (%L, 'core-004', array['augment-animated', null])$q$, p1)]];
  i int;
begin
  -- The constraints are there, and they are 0079's, so the refusals below are their own.
  select pg_get_constraintdef(oid)
    into v_text
    from pg_constraint
   where conrelid = 'public.card_cosmetic_picks'::regclass
     and conname = 'card_cosmetic_picks_full_art_id_check';
  if v_text is null or position('version' in v_text) = 0 then
    raise exception 'FAIL (R2725000): card_cosmetic_picks_full_art_id_check is not 0079''s: %', coalesce(v_text, '(missing)');
  end if;
  if not exists (select 1 from pg_constraint
                  where conrelid = 'public.card_cosmetic_picks'::regclass
                    and conname = 'card_cosmetic_picks_augments_check') then
    raise exception 'FAIL (R2725000): card_cosmetic_picks_augments_check is missing';
  end if;

  -- A version's id beside a switch that is on, augments beside it, and a row of augments alone are kept as
  -- written, and so is a still Epic's row beside them.
  insert into public.card_cosmetic_picks (profile_id, card_id, full_art, full_art_id, augments)
  values (p1, 'core-002', true, 'version-full-art', array['augment-animated', 'augment-signature']),
         (p1, 'core-003', null, null, array['augment-signature']),
         (p1, 'core-008', true, null, null);
  select string_agg(format('%s %s %s %s', card_id, coalesce(full_art::text, '-'), coalesce(full_art_id, '-'),
                           coalesce(array_to_string(augments, '+'), '-')), ', ' order by card_id)
    into v_text
    from public.card_cosmetic_picks
   where profile_id = p1 and card_id in ('core-002', 'core-003', 'core-008');
  if v_text is distinct from 'core-002 true version-full-art augment-animated+augment-signature, '
                             'core-003 - - augment-signature, core-008 true - -' then
    raise exception 'FAIL (R2725000): the picks read back as [%]', coalesce(v_text, '(none)');
  end if;

  for i in 1 .. array_length(probes, 1) loop
    probe := probes[i][1];
    begin
      execute probes[i][2];
      raise exception 'FAIL (R2725000): % was accepted', probe;
    exception when check_violation then
      null;
    end;
  end loop;

  raise notice 'OK (R2725000): a version id and a list of augments are kept, a row of augments alone too, and bad shapes are refused';
end $$;

rollback;

\echo '### R2725000: 0079 re-applies and keeps its checks ###'
begin;
\i /tmp/0079_card_versions.sql
do $$
declare
  p1 constant uuid := '11111111-1111-1111-1111-111111111111';
begin
  insert into public.card_cosmetic_picks (profile_id, card_id, full_art, full_art_id, augments)
  values (p1, 'core-002', true, 'version-full-art', array['augment-animated']);
  begin
    insert into public.card_cosmetic_picks (profile_id, card_id, augments)
    values (p1, 'core-004', array['foil-rainbow']);
    raise exception 'FAIL (R2725000): after re-applying 0079, an augments list naming a foil was accepted';
  exception when check_violation then
    null;
  end;
  raise notice 'OK (R2725000): 0079 re-applies, and its checks still keep a version and augments and refuse a bad one';
end $$;
rollback;
