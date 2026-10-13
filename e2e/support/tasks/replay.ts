// `cy.task("replayHash", …)` folds recorded actions through `jackioh replay` outside the browser
// and compares `hashState` (BUILD M8 spec 01, BUILD M5-T3; docs/v0.3.0/SURFACE.md §12).

import { execFileSync } from "node:child_process";
import { existsSync, mkdirSync, writeFileSync } from "node:fs";
import path from "node:path";

export type ReplayHashPayload = {
  label: string;
  seed: string;
  decks: [string[], string[]];
  log: unknown[];
  state: unknown;
  /** R180: `createGame` handicaps; R187 practice and hotseat replays need nonstandard decks, mana, hands and draws. */
  handicaps?: Partial<Record<"p1" | "p2", unknown>>;
  /** R433: the seats the game dealt (spec 13's practice random deck: the human's), passed to `fold` untouched. */
  dealt?: ("p1" | "p2")[];
};

export type ReplayHashResult = {
  replayHash: string;
  browserHash: string;
  errors: unknown[];
  logFile: string;
};

type RunnerEnvelope =
  | { ok: true; replayHash: string; browserHash: string; errors: unknown[] }
  | { ok: false; error: string };

export function replayHash(projectRoot: string, payload: ReplayHashPayload): ReplayHashResult {
  const repoRoot = path.resolve(projectRoot, "..");
  const artifacts = path.join(projectRoot, "artifacts");
  mkdirSync(artifacts, { recursive: true });
  const logFile = path.join(artifacts, `${payload.label}.json`);
  writeFileSync(
    logFile,
    `${JSON.stringify(
      {
        seed: payload.seed,
        decks: payload.decks,
        log: payload.log,
        ...(payload.handicaps === undefined ? {} : { handicaps: payload.handicaps }),
        ...(payload.dealt === undefined ? {} : { dealt: payload.dealt }),
      },
      null,
      2,
    )}\n`,
    "utf8",
  );

  const tsx = path.join(repoRoot, "node_modules", ".bin", "tsx");
  if (!existsSync(tsx)) {
    throw new Error(
      `replayHash: ${tsx} is missing. Run pnpm install at the repo root; the task folds the ` +
        "recorded log through the `jackioh` CLI (support/tasks/replay-runner.ts) with the repo's own tsx.",
    );
  }
  const runner = path.join(projectRoot, "support", "tasks", "replay-runner.ts");

  let stdout: string;
  try {
    stdout = execFileSync(tsx, [runner], {
      cwd: repoRoot,
      input: JSON.stringify(payload),
      encoding: "utf8",
      maxBuffer: 256 * 1024 * 1024,
    });
  } catch (error) {
    const detail = error instanceof Error && "stderr" in error ? String((error as { stderr?: unknown }).stderr ?? "") : "";
    throw new Error(`replayHash: the engine fold failed to run.\n${detail || String(error)}`, {
      cause: error,
    });
  }

  const line = stdout.trim().split("\n").at(-1) ?? "";
  let envelope: RunnerEnvelope;
  try {
    envelope = JSON.parse(line) as RunnerEnvelope;
  } catch {
    throw new Error(`replayHash: could not read the fold's output:\n${stdout}`);
  }
  if (!envelope.ok) throw new Error(`replayHash: ${envelope.error}`);

  return {
    replayHash: envelope.replayHash,
    browserHash: envelope.browserHash,
    errors: envelope.errors,
    logFile,
  };
}
