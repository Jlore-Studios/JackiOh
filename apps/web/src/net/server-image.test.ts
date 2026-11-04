// scripts/server-image.sh, the server image the two deploy jobs (deploy-staging.yml, promote-production.yml)
// reuse instead of building: its tag named by its inputs, the three ways it gets an image, the rewrite of
// wrangler.jsonc, and what `prune` deletes. Each case runs the script in a repository of its own (a small
// Dockerfile, its inputs and the real wrangler.jsonc), with stand-ins for `pnpm` (wrangler), `docker`, `gh`
// and `curl` on PATH that record each call and answer from files, so neither Docker nor Cloudflare is needed.

import { execFileSync, spawnSync } from "node:child_process";
import { createHash } from "node:crypto";
import { chmodSync, copyFileSync, mkdirSync, mkdtempSync, readFileSync, rmSync, symlinkSync, writeFileSync } from "node:fs";
import { tmpdir } from "node:os";
import { dirname, join } from "node:path";
import { fileURLToPath } from "node:url";

import JSON5 from "json5";
import { afterAll, beforeAll, describe, expect, it } from "vitest";

const HERE = dirname(fileURLToPath(import.meta.url));
const ROOT = join(HERE, "../../../..");
const SCRIPT = join(ROOT, "scripts/server-image.sh");
const TAG = /^[0-9]{14}-[0-9a-f]{12}$/u;

// A Dockerfile shaped as crates/server/Dockerfile: a `COPY` of several sources, one of a directory, and one
// `--from` a stage, which copies nothing from the context.
const DOCKERFILE = `FROM rust AS build
COPY Cargo.toml Cargo.lock ./
COPY crates ./crates
RUN cargo build --release

FROM debian
COPY --from=build /jackioh-server /app/jackioh-server
`;
const INPUTS = ["Cargo.lock", "Cargo.toml", "crates", "crates/server/Dockerfile"];

/** Git's background upkeep is off, as ci-scope.test.ts's (#1016). */
const GIT_CONFIG = ["-c", "user.name=t", "-c", "user.email=t@t", "-c", "commit.gpgsign=false", "-c", "maintenance.auto=false", "-c", "gc.auto=0"];

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
  /** The repository the script runs in, its HEAD the inputs it hashes. */
  repo: string;
  /** The stand-ins' answers and their record of calls. */
  fake: string;
  images?: { name: string; tags: string[] }[];
  apps?: { image: string }[];
  digests?: Record<string, string>;
}

function git(w: World, ...args: string[]): string {
  return execFileSync("git", [...GIT_CONFIG, ...args], { cwd: w.repo, encoding: "utf8" }).trim();
}

function commit(w: World, files: Record<string, string>): void {
  for (const [file, content] of Object.entries(files)) {
    mkdirSync(dirname(join(w.repo, file)), { recursive: true });
    writeFileSync(join(w.repo, file), content);
  }
  git(w, "add", "-A");
  git(w, "commit", "-qm", "change");
}

function world(): World {
  const dir = mkdtempSync(join(bin, "run-"));
  const w: World = { repo: join(dir, "repo"), fake: join(dir, "fake") };
  mkdirSync(w.fake);
  mkdirSync(w.repo);
  git(w, "init", "-q");
  // The script finds json5 from apps/web, as this test does.
  mkdirSync(join(w.repo, "apps/web"), { recursive: true });
  symlinkSync(join(ROOT, "apps/web/node_modules"), join(w.repo, "apps/web/node_modules"));
  copyFileSync(join(ROOT, "wrangler.jsonc"), join(w.repo, "wrangler.jsonc"));
  commit(w, {
    ".gitignore": "node_modules/\n.DS_Store\n",
    "Cargo.toml": "[workspace]\n",
    "Cargo.lock": "version = 4\n",
    "README.md": "fixture\n",
    "crates/server/Dockerfile": DOCKERFILE,
    "crates/server/src/main.rs": "fn main() {}\n",
    "crates/engine/src/lib.rs": "\n",
  });
  return w;
}

function run(w: World, args: string[], env: Record<string, string> = {}) {
  writeFileSync(join(w.fake, "calls"), "");
  writeFileSync(join(w.fake, "out"), "");
  writeFileSync(join(w.fake, "images.json"), JSON.stringify(w.images ?? []));
  writeFileSync(join(w.fake, "apps.json"), JSON.stringify(w.apps ?? []));
  writeFileSync(
    join(w.fake, "digests"),
    Object.entries(w.digests ?? {})
      .map(([tag, digest]) => `${tag} ${digest}\n`)
      .join(""),
  );
  const r = spawnSync("bash", [SCRIPT, ...args], {
    cwd: w.repo,
    encoding: "utf8",
    env: {
      PATH: `${bin}:${process.env.PATH ?? ""}`,
      HOME: process.env.HOME ?? "",
      FAKE: w.fake,
      GITHUB_OUTPUT: join(w.fake, "out"),
      CLOUDFLARE_ACCOUNT_ID: "acct",
      CLOUDFLARE_API_TOKEN: "token",
      ...env,
    },
  });
  const lines = (file: string) => readFileSync(join(w.fake, file), "utf8").split("\n").filter(Boolean);
  const out = Object.fromEntries(lines("out").map((l) => l.split("=") as [string, string]));
  return { status: r.status, text: r.stdout + r.stderr, calls: lines("calls"), out };
}

type Container = { name: string; image: string; image_build_context?: string };
function containers(w: World): { production: Container; staging: Container } {
  const c = JSON5.parse<{ containers: Container[]; env: { staging: { containers: Container[] } } }>(
    readFileSync(join(w.repo, "wrangler.jsonc"), "utf8"),
  );
  return { production: c.containers[0] as Container, staging: c.env.staging.containers[0] as Container };
}

/** The hash the tag ends in: each input's object at HEAD, one a line, in the order above. */
function inputsHash(w: World): string {
  const objects = INPUTS.map((p) => `${git(w, "rev-parse", `HEAD:${p}`)}\n`).join("");
  return createHash("sha256").update(objects).digest("hex").slice(0, 12);
}

/** The tag a first run that pushes gives HEAD's inputs. */
function pushedTag(w: World): string {
  const r = run(w, ["use", "jackioh-server"]);
  expect(r.status, r.text).toBe(0);
  expect(r.out.pushed).toBe("true");
  git(w, "checkout", "-q", "--", "wrangler.jsonc");
  return r.out.tag as string;
}

describe("scripts/server-image.sh use", () => {
  it("loads the CI run's deploy-image when the registry lacks the inputs' image, pushes it and points the container at it", () => {
    const w = world();
    const r = run(w, ["use", "jackioh-staging-server"], { IMAGE_COMMIT: "abc", FAKE_RUN: "123", FAKE_ARTIFACT: "1" });
    expect(r.status, r.text).toBe(0);
    expect(r.out.tag).toMatch(TAG);
    expect(r.out.tag?.slice(15)).toBe(inputsHash(w));
    expect(r.out.pushed).toBe("true");
    expect(r.calls).toContain(
      "gh run list --workflow ci.yml --branch main --event push --status success --commit abc --limit 1 --json databaseId --jq .[0].databaseId // empty",
    );
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
    expect(readFileSync(join(w.repo, "wrangler.jsonc"), "utf8")).toMatch(/^\s*\/\/ /mu);
  });

  it("builds and pushes nothing when the registry holds the inputs' image, whichever site pushed it", () => {
    const w = world();
    const hash = inputsHash(w);
    w.images = [{ name: "jackioh-server", tags: [`20260101000000-${hash}`, "a0816719", `20250101000000-${hash}`] }];
    const r = run(w, ["use", "jackioh-server"], { IMAGE_COMMIT: "abc", FAKE_RUN: "123", FAKE_ARTIFACT: "1" });
    expect(r.status, r.text).toBe(0);
    expect(r.out).toEqual({ pushed: "false", tag: `20260101000000-${hash}` });
    expect(r.calls.filter((c) => /^(docker|gh) |containers push/u.test(c))).toEqual([]);
    expect(containers(w).production.image).toBe(`registry.cloudflare.com/acct/jackioh-server:20260101000000-${hash}`);
  });

  it("names the image by what the Dockerfile copies, so only a change to that moves the tag", () => {
    const w = world();
    const first = pushedTag(w).slice(15);
    commit(w, { "README.md": "elsewhere\n", "apps/web/x.ts": "\n" });
    expect(pushedTag(w).slice(15)).toBe(first);
    for (const file of ["crates/engine/src/lib.rs", "Cargo.lock", "crates/server/Dockerfile"]) {
      const before = pushedTag(w).slice(15);
      commit(w, { [file]: `${readFileSync(join(w.repo, file), "utf8")}# ${file}\n` });
      expect(pushedTag(w).slice(15), file).not.toBe(before);
    }
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
    const plain = run(world(), ["use", "jackioh-server"], { IMAGE_COMMIT: "abc" });
    expect(plain.status, plain.text).toBe(0);
    expect(plain.text).toContain("no passing push-to-main CI run");
    expect(plain.calls).toContain(`docker build -t jackioh-server:${plain.out.tag} -f crates/server/Dockerfile .`);
  });

  it("refuses inputs that differ from HEAD, even an ignored file a COPY would take", () => {
    for (const [file, content] of [
      ["crates/server/.DS_Store", "x"],
      ["Cargo.lock", "changed\n"],
    ] as const) {
      const w = world();
      writeFileSync(join(w.repo, file), content);
      const r = run(w, ["use", "jackioh-server"]);
      expect(r.status, file).not.toBe(0);
      expect(r.text).toContain("differ from HEAD");
      expect(r.calls.filter((c) => /containers push|^docker /u.test(c))).toEqual([]);
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
    // 12 shares the deployed 15's digest. One site runs a0816719 by its digest, as wrangler's own push
    // left it; the other runs 10 by its tag, as `use` leaves it, older than every tag kept.
    w.digests["jackioh-server:20261103000012-aaaaaaaaaa12"] = "sha256:d15";
    w.digests["jackioh-server:a0816719"] = "sha256:aaaa1";
    w.digests["jackioh-staging-server:05a67597"] = "sha256:cccc3";
    w.digests["jackioh-staging-server:082c1cff"] = "sha256:bbbb2";
    w.apps = [
      { image: "registry.cloudflare.com/acct/jackioh-server@sha256:aaaa1" },
      { image: "registry.cloudflare.com/acct/jackioh-server:20261103000010-aaaaaaaaaa10" },
    ];
    const r = run(w, ["prune", "20261103000015-aaaaaaaaaa15"]);
    expect(r.status, r.text).toBe(0);
    const deleted = r.calls.flatMap((c) => /images delete (\S+) -y$/u.exec(c)?.[1] ?? []).sort();
    // The newest nine others (23 to 16, and 14) stay, beside the deployed 15.
    expect(deleted).toEqual([
      "jackioh-server:20261103000011-aaaaaaaaaa11",
      "jackioh-server:20261103000013-aaaaaaaaaa13",
      "jackioh-staging-server:05a67597",
      "jackioh-staging-server:082c1cff",
    ]);
    expect(r.text).not.toContain("secret");
  });

  it("deletes nothing when a kept tag's digest cannot be read, since an older tag may share it", () => {
    const w = world();
    w.images = [{ name: "jackioh-server", tags: ["20261103000015-aaaaaaaaaa15", "20261103000014-aaaaaaaaaa14", "a0816719"] }];
    w.digests = { "jackioh-server:20261103000015-aaaaaaaaaa15": "sha256:d15", "jackioh-server:a0816719": "sha256:d14" };
    w.apps = [{ image: "registry.cloudflare.com/acct/jackioh-server@sha256:d15" }];
    const r = run(w, ["prune", "20261103000015-aaaaaaaaaa15"]);
    expect(r.status).not.toBe(0);
    expect(r.text).toContain("no digest for jackioh-server:20261103000014-aaaaaaaaaa14, a tag kept: nothing deleted");
    expect(r.calls.filter((c) => c.includes("images delete"))).toEqual([]);
  });

  it("deletes nothing when what a container application runs cannot be told", () => {
    for (const apps of [[], [{ image: "registry.cloudflare.com/acct/jackioh-server:gone" }], [{ image: "docker.io/library/x:1" }]]) {
      const w = world();
      w.images = [
        { name: "jackioh-server", tags: ["20261103000015-aaaaaaaaaa15"] },
        { name: "jackioh-staging-server", tags: ["05a67597"] },
      ];
      w.digests = { "jackioh-server:20261103000015-aaaaaaaaaa15": "sha256:d15", "jackioh-staging-server:05a67597": "sha256:cccc3" };
      w.apps = apps;
      const r = run(w, ["prune", "20261103000015-aaaaaaaaaa15"]);
      expect(r.status, JSON.stringify(apps)).not.toBe(0);
      expect(r.text).toContain("nothing deleted");
      expect(r.calls.filter((c) => c.includes("images delete"))).toEqual([]);
    }
  });

  it("refuses a tag the registry does not hold, and deletes nothing", () => {
    const w = world();
    w.images = [{ name: "jackioh-staging-server", tags: ["05a67597"] }];
    w.digests = { "jackioh-staging-server:05a67597": "sha256:cccc3" };
    const r = run(w, ["prune", "20261103000015-aaaaaaaaaa15"]);
    expect(r.status).not.toBe(0);
    expect(r.text).toContain("the registry has no jackioh-server:20261103000015-aaaaaaaaaa15");
    expect(r.calls.filter((c) => c.includes("images delete"))).toEqual([]);
  });
});
