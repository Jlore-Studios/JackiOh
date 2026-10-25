# GitHub: `main`'s branch protection with its five required checks, and the `production` ruleset.
# Not the labels: bot/harness/config.py `LABELS` owns them, and the sweep restores them. Not the
# Actions secrets: state would hold them.
#
# The token is GITHUB_TOKEN, with admin on the repository (README.md, Tokens).
provider "github" {
  owner = "Jlore-Studios"
}

data "github_repository" "jackioh" {
  name = "JackiOh"
}

# The five names ci.yml's summary jobs report and the bots wait on (`required_checks` in
# .harness/config.json and .squishy/config.json); a rename changes all three. The rest is what the
# night bot's `setup --repo-settings` writes (bot/harness/gh.py `set_protection`), so neither undoes
# the other.
resource "github_branch_protection" "main" {
  repository_id = data.github_repository.jackioh.node_id
  pattern       = "main"

  enforce_admins      = false
  allows_deletions    = false
  allows_force_pushes = false

  required_status_checks {
    strict   = false
    contexts = ["rust", "web", "e2e smoke", "db", "bot selftest"]
  }
}

import {
  to = github_branch_protection.main
  id = "JackiOh:main"
}

# docs/deploy-cloudflare.md §4, Step 2: Restrict deletions and Block force pushes on `production`,
# and nothing that would hold back the promotion's pull requests, which no check runs on, or the
# push that creates the branch. The repository had no ruleset at the first import (2026-10-25), so
# this one is created rather than imported: it only adds protection.
resource "github_repository_ruleset" "production" {
  name        = "production"
  repository  = "JackiOh"
  target      = "branch"
  enforcement = "active"

  conditions {
    ref_name {
      include = ["refs/heads/production"]
      exclude = []
    }
  }

  rules {
    deletion         = true
    non_fast_forward = true
  }
}
