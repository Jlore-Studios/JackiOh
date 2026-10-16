// scripts/build-wasm.sh's cache on Vercel (#997): Vercel's build cache keeps only node_modules, so
// the script keeps the modules it built in node_modules/.cache/jackioh-wasm/<key>, the key a hash of
// what they are built from, and a deploy with the same key copies them back instead of building.
// A stale module on staging is the failure that matters, so every input the build reads must change
// the key. Each case runs the real script in a throwaway tree with stand-ins for cargo, rustup,
// wasm-bindgen and the emsdk, which write a module that names the run that built it.

import { spawnSync } from "node:child_process";
import { appendFileSync, chmodSync, copyFileSync, existsSync, mkdirSync, mkdtempSync, readdirSync, readFileSync, rmSync, writeFileSync } from "node:fs";
import { tmpdir } from "node:os";
import { dirname, join, relative } from "node:path";
import { fileURLToPath } from "node:url";

import { afterEach, describe, expect, it } from "vitest";

const HERE = dirname(fileURLToPath(import.meta.url));
const ROOT = join(HERE, "../../../..");
const SCRIPT = readFileSync(join(ROOT, "scripts/build-wasm.sh"), "utf8");
const BINDGEN = /^WASM_BINDGEN_VERSION="([^"]+)"$/mu.exec(SCRIPT)?.[1] ?? "";

/** A file of each kind the build reads, and some it does not. */
const TREE = [
  "Cargo.toml",
  "Cargo.lock",
  "rust-toolchain.toml",
  ".cargo/config.toml",
  "crates/engine/Cargo.toml",
  "crates/engine/src/reduce.rs",
  "crates/cards/Cargo.toml",
  "crates/cards/catalog.json",
  "crates/cards/src/scripts/core/c001_big_d_fender.rs",
  "crates/ai/Cargo.toml",
  "crates/ai/src/decide.rs",
  "crates/luau/Cargo.toml",
  "crates/luau/src/lib.rs",
  "crates/wasm/Cargo.toml",
  "crates/wasm/src/lib.rs",
  "crates/wasm/luau/Cargo.toml",
  "crates/wasm/luau/src/main.rs",
  "crates/server/Cargo.toml",
  "crates/server/src/main.rs",
  "crates/tools/Cargo.toml",
  "crates/tools/src/fuzz.rs",
  "apps/web/src/main.tsx",
  "docs/architecture.md",
];

const sh = (file: string, body: string): void => {
  mkdirSync(dirname(file), { recursive: true });
  writeFileSync(file, `#!/bin/sh\n${body}\n`);
  chmodSync(file, 0o755);
};

const roots: string[] = [];
afterEach(() => {
  for (const root of roots.splice(0)) rmSync(root, { recursive: true, force: true });
});

/** A checkout with the script, the inputs above, and the stand-ins; returns its root. */
function world(): string {
  const root = mkdtempSync(join(tmpdir(), "build-wasm-cache-"));
  roots.push(root);
  mkdirSync(join(root, "scripts"));
  copyFileSync(join(ROOT, "scripts/build-wasm.sh"), join(root, "scripts/build-wasm.sh"));
  for (const file of TREE) {
    mkdirSync(dirname(join(root, file)), { recursive: true });
    writeFileSync(join(root, file), `${file}\n`);
  }
  // cargo logs each build and leaves what that target's build leaves in target/, naming the run.
  sh(
    join(root, "bin/cargo"),
    [
      'echo "$*" >> "$CARGO_LOG"',
      'target=""; prev=""; for a in "$@"; do [ "$prev" = --target ] && target=$a; prev=$a; done',
      'dir="target/$target/release"; mkdir -p "$dir"',
      'case "$target" in',
      '  wasm32-unknown-unknown) echo "normal $RUN" > "$dir/jackioh_wasm.wasm" ;;',
      '  wasm32-unknown-emscripten) echo "luau $RUN" > "$dir/jackioh-wasm-luau.js"; echo "luau $RUN" > "$dir/jackioh_wasm_luau.wasm" ;;',
      "esac",
    ].join("\n"),
  );
  sh(join(root, "bin/rustup"), "printf '%s\\n' wasm32-unknown-unknown wasm32-unknown-emscripten");
  // wasm-bindgen: the pinned version, and the four files it writes from the module it is given.
  sh(
    join(root, ".cache/bin/wasm-bindgen"),
    [
      `[ "$1" = --version ] && { echo "wasm-bindgen ${BINDGEN}"; exit 0; }`,
      'out=""; prev=""; for a in "$@"; do [ "$prev" = --out-dir ] && out=$a; prev=$a; module=$a; done',
      'for f in jackioh_wasm.js jackioh_wasm.d.ts jackioh_wasm_bg.wasm jackioh_wasm_bg.wasm.d.ts; do cp "$module" "$out/$f"; done',
    ].join("\n"),
  );
  sh(join(root, ".cache/emsdk/emsdk"), "exit 0");
  mkdirSync(join(root, ".cache/emsdk/upstream/emscripten"), { recursive: true });
  sh(join(root, ".cache/emsdk/node/22.16.0_64bit/bin/node"), "exit 0");
  return root;
}

interface Run {
  /** The cargo builds the run started (`build -p <crate> …`). */
  builds: string[];
  out: string;
}

/** Runs the script as apps/web's `prebuild` does, on Vercel unless `vercel` is false. */
function run(root: string, name: string, { vercel = true, env = {} as Record<string, string> } = {}): Run {
  const log = join(root, `cargo-${name}.log`);
  writeFileSync(log, "");
  const result = spawnSync("sh", [join(root, "scripts/build-wasm.sh")], {
    encoding: "utf8",
    env: {
      PATH: `${join(root, "bin")}:${process.env.PATH ?? ""}`,
      HOME: root,
      CARGO_LOG: log,
      RUN: name,
      JACKIOH_WASM_PREVIEW: "0",
      ...(vercel ? { VERCEL: "1" } : {}),
      ...env,
    },
  });
  expect(result.status, `${result.stdout}${result.stderr}`).toBe(0);
  return { builds: readFileSync(log, "utf8").split("\n").filter(Boolean), out: result.stdout };
}

/** Every file under `dir` with its contents, by path relative to it. */
function files(dir: string): Record<string, string> {
  const found: Record<string, string> = {};
  const walk = (at: string): void => {
    for (const entry of readdirSync(at, { withFileTypes: true })) {
      const path = join(at, entry.name);
      if (entry.isDirectory()) walk(path);
      else found[relative(dir, path)] = readFileSync(path, "utf8");
    }
  };
  if (existsSync(dir)) walk(dir);
  return found;
}

const pkg = (root: string): string => join(root, "apps/web/src/wasm/pkg");
const cache = (root: string): string => join(root, "node_modules/.cache/jackioh-wasm");
/** A deploy's fresh checkout: git ignores pkg/, so it starts without one. */
const fresh = (root: string): void => rmSync(pkg(root), { recursive: true, force: true });

describe("scripts/build-wasm.sh on Vercel", () => {
  it("keeps the modules it built, and a deploy with the same inputs gets them back and builds nothing", () => {
    const root = world();
    const first = run(root, "first");
    expect(first.builds.map((b) => b.split(" ").slice(0, 3).join(" "))).toEqual(["build -p jackioh-wasm", "build -p jackioh-wasm-luau"]);
    const built = files(pkg(root));
    expect(Object.keys(built).sort()).toEqual([
      "jackioh_wasm.d.ts",
      "jackioh_wasm.js",
      "jackioh_wasm_bg.wasm",
      "jackioh_wasm_bg.wasm.d.ts",
      "luau/jackioh_wasm_luau.d.ts",
      "luau/jackioh_wasm_luau.js",
      "luau/jackioh_wasm_luau.wasm",
    ]);
    expect(built["jackioh_wasm_bg.wasm"]).toBe("normal first\n");
    const kept = readdirSync(cache(root));
    expect(kept).toHaveLength(1);
    expect(files(join(cache(root), kept[0] ?? ""))).toEqual(built);

    fresh(root);
    const second = run(root, "second");
    expect(second.builds).toEqual([]);
    expect(second.out).toContain("restored the modules from Vercel's build cache");
    expect(files(pkg(root))).toEqual(built);
  });

  it("builds again when anything the modules are built from changed", () => {
    const root = world();
    run(root, "base");
    for (const file of [
      "crates/engine/src/reduce.rs",
      "crates/cards/catalog.json",
      "crates/cards/src/scripts/core/c001_big_d_fender.rs",
      "crates/ai/src/decide.rs",
      "crates/luau/src/lib.rs",
      "crates/wasm/src/lib.rs",
      "crates/wasm/luau/src/main.rs",
      // What cargo reads to build any member: the workspace, the lockfile, every manifest, the config.
      "Cargo.toml",
      "Cargo.lock",
      "crates/server/Cargo.toml",
      "crates/tools/Cargo.toml",
      ".cargo/config.toml",
      "rust-toolchain.toml",
      // The script itself pins wasm-bindgen and the emsdk.
      "scripts/build-wasm.sh",
      // A new file is a change too.
      "crates/engine/src/new.rs",
    ]) {
      mkdirSync(dirname(join(root, file)), { recursive: true });
      appendFileSync(join(root, file), "# changed\n");
      fresh(root);
      expect(run(root, file.replaceAll("/", "_")).builds, file).toHaveLength(2);
      // And that build is the one the next deploy of the same inputs gets back.
      fresh(root);
      expect(run(root, "again").builds, file).toEqual([]);
      expect(files(pkg(root))["jackioh_wasm_bg.wasm"], file).toBe(`normal ${file.replaceAll("/", "_")}\n`);
    }
  });

  it("gets the modules back when only files they are not built from changed", () => {
    const root = world();
    run(root, "base");
    for (const file of ["crates/server/src/main.rs", "crates/tools/src/fuzz.rs", "apps/web/src/main.tsx", "docs/architecture.md"]) {
      appendFileSync(join(root, file), "changed\n");
      fresh(root);
      expect(run(root, "after").builds, file).toEqual([]);
      expect(files(pkg(root))["jackioh_wasm_bg.wasm"], file).toBe("normal base\n");
    }
  });

  it("builds again for other flags, and keeps only the newest modules", () => {
    const root = world();
    run(root, "production");
    for (const env of [{ JACKIOH_WASM_PREVIEW: "1" }, { JACKIOH_WASM_DUOS: "1" }, { JACKIOH_WASM_LUAU: "0" }] as Record<string, string>[]) {
      fresh(root);
      expect(run(root, "flagged", { env }).builds.length, JSON.stringify(env)).toBeGreaterThan(0);
      expect(readdirSync(cache(root)), JSON.stringify(env)).toHaveLength(1);
    }
    fresh(root);
    expect(run(root, "production again").builds).toHaveLength(2);
  });

  it("keeps nothing anywhere but Vercel", () => {
    const root = world();
    run(root, "first", { vercel: false });
    expect(existsSync(cache(root))).toBe(false);
    expect(run(root, "second", { vercel: false }).builds).toHaveLength(2);
  });

  it("hashes every crate the modules compile: crates/wasm and each path dependency of the workspace", () => {
    const listed = (/^MODULE_CRATES="([^"]+)"$/mu.exec(SCRIPT)?.[1] ?? "").split(" ").sort();
    const manifest = readFileSync(join(ROOT, "Cargo.toml"), "utf8");
    const paths = [...manifest.matchAll(/path = "(crates\/[^"]+)"/gu)].map((m) => m[1] ?? "");
    expect(paths.length).toBeGreaterThan(0);
    expect(listed).toEqual(["crates/wasm", ...paths].sort());
  });
});
