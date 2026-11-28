// R5269000: the practice page's persona is `jackioh-shared`'s, dealt and rolled through the module's
// `bots`; the client keeps no table or roll of its own (CLAUDE.md rule 9, #5269).

import { execFileSync } from "node:child_process";
import { readFileSync } from "node:fs";
import { dirname, join } from "node:path";
import { fileURLToPath } from "node:url";

import { describe, expect, it } from "vitest";

import { EMOTE_IDS } from "@jackioh/shared";

import { bots, practiceConstants } from "../wasm/index.ts";

const ROOT = join(dirname(fileURLToPath(import.meta.url)), "../../../..");

/** Each declaration in `files` of a persona table or roll that `jackioh-shared` owns. */
export function personaCopies(files: ReadonlyMap<string, string>): string[] {
  const declares = /\b(?:const|let|var|function)\s+(AI_PERSONAS|AI_EMOTE|pickPersona|createEmotePersona|rollForTrigger|rollForReply)\b/g;
  return [...files].flatMap(([file, text]) => [...text.matchAll(declares)].map(([, name]) => `${file}: ${name}`));
}

describe("R5269000 the practice persona through the module", () => {
  const hand = EMOTE_IDS.slice(0, 8);

  it("R5269000 a deal is the module's, on the page's own stream; a lesson's is Silent and draws nothing", () => {
    const dealt = bots({ call: "dealPersona", seed: "practice-persona:0.5", seat: "p2", hand, lesson: false });
    const lesson = bots({ call: "dealPersona", seed: "practice-persona:0.5", seat: "p2", hand, lesson: true });
    if (dealt.answer !== "persona" || lesson.answer !== "persona") throw new Error("a deal answers persona");
    expect(dealt.persona.cursor).toBe(1);
    expect(dealt.persona.seed).toBe("practice-persona:0.5");
    expect(dealt.emotes).toEqual([]);
    expect(lesson.persona.persona.persona).toBe("silent");
    expect(lesson.persona.cursor).toBe(0);
  });

  it("R5269000 the long-turn wait is the module's AI_EMOTE", () => {
    expect(practiceConstants().AI_EMOTE.longTurnMs).toBeGreaterThan(0);
  });

  it("R5269000 the scan finds a persona table or roll declared in the client", () => {
    const files = new Map([
      ["apps/web/src/a.ts", "export const AI_PERSONAS = {};\nexport function pickPersona() {}\n"],
      ["apps/web/src/b.ts", "const longTurn = practiceConstants().AI_EMOTE.longTurnMs;\n"],
    ]);
    expect(personaCopies(files)).toEqual(["apps/web/src/a.ts: AI_PERSONAS", "apps/web/src/a.ts: pickPersona"]);
  });

  it("R5269000 no web source declares a persona table or roll of its own", () => {
    const out = execFileSync("git", ["ls-files", "apps/web/src"], { cwd: ROOT, encoding: "utf8" });
    const sources = out.split("\n").filter((file) => /\.tsx?$/.test(file) && !/\.test\.tsx?$/.test(file));
    const files = new Map(sources.map((file) => [file, readFileSync(join(ROOT, file), "utf8")]));
    expect(files.size).toBeGreaterThan(100);
    expect(personaCopies(files), "call `bots` (wasm/index.ts) instead").toEqual([]);
  });
});
