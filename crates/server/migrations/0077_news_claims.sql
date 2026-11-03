-- ============================================================================
-- Migration 0077: News claims, one per account and entry
-- (SPEC §14.11, §14.13; R2875000, R2875001; issue #2875)
-- ============================================================================
-- Serves R2875000. The landing's News panel lists the entries of
-- crates/server/news.json, every shipped patch and every alert, each with one
-- reward to claim (§14.13). Claiming it is one spend,
-- `POST /api/news/:newsId/claim` (src/economy/news.rs): it pays the reward as
-- ledger rows with the reason 'news' and writes this table's row in the same
-- transaction. This file:
--
--   1. Creates public.news_claims: that the account claimed the entry's
--      reward, and when. Its key (profile_id, news_id) is what makes a claim
--      once: a second insert conflicts and writes nothing, and the claim
--      writes no reward. The entries themselves are data, not rows: the
--      table names an entry only by its id (a slug). A claim is no holding,
--      so it has no ledger of its own; the reward it pays is in the ledgers
--      (§14.11). Append-only, as a ledger is: a row goes only with its
--      account (0012's rule, `on delete cascade`).
--   2. Adds 'news' to the closed sets of reasons of public.jlore_grants and
--      public.inventory_grants (0050), the two ledgers a reward lands in: a
--      Jlore reward in the wallet (§14.1); a card pack, a card token, a
--      cosmetic token or a cosmetic pack in the inventory (§14.3, §14.5,
--      §14.6).
--
-- TRUST BOUNDARY, as card_cosmetic_picks (0061): a profile reads its own rows
-- through the Data API and writes none; the server (service_role, BYPASSRLS;
-- src/economy/store.rs) is the only writer.
--
-- Apply order: ... -> 0045 (the economy) -> ... -> 0050 (the ledgers'
-- reasons) -> ... -> 0076 -> 0077 (this file). Safe to re-apply:
-- create-if-not-exists, drop-trigger-or-policy-if-exists then create, and the
-- reason checks dropped and added again.
-- ============================================================================


-- ----------------------------------------------------------------------------
-- 1. public.news_claims (R2875000).
-- ----------------------------------------------------------------------------
create table if not exists public.news_claims (
  profile_id uuid not null references public.profiles(id) on delete cascade,
  news_id    text not null,
  claimed_at timestamptz not null,
  primary key (profile_id, news_id),
  constraint news_claims_news_check check (news_id ~ '^[a-z][a-z0-9-]*$')
);

alter table public.news_claims enable row level security;

drop trigger if exists news_claims_deny_mutation on public.news_claims;
create trigger news_claims_deny_mutation before update or delete on public.news_claims
  for each row execute function app.deny_row_mutation('profile_id');

-- SPEC §9.1: a profile reads only its own rows, as player_settings (0018). No
-- insert/update/delete policy exists for any client role.
drop policy if exists news_claims_select_own on public.news_claims;
create policy news_claims_select_own
  on public.news_claims
  for select
  to authenticated
  using (profile_id = app.current_profile_id());

revoke all on public.news_claims from public, anon, authenticated;
grant select on public.news_claims to authenticated;


-- ----------------------------------------------------------------------------
-- 2. The ledgers' reason for a news reward (§14.11).
-- ----------------------------------------------------------------------------
alter table public.jlore_grants drop constraint if exists jlore_grants_reason_check;
alter table public.jlore_grants add constraint jlore_grants_reason_check check (
  reason in ('game', 'quest', 'track', 'pack_purchase', 'dev', 'launch', 'news'));

alter table public.inventory_grants drop constraint if exists inventory_grants_reason_check;
alter table public.inventory_grants add constraint inventory_grants_reason_check check (
  reason in ('game', 'quest', 'track', 'pack_purchase', 'pack_open', 'token_bar', 'token_redeem',
             'chase_shard', 'dev', 'launch', 'news'));


comment on table public.news_claims is
  $$R2875000: that an account claimed a news entry's reward (§14.13), once
  per account and entry, by its key. The entry is crates/server/news.json's,
  named by its id; its reward is paid in the ledgers with the reason 'news'.
  Append-only; a row goes only with its account. A client reads its own rows
  and writes none.$$;
