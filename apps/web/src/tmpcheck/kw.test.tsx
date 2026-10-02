import { render } from "@testing-library/react";
import { it } from "vitest";
import { appendFileSync } from "node:fs";
import { CATALOG } from "@jackioh/cards";
import { CardFace } from "../cards/CardFace.tsx";
import { faceModel } from "../cards/model.ts";
import { CardDefsProvider } from "../cards/refContext.tsx";

it("prints", () => {
  for (const id of ["core-003", "core-011", "core-025", "core-055", "core-086", "classic-010", "classicplus-030"]) {
    const def = CATALOG[id];
    if (def === undefined) continue;
    const { container, unmount } = render(<CardDefsProvider defs={CATALOG}><CardFace face={faceModel({ defId: id, def, radiant: false })} layout="full" /></CardDefsProvider>);
    appendFileSync("/tmp/kwout.txt", id + " " + container.querySelector(".card-text")?.outerHTML.slice(0, 900) + "\n");
    unmount();
  }
});
