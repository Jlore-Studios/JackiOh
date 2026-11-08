import { it } from "vitest";
import { writeFileSync } from "node:fs";
import { fullBoardView } from "../test/fixtures.ts";
import { describeEntry, entryOf } from "../test/setPass.ts";
import { motifScale } from "../fx/motifs/build.ts";
import { STINKYSCAPE_TWO_SIGNATURES } from "../fx/cards/stinkyscape/signature.ts";
it("scratch", () => {
  const view = fullBoardView();
  writeFileSync("/tmp/scale.txt", STINKYSCAPE_TWO_SIGNATURES.map((id) => `${id}\t${describeEntry(entryOf(id))}\t${motifScale(id, view, 1).toFixed(2)}`).join("\n") + "\n");
});
