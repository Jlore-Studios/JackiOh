// Runs under the repo's tsx from its root, folds a replay, and compares it with browser state.
// The Rust CLI folds registered scripts; the browser hash mirrors canonical JSON/FNV-1a only for
// comparison (SURFACE §5.2, §12).

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
  /** R433: pass dealt seats to the fold untouched. */
  dealt?: ("p1" | "p2")[];
};

type CliAnswer = { hash: string; errors: unknown[] };

const CLI = path.resolve("target", "release", process.platform === "win32" ? "jackioh.exe" : "jackioh");

/** Canonical JSON has sorted keys, so equal states produce identical text (SURFACE §5.2). */
function canonical(value: unknown): string {
  if (value === null || typeof value !== "object") return JSON.stringify(value) ?? "null";
  if (Array.isArray(value)) return `[${value.map(canonical).join(",")}]`;
  const entries = Object.entries(value as Record<string, unknown>)
    .filter(([, v]) => v !== undefined)
    .sort(([a], [b]) => (a < b ? -1 : a > b ? 1 : 0))
    .map(([k, v]) => `${JSON.stringify(k)}:${canonical(v)}`);
  return `{${entries.join(",")}}`;
}

/**
 * Hash canonical state without bookkeeping `applied` or a Glitch-redrawn opening (R676).
 */
function hashState(state: unknown): string {
  const { applied: _applied, opening: _opening, ...rest } = state as Record<string, unknown>;
  const text = canonical(rest);
  let hash = 0x811c9dc5;
  for (let i = 0; i < text.length; i += 1) {
    hash ^= text.charCodeAt(i);
    hash = Math.imul(hash, 0x01000193) >>> 0;
  }
  return hash.toString(16).padStart(8, "0");
}

function fold(payload: Payload): CliAnswer {
  if (!existsSync(CLI)) {
    throw new Error(
      `${CLI} is missing, so the recorded log cannot be folded by the engine. ` +
        "Build it from the repo root with `cargo build --release -p jackioh-tools`.",
    );
  }
  const input = {
    seed: payload.seed,
    decks: payload.decks,
    log: payload.log,
    ...(payload.handicaps === undefined ? {} : { handicaps: payload.handicaps }),
    ...(payload.dealt === undefined ? {} : { dealt: payload.dealt }),
  };
  const run = spawnSync(CLI, ["replay"], {
    input: JSON.stringify(input),
    encoding: "utf8",
    maxBuffer: 256 * 1024 * 1024,
  });
  if (run.error !== undefined) throw run.error;
  if (run.status !== 0) {
    throw new Error(`jackioh replay exited with ${String(run.status)}:\n${run.stderr || run.stdout}`);
  }
  const line = run.stdout.trim().split("\n").at(-1) ?? "";
  let answer: CliAnswer;
  try {
    answer = JSON.parse(line) as CliAnswer;
  } catch {
    throw new Error(`jackioh replay printed something that is not its answer:\n${run.stdout}`);
  }
  if (typeof answer.hash !== "string" || !Array.isArray(answer.errors)) {
    throw new Error(`jackioh replay's answer has no hash or errors: ${line}`);
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
      browserHash: hashState(payload.state),
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
