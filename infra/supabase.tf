# Supabase: Authentication's Site URL and Redirect URLs of the production project, and nothing else.
# The schema belongs to the migrations, which `jackioh-server release` applies, and the keys are
# secrets (render.yaml's `sync: false` values).
#
# The token is SUPABASE_ACCESS_TOKEN (README.md, Tokens).
provider "supabase" {}

locals {
  # apps/web/.env.production's VITE_SUPABASE_URL, which both sites use.
  supabase_project_ref = "exmjdaswedxhnzmpzqrq"
}

# Emailed links (sign-up confirmation, password reset) and OAuth sign-ins (R666) come back to the
# page's own origin's login path (`authRedirectUrl` in apps/web/src/net/auth.ts), and Supabase
# quietly swaps any `redirect_to` missing from `uri_allow_list` for `site_url`. Staging, which
# shares this project until production has its own (docs/deploy-cloudflare.md), is on the list, so
# its links and sign-ins return to staging: an OAuth sign-in sent to the other origin would fail,
# since its PKCE verifier stays in the browser storage of the origin that started it.
#
# `auth` names only these two keys, and the provider then reads and writes only them. An import reads
# every setting into state, so the first apply narrows the state to these two and sends Supabase the
# same two values back (README.md, The first import).
resource "supabase_settings" "auth" {
  project_ref = local.supabase_project_ref

  auth = jsonencode({
    site_url = "https://jackioh.com"
    uri_allow_list = join(",", [
      "https://jackioh.com/login",
      "https://staging.jackioh.com/login",
    ])
  })
}

import {
  to = supabase_settings.auth
  id = local.supabase_project_ref
}
