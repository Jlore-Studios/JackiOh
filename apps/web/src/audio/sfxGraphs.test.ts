// B14 (docs/polish/2-sound.md), R3405001, R3407001: each recipe's graph as it was tuned by ear, a golden per
// sound id. sfx.test.ts holds the recipe contract (nothing before `at`, every source stopped, only `out`), which
// a sound wired or timed differently still keeps; this holds what a sound is made of: every node the recipe
// makes on the fake context (its kind, type, loop and buffer), when each source starts and stops after `at`,
// where each node connects (`#i` a node it made, `out`, `#i.frequency` a param), each param's every value and
// automation, and the length it returns. A sound tuned on purpose updates its golden with
// `pnpm --dir apps/web exec vitest run src/audio/sfxGraphs.test.ts -u`, and the diff shows what moved.

import { describe, expect, it } from "vitest";

import { SFX, SFX_IDS } from "./sfx.ts";
import { FakeAudio, FakeParam, type FakeNode, type ParamEvent } from "./test/fakeAudio.ts";
import type { SfxId } from "./types.ts";

/** The recipe is asked to start here, after the context's own clock, so every time prints as an offset from it. */
const NOW = 1;
const AT = 1.5;

/** A number as the golden prints it: fixed to six places. */
const num = (value: number): string => value.toFixed(6);

/** A time as an offset from `AT`. */
const after = (time: number): string => num(time - AT);

function eventText(event: ParamEvent): string {
  switch (event.method) {
    case "value":
      return `= ${num(event.value)}`;
    case "setValueAtTime":
      return `set ${num(event.value)} @${after(event.time)}`;
    case "linearRampToValueAtTime":
      return `linear ${num(event.value)} @${after(event.time)}`;
    case "exponentialRampToValueAtTime":
      return `exponential ${num(event.value)} @${after(event.time)}`;
    case "setTargetAtTime":
      return `target ${num(event.value)} @${after(event.time)} τ${num(event.timeConstant)}`;
    case "cancelScheduledValues":
      return `cancel @${after(event.time)}`;
  }
}

/** `id`'s recipe run once at `AT` with no parameters, printed node by node in the order it made them. */
function graphOf(id: SfxId): string {
  const audio = new FakeAudio({ state: "running", currentTime: NOW });
  const out = audio.context.createGain();
  const outNode = audio.nodeOf(out);
  const first = audio.nodes.length;
  const length = SFX[id].recipe(audio.context, out, AT, {});
  const made = audio.nodes.slice(first);
  const name = (target: FakeNode | FakeParam): string => {
    if (target instanceof FakeParam) return `${name(target.node)}.${target.name}`;
    if (target === outNode) return "out";
    const k = made.indexOf(target);
    return k < 0 ? `${target.kind}#${String(target.index)}` : `#${String(k)}`;
  };
  const lines = [`length ${num(length)}`];
  made.forEach((node, k) => {
    const parts = [`#${String(k)} ${node.kind}`];
    if (node.type !== null) parts.push(node.type);
    if (node.loop) parts.push("loop");
    if (node.buffer !== null) parts.push(`buffer ${String(node.buffer.length)}`);
    if (node.startTime !== null) parts.push(`start ${after(node.startTime)}`);
    if (node.startOffset !== null) parts.push(`offset ${num(node.startOffset)}`);
    if (node.playDuration !== null) parts.push(`for ${num(node.playDuration)}`);
    if (node.stopTime !== null) parts.push(`stop ${after(node.stopTime)}`);
    parts.push(`-> ${node.connections.length === 0 ? "nothing" : node.connections.map(name).join(", ")}`);
    lines.push(parts.join(" "));
    for (const [param, record] of node.params) {
      if (record.events.length > 0) lines.push(`  ${param}: ${record.events.map(eventText).join("; ")}`);
    }
  });
  expect(audio.violations, id).toEqual([]);
  return lines.join("\n");
}

describe("B14 R3405001 R3407001 each recipe builds the graph it was tuned to by ear", () => {
  it.each(SFX_IDS)("B14 %s: every node, wire, start, stop and automation as recorded", (id) => {
    expect(graphOf(id)).toMatchSnapshot();
  });
});
