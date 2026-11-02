-- ============================================================================
-- Migration 0076: Constructed and All Random, two ranked ladders
-- (SPEC §9.5, §9.12; R2778000, R603, R605, R607, R610, R611, R612; issue #2778)
-- ============================================================================
-- Serves R2778000. Every season now has two ladders: Constructed (the Best of 1
-- and Conquest queues) and All Random. Each has its own hidden Glicko-2 rating,
-- placements, rank, floor, peaks, streak, record and Jlorious. A ranked game
-- moves only its own mode's ladder (src/api/ranked.rs). This file:
--
--   1. Gives public.profiles the All Random ladder's hidden triple,
--      random_rating / random_rating_deviation / random_rating_volatility.
--      Each starts as a copy of the profile's current triple, which stays as
--      the Constructed ladder's. A new profile starts both at a new player's
--      values (1000, 350, 0.06). R612: no client role reads any of the three.
--      Since 0022, profiles is granted to `authenticated` as a column
--      whitelist, which a new column is not on, so no revoke is needed;
--      tests/sql/12_ranked.sql checks the refusal.
--   2. Puts public.season_ranks on a ladder: `format` ('constructed' or
--      'random'), with the key (season_id, profile_id, format). It copies
--      every row of the newest season onto the All Random ladder, so nobody
--      loses the rank they hold. Older seasons' badges stay on Constructed,
--      the one ladder they were played on. 0064's ladder index is rebuilt over
--      (season_id, format, ladder), the order the rank reads filter in.
--   3. Puts public.bot_ratings on a ladder the same way, key (bot_id, format),
--      every bot's rating copied onto All Random (R610).
--   4. Records the ladder of each public.rated_games row (R611). A row from
--      before this file is Constructed.
--
-- Apply order: ... -> 0022 (seasons, season_ranks, bot_ratings, rated_games) ->
-- ... -> 0064 (season_ranks_season_ladder_idx) -> ... -> 0075 -> 0076 (this
-- file). Safe to re-apply:
--   * the columns are added if not exists;
--   * the copies fill only what is still null or missing;
--   * the keys, checks and index are dropped and made again.
-- ============================================================================


-- ----------------------------------------------------------------------------
-- 1. The All Random ladder's hidden rating (R603, R612).
-- ----------------------------------------------------------------------------
alter table public.profiles
  add column if not exists random_rating            double precision,
  add column if not exists random_rating_deviation  double precision,
  add column if not exists random_rating_volatility double precision;

update public.profiles
   set random_rating            = coalesce(random_rating, rating),
       random_rating_deviation  = coalesce(random_rating_deviation, rating_deviation),
       random_rating_volatility = coalesce(random_rating_volatility, rating_volatility)
 where random_rating is null
    or random_rating_deviation is null
    or random_rating_volatility is null;

alter table public.profiles
  alter column random_rating            set default 1000,
  alter column random_rating_deviation  set default 350,
  alter column random_rating_volatility set default 0.06,
  alter column random_rating            set not null,
  alter column random_rating_deviation  set not null,
  alter column random_rating_volatility set not null;

comment on column public.profiles.random_rating is
  'R2778000: the All Random ladder''s hidden Glicko-2 rating; `rating` is the Constructed ladder''s. Copied from `rating` by 0076. R612 keeps it off every client read, as it does `rating`.';
comment on column public.profiles.random_rating_deviation is
  'R2778000: the All Random ladder''s Glicko-2 RD (RATING_DEVIATION_START for a new player).';
comment on column public.profiles.random_rating_volatility is
  'R2778000: the All Random ladder''s Glicko-2 sigma (RATING_VOLATILITY_START for a new player).';


-- ----------------------------------------------------------------------------
-- 2. A season row per ladder (R605, R607, R608).
-- ----------------------------------------------------------------------------
alter table public.season_ranks
  add column if not exists format text not null default 'constructed';

alter table public.season_ranks drop constraint if exists season_ranks_format_check;
alter table public.season_ranks
  add constraint season_ranks_format_check check (format in ('constructed', 'random'));

alter table public.season_ranks drop constraint if exists season_ranks_pkey;
alter table public.season_ranks
  add constraint season_ranks_pkey primary key (season_id, profile_id, format);

insert into public.season_ranks (season_id, profile_id, format, games, wins, losses, draws, ladder,
                                 floor, streak, peak_ladder, peak_jlorious, updated_at)
select r.season_id, r.profile_id, 'random', r.games, r.wins, r.losses, r.draws, r.ladder,
       r.floor, r.streak, r.peak_ladder, r.peak_jlorious, r.updated_at
  from public.season_ranks r
 where r.format = 'constructed'
   and r.season_id = (select s.id from public.seasons s order by s.started_at desc, s.id desc limit 1)
on conflict (season_id, profile_id, format) do nothing;

drop index if exists public.season_ranks_season_ladder_idx;
create index season_ranks_season_ladder_idx
  on public.season_ranks (season_id, format, ladder);

comment on column public.season_ranks.format is
  'R2778000: the ladder the row is on: constructed (the Best of 1 and Conquest queues) or random (All Random). A player holds one row per ladder per season.';


-- ----------------------------------------------------------------------------
-- 3. A bot's rating per ladder (R610).
-- ----------------------------------------------------------------------------
alter table public.bot_ratings
  add column if not exists format text not null default 'constructed';

alter table public.bot_ratings drop constraint if exists bot_ratings_format_check;
alter table public.bot_ratings
  add constraint bot_ratings_format_check check (format in ('constructed', 'random'));

alter table public.bot_ratings drop constraint if exists bot_ratings_pkey;
alter table public.bot_ratings
  add constraint bot_ratings_pkey primary key (bot_id, format);

insert into public.bot_ratings (bot_id, format, rating, deviation, volatility, games, updated_at,
                                ai_generation)
select b.bot_id, 'random', b.rating, b.deviation, b.volatility, b.games, b.updated_at, b.ai_generation
  from public.bot_ratings b
 where b.format = 'constructed'
on conflict (bot_id, format) do nothing;

comment on column public.bot_ratings.format is
  'R2778000: the ladder the bot''s rating is on; a bot holds one per ladder.';


-- ----------------------------------------------------------------------------
-- 4. The ladder each rated game moved (R611).
-- ----------------------------------------------------------------------------
alter table public.rated_games
  add column if not exists format text not null default 'constructed';

alter table public.rated_games drop constraint if exists rated_games_format_check;
alter table public.rated_games
  add constraint rated_games_format_check check (format in ('constructed', 'random'));

comment on column public.rated_games.format is
  'R2778000: the ladder the game moved; a row from before 0076 is constructed.';
