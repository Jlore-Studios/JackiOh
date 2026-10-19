// R1297000: the deck slots in a real browser. apps/web's vitest suite proves which attributes the
// board writes (game/cosmetics.test.tsx) and what the stylesheet declares (cards/cosmetics.test.tsx);
// what only a style engine can say is measured here, one test per item of the nine: each table
// changes how the viewer's half of the field and its own zones draw and leaves the opponent's zones
// as they were; each backdrop draws between one and BACKDROP_PARTICLE_MAX particles, moves, and lets
// every pointer through to the board; each card back draws differently from the default and says
// nothing; and under Reduce motion every backdrop is still there and nothing in it runs.
//
// A table's look is what the cosmetics can reach, read off the computed styles: the mat the field's
// `::before` paints, and a zone's background, border and slot mark.

import { CardBack } from "../../../apps/web/src/cards/CardBack.tsx";
import {
  DEFAULT_CARD_COSMETICS,
  cosmeticsOf,
  resetCosmeticsForTests,
  writeCosmetics,
  type DeckSlotKind,
} from "../../../apps/web/src/cards/cosmetics.ts";
import { BACKDROP_PARTICLE_MAX } from "../../../apps/web/src/game/config.ts";
import Game from "../../../apps/web/src/game/Game.tsx";
import { writeSettings, __resetSettingsForTests } from "../../../apps/web/src/settings/index.ts";
import { fullBoardView } from "../../../apps/web/src/test/fixtures.ts";

/** A hand-sized card back box, 5:7. */
const BACK_BOX = { width: 150, height: 210 } as const;

/** The items of one kind: every entry but its default. */
function itemsOf(kind: DeckSlotKind): readonly string[] {
  return cosmeticsOf(kind)
    .filter((cosmetic) => !cosmetic.default)
    .map((cosmetic) => cosmetic.id);
}

const TABLES = itemsOf("table");
const FIELDS = itemsOf("field");
const BACKS = itemsOf("cardBack");

function Board() {
  return (
    <div className="app-shell app-shell--wide">
      <Game view={fullBoardView()} legal={[]} onAction={() => undefined} wearCosmetics />
    </div>
  );
}

function must<T extends Element>(found: T | null, what: string): T {
  if (found === null) throw new Error(`no ${what}`);
  return found;
}

/** What a table can change on one zone, as one string: its fill, its border and its slot mark. */
function zoneLook(doc: Document, side: "you" | "opponent", row: "units" | "backrow"): string {
  const zone = must(doc.querySelector<HTMLElement>(`.zone[data-side="${side}"][data-row="${row}"]`), `${side} ${row} zone`);
  const style = getComputedStyle(zone);
  const mark = getComputedStyle(zone, "::before");
  return [
    style.backgroundColor,
    style.backgroundImage,
    style.borderTopColor,
    style.borderTopWidth,
    style.borderLeftColor,
    mark.borderTopColor,
    mark.backgroundImage,
  ].join(" | ");
}

/** The viewer's mat, the viewer's two zones and the opponent's two, as read off the field. */
function tableLooks(doc: Document): { mat: string; own: string; theirs: string } {
  const field = must(doc.querySelector<HTMLElement>(".field"), "field");
  const mat = getComputedStyle(field, "::before");
  return {
    mat: [mat.content, mat.backgroundImage, mat.backgroundColor].join(" | "),
    own: [zoneLook(doc, "you", "units"), zoneLook(doc, "you", "backrow")].join(" || "),
    theirs: [zoneLook(doc, "opponent", "units"), zoneLook(doc, "opponent", "backrow")].join(" || "),
  };
}

function backdropOf(doc: Document): HTMLElement {
  return must(doc.querySelector<HTMLElement>(".board-backdrop"), "backdrop");
}

function running(root: Element): boolean {
  return root.getAnimations({ subtree: true }).some((animation) => animation.playState === "running");
}

function clean(): void {
  __resetSettingsForTests();
  resetCosmeticsForTests();
  writeCosmetics({ ...DEFAULT_CARD_COSMETICS });
}

describe("R1297000 the tables", () => {
  beforeEach(() => {
    clean();
    cy.viewport(1280, 800);
  });

  after(clean);

  for (const table of TABLES) {
    it(`R1297000 ${table} draws under the viewer's half and leaves the opponent's zones as they were`, () => {
      const base: { looks?: ReturnType<typeof tableLooks> } = {};
      cy.mount(<Board />);
      cy.get(".field .zone").should("have.length.greaterThan", 0);
      cy.document().then((doc) => {
        base.looks = tableLooks(doc);
        expect(doc.querySelector(".field")?.hasAttribute("data-table"), "the default board names no table").to.equal(false);
      });
      cy.then(() => {
        writeCosmetics({ table });
      });
      cy.get(`.field[data-table="${table}"]`).should("exist");
      cy.document().should((doc) => {
        const worn = tableLooks(doc);
        const before = base.looks;
        expect(before, "the default board's look").to.not.equal(undefined);
        expect(worn.mat, `${table}'s mat`).to.not.equal(before?.mat);
        expect(worn.own, `${table}'s own zones`).to.not.equal(before?.own);
        expect(worn.theirs, "the opponent's zones").to.equal(before?.theirs);
      });
    });
  }
});

describe("R1297000 the backdrops", () => {
  beforeEach(() => {
    clean();
    cy.viewport(1280, 800);
  });

  after(clean);

  for (const field of FIELDS) {
    it(`R1297000 ${field} draws its particles, moves and takes no input`, () => {
      writeCosmetics({ field });
      cy.mount(<Board />);
      cy.get(`.board-backdrop[data-backdrop="${field}"]`).should("exist");
      cy.document().should((doc) => {
        const backdrop = backdropOf(doc);
        const particles = backdrop.querySelectorAll(".board-backdrop-particle").length;
        expect(particles, "its particles").to.be.within(1, BACKDROP_PARTICLE_MAX);
        expect(getComputedStyle(backdrop).backgroundImage, "its tint").to.not.equal("none");
        expect(running(backdrop), "its particles move").to.equal(true);

        // It sits behind the board and lets every pointer through: the point at its centre is not its own.
        const box = backdrop.getBoundingClientRect();
        expect(box.width, "it has a size").to.be.greaterThan(0);
        expect(box.height, "it has a size").to.be.greaterThan(0);
        const top = doc.elementFromPoint(box.left + box.width / 2, box.top + box.height / 2);
        expect(top, "something is under the pointer").to.not.equal(null);
        expect(backdrop.contains(top), "the backdrop is not what the pointer meets").to.equal(false);
        expect(getComputedStyle(backdrop).pointerEvents).to.equal("none");
      });
    });
  }

  for (const field of FIELDS) {
    it(`R1297000 ${field} holds its still version under Reduce motion`, () => {
      writeSettings({ reduceMotion: true });
      writeCosmetics({ field });
      cy.mount(<Board />);
      cy.get("html").should("have.attr", "data-reduce-motion", "true");
      cy.get(`.board-backdrop[data-backdrop="${field}"]`).should("exist");
      cy.document().should((doc) => {
        const backdrop = backdropOf(doc);
        expect(backdrop.querySelectorAll(".board-backdrop-particle").length, "its particles are there").to.be.within(1, BACKDROP_PARTICLE_MAX);
        expect(getComputedStyle(backdrop).backgroundImage, "its still tint").to.not.equal("none");
        for (const particle of backdrop.querySelectorAll(".board-backdrop-particle")) {
          expect(getComputedStyle(particle).animationName, "a still particle").to.equal("none");
        }
        expect(running(backdrop), "nothing in it runs").to.equal(false);
      });
    });
  }
});

describe("R1297000 the card backs", () => {
  beforeEach(() => {
    clean();
    cy.viewport(400, 300);
  });

  after(clean);

  function Pair({ back }: { back: string }) {
    return (
      <div style={{ display: "flex", gap: 12, padding: 12, background: "#1b1f2b" }}>
        {(["worn", "plain"] as const).map((which) => (
          <div key={which} data-which={which} style={{ ...BACK_BOX, flex: "none", position: "relative" }}>
            {which === "worn" ? <CardBack back={back} /> : <CardBack />}
          </div>
        ))}
      </div>
    );
  }

  for (const back of BACKS) {
    it(`R1297000 ${back} draws differently from the default back and says nothing`, () => {
      cy.mount(<Pair back={back} />);
      cy.document().should((doc) => {
        const worn = must(doc.querySelector<HTMLElement>('[data-which="worn"] > .cf-back'), "worn back");
        const plain = must(doc.querySelector<HTMLElement>('[data-which="plain"] > .cf-back'), "plain back");
        expect(worn.getBoundingClientRect().width, "the back fills its box").to.be.greaterThan(BACK_BOX.width / 2);
        expect(worn.getAttribute("data-card-back")).to.equal(back);
        expect(plain.hasAttribute("data-card-back"), "the default adds no attribute").to.equal(false);
        expect(getComputedStyle(worn).backgroundImage, back).to.not.equal(getComputedStyle(plain).backgroundImage);
        expect(worn.textContent, "a back has no text").to.equal("");
        expect(worn.querySelector("svg title, svg text"), "nor a title").to.equal(null);
      });
    });
  }
});
