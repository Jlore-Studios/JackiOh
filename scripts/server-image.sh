#!/usr/bin/env bash
# The game server's container image for a deploy (deploy-staging.yml, promote-production.yml's
# `deploy` job), pushed to Cloudflare's registry once per set of inputs instead of once per deploy.
#
#   scripts/server-image.sh use <container>   before `wrangler deploy`: <container> is a `name` of
#                                             wrangler.jsonc's `containers` (`jackioh-server`,
#                                             production's; `jackioh-staging-server`, staging's)
#   scripts/server-image.sh prune <tag>       after a deploy of <tag> whose server reported its commit
#
# Left to itself, `wrangler deploy` builds crates/server/Dockerfile from nothing on the runner's
# Docker for every deploy (7 to 8 minutes of a cold release build) and pushes a tag of its own each
# time, even when the commit changed nothing the image is built from. `use` names the image by its
# inputs instead: a hash of what the Dockerfile copies from the build context (its `COPY` lines that
# are not `--from` a stage, read from the Dockerfile itself, so a new COPY line is hashed with no
# change here) and of the Dockerfile, as git holds them at HEAD. Both sites' images go in one
# repository, REPOSITORY, tagged `<UTC time>-<hash>` (TAG_PATTERN), so production finds the image
# staging already pushed for the same inputs. Then, in order:
#
#   1. a tag of REPOSITORY ending in that hash: the image is already there, and nothing is built or
#      pushed. The Worker still restarts the server for the new commit (apps/edge/src/index.ts,
#      `retireOlderImage`), so the deploy reports it as before.
#   2. IMAGE_COMMIT, a commit of main: the `deploy-image` artifact of its push-to-main CI run, which
#      `db (deploy image)` builds from the same Dockerfile and context as wrangler does
#      (crates/server/tests/deploy/rehearse.sh reads both from wrangler.jsonc), kept one day. Its
#      inputs are HEAD's: staging deploys that commit, and production holds nothing main does not.
#   3. Otherwise a build, with buildx reading IMAGE_CACHE_FROM (`--cache-from` specs separated by
#      spaces: CI's GitHub Actions layer cache) when it is set, or plain `docker build`.
#
# A new image is pushed with `wrangler containers push`. Either way the container's `image` in
# WRANGLER_CONFIG (default wrangler.jsonc, the runner's checkout, never committed) is pointed at the
# registry's reference and its `image_build_context` dropped, so `wrangler deploy` builds nothing.
# Run it before anything else writes to the checkout: it refuses inputs that differ from HEAD. The
# tag goes to $GITHUB_OUTPUT as `tag`, with `pushed=true|false`.
#
# `prune` keeps the registry under Cloudflare's 50 GB an account: wrangler's own pushes left a tag
# for every deploy (41 for staging in its first day). It deletes REPOSITORY's tags but the one just
# deployed and the newest KEPT of this script's (so a Worker rolled back a few versions, or a deploy
# of the other site in flight, still finds its image), and every tag of LEGACY_REPOSITORIES, the
# repository wrangler pushed staging's to; never one whose digest a container application runs
# (`wrangler containers list`, whether it names its image by digest or by tag: the other site's
# may be older than every kept tag) or the deployed tag's, since deleting a tag deletes its
# manifest.
#
# Needs CLOUDFLARE_API_TOKEN and CLOUDFLARE_ACCOUNT_ID, as `wrangler deploy` does; Node with the
# workspace installed (wrangler from apps/edge, json5 to check the config it rewrites); and for
# IMAGE_COMMIT, GH_TOKEN and GH_REPO for `gh`.
set -euo pipefail

REPOSITORY=${IMAGE_REPOSITORY:-jackioh-server}
LEGACY_REPOSITORIES=${IMAGE_LEGACY_REPOSITORIES:-jackioh-staging-server}
TAG_PATTERN='^[0-9]{14}-[0-9a-f]{12}$'
KEPT=${IMAGES_KEPT:-10}
REGISTRY=registry.cloudflare.com
CONFIG=${WRANGLER_CONFIG:-wrangler.jsonc}
MANIFEST_TYPES='application/vnd.oci.image.manifest.v1+json, application/vnd.docker.distribution.manifest.v2+json, application/vnd.oci.image.index.v1+json, application/vnd.docker.distribution.manifest.list.v2+json'

say() { echo "server-image: $*"; }
fail() { echo "::error::server-image: $*" >&2; exit 1; }
wrangler() { pnpm --dir apps/edge exec wrangler "$@"; }
output() { if [ -n "${GITHUB_OUTPUT:-}" ]; then echo "$1" >> "$GITHUB_OUTPUT"; fi; }
sha256() { if command -v sha256sum >/dev/null; then sha256sum; else shasum -a 256; fi; }

[ $# -ge 2 ] || fail "usage: $0 use <container> | prune <tag>"
command=$1
[ -n "${CLOUDFLARE_ACCOUNT_ID:-}" ] || fail "CLOUDFLARE_ACCOUNT_ID is not set"
[ -n "${CLOUDFLARE_API_TOKEN:-}" ] || fail "CLOUDFLARE_API_TOKEN is not set"

# The container's `<image>\t<context>` in the config, or with $2, its image pointed at that registry
# reference and its build context dropped, checked by parsing the file back.
config() {
  node - "$CONFIG" "$1" "${2:-}" <<'NODE'
const { createRequire } = require("node:module");
const fs = require("node:fs");
const path = require("node:path");
const [file, name, image] = process.argv.slice(2);
let JSON5;
for (const dir of ["apps/edge", "apps/web", "."]) {
  try { JSON5 = createRequire(path.resolve(dir, "package.json"))("json5"); break; } catch {}
}
if (!JSON5) throw new Error("json5 is not installed: run pnpm install first");
const find = (text) => {
  const parsed = JSON5.parse(text);
  const all = [parsed, ...Object.values(parsed.env ?? {})].flatMap((scope) => scope.containers ?? []);
  const hits = all.filter((c) => c.name === name);
  if (hits.length !== 1) throw new Error(`${file} has ${hits.length} containers named ${name}, not one`);
  return hits[0];
};
const text = fs.readFileSync(file, "utf8");
const container = find(text);
if (!image) {
  process.stdout.write(`${container.image}\t${container.image_build_context ?? "."}\n`);
  process.exit(0);
}
// The lines themselves, so the comments stay: the block's `"image"` and `"image_build_context"`
// after its `"name"` line, before the block closes.
const lines = text.split("\n");
const start = lines.findIndex((l) => new RegExp(`^\\s*"name":\\s*"${name}"`).test(l));
let done = false;
for (let i = start + 1; start >= 0 && i < lines.length && !/^\s*}/.test(lines[i]); i++) {
  if (/^\s*"image":/.test(lines[i])) {
    lines[i] = lines[i].replace(/"image":\s*"[^"]*"/, `"image": "${image}"`);
    done = true;
  } else if (/^\s*"image_build_context":/.test(lines[i])) {
    lines.splice(i, 1);
    i--;
  }
}
const written = lines.join("\n");
const check = find(written);
if (!done || check.image !== image || "image_build_context" in check) {
  throw new Error(`could not point ${name}'s image at ${image} in ${file}`);
}
fs.writeFileSync(file, written);
NODE
}

# A repository's tags in the registry, one a line.
tags() {
  wrangler containers images list --json | node -e '
    let s = "";
    process.stdin.on("data", (d) => (s += d)).on("end", () => {
      const name = process.argv[1];
      for (const r of JSON.parse(s)) if (r.name === name || r.name.endsWith("/" + name)) for (const t of r.tags) console.log(t);
    });' "$1"
}

# CI's image of IMAGE_COMMIT, loaded and tagged $1, or nothing (and a notice) when there is none.
from_ci() {
  local run dir loaded
  [ -n "${IMAGE_COMMIT:-}" ] || return 1
  run=$(gh run list --workflow ci.yml --branch main --event push --status success --commit "$IMAGE_COMMIT" \
    --limit 1 --json databaseId --jq '.[0].databaseId // empty' || true)
  if [ -z "$run" ]; then
    echo "::notice::server-image: no passing push-to-main CI run of ${IMAGE_COMMIT:0:12}, so the image is built"
    return 1
  fi
  dir=$(mktemp -d)
  if ! gh run download "$run" -n deploy-image -D "$dir" >/dev/null 2>&1 || [ ! -f "$dir/deploy-image.tar" ]; then
    echo "::notice::server-image: CI run $run kept no deploy-image (expired after a day, or the rehearsal did not run), so the image is built"
    rm -rf "$dir"
    return 1
  fi
  loaded=$(docker load -i "$dir/deploy-image.tar" | sed -n 's/^Loaded image: //p' | tail -n 1)
  rm -rf "$dir"
  [ -n "$loaded" ] || { echo "::notice::server-image: CI run $run's deploy-image holds no tagged image, so the image is built"; return 1; }
  docker tag "$loaded" "$1"
  say "loaded CI run $run's $loaded as $1"
}

use() {
  local container=$1 spec dockerfile context hash existing tag local_tag paths=()
  spec=$(config "$container")
  dockerfile=${spec%%$'\t'*}
  context=${spec#*$'\t'}
  case $dockerfile in
    "$REGISTRY"/*) fail "$CONFIG already points $container at $dockerfile" ;;
  esac
  dockerfile=${dockerfile#./}
  [ "$context" = . ] || fail "$container's image_build_context is $context: this script hashes from the repository's root"
  [ -f "$dockerfile" ] || fail "$dockerfile (the image of $container in $CONFIG) does not exist"

  # What the Dockerfile copies from the build context: each `COPY` that is not `--from` a stage
  # names its sources before its destination. Sorted bytewise, so every runner hashes them in one
  # order whatever its locale.
  local p
  while IFS= read -r p; do paths+=("$p"); done < <(sed -n 's/^COPY[[:space:]]\{1,\}//p' "$dockerfile" |
    grep -v '^--' | awk '{ for (i = 1; i < NF; i++) print $i }' | LC_ALL=C sort -u)
  [ ${#paths[@]} -gt 0 ] || fail "$dockerfile copies nothing from its context"
  if [ -n "$(git status --porcelain --ignored -- "${paths[@]}" "$dockerfile")" ]; then
    fail "the image's inputs (${paths[*]}) differ from HEAD, so a tag named for HEAD's would not be this image"
  fi
  hash=$(for p in "${paths[@]}" "$dockerfile"; do git rev-parse "HEAD:$p"; done | sha256 | cut -c1-12)
  say "$container's image is built from ${paths[*]} and $dockerfile: inputs $hash"

  existing=$(tags "$REPOSITORY" | grep -E "$TAG_PATTERN" | grep -E -- "-$hash\$" | sort | tail -n 1 || true)
  if [ -n "$existing" ]; then
    tag=$existing
    say "the registry already holds $REPOSITORY:$tag: nothing to build or push"
    output "pushed=false"
  else
    tag="$(date -u +%Y%m%d%H%M%S)-$hash"
    local_tag="$REPOSITORY:$tag"
    if ! from_ci "$local_tag"; then
      if [ -n "${IMAGE_CACHE_FROM:-}" ]; then
        local args=() cache
        for cache in $IMAGE_CACHE_FROM; do args+=(--cache-from "$cache"); done
        docker buildx build "${args[@]}" --load -t "$local_tag" -f "$dockerfile" "$context"
      else
        docker build -t "$local_tag" -f "$dockerfile" "$context"
      fi
    fi
    wrangler containers push "$local_tag"
    output "pushed=true"
  fi
  config "$container" "$REGISTRY/$CLOUDFLARE_ACCOUNT_ID/$REPOSITORY:$tag"
  say "$CONFIG now deploys $container from $REGISTRY/$CLOUDFLARE_ACCOUNT_ID/$REPOSITORY:$tag"
  output "tag=$tag"
}

prune() {
  local keep=$1 password auth live keep_digest apps image all candidates repository t digest
  password=$(curl -fsS -X POST \
    -H "Authorization: Bearer $CLOUDFLARE_API_TOKEN" -H "Content-Type: application/json" \
    -d '{"expiration_minutes": 5, "permissions": ["pull"]}' \
    "https://api.cloudflare.com/client/v4/accounts/$CLOUDFLARE_ACCOUNT_ID/containers/registries/$REGISTRY/credentials" |
    jq -r '.result.password // empty')
  [ -n "$password" ] || fail "Cloudflare gave no registry credentials"
  auth=$(printf 'v1:%s' "$password" | base64 | tr -d '\n')
  if [ -n "${GITHUB_ACTIONS:-}" ]; then echo "::add-mask::$password"; echo "::add-mask::$auth"; fi
  digest_of() {
    curl -fsSI -H "Authorization: Basic $auth" -H "Accept: $MANIFEST_TYPES" \
      "https://$REGISTRY/v2/$CLOUDFLARE_ACCOUNT_ID/$1/manifests/$2" |
      tr -d '\r' | awk -F': ' 'tolower($1) == "docker-content-digest" { print $2 }'
  }

  # The digests no tag may take with it: the deployed tag's, what each container application runs
  # (named by digest, as wrangler's own pushes left it, or by a tag, as `use` leaves it), and what
  # the tags kept below point at (two tags can share one). An application whose image cannot be
  # told, or none listed at all, stops the prune before it deletes anything.
  keep_digest=$(digest_of "$REPOSITORY" "$keep" || true)
  [ -n "$keep_digest" ] || fail "the registry has no $REPOSITORY:$keep"
  live=$keep_digest
  apps=$(wrangler containers list --json | jq -r '.[] | .image // .configuration.image // "?"')
  [ -n "$apps" ] || fail "wrangler lists no container application, so what runs is unknown: nothing deleted"
  while IFS= read -r image; do
    case $image in
      *@sha256:*) digest=${image##*@} ;;
      "$REGISTRY/$CLOUDFLARE_ACCOUNT_ID/"*:*)
        t=${image#"$REGISTRY/$CLOUDFLARE_ACCOUNT_ID/"}
        digest=$(digest_of "${t%%:*}" "${t#*:}" || true)
        ;;
      *) digest= ;;
    esac
    [ -n "$digest" ] || fail "a container application runs $image, whose digest the registry does not give: nothing deleted"
    say "a container application runs $image ($digest)"
    live=$(printf '%s\n%s\n' "$live" "$digest")
  done <<<"$apps"
  for repository in "$REPOSITORY" $LEGACY_REPOSITORIES; do
    all=$(tags "$repository")
    if [ "$repository" = "$REPOSITORY" ]; then
      # All but the deployed tag and the newest KEPT - 1 others of this script's.
      local mine kept
      mine=$(grep -E "$TAG_PATTERN" <<<"$all" | grep -vx -- "$keep" | sort -r || true)
      kept=$(head -n $((KEPT - 1)) <<<"$mine")
      for t in $kept; do
        digest=$(digest_of "$repository" "$t" || true)
        [ -n "$digest" ] || fail "the registry gives no digest for $repository:$t, a tag kept: nothing deleted"
        live=$(printf '%s\n%s\n' "$live" "$digest")
      done
      candidates=$(
        {
          grep -vE "$TAG_PATTERN" <<<"$all" || true
          tail -n +"$KEPT" <<<"$mine"
        } | grep -v '^$' || true
      )
    else
      candidates=$(grep -v '^$' <<<"$all" || true)
    fi
    say "$repository: $(grep -c . <<<"$all" || true) tags, $(grep -c . <<<"$candidates" || true) past keeping"
    for t in $candidates; do
      digest=$(digest_of "$repository" "$t" || true)
      if [ -z "$digest" ]; then say "$repository:$t has no manifest: left as it is"; continue; fi
      if grep -qx -- "$digest" <<<"$live"; then
        say "$repository:$t is running or kept ($digest): left as it is"
        continue
      fi
      wrangler containers images delete "$repository:$t" -y
    done
  done
}

case $command in
  use) use "$2" ;;
  prune) prune "$2" ;;
  *) fail "unknown command $command: use or prune" ;;
esac
