// The real-art delivery convention (R660, ART.md): what a file in `apps/web/public/art/` must be for
// the manifest to list it. Pure: it reads bytes it is handed, so convention.test.ts can hold the real
// directory to it and prove each refusal on bytes made up in the test. Nothing in the bundle calls it.

import { artFileName, type ArtManifest } from "./manifest.ts";

/** A delivered picture is square, this many pixels a side: the art window crops it (`object-fit: cover`). */
export const ART_SIDE_PX = 512;

/** A delivered file's largest size in bytes (96 KiB): the Card Almanac draws every card at once. */
export const ART_MAX_BYTES = 96 * 1024;

/** RIFF's container header: "RIFF", the payload's length, then the form type. */
const RIFF_TAG = "RIFF";
const WEBP_TAG = "WEBP";
const FORM_TAG_OFFSET = 8;
const CHUNK_TAG_OFFSET = 12;
const TAG_LENGTH = 4;

/** The three WebP bitstreams and where each keeps the picture's size (Google's WebP container spec). */
const LOSSY_TAG = "VP8 ";
const LOSSLESS_TAG = "VP8L";
const EXTENDED_TAG = "VP8X";
/** Lossy: a 14-bit width and height, little-endian, after the frame tag's start code. */
const LOSSY_SIZE_OFFSET = 26;
const LOSSY_SIZE_MASK = 0x3fff;
/** Lossless: after the 0x2f signature byte, width − 1 and height − 1 in 14 bits each. */
const LOSSLESS_SIGNATURE_OFFSET = 20;
const LOSSLESS_SIGNATURE = 0x2f;
const LOSSLESS_SIZE_BITS = 14;
/** Extended: canvas width − 1 and height − 1 as 24-bit little-endian numbers. */
const EXTENDED_SIZE_OFFSET = 24;
const EXTENDED_SIZE_BYTES = 3;
const BYTE_BITS = 8;
/** The longest header any of the three needs read. */
const HEADER_BYTES = 30;

function tagAt(bytes: Uint8Array, offset: number): string {
  return String.fromCharCode(...bytes.subarray(offset, offset + TAG_LENGTH));
}

/** A little-endian unsigned number of `count` bytes at `offset`. */
function littleEndian(bytes: Uint8Array, offset: number, count: number): number {
  let value = 0;
  for (let i = count - 1; i >= 0; i -= 1) value = value * (1 << BYTE_BITS) + (bytes[offset + i] ?? 0);
  return value;
}

/** A WebP file's picture size, or null when the bytes are not a WebP file. */
export function webpSize(bytes: Uint8Array): { width: number; height: number } | null {
  if (bytes.length < HEADER_BYTES) return null;
  if (tagAt(bytes, 0) !== RIFF_TAG || tagAt(bytes, FORM_TAG_OFFSET) !== WEBP_TAG) return null;
  const chunk = tagAt(bytes, CHUNK_TAG_OFFSET);
  if (chunk === LOSSY_TAG) {
    return {
      width: littleEndian(bytes, LOSSY_SIZE_OFFSET, 2) & LOSSY_SIZE_MASK,
      height: littleEndian(bytes, LOSSY_SIZE_OFFSET + 2, 2) & LOSSY_SIZE_MASK,
    };
  }
  if (chunk === LOSSLESS_TAG) {
    if (bytes[LOSSLESS_SIGNATURE_OFFSET] !== LOSSLESS_SIGNATURE) return null;
    const bits = littleEndian(bytes, LOSSLESS_SIGNATURE_OFFSET + 1, 4);
    const mask = (1 << LOSSLESS_SIZE_BITS) - 1;
    return { width: (bits & mask) + 1, height: ((bits >>> LOSSLESS_SIZE_BITS) & mask) + 1 };
  }
  if (chunk === EXTENDED_TAG) {
    return {
      width: littleEndian(bytes, EXTENDED_SIZE_OFFSET, EXTENDED_SIZE_BYTES) + 1,
      height: littleEndian(bytes, EXTENDED_SIZE_OFFSET + EXTENDED_SIZE_BYTES, EXTENDED_SIZE_BYTES) + 1,
    };
  }
  return null;
}

/** What a delivered file breaks of the convention, or null when it meets it. */
export function artFileProblem(bytes: Uint8Array): string | null {
  const size = webpSize(bytes);
  if (size === null) return "is not a WebP file";
  if (size.width !== ART_SIDE_PX || size.height !== ART_SIDE_PX) {
    return `is ${String(size.width)}x${String(size.height)}, not ${String(ART_SIDE_PX)}x${String(ART_SIDE_PX)}`;
  }
  if (bytes.length > ART_MAX_BYTES) return `is ${String(bytes.length)} bytes, over ${String(ART_MAX_BYTES)}`;
  return null;
}

/**
 * Every way the art directory and the manifest disagree with each other, the catalog or the
 * convention: a line for an id the catalog lacks or naming no face, a listed file that is missing or
 * breaks the convention, and a file no line lists. Empty when an artist's delivery is complete.
 */
export function artDeliveryProblems(
  manifest: ArtManifest,
  files: ReadonlyMap<string, Uint8Array>,
  catalogIds: ReadonlySet<string>,
): string[] {
  const problems: string[] = [];
  const listed = new Set<string>();
  for (const [id, entry] of Object.entries(manifest)) {
    if (!catalogIds.has(id)) problems.push(`manifest: ${id} is not a catalog card or token`);
    const faces = [entry.base === true ? false : null, entry.radiant === true ? true : null].filter(
      (face): face is boolean => face !== null,
    );
    if (faces.length === 0) problems.push(`manifest: ${id} lists neither face`);
    for (const radiant of faces) {
      const name = artFileName(id, radiant);
      listed.add(name);
      const bytes = files.get(name);
      if (bytes === undefined) {
        problems.push(`${name}: listed in the manifest but missing`);
        continue;
      }
      const problem = artFileProblem(bytes);
      if (problem !== null) problems.push(`${name}: ${problem}`);
    }
  }
  for (const name of files.keys()) {
    if (!listed.has(name)) problems.push(`${name}: in the art directory but not in the manifest`);
  }
  return problems.sort();
}
