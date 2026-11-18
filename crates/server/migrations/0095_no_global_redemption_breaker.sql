-- ============================================================================
-- 0095 — no global breaker: redemption pauses only for the operator (R4131000)
-- ============================================================================
-- app.redeem_invite_code (0001 §6, 0006) refused every redemption, good codes
-- included, once more than app.settings.redemption_failure_threshold (100)
-- attempts had failed system-wide within redemption_failure_window_seconds
-- (600; R106). A guess spread over many addresses could keep that open for as
-- long as it ran and shut sign-ups for everyone, while against 80-bit codes it
-- has no chance of finding one. R4131000 drops it: one profile (step 2) and one
-- address (step 3) are blocked for the rest of their hour, and the operator's
-- switch, app.settings.redemption_enabled, is the only global pause.
--
-- The rest of the function is 0006's, unchanged; it is restated whole because
-- a function body cannot be patched in place. The two settings rows and the
-- partial index only the count read go with it.

create or replace function app.redeem_invite_code(
  p_profile_id uuid,
  p_code_hash  text,
  p_ip_hash    text
)
returns text
language plpgsql
security definer
set search_path = ''
as $$
declare
  v_status                  text;
  v_email_confirmed_at      timestamptz;
  v_profile_recent_attempts bigint;
  v_ip_recent_attempts      bigint;
  v_redemption_enabled      boolean;
  v_attempt_id              bigint;
  v_code_id                 uuid;
  v_code_revoked_at         timestamptz;
  v_code_expires_at         timestamptz;
  v_code_uses               int;
  v_code_max_uses           int;
begin
  -- Step 1 (SPEC §9.4): "reject unless the account is pending with a
  -- verified email." public.profiles has no email column — email lives on
  -- auth.users, which only a SECURITY DEFINER function owned by postgres
  -- (this one) can read; email_confirmed_at is Supabase Auth's own
  -- verification timestamp. `for update of p` takes the profile row lock
  -- this call holds through step 6, so two concurrent redemption attempts
  -- for the same profile serialise instead of racing.
  select p.status, u.email_confirmed_at
    into v_status, v_email_confirmed_at
    from public.profiles p
    join auth.users u on u.id = p.id
   where p.id = p_profile_id
     for update of p;

  if not found or v_status is distinct from 'pending' then
    return 'not_pending';
  end if;

  if v_email_confirmed_at is null then
    return 'email_unverified';
  end if;

  -- Step 2 (SPEC §9.4): "reject if this profile made more than 5 attempts
  -- in the last hour."
  select count(*)
    into v_profile_recent_attempts
    from public.code_attempts
   where profile_id = p_profile_id
     and at > now() - interval '1 hour';

  if v_profile_recent_attempts > 5 then
    return 'rate_limited_profile';
  end if;

  -- Step 3 (SPEC §9.4): "reject if this IP hash made more than 20."
  -- The count and step 4's insert below must not interleave with another
  -- redemption from the same IP hash, or every concurrent caller reads the
  -- same count and all of them pass. The profile row lock above does not
  -- cover it (different profiles, one address), so this takes a
  -- transaction-scoped advisory lock on the IP hash first, released when the
  -- transaction ends. Locks are always taken profile, then IP, then code, so
  -- two redemptions can never wait on each other in a cycle. (Migration 0006.)
  perform pg_advisory_xact_lock(hashtextextended('code_attempts:ip:' || p_ip_hash, 0));

  select count(*)
    into v_ip_recent_attempts
    from public.code_attempts
   where ip_hash = p_ip_hash
     and at > now() - interval '1 hour';

  if v_ip_recent_attempts > 20 then
    return 'rate_limited_ip';
  end if;

  -- The operator's pause (SPEC §9.4, R4131000), placed before the lookup
  -- (step 5): while app.settings.redemption_enabled is false, `moderate
  -- redemption off` has paused redemption for everyone. Nothing else pauses
  -- it: no count of failures across addresses does, since a guess spread over
  -- many addresses against 80-bit codes is hopeless, and a breaker it could
  -- trip would let it shut sign-ups for every honest player. Steps 2 and 3
  -- above are the temporary block on one profile and one address.
  v_redemption_enabled := coalesce((app.setting('redemption_enabled') #>> '{}')::boolean, true);

  if not v_redemption_enabled then
    insert into public.code_attempts (profile_id, ip_hash, succeeded)
    values (p_profile_id, p_ip_hash, false);
    return 'circuit_open';
  end if;

  -- Step 4 (SPEC §9.4): "log the attempt either way." Logged now, before the
  -- lookup at step 5, exactly as SPEC orders it — written false and flipped
  -- true only if step 6 below actually commits the redemption. Because this
  -- insert happens before we know the outcome, it is deliberately not
  -- protected by app.deny_row_mutation() the way collection_grants and
  -- match_actions are (§ shared-object contract): this one function is the
  -- only writer, and it performs exactly one follow-up UPDATE of its own row.
  insert into public.code_attempts (profile_id, ip_hash, succeeded)
  values (p_profile_id, p_ip_hash, false)
  returning id into v_attempt_id;

  -- Step 5 (SPEC §9.4): "look up by hash and reject if revoked, expired or
  -- exhausted." `for update` takes the code row's lock now, so a concurrent
  -- redemption of the same code blocks here rather than both passing this
  -- check and over-consuming the last use.
  select id, revoked_at, expires_at, uses, max_uses
    into v_code_id, v_code_revoked_at, v_code_expires_at, v_code_uses, v_code_max_uses
    from public.invite_codes
   where code_hash = p_code_hash
     for update;

  -- Missing, revoked, expired and exhausted are one combined boolean check
  -- collapsing to one result, per SPEC §9.4: "Missing, expired and exhausted
  -- codes return an identical error in identical time."
  if not found
     or v_code_revoked_at is not null
     or (v_code_expires_at is not null and v_code_expires_at <= now())
     or v_code_uses >= v_code_max_uses
  then
    return 'invalid_code';
  end if;

  -- Step 6 (SPEC §9.4): "increment uses and set the account active,
  -- atomically." Both writes, plus flipping the step-4 log row to
  -- succeeded, commit together with everything above as one statement-level
  -- transaction.
  update public.invite_codes
     set uses = uses + 1
   where id = v_code_id;

  update public.profiles
     set status = 'active',
         activated_at = now()
   where id = p_profile_id;

  update public.code_attempts
     set succeeded = true
   where id = v_attempt_id;

  return 'ok';
end;
$$;

comment on function app.redeem_invite_code(uuid, text, text) is
  'SPEC §9.4: the six-step invite-code redemption transaction, quoted step '
  'by step in the function body. Never raises for an expected rejection — '
  'always returns one of ''ok'', ''not_pending'', ''email_unverified'', '
  '''rate_limited_profile'', ''rate_limited_ip'', ''circuit_open'', '
  '''invalid_code''. Called only by the server (service_role) from '
  'crates/server/src/api/codes.rs; never reachable from a browser.';

-- Only the server calls this, as 0001 granted it: re-created, the function
-- keeps its grants, and restating them keeps this file whole on its own.
revoke execute on function app.redeem_invite_code(uuid, text, text) from public;
grant execute on function app.redeem_invite_code(uuid, text, text) to service_role;

delete from app.settings
 where key in ('redemption_failure_threshold', 'redemption_failure_window_seconds');

drop index if exists public.code_attempts_recent_failures_idx;
