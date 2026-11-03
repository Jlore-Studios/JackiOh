// scripts/server-image.sh, the server image the two deploy jobs (deploy-staging.yml, promote-production.yml)
// reuse instead of building: its tag named by its inputs, the three ways it gets an image, the rewrite of
// wrangler.jsonc, and what `prune` deletes. Stand-ins for `pnpm` (wrangler), `docker`, `gh` and `curl` on
// PATH record each call and answer from files, so neither Docker nor Cloudflare is needed. The script runs
// in the repository itself, whose HEAD names the inputs it hashes.

import { spawnSync } from "node:child_process";
import { chmodSync, copyFileSync, mkdtempSync, readFileSync, rmSync, writeFileSync } from "node:fs";
import { tmpdir } from "node:os";
import { dirname, join } from "node:path";
import { fileURLToPath } from "node:url";

import JSON5 from "json5";
import { afterAll, beforeAll, describe, expect, it } from "vitest";

const HERE = dirname(fileURLToPath(import.meta.url));
const SCRIPT = join(HERE, "../../../../scripts/server-image.sh");
const REPO = spawnSync("git", ["rev-parse", "--show-toplevel"], { cwd: HERE, encoding: "utf8" }).stdout.trim();
const TAG = /^[0-9]{14}-[0-9a-f]{12}$/u;

let bin = "";

const FAKES: Record<string, string> = {
  // `pnpm --dir apps/edge exec wrangler …`
  pnpm: `#!/bin/bash
echo "pnpm $*" >> "$FAKE/calls"
shift 4
case "$1 $2 $3" in
  "containers images list") cat "$FAKE/images.json" ;;
  "containers list --json") cat "$FAKE/apps.json" ;;
  "containers push "*) ;;
  "containers images delete") echo "Deleted $4" ;;
  *) echo "unexpected wrangler $*" >&2; exit 1 ;;
esac
`,
  docker: `#!/bin/bash
echo "docker $*" >> "$FAKE/calls"
if [ "$1" = load ]; then echo "Loaded image: jackioh-server:rehearsal"; fi
`,
  gh: `#!/bin/bash
echo "gh $*" >> "$FAKE/calls"
case "$1 $2" in
  "run list") echo "\${FAKE_RUN:-}" ;;
  "run download")
    shift 2; dir=""
    while [ $# -gt 0 ]; do case $1 in -D) dir=$2; shift 2 ;; *) shift ;; esac; done
    if [ -n "\${FAKE_ARTIFACT:-}" ]; then echo tar > "$dir/deploy-image.tar"; else exit 1; fi ;;
esac
`,
  // Registry credentials, and HEAD manifests/<tag> from $FAKE/digests (\`repository:tag digest\` lines).
  curl: `#!/bin/bash
echo "curl $*" >> "$FAKE/calls"
url=\${*: -1}
case $url in
  *credentials) echo '{"result":{"password":"secret"}}' ;;
  */manifests/*)
    repo=$(echo "$url" | sed -E 's#.*/v2/[^/]+/([^/]+)/manifests/.*#\\1#'); tag=\${url##*/}
    d=$(awk -v k="$repo:$tag" '$1 == k { print $2 }' "$FAKE/digests")
    [ -n "$d" ] || exit 22
    printf 'HTTP/2 200\\r\\ndocker-content-digest: %s\\r\\n\\r\\n' "$d" ;;
esac
`,
};

beforeAll(() => {
  bin = mkdtempSync(join(tmpdir(), "server-image-"));
  for (const [name, body] of Object.entries(FAKES)) {
    writeFileSync(join(bin, name), body);
    chmodSync(join(bin, name), 0o755);
  }
});

afterAll(() => rmSync(bin, { recursive: true, force: true }));

interface World {
  dir: string;
  config: string;
  images?: { name: string; tags: string[] }[];
  apps?: { image: string }[];
  digests?: Record<string, string>;
}

function world(): World {
  const dir = mkdtempSync(join(bin, "run-"));
  const config = join(dir, "wrangler.jsonc");
  copyFileSync(join(REPO, "wrangler.jsonc"), config);
  return { dir, config };
}

function run(w: World, args: string[], env: Record<string, string> = {}) {
  writeFileSync(join(w.dir, "calls"), "");
  writeFileSync(join(w.dir, "out"), "");
  writeFileSync(join(w.dir, "images.json"), JSON.stringify(w.images ?? []));
  writeFileSync(join(w.dir, "apps.json"), JSON.stringify(w.apps ?? []));
  writeFileSync(
    join(w.dir, "digests"),
    Object.entries(w.digests ?? {})
      .map(([tag, digest]) => `${tag} ${digest}\n`)
      .join(""),
  );
  const r = spawnSync("bash", [SCRIPT, ...args], {
    cwd: REPO,
    encoding: "utf8",
    env: {
      PATH: `${bin}:${process.env.PATH ?? ""}`,
      HOME: process.env.HOME ?? "",
      FAKE: w.dir,
      WRANGLER_CONFIG: w.config,
      GITHUB_OUTPUT: join(w.dir, "out"),
      CLOUDFLARE_ACCOUNT_ID: "acct",
      CLOUDFLARE_API_TOKEN: "token",
      ...env,
    },
  });
  const lines = (file: string) => readFileSync(join(w.dir, file), "utf8").split("\n").filter(Boolean);
  const out = Object.fromEntries(lines("out").map((l) => l.split("=") as [string, string]));
  return { status: r.status, text: r.stdout + r.stderr, calls: lines("calls"), out };
}

type Container = { name: string; image: string; image_build_context?: string };
function containers(w: World): Record<string, Container> {
  const c = JSON5.parse<{ containers: Container[]; env: { staging: { containers: Container[] } } }>(readFileSync(w.config, "utf8"));
  return { production: c.containers[0] as Container, staging: c.env.staging.containers[0] as Container };
}

/** The inputs' hash for the repository's HEAD, from a first run that pushes. */
function inputsHash(): string {
  const r = run(world(), ["use", "jackioh-server"]);
  expect(r.status, r.text).toBe(0);
  return (r.out.tag as string).slice(15);
}

describe("scripts/server-image.sh use", () => {
  it("loads the CI run's deploy-image when the registry lacks the inputs' image, pushes it and points the container at it", () => {
    const w = world();
    const r = run(w, ["use", "jackioh-staging-server"], { IMAGE_COMMIT: "abc", FAKE_RUN: "123", FAKE_ARTIFACT: "1" });
    expect(r.status, r.text).toBe(0);
    expect(r.out.tag).toMatch(TAG);
    expect(r.out.pushed).toBe("true");
    expect(r.calls).toContain("gh run list --workflow ci.yml --branch main --event push --status success --commit abc --limit 1 --json databaseId --jq .[0].databaseId // empty");
    expect(r.calls.some((c) => c.startsWith("docker load -i "))).toBe(true);
    expect(r.calls).toContain(`docker tag jackioh-server:rehearsal jackioh-server:${r.out.tag}`);
    expect(r.calls).toContain(`pnpm --dir apps/edge exec wrangler containers push jackioh-server:${r.out.tag}`);
    expect(r.calls.some((c) => /^docker (buildx )?build/u.test(c))).toBe(false);
    const { staging, production } = containers(w);
    expect(staging.image).toBe(`registry.cloudflare.com/acct/jackioh-server:${r.out.tag}`);
    expect(staging).not.toHaveProperty("image_build_context");
    // The other site's container, and the file's comments, are left as they were.
    expect(production.image).toBe("./crates/server/Dockerfile");
    expect(production.image_build_context).toBe(".");
    expect(readFileSync(w.config, "utf8")).toMatch(/^\s*\/\/ /mu);
  });

  it("builds and pushes nothing when the registry holds the inputs' image, whichever site pushed it", () => {
    const hash = inputsHash();
    const w = world();
    w.images = [{ name: "jackioh-server", tags: [`20260101000000-${hash}`, "a0816719", `20250101000000-${hash}`] }];
    const r = run(w, ["use", "jackioh-server"], { IMAGE_COMMIT: "abc", FAKE_RUN: "123", FAKE_ARTIFACT: "1" });
    expect(r.status, r.text).toBe(0);
    expect(r.out).toEqual({ pushed: "false", tag: `20260101000000-${hash}` });
    expect(r.calls.filter((c) => /^(docker|gh) |containers push/u.test(c))).toEqual([]);
    expect(containers(w).production.image).toBe(`registry.cloudflare.com/acct/jackioh-server:20260101000000-${hash}`);
  });

  it("builds on the layer cache when CI kept no image, and plainly without a cache", () => {
    const cached = run(world(), ["use", "jackioh-staging-server"], {
      IMAGE_COMMIT: "abc",
      FAKE_RUN: "123",
      IMAGE_CACHE_FROM: "type=gha,scope=a type=gha,scope=b",
    });
    expect(cached.status, cached.text).toBe(0);
    expect(cached.text).toContain("kept no deploy-image");
    expect(cached.calls).toContain(
      `docker buildx build --cache-from type=gha,scope=a --cache-from type=gha,scope=b --load -t jackioh-server:${cached.out.tag} -f crates/server/Dockerfile .`,
    );
    const plain = run(world(), ["use", "jackioh-server"]);
    expect(plain.status, plain.text).toBe(0);
    expect(plain.calls).toContain(`docker build -t jackioh-server:${plain.out.tag} -f crates/server/Dockerfile .`);
  });

  it("hashes what the Dockerfile copies, read from the Dockerfile", () => {
    const r = run(world(), ["use", "jackioh-server"]);
    expect(r.text).toContain("is built from Cargo.lock Cargo.toml crates crates/ai crates/cards crates/engine crates/server and crates/server/Dockerfile");
  });

  it("refuses inputs that differ from HEAD, even an ignored file a COPY would take", () => {
    const stray = join(REPO, "crates/server/.DS_Store");
    writeFileSync(stray, "x");
    try {
      const r = run(world(), ["use", "jackioh-server"]);
      expect(r.status).not.toBe(0);
      expect(r.text).toContain("differ from HEAD");
      expect(r.calls.filter((c) => c.includes("containers push"))).toEqual([]);
    } finally {
      rmSync(stray, { force: true });
    }
  });

  it("refuses a container wrangler.jsonc does not hold, or one already pointed at the registry", () => {
    expect(run(world(), ["use", "jackioh-nothing"]).status).not.toBe(0);
    const w = world();
    expect(run(w, ["use", "jackioh-server"]).status).toBe(0);
    const again = run(w, ["use", "jackioh-server"]);
    expect(again.status).not.toBe(0);
    expect(again.text).toContain("already points jackioh-server at registry.cloudflare.com/");
  });
});

describe("scripts/server-image.sh prune", () => {
  it("keeps the deployed tag, the newest others and anything running, and deletes the rest", () => {
    const w = world();
    const mine = Array.from({ length: 14 }, (_, i) => `202611030000${10 + i}-aaaaaaaaaa${10 + i}`);
    w.images = [
      { name: "jackioh-server", tags: [...mine, "a0816719"] },
      { name: "jackioh-staging-server", tags: ["05a67597", "082c1cff"] },
    ];
    w.digests = Object.fromEntries(mine.map((t, i) => [`jackioh-server:${t}`, `sha256:d${10 + i}`]));
    // 12 shares the deployed 15's digest; a0816719 and 082c1cff are what the two sites run.
    w.digests["jackioh-server:20261103000012-aaaaaaaaaa12"] = "sha256:d15";
    w.digests["jackioh-server:a0816719"] = "sha256:aaaa1";
    w.digests["jackioh-staging-server:05a67597"] = "sha256:cccc3";
    w.digests["jackioh-staging-server:082c1cff"] = "sha256:bbbb2";
    w.apps = [
      { image: "registry.cloudflare.com/acct/jackioh-server@sha256:aaaa1" },
      { image: "registry.cloudflare.com/acct/jackioh-staging-server@sha256:bbbb2" },
    ];
    const r = run(w, ["prune", "20261103000015-aaaaaaaaaa15"]);
    expect(r.status, r.text).toBe(0);
    const deleted = r.calls.flatMap((c) => /images delete (\S+) -y$/u.exec(c)?.[1] ?? []).sort();
    // The newest nine others (23 to 16, and 14) stay, beside the deployed 15.
    expect(deleted).toEqual([
      "jackioh-server:20261103000010-aaaaaaaaaa10",
      "jackioh-server:20261103000011-aaaaaaaaaa11",
      "jackioh-server:20261103000013-aaaaaaaaaa13",
      "jackioh-staging-server:05a67597",
    ]);
    expect(r.text).not.toContain("secret");
  });

  it("refuses a tag the registry does not hold", () => {
    const r = run(world(), ["prune", "20261103000015-aaaaaaaaaa15"]);
    expect(r.status).not.toBe(0);
    expect(r.text).toContain("the registry has no jackioh-server:20261103000015-aaaaaaaaaa15");
  });
});
