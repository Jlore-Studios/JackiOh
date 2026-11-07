import { CATALOG } from "@jackioh/cards";
import { CardFace, faceModel } from "../../../apps/web/src/cards/index.ts";
import { FullArtShown, OwnCopy } from "../../../apps/web/src/cards/fullArt.ts";
import { VERSION_OUT_OF_FRAME_ID } from "../../../apps/web/src/cards/versions.ts";

type CardDef = (typeof CATALOG)[string];
const ID = "stinkyscape-080";
describe("debug", () => {
  it("dumps", () => {
    cy.viewport(1280, 720);
    cy.mount(
      <OwnCopy own>
        <div style={{ display: "flex", gap: 8, padding: 8, background: "#1b1f2b" }}>
          {[false, VERSION_OUT_OF_FRAME_ID].map((pick) => (
            <div key={String(pick)} data-face={String(pick)} style={{ width: 120, height: 168, flex: "none" }}>
              <FullArtShown defId={ID} pick={pick}>
                <CardFace face={faceModel({ defId: ID, def: CATALOG[ID] as CardDef, radiant: false })} layout="full" worn />
              </FullArtShown>
            </div>
          ))}
        </div>
      </OwnCopy>,
    );
    cy.get(".cf-art").should("exist");
    cy.document().its("fonts.status").should("eq", "loaded");
    cy.get('[data-face="false"]').screenshot("debug-normal", { overwrite: true });
    cy.get('[data-face="version-out-of-frame"]').screenshot("debug-out", { overwrite: true });
    cy.document().then((doc) => {
      for (const pick of ["false", VERSION_OUT_OF_FRAME_ID]) {
        const cf = doc.querySelector(`[data-face="${pick}"] > .cf`) as HTMLElement;
        const gem = cf.querySelector(".cost-gem") as HTMLElement;
        const b = gem.getBoundingClientRect();
        const x = b.left + b.width / 2;
        const y = b.top + b.height / 2;
        const stack = doc.elementsFromPoint(x, y).map((e) => `${e.tagName}.${(e as HTMLElement).className}`);
        cy.log(`${pick}: gem ${JSON.stringify([b.left, b.top, b.width, b.height])} depth=${cf.getAttribute("data-depth")} stack=${stack.join(" < ")} digits=${gem.textContent}`);
        console.log(`${pick}: gem ${JSON.stringify([b.left, b.top, b.width, b.height])} depth=${cf.getAttribute("data-depth")} stack=${stack.join(" < ")} digits=${gem.textContent} fs=${getComputedStyle(gem).fontSize}`);
        cy.task("layout:report", { spec: "zz-debug", shares: { [`${pick} ${stack.join(" < ")} ${JSON.stringify([b.left, b.top, b.width, b.height])} fs=${getComputedStyle(gem).fontSize} text=${gem.textContent}`]: 1 } });
      }
    });
  });
});
