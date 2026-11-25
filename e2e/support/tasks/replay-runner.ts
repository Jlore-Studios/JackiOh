// Runs under the repo's tsx from its root: the Rust CLI folds a replay (`jackioh replay`) and hashes the
// browser's state (`jackioh hash-state`), both by the engine's one `hash_state` (SURFACE §5.2, §12; #4766).

import { spawnSync } from "node:child_process";
import { existsSync, readFileSync } from "node:fs";
import path from "node:path";

type Payload = {
  seed: string;
  decks: [string[], string[]];
  log: unknown[];
  state: unknown;
  /** R180: pass handicaps to the fold untouched. */
  handicaps?: Partial<Record<"p1" | "p2", unknown>>;
  /** R433: dealt seats, likewise. */
  dealt?: ("p1" | "p2")[];
};

type CliAnswer = { hash: string; errors: unknown[] };

const CLI = path.resolve("target", "release", process.platform === "win32" ? "jackioh.exe" : "jackioh");

function requireCli(): void {
  if (!existsSync(CLI)) {
    throw new Error(
      `${CLI} is missing, so the recorded log cannot be folded by the engine. ` +
        "Build it from the repo root with `cargo build --release -p jackioh-tools`.",
    );
  }
}

/** Runs `jackioh <mode>` on `input` and parses its last stdout line. */
function runCli(mode: string, input: unknown): unknown {
  requireCli();
  const run = spawnSync(CLI, [mode], {
    input: JSON.stringify(input),
    encoding: "utf8",
    maxBuffer: 256 * 1024 * 1024,
  });
  if (run.error !== undefined) throw run.error;
  if (run.status !== 0) {
    throw new Error(`jackioh ${mode} exited with ${String(run.status)}:\n${run.stderr || run.stdout}`);
  }
  const line = run.stdout.trim().split("\n").at(-1) ?? "";
  try {
    return JSON.parse(line) as unknown;
  } catch {
    throw new Error(`jackioh ${mode} printed something that is not its answer:\n${run.stdout}`);
  }
}

/** The browser's state hashed by the engine's own `hash_state` (`jackioh hash-state`), so no copy of its recipe lives here. */
function hashViaCli(state: unknown): string {
  const answer = runCli("hash-state", state) as { hash?: unknown };
  if (typeof answer.hash !== "string") {
    throw new Error(`jackioh hash-state's answer has no hash: ${JSON.stringify(answer)}`);
  }
  return answer.hash;
}

function fold(payload: Payload): CliAnswer {
  const input = {
    seed: payload.seed,
    decks: payload.decks,
    log: payload.log,
    ...(payload.handicaps === undefined ? {} : { handicaps: payload.handicaps }),
    ...(payload.dealt === undefined ? {} : { dealt: payload.dealt }),
  };
  const answer = runCli("replay", input) as CliAnswer;
  if (typeof answer.hash !== "string" || !Array.isArray(answer.errors)) {
    throw new Error(`jackioh replay's answer has no hash or errors: ${JSON.stringify(answer)}`);
  }
  return answer;
}

function main(): void {
  const payload = JSON.parse(readFileSync(0, "utf8")) as Payload;
  const result = fold(payload);
  process.stdout.write(
    `${JSON.stringify({
      ok: true,
      replayHash: result.hash,
      browserHash: hashViaCli(payload.state),
      errors: result.errors,
    })}\n`,
  );
}

try {
  main();
} catch (error: unknown) {
  const message = error instanceof Error ? `${error.message}\n${error.stack ?? ""}` : String(error);
  process.stdout.write(`${JSON.stringify({ ok: false, error: message })}\n`);
}
