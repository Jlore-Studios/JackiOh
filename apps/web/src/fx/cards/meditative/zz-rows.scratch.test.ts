// SCRATCH: delete. Prints meditative.md's Entry and Scale for the 32.
import { it } from "vitest";
import { writeFileSync } from "node:fs";
import { fullBoardView } from "../../../test/fixtures.ts";
import { motifScale } from "../../motifs/build.ts";
import { FX_CARD_TABLE, type FxCardEntry } from "../tables.ts";
import { FX_CARD_HOOKS } from "../../motifs/names.ts";
function entryText(entry: FxCardEntry): string {
  if (entry.kind === "none") return `none: ${entry.reason}`;
  if (entry.kind === "bespoke") return `bespoke: ${entry.key}`;
  if (entry.kind === "setPiece") return `setPiece: ${entry.key}`;
  const hooks = FX_CARD_HOOKS.flatMap((hook) => { const spec = entry.hooks[hook]; return spec === undefined ? [] : [[hook, spec.motif, spec.palette].filter((p) => p !== undefined).join(" ")]; });
  const aura = entry.aura === undefined ? [] : [["aura", entry.aura.look, entry.aura.scope, entry.aura.palette].filter((p) => p !== undefined).join(" ")];
  return [...hooks, ...aura].join("; ");
}
it("rows", () => {
  const view = fullBoardView();
  const out: string[] = [];
  for (const [id, entry] of FX_CARD_TABLE) {
    if (!id.startsWith("meditative-") || entry.kind === "none" || entry.signature !== true) continue;
    out.push(`${id}\t${entryText(entry)}\t×${motifScale(id, view, 1).toFixed(2)}`);
  }
  writeFileSync("/tmp/rows.tsv", out.join("\n"));
});
