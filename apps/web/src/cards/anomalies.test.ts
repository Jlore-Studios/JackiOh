// R3337000: the client's ten Anomalies (anomalies.ts) are the cards crate's boards, read from the Anomaly Found!'s
// script: their codes in `ALL_BOARDS`' order, their names, and the three `RICH_BOARDS` a Radiant face lays.

import { readFileSync } from "node:fs";
import { dirname, join } from "node:path";
import { fileURLToPath } from "node:url";

import { describe, expect, it } from "vitest";

import { ANOMALIES, ANOMALY_CODES, isAnomalyCode } from "./anomalies.ts";

const HERE = dirname(fileURLToPath(import.meta.url));
const ROOT = join(HERE, "../../../..");
const SCRIPT = readFileSync(join(ROOT, "crates/cards/src/scripts/stinkyscape/t_anomaly_found.rs"), "utf8");

/** Each `pub const XX: Board = Board { code: "XX", name: "…", … }`: its constant, its code and its name. */
const BOARD = /pub const ([A-Z]+): Board = Board \{\s*code: "([^"]*)",\s*name: "([^"]*)",/g;

/** The constants a `pub const <list>: [Board; n] = [ … ];` names, in its order. */
function boardList(list: string): string[] {
  const found = new RegExp(`pub const ${list}: \\[Board; \\d+\\] = \\[([^\\]]*)\\];`).exec(SCRIPT);
  if (found === null) throw new Error(`t_anomaly_found.rs has no ${list}`);
  return (found[1] ?? "")
    .split(",")
    .map((name) => name.trim())
    .filter((name) => name !== "");
}

describe("R3337000: the client's Anomalies", () => {
  it("R3337000 the codes, names and rich boards are the cards crate's", () => {
    const boards = new Map([...SCRIPT.matchAll(BOARD)].map(([, constant, code, name]) => [constant ?? "", { code: code ?? "", name: name ?? "" }]));
    expect(boards.size).toBe(ANOMALY_CODES.length);
    for (const [constant, board] of boards) expect(board.code, constant).toBe(constant);
    const codeOf = (constant: string): string => {
      const board = boards.get(constant);
      if (board === undefined) throw new Error(`t_anomaly_found.rs has no board ${constant}`);
      return board.code;
    };
    expect(boardList("ALL_BOARDS").map(codeOf)).toEqual([...ANOMALY_CODES]);
    const rich = new Set(boardList("RICH_BOARDS").map(codeOf));
    for (const code of ANOMALY_CODES) {
      expect(ANOMALIES[code].name, code).toBe(boards.get(code)?.name);
      expect(ANOMALIES[code].rich, code).toBe(rich.has(code));
    }
    expect(rich.size).toBe(ANOMALY_CODES.filter((code) => ANOMALIES[code].rich).length);
    expect(ANOMALY_CODES.every((code) => isAnomalyCode(code))).toBe(true);
    expect([isAnomalyCode("ZZ"), isAnomalyCode(null), isAnomalyCode("wh")]).toEqual([false, false, false]);
  });
});
