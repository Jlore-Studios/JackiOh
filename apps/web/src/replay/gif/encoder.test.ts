// The GIF encoder of a shared replay (#513, R513000): the same frames give the same bytes, and the
// bytes decode, with a decoder written here from the format's own rules, as the frames, sizes and
// delays that went in. The decoder is the test's alone: nothing in the client reads a GIF.

import { describe, expect, it } from "vitest";

import { GIF_PALETTE, createGifEncoder, paletteIndex } from "./encoder.ts";

type DecodedFrame = { delayCs: number; indices: Uint8Array };
type Decoded = { version: string; width: number; height: number; palette: Uint8Array; loops: boolean; frames: DecodedFrame[]; trailer: boolean };

/** The LZW codes of one image, read the way a GIF decoder does: the code size grows when the table fills it. */
function lzwDecode(minSize: number, data: Uint8Array): number[] {
  const clear = 1 << minSize;
  const end = clear + 1;
  const fresh = (): number[][] => Array.from({ length: end + 1 }, (_, code) => (code < clear ? [code] : []));
  let table = fresh();
  let size = minSize + 1;
  let previous: number[] | null = null;
  const out: number[] = [];
  let bit = 0;
  for (;;) {
    let code = 0;
    for (let i = 0; i < size; i += 1) {
      const byte = data[(bit + i) >> 3];
      if (byte === undefined) throw new Error("the codes ended before the end code");
      code |= ((byte >> ((bit + i) & 7)) & 1) << i;
    }
    bit += size;
    if (code === clear) {
      table = fresh();
      size = minSize + 1;
      previous = null;
      continue;
    }
    if (code === end) return out;
    let entry: number[];
    if (code < table.length) entry = table[code] ?? [];
    else if (code === table.length && previous !== null) entry = [...previous, previous[0] ?? 0];
    else throw new Error(`code ${String(code)} is past the table's ${String(table.length)} entries`);
    out.push(...entry);
    if (previous !== null && table.length < 4096) table.push([...previous, entry[0] ?? 0]);
    previous = entry;
    if (table.length === 1 << size && size < 12) size += 1;
  }
}

function decode(bytes: Uint8Array): Decoded {
  let at = 0;
  const u8 = (): number => {
    const value = bytes[at];
    if (value === undefined) throw new Error("the file ended early");
    at += 1;
    return value;
  };
  const u16 = (): number => u8() | (u8() << 8);
  const text = (length: number): string => String.fromCharCode(...Array.from({ length }, u8));
  /** The data sub-blocks up to their zero terminator, joined. */
  const blocks = (): Uint8Array => {
    const parts: number[] = [];
    for (let size = u8(); size > 0; size = u8()) for (let i = 0; i < size; i += 1) parts.push(u8());
    return Uint8Array.from(parts);
  };

  const version = text(6);
  const width = u16();
  const height = u16();
  const flags = u8();
  u8();
  u8();
  expect(flags & 0x80, "a global colour table").toBe(0x80);
  const palette = Uint8Array.from(Array.from({ length: 3 << ((flags & 7) + 1) }, u8));
  const decoded: Decoded = { version, width, height, palette, loops: false, frames: [], trailer: false };
  let delayCs = 0;
  for (;;) {
    const marker = u8();
    if (marker === 0x3b) {
      decoded.trailer = at === bytes.length;
      return decoded;
    }
    if (marker === 0x21) {
      const label = u8();
      if (label === 0xff) {
        const size = u8();
        const name = text(size);
        const data = blocks();
        if (name === "NETSCAPE2.0" && data[0] === 1) decoded.loops = true;
      } else if (label === 0xf9) {
        expect(u8()).toBe(4);
        u8();
        delayCs = u16();
        u8();
        expect(u8(), "the block's terminator").toBe(0);
      } else throw new Error(`extension ${label.toString(16)}`);
      continue;
    }
    expect(marker).toBe(0x2c);
    expect([u16(), u16(), u16(), u16()], "the image fills the canvas").toEqual([0, 0, width, height]);
    expect(u8() & 0x80, "no local colour table").toBe(0);
    const minSize = u8();
    const indices = lzwDecode(minSize, blocks());
    decoded.frames.push({ delayCs, indices: Uint8Array.from(indices) });
  }
}

/** A frame of `width * height` pixels from `pixel(x, y)`, as RGBA. */
function frame(width: number, height: number, pixel: (x: number, y: number) => [number, number, number]): Uint8ClampedArray {
  const rgba = new Uint8ClampedArray(width * height * 4);
  for (let y = 0; y < height; y += 1) {
    for (let x = 0; x < width; x += 1) {
      const [r, g, b] = pixel(x, y);
      rgba.set([r, g, b, 255], (y * width + x) * 4);
    }
  }
  return rgba;
}

function expectedIndices(rgba: Uint8ClampedArray): number[] {
  return Array.from({ length: rgba.length / 4 }, (_, i) => paletteIndex(rgba[i * 4] ?? 0, rgba[i * 4 + 1] ?? 0, rgba[i * 4 + 2] ?? 0));
}

const W = 7;
const H = 5;
const FRAMES: { rgba: Uint8ClampedArray; delayCs: number }[] = [
  { rgba: frame(W, H, (x, y) => [x * 36, y * 60, 128]), delayCs: 10 },
  { rgba: frame(W, H, (x, y) => [255 - x * 30, 40, y * 63]), delayCs: 20 },
  { rgba: frame(W, H, () => [12, 200, 90]), delayCs: 100 },
];

function encode(): Uint8Array {
  const encoder = createGifEncoder(W, H);
  for (const { rgba, delayCs } of FRAMES) encoder.add(rgba, delayCs);
  return encoder.finish();
}

describe("R513000 the GIF encoder", () => {
  it("R513000 the same RGBA frames give the same bytes on every run", () => {
    const first = encode();
    expect(Array.from(encode())).toEqual(Array.from(first));
    expect(Array.from(encode())).toEqual(Array.from(first));
  });

  it("decodes as a GIF89a with its frame count, size, delays and pixels", () => {
    const bytes = encode();
    const gif = decode(bytes);
    expect(gif.version).toBe("GIF89a");
    expect([gif.width, gif.height]).toEqual([W, H]);
    expect(gif.loops, "the Netscape loop extension").toBe(true);
    expect(gif.trailer, "the trailer is the last byte").toBe(true);
    expect(Array.from(gif.palette)).toEqual(Array.from(GIF_PALETTE));
    expect(gif.frames.map((f) => f.delayCs)).toEqual([10, 20, 100]);
    gif.frames.forEach((decoded, i) => {
      expect(Array.from(decoded.indices)).toEqual(expectedIndices(FRAMES[i]?.rgba ?? new Uint8ClampedArray()));
    });
  });

  it("a frame that fills the code table still decodes", () => {
    const side = 300;
    // A seeded linear congruential stream: noise the table cannot compress, so it fills and resets.
    let state = 0x2545f491;
    const noise = frame(side, side, () => {
      const next = (): number => {
        state = (Math.imul(state, 1664525) + 1013904223) >>> 0;
        return state >>> 24;
      };
      return [next(), next(), next()];
    });
    const encoder = createGifEncoder(side, side);
    encoder.add(noise, 7);
    encoder.add(noise, 9);
    const gif = decode(encoder.finish());
    expect(gif.frames).toHaveLength(2);
    for (const decoded of gif.frames) expect(Array.from(decoded.indices)).toEqual(expectedIndices(noise));
    expect(gif.frames.map((f) => f.delayCs)).toEqual([7, 9]);
  });

  it("a flat frame, one long run, decodes", () => {
    const flat = frame(64, 48, () => [255, 0, 0]);
    const encoder = createGifEncoder(64, 48);
    encoder.add(flat, 1);
    const gif = decode(encoder.finish());
    expect(Array.from(gif.frames[0]?.indices ?? [])).toEqual(expectedIndices(flat));
  });

  it("the palette's own colours map to themselves", () => {
    expect(GIF_PALETTE).toHaveLength(768);
    for (let i = 0; i < 252; i += 1) {
      expect(paletteIndex(GIF_PALETTE[i * 3] ?? -1, GIF_PALETTE[i * 3 + 1] ?? -1, GIF_PALETTE[i * 3 + 2] ?? -1)).toBe(i);
    }
    expect(paletteIndex(0, 0, 0)).toBe(0);
    expect(paletteIndex(255, 255, 255)).toBe(251);
    // The four entries past the cube are black and nothing maps to them.
    expect(Array.from(GIF_PALETTE.subarray(252 * 3))).toEqual(new Array<number>(12).fill(0));
  });

  it("refuses frames and delays that do not fit, and a finished encoder", () => {
    const encoder = createGifEncoder(2, 2);
    expect(() => {
      encoder.add(new Uint8ClampedArray(12), 1);
    }).toThrow(RangeError);
    const pixels = new Uint8ClampedArray(16);
    for (const delay of [-1, 1.5, 65536, Number.NaN]) {
      expect(() => {
        encoder.add(pixels, delay);
      }).toThrow(RangeError);
    }
    const before = encoder.size;
    encoder.add(pixels, 0);
    expect(encoder.size).toBeGreaterThan(before);
    encoder.finish();
    expect(() => encoder.finish()).toThrow();
    expect(() => {
      encoder.add(pixels, 1);
    }).toThrow();
    expect(() => createGifEncoder(0, 5)).toThrow(RangeError);
  });
});
