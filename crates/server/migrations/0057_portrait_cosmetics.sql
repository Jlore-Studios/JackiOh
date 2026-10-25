-- ============================================================================
-- Migration 0057: The hero portraits become account cosmetics (SPEC §14.2,
-- §14.12; R1827000; issue #1827)
-- ============================================================================
-- Serves SPEC §14.12 (Going live), quoted:
--   "The starter cosmetics stay owned by every person's account. ... every
--   cosmetic added later must be owned"
-- and R1827000: the six hero portraits (R641) are account cosmetics of the
-- kind `portrait`. `portrait-vanilla` is the kind's default and a starter,
-- owned by every account with no row (§14.2); the other five are Epic and
-- not starters, so an account owns one only by a row. Every account that
-- exists when this file runs keeps all six, as every portrait was every
-- account's to pick until now:
--
--   1. The six rows of public.cosmetics, as crates/server/cosmetics.json has
--      them. `seed-catalog` upserts the whole registry into public.cosmetics
--      (0045), but `release` runs the migrations before it, and the grant's
--      foreign keys need these rows to exist. A row seed-catalog already
--      wrote is left as it wrote it.
--   2. The grant (R1827000): every person's profile (kind human, whatever its
--      status; a ladder bot owns nothing, R1454) is given the five portraits
--      that are not starters, each in public.account_cosmetics with its
--      public.cosmetic_grants row: no card, +1, reason 'launch' (the closest
--      of the ledger's closed set of reasons, R1310002) and ref
--      'launch:0057'. A portrait an account already owns is passed over and
--      gets no ledger row, so each ledger still sums to what is held (§14.11).
--      An account made after this owns `portrait-vanilla` alone, with no row,
--      and earns the rest. It runs once per database:
--      app.settings.portraits_granted marks it done, with the time it ran.
--      It runs in the one transaction the migration runner gives a file
--      (src/db/migrate.rs), so it lands whole or not at all.
--
-- Apply order: ... -> 0045 (public.cosmetics, account_cosmetics and
-- cosmetic_grants) -> ... -> 0050 (the reason 'launch' on cosmetic_grants) ->
-- ... -> 0056 -> 0057 (this file).
-- Safe to re-apply: insert-on-conflict-do-nothing, and the grant skipped once
-- portraits_granted is set.
-- ============================================================================


-- ----------------------------------------------------------------------------
-- 1. The portraits in the registry's projection (R1827000).
-- ----------------------------------------------------------------------------
-- MIRRORS crates/server/cosmetics.json (its six `portrait-*` entries).
insert into public.cosmetics (id, kind, name, rarity, is_default)
values ('portrait-vanilla', 'portrait', 'Mr. Vanilla portrait', 'Common', true),
       ('portrait-gary', 'portrait', 'Gary the Gambler portrait', 'Epic', false),
       ('portrait-timmy', 'portrait', 'Tempo Timmy portrait', 'Epic', false),
       ('portrait-dfender', 'portrait', 'Big D-fender portrait', 'Epic', false),
       ('portrait-felinors', 'portrait', 'Duplicating Felinors portrait', 'Epic', false),
       ('portrait-shredder', 'portrait', 'Jlockeed Shredder-10 portrait', 'Epic', false)
on conflict (id) do nothing;


-- ----------------------------------------------------------------------------
-- 2. Every existing person keeps every portrait (R1827000).
-- ----------------------------------------------------------------------------
-- Each owned row and its ledger row are written by one statement: a ledger
-- row only for a row actually inserted.
do $$
begin
  if exists (select 1 from app.settings where key = 'portraits_granted') then
    return;
  end if;

  with granted as (
    insert into public.account_cosmetics (profile_id, cosmetic_id)
    select p.id, c.id
      from public.profiles p
     cross join (values ('portrait-gary'), ('portrait-timmy'), ('portrait-dfender'),
                        ('portrait-felinors'), ('portrait-shredder')) as c(id)
     where p.kind = 'human'
    on conflict (profile_id, cosmetic_id) do nothing
    returning profile_id, cosmetic_id
  )
  insert into public.cosmetic_grants (profile_id, card_id, cosmetic_id, delta, reason, ref)
  select g.profile_id, null, g.cosmetic_id, 1, 'launch', 'launch:0057'
    from granted g;

  insert into app.settings (key, value) values ('portraits_granted', to_jsonb(now()));
end $$;
