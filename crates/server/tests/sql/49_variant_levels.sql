-- Card variants as ladders of levels (migration 0083, SPEC §14.2, §14.11; R3015000, R3015006; issue #3015).
-- Runs after 48_drop_custom_cards.sql; profile 1 (11111111-…) exists by then. The first block checks the
-- registry's new rows; the second gives profile 1 one card per combination an account could own before
-- #3015, applies the file again twice (/tmp/0083_variant_levels.sql, which run.sh copied there) and reads
-- the levels and ledger rows it wrote.
\set ON_ERROR_STOP on

-- Same rules as 01-48: every check raises on failure (`raise exception 'FAIL (…): …'`), each check that
-- could pass vacuously carries a guard, and every block runs inside a transaction that is rolled back.
-- Nothing here selects into the log: the do blocks raise or say OK.

\echo '### R3015000: 0083 wrote the fifteen new levels of public.cosmetics ###'
begin;
do $$
declare
  v_text text;
begin
  -- The registry's entries, as crates/server/cosmetics.json has them: each a version, none a default,
  -- a chase exactly at Legendary and above.
  select string_agg(format('%s %s %s %s', id, rarity, is_default::text, chase::text), ', ' order by id)
    into v_text from public.cosmetics
   where id in ('version-animated', 'version-full-art-animated', 'version-full-art-signed',
                'version-alt-full-art-animated', 'version-alt-full-art-signed', 'version-sketch-full-art',
                'version-sketch-full-art-animated', 'version-sketch-full-art-signed',
                'version-extended-art-animated', 'version-pixel-animated', 'version-comic-animated',
                'version-set-showcase-animated', 'version-out-of-frame-animated', 'version-immersive-animated',
                'version-prismatic-animated')
     and kind = 'version';
  if v_text is distinct from
     'version-alt-full-art-animated Mythic false true, version-alt-full-art-signed Mythic false true, '
     'version-animated Epic false false, version-comic-animated Legendary false true, '
     'version-extended-art-animated Legendary false true, version-full-art-animated Mythic false true, '
     'version-full-art-signed Mythic false true, version-immersive-animated Mythic false true, '
     'version-out-of-frame-animated Mythic false true, version-pixel-animated Legendary false true, '
     'version-prismatic-animated Mythic false true, version-set-showcase-animated Mythic false true, '
     'version-sketch-full-art Epic false false, version-sketch-full-art-animated Legendary false true, '
     'version-sketch-full-art-signed Mythic false true' then
    raise exception 'FAIL (R3015000): the new levels'' rows of public.cosmetics are [%]', coalesce(v_text, '(none)');
  end if;
  raise notice 'OK (R3015000): the fifteen new levels are in public.cosmetics';
end $$;
rollback;

\echo '### R3015006: 0083 turns every combination held into its levels, once, and takes nothing ###'
begin;
-- public.cosmetics is the registry's projection, which seed-catalog writes and this suite does not: the
-- entries held before #3015 stand in for it, a full-art card's four retired full arts among them.
insert into public.cosmetics (id, kind, name, rarity, chase)
values ('version-full-art', 'version', 'Full art', 'Legendary', true),
       ('version-alt-art', 'version', 'Alt art', 'Rare', false),
       ('version-sketch', 'version', 'Sketch', 'Rare', false),
       ('version-alt-full-art', 'version', 'Alt full art', 'Legendary', true),
       ('version-pixel', 'version', 'Pixel', 'Epic', false),
       ('augment-animated', 'augment', 'Animated', 'Epic', false),
       ('augment-signature', 'augment', 'Signature', 'Mythic', true),
       ('full-art-r3015', 'fullArt', 'R3015 full art', 'Epic', false),
       ('full-art-r3015-animated', 'fullArt', 'R3015 animated full art', 'Legendary', true),
       ('full-art-r3015-signed', 'fullArt', 'R3015 signed full art', 'Mythic', true),
       ('animated-r3015', 'fullArt', 'R3015 animated art', 'Epic', false)
on conflict (id) do nothing;
-- One card per combination, and a Legacy card, whose picture never moves (R2724000).
insert into public.cards (id, card_index, name, set_id, type, tags, rarity, token, cost, catalog_version)
select format('r3015-%s', lpad(n::text, 2, '0')), format('3015.%s', n), format('R3015 card %s', n), 'Core', 'Unit',
       case when n = 16 then '{Legacy}'::text[] else '{}'::text[] end, 'Rare', false, '1'::jsonb, 'core-r3015'
  from generate_series(1, 16) as n;
-- What profile 1 holds on each, as the store wrote it before #3015, each row beside its ledger row.
with held(card_id, cosmetic_id) as (
  values ('r3015-01', 'version-full-art'), ('r3015-01', 'augment-animated'),
         ('r3015-02', 'version-full-art'), ('r3015-02', 'augment-animated'), ('r3015-02', 'augment-signature'),
         ('r3015-03', 'full-art-r3015-signed'),
         ('r3015-04', 'augment-animated'),
         ('r3015-05', 'animated-r3015'),
         ('r3015-06', 'version-alt-full-art'),
         ('r3015-07', 'version-sketch'),
         ('r3015-08', 'version-alt-art'), ('r3015-08', 'augment-animated'),
         ('r3015-09', 'version-sketch'), ('r3015-09', 'augment-animated'),
         ('r3015-10', 'augment-signature'),
         ('r3015-11', 'full-art-r3015'),
         ('r3015-12', 'full-art-r3015-animated'),
         ('r3015-13', 'version-alt-full-art'), ('r3015-13', 'augment-animated'),
         ('r3015-14', 'version-pixel'), ('r3015-14', 'augment-animated'),
         ('r3015-15', 'version-full-art'),
         ('r3015-16', 'augment-animated')
),
rows as (
  insert into public.card_cosmetics (profile_id, card_id, cosmetic_id)
  select '11111111-1111-1111-1111-111111111111', card_id, cosmetic_id from held
  returning profile_id, card_id, cosmetic_id
)
insert into public.cosmetic_grants (profile_id, card_id, cosmetic_id, delta, reason, ref)
select profile_id, card_id, cosmetic_id, 1, 'pack_open', 'r3015:before' from rows;

do $$
declare
  n bigint;
begin
  select count(*) into n from public.card_cosmetics
   where profile_id = '11111111-1111-1111-1111-111111111111' and card_id like 'r3015-%';
  if n <> 23 then
    raise exception 'FAIL (R3015006): profile 1 holds % rows on the fixture cards before 0083 runs again, not 23', n;
  end if;
end $$;

-- The migration, as it runs on a database these rows are held in, twice: the second finds every level
-- held and writes nothing.
set local client_min_messages = warning;
\i /tmp/0083_variant_levels.sql
\i /tmp/0083_variant_levels.sql
set local client_min_messages = notice;

do $$
declare
  p1       constant uuid := '11111111-1111-1111-1111-111111111111';
  expected constant text[][] := array[
    -- Full art + Animated: Full art's level 2 (and so its level 1), and Default's animated level.
    ['r3015-01', 'version-animated, version-full-art, version-full-art-animated'],
    -- Full art + Animated + Signature: Full art's level 3.
    ['r3015-02', 'version-animated, version-full-art, version-full-art-animated, version-full-art-signed'],
    -- A retired Mythic full art: Full art's level 3.
    ['r3015-03', 'version-animated, version-full-art, version-full-art-animated, version-full-art-signed'],
    -- Animated on the normal face, and the retired Animated art: Default's level 2.
    ['r3015-04', 'version-animated'],
    ['r3015-05', 'version-animated'],
    -- Alt full art: Alt art's level 2, and so its level 1.
    ['r3015-06', 'version-alt-art, version-alt-full-art'],
    -- Sketch: Sketch's level 1.
    ['r3015-07', 'version-sketch'],
    -- Alt art or Sketch with Animated: its level 1, and Default's animated level.
    ['r3015-08', 'version-alt-art, version-animated'],
    ['r3015-09', 'version-animated, version-sketch'],
    -- Signature alone: no level.
    ['r3015-10', ''],
    -- A retired still Epic: Full art's level 1; a retired Legendary: its level 2.
    ['r3015-11', 'version-full-art'],
    ['r3015-12', 'version-animated, version-full-art, version-full-art-animated'],
    -- Alt full art + Animated: Alt art's level 3.
    ['r3015-13', 'version-alt-art, version-alt-full-art, version-alt-full-art-animated, version-animated'],
    -- A one-art version + Animated: its level 2.
    ['r3015-14', 'version-animated, version-pixel, version-pixel-animated'],
    -- Full art alone: its level 1, nothing more.
    ['r3015-15', 'version-full-art'],
    -- Animated on a Legacy card: no level that moves.
    ['r3015-16', '']];
  i      int;
  v_text text;
  n      bigint;
begin
  for i in 1 .. array_length(expected, 1) loop
    -- The levels held (and no retired row lost), then the ledger rows 0083 wrote.
    select coalesce(string_agg(cosmetic_id, ', ' order by cosmetic_id), '') into v_text
      from public.card_cosmetics
     where profile_id = p1 and card_id = expected[i][1] and cosmetic_id like 'version-%';
    if v_text is distinct from expected[i][2] then
      raise exception 'FAIL (R3015006): % holds the levels [%], not [%]', expected[i][1], v_text, expected[i][2];
    end if;
    select count(*) into n from public.cosmetic_grants g
     where g.profile_id = p1 and g.card_id = expected[i][1] and g.reason = 'launch' and g.ref = 'levels:0083'
       and g.delta = 1
       and not exists (select 1 from public.cosmetic_grants b
                        where b.profile_id = p1 and b.card_id = g.card_id and b.cosmetic_id = g.cosmetic_id
                          and b.ref = 'r3015:before');
    if n <> (select count(*) from public.card_cosmetics c
              where c.profile_id = p1 and c.card_id = expected[i][1]
                and not exists (select 1 from public.cosmetic_grants b
                                 where b.profile_id = p1 and b.card_id = c.card_id and b.cosmetic_id = c.cosmetic_id
                                   and b.ref = 'r3015:before')) then
      raise exception 'FAIL (R3015006): % has % ledger rows of 0083, not one per level it wrote', expected[i][1], n;
    end if;
  end loop;

  -- Every row held before stays, and each ledger still sums to what is held.
  select count(*) into n from public.card_cosmetics
   where profile_id = p1 and card_id like 'r3015-%' and cosmetic_id not like 'version-%';
  if n <> 14 then
    raise exception 'FAIL (R3015006): % of the 14 retired or augment rows held before 0083 remain', n;
  end if;
  select count(*) into n from (
    select c.card_id, c.cosmetic_id from public.card_cosmetics c where c.profile_id = p1 and c.card_id like 'r3015-%'
    except
    select g.card_id, g.cosmetic_id from public.cosmetic_grants g
     where g.profile_id = p1 and g.card_id like 'r3015-%'
     group by g.card_id, g.cosmetic_id having sum(g.delta) = 1
  ) as unledgered;
  if n <> 0 then
    raise exception 'FAIL (R3015006): % held rows on the fixture cards have no ledger summing to one', n;
  end if;
  select count(*) into n from public.cosmetic_grants
   where profile_id = p1 and card_id like 'r3015-%' and ref = 'levels:0083';
  if n <> 23 then
    raise exception 'FAIL (R3015006): 0083 wrote % ledger rows over its two runs, not the 23 levels the first granted', n;
  end if;

  raise notice 'OK (R3015006): 0083 maps every combination held to its levels, once, and every row held stays';
end $$;
rollback;
