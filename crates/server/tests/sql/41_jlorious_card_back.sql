-- The Jlorious card back (migration 0074, SPEC §14.2, §14.11; R2755000, R607, R640000; pull request
-- #2755). Runs after 40_full_art_tiers.sql. The first block checks the registry's row and the ledger's
-- new reason; the second applies the file again (/tmp/0074_jlorious_card_back.sql, which run.sh copied
-- there) over people of its own: one who has held Jlorious, one who never has, one who owns the back
-- already, and a ladder bot whose row says it held Jlorious, which no bot does.
\set ON_ERROR_STOP on

-- Same rules as 01-40: every check raises on failure (`raise exception 'FAIL (…): …'`), each check
-- that could pass vacuously carries a guard, and every block runs inside a transaction that is
-- rolled back. Nothing here selects into the log: the do blocks raise or say OK.

\echo '### R2755000: 0074 wrote the card back and the reason jlorious ###'
begin;
do $$
declare
  v_text text;
begin
  -- The registry's entry, as crates/server/cosmetics.json has it: a Mythic card back, no default and
  -- no chase.
  select format('%s %s %s %s default=%s chase=%s', id, kind, name, rarity, is_default::text, chase::text)
    into v_text from public.cosmetics where id = 'card-back-jlorious';
  if v_text is distinct from 'card-back-jlorious cardBack Jlorious Mythic default=false chase=false' then
    raise exception 'FAIL (R2755000): the card back''s row of public.cosmetics is [%]', coalesce(v_text, '(none)');
  end if;

  select pg_get_constraintdef(c.oid) into v_text
    from pg_constraint c
   where c.conname = 'cosmetic_grants_reason_check' and c.conrelid = 'public.cosmetic_grants'::regclass;
  if v_text is null or position('''jlorious''' in v_text) = 0 or position('''launch''' in v_text) = 0 then
    raise exception 'FAIL (R2755000): cosmetic_grants'' reasons are [%], without jlorious or launch', coalesce(v_text, '(none)');
  end if;

  raise notice 'OK (R2755000): the card back is in public.cosmetics and jlorious is a reason of cosmetic_grants';
end $$;
rollback;

\echo '### R2755000: 0074 over the people of a database: each person who held Jlorious, never a bot ###'
begin;
insert into auth.users (id, email, email_confirmed_at)
values ('e2755000-0000-4000-8000-000000000001', 'r2755000-held@example.test', now()),
       ('e2755000-0000-4000-8000-000000000002', 'r2755000-never@example.test', now()),
       ('e2755000-0000-4000-8000-000000000003', 'r2755000-owns@example.test', now());
insert into public.profiles (id, kind, status) values ('e2755000-0000-4000-8000-000000000004', 'bot', 'active');
insert into public.seasons (id, patch_version, started_at) values ('check-r2755000', 'check-r2755000.0', now());
-- Held Jlorious (#7 at best), never did, held it (#1) and owns the back already, and a bot whose row
-- says #3.
insert into public.season_ranks (season_id, profile_id, games, wins, losses, draws, ladder, floor,
                                 streak, peak_ladder, peak_jlorious, updated_at)
values ('check-r2755000', 'e2755000-0000-4000-8000-000000000001', 40, 30, 10, 0, 1400, 1200, 3, 1400, 7, now()),
       ('check-r2755000', 'e2755000-0000-4000-8000-000000000002', 40, 30, 10, 0, 1400, 1200, 3, 1400, null, now()),
       ('check-r2755000', 'e2755000-0000-4000-8000-000000000003', 40, 30, 10, 0, 1400, 1200, 3, 1400, 1, now()),
       ('check-r2755000', 'e2755000-0000-4000-8000-000000000004', 40, 30, 10, 0, 1400, 1200, 3, 1400, 3, now());
insert into public.account_cosmetics (profile_id, cosmetic_id)
values ('e2755000-0000-4000-8000-000000000003', 'card-back-jlorious');
insert into public.cosmetic_grants (profile_id, card_id, cosmetic_id, delta, reason, ref)
values ('e2755000-0000-4000-8000-000000000003', null, 'card-back-jlorious', 1, 'jlorious', 'match:earlier');

do $$
begin
  if exists (select 1 from public.account_cosmetics
              where profile_id in ('e2755000-0000-4000-8000-000000000001', 'e2755000-0000-4000-8000-000000000002',
                                   'e2755000-0000-4000-8000-000000000004')) then
    raise exception 'FAIL (R2755000): a person or the bot owns an account cosmetic before 0074 runs again';
  end if;
  if (select count(*) from public.profiles
       where id in ('e2755000-0000-4000-8000-000000000001', 'e2755000-0000-4000-8000-000000000002',
                    'e2755000-0000-4000-8000-000000000003') and kind = 'human') <> 3
     or not exists (select 1 from public.profiles where id = 'e2755000-0000-4000-8000-000000000004' and kind = 'bot') then
    raise exception 'FAIL (R2755000): the four profiles are not three people and a bot';
  end if;
end $$;

-- The grant, as it runs on a database these people are signed up to, twice: the second finds every
-- holder and writes nothing.
set local client_min_messages = warning;
\i /tmp/0074_jlorious_card_back.sql
\i /tmp/0074_jlorious_card_back.sql
set local client_min_messages = notice;

do $$
declare
  held  constant uuid := 'e2755000-0000-4000-8000-000000000001';
  never constant uuid := 'e2755000-0000-4000-8000-000000000002';
  owns  constant uuid := 'e2755000-0000-4000-8000-000000000003';
  bot   constant uuid := 'e2755000-0000-4000-8000-000000000004';
  v_text text;
begin
  -- The person who held Jlorious owns the back, with one ledger row of the reason jlorious; the one who
  -- owned it already keeps their one row, and the second application wrote nothing.
  select string_agg(format('%s %s %s', profile_id, cosmetic_id, reason || coalesce(' ' || ref, '')), ', '
                    order by profile_id)
    into v_text from public.cosmetic_grants where profile_id in (held, never, owns, bot);
  if v_text is distinct from format('%1$s card-back-jlorious jlorious jlorious:0074, '
                                    '%2$s card-back-jlorious jlorious match:earlier', held, owns) then
    raise exception 'FAIL (R2755000): the cosmetic ledgers after 0074 are [%]', coalesce(v_text, '(empty)');
  end if;
  if not exists (select 1 from public.account_cosmetics where profile_id = held and cosmetic_id = 'card-back-jlorious') then
    raise exception 'FAIL (R2755000): the person who held Jlorious does not own the card back';
  end if;

  -- Never Jlorious, and a ladder bot, which is never Jlorious whatever its row says (R640000): nothing.
  if exists (select 1 from public.account_cosmetics where profile_id in (never, bot)) then
    raise exception 'FAIL (R2755000): 0074 gave the card back to someone who never held Jlorious, or to the bot';
  end if;

  raise notice 'OK (R2755000): 0074 gives the card back once to each person who held Jlorious, and to no one else';
end $$;
rollback;
