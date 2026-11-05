// scripts/build-wasm.sh's directories: where it builds (CARGO_TARGET_DIR) and where it keeps
// wasm-bindgen and the emsdk (JACKIOH_CACHE_DIR), which caches restore, sometimes from another pin.
// Each case runs the real script in a throwaway tree with stand-ins for cargo, rustup, wasm-bindgen
// and the emsdk, which write a module that names the run that built it.

import { spawnSync } from "node:child_process";
import { chmodSync, copyFileSync, existsSync, mkdirSync, mkdtempSync, readdirSync, readFileSync, renameSync, rmSync, writeFileSync } from "node:fs";
import { tmpdir } from "node:os";
import { dirname, join, relative } from "node:path";
import { fileURLToPath } from "node:url";

import { afterEach, describe, expect, it } from "vitest";

const HERE = dirname(fileURLToPath(import.meta.url));
const ROOT = join(HERE, "../../../..");
const SCRIPT = readFileSync(join(ROOT, "scripts/build-wasm.sh"), "utf8");
const BINDGEN = /^WASM_BINDGEN_VERSION="([^"]+)"$/mu.exec(SCRIPT)?.[1] ?? "";

const sh = (file: string, body: string): void => {
  mkdirSync(dirname(file), { recursive: true });
  writeFileSync(file, `#!/bin/sh\n${body}\n`);
  chmodSync(file, 0o755);
};

const roots: string[] = [];
afterEach(() => {
  for (const root of roots.splice(0)) rmSync(root, { recursive: true, force: true });
});

/** A checkout with the script and the stand-ins; returns its root. */
function world(): string {
  const root = mkdtempSync(join(tmpdir(), "build-wasm-cache-"));
  roots.push(root);
  mkdirSync(join(root, "scripts"));
  copyFileSync(join(ROOT, "scripts/build-wasm.sh"), join(root, "scripts/build-wasm.sh"));
  // cargo logs each build and leaves its output in target/ (CARGO_TARGET_DIR when set), naming the run.
  sh(
    join(root, "bin/cargo"),
    [
      'echo "$*" >> "$CARGO_LOG"',
      'target=""; prev=""; for a in "$@"; do [ "$prev" = --target ] && target=$a; prev=$a; done',
      'dir="${CARGO_TARGET_DIR:-target}/$target/release"; mkdir -p "$dir"',
      'case "$target" in',
      '  wasm32-unknown-unknown) echo "normal $RUN" > "$dir/jackioh_wasm.wasm"; case " $* " in *" jackioh-wasm-c "*) echo "c $RUN" > "$dir/jackioh_wasm_c.wasm" ;; esac ;;',
      '  wasm32-unknown-emscripten) echo "luau $RUN" > "$dir/jackioh-wasm-luau.js"; echo "luau $RUN" > "$dir/jackioh_wasm_luau.wasm" ;;',
      "esac",
    ].join("\n"),
  );
  sh(join(root, "bin/rustup"), "printf '%s\\n' wasm32-unknown-unknown wasm32-unknown-emscripten");
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

/** Runs the script as apps/web's `prebuild` does, with `env` added. */
function run(root: string, name: string, env: Record<string, string> = {}): Run {
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
      ...env,
    },
  });
  expect(result.status, `${result.stdout}${result.stderr}`).toBe(0);
  return { builds: readFileSync(log, "utf8").split("\n").filter(Boolean), out: result.stdout };
}

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

describe("scripts/build-wasm.sh's directories", () => {
  it("builds into CARGO_TARGET_DIR and keeps wasm-bindgen and the emsdk in JACKIOH_CACHE_DIR, as the night bot's model job sets them (#1263)", () => {
    const root = world();
    const shared = mkdtempSync(join(tmpdir(), "build-wasm-shared-"));
    roots.push(shared);
    // The tools live only in the shared cache: a script that looked in the root's .cache would fetch
    // them there, which the last line catches.
    renameSync(join(root, ".cache"), join(shared, "cache"));
    run(root, "shared", { CARGO_TARGET_DIR: join(shared, "target"), JACKIOH_CACHE_DIR: join(shared, "cache") });
    const built = files(pkg(root));
    expect(built["jackioh_wasm_bg.wasm"]).toBe("normal shared\n");
    expect(built["luau/jackioh_wasm_luau.wasm"]).toBe("luau shared\n");
    expect(existsSync(join(root, "target"))).toBe(false);
    expect(existsSync(join(root, ".cache"))).toBe(false);
  });

  it("builds the C module only when asked, and checks its imports with node (#3093)", () => {
    const root = world();
    // A stand-in node that finds no import; the real check is the module's own, read by node.
    sh(join(root, "bin/node"), "exit 0");
    const cBuilds = (result: Run): string[] => result.builds.filter((line) => line.includes("jackioh-wasm-c"));
    expect(cBuilds(run(root, "default", { JACKIOH_WASM_LUAU: "0" }))).toEqual([]);
    const asked = run(root, "asked", { JACKIOH_WASM_LUAU: "0", JACKIOH_WASM_C: "1" });
    expect(cBuilds(asked)).toEqual(["build -p jackioh-wasm-c --release --target wasm32-unknown-unknown"]);
    expect(asked.out).toContain("imports nothing");
  });

  it("builds the C module alone, and fails the build when node finds an import in it (#3093)", () => {
    const root = world();
    // A node that reports one, as the real check's `console.error` and exit 1 do.
    sh(join(root, "bin/node"), 'echo "build-wasm: the C module imports [env]" >&2; exit 1');
    const result = spawnSync("sh", [join(root, "scripts/build-wasm.sh")], {
      encoding: "utf8",
      env: {
        PATH: `${join(root, "bin")}:${process.env.PATH ?? ""}`,
        HOME: root,
        CARGO_LOG: join(root, "cargo-imports.log"),
        RUN: "imports",
        JACKIOH_WASM_NORMAL: "0",
        JACKIOH_WASM_LUAU: "0",
        JACKIOH_WASM_C: "1",
      },
    });
    expect(result.status, result.stdout).toBe(1);
    expect(result.stderr).toContain("the C module imports");
    expect(existsSync(join(pkg(root), "jackioh_wasm_bg.wasm"))).toBe(false);
  });

  it("refuses a JACKIOH_WASM_C that is neither 0 nor 1", () => {
    const root = world();
    const result = spawnSync("sh", [join(root, "scripts/build-wasm.sh")], {
      encoding: "utf8",
      env: { PATH: `${join(root, "bin")}:${process.env.PATH ?? ""}`, HOME: root, CARGO_LOG: join(root, "cargo.log"), JACKIOH_WASM_LUAU: "0", JACKIOH_WASM_C: "2" },
    });
    expect(result.status).toBe(1);
    expect(result.stderr).toContain("JACKIOH_WASM_C is 0 (the default) or 1");
  });

  it("clones the emsdk again when the one it finds cannot activate the pin", () => {
    const root = world();
    // Another pin's emsdk, as a shared or restored one can be: it refuses to activate this one.
    sh(join(root, ".cache/emsdk/emsdk"), '[ "$1" = activate ] && exit 1; exit 0');
    // git clones an emsdk that activates, and marks it.
    sh(
      join(root, "bin/git"),
      [
        'for a in "$@"; do dir=$a; done',
        'mkdir -p "$dir/upstream/emscripten" "$dir/node/22.16.0_64bit/bin"',
        'printf "#!/bin/sh\\nexit 0\\n" > "$dir/emsdk"; chmod +x "$dir/emsdk"',
        'printf "#!/bin/sh\\nexit 0\\n" > "$dir/node/22.16.0_64bit/bin/node"; chmod +x "$dir/node/22.16.0_64bit/bin/node"',
        'touch "$dir/cloned"',
      ].join("\n"),
    );
    const repinned = run(root, "repinned");
    expect(repinned.out).toContain("cannot activate emsdk");
    expect(existsSync(join(root, ".cache/emsdk/cloned"))).toBe(true);
    expect(files(pkg(root))["luau/jackioh_wasm_luau.wasm"]).toBe("luau repinned\n");
  });
});
