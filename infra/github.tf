# GitHub: `main`'s branch protection with its five required checks, `main`'s merge queue, the
# `production` ruleset, and the repository's Actions policy and workflow-token settings.
# Not the labels: bot/go/internal/config `Labels` owns them, and the sweep restores them. Not the
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
# night bot's `setup --repo-settings` writes (bot/go/internal/gh `SetProtection`), so neither undoes
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

# `main`'s merge queue. `strict` above stays off: each pull request's CI ran against `main` as it
# was, so two pull requests each green alone landed red together (four of the five red runs on main
# of 2026-10-29), and `strict` on instead would send every open pull request back to update and
# re-run whenever one merged, several times an hour. The queue lands pull requests in the order they
# became ready to merge. It builds `main` with every one ahead and the next on top, in parallel
# (`max_entries_to_build`), runs ci.yml's `merge_group` there, waits for the same five checks the
# protection above requires, and takes out any whose own build fails, rebuilding those behind it
# without it. `ALLGREEN`: every commit it lands passed CI as it lands, which promote-production.yml
# relies on when it picks a commit of `main`. The timeout is past CI's worst wait for runners
# (90 minutes on 2026-10-29, nearly all of it queued): at the default 60 the queue would take a
# green pull request out for the runners' slowness. Squash, as every bot and lane merges. The
# repository had no such ruleset, so the first apply creates it: no import block.
resource "github_repository_ruleset" "main_merge_queue" {
  name        = "main-merge-queue"
  repository  = "JackiOh"
  target      = "branch"
  enforcement = "active"

  conditions {
    ref_name {
      include = ["refs/heads/main"]
      exclude = []
    }
  }

  rules {
    merge_queue {
      merge_method                      = "SQUASH"
      grouping_strategy                 = "ALLGREEN"
      max_entries_to_build              = 5
      min_entries_to_merge              = 1
      max_entries_to_merge              = 5
      min_entries_to_merge_wait_minutes = 5
      check_response_timeout_minutes    = 240
    }
  }
}

# docs/deploy-cloudflare.md §2.2: Restrict deletions and Block force pushes on `production`,
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

# The repository's Actions policy and its workflow token's defaults, set by hand on 2026-11-04 and
# held here since.
#
# `allowed_actions` stays `all`. Every workflow here uses actions from outside the repository. From
# 18:47Z to 19:49Z on 2026-11-04 it was `local_only`, set while the organization's page was being
# changed, and every workflow failed at startup.
#
# `can_approve_pull_request_reviews` stays on. promote-production.yml's daily path opens its pull
# request into `production` with the workflow's own token and merges it with PROMOTE_TOKEN, which
# GitHub allows only with this on. It was off until 2026-11-04, and every daily promotion before
# #3054 failed on it (#3029).
#
# The token's default stays `read`: each workflow asks for what it writes in its own
# `permissions:`. The organization's setting of the same name must be on as well; the organization
# is not this file's to hold.
resource "github_actions_repository_permissions" "jackioh" {
  repository      = "JackiOh"
  enabled         = true
  allowed_actions = "all"
}

import {
  to = github_actions_repository_permissions.jackioh
  id = "JackiOh"
}

resource "github_workflow_repository_permissions" "jackioh" {
  repository                       = "JackiOh"
  default_workflow_permissions     = "read"
  can_approve_pull_request_reviews = true
}

import {
  to = github_workflow_repository_permissions.jackioh
  id = "JackiOh"
}
