// scripts/build-wasm.sh's directories: where it builds (CARGO_TARGET_DIR) and where it keeps
// wasm-bindgen (JACKIOH_CACHE_DIR), which caches restore, sometimes from another pin. Each case runs
// the real script in a throwaway tree with stand-ins for cargo, rustup and wasm-bindgen, which write a
// module that names the run that built it.

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
  mkdirSync(join(root, "crates/wasm/c/src"), { recursive: true });
  copyFileSync(join(ROOT, "crates/wasm/c/src/lib.rs"), join(root, "crates/wasm/c/src/lib.rs"));
  // cargo logs each build and leaves its output in target/ (CARGO_TARGET_DIR when set), naming the run.
  // The linker self-test's fixture build (#4732, the one with `--manifest-path`) is logged with the C
  // module's profile env to "$CARGO_LOG.profile", as the module's own build is, and answers at once:
  // the failure a real link gives an export named `log` (a clean build when SELFTEST_CLEAN is set).
  sh(
    join(root, "bin/cargo"),
    [
      'echo "${CARGO_PROFILE_RELEASE_OPT_LEVEL:-} ${CARGO_PROFILE_RELEASE_CODEGEN_UNITS:-} ${CARGO_PROFILE_RELEASE_STRIP:-} $*" >> "$CARGO_LOG.profile"',
      'case " $* " in',
      '  *" --manifest-path "*)',
      '    [ -n "${SELFTEST_CLEAN:-}" ] && exit 0',
      '    echo "error: linker stderr: rust-lld: function signature mismatch: log" >&2; exit 101 ;;',
      "esac",
      'echo "$*" >> "$CARGO_LOG"',
      'target=""; prev=""; for a in "$@"; do [ "$prev" = --target ] && target=$a; prev=$a; done',
      'dir="${CARGO_TARGET_DIR:-target}/$target/release"; mkdir -p "$dir"',
      'case "$target" in',
      '  wasm32-unknown-unknown)',
      '    case " $* " in',
      '      *" jackioh-wasm-c "*) echo "c $RUN" > "$dir/jackioh_wasm_c.wasm" ;;',
      '      *) echo "normal $RUN" > "$dir/jackioh_wasm.wasm" ;;',
      "    esac ;;",
      "esac",
    ].join("\n"),
  );
  sh(join(root, "bin/rustup"), "printf '%s\\n' wasm32-unknown-unknown");
  sh(
    join(root, ".cache/bin/wasm-bindgen"),
    [
      `[ "$1" = --version ] && { echo "wasm-bindgen ${BINDGEN}"; exit 0; }`,
      'out=""; prev=""; for a in "$@"; do [ "$prev" = --out-dir ] && out=$a; prev=$a; module=$a; done',
      'for f in jackioh_wasm.js jackioh_wasm.d.ts jackioh_wasm_bg.wasm jackioh_wasm_bg.wasm.d.ts; do cp "$module" "$out/$f"; done',
    ].join("\n"),
  );
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
  it("builds into CARGO_TARGET_DIR and keeps wasm-bindgen in JACKIOH_CACHE_DIR, as the night bot's model job sets them (#1263)", () => {
    const root = world();
    const shared = mkdtempSync(join(tmpdir(), "build-wasm-shared-"));
    roots.push(shared);
    // The tools live only in the shared cache: a script that looked in the root's .cache would fetch
    // them there, which the last line catches.
    renameSync(join(root, ".cache"), join(shared, "cache"));
    run(root, "shared", { CARGO_TARGET_DIR: join(shared, "target"), JACKIOH_CACHE_DIR: join(shared, "cache") });
    const built = files(pkg(root));
    expect(built["jackioh_wasm_bg.wasm"]).toBe("normal shared\n");
    expect(existsSync(join(root, "target"))).toBe(false);
    expect(existsSync(join(root, ".cache"))).toBe(false);
  });

  it("builds the C module only when JACKIOH_WASM_C asks for it, then checks its imports with node (#3093)", () => {
    const root = world();
    // The check runs node on the module; the stand-in finds none.
    sh(join(root, "bin/node"), "exit 0");
    const skipped = run(root, "default");
    expect(skipped.builds.filter((line) => line.includes("jackioh-wasm-c"))).toEqual([]);
    const asked = run(root, "asked", { JACKIOH_WASM_C: "1" });
    expect(asked.builds.filter((line) => line.includes("jackioh-wasm-c"))).toEqual([
      "build -p jackioh-wasm-c --release --target wasm32-unknown-unknown",
    ]);
    expect(asked.out).toContain("imports nothing");
    // The normal module is the one the web loads: the C build leaves it as the run's own.
    expect(files(pkg(root))["jackioh_wasm_bg.wasm"]).toBe("normal asked\n");
  });

  it("fails the build when the C module imports something from its host (#3093)", () => {
    const root = world();
    // node answers the module's imports; this one lists one, as a wasm-bindgen glue's would.
    sh(join(root, "bin/node"), 'echo "build-wasm: the C module imports [{}]" >&2; exit 1');
    const result = spawnSync("sh", [join(root, "scripts/build-wasm.sh")], {
      encoding: "utf8",
      env: {
        PATH: `${join(root, "bin")}:${process.env.PATH ?? ""}`,
        HOME: root,
        CARGO_LOG: join(root, "cargo-imports.log"),
        RUN: "imports",
        JACKIOH_WASM_PREVIEW: "0",
        JACKIOH_WASM_NORMAL: "0",
        JACKIOH_WASM_C: "1",
      },
    });
    expect(result.status).toBe(1);
    expect(result.stderr).toContain("the C module imports");
  });

  it("builds the C module alone with JACKIOH_WASM_NORMAL=0, as CI's C job does, and refuses a JACKIOH_WASM_C that is neither 0 nor 1 or builds nothing for both off (#3093)", () => {
    const root = world();
    const env = { PATH: `${join(root, "bin")}:${process.env.PATH ?? ""}`, HOME: root, CARGO_LOG: join(root, "cargo.log") };
    const script = join(root, "scripts/build-wasm.sh");
    const odd = spawnSync("sh", [script], { encoding: "utf8", env: { ...env, JACKIOH_WASM_C: "2" } });
    expect(odd.status).toBe(1);
    expect(odd.stderr).toContain("JACKIOH_WASM_C is 0 (the default) or 1");
    const nothing = spawnSync("sh", [script], { encoding: "utf8", env: { ...env, JACKIOH_WASM_NORMAL: "0" } });
    expect(nothing.status).toBe(1);
    expect(nothing.stderr).toContain("builds nothing");
    // The C module alone: no wasm-bindgen module is built or written, and the C build is the only one.
    sh(join(root, "bin/node"), "exit 0");
    const alone = run(root, "alone", { JACKIOH_WASM_NORMAL: "0", JACKIOH_WASM_C: "1" });
    expect(alone.builds).toEqual(["build -p jackioh-wasm-c --release --target wasm32-unknown-unknown"]);
    expect(files(pkg(root))).toEqual({});
  });

  it("plays the hotseat fixture on the built C module using reference-host.mjs (#4332)", () => {
    const root = world();
    const nodeLog = join(root, "node.log");
    writeFileSync(nodeLog, "");
    sh(
      join(root, "bin/node"),
      `echo "$*" >> "${nodeLog}"
exit 0`,
    );
    run(root, "fixture", { JACKIOH_WASM_C: "1", JACKIOH_WASM_NORMAL: "0" });
    const calls = readFileSync(nodeLog, "utf8").split("\n").filter(Boolean);
    const fixtureCall = calls.find((line) => line.includes("reference-host.mjs"));
    expect(fixtureCall).toBe(
      `${join(root, "luau/tools/reference-host.mjs")} fixture ${join(root, "target/wasm32-unknown-unknown/release/jackioh_wasm_c.wasm")}`,
    );
  });

  it("fails the build and names the reference host when the fixture run fails (#4332)", () => {
    const root = world();
    sh(
      join(root, "bin/node"),
      `case "$*" in
    *"reference-host.mjs fixture"*)
      echo "reference-host.mjs: hash mismatch" >&2
      exit 1
      ;;
    *) exit 0 ;;
  esac`,
    );
    const result = spawnSync("sh", [join(root, "scripts/build-wasm.sh")], {
      encoding: "utf8",
      env: {
        PATH: `${join(root, "bin")}:${process.env.PATH ?? ""}`,
        HOME: root,
        CARGO_LOG: join(root, "cargo-fixture-fail.log"),
        RUN: "fixture-fail",
        JACKIOH_WASM_PREVIEW: "0",
        JACKIOH_WASM_NORMAL: "0",
        JACKIOH_WASM_C: "1",
      },
    });
    expect(result.status).toBe(1);
    expect(result.stderr).toContain("luau/tools/reference-host.mjs fixture failed");
    expect(result.stderr).toContain(
      `reproduce with: node luau/tools/reference-host.mjs fixture ${join(root, "target/wasm32-unknown-unknown/release/jackioh_wasm_c.wasm")}`,
    );
  });

  it("builds the linker self-test's fixture under the C module's own profile and removes it, and no signature_mismatch grep remains (#4732)", () => {
    const root = world();
    sh(join(root, "bin/node"), "exit 0");
    run(root, "guard", { JACKIOH_WASM_C: "1", JACKIOH_WASM_NORMAL: "0" });
    const profiled = readFileSync(join(root, "cargo-guard.log.profile"), "utf8").split("\n").filter(Boolean);
    expect(profiled).toHaveLength(2);
    expect(profiled[0]).toMatch(/^z 1 symbols build -p jackioh-wasm-c --release --target wasm32-unknown-unknown/u);
    const fixture = /^z 1 symbols build --release --target wasm32-unknown-unknown --manifest-path (.+\/Cargo\.toml)$/u.exec(profiled[1] ?? "");
    expect(fixture, profiled[1]).not.toBeNull();
    // The fixture's directory is gone once the self-test has judged it.
    expect(existsSync(dirname(fixture?.[1] ?? root))).toBe(false);
    // The name the strip removed from the module is not what guards it any more.
    expect(SCRIPT).not.toContain("signature_mismatch");
  });

  it("fails the C build when the linker self-test's fixture links clean, as it does once the lint is silenced (#4732)", () => {
    const root = world();
    sh(join(root, "bin/node"), "exit 0");
    const result = spawnSync("sh", [join(root, "scripts/build-wasm.sh")], {
      encoding: "utf8",
      env: {
        PATH: `${join(root, "bin")}:${process.env.PATH ?? ""}`,
        HOME: root,
        CARGO_LOG: join(root, "cargo-clean.log"),
        RUN: "clean",
        SELFTEST_CLEAN: "1",
        JACKIOH_WASM_PREVIEW: "0",
        JACKIOH_WASM_NORMAL: "0",
        JACKIOH_WASM_C: "1",
      },
    });
    expect(result.status).toBe(1);
    expect(result.stderr).toContain("linker self-test failed");
  });

  it("fails the C build when lib.rs has no deny(linker_messages) line, before it builds the fixture (#4732)", () => {
    const root = world();
    sh(join(root, "bin/node"), "exit 0");
    const lib = join(root, "crates/wasm/c/src/lib.rs");
    const original = readFileSync(lib, "utf8");
    const without = original
      .split("\n")
      .filter((line) => !/^#!\[.*deny\(linker_messages\)/u.test(line))
      .join("\n");
    // The line is there to take out, so the case cannot pass on a lib.rs that never had it.
    expect(without).not.toBe(original);
    writeFileSync(lib, without);
    const result = spawnSync("sh", [join(root, "scripts/build-wasm.sh")], {
      encoding: "utf8",
      env: {
        PATH: `${join(root, "bin")}:${process.env.PATH ?? ""}`,
        HOME: root,
        CARGO_LOG: join(root, "cargo-nolint.log"),
        RUN: "nolint",
        JACKIOH_WASM_PREVIEW: "0",
        JACKIOH_WASM_NORMAL: "0",
        JACKIOH_WASM_C: "1",
      },
    });
    expect(result.status).toBe(1);
    expect(result.stderr).toContain("deny(linker_messages)");
    // Only the module's own build ran: the fixture was never built.
    expect(readFileSync(join(root, "cargo-nolint.log.profile"), "utf8").split("\n").filter(Boolean)).toHaveLength(1);
  });
});
