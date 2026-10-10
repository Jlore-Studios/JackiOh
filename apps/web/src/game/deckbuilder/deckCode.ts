// Deck codes (SPEC §9.4, R255) are shareable deck text. Imports are new saved decks; this module
// does not judge legality (R250, R253).
//
// `JKO<version>.` precedes an unpadded base64url body:
// [name-byte-length] [UTF-8 name] [LEB128 card count] [LEB128 card numbers] [two-byte checksum].
// Version 2 uses a card's set offset plus §5 index (B2.2); non-whole-number Token indices like
// "T-AI-1" cannot be encoded (R251). Names failing D1 fall back to "Imported deck". FNV-1a detects damaged pastes.
// Older codes are refused except Core-only version 1 (R255).
//
// Decoding is total: it rejects over-long or malformed input before resolving name (D1) and cards.
// Trio codes share the body reader, resolver and checksum (R339, `trioCode.ts`).

import { checkDeckDraft, normalizeName, type CatalogSnapshot, type Collection } from "@jackioh/validator";

import type { CardDef } from "@jackioh/shared";
import {
  CATALOG_NUMBER_SET_OFFSETS,
  DECK_CODE_CORE_ONLY_VERSION,
  DECK_CODE_MAX_INPUT_LENGTH,
  DECK_CODE_VERSION,
  DECK_NAME_MAX_LENGTH,
} from "@jackioh/server-config";
import { DECK_SIZE, MAX_COPIES } from "./deckSize.ts";

/** What every code starts with, before its version digits and a ".". */
export const DECK_CODE_PREFIX = "JKO";

/** The name an import gets when the code's own name is unusable (R255). */
export const IMPORTED_DECK_NAME = "Imported deck";

export type DroppedCards = {
  /** Card numbers the catalog does not know, in code order (numbers, not ids). */
  unknown: readonly number[];
  /** Ids of Tokens the code named. */
  tokens: readonly string[];
  /** Ids of copies past MAX_COPIES (one entry per dropped copy). */
  duplicates: readonly string[];
  /** Ids past DECK_SIZE, in code order. */
  overflow: readonly string[];
};

export type DecodedDeck =
  | {
      ok: true;
      name: string;
      cards: readonly string[];
      dropped: DroppedCards;
      unowned: readonly string[];
      nameFellBack: boolean;
    }
  | { ok: false; message: string };

/** Every refusal, as the import dialog shows it. */
export const DECK_CODE_MESSAGES = Object.freeze({
  tooLong: "That’s too long to be a JackiOh deck code.",
  empty: "Paste a deck code to import it.",
  notACode: "That isn’t a JackiOh deck code.",
  trioCode: "That’s a trio code, not a deck code: import it with Import trio, under Trios.",
  newer: "This code was made by a newer version of JackiOh. Reload the page to update, then try again.",
  older: "This code was made by an older version of JackiOh, and this version can’t read it.",
  damaged: "That deck code is damaged. Copy it again from where it was shared.",
  incomplete: "That deck code is incomplete. Copy the whole code and try again.",
  checksum: "That deck code doesn’t check out: part of it was changed or lost. Copy it again from where it was shared.",
  unreadable: "That deck code couldn’t be read.",
});

// Byte format

/** The name's byte length is one byte. */
const NAME_LENGTH_BYTES = 1;
/** The most name bytes one length byte can say. */
const NAME_BYTES_MAX = 0xff;
const CHECKSUM_BYTES = 2;
/** The smallest payload: an empty name's length, a zero count and the checksum. */
const MIN_PAYLOAD_BYTES = NAME_LENGTH_BYTES + 1 + CHECKSUM_BYTES;

const BYTE_BITS = 8;
const BYTE_MASK = 0xff;

/** LEB128: seven bits of value per byte, the top bit set on every byte but the last. */
const VARINT_PAYLOAD_BITS = 7;
const VARINT_BASE = 2 ** VARINT_PAYLOAD_BITS;
const VARINT_PAYLOAD_MASK = 0x7f;
const VARINT_CONTINUE = 0x80;
/** Four bytes carry up to 2^28, far past any card number or count; more is a damaged code. */
const VARINT_MAX_BYTES = 4;

const FNV_OFFSET_BASIS = 0x811c9dc5;
const FNV_PRIME = 0x01000193;
const HALF_BITS = 16;
const HALF_MASK = 0xffff;

const BASE64URL_ALPHABET = "ABCDEFGHIJKLMNOPQRSTUVWXYZabcdefghijklmnopqrstuvwxyz0123456789-_";
const SEXTET_BITS = 6;
const SEXTET_MASK = 0x3f;
/** A base64 group is 4 characters; a lone trailing character carries no whole byte. */
const BASE64_GROUP = 4;
const BASE64_IMPOSSIBLE_REMAINDER = 1;

const BASE64URL_BODY = /^[A-Za-z0-9_-]*$/;
/** Up to two "=" of standard padding, which a chat client or a hand may add. */
const TRAILING_PADDING = /={1,2}$/;
/** Any code's header: three capital letters (`JKO` a deck, `JKT` a trio), the version, a dot. */
const HEADER = /^([A-Z]{3})(\d+)\.(.*)$/su;
const WHITESPACE = /\s+/gu;
/** A card number that can be encoded: a whole number from 1, as a Core `index` is. */
const WHOLE_NUMBER = /^[1-9]\d*$/;

// Helpers

/** FNV-1a (32-bit) over `bytes[0..end)`, folded to 16 bits. */
export function fnv1a16(bytes: readonly number[] | Uint8Array, end: number): number {
  let hash = FNV_OFFSET_BASIS;
  for (let at = 0; at < end; at += 1) {
    hash ^= bytes[at] ?? 0;
    hash = Math.imul(hash, FNV_PRIME) >>> 0;
  }
  return ((hash >>> HALF_BITS) ^ (hash & HALF_MASK)) & HALF_MASK;
}

export function pushVarint(out: number[], value: number): void {
  let rest = value;
  while (rest > VARINT_PAYLOAD_MASK) {
    out.push((rest & VARINT_PAYLOAD_MASK) | VARINT_CONTINUE);
    rest = Math.floor(rest / VARINT_BASE);
  }
  out.push(rest);
}

export function toBase64Url(bytes: readonly number[]): string {
  let out = "";
  let acc = 0;
  let bits = 0;
  for (const byte of bytes) {
    acc = (acc << BYTE_BITS) | (byte & BYTE_MASK);
    bits += BYTE_BITS;
    while (bits >= SEXTET_BITS) {
      bits -= SEXTET_BITS;
      out += BASE64URL_ALPHABET[(acc >>> bits) & SEXTET_MASK] ?? "";
    }
    acc &= (1 << bits) - 1;
  }
  if (bits > 0) out += BASE64URL_ALPHABET[(acc << (SEXTET_BITS - bits)) & SEXTET_MASK] ?? "";
  return out;
}

/** Null when the text is not base64url or has an impossible length. */
export function fromBase64Url(text: string): Uint8Array | null {
  if (!BASE64URL_BODY.test(text)) return null;
  if (text.length % BASE64_GROUP === BASE64_IMPOSSIBLE_REMAINDER) return null;
  const out: number[] = [];
  let acc = 0;
  let bits = 0;
  for (const char of text) {
    const value = BASE64URL_ALPHABET.indexOf(char);
    if (value < 0) return null;
    acc = (acc << SEXTET_BITS) | value;
    bits += SEXTET_BITS;
    if (bits >= BYTE_BITS) {
      bits -= BYTE_BITS;
      out.push((acc >>> bits) & BYTE_MASK);
    }
    acc &= (1 << bits) - 1;
  }
  return Uint8Array.from(out);
}

/** D1: whether `name` may be a saved deck's name. */
function passesD1(name: string): boolean {
  const issues = checkDeckDraft({
    name,
    cards: [],
    isDeckable: () => true,
    nameMaxLength: DECK_NAME_MAX_LENGTH,
  });
  return !issues.some((issue) => issue.rule === "D1");
}

/** A stored, D1-clean code name within `DECK_NAME_MAX_LENGTH`. */
function nameForCode(raw: string): string {
  const cut = normalizeName([...normalizeName(raw)].slice(0, DECK_NAME_MAX_LENGTH).join(""));
  return passesD1(cut) ? cut : IMPORTED_DECK_NAME;
}

/** B2.2: whole-number §5 index plus set offset, or `undefined` when unencodable. */
export function catalogNumberOf(def: Pick<CardDef, "set" | "index">): number | undefined {
  if (!WHOLE_NUMBER.test(def.index)) return undefined;
  const offset = (CATALOG_NUMBER_SET_OFFSETS as Readonly<Record<string, number>>)[def.set];
  return offset === undefined ? undefined : offset + Number(def.index);
}

/** R255: version 1 numbers are Core indices; later versions use catalog numbers. */
function numberOfIn(version: number): (def: CardDef) => number | undefined {
  if (version === DECK_CODE_CORE_ONLY_VERSION) {
    return (def) => (def.set === "Core" && WHOLE_NUMBER.test(def.index) ? Number(def.index) : undefined);
  }
  return catalogNumberOf;
}

function isToken(catalog: CatalogSnapshot, cardId: string): boolean {
  const def = catalog.cards[cardId];
  return def !== undefined && (def.token || def.tags.includes("Token"));
}

// Encode

/** Writes one length byte and UTF-8, using `fallback` if the name does not fit. */
export function pushName(out: number[], name: string, fallback: string): void {
  let bytes = new TextEncoder().encode(name);
  if (bytes.length > NAME_BYTES_MAX) bytes = new TextEncoder().encode(fallback);
  out.push(bytes.length, ...bytes);
}

/** Writes a deck or trio-code body; unknown or unencodable cards are skipped. */
export function writeDeckBody(out: number[], name: string, cardIds: readonly string[], catalog: CatalogSnapshot): void {
  pushName(out, nameForCode(name), IMPORTED_DECK_NAME);
  const numbers: number[] = [];
  for (const cardId of cardIds) {
    const def = catalog.cards[cardId];
    const number = def === undefined ? undefined : catalogNumberOf(def);
    if (number === undefined) continue;
    numbers.push(number);
  }
  pushVarint(out, numbers.length);
  for (const number of numbers) pushVarint(out, number);
}

/** Appends the checksum of everything before it: two bytes, high first. */
export function pushChecksum(out: number[]): void {
  const checksum = fnv1a16(out, out.length);
  out.push((checksum >>> BYTE_BITS) & BYTE_MASK, checksum & BYTE_MASK);
}

/** The code for a deck. */
export function encodeDeckCode(name: string, cardIds: readonly string[], catalog: CatalogSnapshot): string {
  const payload: number[] = [];
  writeDeckBody(payload, name, cardIds, catalog);
  pushChecksum(payload);
  return `${DECK_CODE_PREFIX}${String(DECK_CODE_VERSION)}.${toBase64Url(payload)}`;
}

// Decode

/** Why a payload could not be read: it declares more than it carries, or holds what no code does. */
export type ReadFailure = "incomplete" | "damaged";

/** A cursor over a payload, `end` being where its checksum starts. Shared with `trioCode.ts`. */
export type PayloadReader = { bytes: Uint8Array; at: number; end: number };

/** A reader over `bytes`, or "incomplete" when they cannot even hold `minimum` bytes and a checksum. */
export function payloadReader(bytes: Uint8Array, minimum: number): PayloadReader | "incomplete" {
  if (bytes.length < minimum) return "incomplete";
  return { bytes, at: 0, end: bytes.length - CHECKSUM_BYTES };
}

export function readByte(reader: PayloadReader): number | "incomplete" {
  if (reader.at >= reader.end) return "incomplete";
  const byte = reader.bytes[reader.at] ?? 0;
  reader.at += 1;
  return byte;
}

export function readVarint(reader: PayloadReader): number | ReadFailure {
  let value = 0;
  let scale = 1;
  for (let used = 0; used < VARINT_MAX_BYTES; used += 1) {
    const byte = readByte(reader);
    if (byte === "incomplete") return byte;
    value += (byte & VARINT_PAYLOAD_MASK) * scale;
    if ((byte & VARINT_CONTINUE) === 0) return value;
    scale *= VARINT_BASE;
  }
  return "damaged";
}

/** A name: its length byte, then that many bytes. */
export function readNameBytes(reader: PayloadReader): Uint8Array | "incomplete" {
  const length = readByte(reader);
  if (length === "incomplete") return length;
  if (reader.at + length > reader.end) return "incomplete";
  const bytes = reader.bytes.slice(reader.at, reader.at + length);
  reader.at += length;
  return bytes;
}

/** One deck's body, as `writeDeckBody` wrote it. */
export type DeckBody = { nameBytes: Uint8Array; numbers: number[] };

export function readDeckBody(reader: PayloadReader): DeckBody | ReadFailure {
  const nameBytes = readNameBytes(reader);
  if (nameBytes === "incomplete") return nameBytes;
  const count = readVarint(reader);
  if (typeof count !== "number") return count;
  // Every number takes at least one byte, so a count past what is left cannot be satisfied.
  if (count > reader.end - reader.at) return "incomplete";
  const numbers: number[] = [];
  for (let read = 0; read < count; read += 1) {
    const number = readVarint(reader);
    if (typeof number !== "number") return number;
    numbers.push(number);
  }
  return { nameBytes, numbers };
}

/** Whether the reader stopped exactly at the checksum, and the checksum holds. */
export function checkEnd(reader: PayloadReader): "ok" | "damaged" | "checksum" {
  if (reader.at !== reader.end) return "damaged";
  const { bytes, end } = reader;
  const stored = ((bytes[end] ?? 0) << BYTE_BITS) | (bytes[end + 1] ?? 0);
  return stored === fnv1a16(bytes, end) ? "ok" : "checksum";
}

type Parsed = { ok: true; body: DeckBody } | { ok: false; message: string };

/** Reads the structure between the header and the checksum; the checksum is checked after. */
function parsePayload(bytes: Uint8Array): Parsed {
  const reader = payloadReader(bytes, MIN_PAYLOAD_BYTES);
  if (reader === "incomplete") return { ok: false, message: DECK_CODE_MESSAGES.incomplete };
  const body = readDeckBody(reader);
  if (body === "incomplete") return { ok: false, message: DECK_CODE_MESSAGES.incomplete };
  if (body === "damaged") return { ok: false, message: DECK_CODE_MESSAGES.damaged };
  const end = checkEnd(reader);
  if (end === "damaged") return { ok: false, message: DECK_CODE_MESSAGES.damaged };
  if (end === "checksum") return { ok: false, message: DECK_CODE_MESSAGES.checksum };
  return { ok: true, body };
}

/**
 * A name as a code carried it, or `fallback` when its bytes are not UTF-8 or `valid` refuses it
 * (D1 for a deck, T1 for a trio).
 */
export function decodeName(
  nameBytes: Uint8Array,
  fallback: string,
  valid: (name: string) => boolean,
): { name: string; fellBack: boolean } {
  let text: string;
  try {
    text = new TextDecoder("utf-8", { fatal: true }).decode(nameBytes);
  } catch {
    return { name: fallback, fellBack: true };
  }
  const name = normalizeName(text);
  return valid(name) ? { name, fellBack: false } : { name: fallback, fellBack: true };
}

/** What a deck's body becomes: its name, the cards kept, what was dropped and what is unowned. */
export type ResolvedDeck = Omit<Extract<DecodedDeck, { ok: true }>, "ok">;

/**
 * R255: a deck body against this catalog and collection — numbers the catalog does not know,
 * Tokens, copies past `MAX_COPIES` and cards past `DECK_SIZE` dropped and listed; cards the player
 * does not own kept and flagged. `version` is the code's: a version 1 code's numbers are Core's.
 */
export function resolveDeck(
  body: DeckBody,
  catalog: CatalogSnapshot,
  collection: Collection | null,
  version: number = DECK_CODE_VERSION,
): ResolvedDeck {
  const { name, fellBack } = decodeName(body.nameBytes, IMPORTED_DECK_NAME, passesD1);

  const numberOf = numberOfIn(version);
  const byNumber = new Map<number, string>();
  for (const [cardId, def] of Object.entries(catalog.cards)) {
    const number = numberOf(def);
    if (number !== undefined) byNumber.set(number, cardId);
  }

  const cards: string[] = [];
  const copies = new Map<string, number>();
  const unknown: number[] = [];
  const tokens: string[] = [];
  const duplicates: string[] = [];
  const overflow: string[] = [];
  for (const number of body.numbers) {
    const cardId = byNumber.get(number);
    if (cardId === undefined) {
      unknown.push(number);
      continue;
    }
    if (isToken(catalog, cardId)) {
      tokens.push(cardId);
      continue;
    }
    const held = copies.get(cardId) ?? 0;
    if (held >= MAX_COPIES) {
      duplicates.push(cardId);
      continue;
    }
    if (cards.length >= DECK_SIZE) {
      overflow.push(cardId);
      continue;
    }
    copies.set(cardId, held + 1);
    cards.push(cardId);
  }

  const unowned =
    collection === null ? [] : [...new Set(cards)].filter((cardId) => (collection[cardId] ?? 0) < 1);

  return { name, cards, dropped: { unknown, tokens, duplicates, overflow }, unowned, nameFellBack: fellBack };
}

/** The sentences a code's header can be refused with: a deck code's, or a trio code's. */
export type HeaderMessages = {
  tooLong: string;
  empty: string;
  notACode: string;
  newer: string;
  older: string;
  damaged: string;
};

/**
 * The text of a pasted code, read as far as its payload's bytes: the length cap first, unread
 * (`maxInputLength`), then whitespace, the prefix and version, and base64url. `other` is the prefix
 * of the other kind of code, answered with `otherMessage` so a trio code pasted as a deck (or the
 * reverse) is sent where it belongs. `alsoReads` is the older version still read (R255, R339),
 * and the version read comes back with the bytes.
 */
export function readCodeText(
  text: unknown,
  format: {
    prefix: string;
    version: number;
    alsoReads: number;
    maxInputLength: number;
    other: string;
    otherMessage: string;
  },
  messages: HeaderMessages,
): { ok: true; bytes: Uint8Array; version: number } | { ok: false; message: string } {
  if (typeof text !== "string") return { ok: false, message: messages.notACode };
  // Refused unread: nothing below runs on a paste this long.
  if (text.length > format.maxInputLength) return { ok: false, message: messages.tooLong };

  const compact = text.replace(WHITESPACE, "");
  if (compact.length === 0) return { ok: false, message: messages.empty };

  const header = HEADER.exec(compact);
  if (header === null) return { ok: false, message: messages.notACode };
  if (header[1] === format.other) return { ok: false, message: format.otherMessage };
  if (header[1] !== format.prefix) return { ok: false, message: messages.notACode };
  const version = Number(header[2] ?? "");
  if (version > format.version) return { ok: false, message: messages.newer };
  if (version !== format.version && version !== format.alsoReads) return { ok: false, message: messages.older };

  const bytes = fromBase64Url((header[3] ?? "").replace(TRAILING_PADDING, ""));
  if (bytes === null) return { ok: false, message: messages.damaged };
  return { ok: true, bytes, version };
}

/** The trio code's prefix (R339), which the deck import recognises and sends where it belongs. */
const TRIO_CODE_PREFIX_SEEN = "JKT";

function decodeUnsafe(text: unknown, catalog: CatalogSnapshot, collection: Collection | null): DecodedDeck {
  const read = readCodeText(
    text,
    {
      prefix: DECK_CODE_PREFIX,
      version: DECK_CODE_VERSION,
      alsoReads: DECK_CODE_CORE_ONLY_VERSION,
      maxInputLength: DECK_CODE_MAX_INPUT_LENGTH,
      other: TRIO_CODE_PREFIX_SEEN,
      otherMessage: DECK_CODE_MESSAGES.trioCode,
    },
    DECK_CODE_MESSAGES,
  );
  if (!read.ok) return read;
  const parsed = parsePayload(read.bytes);
  if (!parsed.ok) return parsed;
  return { ok: true, ...resolveDeck(parsed.body, catalog, collection, read.version) };
}

/**
 * What a pasted code holds, or why it cannot be read. Never throws: a player can paste anything,
 * and every failure is a sentence (`DECK_CODE_MESSAGES`).
 */
export function decodeDeckCode(text: unknown, catalog: CatalogSnapshot, collection: Collection | null): DecodedDeck {
  try {
    return decodeUnsafe(text, catalog, collection);
  } catch {
    return { ok: false, message: DECK_CODE_MESSAGES.unreadable };
  }
}
