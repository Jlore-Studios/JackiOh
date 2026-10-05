// R366: the words and the shape every card's printed text uses (patch v0.1.1, issue #27: "Card text
// should be short, neatly formatted, and use consistent terminology").
//
// The checks are the ruling's, one by one: the library is the deck and a tribute a Tribute; an
// embiggen card's bigger price is "Paid (N): …"; the printed keywords lead the text as one
// comma-separated list on a line of their own; each labelled ability starts a line of its own; and
// every other line is sentences that end with a full stop. Patch v0.2.0 turned the cost words round
// (R432, issue #40): "(N) Cost" is the noun ("a (1) Cost or less card", "(4)+ Cost cards") and
// "costs (N)" the verb ("costs (1) less", "costs (0)"). Every face is read with its `params` filled
// in (B3.4 rule 5), as a player reads it. Text is presentation (CLAUDE.md rule 7): no rule reads it,
// which is why the proof is a scan of the catalog rather than a game.

import { describe, expect, it } from "vitest";
import { fillParams, type CardDef, type Keyword } from "@jackioh/shared";
import { CATALOG } from "../src/catalog-data";
import { readPatches, readSnapshot } from "../scripts/patches-io";

const ENTRIES: readonly CardDef[] = Object.values(CATALOG);

type Face = { card: CardDef; face: "base" | "radiant"; text: string; keywords: readonly Keyword[] };

function facesOf(cards: readonly CardDef[]): Face[] {
  return cards.flatMap((card) =>
    (["base", "radiant"] as const).map((face) => ({
      card,
      face,
      text: fillParams(card, face),
      keywords: card[face].keywords,
    })),
  );
}

/**
 * How a printed keyword reads in the text: "Armor 7", "Lucky 1", "Brittle 4", the Bread Token's
 * "Armor X", and "Spell Damage +2" (E6 prints the plus).
 */
function keywordLabel(keyword: Keyword): string {
  if (!("n" in keyword)) return keyword.kind;
  if (keyword.kind === "Spell Damage") return `Spell Damage +${String(keyword.n)}`;
  return keyword.kind === "Armor" && keyword.n === 0 ? "Armor X" : `${keyword.kind} ${String(keyword.n)}`;
}

/** The labels that start a line of their own (R366; Activate is B3.2's, Quest Classic #90's). */
const LABELS = [
  "Cry:",
  "Death:",
  "Start of turn:",
  "End of turn:",
  "Aura:",
  "Paid (4):",
  "Activate:",
  "Activate 2:",
  "Activate ♾️:",
  "Cast on draw:",
  "Quest:",
];

/** Every R366 check a face fails, by name; empty when it passes them all. */
function failures(face: Face): string[] {
  const { text, keywords } = face;
  const out: string[] = [];
  if (/librar(y|ies)/i.test(text)) out.push("says library, not deck");
  if (/sacrific/i.test(text)) out.push("says sacrifice, not Tribute");
  if (/\b\d+-cost\b/i.test(text) || /\bcost(s|ing)? \d/i.test(text)) out.push("writes a cost without (N)");
  // R432: "(N) Cost" is the noun — never the old "Cost (N)", never a lowercase "(N) cost".
  if (/\bCost \(/.test(text)) out.push('writes the noun as "Cost (N)", not "(N) Cost"');
  if (/\(\S+\)\+? cost\b/.test(text)) out.push('writes the noun "(N) cost" without its capital');
  if (/\bcosting \(/.test(text)) out.push('writes a price as "costing (N)", not "costs (N)"');
  if (/\bthat costs \(/i.test(text)) out.push('writes "that costs (N)", not "(N) Cost"');
  if (/\b[A-Za-z]+-cost\b/.test(text)) out.push('writes a kind of cost as "odd-cost", not "odd Cost"');
  if (/\(paid \d/i.test(text)) out.push('writes an embiggen price as "(paid N" rather than "Paid (N):"');
  // Vocabulary table (patch v0.2.4, issue #45): retired words and variants.
  if (/\bbounce(s|d)?\b/i.test(text)) out.push("says bounce, not Return to hand");
  if (/\bbackrow zone\b/i.test(text)) out.push("says backrow zone, not backrow");
  if (/\b(at the (start|end)( and end)? of your turn|(start|end) of your turn)\b/i.test(text)) {
    out.push("writes turn trigger as (Start|End) of your turn, not (Start|End) of turn:");
  }
  if (/\bStart of Game\b/.test(text)) out.push('writes "Start of Game", not "Start of game"');
  if (/\bOnce per Turn\b/.test(text)) out.push('writes "Once per Turn", not "Once per turn"');
  if (/\bCannot be in Defense Position\b/i.test(text)) out.push('writes "Cannot be in Defense Position", not "Can\'t be in Defense Position"');
  if (/\bTrigger the Cry\b/i.test(text)) out.push('writes "Trigger the Cry", not "Trigger a Cry"');
  if (/\bSet a hero's health\b/i.test(text)) out.push('writes "Set a hero\'s health", not "Set health"');
  if (/\bEnd your turn\b/i.test(text)) out.push('writes "End your turn", not "End the turn"');
  // Return sends a card to its owner's hand, so no Return face names "your" hand (Add keeps it).
  if (/\breturn\b[^.\n]*\bto your hand\b/i.test(text)) out.push('writes "to your hand", not "to hand"');
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
    for (const label of LABELS) {
      if (line.indexOf(label) > 0) out.push(`does not start "${label}" on a line of its own`);
    }
  }
  return out;
}

describe("R366 the words a card's text uses (SPEC §11, patch v0.1.1)", () => {
  const swept = facesOf(ENTRIES);

  it("R366 calls the library the deck and a tribute a Tribute on every face", () => {
    const wrong = swept.filter((face) => /librar(y|ies)|sacrific/i.test(face.text));
    expect(wrong.map((face) => `${face.card.id} ${face.face}: ${face.text}`)).toEqual([]);
  });

  it("R432 writes a specific cost as the noun (N) Cost, a price as the verb costs (N) and an embiggen price as Paid (N)", () => {
    const wrong = swept.filter((face) => failures(face).some((why) => why.includes("cost") || why.includes("embiggen")));
    expect(wrong.map((face) => `${face.card.id} ${face.face}: ${face.text}`)).toEqual([]);
  });

  it("R366 leads with the face's printed keywords on a line of their own, starts each labelled ability on its own line, and ends every other line with a full stop", () => {
    const wrong = swept.flatMap((face) => failures(face).map((why) => `${face.card.id} ${face.face} ${why}: ${face.text}`));
    expect(wrong).toEqual([]);
  });

  it("R366 patch v0.2.4 only changes base.text and radiant.text between v0.2.0 and v0.2.4", () => {
    const before = readSnapshot("v0.2.0");
    const after = readSnapshot("v0.2.4");
    const differingCards: string[] = [];
    for (const [id, currentRaw] of Object.entries(after)) {
      const currentCard = currentRaw as unknown as CardDef;
      const priorCard = before[id] as unknown as CardDef | undefined;
      expect(priorCard, `card ${id} existed in v0.2.0`).toBeDefined();
      if (!priorCard) continue;

      const priorNonText = {
        ...priorCard,
        base: { ...priorCard.base, text: "" },
        radiant: { ...priorCard.radiant, text: "" },
      };
      const currentNonText = {
        ...currentCard,
        base: { ...currentCard.base, text: "" },
        radiant: { ...currentCard.radiant, text: "" },
      };
      expect(currentNonText, `non-text fields of ${id}`).toEqual(priorNonText);

      if (JSON.stringify(priorCard) !== JSON.stringify(currentCard)) {
        differingCards.push(id);
      }
    }
    expect(differingCards).toEqual([
      "core-017",
      "core-023",
      "core-024",
      "core-031",
      "core-067",
      "classic-010",
      "classic-014",
      "classic-022",
      "classic-025",
      "classic-029",
      "classic-034",
      "classic-047",
      "classic-054",
      "classic-063",
      "classic-065",
      "classic-066",
      "classicplus-014",
      "classicplus-019-5",
      "classicplus-021",
      "classicplus-026",
      "classicplus-034",
      "classicplus-052",
    ]);
  });

  it("R366 patch v0.2.5 changes only its mechanics cards between v0.2.4 and v0.2.5", () => {
    const before = readSnapshot("v0.2.4");
    const after = readSnapshot("v0.2.5");
    const nonTextChanged: Record<string, string[]> = {};
    const differingCards: string[] = [];
    for (const [id, currentRaw] of Object.entries(after)) {
      const currentCard = currentRaw as unknown as CardDef;
      const priorCard = before[id] as unknown as CardDef | undefined;
      expect(priorCard, `card ${id} existed in v0.2.4`).toBeDefined();
      if (!priorCard) continue;

      const priorNonText = {
        ...priorCard,
        base: { ...priorCard.base, text: "" },
        radiant: { ...priorCard.radiant, text: "" },
      };
      const currentNonText = {
        ...currentCard,
        base: { ...currentCard.base, text: "" },
        radiant: { ...currentCard.radiant, text: "" },
      };
      const changed = [...new Set([...Object.keys(priorNonText), ...Object.keys(currentNonText)])].filter(
        (field) =>
          JSON.stringify(priorNonText[field as keyof CardDef]) !==
          JSON.stringify(currentNonText[field as keyof CardDef]),
      );
      if (changed.length > 0) nonTextChanged[id] = changed;

      if (JSON.stringify(priorCard) !== JSON.stringify(currentCard)) {
        differingCards.push(id);
      }
    }
    // Seventeen cards gain the Plague tag and nothing else; Exile moves its threshold, Blade Storm
    // gains its Whirlwind ref, Adaptive Growth its numbers, Chaos Machine its lines of code.
    expect(nonTextChanged).toEqual({
      "core-091": ["tags"],
      "classic-010": ["params"],
      "classic-027": ["tags"],
      "classic-039": ["tags"],
      "classic-042": ["tags"],
      "classic-043": ["tags"],
      "classic-053": ["tags"],
      "classic-059": ["tags"],
      "classic-061": ["tags"],
      "classic-062": ["tags"],
      "classic-063": ["tags"],
      "classic-069": ["tags"],
      "classic-070": ["tags"],
      "classic-074": ["tags"],
      "classic-076": ["tags"],
      "classic-078": ["tags"],
      "classic-087": ["tags"],
      "classicplus-003": ["tags"],
      "classicplus-032-3": ["refs"],
      "classicplus-050": ["params", "loc"],
      "classicplus-070": ["loc"],
    });
    expect(differingCards).toEqual([
      "core-091",
      "classic-010",
      "classic-027",
      "classic-033",
      "classic-039",
      "classic-042",
      "classic-043",
      "classic-053",
      "classic-059",
      "classic-061",
      "classic-062",
      "classic-063",
      "classic-069",
      "classic-070",
      "classic-074",
      "classic-076",
      "classic-078",
      "classic-087",
      "classicplus-003",
      "classicplus-032-3",
      "classicplus-050",
      "classicplus-070",
    ]);
  });

  it("R366 patch v0.2.10 only Animates eighteen Field Spells, rewords Ivory Tower and moves Final Gambit's loc between v0.2.5 and v0.2.10", () => {
    const before = readSnapshot("v0.2.5");
    const after = readSnapshot("v0.2.10") as unknown as Record<string, CardDef>;
    const animated = new Set([
      "core-014",
      "core-033",
      "core-038",
      "core-065",
      "core-073",
      "classic-004",
      "classic-007",
      "classic-062",
      "classic-064",
      "classic-087",
      "classicplus-007",
      "classicplus-012-5",
      "classicplus-012-7",
      "classicplus-031",
      "classicplus-061",
      "classicplus-063",
      "classicplus-070",
      "classicplus-078",
    ]);
    const changed: string[] = [];
    for (const [id, currentCard] of Object.entries(after)) {
      const priorCard = before[id] as unknown as CardDef | undefined;
      expect(priorCard, `card ${id} existed in v0.2.5`).toBeDefined();
      if (!priorCard) continue;
      if (JSON.stringify(priorCard) !== JSON.stringify(currentCard)) changed.push(id);

      if (animated.has(id)) {
        for (const face of ["base", "radiant"] as const) {
          expect(priorCard[face].attack, `${id} ${face} had no stats before v0.2.10`).toBeUndefined();
          expect(priorCard[face].health, `${id} ${face} had no stats before v0.2.10`).toBeUndefined();
          expect(priorCard[face].keywords, `${id} ${face} had no keywords before v0.2.10`).toEqual([]);
          expect(currentCard[face].keywords, `${id} ${face} gains Animated`).toEqual([{ kind: "Animated" }]);
          expect(typeof currentCard[face].attack, `${id} ${face} gains attack`).toBe("number");
          expect(typeof currentCard[face].health, `${id} ${face} gains health`).toBe("number");
          expect(currentCard[face].text, `${id} ${face} prints Animated`).toBe(`Animated\n${priorCard[face].text}`);
        }
        // Nothing else on the card moves: normalizing the eight Animated fields restores the prior card.
        expect(
          {
            ...currentCard,
            base: {
              ...currentCard.base,
              attack: priorCard.base.attack,
              health: priorCard.base.health,
              keywords: priorCard.base.keywords,
              text: priorCard.base.text,
            },
            radiant: {
              ...currentCard.radiant,
              attack: priorCard.radiant.attack,
              health: priorCard.radiant.health,
              keywords: priorCard.radiant.keywords,
              text: priorCard.radiant.text,
            },
          },
          `only Animated fields differ on ${id}`,
        ).toEqual(priorCard);
      } else if (id === "classicplus-033") {
        // Ivory Tower (issue #113): the Stack aura is gone, replaced by the fusion text; it gains no stats.
        for (const face of ["base", "radiant"] as const) {
          expect(currentCard[face].keywords, `${id} ${face} gains no keywords`).toEqual([]);
          expect(currentCard[face].attack, `${id} ${face} gains no stats`).toBeUndefined();
          expect(currentCard[face].health, `${id} ${face} gains no stats`).toBeUndefined();
        }
        expect(currentCard.base.text).toBe("The first Unit you stack onto this is fused into it.");
        expect(currentCard.radiant.text).toBe("The first Unit you stack onto this becomes Radiant and is fused into it.");
        // Only the two texts and the script's loc move.
        expect(
          {
            ...currentCard,
            base: { ...currentCard.base, text: priorCard.base.text },
            radiant: { ...currentCard.radiant, text: priorCard.radiant.text },
            loc: priorCard.loc,
          },
          `only text and loc differ on ${id}`,
        ).toEqual(priorCard);
      } else if (id === "classic-052") {
        // Final Gambit's follow-up gained its R216 guard: only the script's loc moves.
        expect({ ...currentCard, loc: priorCard.loc }, `only loc differs on ${id}`).toEqual(priorCard);
      } else {
        expect(currentCard, `card ${id} unchanged by v0.2.10`).toEqual(priorCard);
      }
    }
    expect(changed.sort()).toEqual([...animated, "classicplus-033", "classic-052"].sort());
  });

  it("R366 patch v0.2.11 aims Solarius-Prime and Appropriations, keywords Deft Duelist and moves two locs between v0.2.10 and v0.2.11", () => {
    const before = readSnapshot("v0.2.10");
    // Pending until `patches ship` promotes it (R646): the current catalog until then, its snapshot after.
    const shipped = readPatches().some((patch) => patch.version === "v0.2.11");
    const after = shipped ? (readSnapshot("v0.2.11") as unknown as typeof CATALOG) : CATALOG;
    const five = new Set(["classic-003", "classicplus-010", "classicplus-038-1", "classicplus-040", "core-045"]);
    const changed: string[] = [];
    for (const [id, currentCard] of Object.entries(after)) {
      const priorCard = before[id] as unknown as CardDef | undefined;
      expect(priorCard, `card ${id} existed in v0.2.10`).toBeDefined();
      if (!priorCard) continue;
      if (JSON.stringify(priorCard) !== JSON.stringify(currentCard)) changed.push(id);
    }
    // Another pending fragment's cards legitimately differ beside these five (R646), so the diff is
    // read on this patch's claims.
    expect(changed.filter((id) => five.has(id)).sort()).toEqual([...five].sort());
    // Deft Duelist prints the Deft keyword on both faces (R49).
    expect(after["core-045"]?.base.text).toBe("Charge, Deft");
    expect(after["core-045"]?.radiant.text).toBe("Charge, Armor 1, Deft");
    // The aimed casts say so on the face (R656).
    for (const face of ["base", "radiant"] as const) {
      expect(after["classicplus-038-1"]?.[face].text).toContain(
        "Each aims at enemies when it harms and at your side when it helps.",
      );
      expect(after["classicplus-040"]?.[face].text).toContain(
        "aim at enemies when they harm and at your side when they help.",
      );
    }
    // Book of Heal and New Wraps move only their script's loc.
    for (const id of ["classic-003", "classicplus-010"]) {
      const priorCard = before[id] as unknown as CardDef;
      const currentCard = after[id] as unknown as CardDef;
      expect({ ...currentCard, loc: priorCard.loc }).toEqual(priorCard);
    }
  });

  it("R366 patch v0.2.13 (issue #88) changes exactly the balance-patch cards against v0.2.12's snapshot", () => {
    const before = readSnapshot("v0.2.12");
    // Pending until `patches ship` promotes it (R646): the current catalog until then, its snapshot after.
    // Classic #55 is claimed by the other pending fragment, v0.2.14 (issue #271, R671), so it
    // legitimately differs beside these (R646) and is read on that patch's claims instead.
    const wildfire = new Set(["classic-055"]);
    const after = CATALOG as unknown as Record<string, CardDef>;
    // The top-level fields each balance card may move (balance patch 1, issue #88); every other
    // field restores the v0.2.11 card, so no card smuggles an unlisted change.
    const allowed: Record<string, readonly string[]> = {
      "classic-004": ["base", "loc", "tags"],
      "classic-008": ["loc"],
      "classic-010": ["cost"],
      "classic-015": ["base", "radiant"],
      "classic-018": ["base", "loc"],
      "classic-020": ["loc"],
      "classic-022": ["base", "loc", "params", "radiant"],
      "classic-023": ["loc"],
      "classic-025": ["base", "loc"],
      "classic-026": ["loc"],
      "classic-028": ["base"],
      "classic-033": ["radiant"],
      "classic-034": ["base", "loc", "radiant"],
      "classic-037": ["cost"],
      "classic-038": ["base", "radiant"],
      "classic-043": ["cost"],
      "classic-046": ["base"],
      "classic-054": ["base"],
      "classic-064": ["loc"],
      "classic-065": ["loc", "radiant"],
      "classic-074": ["base", "loc", "radiant"],
      "classic-075": ["cost"],
      "classic-080": ["base", "radiant"],
      "classic-083": ["params"],
      "classic-088": ["base", "loc", "radiant"],
      "classic-090": ["base", "radiant"],
      "classicplus-007": ["base"],
      "classicplus-012-6": ["type"],
      "classicplus-014": ["loc", "params", "radiant"],
      "classicplus-019": ["loc"],
      "classicplus-030": ["cost"],
      "classicplus-031": ["base", "radiant"],
      "classicplus-038": ["base", "loc", "params", "radiant"],
      "classicplus-039": ["base", "params", "radiant"],
      "classicplus-040": ["base", "radiant"],
      "classicplus-042": ["base", "radiant", "refs"],
      "classicplus-042-1": ["loc"],
      "classicplus-046": ["base", "loc", "params", "radiant"],
      "classicplus-053": ["base", "loc", "params", "radiant"],
      "classicplus-056": ["loc"],
      "classicplus-060": ["base", "loc", "radiant"],
      "classicplus-063": ["base", "loc", "params", "radiant"],
      "classicplus-065": ["base", "params", "radiant", "refs"],
      "classicplus-065-2": ["base", "params", "radiant"],
      "classicplus-065-4": ["radiant"],
      "classicplus-066": ["base", "radiant", "refs"],
      "classicplus-073-1": ["base", "loc", "radiant"],
      "classicplus-074": ["base", "radiant"],
      "classicplus-078": ["base", "loc", "params", "radiant"],
      "core-021": ["loc"],
      "core-032": ["base", "radiant"],
      "core-067": ["loc"],
    };
    const changed: string[] = [];
    for (const [id, currentCard] of Object.entries(after)) {
      const priorCard = before[id] as unknown as CardDef | undefined;
      expect(priorCard, `card ${id} existed in v0.2.12`).toBeDefined();
      if (!priorCard) continue;
      if (JSON.stringify(priorCard) !== JSON.stringify(currentCard)) changed.push(id);
      const fields = allowed[id];
      if (fields === undefined) {
        if (wildfire.has(id)) continue;
        expect(currentCard, `card ${id} unchanged by v0.2.13`).toEqual(priorCard);
        continue;
      }
      const restored = { ...currentCard } as unknown as Record<string, unknown>;
      const prior = priorCard as unknown as Record<string, unknown>;
      for (const field of fields) restored[field] = prior[field];
      expect(restored, `only ${fields.join(", ")} differ on ${id}`).toEqual(priorCard);
    }
    expect(changed.filter((id) => !wildfire.has(id)).sort()).toEqual(Object.keys(allowed).sort());
  });

  it("R366 patch v0.2.12 removes Animated from the eighteen v0.2.10 Field Spells between v0.2.10 and v0.2.12", () => {
    const before = readSnapshot("v0.2.10");
    // Pending until `patches ship` promotes it (R646): the current catalog until then, its snapshot after.
    const shipped = readPatches().some((patch) => patch.version === "v0.2.12");
    const after = shipped ? (readSnapshot("v0.2.12") as unknown as typeof CATALOG) : CATALOG;
    const unanimated = new Set([
      "core-014",
      "core-033",
      "core-038",
      "core-065",
      "core-073",
      "classic-004",
      "classic-007",
      "classic-062",
      "classic-064",
      "classic-087",
      "classicplus-007",
      "classicplus-012-5",
      "classicplus-012-7",
      "classicplus-031",
      "classicplus-061",
      "classicplus-063",
      "classicplus-070",
      "classicplus-078",
    ]);
    const changed: string[] = [];
    for (const [id, currentCard] of Object.entries(after)) {
      const priorCard = before[id] as unknown as CardDef | undefined;
      expect(priorCard, `card ${id} existed in v0.2.10`).toBeDefined();
      if (!priorCard) continue;
      if (JSON.stringify(priorCard) !== JSON.stringify(currentCard)) changed.push(id);
      if (!unanimated.has(id)) continue;

      for (const face of ["base", "radiant"] as const) {
        expect(currentCard[face].attack, `${id} ${face} loses its stats`).toBeUndefined();
        expect(currentCard[face].health, `${id} ${face} loses its stats`).toBeUndefined();
        expect(currentCard[face].keywords, `${id} ${face} loses Animated`).toEqual([]);
        expect(priorCard[face].keywords, `${id} ${face} printed Animated`).toEqual([{ kind: "Animated" }]);
        expect(typeof priorCard[face].attack, `${id} ${face} printed attack`).toBe("number");
        expect(typeof priorCard[face].health, `${id} ${face} printed health`).toBe("number");
        expect(priorCard[face].text, `${id} ${face} printed Animated`).toBe(`Animated\n${currentCard[face].text}`);
      }
      // Nothing else on the card moves: restoring the eight Animated fields restores the snapshot.
      expect(
        {
          ...currentCard,
          base: {
            ...currentCard.base,
            attack: priorCard.base.attack,
            health: priorCard.base.health,
            keywords: priorCard.base.keywords,
            text: priorCard.base.text,
          },
          radiant: {
            ...currentCard.radiant,
            attack: priorCard.radiant.attack,
            health: priorCard.radiant.health,
            keywords: priorCard.radiant.keywords,
            text: priorCard.radiant.text,
          },
        },
        `only Animated fields differ on ${id}`,
      ).toEqual(priorCard);
    }
    // Another pending fragment's cards legitimately differ beside these eighteen (R646), so the
    // diff is read on this patch's claims.
    expect(changed.filter((id) => unanimated.has(id)).sort()).toEqual([...unanimated].sort());
  });

  it("R366 patch v0.2.4 no printed face uses any word the vocabulary table retired", () => {
    const wrong = swept.filter((face) =>
      failures(face).some(
        (why) =>
          why.includes("bounce") ||
          why.includes("backrow zone") ||
          why.includes("turn trigger") ||
          why.includes("that costs") ||
          why.includes("Start of Game") ||
          why.includes("Once per Turn") ||
          why.includes("Defense Position") ||
          why.includes("Trigger the Cry") ||
          why.includes("Set a hero") ||
          why.includes("End your turn") ||
          why.includes("to your hand"),
      ),
    );
    expect(wrong.map((face) => `${face.card.id} ${face.face}: ${face.text}`)).toEqual([]);
  });

  it("R366 patch v0.2.4 the failure detector catches retired vocabulary terms", () => {
    const dummyCard = ENTRIES[0]!;
    const check = (text: string) => failures({ card: dummyCard, face: "base", text, keywords: [] });
    expect(check("Bounce a target Unit.")).toContain("says bounce, not Return to hand");
    expect(check("Destroy a backrow zone.")).toContain("says backrow zone, not backrow");
    expect(check("Start of your turn: Draw 1.")).toContain("writes turn trigger as (Start|End) of your turn, not (Start|End) of turn:");
    expect(check("End of your turn: Deal 1 damage.")).toContain("writes turn trigger as (Start|End) of your turn, not (Start|End) of turn:");
    expect(check("At the start of your turn, Draw 1.")).toContain("writes turn trigger as (Start|End) of your turn, not (Start|End) of turn:");
    expect(check("At the end of your turn, Draw 1.")).toContain("writes turn trigger as (Start|End) of your turn, not (Start|End) of turn:");
    expect(check("At the start and end of your turn, this attacks.")).toContain("writes turn trigger as (Start|End) of your turn, not (Start|End) of turn:");
    expect(check("Discover a Spell that costs (1).")).toContain('writes "that costs (N)", not "(N) Cost"');
    expect(check("Start of Game: Draw 1.")).toContain('writes "Start of Game", not "Start of game"');
    expect(check("Once per Turn: Gain 1 mana.")).toContain('writes "Once per Turn", not "Once per turn"');
    expect(check("Cannot be in Defense Position.")).toContain('writes "Cannot be in Defense Position", not "Can\'t be in Defense Position"');
    expect(check("Trigger the Cry of a Unit.")).toContain('writes "Trigger the Cry", not "Trigger a Cry"');
    expect(check("Set a hero's health to 13.")).toContain('writes "Set a hero\'s health", not "Set health"');
    expect(check("Cast on draw: End your turn.")).toContain('writes "End your turn", not "End the turn"');
    expect(check("End of turn: Return this to your hand.")).toContain('writes "to your hand", not "to hand"');
    expect(check("End of turn: Return this to hand.")).not.toContain('writes "to your hand", not "to hand"');
    expect(check("Add a card to your hand.")).not.toContain('writes "to your hand", not "to hand"');
    expect(check("Cry: Add 2 random Units to your hand. They cost (1).")).toEqual([]);
  });
});

