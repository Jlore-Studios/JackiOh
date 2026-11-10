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

# #3485: Cloudflare answered http://jackioh.com with the page itself, so Google filed it
# as an alternate of the https:// canonical. Always Use HTTPS answers every http request
# with a 301 to https instead; public/_headers' HSTS keeps browsers there. This updates the
# zone's existing setting in place (no import block: there is no separate object to import).
# A person runs plan/apply with the tokens (README.md, Running it).
resource "cloudflare_zone_setting" "always_use_https" {
  zone_id    = data.cloudflare_zone.jackioh.id
  setting_id = "always_use_https"
  value      = "on"
}

# The zone's records beyond the custom domains' own, read-only AAAAs, each with its import block
# (<zone id>/<record id>, the zone's id written out, since an import's id must be known before the
# plan reads the zone; README.md, The first import, step 2): a record this file does not name is one
# Terraform neither reads nor changes.
#
# Sign-in mail (docs/deploy-cloudflare.md section 8, Email): Resend sends both Supabase projects'
# mail from no-reply@jackioh.com. Resend's own setup made its three records on 2026-11-02 (the return
# path at send. and rsend., and the DKIM key); the DMARC policy only reports (p=none), which
# mailboxes ask of a sender. Rotating the DKIM key in Resend changes resend_dkim's content.
resource "cloudflare_dns_record" "resend_send" {
  zone_id = data.cloudflare_zone.jackioh.id
  name    = "send.jackioh.com"
  type    = "CNAME"
  content = "send.forge.rmta.net"
  ttl     = 3600
  proxied = false
}

import {
  to = cloudflare_dns_record.resend_send
  id = "54d7c89c10cc481c27e19f0922f6ca7a/315cf8354677e75bf55ea2ab867119e2"
}

resource "cloudflare_dns_record" "resend_rsend" {
  zone_id = data.cloudflare_zone.jackioh.id
  name    = "rsend.jackioh.com"
  type    = "CNAME"
  content = "rsend.forge.rmta.net"
  ttl     = 3600
  proxied = false
}

import {
  to = cloudflare_dns_record.resend_rsend
  id = "54d7c89c10cc481c27e19f0922f6ca7a/4c131fb7e09ee55df4b92ed6acfe4999"
}

resource "cloudflare_dns_record" "resend_dkim" {
  zone_id = data.cloudflare_zone.jackioh.id
  name    = "resend._domainkey.jackioh.com"
  type    = "TXT"
  content = "\"p=MIGfMA0GCSqGSIb3DQEBAQUAA4GNADCBiQKBgQDvEf8PLSg2wCTNiFMyRj1CScOyXTv31Wy1Be2le+542aGeUECxT821OXqw7KX7U5YDvJHdxlybsyapfqx8v47Lnp5xtKKHogI7kKDHaJh6HnqWcyXSTEIukDQe+RN4MwMjw3MAQDc2Lw0QiD0CahlolLBTRCW36sivK90ooBtCxQIDAQAB\""
  ttl     = 3600
  proxied = false
}

import {
  to = cloudflare_dns_record.resend_dkim
  id = "54d7c89c10cc481c27e19f0922f6ca7a/d3f383aefb311acb8a907ef091bd1688"
}

resource "cloudflare_dns_record" "dmarc" {
  zone_id = data.cloudflare_zone.jackioh.id
  name    = "_dmarc.jackioh.com"
  type    = "TXT"
  content = "\"v=DMARC1; p=none;\""
  ttl     = 3600
  proxied = false
}

import {
  to = cloudflare_dns_record.dmarc
  id = "54d7c89c10cc481c27e19f0922f6ca7a/dcd252b694315c0f8c5c2c7e1616580b"
}


# Staging's address (docs/deploy-cloudflare.md). Like the apex, Cloudflare makes and keeps its DNS
# record. `jackioh-staging` must exist before the first apply: deploy-staging.yml's first run makes it.
resource "cloudflare_workers_custom_domain" "staging_jackioh_com" {
  account_id = local.cloudflare_account_id
  zone_id    = data.cloudflare_zone.jackioh.id
  hostname   = "staging.jackioh.com"
  service    = "jackioh-staging"
}

# Made through the API on 2026-11-02, with the Access application and its two policies below, so
# staging was behind Access from its first request; these blocks adopt them at the first apply.
import {
  to = cloudflare_workers_custom_domain.staging_jackioh_com
  id = "${local.cloudflare_account_id}/d82eb1df005a97fe5d9bafaf09a03245ca6917cc"
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

import {
  to = cloudflare_zero_trust_access_policy.staging_team
  id = "${local.cloudflare_account_id}/c7a64c29-e1cf-432a-a40f-2ce74e650bac"
}

# deploy-staging.yml's live check: a service token (made in the dashboard, so its secret stays out of
# the state; its pair is the CF_ACCESS_CLIENT_ID and CF_ACCESS_CLIENT_SECRET repository secrets).
resource "cloudflare_zero_trust_access_policy" "staging_service_tokens" {
  account_id = local.cloudflare_account_id
  name       = "JackiOh staging: deploy checks"
  decision   = "non_identity"
  include    = [{ any_valid_service_token = {} }]
}

import {
  to = cloudflare_zero_trust_access_policy.staging_service_tokens
  id = "${local.cloudflare_account_id}/2f4b49c6-3431-4392-837c-406accc64dde"
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

import {
  to = cloudflare_zero_trust_access_application.staging
  id = "accounts/${local.cloudflare_account_id}/e196957e-0899-490e-9471-1f8d84c69d55"
}

# The dev dashboard (apps/admin, R3074000): a Worker of its own per environment, `jackioh-admin`
# at admin.jackioh.com and `jackioh-admin-staging` at admin-staging.jackioh.com (one label deep, so
# the zone's own certificate covers it), each behind Access for the dev accounts' addresses alone,
# on top of the account's own sign-in and the server's DEV_PROFILE_IDS check. apps/admin/wrangler.jsonc
# owns the Workers, with no workers.dev or preview address; promote-production.yml and
# deploy-staging.yml deploy them, so each Worker exists before the apply that gives it its domain
# (apps/admin/README.md says the order). Each domain waits on its Access application, so it never
# serves a request Access has not seen.
variable "admin_emails" {
  description = "The addresses Access lets into the dev dashboard: the dev accounts' own. Personal data in a public repository, so they live in infra/terraform.tfvars, which git ignores (README.md, Running it)."
  type        = list(string)
}

resource "cloudflare_zero_trust_access_policy" "admin_devs" {
  account_id = local.cloudflare_account_id
  name       = "JackiOh dev dashboard: the devs"
  decision   = "allow"
  include    = [for email in var.admin_emails : { email = { email = email } }]
}

resource "cloudflare_zero_trust_access_application" "admin" {
  account_id       = local.cloudflare_account_id
  name             = "JackiOh dev dashboard"
  type             = "self_hosted"
  domain           = "admin.jackioh.com"
  session_duration = "12h"
  policies = [
    { id = cloudflare_zero_trust_access_policy.admin_devs.id, precedence = 1 },
  ]
}

resource "cloudflare_zero_trust_access_application" "admin_staging" {
  account_id       = local.cloudflare_account_id
  name             = "JackiOh dev dashboard (staging)"
  type             = "self_hosted"
  domain           = "admin-staging.jackioh.com"
  session_duration = "12h"
  policies = [
    { id = cloudflare_zero_trust_access_policy.admin_devs.id, precedence = 1 },
  ]
}

resource "cloudflare_workers_custom_domain" "admin_jackioh_com" {
  account_id = local.cloudflare_account_id
  zone_id    = data.cloudflare_zone.jackioh.id
  hostname   = "admin.jackioh.com"
  service    = "jackioh-admin"
  depends_on = [cloudflare_zero_trust_access_application.admin]
}

resource "cloudflare_workers_custom_domain" "admin_staging_jackioh_com" {
  account_id = local.cloudflare_account_id
  zone_id    = data.cloudflare_zone.jackioh.id
  hostname   = "admin-staging.jackioh.com"
  service    = "jackioh-admin-staging"
  depends_on = [cloudflare_zero_trust_access_application.admin_staging]
}
