import { it } from "vitest";
import { appendFileSync, writeFileSync } from "node:fs";
import { galleryRun, describeRun, ceilingOf, latestEnd } from "../test/signaturePass.ts";
it("scratch", () => {
  writeFileSync("/tmp/run.txt", "");
  for (const defId of (process.env.IDS ?? "stinkyscape-018-1").split(",")) {
    const { run } = galleryRun(defId);
    appendFileSync("/tmp/run.txt", `${defId} ceiling ${ceilingOf(defId)} latest ${latestEnd(run)}\n${describeRun(run)}\n`);
  }
});
