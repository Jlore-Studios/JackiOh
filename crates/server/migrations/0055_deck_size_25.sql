-- ============================================================================
-- Migration 0055: Standard deck size 25 (SPEC §2.6, §14.12; R1705000, R1705001,
-- R1705002; issue #1705)
-- ============================================================================
-- Serves SPEC §2.6 (Deckbuilding), quoted:
--   "Exactly 25 cards ... a trio needs 75 distinct cards ... A saved deck may
--   be incomplete while it is being built; these rules are checked when it is
--   queued"
-- and SPEC §14.12 (Going live): "A new account starts with the starter cards".
--
-- The engine's DECK_SIZE is 25 (R1705000), and this file moves the database's
-- own copy of it and the starter cards to match:
--
--   1. app.settings.deck_size, 0003's copy of DECK_SIZE, which the deck save
--      (app.upsert_deck, D2: at most deck_size cards) reads, becomes 25. A deck
--      saved at 20 stays as it is: a draft, which saves and is refused when it
--      is queued (L2) until five cards are added (R1705001).
--   2. app.settings.starter_cards gains the eleven Core cards the three
--      practice presets added to reach 25 (crates/server/src/config.rs
--      STARTER_CARDS, R1705002), as a sorted union with what it holds, so the
--      grant in 3 can read them before seed-catalog next writes the list. An
--      empty list (a project seed-catalog has not run on) is left for
--      seed-catalog to write.
--   3. app.grant_starter_cards_to_active() runs the activation grant
--      (app.grant_launch_collection, R111, R1310000) for every active
--      person's profile (kind human; a ladder bot owns no collection, R1454),
--      so an account that became active under the 39-card list owns the eleven
--      too and can queue every preset (R1705002). It is then run once here.
--
-- Apply order: ... -> 0002 (app.grant_launch_collection) -> 0003 (deck_size)
-- -> ... -> 0050 (starter_cards, the starter-only grant) -> ... -> 0054 ->
-- 0055 (this file).
-- Safe to re-apply: the deck_size update writes the same value, the union
-- holds each id once, create-or-replace of the function and comment-on are
-- idempotent, and app.grant_launch_collection skips every card a profile
-- already has a 'launch' grant for, so a second run grants nothing.
-- ============================================================================


-- ----------------------------------------------------------------------------
-- 1. The deck size (R1705000, R1705001).
-- ----------------------------------------------------------------------------
-- MIRRORS crates/engine/src/config.rs DECK_SIZE, as 0003 did at 20.
update app.settings
   set value = to_jsonb(25), updated_at = now()
 where key = 'deck_size';


-- ----------------------------------------------------------------------------
-- 2. The starter cards the presets added (R1705002).
-- ----------------------------------------------------------------------------
-- MIRRORS crates/server/src/config.rs STARTER_CARDS's eleven added at #1705.
update app.settings s
   set value = (
         select jsonb_agg(ids.id order by ids.id)
           from (
             select jsonb_array_elements_text(s.value) as id
             union
             select unnest(array[
               'core-002', 'core-003', 'core-010', 'core-031', 'core-034', 'core-043',
               'core-047', 'core-072', 'core-080', 'core-089', 'core-091'
             ])
           ) ids
       ),
       updated_at = now()
 where s.key = 'starter_cards'
   and jsonb_array_length(s.value) > 0;


-- ----------------------------------------------------------------------------
-- 3. Every active account owns the starter cards (R1705002).
-- ----------------------------------------------------------------------------
-- 0046's loop, as it was, with no dev accounts passed over: a dev account is
-- granted the eleven too, as its activation granted it the 39.
create or replace function app.grant_starter_cards_to_active()
returns int
language plpgsql
security definer
set search_path = ''
as $$
declare
  v_count    int := 0;
  v_profile  record;
begin
  -- R1454: a ladder bot (kind 'bot') owns nothing. Safe to run repeatedly
  -- because app.grant_launch_collection is itself idempotent (R111).
  for v_profile in
    select id from public.profiles
     where status = 'active' and kind = 'human'
  loop
    perform app.grant_launch_collection(v_profile.id);
    v_count := v_count + 1;
  end loop;

  return v_count;
end;
$$;

comment on function app.grant_starter_cards_to_active() is
  $$R1705002: calls app.grant_launch_collection for every active person's
  profile (kind human; a ladder bot owns no collection, R1454), so each owns
  one copy of every starter card app.setting('starter_cards') names. Migration
  0055 runs it once, after adding the cards the practice presets gained at 25
  cards. Returns the number of profiles touched (visited), not the number that
  received new grants -- a repeat run touches the same count and grants
  nothing.$$;

revoke all on function app.grant_starter_cards_to_active() from public;
grant execute on function app.grant_starter_cards_to_active() to service_role;

-- Once, now: a do block, so applying the file prints nothing.
do $$
begin
  perform app.grant_starter_cards_to_active();
end $$;
