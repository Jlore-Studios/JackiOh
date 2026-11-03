# The hosts' dashboard settings as code (#1708): what each provider below owns is in README.md, and
# what it leaves to wrangler.jsonc. OpenTofu reads the same files.
terraform {
  # 1.5 for `import` blocks, 1.6 for the s3 backend's `endpoints` and `use_path_style`.
  required_version = ">= 1.6.0, < 2.0.0"

  # Each provider reads its token from the environment (README.md, Tokens), never from a file here.
  # The versions are exact, and .terraform.lock.hcl holds their checksums.
  required_providers {
    cloudflare = {
      source  = "cloudflare/cloudflare"
      version = "5.28.1"
    }
    supabase = {
      source  = "supabase/supabase"
      version = "1.11.0"
    }
    github = {
      source  = "integrations/github"
      version = "6.13.0"
    }
  }

  # State holds whatever the providers read, so it lives here and never in git (.gitignore): a
  # Cloudflare R2 bucket through the s3 backend, the settings Cloudflare documents for R2. Its
  # credentials are an R2 API token's S3 pair in AWS_ACCESS_KEY_ID and AWS_SECRET_ACCESS_KEY. No
  # lock: one person applies at a time. CI runs `init -backend=false` and never reads it.
  backend "s3" {
    bucket = "jackioh-terraform-state"
    key    = "infra/terraform.tfstate"
    region = "auto"
    endpoints = {
      s3 = "https://3c0495fcdd2622a08e1a91b8de248b46.r2.cloudflarestorage.com"
    }
    skip_credentials_validation = true
    skip_metadata_api_check     = true
    skip_region_validation      = true
    skip_requesting_account_id  = true
    skip_s3_checksum            = true
    use_path_style              = true
  }
}
