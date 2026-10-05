// The ladder arena's engine bridge (issue #55, phase 1): one long-lived Node process speaking
// JSON lines on stdin/stdout, so a gauntlet of hundreds of games pays one startup, not one per
// decision. It holds games in a map keyed by id and answers `new_game`, `legal`, `observe`, `act`,
// `result`, `close`, `deck` and `cards` from `createGame`, `beginGame`, `legalActions`, `viewFor`
// (CLAUDE.md rule 7: an agent sees only its own view, never the true state) and `reduce`, with
// decks from `buildAiDeck`. Actions are the engine's own `ActionBody` JSON, passed through
// unchanged. Every reply is one JSON line; an unknown command or an illegal action replies
// `{"ok":false,"error":…}` — the bridge never throws.
//
// Node tooling, so it may use stdin/stdout and the process APIs `src/` must not touch.

import { registerAll } from "@jackioh/cards";
import {
  beginGame,
  createGame,
  createRng,
  hashState,
  legalActions,
  query,
  reduce,
  seatToAct,
  viewFor,
  type GameState,
} from "@jackioh/engine";
import type { Action, ActionBody, PlayerId } from "@jackioh/shared";
import { buildAiDeck } from "../src/index";

registerAll();

type GameEntry = { state: GameState; nonces: number };

const games = new Map<string, GameEntry>();

function isPlayerId(value: unknown): value is PlayerId {
  return value === "p1" || value === "p2";
}

function fail(error: string): string {
  return JSON.stringify({ ok: false, error });
}

function handle(line: string): string {
  let msg: unknown;
  try {
    msg = JSON.parse(line);
  } catch {
    return fail("not JSON");
  }
  if (typeof msg !== "object" || msg === null) return fail("not an object");
  const cmd = (msg as Record<string, unknown>)["cmd"];
  try {
    switch (cmd) {
      case "new_game": {
        const { gameId, seed, decks } = msg as { gameId?: unknown; seed?: unknown; decks?: unknown };
        if (typeof gameId !== "string" || gameId === "") return fail("new_game needs a gameId");
        if (typeof seed !== "string" || seed === "") return fail("new_game needs a seed");
        if (!Array.isArray(decks) || decks.length !== 2) return fail("new_game needs decks [[...],[...]]");
        const created = createGame({ seed, decks: decks as [string[], string[]] });
        const started = beginGame(created);
        if (started.error !== undefined) return fail(`beginGame: ${started.error}`);
        games.set(gameId, { state: started.state, nonces: 0 });
        return JSON.stringify({ ok: true, gameId });
      }
      case "legal": {
        const { gameId, seat } = msg as { gameId?: unknown; seat?: unknown };
        const entry = typeof gameId === "string" ? games.get(gameId) : undefined;
        if (entry === undefined) return fail("unknown gameId");
        if (!isPlayerId(seat)) return fail("legal needs seat p1|p2");
        return JSON.stringify({ ok: true, legal: legalActions(entry.state, seat) });
      }
      case "observe": {
        const { gameId, seat } = msg as { gameId?: unknown; seat?: unknown };
        const entry = typeof gameId === "string" ? games.get(gameId) : undefined;
        if (entry === undefined) return fail("unknown gameId");
        if (!isPlayerId(seat)) return fail("observe needs seat p1|p2");
        return JSON.stringify({ ok: true, view: viewFor(entry.state, seat) });
      }
      case "act": {
        const { gameId, seat, action } = msg as { gameId?: unknown; seat?: unknown; action?: unknown };
        const entry = typeof gameId === "string" ? games.get(gameId) : undefined;
        if (entry === undefined) return fail("unknown gameId");
        if (!isPlayerId(seat)) return fail("act needs seat p1|p2");
        if (typeof action !== "object" || action === null) return fail("act needs an action body");
        const nonce = `b${entry.nonces}`;
        const full = { ...(action as ActionBody), playerId: seat, nonce } as Action;
        const out = reduce(entry.state, full);
        if (out.error !== undefined) return fail(out.error);
        entry.state = out.state;
        entry.nonces += 1;
        const next = out.state.result === null ? seatToAct(out.state) : null;
        return JSON.stringify({
          ok: true,
          events: out.events,
          result: out.state.result,
          turn: out.state.turn,
          next,
        });
      }
      case "result": {
        const { gameId } = msg as { gameId?: unknown };
        const entry = typeof gameId === "string" ? games.get(gameId) : undefined;
        if (entry === undefined) return fail("unknown gameId");
        return JSON.stringify({
          ok: true,
          result: entry.state.result,
          turn: entry.state.turn,
          hash: hashState(entry.state),
        });
      }
      case "close": {
        const { gameId } = msg as { gameId?: unknown };
        if (typeof gameId !== "string" || !games.delete(gameId)) return fail("unknown gameId");
        return JSON.stringify({ ok: true });
      }
      case "deck": {
        const { seed, size, manaCap } = msg as { seed?: unknown; size?: unknown; manaCap?: unknown };
        if (typeof seed !== "string" || seed === "") return fail("deck needs a seed");
        if (typeof size !== "number" || !Number.isInteger(size) || size <= 0) {
          return fail("deck needs an integer size");
        }
        const rng = createRng(seed);
        const deck =
          typeof manaCap === "number" ? buildAiDeck(rng, size, { manaCap }) : buildAiDeck(rng, size);
        return JSON.stringify({ ok: true, deck });
      }
      case "cards": {
        // The non-token pool ladder agents may be dealt (SPEC §2.6 L3): every set, no tokens.
        const ids = query().map((def) => def.id);
        return JSON.stringify({ ok: true, cards: ids, count: ids.length });
      }
      default:
        return fail(`unknown cmd ${typeof cmd === "string" ? cmd : "?"}`);
    }
  } catch (error) {
    return fail(error instanceof Error ? error.message : String(error));
  }
}

let pending = "";

process.stdin.setEncoding("utf8");
process.stdin.on("data", (chunk: string) => {
  pending += chunk;
  let at = pending.indexOf("\n");
  while (at >= 0) {
    const line = pending.slice(0, at).trim();
    pending = pending.slice(at + 1);
    if (line !== "") {
      process.stdout.write(`${handle(line)}\n`);
    }
    at = pending.indexOf("\n");
  }
});
