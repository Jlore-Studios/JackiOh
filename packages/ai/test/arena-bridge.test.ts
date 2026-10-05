// The arena bridge's protocol test (issue #55, phase 1): one game end to end through the JSON
// protocol, plus an error reply. It drives the real bridge process over stdio.

import { spawn, type ChildProcess } from "node:child_process";
import { fileURLToPath } from "node:url";
import { describe, expect, it } from "vitest";

const BRIDGE = fileURLToPath(new URL("../scripts/arena-bridge.ts", import.meta.url));
const AI_DIR = fileURLToPath(new URL("..", import.meta.url));

type Reply = Record<string, unknown>;

function startBridge(): { proc: ChildProcess; send: (cmd: unknown) => Promise<Reply> } {
  const proc = spawn("pnpm", ["exec", "tsx", BRIDGE], { cwd: AI_DIR, stdio: ["pipe", "pipe", "pipe"] });
  let pending = "";
  const lines: string[] = [];
  const waiters: ((line: string) => void)[] = [];
  proc.stdout?.setEncoding("utf8");
  proc.stdout?.on("data", (chunk: string) => {
    pending += chunk;
    let at = pending.indexOf("\n");
    while (at >= 0) {
      const line = pending.slice(0, at).trim();
      pending = pending.slice(at + 1);
      if (line !== "") {
        const waiter = waiters.shift();
        if (waiter !== undefined) waiter(line);
        else lines.push(line);
      }
      at = pending.indexOf("\n");
    }
  });
  const send = (cmd: unknown): Promise<Reply> =>
    new Promise((resolve, reject) => {
      const queued = lines.shift();
      const waiter = (line: string): void => {
        try {
          resolve(JSON.parse(line) as Reply);
        } catch (error) {
          reject(error);
        }
      };
      if (queued !== undefined) waiter(queued);
      else waiters.push(waiter);
      proc.stdin?.write(`${JSON.stringify(cmd)}\n`);
    });
  return { proc, send };
}

describe("arena-bridge", () => {
  it("plays one game end to end through the JSON protocol, plus an error reply", async () => {
    const { proc, send } = startBridge();
    try {
      const deckA = await send({ cmd: "deck", seed: "arena-bridge-test:a", size: 20 });
      const deckB = await send({ cmd: "deck", seed: "arena-bridge-test:b", size: 20 });
      expect(deckA["ok"]).toBe(true);
      expect(deckB["ok"]).toBe(true);

      const fresh = await send({
        cmd: "new_game",
        gameId: "g1",
        seed: "arena-bridge-test",
        decks: [deckA["deck"], deckB["deck"]],
      });
      expect(fresh).toEqual({ ok: true, gameId: "g1" });

      const legalP1 = await send({ cmd: "legal", gameId: "g1", seat: "p1" });
      expect(legalP1["ok"]).toBe(true);
      expect(Array.isArray(legalP1["legal"])).toBe(true);

      // Agents see only their own view, never the true state (CLAUDE.md rule 7).
      const viewP1 = await send({ cmd: "observe", gameId: "g1", seat: "p1" });
      expect(viewP1["ok"]).toBe(true);
      expect(viewP1["view"]).toBeDefined();

      // An unknown command replies with an error; the bridge stays up.
      expect(await send({ cmd: "nope" })).toMatchObject({ ok: false });

      // An illegal action replies with an error and leaves the game playable.
      expect(await send({ cmd: "act", gameId: "g1", seat: "p1", action: { type: "nope" } })).toMatchObject({
        ok: false,
      });

      // A concession ends the game: the full open-to-result path through the protocol.
      const conceded = await send({ cmd: "act", gameId: "g1", seat: "p1", action: { type: "concede" } });
      expect(conceded["ok"]).toBe(true);
      expect(conceded["result"]).toMatchObject({ winner: "p2" });

      const result = await send({ cmd: "result", gameId: "g1" });
      expect(result).toMatchObject({ ok: true, result: { winner: "p2" } });
      expect(typeof result["hash"]).toBe("string");

      expect(await send({ cmd: "close", gameId: "g1" })).toEqual({ ok: true });
    } finally {
      proc.kill();
    }
  }, 120000);
});
