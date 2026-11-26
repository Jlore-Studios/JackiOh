// `cy.task("replayHash", …)` folds the log and hashes the browser's state in one
// `jackioh replay-check` call (#4925).

import { execFileSync } from "node:child_process";
import { existsSync, mkdirSync, writeFileSync } from "node:fs";
import path from "node:path";

export type ReplayHashPayload = {
  label: string;
  seed: string;
  decks: [string[], string[]];
  log: unknown[];
  state: unknown;
  /** R180: `createGame` handicaps (R187: nonstandard decks, mana, hands and draws). */
  handicaps?: Partial<Record<"p1" | "p2", unknown>>;
  /** R433: the seats the game dealt, passed to `fold` untouched. */
  dealt?: ("p1" | "p2")[];
};

export type ReplayHashResult = {
  replayHash: string;
  browserHash: string;
  errors: unknown[];
  logFile: string;
};

type CheckAnswer = Pick<ReplayHashResult, "replayHash" | "browserHash" | "errors">;

function isCheckAnswer(answer: unknown): answer is CheckAnswer {
  const found = answer as Partial<CheckAnswer> | null;
  return (
    typeof found?.replayHash === "string" &&
    typeof found?.browserHash === "string" &&
    Array.isArray(found?.errors)
  );
}

export function replayHash(projectRoot: string, payload: ReplayHashPayload): ReplayHashResult {
  const repoRoot = path.join(projectRoot, "..");
  const artifacts = path.join(projectRoot, process.env.E2E_ARTIFACTS ?? "artifacts");
  mkdirSync(artifacts, { recursive: true });
  const logFile = path.join(artifacts, `${payload.label}.json`);
  const game = {
    seed: payload.seed,
    decks: payload.decks,
    log: payload.log,
    ...(payload.handicaps === undefined ? {} : { handicaps: payload.handicaps }),
    ...(payload.dealt === undefined ? {} : { dealt: payload.dealt }),
  };
  writeFileSync(logFile, `${JSON.stringify(game, null, 2)}\n`, "utf8");

  const cli = path.join(repoRoot, "target", "release", process.platform === "win32" ? "jackioh.exe" : "jackioh");
  if (!existsSync(cli)) {
    throw new Error(
      `replayHash: ${cli} is missing, so the recorded log cannot be folded by the engine. Build it from the repo root with \`cargo build --release -p jackioh-tools\`.`,
    );
  }

  let stdout: string;
  try {
    stdout = execFileSync(cli, ["replay-check"], {
      input: JSON.stringify({ ...game, state: payload.state }),
      encoding: "utf8",
      maxBuffer: 256 * 1024 * 1024,
    });
  } catch (error) {
    const detail = error instanceof Error && "stderr" in error ? String((error as { stderr?: unknown }).stderr ?? "") : "";
    throw new Error(
      `replayHash: \`jackioh replay-check\` failed (is target/release/jackioh older than this checkout? rebuild it).\n${detail || String(error)}`,
      { cause: error },
    );
  }

  let answer: unknown;
  try {
    answer = JSON.parse(stdout) as unknown;
  } catch {
    throw new Error(`replayHash: \`jackioh replay-check\` printed something that is not its answer:\n${stdout}`);
  }
  if (!isCheckAnswer(answer)) {
    throw new Error(`replayHash: the answer has no replayHash, browserHash or errors: ${stdout}`);
  }

  return {
    replayHash: answer.replayHash,
    browserHash: answer.browserHash,
    errors: answer.errors,
    logFile,
  };
}
