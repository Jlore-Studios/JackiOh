// apps/web/scripts/json-layout.mjs (R5289000): the record-per-line layout gen-voice.mjs writes the voice
// manifest in, held to crates/tools/src/json_layout.rs byte for byte: the Rust's own cases (format_json.rs,
// `r5289000_lays_wrappers_out_above_the_record_depth_and_each_record_on_one_line`) lay out the same here,
// and the committed manifest, which `cargo jackioh format-json --check` holds to the Rust, is what the
// script writes from it. Run as a script, as gen-voice.mjs runs: a TypeScript import of an `.mjs` has no
// types.

import { spawnSync } from "node:child_process";
import { readFileSync } from "node:fs";
import { dirname, join } from "node:path";
import { fileURLToPath, pathToFileURL } from "node:url";

import { describe, expect, it } from "vitest";

const WEB = join(dirname(fileURLToPath(import.meta.url)), "../..");
const SCRIPT = join(WEB, "scripts/json-layout.mjs");
const MANIFEST = join(WEB, "src/audio/voice-manifest.json");

/** Lays stdin out at the depth given, or at the script's own VOICE_MANIFEST_DEPTH when none is. */
const PROGRAM = [
  'import { readFileSync } from "node:fs";',
  `import { layout, VOICE_MANIFEST_DEPTH } from ${JSON.stringify(pathToFileURL(SCRIPT).href)};`,
  "const depth = process.argv[1] === undefined ? VOICE_MANIFEST_DEPTH : Number(process.argv[1]);",
  'process.stdout.write(layout(readFileSync(0, "utf8"), depth));',
].join("\n");

/** Enough for the manifest, several times over. */
const OUTPUT_MAX_BYTES = 16 * 1024 * 1024;

function layout(json: string, depth?: number): string {
  const args = depth === undefined ? [] : [String(depth)];
  const run = spawnSync(process.execPath, ["--input-type=module", "-e", PROGRAM, ...args], {
    input: json,
    encoding: "utf8",
    maxBuffer: OUTPUT_MAX_BYTES,
  });
  expect(run.status, run.stderr).toBe(0);
  return run.stdout;
}

describe("json-layout.mjs", () => {
  it("R5289000 lays each case out as json_layout.rs does", () => {
    expect(layout('{"a":{"x":[1,2],"y":{"z":null}},"b":[]}', 1)).toBe(
      '{\n  "a": {"x":[1,2],"y":{"z":null}},\n  "b": []\n}\n',
    );
    expect(layout('{"set":{"c1":{"id":"c1","tags":["A"]},"c2":{}},"unset":["c3","c4"]}', 2)).toBe(
      '{\n  "set": {\n    "c1": {"id":"c1","tags":["A"]},\n    "c2": {}\n  },\n  "unset": ["c3","c4"]\n}\n',
    );
    expect(layout('["a","b"]', 1)).toBe('[\n  "a",\n  "b"\n]\n');
    expect(layout("{}", 1)).toBe("{}\n");
    const string = String.raw`"a, [b]: {c} \"d\" \\ e"`;
    expect(layout(string, 2)).toBe(`${string}\n`);
    expect(layout(`{"k": ${string}, "l": [${string}]}`, 1)).toBe(`{\n  "k": ${string},\n  "l": [${string}]\n}\n`);
    expect(layout('{"a": [ 1.0 ,\n 2e3 ] }', 0)).toBe('{"a":[1.0,2e3]}\n');
    expect(layout('{"zh":{"name":"大D守卫","say":"🎺 \\"嘟\\""}}', 1)).toBe(
      '{\n  "zh": {"name":"大D守卫","say":"🎺 \\"嘟\\""}\n}\n',
    );
    const value = { set: { c1: { id: "c1", tags: ["A", "B"], base: { text: 'Hi, {x}: "y"' } } }, unset: ["c2"], n: 1 };
    for (const depth of [0, 1, 2, 3]) {
      expect(layout(JSON.stringify(value, null, 2), depth)).toBe(layout(JSON.stringify(value), depth));
    }
  });

  it("R5289000 lays the committed voice manifest out as it stands", () => {
    const text = readFileSync(MANIFEST, "utf8");
    expect(layout(JSON.stringify(JSON.parse(text)))).toBe(text);
  });
});
