// Polish task 2 (docs/polish/2-sound.md), behaviours B41 to B43 and R655: the lookups in
// `voiceData.ts` that the engine, the cue table and the hook read, its parser of `card-audio.json5`,
// and the file's layout.
//
//   B41  `voiceKey` joins "<defId>-<line>" (defIds contain "-"), `voiceUrl` serves it from
//        BASE_URL's `audio/voice/`, and `lineFor` returns the line's text with the voice its hook
//        names, or null for the sentinel, an unknown id, a hook the card's kind does not have, or a
//        hook that is only an effect.
//   B42  `parseCardAudio` accepts the shipped file, adding only each card's kind and each effect's
//        default pitch and gain, and rejects a malformed one with `card-audio.json5: <path>:
//        <problem>`; it does not check word limits or the lines every card must have (B33, B34 do).
//   B43  `voiceKeysForView` lists, deduped and in this order, the viewer's hand (unit -> play,
//        spell -> cast, traps none), every unit on both boards (death), the viewer's own units
//        (attack, R655) and the viewer's own face-up backrow traps (cast); a card it cannot name, or
//        a hook with no line, adds nothing.
//   R655 every shape error names its path, JSON5's comments and trailing commas parse, a card's kind
//        comes from its catalog type, and `hookFor`, `lineFor` and `effectFor` read a hook that is a
//        line, an effect or both.
//
// Plus the file's layout: voices sorted by name, cards in catalog order, a trailing newline.

import { readFileSync } from "node:fs";
import { dirname, resolve } from "node:path";
import { fileURLToPath } from "node:url";

import type { PlayerView } from "@jackioh/shared";
import JSON5 from "json5";
import { beforeEach, describe, expect, it } from "vitest";

import { HIDDEN_DEF_ID } from "./constants.ts";
import type { CardAudioTable } from "./types.ts";
import {
  CARD_AUDIO,
  type CatalogTypes,
  effectFor,
  emoteLineFor,
  emoteVoiceDef,
  entryFor,
  hookFor,
  lineFor,
  parseCardAudio,
  parseCardAudioText,
  voiceKey,
  voiceKeysForView,
  voiceUrl,
} from "./voiceData.ts";
import { baseView, card, emptySide, faceDownBackrow, faceUpBackrow, resetIds, unit } from "../test/fixtures.ts";

const here = dirname(fileURLToPath(import.meta.url));
const REPO = resolve(here, "../../../..");
const AUDIO_PATH = resolve(here, "card-audio.json5");
const CATALOG_PATH = resolve(REPO, "crates/cards/catalog.json");

const SHIPPED_CATALOG = JSON.parse(readFileSync(CATALOG_PATH, "utf8")) as CatalogTypes;

function shippedText(): string {
  return readFileSync(AUDIO_PATH, "utf8");
}

type RawTable = {
  voices: Record<string, Record<string, unknown>>;
  effects: Record<string, Record<string, unknown>>;
  cards: Record<string, Record<string, unknown>>;
};

/** A fresh copy of the shipped file, as raw JSON5 data. */
function shippedRaw(): RawTable {
  return JSON5.parse<RawTable>(shippedText());
}

/** A tiny catalog: one card of each type the parser maps, by the type the shipped one gives it. */
const CATALOG: CatalogTypes = {
  "core-004": { type: "Unit" },
  "core-005": { type: "Spell" },
  "core-006": { type: "Field Spell" },
  "core-018": { type: "Field Trap" },
  "core-041": { type: "Trap" },
};

const SAY = { say: "Rocko (English (US))", rate: 215, pbas: 50, pmod: 45, web: { pitch: 1, rate: 1.15 }, gain: 1.22 };
const CRONE = { say: "Grandma (English (US))", rate: 170, pbas: 50, pmod: 40, web: { pitch: 1.1, rate: 0.9 } };

/** A small file for one malformed edit at a time: a line, an effect and both on a unit, a line on a spell, an effect on a trap. */
function fixture(): RawTable {
  return {
    voices: { hustler: { ...SAY }, crone: { ...CRONE } },
    effects: {
      thud: { sfx: "impact", pitch: 0.6, params: { amount: 8 } },
      zip: { sfx: "whoosh" },
    },
    cards: {
      "core-004": {
        play: { voice: "hustler", text: "Double or nothing, baby!" },
        attack: { effect: "zip" },
        death: { voice: "hustler", text: "House always wins.", effect: "thud" },
      },
      "core-005": { cast: { voice: "crone", text: "Hoarding is self care." } },
      "core-018": { cast: { effect: "thud" } },
    },
  };
}

function parseError(raw: unknown, catalog: CatalogTypes = CATALOG): string {
  try {
    parseCardAudio(raw, catalog);
  } catch (error) {
    return (error as Error).message;
  }
  return "(no error)";
}

function textError(source: string): string {
  try {
    parseCardAudioText(source, CATALOG);
  } catch (error) {
    return (error as Error).message;
  }
  return "(no error)";
}

beforeEach(() => {
  resetIds();
});

describe("B41 voiceKey, voiceUrl and lineFor", () => {
  it("B41 voiceKey joins a defId and a line, including a defId that has dashes of its own", () => {
    expect(voiceKey("core-004", "play")).toBe("core-004-play");
    expect(voiceKey("core-051-1", "cast")).toBe("core-051-1-cast");
    expect(voiceKey("core-t-sheep", "death")).toBe("core-t-sheep-death");
    expect(voiceKey("core-012", "attack")).toBe("core-012-attack");
  });

  it("B41 voiceUrl serves a key from audio/voice/ under the app's base URL", () => {
    expect(voiceUrl("core-004-play")).toBe(`${import.meta.env.BASE_URL}audio/voice/core-004-play.m4a`);
    expect(voiceUrl("core-051-1-cast")).toBe("/audio/voice/core-051-1-cast.m4a");
  });

  it("B41 lineFor gives a unit's play and death text in the voice each hook names", () => {
    const entry = CARD_AUDIO.cards["core-004"];
    if (entry?.kind !== "unit" || entry.play?.voice === undefined || entry.death?.voice === undefined) {
      throw new Error("core-004 should be a unit with a play and a death line in the shipped file");
    }

    expect(lineFor(CARD_AUDIO, "core-004", "play")).toEqual({ text: entry.play.text, persona: CARD_AUDIO.voices[entry.play.voice] });
    expect(lineFor(CARD_AUDIO, "core-004", "death")).toEqual({ text: entry.death.text, persona: CARD_AUDIO.voices[entry.death.voice] });
  });

  it("B41 lineFor speaks core-016 in a voice of its own, the hustler at rate 170 and pbas 32", () => {
    // docs/polish/2-sound.md, Personas: core-016 is the hustler with `rate 170, pbas 32`, once a
    // per-card override and now the voice `hustler-hitman` (R655).
    const entry = CARD_AUDIO.cards["core-016"];
    if (entry?.kind !== "spell" || entry.cast?.voice === undefined) throw new Error("core-016 should be a spell with a cast line");
    const hustler = CARD_AUDIO.voices.hustler;
    if (hustler === undefined || hustler.backend === "sapi") throw new Error("no say voice hustler");

    const spoken = lineFor(CARD_AUDIO, "core-016", "cast");

    expect(entry.cast.voice).toBe("hustler-hitman");
    expect(spoken?.text).toBe(entry.cast.text);
    expect(spoken?.persona).toEqual({ ...hustler, rate: 170, pbas: 32 });
    expect(hustler.rate).not.toBe(170);
  });

  it("B41 lineFor returns the named voice whole, gain included, on a table of its own", () => {
    const table = parseCardAudio(fixture(), CATALOG);

    expect(lineFor(table, "core-004", "death")).toEqual({ text: "House always wins.", persona: SAY });
    expect(lineFor(table, "core-005", "cast")).toEqual({ text: "Hoarding is self care.", persona: CRONE });
  });

  it("B41 lineFor is null for the sentinel, an unknown id, a hook the card's kind does not have, and an effect alone", () => {
    expect(lineFor(CARD_AUDIO, HIDDEN_DEF_ID, "play")).toBeNull();
    expect(lineFor(CARD_AUDIO, HIDDEN_DEF_ID, "cast")).toBeNull();
    expect(lineFor(CARD_AUDIO, "core-999", "play")).toBeNull();
    expect(lineFor(CARD_AUDIO, "toString", "play")).toBeNull();
    // A unit has no cast line; a spell and a trap have neither a play nor a death line.
    expect(lineFor(CARD_AUDIO, "core-004", "cast")).toBeNull();
    expect(lineFor(CARD_AUDIO, "core-005", "play")).toBeNull();
    expect(lineFor(CARD_AUDIO, "core-005", "death")).toBeNull();
    expect(lineFor(CARD_AUDIO, "core-041", "death")).toBeNull();
    expect(lineFor(CARD_AUDIO, "core-041", "cast")).not.toBeNull();
    // core-066's attack hook is an effect and nothing else.
    expect(hookFor(CARD_AUDIO, "core-066", "attack")).toEqual({ effect: "rumble" });
    expect(lineFor(CARD_AUDIO, "core-066", "attack")).toBeNull();
  });
});

describe("B42 parseCardAudio", () => {
  it("B42 accepts the shipped card-audio.json5, adding only each card's kind and each effect's default pitch and gain", () => {
    const raw = shippedRaw();
    const parsed = parseCardAudioText(shippedText(), SHIPPED_CATALOG);
    const kindOf = (type: string): string => (type === "Unit" ? "unit" : type.endsWith("Spell") ? "spell" : "trap");

    expect(parsed.voices).toEqual(raw.voices);
    expect(parsed.effects).toEqual(
      Object.fromEntries(Object.entries(raw.effects).map(([name, effect]) => [name, { pitch: 1, gain: 1, ...effect }])),
    );
    expect(parsed.cards).toEqual(
      Object.fromEntries(
        Object.entries(raw.cards).map(([id, entry]) => [id, { kind: kindOf(SHIPPED_CATALOG[id]?.type ?? ""), ...entry }]),
      ),
    );
    expect(parseCardAudio(raw, SHIPPED_CATALOG)).toEqual(parsed);
  });

  it("B42 names the path and the problem of a malformed table", () => {
    expect(parseError([])).toBe("card-audio.json5: (root): must be an object");

    // The old file's version key, and its personas section, are a layout error now.
    const versioned = { version: 1, ...fixture() };
    expect(parseError(versioned)).toBe(
      "card-audio.json5: (root): must hold voices, effects, cards, in that order (found version, voices, effects, cards)",
    );

    const unknownVoice = fixture();
    unknownVoice.cards["core-004"] = { ...unknownVoice.cards["core-004"], play: { voice: "nobody", text: "Hi." } };
    expect(parseError(unknownVoice)).toBe('card-audio.json5: cards.core-004.play.voice: unknown voice "nobody"');

    const spellWithPlay = fixture();
    spellWithPlay.cards["core-005"] = { ...spellWithPlay.cards["core-005"], play: { voice: "crone", text: "Hi." } };
    expect(parseError(spellWithPlay)).toBe("card-audio.json5: cards.core-005.play: a spell has no play hook");

    // The old file's per-card kind is no hook.
    const kinded = fixture();
    kinded.cards["core-004"] = { kind: "unit", ...kinded.cards["core-004"] };
    expect(parseError(kinded)).toBe(
      "card-audio.json5: cards.core-004.kind: unknown hook (expected one of play, attack, death, cast)",
    );

    const slow = fixture();
    slow.voices.hustler = { ...SAY, rate: 12 };
    expect(parseError(slow)).toBe("card-audio.json5: voices.hustler.rate: must be a number from 90 to 360");

    const loud = fixture();
    loud.voices.hustler = { ...SAY, web: { pitch: 3, rate: 1 } };
    expect(parseError(loud)).toBe("card-audio.json5: voices.hustler.web.pitch: must be a number from 0 to 2");

    const sentinel = fixture();
    sentinel.cards[HIDDEN_DEF_ID] = { cast: { voice: "crone", text: "Guess who." } };
    expect(parseError(sentinel)).toBe(`card-audio.json5: cards.${HIDDEN_DEF_ID}: the hidden sentinel cannot have sounds`);
  });

  it("B42 leaves word limits and the lines each card must have to the content test", () => {
    const wordy = fixture();
    const long = "one two three four five six seven eight nine ten eleven twelve";
    wordy.cards["core-005"] = { cast: { voice: "crone", text: long } };
    // A unit with no death line, and a catalog card with no entry at all (core-041).
    wordy.cards["core-004"] = { play: { voice: "hustler", text: "Hi." } };

    const table = parseCardAudio(wordy, CATALOG);
    expect(table.cards["core-005"]).toEqual({ kind: "spell", cast: { voice: "crone", text: long } });
    expect(table.cards["core-004"]).toEqual({ kind: "unit", play: { voice: "hustler", text: "Hi." } });
    expect(table.cards["core-041"]).toBeUndefined();
  });
});

describe("R501 SAPI voices", () => {
  const sapi = {
    backend: "sapi",
    voice: "Microsoft Zira Desktop",
    rate: 10,
    semitones: -3,
    filter: "lowpass=f=3500",
    web: { pitch: 0.8, rate: 1.1 },
  };

  function tableWithSapi(): RawTable {
    const table = fixture();
    table.voices["test-sapi"] = { ...sapi };
    table.cards["core-005"] = { cast: { voice: "test-sapi", text: "A voice from another machine." } };
    return table;
  }

  it("R501 parses a SAPI voice and speaks its line with it, as written", () => {
    const table = parseCardAudio(tableWithSapi(), CATALOG);
    expect(table.voices["test-sapi"]).toEqual(sapi);
    expect(lineFor(table, "core-005", "cast")).toEqual({ text: "A voice from another machine.", persona: sapi });
  });

  it("R501 names the path and the problem of a malformed SAPI voice, a `say` field on one included", () => {
    const noVoice = tableWithSapi();
    noVoice.voices["test-sapi"] = { ...sapi, voice: "" };
    expect(parseError(noVoice)).toBe("card-audio.json5: voices.test-sapi.voice: must be a non-empty string");

    const tooHigh = tableWithSapi();
    tooHigh.voices["test-sapi"] = { ...sapi, semitones: 13 };
    expect(parseError(tooHigh)).toBe("card-audio.json5: voices.test-sapi.semitones: must be a number from -12 to 12");

    const noFilter = tableWithSapi();
    noFilter.voices["test-sapi"] = { ...sapi, filter: 3 };
    expect(parseError(noFilter)).toBe("card-audio.json5: voices.test-sapi.filter: must be a string");

    const unknownBackend = tableWithSapi();
    unknownBackend.voices["test-sapi"] = { ...sapi, backend: "espeak" };
    expect(parseError(unknownBackend)).toBe('card-audio.json5: voices.test-sapi.backend: must be "say" or "sapi"');

    // A `say` voice's pitch fields would never reach a SAPI render, so a SAPI voice may not carry them.
    for (const field of ["say", "pbas", "pmod"]) {
      const sayField = tableWithSapi();
      sayField.voices["test-sapi"] = { ...sapi, [field]: field === "say" ? "Fred" : 40 };
      expect(parseError(sayField)).toBe(
        `card-audio.json5: voices.test-sapi.${field}: unknown field (expected backend, voice, rate, semitones, filter, web, gain)`,
      );
    }
  });
});

describe("R655 parseCardAudio's shape errors", () => {
  /** Expect `edit` of a fresh fixture to throw exactly `card-audio.json5: <message>`. */
  function expectError(edit: (table: RawTable) => void, message: string): void {
    const table = fixture();
    edit(table);
    expect(parseError(table)).toBe(`card-audio.json5: ${message}`);
  }

  it("R655 accepts the fixture as written", () => {
    expect(parseError(fixture())).toBe("(no error)");
  });

  it("R655 names a voice the bank does not hold", () => {
    expectError((t) => {
      t.cards["core-004"] = { play: { voice: "x", text: "Hi." } };
    }, 'cards.core-004.play.voice: unknown voice "x"');
  });

  it("R655 names an effect the bank does not hold", () => {
    expectError((t) => {
      t.cards["core-004"] = { attack: { effect: "bang" } };
    }, 'cards.core-004.attack.effect: unknown effect "bang"');
  });

  it("R655 names an unknown hook", () => {
    expectError((t) => {
      t.cards["core-004"] = { jump: { effect: "zip" } };
    }, "cards.core-004.jump: unknown hook (expected one of play, attack, death, cast)");
  });

  it("R655 names a hook the card's kind may not carry", () => {
    expectError((t) => {
      t.cards["core-004"] = { cast: { effect: "zip" } };
    }, "cards.core-004.cast: a unit has no cast hook");
    expectError((t) => {
      t.cards["core-005"] = { attack: { effect: "zip" } };
    }, "cards.core-005.attack: a spell has no attack hook");
    expectError((t) => {
      t.cards["core-018"] = { death: { effect: "zip" } };
    }, "cards.core-018.death: a trap has no death hook");
  });

  it("R655 names a voice with no text, a text with no voice, and a hook with neither a line nor an effect", () => {
    expectError((t) => {
      t.cards["core-004"] = { play: { voice: "hustler" } };
    }, "cards.core-004.play.text: must be a non-empty string");
    expectError((t) => {
      t.cards["core-004"] = { play: { voice: "hustler", text: "  " } };
    }, "cards.core-004.play.text: must be a non-empty string");
    expectError((t) => {
      t.cards["core-004"] = { play: { text: "Hi." } };
    }, "cards.core-004.play.voice: a text needs the voice that speaks it");
    expectError((t) => {
      t.cards["core-004"] = { play: { text: "Hi.", effect: "zip" } };
    }, "cards.core-004.play.voice: a text needs the voice that speaks it");
    expectError((t) => {
      t.cards["core-004"] = { play: {} };
    }, "cards.core-004.play: needs a voice and a text, an effect, or both");
  });

  it("R655 names a card id the catalog does not hold, and a catalog type with no hooks", () => {
    expectError((t) => {
      t.cards["core-999"] = { cast: { effect: "zip" } };
    }, "cards.core-999: not a card in the catalog");
    expectError((t) => {
      t.cards = { ...t.cards, toString: { cast: { effect: "zip" } } };
    }, "cards.toString: not a card in the catalog");
    const odd = fixture();
    odd.cards = { "x-1": { play: { effect: "zip" } } };
    expect(parseError(odd, { "x-1": { type: "Hero" } })).toBe('card-audio.json5: cards.x-1: the catalog type "Hero" has no hooks');
  });

  it("R655 names an effect whose sfx is not a recipe, and a pitch or gain out of range", () => {
    expectError((t) => {
      t.effects.thud = { sfx: "boom" };
    }, 'effects.thud.sfx: unknown recipe "boom" (expected one of sfx.ts SFX_IDS)');
    expectError((t) => {
      t.effects.thud = { sfx: "impact", pitch: 5 };
    }, "effects.thud.pitch: must be a number from 0.25 to 4");
    expectError((t) => {
      t.effects.thud = { sfx: "impact", pitch: 0.2 };
    }, "effects.thud.pitch: must be a number from 0.25 to 4");
    expectError((t) => {
      t.effects.thud = { sfx: "impact", gain: 3 };
    }, "effects.thud.gain: must be a number from 0 to 2");
    expectError((t) => {
      t.effects.thud = { sfx: "summon", params: { timbre: "robot" } };
    }, 'effects.thud.params.timbre: unknown timbre "robot" (expected one of human, felinor, ky, cn, fruit, chaos, quickdraw, token, field, book, pancake, ai)');
  });

  it("R655 names a field no part of the file allows", () => {
    expectError((t) => {
      t.cards["core-004"] = { play: { voice: "hustler", text: "Hi.", rate: 200 } };
    }, "cards.core-004.play.rate: unknown field (expected voice, text, effect)");
    expectError((t) => {
      t.effects.thud = { sfx: "impact", volume: 1 };
    }, "effects.thud.volume: unknown field (expected sfx, pitch, gain, params)");
    expectError((t) => {
      t.effects.thud = { sfx: "impact", params: { loud: true } };
    }, "effects.thud.params.loud: unknown field (expected amount, mine, timbre, mythic, urgent, release)");
    expectError((t) => {
      t.voices.hustler = { ...SAY, semitones: 2 };
    }, "voices.hustler.semitones: unknown field (expected backend, say, rate, pbas, pmod, web, gain)");
  });

  it("R655 names the sections when they are out of order, missing or joined by another", () => {
    const { voices, effects, cards } = fixture();
    expect(parseError({ cards, voices, effects })).toBe(
      "card-audio.json5: (root): must hold voices, effects, cards, in that order (found cards, voices, effects)",
    );
    expect(parseError({ voices, cards })).toBe(
      "card-audio.json5: (root): must hold voices, effects, cards, in that order (found voices, cards)",
    );
    expect(parseError({ personas: voices, effects, cards })).toBe(
      "card-audio.json5: (root): must hold voices, effects, cards, in that order (found personas, effects, cards)",
    );
    expect(parseError({})).toBe("card-audio.json5: (root): must hold voices, effects, cards, in that order (found nothing)");
  });

  it("R655 reports a JSON5 syntax error with its line and column", () => {
    expect(textError("{ voices: {")).toBe("card-audio.json5: (syntax): JSON5: invalid end of input at 1:12");
    expect(textError("{\n  voices: {},\n  effects: {}\n  cards: {},\n}\n")).toBe(
      "card-audio.json5: (syntax): JSON5: invalid character 'c' at 4:3",
    );
  });

  it("R655 refuses a key written twice in one object, which JSON5 alone would let the last of win, naming its path", () => {
    const source = (cards: string, voices = 'crone: { say: "Grandma (English (US))", rate: 170, pbas: 50, pmod: 40, web: { pitch: 1.1, rate: 0.9 } },'): string =>
      `{\n  voices: { ${voices} },\n  effects: { zip: { sfx: "whoosh" } },\n  cards: {\n${cards}\n  },\n}\n`;
    const stockpile = '    "core-005": { // Stockpile\n      cast: { voice: "crone", text: "Hoarding is self care." },\n    },';
    expect(textError(source(stockpile)), "the file once").toBe("(no error)");

    expect(textError(source(`${stockpile}\n${stockpile}`))).toBe(
      "card-audio.json5: cards.core-005: written twice in one object (only the last would count)",
    );
    expect(textError(source('    "core-005": { cast: { effect: "zip" }, "cast": { effect: "zip" } },'))).toBe(
      "card-audio.json5: cards.core-005.cast: written twice in one object (only the last would count)",
    );
    expect(textError(source(stockpile, `crone: { say: "Albert", rate: 170, pbas: 40, pmod: 30, web: { pitch: 1, rate: 1 } }, 'crone': { say: "Albert", rate: 170, pbas: 40, pmod: 30, web: { pitch: 1, rate: 1 } },`))).toBe(
      "card-audio.json5: voices.crone: written twice in one object (only the last would count)",
    );
    // The same key in two different objects, in a string, or in a comment is no duplicate.
    const twoObjects = '    "core-005": { // cast: { "core-005": 1 }\n      cast: { voice: "crone", text: "cast: core-005: crone" },\n    },\n    "core-018": { cast: { effect: "zip" } },';
    expect(textError(source(twoObjects))).toBe("(no error)");
  });

  it("R655 parses JSON5's comments, unquoted keys and trailing commas", () => {
    const source = [
      "// A header comment.",
      "{",
      "  voices: { crone: { say: \"Grandma (English (US))\", rate: 170, pbas: 50, pmod: 40, web: { pitch: 1.1, rate: 0.9 }, }, },",
      "  effects: {",
      "    /* a block comment */ zip: { sfx: \"whoosh\", },",
      "  },",
      "  cards: {",
      "    \"core-005\": { // Stockpile",
      "      cast: { voice: \"crone\", text: \"Hoarding is self care.\", effect: \"zip\", },",
      "    },",
      "  },",
      "}",
      "",
    ].join("\n");

    expect(parseCardAudioText(source, CATALOG)).toEqual({
      voices: { crone: CRONE },
      effects: { zip: { sfx: "whoosh", pitch: 1, gain: 1 } },
      cards: { "core-005": { kind: "spell", cast: { voice: "crone", text: "Hoarding is self care.", effect: "zip" } } },
      emotes: {},
    });
  });

  it("R644 reads the portraits' emote lines through the voices bank, and fails naming the path of a bad one", () => {
    const table = parseCardAudio(
      {
        voices: { hustler: SAY },
        effects: {},
        cards: {},
        emotes: {
          gary: {
            persona: "hustler",
            greetings: "Hey there, friend!",
            wellPlayed: "Well played.",
            oops: "Oops.",
            thanks: "Thanks!",
            threaten: "Double or nothing.",
          },
        },
      },
      {},
    );
    expect(emoteLineFor(table, "gary", "thanks")).toEqual({ text: "Thanks!", persona: SAY });
    expect(emoteLineFor(table, "emote-gary", "thanks")?.text).toBe("Thanks!");
    expect(lineFor(table, "emote-gary", "thanks")?.text).toBe("Thanks!");
    expect(emoteVoiceDef("gary")).toBe("emote-gary");

    const unknownVoice = fixture();
    (unknownVoice as unknown as Record<string, Record<string, unknown>>).emotes = {
      gary: { persona: "nobody", greetings: "Hi.", wellPlayed: "Wp.", oops: "Oops.", thanks: "Thanks.", threaten: "Boo." },
    };
    expect(parseError(unknownVoice)).toBe('card-audio.json5: emotes.gary.persona: unknown voice "nobody"');

    const missingLine = fixture();
    (missingLine as unknown as Record<string, Record<string, unknown>>).emotes = {
      gary: { persona: "hustler", greetings: "Hi.", wellPlayed: "Wp.", oops: "Oops.", threaten: "Boo." },
    };
    expect(parseError(missingLine)).toBe("card-audio.json5: emotes.gary.thanks: must be a non-empty string");

    const misplaced = fixture();
    const { voices, effects, cards } = misplaced;
    const emotes = { gary: {} };
    expect(parseError({ voices, effects, emotes, cards })).toBe(
      "card-audio.json5: (root): must hold voices, effects, cards, in that order (found voices, effects, emotes, cards)",
    );
  });

  it("R655 derives each card's kind from its catalog type: Unit a unit, Spell and Field Spell a spell, Trap and Field Trap a trap", () => {
    const table = parseCardAudio(
      {
        voices: { crone: CRONE },
        effects: { zip: { sfx: "whoosh" } },
        cards: {
          "core-004": { attack: { effect: "zip" } },
          "core-005": { cast: { effect: "zip" } },
          "core-006": { cast: { effect: "zip" } },
          "core-018": { cast: { effect: "zip" } },
          "core-041": { cast: { effect: "zip" } },
        },
      },
      CATALOG,
    );
    expect(Object.fromEntries(Object.entries(table.cards).map(([id, entry]) => [id, entry.kind]))).toEqual({
      "core-004": "unit",
      "core-005": "spell",
      "core-006": "spell",
      "core-018": "trap",
      "core-041": "trap",
    });
  });

  it("R655 gives an effect pitch 1 and gain 1 unless it says otherwise, and keeps its params", () => {
    const table = parseCardAudio(fixture(), CATALOG);
    expect(table.effects).toEqual({
      thud: { sfx: "impact", pitch: 0.6, gain: 1, params: { amount: 8 } },
      zip: { sfx: "whoosh", pitch: 1, gain: 1 },
    });
  });
});

describe("R655 hookFor, lineFor and effectFor", () => {
  const table = parseCardAudio(fixture(), CATALOG);

  it("R655 reads a hook that is a line alone", () => {
    expect(hookFor(table, "core-004", "play")).toEqual({ voice: "hustler", text: "Double or nothing, baby!" });
    expect(lineFor(table, "core-004", "play")).toEqual({ text: "Double or nothing, baby!", persona: SAY });
    expect(effectFor(table, "core-004", "play")).toBeNull();
  });

  it("R655 reads a hook that is an effect alone", () => {
    expect(hookFor(table, "core-004", "attack")).toEqual({ effect: "zip" });
    expect(lineFor(table, "core-004", "attack")).toBeNull();
    expect(effectFor(table, "core-004", "attack")).toEqual({ name: "zip", effect: { sfx: "whoosh", pitch: 1, gain: 1 } });
    expect(effectFor(table, "core-018", "cast")).toEqual({
      name: "thud",
      effect: { sfx: "impact", pitch: 0.6, gain: 1, params: { amount: 8 } },
    });
  });

  it("R655 reads a hook that is both a line and an effect", () => {
    expect(hookFor(table, "core-004", "death")).toEqual({ voice: "hustler", text: "House always wins.", effect: "thud" });
    expect(lineFor(table, "core-004", "death")).toEqual({ text: "House always wins.", persona: SAY });
    expect(effectFor(table, "core-004", "death")?.name).toBe("thud");
  });

  it("R655 reads nothing for the sentinel, an unknown id or a hook the card does not carry", () => {
    for (const hook of ["play", "attack", "death", "cast"] as const) {
      for (const defId of [HIDDEN_DEF_ID, "core-999", "toString", "constructor"]) {
        expect(hookFor(table, defId, hook), `${defId} ${hook}`).toBeNull();
        expect(lineFor(table, defId, hook), `${defId} ${hook}`).toBeNull();
        expect(effectFor(table, defId, hook), `${defId} ${hook}`).toBeNull();
      }
    }
    expect(entryFor(table, HIDDEN_DEF_ID)).toBeNull();
    expect(hookFor(table, "core-005", "attack")).toBeNull();
    expect(effectFor(table, "core-005", "cast")).toBeNull();
    // A card the catalog holds but the file has no entry for.
    expect(entryFor(table, "core-041")).toBeNull();
  });

  it("R655 CARD_AUDIO is the shipped card-audio.json5, parsed against the shipped catalog", () => {
    expect(CARD_AUDIO).toEqual(parseCardAudioText(shippedText(), SHIPPED_CATALOG));
    expect(Object.keys(CARD_AUDIO.cards)).toEqual(Object.keys(SHIPPED_CATALOG));
    expect(effectFor(CARD_AUDIO, "core-066", "attack")?.name).toBe("rumble");
  });
});

describe("B43 voiceKeysForView", () => {
  function viewWith(): PlayerView {
    return baseView({
      you: emptySide("p1", {
        hand: [
          card({ defId: "core-004" }), // unit: play
          card({ defId: "core-005" }), // spell: cast
          card({ defId: "core-018" }), // Field Trap: nothing, a set never speaks (R203)
          card({ defId: "core-004" }), // a second copy: deduped
          card({ defId: "core-006" }), // Field Spell: cast
        ],
        units: [unit("p1", { defId: "core-012" }), null, unit("p1", { defId: "core-t-sheep" }), null, null],
        backrow: [
          faceUpBackrow("p1", { defId: "core-041", type: "Trap" }), // own face-up trap: cast
          faceUpBackrow("p1", { defId: "core-006", type: "Field Spell" }), // not a trap: nothing more
          faceDownBackrow, // cannot be named
          faceUpBackrow("p1", { defId: "core-071", type: "Field Trap" }), // own face-up trap: cast
          null,
        ],
      }),
      opponent: emptySide("p2", {
        hand: { count: 5 },
        units: [unit("p2", { defId: "core-004" }), unit("p2", { defId: HIDDEN_DEF_ID }), unit("p2", { defId: "core-012" }), null, null],
        backrow: [faceUpBackrow("p2", { defId: "core-060", type: "Trap" }), faceDownBackrow, null, null, null],
      }),
    });
  }

  it("B43 lists the hand, then every unit's death line, then the viewer's own face-up traps, each key once", () => {
    // The shipped file has no attack line yet: core-012's attack hook is an effect, so it adds no key.
    expect(hookFor(CARD_AUDIO, "core-012", "attack")).toEqual({ effect: "catChirp" });
    expect(voiceKeysForView(viewWith(), CARD_AUDIO)).toEqual([
      "core-004-play",
      "core-005-cast",
      "core-006-cast",
      "core-012-death",
      "core-t-sheep-death",
      "core-004-death",
      "core-041-cast",
      "core-071-cast",
    ]);
  });

  it("B43 an empty board and hand want nothing preloaded", () => {
    expect(voiceKeysForView(baseView(), CARD_AUDIO)).toEqual([]);
  });

  it("B43 reads only the table it is given", () => {
    const table: CardAudioTable = { voices: CARD_AUDIO.voices, effects: CARD_AUDIO.effects, cards: {}, emotes: {} };
    expect(voiceKeysForView(viewWith(), table)).toEqual([]);
  });

  it("R655 adds the viewer's own units' attack lines after the death lines and before the traps, and skips a hook with no line", () => {
    const catalog: CatalogTypes = {
      "x-unit": { type: "Unit" },
      "x-mute": { type: "Unit" },
      "x-spell": { type: "Spell" },
      "x-trap": { type: "Trap" },
    };
    const table = parseCardAudio(
      {
        voices: { hustler: SAY },
        effects: { zip: { sfx: "whoosh" } },
        cards: {
          // Lines on every hook, the attack one with an effect too.
          "x-unit": {
            play: { voice: "hustler", text: "In." },
            attack: { voice: "hustler", text: "Get him!", effect: "zip" },
            death: { voice: "hustler", text: "Out." },
          },
          // Effects alone: no file to preload.
          "x-mute": { play: { effect: "zip" }, attack: { effect: "zip" }, death: { effect: "zip" } },
          "x-spell": { cast: { effect: "zip" } },
          "x-trap": { cast: { voice: "hustler", text: "Gotcha." } },
        },
      },
      catalog,
    );
    const view = baseView({
      you: emptySide("p1", {
        hand: [card({ defId: "x-mute" }), card({ defId: "x-spell" }), card({ defId: "x-unit" })],
        units: [unit("p1", { defId: "x-mute" }), unit("p1", { defId: "x-unit" }), null, null, null],
        backrow: [faceUpBackrow("p1", { defId: "x-trap", type: "Trap" }), null, null, null, null],
      }),
      opponent: emptySide("p2", {
        hand: { count: 2 },
        // The other seat's unit dies in view, but the viewer never picks it up.
        units: [unit("p2", { defId: "x-unit" }), null, null, null, null],
      }),
    });

    expect(voiceKeysForView(view, table)).toEqual(["x-unit-play", "x-unit-death", "x-unit-attack", "x-trap-cast"]);
  });

  it("R655 adds no attack line for a unit on the other seat's board alone", () => {
    const table = parseCardAudio(
      {
        voices: { hustler: SAY },
        effects: {},
        cards: {
          "x-unit": {
            play: { voice: "hustler", text: "In." },
            attack: { voice: "hustler", text: "Get him!" },
            death: { voice: "hustler", text: "Out." },
          },
        },
      },
      { "x-unit": { type: "Unit" } },
    );
    const view = baseView({
      opponent: emptySide("p2", { hand: { count: 2 }, units: [unit("p2", { defId: "x-unit" }), null, null, null, null] }),
    });

    expect(voiceKeysForView(view, table)).toEqual(["x-unit-death"]);
  });
});

describe("card-audio.json5 layout (docs/polish/2-sound.md; R655)", () => {
  it("keeps the voices sorted by name, the cards in catalog order, and a trailing newline", () => {
    const text = shippedText();
    const table = JSON5.parse<RawTable>(text);

    expect(Object.keys(table.voices)).toEqual(Object.keys(table.voices).sort());
    expect(Object.keys(table.cards)).toEqual(Object.keys(SHIPPED_CATALOG));
    expect(text.endsWith("}\n"), "the file ends with its closing brace and a newline").toBe(true);
  });
});
