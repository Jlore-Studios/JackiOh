-- ============================================================================
-- Migration 0018: Player settings on the account
-- ============================================================================
-- Serves SPEC.md §9.1 ("Player settings") and SPEC §11 R633-R634: the
-- switches, volumes and choices a player has set in the game's settings
-- dialog, kept on the server as well as on the device so a player who signs in
-- on another device, or clears site data, finds them there. The device keeps
-- its own copy as before; the client merges the two (R634). This file is the
-- account's copy, and its one rule is how a write lands:
--
--   * a write names the groups it changes -- the gameplay switches, the audio
--     volumes, the effects, the card display -- each with the time it last
--     changed on the writing device, and a group REPLACES the stored one only
--     when that time is STRICTLY later. A stale device can never undo a newer
--     change, a tie changes nothing (so the same write twice is the same
--     write once), and a change to one group never undoes another group's.
--
-- Nothing here is a game rule: a setting changes how the game looks, sounds
-- and asks, and nothing the engine does reads this row. The server does not
-- know the settings either -- they are the client's -- so a group's values
-- are only checked for their shape (a flat object of booleans, numbers and
-- short strings), and a group the client no longer has is kept and ignored.
--
-- SPEC §9.1's trust model holds as for decks and tutorial progress: the
-- browser reads its own row through the Data API and never writes one. It
-- proposes its settings to the server API (PUT /api/settings), which checks
-- the body and calls app.merge_player_settings below as service_role.
--
-- Apply order: 0001 (`app.settings`, `app.setting`, `app.current_profile_id`,
-- `public.profiles`) -> ... -> 0017 -> 0018 (this file).
-- This file only ADDS objects. Safe to re-apply: create-if-not-exists,
-- create-or-replace, drop-policy-if-exists, and settings inserted with
-- ON CONFLICT DO NOTHING.
-- ============================================================================

-- ----------------------------------------------------------------------------
-- public.player_settings -- one row per profile (R633).
-- ----------------------------------------------------------------------------
-- `groups` is a jsonb object keyed by group id (`gameplay`, `audio`, `fx`,
-- `cards`), each value `{ "at": <epoch ms>, "values": { <key>: <boolean |
-- number | string> } }`. jsonb rather than a column per setting because the
-- settings are the client's and grow with it; the server stores what it was
-- sent, bounded.
create table if not exists public.player_settings (
  profile_id uuid primary key references public.profiles(id) on delete cascade,
  groups     jsonb not null default '{}'::jsonb,
  created_at timestamptz not null default now(),
  updated_at timestamptz not null default now(),
  constraint player_settings_groups_object_check check (jsonb_typeof(groups) = 'object')
);

comment on table public.player_settings is
  $$SPEC §9.1 / R633: an active account's game settings, one jsonb object of
  groups, each { at, values }. Written solely by app.merge_player_settings
  (SECURITY DEFINER, service_role only), which replaces a group only with a
  strictly newer one. A client reads its own row and writes none (SPEC §9.1).$$;

comment on column public.player_settings.groups is
  $$R633, R634: group id -> { at: epoch ms of the change on the writing device
  (never later than the server's when it arrived), values: a flat object of
  booleans, numbers and short strings }. At most player_settings_groups_max
  groups and player_settings_bytes_max bytes of text.$$;

alter table public.player_settings enable row level security;

-- SPEC §9.1: a profile reads only its own row, exactly as decks, trios and
-- tutorial progress (0007, 0011). No insert/update/delete policy exists for
-- any client role: RLS default deny is the enforcement for "no
-- client-writable path", and the only writer is the server.
drop policy if exists player_settings_select_own on public.player_settings;
create policy player_settings_select_own
  on public.player_settings
  for select
  to authenticated
  using (profile_id = app.current_profile_id());

-- ============================================================================
-- Server config the merge reads.
-- ============================================================================
-- These MIRROR apps/server/src/config.ts -- PLAYER_SETTINGS_GROUPS_MAX (8) and
-- PLAYER_SETTINGS_BYTES_MAX (4096), SPEC §11 R633 -- which is where the server
-- reads them. They are repeated here, as 0011 repeated the tutorial caps, so
-- the database refuses an overfull row even from a caller that skipped the
-- server's checks; the server passes its own caps to each merge and the
-- function applies the smaller of the two. Changing one of these numbers for a
-- deployment means config.ts AND a new migration updating the row here.
insert into app.settings (key, value) values ('player_settings_groups_max', to_jsonb(8))
on conflict (key) do nothing;

insert into app.settings (key, value) values ('player_settings_bytes_max', to_jsonb(4096))
on conflict (key) do nothing;

-- ----------------------------------------------------------------------------
-- app.merge_player_settings -- the one write path (R633, R634).
-- ----------------------------------------------------------------------------
-- Merges one device's groups into the profile's row, creating it on the first
-- write, and returns what it did:
--   'merged' -- the row now holds, for each group sent, whichever of the
--               stored and the sent has the strictly later `at` (the stored
--               one on a tie), and every stored group the write did not name;
--   'limit'  -- the result would hold more than least(p_max_groups,
--               player_settings_groups_max) groups or more than
--               least(p_max_bytes, player_settings_bytes_max) bytes of text;
--               nothing written.
--
-- Under the profile row lock (`select ... for update`), so two writes for one
-- profile -- two devices changing settings at once -- run one after the other
-- and neither can lose the other's group.
--
-- Like app.merge_tutorial_progress this is defense in depth, NOT the authority
-- for what a player reads: the server checks the body first
-- (src/api/settings.ts) and answers every malformed request itself. The shape
-- checks below exist so no admin tool or future endpoint can store a group the
-- server would refuse; each raises with a "settings:" prefix, which no player
-- ever sees.
create or replace function app.merge_player_settings(
  p_profile_id  uuid,
  p_groups      jsonb,
  p_at          timestamptz,
  p_max_groups  int,
  p_max_bytes   int
) returns text
language plpgsql
security definer
set search_path = ''
as $$
declare
  v_status   text;
  v_found    boolean;
  v_stored   jsonb;
  v_merged   jsonb;
  v_key      text;
  v_sent     jsonb;
  v_held     jsonb;
begin
  -- SPEC §9.4: a pending account has the code screen and nothing else, and the
  -- API route is `active` for the same reason. Checked against p_profile_id
  -- itself, as app.merge_tutorial_progress does, because the only caller is
  -- service_role, for whom auth.uid() means nothing. `for update` is the lock
  -- the merge runs under.
  select p.status into v_status
    from public.profiles p
   where p.id = p_profile_id
     for update;
  if not found then
    raise exception 'settings: profile % not found', p_profile_id;
  elsif v_status <> 'active' then
    raise exception 'settings: profile % is not active (status=%)', p_profile_id, v_status;
  end if;

  if p_groups is null or jsonb_typeof(p_groups) <> 'object' then
    raise exception 'settings: p_groups must be an object of groups';
  end if;
  for v_key, v_sent in select * from jsonb_each(p_groups) loop
    if jsonb_typeof(v_sent) <> 'object'
       or jsonb_typeof(v_sent -> 'values') is distinct from 'object'
       or jsonb_typeof(v_sent -> 'at') is distinct from 'number'
       or (v_sent ->> 'at') !~ '^[0-9]{1,15}$' then
      raise exception 'settings: group % must be { at: whole milliseconds, values: an object }', v_key;
    end if;
  end loop;

  select s.groups into v_stored
    from public.player_settings s
   where s.profile_id = p_profile_id;
  v_found := found;
  v_merged := coalesce(v_stored, '{}'::jsonb);

  -- R634: a group replaces the stored one only when strictly later.
  for v_key, v_sent in select * from jsonb_each(p_groups) loop
    v_held := v_merged -> v_key;
    if v_held is null or (v_sent ->> 'at')::bigint > (v_held ->> 'at')::bigint then
      v_merged := jsonb_set(v_merged, array[v_key], v_sent, true);
    end if;
  end loop;

  if (select count(*) from jsonb_object_keys(v_merged)) >
       least(p_max_groups, (app.setting('player_settings_groups_max'))::text::int)
     or octet_length(v_merged::text) >
       least(p_max_bytes, (app.setting('player_settings_bytes_max'))::text::int) then
    return 'limit';
  end if;

  if v_found then
    update public.player_settings
       set groups = v_merged,
           updated_at = p_at
     where profile_id = p_profile_id;
  else
    insert into public.player_settings (profile_id, groups, created_at, updated_at)
    values (p_profile_id, v_merged, p_at, p_at);
  end if;
  return 'merged';
end;
$$;

comment on function app.merge_player_settings(uuid, jsonb, timestamptz, int, int) is
  $$R633, R634: the one write path for a player's settings. Merges p_groups into
  the profile's row, a group replacing the stored one only when its `at` is
  strictly later, creating the row on the first write; returns 'merged', or
  'limit' (the result would pass least(p_max_groups, app.settings
  player_settings_groups_max) groups or least(p_max_bytes, app.settings
  player_settings_bytes_max) bytes; nothing written). Under the profile row
  lock. Shape checks (an active profile, groups as { at, values }) raise with a
  "settings:" prefix; the server checks first. service_role only.$$;

-- ============================================================================
-- Explicit grants (default-deny first: revoke everything, then grant back
-- only what SPEC §9.1 allows), exactly as 0007 and 0011 did.
-- ============================================================================
revoke all on public.player_settings from public, anon, authenticated;
grant select on public.player_settings to authenticated;

revoke all on function app.merge_player_settings(uuid, jsonb, timestamptz, int, int) from public;
grant execute on function app.merge_player_settings(uuid, jsonb, timestamptz, int, int) to service_role;

-- ----------------------------------------------------------------------------
-- Closing summary of what this schema lets each role do.
-- ----------------------------------------------------------------------------
-- anon:          nothing on player_settings, and no EXECUTE on the merge.
-- authenticated: SELECT on player_settings, filtered by RLS to the row where
--                profile_id = app.current_profile_id() -- its own settings
--                only. No INSERT, UPDATE or DELETE grant or policy and no
--                EXECUTE on app.merge_player_settings: a client proposes its
--                settings to the server API, which checks them and calls the
--                merge.
-- service_role:  BYPASSRLS covers reads of the table directly (GET
--                /api/settings); app.merge_player_settings is granted EXECUTE
--                explicitly, and every write goes through it.
-- ============================================================================
