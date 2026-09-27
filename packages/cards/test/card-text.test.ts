// R366: the words and the shape every card's printed text uses (patch v0.1.1, issue #27: "Card text
// should be short, neatly formatted, and use consistent terminology").
//
// The checks are the ruling's, one by one: the library is the deck and a tribute a Tribute; a
// specific cost is "Cost (N)" and a price "(N)"; an embiggen card's bigger price is "Paid (N): …";
// the printed keywords lead the text as one comma-separated list on a line of their own; each
// labelled ability starts a line of its own; and every other line is sentences that end with a full
// stop. Text is presentation (CLAUDE.md rule 7): no rule reads it, which is
// why the proof is a scan of the catalog rather than a game.

import { describe, expect, it } from "vitest";
import type { CardDef, CardFace, Keyword } from "@jackioh/shared";
import { CATALOG } from "../src/catalog-data";

const ENTRIES: readonly CardDef[] = Object.values(CATALOG);

/**
 * The cards patch v0.1.1 gave to its other two branches — the engine's (#24, #44, #74, #80, #90,
 * #90.1, #98) and the client's (#93, #93.1) — whose texts those branches rewrite. The last test
 * below holds this list to the cards that still fail a check, so it can only shrink: once a card's
 * new text is merged and passes, the list must drop it.
 */
const NOT_YET_SWEPT: ReadonlySet<string> = new Set([
  "core-024",
  "core-044",
  "core-074",
  "core-080",
  "core-090",
  "core-090-1",
  "core-093",
  "core-093-1",
  "core-098",
]);

type Face = { card: CardDef; face: "base" | "radiant"; text: string; keywords: readonly Keyword[] };

function facesOf(cards: readonly CardDef[]): Face[] {
  return cards.flatMap((card) =>
    (["base", "radiant"] as const).map((face) => {
      const printed: CardFace = card[face];
      return { card, face, text: printed.text, keywords: printed.keywords };
    }),
  );
}

/** How a printed keyword reads in the text: "Armor 7", "Lucky 1", the Bread Token's "Armor X". */
function keywordLabel(keyword: Keyword): string {
  if (!("n" in keyword)) return keyword.kind;
  return keyword.kind === "Armor" && keyword.n === 0 ? "Armor X" : `${keyword.kind} ${String(keyword.n)}`;
}

/** Every R366 check a face fails, by name; empty when it passes them all. */
function failures(face: Face): string[] {
  const { text, keywords } = face;
  const out: string[] = [];
  if (/librar(y|ies)/i.test(text)) out.push("says library, not deck");
  if (/sacrific/i.test(text)) out.push("says sacrifice, not Tribute");
  if (/\b\d+-cost\b/i.test(text) || /\bcost(s|ing)? \d/i.test(text)) out.push("writes a cost without (N)");
  if (/\(paid \d/i.test(text)) out.push('writes an embiggen price as "(paid N" rather than "Paid (N):"');
  const lines = text === "" ? [] : text.split("\n");
  // The keyword list: the first line, when every item on it is a printed keyword or a Tribute cost,
  // which the list carries too (#55, #66). A face with keywords must lead with them.
  const labels = keywords.map(keywordLabel);
  const listItem = (item: string): boolean => labels.includes(item) || /^(Tribute|Echo) \d+$/.test(item);
  const lead = (lines[0] ?? "").split(", ");
  const hasList = lines.length > 0 && lead.every(listItem);
  for (const label of labels) {
    if (!hasList || !lead.includes(label)) out.push(`does not lead with its keyword ${label}`);
  }
  // Every other line is sentences, each ending with a full stop, and a labelled ability ("Death:",
  // "Aura:", "Paid (4):") starts a line of its own.
  for (const [at, line] of lines.entries()) {
    if (at === 0 && hasList) {
      if (/\.$/.test(line)) out.push("ends its keyword line with a full stop");
      continue;
    }
    if (!/\.["”]?$/.test(line)) out.push(`does not end the line "${line}" with a full stop`);
    for (const label of ["Cry:", "Death:", "Start of turn:", "End of turn:", "Aura:", "Paid (4):"]) {
      if (line.indexOf(label) > 0) out.push(`does not start "${label}" on a line of its own`);
    }
  }
  return out;
}

describe("R366 the words a card's text uses (SPEC §11, patch v0.1.1)", () => {
  const swept = facesOf(ENTRIES.filter((card) => !NOT_YET_SWEPT.has(card.id)));

  it("R366 calls the library the deck and a tribute a Tribute on every face", () => {
    const wrong = swept.filter((face) => /librar(y|ies)|sacrific/i.test(face.text));
    expect(wrong.map((face) => `${face.card.id} ${face.face}: ${face.text}`)).toEqual([]);
  });

  it('R366 writes a specific cost as "Cost (N)", a price as "(N)" and an embiggen price as "Paid (N)"', () => {
    const wrong = swept.filter((face) => failures(face).some((why) => why.includes("cost") || why.includes("embiggen")));
    expect(wrong.map((face) => `${face.card.id} ${face.face}: ${face.text}`)).toEqual([]);
  });

  it("R366 leads with the face's printed keywords on a line of their own, starts each labelled ability on its own line, and ends every other line with a full stop", () => {
    const wrong = swept.flatMap((face) => failures(face).map((why) => `${face.card.id} ${face.face} ${why}: ${face.text}`));
    expect(wrong).toEqual([]);
  });

  it("R366 lets the not-yet-swept list name only cards that still fail, so it can only shrink", () => {
    const passing = [...NOT_YET_SWEPT].filter((id) => {
      const card = CATALOG[id];
      return card !== undefined && facesOf([card]).every((face) => failures(face).length === 0);
    });
    expect(passing, "these now pass R366: take them off NOT_YET_SWEPT").toEqual([]);
  });
});
