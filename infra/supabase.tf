# Supabase: Authentication's Site URL, Redirect URLs and leaked-password check of each site's project,
# and nothing else.
# The schema belongs to the migrations, which `jackioh-server release` applies, and the keys are
# secrets (the Workers' `wrangler secret put` values, apps/edge/README.md).
#
# The token is SUPABASE_ACCESS_TOKEN (README.md, Tokens). Both projects are in one organization
# (jlore-studios), so one member's token reaches both.
provider "supabase" {}

locals {
  # Each site's own project (docs/deploy-cloudflare.md section 8): the VITE_SUPABASE_URL of
  # apps/web/.env.production and .env.staging, and each Worker's SUPABASE_URL in wrangler.jsonc.
  # Staging's is the original project, which production used until it got JackiOh-Production.
  supabase_production_ref = "fuzpvmjdejvbjcarypyi"
  supabase_staging_ref    = "exmjdaswedxhnzmpzqrq"
}

# Emailed links (sign-up confirmation, password reset) and OAuth sign-ins (R666) come back to the
# page's own origin's login path (`authRedirectUrl` in apps/web/src/net/auth.ts), and Supabase
# quietly swaps any `redirect_to` missing from `uri_allow_list` for `site_url`. Each project serves
# one site, so each lists that site's `/login` alone and its `site_url` is that site: no link sends a
# player to the other site, where the account does not exist (and an OAuth sign-in there would fail
# anyway, its PKCE verifier being in the storage of the origin that started it).
#
# Leaked-password protection (`password_hibp_enabled`, Pro plan): a sign-up or a new password is
# checked against HaveIBeenPwned's list of breached passwords and refused when it is on it. Only the
# first five characters of the password's SHA-1 hash leave Supabase (k-anonymity). The client says
# why (`weakPasswordPwned` in apps/web/src/net/auth.ts). An existing password is never checked.
#
# `auth` names only these three keys, and the provider then reads and writes only them. An import
# reads every setting into state, so the first apply narrows the state to these three and sends
# Supabase the three values (README.md, The first import).
resource "supabase_settings" "production" {
  project_ref = local.supabase_production_ref

  auth = jsonencode({
    site_url              = "https://jackioh.com"
    uri_allow_list        = "https://jackioh.com/login"
    password_hibp_enabled = true
  })
}

resource "supabase_settings" "staging" {
  project_ref = local.supabase_staging_ref

  auth = jsonencode({
    site_url              = "https://staging.jackioh.com"
    uri_allow_list        = "https://staging.jackioh.com/login"
    password_hibp_enabled = true
  })
}

import {
  to = supabase_settings.production
  id = local.supabase_production_ref
}

import {
  to = supabase_settings.staging
  id = local.supabase_staging_ref
}

# The one project's resource was `supabase_settings.auth`, at the project that is now staging's. A
# state that already holds it carries it over; with none, this does nothing.
moved {
  from = supabase_settings.auth
  to   = supabase_settings.staging
}
