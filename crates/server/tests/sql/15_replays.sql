-- Online replays (migration 0028, issue #510, SPEC §9.3, R768). Runs after 14_patch_retcon.sql;
-- profiles 1 and 2 are active by then (03 activated them).
\set ON_ERROR_STOP on

-- Same rules as 01-14: every check raises on failure and runs inside a transaction that is rolled
-- back.

\echo '### R768: a finished match''s final hash is null until written, written once, and server-only ###'
begin;
do $$
declare
  p1     constant uuid := '11111111-1111-1111-1111-111111111111';
  p2     constant uuid := '22222222-2222-2222-2222-222222222222';
  mid    constant uuid := 'eeeeeeee-0000-0000-0000-000000000768';
  v_hash text;
  v_role text;
begin
  -- A match from before 0028, or one the reaper resolved (R112), carries no hash.
  insert into public.matches (id, status, seed, p1_profile_id, p2_profile_id, p1_deck, p2_deck,
                              catalog_version, ceiling_at, started_at, ended_at)
  values (mid, 'over', 'seed-0768', p1, p2, '[]', '[]', 'core-1', now(), now(), now());
  select final_hash into v_hash from public.matches where id = mid;
  if v_hash is not null then
    raise exception 'FAIL (R768): a match written without a final hash reads %', v_hash;
  end if;

  -- The store's write (src/db/pg.rs, matches_record_final_hash): the first hash stays.
  update public.matches set final_hash = 'hash-1' where id = mid and final_hash is null;
  update public.matches set final_hash = 'hash-2' where id = mid and final_hash is null;
  select final_hash into v_hash from public.matches where id = mid;
  if v_hash is distinct from 'hash-1' then
    raise exception 'FAIL (R768): the final hash reads %, expected the first one written', v_hash;
  end if;

  -- TRUST BOUNDARY: only the server reads it, as it reads the rest of public.matches.
  foreach v_role in array array['anon', 'authenticated'] loop
    if has_column_privilege(v_role, 'public.matches', 'final_hash', 'select') then
      raise exception 'FAIL (R768): % can read public.matches.final_hash', v_role;
    end if;
  end loop;
  raise notice 'OK (R768): null until written, written once, unreadable by client roles';
end $$;
rollback;
