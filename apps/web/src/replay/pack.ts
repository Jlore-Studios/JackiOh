// Packing a downloaded replay file (R5124000, amending R1141000 and R1242000): its steps as the first one whole and each later
// one as its change from the one before, gzipped by the browser's own CompressionStream. A replay's
// steps are full views that differ by a few fields and a few new events, so the change is a small
// fraction of a view, and gzip squeezes what is left: a game's file is a fiftieth to a hundredth of its
// plain JSON, the longer games the smaller share. A file is untrusted, so reading one holds it to `REPLAY_JSON_MAX_BYTES` unzipped and
// `REPLAY_JSON_MAX_STEPS` steps, and refuses a change that does not apply as `NOT_A_REPLAY`.

import { isRecord, isSafeKey } from "../wire/json.ts";
import { REPLAY_JSON_MAX_BYTES, REPLAY_JSON_MAX_STEPS } from "./config.ts";
import { ReplayFailure } from "./source.ts";

/** The key a packed file holds its steps under, in place of `steps`. */
export const PACKED_STEPS_KEY = "packedSteps";

/** The two bytes every gzip stream starts with (RFC 1952). */
const GZIP_MAGIC = [0x1f, 0x8b] as const;

/** The refusals' sentences, kept here so `download.ts` can name them without a cycle. */
export const TOO_BIG = "This replay file is too big to open.";
export const NOT_A_REPLAY = "This file is not a JackiOh replay file.";
export const UNSUPPORTED = "This replay file uses an unsupported format.";

/**
 * How one JSON value became the next: `[]` drops an object's key, `[value]` sets a value whole, an
 * object changes an object key by key, and `{ e: ops }` changes an array, whose ops walk the old
 * array: a positive n keeps n elements, a negative n drops n, an array inserts its elements and an
 * object changes the next element. Old elements past the last op are kept.
 */
export type Change = unknown[] | Record<string, unknown>;
type ArrayOp = number | unknown[] | Record<string, unknown>;

const text = JSON.stringify;

/** How `next` differs from `prev`, or undefined when they are the same JSON. */
export function diff(prev: unknown, next: unknown): Change | undefined {
  if (text(prev) === text(next)) return undefined;
  if (isRecord(prev) && isRecord(next)) {
    const change: Record<string, unknown> = {};
    for (const key of Object.keys(prev)) if (!Object.hasOwn(next, key)) change[key] = [];
    for (const [key, value] of Object.entries(next)) {
      const each = Object.hasOwn(prev, key) ? diff(prev[key], value) : [value];
      if (each !== undefined) change[key] = each;
    }
    return change;
  }
  if (Array.isArray(prev) && Array.isArray(next)) return { e: arrayOps(prev, next) };
  return [next];
}

/**
 * The ops that walk `prev` into `next`: the elements both keep in order (their longest common
 * run, by JSON), and between them each dropped element paired with an inserted one as a change
 * when the change is shorter than the element, so a card whose stats moved is a few bytes.
 */
function arrayOps(prev: readonly unknown[], next: readonly unknown[]): ArrayOp[] {
  const before = prev.map((value) => text(value));
  const after = next.map((value) => text(value));
  const common = Array.from({ length: prev.length + 1 }, () => new Array<number>(next.length + 1).fill(0));
  for (let i = prev.length - 1; i >= 0; i--) {
    for (let j = next.length - 1; j >= 0; j--) {
      common[i]![j] = before[i] === after[j] ? common[i + 1]![j + 1]! + 1 : Math.max(common[i + 1]![j]!, common[i]![j + 1]!);
    }
  }
  const ops: ArrayOp[] = [];
  const push = (op: ArrayOp): void => {
    const last = ops.at(-1);
    if (typeof op === "number" && typeof last === "number" && Math.sign(op) === Math.sign(last)) ops[ops.length - 1] = last + op;
    else if (Array.isArray(op) && Array.isArray(last)) last.push(...op);
    else ops.push(op);
  };
  let dropped: number[] = [];
  let inserted: number[] = [];
  const settle = (): void => {
    const paired = Math.min(dropped.length, inserted.length);
    for (let k = 0; k < paired; k++) {
      const [from, to] = [dropped[k]!, inserted[k]!];
      const change = diff(prev[from], next[to]);
      if (isRecord(change) && text(change).length < after[to]!.length) {
        push(change);
      } else {
        push(-1);
        push([next[to]]);
      }
    }
    if (dropped.length > paired) push(paired - dropped.length);
    if (inserted.length > paired) push(inserted.slice(paired).map((to) => next[to]));
    dropped = [];
    inserted = [];
  };
  let i = 0;
  let j = 0;
  while (i < prev.length || j < next.length) {
    if (i < prev.length && j < next.length && before[i] === after[j]) {
      settle();
      push(1);
      i++;
      j++;
    } else if (j < next.length && (i === prev.length || common[i]![j + 1]! >= common[i + 1]![j]!)) {
      inserted.push(j++);
    } else {
      dropped.push(i++);
    }
  }
  settle();
  const last = ops.at(-1);
  if (typeof last === "number" && last > 0) ops.pop();
  return ops;
}

/** `prev` changed by `change`; throws `NOT_A_REPLAY` on a change that is not `diff`'s shape. */
export function apply(prev: unknown, change: unknown): unknown {
  if (Array.isArray(change)) {
    if (change.length !== 1) throw new ReplayFailure("error", NOT_A_REPLAY);
    return change[0];
  }
  if (!isRecord(change)) throw new ReplayFailure("error", NOT_A_REPLAY);
  if (Array.isArray(prev)) {
    const { e: ops } = change;
    if (!Array.isArray(ops)) throw new ReplayFailure("error", NOT_A_REPLAY);
    const out: unknown[] = [];
    let at = 0;
    for (const op of ops as unknown[]) {
      if (typeof op === "number") {
        if (!Number.isSafeInteger(op)) throw new ReplayFailure("error", NOT_A_REPLAY);
        for (let k = 0; k < op && at + k < prev.length; k++) out.push(prev[at + k]);
        at += Math.abs(op);
      } else if (Array.isArray(op)) {
        for (const value of op) out.push(value);
      } else {
        if (at >= prev.length) throw new ReplayFailure("error", NOT_A_REPLAY);
        out.push(apply(prev[at++], op));
      }
    }
    for (; at < prev.length; at++) out.push(prev[at]);
    return out;
  }
  if (!isRecord(prev)) throw new ReplayFailure("error", NOT_A_REPLAY);
  const out: Record<string, unknown> = { ...prev };
  for (const [key, each] of Object.entries(change)) {
    if (!isSafeKey(key)) throw new ReplayFailure("error", NOT_A_REPLAY);
    if (Array.isArray(each) && each.length === 0) delete out[key];
    else out[key] = apply(out[key], each);
  }
  return out;
}

/** `steps` packed: the first whole, each later one as its change from the one before. */
export function packSteps(steps: readonly unknown[]): unknown[] {
  return steps.map((step, index) => (index === 0 ? step : (diff(steps[index - 1], step) ?? {})));
}

/** The steps `packSteps` packed, held to `REPLAY_JSON_MAX_STEPS` before any is rebuilt. */
export function unpackSteps(packed: unknown): unknown[] {
  if (!Array.isArray(packed) || packed.length === 0) throw new ReplayFailure("error", NOT_A_REPLAY);
  if (packed.length > REPLAY_JSON_MAX_STEPS) throw new ReplayFailure("error", TOO_BIG);
  const steps: unknown[] = [packed[0]];
  for (const change of packed.slice(1) as unknown[]) {
    const prev = steps.at(-1);
    steps.push(isRecord(change) && Object.keys(change).length === 0 ? prev : apply(prev, change));
  }
  return steps;
}

/** Whether this browser can write a packed file: one without CompressionStream saves plain JSON. */
export function canPack(): boolean {
  return typeof CompressionStream === "function";
}

/** `json`, its `steps` packed under `PACKED_STEPS_KEY`, as gzipped bytes. */
export async function packedReplay(json: { steps: readonly unknown[] }): Promise<Uint8Array<ArrayBuffer>> {
  const { steps, ...rest } = json;
  const bytes = new TextEncoder().encode(text({ ...rest, [PACKED_STEPS_KEY]: packSteps(steps) }));
  return through(new CompressionStream("gzip"), bytes, Number.POSITIVE_INFINITY);
}

/** Whether `bytes` start as a gzip stream does. */
export function isGzip(bytes: Uint8Array): boolean {
  return bytes[0] === GZIP_MAGIC[0] && bytes[1] === GZIP_MAGIC[1];
}

/**
 * The JSON text a replay file's bytes hold: gunzipped when they are gzip, refused as `TOO_BIG` the
 * moment the text passes `REPLAY_JSON_MAX_BYTES`, as `UNSUPPORTED` where the browser cannot unzip
 * and as `NOT_A_REPLAY` when the gzip is broken.
 */
export async function replayText(bytes: Uint8Array<ArrayBuffer>): Promise<string> {
  if (!isGzip(bytes)) return new TextDecoder().decode(bytes);
  if (typeof DecompressionStream !== "function") throw new ReplayFailure("error", UNSUPPORTED);
  const unzipped = await through(new DecompressionStream("gzip"), bytes, REPLAY_JSON_MAX_BYTES).catch((cause: unknown) => {
    throw cause instanceof ReplayFailure ? cause : new ReplayFailure("error", NOT_A_REPLAY);
  });
  return new TextDecoder().decode(unzipped);
}

/**
 * The JSON object a packed file holds with its steps rebuilt under `steps`, as the plain file holds
 * them; any other object as it is.
 */
export function unpackedReplay(json: Record<string, unknown>): Record<string, unknown> {
  if (!Object.hasOwn(json, PACKED_STEPS_KEY)) return json;
  const { [PACKED_STEPS_KEY]: packed, ...rest } = json;
  try {
    return { ...rest, steps: unpackSteps(packed) };
  } catch (cause) {
    // A change nested past the stack's depth is a broken file too.
    throw cause instanceof ReplayFailure ? cause : new ReplayFailure("error", NOT_A_REPLAY);
  }
}

/** `bytes` through a (de)compression stream, refused as `TOO_BIG` once its output passes `cap`. */
async function through(stream: CompressionStream | DecompressionStream, bytes: Uint8Array<ArrayBuffer>, cap: number): Promise<Uint8Array<ArrayBuffer>> {
  const writer = stream.writable.getWriter();
  const written = writer.write(bytes).then(() => writer.close());
  // A broken stream fails the read below too, which is the failure reported.
  written.catch(() => undefined);
  const reader = stream.readable.getReader();
  const chunks: Uint8Array[] = [];
  let size = 0;
  for (;;) {
    const { done, value } = await reader.read();
    if (done) break;
    size += value.byteLength;
    if (size > cap) {
      await reader.cancel();
      throw new ReplayFailure("error", TOO_BIG);
    }
    chunks.push(value);
  }
  await written;
  const out = new Uint8Array(size);
  let at = 0;
  for (const chunk of chunks) {
    out.set(chunk, at);
    at += chunk.byteLength;
  }
  return out;
}
