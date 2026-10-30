# infra: the hosts' dashboard settings as Terraform

This module is the contract for the settings around the hosts, the ones only a dashboard used to
hold (#1708). Each host's build and routing was already in code, and stays there. A setting this
module owns is changed here, by a pull request, and never in the dashboard: a dashboard edit is drift,
and the next `terraform plan` shows it. Terraform 1.6 or later; OpenTofu reads the same files.

## What it owns, and what it leaves

| Host | Owned here | File | Left to |
| --- | --- | --- | --- |
| Cloudflare | The jackioh.com zone's DNS records, and the `jackioh` Worker's custom domains (`jackioh.com`, and `www.jackioh.com` once it serves) | `cloudflare.tf` | `wrangler.jsonc` (the Worker, its assets, its workers.dev route), deployed by `promote-production.yml` |
| Supabase (`exmjdaswedxhnzmpzqrq`) | Authentication's Site URL and Redirect URLs | `supabase.tf` | the migrations, which `jackioh-server release` applies; the keys, which are secrets |
| GitHub (`Jlore-Studios/JackiOh`) | `main`'s branch protection with its five required checks, `main`'s merge queue (the `main-merge-queue` ruleset), and the `production` ruleset | `github.tf` | `bot/go/internal/config` `Labels` (the labels, which the sweep restores) |
| Render | nothing | | `render.yaml`, the Blueprint: a Terraform-managed service would fight it |
| Vercel | nothing | | `vercel.json`: its build, routes and headers |

Not here at all: secret values (Actions secrets, Render's `sync: false` variables, the deploy hook
URLs), because the state would hold them. Render's `PUBLIC_ORIGINS` is the one non-secret setting
still only in a dashboard (#256's sign-in outage of 2026-10-04); it needs its own change.

`docs/deploy-cloudflare.md` §4 Steps 2, 3, 4 and 6 point here for the settings above.

`versions.tf` pins Terraform and the three providers, exactly, and `.terraform.lock.hcl` holds their
checksums for Linux, macOS (Apple silicon and Intel) and Windows; both are committed. Bumping a
provider is a pull request that changes its `version` and the lock file together (`terraform
providers lock`, below).

## Tokens

Each provider reads its token from the environment, and no file here and no output holds one:

| Variable | What it is | Scope |
| --- | --- | --- |
| `CLOUDFLARE_API_TOKEN` | A Cloudflare API token of its own (My Profile -> API Tokens -> Create Custom Token), not the deploy job's | Account: Workers Scripts, Edit. Zone `jackioh.com`: Zone, Read; DNS, Edit; Workers Routes, Edit |
| `SUPABASE_ACCESS_TOKEN` | A Supabase personal access token (Account -> Access Tokens) of a member of the project | the whole account: Supabase's tokens have no narrower scope |
| `GITHUB_TOKEN` | A GitHub token of a repository admin | classic: `repo`; fine-grained, on this repository only: Administration, read and write |
| `AWS_ACCESS_KEY_ID`, `AWS_SECRET_ACCESS_KEY` | The S3 pair of an R2 API token (R2 -> Manage API tokens), for the state | Object Read & Write, on the state bucket only |

A plan with read-only tokens works; an apply needs the write scopes.

## State

The state holds whatever the providers read, so it lives in a remote backend and never in git
(`.gitignore` ignores `.terraform/`, `*.tfstate`, `*.tfstate.*` and `*.tfvars`). `versions.tf` keeps
it in the Cloudflare R2 bucket `jackioh-terraform-state` through the `s3` backend, since the account
already exists. That is a recommendation: whoever sets the module up may pick another backend by
changing that block, before the first `init`. There is no state lock, so one person applies at a
time.

## Running it

From the repository root, with the variables above exported:

```
terraform -chdir=infra init       # the backend and the pinned providers
terraform -chdir=infra plan       # what would change; "No changes." when the hosts match the files
terraform -chdir=infra apply      # makes the hosts match the files, after showing the same plan
```

A change to a setting is one pull request: edit the file, run `plan`, put its output on the pull
request (no token is in it), and `apply` once it merges. A `plan` that wants to destroy or replace a
custom domain, a DNS record, the branch protection or the ruleset would take jackioh.com down or
leave a branch unprotected (CLAUDE.md rule 13): stop and find out why instead of applying it.

CI runs `terraform fmt -check -recursive`, `terraform init -backend=false` and `terraform validate`
on a pull request that touches `infra/` (`ci.yml`'s `rust (checks, fuzz 200)`, on
`scripts/ci-scope.sh`'s `infra` line), and fails when `init` had to write a lock file other than the
committed one. `plan` and `apply` need the tokens, so they stay manual.

## The first import

Every resource here that already existed on the hosts comes in through an `import` block beside it,
so the first `apply` changes nothing on them. The `production` ruleset did not exist (the repository
had no rulesets on 2026-10-25), so the first `apply` creates it, which only adds protection. Once in
the state, an `import` block does nothing, so they stay as the record of where each resource came
from.

1. **The state bucket.** Create the bucket (Cloudflare -> R2 -> Create bucket,
   `jackioh-terraform-state`) and the R2 API token for it.
2. **The ids only the accounts show** (filled in on 2026-10-25; redo this for a resource added
   later):
   - The custom domain's id in `cloudflare.tf`: the `id` of `jackioh.com` in
     `curl -sS -H "Authorization: Bearer $CLOUDFLARE_API_TOKEN" https://api.cloudflare.com/client/v4/accounts/3c0495fcdd2622a08e1a91b8de248b46/workers/domains`.
     If that list also has `www.jackioh.com`, give it a `cloudflare_workers_custom_domain` and an
     `import` block like the apex's (it had none on 2026-10-25).
   - The DNS records: the zone's id is on its Overview page, and
     `curl -sS -H "Authorization: Bearer $CLOUDFLARE_API_TOKEN" https://api.cloudflare.com/client/v4/zones/<zone id>/dns_records`
     lists its records. A custom domain's own record (read-only, made by the custom domain) stays
     out; every other record gets a `cloudflare_dns_record` in `cloudflare.tf`, its
     `name` the full hostname and its `type`, `content`, `ttl`, `proxied` (and `priority` for MX)
     as listed, with an `import` block whose `id` is `<zone id>/<record id>`. On 2026-10-25 the
     custom domain's record was the only one.
   - A ruleset that already exists is imported as `<repository>:<ruleset id>`
     (`gh api repos/Jlore-Studios/JackiOh/rulesets` lists them); `production`'s did not exist.
3. **The lock file.** `terraform -chdir=infra providers lock -platform=linux_amd64
   -platform=darwin_arm64 -platform=darwin_amd64 -platform=windows_amd64` writes
   `.terraform.lock.hcl`; commit it.
4. **Plan.** `init`, then `plan`. It reads `Plan: 3 to import, 1 to add, 1 to change, 0 to
   destroy`. The one add is the `production` ruleset, and the one change is `supabase_settings.auth`:
   an import reads every Supabase setting
   into the state, and the first apply drops all but the Site URL and the Redirect URLs from it,
   sending Supabase those two values as they already are. Any other change means a file differs
   from the host. Make the file say what the host says, and when the host is the one that is wrong,
   say so on the pull request and let a person decide: a Redirect URL the list lacks (local
   development's `http://localhost:5173/login`, say), or `strict` on `main`'s protection, which
   `training/README.md` expects on ("require branches to be up to date") and the night bot's
   `setup --repo-settings` writes off.
5. **Apply**, then `plan` again: `No changes.` Put that output on the pull request, with anything
   you would rather not publish (none of it is a secret) redacted.
6. **The drift test.** Remove one Redirect URL in Supabase's dashboard (Authentication -> URL
   Configuration). `plan` shows `uri_allow_list` changing back, and `apply` puts it back.

## The merge queue

`main-merge-queue` is a ruleset of its own, beside the branch protection, so the five checks stay
where the bots read them (`bot/go/internal/gh` `RequiredChecks`). It did not exist, so its first
`apply` creates it (`Plan: 1 to add`), and from then on nothing merges into `main` except through the
queue: a merge button, `gh pr merge --auto` and the bots' auto-merge all put the pull request in the
queue once its own checks pass, and the queue lands it after ci.yml's `merge_group` run passes on
`main` with it on top. Take the `merge_group` trigger in ci.yml to `main` before applying it: a
queue whose checks never report takes every pull request out at the timeout.

After the first `apply`, watch the first few merges. Each queued pull request shows a
`gh-readonly-queue/main/pr-<n>-<sha>` run of CI; the squash commit on `main` should be the pull
request's title and number, as before (if it is the bot's own commit message instead, set Settings
-> General -> Pull Requests -> Allow squash merging -> Default commit message to "Pull request
title"). To undo it, delete the `main_merge_queue` resource and apply, or set its `enforcement` to
`disabled`: pull requests then merge as before, straight onto `main`.
