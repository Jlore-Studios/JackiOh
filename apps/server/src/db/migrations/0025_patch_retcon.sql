-- ============================================================================
-- Migration 0025: the card patches' new names (issue #290, SPEC §11 R739)
-- ============================================================================
-- Issue #290 renamed every card patch after v0.2.0 to the next number in
-- `packages/cards/patches/patches.json`'s order (a micro patch to the next
-- letter): v0.2.4 is v0.2.1, v0.2.5 v0.2.2, v0.2.10 v0.2.3, v0.2.11 v0.2.4,
-- v0.2.12 v0.2.5, v0.2.13 v0.2.6, v0.2.14 v0.2.7, v0.2.14b v0.2.7b and
-- v0.2.16 v0.2.8. A database that served the old names files rows under them,
-- and two of them (v0.2.4, v0.2.5) now name other catalogs, so each row
-- carrying an old name takes the new one here, or the card statistics, decks
-- and rated games would mix two catalogs under one name (R105, R375, R376).
--
-- What moves, for each name: the catalog version of `cards`, `decks`,
-- `tickets`, `matches` and `series`; a game record's patch, with the copies in
-- its record (`game_records_copies_check`) and, for a development record, the
-- patch inside its id (`dev:<patch>:<series>:<n>`, R378); the patch and
-- catalog version of `rated_games`; the patch version of `seasons`; and
-- `app.settings`' catalog version, after the `cards` rows it names, whose
-- launch grant (0016) then finds every card already granted. A series is
-- written by compare-and-set on its version (R263), so its rename takes the
-- next version: a server that read the row before this ran re-reads it
-- instead of writing the old name back.
--
-- What stays: `collection_grants` is append-only (0012) and keeps its
-- `launch:<version>` references, which nothing reads; the loadout tables are
-- never written again (R254). A name no card patch ever had (the patches that
-- changed no card data were never catalog versions) is not touched.
--
-- Order: one name at a time, oldest first, so v0.2.4 and v0.2.5 are renamed
-- away before v0.2.11 and v0.2.12 take them and no row is renamed twice. A
-- development id matches its name followed by ':', so v0.2.14 never takes a
-- v0.2.14b id. Not safe to re-apply: run again, it would rename the new v0.2.4
-- and v0.2.5 rows a second time (migrate.ts applies each file once). On a new
-- project it changes nothing: every row there carries the current name.
-- ============================================================================

do $$
declare
  -- R739: each card patch's name before #290 and after it, oldest first.
  renames constant text[] := array[
    'v0.2.4',   'v0.2.1',
    'v0.2.5',   'v0.2.2',
    'v0.2.10',  'v0.2.3',
    'v0.2.11',  'v0.2.4',
    'v0.2.12',  'v0.2.5',
    'v0.2.13',  'v0.2.6',
    'v0.2.14',  'v0.2.7',
    'v0.2.14b', 'v0.2.7b',
    'v0.2.16',  'v0.2.8'
  ];
  was    text;
  is_now text;
begin
  for i in 1 .. array_length(renames, 1) / 2 loop
    was := renames[2 * i - 1];
    is_now := renames[2 * i];

    update public.cards set catalog_version = is_now where catalog_version = was;
    update public.decks set catalog_version = is_now where catalog_version = was;
    update public.tickets set catalog_version = is_now where catalog_version = was;
    update public.matches set catalog_version = is_now where catalog_version = was;
    update public.series set catalog_version = is_now, version = version + 1
     where catalog_version = was;

    update public.game_records g
       set id = r.renamed,
           patch = is_now,
           record = g.record || jsonb_build_object('id', r.renamed, 'patch', is_now)
      from (select id,
                   case when starts_with(id, 'dev:' || was || ':')
                        then 'dev:' || is_now || ':' || substr(id, length('dev:' || was || ':') + 1)
                        else id
                   end as renamed
              from public.game_records
             where patch = was) r
     where g.id = r.id;

    update public.rated_games set patch_version = is_now where patch_version = was;
    update public.rated_games set catalog_version = is_now where catalog_version = was;
    update public.seasons set patch_version = is_now where patch_version = was;

    update app.settings set value = to_jsonb(is_now)
     where key = 'catalog_version' and value = to_jsonb(was);
  end loop;
end $$;
