// R1011000, issue #1011: the learned evaluation's integer network gives, in the WebAssembly build,
// exactly the corrections the Python reference computed for the parity fixture, bit for bit, as
// `cargo test -p jackioh-ai` holds the native build to the same fixture.

import { readFileSync } from "node:fs";
import { dirname, join } from "node:path";
import { fileURLToPath } from "node:url";

import { describe, expect, it } from "vitest";

import { learnedParity } from "./index.ts";

const here = dirname(fileURLToPath(import.meta.url));
const FIXTURE = join(here, "../../../../crates/ai/tests/fixtures/learned-parity.json");

type Fixture = { cases: { expected: string }[] };

describe("R1011000 the learned evaluation's parity fixture", () => {
  it("R1011000 the WASM build gives the reference's corrections bit for bit", () => {
    const text = readFileSync(FIXTURE, "utf8");
    const fixture = JSON.parse(text) as Fixture;
    const answers = learnedParity(text);
    expect(answers).toHaveLength(fixture.cases.length);
    expect(fixture.cases.length).toBe(40);
    fixture.cases.forEach((one, at) => {
      // The expected value is the shortest text that reads back as its double: Number() reads it
      // exactly, and Object.is tells -0 from 0 and compares the bits otherwise.
      expect(Object.is(answers[at], Number(one.expected)), `case ${at}: ${answers[at]} against ${one.expected}`).toBe(true);
    });
  });
});
