# Cloudflare: the jackioh.com zone's DNS records, the two Workers' custom domains (production's
# `jackioh` at jackioh.com, staging's `jackioh-staging` at staging.jackioh.com) and the Access gate
# that keeps staging to the team. Not the Workers, their assets or their containers: wrangler.jsonc
# owns them, and promote-production.yml and deploy-staging.yml deploy them. Nor production's
# workers.dev route, which stays on (wrangler.jsonc's `workers_dev`) because the deploy job's live
# check reads its URL (#256); staging's is off, so Access has no way around it.
#
# The token is CLOUDFLARE_API_TOKEN (README.md, Tokens).
provider "cloudflare" {}

locals {
  # promote-production.yml's CLOUDFLARE_ACCOUNT_ID, and the R2 endpoint's in versions.tf.
  cloudflare_account_id = "3c0495fcdd2622a08e1a91b8de248b46"
}

data "cloudflare_zone" "jackioh" {
  filter = {
    name = "jackioh.com"
  }
}

# Production's address (docs/deploy-cloudflare.md). Cloudflare makes the hostname's DNS
# record itself and keeps it read-only, so that record is not a cloudflare_dns_record here: removing
# the custom domain removes it, and the site with it. www.jackioh.com gets a block like this one
# only once it serves (#256); until then the zone has no record for it.
resource "cloudflare_workers_custom_domain" "jackioh_com" {
  account_id = local.cloudflare_account_id
  zone_id    = data.cloudflare_zone.jackioh.id
  hostname   = "jackioh.com"
  service    = "jackioh"
}

import {
  to = cloudflare_workers_custom_domain.jackioh_com
  # <account id>/<custom domain id>, from the account's /workers/domains (README.md, The first
  # import, step 2).
  id = "${local.cloudflare_account_id}/4a0a902b4d034376d00a1f4e6286cadf7f927235"
}

# No cloudflare_dns_record: at the first import (2026-10-25) the zone's one record was the custom
# domain's own, read-only AAAA. A record added to the zone gets one here, with its import block
# (<zone id>/<record id>; README.md, The first import, step 2): a record this file does not name is
# one Terraform neither reads nor changes.

# Staging's address (docs/deploy-cloudflare.md). Like the apex, Cloudflare makes and keeps its DNS
# record. `jackioh-staging` must exist before the first apply: deploy-staging.yml's first run makes it.
resource "cloudflare_workers_custom_domain" "staging_jackioh_com" {
  account_id = local.cloudflare_account_id
  zone_id    = data.cloudflare_zone.jackioh.id
  hostname   = "staging.jackioh.com"
  service    = "jackioh-staging"
}

# Staging is internal: Access answers every request to staging.jackioh.com, the API and the sockets
# included, with a sign-in (a one-time code by email) and lets in only the team's addresses. A
# signed-in browser carries Access's cookie on the same-origin API calls and WebSockets, so the app
# works unchanged behind it.
variable "staging_team_emails" {
  description = "The addresses Access lets into staging. Personal data in a public repository, so they live in infra/terraform.tfvars, which git ignores (README.md, Running it)."
  type        = list(string)
}

resource "cloudflare_zero_trust_access_policy" "staging_team" {
  account_id = local.cloudflare_account_id
  name       = "JackiOh staging: the team"
  decision   = "allow"
  include    = [for email in var.staging_team_emails : { email = { email = email } }]
}

# deploy-staging.yml's live check: a service token (made in the dashboard, so its secret stays out of
# the state; its pair is the CF_ACCESS_CLIENT_ID and CF_ACCESS_CLIENT_SECRET repository secrets).
resource "cloudflare_zero_trust_access_policy" "staging_service_tokens" {
  account_id = local.cloudflare_account_id
  name       = "JackiOh staging: deploy checks"
  decision   = "non_identity"
  include    = [{ any_valid_service_token = {} }]
}

resource "cloudflare_zero_trust_access_application" "staging" {
  account_id       = local.cloudflare_account_id
  name             = "JackiOh staging"
  type             = "self_hosted"
  domain           = "staging.jackioh.com"
  session_duration = "168h"
  policies = [
    { id = cloudflare_zero_trust_access_policy.staging_team.id, precedence = 1 },
    { id = cloudflare_zero_trust_access_policy.staging_service_tokens.id, precedence = 2 },
  ]
}
