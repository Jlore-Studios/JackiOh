-- ============================================================================
-- Migration 0034: the age check and the terms each account agreed to
-- ============================================================================
-- Serves SPEC.md §9.4, SPEC §11 R757000 (the age screen) and R757001 (terms
-- acceptance per account and version); issue #757.
--
--   * public.age_checks -- one row per account that passed the age screen:
--     when, and the minimum age it was checked against. NEVER the birth month
--     or year: the server checks them and keeps only that the check passed.
--   * public.terms_acceptances -- one row per account and version of the Terms
--     and the Privacy Policy it agreed to (config.rs `TERMS_VERSION`), with
--     when and how: at sign-up, agreeing again after a change, or the baseline
--     below.
--
-- Baseline: every human account that exists when this runs gets one
-- terms_acceptances row, version 'baseline' and source 'baseline', dated when
-- the account was created. It records the implied agreement the sign-up hint
-- asked for before the box existed, so no existing account has an empty
-- record. 'baseline' is not a version the server asks for, so such an account
-- is asked to agree on its next visit (R757001). An account that was already
-- active is not asked its age; a pending one answers the age screen before it
-- can redeem a code (R757000).
--
-- TRUST BOUNDARY: no client role reads or writes either table. The server
-- (service_role, BYPASSRLS) reads and writes them through the Store port. RLS
-- is on with no policy, and anon and authenticated hold no grant. Both rows
-- cascade with their profile, so deleting an account removes them (0012).
--
-- Apply order: ... -> 0032 -> 0033 -> 0034 (this file).
-- Safe to re-apply: create-if-not-exists, drop-and-add constraints, and an
-- insert that skips the rows it already wrote.
-- ============================================================================

create table if not exists public.age_checks (
  profile_id uuid primary key references public.profiles(id) on delete cascade,
  min_age    smallint not null,
  passed_at  timestamptz not null default now()
);

alter table public.age_checks drop constraint if exists age_checks_min_age_check;
alter table public.age_checks
  add constraint age_checks_min_age_check check (min_age between 1 and 120);

comment on table public.age_checks is
  $$R757000: each account that passed the age screen, when, and the minimum it
  was checked against. Holds no birth date. Server-only (service_role).$$;

comment on column public.age_checks.min_age is
  $$R757000: MIN_ACCOUNT_AGE_YEARS as it stood when the check passed.$$;

create table if not exists public.terms_acceptances (
  profile_id    uuid not null references public.profiles(id) on delete cascade,
  terms_version text not null,
  source        text not null,
  accepted_at   timestamptz not null default now(),
  primary key (profile_id, terms_version)
);

alter table public.terms_acceptances drop constraint if exists terms_acceptances_version_check;
alter table public.terms_acceptances
  add constraint terms_acceptances_version_check
  check (char_length(terms_version) between 1 and 64);

alter table public.terms_acceptances drop constraint if exists terms_acceptances_source_check;
alter table public.terms_acceptances
  add constraint terms_acceptances_source_check
  check (source in ('signup', 'reaccept', 'baseline'));

comment on table public.terms_acceptances is
  $$R757001: each version of the Terms and the Privacy Policy an account agreed
  to, and when. Server-only (service_role).$$;

comment on column public.terms_acceptances.terms_version is
  $$R757001: TERMS_VERSION when the account agreed (the terms' date, then the
  privacy policy's), or 'baseline' for an account that existed before 0034.$$;

comment on column public.terms_acceptances.source is
  $$R757001: signup (agreed while setting up the account), reaccept (agreed
  again after a change) or baseline (existed before 0034).$$;

alter table public.age_checks enable row level security;
alter table public.terms_acceptances enable row level security;
revoke all on public.age_checks from public, anon, authenticated;
revoke all on public.terms_acceptances from public, anon, authenticated;

insert into public.terms_acceptances (profile_id, terms_version, source, accepted_at)
select id, 'baseline', 'baseline', created_at
  from public.profiles
 where kind = 'human'
on conflict do nothing;
