-- The card patches' new names (migration 0025, issue #290, SPEC §11 R743). Runs after
-- 13_glitch.sql; profiles 1 and 2 are active by then (03 activated them).
\set ON_ERROR_STOP on

-- Same rules as 01-13: every check raises on failure and runs inside a transaction that is rolled
-- back. Every row the suite seeds carries `core-1`, so 0025 renamed nothing when run.sh applied
-- it; this check seeds rows under the old names, as a database that served them holds them, and
-- applies 0025 again over them, the way 04 re-applies 0007.

\echo '### R743: 0025 renames each row filed under a card patch''s old name, oldest name first ###'
begin;
do $$
declare
  p1 constant uuid := '11111111-1111-1111-1111-111111111111';
  p2 constant uuid := '22222222-2222-2222-2222-222222222222';
  -- A game record as the store writes one: its id, source, mode and patch copied into it.
  rec_of constant text := '{"pilots": {"p1": "ai", "p2": "ai"}, "game": {"turns": 9}}';
begin
  -- Two of the names are reused for other catalogs (v0.2.4, v0.2.5): a row under each old one.
  insert into public.cards (id, card_index, name, set_id, type, tags, rarity, token, cost, catalog_version)
  values ('r743-a', '743.1', 'Card under v0.2.4', 'Core', 'Unit', '{}', 'Common', false, '1'::jsonb, 'v0.2.4'),
         ('r743-b', '743.2', 'Card under v0.2.11', 'Core', 'Unit', '{}', 'Common', false, '1'::jsonb, 'v0.2.11'),
         ('r743-c', '743.3', 'Card under v0.2.16', 'Core', 'Unit', '{}', 'Common', false, '1'::jsonb, 'v0.2.16'),
         ('r743-d', '743.4', 'Card under v0.2.17', 'Core', 'Unit', '{}', 'Common', false, '1'::jsonb, 'v0.2.17');
  -- The stamp db:seed-catalog left: its launch grant (0016) gives r743-d to every active account now.
  update app.settings set value = '"v0.2.17"' where key = 'catalog_version';

  insert into public.decks (id, profile_id, name, cards, catalog_version)
  values ('d7430000-0000-4000-8000-000000000001', p1, 'Under v0.2.5', '["core-001"]', 'v0.2.5'),
         ('d7430000-0000-4000-8000-000000000002', p1, 'Under core-1', '["core-001"]', 'core-1');
  insert into public.tickets (id, profile_id, rating, frozen_deck, catalog_version, status)
  values ('a7430000-0000-4000-8000-000000000001', p1, 1000, '["core-001"]', 'v0.2.13', 'cancelled');
  insert into public.matches (id, status, seed, p1_profile_id, p2_profile_id, p1_deck, p2_deck,
                              catalog_version, ceiling_at, started_at, ended_at)
  values ('e7430000-0000-4000-8000-000000000005', 'over', 'r743-5', p1, p2, '[]', '[]', 'v0.2.5', now(), now(), now()),
         ('e7430000-0000-4000-8000-000000000012', 'over', 'r743-12', p1, p2, '[]', '[]', 'v0.2.12', now(), now(), now()),
         ('e7430000-0000-4000-8000-000000000020', 'over', 'r743-20', p1, p2, '[]', '[]', 'v0.2.0', now(), now(), now()),
         ('e7430000-0000-4000-8000-000000000016', 'over', 'r743-16b', p1, p2, '[]', '[]', 'v0.2.16b', now(), now(), now());
  insert into public.series (id, p1_profile_id, p2_profile_id, status, next_match_id, version, catalog_version, state)
  values ('f7430000-0000-4000-8000-000000000001', p1, p2, 'playing', 'f7430000-0000-4000-8000-0000000000a1', 3,
          'v0.2.10', '{"sides": [], "games": [], "seedBase": "s"}');

  -- A live record keeps its match id; a development record's id names its patch (R378), and the
  -- one under v0.2.11 takes the id the one under the old v0.2.4 leaves.
  insert into public.game_records (id, source, mode, patch, record)
  select g.id, g.source, 'random', g.patch,
         rec_of::jsonb || jsonb_build_object('id', g.id, 'source', g.source, 'mode', 'random', 'patch', g.patch)
    from (values ('e7430000-0000-4000-8000-0000000000aa', 'live', 'v0.2.4'),
                 ('dev:v0.2.4:r743:1', 'dev', 'v0.2.4'),
                 ('dev:v0.2.11:r743:1', 'dev', 'v0.2.11'),
                 ('dev:v0.2.14:r743:1', 'dev', 'v0.2.14'),
                 ('dev:v0.2.14b:r743:1', 'dev', 'v0.2.14b'),
                 ('dev:v0.2.16b:r743:1', 'dev', 'v0.2.16b'),
                 ('dev:v0.2.17:r743:1', 'dev', 'v0.2.17'),
                 ('dev:v0.2.0:r743:1', 'dev', 'v0.2.0')) as g (id, source, patch);

  insert into public.seasons (id, patch_version, started_at) values ('r743', 'v0.2.10', now());
  insert into public.rated_games (id, kind, season_id, patch_version, catalog_version,
    p1_profile_id, p1_bot_id, p1_pilot, p1_before, p1_after, p1_rank_before, p1_rank_after,
    p2_profile_id, p2_bot_id, p2_pilot, p2_before, p2_after, p2_rank_before, p2_rank_after,
    winner_side, reason, ended_at)
  select g.id, 'match', 'r743', g.patch, g.catalog, p1, null, 'human',
         '{"rating":1000,"deviation":350,"volatility":0.06}'::jsonb,
         '{"rating":1016,"deviation":340,"volatility":0.06}'::jsonb, null, null,
         null, 'ai-easy', 'ai',
         '{"rating":1000,"deviation":350,"volatility":0.06}'::jsonb,
         '{"rating":984,"deviation":340,"volatility":0.06}'::jsonb, null, null,
         0, 'hero-death', now()
    from (values ('a7430000-0000-4000-8000-0000000000a1'::uuid, 'v0.2.11', 'v0.2.12'),
                 ('a7430000-0000-4000-8000-0000000000a2'::uuid, 'v0.2.4', 'v0.2.5')) as g (id, patch, catalog);

  -- Append-only (0012): its launch reference keeps the name it was written with.
  insert into public.collection_grants (profile_id, card_id, delta, reason, ref)
  values (p2, 'core-002', 1, 'launch', 'launch:v0.2.10');
  perform set_config('r743.grants', (select count(*)::text from public.collection_grants), true);
end $$;

\i /tmp/0025_patch_retcon.sql

do $$
declare
  v_text text;
  v_int  int;
  v_ids  text[];
  v_row  record;
  v_rec  jsonb;
begin
  -- Cards and the stamp move together, so the launch grant the stamp fires found nothing to add.
  select array_agg(id || '=' || catalog_version order by id collate "C") into v_ids from public.cards where id like 'r743-%';
  if v_ids <> array['r743-a=v0.2.1', 'r743-b=v0.2.4', 'r743-c=v0.2.8', 'r743-d=v0.2.9'] then
    raise exception 'FAIL (R743): the cards read %, expected v0.2.4 -> v0.2.1, v0.2.11 -> v0.2.4, v0.2.16 -> v0.2.8 and v0.2.17 -> v0.2.9', v_ids;
  end if;
  if app.catalog_version() is distinct from 'v0.2.9' then
    raise exception 'FAIL (R743): app.settings names catalog %, expected v0.2.9', app.catalog_version();
  end if;
  select count(*) into v_int from public.collection_grants;
  if v_int::text <> current_setting('r743.grants') then
    raise exception 'FAIL (R743): the renamed stamp granted cards again (% grants, was %)', v_int, current_setting('r743.grants');
  end if;

  -- The old v0.2.5 is v0.2.2 and the old v0.2.12 takes v0.2.5: neither is renamed twice.
  select array_agg(name || '=' || catalog_version order by name collate "C") into v_ids from public.decks
   where id in ('d7430000-0000-4000-8000-000000000001', 'd7430000-0000-4000-8000-000000000002');
  if v_ids <> array['Under core-1=core-1', 'Under v0.2.5=v0.2.2'] then
    raise exception 'FAIL (R743): the decks read %', v_ids;
  end if;
  select catalog_version into v_text from public.tickets where id = 'a7430000-0000-4000-8000-000000000001';
  if v_text is distinct from 'v0.2.6' then
    raise exception 'FAIL (R743): the ticket under v0.2.13 reads %, expected v0.2.6', v_text;
  end if;
  select array_agg(seed || '=' || catalog_version order by seed collate "C") into v_ids from public.matches where seed like 'r743-%';
  if v_ids <> array['r743-12=v0.2.5', 'r743-16b=v0.2.8b', 'r743-20=v0.2.0', 'r743-5=v0.2.2'] then
    raise exception 'FAIL (R743): the matches read %, expected v0.2.12 -> v0.2.5, v0.2.16b -> v0.2.8b, v0.2.5 -> v0.2.2 and v0.2.0 kept', v_ids;
  end if;

  -- R263: the series' rename is a write, so it takes the next version.
  select catalog_version, version into v_row from public.series where id = 'f7430000-0000-4000-8000-000000000001';
  if v_row.catalog_version is distinct from 'v0.2.3' or v_row.version <> 4 then
    raise exception 'FAIL (R743): the series reads % at version %, expected v0.2.3 at version 4', v_row.catalog_version, v_row.version;
  end if;

  -- Game records: the patch, its copy in the record and a development id's patch move together
  -- (game_records_copies_check), and the rest of the record stays.
  select array_agg(id || '=' || patch order by id collate "C") into v_ids from public.game_records
   where id like 'dev:%:r743:%' or id = 'e7430000-0000-4000-8000-0000000000aa';
  if v_ids <> array['dev:v0.2.0:r743:1=v0.2.0', 'dev:v0.2.1:r743:1=v0.2.1', 'dev:v0.2.4:r743:1=v0.2.4',
                    'dev:v0.2.7:r743:1=v0.2.7', 'dev:v0.2.7b:r743:1=v0.2.7b', 'dev:v0.2.8b:r743:1=v0.2.8b',
                    'dev:v0.2.9:r743:1=v0.2.9', 'e7430000-0000-4000-8000-0000000000aa=v0.2.1'] then
    raise exception 'FAIL (R743): the game records read %', v_ids;
  end if;
  select g.record into v_rec from public.game_records g where g.id = 'dev:v0.2.4:r743:1';
  if v_rec ->> 'patch' <> 'v0.2.4' or v_rec ->> 'id' <> 'dev:v0.2.4:r743:1'
     or v_rec -> 'game' <> '{"turns": 9}'::jsonb or v_rec -> 'pilots' -> 'p1' <> '"ai"'::jsonb then
    raise exception 'FAIL (R743): the record once under v0.2.11 reads %', v_rec;
  end if;

  select patch_version into v_text from public.seasons where id = 'r743';
  if v_text is distinct from 'v0.2.3' then
    raise exception 'FAIL (R743): the season opened by v0.2.10 reads %, expected v0.2.3', v_text;
  end if;
  select array_agg(patch_version || '/' || catalog_version order by id) into v_ids from public.rated_games where season_id = 'r743';
  if v_ids <> array['v0.2.4/v0.2.5', 'v0.2.1/v0.2.2'] then
    raise exception 'FAIL (R743): the rated games read %, expected v0.2.11/v0.2.12 -> v0.2.4/v0.2.5 and v0.2.4/v0.2.5 -> v0.2.1/v0.2.2', v_ids;
  end if;

  if not exists (select 1 from public.collection_grants where ref = 'launch:v0.2.10') then
    raise exception 'FAIL (R743): a launch grant''s reference was rewritten, but collection_grants is append-only';
  end if;
  raise notice 'OK (R743): every old name renamed once, oldest first, the stamp and the grants in step';
end $$;
rollback;
