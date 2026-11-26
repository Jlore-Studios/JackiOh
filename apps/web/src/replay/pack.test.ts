// R5124000 (amending R1141000, R1242000): a downloaded replay file is packed (each step as its change
// from the one before, gzipped) and opens again as the very steps it was made of; a broken or
// oversized one is refused.

import fc from "fast-check";
import { describe, expect, it } from "vitest";

import { foldedReplaySteps } from "../test/replay.ts";
import { REPLAY_JSON_MAX_BYTES, REPLAY_JSON_MAX_STEPS } from "./config.ts";
import { NOT_A_REPLAY, PACKED_STEPS_KEY, TOO_BIG, apply, diff, isGzip, packSteps, packedReplay, replayText, unpackSteps, unpackedReplay } from "./pack.ts";
import { ReplayFailure } from "./source.ts";

/** The share of its plain JSON a folded game's packed file may take at most (measured near 1/50). */
const PACKED_SHARE_AT_MOST = 1 / 20;

const steps = foldedReplaySteps();

/** `value` as a file holds it: written to JSON and read back. */
function throughJson<T>(value: T): T {
  return JSON.parse(JSON.stringify(value)) as T;
}

/** The sentence `run` is refused with. */
async function refusal(run: () => unknown): Promise<string> {
  const failure: unknown = await Promise.resolve()
    .then(run)
    .then(
      () => null,
      (cause: unknown) => cause,
    );
  if (!(failure instanceof ReplayFailure)) throw new Error(`not refused: ${String(failure)}`);
  return failure.message;
}

/** `bytes` gzipped by the browser's own stream. */
async function gzip(bytes: Uint8Array<ArrayBuffer>): Promise<Uint8Array<ArrayBuffer>> {
  const stream = new CompressionStream("gzip");
  const writer = stream.writable.getWriter();
  const written = writer.write(bytes).then(() => writer.close());
  const zipped = new Uint8Array(await new Response(stream.readable).arrayBuffer());
  await written;
  return zipped;
}

/** JSON values with no key that may never become an object key (`isSafeKey`). */
const json = fc.jsonValue().filter((value) => !/"(?:__proto__|constructor|prototype)":/u.test(JSON.stringify(value)));

describe("R5124000 a replay file's packed steps", () => {
  it("R5124000 a folded game's steps unpack to the very steps they were packed from", () => {
    expect(unpackSteps(throughJson(packSteps(steps)))).toEqual([...steps]);
  });

  it("R5124000 any JSON value changed by its diff from another is that other, through a file's JSON", () => {
    fc.assert(
      fc.property(json, json, (prev, next) => {
        const change = diff(prev, next);
        const rebuilt = change === undefined ? prev : apply(prev, throughJson(change));
        expect(throughJson(rebuilt)).toEqual(throughJson(next));
      }),
    );
  });

  it("R5124000 a changed array element is a change, not the element again, and kept elements cost nothing", () => {
    const card = { instanceId: "c1", attack: 2, health: 3, keywords: ["Armor"], text: "a long rules text ".repeat(4) };
    const prev = [card, { instanceId: "c2" }, { instanceId: "c3" }];
    expect(diff(prev, [{ ...card, health: 1 }, ...prev.slice(1)])).toEqual({ e: [{ health: [1] }] });
    expect(diff(prev, [card, { instanceId: "c3" }, { instanceId: "c4" }])).toEqual({ e: [1, -1, 1, [{ instanceId: "c4" }]] });
  });

  it("R5124000 the file is gzip and opens again as the same object, at a small share of its plain JSON", async () => {
    const file = { format: "jackioh-replay", version: 4, catalogVersion: "v", savedAt: 0, game: {}, steps };
    const bytes = await packedReplay(file);
    expect(isGzip(bytes)).toBe(true);
    const opened = unpackedReplay(JSON.parse(await replayText(bytes)) as Record<string, unknown>);
    expect(opened).toEqual(file);
    expect(PACKED_STEPS_KEY in opened).toBe(false);
    expect(bytes.byteLength).toBeLessThan(JSON.stringify(file).length * PACKED_SHARE_AT_MOST);
  });

  it("R5124000 plain JSON bytes read as their text, and an object with no packed steps is left as it is", async () => {
    const plain = { steps: [1, 2] };
    expect(await replayText(new TextEncoder().encode(JSON.stringify(plain)))).toBe(JSON.stringify(plain));
    expect(unpackedReplay(plain)).toBe(plain);
  });

  it("R5124000 gzip that unzips past REPLAY_JSON_MAX_BYTES is refused as too big, and broken gzip as no replay", async () => {
    const bomb = await gzip(new Uint8Array(REPLAY_JSON_MAX_BYTES + 1));
    expect(bomb.byteLength).toBeLessThan(REPLAY_JSON_MAX_BYTES);
    expect(await refusal(() => replayText(bomb))).toBe(TOO_BIG);
    const broken = new Uint8Array([0x1f, 0x8b, 1, 2, 3, 4, 5, 6, 7, 8]);
    expect(await refusal(() => replayText(broken))).toBe(NOT_A_REPLAY);
  });

  it("R5124000 more packed steps than REPLAY_JSON_MAX_STEPS are refused before any is rebuilt", async () => {
    const many = [{ step: 0 }, ...Array.from({ length: REPLAY_JSON_MAX_STEPS }, () => ({ step: [1] }))];
    expect(await refusal(() => unpackedReplay({ [PACKED_STEPS_KEY]: many }))).toBe(TOO_BIG);
    expect(unpackSteps(many.slice(0, REPLAY_JSON_MAX_STEPS))).toHaveLength(REPLAY_JSON_MAX_STEPS);
  });

  it("R5124000 a change of any other shape is refused as no replay", async () => {
    const broken: [unknown, unknown][] = [
      [{ a: 1 }, [1, 2]],
      [{ a: 1 }, 7],
      [[1, 2], { e: "ops" }],
      [[1, 2], { e: [2, { a: [1] }] }],
      [[1, 2], { e: [0.5] }],
      [3, { a: [1] }],
      [{ a: 1 }, JSON.parse('{"__proto__": [{"polluted": true}]}')],
      [{ a: 1 }, { constructor: [1] }],
    ];
    for (const [prev, change] of broken) {
      expect(await refusal(() => apply(prev, change)), JSON.stringify(change)).toBe(NOT_A_REPLAY);
    }
    for (const packed of [[], "steps", null]) {
      expect(await refusal(() => unpackedReplay({ [PACKED_STEPS_KEY]: packed })), JSON.stringify(packed)).toBe(NOT_A_REPLAY);
    }
  });
});
