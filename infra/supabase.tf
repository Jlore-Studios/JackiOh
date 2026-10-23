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

# Emailed links (sign-up confirmation, password reset) come back to the page's own origin's login
# path (`authRedirectUrl` in apps/web/src/net/auth.ts), and Supabase quietly swaps any `redirect_to`
# missing from `uri_allow_list` for `site_url` (docs/deploy-cloudflare.md §4, Step 4). So every site
# that signs in has its /login here: production, and staging while it shares this project.
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
      "https://jackioh.vercel.app/login",
    ])
  })
}

import {
  to = supabase_settings.auth
  id = local.supabase_project_ref
}
