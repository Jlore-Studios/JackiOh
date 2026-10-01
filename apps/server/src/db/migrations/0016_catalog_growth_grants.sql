-- ============================================================================
-- Migration 0016: a catalog that grows is granted to everyone who owns it
-- ============================================================================
-- Serves SPEC §9.1 ("Everyone owns every card at launch; keep the ledger
-- anyway"), §9.4 (every collection change writes `collection` and
-- `collection_grants` in one transaction), R111 (the launch grant) and R481
-- (patch v0.2.0: a new catalog version grants its new cards).
--
-- Why this exists: R111's launch grant rides the pending -> active transition,
-- so an account that was already active when patch v0.2.0 seeds Classic and
-- Classic+ would never own the 168 new cards: its launch grant ran under
-- `core-1`. Everyone owns every card, so a catalog that grows must grant what
-- it adds, through the same ledger and the same idempotent function.
--
-- How: `db:seed-catalog` writes every `public.cards` row of the new version and
-- then, in the same transaction, stamps `app.settings.catalog_version`. A
-- trigger on that stamp calls `app.grant_launch_collection_all()`, which grants
-- every active profile the launch quantity of each non-token card of the
-- current version it holds no 'launch' grant for — so the new cards, and only
-- them, each written to `collection_grants` with reason 'launch' and ref
-- 'launch:<version>' (0002). It is idempotent: a second stamp of the same
-- version, or a reseed, grants nothing. A card removed from the catalog is not
-- taken back: the ledger only ever grows here.
--
-- The backfill at the end grants the current version's cards to every active
-- profile now, which is a no-op unless cards were seeded before this migration
-- ran (a deployment that seeded v0.2.0 first and migrated after).
--
-- Apply order: 0002 (`app.grant_launch_collection_all`) -> ... -> 0016.
-- Safe to re-apply: create-or-replace and drop-trigger-if-exists.
-- ============================================================================

create or replace function app.on_catalog_version_stamped()
returns trigger
language plpgsql
security definer
set search_path = ''
as $$
begin
  -- R481: only the catalog version's own key, and only when it changes.
  if new.key = 'catalog_version'
     and (tg_op = 'INSERT' or new.value is distinct from old.value) then
    perform app.grant_launch_collection_all();
  end if;
  return new;
end;
$$;

comment on function app.on_catalog_version_stamped() is
  $$R481: when app.settings.catalog_version changes, grant every active
  profile the launch quantity of each non-token card of the new version it
  has no 'launch' grant for (app.grant_launch_collection_all, R111's
  idempotent grant). Fired by db:seed-catalog's stamp, in its transaction.$$;

drop trigger if exists settings_grant_catalog_growth on app.settings;
create trigger settings_grant_catalog_growth
  after insert or update of value on app.settings
  for each row
  when (new.key = 'catalog_version')
  execute function app.on_catalog_version_stamped();

revoke all on function app.on_catalog_version_stamped() from public;

-- Backfill: whatever the current version holds that an active profile lacks.
do $$
begin
  perform app.grant_launch_collection_all();
end $$;

-- ----------------------------------------------------------------------------
-- Grants: no role may call the trigger function; it runs as the table's
-- trigger only. public.collection and public.collection_grants keep 0002's.
-- ============================================================================
