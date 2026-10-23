# Cloudflare: the jackioh.com zone's DNS records and the `jackioh` Worker's custom domains. Not the
# Worker or its assets: wrangler.jsonc owns them, and promote-production.yml deploys them. Nor the
# workers.dev route, which stays on (wrangler's default) because the deploy job's `check the live
# site` step reads its URL (#256).
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

# Production's address (docs/deploy-cloudflare.md §4, Step 3). Cloudflare makes the hostname's DNS
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
  # <account id>/<custom domain id>. The id is the one value here only the account shows: README.md,
  # The first import, step 2.
  id = "${local.cloudflare_account_id}/FIRST_IMPORT_CUSTOM_DOMAIN_ID"
}

# Every other DNS record of the zone is a cloudflare_dns_record with its own import block
# (<zone id>/<record id>), added at the first import (README.md, The first import, step 2): none was
# known when this file was written, and a record this file does not name is one Terraform neither
# reads nor changes.
