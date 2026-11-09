// R2784000: every set's effects table (fx/cards/<set>.json5) held to the catalog: every card, tokens
// included, in catalog order, one line each under its own name, Stinkyscape split in two at 091; and
// the loader's errors, each naming its file and card. R502, R670: the seven former `CARD_FX` cards are
// `{ bespoke }` entries but #100 Ceaseless Void, whose `{ setPiece }` lands it with its collapse (R2800000).
// R2787000: an entry's `aura`, and the ten Field Spells that wear one. R2794000: a Unit's `entrance` beside
// its hooks (Nature Titan's, beside its Cry). R3332000: the frame, an `entrance` beside any entry of a Unit
// or a Field Spell or alone, a `takeover` beside a Mythic's and a `signature` naming its first beat, and
// the eight marquee cards' entrances and looks folded into it. R3400000: a bespoke entry's aura, a Unit's
// `tile` aura, and Meditative+'s and Training Grounds' takeover looks and entrances among them. R3389000: a
// signature look a signature's alone, and In Too Deep's "abyss". R3395001: Stinkyscape 001 to 090's signature
// entrances, its five Mythics' own looks and its Field Spells' three signature looks.

import { describe, expect, it } from "vitest";

import { CATALOG } from "../../wire/cards.ts";
import { AURA_LOOK_NAMES, AURA_SCOPES, AURA_SIGNATURE_LOOK_NAMES } from "../aura/looks.ts";
import { entranceKeyOf } from "../cardFx.ts";
import { ENTRANCES } from "../entrances.ts";
import { FX_PALETTES } from "../flavors.ts";
import { FX_MOTIF_NAMES } from "../motifs/names.ts";
import { SET_PIECE_KEYS } from "../setPieceKeys.ts";
import { takeoverLookOf } from "../takeover.ts";
import { FX_CARD_FILES, FX_CARD_TABLE, FX_STINKYSCAPE_SPLIT, OWN_TAKEOVER_LOOKS, entryAura, isSignature, parseFxCardTable, type FxCardFile } from "./tables.ts";

const IDS = Object.keys(CATALOG);
const SPLIT = IDS.indexOf(FX_STINKYSCAPE_SPLIT);

/** The ids `file` must hold, in catalog order. */
function expectedIds(file: FxCardFile): string[] {
  return IDS.filter((id, i) => {
    if (CATALOG[id]?.set !== file.set) return false;
    if (file.file === "stinkyscape-1.json5") return i < SPLIT;
    if (file.file === "stinkyscape-2.json5") return i >= SPLIT;
    return true;
  });
}

const ENTRY_LINE = /^ {2}"([^"]+)": (.+), \/\/ (.+)$/;

/** Each entry line's id, entry and name comment; any other line that is not a comment or a brace fails. */
function linesOf(file: FxCardFile): { id: string; entry: string; name: string }[] {
  return file.source.split("\n").flatMap((line, n) => {
    if (line.trim() === "" || line.startsWith("//") || line === "{" || line === "}") return [];
    const match = ENTRY_LINE.exec(line);
    if (match === null) throw new Error(`fx/cards/${file.file}:${String(n + 1)} is not one entry with its name: ${line}`);
    return [{ id: match[1] ?? "", entry: match[2] ?? "", name: match[3] ?? "" }];
  });
}

const nameOf = (id: string): string => (CATALOG[id]?.name ?? "").replace(/\s+/g, " ").trim();

describe("R2784000 the per-set effects tables", () => {
  it("R2784000 every set's table holds its catalog cards in catalog order, tokens included", () => {
    const sets = new Set(IDS.map((id) => CATALOG[id]?.set));
    expect(new Set(FX_CARD_FILES.map((file) => file.set))).toEqual(sets);
    for (const file of FX_CARD_FILES) {
      expect([...parseFxCardTable(file.file, file.source).keys()], file.file).toEqual(expectedIds(file));
    }
    expect([...FX_CARD_TABLE.keys()]).toEqual(IDS);
  });

  it("R2784000 Stinkyscape splits at 091: the first file ends at 090, the second holds 091 to the end and every token after it", () => {
    const [first, second] = ["stinkyscape-1.json5", "stinkyscape-2.json5"].map((name) => FX_CARD_FILES.find((file) => file.file === name));
    if (first === undefined || second === undefined) throw new Error("Stinkyscape's two files are missing");
    const one = expectedIds(first);
    const two = expectedIds(second);
    expect(one[0]).toBe("stinkyscape-001");
    expect(one.filter((id) => /^stinkyscape-\d{3}$/.test(id)).at(-1)).toBe("stinkyscape-090");
    expect(two[0]).toBe(FX_STINKYSCAPE_SPLIT);
    const tokens = IDS.filter((id) => id.startsWith("stinkyscape-t-"));
    expect(tokens.length).toBeGreaterThan(0);
    for (const token of tokens) expect(two, token).toContain(token);
  });

  it("R2784000 each entry is one line, its comment its catalog name, and no card twice", () => {
    for (const file of FX_CARD_FILES) {
      const lines = linesOf(file);
      expect(lines.map((line) => line.id), file.file).toEqual(expectedIds(file));
      for (const line of lines) expect(line.name, `${file.file} ${line.id}, run cargo jackioh names`).toBe(nameOf(line.id));
    }
  });

  it("R2784000 an unknown hook, motif, palette or recipe, or a hook on the wrong kind of card, fails, naming file and id", () => {
    const fails = (body: string): void => {
      expect(() => parseFxCardTable("test.json5", `{ ${body} }`)).toThrow(/^fx\/cards\/test\.json5: /);
    };
    const says = (body: string, problem: RegExp): void => {
      expect(() => parseFxCardTable("test.json5", `{ ${body} }`)).toThrow(problem);
    };
    says(`"core-035": { spell: { motif: "bolt" } }`, /^fx\/cards\/test\.json5: core-035: unknown hook "spell"$/);
    says(`"core-035": { cast: { motif: "zap" } }`, /core-035: unknown motif "zap" in cast/);
    says(`"core-035": { cast: { motif: "bolt", palette: "Plaid" } }`, /core-035: unknown palette "Plaid" in cast/);
    says(`"core-035": { cast: { motif: "bolt", row: "drop" } }`, /core-035: row in cast/);
    says(`"core-035": { cast: { motif: "bolt", speed: 2 } }`, /core-035: unknown field "speed" in cast/);
    says(`"core-035": { cry: { motif: "bolt" } }`, /core-035: cry is a Unit's play; this card is a Spell/);
    says(`"classic-071": { cast: { motif: "lance" } }`, /classic-071: cast is a non-Unit's play/);
    says(`"core-021": { bespoke: "manaSmash" }`, /core-021: unknown bespoke "manaSmash"/);
    says(`"core-021": { bespoke: "manaCrack", cast: { motif: "bolt" } }`, /core-021: a bespoke entry holds nothing else/);
    says(`"core-035": {}`, /core-035: names no hook/);
    says(`"core-035": "pending"`, /core-035: a string entry must be "none: <reason>"/);
    says(`"core-035": "none:"`, /core-035: a string entry/);
    says(`"core-999": "none: pending"`, /core-999: not a card in the catalog/);
    fails(`"core-035": 7`);
    fails(`"core-035": { cast: "bolt" }`);
    expect(() => parseFxCardTable("test.json5", "{ broken")).toThrow(/^fx\/cards\/test\.json5: \(syntax\): /);
  });

  it("R2800000 a setPiece entry names a known set piece and nothing else", () => {
    for (const key of SET_PIECE_KEYS) {
      expect(parseFxCardTable("test.json5", `{ "core-088": { setPiece: "${key}" } }`).get("core-088"), key).toEqual({ kind: "setPiece", key });
    }
    const says = (body: string, problem: RegExp): void => {
      expect(() => parseFxCardTable("test.json5", `{ ${body} }`)).toThrow(problem);
    };
    says(`"core-088": { setPiece: "maelstrom" }`, /^fx\/cards\/test\.json5: core-088: unknown set piece "maelstrom"$/);
    says(`"core-088": { setPiece: 7 }`, /core-088: unknown set piece "7"/);
    says(`"core-088": { setPiece: "nether", cast: { motif: "wipe" } }`, /core-088: a setPiece entry holds nothing else/);
    says(`"core-088": { setPiece: "nether", bespoke: "voidCollapse" }`, /core-088: a (bespoke|setPiece) entry holds nothing else/);
  });

  it('R2784000 #2792 a palette may name a sub-look as <Flavor>/<sub-look>; an unknown one fails, naming file and id', () => {
    for (const palette of FX_PALETTES) {
      const table = parseFxCardTable("test.json5", `{ "core-035": { cast: { motif: "bolt", palette: "${palette}" } } }`);
      expect(table.get("core-035"), palette).toEqual({ kind: "hooks", hooks: { cast: { motif: "bolt", palette } } });
    }
    for (const bad of ["Tech/lazer", "Tech/", "/laser", "Tech/laser/x", "Brute/laser", "Plaid", "tech/laser", "toString", "Tech/toString"]) {
      expect(() => parseFxCardTable("test.json5", `{ "core-035": { cast: { motif: "bolt", palette: "${bad}" } } }`)).toThrow(
        `fx/cards/test.json5: core-035: unknown palette "${bad}" in cast`,
      );
    }
  });

  it("R2784000 every motif name the design doc gives is accepted, drawn yet or not, with any flavor's palette", () => {
    for (const motif of FX_MOTIF_NAMES) {
      const table = parseFxCardTable("test.json5", `{ "core-035": { cast: { motif: "${motif}", palette: "Tech", row: "keep" } } }`);
      expect(table.get("core-035"), motif).toEqual({ kind: "hooks", hooks: { cast: { motif, palette: "Tech", row: "keep" } } });
    }
    expect(FX_MOTIF_NAMES).toContain("aura");
    const hooks = parseFxCardTable("test.json5", `{ "classic-071": { cry: { motif: "lance" }, death: { motif: "nova" }, trigger: { motif: "bolt" }, activate: { motif: "aura" } } }`);
    expect(hooks.get("classic-071")?.kind).toBe("hooks");
  });

  it("R2784000 R502 R670 R2800000 R2794000 R3332000 the seven former CARD_FX cards keep their recipes: three bespoke, two entrances alone, Nature Titan's entrance beside its Cry, and the Void's set piece lands with its collapse", () => {
    const bespoke = {
      "core-021": { kind: "bespoke", key: "manaCrack" },
      "core-027": { kind: "bespoke", key: "bloodDrain" },
      "classicplus-024": { kind: "bespoke", key: "crushingWalls" },
    };
    for (const [defId, entry] of Object.entries(bespoke)) expect(FX_CARD_TABLE.get(defId), defId).toEqual(entry);
    // R3332000: an entrance stands beside an entry's kind; R3389001 the three Classic marquees' grew into
    // signatures, each its entrance beside a recipe of its own.
    expect(FX_CARD_TABLE.get("classic-056")).toMatchObject({ kind: "bespoke", key: "tyrantCommand", entrance: "tyrantSigil", signature: true });
    expect(FX_CARD_TABLE.get("classic-080")).toMatchObject({ kind: "bespoke", key: "craterLock", entrance: "bigBoom", signature: true });
    expect(FX_CARD_TABLE.get("classic-045")).toMatchObject({ kind: "bespoke", key: "titanLeaf", entrance: "titanBloom", signature: true });
    expect(entranceKeyOf("classic-045")).toBe("titanBloom");
    expect(FX_CARD_TABLE.get("core-100")).toEqual({ kind: "setPiece", key: "voidFold", entrance: "voidCollapse", takeover: "void" });
    expect(entranceKeyOf("core-100")).toBe("voidCollapse");
    const entrances = ["classic-056", "classic-080", "classic-045", "core-100"].map((defId) => entranceKeyOf(defId));
    expect(new Set([...Object.values(bespoke).map((entry) => entry.key), ...entrances]).size).toBe(7);
  });

  it("R2794000 R3332000 an entrance stands beside a Unit's hooks or alone; an unknown one or one on a Spell fails", () => {
    const parse = (body: string) => parseFxCardTable("test.json5", `{ ${body} }`);
    const says = (body: string, problem: RegExp): void => {
      expect(() => parse(body)).toThrow(problem);
    };
    expect(parse(`"classic-045": { entrance: "titanBloom", cry: { motif: "flight" } }`).get("classic-045")).toEqual({
      kind: "hooks",
      hooks: { cry: { motif: "flight" } },
      entrance: "titanBloom",
    });
    expect(parse(`"classic-045": { cry: { motif: "flight" }, entrance: "titanBloom", aura: { look: "bloom", scope: "side" } }`).get("classic-045")).toEqual({
      kind: "hooks",
      hooks: { cry: { motif: "flight" } },
      aura: { look: "bloom", scope: "side" },
      entrance: "titanBloom",
    });
    // No `entrance` key at all on an entry that names none, so the other tables' entries are unchanged.
    expect(Object.keys(parse(`"classic-071": { cry: { motif: "lance" } }`).get("classic-071") ?? {})).toEqual(["kind", "hooks"]);
    says(`"classic-045": { entrance: "titanBlossom", cry: { motif: "flight" } }`, /^fx\/cards\/test\.json5: classic-045: unknown entrance "titanBlossom"$/);
    says(`"classic-045": { entrance: "manaCrack", cry: { motif: "flight" } }`, /classic-045: unknown entrance "manaCrack"$/);
    says(`"classic-045": { entrance: 7, cry: { motif: "flight" } }`, /classic-045: unknown entrance "7"$/);
    says(`"core-035": { entrance: "titanBloom", cast: { motif: "bolt" } }`, /core-035: an entrance is a Unit's or a Field Spell's; this card is a Spell$/);
    // R3332000: an entrance alone is an entry of its own (until then it was a `{ bespoke }`).
    expect(parse(`"classic-045": { entrance: "titanBloom" }`).get("classic-045")).toEqual({ kind: "hooks", hooks: {}, entrance: "titanBloom" });
    says(`"classic-045": { takeover: "void" }`, /classic-045: a takeover is a Mythic's; this card is Legendary$/);
  });

  it("R3332000 the frame: an entrance beside any entry of a Unit or Field Spell or alone, a takeover beside a Mythic's, a signature that names its first beat", () => {
    const parse = (body: string) => parseFxCardTable("test.json5", `{ ${body} }`);
    const says = (body: string, problem: RegExp): void => {
      expect(() => parse(body)).toThrow(problem);
    };
    // Each kind with its frame, the frame's fields spread beside the kind's own.
    expect(parse(`"core-100": { setPiece: "voidFold", entrance: "voidCollapse", takeover: "void", signature: true }`).get("core-100")).toEqual({
      kind: "setPiece",
      key: "voidFold",
      entrance: "voidCollapse",
      takeover: "void",
      signature: true,
    });
    expect(parse(`"core-097": { bespoke: "perfectWish", takeover: "genie", signature: true }`).get("core-097")).toEqual({
      kind: "bespoke",
      key: "perfectWish",
      takeover: "genie",
      signature: true,
    });
    expect(parse(`"core-096": { cast: { motif: "snap" }, takeover: "pawn" }`).get("core-096")).toEqual({ kind: "hooks", hooks: { cast: { motif: "snap" } }, takeover: "pawn" });
    expect(parse(`"core-092": { signature: true, entrance: "felinorAssembly" }`).get("core-092")).toEqual({ kind: "hooks", hooks: {}, entrance: "felinorAssembly", signature: true });
    // A Legendary Field Spell (#93 Combo-Index) may land with an entrance of its own, beside its hooks and aura.
    expect(parse(`"core-093": { cast: { motif: "conjure" }, aura: { look: "storm", scope: "zone" }, entrance: "titanBloom", signature: true }`).get("core-093")).toEqual({
      kind: "hooks",
      hooks: { cast: { motif: "conjure" } },
      aura: { look: "storm", scope: "zone" },
      entrance: "titanBloom",
      signature: true,
    });
    // A token's rarity is the one it prints (a Classic+ token Unit printed Legendary).
    const flagbearer = Object.keys(CATALOG).find((id) => CATALOG[id]?.token === true && CATALOG[id]?.printedRarity === "Legendary" && CATALOG[id]?.type === "Unit");
    if (flagbearer === undefined) throw new Error("no printed-Legendary token Unit");
    expect(isSignature(parse(`"${flagbearer}": { entrance: "titanBloom", signature: true }`).get(flagbearer)), flagbearer).toBe(true);
    // An unframed entry keeps exactly its own keys.
    expect(Object.keys(parse(`"core-021": { bespoke: "manaCrack" }`).get("core-021") ?? {})).toEqual(["kind", "key"]);
    expect(Object.keys(parse(`"core-088": { setPiece: "nether" }`).get("core-088") ?? {})).toEqual(["kind", "key"]);
    expect(Object.keys(parse(`"core-035": { cast: { motif: "bolt" } }`).get("core-035") ?? {})).toEqual(["kind", "hooks"]);
    expect(isSignature(parse(`"core-097": { bespoke: "perfectWish" }`).get("core-097"))).toBe(false);
    expect(isSignature(parse(`"core-092": "none: pending"`).get("core-092"))).toBe(false);
    expect(isSignature(undefined)).toBe(false);
    // Every error names its file and card.
    says(`"core-092": { entrance: "catWalk" }`, /^fx\/cards\/test\.json5: core-092: unknown entrance "catWalk"$/);
    says(`"core-096": { entrance: "voidCollapse", cast: { motif: "snap" } }`, /core-096: an entrance is a Unit's or a Field Spell's; this card is a Trap$/);
    says(`"core-097": { bespoke: "perfectWish", entrance: "voidCollapse" }`, /core-097: an entrance is a Unit's or a Field Spell's; this card is a Spell$/);
    says(`"core-100": { setPiece: "voidFold", takeover: "prismatic" }`, /core-100: unknown takeover "prismatic"$/);
    says(`"core-100": { setPiece: "voidFold", takeover: 7 }`, /core-100: unknown takeover "7"$/);
    says(`"core-092": { entrance: "felinorAssembly", takeover: "void" }`, /core-092: a takeover is a Mythic's; this card is Legendary$/);
    says(`"core-092": { entrance: "felinorAssembly", signature: "yes" }`, /core-092: signature must be true$/);
    says(`"core-092": { entrance: "felinorAssembly", signature: false }`, /core-092: signature must be true$/);
    says(`"core-012": { entrance: "felinorAssembly", signature: true }`, /core-012: a signature is a Legendary's or a Mythic's; this card is Rare$/);
    says(`"core-t-felinor": { entrance: "felinorAssembly", signature: true }`, /core-t-felinor: a signature is a Legendary's or a Mythic's; this card is Token$/);
    says(`"core-092": { cry: { motif: "bless" }, signature: true }`, /core-092: a Unit's signature names its entrance$/);
    says(`"core-093": { cast: { motif: "conjure" }, aura: { look: "storm", scope: "zone" }, signature: true }`, /core-093: a Field Spell's signature names its entrance or a \{ bespoke \} or \{ setPiece \} cast of its own$/);
    says(`"core-097": { cast: { motif: "send" }, signature: true }`, /core-097: a Spell's signature is a \{ bespoke \} or \{ setPiece \} cast of its own$/);
    says(`"core-097": { bespoke: "perfectWish", cast: { motif: "send" } }`, /core-097: a bespoke entry holds nothing else but its frame and its aura$/);
    says(`"core-100": { setPiece: "voidFold", cry: { motif: "vanish" } }`, /core-100: a setPiece entry holds nothing else but its frame$/);
    says(`"core-100": { takeover: "void" }`, /core-100: names no hook$/);
    // A `{ bespoke }` names a card recipe alone: an entrance is no bespoke recipe.
    says(`"classic-080": { bespoke: "bigBoom" }`, /classic-080: unknown bespoke "bigBoom"$/);
    // A Spell's signature may be a set piece, a cast of its own (#87 Pocket Chaos, a Legendary Spell).
    expect(parse(`"core-087": { setPiece: "vortex", signature: true }`).get("core-087")).toEqual({ kind: "setPiece", key: "vortex", signature: true });
  });

  it("R3389000 a signature's bespoke wears its own look, a Unit's on its tile; a signature look off a signature fails", () => {
    const parse = (body: string) => parseFxCardTable("test.json5", `{ ${body} }`);
    const says = (body: string, problem: RegExp): void => {
      expect(() => parse(body)).toThrow(problem);
    };
    // A signature's recipe wears its own look, a Unit's on its own tile (R3400000).
    expect(parse(`"classic-045": { bespoke: "titanLeaf", entrance: "titanBloom", aura: { look: "mossCreep", scope: "tile" }, signature: true }`).get("classic-045")).toEqual({
      kind: "bespoke",
      key: "titanLeaf",
      entrance: "titanBloom",
      aura: { look: "mossCreep", scope: "tile" },
      signature: true,
    });
    // A Unit's signature hooks entry may wear one too.
    expect(entryAura(parse(`"core-092": { entrance: "felinorAssembly", aura: { look: "silkWeb", scope: "tile" }, signature: true }`).get("core-092"))).toEqual({ look: "silkWeb", scope: "tile" });
    // Every signature look parses on a signature and fails anywhere else, a hooks entry's or a recipe's.
    for (const look of AURA_SIGNATURE_LOOK_NAMES) {
      expect(entryAura(parse(`"classic-007": { bespoke: "sceptreLocket", entrance: "sceptrePlant", aura: { look: "${look}", scope: "zone" }, signature: true }`).get("classic-007")), look).toEqual({ look, scope: "zone" });
      says(`"core-018": { aura: { look: "${look}", scope: "zone" } }`, new RegExp(`core-018: aura look "${look}" is a signature's own$`));
      says(`"core-021": { bespoke: "manaCrack", aura: { look: "${look}", scope: "side" } }`, new RegExp(`core-021: aura look "${look}" is a signature's own$`));
      says(`"core-004": { cry: { motif: "bolt" }, aura: { look: "${look}", scope: "tile" } }`, new RegExp(`core-004: aura look "${look}" is a signature's own$`));
    }
    // A set piece still holds nothing but its frame, and a Trap's aura no zone.
    says(`"core-087": { setPiece: "vortex", aura: { look: "frost", scope: "side" }, signature: true }`, /core-087: a setPiece entry holds nothing else but its frame$/);
    says(`"classic-009": { bespoke: "taxSeizure", aura: { look: "storm", scope: "zone" }, signature: true }`, /classic-009: a zone aura stands in a zone; this card is a Trap, whose aura is side or board$/);
    // `entryAura` reads a hooks entry's and a bespoke's, and nothing else.
    expect(entryAura(FX_CARD_TABLE.get("classic-090"))).toEqual({ look: "abyssWhirl", palette: "Dark/shadow", scope: "zone" });
    expect(entryAura(FX_CARD_TABLE.get("classic-045"))).toEqual({ look: "mossCreep", scope: "tile" });
    expect(entryAura(FX_CARD_TABLE.get("core-014"))).toEqual({ look: "storm", scope: "side" });
    expect(entryAura(FX_CARD_TABLE.get("core-021"))).toBeUndefined();
    expect(entryAura(undefined)).toBeUndefined();
    // In Too Deep's takeover is its own.
    expect(parse(`"classic-090": { bespoke: "shrineDescent", entrance: "abyssSurface", takeover: "abyss", signature: true }`).get("classic-090")).toMatchObject({ takeover: "abyss" });
    expect(OWN_TAKEOVER_LOOKS.abyss).toBe(true);
  });

  it("R3332000 R3400000 R3389001 R3405000 the eight marquee cards keep their entrances and looks, beside the signatures'", () => {
    const entrances = { "core-100": "voidCollapse", "classic-080": "bigBoom", "classic-045": "titanBloom", "classic-056": "tyrantSigil" };
    for (const [defId, key] of Object.entries(entrances)) expect(entranceKeyOf(defId), defId).toBe(key);
    // R3400000: the signature Units' entrances and the signature Mythics' own looks.
    const signatureEntrances = {
      "core-092": "felinorAssembly",
      "meditativeplus-024": "snowplowHeave",
      "traininggrounds-038": "chalkboardLesson",
      "traininggrounds-039-1": "golemLashing",
      // R3407000: Stinkyscape part two's.
      "stinkyscape-018-1": "melchiorBoot",
      "stinkyscape-018-2": "balthasarLamp",
      "stinkyscape-018-3": "casperMirror",
      "stinkyscape-109": "freshJackVault",
      "stinkyscape-110": "ikwzkiDawn",
      "stinkyscape-111": "shockwaveRing",
      "stinkyscape-112": "resumeTube",
      "stinkyscape-115": "scornGate",
      "stinkyscape-116": "megabaseDock",
      "stinkyscape-147": "in33dh3Drop",
      "stinkyscape-147-1": "primeCrown",
      "stinkyscape-147-2": "blobPlop",
      "stinkyscape-148": "alteranDive",
      "stinkyscape-150-1": "golemCrash",
      "stinkyscape-152": "sanctumRise",
      // R3395001: Stinkyscape 001 to 090's signature Units and Field Spells.
      "stinkyscape-006": "vaultStride",
      "stinkyscape-007": "heistDrop",
      "stinkyscape-016": "rodRise",
      "stinkyscape-026": "lecternRise",
      "stinkyscape-037": "spotlightKiss",
      "stinkyscape-039": "labVault",
      "stinkyscape-058": "emitterBloom",
      "stinkyscape-059": "barrelSpin",
      "stinkyscape-060": "breechLock",
      "stinkyscape-061": "turretScan",
      "stinkyscape-066": "intakeMaw",
      "stinkyscape-070": "wraithGather",
      "stinkyscape-071": "heraldHorn",
      "stinkyscape-073": "paragonStreak",
      "stinkyscape-074": "paragonDive",
      "stinkyscape-080": "flagshipDescent",
      "stinkyscape-085": "testLurch",
      "stinkyscape-086": "forgeBurst",
      "stinkyscape-087": "redCarpet",
      "stinkyscape-088": "venomStep",
    };
    for (const [defId, key] of Object.entries(signatureEntrances)) expect(entranceKeyOf(defId), defId).toBe(key);
    // R3389001: Classic's signatures' six new entrances, and In Too Deep's own takeover.
    const classicEntrances = {
      "classic-004": "seeingOrb",
      "classic-007": "sceptrePlant",
      "classic-028": "galeBurst",
      "classic-033": "silkDrop",
      "classic-085": "royalCloak",
      "classic-090": "abyssSurface",
    };
    for (const [defId, key] of Object.entries(classicEntrances)) expect(entranceKeyOf(defId), defId).toBe(key);
    // R3405001: Meditative's signature Units' and Field Spells' entrances.
    const meditativeEntrances = {
      "meditative-007": "poolBreath",
      "meditative-011": "floodlights",
      "meditative-026": "crackedEarth",
      "meditative-039-5": "jadeBoulder",
      "meditative-040": "fiveCompass",
      "meditative-045": "chalkMask",
      "meditative-045-1": "maskSplit",
      "meditative-047": "bronzeCauldron",
      "meditative-049": "scrollScholar",
      "meditative-049-1": "pavilionUnfurl",
      "meditative-049-2": "virusBoot",
      "meditative-049-3": "dojoBoot",
      "meditative-051": "cushionDrop",
      "meditative-063": "skullThud",
      "meditative-065": "keyRing",
      "meditative-071": "studyCurve",
      "meditative-071-1": "caneRise",
      "meditative-075": "treeSprout",
      "meditative-079": "chalkHand",
      "meditative-080": "lanternLight",
      "meditative-086": "coinFlip",
      "meditative-087": "totemStack",
      "meditative-095-1": "golemLock",
      "meditative-097-8": "prisonBars",
      "meditative-097-9": "fortressRise",
    };
    for (const [defId, key] of Object.entries(meditativeEntrances)) expect(entranceKeyOf(defId), defId).toBe(key);
    const looks = {
      "core-100": "void",
      "core-096": "pawn",
      "core-097": "genie",
      "meditative-049": "ai",
      "meditativeplus-001": "summit",
      "meditativeplus-026": "canvas",
      "traininggrounds-040": "napkin",
      "classic-090": "abyss",
      // R3407000: Stinkyscape part two's six Mythics.
      "stinkyscape-109": "freshJack",
      "stinkyscape-110": "ikwzki",
      "stinkyscape-111": "shockwave",
      "stinkyscape-148": "alteran",
      "stinkyscape-148-1": "edge",
      "stinkyscape-t-red-narcor": "redNarcor",
      // R3405000: Meditative's other Mythics' own looks.
      "meditative-008": "ascent",
      "meditative-017": "workshop",
      "meditative-020": "brush",
      "meditative-039-4": "mine",
      "meditative-039-5": "quarry",
      "meditative-049-1": "pavilion",
      "meditative-049-2": "virus",
      "meditative-049-3": "dojo",
      "meditative-071": "studyHall",
      "meditative-071-1": "caneRack",
      "meditative-097": "warRoom",
      "meditative-097-9": "fortress",
      // R3399002: Classic+'s four Mythics.
      "classicplus-027": "zealot",
      "classicplus-029": "portal",
      "classicplus-065-5": "vineyard",
      "classicplus-074": "stair",
      // R3395001: Stinkyscape's five Mythics.
      "stinkyscape-006": "vault",
      "stinkyscape-016": "rod",
      "stinkyscape-044": "wire",
      "stinkyscape-074": "whiteDwarf",
      "stinkyscape-086": "forge",
    };
    for (const [defId, look] of Object.entries(looks)) expect(takeoverLookOf(defId), defId).toBe(look);
    expect(Object.keys(OWN_TAKEOVER_LOOKS).sort()).toEqual(Object.values(looks).sort());
    // R3399000: Classic+'s signatures' seventeen entrances.
    const classicPlus = [...FX_CARD_TABLE.keys()].filter((defId) => defId.startsWith("classicplus-")).flatMap((defId) => entranceKeyOf(defId) ?? []);
    expect(classicPlus).toHaveLength(17);
    // Every entrance the table names is a recipe, and the four marquees', the signature Units', (R3334004) the
    // Dormant Starship's and Starfleet Commander's and Classic's and Classic+'s signatures' are all of them.
    const named = [...FX_CARD_TABLE.keys()].flatMap((defId) => {
      const key = entranceKeyOf(defId);
      return key === undefined ? [] : [key];
    });
    for (const key of named) expect(typeof ENTRANCES[key], key).toBe("function");
    expect(new Set(named)).toEqual(
      new Set([...Object.values(entrances), ...Object.values(signatureEntrances), ...Object.values(classicEntrances), ...Object.values(meditativeEntrances), ...classicPlus, "shipyardRollout", "fleetCommand"]),
    );
    expect(Object.keys(ENTRANCES).sort()).toEqual([...new Set(named)].sort());
  });

  it("R2784000 the twelve examples name their motifs", () => {
    const examples = {
      "core-035": { kind: "hooks", hooks: { cast: { motif: "bolt" } } },
      "classicplus-032-2": { kind: "hooks", hooks: { cast: { motif: "wipe" } } },
      "classic-016": { kind: "hooks", hooks: { cast: { motif: "scorch", palette: "Fire" } } },
      "classic-071": { kind: "hooks", hooks: { cry: { motif: "lance" } } },
      "classicplus-021": { kind: "hooks", hooks: { cast: { motif: "nova" } } },
      "classicplus-025": { kind: "hooks", hooks: { cast: { motif: "bolt" } } },
      "meditative-078": { kind: "hooks", hooks: { cast: { motif: "vanish" } } },
      "meditativeplus-005": { kind: "hooks", hooks: { cast: { motif: "nova" } } },
      "stinkyscape-003": { kind: "hooks", hooks: { cast: { motif: "nova" } } },
      "stinkyscape-097": { kind: "hooks", hooks: { cast: { motif: "barrage" } } },
      "traininggrounds-010": { kind: "hooks", hooks: { cast: { motif: "volley" } } },
      "traininggrounds-026": { kind: "hooks", hooks: { cast: { motif: "vanish" } } },
    };
    expect(Object.keys(examples)).toHaveLength(12);
    for (const [defId, entry] of Object.entries(examples)) expect(FX_CARD_TABLE.get(defId), defId).toEqual(entry);
  });

  it("R2787000 an aura parses alone or beside hooks, and an unknown look, scope, palette or field, or a zone aura off the field, fails", () => {
    const parse = (body: string) => parseFxCardTable("test.json5", `{ ${body} }`);
    const says = (body: string, problem: RegExp): void => {
      expect(() => parse(body)).toThrow(problem);
    };
    expect(parse(`"core-014": { aura: { look: "storm", scope: "side" } }`).get("core-014")).toEqual({ kind: "hooks", hooks: {}, aura: { look: "storm", scope: "side" } });
    expect(parse(`"core-035": { cast: { motif: "bolt" }, aura: { look: "ember", palette: "Fire", scope: "side" } }`).get("core-035")).toEqual({
      kind: "hooks",
      hooks: { cast: { motif: "bolt" } },
      aura: { look: "ember", palette: "Fire", scope: "side" },
    });
    expect(parse(`"core-035": { cast: { motif: "bolt" } }`).get("core-035")).toEqual({ kind: "hooks", hooks: { cast: { motif: "bolt" } } });
    for (const look of AURA_LOOK_NAMES) {
      for (const scope of AURA_SCOPES) {
        // R3400000: a tile is a Unit's alone, and core-018 is a Field Trap (its own test below).
        if (scope === "tile") continue;
        expect(parse(`"core-018": { aura: { look: "${look}", scope: "${scope}" } }`).get("core-018"), `${look} ${scope}`).toEqual({ kind: "hooks", hooks: {}, aura: { look, scope } });
      }
    }
    says(`"core-014": { aura: "storm" }`, /^fx\/cards\/test\.json5: core-014: aura must be \{ look, palette\?, scope \}$/);
    says(`"core-014": { aura: { look: "storm", scope: "side", speed: 2 } }`, /core-014: unknown field "speed" in aura$/);
    says(`"core-014": { aura: { look: "sparkle", scope: "side" } }`, /core-014: unknown aura look "sparkle"$/);
    says(`"core-014": { aura: { scope: "side" } }`, /core-014: unknown aura look "undefined"$/);
    says(`"core-014": { aura: { look: "storm", scope: "deck" } }`, /core-014: unknown aura scope "deck"$/);
    says(`"core-014": { aura: { look: "storm", palette: "Plaid", scope: "side" } }`, /core-014: unknown palette "Plaid" in aura$/);
    // #2792: an aura's palette may name a sub-look, as a hook's may.
    expect(parse(`"core-014": { aura: { look: "storm", palette: "Tech/rocket", scope: "side" } }`).get("core-014")).toEqual({ kind: "hooks", hooks: {}, aura: { look: "storm", palette: "Tech/rocket", scope: "side" } });
    says(`"core-014": { aura: { look: "storm", palette: "Tech/lazer", scope: "side" } }`, /core-014: unknown palette "Tech\/lazer" in aura$/);
    says(`"core-035": { aura: { look: "ember", scope: "zone" } }`, /core-035: a zone aura stands in a zone; this card is a Spell, whose aura is side or board$/);
    says(`"core-035": { aura: { look: "ember", scope: "lane" } }`, /core-035: a lane aura stands in a zone; this card is a Spell, whose aura is side or board$/);
    expect(parse(`"core-035": { aura: { look: "ember", scope: "side" } }`).get("core-035")).toEqual({ kind: "hooks", hooks: {}, aura: { look: "ember", scope: "side" } });
    expect(parse(`"core-035": { aura: { look: "ember", scope: "board" } }`).get("core-035")?.kind).toBe("hooks");
    says(`"core-021": { bespoke: "manaCrack", aura: { look: "frost", scope: "zone" } }`, /core-021: a zone aura stands in a zone; this card is a Spell, whose aura is side or board$/);
    // R3400000: a bespoke entry may wear one too (its test below).
    expect(parse(`"core-021": { bespoke: "manaCrack", aura: { look: "frost", scope: "side" } }`).get("core-021")).toEqual({ kind: "bespoke", key: "manaCrack", aura: { look: "frost", scope: "side" } });
    says(`"core-035": {}`, /core-035: names no hook/);
  });

  it("R3400000 a bespoke entry may wear an aura, a tile aura is a Unit's, and summit, canvas and napkin are a Mythic's", () => {
    const parse = (body: string) => parseFxCardTable("test.json5", `{ ${body} }`);
    const says = (body: string, problem: RegExp): void => {
      expect(() => parse(body)).toThrow(problem);
    };
    // A bespoke entry's aura beside its frame, a Unit's on its own tile.
    expect(parse(`"meditativeplus-024": { bespoke: "bobSoldier", entrance: "snowplowHeave", aura: { look: "flurry", palette: "Physical/wind", scope: "tile" }, signature: true }`).get("meditativeplus-024")).toEqual({
      kind: "bespoke",
      key: "bobSoldier",
      entrance: "snowplowHeave",
      aura: { look: "flurry", palette: "Physical/wind", scope: "tile" },
      signature: true,
    });
    expect(parse(`"traininggrounds-039-1": { signature: true, entrance: "golemLashing", aura: { look: "timber", scope: "tile" } }`).get("traininggrounds-039-1")).toEqual({
      kind: "hooks",
      hooks: {},
      aura: { look: "timber", scope: "tile" },
      entrance: "golemLashing",
      signature: true,
    });
    // A bespoke entry's aura is parsed as a hooks entry's: an unknown look, a zone off the field, a tile off a Unit fail.
    says(`"meditativeplus-024": { bespoke: "bobSoldier", aura: { look: "snow", scope: "tile" } }`, /meditativeplus-024: unknown aura look "snow"$/);
    says(`"meditativeplus-024": { bespoke: "bobSoldier", aura: { look: "flurry", scope: "zone" } }`, /meditativeplus-024: a zone aura stands in a zone; this card is a Unit, whose aura is side or board$/);
    says(`"core-021": { bespoke: "manaCrack", aura: { look: "frost", scope: "tile" } }`, /core-021: a tile aura stands on a Unit's own tile; this card is a Spell$/);
    says(`"core-014": { aura: { look: "storm", scope: "tile" } }`, /core-014: a tile aura stands on a Unit's own tile; this card is a Field Spell$/);
    says(`"meditativeplus-024": { bespoke: "bobSoldier", setPiece: "vortex" }`, /meditativeplus-024: a bespoke entry holds nothing else but its frame and its aura$/);
    // The three looks are a Mythic's own, and the table names each once.
    for (const [defId, look] of [["meditativeplus-001", "summit"], ["meditativeplus-026", "canvas"], ["traininggrounds-040", "napkin"]] as const) {
      expect(parse(`"${defId}": { bespoke: "gameDesign", takeover: "${look}" }`).get(defId), defId).toMatchObject({ takeover: look });
      expect(takeoverLookOf(defId), defId).toBe(look);
    }
    says(`"traininggrounds-037": { bespoke: "seance", takeover: "summit" }`, /traininggrounds-037: a takeover is a Mythic's; this card is Legendary$/);
    expect(FX_CARD_TABLE.get("meditativeplus-024")).toMatchObject({ aura: { look: "flurry", scope: "tile" } });
    expect(FX_CARD_TABLE.get("traininggrounds-038")).toMatchObject({ aura: { look: "chalk", scope: "tile" } });
  });

  it("R3399001 a Classic+ signature's own look parses beside its bespoke or alone, a Unit's on its tile, and each permanent wears its own", () => {
    const parse = (body: string) => parseFxCardTable("test.json5", `{ ${body} }`);
    const says = (body: string, problem: RegExp): void => {
      expect(() => parse(body)).toThrow(problem);
    };
    // A Unit's own look on its tile (R3400000), beside a bespoke or alone (Classic+'s Mother Pancake, Top Loser).
    expect(parse(`"classicplus-012": { bespoke: "pancakeFlip", aura: { look: "syrup", scope: "tile" }, entrance: "pancakeStack", signature: true }`).get("classicplus-012")).toEqual({
      kind: "bespoke",
      key: "pancakeFlip",
      aura: { look: "syrup", scope: "tile" },
      entrance: "pancakeStack",
      signature: true,
    });
    expect(parse(`"classicplus-019-1": { aura: { look: "laneWall", scope: "tile" }, entrance: "shieldPlant", signature: true }`).get("classicplus-019-1")).toEqual({
      kind: "hooks",
      hooks: {},
      aura: { look: "laneWall", scope: "tile" },
      entrance: "shieldPlant",
      signature: true,
    });
    // A Unit's aura stands on its tile, never its zone's lane; a Field Spell's on its own zone.
    says(`"classicplus-012": { bespoke: "pancakeFlip", aura: { look: "syrup", scope: "lane" }, entrance: "pancakeStack", signature: true }`, /classicplus-012: a lane aura stands in a zone; this card is a Unit, whose aura is side or board$/);
    expect(entryAura(FX_CARD_TABLE.get("classicplus-012"))).toEqual({ look: "syrup", scope: "tile" });
    expect(entryAura(FX_CARD_TABLE.get("classicplus-019-1"))).toEqual({ look: "laneWall", palette: "Physical/steel", scope: "tile" });
    expect(entryAura(FX_CARD_TABLE.get("classicplus-078"))).toEqual({ look: "serverLights", palette: "Tech/hologram", scope: "zone" });
    // Every Classic+ signature permanent with an aura wears a look of its own; every Unit's is a tile one.
    for (const [defId, entry] of FX_CARD_TABLE) {
      const aura = entryAura(entry);
      if (!defId.startsWith("classicplus-") || !isSignature(entry) || aura === undefined) continue;
      expect(AURA_SIGNATURE_LOOK_NAMES, defId).toContain(aura.look);
      if (CATALOG[defId]?.type === "Unit") expect(aura.scope, defId).toBe("tile");
    }
    says(`"core-088": { setPiece: "nether", aura: { look: "void", scope: "board" } }`, /core-088: a setPiece entry holds nothing else but its frame$/);
    says(`"core-021": { bespoke: "manaCrack", cast: { motif: "bolt" } }`, /core-021: a bespoke entry holds nothing else but its frame and its aura$/);
  });
  it("R3395001 Stinkyscape's signature Field Spells wear their own laserGrid, silverRise and labHaze in their zones, and its signature Units an aura on their tiles", () => {
    const parse = (body: string) => parseFxCardTable("test.json5", `{ ${body} }`);
    for (const look of ["laserGrid", "silverRise", "labHaze"] as const) expect(AURA_SIGNATURE_LOOK_NAMES, look).toContain(look);
    // A signature's own look (R3389000): no entry but a signature wears it.
    expect(() => parse(`"stinkyscape-007": { bespoke: "heistLocks", aura: { look: "laserGrid", scope: "zone" } }`)).toThrow(/aura look "laserGrid" is a signature's own$/);
    expect(FX_CARD_TABLE.get("stinkyscape-007")).toMatchObject({ kind: "bespoke", key: "heistLocks", aura: { look: "laserGrid", palette: "Hustle/cash", scope: "zone" } });
    expect(FX_CARD_TABLE.get("stinkyscape-016")).toMatchObject({ kind: "bespoke", key: "rodBands", aura: { look: "silverRise", palette: "Physical/blade", scope: "zone" } });
    expect(FX_CARD_TABLE.get("stinkyscape-039")).toMatchObject({ kind: "bespoke", key: "narcorCases", aura: { look: "labHaze", palette: "Dark/curse", scope: "zone" } });
    // A Field Spell's zone look is no Unit's tile look, nor a tile a Field Spell's.
    expect(() => parse(`"stinkyscape-007": { bespoke: "heistLocks", entrance: "heistDrop", aura: { look: "laserGrid", scope: "tile" }, signature: true }`)).toThrow(
      /a tile aura stands on a Unit's own tile; this card is a Field Spell$/,
    );
    expect(() => parse(`"stinkyscape-006": { bespoke: "vaultHeist", entrance: "vaultStride", aura: { look: "laserGrid", scope: "zone" }, signature: true }`)).toThrow(/a zone aura stands in a zone; this card is a Unit/);
    // Every signature Unit of the range stands on a tile aura of its own.
    const units = [...FX_CARD_TABLE].filter(([defId, entry]) => /^stinkyscape-0[0-8]\d$|^stinkyscape-090$/.test(defId) && isSignature(entry) && CATALOG[defId]?.type === "Unit");
    expect(units.length).toBe(17);
    for (const [defId, entry] of units) {
      const aura = entry.kind === "hooks" || entry.kind === "bespoke" ? entry.aura : undefined;
      expect(aura?.scope, defId).toBe("tile");
    }
  });

  it("R2787000 R2794000 R3405001 the ten example Field Spells across the sets wear their auras", () => {
  const examples = {
    "core-014": { look: "storm", scope: "side" },
    "core-046": { look: "void", scope: "board" },
    "classic-042": { look: "toxic", scope: "board" },
    "classic-075": { look: "holy", scope: "side" },
    "classicplus-034": { look: "frost", palette: "Tech", scope: "lane" },
    // R3405001: Feng Shui's signature stands its compass ring on its own zone.
    "meditative-040": { look: "orbit", scope: "zone" },
    "meditativeplus-009": { look: "arcane", scope: "board" },
    "stinkyscape-046": { look: "storm", scope: "lane" },
    "stinkyscape-155": { look: "ember", scope: "zone" },
    "traininggrounds-019": { look: "bloom", scope: "zone" },
  };
  expect(Object.keys(examples)).toHaveLength(10);
  // R2794000: Classic's pass gave its two examples their play's motifs beside the aura.
  const hooksOf: Readonly<Record<string, object>> = {
    "classic-042": { cast: { motif: "tally" }, activate: { motif: "tally" } },
    "classic-075": { cast: { motif: "bless" } },
  };
  const files = new Set<string>();
  for (const [defId, aura] of Object.entries(examples)) {
    expect(CATALOG[defId]?.type, defId).toBe("Field Spell");
    // A set's pass may give the card hooks beside its aura (#2793 gave Core #14 and #46 a cast, #2796
    // Meditative #40 and Meditative+ #9, R2795000 Classic+ #34 its bind); the aura stays the example's.
    // R3400000: or a bespoke recipe of its own beside it (R3405001 Meditative #40's signature).
    const entry = FX_CARD_TABLE.get(defId);
    expect(entry?.kind, defId).toBe(defId === "meditative-040" ? "bespoke" : "hooks");
    expect(entry?.kind === "hooks" || entry?.kind === "bespoke" ? entry.aura : undefined, defId).toEqual(aura);
    const hooks = hooksOf[defId];
    if (hooks !== undefined) expect(entry, defId).toEqual({ kind: "hooks", hooks, aura });
    const file = FX_CARD_FILES.find((candidate) => parseFxCardTable(candidate.file, candidate.source).has(defId));
    if (file !== undefined) files.add(file.file);
  }
  expect(files.size).toBe(FX_CARD_FILES.length);
});
});
