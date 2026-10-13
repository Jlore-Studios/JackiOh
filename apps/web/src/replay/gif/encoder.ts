// A GIF89a encoder for a shared replay (#513, R513000): RGBA frames in, bytes out, the same bytes
// for the same frames on every run. It is written by hand instead of pulled in as a dependency: a
// fixed 6x7x6 colour cube for the palette (no per-frame palette search and no dithering, so nothing
// can differ between runs), each frame LZW-compressed with the GIF variant of the code table, and
// the Netscape loop extension so the clip repeats. It loads on demand, with the raster (export.ts),
// so the main bundle does not carry it.

const RED_LEVELS = 6;
const GREEN_LEVELS = 7;
const BLUE_LEVELS = 6;
const CUBE_COLOURS = RED_LEVELS * GREEN_LEVELS * BLUE_LEVELS;
/** A GIF palette is a power of two long: the cube, then black to fill it. */
const PALETTE_SIZE = 256;
const CHANNEL_MAX = 255;
const RED_STRIDE = GREEN_LEVELS * BLUE_LEVELS;

/** Entry `i` of the palette is the cube's `i`th colour, red the slowest axis; entries past the cube are black. */
export const GIF_PALETTE: Uint8Array<ArrayBuffer> = (() => {
  const palette = new Uint8Array(PALETTE_SIZE * 3);
  for (let i = 0; i < CUBE_COLOURS; i += 1) {
    palette[i * 3] = Math.round((Math.floor(i / RED_STRIDE) * CHANNEL_MAX) / (RED_LEVELS - 1));
    palette[i * 3 + 1] = Math.round(((Math.floor(i / BLUE_LEVELS) % GREEN_LEVELS) * CHANNEL_MAX) / (GREEN_LEVELS - 1));
    palette[i * 3 + 2] = Math.round(((i % BLUE_LEVELS) * CHANNEL_MAX) / (BLUE_LEVELS - 1));
  }
  return palette;
})();

/** Each channel value's nearest level on its axis, worked out once. */
function levels(count: number): Uint8Array {
  return Uint8Array.from({ length: CHANNEL_MAX + 1 }, (_, value) => Math.round((value * (count - 1)) / CHANNEL_MAX));
}
const RED_LEVEL = levels(RED_LEVELS);
const GREEN_LEVEL = levels(GREEN_LEVELS);
const BLUE_LEVEL = levels(BLUE_LEVELS);

/** The palette entry nearest a colour: each channel snaps to its own axis. */
export function paletteIndex(r: number, g: number, b: number): number {
  return (RED_LEVEL[r] ?? 0) * RED_STRIDE + (GREEN_LEVEL[g] ?? 0) * BLUE_LEVELS + (BLUE_LEVEL[b] ?? 0);
}

const LZW_MIN_CODE_SIZE = 8;
const CLEAR_CODE = 1 << LZW_MIN_CODE_SIZE;
const END_CODE = CLEAR_CODE + 1;
const FIRST_FREE_CODE = CLEAR_CODE + 2;
const MAX_CODE_SIZE = 12;
const CODE_LIMIT = 1 << MAX_CODE_SIZE;
const MAX_BLOCK = 255;
const MAX_DELAY_CS = 0xffff;
const MAX_DIMENSION = 0xffff;
const INITIAL_CAPACITY = 1 << 16;

export type GifEncoder = {
  /** Appends a frame shown for `delayCs` centiseconds; throws a `RangeError` for pixels of the wrong length or a delay out of range. */
  add(rgba: Uint8ClampedArray, delayCs: number): void;
  /** The bytes written so far, the header included. */
  readonly size: number;
  /** Closes the file and returns its bytes; the encoder is spent after it. */
  finish(): Uint8Array<ArrayBuffer>;
};

export function createGifEncoder(width: number, height: number): GifEncoder {
  if (!Number.isInteger(width) || !Number.isInteger(height) || width < 1 || height < 1 || width > MAX_DIMENSION || height > MAX_DIMENSION) {
    throw new RangeError(`a GIF of ${String(width)} by ${String(height)} pixels`);
  }
  let out: Uint8Array<ArrayBuffer> = new Uint8Array(INITIAL_CAPACITY);
  let length = 0;
  let finished = false;

  const room = (extra: number): void => {
    if (length + extra <= out.length) return;
    let capacity = out.length;
    while (capacity < length + extra) capacity *= 2;
    const grown = new Uint8Array(capacity);
    grown.set(out.subarray(0, length));
    out = grown;
  };
  const byte = (value: number): void => {
    room(1);
    out[length] = value;
    length += 1;
  };
  const u16 = (value: number): void => {
    byte(value & 0xff);
    byte(value >> 8);
  };
  const ascii = (value: string): void => {
    for (let i = 0; i < value.length; i += 1) byte(value.charCodeAt(i));
  };

  ascii("GIF89a");
  u16(width);
  u16(height);
  // A global colour table of 256 entries (flag, then size 7 = 2^(7+1)), background 0, no aspect ratio.
  byte(0xf7);
  byte(0);
  byte(0);
  room(GIF_PALETTE.length);
  out.set(GIF_PALETTE, length);
  length += GIF_PALETTE.length;
  // The Netscape loop: for ever.
  byte(0x21);
  byte(0xff);
  byte(0x0b);
  ascii("NETSCAPE2.0");
  byte(0x03);
  byte(0x01);
  u16(0);
  byte(0);

  /** The LZW codes of one frame's palette indices, packed least significant bit first into sub-blocks. */
  function lzw(indices: Uint8Array): void {
    const block = new Uint8Array(MAX_BLOCK);
    let blockLength = 0;
    let bits = 0;
    let bitCount = 0;
    const flushBlock = (): void => {
      if (blockLength === 0) return;
      byte(blockLength);
      room(blockLength);
      out.set(block.subarray(0, blockLength), length);
      length += blockLength;
      blockLength = 0;
    };
    const pack = (value: number): void => {
      block[blockLength] = value;
      blockLength += 1;
      if (blockLength === MAX_BLOCK) flushBlock();
    };
    let size = LZW_MIN_CODE_SIZE + 1;
    const emit = (code: number): void => {
      bits |= code << bitCount;
      bitCount += size;
      while (bitCount >= 8) {
        pack(bits & 0xff);
        bits >>>= 8;
        bitCount -= 8;
      }
    };

    let table = new Map<number, number>();
    let next = FIRST_FREE_CODE;
    emit(CLEAR_CODE);
    let prefix = indices[0] ?? 0;
    for (let i = 1; i < indices.length; i += 1) {
      const k = indices[i] ?? 0;
      const key = prefix * 256 + k;
      const hit = table.get(key);
      if (hit !== undefined) {
        prefix = hit;
        continue;
      }
      emit(prefix);
      if (next === CODE_LIMIT) {
        emit(CLEAR_CODE);
        table = new Map();
        next = FIRST_FREE_CODE;
        size = LZW_MIN_CODE_SIZE + 1;
      } else {
        if (next >= 1 << size) size += 1;
        table.set(key, next);
        next += 1;
      }
      prefix = k;
    }
    emit(prefix);
    emit(END_CODE);
    if (bitCount > 0) pack(bits & 0xff);
    flushBlock();
    byte(0);
  }

  return {
    add(rgba, delayCs) {
      if (finished) throw new Error("the GIF is already finished");
      if (rgba.length !== width * height * 4) throw new RangeError(`${String(rgba.length)} bytes for a ${String(width)} by ${String(height)} frame`);
      if (!Number.isInteger(delayCs) || delayCs < 0 || delayCs > MAX_DELAY_CS) throw new RangeError(`a delay of ${String(delayCs)} centiseconds`);
      // Graphic control: no disposal, no transparency, the delay.
      byte(0x21);
      byte(0xf9);
      byte(0x04);
      byte(0x00);
      u16(delayCs);
      byte(0x00);
      byte(0x00);
      // Image descriptor: the whole canvas, no local palette.
      byte(0x2c);
      u16(0);
      u16(0);
      u16(width);
      u16(height);
      byte(0x00);
      byte(LZW_MIN_CODE_SIZE);
      const indices = new Uint8Array(width * height);
      for (let i = 0; i < indices.length; i += 1) {
        indices[i] = paletteIndex(rgba[i * 4] ?? 0, rgba[i * 4 + 1] ?? 0, rgba[i * 4 + 2] ?? 0);
      }
      lzw(indices);
    },
    get size() {
      return length;
    },
    finish() {
      if (finished) throw new Error("the GIF is already finished");
      finished = true;
      byte(0x3b);
      return out.slice(0, length);
    },
  };
}
